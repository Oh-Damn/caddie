use crate::clipboard::ClipboardLog;
use crate::context::Recents;
use crate::dto::{DeviceDto, SessionDto};
use crate::os::{self, Os};
use crate::persist::Stored;
use crate::plugins::{layout_for, plugin_for};
use crate::protocol::{
    ClipboardPayload, ConversationKind, LayoutPayload, RunningApp, StatePayload,
};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tauri::Manager;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    handle: tauri::AppHandle,
    port: u16,
    pairing_secret: RwLock<String>,
    stored: RwLock<Stored>,
    persist_path: PathBuf,
    events: broadcast::Sender<SnapshotDelta>,
    snapshot: RwLock<Snapshot>,
    os: Box<dyn Os>,
    apps_cache: RwLock<Vec<crate::protocol::RunningApp>>,

    watched_front: RwLock<Option<os::AppInfo>>,
    tiers: Mutex<Tiers>,
    #[cfg(target_os = "macos")]
    app_watch: Mutex<Option<os::appwatch::AppWatch>>,
    agent_watch: Mutex<Option<crate::agentwatch::AgentWatch>>,
    focus_gen: AtomicU64,
    active: Mutex<Option<ActiveClient>>,
    next_token: AtomicU64,
    uninstall_prompt: AtomicBool,
    lan_ip: RwLock<String>,
    mdns: Mutex<Option<crate::mdns::MdnsGuard>>,
    icon_dir: PathBuf,
    icons: Mutex<HashMap<String, Vec<u8>>>,
    snapshot_busy: AtomicBool,
    clipboard: Mutex<ClipboardLog>,
    mic_level: Mutex<u8>,
    recents: Mutex<Recents>,
    agents: Mutex<crate::agents::Ledger>,
    agents_cache: RwLock<Option<crate::protocol::AgentsSummary>>,
}

struct ActiveClient {
    token: u64,
    evict: tokio::sync::mpsc::Sender<()>,

    visible: bool,
}

struct Tiers {
    apps: std::time::Instant,
    media: std::time::Instant,
    tabs: std::time::Instant,
    badge: std::time::Instant,
    clipboard: std::time::Instant,
    agents: std::time::Instant,

    agents_every: std::time::Duration,
}

const APPS_EVERY: std::time::Duration = std::time::Duration::from_secs(30);
const MEDIA_EVERY: std::time::Duration = std::time::Duration::from_secs(8);
const TABS_EVERY: std::time::Duration = std::time::Duration::from_secs(10);
const BADGE_EVERY: std::time::Duration = std::time::Duration::from_secs(15);
const CLIPBOARD_EVERY: std::time::Duration = std::time::Duration::from_secs(3);

const AGENTS_EVERY: std::time::Duration = std::time::Duration::from_secs(60);
const AGENTS_EVERY_UNWATCHED: std::time::Duration = std::time::Duration::from_secs(4);

const CURSOR_SESSIONS: usize = 20;

#[derive(Clone, Copy)]
struct Due {
    apps: bool,
    media: bool,
    tabs: bool,
    badge: bool,
    clipboard: bool,
    agents: bool,
}

impl Due {
    fn all() -> Self {
        Self {
            apps: true,
            media: true,
            tabs: true,
            badge: true,
            clipboard: true,
            agents: true,
        }
    }
}

impl Tiers {
    fn new() -> Self {
        let past = std::time::Instant::now() - std::time::Duration::from_secs(3600);
        Self {
            apps: past,
            media: past,
            tabs: past,
            badge: past,
            clipboard: past,
            agents: past,
            agents_every: AGENTS_EVERY_UNWATCHED,
        }
    }

    fn take(&mut self) -> Due {
        let now = std::time::Instant::now();
        let mut due = Due {
            apps: false,
            media: false,
            tabs: false,
            badge: false,
            clipboard: false,
            agents: false,
        };
        if now.duration_since(self.apps) >= APPS_EVERY {
            self.apps = now;
            due.apps = true;
        }
        if now.duration_since(self.media) >= MEDIA_EVERY {
            self.media = now;
            due.media = true;
        }
        if now.duration_since(self.tabs) >= TABS_EVERY {
            self.tabs = now;
            due.tabs = true;
        }
        if now.duration_since(self.badge) >= BADGE_EVERY {
            self.badge = now;
            due.badge = true;
        }
        if now.duration_since(self.clipboard) >= CLIPBOARD_EVERY {
            self.clipboard = now;
            due.clipboard = true;
        }
        if now.duration_since(self.agents) >= self.agents_every {
            self.agents = now;
            due.agents = true;
        }
        due
    }

    fn set_watched(&mut self, watched: bool) {
        self.agents_every = if watched {
            AGENTS_EVERY
        } else {
            AGENTS_EVERY_UNWATCHED
        };
    }

    fn expire_agents(&mut self) {
        self.agents = std::time::Instant::now() - self.agents_every;
    }

    fn expire(&mut self) {
        let agents_every = self.agents_every;
        *self = Tiers::new();
        self.agents_every = agents_every;
    }

    fn expire_media(&mut self) {
        self.media = std::time::Instant::now() - APPS_EVERY;
    }

    fn expire_tabs(&mut self) {
        self.tabs = std::time::Instant::now() - APPS_EVERY;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MediaScope {
    Player,
    Browser,
}

fn media_scope(action: &str) -> Option<MediaScope> {
    if action == "browser.media_toggle" {
        return Some(MediaScope::Browser);
    }
    if action.starts_with("media.") || action.starts_with("volume.") {
        return Some(MediaScope::Player);
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotDelta {
    None,
    Position,
    Full,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub app_name: String,
    pub bundle_id: String,
    pub plugin_id: String,
    pub window_title: String,
    pub volume: u8,
    pub muted: bool,
    pub now_playing: Option<crate::protocol::NowPlaying>,
    pub browser_now_playing: Option<crate::protocol::NowPlaying>,

    pub browser_owns_session: bool,
    pub tabs: Vec<crate::protocol::BrowserTab>,
    pub call: Option<crate::protocol::CallSession>,
    pub approval: Option<crate::protocol::ApprovalSession>,
    pub unread: Option<u32>,
    pub current_conversation: Option<crate::protocol::Conversation>,
    pub open_conversations: Vec<crate::protocol::Conversation>,
    pub recent_conversations: Vec<crate::protocol::Conversation>,
    pub pinned_conversations: Vec<crate::protocol::Conversation>,
    pub agents: Option<crate::protocol::AgentsSummary>,
}

impl AppState {
    pub fn new(handle: &tauri::AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let _ = PWA_DIST.set(resolve_pwa_dist(handle));
        let dir = handle.path().app_data_dir()?;
        let clip_dir = handle
            .path()
            .app_cache_dir()
            .unwrap_or_else(|_| dir.clone())
            .join("clipboard");
        let persist_path = dir.join("state.json");
        let icon_dir = dir.join("icons");
        std::fs::create_dir_all(&icon_dir)?;
        let stored = Stored::load_or_init(&persist_path)?;
        let (events, _) = broadcast::channel(32);
        let os = os::platform();
        let blank = Snapshot::default();
        let (snap, apps) = collect_snapshot(
            os.as_ref(),
            None,
            &blank,
            &[],
            Due::all(),
            &crate::providers::Enabled::from_stored(&stored),
        );
        let recents = Recents::from_stored(&stored.conversation_recents);
        let inner = Inner {
            handle: handle.clone(),
            port: crate::protocol::DEFAULT_PORT,
            pairing_secret: RwLock::new(new_secret()),
            stored: RwLock::new(stored),
            persist_path,
            events,
            snapshot: RwLock::new(snap),
            os,
            apps_cache: RwLock::new(apps),
            watched_front: RwLock::new(None),
            tiers: Mutex::new(Tiers::new()),
            #[cfg(target_os = "macos")]
            app_watch: Mutex::new(None),
            agent_watch: Mutex::new(None),
            focus_gen: AtomicU64::new(0),
            active: Mutex::new(None),
            next_token: AtomicU64::new(1),
            uninstall_prompt: AtomicBool::new(false),
            lan_ip: RwLock::new(lan_ip()),
            mdns: Mutex::new(None),
            icon_dir,
            icons: Mutex::new(HashMap::new()),
            snapshot_busy: AtomicBool::new(false),
            clipboard: Mutex::new(ClipboardLog::new(clip_dir)),
            mic_level: Mutex::new(75),
            recents: Mutex::new(recents),
            agents: Mutex::new(crate::agents::Ledger::new()),
            agents_cache: RwLock::new(None),
        };
        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    pub fn port(&self) -> u16 {
        self.inner.port
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SnapshotDelta> {
        self.inner.events.subscribe()
    }

    pub fn notify(&self) {
        let _ = self.inner.events.send(SnapshotDelta::Full);
    }

    pub fn notify_state(&self) {
        let _ = self.inner.events.send(SnapshotDelta::Position);
    }

    pub fn rotate_secret(&self) {
        *self.inner.pairing_secret.write().expect("lock") = new_secret();
    }

    pub fn pairing_secret(&self) -> String {
        self.inner.pairing_secret.read().expect("lock").clone()
    }

    pub fn fingerprint(&self) -> String {
        let id = self.inner.stored.read().expect("lock").server_id.clone();
        let hash = Sha256::digest(id.as_bytes());
        hex::encode(&hash[..4])
    }

    pub fn is_trusted(&self, device_id: &str) -> bool {
        self.inner
            .stored
            .read()
            .expect("lock")
            .is_trusted(device_id)
    }

    pub fn trust(
        &self,
        device_id: String,
        name: String,
        details: crate::protocol::DeviceDetails,
    ) -> Result<(), std::io::Error> {
        let mut stored = self.inner.stored.write().expect("lock");
        stored.trust(device_id, name, details);
        stored.save(&self.inner.persist_path)
    }

    pub fn mark_seen(
        &self,
        device_id: &str,
        name: &str,
        details: &crate::protocol::DeviceDetails,
        peer: Option<std::net::IpAddr>,
    ) {
        let ip = peer.map(|p| p.to_string()).unwrap_or_default();
        let mac = peer.and_then(crate::net::mac_for_ip).unwrap_or_default();
        let mut stored = self.inner.stored.write().expect("lock");
        let changed = stored.mark_seen(device_id, name, details, &ip, &mac);
        if changed {
            let _ = stored.save(&self.inner.persist_path);
        }
    }

    fn device_dto(&self) -> Option<DeviceDto> {
        let stored = self.inner.stored.read().expect("lock");
        let device = stored.device()?;
        Some(DeviceDto {
            name: device.name.clone(),
            model: device.details.model.clone(),
            platform: device.details.platform.clone(),
            platform_version: device.details.platform_version.clone(),
            ip: device.last_ip.clone(),
            mac: device.last_mac.clone(),
            paired_at: device.created_at.clone(),
            last_seen: device.last_seen.clone(),
            connected: self.client_count() > 0,
        })
    }

    pub fn refresh_snapshot(&self) -> SnapshotDelta {

        let gen = self.inner.focus_gen.load(Ordering::SeqCst);
        let due = self.inner.tiers.lock().expect("lock").take();
        self.poll_clipboard_if_due(due);
        let front = self.inner.watched_front.read().expect("lock").clone();
        let (prev, prev_apps) = (
            self.inner.snapshot.read().expect("lock").clone(),
            self.inner.apps_cache.read().expect("lock").clone(),
        );
        let (mut next, running) =
            collect_snapshot(
                self.inner.os.as_ref(),
                front,
                &prev,
                &prev_apps,
                due,
                &self.enabled_providers(),
            );
        *self.inner.apps_cache.write().expect("lock") = running;
        next.pinned_conversations = self.pinned_for(&next);

        if due.agents {
            self.refresh_agents(next.approval.as_ref());
        }
        next.agents = self.inner.agents_cache.read().expect("lock").clone();
        {
            let mut recents = self.inner.recents.lock().expect("lock");
            if let Some(cur) = next.current_conversation.clone() {
                recents.ingest(&next.bundle_id, cur);
            }
            next.recent_conversations = recents
                .list(&next.bundle_id)
                .into_iter()
                .filter(|r| {
                    let is_current = next
                        .current_conversation
                        .as_ref()
                        .is_some_and(|c| c.name == r.name && c.kind == r.kind);
                    let is_pinned = next
                        .pinned_conversations
                        .iter()
                        .any(|p| p.name == r.name && p.kind == r.kind);
                    !is_current && !is_pinned
                })
                .collect();
            let recents_map = recents.snapshot();
            drop(recents);
            self.save_recents_if_changed(recents_map);
        }
        let mut cur = self.inner.snapshot.write().expect("lock");
        if self.inner.focus_gen.load(Ordering::SeqCst) != gen {
            tracing::debug!("discarded stale snapshot for {}", next.bundle_id);
            return SnapshotDelta::None;
        }
        if *cur == next {
            return SnapshotDelta::None;
        }
        let delta = if position_only(&cur, &next) {
            SnapshotDelta::Position
        } else {
            SnapshotDelta::Full
        };
        *cur = next;
        delta
    }

    pub fn try_refresh_snapshot(&self) -> Option<SnapshotDelta> {
        if self
            .inner
            .snapshot_busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return None;
        }
        struct Reset<'a>(&'a AtomicBool);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release);
            }
        }
        let _guard = Reset(&self.inner.snapshot_busy);
        Some(self.refresh_snapshot())
    }

    pub fn snapshot(&self) -> Snapshot {
        self.inner.snapshot.read().expect("lock").clone()
    }

    pub fn layout(&self) -> LayoutPayload {
        let snap = self.snapshot();
        layout_for(&snap.plugin_id, &snap.app_name)
    }

    pub fn state_payload(&self) -> StatePayload {
        let snap = self.snapshot();
        StatePayload {
            app_name: snap.app_name,
            bundle_id: snap.bundle_id,
            plugin_id: snap.plugin_id,
            window_title: snap.window_title,
            volume: snap.volume,
            muted: snap.muted,
            now_playing: snap.now_playing,
            browser_now_playing: snap.browser_now_playing,
            browser_media_owned: snap.browser_owns_session,
            tabs: snap.tabs,
            call: snap.call,
            approval: snap.approval,
            unread: snap.unread,
            current_conversation: snap.current_conversation,
            open_conversations: snap.open_conversations,
            recent_conversations: snap.recent_conversations,
            pinned_conversations: snap.pinned_conversations,
            agents: snap.agents,
        }
    }

    fn on_main<F>(&self, f: F) -> Result<(), String>
    where
        F: FnOnce() -> Result<(), String> + Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        self.inner
            .handle
            .run_on_main_thread(move || {
                let _ = tx.send(f());
            })
            .map_err(|e| e.to_string())?;
        rx.recv_timeout(std::time::Duration::from_secs(2))
            .map_err(|_| "main thread timed out".to_string())?
    }

    pub fn run_command(
        &self,
        action: &str,
        value: Option<f64>,
        target: Option<String>,
    ) -> Result<(), String> {
        match action {
            "media.play_pause" => {
                let os = self.clone();
                let bundle = self.checked_media_target(target)?;
                self.on_main(move || os.inner.os.media_play_pause(bundle.as_deref()))
            }
            "media.next" => {
                let os = self.clone();
                let bundle = self.checked_media_target(target)?;
                self.on_main(move || os.inner.os.media_next(bundle.as_deref()))
            }
            "media.prev" => {
                let os = self.clone();
                let bundle = self.checked_media_target(target)?;
                self.on_main(move || os.inner.os.media_prev(bundle.as_deref()))
            }
            "media.shuffle" => self.inner.os.media_shuffle(),
            "media.like" => {
                let os = self.clone();
                let bundle = self.checked_media_target(target)?;
                if os.inner.os.media_like(bundle.as_deref())? {
                    Ok(())
                } else {
                    let key_os = self.clone();
                    let sent = self.on_main(move || key_os.inner.os.media_like_key());
                    os.inner.os.media_like_restore();
                    sent
                }
            }
            "media.seek_back" => self.inner.os.media_seek_back(),
            "media.seek_forward" => self.inner.os.media_seek_forward(),
            "media.repeat" => self.inner.os.media_repeat(),
            "media.airplay" => self.inner.os.media_airplay(),
            "volume.set" => {
                let v = value.unwrap_or(0.0).clamp(0.0, 100.0) as u8;
                if v > 0 {
                    let _ = self.inner.os.muted_set(false);
                }
                self.inner.os.volume_set(v)
            }
            "volume.toggle_mute" => {
                let muted = self.inner.os.muted_get()?;
                self.inner.os.muted_set(!muted)
            }
            "volume.up" => {
                let v = self.inner.os.volume_get().unwrap_or(0);
                let _ = self.inner.os.muted_set(false);
                self.inner.os.volume_set(v.saturating_add(10).min(100))
            }
            "volume.down" => {
                let v = self.inner.os.volume_get().unwrap_or(0);
                self.inner.os.volume_set(v.saturating_sub(10))
            }
            "app.focus" => {
                let snap = self.snapshot();
                let bundle = target.filter(|s| !s.is_empty()).unwrap_or_else(|| {
                    snap.now_playing
                        .as_ref()
                        .map(|n| n.source_bundle_id.clone())
                        .filter(|s| !s.is_empty())
                        .unwrap_or(snap.bundle_id.clone())
                });
                let window = value.filter(|v| *v >= 1.0).map(|v| v.round() as u32);
                self.inner.os.focus_app(&bundle, window)?;

                if self.apply_focus(&bundle) {
                    self.notify();
                }
                self.settle_focus(&bundle);
                self.fill_focus_detail(&bundle);
                Ok(())
            }
            "browser.media_toggle" => {
                let snap = self.snapshot();
                let cmd = parse_browser_cmd(target, &snap.bundle_id);
                let packed = value
                    .filter(|v| *v >= 1.0)
                    .map(|v| v.round() as u32)
                    .unwrap_or(0);
                let window = packed >> 16;
                let index = packed & 0xffff;
                let tab = resolve_browser_tab(&snap, window, index, &cmd)
                    .ok_or_else(|| "no media tab".to_string())?;
                self.inner.os.tab_media_toggle(&cmd.bundle, &tab)
            }
            "browser.activate_tab" => {
                let snap = self.snapshot();
                let cmd = parse_browser_cmd(target, &snap.bundle_id);
                let packed = value
                    .filter(|v| *v >= 1.0)
                    .map(|v| v.round() as u32)
                    .unwrap_or(0);
                let window = packed >> 16;
                let index = packed & 0xffff;
                let tab = resolve_browser_tab(&snap, window, index, &cmd)
                    .ok_or_else(|| "bad tab".to_string())?;
                self.inner.os.activate_browser_tab(&cmd.bundle, &tab)?;
                self.apply_tab_activation(&tab);
                self.settle_focus(&cmd.bundle);
                Ok(())
            }
            "call.toggle_mic" => self.toggle_input_mute(),
            "call.toggle_camera" => self.call_camera(),
            "call.deafen" => {
                let os = self.clone();
                self.on_main(move || os.inner.os.send_shortcut("shortcut.send:meta+shift+d"))
            }
            "call.leave" => self.call_leave(),
            "agents.ceiling" => {

                let ceiling = crate::agents::clamp_ceiling(value.unwrap_or(0.0));
                {
                    let mut stored = self.inner.stored.write().expect("lock");
                    stored.agent_session_ceiling = ceiling;
                    let _ = stored.save(&self.inner.persist_path);
                }
                let snap = self.snapshot();
                self.refresh_agents(snap.approval.as_ref());
                Ok(())
            }
            "approval.allow" => self.approval_act(true),
            "approval.deny" => self.approval_act(false),
            "conversation.jump" => self.jump_conversation(target),
            "conversation.pin" => self.pin_conversation(target),
            "conversation.unpin" => self.unpin_conversation(target),
            "clipboard.copy" => {
                let id = target.ok_or_else(|| "no clipboard item".to_string())?;
                self.place_clipboard(&id)
            }
            "clipboard.paste" => {
                let id = target.ok_or_else(|| "no clipboard item".to_string())?;
                self.place_clipboard(&id)?;
                let os = self.clone();
                self.on_main(move || os.inner.os.send_shortcut("shortcut.paste"))
            }
            "clipboard.remove" => {
                let id = target.ok_or_else(|| "no clipboard item".to_string())?;
                self.inner.clipboard.lock().expect("lock").remove(&id)
            }
            "clipboard.clear" => {
                self.inner.clipboard.lock().expect("lock").clear();
                Ok(())
            }
            other if other.starts_with("shortcut.") => {
                let os = self.clone();
                let action = other.to_string();
                self.on_main(move || os.inner.os.send_shortcut(&action))
            }
            other => Err(format!("unknown action {other}")),
        }?;
        if action == "app.focus" {

            return Ok(());
        }
        if action == "browser.activate_tab" {

            return Ok(());
        }
        if action == "clipboard.remove" || action == "clipboard.clear" {
            return Ok(());
        }
        if let Some(scope) = media_scope(action) {
            self.push_media_now(scope, true);
            return Ok(());
        }
        match self.try_refresh_snapshot() {
            None | Some(SnapshotDelta::None) => {}
            Some(SnapshotDelta::Position) => self.notify_state(),
            Some(SnapshotDelta::Full) => self.notify(),
        }
        Ok(())
    }

    fn push_media_now(&self, scope: MediaScope, settle: bool) {
        if settle {

            std::thread::sleep(std::time::Duration::from_millis(120));
        }
        for attempt in 0..3 {
            {
                let mut tiers = self.inner.tiers.lock().expect("lock");
                tiers.expire_media();
                if scope == MediaScope::Browser {
                    tiers.expire_tabs();
                }
            }
            match self.try_refresh_snapshot() {
                Some(SnapshotDelta::Position) => {
                    self.notify_state();
                    return;
                }
                Some(SnapshotDelta::Full) => {
                    self.notify();
                    return;
                }
                Some(SnapshotDelta::None) => return,

                None if attempt == 2 => return,
                None => std::thread::sleep(std::time::Duration::from_millis(120)),
            }
        }
    }

    fn settle_focus(&self, bundle: &str) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(600);
        loop {
            if self
                .inner
                .os
                .frontmost_app()
                .is_some_and(|a| a.bundle_id == bundle)
            {
                return;
            }
            if std::time::Instant::now() >= deadline {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    pub fn on_frontmost_change(&self, app: os::AppInfo) {
        {
            let mut front = self.inner.watched_front.write().expect("lock");
            if front.as_ref() == Some(&app) {
                return;
            }
            *front = Some(app.clone());
        }
        self.inner.tiers.lock().expect("lock").expire();
        if self.apply_focus_to(&app) {
            self.notify();
        }
    }

    fn apply_focus_to(&self, app: &os::AppInfo) -> bool {
        let pins = self.pins_for_bundle(&app.bundle_id);
        let title = self.cached_window_title(&app.bundle_id);
        let conversation = conversation_from_title(&app.bundle_id, &title);
        let mut cur = self.inner.snapshot.write().expect("lock");
        if cur.bundle_id == app.bundle_id {
            return false;
        }
        self.inner.focus_gen.fetch_add(1, Ordering::SeqCst);
        cur.app_name = app.name.clone();
        cur.bundle_id = app.bundle_id.clone();
        cur.plugin_id = plugin_for(&app.bundle_id).to_string();
        cur.window_title = title;
        cur.tabs = Vec::new();
        cur.unread = None;
        cur.current_conversation = conversation;
        cur.open_conversations = Vec::new();
        cur.recent_conversations = Vec::new();
        cur.pinned_conversations = pins;
        true
    }

    pub fn on_media_change(&self) {
        if !self.wants_updates() {
            return;
        }

        let state = self.clone();
        std::thread::spawn(move || state.push_media_now(MediaScope::Player, false));
    }

    #[cfg(target_os = "macos")]
    pub fn start_app_watch(&self) {
        use os::appwatch::WatchEvent;
        let state = self.clone();
        let watch = os::appwatch::AppWatch::spawn(move |event| match event {
            WatchEvent::App(app) => state.on_frontmost_change(app),
            WatchEvent::Media => state.on_media_change(),
        });
        *self.inner.app_watch.lock().expect("lock") = Some(watch);
    }

    pub fn start_agent_watch(&self) {
        let paths = crate::agentwatch::paths_for(&self.enabled_providers());
        let state = self.clone();
        let watch = crate::agentwatch::AgentWatch::spawn(&paths, move || state.on_agent_change());
        let watching = watch.is_some();

        *self.inner.agent_watch.lock().expect("lock") = watch;
        self.inner.tiers.lock().expect("lock").set_watched(watching);
        if watching {
            tracing::info!("watching {} agent path(s)", paths.len());
        } else {
            tracing::info!("no agent paths to watch, polling instead");
        }
    }

    fn on_agent_change(&self) {
        if !self.wants_updates() {
            return;
        }
        self.inner.tiers.lock().expect("lock").expire_agents();
        let snap = self.snapshot();
        self.refresh_agents(snap.approval.as_ref());
        self.notify();
    }

    fn fill_focus_detail(&self, bundle: &str) {
        let title = self.inner.os.frontmost_window_title();
        let mut tabs = if plugin_for(bundle) == "browser" {
            self.inner.os.browser_tabs(bundle)
        } else {
            Vec::new()
        };
        os::mark_active_from_window_title(&mut tabs, &title);
        let mut changed = false;
        {
            let mut cur = self.inner.snapshot.write().expect("lock");
            if cur.bundle_id != bundle {
                return;
            }
            if cur.window_title != title {
                cur.window_title = title;
                changed = true;
            }
            if !tabs.is_empty() && cur.tabs != tabs {
                cur.tabs = tabs;
                changed = true;
            }
        }
        if changed {
            self.notify();
        }
    }

    fn apply_tab_activation(&self, tapped: &crate::protocol::BrowserTab) {
        {
            let mut cur = self.inner.snapshot.write().expect("lock");
            os::mark_activated_identity(&mut cur.tabs, tapped);
        }
        self.notify();
    }

    fn apply_focus(&self, bundle: &str) -> bool {
        let cached = self.cached_running(bundle);
        let Some(cached) = cached else {
            self.refresh_frontmost();
            return true;
        };
        let pins = self.pins_for_bundle(bundle);
        let title = window_title_of(&cached);
        let conversation = conversation_from_title(bundle, &title);
        let mut cur = self.inner.snapshot.write().expect("lock");
        if cur.bundle_id == bundle {
            return false;
        }
        self.inner.focus_gen.fetch_add(1, Ordering::SeqCst);
        cur.app_name = cached.name;
        cur.bundle_id = bundle.to_string();
        cur.plugin_id = plugin_for(bundle).to_string();
        cur.window_title = title;
        cur.tabs = Vec::new();
        cur.unread = None;
        cur.current_conversation = conversation;
        cur.open_conversations = Vec::new();
        cur.recent_conversations = Vec::new();
        cur.pinned_conversations = pins;
        true
    }

    fn refresh_frontmost(&self) {
        let Some(app) = self.inner.os.frontmost_app() else {
            return;
        };
        let pins = self.pins_for_bundle(&app.bundle_id);
        let plugin_id = plugin_for(&app.bundle_id).to_string();
        let title = self.cached_window_title(&app.bundle_id);
        let conversation = conversation_from_title(&app.bundle_id, &title);
        let mut cur = self.inner.snapshot.write().expect("lock");
        if cur.bundle_id == app.bundle_id {
            return;
        }
        self.inner.focus_gen.fetch_add(1, Ordering::SeqCst);
        cur.app_name = app.name;
        cur.bundle_id = app.bundle_id;
        cur.plugin_id = plugin_id;
        cur.window_title = title;
        cur.tabs = Vec::new();
        cur.unread = None;
        cur.current_conversation = conversation;
        cur.open_conversations = Vec::new();
        cur.recent_conversations = Vec::new();
        cur.pinned_conversations = pins;
    }

    fn cached_running(&self, bundle: &str) -> Option<RunningApp> {
        self.inner
            .apps_cache
            .read()
            .expect("lock")
            .iter()
            .find(|a| a.bundle_id == bundle)
            .cloned()
    }

    fn cached_window_title(&self, bundle: &str) -> String {
        self.cached_running(bundle)
            .map(|app| window_title_of(&app))
            .unwrap_or_default()
    }

    pub fn list_apps(&self) -> crate::protocol::AppsPayload {
        crate::protocol::AppsPayload {
            items: self.inner.os.list_apps(),
        }
    }

    pub fn clipboard_payload(&self) -> ClipboardPayload {
        self.poll_clipboard();
        ClipboardPayload {
            items: self.inner.clipboard.lock().expect("lock").items(),
        }
    }

    fn checked_media_target(&self, target: Option<String>) -> Result<Option<String>, String> {
        let snap = self.snapshot();
        let bundle = media_command_target(target, &snap);
        if let Some(b) = bundle.as_deref() {
            if os::is_browser(b) && !snap.browser_owns_session {
                return Err(format!(
                    "{} is not the system media source right now",
                    snap.browser_now_playing
                        .as_ref()
                        .map(|np| np.source_name.clone())
                        .unwrap_or_else(|| "That browser".into())
                ));
            }
        }
        Ok(bundle)
    }

    fn place_clipboard(&self, id: &str) -> Result<(), String> {
        let (is_image, text, path) = {
            let log = self.inner.clipboard.lock().expect("lock");
            let item = log
                .get(id)
                .ok_or_else(|| "missing clipboard item".to_string())?;
            (
                item.kind == crate::clipboard::KIND_IMAGE,
                item.text.clone(),
                log.image_path(id),
            )
        };
        if is_image {
            self.inner.os.clipboard_image_set(&path)?;
        } else {
            self.inner.os.clipboard_set(&text)?;
        }
        self.absorb_clipboard_change();
        Ok(())
    }

    fn absorb_clipboard_change(&self) {
        if let Some(probe) = self.inner.os.clipboard_probe() {
            self.inner
                .clipboard
                .lock()
                .expect("lock")
                .changed(probe.change_count);
        }
    }

    fn refresh_agents(&self, approval: Option<&crate::protocol::ApprovalSession>) {
        let payload = self.agents_payload_with(approval);
        *self.inner.agents_cache.write().expect("lock") = Some(payload.summary);
    }

    pub fn agents_payload(&self) -> crate::protocol::AgentsPayload {
        let snap = self.snapshot();
        self.agents_payload_with(snap.approval.as_ref())
    }

    fn agents_payload_with(
        &self,
        approval: Option<&crate::protocol::ApprovalSession>,
    ) -> crate::protocol::AgentsPayload {
        let enabled = self.enabled_providers();
        let externals = self.external_agents(approval, &enabled);
        let mut ledger = self.inner.agents.lock().expect("lock");
        if !enabled.has(crate::providers::CLAUDE_CODE) {

            ledger.forget();
            return crate::agents::external_only(&externals);
        }
        ledger.refresh();
        ledger.payload(self.agent_limits(), &externals)
    }

    pub fn enabled_providers(&self) -> crate::providers::Enabled {
        crate::providers::Enabled::from_stored(&self.inner.stored.read().expect("lock"))
    }

    pub fn set_provider_enabled(&self, id: &str, on: bool) {
        if crate::providers::get(id).is_none() {
            return;
        }
        {
            let mut stored = self.inner.stored.write().expect("lock");
            let changed = if on {
                stored.providers_off.remove(id)
            } else {
                stored.providers_off.insert(id.to_string())
            };
            if !changed {
                return;
            }
            let _ = stored.save(&self.inner.persist_path);
        }

        self.start_agent_watch();
        let snap = self.snapshot();
        self.refresh_agents(snap.approval.as_ref());
        self.notify();
    }

    fn agent_limits(&self) -> crate::agents::Limits {
        let stored = self.inner.stored.read().expect("lock");
        crate::agents::Limits {
            session: stored.agent_session_ceiling,
            week: stored.agent_week_ceiling,
            week_anchor: stored.agent_week_anchor,
        }
    }

    fn prompt_block(&self, running: bool, can_see: bool, bundles: &[&str]) -> Option<String> {
        if !running {
            return None;
        }
        if !can_see {
            return Some("Accessibility off".into());
        }
        let scanned = bundles
            .iter()
            .filter_map(|b| self.inner.os.approval_reachable(b));
        if scanned.clone().count() > 0 && !scanned.into_iter().any(|ok| ok) {
            return Some("App exposes no accessibility tree".into());
        }
        None
    }

    fn external_agents(
        &self,
        approval: Option<&crate::protocol::ApprovalSession>,
        enabled: &crate::providers::Enabled,
    ) -> Vec<crate::agents::ExternalAgent> {

        let can_see = crate::os::permissions::accessibility_trusted();
        let apps = self.inner.apps_cache.read().expect("lock");
        let waiting_bundle = approval.map(|a| a.bundle_id.clone());
        let mut out = Vec::new();
        for provider in crate::providers::ALL {
            let (id, name, bundles) = (provider.id, provider.name, provider.bundles);
            if id == crate::providers::CLAUDE_CODE || !enabled.has(id) {
                continue;
            }
            if id == crate::providers::CODEX {
                if crate::agents::codex_installed() {
                    out.push(crate::agents::ExternalAgent {
                        id: id.into(),
                        name: name.into(),
                        running: false,
                        waiting: 0,
                        active: 0,
                        blocked: None,
                        sessions: Vec::new(),
                    });
                }
                continue;
            }
            let running = apps.iter().any(|a| bundles.contains(&a.bundle_id.as_str()));

            if id == crate::providers::CURSOR {
                out.push(self.cursor_agent(id, name, running, can_see, bundles));
                continue;
            }
            let waiting = u32::from(
                waiting_bundle
                    .as_deref()
                    .is_some_and(|b| bundles.contains(&b)),
            );
            out.push(crate::agents::ExternalAgent {
                id: (*id).into(),
                name: (*name).into(),
                running,
                waiting,
                active: u32::from(running),
                blocked: self.prompt_block(running, can_see, bundles),
                sessions: Vec::new(),
            });
        }
        out
    }

    fn cursor_agent(
        &self,
        id: &str,
        name: &str,
        running: bool,
        can_see: bool,
        bundles: &[&str],
    ) -> crate::agents::ExternalAgent {
        let now = chrono::Utc::now().timestamp();
        let read = if running {
            crate::cursor::read(CURSOR_SESSIONS, now)
        } else {
            None
        };
        let Some(state) = read else {
            return crate::agents::ExternalAgent {
                id: id.into(),
                name: name.into(),
                running,
                waiting: 0,
                active: u32::from(running),
                blocked: self.prompt_block(running, can_see, bundles),
                sessions: Vec::new(),
            };
        };
        crate::agents::ExternalAgent {
            id: id.into(),
            name: name.into(),
            running,
            waiting: state.waiting,
            active: state.active,
            blocked: None,
            sessions: state
                .sessions
                .iter()
                .map(crate::agents::cursor_session)
                .collect(),
        }
    }

    fn poll_clipboard_if_due(&self, due: Due) {
        if due.clipboard {
            self.poll_clipboard();
        }
    }

    fn poll_clipboard(&self) {
        let Some(probe) = self.inner.os.clipboard_probe() else {
            return;
        };
        {
            let mut log = self.inner.clipboard.lock().expect("lock");
            if !log.changed(probe.change_count) {
                return;
            }
        }
        if probe.concealed {
            return;
        }
        match probe.kind {
            os::PasteKind::Text => {
                let Some(text) = self.inner.os.clipboard_get() else {
                    return;
                };
                self.inner.clipboard.lock().expect("lock").ingest(text);
            }
            os::PasteKind::Image => self.poll_clipboard_image(probe.file_url),
            os::PasteKind::None => {}
        }
    }

    fn poll_clipboard_image(&self, file_url: bool) {
        let id = uuid::Uuid::new_v4().to_string();
        let dest = {
            let log = self.inner.clipboard.lock().expect("lock");
            log.image_path(&id)
        };
        let Some(meta) = self.inner.os.clipboard_image_get(&dest) else {
            return;
        };
        if crate::clipboard::ClipboardLog::is_file_icon(meta.width, meta.height, file_url) {
            let _ = std::fs::remove_file(&dest);
            return;
        }
        self.inner.clipboard.lock().expect("lock").ingest_image(
            id,
            meta.width,
            meta.height,
            meta.bytes,
        );
    }

    pub fn clipboard_image_file(&self, id: &str, thumb: bool) -> Option<PathBuf> {
        let log = self.inner.clipboard.lock().expect("lock");
        let item = log.get(id)?;
        if item.kind != crate::clipboard::KIND_IMAGE {
            return None;
        }
        let path = if thumb {
            log.thumb_path(id)
        } else {
            log.image_path(id)
        };
        path.exists().then_some(path)
    }

    fn toggle_input_mute(&self) -> Result<(), String> {
        let cur = self.inner.os.input_volume_get().unwrap_or(0);
        if cur == 0 {
            let restore = (*self.inner.mic_level.lock().expect("lock")).max(10);
            self.inner.os.input_volume_set(restore)
        } else {
            *self.inner.mic_level.lock().expect("lock") = cur;
            self.inner.os.input_volume_set(0)
        }
    }

    fn call_camera(&self) -> Result<(), String> {
        let snap = self.snapshot();
        let call = snap.call.ok_or_else(|| "no active call".to_string())?;
        let chord = crate::call::camera_chord(&call.app).ok_or_else(|| "no camera".to_string())?;
        if call.app == "meet" {
            let found = snap
                .tabs
                .iter()
                .find(|t| os::looks_like_meet_tab(&t.title, &t.url))
                .cloned()
                .ok_or_else(|| "no meet tab".to_string())?;
            self.inner
                .os
                .activate_browser_tab(&call.bundle_id, &found)?;
        } else {
            self.inner.os.focus_app(&call.bundle_id, None)?;
        }
        std::thread::sleep(std::time::Duration::from_millis(180));
        let action = chord.to_string();
        let os = self.clone();
        self.on_main(move || os.inner.os.send_shortcut(&action))
    }

    fn call_leave(&self) -> Result<(), String> {
        let snap = self.snapshot();
        let call = snap.call.ok_or_else(|| "no active call".to_string())?;
        if call.app != "slack" {
            return Err("leave is only for Slack huddle".into());
        }
        self.inner.os.focus_app(&call.bundle_id, None)?;
        std::thread::sleep(std::time::Duration::from_millis(180));
        let os = self.clone();
        self.on_main(move || os.inner.os.send_shortcut("shortcut.send:meta+shift+h"))
    }

    fn approval_act(&self, allow: bool) -> Result<(), String> {
        let snap = self.snapshot();
        let pending = snap
            .approval
            .clone()
            .ok_or_else(|| "no approval".to_string())?;
        self.inner.os.focus_app(&pending.bundle_id, None)?;
        self.settle_focus(&pending.bundle_id);
        if self.inner.os.approval_press(&pending.bundle_id, allow) {
            return Ok(());
        }
        if pending.app != "cursor" {
            return Ok(());
        }
        let chord = if allow {
            "shortcut.send:enter"
        } else {
            "shortcut.send:escape"
        };
        let action = chord.to_string();
        let os = self.clone();
        self.on_main(move || os.inner.os.send_shortcut(&action))
    }

    fn jump_conversation(&self, target: Option<String>) -> Result<(), String> {
        let name = target
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "no conversation".to_string())?;
        let snap = self.snapshot();
        if snap.bundle_id.is_empty() {
            return Err("no app".into());
        }
        self.inner.os.focus_app(&snap.bundle_id, None)?;
        std::thread::sleep(std::time::Duration::from_millis(180));
        let os = self.clone();
        self.on_main(move || os.inner.os.send_shortcut("shortcut.send:meta+k"))?;
        std::thread::sleep(std::time::Duration::from_millis(160));
        let os = self.clone();
        let typed = name;
        self.on_main(move || os.inner.os.type_text(&typed))?;
        std::thread::sleep(std::time::Duration::from_millis(80));
        let os = self.clone();
        self.on_main(move || os.inner.os.send_shortcut("shortcut.send:enter"))
    }

    fn pin_conversation(&self, target: Option<String>) -> Result<(), String> {
        let name = target
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "no conversation".to_string())?;
        let snap = self.snapshot();
        if snap.bundle_id.is_empty() {
            return Err("no app".into());
        }
        let conv = self.resolve_conversation(&snap, &name).unwrap_or_else(|| {
            crate::protocol::Conversation {
                name: name.clone(),
                kind: crate::protocol::ConversationKind::Dm,
                workspace: snap
                    .current_conversation
                    .as_ref()
                    .map(|c| c.workspace.clone())
                    .unwrap_or_default(),
                window_index: None,
            }
        });
        let mut stored = self.inner.stored.write().expect("lock");
        stored.pin_conversation(&snap.bundle_id, conv);
        stored
            .save(&self.inner.persist_path)
            .map_err(|e| e.to_string())
    }

    fn unpin_conversation(&self, target: Option<String>) -> Result<(), String> {
        let name = target
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "no conversation".to_string())?;
        let bundle = self.snapshot().bundle_id;
        if bundle.is_empty() {
            return Err("no app".into());
        }
        let mut stored = self.inner.stored.write().expect("lock");
        stored.unpin_conversation(&bundle, &name);
        stored
            .save(&self.inner.persist_path)
            .map_err(|e| e.to_string())
    }

    fn resolve_conversation(
        &self,
        snap: &Snapshot,
        name: &str,
    ) -> Option<crate::protocol::Conversation> {
        if let Some(c) = snap
            .current_conversation
            .as_ref()
            .filter(|c| c.name.eq_ignore_ascii_case(name))
        {
            return Some(c.clone());
        }
        snap.open_conversations
            .iter()
            .chain(snap.recent_conversations.iter())
            .chain(snap.pinned_conversations.iter())
            .find(|c| c.name.eq_ignore_ascii_case(name))
            .cloned()
    }

    fn pins_for_bundle(&self, bundle_id: &str) -> Vec<crate::protocol::Conversation> {
        if bundle_id.is_empty() {
            return Vec::new();
        }
        self.inner
            .stored
            .read()
            .expect("lock")
            .conversation_pins
            .get(bundle_id)
            .cloned()
            .unwrap_or_default()
    }

    fn pinned_for(&self, snap: &Snapshot) -> Vec<crate::protocol::Conversation> {
        if snap.bundle_id.is_empty() {
            return Vec::new();
        }
        let stored = self.inner.stored.read().expect("lock");
        let pins = stored
            .conversation_pins
            .get(&snap.bundle_id)
            .cloned()
            .unwrap_or_default();
        pins.into_iter()
            .map(|mut p| {
                if let Some(cur) = snap.current_conversation.as_ref() {
                    if cur.name.eq_ignore_ascii_case(&p.name) && cur.kind == p.kind {
                        p.window_index = cur.window_index;
                        return p;
                    }
                }
                if let Some(open) = snap
                    .open_conversations
                    .iter()
                    .find(|o| o.name.eq_ignore_ascii_case(&p.name) && o.kind == p.kind)
                {
                    p.window_index = open.window_index;
                }
                p
            })
            .collect()
    }

    fn save_recents_if_changed(
        &self,
        map: std::collections::HashMap<String, Vec<crate::protocol::Conversation>>,
    ) {
        let mut stored = self.inner.stored.write().expect("lock");
        if stored.conversation_recents == map {
            return;
        }
        stored.conversation_recents = map;
        let _ = stored.save(&self.inner.persist_path);
    }

    pub fn claim_client(&self, evict: tokio::sync::mpsc::Sender<()>) -> u64 {
        let token = self.inner.next_token.fetch_add(1, Ordering::Relaxed);
        let previous = self
            .inner
            .active
            .lock()
            .expect("lock")
            .replace(ActiveClient {
                token,
                evict,
                visible: true,
            });
        if let Some(prev) = previous {
            tracing::info!("client {} replaced by {token}", prev.token);
            let _ = prev.evict.try_send(());
        }
        token
    }

    pub fn release_client(&self, token: u64) {
        let mut active = self.inner.active.lock().expect("lock");
        if active.as_ref().is_some_and(|a| a.token == token) {
            *active = None;
        }
    }

    pub fn set_client_visible(&self, token: u64, visible: bool) -> bool {
        let mut active = self.inner.active.lock().expect("lock");
        match active.as_mut() {
            Some(client) if client.token == token => {
                let woke = visible && !client.visible;
                client.visible = visible;
                woke
            }
            _ => false,
        }
    }

    pub fn wants_updates(&self) -> bool {
        self.inner
            .active
            .lock()
            .expect("lock")
            .as_ref()
            .is_some_and(|c| c.visible)
    }

    pub fn client_count(&self) -> usize {
        usize::from(self.inner.active.lock().expect("lock").is_some())
    }

    pub fn set_mdns(&self, guard: crate::mdns::MdnsGuard) {
        *self.inner.mdns.lock().expect("lock") = Some(guard);
    }

    pub fn set_lan_ip(&self, ip: String) {
        *self.inner.lan_ip.write().expect("lock") = ip;
    }

    pub fn app_icon(&self, bundle_id: &str) -> Option<Vec<u8>> {
        if bundle_id.is_empty() {
            return None;
        }
        if let Ok(cache) = self.inner.icons.lock() {
            if let Some(bytes) = cache.get(bundle_id) {
                return Some(bytes.clone());
            }
        }
        let bytes = self
            .inner
            .os
            .app_icon_png(bundle_id, &self.inner.icon_dir)?;
        if let Ok(mut cache) = self.inner.icons.lock() {
            cache.insert(bundle_id.to_string(), bytes.clone());
        }
        Some(bytes)
    }

    pub fn session(&self) -> SessionDto {
        let ip = self.inner.lan_ip.read().expect("lock").clone();
        let host = local_host_fqdn();
        let port = self.inner.port;
        let secret = self.pairing_secret();
        let snap = self.snapshot();
        SessionDto {
            http_url: format!("http://{host}:{port}/?s={secret}"),
            fallback_http_url: format!("http://{ip}:{port}/?s={secret}"),
            ws_url: format!("wss://{host}:{port}/ws"),
            pairing_secret: secret,
            fingerprint: self.fingerprint(),
            port,
            client_count: self.client_count(),
            live: self.wants_updates(),
            device: self.device_dto(),
            app_name: snap.app_name,
            plugin_id: snap.plugin_id,
            onboarding_complete: self.onboarding_complete(),
            accessibility_trusted: crate::os::permissions::accessibility_trusted(),
            uninstall_prompt: self.uninstall_prompt(),
            bundled: crate::uninstall::bundled_app().is_some(),
        }
    }

    pub fn onboarding_complete(&self) -> bool {
        self.inner.stored.read().expect("lock").onboarding_complete
    }

    pub fn complete_onboarding(&self) -> Result<(), std::io::Error> {
        let mut stored = self.inner.stored.write().expect("lock");
        stored.complete_onboarding();
        stored.save(&self.inner.persist_path)
    }

    pub fn uninstall_prompt(&self) -> bool {
        self.inner.uninstall_prompt.load(Ordering::SeqCst)
    }

    pub fn set_uninstall_prompt(&self, on: bool) {
        self.inner.uninstall_prompt.store(on, Ordering::SeqCst);
    }
}

fn position_only(prev: &Snapshot, next: &Snapshot) -> bool {
    let mut stripped_prev = prev.clone();
    let mut stripped_next = next.clone();
    if let Some(np) = stripped_prev.now_playing.as_mut() {
        np.position_sec = 0.0;
    }
    if let Some(np) = stripped_next.now_playing.as_mut() {
        np.position_sec = 0.0;
    }
    if let Some(np) = stripped_prev.browser_now_playing.as_mut() {
        np.position_sec = 0.0;
    }
    if let Some(np) = stripped_next.browser_now_playing.as_mut() {
        np.position_sec = 0.0;
    }
    if let Some(call) = stripped_prev.call.as_mut() {
        call.muted = false;
    }
    if let Some(call) = stripped_next.call.as_mut() {
        call.muted = false;
    }
    stripped_prev.unread = None;
    stripped_next.unread = None;
    stripped_prev == stripped_next
}

fn window_title_of(app: &RunningApp) -> String {
    app.windows
        .iter()
        .filter(|w| !w.title.is_empty())
        .min_by_key(|w| w.index)
        .map(|w| w.title.clone())
        .unwrap_or_default()
}

fn conversation_from_title(bundle: &str, title: &str) -> Option<crate::protocol::Conversation> {
    crate::context::parse_title(bundle, title).filter(|c| c.kind != ConversationKind::Other)
}

fn collect_snapshot(
    os: &dyn Os,
    front: Option<os::AppInfo>,
    prev: &Snapshot,
    prev_apps: &[crate::protocol::RunningApp],
    due: Due,
    enabled: &crate::providers::Enabled,
) -> (Snapshot, Vec<crate::protocol::RunningApp>) {
    let app = front.or_else(|| os.frontmost_app()).unwrap_or(os::AppInfo {
        name: "Desktop".into(),
        bundle_id: String::new(),
    });
    let plugin_id = plugin_for(&app.bundle_id).to_string();

    let quick = os.quick_state();
    let vol = quick
        .as_ref()
        .map(|q| q.volume)
        .or_else(|| os.volume_state());
    let volume = vol
        .map(|v| v.output)
        .unwrap_or_else(|| os.volume_get().unwrap_or(0));
    let muted = vol
        .map(|v| v.muted)
        .unwrap_or_else(|| os.muted_get().unwrap_or(false));

    let window_title = match quick.as_ref() {
        Some(q) if q.front_bundle.is_empty() || q.front_bundle == app.bundle_id => {
            q.window_title.clone()
        }
        Some(_) => String::new(),
        None => os.frontmost_window_title(),
    };

    let running = if due.apps {
        os.list_apps()
    } else {
        prev_apps.to_vec()
    };
    let mut now_playing = if due.media {
        os.now_playing(Some(&app), &running)
    } else if prev.bundle_id == app.bundle_id {
        prev.now_playing.clone()
    } else {
        os.now_playing(Some(&app), &running)
    };
    if let Some(np) = now_playing.as_mut() {
        os.refresh_liked(np);
    }
    let mut tabs = if plugin_id != "browser" {
        Vec::new()
    } else if due.tabs || prev.bundle_id != app.bundle_id {
        os.browser_tabs(&app.bundle_id)
    } else {
        prev.tabs.clone()
    };
    os::mark_active_from_window_title(&mut tabs, &window_title);
    if plugin_id == "browser" {
        os::mark_window_media_audible(&mut tabs);
    }
    let media_audible = media_tab(&tabs).is_some_and(|t| t.audible);
    let (mut browser_now_playing, browser_from_remote) = if plugin_id == "browser" {
        match browser_session_tagged(&app, os.browser_now_playing(&app), &tabs) {
            Some((np, from_remote)) => (Some(np), from_remote),
            None => (None, false),
        }
    } else if let Some(bg) = background_browser(&running) {

        match os.browser_now_playing(&bg) {
            Some(mut np)
                if !now_playing
                    .as_ref()
                    .is_some_and(|n| titles_overlap(&n.title, &np.title)) =>
            {
                np.source_bundle_id = bg.bundle_id.clone();
                np.source_name = bg.name.clone();
                (Some(np), true)
            }
            _ => (None, false),
        }
    } else {
        (None, false)
    };
    if let Some(np) = browser_now_playing.as_mut() {
        let chromium = os::is_chromium(&app.bundle_id);
        let js = if chromium {
            None
        } else {
            media_tab(&tabs)
                .and_then(|tab| os.tab_playback(&app.bundle_id, tab.window_index, tab.tab_index))
        };
        np.playing = browser_playing(chromium, media_audible, js, np.playing, browser_from_remote);
        if !chromium {
            os::mark_audible_tabs(&mut tabs, &np.title);
        }
    }
    if !os::is_chromium(&app.bundle_id) {
        os::mark_media_tab_audible(&mut tabs);
    }
    let mut call = os.detect_call(&running, &tabs, &app.bundle_id);
    if call.is_none() {
        if let Some(tab) = tabs
            .iter()
            .find(|t| os::looks_like_meet_tab(&t.title, &t.url))
        {
            call = Some(crate::protocol::CallSession {
                active: true,
                app: "meet".into(),
                app_name: app.name.clone(),
                bundle_id: app.bundle_id.clone(),
                title: tab.title.clone(),
                muted: false,
                has_camera: true,
                has_deafen: false,
                has_leave: false,
                tab_window_index: Some(tab.window_index),
                tab_index: Some(tab.tab_index),
            });
        }
    }
    if let Some(session) = call.as_mut() {
        session.muted = match vol {
            Some(v) => v.input == 0,
            None => os.input_volume_get().unwrap_or(1) == 0,
        };
    }
    let mut current_conversation = None;
    let mut open_conversations = Vec::new();
    let mut unread = None;
    if crate::context::is_chat_app(&app.bundle_id) {
        current_conversation = crate::context::parse_title(&app.bundle_id, &window_title);
        let windows: Vec<(u32, String)> = running
            .iter()
            .find(|a| a.bundle_id == app.bundle_id)
            .map(|a| {
                a.windows
                    .iter()
                    .map(|w| (w.index, w.title.clone()))
                    .collect()
            })
            .unwrap_or_default();
        if let Some(cur) = current_conversation.as_mut() {
            if let Some((index, _)) = windows.iter().find(|(_, title)| {
                crate::context::parse_title(&app.bundle_id, title)
                    .is_some_and(|c| c.name == cur.name && c.kind == cur.kind)
            }) {
                cur.window_index = Some(*index);
            }
        }
        open_conversations =
            crate::context::parse_windows(&app.bundle_id, &windows, current_conversation.as_ref());
        unread = if due.badge || prev.bundle_id != app.bundle_id {
            os.dock_badge(&app.name)
        } else {
            prev.unread
        };
    }

    let approval = if enabled.wants(crate::providers::Source::Accessibility) {
        let allowed = enabled.bundles();
        let scannable: Vec<crate::protocol::RunningApp> = running
            .iter()
            .filter(|a| allowed.contains(&a.bundle_id.as_str()))
            .cloned()
            .collect();
        os.detect_approval(&scannable, &tabs, &app.bundle_id)
    } else {
        None
    };
    (
        Snapshot {
            app_name: app.name,
            bundle_id: app.bundle_id,
            plugin_id,
            window_title,
            volume,
            muted,
            now_playing,
            browser_now_playing,
            browser_owns_session: browser_from_remote,
            tabs,
            call,
            approval,
            unread,
            current_conversation,
            open_conversations,
            recent_conversations: Vec::new(),
            pinned_conversations: Vec::new(),

            agents: None,
        },
        running,
    )
}

struct BrowserCmd {
    bundle: String,
    profile: String,
    title: String,
}

fn parse_browser_cmd(target: Option<String>, snap_bundle: &str) -> BrowserCmd {
    let raw = target.unwrap_or_default();
    if raw.is_empty() {
        return BrowserCmd {
            bundle: snap_bundle.to_string(),
            profile: String::new(),
            title: String::new(),
        };
    }
    let mut parts = raw.splitn(3, '\t');
    let first = parts.next().unwrap_or("").to_string();
    match (parts.next(), parts.next()) {
        (Some(profile), Some(title)) => BrowserCmd {
            bundle: first,
            profile: profile.to_string(),
            title: title.to_string(),
        },
        _ => BrowserCmd {
            bundle: raw,
            profile: String::new(),
            title: String::new(),
        },
    }
}

fn resolve_browser_tab(
    snap: &Snapshot,
    window: u32,
    index: u32,
    cmd: &BrowserCmd,
) -> Option<crate::protocol::BrowserTab> {
    if !cmd.title.is_empty() {
        let hits: Vec<&crate::protocol::BrowserTab> = snap
            .tabs
            .iter()
            .filter(|t| t.title == cmd.title && t.profile == cmd.profile)
            .collect();
        if hits.len() == 1 {
            return Some(hits[0].clone());
        }
        if let Some(tab) = hits
            .iter()
            .find(|t| t.window_index == window && t.tab_index == index)
        {
            return Some((*tab).clone());
        }
        if let Some(tab) = hits.first() {
            return Some((*tab).clone());
        }
    }
    snap.tabs
        .iter()
        .find(|t| t.window_index == window && t.tab_index == index)
        .cloned()
}

fn media_command_target(target: Option<String>, snap: &Snapshot) -> Option<String> {
    target.filter(|s| !s.is_empty()).or_else(|| {
        snap.now_playing
            .as_ref()
            .filter(|n| os::is_native_player(&n.source_bundle_id))
            .map(|n| n.source_bundle_id.clone())
            .or_else(|| {
                snap.browser_now_playing
                    .as_ref()
                    .map(|n| n.source_bundle_id.clone())
                    .filter(|s| !s.is_empty())
            })
    })
}

fn background_browser(running: &[crate::protocol::RunningApp]) -> Option<os::AppInfo> {
    running
        .iter()
        .find(|a| os::is_browser(&a.bundle_id))
        .map(|a| os::AppInfo {
            name: a.name.clone(),
            bundle_id: a.bundle_id.clone(),
        })
}

fn browser_session_tagged(
    app: &os::AppInfo,
    remote: Option<crate::protocol::NowPlaying>,
    tabs: &[crate::protocol::BrowserTab],
) -> Option<(crate::protocol::NowPlaying, bool)> {
    if plugin_for(&app.bundle_id) != "browser" {
        return None;
    }
    if let Some(mut np) = remote {
        let from_this_browser = np.source_bundle_id == app.bundle_id
            || tabs.iter().any(|t| titles_overlap(&np.title, &t.title));
        if from_this_browser {
            np.source_bundle_id = app.bundle_id.clone();
            np.source_name = app.name.clone();
            return Some((np, true));
        }
    }
    let tab = media_tab(tabs)?;
    Some((
        crate::protocol::NowPlaying {
            title: strip_media_suffix(&tab.title),
            artist: String::new(),
            artwork_url: String::new(),
            playing: false,
            source_name: app.name.clone(),
            source_bundle_id: app.bundle_id.clone(),
            position_sec: 0.0,
            duration_sec: 0.0,
            shuffle: false,
            liked: false,
            repeat_mode: crate::protocol::RepeatMode::Off,
        },
        false,
    ))
}

#[cfg(test)]
fn browser_session(
    app: &os::AppInfo,
    remote: Option<crate::protocol::NowPlaying>,
    tabs: &[crate::protocol::BrowserTab],
) -> Option<crate::protocol::NowPlaying> {
    browser_session_tagged(app, remote, tabs).map(|(np, _)| np)
}

fn titles_overlap(a: &str, b: &str) -> bool {
    let a = a.trim().to_lowercase();
    let b = b.trim().to_lowercase();
    if a.is_empty() || b.is_empty() {
        return false;
    }
    a.contains(&b) || b.contains(&a)
}

fn browser_playing(
    chromium: bool,
    tab_audible: bool,
    js: Option<bool>,
    remote: bool,
    from_remote: bool,
) -> bool {
    if tab_audible {
        return true;
    }
    if let Some(playing) = js {
        return playing;
    }
    if from_remote {
        return remote;
    }
    if chromium {
        return false;
    }
    remote
}

fn media_tab(tabs: &[crate::protocol::BrowserTab]) -> Option<&crate::protocol::BrowserTab> {
    tabs.iter()
        .find(|t| t.active && os::looks_like_media_tab(&t.title, &t.url))
        .or_else(|| {
            tabs.iter()
                .find(|t| os::looks_like_media_tab(&t.title, &t.url))
        })
}

fn strip_media_suffix(title: &str) -> String {
    let lower = title.to_lowercase();
    for suffix in [" - youtube music", " - youtube", " | youtube"] {
        if let Some(idx) = lower.rfind(suffix) {
            return title[..idx].trim().to_string();
        }
    }
    title.trim().to_string()
}

fn new_secret() -> String {
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..6)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

pub fn lan_ip() -> String {
    local_ip_address::local_ip()
        .ok()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "127.0.0.1".into())
}

pub fn local_host_fqdn() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .map(|h| {
            if h.ends_with(".local") {
                h
            } else {
                format!("{h}.local")
            }
        })
        .unwrap_or_else(|| crate::protocol::LAN_HOST.to_string())
}

static PWA_DIST: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

pub fn pwa_dist() -> PathBuf {
    PWA_DIST.get().cloned().unwrap_or_else(pwa_dist_dev)
}

fn pwa_dist_dev() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../pwa/dist")
}

fn resolve_pwa_dist(handle: &tauri::AppHandle) -> PathBuf {
    if let Ok(res) = handle.path().resource_dir() {
        let candidates = [
            res.join("pwa"),
            res.join("pwa/dist"),
            res.join("dist"),
            res.clone(),
        ];
        if let Some(hit) = candidates
            .into_iter()
            .find(|dir| dir.join("index.html").exists())
        {
            return hit;
        }
    }
    pwa_dist_dev()
}

#[cfg(test)]
mod tests {
    use super::{
        browser_session, media_command_target, parse_browser_cmd, resolve_browser_tab, Snapshot,
    };
    use crate::os::AppInfo;
    use crate::protocol::{BrowserTab, NowPlaying, RepeatMode};

    fn chrome() -> AppInfo {
        AppInfo {
            name: "Google Chrome".into(),
            bundle_id: "com.google.Chrome".into(),
        }
    }

    fn youtube_tab() -> BrowserTab {
        BrowserTab {
            title: "Kanye West - Can't Tell Me Nothing - YouTube".into(),
            window_index: 1,
            tab_index: 1,
            active: true,
            audible: false,
            url: "https://www.youtube.com/watch?v=abc".into(),
            profile: String::new(),
            media: false,
        }
    }

    fn spotify(playing: bool) -> NowPlaying {
        NowPlaying {
            title: "Industry Baby".into(),
            artist: "Lil Nas X".into(),
            artwork_url: String::new(),
            playing,
            source_name: "Spotify".into(),
            source_bundle_id: "com.spotify.client".into(),
            position_sec: 10.0,
            duration_sec: 200.0,
            shuffle: false,
            liked: false,
            repeat_mode: RepeatMode::Off,
        }
    }

    fn empty_snap(now_playing: Option<NowPlaying>) -> Snapshot {
        Snapshot {
            app_name: "Google Chrome".into(),
            bundle_id: "com.google.Chrome".into(),
            plugin_id: "browser".into(),
            window_title: "YouTube".into(),
            volume: 50,
            muted: false,
            now_playing,
            browser_now_playing: None,
            browser_owns_session: false,
            tabs: Vec::new(),
            call: None,
            approval: None,
            unread: None,
            current_conversation: None,
            open_conversations: Vec::new(),
            recent_conversations: Vec::new(),
            pinned_conversations: Vec::new(),
            agents: None,
        }
    }

    #[test]
    fn browser_session_ignores_spotify_remote() {
        let np = browser_session(&chrome(), Some(spotify(true)), &[youtube_tab()]).unwrap();
        assert_eq!(np.source_bundle_id, "com.google.Chrome");
        assert!(!np.playing);
    }

    #[test]
    fn resolve_tab_prefers_title_over_shuffled_index() {
        let mut snap = empty_snap(None);
        let mut youtube = youtube_tab();
        youtube.window_index = 2;
        youtube.profile = "abhishek".into();
        let mut about = youtube_tab();
        about.title = "About Page".into();
        about.window_index = 1;
        about.url.clear();
        about.profile = "Abhishek (oh-damn.com)".into();
        snap.tabs = vec![about, youtube.clone()];
        let cmd = parse_browser_cmd(
            Some(
                "com.google.Chrome\tabhishek\tKanye West - Can't Tell Me Nothing - YouTube".into(),
            ),
            "com.google.Chrome",
        );
        let hit = resolve_browser_tab(&snap, 1, 1, &cmd).unwrap();
        assert_eq!(hit.profile, "abhishek");
        assert_eq!(hit.title, youtube.title);
    }

    #[test]
    fn chrome_remote_session_wins_over_tab_title() {
        let remote = NowPlaying {
            title: "Live set".into(),
            artist: String::new(),
            artwork_url: String::new(),
            playing: true,
            source_name: "Google Chrome".into(),
            source_bundle_id: "com.google.Chrome".into(),
            position_sec: 12.0,
            duration_sec: 240.0,
            shuffle: false,
            liked: false,
            repeat_mode: RepeatMode::Off,
        };
        let np = browser_session(&chrome(), Some(remote), &[youtube_tab()]).unwrap();
        assert!(np.playing);
        assert_eq!(np.title, "Live set");
    }

    #[test]
    fn synthesized_browser_track_is_not_playing() {
        let np = browser_session(&chrome(), None, &[youtube_tab()]).unwrap();
        assert_eq!(np.source_bundle_id, "com.google.Chrome");
        assert!(!np.playing);
        assert_eq!(np.title, "Kanye West - Can't Tell Me Nothing");
    }

    #[test]
    fn matching_remote_title_keeps_playing() {
        let remote = NowPlaying {
            title: "Can't Tell Me Nothing".into(),
            artist: String::new(),
            artwork_url: String::new(),
            playing: true,
            source_name: "Google Chrome".into(),
            source_bundle_id: "com.google.Chrome.helper".into(),
            position_sec: 8.0,
            duration_sec: 120.0,
            shuffle: false,
            liked: false,
            repeat_mode: RepeatMode::Off,
        };
        let np = browser_session(&chrome(), Some(remote), &[youtube_tab()]).unwrap();
        assert!(np.playing);
        assert_eq!(np.source_bundle_id, "com.google.Chrome");
    }

    #[test]
    fn chrome_playing_follows_window_speaker() {
        assert!(super::browser_playing(true, true, None, false, false));

        assert!(!super::browser_playing(true, false, None, true, false));
        assert!(super::browser_playing(
            false,
            false,
            Some(true),
            false,
            false
        ));
        assert!(super::browser_playing(false, false, None, true, false));

        assert!(super::browser_playing(true, false, None, true, true));
        assert!(!super::browser_playing(true, false, None, false, true));
    }

    #[test]
    fn media_command_prefers_explicit_target() {
        let snap = empty_snap(Some(spotify(true)));
        assert_eq!(
            media_command_target(Some("com.google.Chrome".into()), &snap).as_deref(),
            Some("com.google.Chrome")
        );
        assert_eq!(
            media_command_target(None, &snap).as_deref(),
            Some("com.spotify.client")
        );
    }

    #[test]
    fn media_command_falls_back_to_browser() {
        let mut snap = empty_snap(None);
        snap.browser_now_playing = Some(NowPlaying {
            title: "Clip".into(),
            artist: String::new(),
            artwork_url: String::new(),
            playing: false,
            source_name: "Google Chrome".into(),
            source_bundle_id: "com.google.Chrome".into(),
            position_sec: 0.0,
            duration_sec: 0.0,
            shuffle: false,
            liked: false,
            repeat_mode: RepeatMode::Off,
        });
        assert_eq!(
            media_command_target(None, &snap).as_deref(),
            Some("com.google.Chrome")
        );
    }

    #[test]
    #[ignore]
    fn focus_cost() {
        let os = crate::os::platform();
        let Some(original) = os.frontmost_app() else {
            return;
        };
        let target = "com.apple.finder";

        let start = std::time::Instant::now();
        os.focus_app(target, None).unwrap();
        let focused = start.elapsed();

        let settle = std::time::Instant::now();
        let deadline = settle + std::time::Duration::from_millis(600);
        while std::time::Instant::now() < deadline {
            if os
                .frontmost_app()
                .is_some_and(|a| a.bundle_id.eq_ignore_ascii_case(target))
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let settled = settle.elapsed();

        let refresh = std::time::Instant::now();
        let _ = os.frontmost_app();
        let refreshed = refresh.elapsed();

        println!("focus_app         {focused:?}  <- phone is notified after this");
        println!("settle_focus      {settled:?}  (after the push, not before)");
        println!("frontmost_app     {refreshed:?}  (no longer on the focus path)");
        println!("TIME TO NOTIFY    {focused:?}");

        let _ = os.focus_app(&original.bundle_id, None);
    }

    #[test]
    #[ignore]
    fn snapshot_steps() {
        let os = crate::os::platform();
        macro_rules! t {
            ($label:expr, $body:expr) => {{
                let start = std::time::Instant::now();
                let out = $body;
                println!("{:>28}: {:?}", $label, start.elapsed());
                out
            }};
        }
        let app = t!("frontmost_app", os.frontmost_app()).unwrap();
        let _ = t!("volume_state", os.volume_state());
        let running = t!("list_apps", os.list_apps());
        let _ = t!("now_playing", os.now_playing(Some(&app), &running));
        let tabs = t!("browser_tabs", os.browser_tabs(&app.bundle_id));
        let _ = t!(
            "detect_call",
            os.detect_call(&running, &tabs, &app.bundle_id)
        );
        let _ = t!("frontmost_window_title", os.frontmost_window_title());
        let _ = t!("dock_badge", os.dock_badge(&app.name));
        let _ = t!("clipboard_probe", os.clipboard_probe());
    }

    #[test]
    #[ignore]
    fn snapshot_cost() {
        let os = crate::os::platform();
        for i in 0..3 {
            let start = std::time::Instant::now();
            let blank = super::Snapshot::default();
            let (snap, _) =
                super::collect_snapshot(
                    os.as_ref(),
                    None,
                    &blank,
                    &[],
                    super::Due::all(),
                    &crate::providers::Enabled::default(),
                );
            println!(
                "run {i}: collect_snapshot {:?} (app {}, tabs {})",
                start.elapsed(),
                snap.app_name,
                snap.tabs.len()
            );
        }
        let start = std::time::Instant::now();
        let _ = os.frontmost_app();
        println!("frontmost_app only: {:?}", start.elapsed());
    }
}
