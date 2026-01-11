//! Provider discovery module
//!
//! This module provides functionality to discover LLM providers and models
//! from various sources (APIs and hardcoded lists), normalize the data,
//! and format it for consumption.
//!
//! # Features
//!
//! - Fetch models from OpenAI, Anthropic, Hugging Face, and Google Gemini
//! - Rate limiting with exponential backoff
//! - 24-hour caching to reduce API calls
//! - Output as JSON array or Rust enum variants
//!
//! # Examples
//!
//! ```rust,no_run
//! use shared::providers::{generate_provider_list, ProviderListFormat};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Generate JSON array format
//!     let json = generate_provider_list(Some(ProviderListFormat::StringLiterals)).await?;
//!     println!("{}", json);
//!
//!     // Generate Rust enum format
//!     let rust_enum = generate_provider_list(Some(ProviderListFormat::RustEnum)).await?;
//!     println!("{}", rust_enum);
//!
//!     Ok(())
//! }
//! ```

pub mod cache;
pub mod constants;
pub mod curated;
pub mod discovery;
pub mod retry;
pub mod types;
pub mod base;
pub mod zai;
pub mod zenmux;

// Re-export main types and functions
pub use curated::{get_curated_models, LAST_UPDATED, PROVIDER_COUNT};
pub use discovery::{generate_provider_list, ProviderError};
pub use types::{LlmEntry, ProviderListFormat, OpenAIModelsResponse, OpenAIModel, ProviderModel};
pub use base::{
    Provider,
    has_provider_api_key,
    get_api_keys,
    get_provider_models,
    artificial_analysis_url
};
