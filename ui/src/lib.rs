pub mod app;
mod executor;
pub mod logging;

pub use app::{Flags, PrintCountApp};
pub use logging::{LogEntry, LogLevel, LogStore, ReloadHandle, apply_log_level, init_logging};

pub type UiResult = iced::Result;

pub fn run(flags: Flags) -> UiResult {
    iced::application(
        move || PrintCountApp::new(flags.clone()),
        PrintCountApp::update,
        PrintCountApp::view,
    )
    .title(PrintCountApp::title)
    .subscription(PrintCountApp::subscription)
    .decorations(false)
    .executor::<crate::executor::StackSizedTokioExecutor>()
    .run()
}
