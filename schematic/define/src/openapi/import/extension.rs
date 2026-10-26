//! Reads the `x-schematic` extensions back (R10–R13, R54).
//!
//! Each member is decoded from the raw extension value on its own, so a
//! malformed member is reported with its JSON-pointer location and dropped
//! while every other member still applies. What a member means once decoded,
//! and how it reconciles with the standard fields it sits beside, is in
//! [`super::reconcile`].
//!
//! The rules, from `schematic/docs/io/openapi-extensions.md`:
//!
//! - `x-schematic.version` is document-level only. Absent means major 1; any
//!   other value, or an `x-schematic` that is not an object, ignores every
//!   Schematic extension in the document with one WARN.
//! - An unknown member is INFO. A malformed member, including an explicit
//!   `null` anywhere the v1 writer never writes one, is WARN.

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use super::diagnostics::OpenApiDiagnostic;
use crate::builders::{valid_header_name, valid_rust_name, validate_origin};
use crate::cross_origin::CrossOrigin;
use crate::error_table::{ErrorEntry, validate_error_entries};
use crate::headers::EnvMapping;
use crate::metadata::{RateLimit, RetryGuidance};
use crate::openapi::EXTENSION_VERSION;
use crate::request::{ApiRequest, FormFieldKind};
use crate::response::{ResponseShape, SuccessShape, validate_success_shapes};

/// The extension key, at document and operation level.
pub(super) const KEY: &str = "x-schematic";

/// The document-level sibling the generator writes when one document merges
/// several APIs.
const GROUPED_FROM: &str = "x-schematic-grouped-from";

/// Escapes one RFC 6901 reference token.
pub(super) fn escape(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

/// One R54 auth detail, as declared, with where it was declared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AuthDetail {
    /// The `apiKey` header scheme carries a bearer token.
    Bearer,
    /// The prefix prepended to an API key.
    ValuePrefix(String),
}

/// Auth details keyed by security scheme name, each with its location.
pub(super) type AuthDetails = Vec<(String, AuthDetail, String)>;

/// One `params.<location>.<name>` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParamDetail {
    /// `query`, `header`, `cookie`, or `path`.
    pub location: &'static str,
    /// The parameter's wire name.
    pub name: String,
    /// `None` when the entry does not say.
    pub secret: Option<bool>,
    pub rust_name: Option<String>,
    /// The entry's JSON pointer.
    pub pointer: String,
}

/// The document-level members that decoded and validated. `None` and empty
/// mean absent or rejected alike; a rejection was already reported.
#[derive(Debug, Default)]
pub(super) struct DocumentExtension {
    pub module_path: Option<String>,
    pub request_suffix: Option<String>,
    /// With its pointer, for the conflict check against the auth declaration.
    pub env_mapping: Option<(EnvMapping, String)>,
    pub env_auth: Option<(Vec<String>, String)>,
    pub headers: Vec<(String, String)>,
    pub auth: AuthDetails,
    pub rate_limit: Option<RateLimit>,
    pub retry_guidance: Option<RetryGuidance>,
    pub errors: Vec<ErrorEntry>,
    /// `errors` was present but rejected, so no operation's error table can
    /// be decomposed against it.
    pub errors_rejected: bool,
    pub cross_origin: Option<CrossOrigin>,
    pub request_id_header: Option<String>,
    pub secret_names: Vec<String>,
}

/// The operation-level members that decoded and validated, each with the
/// pointer later reconciliation reports against.
#[derive(Debug, Default)]
pub(super) struct OperationExtension {
    /// The extension's own pointer (`…/<method>/x-schematic`).
    pub base: String,
    /// The whole extension was present but not an object.
    pub rejected: bool,
    pub request: Option<ApiRequest>,
    pub response: Option<ResponseShape>,
    pub success: Option<Vec<SuccessShape>>,
    /// `success` was present but rejected; the legacy `response` beside it
    /// then says nothing reliable either.
    pub success_rejected: bool,
    pub headers: Vec<(String, String)>,
    pub rate_limit: Option<RateLimit>,
    pub retry_guidance: Option<RetryGuidance>,
    pub errors: Option<Vec<ErrorEntry>>,
    pub no_declared_errors: Option<bool>,
    /// `errors` or `no_declared_errors` was present but rejected.
    pub errors_rejected: bool,
    pub cross_origin: Option<CrossOrigin>,
    pub auth: AuthDetails,
    pub env_mapping: Option<EnvMapping>,
    pub path_template: Option<String>,
    pub params: Vec<ParamDetail>,
}

impl OperationExtension {
    /// The pointer of member `key`.
    pub fn at(&self, key: &str) -> String {
        format!("{}/{key}", self.base)
    }
}

const DOCUMENT_MEMBERS: [&str; 13] = [
    "version", "module_path", "request_suffix", "env_mapping", "env_auth", "headers", "auth", "rate_limit",
    "retry_guidance", "errors", "cross_origin", "request_id_header", "secret_names",
];

const OPERATION_MEMBERS: [&str; 13] = [
    "request", "response", "success", "headers", "rate_limit", "retry_guidance", "errors", "no_declared_errors",
    "cross_origin", "auth", "env_mapping", "path_template", "params",
];

const ENV_MAPPING_MEMBERS: [&str; 7] = [
    "bearer_token", "basic_user", "basic_pass", "api_key", "oauth_client_id", "oauth_client_secret",
    "oauth_redirect_uri",
];

/// Where the v1 writer itself emits `null`, so `null` there means none.
const ENV_MAPPING_NULLS: [&str; 4] = ["bearer_token", "basic_user", "basic_pass", "api_key"];
const REQUEST_NULLS: [&str; 3] = ["*/fields/*/description", "*/fields/*/kind/Files/min", "*/fields/*/kind/Files/max"];

/// Decodes the document-level extension.
///
/// `None` when the document's Schematic metadata is disabled: an
/// unsupported or malformed version, or an `x-schematic` that is not an
/// object. Operation extensions are then ignored too.
pub(super) fn decode_document(
    extensions: &indexmap::IndexMap<String, Value>,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Option<DocumentExtension> {
    if let Some(grouped) = extensions.get(GROUPED_FROM) {
        diagnostics.push(OpenApiDiagnostic::info(
            format!("#/{GROUPED_FROM}"),
            format!("the document merges several APIs ({grouped}); only the first API's `x-schematic` metadata applies"),
        ));
    }
    let base = format!("#/{KEY}");
    let object = match extensions.get(KEY) {
        None => return Some(DocumentExtension::default()),
        Some(Value::Object(object)) => object,
        Some(other) => {
            diagnostics.push(OpenApiDiagnostic::warn(
                base,
                format!("`{KEY}` must be an object, found {}; every Schematic extension in the document is ignored", kind(other)),
            ));
            return None;
        }
    };
    if let Some(version) = object.get("version")
        && version.as_u64() != Some(u64::from(EXTENSION_VERSION))
    {
        diagnostics.push(OpenApiDiagnostic::warn(
            format!("{base}/version"),
            format!(
                "unsupported `{KEY}` version {version} (this importer reads the integer major {EXTENSION_VERSION}); \
                 every Schematic extension in the document is ignored"
            ),
        ));
        return None;
    }
    report_unknown(object, &DOCUMENT_MEMBERS, &base, diagnostics);

    let mut members = Members { object, base, diagnostics };
    let env_mapping = members.env_mapping("env_mapping");
    let errors = members.decode("errors", &[], |entries: &Vec<ErrorEntry>| validate_error_entries(entries));
    Some(DocumentExtension {
        errors_rejected: object.contains_key("errors") && errors.is_none(),
        errors: errors.unwrap_or_default(),
        module_path: members.decode("module_path", &[], |path: &String| nonempty(path, "module path")),
        request_suffix: members.decode("request_suffix", &[], |suffix: &String| {
            if suffix.is_empty() || !suffix.chars().all(char::is_alphanumeric) {
                return Err(format!("request suffix `{suffix}` must be nonempty and alphanumeric"));
            }
            Ok(())
        }),
        env_auth: members
            .decode("env_auth", &[], |names: &Vec<String>| {
                names.iter().try_for_each(|name| nonempty(name, "environment variable name"))
            })
            .map(|names| (names, members.at("env_auth"))),
        env_mapping: env_mapping.map(|mapping| (mapping, members.at("env_mapping"))),
        headers: members.headers("headers").unwrap_or_default(),
        auth: members.auth("auth"),
        rate_limit: members.decode("rate_limit", &[], |limit: &RateLimit| limit.validate().map_err(|e| e.to_string())),
        retry_guidance: members.decode("retry_guidance", &[], |guidance: &RetryGuidance| {
            guidance.validate().map_err(|e| e.to_string())
        }),
        cross_origin: members.decode("cross_origin", &[], validate_origin),
        request_id_header: members
            .decode("request_id_header", &[], |name: &String| header_name(name, "request-id header"))
            .map(|name| name.to_ascii_lowercase()),
        secret_names: members
            .decode("secret_names", &[], |names: &Vec<String>| {
                names.iter().try_for_each(|name| header_name(name, "secret name"))
            })
            .unwrap_or_default(),
    })
}

/// Decodes one operation's extension; `base` is the operation's pointer.
/// Absent, or ignored because the document disabled Schematic metadata,
/// decodes as no metadata.
pub(super) fn decode_operation(
    extensions: &indexmap::IndexMap<String, Value>,
    base: &str,
    enabled: bool,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> OperationExtension {
    let base = format!("{base}/{KEY}");
    let empty = OperationExtension { base: base.clone(), ..Default::default() };
    let object = match extensions.get(KEY) {
        Some(_) if !enabled => return empty,
        None => return empty,
        Some(Value::Object(object)) => object,
        Some(other) => {
            diagnostics.push(OpenApiDiagnostic::warn(
                base,
                format!("`{KEY}` must be an object, found {}; the operation's Schematic metadata is ignored", kind(other)),
            ));
            return OperationExtension { rejected: true, ..empty };
        }
    };
    report_unknown(object, &OPERATION_MEMBERS, &base, diagnostics);

    let mut members = Members { object, base: base.clone(), diagnostics };
    let success = members.decode("success", &[], |shapes: &Vec<SuccessShape>| validate_success_shapes(shapes));
    let errors = members.decode("errors", &[], |entries: &Vec<ErrorEntry>| validate_error_entries(entries));
    let no_declared_errors = members.decode("no_declared_errors", &[], |_: &bool| Ok(()));
    OperationExtension {
        rejected: false,
        success_rejected: object.contains_key("success") && success.is_none(),
        errors_rejected: (object.contains_key("errors") && errors.is_none())
            || (object.contains_key("no_declared_errors") && no_declared_errors.is_none()),
        request: members.decode("request", &REQUEST_NULLS, validate_request),
        response: members.decode("response", &[], |_: &ResponseShape| Ok(())),
        success,
        headers: members.headers("headers").unwrap_or_default(),
        rate_limit: members.decode("rate_limit", &[], |limit: &RateLimit| limit.validate().map_err(|e| e.to_string())),
        retry_guidance: members.decode("retry_guidance", &[], |guidance: &RetryGuidance| {
            guidance.validate().map_err(|e| e.to_string())
        }),
        errors,
        no_declared_errors,
        cross_origin: members.decode("cross_origin", &[], validate_origin),
        auth: members.auth("auth"),
        env_mapping: members.env_mapping("env_mapping"),
        path_template: members.decode("path_template", &[], |template: &String| nonempty(template, "path template")),
        params: members.params("params"),
        base,
    }
}

/// One extension object's members and where they live.
struct Members<'a> {
    object: &'a Map<String, Value>,
    base: String,
    diagnostics: &'a mut Vec<OpenApiDiagnostic>,
}

impl Members<'_> {
    fn at(&self, key: &str) -> String {
        format!("{}/{key}", self.base)
    }

    fn reject(&mut self, pointer: String, key: &str, problem: impl std::fmt::Display) {
        self.diagnostics.push(OpenApiDiagnostic::warn(pointer, format!("malformed `{KEY}.{key}`: {problem}; the member is ignored")));
    }

    /// Member `key` as a `T` that passes `check`. An explicit `null` is
    /// rejected unless its relative position matches one of `nulls`
    /// (`*` matches any one token).
    fn decode<T: DeserializeOwned>(
        &mut self,
        key: &str,
        nulls: &[&str],
        check: impl FnOnce(&T) -> Result<(), String>,
    ) -> Option<T> {
        let value = self.object.get(key)?;
        if let Some(position) = disallowed_null(value, &mut Vec::new(), nulls) {
            let pointer = format!("{}{position}", self.at(key));
            self.reject(pointer, key, "explicit null");
            return None;
        }
        let decoded = match serde_json::from_value::<T>(value.clone()) {
            Ok(decoded) => decoded,
            Err(error) => {
                self.reject(self.at(key), key, error);
                return None;
            }
        };
        if let Err(problem) = check(&decoded) {
            self.reject(self.at(key), key, problem);
            return None;
        }
        Some(decoded)
    }

    /// `[name, value]` pairs with valid, case-insensitively distinct names;
    /// an empty value is a legal header value.
    fn headers(&mut self, key: &str) -> Option<Vec<(String, String)>> {
        self.decode(key, &[], |headers: &Vec<(String, String)>| {
            for (index, (name, _)) in headers.iter().enumerate() {
                header_name(name, "header name")?;
                if headers[..index].iter().any(|(other, _)| other.eq_ignore_ascii_case(name)) {
                    return Err(format!("header `{name}` is listed more than once"));
                }
            }
            Ok(())
        })
    }

    /// An [`EnvMapping`]: the writer's four legacy nulls mean none, an
    /// unknown sub-key is INFO, and every listed name is nonempty.
    fn env_mapping(&mut self, key: &str) -> Option<EnvMapping> {
        if let Some(Value::Object(object)) = self.object.get(key) {
            report_unknown(object, &ENV_MAPPING_MEMBERS, &self.at(key), self.diagnostics);
        }
        self.decode(key, &ENV_MAPPING_NULLS, |mapping: &EnvMapping| {
            let lists = [&mapping.bearer_token, &mapping.basic_user, &mapping.basic_pass, &mapping.oauth_client_id,
                &mapping.oauth_client_secret, &mapping.oauth_redirect_uri];
            for name in lists.into_iter().flatten().chain(mapping.api_key.as_ref().map(|key| &key.names)).flat_map(|list| list.names()) {
                nonempty(name, "environment variable name")?;
            }
            if let Some(api_key) = &mapping.api_key {
                header_name(&api_key.header, "API-key header")?;
            }
            Ok(())
        })
    }

    /// `{<scheme>: {"bearer": true} | {"value_prefix": string}}`; any other
    /// entry shape rejects the whole member, since no other auth field may be
    /// carried here (R54).
    fn auth(&mut self, key: &str) -> AuthDetails {
        let Some(value) = self.object.get(key) else { return vec![] };
        let base = self.at(key);
        let Value::Object(entries) = value else {
            self.reject(base, key, format!("expected an object, found {}", kind(value)));
            return vec![];
        };
        let mut details = Vec::new();
        for (scheme, entry) in entries {
            let pointer = format!("{base}/{}", escape(scheme));
            let detail = match entry.as_object().map(|object| object.iter().collect::<Vec<_>>()).as_deref() {
                // An entry naming no detail adds nothing.
                Some([]) => continue,
                Some([(name, Value::Bool(true))]) if *name == "bearer" => AuthDetail::Bearer,
                Some([(name, Value::String(prefix))]) if *name == "value_prefix" => AuthDetail::ValuePrefix(prefix.clone()),
                _ => {
                    self.reject(
                        pointer,
                        key,
                        "an entry holds exactly `\"bearer\": true` or a string `value_prefix`, and no other auth field",
                    );
                    return vec![];
                }
            };
            details.push((scheme.clone(), detail, pointer));
        }
        details
    }

    /// `{<location>: {<wire name>: {secret?, rust_name?}}}`. An unknown
    /// location or entry key is INFO; any malformed entry rejects the member.
    fn params(&mut self, key: &str) -> Vec<ParamDetail> {
        let Some(value) = self.object.get(key) else { return vec![] };
        let base = self.at(key);
        let Value::Object(locations) = value else {
            self.reject(base, key, format!("expected an object, found {}", kind(value)));
            return vec![];
        };
        let mut details = Vec::new();
        for (location, entries) in locations {
            let location_pointer = format!("{base}/{}", escape(location));
            let Some(location) = ["query", "header", "cookie", "path"].into_iter().find(|known| known == location) else {
                self.diagnostics.push(OpenApiDiagnostic::info(location_pointer, format!("unknown parameter location `{location}` is ignored")));
                continue;
            };
            let Value::Object(entries) = entries else {
                self.reject(location_pointer, key, format!("expected an object, found {}", kind(entries)));
                return vec![];
            };
            for (name, entry) in entries {
                let pointer = format!("{location_pointer}/{}", escape(name));
                let Some(entry) = entry.as_object().filter(|entry| !entry.is_empty()) else {
                    self.reject(pointer, key, "a parameter entry is a nonempty object");
                    return vec![];
                };
                report_unknown(entry, &["secret", "rust_name"], &pointer, self.diagnostics);
                let secret = match entry.get("secret") {
                    None => None,
                    Some(Value::Bool(secret)) => Some(*secret),
                    Some(other) => {
                        self.reject(format!("{pointer}/secret"), key, format!("`secret` is a boolean, found {}", kind(other)));
                        return vec![];
                    }
                };
                let rust_name = match entry.get("rust_name") {
                    None => None,
                    Some(Value::String(rust_name)) if valid_rust_name(rust_name) => Some(rust_name.clone()),
                    Some(other) => {
                        self.reject(format!("{pointer}/rust_name"), key, format!("`rust_name` is a Rust identifier, found {other}"));
                        return vec![];
                    }
                };
                details.push(ParamDetail { location, name: name.clone(), secret, rust_name, pointer });
            }
        }
        details
    }
}

/// A form body has fields with distinct nonempty names, and a URL-encoded
/// body has no file field.
fn validate_request(request: &ApiRequest) -> Result<(), String> {
    let (fields, url_encoded) = match request {
        ApiRequest::FormData { fields } => (fields, false),
        ApiRequest::UrlEncoded { fields } => (fields, true),
        _ => return Ok(()),
    };
    if fields.is_empty() {
        return Err("a form body declares at least one field".into());
    }
    for (index, field) in fields.iter().enumerate() {
        nonempty(&field.name, "form field name")?;
        if fields[..index].iter().any(|other| other.name == field.name) {
            return Err(format!("form field `{}` is declared more than once", field.name));
        }
        if url_encoded && matches!(field.kind, FormFieldKind::File { .. } | FormFieldKind::Files { .. }) {
            return Err(format!("URL-encoded field `{}` cannot be a file", field.name));
        }
        if let FormFieldKind::Files { min: Some(min), max: Some(max), .. } = field.kind
            && min > max
        {
            return Err(format!("form field `{}` requires at least {min} but at most {max} files", field.name));
        }
    }
    Ok(())
}

fn nonempty(text: &str, what: &str) -> Result<(), String> {
    if text.is_empty() { Err(format!("{what} is empty")) } else { Ok(()) }
}

fn header_name(name: &str, what: &str) -> Result<(), String> {
    if valid_header_name(name) { Ok(()) } else { Err(format!("{what} `{name}` is not a valid header name")) }
}

/// Reports every key of `object` outside `known` as INFO.
fn report_unknown(object: &Map<String, Value>, known: &[&str], base: &str, diagnostics: &mut Vec<OpenApiDiagnostic>) {
    for key in object.keys().filter(|key| !known.contains(&key.as_str())) {
        diagnostics.push(OpenApiDiagnostic::info(
            format!("{base}/{}", escape(key)),
            format!("unknown `{KEY}` member `{key}` is ignored"),
        ));
    }
}

/// The relative pointer of the first `null` in `value` whose position no
/// pattern of `allowed` matches.
fn disallowed_null(value: &Value, position: &mut Vec<String>, allowed: &[&str]) -> Option<String> {
    let found = |position: &[String]| position.iter().map(|token| format!("/{}", escape(token))).collect::<String>();
    match value {
        Value::Null => {
            let matches = allowed.iter().any(|pattern| {
                let tokens: Vec<&str> = pattern.split('/').collect();
                tokens.len() == position.len() && tokens.iter().zip(position.iter()).all(|(token, at)| *token == "*" || token == at)
            });
            (!matches).then(|| found(position))
        }
        Value::Array(items) => items.iter().enumerate().find_map(|(index, item)| {
            position.push(index.to_string());
            let hit = disallowed_null(item, position, allowed);
            position.pop();
            hit
        }),
        Value::Object(members) => members.iter().find_map(|(key, member)| {
            position.push(key.clone());
            let hit = disallowed_null(member, position, allowed);
            position.pop();
            hit
        }),
        _ => None,
    }
}

fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}
