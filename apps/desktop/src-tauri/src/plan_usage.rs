use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

const STALE_AFTER_SECS: i64 = 45 * 60;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanUsage {

    pub five_hour: u8,
    pub seven_day: u8,

    pub age_secs: i64,
}

#[derive(Deserialize)]
struct History {
    #[serde(default)]
    samples: Vec<Sample>,
}

#[derive(Deserialize)]
struct Sample {

    #[serde(default)]
    t: i64,
    #[serde(default)]
    u: Option<Windows>,
}

#[derive(Deserialize)]
struct Windows {
    #[serde(default)]
    fh: f64,
    #[serde(default)]
    sd: f64,
}

static CACHE: Mutex<Option<(Stamp, Option<Newest>)>> = Mutex::new(None);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stamp {
    modified: Option<SystemTime>,
    len: u64,
}

#[derive(Debug, Clone, Copy)]
struct Newest {
    at: i64,
    five_hour: u8,
    seven_day: u8,
}

pub fn read(now: i64) -> Option<PlanUsage> {
    let newest = newest_sample()?;
    let age = now - newest.at;

    if !(0..=STALE_AFTER_SECS).contains(&age) {
        return None;
    }
    Some(PlanUsage {
        five_hour: newest.five_hour,
        seven_day: newest.seven_day,
        age_secs: age,
    })
}

fn newest_sample() -> Option<Newest> {
    let path = history_path()?;
    let meta = std::fs::metadata(&path).ok()?;
    let stamp = Stamp {
        modified: meta.modified().ok(),
        len: meta.len(),
    };
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((seen, parsed)) = cache.as_ref() {
        if *seen == stamp {
            return *parsed;
        }
    }
    let parsed = parse(&std::fs::read_to_string(&path).ok()?);
    *cache = Some((stamp, parsed));
    parsed
}

fn parse(raw: &str) -> Option<Newest> {
    let history: History = serde_json::from_str(raw).ok()?;

    history
        .samples
        .iter()
        .filter_map(|s| {
            let u = s.u.as_ref()?;
            Some(Newest {
                at: s.t / 1000,
                five_hour: clamp_percent(u.fh),
                seven_day: clamp_percent(u.sd),
            })
        })
        .max_by_key(|s| s.at)
}

fn clamp_percent(value: f64) -> u8 {
    value.round().clamp(0.0, 100.0) as u8
}

fn history_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").filter(|h| !h.is_empty())?;
    Some(
        PathBuf::from(home).join("Library/Application Support/Claude/plan-usage-history.json"),
    )
}

#[cfg(test)]
#[test]
#[ignore]
fn dump_plan_usage() {
    let now = chrono::Utc::now().timestamp();
    match read(now) {
        None => println!("no usable sample (Claude Desktop closed, or file absent)"),
        Some(u) => println!(
            "five_hour={}% seven_day={}% sampled {}m ago",
            u.five_hour,
            u.seven_day,
            u.age_secs / 60
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"{"version":2,"samples":[
        {"t":1787950000000,"org":"o","u":{"fh":26,"sd":12}},
        {"t":1787953450000,"org":"o","u":{"fh":32,"sd":12}},
        {"t":1787951000000,"org":"o","u":{"fh":29,"sd":12}}
    ]}"#;

    #[test]
    fn takes_the_newest_sample_not_the_last() {
        let newest = parse(DOC).unwrap();
        assert_eq!(newest.five_hour, 32);
        assert_eq!(newest.seven_day, 12);
        assert_eq!(newest.at, 1787953450);
    }

    #[test]
    fn a_sample_with_no_windows_is_skipped() {
        let doc = r#"{"samples":[{"t":1787953450000,"org":"o"},
                                 {"t":1787950000000,"org":"o","u":{"fh":26,"sd":12}}]}"#;
        assert_eq!(parse(doc).unwrap().five_hour, 26);
    }

    #[test]
    fn an_empty_history_reads_as_nothing() {
        assert!(parse(r#"{"version":2,"samples":[]}"#).is_none());
        assert!(parse("not json").is_none());
    }

    #[test]
    fn percentages_are_clamped() {
        let doc = r#"{"samples":[{"t":1000,"org":"o","u":{"fh":140.6,"sd":-3}}]}"#;
        let newest = parse(doc).unwrap();
        assert_eq!(newest.five_hour, 100);
        assert_eq!(newest.seven_day, 0);
    }

    #[test]
    fn a_fractional_percentage_rounds() {
        let doc = r#"{"samples":[{"t":1000,"org":"o","u":{"fh":31.6,"sd":11.2}}]}"#;
        let newest = parse(doc).unwrap();
        assert_eq!(newest.five_hour, 32);
        assert_eq!(newest.seven_day, 11);
    }
}
