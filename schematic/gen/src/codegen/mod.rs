//! Code generation modules for schematic.
//!
//! This module contains generators that produce Rust source code from
//! REST API definitions. Generated clients are thin: request execution lives
//! in the hand-written `schematic-runtime` crate, and generated code declares
//! only what is specific to each API.
//!
//! ## Submodules
//!
//! - [`api_struct`] - Generates each API's client struct, tag type, and
//!   API-specific items
//! - [`endpoint_methods`] - Generates each endpoint's method on its API
//!   struct and the call type it returns
//! - [`request_structs`] - Generates per-endpoint request structs and their
//!   `schematic_runtime::EndpointSpec` implementations
//! - `responses` - Generates each endpoint's success and failure types: the
//!   multi-shape and failure enums, stream end markers, and the error table
//! - `graphql` - Generates a request struct per GraphQL operation
//! - [`pagination`] - Generates the `schematic_runtime::pager::Paginated`
//!   and capability marker impls of paginated list endpoints
//! - [`shared`] - Generates the request-side helpers of `shared.rs`
//! - `value_rules` - Compiles each request's value rules into the
//!   `schematic_runtime::validate` plans its `validate()` and `into_parts`
//!   check
//!
//! ## Code Generation Flow
//!
//! 1. Each endpoint gets a request struct via [`generate_request_struct`]
//!    (and each GraphQL operation one via `graphql`)
//! 2. Each paginated endpoint gets its pager impls via
//!    [`generate_pagination_impls`]
//! 3. Each API gets its client struct and tag via [`generate_api_struct`]
//! 4. Each API gets its endpoint methods and call types via
//!    [`generate_endpoint_methods`]
//! 5. `shared.rs` gets the request helpers via [`generate_request_helpers`]
//!
//! ## Output Format
//!
//! All generators return `proc_macro2::TokenStream`, which is then:
//! - Validated with `syn::parse2` to ensure correctness
//! - Formatted with `prettyplease` for consistent style
//!
//! See [`crate::output`] for the assembly and file writing logic.

pub mod api_struct;
pub mod endpoint_methods;
pub(crate) mod graphql;
pub mod module_docs;
pub mod pagination;
pub mod request_structs;
pub(crate) mod responses;
pub mod shared;
pub(crate) mod value_rules;

pub use api_struct::generate_api_struct;
pub use endpoint_methods::generate_endpoint_methods;
pub use module_docs::ModuleDocBuilder;
pub(crate) use module_docs::lines_to_doc_comments;
pub use request_structs::{
    PLACEHOLDER_API_MARKER, generate_request_struct, generate_request_struct_with_options,
    generate_request_struct_with_suffix,
};
pub(crate) use request_structs::generate_request_struct_with_rules;
pub use shared::generate_request_helpers;
pub use pagination::generate_pagination_impls;
