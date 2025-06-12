use clap::{Parser, Subcommand};
use log::warn;
use zbus::zvariant;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// D-Bus interface and service name to monitor
    #[arg(long)]
    pub interface: String,

    /// D-Bus member (signal/method) to monitor
    #[arg(long)]
    pub monitor: String,

    /// Initial status check in format "service/path interface property" (optional)
    #[arg(long)]
    pub status: Option<String>,

    /// Type handler for the monitored data
    #[command(subcommand)]
    pub type_handler: TypeHandler,
}

#[derive(Debug, Clone)]
pub struct StatusConfig {
    pub service: String,
    pub object_path: String,
    pub interface: String,
    pub property: String,
}

impl Config {
    /// Parse and validate the status configuration
    pub fn parse_status(&self) -> Result<Option<StatusConfig>, String> {
        if let Some(status_str) = &self.status {
            // Expected format: "service/path interface property"
            // Handle cases like "service / interface property" (with spaces around /)
            let trimmed = status_str.trim();

            // First, try to find the last two whitespace-separated tokens (interface and property)
            let parts: Vec<&str> = trimmed.split_whitespace().collect();

            if parts.len() < 3 {
                return Err(format!(
                    "Invalid status format. Expected: 'service/path interface property', got: '{}'",
                    status_str
                ));
            }

            // Take the last two parts as interface and property
            let interface = parts[parts.len() - 2];
            let property = parts[parts.len() - 1];

            // Everything before the last two parts is the service/path
            let service_path_parts = &parts[..parts.len() - 2];
            let service_path = service_path_parts.join(" ");

            // Now parse service/path - handle spaces around the slash
            let service_path_clean = service_path
                .replace(" / ", "/")
                .replace(" /", "/")
                .replace("/ ", "/");

            if let Some(slash_pos) = service_path_clean.find('/') {
                let service = service_path_clean[..slash_pos].to_string();
                let object_path = service_path_clean[slash_pos..].to_string();

                if service.is_empty() || object_path.is_empty() {
                    return Err(format!(
                        "Invalid service/path format in: '{}'. Service and path cannot be empty.",
                        service_path_clean
                    ));
                }

                // Validate that object_path starts with '/' (which it should since we split on '/')
                if !object_path.starts_with('/') {
                    return Err(format!(
                        "Invalid object path: '{}'. Object paths must start with '/'.",
                        object_path
                    ));
                }

                Ok(Some(StatusConfig {
                    service,
                    object_path,
                    interface: interface.to_string(),
                    property: property.to_string(),
                }))
            } else {
                Err(format!(
                    "Invalid service/path format: '{}'. Expected format: 'service/path'",
                    service_path_clean
                ))
            }
        } else {
            Ok(None)
        }
    }

    /// Validate the configuration and return an error if invalid
    pub fn validate(&self) -> Result<(), String> {
        // Validate status format if provided
        self.parse_status().map(|_| ())
    }
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
    // TODO: Implement additional type handlers:
    // String { ... },
    // Integer { ... },
}

impl TypeHandler {
    /// Process the raw D-Bus data and print the result directly
    /// Returns true if processing was successful, false otherwise
    pub fn process_and_print(&self, body: &zvariant::Value) -> bool {
        match self {
            TypeHandler::Boolean {
                return_true,
                return_false,
            } => {
                if let zvariant::Value::Bool(b) = body {
                    let output = if *b { return_true } else { return_false };
                    println!("{}", output);
                    true
                } else {
                    // Log an error if the type doesn't match
                    warn!("Expected boolean, got {:?}", body);
                    false
                }
            }
        }
    }

    /// Deserialize D-Bus message body and process it according to the type handler
    /// Returns true if processing was successful, false otherwise
    pub fn deserialize_and_process(&self, body: &zbus::message::Body) -> bool {
        match self {
            TypeHandler::Boolean {
                return_true,
                return_false,
            } => {
                // For boolean signals, we expect a direct boolean with signature "b"
                match body.deserialize::<bool>() {
                    Ok(value) => {
                        let output = if value { return_true } else { return_false };
                        println!("{}", output);
                        true
                    }
                    Err(e) => {
                        warn!(
                            "Failed to deserialize boolean from signature '{}': {}",
                            body.signature(),
                            e
                        );
                        false
                    }
                }
            }
        }
    }
}
