use clap::{Parser, Subcommand};
use serde_json;
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
    fn extract_boolean(value: &zvariant::Value) -> Option<bool> {
        match value {
            zvariant::Value::Bool(b) => Some(*b),
            zvariant::Value::Value(v) => Self::extract_boolean(v),
            _ => {
                log::debug!("warn: Could not extract boolean from value: {:?}", value);
                None
            }
        }
    }

    /// Helper method to format and print a boolean value as Waybar JSON
    fn format_and_print_boolean(&self, value: bool) -> Result<(), String> {
        match self {
            TypeHandler::Boolean {
                return_true,
                return_false,
            } => {
                let text = if value { return_true } else { return_false };
                let tooltip = if value { "enabled" } else { "disabled" };

                // Use serde_json for proper escaping and formatting
                let json_output = serde_json::json!({
                    "text": text,
                    "tooltip": tooltip
                });

                println!("{}", json_output);
                std::io::stdout()
                    .flush()
                    .map_err(|e| format!("Failed to flush stdout: {}", e))
            }
        }
    }

    /// Process a D-Bus message and print formatted output
    pub fn process_message(&self, message: &zbus::Message) -> Result<(), String> {
        match self {
            TypeHandler::Boolean { .. } => {
                // Try direct boolean deserialization first for efficiency
                let bool_value = message.body().deserialize::<bool>().or_else(|_| {
                    // Fall back to generic deserialization and extraction
                    message
                        .body()
                        .deserialize::<zvariant::Value>()
                        .map_err(|e| format!("Failed to deserialize message: {}", e))
                        .and_then(|value| {
                            Self::extract_boolean(&value)
                                .ok_or_else(|| format!("Could not extract boolean: {:?}", value))
                        })
                })?;

                self.format_and_print_boolean(bool_value)
            }
        }
    }

    /// Process the raw D-Bus data and print the result
    pub fn process_and_print(&self, value: &zvariant::Value) -> bool {
        match self {
            TypeHandler::Boolean { .. } => {
                if let Some(b) = Self::extract_boolean(value) {
                    match self.format_and_print_boolean(b) {
                        Ok(_) => true,
                        Err(e) => {
                            log::debug!("error: {}", e);
                            false
                        }
                    }
                } else {
                    log::debug!("warn: Could not convert value to boolean: {:?}", value);
                    false
                }
            }
        }
    }
}
