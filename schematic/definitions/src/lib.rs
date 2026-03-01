//! Schematic API Definitions
//!
//! This crate contains actual REST API definitions that use the primitives
//! from `schematic-define`. Each API is organized in its own module.
//!
//! ## Available APIs
//!
//! - [`anthropic`] - Anthropic Messages API for Claude AI and agent tool use
//! - [`bitbucket`] - Bitbucket Cloud REST API for repositories, PRs, issues, and tags
//! - [`elevenlabs`] - ElevenLabs TTS and voice management API definition
//! - [`emqx`] - EMQX Broker REST API (Basic Auth + Bearer Token variants)
//! - [`eversolo`] - Eversolo DMP-A8 local HTTP control API (device, playback, I/O, display)
//! - [`gitea`] - Gitea REST API for self-hosted Git forge instances
//! - [`github`] - GitHub REST API for repositories, PRs, issues, and releases
//! - [`gitlab`] - GitLab REST API for repositories, MRs, issues, and releases
//! - [`huggingface`] - Hugging Face Hub API for model/dataset discovery
//! - [`lmstudio`] - LM Studio local LLM inference (v1 native API)
//! - [`ollama`] - Ollama local LLM inference (native + OpenAI-compatible APIs)
//! - [`openai`] - OpenAI Models API definition
//! - [`unfolded_circle`] - Unfolded Circle Core REST + WebSocket API definitions
//!
//! ## Examples
//!
//! ```
//! use schematic_definitions::anthropic::define_anthropic_api;
//!
//! let api = define_anthropic_api();
//! assert_eq!(api.name, "Anthropic");
//! assert_eq!(api.endpoints.len(), 4);
//! ```
//!
//! ```
//! use schematic_definitions::openai::define_openai_api;
//!
//! let api = define_openai_api();
//! assert_eq!(api.name, "OpenAI");
//! assert_eq!(api.endpoints.len(), 3);
//! ```
//!
//! ```
//! use schematic_definitions::elevenlabs::{define_elevenlabs_rest_api, define_elevenlabs_websocket_api};
//!
//! let rest_api = define_elevenlabs_rest_api();
//! assert_eq!(rest_api.name, "ElevenLabs");
//! assert!(rest_api.endpoints.len() >= 35);
//!
//! let ws_api = define_elevenlabs_websocket_api();
//! assert_eq!(ws_api.name, "ElevenLabsTTS");
//! ```
//!
//! ```
//! use schematic_definitions::huggingface::define_huggingface_hub_api;
//!
//! let api = define_huggingface_hub_api();
//! assert_eq!(api.name, "HuggingFaceHub");
//! assert!(api.endpoints.len() >= 26);
//! ```
//!
//! ```
//! use schematic_definitions::ollama::{define_ollama_native_api, define_ollama_openai_api};
//!
//! let native_api = define_ollama_native_api();
//! assert_eq!(native_api.name, "OllamaNative");
//! assert_eq!(native_api.endpoints.len(), 11);
//!
//! let openai_api = define_ollama_openai_api();
//! assert_eq!(openai_api.name, "OllamaOpenAI");
//! assert_eq!(openai_api.endpoints.len(), 4);
//! ```
//!
//! ```
//! use schematic_definitions::lmstudio::define_lmstudio_api;
//!
//! let api = define_lmstudio_api();
//! assert_eq!(api.name, "LmStudio");
//! assert_eq!(api.endpoints.len(), 6);
//! ```
//!
//! ```
//! use schematic_definitions::emqx::{define_emqx_basic_api, define_emqx_bearer_api};
//!
//! let basic_api = define_emqx_basic_api();
//! assert_eq!(basic_api.name, "EmqxBasic");
//! assert!(basic_api.endpoints.len() >= 30);
//!
//! let bearer_api = define_emqx_bearer_api();
//! assert_eq!(bearer_api.name, "EmqxBearer");
//! // Bearer API has login/logout plus all common endpoints
//! assert!(bearer_api.endpoints.len() > basic_api.endpoints.len());
//! ```
//!
//! ```
//! use schematic_definitions::eversolo::define_eversolo_api;
//!
//! let api = define_eversolo_api();
//! assert_eq!(api.name, "Eversolo");
//! assert_eq!(api.endpoints.len(), 24);
//! ```
//!
//! ```
//! use schematic_definitions::github::define_github_api;
//!
//! let api = define_github_api();
//! assert_eq!(api.name, "GitHub");
//! assert_eq!(api.endpoints.len(), 16);
//! ```
//!
//! ```
//! use schematic_definitions::gitea::define_gitea_api;
//!
//! let api = define_gitea_api();
//! assert_eq!(api.name, "Gitea");
//! assert_eq!(api.endpoints.len(), 15);
//! ```
//!
//! ```
//! use schematic_definitions::gitlab::define_gitlab_api;
//!
//! let api = define_gitlab_api();
//! assert_eq!(api.name, "GitLab");
//! assert_eq!(api.endpoints.len(), 18);
//! ```
//!
//! ```
//! use schematic_definitions::bitbucket::define_bitbucket_api;
//!
//! let api = define_bitbucket_api();
//! assert_eq!(api.name, "Bitbucket");
//! assert_eq!(api.endpoints.len(), 15);
//! ```

pub mod anthropic;
pub mod bitbucket;
pub mod elevenlabs;
pub mod emqx;
pub mod eversolo;
pub mod gitea;
pub mod github;
pub mod gitlab;
pub mod huggingface;
pub mod lmstudio;
pub mod ollama;
pub mod openai;
pub mod prelude;
pub mod registry;
pub mod unfolded_circle;

// Re-export API definition functions for convenience
pub use anthropic::define_anthropic_api;
pub use bitbucket::define_bitbucket_api;
pub use elevenlabs::{define_elevenlabs_rest_api, define_elevenlabs_websocket_api};
pub use emqx::{define_emqx_basic_api, define_emqx_bearer_api};
pub use eversolo::define_eversolo_api;
pub use gitea::define_gitea_api;
pub use github::define_github_api;
pub use gitlab::define_gitlab_api;
pub use huggingface::define_huggingface_hub_api;
pub use lmstudio::define_lmstudio_api;
pub use ollama::{define_ollama_native_api, define_ollama_openai_api};
pub use openai::define_openai_api;
pub use unfolded_circle::{
    define_unfolded_circle_core_rest_api, define_unfolded_circle_core_ws_api,
    define_unfolded_circle_dock_ws_api, define_unfolded_circle_integration_ws_api,
};
