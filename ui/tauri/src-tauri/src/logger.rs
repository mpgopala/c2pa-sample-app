use serde::Serialize;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;
use tracing::{field::{Field, Visit}, Level, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

// ── public log entry types ────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LogEntry {
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    /// Unix timestamp in milliseconds.
    pub ts_ms: u64,
}

// ── runtime level filter ──────────────────────────────────────────────────────
// 0=Error 1=Warn 2=Info 3=Debug 4=Trace

static MIN_LEVEL: AtomicU8 = AtomicU8::new(4); // default: Trace

fn level_to_u8(level: &LogLevel) -> u8 {
    match level {
        LogLevel::Error => 0,
        LogLevel::Warn  => 1,
        LogLevel::Info  => 2,
        LogLevel::Debug => 3,
        LogLevel::Trace => 4,
    }
}

pub fn set_log_level(level: &LogLevel) {
    MIN_LEVEL.store(level_to_u8(level), Ordering::Relaxed);
}

// ── global ring buffer ────────────────────────────────────────────────────────

const MAX_ENTRIES: usize = 500;

static LOG_BUFFER: Mutex<Vec<LogEntry>> = Mutex::new(Vec::new());

pub fn drain_logs() -> Vec<LogEntry> {
    let mut buf = LOG_BUFFER.lock().unwrap();
    buf.drain(..).collect()
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn push_entry(entry: LogEntry) {
    let mut buf = LOG_BUFFER.lock().unwrap();
    if buf.len() >= MAX_ENTRIES {
        buf.remove(0);
    }
    buf.push(entry);
}

// ── tracing Layer ─────────────────────────────────────────────────────────────

struct MessageVisitor(String);

impl Visit for MessageVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_string();
        }
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{value:?}");
        }
    }
}

pub struct UiLogLayer;

impl<S> Layer<S> for UiLogLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let level = match *event.metadata().level() {
            Level::ERROR => LogLevel::Error,
            Level::WARN  => LogLevel::Warn,
            Level::INFO  => LogLevel::Info,
            Level::DEBUG => LogLevel::Debug,
            Level::TRACE => LogLevel::Trace,
        };

        if level_to_u8(&level) > MIN_LEVEL.load(Ordering::Relaxed) {
            return;
        }

        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);

        let target = event.metadata().target();
        if target.starts_with("wry")
            || target.starts_with("tao")
            || target.starts_with("tokio")
            || target.starts_with("hyper")
        {
            return;
        }

        push_entry(LogEntry {
            level,
            target: target.to_string(),
            message: visitor.0,
            ts_ms: now_ms(),
        });
    }
}
