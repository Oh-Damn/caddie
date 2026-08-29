use crate::protocol::BrowserTab;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

struct ProfileCache {
    path: PathBuf,
    mtime: Option<SystemTime>,
    profiles: Vec<ChromeProfile>,
}

static PROFILE_CACHE: Mutex<Option<ProfileCache>> = Mutex::new(None);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeProfile {
    pub name: String,
    pub gaia_given: String,
    pub gaia_name: String,
    pub email: String,
    pub display: String,
}

#[derive(Deserialize)]
struct LocalStateFile {
    profile: Option<LocalStateProfile>,
}

#[derive(Deserialize)]
struct LocalStateProfile {
    info_cache: Option<HashMap<String, ProfileInfo>>,
}

#[derive(Deserialize, Default)]
struct ProfileInfo {
    #[serde(default)]
    name: String,
    #[serde(default)]
    gaia_name: String,
    #[serde(default)]
    gaia_given_name: String,
    #[serde(default)]
    user_name: String,
}

struct Alias {
    needle: String,
    display: String,
}

pub fn product_label(bundle_id: &str) -> Option<&'static str> {
    match bundle_id {
        "com.google.Chrome"
        | "com.google.Chrome.beta"
        | "com.google.Chrome.dev"
        | "com.google.Chrome.canary" => Some("Google Chrome"),
        "com.brave.Browser" => Some("Brave"),
        _ => None,
    }
}

pub fn load_profiles(bundle_id: &str) -> Vec<ChromeProfile> {
    let Some(path) = local_state_path(bundle_id) else {
        return Vec::new();
    };
    load_profiles_at(&path)
}

pub fn attach_profiles(
    tabs: &mut [BrowserTab],
    profiles: &[ChromeProfile],
    ax_titles: &[String],
    product: &str,
) {
    if tabs.is_empty() {
        return;
    }
    let aliases = profile_aliases(profiles);
    let mut unused: Vec<(usize, String)> = ax_titles.iter().cloned().enumerate().collect();
    let mut windows: Vec<u32> = tabs.iter().map(|t| t.window_index).collect();
    windows.sort_unstable();
    windows.dedup();

    let mut labels: HashMap<u32, String> = HashMap::new();
    for wi in windows {
        if tabs
            .iter()
            .any(|t| t.window_index == wi && t.profile.eq_ignore_ascii_case("incognito"))
        {
            labels.insert(wi, "Incognito".into());
            continue;
        }
        let active = tabs
            .iter()
            .find(|t| t.window_index == wi && t.active)
            .or_else(|| tabs.iter().find(|t| t.window_index == wi));
        let Some(active) = active else {
            continue;
        };
        let hit = take_ax(&mut unused, &active.title, product);
        let marker = hit
            .as_deref()
            .and_then(|ax| extract_marker(ax, product, &aliases));
        let label = match marker.as_deref() {
            Some(m) => resolve_marker(m, &aliases, profiles),
            None if profiles.len() == 1 => profiles[0].display.clone(),
            None => String::new(),
        };
        labels.insert(wi, label);
    }

    for tab in tabs.iter_mut() {
        if tab.profile.eq_ignore_ascii_case("incognito") {
            tab.profile = "Incognito".into();
            continue;
        }
        if let Some(label) = labels.get(&tab.window_index) {
            tab.profile = label.clone();
        }
    }
}

pub fn parse_local_state(raw: &str) -> Vec<ChromeProfile> {
    let Ok(parsed) = serde_json::from_str::<LocalStateFile>(raw) else {
        return Vec::new();
    };
    let Some(cache) = parsed.profile.and_then(|p| p.info_cache) else {
        return Vec::new();
    };
    let mut profiles: Vec<ChromeProfile> = cache
        .into_values()
        .map(|info| ChromeProfile {
            name: info.name.trim().to_string(),
            gaia_given: info.gaia_given_name.trim().to_string(),
            gaia_name: info.gaia_name.trim().to_string(),
            email: info.user_name.trim().to_string(),
            display: String::new(),
        })
        .filter(|p| !p.name.is_empty() || !p.email.is_empty() || !p.gaia_given.is_empty())
        .collect();
    set_unique_displays(&mut profiles);
    profiles
}

fn local_state_path(bundle_id: &str) -> Option<PathBuf> {
    let rel = match bundle_id {
        "com.google.Chrome" => "Google/Chrome/Local State",
        "com.google.Chrome.canary" => "Google/Chrome Canary/Local State",
        "com.google.Chrome.beta" => "Google/Chrome Beta/Local State",
        "com.google.Chrome.dev" => "Google/Chrome Dev/Local State",
        "com.brave.Browser" => "BraveSoftware/Brave-Browser/Local State",
        _ => return None,
    };
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library/Application Support")
            .join(rel),
    )
}

fn load_profiles_at(path: &Path) -> Vec<ChromeProfile> {
    let mtime = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());
    {
        let cache = PROFILE_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = cache.as_ref() {
            if entry.path == path && entry.mtime == mtime {
                return entry.profiles.clone();
            }
        }
    }
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    let profiles = parse_local_state(&raw);
    let mut cache = PROFILE_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    *cache = Some(ProfileCache {
        path: path.to_path_buf(),
        mtime,
        profiles: profiles.clone(),
    });
    profiles
}

fn set_unique_displays(profiles: &mut [ChromeProfile]) {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for p in profiles.iter() {
        let key = p.name.to_ascii_lowercase();
        *counts.entry(key).or_default() += 1;
    }
    for p in profiles.iter_mut() {
        let key = p.name.to_ascii_lowercase();
        let clash = counts.get(&key).copied().unwrap_or(0) > 1;
        p.display = if !clash && !p.name.is_empty() {
            p.name.clone()
        } else if !p.gaia_given.is_empty() && !p.name.is_empty() {
            format!("{} ({})", p.gaia_given, p.name)
        } else if !p.email.is_empty() {
            p.email.clone()
        } else if !p.name.is_empty() {
            p.name.clone()
        } else {
            p.gaia_given.clone()
        };
    }
}

fn profile_aliases(profiles: &[ChromeProfile]) -> Vec<Alias> {
    let mut out = Vec::new();
    for p in profiles {
        push_alias(&mut out, &p.display, &p.display);
        push_alias(&mut out, &p.name, &p.display);
        if !p.gaia_given.is_empty() && !p.name.is_empty() {
            push_alias(
                &mut out,
                &format!("{} ({})", p.gaia_given, p.name),
                &p.display,
            );
        }
        if !p.gaia_name.is_empty() && !p.name.is_empty() {
            push_alias(
                &mut out,
                &format!("{} ({})", p.gaia_name, p.name),
                &p.display,
            );
        }
        push_alias(&mut out, &p.email, &p.display);
    }
    out.sort_by_key(|alias| std::cmp::Reverse(alias.needle.len()));
    out
}

fn push_alias(out: &mut Vec<Alias>, needle: &str, display: &str) {
    let needle = needle.trim();
    if needle.is_empty() {
        return;
    }
    if out
        .iter()
        .any(|a| a.needle.eq_ignore_ascii_case(needle) && a.display == display)
    {
        return;
    }
    out.push(Alias {
        needle: needle.to_string(),
        display: display.to_string(),
    });
}

fn extract_marker(ax_title: &str, product: &str, aliases: &[Alias]) -> Option<String> {
    let pos = ax_title.rfind(product)?;
    let after = strip_product_suffix(ax_title[pos + product.len()..].trim());
    if !after.is_empty() {
        return Some(after.to_string());
    }
    let before = trim_trailing_seps(&ax_title[..pos]);
    for alias in aliases {
        for sep in [" - ", " \u{2013} ", " \u{2014} "] {
            let suffix = format!("{sep}{}", alias.needle);
            if before.len() >= suffix.len()
                && before[before.len() - suffix.len()..].eq_ignore_ascii_case(&suffix)
            {
                return Some(alias.needle.clone());
            }
        }
    }
    None
}

fn strip_product_suffix(rest: &str) -> &str {
    let rest = rest.trim();
    for channel in ["Canary", "Beta", "Dev"] {
        if let Some(stripped) = rest.strip_prefix(channel) {
            return trim_leading_seps(stripped);
        }
    }
    trim_leading_seps(rest)
}

fn trim_leading_seps(value: &str) -> &str {
    value
        .trim_start_matches(|c: char| {
            c == '-' || c == '\u{2013}' || c == '\u{2014}' || c.is_whitespace()
        })
        .trim()
}

fn trim_trailing_seps(value: &str) -> &str {
    value
        .trim_end_matches(|c: char| {
            c == '-' || c == '\u{2013}' || c == '\u{2014}' || c.is_whitespace()
        })
        .trim()
}

fn resolve_marker(marker: &str, aliases: &[Alias], profiles: &[ChromeProfile]) -> String {
    let marker = marker.trim();
    if marker.is_empty() {
        return if profiles.len() == 1 {
            profiles[0].display.clone()
        } else {
            String::new()
        };
    }
    if marker.eq_ignore_ascii_case("incognito") {
        return "Incognito".into();
    }
    if marker.eq_ignore_ascii_case("guest") {
        return "Guest".into();
    }
    let stripped = marker.trim_matches(|c| c == '(' || c == ')').trim();
    for alias in aliases {
        if alias.needle.eq_ignore_ascii_case(marker) || alias.needle.eq_ignore_ascii_case(stripped)
        {
            return alias.display.clone();
        }
    }
    marker.to_string()
}

fn take_ax(unused: &mut Vec<(usize, String)>, tab_title: &str, product: &str) -> Option<String> {
    let by_title = unused.iter().position(|(_, ax)| {
        let ax = super::strip_window_media_prefix(ax);
        ax.contains(product) && super::window_title_has_tab(ax, tab_title)
    });
    by_title.map(|i| unused.remove(i).1)
}

#[cfg(test)]
mod tests {
    use super::{attach_profiles, parse_local_state, ChromeProfile};
    use crate::protocol::BrowserTab;

    fn tab(title: &str, window: u32, index: u32, active: bool) -> BrowserTab {
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

    fn sample_profiles() -> Vec<ChromeProfile> {
        parse_local_state(
            r#"{
              "profile": {
                "info_cache": {
                  "Default": {
                    "name": "abhishek",
                    "gaia_given_name": "abhishek",
                    "gaia_name": "abhishek sharma",
                    "user_name": "abhi.sharma.0873@gmail.com"
                  },
                  "Profile 9": {
                    "name": "oh-damn.com",
                    "gaia_given_name": "Abhishek",
                    "gaia_name": "Abhishek Sharma",
                    "user_name": "abhishek@oh-damn.com"
                  },
                  "Profile 5": {
                    "name": "oh-damn.com",
                    "gaia_given_name": "Hello",
                    "gaia_name": "Hello OhDamn",
                    "user_name": "hello@oh-damn.com"
                  }
                }
              }
            }"#,
        )
    }

    #[test]
    fn duplicate_profile_names_use_given_name() {
        let profiles = sample_profiles();
        let displays: Vec<&str> = profiles.iter().map(|p| p.display.as_str()).collect();
        assert!(displays.contains(&"abhishek"));
        assert!(displays.contains(&"Abhishek (oh-damn.com)"));
        assert!(displays.contains(&"Hello (oh-damn.com)"));
    }

    #[test]
    fn groups_windows_from_ax_titles() {
        let profiles = sample_profiles();
        let mut tabs = vec![
            tab("YouTube", 1, 1, true),
            tab("Gmail", 1, 2, false),
            tab("Summary - Overview", 2, 1, true),
        ];
        let ax = vec![
            "YouTube - Google Chrome \u{2013} abhishek".into(),
            "Summary - Overview - Google Chrome \u{2013} Abhishek (oh-damn.com)".into(),
        ];
        attach_profiles(&mut tabs, &profiles, &ax, "Google Chrome");
        assert_eq!(tabs[0].profile, "abhishek");
        assert_eq!(tabs[1].profile, "abhishek");
        assert_eq!(tabs[2].profile, "Abhishek (oh-damn.com)");
    }

    #[test]
    fn reads_profile_before_product_in_older_titles() {
        let profiles = sample_profiles();
        let mut tabs = vec![tab("Inbox", 1, 1, true)];
        let ax = vec!["Inbox - abhishek - Google Chrome".into()];
        attach_profiles(&mut tabs, &profiles, &ax, "Google Chrome");
        assert_eq!(tabs[0].profile, "abhishek");
    }

    #[test]
    fn keeps_incognito_windows_separate() {
        let profiles = sample_profiles();
        let mut tabs = vec![tab("YouTube", 1, 1, true)];
        tabs[0].profile = "incognito".into();
        attach_profiles(
            &mut tabs,
            &profiles,
            &["YouTube - Google Chrome".into()],
            "Google Chrome",
        );
        assert_eq!(tabs[0].profile, "Incognito");
    }

    #[test]
    fn does_not_assign_profile_by_window_index() {
        let profiles = sample_profiles();
        let mut tabs = vec![
            tab("About Page", 1, 1, true),
            tab("i'm going back to 505 - YouTube", 2, 1, true),
        ];
        let ax = vec![
            "\u{1F50A} i'm going back to 505 - YouTube - Google Chrome \u{2013} abhishek".into(),
            "About Page - Google Chrome \u{2013} Abhishek (oh-damn.com)".into(),
        ];
        attach_profiles(&mut tabs, &profiles, &ax, "Google Chrome");
        assert_eq!(tabs[0].profile, "Abhishek (oh-damn.com)");
        assert_eq!(tabs[1].profile, "abhishek");
    }
}
