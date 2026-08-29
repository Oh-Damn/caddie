use core_foundation::array::CFArray;
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::string::{CFString, CFStringRef};
use std::ffi::c_void;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

type AXUIElementRef = *const c_void;
type AXError = i32;

const AX_OK: AXError = 0;
const READ_BUDGET: Duration = Duration::from_millis(180);
const FRESH: Duration = Duration::from_millis(1500);

static LAST: Mutex<Option<(Instant, bool)>> = Mutex::new(None);

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

pub fn note(liked: bool) {
    *LAST.lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), liked));
}

pub fn apply(np: &mut crate::protocol::NowPlaying) {
    if np.source_bundle_id != "com.spotify.client" {
        return;
    }
    if let Some((at, liked)) = *LAST.lock().unwrap_or_else(|e| e.into_inner()) {
        if at.elapsed() < FRESH {
            np.liked = liked;
            return;
        }
    }
    if let Some(liked) = liked(false) {
        np.liked = liked;
        note(liked);
    }
}

pub fn liked(click: bool) -> Option<bool> {
    if !super::permissions::accessibility_trusted() {
        return None;
    }
    let pid = spotify_pid()?;
    let app = unsafe { AXUIElementCreateApplication(pid) };
    if app.is_null() {
        return None;
    }
    let _owned = unsafe { CFType::wrap_under_create_rule(app as CFTypeRef) };
    let deadline = Instant::now() + READ_BUDGET;
    if let Some(hit) = scan_app(app, click, deadline) {
        return Some(hit);
    }
    enable_manual(app);
    scan_app(app, click, deadline)
}

fn spotify_pid() -> Option<i32> {
    let out = Command::new("/usr/bin/pgrep")
        .args(["-x", "Spotify"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|line| line.trim().parse().ok())
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

fn scan_app(app: AXUIElementRef, click: bool, deadline: Instant) -> Option<bool> {
    let windows = attr_array(app, "AXWindows")?;
    for win in windows.get_all_values().into_iter().rev() {
        if let Some(hit) = walk(win as AXUIElementRef, click, 0, deadline) {
            return Some(hit);
        }
    }
    None
}

fn walk(el: AXUIElementRef, click: bool, depth: u8, deadline: Instant) -> Option<bool> {
    if el.is_null() || depth > 14 || Instant::now() >= deadline {
        return None;
    }
    if let Some(hit) = hint(el) {
        if click && !press(el) {
            return None;
        }
        return Some(hit);
    }
    let kids = attr_array(el, "AXChildren")?;
    for child in kids.get_all_values().into_iter().rev() {
        if let Some(hit) = walk(child as AXUIElementRef, click, depth + 1, deadline) {
            return Some(hit);
        }
    }
    None
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

fn hint(el: AXUIElementRef) -> Option<bool> {
    let role = ax_string(el, "AXRole")?;
    if role != "AXButton" && role != "AXCheckBox" && role != "AXToggle" {
        return None;
    }
    let blob = format!(
        "{} {} {}",
        ax_string(el, "AXDescription").unwrap_or_default(),
        ax_string(el, "AXTitle").unwrap_or_default(),
        ax_string(el, "AXHelp").unwrap_or_default(),
    );
    like_from_blob(&blob)
}

fn like_from_blob(blob: &str) -> Option<bool> {
    if blob.contains("Remove from Your Library")
        || blob.contains("Remove from Liked Songs")
        || blob.contains("Unlike this")
        || blob.contains("Added to Liked Songs")
    {
        return Some(true);
    }
    if blob.contains("Save to Your Library")
        || blob.contains("Add to Your Library")
        || blob.contains("Add to Liked Songs")
        || blob.contains("Save to Liked Songs")
        || blob.contains("Like this song")
    {
        return Some(false);
    }
    None
}

fn ax_string(el: AXUIElementRef, attr: &'static str) -> Option<String> {
    let name = CFString::from_static_string(attr);
    let mut raw: CFTypeRef = std::ptr::null();
    let err = unsafe { AXUIElementCopyAttributeValue(el, name.as_concrete_TypeRef(), &mut raw) };
    if err != AX_OK || raw.is_null() {
        return None;
    }
    let s = unsafe { CFString::wrap_under_create_rule(raw as _) };
    let out = s.to_string();
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn press(el: AXUIElementRef) -> bool {
    let action = CFString::from_static_string("AXPress");
    unsafe { AXUIElementPerformAction(el, action.as_concrete_TypeRef()) == AX_OK }
}
