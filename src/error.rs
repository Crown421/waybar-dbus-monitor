/// Error handling module with HTTP-inspired error codes for waybar integration
///
/// This module defines error types that can be displayed as error codes (E<code>)
/// to help waybar or other status bars understand the current state of the application.
use thiserror::Error;

/// Application error type with HTTP-inspired error codes for waybar integration
#[derive(Error, Debug)]
pub enum AppError {
    /// E503 - Service Unavailable (D-Bus interface not available)
    #[error("D-Bus interface not available: {0}")]
    ServiceUnavailable(
        String,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),

    /// E502 - Bad Gateway (D-Bus connection failed)
    #[error("D-Bus connection failed: {0}")]
    BadGateway(
        String,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),

    /// E404 - Not Found (Interface or member not found)
    #[error("Interface or member not found: {0}")]
    NotFound(
        String,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),

    /// E422 - Unprocessable Entity (Invalid message format)
    #[error("Invalid message format: {0}")]
    UnprocessableEntity(
        String,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),
}

impl AppError {
    /// Get the numeric error code
    pub fn code(&self) -> u16 {
        match self {
            Self::ServiceUnavailable(_, _) => 503,
            Self::BadGateway(_, _) => 502,
            Self::NotFound(_, _) => 404,
            Self::UnprocessableEntity(_, _) => 422,
        }
    }

    /// Format the error code for waybar display
    pub fn format_for_waybar(&self) -> String {
        format!("E{}", self.code())
    }

    /// Print the error code to stdout for waybar
    pub fn print_error_code(&self) {
        println!("{}", self.format_for_waybar());
    }

    /// Create a connection error
    pub fn connection_failed(err: zbus::Error) -> Self {
        Self::BadGateway(
            format!("D-Bus connection error: {}", err),
            Some(Box::new(err)),
        )
    }

    /// Create a service unavailable error
    pub fn service_unavailable(msg: impl Into<String>) -> Self {
        Self::ServiceUnavailable(msg.into(), None)
    }

    /// Create a not found error
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into(), None)
    }

    /// Create a message processing error
    pub fn message_processing(msg: impl Into<String>) -> Self {
        Self::UnprocessableEntity(msg.into(), None)
    }

    /// Check if this error represents a permanent failure that shouldn't be retried
    pub fn is_permanent(&self) -> bool {
        match self {
            Self::NotFound(_, _) => true, // Invalid interface/member names
            Self::UnprocessableEntity(_, _) => true, // Invalid message format
            Self::ServiceUnavailable(_, _) => false, // Service might come back
            Self::BadGateway(_, _) => false, // Connection issues are temporary
        }
    }
}

/// Convenience macro for error reporting (prints error code and logs error)
#[macro_export]
macro_rules! report_error {
    ($error:expr) => {
        $error.print_error_code();
        log::debug!("error: {}", $error);
    };
    ($error:expr, $msg:expr) => {
        $error.print_error_code();
        log::debug!("error: {}: {}", $msg, $error);
    };
}

/// Convenience macros for streamlined error creation
#[macro_export]
macro_rules! error_not_found {
    ($($arg:tt)*) => {
        $crate::error::AppError::not_found(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! error_service_unavailable {
    ($($arg:tt)*) => {
        $crate::error::AppError::service_unavailable(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! error_message_processing {
    ($($arg:tt)*) => {
        $crate::error::AppError::message_processing(format!($($arg)*))
    };
}

impl From<zbus::Error> for AppError {
    fn from(err: zbus::Error) -> Self {
        // Map specific zbus errors to appropriate error codes
        match &err {
            zbus::Error::MethodError(name, _, _) if name.contains("NotFound") => {
                error_not_found!("D-Bus method not found: {}", err)
            }
            zbus::Error::InterfaceNotFound => {
                error_service_unavailable!("D-Bus interface not found")
            }
            // Handle various service unavailability errors
            zbus::Error::MethodError(name, _, _)
                if name.contains("ServiceUnknown")
                    || name.contains("UnknownObject")
                    || name.contains("UnknownInterface") =>
            {
                error_service_unavailable!("D-Bus service not available: {}", err)
            }
            // Check error message for service unavailability indicators
            _ if err.to_string().contains("not found")
                || err.to_string().contains("NotFound")
                || err.to_string().contains("ServiceUnknown")
                || err.to_string().contains("UnknownObject")
                || err.to_string().contains("UnknownInterface")
                || err.to_string().contains("service does not exist")
                || err.to_string().contains("No such service") =>
            {
                error_service_unavailable!("D-Bus service not available: {}", err)
            }
            // For most other zbus errors, treat as connection issues
            _ => AppError::connection_failed(err),
        }
    }
}
