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
//! ```ignore
//! use schematic_define::openapi::{export, ExportOptions, ExportFormat, serialize};
//! use schematic_definitions::openai::{define_openai_api, openapi_registry};
//!
//! let api = define_openai_api();
//! let registry = openapi_registry();
//! let options = ExportOptions::new()
//!     .with_version("1.0.0")
//!     .with_format(ExportFormat::Yaml);
//!
//! let doc = export(&api, &registry, &options)?;
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
pub use export::{export, extract_path_params, map_security, SchemaRegistryLike};
pub use extensions::{
    AuthExtension, SchematicDocExtension, SchematicOpExtension, SchematicSchemaExtension,
};
pub use import::{
    BaseUrlPolicy, ContentPreference, DiagnosticSeverity, OpenApiDiagnostic, OpenApiImport,
    OpenApiImportOptions, OpenApiImportResult, RefResolver,
};
pub use options::{serialize, ExportFormat, ExportOptions};
pub use source::OpenApiSource;
