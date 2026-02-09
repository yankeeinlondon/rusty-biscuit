//! Model scanner traits and implementations.
//!
//! This module defines the `ModelScanner` trait for discovering models from
//! different runners (Ollama, LM Studio, Llama.cpp) and aggregating them
//! through the `ModelRegistry`.
//!
//! ## Available Scanners
//!
//! - [`OllamaScanner`]: Scans Ollama manifests and enriches via API
//! - [`LmStudioScanner`]: Scans LM Studio's models directory for GGUF files
//! - [`LlamaCppScanner`]: Scans user-configured directories for GGUF files

mod llamacpp;
mod lmstudio;
mod ollama;

pub use llamacpp::LlamaCppScanner;
pub use lmstudio::LmStudioScanner;
pub use ollama::OllamaScanner;

use crate::{ModelCitizenError, UnifiedModel};
use async_trait::async_trait;

/// Trait for scanning models from a specific runner.
///
/// Implementations should be thread-safe (`Send + Sync`) to support concurrent
/// scanning across multiple runners.
///
/// ## Example
///
/// ```ignore
/// use model_citizen::scanner::ModelScanner;
///
/// struct MyScanner;
///
/// #[async_trait]
/// impl ModelScanner for MyScanner {
///     fn name(&self) -> &'static str { "my-scanner" }
///
///     async fn is_available(&self) -> bool { true }
///
///     async fn scan(&self) -> Result<Vec<UnifiedModel>, ModelCitizenError> {
///         Ok(vec![])
///     }
/// }
/// ```
#[async_trait]
pub trait ModelScanner: Send + Sync {
    /// Returns the unique name of this scanner.
    fn name(&self) -> &'static str;

    /// Checks if the scanner's backend is available.
    ///
    /// For example, checks if Ollama is installed and running, or if the
    /// LM Studio models directory exists.
    async fn is_available(&self) -> bool;

    /// Scans for models and returns a list of discovered models.
    ///
    /// ## Errors
    ///
    /// Returns an error if the scan fails (e.g., I/O error, API error).
    async fn scan(&self) -> Result<Vec<UnifiedModel>, ModelCitizenError>;

    /// Enriches a model with additional metadata from the provider.
    ///
    /// Called lazily (e.g., during `model info`) rather than during scan.
    /// Default implementation is a no-op. Override for providers that
    /// require an extra API call for detailed metadata (e.g., Ollama's
    /// `/api/show`).
    ///
    /// ## Errors
    ///
    /// Returns an error if enrichment fails (e.g., API unreachable).
    async fn enrich(&self, _model: &mut UnifiedModel) -> Result<(), ModelCitizenError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify trait is object-safe
    #[test]
    fn scanner_is_object_safe() {
        fn _accepts_boxed(_: Box<dyn ModelScanner>) {}
    }

    #[test]
    fn scanner_is_send_sync() {
        fn _assert_send<T: Send>() {}
        fn _assert_sync<T: Sync>() {}

        // The trait bounds require Send + Sync
        fn _check<T: ModelScanner>() {
            _assert_send::<T>();
            _assert_sync::<T>();
        }
    }
}
