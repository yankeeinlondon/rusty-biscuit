//! Client adaptors for various AI providers using rig-core.
//!
//! This module provides thin wrappers around OpenAI-compatible providers,
//! allowing unified access through the rig-core framework.
//!
//! ## Available Adaptors
//!
//! - [`zai`] - Z.ai API (GLM models)
//! - [`zenmux`] - ZenMux AI gateway
//!
//! ## Example
//!
//! ```rust,ignore
//! use ai_pipeline::rigging::providers::client_adaptors::zai;
//!
//! let client = zai::Client::from_env()?;
//! let model = client.completion_model(zai::GLM_4_7);
//! ```

pub mod zai;
pub mod zenmux;

pub use zai::{Client as ZaiClient, ClientBuilder as ZaiClientBuilder};
pub use zenmux::{Client as ZenmuxClient, ClientBuilder as ZenmuxClientBuilder};
