//! Shared export helpers for API artifact generation.
//!
//! Provides normalized representations that both OpenAPI and Postman writers consume.

pub mod auth;
pub mod body;
pub mod examples;
pub mod http;
pub mod openapi;
pub mod path_params;
pub mod postman;
pub(crate) mod vocabulary;

pub use auth::ExportAuth;
pub use body::{ExportBody, FormField, FormFieldExportKind};
pub use http::{ExportEndpoint, ExportParam};
pub use path_params::{extract_folder_key, strip_reserved_markers};
