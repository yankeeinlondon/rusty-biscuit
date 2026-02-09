//! Ollama model scanner.
//!
//! Scans for models managed by Ollama using both the filesystem (manifest files)
//! and the REST API for enrichment.

use crate::scanner::ModelScanner;
use crate::{Config, ModelArchitecture, ModelCitizenError, ModelFormat, ModelMetadata, ModelSource, QuantizationType, UnifiedModel};
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
                let api_match = api_models.iter().find(|am| am.name == m.name);
                let (quantization, size) = api_match
                    .map(|am| (am.quantization, am.size_bytes))
                    .unwrap_or((m.quantization, m.size_bytes));

                let metadata = ModelMetadata {
                    modified_at: api_match.and_then(|am| am.modified_at.clone()),
                    ..ModelMetadata::default()
                };

                UnifiedModel::new(
                    m.name,
                    size,
                    quantization,
                    m.architecture,
                    ModelSource::Ollama,
                    ModelFormat::Gguf,
                    m.manifest_path,
                )
                .with_metadata(metadata)
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

                let modified_at = model
                    .get("modified_at")
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string());

                models.push(ApiModel {
                    name,
                    size_bytes,
                    quantization,
                    modified_at,
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

    /// Fetches detailed model info via `POST /api/show`.
    ///
    /// Called lazily during `model info` to avoid N API calls during listing.
    async fn fetch_show(&self, model_name: &str) -> Option<schematic_definitions::ollama::ShowModelResponse> {
        let url = format!("{}/api/show", self.api_host);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .ok()?;

        let body = schematic_definitions::ollama::ShowModelBody {
            name: model_name.to_string(),
            verbose: Some(true),
        };

        let response = client.post(&url).json(&body).send().await.ok()?;
        response.json().await.ok()
    }

    /// Extracts `ModelMetadata` from a `ShowModelResponse`.
    fn metadata_from_show(show: &schematic_definitions::ollama::ShowModelResponse, existing: &ModelMetadata) -> ModelMetadata {
        let mut meta = existing.clone();

        if let Some(details) = &show.details {
            if meta.parameters.is_none() {
                meta.parameters = details.parameter_size.clone();
            }
            meta.families = details.families.clone();
            meta.parent_model = details.parent_model.clone();
        }

        // License: take first line only
        if let Some(license) = &show.license {
            meta.license = license.lines().next().map(|l| {
                if l.len() > 80 {
                    format!("{}...", &l[..77])
                } else {
                    l.to_string()
                }
            });
        }

        meta.chat_template = show.template.clone();

        // Extract arch-prefixed metadata from model_info JSON
        if let Some(info) = &show.model_info {
            let arch = info
                .get("general.architecture")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            if !arch.is_empty() {
                if meta.context_length.is_none() {
                    meta.context_length = info
                        .get(format!("{arch}.context_length"))
                        .and_then(|v| v.as_u64());
                }
                if meta.embedding_length.is_none() {
                    meta.embedding_length = info
                        .get(format!("{arch}.embedding_length"))
                        .and_then(|v| v.as_u64());
                }
                if meta.head_count.is_none() {
                    meta.head_count = info
                        .get(format!("{arch}.attention.head_count"))
                        .and_then(|v| v.as_u64());
                }
                if meta.layer_count.is_none() {
                    meta.layer_count = info
                        .get(format!("{arch}.block_count"))
                        .and_then(|v| v.as_u64());
                }
            }
        }

        // Extract HuggingFace repo from model_info
        if meta.huggingface_repo.is_none() {
            if let Some(info) = &show.model_info {
                meta.huggingface_repo = info
                    .get("general.source.huggingface.repository")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        info.get("general.source.url")
                            .and_then(|v| v.as_str())
                            .and_then(crate::huggingface_repo_from_url)
                    });
            }
        }

        // Parse inference defaults from parameters string
        if let Some(params_str) = &show.parameters {
            Self::parse_parameters(params_str, &mut meta);
        }

        // Extract capabilities array
        meta.capabilities = show.capabilities.clone();

        // Derive vision/function_calling from capabilities
        if let Some(caps) = &meta.capabilities {
            if meta.vision.is_none() {
                meta.vision = Some(caps.iter().any(|c| c == "vision"));
            }
            if meta.function_calling.is_none() {
                meta.function_calling = Some(caps.iter().any(|c| c == "tools"));
            }
        }

        meta
    }

    /// Parses the newline-separated key-value `parameters` string from Ollama's
    /// `/api/show` response into typed metadata fields.
    fn parse_parameters(params_str: &str, meta: &mut ModelMetadata) {
        for line in params_str.lines() {
            let mut parts = line.split_whitespace();
            let key = match parts.next() {
                Some(k) => k,
                None => continue,
            };
            let value = match parts.next() {
                Some(v) => v,
                None => continue,
            };

            match key {
                "temperature" => meta.temperature = value.parse().ok(),
                "top_k" => meta.top_k = value.parse().ok(),
                "top_p" => meta.top_p = value.parse().ok(),
                "repeat_penalty" => meta.repeat_penalty = value.parse().ok(),
                "stop" => {
                    let cleaned = value.trim_matches('"');
                    meta.stop
                        .get_or_insert_with(Vec::new)
                        .push(cleaned.to_string());
                }
                _ => {} // Ignore unknown keys
            }
        }
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
    modified_at: Option<String>,
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

    async fn enrich(&self, model: &mut UnifiedModel) -> Result<(), ModelCitizenError> {
        if let Some(show) = self.fetch_show(&model.name).await {
            model.metadata = Self::metadata_from_show(&show, &model.metadata);
        }
        Ok(())
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
                        ModelFormat::Gguf,
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

    #[test]
    fn parse_parameters_extracts_temperature_and_top_p() {
        let params = "temperature 0.6\ntop_p 0.95\n";
        let mut meta = ModelMetadata::default();
        OllamaScanner::parse_parameters(params, &mut meta);
        assert_eq!(meta.temperature, Some(0.6));
        assert_eq!(meta.top_p, Some(0.95));
        assert!(meta.top_k.is_none());
        assert!(meta.repeat_penalty.is_none());
    }

    #[test]
    fn parse_parameters_extracts_all_fields() {
        let params = "temperature 0.6\ntop_p 0.95\ntop_k 20\nrepeat_penalty 1\n";
        let mut meta = ModelMetadata::default();
        OllamaScanner::parse_parameters(params, &mut meta);
        assert_eq!(meta.temperature, Some(0.6));
        assert_eq!(meta.top_p, Some(0.95));
        assert_eq!(meta.top_k, Some(20));
        assert_eq!(meta.repeat_penalty, Some(1.0));
    }

    #[test]
    fn parse_parameters_accumulates_stop_tokens() {
        let params = "stop \"<|start_header_id|>\"\nstop \"<|end_header_id|>\"\nstop \"<|eot_id|>\"\n";
        let mut meta = ModelMetadata::default();
        OllamaScanner::parse_parameters(params, &mut meta);
        let stops = meta.stop.unwrap();
        assert_eq!(stops.len(), 3);
        assert_eq!(stops[0], "<|start_header_id|>");
        assert_eq!(stops[1], "<|end_header_id|>");
        assert_eq!(stops[2], "<|eot_id|>");
    }

    #[test]
    fn parse_parameters_handles_empty_string() {
        let mut meta = ModelMetadata::default();
        OllamaScanner::parse_parameters("", &mut meta);
        assert!(meta.temperature.is_none());
        assert!(meta.stop.is_none());
    }

    #[test]
    fn parse_parameters_ignores_unknown_keys() {
        let params = "num_ctx 4096\ntemperature 0.7\n";
        let mut meta = ModelMetadata::default();
        OllamaScanner::parse_parameters(params, &mut meta);
        assert_eq!(meta.temperature, Some(0.7));
    }

    fn empty_show_response() -> schematic_definitions::ollama::ShowModelResponse {
        schematic_definitions::ollama::ShowModelResponse {
            modelfile: None,
            parameters: None,
            template: None,
            system: None,
            details: None,
            model_info: None,
            license: None,
            capabilities: None,
        }
    }

    #[test]
    fn metadata_from_show_extracts_huggingface_repo() {
        let mut show = empty_show_response();
        show.model_info = Some(serde_json::json!({
            "general.architecture": "llama",
            "general.source.huggingface.repository": "meta-llama/Llama-3-8B-GGUF",
            "llama.context_length": 8192
        }));

        let meta = OllamaScanner::metadata_from_show(&show, &ModelMetadata::default());
        assert_eq!(
            meta.huggingface_repo,
            Some("meta-llama/Llama-3-8B-GGUF".to_string()),
        );
    }

    #[test]
    fn metadata_from_show_extracts_huggingface_repo_from_source_url() {
        let mut show = empty_show_response();
        show.model_info = Some(serde_json::json!({
            "general.architecture": "llama",
            "general.source.url": "https://huggingface.co/meta-llama/Llama-3.3-70B-Instruct",
            "llama.context_length": 131072
        }));

        let meta = OllamaScanner::metadata_from_show(&show, &ModelMetadata::default());
        assert_eq!(
            meta.huggingface_repo,
            Some("meta-llama/Llama-3.3-70B-Instruct".to_string()),
        );
    }

    #[test]
    fn metadata_from_show_no_huggingface_repo() {
        let mut show = empty_show_response();
        show.model_info = Some(serde_json::json!({
            "general.architecture": "llama",
            "llama.context_length": 8192
        }));

        let meta = OllamaScanner::metadata_from_show(&show, &ModelMetadata::default());
        assert!(meta.huggingface_repo.is_none());
    }

    #[test]
    fn parse_parameters_skips_lines_without_value() {
        let params = "temperature\ntop_p 0.9\n";
        let mut meta = ModelMetadata::default();
        OllamaScanner::parse_parameters(params, &mut meta);
        assert!(meta.temperature.is_none());
        assert_eq!(meta.top_p, Some(0.9));
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
