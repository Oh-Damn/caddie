use crate::protocol::ClipboardItem;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAX_ITEMS: usize = 50;
const MAX_IMAGES: usize = 20;
const MAX_CHARS: usize = 8 * 1024;
const PREVIEW_CHARS: usize = 120;
const THUMB_PX: u32 = 256;

const MAX_IMAGE_BYTES: u64 = 32 * 1024 * 1024;

pub const KIND_TEXT: &str = "text";
pub const KIND_IMAGE: &str = "image";

struct Entry {
    item: ClipboardItem,
    hash: String,
}

pub struct ClipboardLog {
    entries: Vec<Entry>,
    dir: PathBuf,
    last_change: Option<i64>,
    suppressed: HashSet<String>,
}

impl ClipboardLog {

    pub fn new(dir: PathBuf) -> Self {
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        Self {
            entries: Vec::new(),
            dir,
            last_change: None,
            suppressed: HashSet::new(),
        }
    }

    pub fn changed(&mut self, change_count: i64) -> bool {
        if self.last_change == Some(change_count) {
            return false;
        }
        self.last_change = Some(change_count);
        true
    }

    pub fn ingest(&mut self, text: String) -> bool {
        let trimmed = text.trim_end_matches('\0').to_string();
        if trimmed.trim().is_empty() || is_sensitive(&trimmed) {
            return false;
        }
        let stored: String = if trimmed.chars().count() > MAX_CHARS {
            trimmed.chars().take(MAX_CHARS).collect()
        } else {
            trimmed
        };
        if self
            .entries
            .first()
            .is_some_and(|e| e.item.kind == KIND_TEXT && e.item.text == stored)
        {
            return false;
        }
        if let Some(idx) = self
            .entries
            .iter()
            .position(|e| e.item.kind == KIND_TEXT && e.item.text == stored)
        {
            let entry = self.entries.remove(idx);
            self.entries.insert(0, entry);
            return true;
        }
        self.entries.insert(
            0,
            Entry {
                hash: String::new(),
                item: ClipboardItem {
                    id: Uuid::new_v4().to_string(),
                    kind: KIND_TEXT.into(),
                    preview: preview(&stored),
                    text: stored,
                    width: None,
                    height: None,
                },
            },
        );
        self.trim();
        true
    }

    pub fn ingest_image(&mut self, id: String, width: u32, height: u32, bytes: u64) -> bool {
        let full = self.image_path(&id);
        if bytes == 0 || bytes > MAX_IMAGE_BYTES {
            let _ = std::fs::remove_file(&full);
            return false;
        }
        let Some(hash) = file_hash(&full) else {
            let _ = std::fs::remove_file(&full);
            return false;
        };
        if self.suppressed.contains(&hash) {
            let _ = std::fs::remove_file(&full);
            return false;
        }
        if let Some(idx) = self.entries.iter().position(|e| e.hash == hash) {
            let _ = std::fs::remove_file(&full);
            let entry = self.entries.remove(idx);
            self.entries.insert(0, entry);
            return true;
        }
        if !make_thumb(&full, &self.thumb_path(&id)) {
            let _ = std::fs::remove_file(&full);
            return false;
        }
        self.suppressed.clear();
        self.entries.insert(
            0,
            Entry {
                hash,
                item: ClipboardItem {
                    id,
                    kind: KIND_IMAGE.into(),
                    preview: format!("Image {width} x {height}"),
                    text: String::new(),
                    width: Some(width),
                    height: Some(height),
                },
            },
        );
        self.trim();
        true
    }

    pub fn items(&self) -> Vec<ClipboardItem> {
        self.entries.iter().map(|e| e.item.clone()).collect()
    }

    pub fn get(&self, id: &str) -> Option<&ClipboardItem> {
        self.entries.iter().map(|e| &e.item).find(|i| i.id == id)
    }

    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        let idx = self
            .entries
            .iter()
            .position(|e| e.item.id == id)
            .ok_or_else(|| "missing clipboard item".to_string())?;
        let entry = self.entries.remove(idx);
        if !entry.hash.is_empty() {
            self.suppressed.insert(entry.hash);
        }
        if entry.item.kind == KIND_IMAGE {
            self.drop_files(&entry.item.id);
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        for entry in &self.entries {
            if !entry.hash.is_empty() {
                self.suppressed.insert(entry.hash.clone());
            }
        }
        let ids: Vec<String> = self
            .entries
            .iter()
            .filter(|e| e.item.kind == KIND_IMAGE)
            .map(|e| e.item.id.clone())
            .collect();
        self.entries.clear();
        for id in ids {
            self.drop_files(&id);
        }
    }

    pub fn is_file_icon(width: u32, height: u32, file_url: bool) -> bool {
        file_url && width == height && matches!(width, 16 | 32 | 64 | 128 | 256 | 512 | 1024)
    }

    pub fn image_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.png"))
    }

    pub fn thumb_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.thumb.png"))
    }

    fn drop_files(&self, id: &str) {
        let _ = std::fs::remove_file(self.image_path(id));
        let _ = std::fs::remove_file(self.thumb_path(id));
    }

    fn trim(&mut self) {
        let mut images = 0;
        let mut drop_ids = Vec::new();
        let mut keep = Vec::with_capacity(self.entries.len());
        for entry in self.entries.drain(..) {
            let is_image = entry.item.kind == KIND_IMAGE;
            if is_image {
                images += 1;
            }
            if keep.len() >= MAX_ITEMS || (is_image && images > MAX_IMAGES) {
                if is_image {
                    drop_ids.push(entry.item.id.clone());
                }
                continue;
            }
            keep.push(entry);
        }
        self.entries = keep;
        for id in drop_ids {
            self.drop_files(&id);
        }
    }
}

fn file_hash(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(hex::encode(Sha256::digest(&bytes)))
}

fn make_thumb(src: &Path, dest: &Path) -> bool {
    let (Some(src), Some(dest)) = (src.to_str(), dest.to_str()) else {
        return false;
    };
    std::process::Command::new("sips")
        .args([
            "-s",
            "format",
            "png",
            "-Z",
            &THUMB_PX.to_string(),
            src,
            "--out",
            dest,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

pub fn is_sensitive(text: &str) -> bool {
    let t = text.trim();
    if t.contains("PRIVATE KEY") && t.contains("BEGIN ") {
        return true;
    }
    t.to_ascii_lowercase().contains("otpauth://")
}

fn preview(text: &str) -> String {
    let line = text.lines().next().unwrap_or("").trim();
    let mut out: String = line.chars().take(PREVIEW_CHARS).collect();
    if line.chars().count() > PREVIEW_CHARS {
        out.push_str("...");
    }
    if text.lines().nth(1).is_some() && !out.ends_with("...") {
        out.push_str(" ...");
    }
    if out.is_empty() {
        out = "(whitespace)".into();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{file_hash, is_sensitive, ClipboardLog, Entry, KIND_IMAGE, KIND_TEXT};
    use crate::protocol::ClipboardItem;

    fn log() -> ClipboardLog {
        ClipboardLog::new(
            std::env::temp_dir().join(format!("caddie-test-{}", uuid::Uuid::new_v4())),
        )
    }

    fn push_image(log: &mut ClipboardLog, id: &str) {
        std::fs::write(log.image_path(id), b"png").unwrap();
        std::fs::write(log.thumb_path(id), b"thumb").unwrap();
        log.entries.insert(
            0,
            Entry {
                hash: "x".into(),
                item: ClipboardItem {
                    id: id.into(),
                    kind: KIND_IMAGE.into(),
                    preview: "Image 1 x 1".into(),
                    text: String::new(),
                    width: Some(1),
                    height: Some(1),
                },
            },
        );
    }

    #[test]
    fn newest_item_moves_to_front() {
        let mut log = log();
        assert!(log.ingest("one".into()));
        assert!(log.ingest("two".into()));
        assert_eq!(log.items()[0].text, "two");
        assert!(log.ingest("one".into()));
        assert_eq!(log.items()[0].text, "one");
        assert_eq!(log.items().len(), 2);
        assert_eq!(log.items()[0].kind, KIND_TEXT);
    }

    #[test]
    fn skips_duplicate_head_and_secrets() {
        let mut log = log();
        assert!(log.ingest("hello".into()));
        assert!(!log.ingest("hello".into()));
        assert!(!log.ingest("-----BEGIN RSA PRIVATE KEY-----\nabc".into()));
        assert!(is_sensitive("otpauth://totp/Example"));
    }

    #[test]
    fn change_count_gates_reads() {
        let mut log = log();
        assert!(log.changed(4));
        assert!(!log.changed(4));
        assert!(log.changed(5));
        assert!(!log.changed(5));
    }

    #[test]
    fn oversized_image_is_rejected_and_file_removed() {
        let mut log = log();
        let id = "abc".to_string();
        std::fs::write(log.image_path(&id), b"not really a png").unwrap();
        assert!(!log.ingest_image(id.clone(), 10, 10, u64::MAX));
        assert!(!log.image_path(&id).exists());
    }

    #[test]
    fn remove_drops_text_and_unknown_id_errors() {
        let mut log = log();
        assert!(log.ingest("keep".into()));
        assert!(log.ingest("drop".into()));
        let drop_id = log.items()[0].id.clone();
        log.remove(&drop_id).unwrap();
        assert_eq!(log.items().len(), 1);
        assert_eq!(log.items()[0].text, "keep");
        assert_eq!(log.remove("missing").unwrap_err(), "missing clipboard item");
    }

    #[test]
    fn remove_image_deletes_files() {
        let mut log = log();
        push_image(&mut log, "img-a");
        assert!(log.image_path("img-a").exists());
        assert!(log.thumb_path("img-a").exists());
        log.remove("img-a").unwrap();
        assert!(log.items().is_empty());
        assert!(!log.image_path("img-a").exists());
        assert!(!log.thumb_path("img-a").exists());
    }

    #[test]
    fn clear_empties_and_deletes_image_files() {
        let mut log = log();
        assert!(log.ingest("hello".into()));
        push_image(&mut log, "img-b");
        log.clear();
        assert!(log.items().is_empty());
        assert!(!log.image_path("img-b").exists());
        assert!(!log.thumb_path("img-b").exists());
    }

    #[test]
    fn finder_icon_sizes_are_skipped_with_file_url() {
        assert!(ClipboardLog::is_file_icon(1024, 1024, true));
        assert!(ClipboardLog::is_file_icon(512, 512, true));
        assert!(!ClipboardLog::is_file_icon(1024, 1024, false));
        assert!(!ClipboardLog::is_file_icon(1388, 788, true));
    }

    #[test]
    fn dismissed_image_hash_is_not_reingested() {
        let mut log = log();
        let id = "img-a";
        let bytes = b"png-bytes";
        std::fs::write(log.image_path(id), bytes).unwrap();
        std::fs::write(log.thumb_path(id), b"thumb").unwrap();
        log.entries.insert(
            0,
            Entry {
                hash: file_hash(&log.image_path(id)).unwrap(),
                item: ClipboardItem {
                    id: id.into(),
                    kind: KIND_IMAGE.into(),
                    preview: "Image 1024 x 1024".into(),
                    text: String::new(),
                    width: Some(1024),
                    height: Some(1024),
                },
            },
        );
        log.remove(id).unwrap();
        let id2 = "img-b".to_string();
        std::fs::write(log.image_path(&id2), bytes).unwrap();
        assert!(!log.ingest_image(id2.clone(), 1024, 1024, bytes.len() as u64));
        assert!(!log.image_path(&id2).exists());
        assert!(log.items().is_empty());
    }
}
