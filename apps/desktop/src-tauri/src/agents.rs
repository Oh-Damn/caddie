use crate::protocol::{AgentProvider, AgentSession, AgentWindow, AgentsPayload, AgentsSummary};
use chrono::{DateTime, Datelike, Local, TimeZone, Utc};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const SESSION_WINDOW_SECS: i64 = 5 * 3600;
const WEEK_WINDOW_SECS: i64 = 7 * 24 * 3600;
const RETAIN_SECS: i64 = 8 * 24 * 3600;
const ACTIVE_WITHIN_SECS: i64 = 90;

const WAITING_AFTER_SECS: i64 = 8;

const WAITING_UNTIL_SECS: i64 = 4 * 3600;

const W_INPUT: f64 = 1.0;
const W_CACHE_WRITE: f64 = 1.25;
const W_CACHE_READ: f64 = 0.1;
const W_OUTPUT: f64 = 5.0;

const FLOOR_CEILING: f64 = 2_000_000.0;

const MAX_CEILING: f64 = 10_000_000_000.0;

const MAX_LINE: u64 = 8 * 1024 * 1024;

const MAX_EVENTS: usize = 200_000;

pub fn clamp_ceiling(millions: f64) -> Option<f64> {
    if !millions.is_finite() || millions <= 0.0 {
        return None;
    }
    Some((millions * 1_000_000.0).min(MAX_CEILING))
}

#[derive(Debug, Clone)]
struct Event {
    at: i64,
    session: String,
    project: String,
    branch: String,
    model: String,
    weighted: f64,
    tokens: u64,
    sidechain: bool,
}

#[derive(Debug, Clone)]
struct Tail {
    at: i64,
    session: String,
    waiting: bool,
}

#[derive(Default)]
struct Cursor {
    len: u64,
    offset: u64,
}

#[derive(Deserialize)]
struct Usage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

#[derive(Deserialize)]
struct Msg {
    #[serde(default)]
    id: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    role: String,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Line {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    timestamp: String,
    #[serde(default, rename = "sessionId")]
    session_id: String,
    #[serde(default)]
    cwd: String,
    #[serde(default, rename = "gitBranch")]
    git_branch: String,
    #[serde(default, rename = "requestId")]
    request_id: String,
    #[serde(default, rename = "isSidechain")]
    is_sidechain: bool,
    #[serde(default)]
    message: Option<Msg>,
}

pub struct Ledger {
    root: PathBuf,
    cursors: HashMap<PathBuf, Cursor>,
    seen: HashSet<String>,
    events: Vec<Event>,
    tails: HashMap<String, Tail>,

    high_water: f64,
}

impl Ledger {
    pub fn new() -> Self {
        Self::at(claude_projects_dir())
    }

    fn at(root: PathBuf) -> Self {
        Self {
            root,
            cursors: HashMap::new(),
            seen: HashSet::new(),
            events: Vec::new(),
            tails: HashMap::new(),
            high_water: 0.0,
        }
    }

    pub fn refresh(&mut self) {
        let now = Utc::now().timestamp();
        for path in transcripts(&self.root) {
            self.read_file(&path, now);
        }
        let cutoff = now - RETAIN_SECS;
        self.events.retain(|e| e.at >= cutoff);
        if self.events.len() > MAX_EVENTS {
            self.events.sort_by_key(|e| e.at);
            let drop = self.events.len() - MAX_EVENTS;
            self.events.drain(..drop);
        }
        self.tails.retain(|_, t| t.at >= cutoff);
        if self.seen.len() > 200_000 {

            self.seen.clear();
        }
    }

    fn read_file(&mut self, path: &Path, now: i64) {
        let Ok(meta) = std::fs::metadata(path) else {
            return;
        };
        let len = meta.len();

        if !self.cursors.contains_key(path) && older_than_retention(&meta) {
            self.cursors
                .insert(path.to_path_buf(), Cursor { len, offset: len });
            return;
        }
        let cursor = self.cursors.entry(path.to_path_buf()).or_default();
        if len == cursor.len {
            return;
        }
        if len < cursor.len {
            cursor.offset = 0;
        }
        cursor.len = len;
        let start = cursor.offset;
        let Ok(file) = std::fs::File::open(path) else {
            return;
        };
        let mut reader = BufReader::new(file);
        if start > 0 && reader.seek(SeekFrom::Start(start)).is_err() {
            return;
        }
        let mut consumed = start;
        let mut buf = Vec::new();
        loop {
            buf.clear();
            let read = {
                let mut capped = reader.by_ref().take(MAX_LINE);
                capped.read_until(b'\n', &mut buf)
            };
            match read {
                Ok(0) => break,
                Ok(n) => {
                    if buf.last() == Some(&b'\n') {
                        consumed += n as u64;
                        self.ingest(&buf, now);
                        continue;
                    }
                    if n as u64 == MAX_LINE {

                        match skip_line(&mut reader) {
                            Some(rest) => consumed += n as u64 + rest,
                            None => break,
                        }
                        continue;
                    }

                    break;
                }
                Err(_) => break,
            }
        }
        if let Some(cursor) = self.cursors.get_mut(path) {
            cursor.offset = consumed;
        }
    }

    fn ingest(&mut self, raw: &[u8], now: i64) {
        let Ok(line) = serde_json::from_slice::<Line>(raw) else {
            return;
        };
        let Some(at) = parse_ts(&line.timestamp) else {
            return;
        };
        if at < now - RETAIN_SECS {
            return;
        }
        let Some(msg) = line.message.as_ref() else {
            return;
        };
        let project = project_name(&line.cwd);
        if line.kind == "assistant" && !line.is_sidechain {
            let waiting = msg.stop_reason.as_deref() == Some("tool_use");
            self.tails.insert(
                line.session_id.clone(),
                Tail {
                    at,
                    session: line.session_id.clone(),
                    waiting,
                },
            );
        }
        if line.kind == "user" || msg.role == "user" {
            if let Some(tail) = self.tails.get_mut(&line.session_id) {

                tail.waiting = false;
                tail.at = at;
            }
            return;
        }
        let Some(usage) = msg.usage.as_ref() else {
            return;
        };
        let key = if line.request_id.is_empty() {
            format!("{}:{}", line.session_id, msg.id)
        } else {
            line.request_id.clone()
        };
        if !self.seen.insert(key) {
            return;
        }

        let tokens = usage
            .input_tokens
            .saturating_add(usage.cache_creation_input_tokens)
            .saturating_add(usage.cache_read_input_tokens)
            .saturating_add(usage.output_tokens);
        let weighted = usage.input_tokens as f64 * W_INPUT
            + usage.cache_creation_input_tokens as f64 * W_CACHE_WRITE
            + usage.cache_read_input_tokens as f64 * W_CACHE_READ
            + usage.output_tokens as f64 * W_OUTPUT;
        self.events.push(Event {
            at,
            session: line.session_id,
            project,
            branch: line.git_branch,
            model: short_model(&msg.model),
            weighted,
            tokens,
            sidechain: line.is_sidechain,
        });
    }

    fn block_start(&self, now: i64) -> Option<i64> {
        let mut sorted: Vec<i64> = self.events.iter().map(|e| e.at).collect();
        sorted.sort_unstable();
        let mut start = *sorted.first()?;
        for at in sorted {
            if at >= start + SESSION_WINDOW_SECS {
                start = at;
            }
        }
        if now >= start + SESSION_WINDOW_SECS {
            return None;
        }
        Some(start)
    }

    fn calibrate(&mut self, now: i64) {
        let mut sorted: Vec<&Event> = self.events.iter().collect();
        sorted.sort_by_key(|e| e.at);
        let mut start = match sorted.first() {
            Some(e) => e.at,
            None => return,
        };
        let mut acc = 0.0;
        for event in sorted {
            if event.at >= start + SESSION_WINDOW_SECS {

                if start + SESSION_WINDOW_SECS <= now {
                    self.high_water = self.high_water.max(acc);
                }
                start = event.at;
                acc = 0.0;
            }
            acc += event.weighted;
        }
        if start + SESSION_WINDOW_SECS <= now {
            self.high_water = self.high_water.max(acc);
        }
    }

    pub fn payload(&mut self, limits: Limits, approvals: &[ExternalAgent]) -> AgentsPayload {
        let now = Utc::now().timestamp();
        self.calibrate(now);
        let session_ceiling = limits
            .session
            .unwrap_or_else(|| self.high_water.max(FLOOR_CEILING));
        let week_ceiling = limits.week.unwrap_or(session_ceiling * 12.0);

        let block = self.block_start(now);
        let block_from = block.unwrap_or(now);
        let session_used: f64 = self
            .events
            .iter()
            .filter(|e| e.at >= block_from)
            .map(|e| e.weighted)
            .sum();
        let week_from = now - WEEK_WINDOW_SECS;
        let week_used: f64 = self
            .events
            .iter()
            .filter(|e| e.at >= week_from)
            .map(|e| e.weighted)
            .sum();

        let sessions = self.sessions(now);
        let waiting = self
            .tails
            .values()
            .filter(|t| is_waiting(t, now))
            .map(|t| t.session.clone())
            .collect::<Vec<_>>();
        let active = sessions.iter().filter(|s| s.active).count() as u32;

        let plan = crate::plan_usage::read(now);
        let session_window = match plan {
            Some(plan) => AgentWindow {
                label: "Current session".into(),
                used: 0,
                limit: None,
                percent: plan.five_hour,
                resets_at: block.map(|s| iso(s + SESSION_WINDOW_SECS)),
            },
            None => AgentWindow {
                label: "Current block".into(),
                used: session_used.round() as u64,
                limit: Some(session_ceiling.round() as u64),
                percent: percent(session_used, session_ceiling),
                resets_at: block.map(|s| iso(s + SESSION_WINDOW_SECS)),
            },
        };
        let week_window = match plan {
            Some(plan) => AgentWindow {
                label: "Last 7 days".into(),
                used: 0,
                limit: None,
                percent: plan.seven_day,
                resets_at: None,
            },
            None => AgentWindow {
                label: "Last 7 days".into(),
                used: week_used.round() as u64,
                limit: Some(week_ceiling.round() as u64),
                percent: percent(week_used, week_ceiling),
                resets_at: limits.week_anchor.map(|day| iso(next_anchor(now, day))),
            },
        };

        let claude_code = AgentProvider {
            id: "claude-code".into(),
            name: "Claude Code".into(),
            available: !self.events.is_empty() || !self.tails.is_empty(),
            estimated: plan.is_none(),
            active,
            waiting: waiting.len() as u32,
            blocked: None,
            session: Some(session_window),
            week: Some(week_window),
            sessions,
        };

        let mut providers = vec![claude_code];
        providers.extend(approvals.iter().map(external_provider));

        AgentsPayload {
            summary: summarize(&providers),
            providers,
        }
    }

    pub fn forget(&mut self) {
        self.cursors.clear();
        self.seen.clear();
        self.events.clear();
        self.tails.clear();
        self.high_water = 0.0;
    }

    fn sessions(&self, now: i64) -> Vec<AgentSession> {
        let mut by_id: HashMap<&str, AgentSession> = HashMap::new();
        for event in &self.events {
            let entry = by_id
                .entry(event.session.as_str())
                .or_insert_with(|| AgentSession {
                    id: event.session.clone(),
                    project: event.project.clone(),
                    branch: event.branch.clone(),
                    model: event.model.clone(),
                    tokens: 0,
                    weighted: 0,
                    subagent_tokens: 0,
                    last_active: iso(event.at),
                    active: false,
                    waiting: false,
                    detail: String::new(),
                });
            entry.tokens = entry.tokens.saturating_add(event.tokens);
            entry.weighted = entry.weighted.saturating_add(event.weighted.round() as u64);
            if event.sidechain {
                entry.subagent_tokens = entry.subagent_tokens.saturating_add(event.tokens);
            }
            if !event.branch.is_empty() {
                entry.branch = event.branch.clone();
            }
            if !event.model.is_empty() {
                entry.model = event.model.clone();
            }
            if event.at >= parse_ts(&entry.last_active).unwrap_or(0) {
                entry.last_active = iso(event.at);
                entry.active = now - event.at <= ACTIVE_WITHIN_SECS;
            }
        }
        for tail in self.tails.values() {
            if let Some(entry) = by_id.get_mut(tail.session.as_str()) {
                entry.waiting = is_waiting(tail, now);
            }
        }
        let mut out: Vec<AgentSession> = by_id.into_values().collect();
        out.sort_by(|a, b| b.last_active.cmp(&a.last_active));
        out.truncate(20);
        out
    }
}

fn external_provider(agent: &ExternalAgent) -> AgentProvider {
    AgentProvider {
        id: agent.id.clone(),
        name: agent.name.clone(),
        available: agent.running,
        estimated: false,
        active: agent.active,
        waiting: agent.waiting,
        blocked: agent.blocked.clone(),
        session: None,
        week: None,
        sessions: agent.sessions.clone(),
    }
}

pub fn external_only(approvals: &[ExternalAgent]) -> AgentsPayload {
    let providers: Vec<AgentProvider> = approvals.iter().map(external_provider).collect();
    AgentsPayload {
        summary: summarize(&providers),
        providers,
    }
}

pub fn cursor_session(session: &crate::cursor::Session) -> AgentSession {
    let mut detail = String::new();
    if session.lines_added > 0 || session.lines_removed > 0 {
        detail = format!("+{} -{}", session.lines_added, session.lines_removed);
    }
    if session.unread {
        detail = if detail.is_empty() {
            "unread".into()
        } else {
            format!("{detail} · unread")
        };
    }
    AgentSession {
        id: session.id.clone(),
        project: session.project.clone(),
        branch: session.branch.clone(),
        model: session.mode.clone(),
        tokens: 0,
        weighted: 0,
        subagent_tokens: 0,
        last_active: iso(session.last_active),
        active: session.waiting || session.unread,
        waiting: session.waiting,
        detail,
    }
}

#[derive(Debug, Clone)]
pub struct ExternalAgent {
    pub id: String,
    pub name: String,
    pub running: bool,
    pub waiting: u32,
    pub active: u32,

    pub blocked: Option<String>,
    pub sessions: Vec<AgentSession>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Limits {
    pub session: Option<f64>,
    pub week: Option<f64>,

    pub week_anchor: Option<u32>,
}

pub fn summarize(providers: &[AgentProvider]) -> AgentsSummary {
    let peak = providers
        .iter()
        .filter_map(|p| p.session.as_ref())
        .map(|w| w.percent)
        .max()
        .unwrap_or(0);
    AgentsSummary {
        percent: peak,
        waiting: providers.iter().map(|p| p.waiting).sum(),
        active: providers.iter().map(|p| p.active).sum(),
    }
}

fn is_waiting(tail: &Tail, now: i64) -> bool {
    if !tail.waiting {
        return false;
    }
    let age = now - tail.at;
    (WAITING_AFTER_SECS..WAITING_UNTIL_SECS).contains(&age)
}

fn percent(used: f64, ceiling: f64) -> u8 {
    if ceiling <= 0.0 {
        return 0;
    }
    ((used / ceiling) * 100.0).round().clamp(0.0, 100.0) as u8
}

fn skip_line(reader: &mut BufReader<std::fs::File>) -> Option<u64> {
    let mut sink = Vec::new();
    let mut skipped = 0u64;
    loop {
        sink.clear();
        let mut capped = reader.by_ref().take(MAX_LINE);
        match capped.read_until(b'\n', &mut sink) {
            Ok(0) => return None,
            Ok(n) => {
                skipped += n as u64;
                if sink.last() == Some(&b'\n') {
                    return Some(skipped);
                }
                if (n as u64) < MAX_LINE {

                    return None;
                }
            }
            Err(_) => return None,
        }
    }
}

fn older_than_retention(meta: &std::fs::Metadata) -> bool {
    let Ok(modified) = meta.modified() else {
        return false;
    };
    let Ok(age) = SystemTime::now().duration_since(modified) else {
        return false;
    };
    age.as_secs() as i64 > RETAIN_SECS
}

fn claude_projects_dir() -> PathBuf {

    match std::env::var_os("HOME").filter(|h| !h.is_empty()) {
        Some(home) => PathBuf::from(home).join(".claude/projects"),
        None => PathBuf::new(),
    }
}

pub fn codex_installed() -> bool {
    match std::env::var_os("HOME").filter(|h| !h.is_empty()) {
        Some(home) => PathBuf::from(home).join(".codex").is_dir(),
        None => false,
    }
}

fn transcripts(root: &Path) -> Vec<PathBuf> {
    let Ok(dirs) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for dir in dirs.flatten() {
        let Ok(files) = std::fs::read_dir(dir.path()) else {
            continue;
        };
        for file in files.flatten() {
            let path = file.path();
            if path.extension().is_some_and(|e| e == "jsonl") {
                out.push(path);
            }
        }
    }
    out
}

const GENERIC_DIRS: &[&str] = &[
    "src",
    "lib",
    "app",
    "apps",
    "packages",
    "server",
    "client",
    "web",
    "api",
    "backend",
    "frontend",
    "src-tauri",
];

pub fn project_name(cwd: &str) -> String {
    let path = Path::new(cwd);
    let Some(last) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
        return String::new();
    };
    if !GENERIC_DIRS.contains(&last.as_str()) {
        return last;
    }
    match path.parent().and_then(|p| p.file_name()) {
        Some(parent) => format!("{}/{last}", parent.to_string_lossy()),
        None => last,
    }
}

fn short_model(model: &str) -> String {
    model
        .strip_prefix("claude-")
        .unwrap_or(model)
        .split('-')
        .take(2)
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_ts(raw: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp())
}

fn iso(secs: i64) -> String {
    Utc.timestamp_opt(secs, 0)
        .single()
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default()
}

fn next_anchor(now: i64, weekday: u32) -> i64 {
    let Some(local) = Local.timestamp_opt(now, 0).single() else {
        return now;
    };
    let today = local.weekday().num_days_from_monday();
    let mut ahead = (weekday + 7 - today) % 7;
    if ahead == 0 {
        ahead = 7;
    }
    let midnight = local
        .date_naive()
        .succ_opt()
        .and_then(|d| d.checked_add_days(chrono::Days::new(u64::from(ahead) - 1)))
        .and_then(|d| d.and_hms_opt(0, 0, 0));
    match midnight.and_then(|naive| Local.from_local_datetime(&naive).single()) {
        Some(dt) => dt.timestamp(),
        None => now,
    }
}

#[cfg(test)]
#[test]
#[ignore]
fn bench_ledger() {
    let mut ledger = Ledger::new();
    let t0 = std::time::Instant::now();
    ledger.refresh();
    let cold = t0.elapsed();
    let t1 = std::time::Instant::now();
    ledger.refresh();
    let warm = t1.elapsed();
    println!(
        "cold={:?} warm={:?} events={} seen={} tails={}",
        cold,
        warm,
        ledger.events.len(),
        ledger.seen.len(),
        ledger.tails.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(ts: &str, session: &str, req: &str, out: u64) -> String {
        format!(
            r#"{{"type":"assistant","timestamp":"{ts}","sessionId":"{session}","cwd":"/Users/x/Desktop/demo","gitBranch":"main","requestId":"{req}","message":{{"id":"m1","model":"claude-opus-5","role":"assistant","stop_reason":"end_turn","usage":{{"input_tokens":10,"cache_creation_input_tokens":100,"cache_read_input_tokens":1000,"output_tokens":{out}}}}}}}"#
        )
    }

    fn ledger_with(lines: &[String]) -> Ledger {
        let mut ledger = Ledger::at(PathBuf::from("/nonexistent"));
        let now = Utc::now().timestamp();
        for raw in lines {
            ledger.ingest(raw.as_bytes(), now);
        }
        ledger
    }

    #[test]
    fn walks_past_an_oversized_line() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("caddie-ledger-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.jsonl");
        let ts = Utc::now().to_rfc3339();
        {
            let mut f = std::fs::File::create(&path).unwrap();

            let huge = "x".repeat((MAX_LINE as usize) + 4096);
            writeln!(f, r#"{{"type":"user","timestamp":"{ts}","blob":"{huge}"}}"#).unwrap();
            writeln!(f, "{}", line(&ts, "s1", "req_a", 100)).unwrap();
        }
        let mut ledger = Ledger::at(dir.clone());
        ledger.read_file(&path, Utc::now().timestamp());
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(
            ledger.events.len(),
            1,
            "the record after the huge line is read"
        );
        assert_eq!(ledger.events[0].session, "s1");
    }

    #[test]
    fn clamps_a_hostile_ceiling() {

        assert_eq!(clamp_ceiling(f64::INFINITY), None);
        assert_eq!(clamp_ceiling(f64::NAN), None);
        assert_eq!(clamp_ceiling(-5.0), None);
        assert_eq!(clamp_ceiling(0.0), None);
        assert_eq!(clamp_ceiling(10.0), Some(10_000_000.0));
        assert_eq!(clamp_ceiling(1e9), Some(MAX_CEILING));
    }

    #[test]
    fn dedupes_repeated_request_ids() {
        let ts = Utc::now().to_rfc3339();
        let ledger = ledger_with(&[
            line(&ts, "s1", "req_a", 50),
            line(&ts, "s1", "req_a", 50),
            line(&ts, "s1", "req_b", 50),
        ]);
        assert_eq!(ledger.events.len(), 2);
    }

    #[test]
    fn weights_cache_reads_below_output() {
        let ts = Utc::now().to_rfc3339();
        let ledger = ledger_with(&[line(&ts, "s1", "req_a", 100)]);
        let event = &ledger.events[0];
        assert_eq!(event.tokens, 1210);

        assert!((event.weighted - 735.0).abs() < 0.001);
    }

    #[test]
    fn rolls_sessions_up_by_id() {
        let ts = Utc::now().to_rfc3339();
        let ledger = ledger_with(&[
            line(&ts, "s1", "req_a", 100),
            line(&ts, "s1", "req_b", 100),
            line(&ts, "s2", "req_c", 100),
        ]);
        let sessions = ledger.sessions(Utc::now().timestamp());
        assert_eq!(sessions.len(), 2);
        let first = sessions.iter().find(|s| s.id == "s1").unwrap();
        assert_eq!(first.tokens, 2420);
        assert_eq!(first.project, "demo");
        assert!(first.active);
    }

    #[test]
    fn block_lapses_after_five_hours() {
        let now = Utc::now();
        let old = (now - chrono::Duration::hours(9)).to_rfc3339();
        let recent = (now - chrono::Duration::minutes(30)).to_rfc3339();
        let ledger = ledger_with(&[
            line(&old, "s1", "req_a", 100),
            line(&recent, "s1", "req_b", 100),
        ]);
        let start = ledger.block_start(now.timestamp()).unwrap();
        assert!(now.timestamp() - start < SESSION_WINDOW_SECS);
        assert!(start > (now - chrono::Duration::hours(1)).timestamp());
    }

    #[test]
    fn tool_use_tail_reads_as_waiting() {
        let now = Utc::now().timestamp();
        let tail = Tail {
            at: now - 30,
            session: "s1".into(),
            waiting: true,
        };
        assert!(is_waiting(&tail, now));
        let fresh = Tail {
            at: now,
            ..tail.clone()
        };
        assert!(!is_waiting(&fresh, now));
        let stale = Tail {
            at: now - WAITING_UNTIL_SECS - 1,
            ..tail
        };
        assert!(!is_waiting(&stale, now));
    }
}
