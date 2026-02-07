//! Ollama model scanner.
//!
//! Scans for models managed by Ollama using both the filesystem (manifest files)
//! and the REST API for enrichment.

use crate::scanner::ModelScanner;
use crate::{Config, ModelArchitecture, ModelCitizenError, ModelSource, QuantizationType, UnifiedModel};
use async_trait::async_trait;
use std::path::PathBuf;

/// Scanner for Ollama models.
///
/// Ollama stores models in a manifest-based format with blob storage.
/// This scanner reads the manifests to discover models and optionally
/// enriches metadata via the Ollama API.
pub struct OllamaScanner {
    /// Base path for Ollama models directory.
    models_dir: PathBuf,
    /// API host URL.
    api_host: String,
    /// Request timeout in seconds.
    timeout_secs: u64,
}

impl OllamaScanner {
    /// Creates a new Ollama scanner with the given configuration.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        Self {
            models_dir: Self::default_models_dir(),
            api_host: config.ollama_host().to_string(),
            timeout_secs: config.scanners.ollama.timeout_secs,
        }
    }

    /// Creates a scanner with custom paths (for testing).
    #[must_use]
    pub fn with_paths(models_dir: PathBuf, api_host: String) -> Self {
        Self {
            models_dir,
            api_host,
            timeout_secs: 5,
        }
    }

    /// Returns the default Ollama models directory for the current platform.
    #[must_use]
    pub fn default_models_dir() -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir()
                .map(|h| h.join(".ollama/models"))
                .unwrap_or_else(|| PathBuf::from("/var/lib/ollama/models"))
        }
        #[cfg(target_os = "linux")]
        {
            // Check for system-wide installation first
            let system_path = PathBuf::from("/usr/share/ollama/.ollama/models");
            if system_path.exists() {
                return system_path;
            }
            dirs::home_dir()
                .map(|h| h.join(".ollama/models"))
                .unwrap_or_else(|| PathBuf::from("/var/lib/ollama/models"))
        }
        #[cfg(target_os = "windows")]
        {
            dirs::data_local_dir()
                .map(|d| d.join("Ollama/models"))
                .unwrap_or_else(|| PathBuf::from("C:\\Users\\Public\\.ollama\\models"))
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            PathBuf::from(".ollama/models")
        }
    }

    /// Scans the filesystem for Ollama manifests.
    fn scan_filesystem(&self) -> Result<Vec<OllamaModel>, ModelCitizenError> {
        let manifests_dir = self.models_dir.join("manifests");
        if !manifests_dir.exists() {
            return Ok(Vec::new());
        }

        let mut models = Vec::new();

        // Structure: manifests/<registry>/<namespace>/<name>/<tag>
        // Most common: manifests/registry.ollama.ai/library/<model>/<tag>
        Self::walk_manifests(&manifests_dir, &mut models)?;

        Ok(models)
    }

    /// Recursively walks manifest directories.
    fn walk_manifests(dir: &std::path::Path, models: &mut Vec<OllamaModel>) -> Result<(), ModelCitizenError> {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                Self::walk_manifests(&path, models)?;
            } else if path.is_file() {
                // This is a manifest file (the tag)
                if let Some(model) = Self::parse_manifest(&path) {
                    models.push(model);
                }
            }
        }

        Ok(())
    }

    /// Parses a manifest file to extract model information.
    fn parse_manifest(path: &std::path::Path) -> Option<OllamaModel> {
        let content = std::fs::read_to_string(path).ok()?;
        let manifest: serde_json::Value = serde_json::from_str(&content).ok()?;

        // Extract model name from path
        // Path pattern: manifests/<registry>/<namespace>/<model>/<tag>
        let components: Vec<_> = path.components().collect();
        let len = components.len();
        if len < 2 {
            return None;
        }

        let tag = components[len - 1].as_os_str().to_str()?;
        let model_name = components[len - 2].as_os_str().to_str()?;

        // Get size from manifest layers
        let mut total_size = 0u64;
        if let Some(layers) = manifest.get("layers").and_then(|l| l.as_array()) {
            for layer in layers {
                if let Some(size) = layer.get("size").and_then(|s| s.as_u64()) {
                    total_size += size;
                }
            }
        }

        // Format: "model:tag"
        let full_name = if tag == "latest" {
            model_name.to_string()
        } else {
            format!("{}:{}", model_name, tag)
        };

        Some(OllamaModel {
            name: full_name,
            size_bytes: total_size,
            quantization: QuantizationType::Unknown, // Will be enriched by API
            architecture: ModelArchitecture::from_name(model_name),
            manifest_path: path.to_path_buf(),
        })
    }

    /// Enriches models with API data if available.
    async fn enrich_from_api(&self, models: Vec<OllamaModel>) -> Vec<UnifiedModel> {
        // Try to get API data
        let api_models = self.fetch_api_models().await;

        models
            .into_iter()
            .map(|m| {
                // Look for API match
                let (quantization, size) = api_models
                    .iter()
                    .find(|am| am.name == m.name)
                    .map(|am| (am.quantization, am.size_bytes))
                    .unwrap_or((m.quantization, m.size_bytes));

                UnifiedModel::new(
                    m.name,
                    size,
                    quantization,
                    m.architecture,
                    ModelSource::Ollama,
                    m.manifest_path,
                )
            })
            .collect()
    }

    /// Fetches model list from the Ollama API.
    async fn fetch_api_models(&self) -> Vec<ApiModel> {
        let url = format!("{}/api/tags", self.api_host);

        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
        {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let response = match client.get(&url).send().await {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        let json: serde_json::Value = match response.json().await {
            Ok(j) => j,
            Err(_) => return Vec::new(),
        };

        let mut models = Vec::new();

        if let Some(model_list) = json.get("models").and_then(|m| m.as_array()) {
            for model in model_list {
                let name = model
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or_default()
                    .to_string();

                let size_bytes = model.get("size").and_then(|s| s.as_u64()).unwrap_or(0);

                let quantization = model
                    .get("details")
                    .and_then(|d| d.get("quantization_level"))
                    .and_then(|q| q.as_str())
                    .map(QuantizationType::from_str_loose)
                    .unwrap_or(QuantizationType::Unknown);

                models.push(ApiModel {
                    name,
                    size_bytes,
                    quantization,
                });
            }
        }

        models
    }

    /// Checks if Ollama API is reachable.
    async fn check_api_available(&self) -> bool {
        let url = format!("{}/api/tags", self.api_host);

        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };

        client.get(&url).send().await.is_ok()
    }
}

/// Internal representation of an Ollama model from filesystem.
struct OllamaModel {
    name: String,
    size_bytes: u64,
    quantization: QuantizationType,
    architecture: ModelArchitecture,
    manifest_path: PathBuf,
}

/// Internal representation of an Ollama model from API.
struct ApiModel {
    name: String,
    size_bytes: u64,
    quantization: QuantizationType,
}

#[async_trait]
impl ModelScanner for OllamaScanner {
    fn name(&self) -> &'static str {
        "ollama"
    }

    async fn is_available(&self) -> bool {
        // Check filesystem OR API
        self.models_dir.exists() || self.check_api_available().await
    }

    async fn scan(&self) -> Result<Vec<UnifiedModel>, ModelCitizenError> {
        // Try filesystem first
        let fs_models = self.scan_filesystem()?;

        if fs_models.is_empty() {
            // Fallback to API-only if no filesystem models
            let api_models = self.fetch_api_models().await;
            return Ok(api_models
                .into_iter()
                .map(|m| {
                    UnifiedModel::new(
                        m.name.clone(),
                        m.size_bytes,
                        m.quantization,
                        ModelArchitecture::from_name(&m.name),
                        ModelSource::Ollama,
                        PathBuf::new(), // No path for API-only
                    )
                })
                .collect());
        }

        // Enrich filesystem models with API data
        Ok(self.enrich_from_api(fs_models).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_manifest(dir: &std::path::Path, model: &str, tag: &str, size: u64) {
        let path = dir
            .join("manifests")
            .join("registry.ollama.ai")
            .join("library")
            .join(model);
        std::fs::create_dir_all(&path).unwrap();

        let manifest = serde_json::json!({
            "schemaVersion": 2,
            "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
            "layers": [
                {
                    "mediaType": "application/vnd.ollama.image.model",
                    "digest": "sha256:abc123",
                    "size": size
                }
            ]
        });

        let manifest_path = path.join(tag);
        let mut file = std::fs::File::create(manifest_path).unwrap();
        file.write_all(manifest.to_string().as_bytes()).unwrap();
    }

    #[test]
    fn default_models_dir_returns_path() {
        let path = OllamaScanner::default_models_dir();
        assert!(!path.as_os_str().is_empty());
    }

    #[test]
    fn scanner_name_is_ollama() {
        let config = Config::default();
        let scanner = OllamaScanner::new(&config);
        assert_eq!(scanner.name(), "ollama");
    }

    #[test]
    fn scan_filesystem_returns_empty_for_missing_dir() {
        let scanner = OllamaScanner::with_paths(
            PathBuf::from("/nonexistent/ollama/models"),
            "http://localhost:11434".to_string(),
        );
        let models = scanner.scan_filesystem().unwrap();
        assert!(models.is_empty());
    }

    #[test]
    fn scan_filesystem_finds_manifests() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_manifest(temp_dir.path(), "llama3", "latest", 4_000_000_000);
        create_test_manifest(temp_dir.path(), "mistral", "7b", 7_000_000_000);

        let scanner = OllamaScanner::with_paths(
            temp_dir.path().to_path_buf(),
            "http://localhost:11434".to_string(),
        );

        let models = scanner.scan_filesystem().unwrap();
        assert_eq!(models.len(), 2);

        let names: Vec<_> = models.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"llama3"));
        assert!(names.contains(&"mistral:7b"));
    }

    #[test]
    fn parse_manifest_extracts_size() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_manifest(temp_dir.path(), "test-model", "v1", 5_000_000_000);

        let manifest_path = temp_dir
            .path()
            .join("manifests/registry.ollama.ai/library/test-model/v1");

        let model = OllamaScanner::parse_manifest(&manifest_path).unwrap();
        assert_eq!(model.name, "test-model:v1");
        assert_eq!(model.size_bytes, 5_000_000_000);
    }

    #[tokio::test]
    async fn is_available_returns_true_for_existing_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let scanner = OllamaScanner::with_paths(
            temp_dir.path().to_path_buf(),
            "http://localhost:99999".to_string(), // Invalid port
        );

        // Should be available because directory exists
        assert!(scanner.is_available().await);
    }

    #[tokio::test]
    async fn is_available_returns_false_for_missing_dir_and_api() {
        let scanner = OllamaScanner::with_paths(
            PathBuf::from("/nonexistent/path"),
            "http://localhost:99999".to_string(),
        );

        assert!(!scanner.is_available().await);
    }

    #[tokio::test]
    async fn scan_returns_models_from_filesystem() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_manifest(temp_dir.path(), "llama3", "latest", 4_000_000_000);

        let scanner = OllamaScanner::with_paths(
            temp_dir.path().to_path_buf(),
            "http://localhost:99999".to_string(),
        );

        let models = scanner.scan().await.unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "llama3");
        assert_eq!(models[0].source, ModelSource::Ollama);
    }
}
