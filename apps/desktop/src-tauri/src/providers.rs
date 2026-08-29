use crate::persist::Stored;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {

    ClaudeTranscripts,

    CursorDatabase,

    Accessibility,

    CodexSessions,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::ClaudeTranscripts => "Claude Code transcripts",
            Source::CursorDatabase => "Cursor's local database",
            Source::Accessibility => "Accessibility tree",
            Source::CodexSessions => "Codex session files",
        }
    }
}

pub struct Provider {
    pub id: &'static str,
    pub name: &'static str,

    pub bundles: &'static [&'static str],
    pub sources: &'static [Source],
}

pub const CLAUDE_CODE: &str = "claude-code";
pub const CLAUDE_DESKTOP: &str = "claude-desktop";
pub const CURSOR: &str = "cursor";
pub const CODEX: &str = "codex";

pub const ALL: &[Provider] = &[
    Provider {
        id: CLAUDE_CODE,
        name: "Claude Code",
        bundles: &[],
        sources: &[Source::ClaudeTranscripts],
    },
    Provider {
        id: CLAUDE_DESKTOP,
        name: "Claude Desktop",
        bundles: &["com.anthropic.claudefordesktop"],
        sources: &[Source::Accessibility],
    },
    Provider {
        id: CURSOR,
        name: "Cursor",
        bundles: &["com.todesktop.230313mzl4w4u92", "com.cursor.Cursor"],
        sources: &[Source::CursorDatabase, Source::Accessibility],
    },
    Provider {
        id: CODEX,
        name: "Codex",
        bundles: &[],
        sources: &[Source::CodexSessions],
    },
];

pub fn get(id: &str) -> Option<&'static Provider> {
    ALL.iter().find(|p| p.id == id)
}

#[derive(Debug, Clone, Default)]
pub struct Enabled {
    off: std::collections::BTreeSet<String>,
}

impl Enabled {
    pub fn from_stored(stored: &Stored) -> Self {
        Self {
            off: stored.providers_off.clone(),
        }
    }

    #[cfg(test)]
    pub fn none() -> Self {
        Self {
            off: ALL.iter().map(|p| p.id.to_string()).collect(),
        }
    }

    pub fn has(&self, id: &str) -> bool {
        !self.off.contains(id)
    }

    pub fn wants(&self, source: Source) -> bool {
        ALL.iter()
            .filter(|p| self.has(p.id))
            .any(|p| p.sources.contains(&source))
    }

    pub fn bundles(&self) -> Vec<&'static str> {
        ALL.iter()
            .filter(|p| self.has(p.id))
            .flat_map(|p| p.bundles.iter().copied())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled(off: &[&str]) -> Enabled {
        Enabled {
            off: off.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn unknown_ids_are_on() {
        assert!(enabled(&[]).has("something-new"));
    }

    #[test]
    fn switching_cursor_off_drops_its_bundles() {
        let on = enabled(&[]);
        let off = enabled(&[CURSOR]);
        assert!(on.bundles().contains(&"com.cursor.Cursor"));
        assert!(!off.bundles().contains(&"com.cursor.Cursor"));
        assert!(off.bundles().contains(&"com.anthropic.claudefordesktop"));
    }

    #[test]
    fn a_shared_source_survives_one_provider_going_off() {

        assert!(enabled(&[CURSOR]).wants(Source::Accessibility));
        assert!(!enabled(&[CURSOR, CLAUDE_DESKTOP]).wants(Source::Accessibility));
    }

    #[test]
    fn a_sole_source_stops_with_its_provider() {
        assert!(enabled(&[]).wants(Source::CursorDatabase));
        assert!(!enabled(&[CURSOR]).wants(Source::CursorDatabase));
        assert!(!enabled(&[CLAUDE_CODE]).wants(Source::ClaudeTranscripts));
    }

    #[test]
    fn every_catalogue_id_resolves() {
        for p in ALL {
            assert_eq!(get(p.id).map(|q| q.id), Some(p.id));
        }
    }
}
