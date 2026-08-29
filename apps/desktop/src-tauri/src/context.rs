use crate::call::{is_discord, is_slack, is_teams};
use crate::protocol::{Conversation, ConversationKind};
use std::collections::HashMap;

const RECENTS_CAP: usize = 12;

#[derive(Default)]
pub struct Recents {
    by_bundle: HashMap<String, Vec<Conversation>>,
}

impl Recents {
    pub fn from_stored(map: &HashMap<String, Vec<Conversation>>) -> Self {
        Self {
            by_bundle: map.clone(),
        }
    }

    pub fn snapshot(&self) -> HashMap<String, Vec<Conversation>> {
        self.by_bundle.clone()
    }

    pub fn ingest(&mut self, bundle_id: &str, conv: Conversation) {
        if bundle_id.is_empty() || conv.kind == ConversationKind::Other {
            return;
        }
        if is_slack(bundle_id) && is_slack_view(&conv.name) {
            return;
        }
        let list = self.by_bundle.entry(bundle_id.to_string()).or_default();
        if let Some(idx) = list
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(&conv.name) && c.kind == conv.kind)
        {
            list.remove(idx);
        }
        let mut stored = conv;
        stored.window_index = None;
        list.insert(0, stored);
        list.truncate(RECENTS_CAP);
    }

    pub fn list(&self, bundle_id: &str) -> Vec<Conversation> {
        let slack = is_slack(bundle_id);
        self.by_bundle
            .get(bundle_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|c| c.kind != ConversationKind::Other)
            .filter(|c| !slack || !is_slack_view(&c.name))
            .collect()
    }
}

pub fn is_chat_app(bundle_id: &str) -> bool {
    is_slack(bundle_id) || is_discord(bundle_id) || is_teams(bundle_id)
}

pub fn parse_title(bundle_id: &str, title: &str) -> Option<Conversation> {
    if is_slack(bundle_id) {
        parse_slack(title)
    } else if is_discord(bundle_id) {
        parse_discord(title)
    } else if is_teams(bundle_id) {
        parse_teams(title)
    } else {
        None
    }
}

pub fn parse_windows(
    bundle_id: &str,
    windows: &[(u32, String)],
    current: Option<&Conversation>,
) -> Vec<Conversation> {
    let mut out = Vec::new();
    for (index, title) in windows {
        let Some(mut conv) = parse_title(bundle_id, title) else {
            continue;
        };
        if current.is_some_and(|c| c.name == conv.name && c.kind == conv.kind) {
            continue;
        }
        conv.window_index = Some(*index);
        if out
            .iter()
            .any(|c: &Conversation| c.name == conv.name && c.kind == conv.kind)
        {
            continue;
        }
        out.push(conv);
    }
    out
}

fn parse_slack(title: &str) -> Option<Conversation> {
    let cleaned = strip_app_suffix(&normalize_dashes(title), "slack")?;
    if cleaned.is_empty() || looks_like_placeholder(&cleaned) {
        return None;
    }
    let cleaned = strip_item_count(&cleaned);
    let cleaned = strip_leading_badge(&cleaned);
    let lower = cleaned.to_ascii_lowercase();
    if lower.starts_with("huddle:") || lower.starts_with("huddle ") {
        let rest = cleaned
            .split_once(':')
            .map(|(_, r)| r)
            .unwrap_or(&cleaned)
            .trim();
        let (name, workspace) = split_workspace(rest);
        let (name, _) = split_kind_tag(&name);
        return Some(conv(
            if name.is_empty() { "Huddle" } else { &name },
            ConversationKind::Huddle,
            &workspace,
        ));
    }
    let (name, workspace) = split_workspace(&cleaned);
    let (name, tagged) = split_kind_tag(&name);
    Some(conv(
        &name,
        tagged.unwrap_or_else(|| slack_kind(&name)),
        &workspace,
    ))
}

fn slack_kind(name: &str) -> ConversationKind {
    let lower = name.to_ascii_lowercase();
    if lower.starts_with('#') {
        return ConversationKind::Channel;
    }
    if is_slack_view(name) {
        return ConversationKind::Other;
    }
    if lower.starts_with("thread:") || lower.contains(" thread") {
        return ConversationKind::Thread;
    }
    ConversationKind::Dm
}

pub fn is_slack_view(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "threads"
            | "activity"
            | "mentions"
            | "mentions & reactions"
            | "unreads"
            | "all unreads"
            | "unread messages"
            | "later"
            | "drafts"
            | "drafts & sent"
            | "saved items"
            | "saved"
            | "home"
            | "dms"
            | "huddles"
            | "channel browser"
            | "browse channels"
            | "people"
            | "people & user groups"
            | "files"
            | "search"
            | "slack connect"
            | "canvases"
            | "automations"
            | "templates"
            | "more"
    )
}

fn looks_like_placeholder(s: &str) -> bool {
    let Some(rest) = s.trim().strip_prefix("Window ") else {
        return false;
    };
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

fn strip_item_count(s: &str) -> String {
    let Some((left, right)) = s.rsplit_once(" - ") else {
        return s.to_string();
    };
    if is_item_count(right.trim()) {
        return strip_item_count(left.trim());
    }
    s.to_string()
}

fn is_item_count(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    if matches!(lower.as_str(), "new items" | "new item") {
        return true;
    }
    let mut parts = lower.splitn(2, ' ');
    let n = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("");
    !n.is_empty()
        && n.chars().all(|c| c.is_ascii_digit())
        && matches!(rest, "new items" | "new item" | "items")
}

fn strip_leading_badge(s: &str) -> String {
    let t = s.trim();
    if let Some(rest) = t.strip_prefix('•') {
        return rest.trim_start().to_string();
    }
    if let Some(end) = t.find(')') {
        if t.starts_with('(') {
            let inner = &t[1..end];
            if !inner.is_empty() && inner.chars().all(|c| c.is_ascii_digit() || c == '+') {
                return t[end + 1..].trim_start().to_string();
            }
        }
    }
    t.to_string()
}

fn split_kind_tag(name: &str) -> (String, Option<ConversationKind>) {
    let t = name.trim();
    let Some(open) = t.rfind(" (") else {
        return (t.to_string(), None);
    };
    if !t.ends_with(')') {
        return (t.to_string(), None);
    }
    let tag = t[open + 2..t.len() - 1].trim().to_ascii_lowercase();
    let base = t[..open].trim();
    if base.is_empty() {
        return (t.to_string(), None);
    }
    let kind = match tag.as_str() {
        "dm" | "im" | "group" | "group dm" | "mpim" => Some(ConversationKind::Dm),
        "channel" | "private channel" | "public channel" => Some(ConversationKind::Channel),
        "thread" | "threads" => Some(ConversationKind::Thread),
        "huddle" => Some(ConversationKind::Huddle),
        _ => None,
    };
    if kind.is_some() {
        (base.to_string(), kind)
    } else {
        (t.to_string(), None)
    }
}

fn parse_discord(title: &str) -> Option<Conversation> {
    let cleaned = strip_app_suffix(&normalize_dashes(title), "discord")?;
    if cleaned.is_empty() {
        return None;
    }
    if let Some((left, right)) = cleaned.split_once('|') {
        let name = left.trim();
        let workspace = right.trim();
        if name.is_empty() {
            return None;
        }
        let kind = if name.starts_with('@') {
            ConversationKind::Dm
        } else {
            ConversationKind::Channel
        };
        return Some(conv(name, kind, workspace));
    }
    let kind = if cleaned.starts_with('@') {
        ConversationKind::Dm
    } else if matches!(
        cleaned.to_ascii_lowercase().as_str(),
        "friends" | "nitro" | "library" | "discover" | "shop"
    ) {
        ConversationKind::Other
    } else if cleaned.starts_with('#') {
        ConversationKind::Channel
    } else {
        ConversationKind::Other
    };
    Some(conv(&cleaned, kind, ""))
}

fn parse_teams(title: &str) -> Option<Conversation> {
    let cleaned = strip_app_suffix(&normalize_dashes(title), "microsoft teams")
        .or_else(|| strip_app_suffix(&normalize_dashes(title), "teams"))?;
    if cleaned.is_empty() {
        return None;
    }
    let parts: Vec<&str> = cleaned
        .split('|')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() {
        return None;
    }
    let lower0 = parts[0].to_ascii_lowercase();
    if lower0.contains("meeting") {
        return Some(conv(parts[0], ConversationKind::Huddle, ""));
    }
    if matches!(lower0.as_str(), "chat" | "chats") {
        let name = parts.get(1).copied().unwrap_or("Chat");
        let workspace = parts.get(2).copied().unwrap_or("");
        return Some(conv(name, ConversationKind::Dm, workspace));
    }
    if matches!(
        lower0.as_str(),
        "activity" | "calendar" | "calls" | "teams" | "files"
    ) {
        return Some(conv(parts[0], ConversationKind::Other, ""));
    }
    Some(conv(
        parts[0],
        ConversationKind::Dm,
        parts.get(1).copied().unwrap_or(""),
    ))
}

fn conv(name: &str, kind: ConversationKind, workspace: &str) -> Conversation {
    Conversation {
        name: name.trim().to_string(),
        kind,
        workspace: workspace.trim().to_string(),
        window_index: None,
    }
}

fn normalize_dashes(title: &str) -> String {
    title.replace(['\u{2013}', '\u{2014}', '\u{2012}'], "-")
}

fn strip_app_suffix(title: &str, app: &str) -> Option<String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower == app {
        return None;
    }

    let rest = [" - ", " | ", " \u{2013} ", " \u{2014} "]
        .iter()
        .find_map(|sep| lower.strip_suffix(&format!("{sep}{app}")))?;
    let cleaned = trimmed[..rest.len()].trim().to_string();
    if cleaned.to_ascii_lowercase() == app || cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn split_workspace(title: &str) -> (String, String) {
    match title.rsplit_once(" - ") {
        Some((left, right)) if !left.trim().is_empty() && !right.trim().is_empty() => {
            (left.trim().to_string(), right.trim().to_string())
        }
        _ => (title.trim().to_string(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_title, Recents};
    use crate::protocol::ConversationKind;

    #[test]
    fn slack_channel_and_dm() {
        let ch = parse_title(
            "com.tinyspeck.slackmacgap",
            "#design \u{2013} Acme \u{2013} Slack",
        )
        .unwrap();
        assert_eq!(ch.name, "#design");
        assert_eq!(ch.kind, ConversationKind::Channel);
        assert_eq!(ch.workspace, "Acme");

        let dm = parse_title("com.tinyspeck.slackmacgap", "Alice Chen - Acme - Slack").unwrap();
        assert_eq!(dm.name, "Alice Chen");
        assert_eq!(dm.kind, ConversationKind::Dm);

        let threads = parse_title("com.tinyspeck.slackmacgap", "Threads - Acme - Slack").unwrap();
        assert_eq!(threads.kind, ConversationKind::Other);
        let thread = parse_title(
            "com.tinyspeck.slackmacgap",
            "Thread: #design - Acme - Slack",
        )
        .unwrap();
        assert_eq!(thread.kind, ConversationKind::Thread);

        let huddle =
            parse_title("com.tinyspeck.slackmacgap", "Huddle: design - Acme - Slack").unwrap();
        assert_eq!(huddle.kind, ConversationKind::Huddle);
        assert_eq!(huddle.name, "design");

        assert!(parse_title("com.tinyspeck.slackmacgap", "Slack").is_none());

        let live = parse_title(
            "com.tinyspeck.slackmacgap",
            "Abhi (DM) - OhDamn! - 4 new items - Slack",
        )
        .unwrap();
        assert_eq!(live.name, "Abhi");
        assert_eq!(live.kind, ConversationKind::Dm);
        assert_eq!(live.workspace, "OhDamn!");
    }

    #[test]
    fn discord_channel_and_dm() {
        let ch = parse_title("com.hnc.Discord", "#general | Caddie - Discord").unwrap();
        assert_eq!(ch.name, "#general");
        assert_eq!(ch.workspace, "Caddie");
        assert_eq!(ch.kind, ConversationKind::Channel);

        let dm = parse_title("com.hnc.Discord", "@alice - Discord").unwrap();
        assert_eq!(dm.kind, ConversationKind::Dm);
        assert_eq!(dm.name, "@alice");

        assert!(parse_title("com.hnc.Discord", "Discord").is_none());
    }

    #[test]
    fn teams_chat() {
        let chat = parse_title(
            "com.microsoft.teams2",
            "Chat | Alice | Contoso | Microsoft Teams",
        )
        .unwrap();
        assert_eq!(chat.name, "Alice");
        assert_eq!(chat.kind, ConversationKind::Dm);
        assert_eq!(chat.workspace, "Contoso");
    }

    #[test]
    fn recents_move_to_front() {
        let mut recents = Recents::default();
        recents.ingest(
            "com.hnc.Discord",
            parse_title("com.hnc.Discord", "#one | S - Discord").unwrap(),
        );
        recents.ingest(
            "com.hnc.Discord",
            parse_title("com.hnc.Discord", "#two | S - Discord").unwrap(),
        );
        recents.ingest(
            "com.hnc.Discord",
            parse_title("com.hnc.Discord", "#one | S - Discord").unwrap(),
        );
        let list = recents.list("com.hnc.Discord");
        assert_eq!(list[0].name, "#one");
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn recents_skip_chrome_views() {
        let mut recents = Recents::default();
        recents.ingest(
            "com.hnc.Discord",
            parse_title("com.hnc.Discord", "Friends - Discord").unwrap(),
        );
        assert!(recents.list("com.hnc.Discord").is_empty());
    }

    #[test]
    fn open_windows_skip_current() {
        use super::parse_windows;
        let current = parse_title("com.tinyspeck.slackmacgap", "#design - Acme - Slack").unwrap();
        let open = parse_windows(
            "com.tinyspeck.slackmacgap",
            &[
                (1, "#design - Acme - Slack".into()),
                (2, "Alice Chen - Acme - Slack".into()),
            ],
            Some(&current),
        );
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].name, "Alice Chen");
        assert_eq!(open[0].window_index, Some(2));
    }
}

#[cfg(test)]
mod view_tests {
    use super::{is_slack_view, parse_title};
    use crate::protocol::ConversationKind;

    const SLACK: &str = "com.tinyspeck.slackmacgap";

    #[test]
    fn slack_views_are_not_conversations() {
        for title in [
            "Threads - ohdamn! - Slack",
            "Unread messages - ohdamn! - Slack",
            "Activity - ohdamn! - Slack",
            "Drafts & sent - ohdamn! - Slack",
        ] {
            let conv = parse_title(SLACK, title).expect("parsed");
            assert_eq!(conv.kind, ConversationKind::Other, "{title}");
        }
    }

    #[test]
    fn real_conversations_still_parse() {
        let dm = parse_title(SLACK, "Sid - ohdamn! - Slack").unwrap();
        assert_eq!(dm.kind, ConversationKind::Dm);
        let channel = parse_title(SLACK, "#design - ohdamn! - Slack").unwrap();
        assert_eq!(channel.kind, ConversationKind::Channel);
        let thread = parse_title(SLACK, "Thread: #design - ohdamn! - Slack").unwrap();
        assert_eq!(thread.kind, ConversationKind::Thread);
        assert!(!is_slack_view("Sid"));
    }
}

#[cfg(test)]
mod suffix_tests {
    use super::parse_title;

    const SLACK: &str = "com.tinyspeck.slackmacgap";

    #[test]
    fn other_apps_windows_are_not_slack_conversations() {

        for title in [
            "pnpm",
            "Inbox (714) - abhishek@oh-damn.com - OhDamn Mail - Google Chrome",
            "Pipelines - Run 20260825.2 - Google Chrome",
            "Spotify Premium",
        ] {
            assert!(parse_title(SLACK, title).is_none(), "{title}");
        }
    }

    #[test]
    fn real_slack_titles_still_parse() {
        assert!(parse_title(SLACK, "#petdex - OhDamn! - Slack").is_some());
        assert!(parse_title(SLACK, "Abhi (DM) - OhDamn! - Slack").is_some());
    }
}
