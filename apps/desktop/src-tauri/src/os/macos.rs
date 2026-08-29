use super::{is_browser, is_companion, AppInfo, MediaRoute, Os};
use crate::protocol::{AppWindow, BrowserTab, NowPlaying, RepeatMode, RunningApp};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const OSASCRIPT_TIMEOUT: Duration = Duration::from_secs(2);

const CONSENT_TIMEOUT: Duration = Duration::from_secs(180);

const CONSENT_RETRY: Duration = Duration::from_secs(300);
const SPOTIFY_LIKED_TTL: Duration = Duration::from_secs(5);

struct LikedCache {
    track_id: String,
    liked: bool,
    checked_at: Instant,
}

static SPOTIFY_LIKED: Mutex<Option<LikedCache>> = Mutex::new(None);
static LIKE_RESTORE: Mutex<Option<String>> = Mutex::new(None);
static CONSENT_PROBED: Mutex<Vec<(String, Instant)>> = Mutex::new(Vec::new());

pub struct MacOs;

impl Os for MacOs {
    fn frontmost_app(&self) -> Option<AppInfo> {
        let out = osascript(
            r#"tell application "System Events"
  set p to first application process whose frontmost is true
  set n to name of p
  set b to ""
  try
    set b to bundle identifier of p
  end try
  return n & tab & b
end tell"#,
        )
        .ok()?;
        let mut parts = out.split('\t');
        let name = parts.next()?.trim().to_string();
        let bundle_id = parts.next().unwrap_or("").trim().to_string();
        if name.is_empty() {
            return None;
        }
        Some(AppInfo { name, bundle_id })
    }

    fn frontmost_window_title(&self) -> String {
        osascript(
            r#"tell application "System Events"
  set p to first application process whose frontmost is true
  try
    set t to name of front window of p
    if t is not "" then return t
  end try
  try
    repeat with w in windows of p
      set t to ""
      try
        set t to name of w
      end try
      if t is not "" then return t
    end repeat
  end try
  return ""
end tell"#,
        )
        .unwrap_or_default()
    }

    fn volume_get(&self) -> Result<u8, String> {
        let out = osascript("output volume of (get volume settings)")?;
        let v: f64 = out.parse().map_err(|_| "bad volume".to_string())?;
        Ok(v.clamp(0.0, 100.0) as u8)
    }

    fn volume_set(&self, value: u8) -> Result<(), String> {
        osascript(&format!("set volume output volume {value}"))?;
        Ok(())
    }

    fn muted_get(&self) -> Result<bool, String> {
        let out = osascript("output muted of (get volume settings)")?;
        Ok(out.eq_ignore_ascii_case("true"))
    }

    fn muted_set(&self, muted: bool) -> Result<(), String> {
        if muted {
            osascript("set volume with output muted")?;
        } else {
            osascript("set volume without output muted")?;
        }
        Ok(())
    }

    fn media_play_pause(&self, target: Option<&str>) -> Result<(), String> {
        dispatch_media("playpause", target)
    }

    fn media_next(&self, target: Option<&str>) -> Result<(), String> {
        dispatch_media("next track", target)
    }

    fn media_prev(&self, target: Option<&str>) -> Result<(), String> {
        dispatch_media("previous track", target)
    }

    fn media_shuffle(&self) -> Result<(), String> {
        dispatch_player_script(|name| {
            if name == "Music" {
                r#"tell application "Music" to set shuffle enabled to not shuffle enabled"#.into()
            } else {
                r#"tell application "Spotify" to set shuffling to not shuffling"#.into()
            }
        })
    }

    fn media_like(&self, target: Option<&str>) -> Result<bool, String> {
        let name = match super::media_route(target.unwrap_or("")) {
            MediaRoute::Native(name) => Some(name),
            _ => active_player_name(),
        };
        match name {
            Some("Spotify") => spotify_toggle_like(),
            Some("Music") => osascript(
                r#"tell application "Music"
  try
    set favorited of current track to not (favorited of current track)
  on error
    set loved of current track to not (loved of current track)
  end try
end tell"#,
            )
            .map(|_| true),
            _ => Err("No Spotify or Music player available".into()),
        }
    }

    fn media_like_key(&self) -> Result<(), String> {
        super::media_keys::spotify_like()?;
        flip_spotify_liked_cache();
        Ok(())
    }

    fn media_like_restore(&self) {
        restore_like_frontmost();
    }

    fn media_seek_back(&self) -> Result<(), String> {
        dispatch_player_script(|name| {
            format!(
                r#"tell application "{name}"
  set p to player position
  set player position to (p - 15)
end tell"#
            )
        })
    }

    fn media_seek_forward(&self) -> Result<(), String> {
        dispatch_player_script(|name| {
            format!(
                r#"tell application "{name}"
  set p to player position
  set player position to (p + 15)
end tell"#
            )
        })
    }

    fn media_repeat(&self) -> Result<(), String> {
        dispatch_player_script(|name| {
            if name == "Music" {
                r#"tell application "Music"
  if song repeat is off then
    set song repeat to all
  else if song repeat is all then
    set song repeat to one
  else
    set song repeat to off
  end if
end tell"#
                    .into()
            } else {
                r#"tell application "Spotify" to set repeating to not repeating"#.into()
            }
        })
    }

    fn media_airplay(&self) -> Result<(), String> {
        osascript(
            r#"tell application "System Events"
  try
    tell process "SystemUIServer"
      set volItem to (first menu bar item of menu bar 1 whose description contains "volume")
      click volItem
    end tell
  on error errMsg
    error "Could not open sound output menu: " & errMsg
  end try
end tell"#,
        )?;
        Ok(())
    }

    fn now_playing(
        &self,
        front: Option<&super::AppInfo>,
        running: &[RunningApp],
    ) -> Option<NowPlaying> {
        if let Some(app) = front {
            if app.bundle_id == "com.spotify.client" {
                return tagged_player(running, "Spotify", &app.bundle_id, &app.name);
            }
            if app.bundle_id == "com.apple.Music" {
                return tagged_player(running, "Music", &app.bundle_id, &app.name);
            }
        }
        playing_player(running)
    }

    fn refresh_liked(&self, np: &mut NowPlaying) {
        super::spotify_ax::apply(np);
    }

    fn browser_now_playing(&self, front: &super::AppInfo) -> Option<NowPlaying> {
        if !is_browser(&front.bundle_id) {
            return None;
        }
        let sys = super::media_remote::system_now_playing()?;
        if sys.title.is_empty() {
            return None;
        }
        if !sys.bundle_id.is_empty()
            && !super::is_browser_media_client(&sys.bundle_id, &front.bundle_id)
        {
            return None;
        }
        Some(NowPlaying {
            title: sys.title,
            artist: sys.artist,
            artwork_url: String::new(),
            playing: sys.playing,
            source_name: front.name.clone(),
            source_bundle_id: front.bundle_id.clone(),
            position_sec: sys.position_sec,
            duration_sec: sys.duration_sec,
            shuffle: false,
            liked: false,
            repeat_mode: RepeatMode::Off,
        })
    }

    fn tab_playback(&self, bundle_id: &str, window_index: u32, tab_index: u32) -> Option<bool> {
        tab_playback_state(bundle_id, window_index, tab_index)
    }

    fn tab_media_toggle(&self, bundle_id: &str, tab: &BrowserTab) -> Result<(), String> {
        tab_media_toggle_js(bundle_id, tab)
    }

    fn send_shortcut(&self, action: &str) -> Result<(), String> {
        super::media_keys::shortcut(action)
    }

    fn focus_app(&self, bundle_id: &str, window_index: Option<u32>) -> Result<(), String> {
        if bundle_id.is_empty() {
            return Err("no app to focus".into());
        }
        let id_literal = script_literal(bundle_id);
        osascript(&format!("tell application id {id_literal} to activate"))?;
        if let Some(index) = window_index.filter(|i| *i >= 1) {
            let _ = osascript(&format!(
                r#"tell application "System Events"
  tell (first process whose bundle identifier is {id_literal})
    set frontmost to true
    perform action "AXRaise" of window {index}
  end tell
end tell"#
            ));
        }
        Ok(())
    }

    fn list_apps(&self) -> Vec<RunningApp> {
        list_running_apps()
    }

    fn browser_tabs(&self, bundle_id: &str) -> Vec<BrowserTab> {
        list_browser_tabs(bundle_id)
    }

    fn activate_browser_tab(&self, bundle_id: &str, tab: &BrowserTab) -> Result<(), String> {
        focus_browser_tab(bundle_id, tab)
    }

    fn app_icon_png(&self, bundle_id: &str, cache_dir: &std::path::Path) -> Option<Vec<u8>> {
        extract_icon_png(bundle_id, cache_dir)
    }

    fn volume_state(&self) -> Option<super::VolumeState> {
        let raw = osascript(
            r#"set v to get volume settings
return (output volume of v as text) & "," & (output muted of v as text) & "," & (input volume of v as text)"#,
        )
        .ok()?;
        let mut parts = raw.split(',');
        let output: f64 = parts.next()?.trim().parse().ok()?;
        let muted = parse_bool(parts.next()?.trim());
        let input: f64 = parts.next()?.trim().parse().unwrap_or(0.0);
        Some(super::VolumeState {
            output: output.clamp(0.0, 100.0) as u8,
            muted,
            input: input.clamp(0.0, 100.0) as u8,
        })
    }

    fn quick_state(&self) -> Option<super::QuickState> {
        let raw = osascript(
            r#"set v to get volume settings
set t to ""
set b to ""
try
  tell application "System Events"
    set p to first application process whose frontmost is true
    set b to bundle identifier of p
    try
      set t to name of front window of p
    end try
  end tell
end try
return (output volume of v as text) & "|" & (output muted of v as text) & "|" & (input volume of v as text) & "|" & b & "|" & t"#,
        )
        .ok()?;
        let mut parts = raw.splitn(5, '|');
        let output: f64 = parts.next()?.trim().parse().ok()?;
        let muted = parse_bool(parts.next()?.trim());
        let input: f64 = parts.next()?.trim().parse().unwrap_or(0.0);
        let front_bundle = parts.next().unwrap_or("").trim().to_string();
        let window_title = parts.next().unwrap_or("").trim().to_string();
        Some(super::QuickState {
            volume: super::VolumeState {
                output: output.clamp(0.0, 100.0) as u8,
                muted,
                input: input.clamp(0.0, 100.0) as u8,
            },
            window_title,
            front_bundle,
        })
    }

    fn input_volume_get(&self) -> Result<u8, String> {
        let out = osascript("input volume of (get volume settings)")?;
        let v: f64 = out.parse().map_err(|_| "bad input volume".to_string())?;
        Ok(v.clamp(0.0, 100.0) as u8)
    }

    fn input_volume_set(&self, value: u8) -> Result<(), String> {
        osascript(&format!("set volume input volume {value}"))?;
        Ok(())
    }

    fn clipboard_get(&self) -> Option<String> {
        if clipboard_concealed() {
            return None;
        }
        let mut child = Command::new("pbpaste")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let deadline = Instant::now() + Duration::from_millis(400);
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(10)),
                Err(_) => return None,
            }
        }
        let out = child.wait_with_output().ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        if text.trim().is_empty() {
            None
        } else {
            Some(text)
        }
    }

    fn clipboard_set(&self, text: &str) -> Result<(), String> {
        let mut child = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(text.as_bytes())
                .map_err(|e| e.to_string())?;
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err("pbcopy failed".into())
        }
    }

    fn clipboard_probe(&self) -> Option<super::ClipboardProbe> {
        super::pasteboard::probe()
    }

    fn clipboard_image_get(&self, dest: &std::path::Path) -> Option<super::ClipboardImage> {
        super::pasteboard::read_image(dest)
    }

    fn clipboard_image_set(&self, src: &std::path::Path) -> Result<(), String> {
        super::pasteboard::write_image(src)
    }

    fn detect_call(
        &self,
        running: &[RunningApp],
        tabs: &[BrowserTab],
        tabs_bundle: &str,
    ) -> Option<crate::protocol::CallSession> {
        detect_call_session(running, tabs, tabs_bundle)
    }

    fn detect_approval(
        &self,
        running: &[RunningApp],
        tabs: &[BrowserTab],
        tabs_bundle: &str,
    ) -> Option<crate::protocol::ApprovalSession> {
        super::approval_ax::probe(running, tabs, tabs_bundle)
    }

    fn approval_press(&self, bundle_id: &str, allow: bool) -> bool {
        super::approval_ax::press(bundle_id, allow)
    }

    fn approval_reachable(&self, bundle_id: &str) -> Option<bool> {
        super::approval_ax::tree_reachable(bundle_id)
    }

    fn dock_badge(&self, app_name: &str) -> Option<u32> {
        dock_badge_count(app_name)
    }

    fn type_text(&self, text: &str) -> Result<(), String> {
        super::media_keys::type_text(text)
    }
}

fn list_running_apps() -> Vec<RunningApp> {
    let Ok(raw) = osascript(
        r#"tell application "System Events"
  set US to character id 31
  set RS to character id 30
  set GS to character id 29
  set out to ""
  set procs to application processes whose background only is false
  repeat with p in procs
    set n to name of p
    set b to ""
    try
      set b to bundle identifier of p
    end try
    if b is not "" then
      set wins to ""
      set i to 1
      try
        repeat with w in windows of p
          set t to ""
          try
            set t to name of w
          end try
          if t is not "" then
            if wins is not "" then set wins to wins & GS
            set wins to wins & (i as text) & US & t
          end if
          set i to i + 1
        end repeat
      end try
      if out is not "" then set out to out & RS
      set out to out & n & US & b & US & wins
    end if
  end repeat
  return out
end tell"#,
    ) else {
        return Vec::new();
    };
    parse_running_apps(&raw)
}

fn parse_running_apps(raw: &str) -> Vec<RunningApp> {
    const RS: char = '\u{1e}';
    const US: char = '\u{1f}';
    const GS: char = '\u{1d}';
    if raw.is_empty() {
        return Vec::new();
    }
    let mut items = Vec::new();
    for block in raw.split(RS) {
        let mut parts = block.splitn(3, US);
        let name = parts.next().unwrap_or("").trim();
        let bundle_id = parts.next().unwrap_or("").trim();
        let win_raw = parts.next().unwrap_or("");
        if name.is_empty() || bundle_id.is_empty() || is_companion(bundle_id) {
            continue;
        }
        let mut windows = Vec::new();
        if !win_raw.is_empty() {
            for w in win_raw.split(GS) {
                let mut wp = w.splitn(2, US);
                let index: u32 = wp.next().unwrap_or("").parse().unwrap_or(0);
                let title = wp.next().unwrap_or("").trim();
                if index == 0 || title.is_empty() {
                    continue;
                }
                windows.push(AppWindow {
                    title: title.to_string(),
                    index,
                });
            }
        }
        items.push(RunningApp {
            name: name.to_string(),
            bundle_id: bundle_id.to_string(),
            windows,
        });
    }
    items
}

fn clipboard_concealed() -> bool {
    let info = osascript(
        r#"try
  clipboard info as string
on error
  return ""
end try"#,
    )
    .unwrap_or_default();
    info.to_ascii_lowercase().contains("concealed")
}

fn dock_badge_count(app_name: &str) -> Option<u32> {
    let name = app_name.trim();
    if name.is_empty() {
        return None;
    }
    let quoted = name.replace('\\', "\\\\").replace('"', "\\\"");
    let raw = osascript(&format!(
        r#"tell application "System Events"
  tell process "Dock"
    try
      return value of attribute "AXStatusLabel" of UI element "{quoted}" of list 1
    on error
      return ""
    end try
  end tell
end tell"#
    ))
    .ok()?;
    parse_badge(&raw)
}

fn parse_badge(raw: &str) -> Option<u32> {
    let t = raw.trim();
    if t.is_empty() {
        return None;
    }
    if let Ok(n) = t.parse::<u32>() {
        return Some(n);
    }
    if let Some(num) = t.strip_suffix('+') {
        if let Ok(n) = num.trim().parse::<u32>() {
            return Some(n);
        }
    }
    Some(1)
}

fn detect_call_session(
    running: &[RunningApp],
    tabs: &[BrowserTab],
    tabs_bundle: &str,
) -> Option<crate::protocol::CallSession> {
    let natives: Vec<crate::call::NativeApp> = running
        .iter()
        .filter(|app| {
            crate::call::is_discord(&app.bundle_id)
                || crate::call::is_slack(&app.bundle_id)
                || crate::call::is_zoom(&app.bundle_id)
                || crate::call::is_teams(&app.bundle_id)
                || super::is_browser(&app.bundle_id)
        })
        .map(|app| crate::call::NativeApp {
            name: app.name.clone(),
            bundle_id: app.bundle_id.clone(),
            windows: app.windows.iter().map(|w| w.title.clone()).collect(),
        })
        .collect();
    let meet = natives.iter().find_map(|app| {
        if !super::is_browser(&app.bundle_id) {
            return None;
        }

        if app.bundle_id == tabs_bundle {
            return meet_from_tabs(&app.bundle_id, &app.name, tabs);
        }
        find_meet_tab(&app.bundle_id, &app.name)
    });
    crate::call::pick_session(&natives, meet.as_ref())
}

fn meet_from_tabs(
    bundle_id: &str,
    app_name: &str,
    tabs: &[BrowserTab],
) -> Option<crate::call::MeetHit> {
    let tab = tabs
        .iter()
        .find(|t| super::looks_like_meet_tab(&t.title, &t.url))?;
    Some(crate::call::MeetHit {
        bundle_id: bundle_id.to_string(),
        app_name: app_name.to_string(),
        title: tab.title.clone(),
        window_index: tab.window_index,
        tab_index: tab.tab_index,
    })
}

fn find_meet_tab(bundle_id: &str, app_name: &str) -> Option<crate::call::MeetHit> {
    let tabs = list_browser_tabs(bundle_id);
    let tab = tabs
        .iter()
        .find(|t| super::looks_like_meet_tab(&t.title, &t.url))?;
    Some(crate::call::MeetHit {
        bundle_id: bundle_id.to_string(),
        app_name: app_name.to_string(),
        title: tab.title.clone(),
        window_index: tab.window_index,
        tab_index: tab.tab_index,
    })
}

fn dispatch_player_script<F>(script_for: F) -> Result<(), String>
where
    F: Fn(&'static str) -> String,
{
    if let Some(name) = active_player_name() {
        osascript(&script_for(name)).map(|_| ())
    } else {
        Err("No Spotify or Music player available".into())
    }
}

fn active_player_name() -> Option<&'static str> {
    let front = MacOs.frontmost_app();
    if let Some(app) = front.as_ref() {
        if app.bundle_id == "com.apple.Music" {
            return Some("Music");
        }
        if app.bundle_id == "com.spotify.client" {
            return Some("Spotify");
        }
    }
    if app_running("Spotify") {
        return Some("Spotify");
    }
    if app_running("Music") {
        return Some("Music");
    }
    None
}

fn dispatch_media(verb: &str, target: Option<&str>) -> Result<(), String> {
    match super::media_route(target.unwrap_or("")) {
        MediaRoute::Native(name) => {
            osascript(&format!("tell application \"{name}\" to {verb}")).map(|_| ())
        }
        MediaRoute::Browser => system_media(verb),
        MediaRoute::System => {
            if let Some(name) = active_player_name() {
                return osascript(&format!("tell application \"{name}\" to {verb}")).map(|_| ());
            }
            system_media(verb)
        }
    }
}

fn system_media(verb: &str) -> Result<(), String> {
    match verb {
        "playpause" => super::media_keys::play_pause(),
        "next track" => super::media_keys::next_track(),
        "previous track" => super::media_keys::prev_track(),
        _ => Err(format!("unknown media verb {verb}")),
    }
}

fn playing_player(running: &[RunningApp]) -> Option<NowPlaying> {
    let spotify = tagged_player(running, "Spotify", "com.spotify.client", "Spotify");
    if spotify.as_ref().is_some_and(|np| np.playing) {
        return spotify;
    }
    let music = tagged_player(running, "Music", "com.apple.Music", "Music");
    if music.as_ref().is_some_and(|np| np.playing) {
        return music;
    }
    spotify.or(music)
}

fn tagged_player(
    running: &[RunningApp],
    app: &str,
    bundle_id: &str,
    source_name: &str,
) -> Option<NowPlaying> {
    if !running.iter().any(|a| a.bundle_id == bundle_id) {
        return None;
    }
    let mut np = now_playing_app(app)?;
    np.source_name = source_name.to_string();
    np.source_bundle_id = bundle_id.to_string();
    Some(np)
}

fn list_browser_tabs(bundle_id: &str) -> Vec<BrowserTab> {
    let Some(name) = browser_app_name(bundle_id) else {
        return Vec::new();
    };
    let script = if bundle_id == "com.apple.Safari" {
        safari_tabs_script()
    } else {
        chromium_tabs_script(name)
    };
    let raw = match osascript(&script) {
        Ok(raw) => raw,
        Err(err) => {
            if err == "timed out" {
                probe_consent(name);
            } else {
                tracing::warn!("{name} tabs failed: {err}");
            }
            return Vec::new();
        }
    };
    let mut tabs = parse_browser_tabs(&raw);
    super::mark_window_media_audible(&mut tabs);
    if let Some(product) = super::chrome::product_label(bundle_id) {
        let profiles = super::chrome::load_profiles(bundle_id);
        let ax = if profiles.len() <= 1 {
            Vec::new()
        } else {
            ax_window_titles(bundle_id)
        };
        super::chrome::attach_profiles(&mut tabs, &profiles, &ax, product);
        if let Some(front) = ax.first().filter(|s| !s.is_empty()) {
            super::mark_active_from_window_title(&mut tabs, front);
        } else {
            super::mark_frontmost_active(&mut tabs);
        }
    } else {
        super::mark_frontmost_active(&mut tabs);
    }
    tabs
}

fn ax_window_titles(bundle_id: &str) -> Vec<String> {
    let script = format!(
        r#"tell application "System Events"
  set RS to character id 30
  set out to ""
  set bid to "{bundle_id}"
  if not (exists (first process whose bundle identifier is bid)) then return ""
  try
    tell (first process whose bundle identifier is bid and frontmost is true)
      repeat with w in windows
        set n to ""
        try
          set n to name of w
        end try
        if out is not "" then set out to out & RS
        set out to out & n
      end repeat
    end tell
  end try
  set procList to every process whose bundle identifier is bid
  repeat with i from 1 to count of procList
    set p to item i of procList
    set skip to false
    try
      set skip to frontmost of p
    end try
    if skip is false then
      tell p
        repeat with w in windows
          set n to ""
          try
            set n to name of w
          end try
          if out is not "" then set out to out & RS
          set out to out & n
        end repeat
      end tell
    end if
  end repeat
  return out
end tell"#
    );
    let Ok(raw) = osascript(&script) else {
        return Vec::new();
    };
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split('\u{1e}').map(|s| s.to_string()).collect()
}

fn script_literal(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn focus_browser_tab(bundle_id: &str, tab: &BrowserTab) -> Result<(), String> {
    if !tab.title.is_empty() && focus_browser_tab_named(bundle_id, &tab.title, &tab.url).is_ok() {
        return Ok(());
    }
    focus_browser_tab_at(bundle_id, tab.window_index, tab.tab_index)
}

fn focus_browser_tab_named(bundle_id: &str, title: &str, url: &str) -> Result<(), String> {
    let name = browser_app_name(bundle_id).ok_or_else(|| "unsupported browser".to_string())?;
    let want_title = script_literal(title);
    let want_url = script_literal(url);
    let script = if bundle_id == "com.apple.Safari" {
        format!(
            r#"tell application "Safari"
  activate
  set wantTitle to {want_title}
  set wantUrl to {want_url}
  repeat with w in windows
    set ti to 1
    repeat with t in tabs of w
      set ttitle to ""
      try
        set ttitle to name of t
      end try
      set turl to ""
      try
        set turl to URL of t
      end try
      if ttitle is wantTitle and (wantUrl is "" or turl is wantUrl) then
        set current tab of w to t
        set index of w to 1
        return "ok"
      end if
      set ti to ti + 1
    end repeat
  end repeat
  error "missing"
end tell"#
        )
    } else {
        format!(
            r#"tell application "{name}"
  activate
  set wantTitle to {want_title}
  set wantUrl to {want_url}
  repeat with w in windows
    set ti to 1
    repeat with t in tabs of w
      set ttitle to ""
      try
        set ttitle to title of t
      end try
      set turl to ""
      try
        set turl to URL of t
      end try
      if ttitle is wantTitle and (wantUrl is "" or turl is wantUrl) then
        set index of w to 1
        set active tab index of window 1 to ti
        return "ok"
      end if
      set ti to ti + 1
    end repeat
  end repeat
  error "missing"
end tell"#
        )
    };
    osascript(&script).map(|_| ())
}

fn focus_browser_tab_at(bundle_id: &str, window_index: u32, tab_index: u32) -> Result<(), String> {
    if window_index < 1 || tab_index < 1 {
        return Err("bad tab".into());
    }
    let name = browser_app_name(bundle_id).ok_or_else(|| "unsupported browser".to_string())?;
    let script = if bundle_id == "com.apple.Safari" {
        format!(
            r#"tell application "Safari"
  activate
  set current tab of window {window_index} to tab {tab_index} of window {window_index}
end tell"#
        )
    } else {
        format!(
            r#"tell application "{name}"
  activate
  set index of window {window_index} to 1
  set active tab index of window 1 to {tab_index}
end tell"#
        )
    };
    osascript(&script).map(|_| ())
}

const TOGGLE_JS: &str = "(() => { const n = [...document.querySelectorAll('video,audio')]; if (!n.length) return 'none'; const v = n.find((x) => !x.paused && !x.ended) || n[0]; if (v.paused) { v.play(); return 'playing'; } v.pause(); return 'paused'; })()";

fn tab_media_toggle_js(bundle_id: &str, tab: &BrowserTab) -> Result<(), String> {
    if !tab.title.is_empty() {
        match tab_media_toggle_named(bundle_id, &tab.title, &tab.url) {
            Ok(()) => return Ok(()),
            Err(err) if err.contains("Access not allowed") || err.contains("-1723") => {
                return Err(err);
            }
            Err(_) => {}
        }
    }
    tab_media_toggle_at(bundle_id, tab.window_index, tab.tab_index)
}

fn tab_media_toggle_named(bundle_id: &str, title: &str, url: &str) -> Result<(), String> {
    let name = browser_app_name(bundle_id).ok_or_else(|| "unsupported browser".to_string())?;
    let want_title = script_literal(title);
    let want_url = script_literal(url);
    let script = if bundle_id == "com.apple.Safari" {
        format!(
            r#"tell application "Safari"
  set wantTitle to {want_title}
  set wantUrl to {want_url}
  repeat with w in windows
    repeat with t in tabs of w
      set ttitle to ""
      try
        set ttitle to name of t
      end try
      set turl to ""
      try
        set turl to URL of t
      end try
      if ttitle is wantTitle and (wantUrl is "" or turl is wantUrl) then
        return do JavaScript "{TOGGLE_JS}" in t
      end if
    end repeat
  end repeat
  error "missing"
end tell"#
        )
    } else {
        format!(
            r#"tell application "{name}"
  set wantTitle to {want_title}
  set wantUrl to {want_url}
  repeat with w in windows
    repeat with t in tabs of w
      set ttitle to ""
      try
        set ttitle to title of t
      end try
      set turl to ""
      try
        set turl to URL of t
      end try
      if ttitle is wantTitle and (wantUrl is "" or turl is wantUrl) then
        tell t
          return execute javascript "{TOGGLE_JS}"
        end tell
      end if
    end repeat
  end repeat
  error "missing"
end tell"#
        )
    };
    match osascript(&script) {
        Ok(out) if out.trim() == "none" => Err("No media on that tab".into()),
        Ok(_) => Ok(()),
        Err(err) if err.contains("Access not allowed") || err.contains("-1723") => Err(format!(
            "Turn on {name} > View > Developer > Allow JavaScript from Apple Events to control tab media"
        )),
        Err(err) => Err(err),
    }
}

fn tab_media_toggle_at(bundle_id: &str, window_index: u32, tab_index: u32) -> Result<(), String> {
    if window_index < 1 || tab_index < 1 {
        return Err("no media tab".into());
    }
    let name = browser_app_name(bundle_id).ok_or_else(|| "unsupported browser".to_string())?;
    let script = if bundle_id == "com.apple.Safari" {
        format!(
            r#"tell application "Safari" to do JavaScript "{TOGGLE_JS}" in tab {tab_index} of window {window_index}"#
        )
    } else {
        format!(
            r#"tell application "{name}"
  tell tab {tab_index} of window {window_index}
    execute javascript "{TOGGLE_JS}"
  end tell
end tell"#
        )
    };
    match osascript(&script) {
        Ok(out) if out.trim() == "none" => Err("No media on that tab".into()),
        Ok(_) => Ok(()),
        Err(err) if err.contains("Access not allowed") || err.contains("-1723") => Err(format!(
            "Turn on {name} > View > Developer > Allow JavaScript from Apple Events to control tab media"
        )),
        Err(err) => Err(err),
    }
}

fn tab_playback_state(bundle_id: &str, window_index: u32, tab_index: u32) -> Option<bool> {
    if window_index < 1 || tab_index < 1 {
        return None;
    }
    let name = browser_app_name(bundle_id)?;

    let js = "(() => { const nodes = [...document.querySelectorAll('video,audio')]; if (!nodes.length) return 'none'; return nodes.some((v) => !v.paused && !v.ended) ? 'playing' : 'paused'; })()";
    let script = if bundle_id == "com.apple.Safari" {
        format!(
            r#"tell application "Safari"
  try
    do JavaScript "{js}" in tab {tab_index} of window {window_index}
  on error
    return "none"
  end try
end tell"#
        )
    } else {
        format!(
            r#"tell application "{name}"
  try
    tell tab {tab_index} of window {window_index}
      execute javascript "{js}"
    end tell
  on error
    return "none"
  end try
end tell"#
        )
    };
    let raw = osascript(&script).ok()?;
    super::parse_tab_playback(&raw)
}

fn browser_app_name(bundle_id: &str) -> Option<&'static str> {
    match bundle_id {
        "com.apple.Safari" => Some("Safari"),
        "com.google.Chrome" | "com.google.Chrome.beta" | "com.google.Chrome.dev" => {
            Some("Google Chrome")
        }
        "com.google.Chrome.canary" => Some("Google Chrome Canary"),
        "com.brave.Browser" => Some("Brave Browser"),
        "company.thebrowser.Browser" => Some("Arc"),
        _ => None,
    }
}

fn chromium_tabs_script(name: &str) -> String {
    format!(
        r#"tell application "{name}"
  set RS to character id 30
  set US to character id 31
  set out to ""
  if (count of windows) is 0 then return ""
    set wi to 1
  repeat with w in windows
    set activeIdx to active tab index of w
    set wmode to "normal"
    try
      set wmode to mode of w as string
    end try
    set wname to ""
    try
      set wname to name of w
    end try
    set waud to "0"
    -- Chrome puts a speaker in the window name while a tab is audible.
    -- MediaRemote reports playbackRate 0 for YouTube even when audio is on.
    if wname contains (character id 128266) then set waud to "1"
    if wname contains (character id 128265) then set waud to "1"
    if wname contains (character id 128264) then set waud to "1"
    if wname contains (character id 128263) then set waud to "1"
    -- Two Apple Events per window instead of two per tab. Asking tab by tab
    -- put a heavy window past the osascript timeout, which reads back as a
    -- missing Automation grant.
    set titles to {{}}
    set urls to {{}}
    try
      set titles to title of tabs of w
      set urls to URL of tabs of w
    end try
    set n to count of titles
    if (count of urls) is not n then set urls to {{}}
    repeat with ti from 1 to n
      set ttitle to item ti of titles
      if ttitle is missing value then set ttitle to ""
      set turl to ""
      if (count of urls) is n then
        set turl to item ti of urls
        if turl is missing value then set turl to ""
      end if
      if ttitle is "" then set ttitle to "Tab " & ti
      set act to "0"
      if ti is activeIdx then set act to "1"
      if out is not "" then set out to out & RS
      set out to out & (wi as text) & US & (ti as text) & US & act & US & ttitle & US & turl & US & wmode & US & waud
    end repeat
    set wi to wi + 1
  end repeat
  return out
end tell"#
    )
}

fn safari_tabs_script() -> String {
    r#"tell application "Safari"
  set RS to character id 30
  set US to character id 31
  set out to ""
  if (count of windows) is 0 then return ""
  set wi to 1
  repeat with w in windows
    set cur to 0
    try
      set cur to index of current tab of w
    end try
    set ti to 1
    repeat with t in tabs of w
      set ttitle to ""
      try
        set ttitle to name of t
      end try
      set turl to ""
      try
        set turl to URL of t
      end try
      if ttitle is "" then set ttitle to "Tab " & ti
      set act to "0"
      if ti is cur then set act to "1"
      if out is not "" then set out to out & RS
      set out to out & (wi as text) & US & (ti as text) & US & act & US & ttitle & US & turl
      set ti to ti + 1
    end repeat
    set wi to wi + 1
  end repeat
  return out
end tell"#
        .into()
}

fn parse_browser_tabs(raw: &str) -> Vec<BrowserTab> {
    const RS: char = '\u{1e}';
    const US: char = '\u{1f}';
    if raw.is_empty() {
        return Vec::new();
    }
    let mut tabs = Vec::new();
    for block in raw.split(RS) {
        let mut parts = block.splitn(7, US);
        let window_index: u32 = parts.next().unwrap_or("").parse().unwrap_or(0);
        let tab_index: u32 = parts.next().unwrap_or("").parse().unwrap_or(0);
        let active = parts.next().unwrap_or("") == "1";
        let title = parts.next().unwrap_or("").trim();
        let url = parts.next().unwrap_or("").trim();
        let mode = parts.next().unwrap_or("").trim();
        let window_audible = parts.next().unwrap_or("").trim() == "1";
        if window_index == 0 || tab_index == 0 {
            continue;
        }
        tabs.push(BrowserTab {
            title: if title.is_empty() {
                format!("Tab {tab_index}")
            } else {
                title.to_string()
            },
            window_index,
            tab_index,
            active,
            audible: window_audible,
            url: url.to_string(),
            profile: if mode.eq_ignore_ascii_case("incognito") {
                "Incognito".into()
            } else {
                String::new()
            },
            media: super::looks_like_media_tab(title, url),
        });
    }
    tabs
}

pub(super) fn osascript(source: &str) -> Result<String, String> {
    osascript_timed(source, OSASCRIPT_TIMEOUT)
}

pub(super) fn osascript_consent(source: &str) -> Result<String, String> {
    osascript_timed(source, CONSENT_TIMEOUT)
}

pub(super) fn probe_consent(app_name: &str) {
    {
        let mut probed = CONSENT_PROBED.lock().unwrap_or_else(|e| e.into_inner());
        probed.retain(|(_, at)| at.elapsed() < CONSENT_RETRY);
        if probed.iter().any(|(name, _)| name == app_name) {
            return;
        }
        probed.push((app_name.to_string(), Instant::now()));
    }
    let name = app_name.to_string();
    let script = format!(r#"tell application "{app_name}" to get name"#);
    std::thread::spawn(move || {
        let granted = osascript_timed(&script, CONSENT_TIMEOUT).is_ok();
        if granted {
            return;
        }

        let mut probed = CONSENT_PROBED.lock().unwrap_or_else(|e| e.into_inner());
        probed.retain(|(n, _)| n != &name);
    });
}

fn osascript_timed(source: &str, timeout: Duration) -> Result<String, String> {
    let mut child = Command::new("osascript")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(source.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("timed out".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(15)),
            Err(e) => return Err(e.to_string()),
        }
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn app_running(name: &str) -> bool {
    osascript(&format!(
        r#"tell application "System Events" to (name of processes) contains "{name}""#
    ))
    .ok()
    .is_some_and(|s| s == "true")
}

fn now_playing_app(app: &str) -> Option<NowPlaying> {
    let script = if app == "Music" {
        r#"tell application "Music"
  if player state is stopped then return ""
  set n to name of current track
  set a to artist of current track
  set s to player state as string
  set u to ""
  try
    set u to artwork url of current track
  end try
  set pos to player position
  set dur to duration of current track
  set sh to shuffle enabled
  set lv to false
  try
    set lv to favorited of current track
  on error
    try
      set lv to loved of current track
    end try
  end try
  set rep to song repeat as string
  return n & tab & a & tab & s & tab & u & tab & pos & tab & dur & tab & sh & tab & lv & tab & rep
end tell"#
    } else {
        r#"tell application "Spotify"
  if player state is stopped then return ""
  set n to name of current track
  set a to artist of current track
  set s to player state as string
  set u to ""
  try
    set u to artwork url of current track
  end try
  set pos to player position
  set dur to duration of current track
  set sh to shuffling
  set rep to "off"
  if repeating then set rep to "all"
  set starVal to ""
  try
    if starred of current track then set starVal to "true"
  end try
  set tid to id of current track
  return n & tab & a & tab & s & tab & u & tab & pos & tab & dur & tab & sh & tab & starVal & tab & rep & tab & tid
end tell"#
    };
    let out = osascript(script).ok()?;
    if out.is_empty() {
        return None;
    }
    let mut parts = out.split('\t');
    let title = parts.next()?.to_string();
    let artist = parts.next().unwrap_or("").to_string();
    let playing = parts.next().unwrap_or("").eq_ignore_ascii_case("playing");
    let artwork_url = parts.next().unwrap_or("").to_string();
    let mut position_sec = parts
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let mut duration_sec = parts
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let shuffle = parse_bool(parts.next().unwrap_or(""));
    let liked_raw = parts.next().unwrap_or("");
    let repeat_mode = parse_repeat_mode(parts.next().unwrap_or(""));
    let track_id = parts.next().unwrap_or("").to_string();
    let liked = if app == "Spotify" {
        spotify_liked_status(track_id, liked_raw)
    } else {
        parse_bool(liked_raw)
    };
    if app == "Spotify" {
        duration_sec /= 1000.0;
    }
    position_sec = position_sec.clamp(0.0, duration_sec.max(1.0));
    Some(NowPlaying {
        title,
        artist,
        artwork_url,
        playing,
        source_name: app.to_string(),
        source_bundle_id: String::new(),
        position_sec,
        duration_sec,
        shuffle,
        liked,
        repeat_mode,
    })
}

fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "yes" | "1"
    )
}

fn liked_cache() -> std::sync::MutexGuard<'static, Option<LikedCache>> {
    SPOTIFY_LIKED.lock().unwrap_or_else(|e| e.into_inner())
}

fn spotify_liked_status(track_id: String, _starred: &str) -> bool {
    if !track_id.is_empty() {
        let cache = liked_cache();
        if let Some(entry) = cache.as_ref() {
            if entry.track_id == track_id && entry.checked_at.elapsed() < SPOTIFY_LIKED_TTL {
                return entry.liked;
            }
        }
    }
    let fallback = liked_cache()
        .as_ref()
        .filter(|entry| entry.track_id == track_id)
        .map(|entry| entry.liked);
    if let Some(liked) = read_spotify_liked_from_ui() {
        remember_spotify_liked(&track_id, liked);
        return liked;
    }
    fallback.unwrap_or(false)
}

fn remember_spotify_liked(track_id: &str, liked: bool) {
    super::spotify_ax::note(liked);
    if track_id.is_empty() {
        return;
    }
    *liked_cache() = Some(LikedCache {
        track_id: track_id.to_string(),
        liked,
        checked_at: Instant::now(),
    });
}

fn flip_spotify_liked_cache() {
    let Ok(id) = osascript(r#"tell application "Spotify" to id of current track"#) else {
        return;
    };
    let cached = liked_cache()
        .as_ref()
        .filter(|entry| entry.track_id == id)
        .map(|entry| entry.liked);
    if let Some(was) = cached {
        remember_spotify_liked(&id, !was);
        return;
    }
    if let Some(liked) = read_spotify_liked_from_ui() {
        remember_spotify_liked(&id, liked);
    }
}

fn apply_ax_like(was_liked: bool) {
    super::spotify_ax::note(!was_liked);
    if let Ok(id) = osascript(r#"tell application "Spotify" to id of current track"#) {
        remember_spotify_liked(&id, !was_liked);
    }
}

fn spotify_toggle_like() -> Result<bool, String> {
    if let Some(was_liked) = spotify_like_ax(true) {
        apply_ax_like(was_liked);
        return Ok(true);
    }
    let previous = focus_spotify_for_like()?;
    stash_like_restore(previous);
    if let Some(was_liked) = spotify_like_ax(true) {
        apply_ax_like(was_liked);
        restore_like_frontmost();
        return Ok(true);
    }
    std::thread::sleep(Duration::from_millis(80));
    Ok(false)
}

fn focus_spotify_for_like() -> Result<String, String> {
    let out = osascript_timed(
        r#"tell application "System Events"
  set prev to ""
  try
    set prev to bundle identifier of first application process whose frontmost is true
  end try
end tell
tell application "Spotify" to activate
repeat 20 times
  tell application "System Events"
    if (exists process "Spotify") and (frontmost of process "Spotify") then return prev
  end tell
  delay 0.05
end repeat
error "Spotify did not come to the front""#,
        Duration::from_millis(2500),
    )?;
    Ok(out)
}

fn stash_like_restore(bundle_id: String) {
    let keep = (!bundle_id.is_empty() && bundle_id != "com.spotify.client").then_some(bundle_id);
    *LIKE_RESTORE.lock().unwrap_or_else(|e| e.into_inner()) = keep;
}

fn restore_like_frontmost() {
    let bundle = LIKE_RESTORE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    if let Some(bundle_id) = bundle {
        let _ = MacOs.focus_app(&bundle_id, None);
    }
}

fn spotify_like_ax(click: bool) -> Option<bool> {
    if let Some(liked) = super::spotify_ax::liked(click) {
        return Some(liked);
    }
    if !click {
        return None;
    }
    let script = spotify_like_script(true);
    let out = osascript_timed(&script, Duration::from_secs(2)).ok()?;
    match out.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn read_spotify_liked_from_ui() -> Option<bool> {
    super::spotify_ax::liked(false)
}

fn spotify_like_script(click: bool) -> String {
    let click_flag = if click { "true" } else { "false" };
    format!(
        r#"property doClick : {click_flag}
tell application "System Events"
  if not (exists process "Spotify") then return ""
  tell process "Spotify"
    if (count of windows) is 0 then return ""
    return my scanLike(window 1, 0)
  end tell
end tell

on scanLike(elem, depth)
  if depth > 8 then return ""
  try
    repeat with b in buttons of elem
      set hit to my likeHint(b)
      if hit is not "" then
        if doClick then
          try
            perform action "AXPress" of b
          on error
            try
              click b
            on error
              return ""
            end try
          end try
        end if
        return hit
      end if
    end repeat
  end try
  try
    repeat with b in checkboxes of elem
      set hit to my likeHint(b)
      if hit is not "" then
        if doClick then
          try
            perform action "AXPress" of b
          on error
            try
              click b
            on error
              return ""
            end try
          end try
        end if
        return hit
      end if
    end repeat
  end try
  try
    set hit to my scanKids(groups of elem, depth)
    if hit is not "" then return hit
  end try
  try
    set hit to my scanKids(scroll areas of elem, depth)
    if hit is not "" then return hit
  end try
  try
    set hit to my scanKids(splitter groups of elem, depth)
    if hit is not "" then return hit
  end try
  try
    set hit to my scanKids(toolbars of elem, depth)
    if hit is not "" then return hit
  end try
  try
    set hit to my scanKids(lists of elem, depth)
    if hit is not "" then return hit
  end try
  return ""
end scanLike

on scanKids(kids, depth)
  try
    set i to count of kids
    repeat while i is greater than 0
      set hit to my scanLike(item i of kids, depth + 1)
      if hit is not "" then return hit
      set i to i - 1
    end repeat
  end try
  return ""
end scanKids

on likeHint(el)
  set d to ""
  set n to ""
  set h to ""
  try
    set d to description of el as text
  end try
  try
    set n to name of el as text
  end try
  try
    set h to help of el as text
  end try
  set blob to d & " " & n & " " & h
  if blob contains "Remove from Your Library" then return "true"
  if blob contains "Remove from Liked Songs" then return "true"
  if blob contains "Unlike this" then return "true"
  if blob contains "Added to Liked Songs" then return "true"
  if blob contains "Save to Your Library" then return "false"
  if blob contains "Add to Your Library" then return "false"
  if blob contains "Add to Liked Songs" then return "false"
  if blob contains "Save to Liked Songs" then return "false"
  if blob contains "Like this song" then return "false"
  return ""
end likeHint
"#
    )
}

fn parse_repeat_mode(value: &str) -> RepeatMode {
    match value.to_ascii_lowercase().as_str() {
        "all" => RepeatMode::All,
        "one" => RepeatMode::One,
        _ => RepeatMode::Off,
    }
}

fn extract_icon_png(bundle_id: &str, cache_dir: &std::path::Path) -> Option<Vec<u8>> {
    let _ = std::fs::create_dir_all(cache_dir);
    let safe: String = bundle_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let out = cache_dir.join(format!("{safe}.png"));
    if out.exists() {
        return std::fs::read(&out).ok();
    }
    let app = app_bundle_path(bundle_id)?;
    let icns = find_icns(&app)?;
    let status = Command::new("sips")
        .args([
            "-s",
            "format",
            "png",
            "-Z",
            "256",
            icns.to_str()?,
            "--out",
            out.to_str()?,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }
    std::fs::read(&out).ok()
}

fn app_bundle_path(bundle_id: &str) -> Option<std::path::PathBuf> {
    let id_literal = script_literal(bundle_id);
    let from_id = osascript(&format!(
        "POSIX path of (path to application id {id_literal})"
    ))
    .ok();
    let raw = if let Some(p) = from_id.filter(|s| !s.is_empty()) {
        p
    } else {
        osascript(&format!(
            r#"tell application "System Events"
  POSIX path of application file of first process whose bundle identifier is {id_literal}
end tell"#
        ))
        .ok()?
    };
    let path = std::path::PathBuf::from(raw.trim().trim_end_matches('/'));
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

fn find_icns(app: &std::path::Path) -> Option<std::path::PathBuf> {
    let res = app.join("Contents/Resources");
    for name in [
        "AppIcon.icns",
        "app.icns",
        "Icon.icns",
        "electron.icns",
        "Cursor.icns",
    ] {
        let p = res.join(name);
        if p.exists() {
            return Some(p);
        }
    }
    let dir = std::fs::read_dir(&res).ok()?;
    dir.filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|s| s.to_str()) == Some("icns"))
}

#[cfg(test)]
mod tests {
    use super::{parse_badge, parse_bool, parse_browser_tabs};
    use crate::protocol::BrowserTab;

    #[test]
    fn parse_bool_accepts_common_truthy() {
        assert!(parse_bool("true"));
        assert!(parse_bool("YES"));
        assert!(parse_bool("1"));
        assert!(!parse_bool("false"));
        assert!(!parse_bool(""));
    }

    #[test]
    fn parse_browser_tabs_splits_records() {
        let raw = "1\u{1f}2\u{1f}1\u{1f}Inbox\u{1e}1\u{1f}3\u{1f}0\u{1f}YouTube";
        let tabs = parse_browser_tabs(raw);
        assert_eq!(
            tabs,
            vec![
                BrowserTab {
                    title: "Inbox".into(),
                    window_index: 1,
                    tab_index: 2,
                    active: true,
                    audible: false,
                    url: String::new(),
                    profile: String::new(),
                    media: false,
                },
                BrowserTab {
                    title: "YouTube".into(),
                    window_index: 1,
                    tab_index: 3,
                    active: false,
                    audible: false,
                    url: String::new(),
                    profile: String::new(),
                    media: false,
                },
            ]
        );
    }

    #[test]
    fn parse_browser_tabs_marks_incognito() {
        let raw = "2\u{1f}1\u{1f}1\u{1f}Secret\u{1f}https://example.com\u{1f}incognito";
        let tabs = parse_browser_tabs(raw);
        assert_eq!(tabs[0].profile, "Incognito");
        assert_eq!(tabs[0].url, "https://example.com");
    }

    #[test]
    fn parse_browser_tabs_reads_window_audio() {
        let raw = "2\u{1f}1\u{1f}1\u{1f}YouTube\u{1f}https://www.youtube.com/watch?v=a\u{1f}normal\u{1f}1";
        let tabs = parse_browser_tabs(raw);
        assert!(tabs[0].audible);
        assert_eq!(tabs[0].window_index, 2);
    }

    #[test]
    fn parse_badge_reads_numbers() {
        assert_eq!(parse_badge("5"), Some(5));
        assert_eq!(parse_badge("9+"), Some(9));
        assert_eq!(parse_badge(""), None);
        assert_eq!(parse_badge("•"), Some(1));
    }
}
