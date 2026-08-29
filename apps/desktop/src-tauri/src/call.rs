use crate::protocol::CallSession;

#[derive(Debug, Clone)]
pub struct NativeApp {
    pub name: String,
    pub bundle_id: String,
    pub windows: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MeetHit {
    pub bundle_id: String,
    pub app_name: String,
    pub title: String,
    pub window_index: u32,
    pub tab_index: u32,
}

pub fn looks_like_meet_tab(title: &str, url: &str) -> bool {
    let url = url.to_ascii_lowercase();
    if url.contains("meet.google.com") {
        return true;
    }
    let title = title.to_ascii_lowercase();
    title.contains("google meet") || title.contains("meet.google.com")
}

pub fn pick_session(natives: &[NativeApp], meet: Option<&MeetHit>) -> Option<CallSession> {
    if let Some(meet) = meet {
        return Some(session(
            "meet",
            &meet.app_name,
            &meet.bundle_id,
            &meet.title,
            true,
            false,
            false,
            Some(meet.window_index),
            Some(meet.tab_index),
        ));
    }
    if let Some(app) = natives.iter().find(|a| is_slack(&a.bundle_id)) {
        if app.windows.iter().any(|t| huddle_title(t)) {
            let title = app
                .windows
                .iter()
                .find(|t| huddle_title(t))
                .cloned()
                .unwrap_or_else(|| app.name.clone());
            return Some(session(
                "slack",
                &app.name,
                &app.bundle_id,
                &title,
                false,
                false,
                true,
                None,
                None,
            ));
        }
    }
    if let Some(app) = natives.iter().find(|a| is_zoom(&a.bundle_id)) {
        if zoom_meeting(&app.windows) {
            let title = app
                .windows
                .iter()
                .find(|t| {
                    let l = t.to_ascii_lowercase();
                    l.contains("meeting") || l.contains("webinar") || l.contains("zoom.us")
                })
                .cloned()
                .unwrap_or_else(|| app.name.clone());
            return Some(session(
                "zoom",
                &app.name,
                &app.bundle_id,
                &title,
                true,
                false,
                false,
                None,
                None,
            ));
        }
    }
    if let Some(app) = natives.iter().find(|a| is_teams(&a.bundle_id)) {
        if teams_meeting(&app.windows) {
            return Some(session(
                "teams",
                &app.name,
                &app.bundle_id,
                &app.name,
                true,
                false,
                false,
                None,
                None,
            ));
        }
    }
    if let Some(app) = natives.iter().find(|a| is_discord(&a.bundle_id)) {
        return Some(session(
            "discord",
            &app.name,
            &app.bundle_id,
            "Discord",
            false,
            true,
            false,
            None,
            None,
        ));
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn session(
    app: &str,
    app_name: &str,
    bundle_id: &str,
    title: &str,
    has_camera: bool,
    has_deafen: bool,
    has_leave: bool,
    tab_window_index: Option<u32>,
    tab_index: Option<u32>,
) -> CallSession {
    CallSession {
        active: true,
        app: app.into(),
        app_name: app_name.into(),
        bundle_id: bundle_id.into(),
        title: title.into(),
        muted: false,
        has_camera,
        has_deafen,
        has_leave,
        tab_window_index,
        tab_index,
    }
}

pub fn is_discord(bundle_id: &str) -> bool {
    matches!(
        bundle_id,
        "com.hnc.Discord" | "com.hnc.DiscordCanary" | "com.hnc.DiscordPTB"
    )
}

pub fn is_slack(bundle_id: &str) -> bool {
    bundle_id == "com.tinyspeck.slackmacgap"
}

pub fn is_zoom(bundle_id: &str) -> bool {
    bundle_id == "us.zoom.xos"
}

pub fn is_teams(bundle_id: &str) -> bool {
    matches!(bundle_id, "com.microsoft.teams2" | "com.microsoft.teams")
}

fn huddle_title(title: &str) -> bool {
    title.to_ascii_lowercase().contains("huddle")
}

pub fn zoom_meeting(windows: &[String]) -> bool {
    windows.iter().any(|t| {
        let l = t.to_ascii_lowercase();
        if l == "zoom" || l == "zoom workplace" {
            return false;
        }
        l.contains("meeting") || l.contains("webinar") || l.contains("zoom.us")
    })
}

fn teams_meeting(windows: &[String]) -> bool {
    windows.iter().any(|t| {
        let l = t.to_ascii_lowercase();
        l.contains("meeting") || l.contains("| microsoft teams")
    })
}

pub fn camera_chord(app: &str) -> Option<&'static str> {
    match app {
        "meet" => Some("shortcut.send:meta+e"),
        "zoom" => Some("shortcut.send:meta+shift+v"),
        "teams" => Some("shortcut.send:meta+shift+o"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{looks_like_meet_tab, pick_session, zoom_meeting, MeetHit, NativeApp};

    fn app(name: &str, bundle: &str, windows: &[&str]) -> NativeApp {
        NativeApp {
            name: name.into(),
            bundle_id: bundle.into(),
            windows: windows.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn meet_wins_over_discord() {
        let natives = [app("Discord", "com.hnc.Discord", &["Discord"])];
        let meet = MeetHit {
            bundle_id: "com.google.Chrome".into(),
            app_name: "Google Chrome".into(),
            title: "Standup - Google Meet".into(),
            window_index: 1,
            tab_index: 3,
        };
        let session = pick_session(&natives, Some(&meet)).unwrap();
        assert_eq!(session.app, "meet");
        assert!(session.has_camera);
        assert_eq!(session.tab_index, Some(3));
    }

    #[test]
    fn slack_huddle_detected() {
        let natives = [app(
            "Slack",
            "com.tinyspeck.slackmacgap",
            &["#general", "Huddle: design"],
        )];
        let session = pick_session(&natives, None).unwrap();
        assert_eq!(session.app, "slack");
        assert!(session.has_leave);
    }

    #[test]
    fn zoom_idle_is_ignored() {
        assert!(!zoom_meeting(&["Zoom".into(), "Zoom Workplace".into()]));
        assert!(zoom_meeting(&["Zoom Meeting".into()]));
        let natives = [app("Zoom", "us.zoom.xos", &["Zoom"])];
        let discord = app("Discord", "com.hnc.Discord", &["Discord"]);
        assert_eq!(
            pick_session(&[natives[0].clone(), discord.clone()], None)
                .unwrap()
                .app,
            "discord"
        );
        let in_call = [app("Zoom", "us.zoom.xos", &["Zoom Meeting"]), discord];
        assert_eq!(pick_session(&in_call, None).unwrap().app, "zoom");
    }

    #[test]
    fn meet_url_and_title() {
        assert!(looks_like_meet_tab(
            "x",
            "https://meet.google.com/abc-defg-hij"
        ));
        assert!(looks_like_meet_tab("Weekly - Google Meet", ""));
        assert!(!looks_like_meet_tab("Inbox", "https://mail.google.com"));
    }
}
