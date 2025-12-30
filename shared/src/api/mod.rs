//! API module for provider interactions
//!
//! This module provides utilities for interacting with LLM provider APIs,
//! including authentication, endpoint configuration, and OpenAI-compatible
//! API utilities.
//!
//! Created during Phase 0 of the provider refactoring (2025-12-30).

pub mod types;
pub mod openai_compat;

pub use types::{ApiAuth, ApiEndpoint};
pub use openai_compat::{get_provider_models_from_api, get_all_provider_models};
