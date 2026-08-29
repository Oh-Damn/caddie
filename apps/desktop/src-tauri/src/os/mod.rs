#[cfg(target_os = "macos")]
mod approval_ax;
#[cfg(target_os = "macos")]
pub mod appwatch;
mod chrome;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod media_keys;
#[cfg(target_os = "macos")]
mod media_remote;
#[cfg(target_os = "macos")]
pub mod pasteboard;
pub mod permissions;
#[cfg(target_os = "macos")]
mod spotify_ax;
#[cfg(not(target_os = "macos"))]
mod stub;

use crate::protocol::{ApprovalSession, BrowserTab, CallSession, NowPlaying, RunningApp};

#[cfg(target_os = "macos")]
const SELF_BUNDLES: &[&str] = &[
    "dev.caddie.desktop",
    "dev.deskthing.desktop",
    "dev.companion.desktop",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    pub name: String,
    pub bundle_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteKind {
    None,
    Text,
    Image,
}

#[derive(Debug, Clone)]
pub struct ClipboardProbe {
    pub change_count: i64,
    pub kind: PasteKind,
    pub concealed: bool,

    pub file_url: bool,
}

#[derive(Debug, Clone)]
pub struct QuickState {
    pub volume: VolumeState,
    pub window_title: String,

    pub front_bundle: String,
}

#[derive(Debug, Clone, Copy)]
pub struct VolumeState {
    pub output: u8,
    pub muted: bool,
    pub input: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct ClipboardImage {
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
}

#[cfg(target_os = "macos")]
pub fn is_companion(bundle_id: &str) -> bool {
    SELF_BUNDLES.contains(&bundle_id)
}

pub trait Os: Send + Sync {
    fn frontmost_app(&self) -> Option<AppInfo>;
    fn frontmost_window_title(&self) -> String;
    fn volume_get(&self) -> Result<u8, String>;

    fn volume_state(&self) -> Option<VolumeState>;

    fn quick_state(&self) -> Option<QuickState>;
    fn volume_set(&self, value: u8) -> Result<(), String>;
    fn muted_get(&self) -> Result<bool, String>;
    fn muted_set(&self, muted: bool) -> Result<(), String>;
    fn media_play_pause(&self, target: Option<&str>) -> Result<(), String>;
    fn media_next(&self, target: Option<&str>) -> Result<(), String>;
    fn media_prev(&self, target: Option<&str>) -> Result<(), String>;
    fn media_shuffle(&self) -> Result<(), String>;

    fn media_like(&self, target: Option<&str>) -> Result<bool, String>;
    fn media_like_key(&self) -> Result<(), String>;
    fn media_like_restore(&self);
    fn media_seek_back(&self) -> Result<(), String>;
    fn media_seek_forward(&self) -> Result<(), String>;
    fn media_repeat(&self) -> Result<(), String>;
    fn media_airplay(&self) -> Result<(), String>;

    fn now_playing(&self, front: Option<&AppInfo>, running: &[RunningApp]) -> Option<NowPlaying>;

    fn refresh_liked(&self, _np: &mut NowPlaying) {}

    fn browser_now_playing(&self, front: &AppInfo) -> Option<NowPlaying>;

    fn tab_playback(&self, bundle_id: &str, window_index: u32, tab_index: u32) -> Option<bool>;

    fn tab_media_toggle(&self, bundle_id: &str, tab: &BrowserTab) -> Result<(), String>;
    fn send_shortcut(&self, action: &str) -> Result<(), String>;
    fn focus_app(&self, bundle_id: &str, window_index: Option<u32>) -> Result<(), String>;
    fn list_apps(&self) -> Vec<RunningApp>;
    fn browser_tabs(&self, bundle_id: &str) -> Vec<BrowserTab>;
    fn activate_browser_tab(&self, bundle_id: &str, tab: &BrowserTab) -> Result<(), String>;
    fn app_icon_png(&self, bundle_id: &str, cache_dir: &std::path::Path) -> Option<Vec<u8>>;
    fn input_volume_get(&self) -> Result<u8, String>;
    fn input_volume_set(&self, value: u8) -> Result<(), String>;
    fn clipboard_get(&self) -> Option<String>;
    fn clipboard_set(&self, text: &str) -> Result<(), String>;
    fn clipboard_probe(&self) -> Option<ClipboardProbe>;
    fn clipboard_image_get(&self, dest: &std::path::Path) -> Option<ClipboardImage>;
    fn clipboard_image_set(&self, src: &std::path::Path) -> Result<(), String>;

    fn detect_call(
        &self,
        running: &[RunningApp],
        tabs: &[BrowserTab],
        tabs_bundle: &str,
    ) -> Option<CallSession>;
    fn detect_approval(
        &self,
        _running: &[RunningApp],
        _tabs: &[BrowserTab],
        _tabs_bundle: &str,
    ) -> Option<ApprovalSession> {
        None
    }
    fn approval_press(&self, _bundle_id: &str, _allow: bool) -> bool {
        false
    }

    fn approval_reachable(&self, _bundle_id: &str) -> Option<bool> {
        None
    }
    fn dock_badge(&self, app_name: &str) -> Option<u32>;
    fn type_text(&self, text: &str) -> Result<(), String>;
}

#[cfg(target_os = "macos")]
pub fn platform() -> Box<dyn Os> {
    Box::new(macos::MacOs)
}

#[cfg(not(target_os = "macos"))]
pub fn platform() -> Box<dyn Os> {
    Box::new(stub::StubOs)
}

pub fn is_browser(bundle_id: &str) -> bool {
    matches!(
        bundle_id,
        "com.google.Chrome"
            | "com.google.Chrome.canary"
            | "com.google.Chrome.beta"
            | "com.google.Chrome.dev"
            | "com.brave.Browser"
            | "com.apple.Safari"
            | "company.thebrowser.Browser"
            | "org.mozilla.firefox"
    )
}

pub fn is_chromium(bundle_id: &str) -> bool {
    matches!(
        bundle_id,
        "com.google.Chrome"
            | "com.google.Chrome.canary"
            | "com.google.Chrome.beta"
            | "com.google.Chrome.dev"
            | "com.brave.Browser"
            | "company.thebrowser.Browser"
    )
}

pub fn is_browser_media_client(client_bundle: &str, front_bundle: &str) -> bool {
    if client_bundle.is_empty() {
        return false;
    }
    if is_browser(client_bundle) || client_bundle == front_bundle {
        return true;
    }
    !front_bundle.is_empty() && client_bundle.starts_with(front_bundle)
}

pub fn parse_tab_playback(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "playing" => Some(true),
        "paused" => Some(false),
        _ => None,
    }
}

pub fn is_native_player(bundle_id: &str) -> bool {
    matches!(bundle_id, "com.spotify.client" | "com.apple.Music")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaRoute {
    Native(&'static str),
    Browser,
    System,
}

pub fn media_route(bundle_id: &str) -> MediaRoute {
    if bundle_id == "com.apple.Music" {
        return MediaRoute::Native("Music");
    }
    if bundle_id == "com.spotify.client" {
        return MediaRoute::Native("Spotify");
    }
    if is_browser(bundle_id) {
        return MediaRoute::Browser;
    }
    MediaRoute::System
}

pub fn looks_like_media_tab(title: &str, url: &str) -> bool {
    let url = url.to_lowercase();
    if url.contains("youtube.com/watch")
        || url.contains("youtube.com/shorts")
        || url.contains("music.youtube.com")
        || url.contains("youtu.be/")
        || url.contains("soundcloud.com")
        || url.contains("open.spotify.com")
    {
        return true;
    }
    let title = title.to_lowercase();
    title.contains(" - youtube") || title.contains(" | youtube") || title.contains("youtube music")
}

pub fn looks_like_meet_tab(title: &str, url: &str) -> bool {
    crate::call::looks_like_meet_tab(title, url)
}

pub fn mark_audible_tabs(tabs: &mut [BrowserTab], title: &str) {
    let needle = normalize_title(title);
    if needle.is_empty() {
        return;
    }
    let mut best: Option<(usize, usize)> = None;
    for (i, tab) in tabs.iter().enumerate() {
        let hay = normalize_title(&tab.title);
        if hay.is_empty() {
            continue;
        }
        let score = if hay.contains(&needle) {
            needle.len()
        } else if needle.contains(&hay) {
            hay.len()
        } else {
            continue;
        };
        if best.map(|(_, s)| score > s).unwrap_or(true) {
            best = Some((i, score));
        }
    }
    if let Some((i, _)) = best {
        tabs[i].audible = true;
    }
}

pub fn mark_active_from_window_title(tabs: &mut [BrowserTab], window_title: &str) {
    if let Some(keep) = match_tab_for_window(tabs, window_title) {
        for (i, tab) in tabs.iter_mut().enumerate() {
            tab.active = i == keep;
        }
        return;
    }
    mark_frontmost_active(tabs);
}

pub fn mark_frontmost_active(tabs: &mut [BrowserTab]) {
    let Some(front) = tabs.iter().map(|t| t.window_index).min() else {
        return;
    };
    let keep = tabs
        .iter()
        .position(|t| t.window_index == front && t.active)
        .or_else(|| tabs.iter().position(|t| t.window_index == front));
    for (i, tab) in tabs.iter_mut().enumerate() {
        tab.active = Some(i) == keep;
    }
}

fn match_tab_for_window(tabs: &[BrowserTab], window_title: &str) -> Option<usize> {
    let ax = strip_window_media_prefix(window_title);
    if ax.is_empty() {
        return None;
    }
    let ax_lower = ax.to_lowercase();
    let mut best: Option<(usize, usize, bool, bool)> = None;
    for (i, tab) in tabs.iter().enumerate() {
        let title = tab.title.trim();
        if title.is_empty() || !window_title_has_tab(ax, title) {
            continue;
        }
        let profile = tab.profile.trim();
        let profile_hit = !profile.is_empty() && ax_lower.contains(&profile.to_lowercase());
        let cand = (title.len(), profile_hit, tab.active);
        if best
            .map(|(_, len, ph, act)| cand > (len, ph, act))
            .unwrap_or(true)
        {
            best = Some((i, cand.0, cand.1, cand.2));
        }
    }
    best.map(|(i, _, _, _)| i)
}

pub(super) fn window_title_has_tab(window_title: &str, tab_title: &str) -> bool {
    if window_title == tab_title {
        return true;
    }
    for sep in [" - ", " \u{2013} ", " \u{2014} "] {
        if window_title.starts_with(&format!("{tab_title}{sep}")) {
            return true;
        }
    }
    false
}

pub(super) fn strip_window_media_prefix(title: &str) -> &str {
    title.trim_start_matches(|c: char| {
        matches!(
            c,
            '\u{1F507}' | '\u{1F508}' | '\u{1F509}' | '\u{1F50A}' | '\u{FE0F}'
        ) || c.is_whitespace()
    })
}

pub fn mark_activated_identity(tabs: &mut [BrowserTab], tapped: &BrowserTab) {
    for tab in tabs.iter_mut() {
        tab.active = tab.title == tapped.title
            && tab.profile == tapped.profile
            && tab.tab_index == tapped.tab_index
            && (tapped.url.is_empty() || tab.url == tapped.url);
    }
}

pub fn mark_media_tab_audible(tabs: &mut [BrowserTab]) {
    if tabs.iter().any(|t| t.audible) {
        return;
    }
    let idx = tabs
        .iter()
        .position(|t| t.active && looks_like_media_tab(&t.title, &t.url))
        .or_else(|| {
            tabs.iter()
                .position(|t| looks_like_media_tab(&t.title, &t.url))
        });
    if let Some(i) = idx {
        tabs[i].audible = true;
    }
}

pub fn mark_window_media_audible(tabs: &mut [BrowserTab]) {
    let noisy: Vec<u32> = tabs
        .iter()
        .filter(|t| t.audible)
        .map(|t| t.window_index)
        .collect();
    if noisy.is_empty() {
        return;
    }
    for tab in tabs {
        tab.audible =
            noisy.contains(&tab.window_index) && looks_like_media_tab(&tab.title, &tab.url);
    }
}

fn normalize_title(value: &str) -> String {
    value.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::mark_audible_tabs;
    use crate::protocol::BrowserTab;

    fn tab(title: &str) -> BrowserTab {
        BrowserTab {
            title: title.into(),
            window_index: 1,
            tab_index: 1,
            active: false,
            audible: false,
            url: String::new(),
            profile: String::new(),
            media: false,
        }
    }

    fn tab_at(title: &str, window: u32, index: u32, active: bool) -> BrowserTab {
        BrowserTab {
            title: title.into(),
            window_index: window,
            tab_index: index,
            active,
            audible: false,
            url: String::new(),
            profile: String::new(),
            media: false,
        }
    }

    #[test]
    fn marks_tab_whose_title_contains_track() {
        let mut tabs = vec![
            tab("WhatsApp"),
            tab("INDUSTRY BABY (feat. Jack Harlow) - YouTube"),
        ];
        mark_audible_tabs(&mut tabs, "INDUSTRY BABY (feat. Jack Harlow)");
        assert!(!tabs[0].audible);
        assert!(tabs[1].audible);
    }

    #[test]
    fn frontmost_active_clears_other_windows() {
        let mut tabs = vec![tab_at("Video", 1, 1, true), tab_at("WhatsApp", 2, 1, true)];
        super::mark_frontmost_active(&mut tabs);
        assert!(tabs[0].active);
        assert!(!tabs[1].active);
    }

    #[test]
    fn window_title_marks_the_other_profile() {
        let mut tabs = vec![
            tab_at("About Page", 1, 1, true),
            tab_at("Pipelines", 1, 2, false),
            tab_at("i'm going back to 505 - YouTube", 2, 1, true),
        ];
        tabs[0].profile = "Abhishek (oh-damn.com)".into();
        tabs[1].profile = "Abhishek (oh-damn.com)".into();
        tabs[2].profile = "abhishek".into();
        super::mark_active_from_window_title(
            &mut tabs,
            "\u{1F50A} i'm going back to 505 - YouTube - Google Chrome \u{2013} abhishek",
        );
        assert!(!tabs[0].active);
        assert!(!tabs[1].active);
        assert!(tabs[2].active);
    }

    #[test]
    fn window_title_does_not_match_a_prefix() {
        assert!(!super::window_title_has_tab(
            "About Page - Google Chrome",
            "About",
        ));
        assert!(super::window_title_has_tab(
            "About Page - Google Chrome",
            "About Page",
        ));
    }

    #[test]
    fn empty_window_title_keeps_lowest_index() {
        let mut tabs = vec![
            tab_at("About Page", 1, 1, true),
            tab_at("YouTube", 2, 1, true),
        ];
        super::mark_active_from_window_title(&mut tabs, "");
        assert!(tabs[0].active);
        assert!(!tabs[1].active);
    }

    #[test]
    fn activated_identity_ignores_shuffled_indexes() {
        let mut tabs = vec![
            tab_at("About Page", 1, 1, true),
            tab_at("S1E4", 2, 1, false),
        ];
        tabs[1].profile = "abhishek".into();
        let tapped = tab_at("S1E4", 1, 1, false);
        let mut tapped = tapped;
        tapped.profile = "abhishek".into();
        super::mark_activated_identity(&mut tabs, &tapped);
        assert!(!tabs[0].active);
        assert!(tabs[1].active);
    }

    #[test]
    fn media_tab_audible_when_youtube_is_open() {
        let mut tabs = vec![
            BrowserTab {
                title: "WhatsApp".into(),
                window_index: 2,
                tab_index: 1,
                active: true,
                audible: false,
                url: "https://web.whatsapp.com/".into(),
                profile: String::new(),
                media: false,
            },
            BrowserTab {
                title: "Akal Ke Ghode Ep11".into(),
                window_index: 1,
                tab_index: 1,
                active: false,
                audible: false,
                url: "https://www.youtube.com/watch?v=abc".into(),
                profile: String::new(),
                media: false,
            },
        ];
        super::mark_media_tab_audible(&mut tabs);
        assert!(!tabs[0].audible);
        assert!(tabs[1].audible);
    }

    #[test]
    fn window_speaker_marks_only_media_tabs() {
        let mut tabs = vec![
            BrowserTab {
                title: "WhatsApp".into(),
                window_index: 2,
                tab_index: 1,
                active: true,
                audible: true,
                url: "https://web.whatsapp.com/".into(),
                profile: String::new(),
                media: false,
            },
            BrowserTab {
                title: "Baat Bangayi - YouTube".into(),
                window_index: 2,
                tab_index: 2,
                active: false,
                audible: true,
                url: "https://www.youtube.com/watch?v=abc".into(),
                profile: String::new(),
                media: false,
            },
            BrowserTab {
                title: "Inbox".into(),
                window_index: 1,
                tab_index: 1,
                active: false,
                audible: false,
                url: "https://mail.google.com/".into(),
                profile: String::new(),
                media: false,
            },
        ];
        super::mark_window_media_audible(&mut tabs);
        assert!(!tabs[0].audible);
        assert!(tabs[1].audible);
        assert!(!tabs[2].audible);
    }

    #[test]
    fn youtube_watch_url_counts_as_media() {
        assert!(super::looks_like_media_tab(
            "Kanye West - Can't Tell Me Nothing",
            "https://www.youtube.com/watch?v=abc",
        ));
        assert!(super::looks_like_media_tab(
            "Kanye West - Can't Tell Me Nothing - YouTube",
            "",
        ));
        assert!(!super::looks_like_media_tab(
            "WhatsApp",
            "https://web.whatsapp.com/",
        ));
    }

    #[test]
    fn meet_tab_from_url() {
        assert!(super::looks_like_meet_tab(
            "Standup",
            "https://meet.google.com/abc-defg-hij",
        ));
        assert!(!super::looks_like_meet_tab(
            "Inbox",
            "https://mail.google.com",
        ));
    }

    #[test]
    fn media_route_prefers_source_bundle() {
        assert_eq!(
            super::media_route("com.spotify.client"),
            super::MediaRoute::Native("Spotify")
        );
        assert_eq!(
            super::media_route("com.apple.Music"),
            super::MediaRoute::Native("Music")
        );
        assert_eq!(
            super::media_route("com.google.Chrome"),
            super::MediaRoute::Browser
        );
        assert_eq!(super::media_route(""), super::MediaRoute::System);
        assert_eq!(
            super::media_route("com.apple.finder"),
            super::MediaRoute::System
        );
    }

    #[test]
    fn parse_tab_playback_reads_page_state() {
        assert_eq!(super::parse_tab_playback("playing"), Some(true));
        assert_eq!(super::parse_tab_playback("PAUSED"), Some(false));
        assert_eq!(super::parse_tab_playback("none"), None);
    }

    #[test]
    fn chrome_helper_counts_as_browser_media() {
        assert!(super::is_browser_media_client(
            "com.google.Chrome.helper",
            "com.google.Chrome",
        ));
        assert!(!super::is_browser_media_client(
            "com.spotify.client",
            "com.google.Chrome",
        ));
    }
}
