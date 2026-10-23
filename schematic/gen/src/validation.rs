//! Per-output validation of a batch of API definitions.
//!
//! Each generated output has its own validator, and each validator takes the
//! whole selected batch: [`validate_rust_client`], [`validate_openapi`], and
//! [`validate_postman`]. An output is written only when its validator reports
//! no [`Severity::Error`]; one output's findings never block another output.
//! Rust-only rules (identifiers, type syntax, generated-name collisions) are
//! not applied to the exports, which carry those names as data. The Rust
//! client and OpenAPI validators both resolve every JSON body reference in
//! the API's own [`RestApi::schemas`]; Postman never reads a schema.
//!
//! The behavior is described for users in
//! `schematic/docs/generation/validation.md`.
//!
//! ## Examples
//!
//! ```
//! use schematic_define::{DefinitionContext, Endpoint, RestApi, SuccessShape};
//! use schematic_gen::diagnostics::{Feature, Severity};
//! use schematic_gen::validation::{validate_postman, validate_rust_client};
//!
//! let ctx = DefinitionContext::types_in("fixture");
//! let endpoint = Endpoint::get("ListItems", "/items")
//!     .describes("List items").returns_empty().build().unwrap();
//! let mut api = RestApi::builder(&ctx, "TestApi", "https://api.test.com")
//!     .describes("Test API").no_auth().endpoints([endpoint]).build().unwrap();
//! // A reference to a component the API does not carry blocks the Rust
//! // client, not the Postman export.
//! api.endpoints[0].response = vec![SuccessShape::json_type("Item")];
//!
//! let rust = validate_rust_client(&[&api]);
//! assert_eq!(rust.len(), 1);
//! assert_eq!(rust[0].severity, Severity::Error);
//! assert_eq!(rust[0].feature, Feature::InvalidSchemaReference);
//! assert_eq!(rust[0].endpoint.as_deref(), Some("ListItems"));
//!
//! assert!(validate_postman(&[&api]).is_empty());
//! ```

use std::collections::{BTreeMap, BTreeSet, HashMap};

use schematic_define::openapi::import::naming::sanitize_rust_field_ident;
use schematic_define::{
    ApiKeyLocation, ApiRequest, AuthMethod, DefinitionError, Endpoint,
    EnvAuthStrategy, ErrorEntry, FormFieldKind, GraphQlOperation, GraphQlVariables, ParamDef,
    QueryParamType, ResponseShape, RestApi, StreamFraming, TypeRef,
};
use schematic_define::pagination::{
    PaginationRuleGroup, Resolution, Step, resolve, validate_pagination,
};
use serde_json::Value;

use crate::codegen::endpoint_methods::{call_type_name, resolve_names};
use crate::codegen::request_structs::param_enums::variant_collisions;
use schematic_definitions::registry::{ApiRegistration, registrations};

use crate::codegen::request_structs::path_params::{PathValue, typed_path_fields};
use crate::codegen::request_structs::shared::{extract_query_params, query_shape};
use crate::codegen::pagination::pager_view;
use crate::codegen::responses::{end_marker_names, failure_enum_needed, success_variant_names};
use crate::diagnostics::{Feature, Output, Severity, ValidationDiagnostic};
use crate::errors::GeneratorError;
use crate::output::assemble::definitions_module_path;
use crate::output::get_request_suffix;
use schematic_definitions::naming::resolve_module_name;
use crate::parser::extract_path_params;

/// Severity of the `query-in-path` check.
///
/// Query inputs must be declared separately from the path template.
pub const QUERY_IN_PATH_SEVERITY: Severity = Severity::Error;

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
/// file names, type syntax, declared path parameters, generated-name
/// collisions across each module group, endpoint-method and setter renames (warnings), and the request
/// inputs the client sends verbatim: header and cookie parameter names, raw
/// body content types, and API-key parameter names. A module path may not
/// take a shipped WebSocket module's name.
pub fn validate_rust_client(apis: &[&RestApi]) -> Vec<ValidationDiagnostic> {
    validate_rust_client_in(apis, registrations())
}

/// [`validate_rust_client`] for a crate that packages the WebSocket modules
/// of `registrations`.
pub fn validate_rust_client_in(
    apis: &[&RestApi],
    registrations: &[ApiRegistration],
) -> Vec<ValidationDiagnostic> {
    let mut diagnostics = Vec::new();
    for api in apis {
        check_routes(api, Output::Rust, &mut diagnostics);
        check_schema_references(api, Output::Rust, &mut diagnostics);
        check_rust_api(api, registrations, &mut diagnostics);
    }
    check_stem_case_collisions(apis, Output::Rust, &mut diagnostics);

    let mut groups: BTreeMap<String, Vec<&RestApi>> = BTreeMap::new();
    for api in apis {
        groups.entry(resolve_module_name(api)).or_default().push(api);
    }
    for group in groups.values() {
        check_group_type_names(group, &mut diagnostics);
    }
    diagnostics
}

/// Validates a batch of APIs for OpenAPI export.
///
/// Checks route identity, the `query-in-path` error, the document file
/// name, and that every JSON body reference resolves in the API's own
/// [`RestApi::schemas`], which become the document's `components.schemas`.
pub fn validate_openapi(apis: &[&RestApi]) -> Vec<ValidationDiagnostic> {
    let mut diagnostics = Vec::new();
    for api in apis {
        check_routes(api, Output::OpenApi, &mut diagnostics);
        check_export_filename(api, Output::OpenApi, &mut diagnostics);
        check_schema_references(api, Output::OpenApi, &mut diagnostics);
        diagnostics.extend(crate::export::vocabulary::diagnostics(api, Output::OpenApi));
    }
    check_stem_case_collisions(apis, Output::OpenApi, &mut diagnostics);
    diagnostics
}

/// Validates a batch of APIs for Postman export.
///
/// Checks route identity, the `query-in-path` error, and the collection
/// file name.
pub fn validate_postman(apis: &[&RestApi]) -> Vec<ValidationDiagnostic> {
    let mut diagnostics = Vec::new();
    for api in apis {
        check_routes(api, Output::Postman, &mut diagnostics);
        check_export_filename(api, Output::Postman, &mut diagnostics);
        diagnostics.extend(crate::export::vocabulary::diagnostics(api, Output::Postman));
    }
    check_stem_case_collisions(apis, Output::Postman, &mut diagnostics);
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

/// Checks that every JSON body reference of `api` resolves in
/// [`RestApi::schemas`], reporting each dangling name with its endpoint.
///
/// When every name resolves, [`RestApi::validate_references`] and
/// [`RestApi::validate_schema_closure`] still run, so a malformed component,
/// an unsupported reference form, or a `$ref` inside a component that no
/// component satisfies is reported too.
fn check_schema_references(api: &RestApi, output: Output, diagnostics: &mut Vec<ValidationDiagnostic>) {
    let before = diagnostics.len();
    for endpoint in &api.endpoints {
        for reference in endpoint.json_references() {
            for name in reference.names() {
                if api.component(name).is_none_or(|component| component.rust_path.is_none()) {
                    diagnostics.push(ValidationDiagnostic::new(
                        Severity::Error,
                        &api.name,
                        Some(&endpoint.id),
                        Feature::InvalidSchemaReference,
                        output,
                        format!(
                            "schema reference `{name}` does not name a registered component of \
                             API `{}`; register the type in the API's definition context",
                            api.name
                        ),
                    ));
                }
            }
        }
    }
    if diagnostics.len() > before {
        return;
    }
    if let Err(error) = api.validate_references().and_then(|()| api.validate_schema_closure()) {
        let endpoint = match &error {
            DefinitionError::UnresolvedReference { endpoint, .. }
            | DefinitionError::InvalidDefinition { endpoint, .. }
                if endpoint != "<api>" => Some(endpoint.as_str()),
            _ => None,
        };
        diagnostics.push(ValidationDiagnostic::new(
            Severity::Error,
            &api.name,
            endpoint,
            Feature::InvalidSchemaReference,
            output,
            error.to_string(),
        ));
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

/// Checks that no two module names of the batch differ only by letter case.
///
/// Case-insensitive file systems (the macOS and Windows defaults) would
/// write both modules to one file.
fn check_stem_case_collisions(apis: &[&RestApi], output: Output, diagnostics: &mut Vec<ValidationDiagnostic>) {
    let mut seen: HashMap<String, String> = HashMap::new();
    for api in apis {
        let stem = resolve_module_name(api);
        let first = seen.entry(stem.to_lowercase()).or_insert_with(|| stem.clone());
        if *first != stem {
            diagnostics.push(ValidationDiagnostic::new(
                Severity::Error,
                &api.name,
                None,
                Feature::InvalidOutputFilename,
                output,
                format!("module name `{stem}` differs from `{first}` only by letter case, so both \
                         would name one file on a case-insensitive file system"),
            ));
        }
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
fn check_rust_api(api: &RestApi, registrations: &[ApiRegistration], diagnostics: &mut Vec<ValidationDiagnostic>) {
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

    check_module_path(api, registrations, diagnostics);
    check_api_key_param(api, diagnostics);
    check_endpoint_method_names(api, diagnostics);

    for operation in api.graphql.iter().flat_map(|surface| &surface.operations) {
        if let Err(reason) = check_type_ident(&operation.id) {
            diagnostics.push(error(
                Some(&operation.id),
                Feature::InvalidIdentifier,
                format!("GraphQL operation id `{}` {reason}", operation.id),
            ));
        }
    }

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

        for (role, type_name) in referenced_type_names(api, endpoint) {
            if syn::parse_str::<syn::Type>(&type_name).is_err() {
                diagnostics.push(error(
                    id,
                    Feature::InvalidTypeSyntax,
                    format!("{role} type `{type_name}` is not valid Rust type syntax"),
                ));
            }
        }

        check_typed_path_params(api, endpoint, diagnostics);
        check_request_struct_fields(api, endpoint, diagnostics);
        check_raw_body_content_type(api, endpoint, diagnostics);
        check_sent_params(api, endpoint, diagnostics);
        check_query_shapes(api, endpoint, diagnostics);
        check_success_variants(api, endpoint, diagnostics);
    }

    check_pagination(api, diagnostics);
}

/// Checks the API's pagination: the define-side rules, then the item types
/// and item locations the Rust client relies on.
///
/// Item checks run only for paginated lists. A named item type is checked
/// against the API's [`RestApi::schemas`], top-level and nested components
/// alike, and an item location is walked through the schema of the
/// response's named component. Where the components cannot describe a type
/// (a qualified or primitive item type, a response that is not a single
/// name, an unresolvable `$ref`, a union), the check is skipped and the
/// runtime enforces the location instead; nothing is reported as checked.
fn check_pagination(api: &RestApi, diagnostics: &mut Vec<ValidationDiagnostic>) {
    if api.default_pagination.is_none() && api.endpoints.iter().all(|endpoint| endpoint.pagination.is_none()) {
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

    for endpoint in &api.endpoints {
        let Resolution::Paginated { list, .. } = resolve(api, endpoint) else {
            continue;
        };
        let id = Some(endpoint.id.as_str());
        let authored_type = endpoint.pagination.as_ref().and_then(|collection| collection.item_type.as_deref());

        if let Some(item_type) = authored_type.filter(|ty| !ty.trim().is_empty()) {
            if syn::parse_str::<syn::Type>(item_type).is_err() {
                diagnostics.push(error(
                    id,
                    Feature::PaginationException,
                    format!("invalid-item-type: item type `{item_type}` is not valid Rust type syntax"),
                ));
            } else if is_plain_named_type(item_type) && api.component(item_type).is_none() {
                diagnostics.push(error(
                    id,
                    Feature::PaginationException,
                    format!(
                        "unknown-item-type: item type `{item_type}` is not a schema component of \
                         this API; use a registered type, a primitive, or a qualified path such \
                         as `serde_json::Value`"
                    ),
                ));
            }
        }

        let [only] = endpoint.response.as_slice() else {
            continue;
        };
        let ResponseShape::Json(TypeRef::Named(name)) = &only.shape else {
            continue;
        };
        if list.items.is_root_array() {
            continue;
        }
        let Some(root) = api.component(name) else {
            continue;
        };
        if item_location_is_array(api, &root.schema, &list.items.record) == Some(false) {
            diagnostics.push(error(
                id,
                Feature::PaginationException,
                format!(
                    "not-an-array-location: `{}` is not an array in the registered schema of `{name}`",
                    list.items
                ),
            ));
        }
    }
}

/// Whether `type_name` is a single identifier naming a non-primitive type,
/// the only kind a schema component can name.
fn is_plain_named_type(type_name: &str) -> bool {
    const PRIMITIVES: &[&str] = &[
        "bool", "char", "str", "String", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16",
        "u32", "u64", "u128", "usize", "f32", "f64",
    ];
    syn::parse_str::<syn::Ident>(type_name).is_ok() && !PRIMITIVES.contains(&type_name)
}

/// Walks a selector's record steps through a component's JSON schema.
///
/// `Some(false)` means the schema demonstrably has no array at some fan-out;
/// `Some(true)` that every fan-out is an array; `None` that the schema cannot
/// tell (a missing property, an unresolved reference, a union).
fn item_location_is_array(api: &RestApi, root: &Value, steps: &[Step]) -> Option<bool> {
    let mut node = root;
    for step in steps {
        node = resolve_schema_node(api, node)?;
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
///
/// Components are stored with their nested definitions flattened out, so a
/// `$ref` resolves to another component by name, never into the referring
/// schema.
fn resolve_schema_node<'a>(api: &'a RestApi, mut node: &'a Value) -> Option<&'a Value> {
    // A bound against reference cycles; real schemas need a few hops.
    for _ in 0..32 {
        if let Some(reference) = node.get("$ref").and_then(Value::as_str) {
            let name = ["#/components/schemas/", "#/$defs/", "#/definitions/"]
                .iter()
                .find_map(|prefix| reference.strip_prefix(prefix))
                .filter(|name| !name.contains('/'))?;
            node = &api.component(name)?.schema;
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
/// `schematic_definitions::<a>::<b>` import path, and names the module's
/// Cargo feature, so it must be one plain identifier that names no other
/// generated file and is not `default`.
fn check_module_path(api: &RestApi, registrations: &[ApiRegistration], diagnostics: &mut Vec<ValidationDiagnostic>) {
    let module_path = resolve_module_name(api);
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

    if module_path == "default" {
        push(
            diagnostics,
            Feature::InvalidModulePath,
            "module path `default` would name the generated crate's `default` Cargo feature, \
             which enables every module"
                .to_string(),
        );
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

    if RESERVED_MODULE_NAMES.iter().any(|reserved| reserved.eq_ignore_ascii_case(&module_path))
        || crate::output::ws_modules::ws_registrations(registrations)
            .any(|(entry, _)| entry.module.eq_ignore_ascii_case(&module_path))
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
            // A query parameter's shape, nested arrays included, is
            // checked by `check_query_shapes`.
            if nested && feature != Feature::QueryParameter {
                push(format!(
                    "{location} parameter `{}` is an array of arrays, which has no {location} \
                     serialization",
                    param.name
                ));
            }
        }
    }
}

/// Checks the query parameters of the endpoint's request type, which leaves
/// out the parameters its pagination strategy adopts.
///
/// A parameter whose type, style, and `explode` have no Rust serialization
/// is left out of the request type: a warning when it is optional, an error
/// when it is required, since no request could then be correct. An enum must
/// have values, and its values must generate distinct variants.
fn check_query_shapes(api: &RestApi, endpoint: &Endpoint, diagnostics: &mut Vec<ValidationDiagnostic>) {
    let viewed = pager_view(api, endpoint);
    let Some(params) = &viewed.params else {
        return;
    };
    let mut push = |severity, feature, message: String| {
        diagnostics.push(ValidationDiagnostic::new(
            severity,
            &api.name,
            Some(&endpoint.id),
            feature,
            Output::Rust,
            message,
        ));
    };
    for param in &params.query {
        let name = &param.name;
        match query_shape(param, "") {
            Err(declared) if param.required => push(
                Severity::Error,
                Feature::QueryParameter,
                format!(
                    "required query parameter `{name}` is {declared}, which the Rust client cannot \
                     serialize"
                ),
            ),
            Err(declared) => push(
                Severity::Warning,
                Feature::QueryParameter,
                format!(
                    "query parameter `{name}` is {declared}, which the Rust client cannot serialize; \
                     it is left out of the request type"
                ),
            ),
            Ok(shape) => match shape.enum_type() {
                Some((_, [])) => push(
                    Severity::Error,
                    Feature::QueryParameter,
                    format!("query parameter `{name}` declares an enum with no values"),
                ),
                Some((_, values)) => {
                    for (variant, wire_values) in variant_collisions(values) {
                        push(
                            Severity::Error,
                            Feature::GeneratedNameCollision,
                            format!(
                                "query parameter `{name}` enum values `{}` all generate the variant `{variant}`",
                                wire_values.join("`, `")
                            ),
                        );
                    }
                }
                None => {}
            },
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

/// Lists the Rust types an endpoint asks the Rust client to spell, with the
/// role each plays. A reference that does not resolve is left to
/// [`check_schema_references`].
fn referenced_type_names(api: &RestApi, endpoint: &Endpoint) -> Vec<(&'static str, String)> {
    let errors = api.effective_errors(endpoint);
    let mut references: Vec<(&'static str, &TypeRef)> = Vec::new();
    match &endpoint.request {
        Some(ApiRequest::Json(reference)) => references.push(("request body", reference)),
        Some(ApiRequest::FormData { fields }) => {
            for field in fields {
                if let FormFieldKind::Json(reference) = &field.kind {
                    references.push(("form field", reference));
                }
            }
        }
        _ => {}
    }
    for shape in endpoint.success_shapes() {
        match &shape.shape {
            ResponseShape::Json(reference) => references.push(("response", reference)),
            ResponseShape::Stream { framing: StreamFraming::Sse(reference) | StreamFraming::Ndjson(reference), .. } => {
                references.push(("stream item", reference));
            }
            _ => {}
        }
    }
    references.extend(errors.entries().iter().filter_map(error_reference));
    references
        .into_iter()
        .filter_map(|(role, reference)| Some((role, api.rust_type(reference)?)))
        .collect()
}

/// The body types a GraphQL operation's generated code names.
fn operation_referenced_type_names(api: &RestApi, operation: &GraphQlOperation) -> Vec<(&'static str, String)> {
    let errors = api.effective_operation_errors(operation);
    let mut references: Vec<(&'static str, &TypeRef)> = vec![("data", &operation.data)];
    if let GraphQlVariables::Typed(reference) = &operation.variables {
        references.push(("variables", reference));
    }
    references.extend(errors.entries().iter().filter_map(error_reference));
    references
        .into_iter()
        .filter_map(|(role, reference)| Some((role, api.rust_type(reference)?)))
        .collect()
}

fn error_reference(entry: &ErrorEntry) -> Option<(&'static str, &TypeRef)> {
    match &entry.shape {
        ResponseShape::Json(reference) => Some(("error", reference)),
        _ => None,
    }
}

/// The items generated for `endpoint`'s response types (Defaults D-15,
/// D-23): `{id}Response` when it declares several success shapes, a stream
/// end marker per stream with a terminator, and `{id}Error` when its
/// effective error table needs the failure enum.
fn endpoint_enum_names(api: &RestApi, endpoint: &Endpoint) -> Vec<(String, &'static str)> {
    let shapes = endpoint.success_shapes();
    let mut names = Vec::new();
    if shapes.len() > 1 {
        names.push((format!("{}Response", endpoint.id), "success enum"));
    }
    names.extend(end_marker_names(&endpoint.id, shapes).into_iter().map(|name| (name, "stream end marker")));
    if failure_enum_needed(api, &api.effective_errors(endpoint)) {
        names.push((format!("{}Error", endpoint.id), "failure enum"));
    }
    names
}

/// Reports success-enum variants that the derived names (D-15) leave
/// colliding, which only the author can resolve.
fn check_success_variants(api: &RestApi, endpoint: &Endpoint, diagnostics: &mut Vec<ValidationDiagnostic>) {
    let shapes = endpoint.success_shapes();
    if shapes.len() < 2 {
        return;
    }
    let names = success_variant_names(shapes);
    for (index, name) in names.iter().enumerate() {
        if names[..index].contains(name) || names[index + 1..].iter().all(|other| other != name) {
            continue;
        }
        let colliding: Vec<String> = names.iter().enumerate().filter(|(_, other)| *other == name).map(|(position, _)| {
            let shape = &shapes[position];
            match shape.media_type() {
                Some(media) => format!("success shape {} ({media})", position + 1),
                None => format!("success shape {}", position + 1),
            }
        }).collect();
        diagnostics.push(ValidationDiagnostic::new(
            Severity::Error,
            &api.name,
            Some(&endpoint.id),
            Feature::GeneratedNameCollision,
            Output::Rust,
            format!(
                "API {} endpoint {}: {} all generate the variant `{name}` of `{}Response`; name them with `Success::variant`",
                api.name, endpoint.id, colliding.join(", "), endpoint.id
            ),
        ));
    }
}

/// Checks an endpoint's declared path parameters against what the Rust
/// emitters can generate.
///
/// Definition build already rejects these; they are checked again because
/// every field of a definition stays public. A declaration without a
/// placeholder or a placeholder without a declaration, an array or JSON
/// type, an empty enum, and an invalid `rust_name` are
/// [`Feature::PathParameter`] errors. Two enum values whose variant
/// identifiers coincide (`a-b` and `a_b`) are a
/// [`Feature::GeneratedNameCollision`]: the emitter never renames a variant.
fn check_typed_path_params(api: &RestApi, endpoint: &Endpoint, diagnostics: &mut Vec<ValidationDiagnostic>) {
    let Some(params) = endpoint.params.as_ref().filter(|params| !params.path.is_empty()) else {
        return;
    };
    let mut push = |feature, message: String| {
        diagnostics.push(ValidationDiagnostic::new(
            Severity::Error,
            &api.name,
            Some(&endpoint.id),
            feature,
            Output::Rust,
            message,
        ));
    };
    let placeholders = endpoint.path_placeholders();
    for placeholder in &placeholders {
        if !params.path.iter().any(|param| param.name == *placeholder) {
            push(
                Feature::PathParameter,
                format!("path placeholder `{{{placeholder}}}` has no declared path parameter"),
            );
        }
    }
    for param in &params.path {
        let name = &param.name;
        if !placeholders.contains(&name.as_str()) {
            push(
                Feature::PathParameter,
                format!("path parameter `{name}` has no `{{{name}}}` placeholder in `{}`", endpoint.path),
            );
        }
        if let Some(rust_name) = &param.rust_name
            && !is_field_ident(rust_name)
        {
            push(Feature::PathParameter, format!("path parameter `{name}` has invalid Rust name `{rust_name}`"));
        }
        match &param.param_type {
            QueryParamType::Array(_) | QueryParamType::Json => push(
                Feature::PathParameter,
                format!("path parameter `{name}` must be a scalar or enum, not an array or JSON value"),
            ),
            QueryParamType::Enum(values) if values.is_empty() => {
                push(Feature::PathParameter, format!("path parameter `{name}` declares an enum with no values"));
            }
            QueryParamType::Enum(values) => {
                for (variant, wire_values) in variant_collisions(values) {
                    push(
                        Feature::GeneratedNameCollision,
                        format!(
                            "path parameter `{name}` enum values `{}` all generate the variant `{variant}`",
                            wire_values.join("`, `")
                        ),
                    );
                }
            }
            _ => {}
        }
    }
}

/// Whether `name` can be a generated struct field and constructor argument.
fn is_field_ident(name: &str) -> bool {
    syn::parse_str::<syn::Ident>(name).is_ok() && !matches!(name, "self" | "Self" | "crate" | "super" | "_")
}

/// Checks the fields of an endpoint's request struct and form body struct.
///
/// Field names preserve explicit Rust names or use [`sanitize_rust_field_ident`],
/// exactly as the emitters derive them, so distinct wire names can still collide
/// (`{user-id}` and `user_id`). Typed path parameters count once per distinct
/// placeholder, under their `rust_name` when they declare one. Query, header,
/// and cookie builders are all `with_<field>`, so a builder collision is a
/// field collision.
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
    let declared_paths = endpoint.params.as_ref().map(|params| params.path.as_slice()).unwrap_or_default();
    if declared_paths.is_empty() {
        for name in extract_path_params(&endpoint.path) {
            fields.push((format!("path parameter `{name}`"), name.to_string(), false));
        }
    } else {
        for name in endpoint.path_placeholders() {
            let rust_name = declared_paths
                .iter()
                .find(|param| param.name == name)
                .and_then(|param| param.rust_name.clone());
            let explicit = rust_name.is_some();
            fields.push((format!("path parameter `{name}`"), rust_name.unwrap_or_else(|| name.to_string()), explicit));
        }
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
/// the emitted names apart. The enums of enum-typed path parameters are
/// emitted names too. A referenced body or response type that equals an
/// emitted name is a collision as well: the local item would shadow the
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
            for path in typed_path_fields(endpoint, &format!("{}{suffix}", endpoint.id)).unwrap_or_default() {
                if let PathValue::Enum { type_name, .. } = path.value {
                    emitted.entry(type_name).or_default().push(Owner {
                        api: &api.name,
                        endpoint: Some(&endpoint.id),
                        role: format!("the `{}` path-parameter enum of {}::{}", path.wire, api.name, endpoint.id),
                    });
                }
            }
            let viewed = pager_view(api, endpoint);
            for param in extract_query_params(&viewed.params, &format!("{}{suffix}", endpoint.id)) {
                if let Some((type_name, _)) = param.shape.as_ref().and_then(|shape| shape.enum_type()) {
                    emitted.entry(type_name.to_string()).or_default().push(Owner {
                        api: &api.name,
                        endpoint: Some(&endpoint.id),
                        role: format!("the `{}` query-parameter enum of {}::{}", param.name, api.name, endpoint.id),
                    });
                }
            }
            for (name, role) in endpoint_enum_names(api, endpoint) {
                emitted.entry(name).or_default().push(Owner {
                    api: &api.name,
                    endpoint: Some(&endpoint.id),
                    role: format!("the {role} of {}::{}", api.name, endpoint.id),
                });
            }
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
        // Operations share the endpoints' naming space (R35).
        for operation in api.graphql.iter().flat_map(|surface| &surface.operations) {
            let request = format!("{}{suffix}", operation.id);
            let failure = failure_enum_needed(api, &api.effective_operation_errors(operation))
                .then(|| (format!("{}Error", operation.id), "failure enum"));
            for (name, role) in [(call_type_name(&request), "call type"), (request, "request struct")].into_iter().chain(failure) {
                emitted.entry(name).or_default().push(Owner {
                    api: &api.name,
                    endpoint: Some(&operation.id),
                    role: format!("the {role} of GraphQL operation {}::{}", api.name, operation.id),
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
        let endpoints = api.endpoints.iter().map(|endpoint| (endpoint.id.as_str(), referenced_type_names(api, endpoint)));
        let operations = api.graphql.iter().flat_map(|surface| &surface.operations)
            .map(|operation| (operation.id.as_str(), operation_referenced_type_names(api, operation)));
        for (id, references) in endpoints.chain(operations) {
            for (role, type_name) in references {
                let Some(owners) = emitted.get(type_name.as_str()) else {
                    continue;
                };
                if owners.is_empty() {
                    continue;
                }
                let own_wrapper = format!("{id}{suffix}");
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
                    endpoint: Some(id),
                    role: String::new(),
                };
                let mut involved: Vec<&Owner<'_>> = owners.iter().collect();
                involved.push(&referencing);
                diagnostics.push(diagnostic(
                    &involved,
                    format!(
                        "{role} type `{type_name}` of {}::{id} is shadowed by {}{hint}",
                        api.name,
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
    use schematic_define::{ApiKeyLocation, AuthStrategy, FormField, RestMethod, SuccessShape};

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
            let mut endpoint = Endpoint::new(id.to_string(), method, "/fixture")
                .describes(format!("Test endpoint {id}"))
                .returns_empty()
                .build().expect("valid endpoint fixture");
            endpoint.path = path.to_owned();
            endpoint.response = vec![SuccessShape::json_type("Response")];
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
        ParamDef { deprecated: false, rust_name: None, secret: false, pager_managed: false,
            name: name.to_string(),
            required,
            description: None,
            param_type: QueryParamType::String,
            explode: false,
            style: ParamStyle::Form,
        }
    }

    fn params(query: Vec<ParamDef>, header: Vec<ParamDef>, cookie: Vec<ParamDef>) -> EndpointParams {
        EndpointParams { path: vec![],
            query,
            header,
            cookie,

        }
    }

    /// An API with `endpoints` and a component, spelled by its bare name,
    /// for every name they reference.
    fn api_with(endpoints: Vec<Endpoint>) -> RestApi {
        let mut api = make_test_api();
        api.endpoints = endpoints;
        crate::test_utils::with_referenced_components(api)
    }

    /// [`api_with`], with the component `name` spelled `rust_path`.
    fn api_spelling(endpoints: Vec<Endpoint>, name: &str, rust_path: &str) -> RestApi {
        let mut api = api_with(endpoints);
        api.schemas.iter_mut().find(|c| c.name == name).expect("referenced component").rust_path =
            Some(rust_path.to_string());
        api
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
                "wrapped, primitive, and qualified types",
                api_spelling(
                    vec![
                        {
                            let mut endpoint = get("ListModels");
                            endpoint.response = vec![SuccessShape::json_vec_type("Model")];
                            endpoint
                        },
                        {
                            let mut endpoint = get("Raw");
                            endpoint.response = vec![SuccessShape::json(TypeRef::Primitive(schematic_define::PrimitiveType::Json))];
                            endpoint
                        },
                        {
                            let mut endpoint = get("Map");
                            endpoint.response = vec![SuccessShape::json(TypeRef::Map(Box::new(TypeRef::Array(Box::new(
                                TypeRef::Named("Model".into()),
                            )))))];
                            endpoint
                        },
                        {
                            let mut endpoint = with_body("Patch", "Patch");
                            endpoint.request = Some(ApiRequest::json(TypeRef::Optional(Box::new(TypeRef::Named("Patch".into())))));
                            endpoint
                        },
                    ],
                    "Patch",
                    "crate::Patch",
                ),
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
                        endpoint.response = vec![SuccessShape::text()];
                        endpoint
                    },
                    {
                        let mut endpoint = get("GetArchive");
                        endpoint.response = vec![SuccessShape::binary()];
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
                "unclosed generic component path",
                api_spelling(
                    vec![{
                        let mut endpoint = get("ListModels");
                        endpoint.response = vec![SuccessShape::json_type("Model")];
                        endpoint
                    }],
                    "Model",
                    "Vec<Model",
                ),
                Feature::InvalidTypeSyntax,
                Some("ListModels"),
            ),
            (
                "body component path with a space",
                api_spelling(vec![with_body("Create", "CreateBody")], "CreateBody", "Create Body"),
                Feature::InvalidTypeSyntax,
                Some("Create"),
            ),
            (
                "form JSON field component path",
                api_spelling(
                    vec![{
                        let mut endpoint = endpoint("Upload", RestMethod::Post, "/upload");
                        endpoint.request = Some(ApiRequest::form_data(vec![FormField::json("meta", TypeRef::Named("Meta".into()))]));
                        endpoint
                    }],
                    "Meta",
                    "Meta>",
                ),
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
                "module path naming the default feature",
                {
                    let mut api = make_test_api();
                    api.module_path = Some("default".to_string());
                    api
                },
                Feature::InvalidModulePath,
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
                        endpoint.response = vec![SuccessShape::json_type("UploadForm")];
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
    fn query_in_path_is_one_error_per_output() {
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
    fn query_in_path_severity_is_an_error() {
        assert_eq!(QUERY_IN_PATH_SEVERITY, Severity::Error);
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
            crate::test_utils::with_referenced_components(api)
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
    fn a_dangling_reference_names_the_api_endpoint_and_name_for_rust_and_openapi() {
        let mut api = api_with(vec![get("ListModels")]);
        api.schemas.clear();

        for diagnostics in [rust(&api), validate_openapi(&[&api])] {
            assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
            let diagnostic = &diagnostics[0];
            assert_eq!((diagnostic.severity, diagnostic.feature), (Severity::Error, Feature::InvalidSchemaReference));
            assert_eq!(diagnostic.api, ["TestApi"]);
            assert_eq!(diagnostic.endpoint.as_deref(), Some("ListModels"));
            assert!(diagnostic.message.contains("`Response`"), "{diagnostic}");
        }
        assert!(validate_postman(&[&api]).is_empty());
    }

    #[test]
    fn a_nested_only_component_cannot_be_referenced() {
        let mut api = api_with(vec![get("ListModels")]);
        api.schemas[0].rust_path = None;

        let diagnostics = rust(&api);
        assert_eq!(features(&diagnostics), [(Severity::Error, Feature::InvalidSchemaReference)]);
        assert!(diagnostics[0].message.contains("`Response`"), "{}", diagnostics[0]);
    }

    #[test]
    fn a_dangling_reference_inside_a_component_blocks_rust_and_openapi_with_its_location() {
        let mut api = api_with(vec![get("ListModels")]);
        api.schemas[0].schema = serde_json::json!({"type": "object", "properties": {"owner": {"$ref": "#/components/schemas/Gone"}}});

        for diagnostics in [rust(&api), validate_openapi(&[&api])] {
            assert_eq!(features(&diagnostics), [(Severity::Error, Feature::InvalidSchemaReference)]);
            assert_eq!(diagnostics[0].endpoint, None);
            assert_eq!(
                diagnostics[0].message,
                format!("TestApi schema {} at /properties/owner: unresolved schema reference \"Gone\"", api.schemas[0].name)
            );
        }
    }

    #[test]
    fn a_malformed_component_is_reported_for_the_api() {
        let mut api = api_with(vec![get("ListModels")]);
        let duplicate = api.schemas[0].clone();
        api.schemas.push(duplicate);

        let diagnostics = validate_openapi(&[&api]);
        assert_eq!(features(&diagnostics), [(Severity::Error, Feature::InvalidSchemaReference)]);
        assert_eq!(diagnostics[0].endpoint, None);
        assert!(diagnostics[0].message.contains("defined more than once"), "{}", diagnostics[0]);
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

    // ---- typed path parameters ----

    fn typed_path(id: &str, path: &str, declared: Vec<ParamDef>, query: Vec<ParamDef>) -> Endpoint {
        let mut endpoint = endpoint(id, RestMethod::Get, path);
        endpoint.params = Some(EndpointParams { path: declared, query, header: vec![], cookie: vec![] });
        endpoint
    }

    /// Only Rust-output diagnostics, and none of them reaches another output.
    fn rust_only(apis: &[&RestApi]) -> Vec<ValidationDiagnostic> {
        let rust = validate_rust_client(apis);
        for other in [validate_openapi(apis), validate_postman(apis)] {
            assert!(
                other.iter().all(|d| d.feature != Feature::GeneratedNameCollision && d.output != Output::Rust),
                "a Rust-only rule reached another output: {other:?}"
            );
        }
        rust
    }

    #[test]
    fn path_enum_values_with_one_variant_identifier_are_a_rust_collision() {
        let kind = ParamDef::path("kind", QueryParamType::Enum(vec!["a-b".into(), "c".into(), "a_b".into()]));
        let api = api_with(vec![typed_path("GetItem", "/items/{kind}", vec![kind], vec![])]);

        let diagnostics = rust_only(&[&api]);

        assert_eq!(features(&diagnostics), [(Severity::Error, Feature::GeneratedNameCollision)]);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.api, ["TestApi"]);
        assert_eq!(diagnostic.endpoint.as_deref(), Some("GetItem"));
        assert_eq!(diagnostic.output, Output::Rust);
        assert_eq!(
            diagnostic.message,
            "path parameter `kind` enum values `a-b`, `a_b` all generate the variant `AB`"
        );
    }

    #[test]
    fn a_path_enum_type_colliding_with_a_grouped_request_struct_names_both_apis() {
        let kind = ParamDef::path("kind", QueryParamType::Enum(vec!["a".into()]));
        let first = group_member("First", None, vec![typed_path("Get", "/{kind}", vec![kind], vec![])]);
        let second = group_member("Second", Some("Kind"), vec![get("GetRequest")]);

        let diagnostics = rust_only(&[&first, &second]);

        assert_eq!(features(&diagnostics), [(Severity::Error, Feature::GeneratedNameCollision)]);
        assert_eq!(diagnostics[0].api, ["First", "Second"]);
        assert_eq!(diagnostics[0].output, Output::Rust);
        assert_eq!(
            diagnostics[0].message,
            "`GetRequestKind` is generated for the `kind` path-parameter enum of First::Get and the \
             request struct of Second::GetRequest"
        );
    }

    fn operation(id: &str, data: &str) -> schematic_define::GraphQlOperation {
        let mut ctx = schematic_define::DefinitionContext::types_in("::fixture");
        let mut operation = schematic_define::GraphQlOperation::query(id, "{ a }").describes("An operation").no_variables()
            .data::<serde_json::Value>(&mut ctx).build().unwrap();
        operation.data = TypeRef::Named(data.into());
        operation
    }

    fn with_operations(endpoints: Vec<Endpoint>, operations: Vec<schematic_define::GraphQlOperation>) -> RestApi {
        let mut api = make_test_api();
        api.endpoints = endpoints;
        let mut surface = schematic_define::GraphQlSurface::new(schematic_define::AddressRule::github());
        surface.operations = operations;
        api.graphql = Some(surface);
        crate::test_utils::with_referenced_components(api)
    }

    #[test]
    fn an_operation_shares_the_endpoint_naming_space() {
        let api = with_operations(vec![get("Viewer")], vec![operation("Viewer", "ViewerData")]);
        let messages: Vec<String> = rust_only(&[&api]).into_iter().map(|d| d.message).collect();
        assert!(messages.contains(&"`ViewerRequest` is generated for the request struct of TestApi::Viewer and the request struct of GraphQL operation TestApi::Viewer".to_string()), "{messages:?}");
        assert!(messages.contains(&"`ViewerCall` is generated for the call type of TestApi::Viewer and the call type of GraphQL operation TestApi::Viewer".to_string()), "{messages:?}");

        let api = with_operations(vec![get("List")], vec![operation("Viewer", "ListRequest")]);
        let diagnostics = rust_only(&[&api]);
        assert_eq!(features(&diagnostics), [(Severity::Error, Feature::GeneratedNameCollision)]);
        assert_eq!(diagnostics[0].message, "data type `ListRequest` of TestApi::Viewer is shadowed by the request struct of TestApi::List");

        let api = with_operations(vec![get("List")], vec![operation("Viewer", "ViewerData")]);
        assert!(rust_only(&[&api]).is_empty());
        let api = with_operations(vec![], vec![operation("viewer-data", "ViewerData")]);
        assert!(rust(&api).iter().any(|d| d.feature == Feature::InvalidIdentifier && d.message.starts_with("GraphQL operation id `viewer-data`")));

        // Several distinct HTTP error shapes give the operation a failure
        // enum, which joins the same naming space.
        use schematic_define::{ErrorEntry, ResponseShape, StatusMatch};
        let mut viewer = operation("Viewer", "ViewerError");
        viewer.errors = vec![ErrorEntry::new(StatusMatch::exact(403), ResponseShape::Text), ErrorEntry::new(StatusMatch::class(5), ResponseShape::Binary)];
        let api = with_operations(vec![], vec![viewer.clone()]);
        let messages: Vec<String> = rust_only(&[&api]).into_iter().map(|d| d.message).collect();
        assert_eq!(messages, ["data type `ViewerError` of TestApi::Viewer is shadowed by the failure enum of GraphQL operation TestApi::Viewer"]);
        viewer.errors.truncate(1);
        let api = with_operations(vec![], vec![viewer]);
        assert!(rust_only(&[&api]).is_empty(), "one distinct shape needs no failure enum");
    }

    #[test]
    fn a_failure_enum_exists_for_several_distinct_error_shapes_and_may_shadow_a_type() {
        use schematic_define::{ErrorEntry, ResponseShape, StatusMatch};
        let mut endpoint = get("Get");
        endpoint.errors = vec![ErrorEntry::new(StatusMatch::exact(404), ResponseShape::Json(TypeRef::Named("GetError".into())))];
        let api = api_with(vec![endpoint.clone()]);
        assert!(rust_only(&[&api]).is_empty(), "one distinct shape needs no failure enum");

        endpoint.errors.push(ErrorEntry::new(StatusMatch::class(5), ResponseShape::Text));
        let api = api_with(vec![endpoint.clone()]);
        let diagnostics = rust_only(&[&api]);
        assert_eq!(features(&diagnostics), [(Severity::Error, Feature::GeneratedNameCollision)]);
        assert_eq!(diagnostics[0].message, "error type `GetError` of TestApi::Get is shadowed by the failure enum of TestApi::Get");

        // An opted-out endpoint has no failure enum.
        endpoint.errors.clear();
        endpoint.no_declared_errors = true;
        let mut api = api_with(vec![endpoint]);
        api.errors = vec![ErrorEntry::new(StatusMatch::exact(404), ResponseShape::Json(TypeRef::Named("GetError".into()))),
            ErrorEntry::new(StatusMatch::class(5), ResponseShape::Text)];
        api.schemas.push(schematic_define::SchemaComponent { name: "GetError".into(), rust_path: Some("GetError".into()), schema: serde_json::json!({"type": "object"}) });
        assert!(rust_only(&[&api]).is_empty());
    }

    fn with_shapes(id: &str, shapes: Vec<schematic_define::SuccessShape>) -> Endpoint {
        let mut endpoint = get(id);
        endpoint.response = shapes;
        endpoint
    }

    #[test]
    fn success_variants_left_colliding_by_derived_names_ask_for_author_names() {
        use schematic_define::{ResponseShape, SuccessShape};
        let status = |code: u16, variant: Option<&str>| {
            let mut shape = SuccessShape::new(ResponseShape::Json(TypeRef::Named("Response".into())));
            shape.statuses = vec![code];
            shape.variant = variant.map(str::to_owned);
            shape
        };
        let api = api_with(vec![with_shapes("Create", vec![status(200, None), status(202, None)])]);

        let diagnostics = rust_only(&[&api]);

        assert_eq!(features(&diagnostics), [(Severity::Error, Feature::GeneratedNameCollision)]);
        assert_eq!(diagnostics[0].endpoint.as_deref(), Some("Create"));
        assert_eq!(diagnostics[0].message, "API TestApi endpoint Create: success shape 1 (application/json), success shape 2 (application/json) \
            all generate the variant `Json` of `CreateResponse`; name them with `Success::variant`");
        let named = api_with(vec![with_shapes("Create", vec![status(200, Some("Record")), status(202, Some("Accepted"))])]);
        assert!(rust_only(&[&named]).is_empty());
    }

    #[test]
    fn a_stream_end_marker_joins_the_generated_names() {
        use schematic_define::{ResponseShape, StreamEnd, StreamFraming, SuccessShape};
        let mut api = api_with(vec![get("Other")]);
        let sse = ResponseShape::Stream { framing: StreamFraming::Sse(TypeRef::Named("Response".into())),
            end: StreamEnd::DataLineEquals("[DONE]".into()), keep: false };
        api.endpoints.push(with_shapes("Chat", vec![SuccessShape::new(sse)]));
        api.schemas.iter_mut().find(|c| c.name == "Response").expect("component").rust_path = Some("ChatEnd".into());

        let diagnostics = rust_only(&[&api]);

        assert!(diagnostics.iter().all(|d| d.feature == Feature::GeneratedNameCollision), "{diagnostics:?}");
        assert!(diagnostics.iter().any(|d| d.message == "response type `ChatEnd` of TestApi::Other is shadowed by the stream end marker of TestApi::Chat"), "{diagnostics:?}");
        assert!(diagnostics.iter().any(|d| d.message == "stream item type `ChatEnd` of TestApi::Chat is shadowed by the stream end marker of TestApi::Chat"), "{diagnostics:?}");
    }

    #[test]
    fn a_lone_json_string_error_needs_the_failure_enum() {
        use schematic_define::{ErrorEntry, PrimitiveType, ResponseShape, StatusMatch};
        let mut endpoint = get("Get");
        endpoint.errors = vec![ErrorEntry::new(StatusMatch::exact(400), ResponseShape::Json(TypeRef::Primitive(PrimitiveType::String)))];
        let mut api = api_with(vec![endpoint]);
        api.schemas.iter_mut().find(|c| c.name == "Response").expect("component").rust_path = Some("GetError".into());

        let diagnostics = rust_only(&[&api]);

        assert_eq!(features(&diagnostics), [(Severity::Error, Feature::GeneratedNameCollision)]);
        assert_eq!(diagnostics[0].message, "response type `GetError` of TestApi::Get is shadowed by the failure enum of TestApi::Get");
    }

    #[test]
    fn a_path_enum_type_shadowing_a_referenced_type_is_a_collision() {
        let kind = ParamDef::path("kind", QueryParamType::Enum(vec!["a".into()]));
        let mut endpoint = typed_path("Get", "/{kind}", vec![kind], vec![]);
        endpoint.response = vec![SuccessShape::json_type("GetRequestKind")];
        let api = api_with(vec![endpoint]);

        let diagnostics = rust_only(&[&api]);

        assert_eq!(features(&diagnostics), [(Severity::Error, Feature::GeneratedNameCollision)]);
        assert_eq!(diagnostics[0].endpoint.as_deref(), Some("Get"));
        assert_eq!(
            diagnostics[0].message,
            "response type `GetRequestKind` of TestApi::Get is shadowed by the `kind` path-parameter enum of TestApi::Get"
        );
    }

    #[test]
    fn a_path_rust_name_or_sanitized_name_colliding_with_a_query_field_is_a_collision() {
        let aliased = ParamDef::path("id", QueryParamType::Integer).rust_name("page");
        let by_rust_name = typed_path("ById", "/{id}", vec![aliased], vec![param("page", false)]);
        let camel = ParamDef::path("userId", QueryParamType::String);
        let by_sanitizer = typed_path("BySanitizer", "/users/{userId}", vec![camel], vec![param("user_id", false)]);
        let api = api_with(vec![by_rust_name, by_sanitizer]);

        let diagnostics = rust_only(&[&api]);

        assert_eq!(features(&diagnostics), vec![(Severity::Error, Feature::GeneratedNameCollision); 2]);
        assert_eq!(diagnostics[0].endpoint.as_deref(), Some("ById"));
        assert_eq!(
            diagnostics[0].message,
            "request struct field `page` is generated for path parameter `id` and query parameter `page`"
        );
        assert_eq!(diagnostics[1].endpoint.as_deref(), Some("BySanitizer"));
        assert_eq!(
            diagnostics[1].message,
            "request struct field `user_id` is generated for path parameter `userId` and query parameter `user_id`"
        );
    }

    #[test]
    fn a_repeated_typed_placeholder_is_one_field_and_valid() {
        let id = ParamDef::path("id", QueryParamType::Integer);
        let kind = ParamDef::path("kind", QueryParamType::Enum(vec!["a-b".into(), "c".into()]));
        let api = api_with(vec![typed_path("Copy", "/{id}/copies/{+id}/{kind}", vec![kind, id], vec![])]);

        assert!(validate_rust_client(&[&api]).is_empty());
    }

    #[test]
    fn path_declarations_the_rust_client_cannot_generate_are_path_parameter_errors() {
        let mut list = ParamDef::path("ids", QueryParamType::Array(Box::new(QueryParamType::Integer)));
        list.rust_name = Some("not a name".into());
        let api = api_with(vec![typed_path("Get", "/{ids}/{other}", vec![list], vec![])]);

        let diagnostics = validate_rust_client(&[&api]);

        assert_eq!(features(&diagnostics), vec![(Severity::Error, Feature::PathParameter); 3]);
        let messages: Vec<&str> = diagnostics.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(
            messages,
            [
                "path placeholder `{other}` has no declared path parameter",
                "path parameter `ids` has invalid Rust name `not a name`",
                "path parameter `ids` must be a scalar or enum, not an array or JSON value",
            ]
        );
    }
}
