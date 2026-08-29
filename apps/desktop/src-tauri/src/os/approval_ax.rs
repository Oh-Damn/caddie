use crate::protocol::{ApprovalSession, BrowserTab, RunningApp};
use core_foundation::array::CFArray;
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::string::{CFString, CFStringRef};
use std::collections::VecDeque;
use std::ffi::c_void;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

type AXUIElementRef = *const c_void;
type AXError = i32;

const AX_OK: AXError = 0;
const READ_BUDGET: Duration = Duration::from_millis(280);
const FRESH: Duration = Duration::from_millis(800);
const MAX_NODES: usize = 500;
const MAX_LABEL: usize = 160;

const CURSOR_BUNDLES: &[&str] = &["com.todesktop.230313mzl4w4u92", "com.cursor.Cursor"];
const CLAUDE_BUNDLES: &[&str] = &["com.anthropic.claudefordesktop"];
const BROWSER_BUNDLES: &[&str] = &[
    "com.google.Chrome",
    "com.google.Chrome.canary",
    "com.google.Chrome.beta",
    "com.google.Chrome.dev",
    "com.brave.Browser",
    "com.apple.Safari",
    "company.thebrowser.Browser",
];

static LAST: Mutex<Option<(Instant, Option<ApprovalSession>)>> = Mutex::new(None);
static LAST_UNMATCHED: Mutex<Option<String>> = Mutex::new(None);
static AX_REACH: Mutex<Vec<(String, bool)>> = Mutex::new(Vec::new());

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> AXError;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> AXError;
    fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> AXError;
}

#[derive(Clone, Copy)]
enum Kind {
    Cursor,
    Claude,
}

pub fn probe(
    running: &[RunningApp],
    tabs: &[BrowserTab],
    tabs_bundle: &str,
) -> Option<ApprovalSession> {
    if let Some((at, hit)) = LAST.lock().unwrap_or_else(|e| e.into_inner()).clone() {
        let ttl = if hit.is_some() {
            FRESH
        } else {
            Duration::from_millis(250)
        };
        if at.elapsed() < ttl {
            return hit;
        }
    }
    let found = scan_running(running, tabs, tabs_bundle);
    *LAST.lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), found.clone()));
    found
}

pub fn press(bundle_id: &str, allow: bool) -> bool {
    if !super::permissions::accessibility_trusted() {
        return false;
    }
    let Some(kind) = kind_for(bundle_id) else {
        return false;
    };
    let hint = if is_cursor(bundle_id) {
        "Cursor"
    } else if is_claude(bundle_id) {
        "Claude"
    } else {
        ""
    };
    let Some(pid) = pid_for(bundle_id, hint) else {
        return false;
    };
    let app = unsafe { AXUIElementCreateApplication(pid) };
    if app.is_null() {
        return false;
    }
    let _owned = unsafe { CFType::wrap_under_create_rule(app as CFTypeRef) };
    enable_manual(app);
    let deadline = Instant::now() + READ_BUDGET;
    if press_windows(app, kind, allow, deadline) {
        return true;
    }
    enable_manual(app);
    press_windows(app, kind, allow, Instant::now() + READ_BUDGET)
}

fn scan_running(
    running: &[RunningApp],
    tabs: &[BrowserTab],
    tabs_bundle: &str,
) -> Option<ApprovalSession> {
    if !super::permissions::accessibility_trusted() {
        return None;
    }
    for app in running {
        if is_cursor(&app.bundle_id) {
            if let Some(hit) = scan_app(app, Kind::Cursor) {
                return Some(hit);
            }
        }
    }
    for app in running {
        if is_claude(&app.bundle_id) {
            if let Some(hit) = scan_app(app, Kind::Claude) {
                return Some(hit);
            }
        }
    }
    for app in running {
        if is_browser(&app.bundle_id) && claude_target(app, tabs, tabs_bundle) {
            if let Some(hit) = scan_app(app, Kind::Claude) {
                return Some(hit);
            }
        }
    }
    None
}

fn claude_target(app: &RunningApp, tabs: &[BrowserTab], tabs_bundle: &str) -> bool {
    claude_window(app)
        || (app.bundle_id == tabs_bundle
            && tabs.iter().any(|t| t.active && looks_like_claude(&t.title)))
}

fn claude_window(app: &RunningApp) -> bool {
    app.windows.iter().any(|w| looks_like_claude(&w.title))
}

fn looks_like_claude(title: &str) -> bool {
    let t = title.to_ascii_lowercase();
    t == "claude"
        || t.contains("claude.ai")
        || t.ends_with(" claude")
        || t.contains(" | claude")
        || t.contains(" - claude")
}

fn scan_app(app: &RunningApp, kind: Kind) -> Option<ApprovalSession> {
    let pid = pid_for(&app.bundle_id, &app.name)?;
    let ax = unsafe { AXUIElementCreateApplication(pid) };
    if ax.is_null() {
        return None;
    }
    let _owned = unsafe { CFType::wrap_under_create_rule(ax as CFTypeRef) };
    enable_manual(ax);
    let mut found = Found::default();
    collect_windows(ax, &mut found, Instant::now() + READ_BUDGET);
    if classify(&found, kind).is_none() {
        enable_manual(ax);
        found = Found::default();
        collect_windows(ax, &mut found, Instant::now() + READ_BUDGET);
    }
    note_reach(&app.bundle_id, &found);
    let Some((_, can_allow, can_deny)) = classify(&found, kind) else {
        report_unmatched(&app.name, &found);
        return None;
    };
    Some(ApprovalSession {
        app: match kind {
            Kind::Cursor => "cursor".into(),
            Kind::Claude => "claude".into(),
        },
        app_name: app.name.clone(),
        bundle_id: app.bundle_id.clone(),
        title: "Waiting for approval".into(),
        can_allow,
        can_deny,
    })
}

fn note_reach(bundle_id: &str, found: &Found) {
    let reachable = !found.buttons.is_empty() || found.blob.len() > 32;
    let mut reach = AX_REACH.lock().unwrap_or_else(|e| e.into_inner());
    match reach.iter_mut().find(|(id, _)| id == bundle_id) {
        Some(entry) => entry.1 = reachable,
        None => reach.push((bundle_id.to_string(), reachable)),
    }
}

pub fn tree_reachable(bundle_id: &str) -> Option<bool> {
    AX_REACH
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .find(|(id, _)| id == bundle_id)
        .map(|(_, ok)| *ok)
}

fn report_unmatched(app_name: &str, found: &Found) {
    if found.buttons.is_empty() {
        return;
    }
    let labels = found.buttons.join(" | ");
    let mut last = LAST_UNMATCHED.lock().unwrap_or_else(|e| e.into_inner());
    if last.as_deref() == Some(labels.as_str()) {
        return;
    }
    *last = Some(labels.clone());
    tracing::info!("{app_name} approval scan matched nothing. buttons: {labels}");
}

fn classify(found: &Found, kind: Kind) -> Option<(bool, bool, bool)> {
    match kind {
        Kind::Cursor => classify_cursor(&found.blob, &found.buttons),
        Kind::Claude => classify_claude(&found.blob, &found.buttons),
    }
}

#[derive(Default)]
struct Found {
    blob: String,
    buttons: Vec<String>,
}

fn collect_windows(app: AXUIElementRef, found: &mut Found, deadline: Instant) {
    let windows = attr_elements(app, "AXWindows");
    if windows.is_empty() {
        collect_bfs(app, found, deadline);
        return;
    }
    for win in windows.into_iter().rev() {
        collect_bfs(win.as_CFTypeRef() as AXUIElementRef, found, deadline);
        if Instant::now() >= deadline {
            return;
        }
    }
}

fn collect_bfs(root: AXUIElementRef, found: &mut Found, deadline: Instant) {
    if root.is_null() {
        return;
    }

    let mut q: VecDeque<(CFType, u8)> = VecDeque::new();
    q.push_back((unsafe { CFType::wrap_under_get_rule(root) }, 0));
    let mut nodes = 0usize;
    while let Some((el_ref, depth)) = q.pop_front() {
        if depth > 12 || nodes >= MAX_NODES || Instant::now() >= deadline {
            return;
        }
        let el = el_ref.as_CFTypeRef() as AXUIElementRef;
        nodes += 1;
        let role = ax_string(el, "AXRole").unwrap_or_default();
        let title = ax_string(el, "AXTitle").unwrap_or_default();
        let desc = ax_string(el, "AXDescription").unwrap_or_default();
        let help = ax_string(el, "AXHelp").unwrap_or_default();
        let value = ax_string(el, "AXValue").unwrap_or_default();
        push_label(found, &title);
        push_label(found, &desc);
        push_label(found, &help);
        push_label(found, &value);
        if role == "AXButton" || role == "AXCheckBox" || role == "AXLink" {
            let label = [title.as_str(), desc.as_str(), help.as_str(), value.as_str()]
                .into_iter()
                .find(|s| !s.is_empty())
                .unwrap_or("")
                .to_string();
            if !label.is_empty() {
                found.buttons.push(label);
            }
        }
        if skip_dive(&role) {
            continue;
        }
        let Some(kids) = attr_array(el, "AXChildren") else {
            continue;
        };
        for child in kids.get_all_values().into_iter().rev() {
            if child.is_null() {
                continue;
            }
            q.push_back((unsafe { CFType::wrap_under_get_rule(child) }, depth + 1));
        }
    }
}

fn push_label(found: &mut Found, raw: &str) {
    let t = raw.trim();
    if t.is_empty() || t.len() > MAX_LABEL {
        return;
    }
    found.blob.push(' ');
    found.blob.push_str(t);
}

fn skip_dive(role: &str) -> bool {
    matches!(
        role,
        "AXTextArea" | "AXTable" | "AXOutline" | "AXMenu" | "AXMenuBar"
    )
}

fn press_windows(app: AXUIElementRef, kind: Kind, allow: bool, deadline: Instant) -> bool {
    let windows = attr_elements(app, "AXWindows");
    if windows.is_empty() {
        return press_bfs(app, kind, allow, deadline);
    }
    for win in windows.into_iter().rev() {
        if press_bfs(win.as_CFTypeRef() as AXUIElementRef, kind, allow, deadline) {
            return true;
        }
    }
    false
}

fn press_bfs(root: AXUIElementRef, kind: Kind, allow: bool, deadline: Instant) -> bool {
    if root.is_null() {
        return false;
    }

    let mut q: VecDeque<(CFType, u8)> = VecDeque::new();
    q.push_back((unsafe { CFType::wrap_under_get_rule(root) }, 0));
    let mut nodes = 0usize;
    while let Some((el_ref, depth)) = q.pop_front() {
        if depth > 12 || nodes >= MAX_NODES || Instant::now() >= deadline {
            return false;
        }
        let el = el_ref.as_CFTypeRef() as AXUIElementRef;
        nodes += 1;
        let role = ax_string(el, "AXRole").unwrap_or_default();
        if role == "AXButton" || role == "AXCheckBox" || role == "AXLink" {
            let label = format!(
                "{} {} {} {}",
                ax_string(el, "AXTitle").unwrap_or_default(),
                ax_string(el, "AXDescription").unwrap_or_default(),
                ax_string(el, "AXHelp").unwrap_or_default(),
                ax_string(el, "AXValue").unwrap_or_default(),
            );
            if button_matches(&label, kind, allow) && ax_press(el) {
                return true;
            }
        }
        if skip_dive(&role) {
            continue;
        }
        let Some(kids) = attr_array(el, "AXChildren") else {
            continue;
        };
        for child in kids.get_all_values().into_iter().rev() {
            if child.is_null() {
                continue;
            }
            q.push_back((unsafe { CFType::wrap_under_get_rule(child) }, depth + 1));
        }
    }
    false
}

fn classify_cursor(blob: &str, buttons: &[String]) -> Option<(bool, bool, bool)> {
    let b = blob.to_ascii_lowercase();
    let labels: Vec<String> = buttons.iter().map(|s| s.to_ascii_lowercase()).collect();
    let all = format!("{b} {}", labels.join(" "));
    let waiting = all.contains("waiting for approval")
        || all.contains("waiting for your approval")
        || all.contains("awaiting approval")
        || all.contains("needs your approval")
        || all.contains("needs approval");
    let run = labels.iter().any(|l| is_cursor_allow(l)) || all.contains(" run ");
    let skip = labels.iter().any(|l| is_cursor_deny(l));
    if !waiting && !(run && skip) {
        return None;
    }
    Some((true, true, true))
}

fn classify_claude(blob: &str, buttons: &[String]) -> Option<(bool, bool, bool)> {
    let b = blob.to_ascii_lowercase();
    let labels: Vec<String> = buttons.iter().map(|s| s.to_ascii_lowercase()).collect();
    let all = format!("{b} {}", labels.join(" "));
    let in_text = all.contains("allow once")
        || all.contains("always allow")
        || all.contains("allow for this session")
        || all.contains("allow for this chat");
    let allow_btn = labels.iter().any(|l| is_claude_allow(l));
    let deny_btn = labels.iter().any(|l| is_claude_deny(l));
    if !in_text && !allow_btn && !deny_btn {
        return None;
    }
    Some((true, allow_btn || in_text, deny_btn || in_text))
}

fn button_matches(label: &str, kind: Kind, allow: bool) -> bool {
    let l = label.to_ascii_lowercase();
    match (kind, allow) {
        (Kind::Cursor, true) => is_cursor_allow(&l),
        (Kind::Cursor, false) => is_cursor_deny(&l),
        (Kind::Claude, true) => is_claude_allow(&l),
        (Kind::Claude, false) => is_claude_deny(&l),
    }
}

#[cfg(test)]
#[test]
#[ignore]
fn dump_agent_ax() {
    assert!(
        super::permissions::accessibility_trusted(),
        "grant Accessibility to the terminal running this"
    );
    for (name, bundle) in [
        ("Cursor", "com.todesktop.230313mzl4w4u92"),
        ("Claude", "com.anthropic.claudefordesktop"),
        ("Finder", "com.apple.finder"),
    ] {
        let Some(pid) = pid_for(bundle, name) else {
            println!("{name}: not running");
            continue;
        };
        let ax = unsafe { AXUIElementCreateApplication(pid) };
        let _owned = unsafe { CFType::wrap_under_create_rule(ax as CFTypeRef) };
        enable_manual(ax);
        let windows = attr_array(ax, "AXWindows").map(|w| w.len()).unwrap_or(-1);
        let mut found = Found::default();
        collect_windows(ax, &mut found, Instant::now() + Duration::from_secs(5));
        println!(
            "{name} pid {pid}: windows={windows} buttons={} blob={}",
            found.buttons.len(),
            found.blob.len()
        );
        if !found.buttons.is_empty() {
            println!("   {:?}", found.buttons);
        }
    }
}

fn is_cursor_allow(label: &str) -> bool {
    let t = label.trim();
    t == "run"
        || t.starts_with("run ")
        || t == "allow"
        || t == "accept"
        || t == "approve"
        || t.contains("run command")
}

fn is_cursor_deny(label: &str) -> bool {
    let t = label.trim();
    t.contains("skip") || t.contains("reject") || t == "deny"
}

fn is_claude_allow(label: &str) -> bool {
    let t = label.trim();
    t.contains("allow once")
        || t.contains("always allow")
        || t.contains("allow for this session")
        || t.contains("allow for this chat")
        || t == "allow"
}

fn is_claude_deny(label: &str) -> bool {
    let t = label.trim();
    t == "deny" || t.contains("don't allow") || t.contains("not now")
}

fn kind_for(bundle_id: &str) -> Option<Kind> {
    if is_cursor(bundle_id) {
        Some(Kind::Cursor)
    } else if is_claude(bundle_id) || is_browser(bundle_id) {
        Some(Kind::Claude)
    } else {
        None
    }
}

fn is_cursor(bundle_id: &str) -> bool {
    CURSOR_BUNDLES.contains(&bundle_id)
}

fn is_claude(bundle_id: &str) -> bool {
    CLAUDE_BUNDLES.contains(&bundle_id)
}

fn is_browser(bundle_id: &str) -> bool {
    BROWSER_BUNDLES.contains(&bundle_id)
}

fn pid_for(bundle_id: &str, name: &str) -> Option<i32> {
    pid_named(name)
        .or_else(|| {
            if is_cursor(bundle_id) {
                pid_named("Cursor")
            } else if is_claude(bundle_id) {
                pid_named("Claude")
            } else {
                None
            }
        })
        .or_else(|| pid_by_bundle(bundle_id))
}

fn pid_named(name: &str) -> Option<i32> {
    let t = name.trim();
    if t.is_empty() {
        return None;
    }
    let out = Command::new("/usr/bin/pgrep")
        .args(["-x", t])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|line| line.trim().parse().ok())
}

fn pid_by_bundle(bundle_id: &str) -> Option<i32> {
    let quoted = bundle_id.replace('\\', "\\\\").replace('"', "\\\"");
    let out = Command::new("/usr/bin/osascript")
        .args([
            "-e",
            &format!(
                r#"tell application "System Events"
  repeat with p in (every process whose bundle identifier is "{quoted}")
    try
      if (count of windows of p) > 0 then return unix id of p
    end try
  end repeat
  try
    return unix id of first process whose bundle identifier is "{quoted}"
  end try
  return 0
end tell"#
            ),
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let n: i32 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
    (n > 0).then_some(n)
}

fn enable_manual(app: AXUIElementRef) {
    let attr = CFString::from_static_string("AXManualAccessibility");
    unsafe {
        AXUIElementSetAttributeValue(
            app,
            attr.as_concrete_TypeRef(),
            CFBoolean::true_value().as_CFTypeRef(),
        );
    }
}

fn attr_array(el: AXUIElementRef, attr: &'static str) -> Option<CFArray> {
    let name = CFString::from_static_string(attr);
    let mut raw: CFTypeRef = std::ptr::null();
    let err = unsafe { AXUIElementCopyAttributeValue(el, name.as_concrete_TypeRef(), &mut raw) };
    if err != AX_OK || raw.is_null() {
        return None;
    }
    Some(unsafe { CFArray::wrap_under_create_rule(raw as _) })
}

fn attr_elements(el: AXUIElementRef, attr: &'static str) -> Vec<CFType> {
    let Some(array) = attr_array(el, attr) else {
        return Vec::new();
    };
    array
        .get_all_values()
        .into_iter()
        .filter(|raw| !raw.is_null())
        .map(|raw| unsafe { CFType::wrap_under_get_rule(raw) })
        .collect()
}

fn ax_string(el: AXUIElementRef, attr: &'static str) -> Option<String> {
    let name = CFString::from_static_string(attr);
    let mut raw: CFTypeRef = std::ptr::null();
    let err = unsafe { AXUIElementCopyAttributeValue(el, name.as_concrete_TypeRef(), &mut raw) };
    if err != AX_OK || raw.is_null() {
        return None;
    }
    let ty = unsafe { CFType::wrap_under_create_rule(raw) };
    if !ty.instance_of::<CFString>() {
        return None;
    }
    let s = unsafe { CFString::wrap_under_get_rule(raw as CFStringRef) };
    let out = s.to_string();
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn ax_press(el: AXUIElementRef) -> bool {
    let action = CFString::from_static_string("AXPress");
    unsafe { AXUIElementPerformAction(el, action.as_concrete_TypeRef()) == AX_OK }
}

#[cfg(test)]
mod tests {
    use super::{classify_claude, classify_cursor, claude_window, looks_like_claude};
    use crate::protocol::{AppWindow, RunningApp};

    #[test]
    fn cursor_status_line() {
        let hit = classify_cursor("Agent Waiting for approval…", &[]).unwrap();
        assert!(hit.0 && hit.1 && hit.2);
    }

    #[test]
    fn cursor_run_and_skip() {
        let hit = classify_cursor("", &["Run".into(), "Skip".into()]).unwrap();
        assert!(hit.0);
    }

    #[test]
    fn cursor_ignores_lone_run() {
        assert!(classify_cursor("Run tests", &["Run".into()]).is_none());
    }

    #[test]
    fn claude_allow_once() {
        let hit = classify_claude(
            "Allow once Always allow",
            &["Allow once".into(), "Deny".into()],
        )
        .unwrap();
        assert!(hit.0 && hit.1 && hit.2);
    }

    #[test]
    fn claude_text_without_buttons() {
        let hit = classify_claude("Allow for this session", &[]).unwrap();
        assert!(hit.0);
        assert!(hit.1);
        assert!(hit.2);
    }

    #[test]
    fn chrome_claude_tab_title() {
        let app = RunningApp {
            name: "Google Chrome".into(),
            bundle_id: "com.google.Chrome".into(),
            windows: vec![AppWindow {
                title: "Plan the header - Claude".into(),
                index: 1,
            }],
        };
        assert!(claude_window(&app));
        assert!(looks_like_claude("Chat | Claude"));
        assert!(looks_like_claude("Claude.ai"));
        assert!(!looks_like_claude("Inbox - Gmail"));
    }
}
