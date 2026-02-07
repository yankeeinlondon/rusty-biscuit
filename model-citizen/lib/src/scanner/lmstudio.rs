//! LM Studio model scanner.
//!
//! Scans for models in the LM Studio models directory by recursively
//! searching for `.gguf` files.

use crate::gguf::{detect_quantization, model_name_from_filename};
use crate::scanner::ModelScanner;
use crate::{Config, ModelArchitecture, ModelCitizenError, ModelSource, UnifiedModel};
use async_trait::async_trait;
use std::path::PathBuf;

/// Scanner for LM Studio models.
///
/// LM Studio stores models as GGUF files in a user-specific directory.
/// The scanner recursively searches for `.gguf` files and extracts
/// metadata from filenames and file headers.
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
    #[must_use]
    pub fn new(config: &Config) -> Self {
        Self {
            models_dir: Self::default_models_dir(),
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

    /// Recursively scans for GGUF files.
    fn scan_directory(&self, dir: &std::path::Path) -> Vec<UnifiedModel> {
        let mut models = Vec::new();
        Self::walk_directory(dir, &mut models);
        models
    }

    /// Recursively walks a directory tree looking for GGUF files.
    fn walk_directory(dir: &std::path::Path, models: &mut Vec<UnifiedModel>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                Self::walk_directory(&path, models);
            } else if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.eq_ignore_ascii_case("gguf") {
                        if let Some(model) = Self::parse_gguf_file(&path) {
                            models.push(model);
                        }
                    }
                }
            }
        }
    }

    /// Parses a GGUF file to create a UnifiedModel.
    fn parse_gguf_file(path: &std::path::Path) -> Option<UnifiedModel> {
        // Get file size
        let metadata = std::fs::metadata(path).ok()?;
        let size_bytes = metadata.len();

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

        Some(UnifiedModel::new(
            name,
            size_bytes,
            quantization,
            architecture,
            ModelSource::LmStudio,
            path,
        ))
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

        Ok(self.scan_directory(&self.models_dir))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
