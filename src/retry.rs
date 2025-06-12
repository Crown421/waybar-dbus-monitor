/// Retry logic for D-Bus operations
///
/// This module provides retry functionality for operations that may fail
/// temporarily, such as D-Bus connections or interface availability.
use crate::error::AppError;
use log::debug;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: usize,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 500,
            max_delay_ms: 5000,
            backoff_factor: 1.5,
        }
    }
}

impl RetryConfig {
    /// Calculate delay for a given attempt (0-based)
    fn delay_for_attempt(&self, attempt: usize) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(self.initial_delay_ms);
        }

        let delay_ms = (self.initial_delay_ms as f64 * self.backoff_factor.powi(attempt as i32))
            .min(self.max_delay_ms as f64) as u64;

        Duration::from_millis(delay_ms)
    }
}

/// Streamlined retry function with configurable retry policy
pub async fn retry_operation_with_config<F, Fut, T>(
    operation: F,
    operation_name: &str,
    config: RetryConfig,
) -> Result<T, AppError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    let mut last_error = None;

    for attempt in 0..config.max_attempts {
        // Skip delay for the first attempt (attempt 0)
        if attempt > 0 {
            let delay = config.delay_for_attempt(attempt - 1);
            debug!(
                "Retrying {} (attempt {}/{}) after {:?} delay",
                operation_name,
                attempt + 1,
                config.max_attempts,
                delay
            );
            sleep(delay).await;
        } else {
            debug!(
                "Attempting {} (attempt 1/{})",
                operation_name, config.max_attempts
            );
        }

        match operation().await {
            Ok(result) => {
                if attempt > 0 {
                    debug!("{} succeeded on attempt {}", operation_name, attempt + 1);
                }
                return Ok(result);
            }
            Err(error) => {
                // Print error code and log the error if this is not the first attempt
                // For first attempt, don't show error codes to avoid flicker
                if attempt > 0 {
                    // Print only the error code, no additional text
                    error.print_error_code();
                    debug!(
                        "warn: {} failed on attempt {}/{}: {}",
                        operation_name,
                        attempt + 1,
                        config.max_attempts,
                        error
                    );
                } else {
                    debug!(
                        "warn: {} failed on attempt {}/{}: {}",
                        operation_name,
                        attempt + 1,
                        config.max_attempts,
                        error
                    );
                }

                // Check if this is a permanent error that shouldn't be retried
                if error.is_permanent() {
                    debug!("Permanent error detected, stopping retries: {}", error);
                    // Print only the error code for permanent errors, no additional text
                    error.print_error_code();
                    return Err(error);
                }

                last_error = Some(error);
            }
        }
    }

    // All attempts failed, return the last error
    let final_error = last_error.unwrap();
    // Print only the error code after all retries are exhausted, no additional text
    final_error.print_error_code();
    Err(final_error)
}

/// Streamlined retry function with default config
pub async fn retry_operation<F, Fut, T>(operation: F, operation_name: &str) -> Result<T, AppError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    retry_operation_with_config(operation, operation_name, RetryConfig::default()).await
}
