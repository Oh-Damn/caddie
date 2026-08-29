use crate::protocol::{LayoutPayload, Widget};
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
struct ProfileFile {
    id: String,
    #[allow(dead_code)]
    name: String,
    #[serde(rename = "match")]
    matcher: Matcher,
    pad: Vec<Vec<PadItem>>,
}

#[derive(Debug, Deserialize)]
struct Matcher {
    #[serde(rename = "bundleIds")]
    bundle_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PadItem {
    id: String,
    title: String,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    chord: Option<Vec<String>>,
}

struct Profile {
    id: String,
    bundle_ids: Vec<String>,
    pad: Vec<Vec<PadItem>>,
}

fn catalog() -> &'static [Profile] {
    static CATALOG: OnceLock<Vec<Profile>> = OnceLock::new();
    CATALOG.get_or_init(load_profiles)
}

fn load_profiles() -> Vec<Profile> {
    const FILES: &[&str] = &[
        include_str!("../profiles/generic.json"),
        include_str!("../profiles/slack.json"),
        include_str!("../profiles/discord.json"),
        include_str!("../profiles/vscode.json"),
        include_str!("../profiles/cursor.json"),
        include_str!("../profiles/claude.json"),
        include_str!("../profiles/zoom.json"),
        include_str!("../profiles/teams.json"),
    ];
    FILES
        .iter()
        .map(|raw| {
            let file: ProfileFile = serde_json::from_str(raw).expect("profile json");
            Profile {
                id: file.id,
                bundle_ids: file.matcher.bundle_ids,
                pad: file.pad,
            }
        })
        .collect()
}

pub fn id_for(bundle_id: &str) -> Option<&'static str> {
    if bundle_id.is_empty() {
        return None;
    }
    catalog()
        .iter()
        .find(|p| p.bundle_ids.iter().any(|b| b == bundle_id))
        .map(|p| p.id.as_str())
}

pub fn layout(plugin_id: &str, title: &str) -> LayoutPayload {
    let profile = catalog()
        .iter()
        .find(|p| p.id == plugin_id)
        .or_else(|| catalog().iter().find(|p| p.id == "generic"));
    let Some(profile) = profile else {
        return LayoutPayload {
            screen: "generic".into(),
            title: title.to_string(),
            widgets: vec![],
        };
    };
    LayoutPayload {
        screen: profile.id.clone(),
        title: if title.is_empty() {
            "Desktop".into()
        } else {
            title.to_string()
        },
        widgets: vec![Widget::Stack {
            id: "root".into(),
            children: profile
                .pad
                .iter()
                .enumerate()
                .map(|(i, row)| Widget::Row {
                    id: format!("row{i}"),
                    children: row.iter().map(item_button).collect(),
                })
                .collect(),
        }],
    }
}

fn item_button(item: &PadItem) -> Widget {
    let action = if let Some(action) = item.action.as_ref().filter(|s| !s.is_empty()) {
        action.clone()
    } else if let Some(chord) = &item.chord {
        format!("shortcut.send:{}", chord.join("+"))
    } else {
        format!("shortcut.{}", item.id)
    };
    button(&item.id, &item.title, &action)
}

fn button(id: &str, title: &str, action: &str) -> Widget {
    Widget::Button {
        id: id.into(),
        title: title.into(),
        action: action.into(),
        target: None,
        icon: None,
        kind: None,
        value_key: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{id_for, layout, load_profiles};

    #[test]
    fn loads_builtin_profiles() {
        let profiles = load_profiles();
        let ids: Vec<_> = profiles.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"generic"));
        assert!(ids.contains(&"slack"));
        assert!(ids.contains(&"discord"));
        assert!(ids.contains(&"vscode"));
        assert!(ids.contains(&"cursor"));
        assert!(ids.contains(&"claude"));
    }

    #[test]
    fn maps_known_bundles() {
        assert_eq!(id_for("com.tinyspeck.slackmacgap"), Some("slack"));
        assert_eq!(id_for("com.hnc.Discord"), Some("discord"));
        assert_eq!(id_for("com.microsoft.VSCode"), Some("vscode"));
        assert_eq!(id_for("com.todesktop.230313mzl4w4u92"), Some("cursor"));
        assert_eq!(id_for("com.cursor.Cursor"), Some("cursor"));
        assert_eq!(id_for("com.anthropic.claudefordesktop"), Some("claude"));
        assert_eq!(id_for("us.zoom.xos"), Some("zoom"));
        assert_eq!(id_for("unknown.app"), None);
    }

    #[test]
    fn discord_pad_uses_send_chords() {
        let layout = layout("discord", "Discord");
        assert_eq!(layout.screen, "discord");
        let json = serde_json::to_string(&layout.widgets).unwrap();
        assert!(json.contains("shortcut.send:meta+shift+m"));
        assert!(json.contains("Deafen"));
    }

    #[test]
    fn cursor_pad_uses_agent_chords() {
        let layout = layout("cursor", "Cursor");
        assert_eq!(layout.screen, "cursor");
        let json = serde_json::to_string(&layout.widgets).unwrap();
        assert!(json.contains("shortcut.send:meta+i"));
        assert!(json.contains("shortcut.send:meta+shift+backspace"));
        assert!(json.contains("Agent"));
    }

    #[test]
    fn claude_pad_uses_jump() {
        let layout = layout("claude", "Claude");
        assert_eq!(layout.screen, "claude");
        let json = serde_json::to_string(&layout.widgets).unwrap();
        assert!(json.contains("shortcut.send:meta+k"));
        assert!(json.contains("Jump"));
    }
}
