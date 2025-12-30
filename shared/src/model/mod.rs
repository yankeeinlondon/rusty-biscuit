//! Model selection and provider management
//!
//! This module provides centralized model selection with fallback stacking
//! and multi-provider support.
//!
//! # Examples
//!
//! ```rust
//! use shared::model::{get_model, ModelKind, ModelQuality};
//!
//! // Select a fast model for scraping
//! let client = get_model(
//!     ModelKind::Quality(ModelQuality::Fast),
//!     Some("scrape web content")
//! )?;
//! ```

pub mod selection;
pub mod types;

pub use selection::{get_model, ModelError};
pub use types::{LlmClient, ModelKind, ModelProvider, ModelQuality, ModelStack, TaskKind};
