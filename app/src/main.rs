#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod single_instance;

use tracing::Level;

use printcountpay_core::targets;
use printcountpay_ui::logging::{LogLevel, LogStore, init_logging};
use printcountpay_ui::{Flags, UiResult, run};

fn main() -> UiResult {
    let _instance_guard = match single_instance::SingleInstanceGuard::acquire() {
        Ok(Some(guard)) => guard,
        Ok(None) => {
            single_instance::show_already_running_message();
            return Ok(());
        }
        Err(error) => {
            single_instance::show_startup_error(&error.to_string());
            return Ok(());
        }
    };

    let log_store = LogStore::new(2000);
    let reload_handle = init_logging(log_store.clone(), LogLevel::Info);

    tracing::info!(target: targets::UI, "PrintCount starting");
    tracing::info!(target: targets::DISCOVERY, "Discovery target ready");
    tracing::info!(target: targets::SNMP, "SNMP target ready");
    tracing::info!(target: targets::POLLING, "Polling target ready");
    tracing::info!(target: targets::STORAGE, "Storage target ready");
    tracing::event!(target: targets::UI, Level::DEBUG, "Logging infrastructure online");

    run(Flags {
        log_store,
        reload_handle,
    })
}
