//! Code generation modules for schematic.
//!
//! This module contains generators that produce Rust source code from
//! REST API definitions. Each submodule handles a specific component
//! of the generated API client.
//!
//! ## Submodules
//!
//! - [`api_struct`] - Generates the main API client struct with constructors
//! - [`client`] - Generates the async `request()` method with auth handling
//! - [`error`] - Generates the `SchematicError` enum for runtime errors
//! - [`request_enum`] - Generates the unified request enum for all endpoints
//! - [`request_structs`] - Generates per-endpoint request structs
//!
//! ## Code Generation Flow
//!
//! 1. Each endpoint gets a request struct via [`generate_request_struct`]
//! 2. All request structs are unified in an enum via [`generate_request_enum`]
//! 3. The API struct is created via [`generate_api_struct`]
//! 4. The request method is generated via [`generate_request_method`]
//! 5. The error type is generated via [`generate_error_type`]
//!
//! ## Output Format
//!
//! All generators return `proc_macro2::TokenStream`, which is then:
//! - Validated with `syn::parse2` to ensure correctness
//! - Formatted with `prettyplease` for consistent style
//!
//! See [`crate::output`] for the assembly and file writing logic.

pub mod api_struct;
pub mod client;
pub mod error;
pub mod module_docs;
pub mod request_enum;
pub mod request_structs;

pub use api_struct::generate_api_struct;
pub use client::{generate_request_method, generate_request_method_with_suffix};
pub use error::{generate_error_type, generate_request_parts_type};
pub use module_docs::ModuleDocBuilder;
pub use request_enum::{generate_request_enum, generate_request_enum_with_suffix};
pub use request_structs::{
    generate_request_struct, generate_request_struct_with_options,
    generate_request_struct_with_suffix,
};
