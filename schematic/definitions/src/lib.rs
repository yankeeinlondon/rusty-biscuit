//! Schematic API Definitions
//!
//! This crate contains actual REST API definitions that use the primitives
//! from `schematic-define`. Each API is organized in its own module.
//!
//! ## Available APIs
//!
//! - [`anthropic`] - Anthropic Messages API for Claude AI and agent tool use
//! - [`openai`] - OpenAI Models API definition
//! - [`elevenlabs`] - ElevenLabs TTS and voice management API definition
//! - [`huggingface`] - Hugging Face Hub API for model/dataset discovery
//! - [`ollama`] - Ollama local LLM inference (native + OpenAI-compatible APIs)
//! - [`emqx`] - EMQX Broker REST API (Basic Auth + Bearer Token variants)
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

pub mod anthropic;
pub mod elevenlabs;
pub mod emqx;
pub mod huggingface;
pub mod ollama;
pub mod openai;
pub mod prelude;

// Re-export API definition functions for convenience
pub use anthropic::define_anthropic_api;
pub use elevenlabs::{define_elevenlabs_rest_api, define_elevenlabs_websocket_api};
pub use emqx::{define_emqx_basic_api, define_emqx_bearer_api};
pub use huggingface::define_huggingface_hub_api;
pub use ollama::{define_ollama_native_api, define_ollama_openai_api};
pub use openai::define_openai_api;
