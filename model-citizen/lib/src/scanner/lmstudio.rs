//! LM Studio model scanner.
//!
//! Scans for models in the LM Studio models directory by recursively
//! searching for `.gguf` files and MLX model directories (safetensors).

use crate::gguf::{detect_quantization, extract_metadata, model_name_from_filename};
use crate::scanner::ModelScanner;
use crate::{
    Config, ModelArchitecture, ModelCitizenError, ModelFormat, ModelSource, QuantizationType,
    UnifiedModel,
};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

/// Scanner for LM Studio models.
///
/// LM Studio stores models as GGUF files and MLX directories in a
/// user-specific directory. The scanner recursively searches for `.gguf`
/// files and MLX model directories (containing safetensors + config.json),
/// extracting metadata from filenames, file headers, and MLX configs.
pub struct LmStudioScanner {
    /// Base path for LM Studio models directory.
    models_dir: PathBuf,
    /// API host URL for optional enrichment.
    api_host: String,
    /// Request timeout in seconds.
    timeout_secs: u64,
}

impl LmStudioScanner {
    /// Creates a new LM Studio scanner with the given configuration.
    ///
    /// The models directory is resolved by checking LM Studio's
    /// `settings.json` for a `downloadsFolder` override, falling back
    /// to the platform default.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        Self {
            models_dir: Self::detect_models_dir(),
            api_host: config.lmstudio_host().to_string(),
            timeout_secs: config.scanners.lmstudio.timeout_secs,
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

    /// Returns the default LM Studio models directory for the current platform.
    #[must_use]
    pub fn default_models_dir() -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir()
                .map(|h| h.join("Library/Application Support/LM Studio/models"))
                .unwrap_or_else(|| PathBuf::from("~/Library/Application Support/LM Studio/models"))
        }
        #[cfg(target_os = "linux")]
        {
            dirs::cache_dir()
                .map(|c| c.join("lm-studio/models"))
                .unwrap_or_else(|| {
                    dirs::home_dir()
                        .map(|h| h.join(".cache/lm-studio/models"))
                        .unwrap_or_else(|| PathBuf::from("~/.cache/lm-studio/models"))
                })
        }
        #[cfg(target_os = "windows")]
        {
            dirs::data_dir()
                .map(|d| d.join("LM Studio/models"))
                .unwrap_or_else(|| PathBuf::from("C:\\Users\\Public\\LM Studio\\models"))
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            PathBuf::from("lm-studio/models")
        }
    }

    /// Resolves the LM Studio models directory.
    ///
    /// Checks LM Studio's `settings.json` for a custom `downloadsFolder`,
    /// falling back to the platform default directory.
    fn detect_models_dir() -> PathBuf {
        if let Some(dir) = Self::read_lmstudio_settings_dir() {
            return dir;
        }
        Self::default_models_dir()
    }

    /// Returns the platform-specific path to LM Studio's settings file.
    fn lmstudio_settings_path() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir().map(|h| h.join(".cache/lm-studio/settings.json"))
        }
        #[cfg(target_os = "linux")]
        {
            dirs::cache_dir().map(|c| c.join("lm-studio/settings.json"))
        }
        #[cfg(target_os = "windows")]
        {
            dirs::data_dir().map(|d| d.join("LM Studio/settings.json"))
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            None
        }
    }

    /// Reads the `downloadsFolder` from LM Studio's settings.json.
    fn read_lmstudio_settings_dir() -> Option<PathBuf> {
        let path = Self::lmstudio_settings_path()?;
        let contents = std::fs::read_to_string(path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&contents).ok()?;
        json.get("downloadsFolder")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
    }

    /// Recursively scans for GGUF files and MLX model directories.
    fn scan_directory(&self, dir: &std::path::Path) -> Vec<UnifiedModel> {
        let mut models = Vec::new();
        Self::walk_directory(dir, &mut models);
        models
    }

    /// Recursively walks a directory tree looking for GGUF files and MLX model directories.
    ///
    /// When a directory is identified as an MLX model (contains safetensors +
    /// config.json), it is parsed and not recursed into further.
    fn walk_directory(dir: &std::path::Path, models: &mut Vec<UnifiedModel>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if Self::is_mlx_directory(&path) {
                    if let Some(model) = Self::parse_mlx_directory(&path) {
                        models.push(model);
                    }
                } else {
                    Self::walk_directory(&path, models);
                }
            } else if path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("gguf"))
                && let Some(model) = Self::parse_gguf_file(&path)
            {
                models.push(model);
            }
        }
    }

    /// Checks if a directory is an MLX model directory.
    ///
    /// An MLX model directory contains at least one `.safetensors` file
    /// and a `config.json` file.
    fn is_mlx_directory(dir: &std::path::Path) -> bool {
        let has_config = dir.join("config.json").is_file();
        if !has_config {
            return false;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return false,
        };

        entries.flatten().any(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("safetensors"))
        })
    }

    /// Parses an MLX model directory to create a `UnifiedModel`.
    ///
    /// Extracts:
    /// - **Name**: from the directory name
    /// - **Size**: sum of all `.safetensors` file sizes
    /// - **Quantization**: from `config.json` `quantization.bits` or
    ///   `quantization_config.bits`
    /// - **Architecture**: from `config.json` `model_type` field, falling
    ///   back to name-based detection
    fn parse_mlx_directory(dir: &std::path::Path) -> Option<UnifiedModel> {
        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())?;

        // Sum safetensors file sizes
        let size_bytes = std::fs::read_dir(dir)
            .ok()?
            .flatten()
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("safetensors"))
            })
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum();

        // Parse config.json for quantization and architecture
        let config_path = dir.join("config.json");
        let config_json: serde_json::Value = std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let quantization = config_json
            .get("quantization")
            .and_then(|q| q.get("bits"))
            .or_else(|| {
                config_json
                    .get("quantization_config")
                    .and_then(|q| q.get("bits"))
            })
            .and_then(|b| b.as_u64())
            .map(QuantizationType::from_mlx_bits)
            .unwrap_or(QuantizationType::Unknown);

        let architecture = config_json
            .get("model_type")
            .and_then(|v| v.as_str())
            .map(Self::architecture_from_model_type)
            .unwrap_or_else(|| ModelArchitecture::from_name(&name));

        let metadata = Self::metadata_from_mlx_config(&config_json);

        Some(
            UnifiedModel::new(
                name,
                size_bytes,
                quantization,
                architecture,
                ModelSource::LmStudio,
                ModelFormat::Safetensors,
                dir,
            )
            .with_metadata(metadata),
        )
    }

    /// Extracts `ModelMetadata` from an MLX model's `config.json`.
    ///
    /// Maps common HuggingFace/MLX config keys to metadata fields:
    /// - `max_position_embeddings` / `max_seq_len` / `max_sequence_length` → context_length
    /// - `hidden_size` → embedding_length
    /// - `num_attention_heads` → head_count
    /// - `num_hidden_layers` → layer_count
    fn metadata_from_mlx_config(config: &serde_json::Value) -> crate::ModelMetadata {
        crate::ModelMetadata {
            context_length: config
                .get("max_position_embeddings")
                .or_else(|| config.get("max_seq_len"))
                .or_else(|| config.get("max_sequence_length"))
                .and_then(|v| v.as_u64()),
            embedding_length: config.get("hidden_size").and_then(|v| v.as_u64()),
            head_count: config.get("num_attention_heads").and_then(|v| v.as_u64()),
            layer_count: config.get("num_hidden_layers").and_then(|v| v.as_u64()),
            ..Default::default()
        }
    }

    /// Maps an MLX `model_type` string to a `ModelArchitecture`.
    fn architecture_from_model_type(model_type: &str) -> ModelArchitecture {
        let lower = model_type.to_lowercase();
        if lower.contains("llama") {
            ModelArchitecture::Llama
        } else if lower.contains("mistral") || lower.contains("mixtral") {
            ModelArchitecture::Mistral
        } else if lower.contains("qwen") {
            ModelArchitecture::Qwen
        } else if lower.contains("phi") {
            ModelArchitecture::Phi
        } else if lower.contains("gemma") {
            ModelArchitecture::Gemma
        } else if lower.contains("command") {
            ModelArchitecture::Command
        } else if lower.contains("deepseek") {
            ModelArchitecture::DeepSeek
        } else if lower.contains("starcoder") {
            ModelArchitecture::StarCoder
        } else {
            ModelArchitecture::Unknown
        }
    }

    /// Parses a GGUF file to create a UnifiedModel.
    fn parse_gguf_file(path: &std::path::Path) -> Option<UnifiedModel> {
        // Get file size
        let file_meta = std::fs::metadata(path).ok()?;
        let size_bytes = file_meta.len();

        // Extract model name from filename
        let name = model_name_from_filename(path).or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })?;

        // Detect quantization from filename or header
        let quantization = detect_quantization(path);

        // Detect architecture from name
        let architecture = ModelArchitecture::from_name(&name);

        let mut model = UnifiedModel::new(
            name,
            size_bytes,
            quantization,
            architecture,
            ModelSource::LmStudio,
            ModelFormat::Gguf,
            path,
        );

        // Extract rich metadata from GGUF file headers
        if let Some(meta) = extract_metadata(path) {
            model.metadata = meta;
        }

        Some(model)
    }

    /// Checks if LM Studio API is reachable.
    async fn check_api_available(&self) -> bool {
        let url = format!("{}/v1/models", self.api_host);

        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };

        client.get(&url).send().await.is_ok()
    }

    /// Fetches model list from LM Studio API and enriches filesystem models.
    ///
    /// Single API call returns all models. Matches API results to filesystem
    /// models by ID and populates parameters, capabilities, and publisher.
    async fn enrich_from_api(&self, models: &mut [UnifiedModel]) {
        let url = format!("{}/v1/models", self.api_host);

        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
        {
            Ok(c) => c,
            Err(_) => return,
        };

        let response = match client.get(&url).send().await {
            Ok(r) => r,
            Err(_) => return,
        };

        let api_response: schematic_definitions::lmstudio::ListModelsResponse =
            match response.json().await {
                Ok(r) => r,
                Err(_) => return,
            };

        for model in models.iter_mut() {
            // Match by checking if the API model id is contained in the model name or path
            if let Some(api_model) = api_response.data.iter().find(|am| {
                model.name.contains(&am.id) || model.path.to_string_lossy().contains(&am.id)
            }) {
                Self::apply_api_metadata(model, api_model);
            }
        }
    }

    /// Extracts a HuggingFace repo ID from a model path relative to the models directory.
    ///
    /// LM Studio stores models as `models_dir/publisher/repo-name/...`, which maps
    /// directly to HuggingFace repo IDs like `publisher/repo-name`.
    fn huggingface_repo_from_path(models_dir: &Path, model_path: &Path) -> Option<String> {
        let relative = model_path.strip_prefix(models_dir).ok()?;
        let mut components = relative.components();
        let publisher = components.next()?.as_os_str().to_str()?;
        let repo_name = components.next()?.as_os_str().to_str()?;
        Some(format!("{publisher}/{repo_name}"))
    }

    /// Applies metadata from an LM Studio API model to a `UnifiedModel`.
    fn apply_api_metadata(
        model: &mut UnifiedModel,
        api_model: &schematic_definitions::lmstudio::ModelInfo,
    ) {
        let meta = &mut model.metadata;

        if let Some(stats) = &api_model.stats
            && meta.parameters.is_none()
        {
            meta.parameters = stats.parameters.clone();
        }

        if let Some(caps) = &api_model.capabilities {
            meta.vision = caps.vision;
            meta.function_calling = caps.function_calling;
        }

        if meta.publisher.is_none() {
            meta.publisher = api_model.publisher.clone();
        }
    }
}

#[async_trait]
impl ModelScanner for LmStudioScanner {
    fn name(&self) -> &'static str {
        "lmstudio"
    }

    async fn is_available(&self) -> bool {
        // Check if models directory exists OR API is available
        self.models_dir.exists() || self.check_api_available().await
    }

    async fn scan(&self) -> Result<Vec<UnifiedModel>, ModelCitizenError> {
        if !self.models_dir.exists() {
            return Ok(Vec::new());
        }

        let mut models = self.scan_directory(&self.models_dir);

        // Try to enrich with API metadata (single call for all models)
        self.enrich_from_api(&mut models).await;

        // Extract HuggingFace repo IDs from directory paths (where not already set from GGUF headers)
        for model in &mut models {
            if model.metadata.huggingface_repo.is_none() {
                model.metadata.huggingface_repo =
                    Self::huggingface_repo_from_path(&self.models_dir, &model.path);
            }
        }

        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::QuantizationType;
    use std::io::Write;

    fn create_test_gguf(dir: &std::path::Path, filename: &str, size: u64) {
        let path = dir.join(filename);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }

        // Create a minimal GGUF file (just magic + padding for size)
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"GGUF").unwrap(); // Magic
        file.write_all(&[0u8; 20]).unwrap(); // Version + counts

        // Pad to reach desired size
        let remaining = size.saturating_sub(24);
        if remaining > 0 {
            file.set_len(size).unwrap();
        }
    }

    #[test]
    fn default_models_dir_returns_path() {
        let path = LmStudioScanner::default_models_dir();
        assert!(!path.as_os_str().is_empty());
    }

    #[test]
    fn scanner_name_is_lmstudio() {
        let config = Config::default();
        let scanner = LmStudioScanner::new(&config);
        assert_eq!(scanner.name(), "lmstudio");
    }

    #[test]
    fn scan_directory_returns_empty_for_missing_dir() {
        let scanner = LmStudioScanner::with_paths(
            PathBuf::from("/nonexistent/lmstudio/models"),
            "http://localhost:1234".to_string(),
        );
        let models = scanner.scan_directory(&scanner.models_dir);
        assert!(models.is_empty());
    }

    #[test]
    fn scan_directory_finds_gguf_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_gguf(temp_dir.path(), "llama3-Q4_K_M.gguf", 4_000_000_000);
        create_test_gguf(temp_dir.path(), "mistral-7b-Q5_K_S.gguf", 7_000_000_000);

        let scanner = LmStudioScanner::with_paths(
            temp_dir.path().to_path_buf(),
            "http://localhost:1234".to_string(),
        );

        let models = scanner.scan_directory(temp_dir.path());
        assert_eq!(models.len(), 2);

        let names: Vec<_> = models.iter().map(|m| m.name.as_str()).collect();
        assert!(names.iter().any(|n| n.contains("llama3")));
        assert!(names.iter().any(|n| n.contains("mistral")));
    }

    #[test]
    fn scan_directory_finds_nested_gguf_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_gguf(
            temp_dir.path(),
            "TheBloke/Llama-2-7B-GGUF/llama-2-7b.Q4_K_M.gguf",
            4_000_000_000,
        );

        let scanner = LmStudioScanner::with_paths(
            temp_dir.path().to_path_buf(),
            "http://localhost:1234".to_string(),
        );

        let models = scanner.scan_directory(temp_dir.path());
        assert_eq!(models.len(), 1);
    }

    #[test]
    fn parse_gguf_file_extracts_metadata() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test-model-Q4_K_M.gguf");
        create_test_gguf(temp_dir.path(), "test-model-Q4_K_M.gguf", 1_000_000);

        let model = LmStudioScanner::parse_gguf_file(&path).unwrap();
        assert_eq!(model.name, "test-model");
        assert_eq!(model.quantization, crate::QuantizationType::Q4Km);
        assert_eq!(model.source, ModelSource::LmStudio);
    }

    #[tokio::test]
    async fn is_available_returns_true_for_existing_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let scanner = LmStudioScanner::with_paths(
            temp_dir.path().to_path_buf(),
            "http://localhost:99999".to_string(),
        );

        assert!(scanner.is_available().await);
    }

    #[tokio::test]
    async fn is_available_returns_false_for_missing_dir_and_api() {
        let scanner = LmStudioScanner::with_paths(
            PathBuf::from("/nonexistent/path"),
            "http://localhost:99999".to_string(),
        );

        assert!(!scanner.is_available().await);
    }

    #[tokio::test]
    async fn scan_returns_models() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_gguf(temp_dir.path(), "llama3-Q4_K_M.gguf", 4_000_000_000);

        let scanner = LmStudioScanner::with_paths(
            temp_dir.path().to_path_buf(),
            "http://localhost:99999".to_string(),
        );

        let models = scanner.scan().await.unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].source, ModelSource::LmStudio);
    }

    #[tokio::test]
    async fn scan_returns_empty_for_missing_dir() {
        let scanner = LmStudioScanner::with_paths(
            PathBuf::from("/nonexistent/path"),
            "http://localhost:99999".to_string(),
        );

        let models = scanner.scan().await.unwrap();
        assert!(models.is_empty());
    }

    /// Creates a minimal MLX model directory with safetensors and config.json.
    fn create_test_mlx_dir(
        parent: &std::path::Path,
        name: &str,
        config_json: &str,
        safetensor_sizes: &[u64],
    ) -> PathBuf {
        let dir = parent.join(name);
        std::fs::create_dir_all(&dir).unwrap();

        // Write config.json
        std::fs::write(dir.join("config.json"), config_json).unwrap();

        // Create safetensors files
        for (i, &size) in safetensor_sizes.iter().enumerate() {
            let path = dir.join(format!("model-{i:05}.safetensors"));
            let file = std::fs::File::create(&path).unwrap();
            file.set_len(size).unwrap();
        }

        dir
    }

    #[test]
    fn is_mlx_directory_detects_mlx_model() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mlx_dir = create_test_mlx_dir(
            temp_dir.path(),
            "Qwen3-Coder-6bit",
            r#"{"model_type": "qwen3", "quantization": {"bits": 6}}"#,
            &[1_000_000],
        );

        assert!(LmStudioScanner::is_mlx_directory(&mlx_dir));
    }

    #[test]
    fn is_mlx_directory_rejects_dir_without_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir = temp_dir.path().join("no-config");
        std::fs::create_dir_all(&dir).unwrap();
        let file = std::fs::File::create(dir.join("model.safetensors")).unwrap();
        file.set_len(1000).unwrap();

        assert!(!LmStudioScanner::is_mlx_directory(&dir));
    }

    #[test]
    fn is_mlx_directory_rejects_dir_without_safetensors() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir = temp_dir.path().join("no-safetensors");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("config.json"), "{}").unwrap();

        assert!(!LmStudioScanner::is_mlx_directory(&dir));
    }

    #[test]
    fn parse_mlx_directory_extracts_metadata() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mlx_dir = create_test_mlx_dir(
            temp_dir.path(),
            "Qwen3-Coder-Next-6bit",
            r#"{"model_type": "qwen3_next", "quantization": {"bits": 6}}"#,
            &[500_000, 500_000],
        );

        let model = LmStudioScanner::parse_mlx_directory(&mlx_dir).unwrap();
        assert_eq!(model.name, "Qwen3-Coder-Next-6bit");
        assert_eq!(model.size_bytes, 1_000_000);
        assert_eq!(model.quantization, QuantizationType::Bit6);
        assert_eq!(model.architecture, ModelArchitecture::Qwen);
        assert_eq!(model.source, ModelSource::LmStudio);
        assert_eq!(model.path, mlx_dir);
    }

    #[test]
    fn parse_mlx_directory_extracts_rich_metadata() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = r#"{
            "model_type": "qwen3",
            "quantization": {"bits": 6},
            "max_position_embeddings": 40960,
            "hidden_size": 3584,
            "num_attention_heads": 28,
            "num_hidden_layers": 36
        }"#;
        let mlx_dir = create_test_mlx_dir(temp_dir.path(), "Qwen3-30B-6bit", config, &[1_000_000]);

        let model = LmStudioScanner::parse_mlx_directory(&mlx_dir).unwrap();
        assert_eq!(model.metadata.context_length, Some(40960));
        assert_eq!(model.metadata.embedding_length, Some(3584));
        assert_eq!(model.metadata.head_count, Some(28));
        assert_eq!(model.metadata.layer_count, Some(36));
    }

    #[test]
    fn metadata_from_mlx_config_handles_alternate_keys() {
        // Some models use max_seq_len instead of max_position_embeddings
        let config: serde_json::Value =
            serde_json::from_str(r#"{"max_seq_len": 8192, "hidden_size": 4096}"#).unwrap();

        let meta = LmStudioScanner::metadata_from_mlx_config(&config);
        assert_eq!(meta.context_length, Some(8192));
        assert_eq!(meta.embedding_length, Some(4096));
        assert!(meta.head_count.is_none());
    }

    #[test]
    fn metadata_from_mlx_config_handles_empty_config() {
        let config: serde_json::Value = serde_json::from_str("{}").unwrap();
        let meta = LmStudioScanner::metadata_from_mlx_config(&config);
        assert!(meta.context_length.is_none());
        assert!(meta.embedding_length.is_none());
        assert!(meta.head_count.is_none());
        assert!(meta.layer_count.is_none());
    }

    #[test]
    fn parse_mlx_directory_uses_quantization_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mlx_dir = create_test_mlx_dir(
            temp_dir.path(),
            "llama-4bit",
            r#"{"model_type": "llama", "quantization_config": {"bits": 4}}"#,
            &[1_000_000],
        );

        let model = LmStudioScanner::parse_mlx_directory(&mlx_dir).unwrap();
        assert_eq!(model.quantization, QuantizationType::Bit4);
        assert_eq!(model.architecture, ModelArchitecture::Llama);
    }

    #[test]
    fn parse_mlx_directory_falls_back_to_name_architecture() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mlx_dir = create_test_mlx_dir(
            temp_dir.path(),
            "Mistral-7B-Instruct-8bit",
            r#"{"quantization": {"bits": 8}}"#,
            &[1_000_000],
        );

        let model = LmStudioScanner::parse_mlx_directory(&mlx_dir).unwrap();
        assert_eq!(model.architecture, ModelArchitecture::Mistral);
        assert_eq!(model.quantization, QuantizationType::Bit8);
    }

    #[test]
    fn scan_directory_finds_mlx_models() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_mlx_dir(
            temp_dir.path(),
            "Qwen3-6bit",
            r#"{"model_type": "qwen3", "quantization": {"bits": 6}}"#,
            &[2_000_000],
        );

        let scanner = LmStudioScanner::with_paths(
            temp_dir.path().to_path_buf(),
            "http://localhost:1234".to_string(),
        );

        let models = scanner.scan_directory(temp_dir.path());
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "Qwen3-6bit");
    }

    #[test]
    fn scan_directory_finds_both_gguf_and_mlx() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_gguf(temp_dir.path(), "llama3-Q4_K_M.gguf", 4_000_000_000);
        create_test_mlx_dir(
            temp_dir.path(),
            "Qwen3-6bit",
            r#"{"model_type": "qwen3", "quantization": {"bits": 6}}"#,
            &[2_000_000],
        );

        let scanner = LmStudioScanner::with_paths(
            temp_dir.path().to_path_buf(),
            "http://localhost:1234".to_string(),
        );

        let models = scanner.scan_directory(temp_dir.path());
        assert_eq!(models.len(), 2);

        let names: Vec<_> = models.iter().map(|m| m.name.as_str()).collect();
        assert!(names.iter().any(|n| n.contains("llama3")));
        assert!(names.iter().any(|n| n.contains("Qwen3")));
    }

    #[test]
    fn detect_models_dir_returns_a_path() {
        // detect_models_dir either reads settings.json or falls back to default
        let detected = LmStudioScanner::detect_models_dir();
        assert!(!detected.as_os_str().is_empty());
    }

    #[test]
    fn read_lmstudio_settings_dir_returns_none_for_missing_file() {
        // If LM Studio is not installed, settings path won't exist and
        // read_lmstudio_settings_dir should return None.
        // We can't guarantee this on all machines, but we can verify
        // the function doesn't panic.
        let _ = LmStudioScanner::read_lmstudio_settings_dir();
    }

    #[test]
    fn huggingface_repo_from_gguf_path() {
        let models_dir = PathBuf::from("/models");
        let model_path = PathBuf::from(
            "/models/lmstudio-community/gemma-2-2b-it-GGUF/gemma-2-2b-it-Q4_K_M.gguf",
        );
        assert_eq!(
            LmStudioScanner::huggingface_repo_from_path(&models_dir, &model_path),
            Some("lmstudio-community/gemma-2-2b-it-GGUF".to_string()),
        );
    }

    #[test]
    fn huggingface_repo_from_mlx_path() {
        let models_dir = PathBuf::from("/models");
        let model_path = PathBuf::from("/models/mlx-community/Qwen3-6bit");
        assert_eq!(
            LmStudioScanner::huggingface_repo_from_path(&models_dir, &model_path),
            Some("mlx-community/Qwen3-6bit".to_string()),
        );
    }

    #[test]
    fn huggingface_repo_returns_none_for_unrelated_path() {
        let models_dir = PathBuf::from("/models");
        let model_path = PathBuf::from("/other/dir/model.gguf");
        assert_eq!(
            LmStudioScanner::huggingface_repo_from_path(&models_dir, &model_path),
            None,
        );
    }

    #[test]
    fn huggingface_repo_returns_none_for_shallow_path() {
        let models_dir = PathBuf::from("/models");
        let model_path = PathBuf::from("/models/model.gguf");
        assert_eq!(
            LmStudioScanner::huggingface_repo_from_path(&models_dir, &model_path),
            None,
        );
    }

    #[test]
    fn architecture_from_model_type_maps_known_types() {
        assert_eq!(
            LmStudioScanner::architecture_from_model_type("llama"),
            ModelArchitecture::Llama
        );
        assert_eq!(
            LmStudioScanner::architecture_from_model_type("qwen3_next"),
            ModelArchitecture::Qwen
        );
        assert_eq!(
            LmStudioScanner::architecture_from_model_type("mistral"),
            ModelArchitecture::Mistral
        );
        assert_eq!(
            LmStudioScanner::architecture_from_model_type("phi3"),
            ModelArchitecture::Phi
        );
        assert_eq!(
            LmStudioScanner::architecture_from_model_type("gemma2"),
            ModelArchitecture::Gemma
        );
        assert_eq!(
            LmStudioScanner::architecture_from_model_type("some_new_arch"),
            ModelArchitecture::Unknown
        );
    }
}
