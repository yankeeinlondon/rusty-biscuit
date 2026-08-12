//! OpenAPI import functionality.
//!
//! This module provides the core import pipeline that transforms `openapiv3::OpenAPI`
//! documents into Schematic `RestApi` and `ModelCatalog` definitions.
//!
//! ## Overview
//!
//! The import process converts OpenAPI 3.x specifications into Schematic's
//! internal representation for code generation. It handles:
//!
//! - Security schemes → `AuthStrategy`
//! - Paths and operations → `Endpoint`
//! - Component schemas → `ModelCatalog`
//! - Parameters → `EndpointParams`
//!
//! ## Examples
//!
//! ```text
//! use schematic_define::openapi::{OpenApiImport, OpenApiSource};
//!
//! let source = OpenApiSource::yaml(r#"
//! openapi: "3.0.0"
//! info:
//!   title: Pet Store
//!   version: "1.0.0"
//! paths:
//!   /pets:
//!     get:
//!       operationId: listPets
//!       responses:
//!         '200':
//!           description: A list of pets
//! "#);
//!
//! let result = OpenApiImport::new(source)
//!     .api_name("PetStore")
//!     .build()?;
//!
//! assert_eq!(result.api.name, "PetStore");
//! ```

mod auth;
mod builder;
mod diagnostics;
mod mappings;
pub mod naming;
mod normalize;
mod resolver;

pub use builder::{
    BaseUrlPolicy, ContentPreference, MAX_COMPONENT_COUNT, MAX_NESTING_DEPTH, OpenApiImport,
    OpenApiImportOptions, OpenApiImportResult,
};
pub use diagnostics::{DiagnosticSeverity, OpenApiDiagnostic};
pub use normalize::clamp_numeric_bounds;
pub use resolver::RefResolver;
