use clap::{Parser, Subcommand};
use std::io::Write;
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
    /// Extract a boolean from various zvariant::Value types
    fn extract_boolean(&self, value: &zvariant::Value) -> Option<bool> {
        match value {
            // Direct boolean
            zvariant::Value::Bool(b) => Some(*b),

            // Handle variant inside variant (common with properties)
            zvariant::Value::Value(v) => self.extract_boolean(v),

            // Could not extract boolean
            _ => {
                log::debug!("warn: Could not extract boolean from value: {:?}", value);
                None
            }
        }
    }

    /// Deserialize a boolean value directly from a D-Bus message
    /// This optimizes the message handling by attempting direct type deserialization first
    pub fn deserialize_from_message(&self, message: &zbus::Message) -> Result<bool, String> {
        match self {
            TypeHandler::Boolean { .. } => {
                // Try direct boolean deserialization first for efficiency
                match message.body().deserialize::<bool>() {
                    Ok(value) => Ok(value),
                    Err(_) => {
                        // Fall back to generic deserialization and extraction
                        match message.body().deserialize::<zvariant::Value>() {
                            Ok(value) => self.extract_boolean(&value).ok_or_else(|| {
                                format!("Could not extract boolean from value: {:?}", value)
                            }),
                            Err(e) => Err(format!("Failed to deserialize message: {}", e)),
                        }
                    }
                }
            }
        }
    }

    /// Process the raw D-Bus data and print the result directly
    /// Returns true if processing was successful, false otherwise
    pub fn process_and_print(&self, value: &zvariant::Value) -> bool {
        match self {
            TypeHandler::Boolean {
                return_true,
                return_false,
            } => {
                if let Some(b) = self.extract_boolean(value) {
                    let output = if b { return_true } else { return_false };
                    println!("{}", output);
                    // Flush stdout to ensure waybar gets the output immediately
                    if let Err(e) = std::io::stdout().flush() {
                        log::debug!("error: Failed to flush stdout: {}", e);
                    }
                    true
                } else {
                    log::debug!("warn: Could not convert value to boolean: {:?}", value);
                    false
                }
            }
        }
    }

    /// Print a formatted output based on a boolean value
    /// This helper method is used to avoid code duplication
    pub fn print_boolean_output(&self, value: bool) -> Result<(), String> {
        match self {
            TypeHandler::Boolean {
                return_true,
                return_false,
            } => {
                let output = if value { return_true } else { return_false };
                println!("{}", output);
                // Flush stdout to ensure waybar gets the output immediately
                if let Err(e) = std::io::stdout().flush() {
                    return Err(format!("Failed to flush stdout: {}", e));
                }
                Ok(())
            }
        }
    }
}
