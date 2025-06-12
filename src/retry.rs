/// Retry logic for D-Bus operations
///
/// This module provides retry functionality for operations that may fail
/// temporarily, such as D-Bus connections or interface availability.
use crate::error::AppError;
use log::{debug, warn};
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

/// D-Bus retry delays: [no delay, 1s, 2s]
const DBUS_RETRY_DELAYS: [Duration; 3] = [
    Duration::from_secs(0), // First attempt, no delay
    Duration::from_secs(1), // Second attempt after 1s
    Duration::from_secs(2), // Third attempt after 2s
];

/// Get the delay for a given attempt number (0-based)
fn delay_for_attempt(attempt: usize) -> Duration {
    DBUS_RETRY_DELAYS
        .get(attempt)
        .copied()
        .unwrap_or(Duration::from_secs(5))
}

/// Streamlined retry function with fewer parameters
pub async fn retry_operation<F, Fut, T>(operation: F, operation_name: &str) -> Result<T, AppError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    const MAX_ATTEMPTS: usize = 3;
    let mut last_error = None;

    for attempt in 0..MAX_ATTEMPTS {
        // Apply delay before retries (none on first attempt)
        let delay = delay_for_attempt(attempt);
        if attempt > 0 {
            debug!(
                "Retrying {} (attempt {}/{}) after {:?} delay",
                operation_name,
                attempt + 1,
                MAX_ATTEMPTS,
                delay
            );
            sleep(delay).await;
        } else {
            debug!("Attempting {} (attempt 1/{})", operation_name, MAX_ATTEMPTS);
        }

        match operation().await {
            Ok(result) => {
                if attempt > 0 {
                    debug!("{} succeeded on attempt {}", operation_name, attempt + 1);
                }
                return Ok(result);
            }
            Err(error) => {
                warn!(
                    "{} failed on attempt {}/{}: {}",
                    operation_name,
                    attempt + 1,
                    MAX_ATTEMPTS,
                    error
                );

                // Check if this is a permanent error that shouldn't be retried
                if error.is_permanent() {
                    debug!("Permanent error detected, stopping retries: {}", error);
                    return Err(error);
                }

                last_error = Some(error);
            }
        }
    }

    // All attempts failed, return the last error
    Err(last_error.unwrap())
}

/// Retry an operation specifically for D-Bus connections
pub async fn retry_dbus_operation<F, Fut, T>(
    operation: F,
    operation_name: &str,
) -> Result<T, AppError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    retry_operation(operation, operation_name).await
}
