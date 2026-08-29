use serde::Serialize;
use tauri::AppHandle;

#[cfg(debug_assertions)]
use std::collections::VecDeque;
#[cfg(debug_assertions)]
use std::path::PathBuf;
#[cfg(debug_assertions)]
use std::sync::{Mutex, OnceLock};
#[cfg(debug_assertions)]
use tauri::{Emitter, Manager};

#[cfg(debug_assertions)]
const RING_CAP: usize = 500;

#[derive(Clone, Serialize)]
pub struct LogLine {
    pub ts: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub kind: String,
}

#[derive(Clone, Serialize)]
pub struct Snapshot {
    pub lines: Vec<LogLine>,
    pub crash: Option<String>,
}

#[cfg(debug_assertions)]
static RING: Mutex<VecDeque<LogLine>> = Mutex::new(VecDeque::new());
#[cfg(debug_assertions)]
static CRASH: Mutex<Option<String>> = Mutex::new(None);
#[cfg(debug_assertions)]
static APP: OnceLock<AppHandle> = OnceLock::new();
#[cfg(debug_assertions)]
static CRASH_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn init_tracing() {
    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());

    #[cfg(debug_assertions)]
    {
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .with(UiLayer)
            .init();
        install_panic_hook();
    }

    #[cfg(not(debug_assertions))]
    {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }
}

pub fn attach(app: &AppHandle) {
    #[cfg(debug_assertions)]
    attach_debug(app);
    #[cfg(not(debug_assertions))]
    let _ = app;
}

pub fn snapshot() -> Snapshot {
    #[cfg(debug_assertions)]
    {
        Snapshot {
            lines: ring().iter().cloned().collect(),
            crash: crash_text(),
        }
    }
    #[cfg(not(debug_assertions))]
    {
        Snapshot {
            lines: Vec::new(),
            crash: None,
        }
    }
}

pub fn clear() {
    #[cfg(debug_assertions)]
    {
        ring().clear();
        *crash_slot() = None;
        if let Some(path) = CRASH_PATH.get() {
            let _ = std::fs::remove_file(path);
        }
    }
}

pub fn record_client(message: String) {
    #[cfg(debug_assertions)]
    push(make_line("ERROR", "webview", &message, "client"));
    #[cfg(not(debug_assertions))]
    let _ = message;
}

pub fn record_crash(message: String) {
    #[cfg(debug_assertions)]
    {
        set_crash(message.clone());
        persist_crash(&message);
        push(make_line("ERROR", "panic", &message, "panic"));
    }
    #[cfg(not(debug_assertions))]
    let _ = message;
}

#[cfg(debug_assertions)]
fn attach_debug(app: &AppHandle) {
    let _ = APP.set(app.clone());
    if let Ok(dir) = app.path().app_log_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("last-crash.txt");
        if let Ok(text) = std::fs::read_to_string(&path) {
            let msg = text.trim();
            if !msg.is_empty() {
                set_crash(msg.to_string());
                push(make_line("ERROR", "panic", msg, "panic"));
            }
            let _ = std::fs::remove_file(&path);
        }
        let _ = CRASH_PATH.set(path);
    }
}

#[cfg(debug_assertions)]
fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        record_crash(info.to_string());
        prev(info);
    }));
}

#[cfg(debug_assertions)]
fn ring() -> std::sync::MutexGuard<'static, VecDeque<LogLine>> {
    RING.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(debug_assertions)]
fn crash_slot() -> std::sync::MutexGuard<'static, Option<String>> {
    CRASH.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(debug_assertions)]
fn crash_text() -> Option<String> {
    crash_slot().clone()
}

#[cfg(debug_assertions)]
fn set_crash(message: String) {
    *crash_slot() = Some(message);
}

#[cfg(debug_assertions)]
fn persist_crash(message: &str) {
    if let Some(path) = CRASH_PATH.get() {
        let _ = std::fs::write(path, message);
    }
}

#[cfg(debug_assertions)]
fn make_line(level: &str, target: &str, message: &str, kind: &str) -> LogLine {
    LogLine {
        ts: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
        level: level.to_string(),
        target: target.to_string(),
        message: message.to_string(),
        kind: kind.to_string(),
    }
}

#[cfg(debug_assertions)]
fn push(line: LogLine) {
    {
        let mut ring = ring();
        if ring.len() >= RING_CAP {
            ring.pop_front();
        }
        ring.push_back(line.clone());
    }
    if let Some(app) = APP.get() {
        let _ = app.emit("dev-log", &line);
        if line.kind == "panic" {
            let _ = app.emit("dev-crash", &line.message);
        }
    }
}

#[cfg(debug_assertions)]
struct UiLayer;

#[cfg(debug_assertions)]
impl<S> tracing_subscriber::Layer<S> for UiLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let meta = event.metadata();
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);
        let message = if visitor.message.is_empty() {
            visitor.fields
        } else if visitor.fields.is_empty() {
            visitor.message
        } else {
            format!("{} {}", visitor.message, visitor.fields)
        };
        push(make_line(
            meta.level().as_str(),
            meta.target(),
            &message,
            "log",
        ));
    }
}

#[cfg(debug_assertions)]
#[derive(Default)]
struct FieldVisitor {
    message: String,
    fields: String,
}

#[cfg(debug_assertions)]
impl tracing::field::Visit for FieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let rendered = format!("{value:?}");
        if field.name() == "message" {
            self.message = trim_debug_string(rendered);
        } else {
            append_field(&mut self.fields, field.name(), &rendered);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            append_field(&mut self.fields, field.name(), value);
        }
    }
}

#[cfg(debug_assertions)]
fn trim_debug_string(value: String) -> String {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        value[1..value.len() - 1].to_string()
    } else {
        value
    }
}

#[cfg(debug_assertions)]
fn append_field(buf: &mut String, name: &str, value: &str) {
    if !buf.is_empty() {
        buf.push(' ');
    }
    buf.push_str(name);
    buf.push('=');
    buf.push_str(value);
}
