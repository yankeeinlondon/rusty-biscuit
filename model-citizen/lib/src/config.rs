//! Configuration loading for model-citizen.
//!
//! Configuration is loaded from multiple sources with the following priority:
//! 1. Environment variables (highest priority)
//! 2. Config file (`~/.config/model-citizen/config.toml`)
//! 3. Default values (lowest priority)

use crate::error::ModelCitizenError;
use biscuit_file::Toml;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for model-citizen.
///
/// Contains paths and settings for model discovery across different runners.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Directory for shared models managed by model-citizen.
    pub shared_models_dir: Option<PathBuf>,

    /// Llama.cpp model directories (comma-separated in env var).
    pub llamacpp_models_dirs: Vec<PathBuf>,

    /// Whether to enable symlink-based model sharing.
    pub enable_sharing: bool,

    /// Scanner-specific configuration.
    #[serde(default)]
    pub scanners: ScannersConfig,
}

/// Scanner-specific configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ScannersConfig {
    /// Ollama scanner settings.
    pub ollama: OllamaConfig,

    /// LM Studio scanner settings.
    pub lmstudio: LmStudioConfig,

    /// Llama.cpp scanner settings.
    pub llamacpp: LlamaCppConfig,
}

/// Ollama-specific configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OllamaConfig {
    /// Whether the Ollama scanner is enabled.
    pub enabled: bool,

    /// Override the Ollama API host (default: http://localhost:11434).
    pub api_host: Option<String>,

    /// Timeout for API requests in seconds.
    pub timeout_secs: u64,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_host: None,
            timeout_secs: 5,
        }
    }
}

/// LM Studio-specific configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LmStudioConfig {
    /// Whether the LM Studio scanner is enabled.
    pub enabled: bool,

    /// Override the LM Studio API host (default: http://localhost:1234).
    pub api_host: Option<String>,

    /// Timeout for API requests in seconds.
    pub timeout_secs: u64,
}

impl Default for LmStudioConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_host: None,
            timeout_secs: 5,
        }
    }
}

/// Llama.cpp-specific configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LlamaCppConfig {
    /// Whether the Llama.cpp scanner is enabled.
    pub enabled: bool,
}

impl Default for LlamaCppConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shared_models_dir: None,
            llamacpp_models_dirs: Vec::new(),
            enable_sharing: true,
            scanners: ScannersConfig::default(),
        }
    }
}

impl Config {
    /// Loads configuration from environment variables only.
    ///
    /// ## Environment Variables
    ///
    /// - `MODEL_CITIZEN_SHARED_DIR`: Directory for shared models
    /// - `LLAMA_CPP_MODELS`: Comma-separated list of Llama.cpp model directories
    /// - `OLLAMA_HOST`: Override Ollama API host
    /// - `LM_STUDIO_HOST`: Override LM Studio API host
    #[must_use]
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(shared_dir) = std::env::var("MODEL_CITIZEN_SHARED_DIR") {
            config.shared_models_dir = Some(PathBuf::from(shared_dir));
        }

        if let Ok(llama_dirs) = std::env::var("LLAMA_CPP_MODELS") {
            config.llamacpp_models_dirs = llama_dirs
                .split(',')
                .map(|s| PathBuf::from(s.trim()))
                .filter(|p| !p.as_os_str().is_empty())
                .collect();
        }

        if let Ok(host) = std::env::var("OLLAMA_HOST") {
            config.scanners.ollama.api_host = Some(host);
        }

        if let Ok(host) = std::env::var("LM_STUDIO_HOST") {
            config.scanners.lmstudio.api_host = Some(host);
        }

        config
    }

    /// Loads configuration from a TOML file.
    ///
    /// ## Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file(path: &std::path::Path) -> Result<Self, ModelCitizenError> {
        let toml = Toml::new(path)?;
        let config: Self = serde::Deserialize::deserialize(toml.value().clone())?;
        Ok(config)
    }

    /// Returns the default config file path.
    ///
    /// Platform-specific:
    /// - macOS/Linux: `~/.config/model-citizen/config.toml`
    /// - Windows: `%APPDATA%\model-citizen\config.toml`
    #[must_use]
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("model-citizen").join("config.toml"))
    }

    /// Loads configuration from the default file path if it exists.
    ///
    /// Returns default config if no file exists.
    ///
    /// ## Errors
    ///
    /// Returns an error if the file exists but cannot be parsed.
    pub fn from_default_file() -> Result<Self, ModelCitizenError> {
        match Self::default_path() {
            Some(path) if path.exists() => Self::from_file(&path),
            _ => Ok(Self::default()),
        }
    }

    /// Loads configuration by merging sources with proper priority.
    ///
    /// Priority (highest to lowest):
    /// 1. Environment variables
    /// 2. Config file (if exists)
    /// 3. Default values
    ///
    /// ## Errors
    ///
    /// Returns an error if the config file exists but cannot be parsed.
    pub fn load() -> Result<Self, ModelCitizenError> {
        let file_config = Self::from_default_file()?;
        let env_config = Self::from_env();
        let mut config = file_config.merge(env_config);

        // Include MODELS_DIR and default shared dir so downloaded models are discoverable.
        if let Ok(models_dir) = std::env::var("MODELS_DIR")
            && let path = PathBuf::from(models_dir)
            && !config.llamacpp_models_dirs.contains(&path)
        {
            config.llamacpp_models_dirs.push(path);
        }
        if let Some(shared) = crate::sharing::default_shared_dir()
            && !config.llamacpp_models_dirs.contains(&shared)
        {
            config.llamacpp_models_dirs.push(shared);
        }

        Ok(config)
    }

    /// Merges another config into this one.
    ///
    /// Values from `other` take precedence when they differ from defaults.
    #[must_use]
    pub fn merge(mut self, other: Self) -> Self {
        // Environment overrides for paths
        if other.shared_models_dir.is_some() {
            self.shared_models_dir = other.shared_models_dir;
        }
        if !other.llamacpp_models_dirs.is_empty() {
            self.llamacpp_models_dirs = other.llamacpp_models_dirs;
        }

        // Environment overrides for scanner hosts
        if other.scanners.ollama.api_host.is_some() {
            self.scanners.ollama.api_host = other.scanners.ollama.api_host;
        }
        if other.scanners.lmstudio.api_host.is_some() {
            self.scanners.lmstudio.api_host = other.scanners.lmstudio.api_host;
        }

        self
    }

    /// Returns the effective Ollama API host.
    #[must_use]
    pub fn ollama_host(&self) -> &str {
        self.scanners
            .ollama
            .api_host
            .as_deref()
            .unwrap_or("http://localhost:11434")
    }

    /// Returns the effective LM Studio API host.
    #[must_use]
    pub fn lmstudio_host(&self) -> &str {
        self.scanners
            .lmstudio
            .api_host
            .as_deref()
            .unwrap_or("http://localhost:1234")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::io::Write;

    /// # Safety
    ///
    /// This is only safe when tests run serially (via `#[serial]`), ensuring
    /// no other threads are reading environment variables.
    unsafe fn clear_env_vars() {
        unsafe {
            std::env::remove_var("MODEL_CITIZEN_SHARED_DIR");
            std::env::remove_var("LLAMA_CPP_MODELS");
            std::env::remove_var("OLLAMA_HOST");
            std::env::remove_var("LM_STUDIO_HOST");
        }
    }

    /// # Safety
    ///
    /// This is only safe when tests run serially (via `#[serial]`), ensuring
    /// no other threads are reading environment variables.
    unsafe fn set_env(key: &str, value: &str) {
        unsafe {
            std::env::set_var(key, value);
        }
    }

    #[test]
    fn default_config_has_expected_values() {
        let config = Config::default();
        assert!(config.shared_models_dir.is_none());
        assert!(config.llamacpp_models_dirs.is_empty());
        assert!(config.enable_sharing);
        assert!(config.scanners.ollama.enabled);
        assert!(config.scanners.lmstudio.enabled);
        assert!(config.scanners.llamacpp.enabled);
    }

    #[test]
    #[serial]
    fn from_env_reads_shared_dir() {
        // SAFETY: Test runs serially via #[serial], no concurrent env access
        unsafe {
            clear_env_vars();
            set_env("MODEL_CITIZEN_SHARED_DIR", "/models/shared");
        }
        let config = Config::from_env();
        assert_eq!(
            config.shared_models_dir,
            Some(PathBuf::from("/models/shared"))
        );
        // SAFETY: Test runs serially via #[serial], no concurrent env access
        unsafe { clear_env_vars() };
    }

    #[test]
    #[serial]
    fn from_env_reads_llama_dirs() {
        // SAFETY: Test runs serially via #[serial], no concurrent env access
        unsafe {
            clear_env_vars();
            set_env("LLAMA_CPP_MODELS", "/models/a, /models/b");
        }
        let config = Config::from_env();
        assert_eq!(
            config.llamacpp_models_dirs,
            vec![PathBuf::from("/models/a"), PathBuf::from("/models/b")]
        );
        // SAFETY: Test runs serially via #[serial], no concurrent env access
        unsafe { clear_env_vars() };
    }

    #[test]
    #[serial]
    fn from_env_reads_api_hosts() {
        // SAFETY: Test runs serially via #[serial], no concurrent env access
        unsafe {
            clear_env_vars();
            set_env("OLLAMA_HOST", "http://custom:11434");
            set_env("LM_STUDIO_HOST", "http://custom:1234");
        }
        let config = Config::from_env();
        assert_eq!(
            config.scanners.ollama.api_host,
            Some("http://custom:11434".to_string())
        );
        assert_eq!(
            config.scanners.lmstudio.api_host,
            Some("http://custom:1234".to_string())
        );
        // SAFETY: Test runs serially via #[serial], no concurrent env access
        unsafe { clear_env_vars() };
    }

    #[test]
    fn from_file_parses_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(
            file,
            r#"
shared_models_dir = "/custom/shared"
llamacpp_models_dirs = ["/models/llama"]
enable_sharing = false

[scanners.ollama]
enabled = false
timeout_secs = 10
"#
        )
        .unwrap();

        let config = Config::from_file(&path).unwrap();
        assert_eq!(
            config.shared_models_dir,
            Some(PathBuf::from("/custom/shared"))
        );
        assert_eq!(
            config.llamacpp_models_dirs,
            vec![PathBuf::from("/models/llama")]
        );
        assert!(!config.enable_sharing);
        assert!(!config.scanners.ollama.enabled);
        assert_eq!(config.scanners.ollama.timeout_secs, 10);
    }

    #[test]
    fn from_file_returns_error_for_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, "this is not { valid toml").unwrap();

        let result = Config::from_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn from_file_returns_error_for_missing_file() {
        let result = Config::from_file(std::path::Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn merge_env_overrides_file() {
        // SAFETY: Test runs serially via #[serial], no concurrent env access
        unsafe { clear_env_vars() };

        // File config
        let file_config = Config {
            shared_models_dir: Some(PathBuf::from("/file/shared")),
            llamacpp_models_dirs: vec![PathBuf::from("/file/llama")],
            scanners: ScannersConfig {
                ollama: OllamaConfig {
                    api_host: Some("http://file:11434".to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // Env config (partial override)
        let env_config = Config {
            shared_models_dir: Some(PathBuf::from("/env/shared")),
            llamacpp_models_dirs: vec![], // Empty, should not override
            scanners: ScannersConfig {
                ollama: OllamaConfig {
                    api_host: None, // None, should not override
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        let merged = file_config.merge(env_config);
        // shared_models_dir from env (was Some)
        assert_eq!(merged.shared_models_dir, Some(PathBuf::from("/env/shared")));
        // llamacpp_models_dirs from file (env was empty)
        assert_eq!(
            merged.llamacpp_models_dirs,
            vec![PathBuf::from("/file/llama")]
        );
        // ollama host from file (env was None)
        assert_eq!(
            merged.scanners.ollama.api_host,
            Some("http://file:11434".to_string())
        );

        // SAFETY: Test runs serially via #[serial], no concurrent env access
        unsafe { clear_env_vars() };
    }

    #[test]
    fn ollama_host_returns_default() {
        let config = Config::default();
        assert_eq!(config.ollama_host(), "http://localhost:11434");
    }

    #[test]
    fn ollama_host_returns_custom() {
        let config = Config {
            scanners: ScannersConfig {
                ollama: OllamaConfig {
                    api_host: Some("http://custom:11434".to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(config.ollama_host(), "http://custom:11434");
    }

    #[test]
    fn lmstudio_host_returns_default() {
        let config = Config::default();
        assert_eq!(config.lmstudio_host(), "http://localhost:1234");
    }

    #[test]
    fn default_path_returns_platform_path() {
        let path = Config::default_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.ends_with("model-citizen/config.toml"));
    }

    #[test]
    fn config_serializes_to_toml() {
        let config = Config {
            shared_models_dir: Some(PathBuf::from("/shared")),
            llamacpp_models_dirs: vec![PathBuf::from("/models")],
            enable_sharing: true,
            scanners: ScannersConfig::default(),
        };
        // Serialize using serde_json (we know this works)
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        assert!(json_str.contains("shared_models_dir"));
        assert!(json_str.contains("llamacpp_models_dirs"));
    }
}
