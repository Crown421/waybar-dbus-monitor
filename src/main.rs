mod cli;
mod dbus_listener;

use clap::Parser;
use dbus_listener::DBusListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = cli::Config::parse();

    eprintln!("Starting waybar-dbus-monitor");
    eprintln!("Interface: {}", config.interface);
    eprintln!("Member: {}", config.member);
    eprintln!("Type handler: {:?}", config.type_handler);

    match &config.type_handler {
        cli::TypeHandler::Boolean {
            return_true,
            return_false,
        } => {
            eprintln!("Boolean handler configured:");
            eprintln!("  Return on true: '{}'", return_true);
            eprintln!("  Return on false: '{}'", return_false);
        }
    }

    // Create and start the D-Bus listener
    let listener = DBusListener::new(config.interface, config.member);

    // Start listening (this will run indefinitely until an error occurs)
    if let Err(e) = listener.listen().await {
        eprintln!("D-Bus listener error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
