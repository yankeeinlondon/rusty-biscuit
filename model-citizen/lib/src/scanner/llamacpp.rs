//! Llama.cpp model scanner.
//!
//! Scans for GGUF models in user-configured directories. Unlike Ollama and
//! LM Studio which have fixed locations, Llama.cpp users typically have
//! models in custom directories.

use crate::gguf::{detect_quantization, extract_metadata, model_name_from_filename};
use crate::scanner::ModelScanner;
use crate::{Config, ModelArchitecture, ModelCitizenError, ModelFormat, ModelSource, UnifiedModel};
use async_trait::async_trait;
use std::path::PathBuf;

/// Scanner for Llama.cpp models.
///
/// Llama.cpp doesn't have a fixed models directory - users store GGUF files
/// wherever they prefer. This scanner uses directories from:
/// 1. `LLAMA_CPP_MODELS` environment variable (comma-separated)
/// 2. Configuration file `llamacpp.models_dirs` array
pub struct LlamaCppScanner {
    /// Directories to scan for models.
    models_dirs: Vec<PathBuf>,
}

impl LlamaCppScanner {
    /// Creates a new Llama.cpp scanner with the given configuration.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        Self {
            models_dirs: config.llamacpp_models_dirs.clone(),
        }
    }

    /// Creates a scanner with custom directories (for testing).
    #[must_use]
    pub fn with_dirs(dirs: Vec<PathBuf>) -> Self {
        Self { models_dirs: dirs }
    }

    /// Scans all configured directories for GGUF files.
    fn scan_all_directories(&self) -> Vec<UnifiedModel> {
        let mut models = Vec::new();

        for dir in &self.models_dirs {
            if dir.exists() && dir.is_dir() {
                Self::walk_directory(dir, &mut models);
            }
        }

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

        let mut model = UnifiedModel::new(
            name,
            size_bytes,
            quantization,
            architecture,
            ModelSource::LlamaCpp,
            ModelFormat::Gguf,
            path,
        );

        // Extract rich metadata from GGUF file headers
        if let Some(meta) = extract_metadata(path) {
            model.metadata = meta;
        }

        Some(model)
    }
}

#[async_trait]
impl ModelScanner for LlamaCppScanner {
    fn name(&self) -> &'static str {
        "llamacpp"
    }

    async fn is_available(&self) -> bool {
        // Available if at least one configured directory exists
        self.models_dirs.iter().any(|d| d.exists())
    }

    async fn scan(&self) -> Result<Vec<UnifiedModel>, ModelCitizenError> {
        if self.models_dirs.is_empty() {
            return Ok(Vec::new());
        }

        Ok(self.scan_all_directories())
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
    fn scanner_name_is_llamacpp() {
        let scanner = LlamaCppScanner::with_dirs(vec![]);
        assert_eq!(scanner.name(), "llamacpp");
    }

    #[test]
    fn scan_all_directories_returns_empty_for_no_dirs() {
        let scanner = LlamaCppScanner::with_dirs(vec![]);
        let models = scanner.scan_all_directories();
        assert!(models.is_empty());
    }

    #[test]
    fn scan_all_directories_finds_gguf_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_gguf(temp_dir.path(), "llama3-Q4_K_M.gguf", 4_000_000_000);
        create_test_gguf(temp_dir.path(), "mistral-7b-Q5_K_S.gguf", 7_000_000_000);

        let scanner = LlamaCppScanner::with_dirs(vec![temp_dir.path().to_path_buf()]);

        let models = scanner.scan_all_directories();
        assert_eq!(models.len(), 2);

        for model in &models {
            assert_eq!(model.source, ModelSource::LlamaCpp);
        }
    }

    #[test]
    fn scan_all_directories_scans_multiple_dirs() {
        let temp_dir1 = tempfile::tempdir().unwrap();
        let temp_dir2 = tempfile::tempdir().unwrap();

        create_test_gguf(temp_dir1.path(), "model1-Q4_K_M.gguf", 1_000_000);
        create_test_gguf(temp_dir2.path(), "model2-Q5_K_S.gguf", 2_000_000);

        let scanner = LlamaCppScanner::with_dirs(vec![
            temp_dir1.path().to_path_buf(),
            temp_dir2.path().to_path_buf(),
        ]);

        let models = scanner.scan_all_directories();
        assert_eq!(models.len(), 2);
    }

    #[test]
    fn scan_all_directories_ignores_missing_dirs() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_gguf(temp_dir.path(), "model-Q4_K_M.gguf", 1_000_000);

        let scanner = LlamaCppScanner::with_dirs(vec![
            temp_dir.path().to_path_buf(),
            PathBuf::from("/nonexistent/path"),
        ]);

        let models = scanner.scan_all_directories();
        assert_eq!(models.len(), 1);
    }

    #[test]
    fn scan_all_directories_finds_nested_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_gguf(temp_dir.path(), "subdir/model-Q4_K_M.gguf", 1_000_000);

        let scanner = LlamaCppScanner::with_dirs(vec![temp_dir.path().to_path_buf()]);

        let models = scanner.scan_all_directories();
        assert_eq!(models.len(), 1);
    }

    #[test]
    fn parse_gguf_file_extracts_metadata() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_gguf(temp_dir.path(), "llama3-8b-Q4_K_M.gguf", 1_000_000);

        let path = temp_dir.path().join("llama3-8b-Q4_K_M.gguf");
        let model = LlamaCppScanner::parse_gguf_file(&path).unwrap();

        assert_eq!(model.name, "llama3-8b");
        assert_eq!(model.quantization, crate::QuantizationType::Q4Km);
        assert_eq!(model.architecture, crate::ModelArchitecture::Llama);
        assert_eq!(model.source, ModelSource::LlamaCpp);
    }

    #[tokio::test]
    async fn is_available_returns_true_for_existing_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let scanner = LlamaCppScanner::with_dirs(vec![temp_dir.path().to_path_buf()]);

        assert!(scanner.is_available().await);
    }

    #[tokio::test]
    async fn is_available_returns_false_for_no_dirs() {
        let scanner = LlamaCppScanner::with_dirs(vec![]);
        assert!(!scanner.is_available().await);
    }

    #[tokio::test]
    async fn is_available_returns_false_for_missing_dirs() {
        let scanner = LlamaCppScanner::with_dirs(vec![PathBuf::from("/nonexistent/path")]);
        assert!(!scanner.is_available().await);
    }

    #[tokio::test]
    async fn is_available_returns_true_if_any_dir_exists() {
        let temp_dir = tempfile::tempdir().unwrap();
        let scanner = LlamaCppScanner::with_dirs(vec![
            PathBuf::from("/nonexistent/path"),
            temp_dir.path().to_path_buf(),
        ]);

        assert!(scanner.is_available().await);
    }

    #[tokio::test]
    async fn scan_returns_models() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_gguf(temp_dir.path(), "model-Q4_K_M.gguf", 1_000_000);

        let scanner = LlamaCppScanner::with_dirs(vec![temp_dir.path().to_path_buf()]);

        let models = scanner.scan().await.unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].source, ModelSource::LlamaCpp);
    }

    #[tokio::test]
    async fn scan_returns_empty_for_no_dirs() {
        let scanner = LlamaCppScanner::with_dirs(vec![]);
        let models = scanner.scan().await.unwrap();
        assert!(models.is_empty());
    }

    #[test]
    fn new_from_config_uses_config_dirs() {
        let config = Config {
            llamacpp_models_dirs: vec![PathBuf::from("/test/path")],
            ..Default::default()
        };

        let scanner = LlamaCppScanner::new(&config);
        assert_eq!(scanner.models_dirs.len(), 1);
        assert_eq!(scanner.models_dirs[0], PathBuf::from("/test/path"));
    }
}
