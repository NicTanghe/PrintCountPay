pub mod app;
mod executor;
pub mod logging;

use iced::Application;

pub use app::{Flags, PrintCountApp};
pub use logging::{
    apply_log_level, init_logging, LogEntry, LogLevel, LogStore, ReloadHandle,
};

pub type UiResult = iced::Result;

pub fn run(flags: Flags) -> UiResult {
    PrintCountApp::run(iced::Settings::with_flags(flags))
}
