//! Applies decoded `x-schematic` members beside the standard fields (R10,
//! R12, R54).
//!
//! Standard OpenAPI is authoritative. Where an extension member and a
//! standard field describe the same thing, the member is re-exported and
//! compared with the field: a match applies the member's extra detail, a
//! mismatch keeps the standard-derived value and reports a WARN at the
//! member's location. Members with no standard counterpart apply directly.

use indexmap::IndexMap;
use openapiv3::{OpenAPI, Operation, ReferenceOr, SecurityRequirement, SecurityScheme};
use serde_json::Value;

use super::diagnostics::OpenApiDiagnostic;
use super::extension::{AuthDetail, AuthDetails, DocumentExtension, OperationExtension, escape};
use super::mappings;
use super::naming::sanitize_rust_ident;
use super::OpenApiImportOptions;
use crate::auth::{AuthMethod, AuthPolicy, AuthStrategy, EndpointAuth, EnvAuthStrategy};
use crate::builders::env_mapping_conflicts;
use crate::error_table::{ErrorEntry, ErrorTable};
use crate::openapi::export::{map_request_body, map_responses, strip_reserved_markers};
use crate::params::{EndpointParams, ParamDef, QueryParamType};
use crate::request::{ApiRequest, FormField, FormFieldKind};
use crate::response::{ResponseShape, StreamFraming, SuccessShape};
use crate::types::{Endpoint, RestApi};
use crate::{PrimitiveType, TypeRef};

/// The API's auth from the root `security` and the document's auth details.
///
/// With a root `security`, its requirements are authoritative: none is
/// anonymous, one is an [`AuthStrategy`], several are an [`AuthPolicy`]
/// accepting each. Without one, the first supported component scheme is
/// taken.
pub(super) fn map_root_auth(
    doc: &OpenAPI,
    details: &AuthDetails,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> (AuthStrategy, Option<AuthPolicy>) {
    let schemes = doc.components.as_ref().map(|components| &components.security_schemes);
    let methods = match &doc.security {
        Some(requirements) => requirement_methods(requirements, "#/security", schemes, details, diagnostics),
        None => {
            let first = schemes.into_iter().flatten().find_map(|(name, scheme)| {
                let location = format!("#/components/securitySchemes/{}", escape(name));
                match scheme {
                    ReferenceOr::Item(scheme) if unnamed_api_key(scheme, &location, diagnostics) => None,
                    ReferenceOr::Item(scheme) => match mappings::map_security_scheme(scheme, diagnostics, &location) {
                        Ok(strategy) => Some((name, scheme, strategy)),
                        Err(message) => {
                            diagnostics.push(OpenApiDiagnostic::warn(location, message));
                            None
                        }
                    },
                    ReferenceOr::Reference { reference } => {
                        diagnostics.push(OpenApiDiagnostic::warn(location, format!("Security scheme reference '{reference}' not supported")));
                        None
                    }
                }
            });
            let applied: Vec<&str> = first.iter().map(|(name, ..)| name.as_str()).collect();
            report_unapplied(details, &applied, "the document applies", diagnostics);
            first
                .and_then(|(name, scheme, strategy)| method_with_detail(name, scheme, strategy, details, diagnostics))
                .into_iter()
                .collect()
        }
    };
    match methods.as_slice() {
        [] => (AuthStrategy::None, None),
        [only] => (strategy_of(only), None),
        several => {
            let policy = policy_of(several.to_vec());
            let primary = several.iter().find(|method| env_form(method).is_some()).unwrap_or(&several[0]);
            (strategy_of(primary), Some(policy))
        }
    }
}

/// The endpoint auth an operation's own `security` declares.
///
/// Absent inherits; `[]` is public; the document's own requirements, with no
/// operation auth details or `env_mapping`, also inherit. Operation auth details and `env_mapping`
/// apply only to an override (R54); elsewhere they are WARN.
pub(super) fn map_operation_auth(
    doc: &OpenAPI,
    operation: &Operation,
    base: &str,
    ext: &mut OperationExtension,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Option<EndpointAuth> {
    let env_mapping = ext.env_mapping.take();
    let without_override = |what: &str, pointer: String, diagnostics: &mut Vec<OpenApiDiagnostic>| {
        diagnostics.push(OpenApiDiagnostic::warn(
            pointer,
            format!("{what} applies only to an operation whose own `security` replaces the document's; ignored"),
        ));
    };
    let requirements = match &operation.security {
        Some(requirements) if !requirements.is_empty() => requirements,
        security => {
            for (_, _, pointer) in &ext.auth {
                without_override("an auth detail", pointer.clone(), diagnostics);
            }
            if env_mapping.is_some() {
                without_override("`env_mapping`", ext.at("env_mapping"), diagnostics);
            }
            return security.as_ref().map(|_| EndpointAuth::Public);
        }
    };
    if doc.security.as_ref() == Some(requirements) && ext.auth.is_empty() && env_mapping.is_none() {
        return None;
    }
    let schemes = doc.components.as_ref().map(|components| &components.security_schemes);
    let methods = requirement_methods(requirements, &format!("{base}/security"), schemes, &ext.auth, diagnostics);
    if methods.is_empty() {
        return None;
    }
    Some(EndpointAuth::Policy { policy: policy_of(methods), env_mapping })
}

/// Drops the environment fallback of each endpoint policy whose effective
/// mapping cannot supply it; standard OpenAPI says nothing about one.
pub(super) fn settle_endpoint_fallbacks(api: &mut RestApi) {
    for index in 0..api.endpoints.len() {
        if crate::builders::credential_source_problem(api, &api.endpoints[index]).is_some()
            && let Some(EndpointAuth::Policy { policy, .. }) = &mut api.endpoints[index].auth
        {
            policy.env_fallback = None;
        }
    }
}

/// Every representable method of `requirements`, in order and once each.
fn requirement_methods(
    requirements: &[SecurityRequirement],
    base: &str,
    schemes: Option<&IndexMap<String, ReferenceOr<SecurityScheme>>>,
    details: &AuthDetails,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Vec<AuthMethod> {
    let mut methods = Vec::new();
    let mut applied = Vec::new();
    for (index, requirement) in requirements.iter().enumerate() {
        let location = format!("{base}/{index}");
        let name = match requirement.keys().collect::<Vec<_>>().as_slice() {
            [] => {
                diagnostics.push(OpenApiDiagnostic::info(location, "an empty requirement also accepts anonymous requests".into()));
                continue;
            }
            [name] => name.as_str(),
            _ => {
                diagnostics.push(OpenApiDiagnostic::warn(
                    location,
                    "a requirement naming several schemes at once cannot be represented; it is ignored".into(),
                ));
                continue;
            }
        };
        applied.push(name);
        let scheme_location = format!("#/components/securitySchemes/{}", escape(name));
        let scheme = match schemes.and_then(|schemes| schemes.get(name)) {
            Some(ReferenceOr::Item(scheme)) => scheme,
            Some(ReferenceOr::Reference { reference }) => {
                diagnostics.push(OpenApiDiagnostic::warn(scheme_location, format!("Security scheme reference '{reference}' not supported")));
                continue;
            }
            None => {
                diagnostics.push(OpenApiDiagnostic::error(location, format!("security scheme `{name}` is not declared in components")));
                continue;
            }
        };
        if unnamed_api_key(scheme, &scheme_location, diagnostics) {
            continue;
        }
        let strategy = match mappings::map_security_scheme(scheme, diagnostics, &scheme_location) {
            Ok(strategy) => strategy,
            Err(message) => {
                diagnostics.push(OpenApiDiagnostic::warn(scheme_location, message));
                continue;
            }
        };
        if let Some(method) = method_with_detail(name, scheme, strategy, details, diagnostics)
            && !methods.contains(&method)
        {
            methods.push(method);
        }
    }
    report_unapplied(details, &applied, "this `security` applies", diagnostics);
    methods
}

/// An `apiKey` scheme with an empty `name`, which `openapiv3` accepts and
/// OpenAPI does not, is an ERROR and is not mapped.
fn unnamed_api_key(scheme: &SecurityScheme, location: &str, diagnostics: &mut Vec<OpenApiDiagnostic>) -> bool {
    let unnamed = matches!(scheme, SecurityScheme::APIKey { name, .. } if name.is_empty());
    if unnamed {
        diagnostics.push(OpenApiDiagnostic::error(format!("{location}/name"), "an `apiKey` scheme needs a nonempty `name`".to_string()));
    }
    unnamed
}

/// `strategy`'s method with the detail declared for scheme `name`, which
/// must be an `apiKey` header scheme; otherwise the standard scheme wins.
fn method_with_detail(
    name: &str,
    scheme: &SecurityScheme,
    strategy: AuthStrategy,
    details: &AuthDetails,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Option<AuthMethod> {
    let method = AuthPolicy::from_auth_strategy(&strategy).explicit.into_iter().next()?;
    let Some((_, detail, pointer)) = details.iter().find(|(scheme, ..)| scheme == name) else { return Some(method) };
    let header = match scheme {
        SecurityScheme::APIKey { location: openapiv3::APIKeyLocation::Header, name, .. } => name.clone(),
        _ => {
            diagnostics.push(OpenApiDiagnostic::warn(
                pointer.clone(),
                format!("scheme `{name}` is not an `apiKey` header scheme, so it cannot carry this detail; the standard scheme wins"),
            ));
            return Some(method);
        }
    };
    Some(match detail {
        AuthDetail::Bearer => AuthMethod::BearerToken { header: Some(header) },
        AuthDetail::ValuePrefix(prefix) => AuthMethod::ApiKey { header, value_prefix: Some(prefix.clone()) },
    })
}

fn report_unapplied(details: &AuthDetails, applied: &[&str], what: &str, diagnostics: &mut Vec<OpenApiDiagnostic>) {
    for (scheme, _, pointer) in details {
        if !applied.contains(&scheme.as_str()) {
            diagnostics.push(OpenApiDiagnostic::warn(
                pointer.clone(),
                format!("auth detail names `{scheme}`, which is not a scheme {what}; ignored"),
            ));
        }
    }
}

/// A policy accepting `methods`, falling back to the first one the
/// environment can supply.
fn policy_of(methods: Vec<AuthMethod>) -> AuthPolicy {
    let env_fallback = methods.iter().find_map(env_form);
    AuthPolicy { explicit: methods, env_fallback }
}

fn env_form(method: &AuthMethod) -> Option<EnvAuthStrategy> {
    Some(match method {
        AuthMethod::BearerToken { header } => EnvAuthStrategy::BearerToken { header: header.clone() },
        AuthMethod::ApiKey { header, value_prefix } => EnvAuthStrategy::ApiKey { header: header.clone(), value_prefix: value_prefix.clone() },
        AuthMethod::ApiKeyParam { name, location } => EnvAuthStrategy::ApiKeyParam { name: name.clone(), location: *location },
        AuthMethod::Basic => EnvAuthStrategy::Basic,
        _ => return None,
    })
}

fn strategy_of(method: &AuthMethod) -> AuthStrategy {
    match method {
        AuthMethod::BearerToken { header } => AuthStrategy::BearerToken { header: header.clone() },
        AuthMethod::ApiKey { header, value_prefix } => AuthStrategy::ApiKey { header: header.clone(), value_prefix: value_prefix.clone() },
        AuthMethod::ApiKeyParam { name, location } => AuthStrategy::ApiKeyParam { name: name.clone(), location: *location },
        AuthMethod::Basic => AuthStrategy::Basic {
            username_env: crate::headers::EnvList::new(vec![]),
            password_env: crate::headers::EnvList::new(vec![]),
        },
        AuthMethod::OAuth2(config) => AuthStrategy::OAuth2(config.clone()),
    }
}

/// Applies the document's members once its endpoints are mapped. The CLI's
/// `--module-path` wins over the extension's.
pub(super) fn apply_document(
    api: &mut RestApi,
    ext: DocumentExtension,
    options: &OpenApiImportOptions,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) {
    if options.module_path.is_none() {
        api.module_path = ext.module_path;
    }
    api.request_suffix = ext.request_suffix;
    if let Some((names, pointer)) = ext.env_auth {
        if matches!(api.auth, AuthStrategy::Basic { .. }) && !names.is_empty() {
            diagnostics.push(OpenApiDiagnostic::warn(
                pointer,
                "Basic authentication carries its own environment lists, so `env_auth` must be empty; ignored".into(),
            ));
        } else {
            api.env_auth = names;
        }
    }
    if let Some((mapping, pointer)) = ext.env_mapping {
        // Basic's own lists are the mapping's: the builder writes both.
        if let AuthStrategy::Basic { username_env, password_env } = &mut api.auth
            && username_env.names().is_empty()
            && password_env.names().is_empty()
        {
            if let Some(user) = &mapping.basic_user { *username_env = user.clone(); }
            if let Some(pass) = &mapping.basic_pass { *password_env = pass.clone(); }
        }
        if env_mapping_conflicts(&api.auth, &api.env_auth, &mapping) {
            diagnostics.push(OpenApiDiagnostic::warn(
                pointer,
                "`env_mapping` names different variables than the authentication declaration; ignored".into(),
            ));
        } else {
            api.env_mapping = Some(mapping);
        }
    }
    api.headers = ext.headers;
    api.rate_limit = ext.rate_limit;
    api.retry_guidance = ext.retry_guidance;
    api.errors = ext.errors;
    api.errors.iter_mut().for_each(|entry| sanitize_shape(&mut entry.shape));
    if let Some(policy) = ext.cross_origin {
        api.cross_origin = policy;
    }
    api.request_id_header = ext.request_id_header;
    api.secret_names = ext.secret_names;
}

/// What reconciling one operation's responses decided, for the builder's
/// pass over every operation.
#[derive(Debug, Default)]
pub(super) struct OperationOutcome {
    /// The success shapes came from the extension, so the standard
    /// classification's 2xx diagnostics no longer describe the endpoint.
    pub success_from_extension: bool,
    /// The error table came from the extension (likewise for 4xx/5xx).
    pub errors_from_extension: bool,
    /// The applied error table inherits the document's `errors`; the
    /// standard-derived entries to restore if those are withdrawn.
    pub inherits_document_errors: Option<Vec<ErrorEntry>>,
    /// The operation's standard responses differ from its declared table
    /// only at statuses the document's `errors` alone export: the document's
    /// table is the conflict, and the standard-derived table is kept.
    pub contradicts_document_errors: bool,
}

/// Applies one operation's members to its standard-derived `endpoint`.
///
/// `path_key` is the operation's OpenAPI path; `doc_ext` the document's
/// decoded members, when its Schematic metadata is enabled.
pub(super) fn apply_operation(
    endpoint: &mut Endpoint,
    operation: &Operation,
    path_key: &str,
    mut ext: OperationExtension,
    doc_ext: Option<&DocumentExtension>,
    doc: &OpenAPI,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> OperationOutcome {
    let base = ext.base.trim_end_matches("/x-schematic").to_string();
    endpoint.auth = map_operation_auth(doc, operation, &base, &mut ext, diagnostics);
    reconcile_request(endpoint, operation, &ext, doc, diagnostics);
    let outcome = reconcile_responses(endpoint, operation, &ext, doc_ext, diagnostics);

    endpoint.headers = std::mem::take(&mut ext.headers);
    endpoint.rate_limit = ext.rate_limit.take();
    endpoint.retry_guidance = ext.retry_guidance.take();
    endpoint.cross_origin = ext.cross_origin.take();
    let api_headers = doc_ext.map_or(&[][..], |doc_ext| doc_ext.headers.as_slice());
    drop_fixed_header_parameters(endpoint, api_headers);

    if let Some(template) = ext.path_template.take() {
        if strip_reserved_markers(&template) == path_key {
            endpoint.path = template;
        } else {
            diagnostics.push(OpenApiDiagnostic::warn(
                ext.at("path_template"),
                format!("`{template}` without its reserved-expansion markers is not the path `{path_key}`; the path wins"),
            ));
        }
    }
    apply_param_details(endpoint, &ext, diagnostics);
    outcome
}

/// Withdraws the document's `errors` when an operation's standard responses
/// contradict them (R12): one WARN, no API defaults, and every endpoint that
/// inherited them gets its standard-derived table back.
pub(super) fn settle_document_errors(
    endpoints: &mut [Endpoint],
    outcomes: Vec<OperationOutcome>,
    doc_ext: &mut DocumentExtension,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) {
    let contradicted = outcomes.iter().filter(|outcome| outcome.contradicts_document_errors).count();
    if contradicted == 0 {
        return;
    }
    diagnostics.push(OpenApiDiagnostic::warn(
        "#/x-schematic/errors".to_string(),
        format!("the default error table disagrees with the standard responses of {contradicted} operation(s); it is not applied and each operation keeps its standard error responses"),
    ));
    doc_ext.errors.clear();
    for (endpoint, outcome) in endpoints.iter_mut().zip(outcomes) {
        if let Some(standard) = outcome.inherits_document_errors {
            endpoint.errors = standard;
            endpoint.no_declared_errors = false;
        }
    }
}

/// A header parameter that is the single-value `enum` of an effective fixed
/// header is that header's standard form (as `export/headers.rs` writes it),
/// not a parameter.
fn drop_fixed_header_parameters(endpoint: &mut Endpoint, api_headers: &[(String, String)]) {
    let Some(params) = &mut endpoint.params else { return };
    let effective = |name: &str| {
        endpoint.headers.iter().chain(api_headers.iter().filter(|(api, _)| {
            !endpoint.headers.iter().any(|(own, _)| own.eq_ignore_ascii_case(api))
        })).find(|(header, _)| header.eq_ignore_ascii_case(name)).map(|(_, value)| value)
    };
    params.header.retain(|param| {
        !(param.required && effective(&param.name).is_some_and(|value| param.param_type == QueryParamType::Enum(vec![value.clone()])))
    });
    if params == &EndpointParams::default() {
        endpoint.params = None;
    }
}

/// `params.<location>.<name>` details: a secret marker and a Rust name on
/// a standard parameter. A plain-string path parameter is imported only
/// when a detail needs it.
fn apply_param_details(endpoint: &mut Endpoint, ext: &OperationExtension, diagnostics: &mut Vec<OpenApiDiagnostic>) {
    for detail in &ext.params {
        let placeholders: Vec<String> = endpoint.path_placeholders().into_iter().map(str::to_string).collect();
        let params = endpoint.params.get_or_insert_with(EndpointParams::default);
        if detail.location == "path" && params.path.is_empty() && placeholders.contains(&detail.name) {
            params.path = placeholders.iter().map(|name| ParamDef::path(name.clone(), QueryParamType::String)).collect();
        }
        let list = match detail.location {
            "query" => &mut params.query,
            "header" => &mut params.header,
            "cookie" => &mut params.cookie,
            _ => &mut params.path,
        };
        let Some(param) = list.iter_mut().find(|param| {
            if detail.location == "header" { param.name.eq_ignore_ascii_case(&detail.name) } else { param.name == detail.name }
        }) else {
            diagnostics.push(OpenApiDiagnostic::warn(
                detail.pointer.clone(),
                format!("names no standard {} parameter `{}`; ignored", detail.location, detail.name),
            ));
            continue;
        };
        if let Some(secret) = detail.secret {
            param.secret = secret;
        }
        if let Some(rust_name) = &detail.rust_name {
            if param.rust_name.as_ref().is_some_and(|existing| existing != rust_name) {
                diagnostics.push(OpenApiDiagnostic::warn(
                    detail.pointer.clone(),
                    format!("`rust_name` `{rust_name}` replaces the query-path normalization's `{}`", param.rust_name.as_deref().unwrap_or_default()),
                ));
            }
            param.rust_name = Some(rust_name.clone());
        }
    }
    if endpoint.params.as_ref() == Some(&EndpointParams::default()) {
        endpoint.params = None;
    }
}

/// A form body's extension fills what its standard schema left unsaid:
/// `accept` without an encoding `contentType`, file counts without
/// `minItems`/`maxItems`, and a JSON part's reference. A JSON, text, or
/// binary body is standard alone; an extension copy of one is only compared.
fn reconcile_request(
    endpoint: &mut Endpoint,
    operation: &Operation,
    ext: &OperationExtension,
    doc: &OpenAPI,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) {
    let Some(declared) = &ext.request else { return };
    let defined = |name: &str| doc.components.as_ref().is_some_and(|components| components.schemas.contains_key(name));
    let pointer = ext.at("request");
    let mismatch = |diagnostics: &mut Vec<OpenApiDiagnostic>, what: String| {
        diagnostics.push(OpenApiDiagnostic::warn(pointer.clone(), format!("{what}; the standard requestBody wins")));
    };
    match (declared, &mut endpoint.request) {
        (ApiRequest::FormData { fields: declared }, Some(ApiRequest::FormData { fields }))
        | (ApiRequest::UrlEncoded { fields: declared }, Some(ApiRequest::UrlEncoded { fields })) => {
            for (index, extra) in declared.iter().enumerate() {
                if !fields.iter().any(|field| field.name == extra.name) {
                    mismatch(diagnostics, format!("field {index} `{}` names no standard part", extra.name));
                }
            }
            for field in fields.iter_mut() {
                let Some(extra) = declared.iter().find(|extra| extra.name == field.name) else { continue };
                // A reference the extension would fill must close (R7).
                if let FormFieldKind::Json(TypeRef::Named(name)) = &extra.kind
                    && field.kind == FormFieldKind::Json(TypeRef::Primitive(PrimitiveType::Json))
                    && !defined(name)
                {
                    diagnostics.push(OpenApiDiagnostic::error(
                        pointer.clone(),
                        format!("part `{}` names schema `{name}`, which the document does not define; the part stays untyped", field.name),
                    ));
                    continue;
                }
                if let Some(problem) = fill_field(field, extra) {
                    mismatch(diagnostics, problem);
                }
            }
        }
        (ApiRequest::FormData { .. } | ApiRequest::UrlEncoded { .. }, _) => {
            mismatch(diagnostics, "the extension declares a form body the standard requestBody does not".into());
        }
        (legacy, _) => {
            let standard = operation.request_body.as_ref().and_then(|body| match body {
                ReferenceOr::Item(body) => serde_json::to_value(&body.content).ok(),
                ReferenceOr::Reference { .. } => None,
            });
            let exported = map_request_body(legacy).ok().and_then(|body| serde_json::to_value(&body.content).ok());
            if standard != exported {
                mismatch(diagnostics, "the extension's request body differs from the standard requestBody".into());
            }
        }
    }
}

/// Fills `field` from its extension copy; a disagreement the standard
/// schema settles is returned as a problem, and the standard value kept.
fn fill_field(field: &mut FormField, extra: &FormField) -> Option<String> {
    let name = &field.name;
    let mut problems = Vec::new();
    if field.required != extra.required {
        problems.push("required");
    }
    // An empty description says no more than an absent one.
    if extra.description.as_deref().is_some_and(|description| !description.is_empty()) && field.description != extra.description {
        problems.push("description");
    }
    let fill_accept = |accept: &mut Vec<String>, declared: &Vec<String>, problems: &mut Vec<&str>| {
        if accept.is_empty() { accept.clone_from(declared) } else if !declared.is_empty() && accept != declared { problems.push("accept") }
    };
    let fill_count = |count: &mut Option<u32>, declared: Option<u32>, problems: &mut Vec<&str>| match (*count, declared) {
        (None, declared) => *count = declared,
        (Some(own), Some(declared)) if own != declared => problems.push("file count"),
        _ => {}
    };
    match (&mut field.kind, &extra.kind) {
        (FormFieldKind::Text, FormFieldKind::Text) => {}
        (FormFieldKind::File { accept }, FormFieldKind::File { accept: declared }) => fill_accept(accept, declared, &mut problems),
        (FormFieldKind::Files { accept, min, max }, FormFieldKind::Files { accept: declared, min: declared_min, max: declared_max }) => {
            fill_accept(accept, declared, &mut problems);
            fill_count(min, *declared_min, &mut problems);
            fill_count(max, *declared_max, &mut problems);
        }
        (FormFieldKind::Json(reference), FormFieldKind::Json(declared)) => {
            let declared = match declared {
                TypeRef::Named(name) => TypeRef::Named(sanitize_rust_ident(name)),
                other => other.clone(),
            };
            if *reference == TypeRef::Primitive(PrimitiveType::Json) {
                *reference = declared;
            } else if *reference != declared {
                problems.push("JSON reference");
            }
        }
        _ => problems.push("kind"),
    }
    (!problems.is_empty()).then(|| format!("part `{name}` disagrees on {}", problems.join(", ")))
}

/// `success`, `response`, `errors`, and `no_declared_errors` apply only when
/// re-exporting them reproduces the operation's standard responses, success
/// and error statuses compared apart.
///
/// A rejected member (already reported) leaves the standard-derived value
/// without a second WARN; so does a legacy `response` that disagrees, since
/// it only ever summarized the first shape. An error table that disagrees is
/// reported at this operation's `x-schematic/errors`, unless dropping the
/// document's `errors` would make it agree: then the document's table is the
/// conflict, which [`settle_document_errors`] reports once.
fn reconcile_responses(
    endpoint: &mut Endpoint,
    operation: &Operation,
    ext: &OperationExtension,
    doc_ext: Option<&DocumentExtension>,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> OperationOutcome {
    let mut outcome = OperationOutcome::default();
    let Some((standard_success, standard_errors)) = response_parts(&operation.responses) else { return outcome };
    if ext.rejected {
        return outcome;
    }

    let candidate = match (&ext.success, &ext.response) {
        (Some(shapes), _) => Some((shapes.clone(), true)),
        (None, Some(shape)) if !ext.success_rejected => Some((vec![SuccessShape::new(shape.clone())], false)),
        _ => None,
    };
    if let Some((mut shapes, reported)) = candidate {
        let exported = map_responses(&shapes, &ErrorTable::resolve(&[], &[], false)).ok().as_ref().and_then(response_parts);
        if exported.is_some_and(|(success, _)| success == standard_success) {
            shapes.iter_mut().for_each(|success| sanitize_shape(&mut success.shape));
            endpoint.response = shapes;
            outcome.success_from_extension = true;
        } else if reported {
            diagnostics.push(OpenApiDiagnostic::warn(
                ext.at("success"),
                "`success` disagrees with the operation's standard success responses; the standard responses win".into(),
            ));
        }
    }

    let doc_errors = doc_ext.map_or(&[][..], |doc_ext| doc_ext.errors.as_slice());
    let doc_rejected = doc_ext.is_some_and(|doc_ext| doc_ext.errors_rejected);
    let declared = !doc_errors.is_empty() || ext.errors.is_some() || ext.no_declared_errors.is_some();
    if !declared || doc_rejected || ext.errors_rejected {
        return outcome;
    }
    let own = ext.errors.clone().unwrap_or_default();
    let opted_out = ext.no_declared_errors.unwrap_or(false);
    let exported = |inherited: &[ErrorEntry], own: &[ErrorEntry], opted_out: bool| {
        map_responses(&[], &ErrorTable::resolve(inherited, own, opted_out)).ok().as_ref().and_then(response_parts)
            .map(|(_, errors)| errors).unwrap_or_default()
    };
    let candidate = exported(doc_errors, &own, opted_out);
    let inherits = !doc_errors.is_empty() && !opted_out;
    if candidate != standard_errors {
        // Statuses only a document entry exports put the conflict on the
        // document member, reported once for every operation.
        let differing = differing_keys(&candidate, &standard_errors);
        let (from_document, from_own) = (exported(doc_errors, &[], false), exported(&[], &own, false));
        if inherits && differing.iter().all(|key| from_document.get(key).is_some() && from_own.get(key).is_none()) {
            outcome.contradicts_document_errors = true;
        } else {
            diagnostics.push(OpenApiDiagnostic::warn(
                ext.at("errors"),
                "the declared error table disagrees with the operation's standard error responses; the standard responses win".into(),
            ));
        }
        return outcome;
    }
    if inherits {
        outcome.inherits_document_errors = Some(endpoint.errors.clone());
    }
    endpoint.errors = own;
    endpoint.errors.iter_mut().for_each(|entry| sanitize_shape(&mut entry.shape));
    endpoint.no_declared_errors = opted_out;
    outcome.errors_from_extension = true;
    outcome
}

/// Spells `shape`'s component names as the catalog does. The extension
/// carries the definition's names (`PaginatedResponse<Tag>`), which the
/// comparison with the standard `$ref`s needs, but the imported components
/// are sanitized Rust identifiers.
fn sanitize_shape(shape: &mut ResponseShape) {
    match shape {
        ResponseShape::Json(reference)
        | ResponseShape::Stream { framing: StreamFraming::Sse(reference) | StreamFraming::Ndjson(reference), .. } => {
            sanitize_names(reference);
        }
        _ => {}
    }
}

fn sanitize_names(reference: &mut TypeRef) {
    match reference {
        TypeRef::Named(name) => *name = sanitize_rust_ident(name),
        TypeRef::Array(inner) | TypeRef::Map(inner) | TypeRef::Optional(inner) | TypeRef::KeyedMap { value: inner, .. } => {
            sanitize_names(inner);
        }
        TypeRef::OneOf(options) | TypeRef::AnyOf(options) | TypeRef::AllOf(options) => options.iter_mut().for_each(sanitize_names),
        _ => {}
    }
}

/// The status keys whose responses differ between two response maps.
fn differing_keys(left: &Value, right: &Value) -> Vec<String> {
    let (Some(left), Some(right)) = (left.as_object(), right.as_object()) else { return vec![] };
    left.keys().chain(right.keys()).filter(|key| left.get(*key) != right.get(*key)).cloned().collect()
}

/// The 2xx and the other responses as values, each response's description
/// removed (it is prose, not meaning).
fn response_parts(responses: &openapiv3::Responses) -> Option<(Value, Value)> {
    let Value::Object(mut all) = serde_json::to_value(responses).ok()? else { return None };
    let mut success = serde_json::Map::new();
    let mut errors = serde_json::Map::new();
    for (status, mut response) in std::mem::take(&mut all) {
        if let Value::Object(response) = &mut response {
            response.remove("description");
        }
        if status.starts_with('2') { success.insert(status, response) } else { errors.insert(status, response) };
    }
    Some((Value::Object(success), Value::Object(errors)))
}
