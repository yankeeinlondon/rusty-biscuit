//! HTTP and retry constants for provider API interactions
//!
//! This module defines shared constants used across the provider system for
//! HTTP request configuration and exponential backoff retry logic.
//!
//! Moved from `base.rs` and `discovery.rs` during Phase 0 refactoring (2025-12-30)
//! to eliminate code duplication.

use std::time::Duration;

/// Maximum time to wait for an HTTP request to complete
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum size allowed for API responses (10MB)
pub const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024;

/// Initial delay before first retry attempt
pub const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(1);

/// Maximum delay between retry attempts
pub const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

/// Multiplier for exponential backoff (delay doubles each retry)
pub const RETRY_MULTIPLIER: f64 = 2.0;

/// Maximum number of retry attempts for rate-limited requests
pub const MAX_RETRIES: u32 = 3;
