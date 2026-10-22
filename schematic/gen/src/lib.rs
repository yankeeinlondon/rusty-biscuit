//! Schematic code generator library.
//!
//! This crate generates strongly-typed Rust client code from REST API
//! definitions created with `schematic-define`. Generated clients are thin:
//! the hand-written `schematic-runtime` crate sends their requests. Per API
//! the generated code includes:
//!
//! - a client struct named after the API, the only name a caller writes,
//!   wrapping `schematic_runtime::Client<{Name}Api>`; the
//!   `schematic_runtime::api_client!` macro gives it every client-wide
//!   setting and send, and generation adds `BASE_URL`, `DOCS_URL`, `definition()`, an
//!   infallible `new()`, and `basic_auth()` when the API accepts HTTP Basic
//! - a `{Name}Api` tag type that the API's request types name, so the client
//!   sends only that API's requests
//! - one request struct per endpoint, implementing
//!   `schematic_runtime::EndpointSpec` (its API, success type, codec, and
//!   endpoint id)
//!
//! ## Modules
//!
//! - [`codegen`] - Code generation for individual components (structs, enums, etc.)
//! - [`output`] - Final assembly, validation, and file writing
//! - [`cargo_gen`] - Cargo.toml generation for the output package
//! - [`parser`] - Path parameter extraction utilities
//! - [`errors`] - Error types for the generator
//! - [`validation`] / [`diagnostics`] - Per-output validation and its diagnostics
//!
//! ## Example Usage
//!
//! ```no_run
//! use std::path::Path;
//! use schematic_definitions::openai::define_openai_api;
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
//! pub struct OpenAI(schematic_runtime::Client<OpenAIApi>);
//! pub enum OpenAIApi {}
//! schematic_runtime::api_client!(OpenAI, OpenAIApi);
//!
//! pub struct ListModelsRequest { ... }
//! pub struct RetrieveModelRequest { pub model: String, ... }
//!
//! impl schematic_runtime::EndpointSpec for RetrieveModelRequest {
//!     type Api = OpenAIApi;
//!     type Success = Model;
//!     type Decoder = schematic_runtime::Json<Model>;
//!     ...
//! }
//!
//! pub struct RetrieveModelCall<'a>(schematic_runtime::Call<'a, OpenAIApi, RetrieveModelRequest>);
//!
//! impl OpenAI {
//!     pub fn retrieve_model(&self, model: impl Into<String>) -> RetrieveModelCall<'_> { ... }
//! }
//!
//! // A caller calls the endpoint method; awaiting its call sends it, and the
//! // request type fixes the response type.
//! let client = OpenAI::new();
//! let model = client.retrieve_model("gpt-4").await?;
//! ```

pub mod asyncapi_import;
pub mod cargo_gen;
pub mod codegen;
pub mod commands;
pub mod definition_gen;
pub mod diagnostics;
pub mod errors;
pub mod export;
pub mod import_pipeline;
pub mod inference;
pub mod model_gen;
pub mod openapi_output;
pub mod output;
pub mod parser;
pub mod pipeline;
pub mod postman_output;
pub mod validation;
pub mod ws_codegen;

pub use inference::infer_module_path;

/// The `schematic_runtime::ABI` this generator emits code for.
///
/// Generated `lib.rs` declares it as `SCHEMATIC_RUNTIME_ABI` and asserts at
/// compile time that the runtime it builds against has the same value;
/// partial generation refuses to keep modules generated for another value.
/// It changes together with the runtime's constant, which a test checks.
pub const RUNTIME_ABI: u32 = 5;


pub use export::openapi::{
    build_openapi_grouped, operation_id_diagnostics, write_openapi, write_openapi_grouped,
};
pub use export::postman::{write_postman, write_postman_grouped};

pub use validation::{
    validate_openapi, validate_postman, validate_rust_client,
};

#[cfg(test)]
pub mod test_utils;

#[cfg(test)]
mod runtime_abi_tests {
    /// The generator is built in the same workspace as the runtime it emits
    /// code for, so their ABI values must agree; generated code built
    /// against a disagreeing runtime fails its compile-time assertion.
    #[test]
    fn the_generator_emits_the_abi_of_the_workspace_runtime() {
        let source = include_str!("../../runtime/src/lib.rs");
        let file = syn::parse_file(source).expect("the runtime's lib.rs parses");
        let abi = file
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Const(constant) if constant.ident == "ABI" => match &*constant.expr {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Int(value),
                        ..
                    }) => Some(value.base10_parse::<u32>().expect("a u32 literal")),
                    _ => None,
                },
                _ => None,
            })
            .expect("the runtime declares `pub const ABI: u32 = <literal>;`");
        assert_eq!(super::RUNTIME_ABI, abi);
    }
}
