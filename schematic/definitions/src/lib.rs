//! Schematic API Definitions
//!
//! This crate contains actual REST API definitions that use the primitives
//! from `schematic-define`. Each API is organized in its own module.
//!
//! ## Available APIs
//!
//! - [`openai`] - OpenAI Models API definition
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

pub mod openai;
pub mod prelude;

// Re-export API definition functions for convenience
pub use openai::define_openai_api;
