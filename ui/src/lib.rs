pub mod app;
mod executor;
pub mod logging;
pub mod sync;

use iced::{Color, Theme, theme};

pub use app::{Flags, PrintCountApp};
pub use logging::{LogEntry, LogLevel, LogStore, ReloadHandle, apply_log_level, init_logging};

pub type UiResult = iced::Result;

fn application_theme(_state: &PrintCountApp) -> Theme {
    Theme::Light
}

fn application_style(_state: &PrintCountApp, theme: &Theme) -> theme::Style {
    theme::Style {
        background_color: Color::TRANSPARENT,
        text_color: theme.extended_palette().background.base.text,
    }
}

pub fn run(flags: Flags) -> UiResult {
    iced::application(
        move || PrintCountApp::new(flags.clone()),
        PrintCountApp::update,
        PrintCountApp::view,
    )
    .title(PrintCountApp::title)
    .subscription(PrintCountApp::subscription)
    .theme(application_theme)
    .style(application_style)
    .decorations(false)
    .transparent(true)
    .executor::<crate::executor::StackSizedTokioExecutor>()
    .run()
}
