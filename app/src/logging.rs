use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::Subscriber;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::Context;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{reload, EnvFilter, Layer, Registry};

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: SystemTime,
    pub level: tracing::Level,
    pub target: String,
    pub message: String,
}

impl LogEntry {
    pub fn timestamp_secs(&self) -> u64 {
        self.timestamp
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    }

    pub fn format_line(&self) -> String {
        format!(
            "[{:>10}] {:<5} {:<10} {}",
            self.timestamp_secs(),
            self.level.as_str(),
            self.target,
            self.message
        )
    }
}

#[derive(Debug, Clone)]
pub struct LogStore {
    inner: Arc<Mutex<VecDeque<LogEntry>>>,
    capacity: usize,
}

impl LogStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            capacity,
        }
    }

    pub fn push(&self, entry: LogEntry) {
        if let Ok(mut guard) = self.inner.lock() {
            if guard.len() >= self.capacity {
                guard.pop_front();
            }
            guard.push_back(entry);
        }
    }

    pub fn snapshot(&self) -> Vec<LogEntry> {
        if let Ok(guard) = self.inner.lock() {
            return guard.iter().cloned().collect();
        }
        Vec::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub const ALL: [LogLevel; 5] = [
        LogLevel::Error,
        LogLevel::Warn,
        LogLevel::Info,
        LogLevel::Debug,
        LogLevel::Trace,
    ];

    pub fn to_level_filter(self) -> LevelFilter {
        match self {
            LogLevel::Error => LevelFilter::ERROR,
            LogLevel::Warn => LevelFilter::WARN,
            LogLevel::Info => LevelFilter::INFO,
            LogLevel::Debug => LevelFilter::DEBUG,
            LogLevel::Trace => LevelFilter::TRACE,
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Error => f.write_str("Error"),
            LogLevel::Warn => f.write_str("Warn"),
            LogLevel::Info => f.write_str("Info"),
            LogLevel::Debug => f.write_str("Debug"),
            LogLevel::Trace => f.write_str("Trace"),
        }
    }
}

pub type ReloadHandle = reload::Handle<EnvFilter, Registry>;

pub fn init_logging(store: LogStore, level: LogLevel) -> ReloadHandle {
    let env_filter = EnvFilter::default().add_directive(level.to_level_filter().into());
    let (reload_layer, handle) = reload::Layer::new(env_filter);

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_ansi(false);

    let subscriber = Registry::default()
        .with(reload_layer)
        .with(LogCaptureLayer::new(store))
        .with(fmt_layer);

    let _ = tracing::subscriber::set_global_default(subscriber);

    handle
}

pub fn apply_log_level(handle: &ReloadHandle, level: LogLevel) {
    let new_filter = EnvFilter::default().add_directive(level.to_level_filter().into());
    let _ = handle.modify(|filter| {
        *filter = new_filter;
    });
}

struct LogCaptureLayer {
    store: LogStore,
}

impl LogCaptureLayer {
    fn new(store: LogStore) -> Self {
        Self { store }
    }
}

impl<S> Layer<S> for LogCaptureLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        let message = visitor.message.unwrap_or_else(|| visitor.fields.join(", "));

        self.store.push(LogEntry {
            timestamp: SystemTime::now(),
            level: *metadata.level(),
            target: metadata.target().to_string(),
            message,
        });
    }
}

#[derive(Default)]
struct FieldVisitor {
    message: Option<String>,
    fields: Vec<String>,
}

impl tracing::field::Visit for FieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        let value = format!("{value:?}");
        if field.name() == "message" {
            self.message = Some(value.trim_matches('"').to_string());
        } else {
            self.fields
                .push(format!("{}={}", field.name(), value.trim_matches('"')));
        }
    }
}
