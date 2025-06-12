/// Error handling module with HTML-inspired error codes for waybar integration
///
/// This module defines error types that can be displayed as error codes (E<code>)
/// to help waybar or other status bars understand the current state of the application.
use std::fmt;

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
#[derive(Debug)]
pub enum AppError {
    /// D-Bus connection errors
    Connection(zbus::Error, ErrorCode),
    /// D-Bus interface/member not found
    NotFound(String, ErrorCode),
    /// Message processing errors
    MessageProcessing(String, ErrorCode),
    /// General errors
    General(String, ErrorCode),
}

impl AppError {
    /// Get the error code for this error
    pub fn error_code(&self) -> ErrorCode {
        match self {
            AppError::Connection(_, code) => *code,
            AppError::NotFound(_, code) => *code,
            AppError::MessageProcessing(_, code) => *code,
            AppError::General(_, code) => *code,
        }
    }

    /// Print the error code to stdout for waybar
    pub fn print_error_code(&self) {
        println!("{}", self.error_code().format_for_waybar());
    }

    /// Create a connection error
    pub fn connection_failed(err: zbus::Error) -> Self {
        AppError::Connection(err, ErrorCode::BadGateway)
    }

    /// Create a service unavailable error
    pub fn service_unavailable(msg: impl Into<String>) -> Self {
        AppError::General(msg.into(), ErrorCode::ServiceUnavailable)
    }

    /// Create a not found error
    pub fn not_found(msg: impl Into<String>) -> Self {
        AppError::NotFound(msg.into(), ErrorCode::NotFound)
    }

    /// Create a message processing error
    pub fn message_processing(msg: impl Into<String>) -> Self {
        AppError::MessageProcessing(msg.into(), ErrorCode::UnprocessableEntity)
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

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Connection(err, code) => {
                write!(f, "{}: D-Bus connection error: {}", code, err)
            }
            AppError::NotFound(msg, code) => {
                write!(f, "{}: {}", code, msg)
            }
            AppError::MessageProcessing(msg, code) => {
                write!(f, "{}: Message processing error: {}", code, msg)
            }
            AppError::General(msg, code) => {
                write!(f, "{}: {}", code, msg)
            }
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Connection(err, _) => Some(err),
            _ => None,
        }
    }
}

impl From<zbus::Error> for AppError {
    fn from(err: zbus::Error) -> Self {
        // Map specific zbus errors to appropriate error codes
        match &err {
            zbus::Error::MethodError(name, _, _) if name.contains("NotFound") => {
                AppError::not_found(format!("D-Bus method not found: {}", err))
            }
            zbus::Error::InterfaceNotFound => {
                AppError::service_unavailable("D-Bus interface not found")
            }
            // For most other zbus errors, treat as connection issues
            _ => AppError::connection_failed(err),
        }
    }
}
