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

    debug!("Starting waybar-dbus-monitor");
    debug!("Interface: {}", config.interface);
    debug!("Member: {}", config.member);
    debug!("Type handler: {:?}", config.type_handler);

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

    let listener = DBusListener::new(config.interface, config.member, config.type_handler);

    // Handle errors by printing error codes for waybar
    if let Err(error) = listener.listen().await {
        error.print_error_code();
        eprintln!("Fatal error: {}", error);
        return Err(error);
    }

    Ok(())
}
