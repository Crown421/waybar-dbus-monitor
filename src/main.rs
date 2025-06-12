mod cli;
mod dbus_listener;

use clap::Parser;
use dbus_listener::DBusListener;
use log::debug;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // Create and start the D-Bus listener
    let listener = DBusListener::new(config.interface, config.member, config.type_handler);

    // Start listening (this will run indefinitely until an error occurs)
    if let Err(e) = listener.listen().await {
        eprintln!("D-Bus listener error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
