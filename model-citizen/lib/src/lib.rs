//! Model Citizen: Local LLM model management across multiple runners.
//!
//! This library provides unified model discovery and management for:
//! - **Ollama**: API-driven model management
//! - **LM Studio**: Application-based model discovery
//! - **Llama.cpp**: Directory-based model scanning
//!
//! ## Quick Start
//!
//! ```no_run
//! use model_citizen::{Config, UnifiedModel, ModelSource};
//!
//! // Load configuration from env vars and config file
//! let config = Config::load()?;
//!
//! // Access configuration
//! println!("Ollama host: {}", config.ollama_host());
//! println!("LM Studio host: {}", config.lmstudio_host());
//! # Ok::<(), model_citizen::ModelCitizenError>(())
//! ```
//!
//! ## Scanning Models
//!
//! ```ignore
//! use model_citizen::{ModelRegistry, scanner::ModelScanner};
//!
//! let mut registry = ModelRegistry::new();
//! // Add scanners for different runners
//! // registry.add_scanner(OllamaScanner::new(config.clone()));
//! // registry.add_scanner(LmStudioScanner::new(config.clone()));
//!
//! // Scan all runners concurrently
//! let models = registry.scan_all().await;
//! for model in models {
//!     println!("{}: {} ({})", model.name, model.size_display(), model.quantization.as_str());
//! }
//! ```

mod config;
mod error;
pub mod gguf;
pub mod huggingface;
mod model;
pub mod registry;
pub mod scanner;
pub mod sharing;

pub use config::{Config, LlamaCppConfig, LmStudioConfig, OllamaConfig, ScannersConfig};
pub use error::ModelCitizenError;
pub use model::{huggingface_repo_from_url, ModelArchitecture, ModelFormat, ModelMetadata, ModelSource, QuantizationType, UnifiedModel};
pub use huggingface::SortOrder;
pub use registry::{ModelRegistry, ScanResult, DEFAULT_SCANNER_TIMEOUT};

/// Result type alias for model-citizen operations.
pub type Result<T> = std::result::Result<T, ModelCitizenError>;
