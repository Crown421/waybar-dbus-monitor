use clap::{Parser, Subcommand};
use log::warn;
use zbus::zvariant;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// D-Bus interface to monitor
    #[arg(long)]
    pub interface: String,

    /// D-Bus member (signal/method) to monitor
    #[arg(long)]
    pub member: String,

    /// Type handler for the monitored data
    #[command(subcommand)]
    pub type_handler: TypeHandler,
}

#[derive(Subcommand, Debug)]
pub enum TypeHandler {
    /// Monitor a boolean value
    Boolean {
        /// String to return when value is true
        #[arg(long, default_value = "true")]
        return_true: String,

        /// String to return when value is false
        #[arg(long, default_value = "false")]
        return_false: String,
    },
    // Future expansion:
    // String { ... },
    // Integer { ... },
}

impl TypeHandler {
    /// Process the raw D-Bus data and return the string to print
    pub fn process(&self, body: &zvariant::Value) -> Option<String> {
        match self {
            TypeHandler::Boolean {
                return_true,
                return_false,
            } => {
                if let zvariant::Value::Bool(b) = body {
                    let output = if *b { return_true } else { return_false };
                    Some(output.clone())
                } else {
                    // Log an error if the type doesn't match
                    warn!("Expected boolean, got {:?}", body);
                    None
                }
            } // Future:
              // TypeHandler::String { prefix, suffix } => { ... }
        }
    }
}
