/// Error handling module with HTTP-inspired error codes for waybar integration
///
/// This module defines error types that can be displayed as error codes (E<code>)
/// to help waybar or other status bars understand the current state of the application.
use std::fmt;
use thiserror::Error;

/// Error codes inspired by HTTP status codes for waybar display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// E503 - Service Unavailable (D-Bus interface not available)
    ServiceUnavailable = 503,
    /// E502 - Bad Gateway (D-Bus connection failed)
    BadGateway = 502,
    /// E404 - Not Found (Interface or member not found)
    NotFound = 404,
    /// E422 - Unprocessable Entity (Invalid message format)
    UnprocessableEntity = 422,
}

impl ErrorCode {
    /// Get the numeric error code
    pub fn code(&self) -> u16 {
        *self as u16
    }

    /// Format the error code for waybar display
    pub fn format_for_waybar(&self) -> String {
        format!("E{}", self.code())
    }

    /// Get a human-readable description of the error
    pub fn description(&self) -> &'static str {
        match self {
            ErrorCode::ServiceUnavailable => "D-Bus interface not available",
            ErrorCode::BadGateway => "D-Bus connection failed",
            ErrorCode::NotFound => "Interface or member not found",
            ErrorCode::UnprocessableEntity => "Invalid message format",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.format_for_waybar(), self.description())
    }
}

/// Application-specific error type that maps to error codes
#[derive(Error, Debug)]
pub enum AppError {
    /// D-Bus connection errors
    #[error("D-Bus connection error: {0}")]
    Connection(#[source] zbus::Error),

    /// D-Bus interface/member not found
    #[error("{0}")]
    NotFound(String),

    /// Message processing errors
    #[error("Message processing error: {0}")]
    MessageProcessing(String),

    /// General errors with flexible error codes
    #[error("{1}")]
    General(ErrorCode, String),
}

impl AppError {
    /// Get the error code for this error
    pub fn error_code(&self) -> ErrorCode {
        match self {
            AppError::Connection(_) => ErrorCode::BadGateway,
            AppError::NotFound(_) => ErrorCode::NotFound,
            AppError::MessageProcessing(_) => ErrorCode::UnprocessableEntity,
            AppError::General(code, _) => *code,
        }
    }

    /// Print the error code to stdout for waybar
    pub fn print_error_code(&self) {
        println!("{}", self.error_code().format_for_waybar());
    }

    /// Create a connection error
    pub fn connection_failed(err: zbus::Error) -> Self {
        AppError::Connection(err)
    }

    /// Create a service unavailable error
    pub fn service_unavailable(msg: impl Into<String>) -> Self {
        AppError::General(ErrorCode::ServiceUnavailable, msg.into())
    }

    /// Create a not found error
    pub fn not_found(msg: impl Into<String>) -> Self {
        AppError::NotFound(msg.into())
    }

    /// Create a message processing error
    pub fn message_processing(msg: impl Into<String>) -> Self {
        AppError::MessageProcessing(msg.into())
    }

    /// Check if this error represents a permanent failure that shouldn't be retried
    pub fn is_permanent(&self) -> bool {
        match self.error_code() {
            ErrorCode::NotFound => true,            // Invalid interface/member names
            ErrorCode::UnprocessableEntity => true, // Invalid message format
            ErrorCode::ServiceUnavailable => false, // Service might come back
            ErrorCode::BadGateway => false,         // Connection issues are temporary
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
