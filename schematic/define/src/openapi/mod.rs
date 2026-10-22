//! OpenAPI specification parsing, conversion, and export.
//!
//! This module provides types and utilities for working with OpenAPI 3.x specifications.
//! It supports both importing OpenAPI documents and exporting schematic's REST API
//! definitions to OpenAPI format with vendor extensions for round-trip fidelity.
//!
//! ## Core Types
//!
//! ### Import
//!
//! - [`OpenApiSource`] - Input source for OpenAPI documents (file, string, bytes, or parsed)
//! - [`OpenApiError`] - Errors that can occur during OpenAPI processing
//! - [`classify_responses`] - One operation's responses as response model v2
//!   success shapes and error entries
//!
//! ### Export
//!
//! - [`export`] - Export a `RestApi` to an OpenAPI 3.0.3 document
//! - [`ExportOptions`] - Configuration options for export
//! - [`ExportFormat`] - Output format (JSON or YAML)
//! - [`serialize`] - Serialize an OpenAPI document to string
//!
//! ### Vendor Extensions
//!
//! - [`SchematicDocExtension`] - Document-level `x-schematic` extension
//! - [`SchematicOpExtension`] - Operation-level `x-schematic` extension
//! - [`SchematicSchemaExtension`] - Schema-level `x-schematic` extension
//!
//! ## Examples
//!
//! ### Import from file
//!
//! ```
//! use schematic_define::openapi::OpenApiSource;
//!
//! let source = OpenApiSource::path("/path/to/openapi.yaml");
//! ```
//!
//! ### Import from YAML string
//!
//! ```
//! use schematic_define::openapi::OpenApiSource;
//!
//! let yaml = r#"
//! openapi: "3.0.0"
//! info:
//!   title: My API
//!   version: "1.0.0"
//! paths: {}
//! "#;
//!
//! let source = OpenApiSource::yaml(yaml);
//! ```
//!
//! ### Export to OpenAPI
//!
//! ```text
//! use schematic_define::openapi::{export, ExportOptions, ExportFormat, serialize};
//! use schematic_definitions::openai::define_openai_api;
//!
//! // Component schemas come from the API's own `schemas`.
//! let api = define_openai_api();
//! let options = ExportOptions::new()
//!     .with_version("1.0.0")
//!     .with_format(ExportFormat::Yaml);
//!
//! let doc = export(&api, &options)?;
//! let yaml_str = serialize(&doc, ExportFormat::Yaml)?;
//! ```
//!
//! ## Feature Flag
//!
//! This module is only available when the `openapi` feature is enabled:
//!
//! ```toml
//! [dependencies]
//! schematic-define = { version = "0.1", features = ["openapi"] }
//! ```

mod error;
mod export;
mod extensions;
pub mod import;
mod options;
mod source;

pub use error::OpenApiError;
pub use export::{export, extract_path_params, map_security};
pub use extensions::{
    AuthExtension, SchematicDocExtension, SchematicOpExtension, SchematicSchemaExtension,
};
pub use import::{
    BaseUrlPolicy, ContentPreference, DiagnosticSeverity, OpenApiDiagnostic, OpenApiImport,
    OpenApiImportOptions, OpenApiImportResult, RefResolver, ResponseClassification,
    classify_responses,
};
pub use options::{ExportFormat, ExportOptions, serialize};
pub use source::OpenApiSource;
