use clap::{Parser, Subcommand};
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
            // Split by whitespace into exactly 3 parts
            let parts: Vec<&str> = status_str.trim().split_whitespace().collect();

            if parts.len() != 3 {
                return Err(format!(
                    "Invalid status format. Expected: 'service/path interface property', got: '{}'",
                    status_str
                ));
            }

            // First part must contain exactly one slash to separate service and path
            let service_path = parts[0];
            if !service_path.contains('/') {
                return Err(format!(
                    "Invalid format: '{}'. First parameter must be 'service/path'",
                    service_path
                ));
            }

            // Split service and path at the slash
            let slash_pos = service_path.find('/').unwrap();
            let service = service_path[..slash_pos].to_string();
            let object_path = service_path[slash_pos..].to_string();

            // Basic validation
            if service.is_empty() {
                return Err("Service name cannot be empty".to_string());
            }
            if object_path.len() <= 1 && object_path != "/" {
                return Err("Object path must be '/' or longer".to_string());
            }

            Ok(Some(StatusConfig {
                service,
                object_path,
                interface: parts[1].to_string(),
                property: parts[2].to_string(),
            }))
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
                    // Debug log if the type doesn't match
                    log::debug!("warn: Expected boolean, got {:?}", body);
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
                        log::debug!(
                            "error: Failed to deserialize boolean from signature '{}': {}",
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
