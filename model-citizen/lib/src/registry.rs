//! Model registry for aggregating results from multiple scanners.
//!
//! The `ModelRegistry` holds references to multiple `ModelScanner` implementations
//! and can scan all of them concurrently, deduplicating results.

use crate::UnifiedModel;
use crate::scanner::ModelScanner;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// Default timeout for individual scanner operations.
pub const DEFAULT_SCANNER_TIMEOUT: Duration = Duration::from_secs(5);

/// Result of a single scanner's operation.
#[derive(Debug)]
pub enum ScanResult {
    /// Scan completed successfully.
    Success {
        /// Scanner name.
        name: &'static str,
        /// Discovered models.
        models: Vec<UnifiedModel>,
    },
    /// Scan failed with an error.
    Error {
        /// Scanner name.
        name: &'static str,
        /// Error message.
        error: String,
    },
    /// Scan timed out.
    Timeout {
        /// Scanner name.
        name: &'static str,
    },
    /// Scanner was not available.
    Unavailable {
        /// Scanner name.
        name: &'static str,
    },
}

impl ScanResult {
    /// Returns the scanner name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Success { name, .. }
            | Self::Error { name, .. }
            | Self::Timeout { name }
            | Self::Unavailable { name } => name,
        }
    }

    /// Returns the models if successful, empty vec otherwise.
    #[must_use]
    pub fn models(&self) -> Vec<UnifiedModel> {
        match self {
            Self::Success { models, .. } => models.clone(),
            _ => Vec::new(),
        }
    }

    /// Returns true if the scan was successful.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }
}

/// Registry for managing multiple model scanners.
///
/// Supports concurrent scanning with timeout and graceful degradation
/// when individual scanners fail.
///
/// ## Example
///
/// ```ignore
/// use model_citizen::registry::ModelRegistry;
///
/// let mut registry = ModelRegistry::new();
/// registry.add_scanner(OllamaScanner::new());
/// registry.add_scanner(LmStudioScanner::new());
///
/// // Scan all runners concurrently
/// let models = registry.scan_all().await;
/// ```
pub struct ModelRegistry {
    scanners: Vec<Arc<dyn ModelScanner>>,
    timeout: Duration,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelRegistry {
    /// Creates a new empty registry with default timeout.
    #[must_use]
    pub fn new() -> Self {
        Self {
            scanners: Vec::new(),
            timeout: DEFAULT_SCANNER_TIMEOUT,
        }
    }

    /// Creates a new registry with a custom timeout.
    #[must_use]
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            scanners: Vec::new(),
            timeout,
        }
    }

    /// Sets the timeout for scanner operations.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Adds a scanner to the registry.
    pub fn add_scanner(&mut self, scanner: impl ModelScanner + 'static) {
        self.scanners.push(Arc::new(scanner));
    }

    /// Adds a pre-wrapped Arc scanner to the registry.
    pub fn add_scanner_arc(&mut self, scanner: Arc<dyn ModelScanner>) {
        self.scanners.push(scanner);
    }

    /// Returns the number of registered scanners.
    #[must_use]
    pub fn scanner_count(&self) -> usize {
        self.scanners.len()
    }

    /// Returns true if no scanners are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.scanners.is_empty()
    }

    /// Scans all registered scanners concurrently and returns aggregated results.
    ///
    /// - Applies timeout to each scanner
    /// - Deduplicates models by file path
    /// - Continues even if individual scanners fail
    pub async fn scan_all(&self) -> Vec<UnifiedModel> {
        let results = self.scan_all_with_details().await;
        Self::deduplicate(results.into_iter().flat_map(|r| r.models()).collect())
    }

    /// Scans all registered scanners and returns detailed results for each.
    ///
    /// Useful for diagnostics and reporting which scanners succeeded/failed.
    pub async fn scan_all_with_details(&self) -> Vec<ScanResult> {
        if self.scanners.is_empty() {
            return Vec::new();
        }

        let timeout_duration = self.timeout;
        let futures: Vec<_> = self
            .scanners
            .iter()
            .map(|scanner| {
                let scanner = Arc::clone(scanner);
                async move { Self::scan_one(scanner, timeout_duration).await }
            })
            .collect();

        futures::future::join_all(futures).await
    }

    /// Scans only available scanners (checks `is_available()` first).
    pub async fn scan_available(&self) -> Vec<UnifiedModel> {
        let timeout_duration = self.timeout;
        let futures: Vec<_> = self
            .scanners
            .iter()
            .map(|scanner| {
                let scanner = Arc::clone(scanner);
                async move { Self::scan_if_available(scanner, timeout_duration).await }
            })
            .collect();

        let results = futures::future::join_all(futures).await;
        Self::deduplicate(results.into_iter().flat_map(|r| r.models()).collect())
    }

    /// Enriches a model with additional metadata from its source scanner.
    ///
    /// Dispatches to the matching scanner's `enrich()` method based on
    /// `model.source`. Errors are silently ignored since enrichment is
    /// best-effort.
    pub async fn enrich(&self, model: &mut UnifiedModel) {
        let source_name = model.source.as_str();
        for scanner in &self.scanners {
            if scanner.name() == source_name {
                let _ = scanner.enrich(model).await;
                return;
            }
        }
    }

    /// Scans a single scanner with timeout.
    async fn scan_one(scanner: Arc<dyn ModelScanner>, timeout_duration: Duration) -> ScanResult {
        let name = scanner.name();

        match timeout(timeout_duration, scanner.scan()).await {
            Ok(Ok(models)) => ScanResult::Success { name, models },
            Ok(Err(e)) => ScanResult::Error {
                name,
                error: e.to_string(),
            },
            Err(_) => ScanResult::Timeout { name },
        }
    }

    /// Scans a scanner only if it's available.
    async fn scan_if_available(
        scanner: Arc<dyn ModelScanner>,
        timeout_duration: Duration,
    ) -> ScanResult {
        let name = scanner.name();

        // Check availability with timeout
        let available = match timeout(timeout_duration, scanner.is_available()).await {
            Ok(available) => available,
            Err(_) => return ScanResult::Timeout { name },
        };

        if !available {
            return ScanResult::Unavailable { name };
        }

        Self::scan_one(scanner, timeout_duration).await
    }

    /// Deduplicates models by file path.
    ///
    /// When the same model file is found by multiple scanners (e.g., shared via symlink),
    /// only the first occurrence is kept.
    fn deduplicate(models: Vec<UnifiedModel>) -> Vec<UnifiedModel> {
        let mut seen_paths = HashSet::new();
        let mut unique = Vec::with_capacity(models.len());

        for model in models {
            let path_str = model.path.to_string_lossy().to_string();
            if seen_paths.insert(path_str) {
                unique.push(model);
            }
        }

        unique
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ModelArchitecture, ModelCitizenError, ModelFormat, ModelSource, QuantizationType};
    use async_trait::async_trait;

    /// Mock scanner for testing.
    struct MockScanner {
        name: &'static str,
        available: bool,
        models: Vec<UnifiedModel>,
        delay_ms: Option<u64>,
        should_error: bool,
    }

    impl MockScanner {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                available: true,
                models: Vec::new(),
                delay_ms: None,
                should_error: false,
            }
        }

        fn with_models(mut self, models: Vec<UnifiedModel>) -> Self {
            self.models = models;
            self
        }

        fn unavailable(mut self) -> Self {
            self.available = false;
            self
        }

        fn with_delay(mut self, delay_ms: u64) -> Self {
            self.delay_ms = Some(delay_ms);
            self
        }

        fn with_error(mut self) -> Self {
            self.should_error = true;
            self
        }
    }

    #[async_trait]
    impl ModelScanner for MockScanner {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn is_available(&self) -> bool {
            if let Some(delay) = self.delay_ms {
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
            self.available
        }

        async fn scan(&self) -> Result<Vec<UnifiedModel>, ModelCitizenError> {
            if let Some(delay) = self.delay_ms {
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
            if self.should_error {
                return Err(ModelCitizenError::network("mock error"));
            }
            Ok(self.models.clone())
        }
    }

    fn make_model(name: &str, path: &str) -> UnifiedModel {
        UnifiedModel::new(
            name,
            1024,
            QuantizationType::Q4Km,
            ModelArchitecture::Llama,
            ModelSource::LlamaCpp,
            ModelFormat::Gguf,
            path,
        )
    }

    #[test]
    fn new_registry_is_empty() {
        let registry = ModelRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.scanner_count(), 0);
    }

    #[test]
    fn add_scanner_increases_count() {
        let mut registry = ModelRegistry::new();
        registry.add_scanner(MockScanner::new("test"));
        assert_eq!(registry.scanner_count(), 1);
        assert!(!registry.is_empty());
    }

    #[tokio::test]
    async fn empty_registry_returns_empty_vec() {
        let registry = ModelRegistry::new();
        let models = registry.scan_all().await;
        assert!(models.is_empty());
    }

    #[tokio::test]
    async fn scan_all_aggregates_models() {
        let mut registry = ModelRegistry::new();

        registry.add_scanner(
            MockScanner::new("scanner1").with_models(vec![make_model("model1", "/path/a.gguf")]),
        );
        registry.add_scanner(MockScanner::new("scanner2").with_models(vec![
            make_model("model2", "/path/b.gguf"),
            make_model("model3", "/path/c.gguf"),
        ]));

        let models = registry.scan_all().await;
        assert_eq!(models.len(), 3);
    }

    #[tokio::test]
    async fn scan_all_deduplicates_by_path() {
        let mut registry = ModelRegistry::new();

        // Both scanners find the same file
        registry.add_scanner(
            MockScanner::new("scanner1")
                .with_models(vec![make_model("model1", "/shared/model.gguf")]),
        );
        registry.add_scanner(
            MockScanner::new("scanner2")
                .with_models(vec![make_model("model1", "/shared/model.gguf")]),
        );

        let models = registry.scan_all().await;
        assert_eq!(models.len(), 1);
    }

    #[tokio::test]
    async fn scan_all_handles_scanner_error() {
        let mut registry = ModelRegistry::new();

        registry.add_scanner(
            MockScanner::new("good").with_models(vec![make_model("model1", "/path/a.gguf")]),
        );
        registry.add_scanner(MockScanner::new("bad").with_error());

        let models = registry.scan_all().await;
        // Should still get models from the good scanner
        assert_eq!(models.len(), 1);
    }

    #[tokio::test]
    async fn scan_all_handles_timeout() {
        let mut registry = ModelRegistry::with_timeout(Duration::from_millis(50));

        registry.add_scanner(
            MockScanner::new("fast").with_models(vec![make_model("model1", "/path/a.gguf")]),
        );
        registry.add_scanner(MockScanner::new("slow").with_delay(200)); // Exceeds timeout

        let models = registry.scan_all().await;
        // Should still get models from the fast scanner
        assert_eq!(models.len(), 1);
    }

    #[tokio::test]
    async fn scan_available_skips_unavailable() {
        let mut registry = ModelRegistry::new();

        registry.add_scanner(
            MockScanner::new("available").with_models(vec![make_model("model1", "/path/a.gguf")]),
        );
        registry.add_scanner(
            MockScanner::new("unavailable")
                .unavailable()
                .with_models(vec![make_model("model2", "/path/b.gguf")]),
        );

        let models = registry.scan_available().await;
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "model1");
    }

    #[tokio::test]
    async fn scan_all_with_details_returns_all_results() {
        let mut registry = ModelRegistry::with_timeout(Duration::from_millis(50));

        registry.add_scanner(
            MockScanner::new("success").with_models(vec![make_model("model1", "/path/a.gguf")]),
        );
        registry.add_scanner(MockScanner::new("error").with_error());
        registry.add_scanner(MockScanner::new("timeout").with_delay(200));

        let results = registry.scan_all_with_details().await;
        assert_eq!(results.len(), 3);

        assert!(
            results
                .iter()
                .any(|r| r.name() == "success" && r.is_success())
        );
        assert!(
            results
                .iter()
                .any(|r| matches!(r, ScanResult::Error { name: "error", .. }))
        );
        assert!(
            results
                .iter()
                .any(|r| matches!(r, ScanResult::Timeout { name: "timeout" }))
        );
    }

    #[test]
    fn scan_result_name_returns_correct_value() {
        let success = ScanResult::Success {
            name: "test",
            models: vec![],
        };
        let error = ScanResult::Error {
            name: "test",
            error: "err".to_string(),
        };
        let timeout = ScanResult::Timeout { name: "test" };
        let unavailable = ScanResult::Unavailable { name: "test" };

        assert_eq!(success.name(), "test");
        assert_eq!(error.name(), "test");
        assert_eq!(timeout.name(), "test");
        assert_eq!(unavailable.name(), "test");
    }

    #[test]
    fn scan_result_models_returns_empty_for_non_success() {
        let error = ScanResult::Error {
            name: "test",
            error: "err".to_string(),
        };
        let timeout = ScanResult::Timeout { name: "test" };
        let unavailable = ScanResult::Unavailable { name: "test" };

        assert!(error.models().is_empty());
        assert!(timeout.models().is_empty());
        assert!(unavailable.models().is_empty());
    }
}
