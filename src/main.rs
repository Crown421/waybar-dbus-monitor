mod cli;
mod dbus_listener;
mod error;
mod retry;

use clap::Parser;
use dbus_listener::DBusListener;
use error::AppError;
use log::debug;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), AppError> {
    // Initialize logger
    env_logger::init();

    let config = cli::Config::parse();

    // Validate configuration
    if let Err(e) = config.validate() {
        eprintln!("Configuration error: {}", e);
        std::process::exit(1);
    }

    debug!("Starting waybar-dbus-monitor");
    debug!("Interface: {}", config.interface);
    debug!("Monitor: {}", config.monitor);
    debug!("Type handler: {:?}", config.type_handler);

    if let Some(status) = &config.status {
        debug!("Status configuration: {}", status);
        if let Ok(Some(status_config)) = config.parse_status() {
            debug!("  Service: {}", status_config.service);
            debug!("  Object path: {}", status_config.object_path);
            debug!("  Interface: {}", status_config.interface);
            debug!("  Property: {}", status_config.property);
        }
    }

    match &config.type_handler {
        cli::TypeHandler::Boolean {
            return_true,
            return_false,
        } => {
            debug!("Boolean handler configured:");
            debug!("  Return on true: '{}'", return_true);
            debug!("  Return on false: '{}'", return_false);
        }
    }

    let listener = DBusListener::new(config);

    // Handle errors by printing error codes for waybar
    if let Err(error) = listener.listen().await {
        error.print_error_code();
        eprintln!("Fatal error: {}", error);
        return Err(error);
    }

    Ok(())
}
