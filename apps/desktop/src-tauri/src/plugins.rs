use crate::profiles;
use crate::protocol::LayoutPayload;

pub fn plugin_for(bundle_id: &str) -> &'static str {
    match bundle_id {
        "com.spotify.client" | "com.apple.Music" => "media",
        "com.google.Chrome"
        | "com.google.Chrome.canary"
        | "com.google.Chrome.beta"
        | "com.google.Chrome.dev"
        | "com.brave.Browser"
        | "com.apple.Safari"
        | "company.thebrowser.Browser" => "browser",
        _ => profiles::id_for(bundle_id).unwrap_or("generic"),
    }
}

pub fn layout_for(plugin_id: &str, title: &str) -> LayoutPayload {
    match plugin_id {
        "media" => LayoutPayload {
            screen: "media".into(),
            title: title.to_string(),
            widgets: vec![],
        },
        "browser" => LayoutPayload {
            screen: "browser".into(),
            title: title.to_string(),
            widgets: vec![],
        },
        _ => profiles::layout(plugin_id, title),
    }
}

#[cfg(test)]
mod tests {
    use super::plugin_for;

    #[test]
    fn selects_plugins() {
        assert_eq!(plugin_for("com.spotify.client"), "media");
        assert_eq!(plugin_for("com.google.Chrome"), "browser");
        assert_eq!(plugin_for("com.hnc.Discord"), "discord");
        assert_eq!(plugin_for("com.tinyspeck.slackmacgap"), "slack");
        assert_eq!(plugin_for("com.microsoft.VSCode"), "vscode");
        assert_eq!(plugin_for("com.todesktop.230313mzl4w4u92"), "cursor");
        assert_eq!(plugin_for("com.anthropic.claudefordesktop"), "claude");
        assert_eq!(plugin_for("com.unknown.app"), "generic");
    }
}
