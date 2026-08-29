use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

const BUSY_TIMEOUT: Duration = Duration::from_millis(200);
const ACTIVE_WITHIN_SECS: i64 = 90;

const WAITING_MAX_AGE_SECS: i64 = 12 * 3600;

static REPORTED: Mutex<Option<String>> = Mutex::new(None);

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub project: String,
    pub branch: String,
    pub mode: String,
    pub last_active: i64,
    pub waiting: bool,
    pub unread: bool,
    pub lines_added: u32,
    pub lines_removed: u32,
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub sessions: Vec<Session>,
    pub waiting: u32,
    pub active: u32,
}

#[derive(Deserialize)]
struct Header {
    #[serde(default, rename = "composerId")]
    composer_id: String,
    #[serde(default, rename = "lastUpdatedAt")]
    last_updated_at: i64,
    #[serde(default, rename = "unifiedMode")]
    unified_mode: String,
    #[serde(default, rename = "isDraft")]
    is_draft: bool,
    #[serde(default, rename = "hasBlockingPendingActions")]
    has_blocking_pending_actions: bool,
    #[serde(default, rename = "hasPendingPlan")]
    has_pending_plan: bool,
    #[serde(default, rename = "hasUnreadMessages")]
    has_unread_messages: bool,
    #[serde(default, rename = "totalLinesAdded")]
    total_lines_added: u32,
    #[serde(default, rename = "totalLinesRemoved")]
    total_lines_removed: u32,
    #[serde(default, rename = "workspaceIdentifier")]
    workspace: Option<Workspace>,
    #[serde(default, rename = "trackedGitRepos")]
    tracked_git_repos: Vec<TrackedRepo>,
}

#[derive(Deserialize)]
struct Workspace {
    #[serde(default)]
    uri: Option<WorkspaceUri>,
}

#[derive(Deserialize)]
struct WorkspaceUri {
    #[serde(default, rename = "fsPath")]
    fs_path: String,
}

#[derive(Deserialize)]
struct TrackedRepo {
    #[serde(default)]
    branches: Vec<Branch>,
}

#[derive(Deserialize)]
struct Branch {
    #[serde(default, rename = "branchName")]
    branch_name: String,
    #[serde(default, rename = "lastInteractionAt")]
    last_interaction_at: i64,
}

pub fn read(limit: usize, now: i64) -> Option<State> {
    let path = db_path()?;

    let conn = match Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(conn) => conn,
        Err(err) => {
            report(&format!("open: {err}"));
            return None;
        }
    };
    if conn.busy_timeout(BUSY_TIMEOUT).is_err() {
        return None;
    }
    let mut stmt = conn
        .prepare(
            "select value from composerHeaders \
             where isSubagent = 0 and isArchived = 0 \
             order by recency desc limit ?1",
        )
        .map_err(|err| report(&format!("prepare: {err}")))
        .ok()?;
    let rows = stmt
        .query_map([limit as i64], |row| row.get::<_, String>(0))
        .map_err(|err| report(&format!("query: {err}")))
        .ok()?;

    let mut state = State::default();
    for raw in rows.flatten() {
        let Ok(header) = serde_json::from_str::<Header>(&raw) else {
            continue;
        };

        if header.is_draft || header.composer_id.is_empty() {
            continue;
        }
        let Some(session) = session_from(header, now) else {
            continue;
        };
        if now - session.last_active <= ACTIVE_WITHIN_SECS {
            state.active += 1;
        }
        state.sessions.push(session);
    }
    drop_finished_runs(&conn, &mut state.sessions);
    state.waiting = state.sessions.iter().filter(|s| s.waiting).count() as u32;
    clear_report();
    Some(state)
}

fn drop_finished_runs(conn: &Connection, sessions: &mut [Session]) {
    if !sessions.iter().any(|s| s.waiting) {
        return;
    }
    let Ok(mut stmt) =
        conn.prepare("select json_extract(value, '$.status') from cursorDiskKV where key = ?1")
    else {
        return;
    };
    for session in sessions.iter_mut().filter(|s| s.waiting) {
        let key = format!("composerData:{}", session.id);
        let status: Option<String> = stmt
            .query_row([&key], |row| row.get::<_, Option<String>>(0))
            .ok()
            .flatten();
        if status.as_deref().is_some_and(run_is_over) {
            session.waiting = false;
        }
    }
}

fn run_is_over(status: &str) -> bool {
    matches!(status, "aborted" | "completed")
}

fn session_from(header: Header, now: i64) -> Option<Session> {
    let last_active = header.last_updated_at / 1000;
    if last_active <= 0 || last_active > now + 3600 {
        return None;
    }
    let project = header
        .workspace
        .as_ref()
        .and_then(|w| w.uri.as_ref())
        .map(|uri| crate::agents::project_name(&uri.fs_path))
        .unwrap_or_default();
    let branch = header
        .tracked_git_repos
        .iter()
        .flat_map(|repo| repo.branches.iter())
        .max_by_key(|b| b.last_interaction_at)
        .map(|b| b.branch_name.clone())
        .unwrap_or_default();
    Some(Session {
        id: header.composer_id,
        project,
        branch,
        mode: header.unified_mode,
        last_active,

        waiting: (header.has_blocking_pending_actions || header.has_pending_plan)
            && now - last_active <= WAITING_MAX_AGE_SECS,
        unread: header.has_unread_messages,
        lines_added: header.total_lines_added,
        lines_removed: header.total_lines_removed,
    })
}

fn db_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").filter(|h| !h.is_empty())?;
    Some(
        PathBuf::from(home)
            .join("Library/Application Support/Cursor/User/globalStorage/state.vscdb"),
    )
}

fn report(message: &str) {
    let mut last = REPORTED.lock().unwrap_or_else(|e| e.into_inner());
    if last.as_deref() == Some(message) {
        return;
    }
    *last = Some(message.to_string());
    tracing::debug!("cursor headers unreadable, {message}");
}

fn clear_report() {
    *REPORTED.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

#[cfg(test)]
#[test]
#[ignore]
fn dump_cursor() {
    let now = chrono::Utc::now().timestamp();
    match read(20, now) {
        None => println!("unreadable (Cursor closed?)"),
        Some(state) => {
            println!(
                "waiting={} active={} rows={}",
                state.waiting,
                state.active,
                state.sessions.len()
            );
            for s in state.sessions.iter().take(8) {
                println!(
                    "  {:<22} {:<10} {:<7} wait={} unread={} +{}/-{} {}s ago",
                    s.project,
                    s.branch,
                    s.mode,
                    s.waiting,
                    s.unread,
                    s.lines_added,
                    s.lines_removed,
                    now - s.last_active
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(json: &str) -> Option<Session> {
        let header: Header = serde_json::from_str(json).unwrap();
        session_from(header, 1_787_940_000)
    }

    #[test]
    fn reads_project_and_branch() {
        let s = header(
            r#"{"composerId":"c1","lastUpdatedAt":1787912876009,"unifiedMode":"agent",
                "workspaceIdentifier":{"id":"x","uri":{"fsPath":"/Users/x/Desktop/min-it/consumer-web"}},
                "trackedGitRepos":[{"repoPath":"/p","branches":[
                  {"branchName":"main","lastInteractionAt":1},
                  {"branchName":"stage","lastInteractionAt":9}]}]}"#,
        )
        .unwrap();
        assert_eq!(s.project, "consumer-web");
        assert_eq!(s.branch, "stage");
        assert_eq!(s.mode, "agent");
        assert!(!s.waiting);
    }

    #[test]
    fn a_pending_plan_counts_as_waiting() {
        let s =
            header(r#"{"composerId":"c1","lastUpdatedAt":1787912876009,"hasPendingPlan":true}"#)
                .unwrap();
        assert!(s.waiting);
    }

    #[test]
    fn blocking_actions_count_as_waiting() {
        let s = header(
            r#"{"composerId":"c1","lastUpdatedAt":1787912876009,"hasBlockingPendingActions":true}"#,
        )
        .unwrap();
        assert!(s.waiting);
    }

    #[test]
    fn missing_fields_do_not_fail_the_row() {
        let s = header(r#"{"composerId":"c1","lastUpdatedAt":1787912876009}"#).unwrap();
        assert_eq!(s.project, "");
        assert_eq!(s.branch, "");
        assert!(!s.unread);
    }

    fn db_with(rows: &[(&str, &str)]) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("create table cursorDiskKV (key text primary key, value text)", [])
            .unwrap();
        for (id, status) in rows {
            conn.execute(
                "insert into cursorDiskKV (key, value) values (?1, ?2)",
                [
                    &format!("composerData:{id}"),
                    &format!(r#"{{"status":"{status}"}}"#),
                ],
            )
            .unwrap();
        }
        conn
    }

    fn waiting_session(id: &str) -> Session {
        Session {
            id: id.into(),
            project: String::new(),
            branch: String::new(),
            mode: String::new(),
            last_active: 1_787_950_933,
            waiting: true,
            unread: false,
            lines_added: 0,
            lines_removed: 0,
        }
    }

    #[test]
    fn an_aborted_run_is_not_waiting() {

        let conn = db_with(&[("a", "aborted")]);
        let mut sessions = vec![waiting_session("a")];
        drop_finished_runs(&conn, &mut sessions);
        assert!(!sessions[0].waiting);
    }

    #[test]
    fn a_completed_run_is_not_waiting() {
        let conn = db_with(&[("a", "completed")]);
        let mut sessions = vec![waiting_session("a")];
        drop_finished_runs(&conn, &mut sessions);
        assert!(!sessions[0].waiting);
    }

    #[test]
    fn a_running_conversation_stays_waiting() {
        let conn = db_with(&[("a", "none")]);
        let mut sessions = vec![waiting_session("a")];
        drop_finished_runs(&conn, &mut sessions);
        assert!(sessions[0].waiting);
    }

    #[test]
    fn a_body_that_cannot_be_read_stays_waiting() {

        let conn = db_with(&[]);
        let mut sessions = vec![waiting_session("missing")];
        drop_finished_runs(&conn, &mut sessions);
        assert!(sessions[0].waiting);
    }

    #[test]
    fn only_the_finished_one_is_cleared() {
        let conn = db_with(&[("a", "aborted"), ("b", "none")]);
        let mut sessions = vec![waiting_session("a"), waiting_session("b")];
        drop_finished_runs(&conn, &mut sessions);
        assert!(!sessions[0].waiting);
        assert!(sessions[1].waiting);
    }

    #[test]
    fn stale_flags_do_not_count_as_waiting() {
        let old = (1_787_940_000i64 - WAITING_MAX_AGE_SECS - 60) * 1000;
        let s = header(&format!(
            r#"{{"composerId":"c1","lastUpdatedAt":{old},"hasBlockingPendingActions":true}}"#
        ))
        .unwrap();
        assert!(!s.waiting);
    }

    #[test]
    fn drops_rows_with_no_timestamp() {
        assert!(header(r#"{"composerId":"c1"}"#).is_none());
    }
}
