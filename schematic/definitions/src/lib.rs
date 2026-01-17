//! Schematic API Definitions
//!
//! This crate contains actual REST API definitions that use the primitives
//! from `schematic-define`. Each API is organized in its own module.
//!
//! ## Available APIs
//!
//! - [`openai`] - OpenAI Models API definition
//! - [`elevenlabs`] - ElevenLabs TTS and voice management API definition
//!
//! ## Examples
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

pub mod elevenlabs;
pub mod openai;
pub mod prelude;

// Re-export API definition functions for convenience
pub use elevenlabs::{define_elevenlabs_rest_api, define_elevenlabs_websocket_api};
pub use openai::define_openai_api;
