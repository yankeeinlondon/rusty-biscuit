//! Retry logic utilities for provider API requests
//!
//! This module provides exponential backoff retry logic for handling rate limiting
//! and transient failures when communicating with LLM provider APIs.
//!
//! Moved from `base.rs` and `discovery.rs` during Phase 0 refactoring (2025-12-30)
//! to eliminate code duplication.

use super::constants::*;
use super::discovery::ProviderError;
use std::future::Future;
use tracing::warn;

/// Check if an error is a rate limit error (HTTP 429)
///
/// ## Arguments
/// * `error` - The provider error to check
///
/// ## Returns
/// Returns `true` if the error indicates rate limiting, `false` otherwise
///
/// ## Examples
/// ```
/// use shared::providers::retry::is_rate_limit_error;
/// use shared::providers::discovery::ProviderError;
///
/// let rate_limit_err = ProviderError::RateLimitExceeded {
///     provider: "test".to_string(),
/// };
/// assert!(is_rate_limit_error(&rate_limit_err));
///
/// let auth_err = ProviderError::AuthenticationFailed {
///     provider: "test".to_string(),
/// };
/// assert!(!is_rate_limit_error(&auth_err));
/// ```
pub fn is_rate_limit_error(error: &ProviderError) -> bool {
    match error {
        ProviderError::HttpError(e) => e
            .status()
            .map(|s| s.as_u16() == 429)
            .unwrap_or(false),
        ProviderError::RateLimitExceeded { .. } => true,
        _ => false,
    }
}

/// Fetch with exponential backoff retry logic
///
/// Executes an async operation with automatic retries on rate limit errors (429).
/// Uses exponential backoff with configurable delays and maximum retry attempts.
///
/// ## Arguments
/// * `fetch_fn` - Async function to execute (must be callable multiple times)
/// * `provider_name` - Provider name for logging and error messages
///
/// ## Returns
/// Returns `Ok(T)` if the operation succeeds within the retry limit, or
/// `Err(ProviderError)` if all retries are exhausted or a non-retriable error occurs.
///
/// ## Retry Strategy
/// - Initial delay: 1 second (configurable via `INITIAL_RETRY_DELAY`)
/// - Delay multiplier: 2x (configurable via `RETRY_MULTIPLIER`)
/// - Maximum delay: 30 seconds (configurable via `MAX_RETRY_DELAY`)
/// - Maximum retries: 3 attempts (configurable via `MAX_RETRIES`)
/// - Timeout per attempt: 30 seconds (configurable via `REQUEST_TIMEOUT`)
///
/// ## Errors
/// - `ProviderError::Timeout` - Operation exceeded timeout
/// - `ProviderError::RateLimitExceeded` - Rate limit hit after all retries
/// - Other `ProviderError` variants - Non-retriable errors (returned immediately)
///
/// ## Examples
/// ```no_run
/// use shared::providers::retry::fetch_with_retry;
/// use shared::providers::discovery::ProviderError;
///
/// #[tokio::main]
/// async fn main() -> Result<(), ProviderError> {
///     let result = fetch_with_retry(
///         || async {
///             // Your API call here
///             Ok::<String, ProviderError>("data".to_string())
///         },
///         "openai"
///     ).await?;
///
///     println!("Result: {}", result);
///     Ok(())
/// }
/// ```
#[tracing::instrument(skip(fetch_fn))]
pub async fn fetch_with_retry<F, Fut, T>(
    fetch_fn: F,
    provider_name: &str,
) -> Result<T, ProviderError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, ProviderError>>,
{
    let mut delay = INITIAL_RETRY_DELAY;

    for attempt in 0..=MAX_RETRIES {
        match tokio::time::timeout(REQUEST_TIMEOUT, fetch_fn()).await {
            Ok(Ok(result)) => return Ok(result),
            Ok(Err(e)) if is_rate_limit_error(&e) && attempt < MAX_RETRIES => {
                warn!(
                    "Rate limit hit for {}, retry {} after {:?}",
                    provider_name,
                    attempt + 1,
                    delay
                );
                tokio::time::sleep(delay).await;
                delay = std::cmp::min(
                    std::time::Duration::from_secs_f64(delay.as_secs_f64() * RETRY_MULTIPLIER),
                    MAX_RETRY_DELAY,
                );
            }
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                return Err(ProviderError::Timeout {
                    provider: provider_name.to_string(),
                })
            }
        }
    }

    Err(ProviderError::RateLimitExceeded {
        provider: provider_name.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_is_rate_limit_error_with_rate_limit_variant() {
        let rate_limit_err = ProviderError::RateLimitExceeded {
            provider: "test".to_string(),
        };
        assert!(is_rate_limit_error(&rate_limit_err));
    }

    #[tokio::test]
    async fn test_is_rate_limit_error_with_auth_error() {
        let auth_err = ProviderError::AuthenticationFailed {
            provider: "test".to_string(),
        };
        assert!(!is_rate_limit_error(&auth_err));
    }

    #[tokio::test]
    async fn test_fetch_with_retry_success() {
        let result = fetch_with_retry(
            || async { Ok::<i32, ProviderError>(42) },
            "test_provider"
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_fetch_with_retry_timeout() {
        let result = fetch_with_retry(
            || async {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok::<i32, ProviderError>(42)
            },
            "test_provider"
        ).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ProviderError::Timeout { provider } => {
                assert_eq!(provider, "test_provider");
            }
            _ => panic!("Expected Timeout error"),
        }
    }

    #[tokio::test]
    async fn test_fetch_with_retry_non_retriable_error() {
        let result = fetch_with_retry(
            || async {
                Err::<i32, ProviderError>(ProviderError::AuthenticationFailed {
                    provider: "test".to_string(),
                })
            },
            "test_provider"
        ).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ProviderError::AuthenticationFailed { provider } => {
                assert_eq!(provider, "test");
            }
            _ => panic!("Expected AuthenticationFailed error"),
        }
    }

    #[tokio::test]
    async fn test_fetch_with_retry_exhausts_retries() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();

        let result = fetch_with_retry(
            move || {
                let count = attempt_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, ProviderError>(ProviderError::RateLimitExceeded {
                        provider: "test".to_string(),
                    })
                }
            },
            "test_provider"
        ).await;

        assert!(result.is_err());
        // MAX_RETRIES + 1 initial attempt = 4 total attempts
        assert_eq!(attempt_count.load(Ordering::SeqCst), MAX_RETRIES + 1);
    }
}
