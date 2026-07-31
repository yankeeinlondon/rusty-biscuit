//! Shared module assembly (shared.rs).

use proc_macro2::TokenStream;
use quote::quote;

use crate::codegen::{
    generate_error_type, generate_paginated_trait, generate_request_parts_type,
    generate_response_kind_type, generate_variant_types,
};

/// Assembles the shared module code (shared.rs).
///
/// This function generates code for the shared module, containing:
/// - Module documentation
/// - Common error type used by all API clients
/// - Common type aliases (e.g., `RequestParts`)
/// - Re-export of reqwest for downstream crates
/// - `Paginated` marker trait for paginated request types
///
/// ## Returns
///
/// A TokenStream containing the shared module code.
pub fn assemble_shared_module() -> TokenStream {
    let request_parts_type = generate_request_parts_type();
    let response_kind_type = generate_response_kind_type();
    let error_type = generate_error_type();
    let variant_types = generate_variant_types();
    let paginated_trait = generate_paginated_trait();

    quote! {
        //! Shared types and utilities for generated API clients.

        // `FormFile` is a field type on generated form-body structs, which derive
        // both traits, so it must derive them too.
        use serde::{Deserialize, Serialize};

        /// Re-export reqwest for downstream crates that need to make custom requests.
        ///
        /// This allows consumers to use the same HTTP client types without adding
        /// reqwest as a direct dependency.
        pub use reqwest;

        /// Re-export `schematic-define` for downstream crates.
        ///
        /// Generated client builder methods expose `schematic-define` types in
        /// their public signatures (e.g. [`auth_update`](AuthStrategy)). Re-exporting
        /// the crate lets consumers name those types without adding
        /// `schematic-define` as a direct dependency.
        pub use schematic_define;

        /// Auth-strategy types re-exported flat from `schematic-define` for
        /// ergonomic use in generated client builder calls.
        pub use schematic_define::{AuthPolicy, AuthStrategy, EnvAuthStrategy, Headers, OAuth2Config, UpdateStrategy};

        #request_parts_type

        #response_kind_type

        #error_type

        #variant_types

        #paginated_trait
    }
}
