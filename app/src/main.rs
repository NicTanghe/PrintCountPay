mod app;
mod logging;

use iced::{Application, Settings};
use tracing::Level;

use printcountpay_core::targets;

use crate::app::{Flags, PrintCountApp};
use crate::logging::{init_logging, LogLevel, LogStore};

fn main() -> iced::Result {
    let log_store = LogStore::new(2000);
    let reload_handle = init_logging(log_store.clone(), LogLevel::Info);

    tracing::info!(target: targets::UI, "PrintCount starting");
    tracing::info!(target: targets::DISCOVERY, "Discovery target ready");
    tracing::info!(target: targets::SNMP, "SNMP target ready");
    tracing::info!(target: targets::POLLING, "Polling target ready");
    tracing::info!(target: targets::STORAGE, "Storage target ready");
    tracing::event!(target: targets::UI, Level::DEBUG, "Logging infrastructure online");

    PrintCountApp::run(Settings::with_flags(Flags {
        log_store,
        reload_handle,
    }))
}
