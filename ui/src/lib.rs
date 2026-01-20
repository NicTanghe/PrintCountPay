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
    let mut settings = iced::Settings::with_flags(flags);
    settings.window.decorations = false;
    PrintCountApp::run(settings)
}
