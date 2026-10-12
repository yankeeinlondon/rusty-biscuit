//! Per-output validation of a batch of API definitions.
//!
//! Each generated output has its own validator, and each validator takes the
//! whole selected batch: [`validate_rust_client`], [`validate_openapi`] (plus
//! [`validate_openapi_registry`] for schema registries), and
//! [`validate_postman`]. An output is written only when its validator reports
//! no [`Severity::Error`]; one output's findings never block another output.
//! Rust-only rules (identifiers, type syntax, generated-name collisions) are
//! not applied to the exports, which carry those names as data.
//!
//! The behavior is described for users in
//! `schematic/docs/generation/validation.md`.
//!
//! ## Examples
//!
//! ```
//! use schematic_define::{ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod};
//! use schematic_gen::diagnostics::{Feature, Severity};
//! use schematic_gen::validation::{validate_postman, validate_rust_client};
//!
//! let api = RestApi {
//!     name: "TestApi".to_string(),
//!     description: "Test API".to_string(),
//!     base_url: "https://api.test.com".to_string(),
//!     docs_url: None,
//!     auth: AuthStrategy::None,
//!     auth_policy: None,
//!     env_auth: vec![],
//!     env_username: None,
//!     env_mapping: None,
//!     headers: vec![],
//!     endpoints: vec![Endpoint {
//!         id: "ListItems".to_string(),
//!         method: RestMethod::Get,
//!         path: "/items".to_string(),
//!         description: "List items".to_string(),
//!         request: None,
//!         // Not a Rust type: blocks the Rust client, not the exports.
//!         response: ApiResponse::json_type("Vec<Item"),
//!         headers: vec![],
//!         params: None,
//!         oauth_scopes: None,
//!     }],
//!     module_path: None,
//!     request_suffix: None,
//!     version: None,
//! };
//!
//! let rust = validate_rust_client(&[&api]);
//! assert_eq!(rust.len(), 1);
//! assert_eq!(rust[0].severity, Severity::Error);
//! assert_eq!(rust[0].feature, Feature::InvalidTypeSyntax);
//!
//! assert!(validate_postman(&[&api]).is_empty());
//! ```

use std::collections::{BTreeMap, BTreeSet, HashMap};

use schematic_define::openapi::import::naming::sanitize_rust_field_ident;
use schematic_define::{
    ApiRequest, ApiResponse, AuthMethod, Endpoint, EnvAuthStrategy, FormFieldKind, ParamDef,
    RestApi,
};
use schematic_definitions::registry::SchemaRegistry;

use crate::codegen::client::to_snake_case;
use crate::diagnostics::{Feature, Output, Severity, ValidationDiagnostic};
use crate::errors::GeneratorError;
use crate::export::resolve_module_name;
use crate::output::assemble::definitions_module_path;
use crate::output::{get_module_path, get_request_suffix};
use crate::parser::extract_path_params;

/// Severity of the `query-in-path` check.
///
/// A path template that embeds a query string (`/tree?recursive=true`) is
/// reported but does not block any output while the shipped definitions still
/// contain such templates. Raising this to [`Severity::Error`] is the single
/// change needed once they are migrated to declared query parameters.
pub const QUERY_IN_PATH_SEVERITY: Severity = Severity::Warning;

/// Severity of the `duplicate-method-path` check within one API.
///
/// Two endpoints of one API on the same method and normalized path are told
/// apart only by their response or query (GitHub `GetGitTree` and
/// `GetGitTreeRecursive`, Bitbucket `ListDirectoryContents` and
/// `GetFileContentRaw`). The Rust client keeps both, but an OpenAPI path item
/// holds one operation per method, so the document keeps only the later one.
/// The finding is a warning until endpoints can be distinguished on the same
/// route; raising this constant is the single change that makes it blocking.
pub const DUPLICATE_METHOD_PATH_SEVERITY: Severity = Severity::Warning;

/// Names every generated module brings into scope with `use`.
///
/// A generated item with one of these names would clash with the import.
const IMPORTED_TYPE_NAMES: &[&str] = &[
    "Deserialize",
    "Serialize",
    "RequestParts",
    "SchematicError",
    "Paginated",
];

/// Generated crate files a REST module name must not reuse.
const RESERVED_MODULE_NAMES: &[&str] = &["lib", "shared", "prelude", "ws_shared"];

/// The identity two endpoints of one API may not share.
///
/// Built only by [`endpoint_uniqueness_key`], the one place that decides
/// which endpoints count as the same route.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EndpointUniquenessKey {
    /// The HTTP method, upper case (`GET`).
    pub method: String,
    /// The path template with every placeholder written as `{}`.
    pub normalized_path: String,
}

/// Returns the route identity of `endpoint`.
///
/// Path-parameter names do not distinguish routes: `GET /items/{id}` and
/// `GET /items/{name}` share a key, while `GET` and `DELETE` on one path do
/// not.
pub fn endpoint_uniqueness_key(endpoint: &Endpoint) -> EndpointUniquenessKey {
    let mut normalized_path = String::with_capacity(endpoint.path.len());
    let mut in_placeholder = false;
    for c in endpoint.path.chars() {
        match c {
            '{' if !in_placeholder => {
                in_placeholder = true;
                normalized_path.push_str("{}");
            }
            '}' if in_placeholder => in_placeholder = false,
            _ if in_placeholder => {}
            _ => normalized_path.push(c),
        }
    }
    EndpointUniquenessKey {
        method: endpoint.method.to_string(),
        normalized_path,
    }
}

/// Validates a batch of APIs for the generated Rust client.
///
/// Covers everything the OpenAPI and Postman validators check about routes,
/// plus the Rust-only rules: identifiers, request suffixes, module paths and
/// file names, type syntax, generated-name collisions across each module
/// group, and request inputs the generated client cannot send yet.
pub fn validate_rust_client(apis: &[&RestApi]) -> Vec<ValidationDiagnostic> {
    let mut diagnostics = Vec::new();
    for api in apis {
        check_routes(api, Output::Rust, &mut diagnostics);
        check_rust_api(api, &mut diagnostics);
    }

    let mut groups: BTreeMap<String, Vec<&RestApi>> = BTreeMap::new();
    for api in apis {
        groups.entry(get_module_path(api)).or_default().push(api);
    }
    for group in groups.values() {
        check_group_type_names(group, &mut diagnostics);
    }
    diagnostics
}

/// Validates a batch of APIs for OpenAPI export.
///
/// Checks route identity, the `query-in-path` warning, and the document file
/// name. Schema-registry completeness is checked separately by
/// [`validate_openapi_registry`], because the registry is chosen per module.
pub fn validate_openapi(apis: &[&RestApi]) -> Vec<ValidationDiagnostic> {
    let mut diagnostics = Vec::new();
    for api in apis {
        check_routes(api, Output::OpenApi, &mut diagnostics);
        check_export_filename(api, Output::OpenApi, &mut diagnostics);
    }
    diagnostics
}

/// Validates that `registry` can describe every JSON type of a module's APIs.
///
/// `registry` is `None` when the module has no registry at all; that is one
/// error naming every API of the module.
pub fn validate_openapi_registry(
    module_name: &str,
    apis: &[&RestApi],
    registry: Option<&SchemaRegistry>,
) -> Vec<ValidationDiagnostic> {
    let Some(registry) = registry else {
        return vec![ValidationDiagnostic::for_group(
            Severity::Error,
            apis.iter().map(|api| api.name.clone()),
            Feature::IncompleteSchemaRegistry,
            Output::OpenApi,
            format!(
                "no schema registry for module `{module_name}`; add openapi_registry() to \
                 schematic-definitions or skip with --no-openapi"
            ),
        )];
    };

    apis.iter()
        .filter_map(|api| {
            let missing = registry.validate_completeness(api).err()?;
            Some(ValidationDiagnostic::new(
                Severity::Error,
                &api.name,
                None,
                Feature::IncompleteSchemaRegistry,
                Output::OpenApi,
                format!(
                    "module `{module_name}` registry is missing schema(s) {missing:?}; add a \
                     JsonSchema derive and a register::<T>(\"{}\") entry in \
                     schematic-definitions, or skip with --no-openapi",
                    missing.first().map(String::as_str).unwrap_or_default()
                ),
            ))
        })
        .collect()
}

/// Validates a batch of APIs for Postman export.
///
/// Checks route identity, the `query-in-path` warning, the collection file
/// name, and header or cookie parameters, which collections do not carry.
pub fn validate_postman(apis: &[&RestApi]) -> Vec<ValidationDiagnostic> {
    let mut diagnostics = Vec::new();
    for api in apis {
        check_routes(api, Output::Postman, &mut diagnostics);
        check_export_filename(api, Output::Postman, &mut diagnostics);
        for endpoint in &api.endpoints {
            check_unsent_params(api, endpoint, Output::Postman, &mut diagnostics);
        }
    }
    diagnostics
}

/// Turns blocking `diagnostics` into [`GeneratorError::Validation`].
///
/// Used by the public writers so that a caller who skips the pipeline gets
/// the same boundary: nothing of `output` is written while an error stands.
pub(crate) fn ensure_no_errors(
    output: Output,
    diagnostics: Vec<ValidationDiagnostic>,
) -> Result<(), GeneratorError> {
    if diagnostics.iter().any(ValidationDiagnostic::is_error) {
        return Err(GeneratorError::Validation {
            output,
            diagnostics,
        });
    }
    Ok(())
}

/// Checks endpoint ids, route identity, and embedded queries for one output.
fn check_routes(api: &RestApi, output: Output, diagnostics: &mut Vec<ValidationDiagnostic>) {
    let mut ids: HashMap<&str, usize> = HashMap::new();
    let mut routes: HashMap<EndpointUniquenessKey, &str> = HashMap::new();

    for endpoint in &api.endpoints {
        let count = ids.entry(endpoint.id.as_str()).or_default();
        *count += 1;
        if *count == 2 {
            diagnostics.push(ValidationDiagnostic::new(
                Severity::Error,
                &api.name,
                Some(&endpoint.id),
                Feature::DuplicateEndpointId,
                output,
                "endpoint id is declared more than once in this API",
            ));
        }

        let key = endpoint_uniqueness_key(endpoint);
        if let Some(first) = routes.get(&key) {
            let consequence = if output == Output::OpenApi {
                "; the document keeps only one of the two operations"
            } else {
                ""
            };
            diagnostics.push(ValidationDiagnostic::new(
                DUPLICATE_METHOD_PATH_SEVERITY,
                &api.name,
                Some(&endpoint.id),
                Feature::DuplicateMethodPath,
                output,
                format!(
                    "{} {} is already declared by endpoint `{first}`{consequence}",
                    key.method, key.normalized_path
                ),
            ));
        } else {
            routes.insert(key, &endpoint.id);
        }

        if endpoint.path.contains('?') {
            diagnostics.push(ValidationDiagnostic::new(
                QUERY_IN_PATH_SEVERITY,
                &api.name,
                Some(&endpoint.id),
                Feature::QueryInPath,
                output,
                format!(
                    "path template `{}` embeds a query string; declare it as query \
                     parameters so it can be separated from the path",
                    endpoint.path
                ),
            ));
        }
    }
}

/// Checks that an export's file stem is one safe path component.
fn check_export_filename(
    api: &RestApi,
    output: Output,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    let stem = resolve_module_name(api);
    if let Err(reason) = check_safe_file_stem(&stem) {
        diagnostics.push(ValidationDiagnostic::new(
            Severity::Error,
            &api.name,
            None,
            Feature::InvalidOutputFilename,
            output,
            format!("output file name `{stem}` {reason}"),
        ));
    }
}

/// Returns why `stem` cannot name a file directly inside the output root.
fn check_safe_file_stem(stem: &str) -> Result<(), &'static str> {
    if stem.is_empty() {
        return Err("is empty");
    }
    if stem == "." || stem == ".." {
        return Err("would leave the output directory");
    }
    if stem
        .chars()
        .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*'))
    {
        return Err("is not a single file name (it contains a path separator or a character \
                    Windows forbids)");
    }
    Ok(())
}

/// Checks the Rust-only rules for one API and its endpoints.
fn check_rust_api(api: &RestApi, diagnostics: &mut Vec<ValidationDiagnostic>) {
    let error = |endpoint: Option<&str>, feature: Feature, message: String| {
        ValidationDiagnostic::new(
            Severity::Error,
            &api.name,
            endpoint,
            feature,
            Output::Rust,
            message,
        )
    };

    if let Some(suffix) = &api.request_suffix {
        if suffix.is_empty() {
            diagnostics.push(error(
                None,
                Feature::InvalidRequestSuffix,
                "request suffix cannot be empty".to_string(),
            ));
        } else if !suffix.chars().all(char::is_alphanumeric) {
            diagnostics.push(error(
                None,
                Feature::InvalidRequestSuffix,
                format!(
                    "request suffix `{suffix}` must contain only alphanumeric characters \
                     (letters and numbers)"
                ),
            ));
        }
    }

    // The API name becomes the client type and `{name}Request`.
    if let Err(reason) = check_type_ident(&api.name) {
        diagnostics.push(error(
            None,
            Feature::InvalidIdentifier,
            format!("API name `{}` {reason}", api.name),
        ));
    }

    check_module_path(api, diagnostics);
    check_auth_support(api, diagnostics);
    check_convenience_methods(api, diagnostics);

    for endpoint in &api.endpoints {
        let id = Some(endpoint.id.as_str());

        // Endpoint ids become type and variant identifiers.
        if let Err(reason) = check_type_ident(&endpoint.id) {
            diagnostics.push(error(
                id,
                Feature::InvalidIdentifier,
                format!("endpoint id `{}` {reason}", endpoint.id),
            ));
        }

        for (role, type_name) in referenced_type_names(endpoint) {
            if syn::parse_str::<syn::Type>(type_name).is_err() {
                diagnostics.push(error(
                    id,
                    Feature::InvalidTypeSyntax,
                    format!("{role} type `{type_name}` is not valid Rust type syntax"),
                ));
            }
        }

        check_request_struct_fields(api, endpoint, diagnostics);
        check_request_body_support(api, endpoint, diagnostics);
        check_unsent_params(api, endpoint, Output::Rust, diagnostics);
    }
}

/// Returns why `name` cannot be used as a generated type or variant name.
///
/// Emitters pass these names to `format_ident!`, which panics on anything
/// [`syn::Ident`] rejects: an empty string, a leading digit, punctuation, or
/// a keyword.
fn check_type_ident(name: &str) -> Result<(), &'static str> {
    if syn::parse_str::<syn::Ident>(name).is_ok() {
        Ok(())
    } else if name.is_empty() {
        Err("is empty")
    } else if name.starts_with(|c: char| c.is_ascii_digit()) {
        Err("cannot start with a digit in a Rust identifier")
    } else {
        Err("is a Rust keyword or contains characters that are not valid in an identifier")
    }
}

/// Checks the resolved module path the generated crate declares and files.
///
/// The module path becomes `pub mod <path>;`, the file `<path>.rs` or
/// directory `<path>/`, and (through its definitions mapping) a
/// `schematic_definitions::<a>::<b>` import path, so it must be one plain
/// identifier that names no other generated file.
fn check_module_path(api: &RestApi, diagnostics: &mut Vec<ValidationDiagnostic>) {
    let module_path = get_module_path(api);
    let push = |diagnostics: &mut Vec<ValidationDiagnostic>, feature, message: String| {
        diagnostics.push(ValidationDiagnostic::new(
            Severity::Error,
            &api.name,
            None,
            feature,
            Output::Rust,
            message,
        ));
    };

    if module_path.starts_with("r#") || syn::parse_str::<syn::Ident>(&module_path).is_err() {
        push(
            diagnostics,
            Feature::InvalidModulePath,
            format!(
                "module path `{module_path}` must be a single Rust identifier, not a keyword or \
                 a `::`-separated path"
            ),
        );
        return;
    }

    let definitions_path = definitions_module_path(&module_path);
    if definitions_path
        .split("::")
        .any(|segment| syn::parse_str::<syn::Ident>(segment).is_err())
    {
        push(
            diagnostics,
            Feature::InvalidModulePath,
            format!("definitions module path `{definitions_path}` is not a Rust path"),
        );
    }

    let ws_modules = crate::output::ws_modules::ws_definition_modules();
    if RESERVED_MODULE_NAMES.contains(&module_path.as_str())
        || ws_modules.iter().any(|ws| ws.module == module_path)
    {
        push(
            diagnostics,
            Feature::InvalidOutputFilename,
            format!(
                "module path `{module_path}` would overwrite the generated `{module_path}.rs` \
                 shared file"
            ),
        );
    }
}

/// Reports API-key parameter authentication, which the client cannot send yet.
///
/// Reads the effective policy, so explicit methods and environment fallbacks
/// count as well as the legacy `auth` field.
fn check_auth_support(api: &RestApi, diagnostics: &mut Vec<ValidationDiagnostic>) {
    let policy = api.effective_auth_policy();
    let explicit = policy
        .explicit
        .iter()
        .any(|method| matches!(method, AuthMethod::ApiKeyParam { .. }));
    let fallback = matches!(policy.env_fallback, Some(EnvAuthStrategy::ApiKeyParam { .. }));
    if explicit || fallback {
        diagnostics.push(ValidationDiagnostic::new(
            Severity::Error,
            &api.name,
            None,
            Feature::ApiKeyParam,
            Output::Rust,
            "the generated client cannot send an API key as a query parameter or cookie yet",
        ));
    }
}

/// Reports text and binary request bodies, which the client drops.
fn check_request_body_support(
    api: &RestApi,
    endpoint: &Endpoint,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    let feature = match &endpoint.request {
        Some(ApiRequest::Text { .. }) => Feature::TextRequestBody,
        Some(ApiRequest::Binary { .. }) => Feature::BinaryRequestBody,
        _ => return,
    };
    diagnostics.push(ValidationDiagnostic::new(
        Severity::Error,
        &api.name,
        Some(&endpoint.id),
        feature,
        Output::Rust,
        "the generated client cannot send this request body yet",
    ));
}

/// Reports header and cookie parameters `output` does not send.
///
/// A required parameter that cannot be sent is an error; an optional one is a
/// warning, since every request the output can build is still valid.
fn check_unsent_params(
    api: &RestApi,
    endpoint: &Endpoint,
    output: Output,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    let Some(params) = &endpoint.params else {
        return;
    };
    let located = [
        (Feature::HeaderParameter, "header", &params.header),
        (Feature::CookieParameter, "cookie", &params.cookie),
    ];
    for (feature, location, declared) in located {
        for param in declared {
            let severity = if param.required {
                Severity::Error
            } else {
                Severity::Warning
            };
            diagnostics.push(ValidationDiagnostic::new(
                severity,
                &api.name,
                Some(&endpoint.id),
                feature,
                output,
                format!(
                    "{} {location} parameter `{}` is not sent by this output",
                    if param.required { "required" } else { "optional" },
                    param.name
                ),
            ));
        }
    }
}

/// Lists the type names an endpoint asks the Rust client to spell, with the
/// role each plays.
fn referenced_type_names(endpoint: &Endpoint) -> Vec<(&'static str, &str)> {
    let mut names = Vec::new();
    match &endpoint.request {
        Some(ApiRequest::Json(schema)) => names.push(("request body", schema.type_name.as_str())),
        Some(ApiRequest::FormData { fields }) => {
            for field in fields {
                if let FormFieldKind::Json(schema) = &field.kind {
                    names.push(("form field", schema.type_name.as_str()));
                }
            }
        }
        _ => {}
    }
    if let ApiResponse::Json(schema) = &endpoint.response {
        names.push(("response", schema.type_name.as_str()));
    }
    names
}

/// Checks the fields of an endpoint's request struct and form body struct.
///
/// Field names come from [`sanitize_rust_field_ident`], exactly as the
/// emitters derive them, so distinct wire names can still collide
/// (`{user-id}` and `user_id`). Query builders are `with_<field>`, so a
/// builder collision is a query field collision.
fn check_request_struct_fields(
    api: &RestApi,
    endpoint: &Endpoint,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    let push = |diagnostics: &mut Vec<ValidationDiagnostic>, feature, message: String| {
        diagnostics.push(ValidationDiagnostic::new(
            Severity::Error,
            &api.name,
            Some(&endpoint.id),
            feature,
            Output::Rust,
            message,
        ));
    };

    let mut fields: Vec<(String, String)> = Vec::new();
    for name in extract_path_params(&endpoint.path) {
        fields.push((format!("path parameter `{name}`"), name.to_string()));
    }
    let query: &[ParamDef] = endpoint
        .params
        .as_ref()
        .map(|params| params.query.as_slice())
        .unwrap_or_default();
    for param in query {
        fields.push((format!("query parameter `{}`", param.name), param.name.clone()));
    }

    let mut struct_fields: Vec<(String, String)> = Vec::new();
    for (source, wire) in &fields {
        let field = sanitize_rust_field_ident(wire);
        // An unrepresentable name falls back to an opaque `field`.
        if field == "field" && wire != "field" {
            push(
                diagnostics,
                Feature::InvalidIdentifier,
                format!("{source} has no representable identifier characters"),
            );
        }
        struct_fields.push((source.clone(), field));
    }
    if matches!(
        endpoint.request,
        Some(ApiRequest::Json(_) | ApiRequest::FormData { .. } | ApiRequest::UrlEncoded { .. })
    ) {
        struct_fields.push(("the request body".to_string(), "body".to_string()));
    }
    for message in duplicate_names(&struct_fields, "request struct field") {
        push(diagnostics, Feature::GeneratedNameCollision, message);
    }

    if let Some(ApiRequest::FormData { fields } | ApiRequest::UrlEncoded { fields }) =
        &endpoint.request
    {
        let mut form_fields = Vec::new();
        for field in fields {
            let name = sanitize_rust_field_ident(&field.name);
            if name == "field" && field.name != "field" {
                push(
                    diagnostics,
                    Feature::InvalidIdentifier,
                    format!(
                        "form field `{}` has no representable identifier characters",
                        field.name
                    ),
                );
            }
            form_fields.push((format!("form field `{}`", field.name), name));
        }
        for message in duplicate_names(&form_fields, "form body field") {
            push(diagnostics, Feature::GeneratedNameCollision, message);
        }
    }
}

/// Returns one message per generated name claimed by more than one source.
fn duplicate_names(entries: &[(String, String)], kind: &str) -> Vec<String> {
    let mut by_name: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (source, name) in entries {
        by_name.entry(name).or_default().push(source);
    }
    by_name
        .into_iter()
        .filter(|(_, sources)| sources.len() > 1)
        .map(|(name, sources)| format!("{kind} `{name}` is generated for {}", sources.join(" and ")))
        .collect()
}

/// Checks the convenience methods generated on the client struct.
///
/// Non-JSON endpoints get an inherent method named after the snake-cased
/// endpoint id. It must not repeat another endpoint's method or a method the
/// client always has. The check counts the inherent methods of the client
/// the emitters generate for this API, so it cannot drift from them.
fn check_convenience_methods(api: &RestApi, diagnostics: &mut Vec<ValidationDiagnostic>) {
    let convenience: Vec<(&str, String)> = api
        .endpoints
        .iter()
        .filter(|endpoint| !endpoint.response.is_json())
        .map(|endpoint| (endpoint.id.as_str(), to_snake_case(&endpoint.id)))
        .collect();
    // Emitting the client needs a valid name, suffix, and endpoint ids;
    // invalid ones are already reported.
    let suffix_valid = api
        .request_suffix
        .as_deref()
        .is_none_or(|suffix| !suffix.is_empty() && suffix.chars().all(char::is_alphanumeric));
    if convenience.is_empty()
        || check_type_ident(&api.name).is_err()
        || !suffix_valid
        || convenience.iter().any(|(id, _)| check_type_ident(id).is_err())
    {
        return;
    }

    let generated = client_method_counts(api);
    let mut first_owner: HashMap<&str, &str> = HashMap::new();
    for (id, method) in &convenience {
        let from_endpoints = convenience.iter().filter(|(_, m)| m == method).count();
        let always_present = generated.get(method).copied().unwrap_or(0) > from_endpoints;
        let clash = if always_present {
            Some("a method every generated client has".to_string())
        } else {
            first_owner
                .get(method.as_str())
                .map(|first| format!("the method of endpoint `{first}`"))
        };
        if let Some(clash) = clash {
            diagnostics.push(ValidationDiagnostic::new(
                Severity::Error,
                &api.name,
                Some(id),
                Feature::GeneratedNameCollision,
                Output::Rust,
                format!("client method `{method}` repeats {clash}"),
            ));
        }
        first_owner.entry(method).or_insert(id);
    }
}

/// Counts the inherent methods of the client generated for `api` by name.
fn client_method_counts(api: &RestApi) -> HashMap<String, usize> {
    let suffix = get_request_suffix(api);
    let tokens = {
        let client = crate::codegen::generate_api_struct(api);
        let request = crate::codegen::generate_request_method_with_suffix(api, &suffix);
        quote::quote! { #client #request }
    };
    let Ok(file) = syn::parse2::<syn::File>(tokens) else {
        return HashMap::new();
    };

    let mut counts = HashMap::new();
    for item in file.items {
        let syn::Item::Impl(item_impl) = item else {
            continue;
        };
        let is_client = matches!(
            &*item_impl.self_ty,
            syn::Type::Path(path) if path.path.segments.last().is_some_and(|s| s.ident == api.name)
        );
        if item_impl.trait_.is_some() || !is_client {
            continue;
        }
        for impl_item in item_impl.items {
            if let syn::ImplItem::Fn(function) = impl_item {
                *counts.entry(function.sig.ident.to_string()).or_insert(0) += 1;
            }
        }
    }
    counts
}

/// Checks the type names a module group emits into one Rust module.
///
/// APIs sharing a module path are generated into one namespace, so raw
/// endpoint ids may repeat across them as long as their request suffixes keep
/// the emitted names apart. A referenced body or response type that equals
/// an emitted name is a collision too: the local item would shadow the
/// definitions type.
fn check_group_type_names(group: &[&RestApi], diagnostics: &mut Vec<ValidationDiagnostic>) {
    struct Owner<'a> {
        api: &'a str,
        endpoint: Option<&'a str>,
        role: String,
    }

    let mut emitted: BTreeMap<String, Vec<Owner<'_>>> = BTreeMap::new();
    for name in IMPORTED_TYPE_NAMES {
        emitted.entry((*name).to_string()).or_default();
    }
    for api in group {
        let suffix = get_request_suffix(api);
        let api_items = [
            (api.name.clone(), "client struct"),
            (format!("{}Request", api.name), "request enum"),
            (format!("{}VariantBuilder", api.name), "variant builder"),
        ];
        for (name, role) in api_items {
            emitted.entry(name).or_default().push(Owner {
                api: &api.name,
                endpoint: None,
                role: format!("the {role} of {}", api.name),
            });
        }
        for endpoint in &api.endpoints {
            emitted
                .entry(format!("{}{suffix}", endpoint.id))
                .or_default()
                .push(Owner {
                    api: &api.name,
                    endpoint: Some(&endpoint.id),
                    role: format!("the request struct of {}::{}", api.name, endpoint.id),
                });
            if matches!(
                endpoint.request,
                Some(ApiRequest::FormData { .. } | ApiRequest::UrlEncoded { .. })
            ) {
                emitted
                    .entry(format!("{}Form", endpoint.id))
                    .or_default()
                    .push(Owner {
                        api: &api.name,
                        endpoint: Some(&endpoint.id),
                        role: format!("the form body of {}::{}", api.name, endpoint.id),
                    });
            }
        }
    }

    let diagnostic = |owners: &[&Owner<'_>], message: String| {
        let apis: BTreeSet<&str> = owners.iter().map(|owner| owner.api).collect();
        if apis.len() == 1 {
            let endpoint = owners.iter().rev().find_map(|owner| owner.endpoint);
            ValidationDiagnostic::new(
                Severity::Error,
                owners[0].api,
                endpoint,
                Feature::GeneratedNameCollision,
                Output::Rust,
                message,
            )
        } else {
            ValidationDiagnostic::for_group(
                Severity::Error,
                apis,
                Feature::GeneratedNameCollision,
                Output::Rust,
                message,
            )
        }
    };

    for (name, owners) in &emitted {
        let imported = IMPORTED_TYPE_NAMES.contains(&name.as_str());
        if owners.len() + usize::from(imported) < 2 {
            continue;
        }
        let mut roles: Vec<&str> = owners.iter().map(|owner| owner.role.as_str()).collect();
        if imported {
            roles.push("a name every generated module imports");
        }
        let owners: Vec<&Owner<'_>> = owners.iter().collect();
        diagnostics.push(diagnostic(
            &owners,
            format!("`{name}` is generated for {}", roles.join(" and ")),
        ));
    }

    for api in group {
        let suffix = get_request_suffix(api);
        for endpoint in &api.endpoints {
            for (role, type_name) in referenced_type_names(endpoint) {
                let Some(owners) = emitted.get(type_name) else {
                    continue;
                };
                if owners.is_empty() {
                    continue;
                }
                let own_wrapper = format!("{}{suffix}", endpoint.id);
                let hint = if type_name == own_wrapper {
                    format!(
                        "; rename the {role} type to `{}Body`",
                        type_name.trim_end_matches(suffix.as_str())
                    )
                } else {
                    String::new()
                };
                let referencing = Owner {
                    api: &api.name,
                    endpoint: Some(&endpoint.id),
                    role: String::new(),
                };
                let mut involved: Vec<&Owner<'_>> = owners.iter().collect();
                involved.push(&referencing);
                diagnostics.push(diagnostic(
                    &involved,
                    format!(
                        "{role} type `{type_name}` of {}::{} is shadowed by {}{hint}",
                        api.name,
                        endpoint.id,
                        owners
                            .iter()
                            .map(|owner| owner.role.as_str())
                            .collect::<Vec<_>>()
                            .join(" and ")
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::params::{EndpointParams, ParamStyle, QueryParamType};
    use schematic_define::{ApiKeyLocation, AuthStrategy, FormField, RestMethod};

    fn make_test_api() -> RestApi {
        RestApi {
            name: "TestApi".to_string(),
            description: "Test API".to_string(),
            base_url: "https://api.test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            version: None,
        }
    }

    fn endpoint(id: &str, method: RestMethod, path: &str) -> Endpoint {
        Endpoint {
            id: id.to_string(),
            method,
            path: path.to_string(),
            description: format!("Test endpoint {id}"),
            request: None,
            response: ApiResponse::json_type("Response"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        }
    }

    fn get(id: &str) -> Endpoint {
        endpoint(id, RestMethod::Get, &format!("/{}", id.to_lowercase()))
    }

    fn with_body(id: &str, body_type: &str) -> Endpoint {
        Endpoint {
            request: Some(ApiRequest::json_type(body_type)),
            ..endpoint(id, RestMethod::Post, &format!("/{}", id.to_lowercase()))
        }
    }

    fn param(name: &str, required: bool) -> ParamDef {
        ParamDef {
            name: name.to_string(),
            required,
            description: None,
            param_type: QueryParamType::String,
            explode: false,
            style: ParamStyle::Form,
        }
    }

    fn params(query: Vec<ParamDef>, header: Vec<ParamDef>, cookie: Vec<ParamDef>) -> EndpointParams {
        EndpointParams {
            query,
            header,
            cookie,
            pagination: None,
            response_pagination: None,
        }
    }

    fn api_with(endpoints: Vec<Endpoint>) -> RestApi {
        RestApi {
            endpoints,
            ..make_test_api()
        }
    }

    fn rust(api: &RestApi) -> Vec<ValidationDiagnostic> {
        validate_rust_client(&[api])
    }

    /// Every output's diagnostics, for rules that apply to all of them.
    fn all_outputs(api: &RestApi) -> Vec<ValidationDiagnostic> {
        let mut all = rust(api);
        all.extend(validate_openapi(&[api]));
        all.extend(validate_postman(&[api]));
        all
    }

    fn features(diagnostics: &[ValidationDiagnostic]) -> Vec<(Severity, Feature)> {
        diagnostics.iter().map(|d| (d.severity, d.feature)).collect()
    }

    // ---- accept table ----

    #[test]
    fn accepted_definitions_produce_no_diagnostics_for_any_output() {
        let cases: Vec<(&str, RestApi)> = vec![
            ("empty API", make_test_api()),
            (
                "distinct bodies and routes",
                api_with(vec![
                    with_body("CreateUser", "CreateUserBody"),
                    with_body("UpdateUser", "UserUpdate"),
                    get("ListUsers"),
                ]),
            ),
            (
                "generic and qualified types",
                api_with(vec![
                    Endpoint {
                        response: ApiResponse::json_type("Vec<Model>"),
                        ..get("ListModels")
                    },
                    Endpoint {
                        response: ApiResponse::json_type("serde_json::Value"),
                        ..get("Raw")
                    },
                    Endpoint {
                        response: ApiResponse::json_type("std::collections::HashMap<String, Vec<Model>>"),
                        ..get("Map")
                    },
                    with_body("Patch", "Option<crate::Patch>"),
                ]),
            ),
            (
                "a legitimate repeated segment",
                RestApi {
                    base_url: "https://host.example/api".to_string(),
                    ..api_with(vec![endpoint("Nested", RestMethod::Get, "/api/api/items")])
                },
            ),
            (
                "same path, different methods",
                api_with(vec![
                    endpoint("GetItem", RestMethod::Get, "/items/{id}"),
                    endpoint("DeleteItem", RestMethod::Delete, "/items/{id}"),
                ]),
            ),
            (
                "special characters that sanitize to distinct fields",
                api_with(vec![Endpoint {
                    params: Some(params(vec![param("$top", false), param("user.name", false)], vec![], vec![])),
                    ..endpoint("Search", RestMethod::Get, "/users/{user-id}")
                }]),
            ),
            (
                "alphanumeric custom suffix",
                RestApi {
                    request_suffix: Some("V2".to_string()),
                    ..api_with(vec![with_body("CreateUser", "CreateUserBody")])
                },
            ),
            (
                "form bodies",
                api_with(vec![Endpoint {
                    request: Some(ApiRequest::form_data(vec![
                        FormField::file("document"),
                        FormField::text("title"),
                    ])),
                    ..endpoint("UploadFile", RestMethod::Post, "/upload")
                }]),
            ),
            (
                "non-JSON responses with distinct methods",
                api_with(vec![
                    Endpoint {
                        response: ApiResponse::Text,
                        ..get("GetReadme")
                    },
                    Endpoint {
                        response: ApiResponse::Binary,
                        ..get("GetArchive")
                    },
                ]),
            ),
        ];

        for (name, api) in cases {
            let diagnostics = all_outputs(&api);
            assert!(diagnostics.is_empty(), "{name}: {diagnostics:#?}");
        }
    }

    // ---- reject table (Rust only) ----

    #[test]
    fn rust_only_rejections_name_feature_and_endpoint() {
        let cases: Vec<(&str, RestApi, Feature, Option<&str>)> = vec![
            (
                "suffix with hyphen",
                RestApi {
                    request_suffix: Some("Request-Type".to_string()),
                    ..make_test_api()
                },
                Feature::InvalidRequestSuffix,
                None,
            ),
            (
                "empty suffix",
                RestApi {
                    request_suffix: Some(String::new()),
                    ..make_test_api()
                },
                Feature::InvalidRequestSuffix,
                None,
            ),
            (
                "suffix with underscore",
                RestApi {
                    request_suffix: Some("Request_Type".to_string()),
                    ..make_test_api()
                },
                Feature::InvalidRequestSuffix,
                None,
            ),
            (
                "API name starting with a digit",
                RestApi {
                    name: "9Lives".to_string(),
                    module_path: Some("lives".to_string()),
                    ..make_test_api()
                },
                Feature::InvalidIdentifier,
                None,
            ),
            ("endpoint id with a digit", api_with(vec![get("2fa")]), Feature::InvalidIdentifier, Some("2fa")),
            (
                "endpoint id with punctuation",
                api_with(vec![get("list-models")]),
                Feature::InvalidIdentifier,
                Some("list-models"),
            ),
            ("keyword endpoint id", api_with(vec![get("struct")]), Feature::InvalidIdentifier, Some("struct")),
            (
                "unrepresentable query parameter",
                api_with(vec![Endpoint {
                    params: Some(params(vec![param("", false)], vec![], vec![])),
                    ..get("ListItems")
                }]),
                Feature::InvalidIdentifier,
                Some("ListItems"),
            ),
            (
                "unclosed generic response",
                api_with(vec![Endpoint {
                    response: ApiResponse::json_type("Vec<Model"),
                    ..get("ListModels")
                }]),
                Feature::InvalidTypeSyntax,
                Some("ListModels"),
            ),
            (
                "body type with a space",
                api_with(vec![with_body("Create", "Create Body")]),
                Feature::InvalidTypeSyntax,
                Some("Create"),
            ),
            (
                "form JSON field type",
                api_with(vec![Endpoint {
                    request: Some(ApiRequest::form_data(vec![FormField::json("meta", schematic_define::Schema::new("Meta>"))])),
                    ..endpoint("Upload", RestMethod::Post, "/upload")
                }]),
                Feature::InvalidTypeSyntax,
                Some("Upload"),
            ),
            (
                "module path with a hyphen",
                RestApi {
                    module_path: Some("my-api".to_string()),
                    ..make_test_api()
                },
                Feature::InvalidModulePath,
                None,
            ),
            (
                "qualified module path",
                RestApi {
                    module_path: Some("vendor::api".to_string()),
                    ..make_test_api()
                },
                Feature::InvalidModulePath,
                None,
            ),
            (
                "keyword module path",
                RestApi {
                    module_path: Some("type".to_string()),
                    ..make_test_api()
                },
                Feature::InvalidModulePath,
                None,
            ),
            (
                "module path naming a shared file",
                RestApi {
                    module_path: Some("prelude".to_string()),
                    ..make_test_api()
                },
                Feature::InvalidOutputFilename,
                None,
            ),
            (
                "module path naming a WebSocket module",
                RestApi {
                    module_path: Some("elevenlabs_ws".to_string()),
                    ..make_test_api()
                },
                Feature::InvalidOutputFilename,
                None,
            ),
            (
                "path and query parameter sanitize to one field",
                api_with(vec![Endpoint {
                    params: Some(params(vec![param("user_id", false)], vec![], vec![])),
                    ..endpoint("GetUser", RestMethod::Get, "/users/{user-id}")
                }]),
                Feature::GeneratedNameCollision,
                Some("GetUser"),
            ),
            (
                "path parameter repeated",
                api_with(vec![endpoint("Pair", RestMethod::Get, "/a/{id}/b/{id}")]),
                Feature::GeneratedNameCollision,
                Some("Pair"),
            ),
            (
                "query parameter named body beside a body",
                api_with(vec![Endpoint {
                    params: Some(params(vec![param("body", false)], vec![], vec![])),
                    ..with_body("Create", "CreateBody")
                }]),
                Feature::GeneratedNameCollision,
                Some("Create"),
            ),
            (
                "form fields sanitize to one field",
                api_with(vec![Endpoint {
                    request: Some(ApiRequest::form_data(vec![
                        FormField::text("file-name"),
                        FormField::text("file_name"),
                    ])),
                    ..endpoint("Upload", RestMethod::Post, "/upload")
                }]),
                Feature::GeneratedNameCollision,
                Some("Upload"),
            ),
            (
                "body type equals its request struct",
                api_with(vec![with_body("CreateUser", "CreateUserRequest")]),
                Feature::GeneratedNameCollision,
                Some("CreateUser"),
            ),
            (
                "body type equals its request struct with a custom suffix",
                RestApi {
                    request_suffix: Some("Params".to_string()),
                    ..api_with(vec![with_body("CreateUser", "CreateUserParams")])
                },
                Feature::GeneratedNameCollision,
                Some("CreateUser"),
            ),
            (
                "response type equals another endpoint's form body",
                api_with(vec![
                    Endpoint {
                        request: Some(ApiRequest::form_data(vec![FormField::text("a")])),
                        ..endpoint("Upload", RestMethod::Post, "/upload")
                    },
                    Endpoint {
                        response: ApiResponse::json_type("UploadForm"),
                        ..get("Status")
                    },
                ]),
                Feature::GeneratedNameCollision,
                Some("Status"),
            ),
            (
                "endpoint struct clashes with an import",
                RestApi {
                    request_suffix: Some("Parts".to_string()),
                    ..api_with(vec![get("Request")])
                },
                Feature::GeneratedNameCollision,
                Some("Request"),
            ),
            (
                "snake-cased convenience methods collide",
                api_with(vec![
                    Endpoint {
                        response: ApiResponse::Text,
                        ..get("FooBar")
                    },
                    Endpoint {
                        response: ApiResponse::Text,
                        ..endpoint("Foo_bar", RestMethod::Get, "/foo_bar2")
                    },
                ]),
                Feature::GeneratedNameCollision,
                Some("Foo_bar"),
            ),
            (
                "convenience method repeats a fixed client method",
                api_with(vec![Endpoint {
                    response: ApiResponse::Empty,
                    ..get("RequestEmpty")
                }]),
                Feature::GeneratedNameCollision,
                Some("RequestEmpty"),
            ),
            (
                "text request body",
                api_with(vec![Endpoint {
                    request: Some(ApiRequest::Text {
                        content_type: "text/plain".to_string(),
                    }),
                    ..endpoint("Note", RestMethod::Post, "/note")
                }]),
                Feature::TextRequestBody,
                Some("Note"),
            ),
            (
                "binary request body",
                api_with(vec![Endpoint {
                    request: Some(ApiRequest::Binary {
                        content_type: "application/octet-stream".to_string(),
                    }),
                    ..endpoint("Blob", RestMethod::Put, "/blob")
                }]),
                Feature::BinaryRequestBody,
                Some("Blob"),
            ),
            (
                "legacy API-key parameter auth",
                RestApi {
                    auth: AuthStrategy::ApiKeyParam {
                        name: "api_key".to_string(),
                        location: ApiKeyLocation::Query,
                    },
                    ..make_test_api()
                },
                Feature::ApiKeyParam,
                None,
            ),
        ];

        for (name, api, feature, endpoint) in cases {
            let diagnostics = rust(&api);
            let found = diagnostics
                .iter()
                .find(|d| d.feature == feature)
                .unwrap_or_else(|| panic!("{name}: no {feature} in {diagnostics:#?}"));
            assert_eq!(found.severity, Severity::Error, "{name}");
            assert_eq!(found.output, Output::Rust, "{name}");
            assert_eq!(found.api, vec![api.name.clone()], "{name}");
            assert_eq!(found.endpoint.as_deref(), endpoint, "{name}");

            // Rust-only rules never reach the exports.
            let exports: Vec<_> = validate_openapi(&[&api])
                .into_iter()
                .chain(validate_postman(&[&api]))
                .filter(|d| d.feature == feature)
                .collect();
            assert!(exports.is_empty(), "{name}: exports blocked by {exports:#?}");
        }
    }

    #[test]
    fn body_collision_message_suggests_a_rename() {
        let api = api_with(vec![with_body("CreateUser", "CreateUserRequest")]);
        let diagnostic = &rust(&api)[0];

        assert!(diagnostic.message.contains("CreateUserRequest"), "{diagnostic}");
        assert!(diagnostic.message.contains("`CreateUserBody`"), "{diagnostic}");
    }

    #[test]
    fn api_key_param_is_found_through_the_effective_policy() {
        use schematic_define::AuthPolicy;

        let explicit = RestApi {
            auth_policy: Some(AuthPolicy {
                explicit: vec![AuthMethod::ApiKeyParam {
                    name: "token".to_string(),
                    location: ApiKeyLocation::Cookie,
                }],
                env_fallback: None,
            }),
            ..make_test_api()
        };
        // A policy override that drops the legacy strategy hides it.
        let overridden = RestApi {
            auth: AuthStrategy::ApiKeyParam {
                name: "api_key".to_string(),
                location: ApiKeyLocation::Query,
            },
            auth_policy: Some(AuthPolicy::default()),
            ..make_test_api()
        };

        assert_eq!(
            features(&rust(&explicit)),
            vec![(Severity::Error, Feature::ApiKeyParam)]
        );
        assert!(rust(&overridden).is_empty());
    }

    // ---- rules for every output ----

    #[test]
    fn duplicate_endpoint_id_in_one_api_fails_every_output() {
        let api = api_with(vec![
            endpoint("GetItem", RestMethod::Get, "/items/{id}"),
            endpoint("GetItem", RestMethod::Get, "/other/{id}"),
        ]);

        for output in [Output::Rust, Output::OpenApi, Output::Postman] {
            let diagnostics: Vec<_> = all_outputs(&api)
                .into_iter()
                .filter(|d| d.output == output && d.feature == Feature::DuplicateEndpointId)
                .collect();
            assert_eq!(diagnostics.len(), 1, "{output}: {diagnostics:#?}");
            assert_eq!(diagnostics[0].severity, Severity::Error);
            assert_eq!(diagnostics[0].endpoint.as_deref(), Some("GetItem"));
        }
    }

    #[test]
    fn path_parameter_names_do_not_distinguish_routes() {
        let collide = [
            endpoint("ById", RestMethod::Get, "/items/{id}"),
            endpoint("ByName", RestMethod::Get, "/items/{name}"),
        ];
        let distinct = [
            endpoint("Read", RestMethod::Get, "/items/{id}"),
            endpoint("Remove", RestMethod::Delete, "/items/{name}"),
        ];

        assert_eq!(
            endpoint_uniqueness_key(&collide[0]),
            endpoint_uniqueness_key(&collide[1])
        );
        assert_eq!(endpoint_uniqueness_key(&collide[0]).normalized_path, "/items/{}");
        assert_ne!(
            endpoint_uniqueness_key(&distinct[0]),
            endpoint_uniqueness_key(&distinct[1])
        );
        // The reserved-expansion marker is a placeholder like any other.
        assert_eq!(
            endpoint_uniqueness_key(&endpoint("Raw", RestMethod::Get, "/repos/{+repo}/x"))
                .normalized_path,
            "/repos/{}/x"
        );

        let api = api_with(collide.to_vec());
        for output in [Output::Rust, Output::OpenApi, Output::Postman] {
            let found: Vec<_> = all_outputs(&api)
                .into_iter()
                .filter(|d| d.output == output)
                .collect();
            assert_eq!(
                features(&found),
                vec![(DUPLICATE_METHOD_PATH_SEVERITY, Feature::DuplicateMethodPath)]
            );
            assert_eq!(found[0].endpoint.as_deref(), Some("ByName"));
            assert!(found[0].message.contains("`ById`"), "{}", found[0]);
        }
        assert!(all_outputs(&api_with(distinct.to_vec())).is_empty());
    }

    #[test]
    fn query_in_path_is_one_warning_per_output() {
        let api = api_with(vec![
            endpoint("Tree", RestMethod::Get, "/tree?recursive=true"),
            get("Clean"),
        ]);

        let diagnostics = all_outputs(&api);
        assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
        for diagnostic in &diagnostics {
            assert_eq!(diagnostic.feature, Feature::QueryInPath);
            assert_eq!(diagnostic.severity, QUERY_IN_PATH_SEVERITY);
            assert_eq!(diagnostic.endpoint.as_deref(), Some("Tree"));
        }
        let outputs: BTreeSet<_> = diagnostics.iter().map(|d| d.output).collect();
        assert_eq!(outputs.len(), 3);
    }

    #[test]
    fn interim_route_checks_are_warnings() {
        assert_eq!(QUERY_IN_PATH_SEVERITY, Severity::Warning);
        assert_eq!(DUPLICATE_METHOD_PATH_SEVERITY, Severity::Warning);
    }

    #[test]
    fn export_file_names_must_be_one_safe_component() {
        let bad = ["", ".", "..", "a/b", "a\\b", "C:", "con|x", "x\0y"];
        for module_path in bad {
            let api = RestApi {
                module_path: Some(module_path.to_string()),
                ..make_test_api()
            };
            for (output, diagnostics) in [
                (Output::OpenApi, validate_openapi(&[&api])),
                (Output::Postman, validate_postman(&[&api])),
            ] {
                assert_eq!(
                    features(&diagnostics),
                    vec![(Severity::Error, Feature::InvalidOutputFilename)],
                    "{module_path:?} {output}"
                );
            }
        }
        for module_path in ["openai", "unfolded_circle_core_rest", "my-api", "v1.2"] {
            let api = RestApi {
                module_path: Some(module_path.to_string()),
                ..make_test_api()
            };
            assert!(validate_openapi(&[&api]).is_empty(), "{module_path}");
            assert!(validate_postman(&[&api]).is_empty(), "{module_path}");
        }
    }

    #[test]
    fn unsent_header_and_cookie_parameters_are_reported_by_requiredness() {
        let api = api_with(vec![Endpoint {
            params: Some(params(
                vec![],
                vec![param("x-optional", false), param("x-required", true)],
                vec![param("session", false)],
            )),
            ..get("Items")
        }]);

        for (output, diagnostics) in [
            (Output::Rust, rust(&api)),
            (Output::Postman, validate_postman(&[&api])),
        ] {
            assert_eq!(
                features(&diagnostics),
                vec![
                    (Severity::Warning, Feature::HeaderParameter),
                    (Severity::Error, Feature::HeaderParameter),
                    (Severity::Warning, Feature::CookieParameter),
                ],
                "{output}"
            );
        }
        // OpenAPI declares header and cookie parameters.
        assert!(validate_openapi(&[&api]).is_empty());
    }

    // ---- module groups ----

    fn group_member(name: &str, suffix: Option<&str>, endpoints: Vec<Endpoint>) -> RestApi {
        RestApi {
            name: name.to_string(),
            module_path: Some("shared_group".to_string()),
            request_suffix: suffix.map(str::to_string),
            endpoints,
            ..make_test_api()
        }
    }

    #[test]
    fn group_members_may_share_raw_ids_when_suffixes_differ() {
        let native = group_member("Native", None, vec![get("ListModels")]);
        let compat = group_member("Compat", Some("CompatRequest"), vec![get("ListModels")]);

        assert!(validate_rust_client(&[&native, &compat]).is_empty());
        // Raw ids are per-API for the exports as well.
        assert!(validate_openapi(&[&native, &compat]).is_empty());
        assert!(validate_postman(&[&native, &compat]).is_empty());
    }

    #[test]
    fn group_members_whose_emitted_names_collide_name_both_apis() {
        let first = group_member("First", None, vec![get("ListModels")]);
        let second = group_member("Second", None, vec![get("ListModels")]);

        let diagnostics = validate_rust_client(&[&first, &second]);
        assert_eq!(
            features(&diagnostics),
            vec![(Severity::Error, Feature::GeneratedNameCollision)]
        );
        assert_eq!(diagnostics[0].api, vec!["First", "Second"]);
        assert_eq!(diagnostics[0].endpoint, None);
        assert!(diagnostics[0].message.contains("`ListModelsRequest`"));

        // Separate modules keep separate namespaces.
        let elsewhere = RestApi {
            module_path: Some("other".to_string()),
            ..second.clone()
        };
        assert!(validate_rust_client(&[&first, &elsewhere]).is_empty());
    }

    #[test]
    fn live_shared_groups_are_accepted() {
        use schematic_definitions::emqx::{define_emqx_basic_api, define_emqx_bearer_api};
        use schematic_definitions::ollama::{define_ollama_native_api, define_ollama_openai_api};

        let groups = [
            [define_ollama_native_api(), define_ollama_openai_api()],
            [define_emqx_basic_api(), define_emqx_bearer_api()],
        ];
        for [a, b] in &groups {
            // Both groups repeat raw endpoint ids across their members.
            let shared_ids = a
                .endpoints
                .iter()
                .filter(|e| b.endpoints.iter().any(|f| f.id == e.id))
                .count();
            assert!(shared_ids > 0, "{} and {} share no ids", a.name, b.name);

            let errors: Vec<_> = validate_rust_client(&[a, b])
                .into_iter()
                .filter(ValidationDiagnostic::is_error)
                .collect();
            assert!(errors.is_empty(), "{errors:#?}");
        }
    }
}
