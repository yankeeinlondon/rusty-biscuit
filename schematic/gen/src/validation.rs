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
//! use schematic_define::{ApiResponse, DefinitionContext, Endpoint, RestApi};
//! use schematic_gen::diagnostics::{Feature, Severity};
//! use schematic_gen::validation::{validate_postman, validate_rust_client};
//!
//! let ctx = DefinitionContext::types_in("fixture");
//! let mut endpoint = Endpoint::get("ListItems", "/items")
//!     .describes("List items").returns_empty().build().unwrap();
//! // Malformed stored data blocks the Rust client, not the exports.
//! endpoint.response = ApiResponse::json_type("Vec<Item");
//! let api = RestApi::builder(&ctx, "TestApi", "https://api.test.com")
//!     .describes("Test API").no_auth().endpoints([endpoint]).build().unwrap();
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
    ApiKeyLocation, ApiRequest, ApiResponse, AuthMethod, Endpoint, EnvAuthStrategy, FormFieldKind,
    ParamDef, QueryParamType, RestApi,
};
use schematic_define::pagination::{
    PaginationRuleGroup, Resolution, Step, resolve, validate_pagination,
};
use schematic_definitions::registry::{SchemaRegistry, get_registry, registry_key_for};
use serde_json::Value;

use crate::codegen::endpoint_methods::{call_type_name, resolve_names};
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

/// Names every generated module brings into scope with `use`.
///
/// A generated item with one of these names would clash with the import.
const IMPORTED_TYPE_NAMES: &[&str] = &["Deserialize", "Serialize"];

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
/// group, endpoint-method and setter renames (warnings), and the request
/// inputs the client sends verbatim: header and cookie parameter names, raw
/// body content types, and API-key parameter names.
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
/// Checks route identity, the `query-in-path` warning, and the collection
/// file name.
pub fn validate_postman(apis: &[&RestApi]) -> Vec<ValidationDiagnostic> {
    let mut diagnostics = Vec::new();
    for api in apis {
        check_routes(api, Output::Postman, &mut diagnostics);
        check_export_filename(api, Output::Postman, &mut diagnostics);
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
            diagnostics.push(ValidationDiagnostic::new(
                Severity::Error,
                &api.name,
                Some(&endpoint.id),
                Feature::DuplicateMethodPath,
                output,
                format!(
                    "{} {} is already declared by endpoint `{first}`; an endpoint is its \
                     method and path, so merge the two into one endpoint",
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

    // The API name becomes the client struct and `{name}Api`, its tag type.
    if let Err(reason) = check_type_ident(&api.name) {
        diagnostics.push(error(
            None,
            Feature::InvalidIdentifier,
            format!("API name `{}` {reason}", api.name),
        ));
    }

    check_module_path(api, diagnostics);
    check_api_key_param(api, diagnostics);
    check_endpoint_method_names(api, diagnostics);

    for endpoint in &api.endpoints {
        let id = Some(endpoint.id.as_str());

        // Endpoint ids become request and form struct identifiers.
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
        check_raw_body_content_type(api, endpoint, diagnostics);
        check_sent_params(api, endpoint, diagnostics);
    }

    check_pagination(api, diagnostics);
}

/// Checks the API's pagination: the define-side rules, then the item types
/// and item locations the Rust client relies on.
///
/// Item checks run only for paginated lists. A named item type is checked
/// against the API's schema registry, top-level and nested schemas alike, and
/// an item location is walked through the response type's registered schema.
/// Where the registry cannot describe a type (no registry, an unregistered or
/// qualified type, an unresolvable reference), the check is skipped and the
/// runtime enforces the location instead; nothing is reported as checked.
fn check_pagination(api: &RestApi, diagnostics: &mut Vec<ValidationDiagnostic>) {
    if api.default_pagination.is_none() && api.pagination_exceptions.is_empty() {
        return;
    }
    let error = |endpoint: Option<&str>, feature: Feature, message: String| {
        ValidationDiagnostic::new(Severity::Error, &api.name, endpoint, feature, Output::Rust, message)
    };

    for diagnostic in validate_pagination(api) {
        let feature = match diagnostic.error.group() {
            PaginationRuleGroup::Strategy => Feature::PaginationStrategy,
            PaginationRuleGroup::Exception => Feature::PaginationException,
            PaginationRuleGroup::Parameter => Feature::PaginationParameter,
        };
        diagnostics.push(error(diagnostic.endpoint.as_deref(), feature, diagnostic.to_string()));
    }

    // Built on first use: most APIs name no item type the registry could check.
    let mut registry: Option<Option<SchemaRegistry>> = None;
    let mut known_names: Option<BTreeSet<String>> = None;
    for endpoint in &api.endpoints {
        let Resolution::Paginated { list, .. } = resolve(api, endpoint) else {
            continue;
        };
        let id = Some(endpoint.id.as_str());
        let authored_type = api
            .pagination_exceptions
            .iter()
            .find(|entry| entry.endpoint == endpoint.id)
            .and_then(|entry| entry.collection.item_type.as_deref());

        if let Some(item_type) = authored_type.filter(|ty| !ty.trim().is_empty()) {
            if syn::parse_str::<syn::Type>(item_type).is_err() {
                diagnostics.push(error(
                    id,
                    Feature::PaginationException,
                    format!("invalid-item-type: item type `{item_type}` is not valid Rust type syntax"),
                ));
            } else if is_plain_named_type(item_type) {
                let registry = registry.get_or_insert_with(|| get_registry(&registry_key_for(&api.name)));
                if let Some(registry) = registry {
                    let names = known_names.get_or_insert_with(|| {
                        registry.to_openapi_schemas().into_keys().collect()
                    });
                    if !names.contains(item_type) {
                        diagnostics.push(error(
                            id,
                            Feature::PaginationException,
                            format!(
                                "unknown-item-type: item type `{item_type}` is not a schema of \
                                 this API's registry; use a registered type, a primitive, or a \
                                 qualified path such as `serde_json::Value`"
                            ),
                        ));
                    }
                }
            }
        }

        let ApiResponse::Json(schema) = &endpoint.response else {
            continue;
        };
        if list.items.is_root_array() || !is_plain_named_type(&schema.type_name) {
            continue;
        }
        let registry = registry.get_or_insert_with(|| get_registry(&registry_key_for(&api.name)));
        let Some(root) = registry.as_ref().and_then(|r| r.get(&schema.type_name)) else {
            continue;
        };
        if item_location_is_array(root.as_value(), &list.items.record) == Some(false) {
            diagnostics.push(error(
                id,
                Feature::PaginationException,
                format!(
                    "not-an-array-location: `{}` is not an array in the registered schema of `{}`",
                    list.items, schema.type_name
                ),
            ));
        }
    }
}

/// Whether `type_name` is a single identifier naming a non-primitive type,
/// the only kind a schema registry can know.
fn is_plain_named_type(type_name: &str) -> bool {
    const PRIMITIVES: &[&str] = &[
        "bool", "char", "str", "String", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16",
        "u32", "u64", "u128", "usize", "f32", "f64",
    ];
    syn::parse_str::<syn::Ident>(type_name).is_ok() && !PRIMITIVES.contains(&type_name)
}

/// Walks a selector's record steps through a JSON schema.
///
/// `Some(false)` means the schema demonstrably has no array at some fan-out;
/// `Some(true)` that every fan-out is an array; `None` that the schema cannot
/// tell (a missing property, an unresolved reference, a union).
fn item_location_is_array(root: &Value, steps: &[Step]) -> Option<bool> {
    let mut node = root;
    for step in steps {
        node = resolve_schema_node(root, node)?;
        match step {
            Step::Key(key) => node = node.get("properties")?.get(key)?,
            Step::Each => {
                let types: Vec<&str> = match node.get("type")? {
                    Value::String(single) => vec![single.as_str()],
                    Value::Array(many) => many.iter().filter_map(Value::as_str).collect(),
                    _ => return None,
                };
                if !types.contains(&"array") {
                    return Some(false);
                }
                node = node.get("items")?;
            }
        }
    }
    Some(true)
}

/// Follows `$ref`, a single-member `allOf`, and a nullable `anyOf`/`oneOf`
/// to the schema node that describes a value.
fn resolve_schema_node<'a>(root: &'a Value, mut node: &'a Value) -> Option<&'a Value> {
    // A bound against reference cycles; real schemas need a few hops.
    for _ in 0..32 {
        if let Some(reference) = node.get("$ref").and_then(Value::as_str) {
            node = root.pointer(reference.strip_prefix('#')?)?;
            continue;
        }
        if let Some([only]) = node.get("allOf").and_then(Value::as_array).map(Vec::as_slice) {
            node = only;
            continue;
        }
        if let Some(variants) = node
            .get("anyOf")
            .or_else(|| node.get("oneOf"))
            .and_then(Value::as_array)
        {
            let mut non_null = variants
                .iter()
                .filter(|variant| variant.get("type").and_then(Value::as_str) != Some("null"));
            match (non_null.next(), non_null.next()) {
                (Some(only), None) => {
                    node = only;
                    continue;
                }
                _ => return None,
            }
        }
        return Some(node);
    }
    None
}

/// Reports each endpoint-method or setter rename as a warning, and each name
/// still claimed twice after renaming as an error.
fn check_endpoint_method_names(api: &RestApi, diagnostics: &mut Vec<ValidationDiagnostic>) {
    let names = resolve_names(api);
    for rename in &names.renames {
        diagnostics.push(ValidationDiagnostic::new(
            Severity::Warning,
            &api.name,
            Some(&rename.endpoint),
            Feature::EndpointMethodRename,
            Output::Rust,
            rename.to_string(),
        ));
    }
    for conflict in &names.conflicts {
        diagnostics.push(ValidationDiagnostic::new(
            Severity::Error,
            &api.name,
            conflict.endpoints.last().map(String::as_str),
            Feature::GeneratedNameCollision,
            Output::Rust,
            conflict.to_string(),
        ));
    }
}

/// Returns why `name` cannot be used as a generated type name.
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

/// Checks the parameter names of API-key parameter authentication.
///
/// Reads the effective policy, so explicit methods and environment fallbacks
/// count as well as the legacy `auth` field. A cookie key needs a valid
/// cookie name; a query key only a non-empty one, since it is percent-encoded.
fn check_api_key_param(api: &RestApi, diagnostics: &mut Vec<ValidationDiagnostic>) {
    let policy = api.effective_auth_policy();
    let explicit = policy.explicit.iter().filter_map(|method| match method {
        AuthMethod::ApiKeyParam { name, location } => Some((name, location)),
        _ => None,
    });
    let fallback = match &policy.env_fallback {
        Some(EnvAuthStrategy::ApiKeyParam { name, location }) => Some((name, location)),
        _ => None,
    };
    for (name, location) in explicit.chain(fallback) {
        let problem = match location {
            ApiKeyLocation::Cookie if !is_http_token(name) => "is not a valid cookie name",
            _ if name.is_empty() => "is empty",
            _ => continue,
        };
        diagnostics.push(ValidationDiagnostic::new(
            Severity::Error,
            &api.name,
            None,
            Feature::ApiKeyParam,
            Output::Rust,
            format!("API-key parameter name `{name}` {problem}"),
        ));
    }
}

/// Checks the declared content type of a text or binary request body, which
/// the client sends verbatim as the `Content-Type` header value.
fn check_raw_body_content_type(
    api: &RestApi,
    endpoint: &Endpoint,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    let (feature, content_type) = match &endpoint.request {
        Some(ApiRequest::Text { content_type }) => (Feature::TextRequestBody, content_type),
        Some(ApiRequest::Binary { content_type }) => (Feature::BinaryRequestBody, content_type),
        _ => return,
    };
    if !content_type.is_empty() && is_header_value(content_type) {
        return;
    }
    diagnostics.push(ValidationDiagnostic::new(
        Severity::Error,
        &api.name,
        Some(&endpoint.id),
        feature,
        Output::Rust,
        format!("content type `{}` is not a valid header value", content_type.escape_debug()),
    ));
}

/// Checks the query, header, and cookie parameters the client sends.
///
/// Header and cookie names are sent as written, so they must be HTTP tokens;
/// query names are percent-encoded and need no check here. Every location
/// sends each array element through its `to_string()`, so an array of arrays
/// has no serialization anywhere and is rejected rather than dropped.
fn check_sent_params(api: &RestApi, endpoint: &Endpoint, diagnostics: &mut Vec<ValidationDiagnostic>) {
    let Some(params) = &endpoint.params else {
        return;
    };
    let located = [
        (Feature::QueryParameter, "query", &params.query, false),
        (Feature::HeaderParameter, "header", &params.header, true),
        (Feature::CookieParameter, "cookie", &params.cookie, true),
    ];
    for (feature, location, declared, wire_name_is_token) in located {
        for param in declared {
            let mut push = |message: String| {
                diagnostics.push(ValidationDiagnostic::new(
                    Severity::Error,
                    &api.name,
                    Some(&endpoint.id),
                    feature,
                    Output::Rust,
                    message,
                ));
            };
            if let Some(name) = &param.rust_name
                && (syn::parse_str::<syn::Ident>(name).is_err() || name == "self" || name == "Self" || name == "crate" || name == "super" || name == "_")
            {
                push(format!("{location} parameter `{}` has invalid Rust name `{name}`", param.name));
            }
            if wire_name_is_token && !is_http_token(&param.name) {
                push(format!(
                    "{location} parameter name `{}` is not a valid {location} name",
                    param.name.escape_debug()
                ));
            }
            let nested = matches!(
                &param.param_type,
                QueryParamType::Array(inner) if matches!(**inner, QueryParamType::Array(_))
            );
            if nested {
                push(format!(
                    "{location} parameter `{}` is an array of arrays, which has no {location} \
                     serialization",
                    param.name
                ));
            }
        }
    }
}

/// An RFC 9110 token, the grammar of header and cookie names.
fn is_http_token(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
}

/// Whether `value` is a valid HTTP header value: visible ASCII, spaces, tabs.
fn is_header_value(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte == b'\t' || (0x20..=0x7E).contains(&byte))
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
/// Field names preserve explicit Rust names or use [`sanitize_rust_field_ident`],
/// exactly as the emitters derive them, so distinct wire names can still collide
/// (`{user-id}` and `user_id`). Query, header, and cookie builders are all
/// `with_<field>`, so a builder collision is a field collision.
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

    let mut fields: Vec<(String, String, bool)> = Vec::new();
    for name in extract_path_params(&endpoint.path) {
        fields.push((format!("path parameter `{name}`"), name.to_string(), false));
    }
    let no_params: &[ParamDef] = &[];
    let (query, header, cookie) = endpoint.params.as_ref().map_or(
        (no_params, no_params, no_params),
        |params| (params.query.as_slice(), params.header.as_slice(), params.cookie.as_slice()),
    );
    for (location, declared) in [("query", query), ("header", header), ("cookie", cookie)] {
        for param in declared {
            let field = param.rust_name.as_ref().unwrap_or(&param.name).clone();
            fields.push((format!("{location} parameter `{}`", param.name), field, param.rust_name.is_some()));
        }
    }

    let mut struct_fields: Vec<(String, String)> = Vec::new();
    for (source, wire, explicit) in &fields {
        let field = if *explicit { wire.clone() } else { sanitize_rust_field_ident(wire) };
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
        Some(
            ApiRequest::Json(_)
                | ApiRequest::FormData { .. }
                | ApiRequest::UrlEncoded { .. }
                | ApiRequest::Text { .. }
                | ApiRequest::Binary { .. }
        )
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
            (api.name.clone(), "API client struct"),
            (crate::codegen::api_struct::api_tag_name(api), "API tag type"),
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
            emitted
                .entry(call_type_name(&format!("{}{suffix}", endpoint.id)))
                .or_default()
                .push(Owner {
                    api: &api.name,
                    endpoint: Some(&endpoint.id),
                    role: format!("the call type of {}::{}", api.name, endpoint.id),
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

    #[test]
    fn rust_parameter_aliases_are_validated_and_collide_by_rust_name_only() {
        let mut ep = get("List");
        let mut reference = param("ref", false);
        reference.rust_name = Some("git_ref".into());
        ep.params = Some(params(vec![reference.clone(), param("git_ref", false)], vec![], vec![]));
        let api = api_with(vec![ep.clone()]);
        assert!(validate_rust_client(&[&api]).iter().any(|d| d.feature == Feature::GeneratedNameCollision));
        assert!(validate_openapi(&[&api]).is_empty());
        assert!(validate_postman(&[&api]).is_empty());
        reference.rust_name = Some("not a name".into());
        ep.params = Some(params(vec![reference], vec![], vec![]));
        let api = api_with(vec![ep]);
        assert!(validate_rust_client(&[&api]).iter().any(|d| d.message.contains("invalid Rust name")));
        assert!(validate_openapi(&[&api]).is_empty());
        assert!(validate_postman(&[&api]).is_empty());
    }

    fn make_test_api() -> RestApi {
        {
            schematic_define::RestApi::builder(&schematic_define::DefinitionContext::types_in("fixture"), "TestApi".to_string(), "https://api.test.com".to_string())
                .describes("Test API".to_string())
                .authentication(AuthStrategy::None, Vec::<String>::new())
                .build().expect("valid API fixture")
        }
    }

    fn endpoint(id: &str, method: RestMethod, path: &str) -> Endpoint {
        {
            let mut endpoint = Endpoint::new(id.to_string(), method, path.to_string())
                .describes(format!("Test endpoint {id}"))
                .returns_empty()
                .build().expect("valid endpoint fixture");
            endpoint.response = ApiResponse::json_type("Response");
            endpoint
        }
    }

    fn get(id: &str) -> Endpoint {
        endpoint(id, RestMethod::Get, &format!("/{}", id.to_lowercase()))
    }

    fn with_body(id: &str, body_type: &str) -> Endpoint {
        {
            let mut endpoint = endpoint(id, RestMethod::Post, &format!("/{}", id.to_lowercase()));
            endpoint.request = Some(ApiRequest::json_type(body_type));
            endpoint
        }
    }

    fn param(name: &str, required: bool) -> ParamDef {
        ParamDef { rust_name: None, secret: false, pager_managed: false,
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
        {
            let mut api = make_test_api();
            api.endpoints = endpoints;
            api
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
                    {
                        let mut endpoint = get("ListModels");
                        endpoint.response = ApiResponse::json_type("Vec<Model>");
                        endpoint
                    },
                    {
                        let mut endpoint = get("Raw");
                        endpoint.response = ApiResponse::json_type("serde_json::Value");
                        endpoint
                    },
                    {
                        let mut endpoint = get("Map");
                        endpoint.response = ApiResponse::json_type("std::collections::HashMap<String, Vec<Model>>");
                        endpoint
                    },
                    with_body("Patch", "Option<crate::Patch>"),
                ]),
            ),
            (
                "a legitimate repeated segment",
                {
                    let mut api = api_with(vec![endpoint("Nested", RestMethod::Get, "/api/api/items")]);
                    api.base_url = "https://host.example/api".to_string();
                    api
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
                api_with(vec![{
                    let mut endpoint = endpoint("Search", RestMethod::Get, "/users/{user-id}");
                    endpoint.params = Some(params(vec![param("$top", false), param("user.name", false)], vec![], vec![]));
                    endpoint
                }]),
            ),
            (
                "alphanumeric custom suffix",
                {
                    let mut api = api_with(vec![with_body("CreateUser", "CreateUserBody")]);
                    api.request_suffix = Some("V2".to_string());
                    api
                },
            ),
            (
                "form bodies",
                api_with(vec![{
                    let mut endpoint = endpoint("UploadFile", RestMethod::Post, "/upload");
                    endpoint.request = Some(ApiRequest::form_data(vec![
                        FormField::file("document"),
                        FormField::text("title"),
                    ]));
                    endpoint
                }]),
            ),
            (
                "non-JSON responses with distinct methods",
                api_with(vec![
                    {
                        let mut endpoint = get("GetReadme");
                        endpoint.response = ApiResponse::Text;
                        endpoint
                    },
                    {
                        let mut endpoint = get("GetArchive");
                        endpoint.response = ApiResponse::Binary;
                        endpoint
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
                {
                    let mut api = make_test_api();
                    api.request_suffix = Some("Request-Type".to_string());
                    api
                },
                Feature::InvalidRequestSuffix,
                None,
            ),
            (
                "empty suffix",
                {
                    let mut api = make_test_api();
                    api.request_suffix = Some(String::new());
                    api
                },
                Feature::InvalidRequestSuffix,
                None,
            ),
            (
                "suffix with underscore",
                {
                    let mut api = make_test_api();
                    api.request_suffix = Some("Request_Type".to_string());
                    api
                },
                Feature::InvalidRequestSuffix,
                None,
            ),
            (
                "API name starting with a digit",
                {
                    let mut api = make_test_api();
                    api.name = "9Lives".to_string();
                    api.module_path = Some("lives".to_string());
                    api
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
                api_with(vec![{
                    let mut endpoint = get("ListItems");
                    endpoint.params = Some(params(vec![param("", false)], vec![], vec![]));
                    endpoint
                }]),
                Feature::InvalidIdentifier,
                Some("ListItems"),
            ),
            (
                "unclosed generic response",
                api_with(vec![{
                    let mut endpoint = get("ListModels");
                    endpoint.response = ApiResponse::json_type("Vec<Model");
                    endpoint
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
                api_with(vec![{
                    let mut endpoint = endpoint("Upload", RestMethod::Post, "/upload");
                    endpoint.request = Some(ApiRequest::form_data(vec![FormField::json("meta", schematic_define::Schema::new("Meta>"))]));
                    endpoint
                }]),
                Feature::InvalidTypeSyntax,
                Some("Upload"),
            ),
            (
                "module path with a hyphen",
                {
                    let mut api = make_test_api();
                    api.module_path = Some("my-api".to_string());
                    api
                },
                Feature::InvalidModulePath,
                None,
            ),
            (
                "qualified module path",
                {
                    let mut api = make_test_api();
                    api.module_path = Some("vendor::api".to_string());
                    api
                },
                Feature::InvalidModulePath,
                None,
            ),
            (
                "keyword module path",
                {
                    let mut api = make_test_api();
                    api.module_path = Some("type".to_string());
                    api
                },
                Feature::InvalidModulePath,
                None,
            ),
            (
                "module path naming a shared file",
                {
                    let mut api = make_test_api();
                    api.module_path = Some("prelude".to_string());
                    api
                },
                Feature::InvalidOutputFilename,
                None,
            ),
            (
                "module path naming a WebSocket module",
                {
                    let mut api = make_test_api();
                    api.module_path = Some("elevenlabs_ws".to_string());
                    api
                },
                Feature::InvalidOutputFilename,
                None,
            ),
            (
                "path and query parameter sanitize to one field",
                api_with(vec![{
                    let mut endpoint = endpoint("GetUser", RestMethod::Get, "/users/{user-id}");
                    endpoint.params = Some(params(vec![param("user_id", false)], vec![], vec![]));
                    endpoint
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
                api_with(vec![{
                    let mut endpoint = with_body("Create", "CreateBody");
                    endpoint.params = Some(params(vec![param("body", false)], vec![], vec![]));
                    endpoint
                }]),
                Feature::GeneratedNameCollision,
                Some("Create"),
            ),
            (
                "form fields sanitize to one field",
                api_with(vec![{
                    let mut endpoint = endpoint("Upload", RestMethod::Post, "/upload");
                    endpoint.request = Some(ApiRequest::form_data(vec![
                        FormField::text("file-name"),
                        FormField::text("file_name"),
                    ]));
                    endpoint
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
                {
                    let mut api = api_with(vec![with_body("CreateUser", "CreateUserParams")]);
                    api.request_suffix = Some("Params".to_string());
                    api
                },
                Feature::GeneratedNameCollision,
                Some("CreateUser"),
            ),
            (
                "response type equals another endpoint's form body",
                api_with(vec![
                    {
                        let mut endpoint = endpoint("Upload", RestMethod::Post, "/upload");
                        endpoint.request = Some(ApiRequest::form_data(vec![FormField::text("a")]));
                        endpoint
                    },
                    {
                        let mut endpoint = get("Status");
                        endpoint.response = ApiResponse::json_type("UploadForm");
                        endpoint
                    },
                ]),
                Feature::GeneratedNameCollision,
                Some("Status"),
            ),
            (
                "endpoint struct clashes with an import",
                {
                    let mut api = api_with(vec![get("Serial")]);
                    api.request_suffix = Some("ize".to_string());
                    api
                },
                Feature::GeneratedNameCollision,
                Some("Serial"),
            ),
            (
                "text request body with a control character in its content type",
                api_with(vec![{
                    let mut endpoint = endpoint("Note", RestMethod::Post, "/note");
                    endpoint.request = Some(ApiRequest::Text {
                        content_type: "text/plain\r\nX-Injected: 1".to_string(),
                    });
                    endpoint
                }]),
                Feature::TextRequestBody,
                Some("Note"),
            ),
            (
                "binary request body with an empty content type",
                api_with(vec![{
                    let mut endpoint = endpoint("Blob", RestMethod::Put, "/blob");
                    endpoint.request = Some(ApiRequest::Binary {
                        content_type: String::new(),
                    });
                    endpoint
                }]),
                Feature::BinaryRequestBody,
                Some("Blob"),
            ),
            (
                "API-key cookie whose name is not a cookie name",
                {
                    let mut api = make_test_api();
                    api.auth = AuthStrategy::ApiKeyParam {
                        name: "session id".to_string(),
                        location: ApiKeyLocation::Cookie,
                    };
                    api
                },
                Feature::ApiKeyParam,
                None,
            ),
            (
                "API-key query parameter with an empty name",
                {
                    let mut api = make_test_api();
                    api.auth = AuthStrategy::ApiKeyParam {
                        name: String::new(),
                        location: ApiKeyLocation::Query,
                    };
                    api
                },
                Feature::ApiKeyParam,
                None,
            ),
            (
                "header and query parameters sanitize to one field",
                api_with(vec![{
                    let mut endpoint = get("Items");
                    endpoint.params = Some(params(
                        vec![param("x_id", false)],
                        vec![param("X-Id", false)],
                        vec![],
                    ));
                    endpoint
                }]),
                Feature::GeneratedNameCollision,
                Some("Items"),
            ),
            (
                "cookie parameter named like the raw body field",
                api_with(vec![{
                    let mut endpoint = endpoint("Note", RestMethod::Post, "/note");
                    endpoint.request = Some(ApiRequest::Text {
                        content_type: "text/plain".to_string(),
                    });
                    endpoint.params = Some(params(vec![], vec![], vec![param("body", false)]));
                    endpoint
                }]),
                Feature::GeneratedNameCollision,
                Some("Note"),
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

        let explicit = {
            let mut api = make_test_api();
            api.auth_policy = Some(AuthPolicy {
                explicit: vec![AuthMethod::ApiKeyParam {
                    name: "bad name".to_string(),
                    location: ApiKeyLocation::Cookie,
                }],
                env_fallback: None,
            });
            api
        };
        let fallback = {
            let mut api = make_test_api();
            api.auth_policy = Some(AuthPolicy {
                explicit: vec![],
                env_fallback: Some(EnvAuthStrategy::ApiKeyParam {
                    name: "bad name".to_string(),
                    location: ApiKeyLocation::Cookie,
                }),
            });
            api
        };
        // A policy override that drops the legacy strategy hides it.
        let overridden = {
            let mut api = make_test_api();
            api.auth = AuthStrategy::ApiKeyParam {
                name: "bad name".to_string(),
                location: ApiKeyLocation::Cookie,
            };
            api.auth_policy = Some(AuthPolicy::default());
            api
        };

        for api in [&explicit, &fallback] {
            assert_eq!(features(&rust(api)), vec![(Severity::Error, Feature::ApiKeyParam)]);
        }
        assert!(rust(&overridden).is_empty());
    }

    #[test]
    fn invalid_header_and_cookie_parameters_fail_only_the_rust_client() {
        let cases: Vec<(&str, RestApi, Feature, Option<&str>)> = vec![
            (
                "header parameter whose name is not a header name",
                api_with(vec![{
                    let mut endpoint = get("Items");
                    endpoint.params = Some(params(vec![], vec![param("X Trace", false)], vec![]));
                    endpoint
                }]),
                Feature::HeaderParameter,
                Some("Items"),
            ),
            (
                "cookie parameter whose name holds a separator",
                api_with(vec![{
                    let mut endpoint = get("Items");
                    endpoint.params = Some(params(vec![], vec![], vec![param("a;b", false)]));
                    endpoint
                }]),
                Feature::CookieParameter,
                Some("Items"),
            ),
            (
                "header parameter that is an array of arrays",
                api_with(vec![{
                    let mut endpoint = get("Items");
                    endpoint.params = Some(params(
                        vec![],
                        vec![ParamDef { rust_name: None, secret: false, pager_managed: false,
                            param_type: QueryParamType::Array(Box::new(QueryParamType::Array(
                                Box::new(QueryParamType::String),
                            ))),
                            ..param("X-Matrix", false)
                        }],
                        vec![],
                    ));
                    endpoint
                }]),
                Feature::HeaderParameter,
                Some("Items"),
            ),
        ];

        for (name, api, feature, endpoint) in cases {
            let rust = rust(&api);
            assert_eq!(features(&rust), vec![(Severity::Error, feature)], "{name}");
            assert_eq!(rust[0].endpoint.as_deref(), endpoint, "{name}");
            assert!(validate_openapi(&[&api]).is_empty(), "{name}");
            assert!(validate_postman(&[&api]).is_empty(), "{name}");
        }
    }

    #[test]
    fn request_inputs_the_client_sends_pass_rust_validation() {
        let array = |name: &str| ParamDef { rust_name: None, secret: false, pager_managed: false,
            param_type: QueryParamType::Array(Box::new(QueryParamType::Integer)),
            ..param(name, true)
        };
        let endpoints = vec![
            {
                let mut endpoint = endpoint("Note", RestMethod::Post, "/note");
                endpoint.request = Some(ApiRequest::Text {
                    content_type: "text/plain; charset=utf-8".to_string(),
                });
                endpoint
            },
            {
                let mut endpoint = endpoint("Blob", RestMethod::Put, "/blob");
                endpoint.request = Some(ApiRequest::Binary {
                    content_type: "application/octet-stream".to_string(),
                });
                endpoint
            },
            {
                let mut endpoint = get("Items");
                endpoint.params = Some(params(
                    vec![param("q", false)],
                    vec![param("X-Trace-Id", true), array("X-Ids"), param("openai-beta", false)],
                    vec![param("session", true), array("prefs"), param("__Host-id", false)],
                ));
                endpoint
            },
        ];
        for (location, name) in [(ApiKeyLocation::Query, "api key"), (ApiKeyLocation::Cookie, "sid")] {
            let api = {
                let mut api = api_with(endpoints.clone());
                api.auth = AuthStrategy::ApiKeyParam {
                    name: name.to_string(),
                    location,
                };
                api
            };
            assert!(rust(&api).is_empty(), "{location:?}: {:#?}", rust(&api));
        }
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
                vec![(Severity::Error, Feature::DuplicateMethodPath)]
            );
            assert_eq!(found[0].endpoint.as_deref(), Some("ByName"));
            assert!(found[0].message.contains("`ById`"), "{}", found[0]);
        }
        assert!(all_outputs(&api_with(distinct.to_vec())).is_empty());
    }

    #[test]
    fn same_route_in_different_apis_is_not_a_duplicate() {
        let route = || endpoint("ById", RestMethod::Get, "/items/{id}");
        let first = {
            let mut api = api_with(vec![route()]);
            api.name = "First".to_string();
            api.module_path = Some("first".to_string());
            api
        };
        let second = {
            let mut api = api_with(vec![route()]);
            api.name = "Second".to_string();
            api.module_path = Some("second".to_string());
            api
        };

        assert!(validate_rust_client(&[&first, &second]).is_empty());
        assert!(validate_openapi(&[&first, &second]).is_empty());
        assert!(validate_postman(&[&first, &second]).is_empty());
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
    fn query_in_path_severity_is_a_warning() {
        assert_eq!(QUERY_IN_PATH_SEVERITY, Severity::Warning);
    }

    #[test]
    fn export_file_names_must_be_one_safe_component() {
        let bad = ["", ".", "..", "a/b", "a\\b", "C:", "con|x", "x\0y"];
        for module_path in bad {
            let api = {
                let mut api = make_test_api();
                api.module_path = Some(module_path.to_string());
                api
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
            let api = {
                let mut api = make_test_api();
                api.module_path = Some(module_path.to_string());
                api
            };
            assert!(validate_openapi(&[&api]).is_empty(), "{module_path}");
            assert!(validate_postman(&[&api]).is_empty(), "{module_path}");
        }
    }

    // ---- module groups ----

    fn group_member(name: &str, suffix: Option<&str>, endpoints: Vec<Endpoint>) -> RestApi {
        {
            let mut api = make_test_api();
            api.name = name.to_string();
            api.module_path = Some("shared_group".to_string());
            api.request_suffix = suffix.map(str::to_string);
            api.endpoints = endpoints;
            api
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

        // The request struct and the call type each collide.
        let diagnostics = validate_rust_client(&[&first, &second]);
        assert_eq!(
            features(&diagnostics),
            vec![(Severity::Error, Feature::GeneratedNameCollision); 2]
        );
        for diagnostic in &diagnostics {
            assert_eq!(diagnostic.api, vec!["First", "Second"]);
            assert_eq!(diagnostic.endpoint, None);
        }
        assert!(diagnostics[0].message.starts_with("`ListModelsCall` is generated for the call type"));
        assert!(diagnostics[1].message.starts_with("`ListModelsRequest` is generated for the request struct"));

        // Separate modules keep separate namespaces.
        let elsewhere = {
            let mut api = second.clone();
            api.module_path = Some("other".to_string());
            api
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
