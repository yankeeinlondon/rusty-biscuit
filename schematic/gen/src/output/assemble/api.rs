//! API module assembly for generated code modules.
//!
//! An API module holds the API's client struct and tag type, one request
//! struct per endpoint with its `schematic_runtime::EndpointSpec`
//! implementation, one call type per endpoint with the endpoint method that
//! returns it, and the re-exported definition types. Its sends are the
//! runtime client's, forwarded by `schematic_runtime::api_client!` and by
//! the call types.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{Endpoint, RestApi};

use crate::codegen::api_struct::api_tag_name;
use crate::codegen::pagination::{api_pager_view, ordinary_send_defaults};
use crate::codegen::responses::api_error_body_impls;
use crate::codegen::{
    ModuleDocBuilder, generate_api_struct, generate_endpoint_methods, generate_pagination_impls,
    generate_request_struct_with_rules,
};
use crate::codegen::value_rules::ApiValueRules;

use super::helpers::{build_definitions_import, get_request_suffix};
use schematic_definitions::naming::resolve_module_name;
use crate::output::options::OutputOptions;

/// Threshold (in lines of formatted code) above which an API module is split
/// into a directory with submodules instead of a single file.
pub const SPLIT_THRESHOLD_LINES: usize = 2000;

/// Parts produced when splitting a large API module into submodules.
pub struct SplitApiParts {
    /// Module docs, submodule declarations, re-exports, and each API's
    /// client struct and tag type.
    pub mod_rs: TokenStream,
    /// Request structs, their `EndpointSpec` and pager impls, form bodies,
    /// call types, and the endpoint methods.
    pub requests_rs: TokenStream,
    /// The re-exported definition types.
    pub responses_rs: TokenStream,
}

/// The request structs, the `ErrorBody` impls of the API's registered JSON
/// failure types (one per type, however many endpoints share it), pager
/// impls, call types, and endpoint methods of one API.
///
/// They are generated from the API's pager view, so a parameter a
/// pagination strategy adopts is in none of them. The value-rule plans of
/// every request share one plans module.
///
/// An API-level `Cookie` header cannot be a client default header: the
/// runtime refuses a default header that can carry a credential. So the API's
/// `Cookie` headers are folded into every endpoint's static headers here,
/// ahead of the endpoint's own, where they merge with its cookies and cookie
/// parameters into one `Cookie` header. `generate_api_struct` leaves them out
/// of the default headers.
fn api_requests(api: &RestApi, module_path: &str) -> TokenStream {
    // The view keeps the API's endpoints in order, so the original endpoint
    // at the same index still declares what the view removed.
    let declared = api;
    let api = api_pager_view(api);
    let api = api.as_ref();
    let suffix = get_request_suffix(api);
    let api_cookies: Vec<(String, String)> = api
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("cookie"))
        .cloned()
        .collect();

    let mut tokens = TokenStream::new();
    let mut rules = ApiValueRules::new(api);
    for (endpoint, original) in api.endpoints.iter().zip(&declared.endpoints) {
        let endpoint = with_api_cookies(endpoint, &api_cookies);
        tokens.extend(generate_request_struct_with_rules(
            api,
            &endpoint,
            &suffix,
            Some(module_path),
            &api_tag_name(api),
            &ordinary_send_defaults(declared, original),
            &mut rules,
        ));
    }
    tokens.extend(rules.into_tokens());
    for operation in api.graphql.iter().flat_map(|surface| &surface.operations) {
        tokens.extend(crate::codegen::graphql::generate_operation_request(api, operation, &suffix, &api_tag_name(api)));
    }
    tokens.extend(api_error_body_impls(api, &api_tag_name(api)));
    tokens.extend(generate_pagination_impls(api, &suffix));
    tokens.extend(generate_endpoint_methods(api));
    tokens
}

fn with_api_cookies(endpoint: &Endpoint, api_cookies: &[(String, String)]) -> Endpoint {
    let mut endpoint = endpoint.clone();
    if !api_cookies.is_empty() {
        let mut headers = api_cookies.to_vec();
        headers.append(&mut endpoint.headers);
        endpoint.headers = headers;
    }
    endpoint
}

/// The pieces one module group's generated module is built from, in either
/// layout: [`Self::single_file`] or [`Self::split`].
///
/// Both layouts order the same pieces, so a caller can generate (and parse)
/// each piece once and build whichever layout the module's size selects;
/// `assemble_modules` needs the single-file form to measure a module before
/// it knows whether to split it.
pub(crate) struct ModulePieces<T> {
    /// Module documentation: inner doc attributes only.
    pub docs: T,
    pub serde_import: T,
    /// The re-exported definition types; the whole of `responses.rs`.
    pub definitions_import: T,
    /// Each API's client struct and tag type, in group order.
    pub api_structs: Vec<T>,
    /// Each API's request items ([`api_requests`]), in group order.
    pub requests: Vec<T>,
    /// The submodule declarations and re-exports of a split `mod.rs`.
    pub submodules: T,
    /// The imports a split `requests.rs` adds after the serde import.
    pub split_imports: T,
}

impl<T> ModulePieces<T> {
    /// Applies `convert` to every piece, failing on the first error.
    pub(crate) fn try_map<U, E>(&self, mut convert: impl FnMut(&T) -> Result<U, E>) -> Result<ModulePieces<U>, E> {
        Ok(ModulePieces {
            docs: convert(&self.docs)?,
            serde_import: convert(&self.serde_import)?,
            definitions_import: convert(&self.definitions_import)?,
            api_structs: self.api_structs.iter().map(&mut convert).collect::<Result<_, _>>()?,
            requests: self.requests.iter().map(&mut convert).collect::<Result<_, _>>()?,
            submodules: convert(&self.submodules)?,
            split_imports: convert(&self.split_imports)?,
        })
    }

    /// The pieces of the single-file module, in file order: each API's
    /// struct is followed by its requests.
    pub(crate) fn single_file(&self) -> Vec<&T> {
        let mut pieces = vec![&self.docs, &self.serde_import, &self.definitions_import];
        for (api_struct, requests) in self.api_structs.iter().zip(&self.requests) {
            pieces.extend([api_struct, requests]);
        }
        pieces
    }

    /// The pieces of the split module's `mod.rs`, `requests.rs`, and
    /// `responses.rs`, in file order.
    pub(crate) fn split(&self) -> [Vec<&T>; 3] {
        let mod_rs = [&self.docs, &self.submodules].into_iter().chain(&self.api_structs).collect();
        let requests_rs = [&self.serde_import, &self.split_imports].into_iter().chain(&self.requests).collect();
        [mod_rs, requests_rs, vec![&self.definitions_import]]
    }
}

/// The pieces of the module shared by `apis`, documented by `docs`, with
/// `definitions_import` re-exporting its definition types.
fn module_pieces(apis: &[&RestApi], docs: TokenStream, definitions_import: TokenStream) -> ModulePieces<TokenStream> {
    let module_path = resolve_module_name(apis[0]);
    // The endpoint methods in `requests.rs` extend the API structs.
    let api_items: Vec<_> = apis
        .iter()
        .flat_map(|api| [format_ident!("{}", api.name), format_ident!("{}", api_tag_name(api))])
        .collect();
    // JSON bodies are spelled by their components' qualified Rust paths, so
    // only an unqualified pager item type names something in `responses`.
    let needs_response_import = apis.iter().any(|api| api.endpoints.iter().any(|endpoint| {
        let resolution = schematic_define::pagination::resolve(api, endpoint);
        resolution.strategy().is_some()
            && resolution.list().and_then(|list| list.item_type.as_deref()).is_some_and(|name| !name.contains("::"))
    }));
    let response_import = needs_response_import.then(|| quote! { use super::responses::*; });

    ModulePieces {
        docs,
        serde_import: quote! { use serde::{Deserialize, Serialize}; },
        definitions_import,
        api_structs: apis.iter().map(|api| generate_api_struct(api)).collect(),
        requests: apis.iter().map(|api| api_requests(api, &module_path)).collect(),
        submodules: quote! {
            pub mod requests;
            pub mod responses;

            pub use requests::*;
            pub use responses::*;
        },
        split_imports: quote! {
            #response_import
            use super::{#(#api_items),*};
        },
    }
}

/// The pieces of one module group's generated module: a single API's own
/// module documentation, or, for several, [`build_combined_module_docs`].
///
/// ## Panics
///
/// Panics if `apis` is empty.
pub(crate) fn module_group_pieces(apis: &[&RestApi]) -> ModulePieces<TokenStream> {
    assert!(!apis.is_empty(), "a module group has at least one API");
    let docs = match apis {
        [api] => ModuleDocBuilder::new(api).build(),
        _ => build_combined_module_docs(apis),
    };
    module_pieces(apis, docs, build_definitions_import(&resolve_module_name(apis[0])))
}

fn concat(pieces: Vec<&TokenStream>) -> TokenStream {
    pieces.into_iter().cloned().collect()
}

fn split_parts(pieces: &ModulePieces<TokenStream>) -> SplitApiParts {
    let [mod_rs, requests_rs, responses_rs] = pieces.split().map(concat);
    SplitApiParts { mod_rs, requests_rs, responses_rs }
}

/// The definitions import of a single API's module: the shipped definitions
/// module, or the crate's own `types` module when standalone.
fn single_api_definitions_import(api: &RestApi, options: &OutputOptions) -> TokenStream {
    if options.standalone {
        quote! { pub use crate::types::*; }
    } else {
        build_definitions_import(&resolve_module_name(api))
    }
}

/// Assembles an API module with configurable options.
pub fn assemble_api_module_with_options(api: &RestApi, options: &OutputOptions) -> TokenStream {
    let docs = ModuleDocBuilder::new(api).build();
    concat(module_pieces(&[api], docs, single_api_definitions_import(api, options)).single_file())
}

/// Assembles the API module code (e.g., anthropic.rs): module documentation,
/// the re-exported definition types, the API's client struct and tag type,
/// and its request structs.
pub fn assemble_api_module(api: &RestApi) -> TokenStream {
    assemble_api_module_with_options(api, &OutputOptions::default())
}

/// Assembles a combined API module for multiple APIs sharing the same module path.
///
/// When two or more APIs share a `module_path` (e.g., `OllamaNative` and `OllamaOpenAI`
/// both use `"ollama"`), this function generates a single `.rs` file containing all
/// of their types.
///
/// ## Panics
///
/// Panics if `apis` is empty.
pub fn assemble_combined_api_module(apis: &[&RestApi]) -> TokenStream {
    assert!(
        !apis.is_empty(),
        "assemble_combined_api_module requires at least one API"
    );
    let docs = build_combined_module_docs(apis);
    concat(module_pieces(apis, docs, build_definitions_import(&resolve_module_name(apis[0]))).single_file())
}

/// Builds combined module documentation for multiple APIs sharing a module:
/// a summary naming every API, then one `##` section per API holding the
/// documentation its own module would have, each heading one level deeper.
fn build_combined_module_docs(apis: &[&RestApi]) -> TokenStream {
    let mut names: Vec<String> = apis.iter().map(|api| format!("[`{}`]", api.name)).collect();
    let last = names.pop().unwrap_or_default();
    let names = match names.len() {
        0 => last,
        1 => format!("{} and {last}", names[0]),
        _ => format!("{}, and {last}", names.join(", ")),
    };
    let mut doc = format!("Generated API clients for {names}, which share this module.");
    for api in apis {
        doc.push_str(&format!("\n\n## {}\n\n", api.name));
        doc.push_str(&demote_headings(&ModuleDocBuilder::new(api).markdown()));
    }
    crate::codegen::lines_to_doc_comments(&doc)
}

/// `markdown` with every heading outside a code fence one level deeper.
fn demote_headings(markdown: &str) -> String {
    let mut in_fence = false;
    markdown
        .lines()
        .map(|line| {
            if line.starts_with("```") {
                in_fence = !in_fence;
            }
            if !in_fence && line.starts_with('#') { format!("#{line}") } else { line.to_string() }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Assembles a single API module split into submodules for large APIs.
///
/// Produces three TokenStreams intended for:
/// - `mod.rs` - module docs, submodule declarations, re-exports, and the
///   API's client struct and tag type
/// - `requests.rs` - all request structs with their `EndpointSpec` and
///   pager impls, the call types, and the endpoint methods
/// - `responses.rs` - definitions re-exports
pub fn assemble_split_api_module(api: &RestApi, options: &OutputOptions) -> SplitApiParts {
    let docs = ModuleDocBuilder::new(api).build();
    split_parts(&module_pieces(&[api], docs, single_api_definitions_import(api, options)))
}

/// Assembles a combined API module split into submodules for large APIs.
///
/// This is the split version of [`assemble_combined_api_module`] for when
/// multiple APIs share the same module path and the combined output exceeds
/// the threshold.
///
/// ## Panics
///
/// Panics if `apis` is empty.
pub fn assemble_split_combined_api_module(apis: &[&RestApi]) -> SplitApiParts {
    assert!(
        !apis.is_empty(),
        "assemble_split_combined_api_module requires at least one API"
    );
    let docs = build_combined_module_docs(apis);
    split_parts(&module_pieces(apis, docs, build_definitions_import(&resolve_module_name(apis[0]))))
}

#[doc(hidden)]
pub fn assemble_api_code(api: &RestApi) -> TokenStream {
    assemble_api_module(api)
}

#[cfg(test)]
mod tests {
    use schematic_define::{AuthStrategy, Backoff, RateLimit, ResetUnit, RetryGuidance, StatusSelector};

    use super::build_combined_module_docs;
    use crate::test_utils::make_api;

    #[test]
    fn grouped_module_docs_keep_each_apis_rate_limit_and_retry_guidance() {
        let mut first = make_api("First", "https://first.test", AuthStrategy::None, vec![]);
        first.rate_limit = Some(RateLimit::new("x-limit", "x-remaining", "x-reset", ResetUnit::Seconds).unwrap());
        let mut second = make_api("Second", "https://second.test", AuthStrategy::None, vec![]);
        second.retry_guidance = Some(
            RetryGuidance::new([StatusSelector::Code(429)], true, 3, Backoff::Fixed { delay_ms: 500 }).unwrap(),
        );
        let plain = make_api("Plain", "https://plain.test", AuthStrategy::None, vec![]);

        let docs = build_combined_module_docs(&[&first, &second, &plain]).to_string();
        assert!(docs.contains("x-limit") && docs.contains("x-reset"), "{docs}");
        assert!(docs.contains("Retry guidance") && docs.contains("429"), "{docs}");
        assert_eq!(docs.matches("Rate limit").count(), 1, "{docs}");
        assert_eq!(docs.matches("Retry guidance").count(), 1, "{docs}");
    }

    #[test]
    fn demoted_headings_leave_hidden_doctest_lines_alone() {
        let markdown = "Intro\n\n## Examples\n\n```no_run\n# async fn example() {\n# }\n```\n\n### Override Base URL";

        assert_eq!(
            super::demote_headings(markdown),
            "Intro\n\n### Examples\n\n```no_run\n# async fn example() {\n# }\n```\n\n#### Override Base URL"
        );
    }

    #[test]
    fn a_json_failure_type_shared_by_endpoints_gets_one_error_body_impl_per_api() {
        use schematic_define::response::ResponseShape;
        use schematic_define::{SuccessShape, ErrorEntry, RestMethod, SchemaComponent, StatusMatch, TypeRef};

        let problem = || ResponseShape::Json(TypeRef::Named("Problem".into()));
        let endpoint = |id: &str, path: &str| {
            let mut endpoint = crate::test_utils::make_endpoint(id, RestMethod::Get, path, None);
            endpoint.response = vec![SuccessShape::empty()];
            endpoint
        };
        let mut overlaid = endpoint("Delete", "/items/delete");
        overlaid.errors = vec![ErrorEntry::new(StatusMatch::class(5), problem())];
        let mut several = endpoint("Update", "/items/update");
        several.errors = vec![ErrorEntry::new(StatusMatch::class(5), ResponseShape::Text)];
        let mut api = make_api("Fixture", "https://fixture.test", AuthStrategy::None, vec![]);
        api.endpoints = vec![endpoint("Get", "/items"), overlaid, several];
        api.errors = vec![ErrorEntry::new(StatusMatch::exact(404), problem())];
        api.schemas = vec![SchemaComponent { name: "Problem".into(), rust_path: Some("::defs::Problem".into()), schema: serde_json::json!({"type": "object"}) }];

        let tokens = super::assemble_api_module(&api);
        crate::test_utils::validate_generated_code(&tokens).expect("module parses");
        let code: String = tokens.to_string().split_whitespace().collect();
        assert_eq!(code.matches("typeFailure=::defs::Problem;").count(), 2, "{code}");
        assert_eq!(code.matches("implschematic_runtime::ErrorBody<FixtureApi>for::defs::Problem{").count(), 1, "{code}");
        assert_eq!(code.matches("implschematic_runtime::ErrorBody<FixtureApi>forUpdateError{").count(), 1, "{code}");
    }
}
