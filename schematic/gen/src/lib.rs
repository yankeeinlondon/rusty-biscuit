//! Schematic code generator library.
//!
//! This crate generates strongly-typed Rust client code from REST API
//! definitions created with `schematic-define`. The generated code includes:
//!
//! - A client struct with `new()` and `with_base_url()` constructors
//! - Request structs for each endpoint (with path parameters as fields)
//! - A unified request enum for type-safe request handling
//! - An async `request()` method that handles HTTP communication
//! - A `SchematicError` type for comprehensive error handling
//!
//! ## Modules
//!
//! - [`codegen`] - Code generation for individual components (structs, enums, etc.)
//! - [`output`] - Final assembly, validation, and file writing
//! - [`cargo_gen`] - Cargo.toml generation for the output package
//! - [`parser`] - Path parameter extraction utilities
//! - [`errors`] - Error types for the generator
//!
//! ## Example Usage
//!
//! ```no_run
//! use std::path::Path;
//! use schematic_define::apis::define_openai_api;
//! use schematic_gen::output::generate_and_write;
//!
//! let api = define_openai_api();
//! let output_dir = Path::new("generated/src");
//!
//! // Generate code (dry_run=false writes to disk)
//! let code = generate_and_write(&api, output_dir, true).unwrap();
//! println!("{}", code);
//! ```
//!
//! ## Generated Code Structure
//!
//! For an API named "OpenAI" with endpoints `ListModels` and `RetrieveModel`:
//!
//! ```text
//! // Error type
//! pub enum SchematicError { ... }
//!
//! // Per-endpoint request structs
//! pub struct ListModelsRequest { ... }
//! pub struct RetrieveModelRequest { pub model: String, ... }
//!
//! // Unified request enum
//! pub enum OpenAIRequest {
//!     ListModels(ListModelsRequest),
//!     RetrieveModel(RetrieveModelRequest),
//! }
//!
//! // Client struct
//! pub struct OpenAI {
//!     client: reqwest::Client,
//!     base_url: String,
//! }
//!
//! impl OpenAI {
//!     pub async fn request<T>(&self, req: impl Into<OpenAIRequest>) -> Result<T, SchematicError>;
//! }
//! ```

pub mod cargo_gen;
pub mod codegen;
pub mod errors;
pub mod output;
pub mod parser;
