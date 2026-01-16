//! Convenient re-exports for working with API definitions.
//!
//! This prelude provides all API definition functions and their associated types.
//!
//! ## Examples
//!
//! ```
//! use schematic_definitions::prelude::*;
//!
//! let api = define_openai_api();
//! assert_eq!(api.name, "OpenAI");
//! ```

// API definition functions
pub use crate::openai::define_openai_api;

// Response types for each API
pub use crate::openai::{DeleteModelResponse, ListModelsResponse, Model};
