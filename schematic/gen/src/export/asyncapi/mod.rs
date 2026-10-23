//! AsyncAPI 3.0 export of WebSocket API definitions.
//!
//! One document per [`WebSocketApi`], written from the client application's
//! point of view for every endpoint role: a client message is a `send`
//! operation, a server message a `receive` operation, and a bidirectional
//! message both. The definition's own role travels in `x-schematic-role`.
//!
//! Standard AsyncAPI cannot say which inbound message a frame is, carry a
//! heartbeat hint, or describe a handshake parameter's Rust-side details, so
//! those travel in Schematic extensions that third-party tools ignore. The
//! [`asyncapi_import`](crate::asyncapi_import) reader is the guarantee that
//! they round-trip. See `schematic/docs/io/asyncapi.md` for the supported
//! subset and what it leaves out.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use schematic_define::websocket::{MessageDirection, MessageSchema, WebSocketApi, WebSocketEndpoint};
use schematic_define::{ApiKeyLocation, AuthStrategy, ParamDef, ParamStyle, QueryParamType};
use schematic_definitions::registry::ApiRegistration;
use serde_json::{Map, Value, json};

use crate::diagnostics::{Feature, Output, Severity, ValidationDiagnostic};
use crate::errors::GeneratorError;
use crate::output::prepare_export_output;
use crate::output::publish::MutationPlan;

/// The AsyncAPI version every exported document declares.
pub const ASYNCAPI_VERSION: &str = "3.0.0";

/// The key of the one server an exported document declares.
pub const SERVER_KEY: &str = "default";

/// The version of the WebSocket channel binding the exporter writes.
pub const WS_BINDING_VERSION: &str = "0.1.0";

/// Message extension: the inbound selectors, `[{field, value}]`.
pub const X_SELECTORS: &str = "x-schematic-selectors";
/// Message extension: `true` on the fallback message of its direction.
pub const X_FALLBACK: &str = "x-schematic-fallback";
/// Channel extension: the endpoint role, `client`, `host`, or `both`.
pub const X_ROLE: &str = "x-schematic-role";
/// Channel extension: the heartbeat hint, `{interval_secs, timeout_secs?}`.
pub const X_HEARTBEAT: &str = "x-schematic-heartbeat";
/// Parameter extension: the parameter details standard AsyncAPI cannot
/// express (`type` on a path parameter, `style`, `explode`, `rust_name`,
/// `secret`, `deprecated`, `pager_managed`).
pub const X_PARAM: &str = "x-schematic-param";
/// Security scheme extension: the prefix written before an API key value.
pub const X_VALUE_PREFIX: &str = "x-schematic-value-prefix";
/// Security scheme extension: the header that carries a bearer token
/// instead of `Authorization`.
pub const X_BEARER_HEADER: &str = "x-schematic-bearer-header";

/// One exported document and what the export could not carry.
#[derive(Debug, Clone)]
pub struct AsyncApiExport {
    /// The AsyncAPI 3.0 document.
    pub document: Value,
    /// Declarations the document cannot carry, as warnings. An error-level
    /// diagnostic never reaches here; [`export_asyncapi`] fails instead.
    pub diagnostics: Vec<ValidationDiagnostic>,
}

/// One WebSocket API and the file its document is written to.
#[derive(Debug, Clone)]
pub struct AsyncApiDocument {
    /// The output file name, `{module}.json`, or `{module}.{api}.json` when
    /// several WebSocket APIs share a module.
    pub file_name: String,
    /// The definition.
    pub api: WebSocketApi,
}

/// Exports `api` as an AsyncAPI 3.0 document.
///
/// ## Errors
///
/// [`GeneratorError::Validation`] for output `asyncapi` when the definition
/// cannot become a valid document: a base URL that is not `ws://` or
/// `wss://`, a name AsyncAPI keys cannot hold, two messages of one endpoint
/// sharing a name, or a `responds_with` naming no message of its endpoint.
pub fn export_asyncapi(api: &WebSocketApi) -> Result<AsyncApiExport, GeneratorError> {
    let mut exporter = Exporter { api, diagnostics: Vec::new() };
    let document = exporter.document();
    let dangling = dangling_refs(&document);
    for reference in dangling {
        exporter.error(None, format!("the exported document has a dangling reference `{reference}`"));
    }
    if exporter.diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(GeneratorError::Validation { output: Output::AsyncApi, diagnostics: exporter.diagnostics });
    }
    Ok(AsyncApiExport { document, diagnostics: exporter.diagnostics })
}

/// [`export_asyncapi`] rendered as pretty JSON with a trailing newline, the
/// committed artifact form.
///
/// ## Errors
///
/// As [`export_asyncapi`].
pub fn render_asyncapi(api: &WebSocketApi) -> Result<String, GeneratorError> {
    let export = export_asyncapi(api)?;
    serde_json::to_string_pretty(&export.document)
        .map(|text| text + "\n")
        .map_err(|e| GeneratorError::CodeGenError(format!("Failed to serialize AsyncAPI document: {e}")))
}

/// The WebSocket APIs of `registrations`, each with its document file name.
///
/// The file stem is the registered module. When several WebSocket APIs share
/// a module, each stem gains the API's snake-case name (`{module}.{api}`), so
/// no two documents collide and each keeps one application identity.
#[must_use]
pub fn asyncapi_documents(registrations: &[ApiRegistration]) -> Vec<AsyncApiDocument> {
    let websocket: Vec<&ApiRegistration> = registrations.iter().filter(|entry| entry.websocket().is_some()).collect();
    let mut per_module: BTreeMap<&str, usize> = BTreeMap::new();
    for entry in &websocket {
        *per_module.entry(entry.module).or_default() += 1;
    }
    websocket
        .into_iter()
        .filter_map(|entry| {
            let ws = entry.websocket()?;
            let file_name = if per_module[entry.module] > 1 {
                format!("{}.{}.json", entry.module, snake_case(entry.name))
            } else {
                format!("{}.json", entry.module)
            };
            Some(AsyncApiDocument { file_name, api: (ws.define)() })
        })
        .collect()
}

/// Plans the publication of every WebSocket API's document under `root`.
///
/// A regular `*.json` file under `root` counts as generator-owned; `complete`
/// deletes such files no API produces. Nothing touches the filesystem until
/// the returned plan is published.
///
/// ## Errors
///
/// [`GeneratorError::Validation`] when any document cannot be exported (no
/// plan is made), or [`GeneratorError::UnsafeOutput`] listing every path
/// conflict.
pub fn prepare_asyncapi_output(
    root: &Path,
    documents: &[AsyncApiDocument],
    complete: bool,
) -> Result<(MutationPlan, Vec<ValidationDiagnostic>), GeneratorError> {
    let mut files = Vec::new();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    for document in documents {
        match export_asyncapi(&document.api) {
            Ok(export) => {
                warnings.extend(export.diagnostics);
                let text = serde_json::to_string_pretty(&export.document).map_err(|e| {
                    GeneratorError::CodeGenError(format!("Failed to serialize AsyncAPI document: {e}"))
                })?;
                files.push((document.file_name.clone(), text + "\n"));
            }
            Err(GeneratorError::Validation { diagnostics, .. }) => errors.extend(diagnostics),
            Err(other) => return Err(other),
        }
    }
    if !errors.is_empty() {
        errors.extend(warnings);
        return Err(GeneratorError::Validation { output: Output::AsyncApi, diagnostics: errors });
    }
    let plan = prepare_export_output(root, files, complete, |name| name.ends_with(".json"))?;
    Ok((plan, warnings))
}

/// Every local `$ref` in `document` that does not resolve inside it, in
/// document order. A reference that is not local (`#/...`) is dangling too:
/// exported documents are self-contained.
#[must_use]
pub fn dangling_refs(document: &Value) -> Vec<String> {
    fn walk(root: &Value, value: &Value, dangling: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                if let Some(Value::String(reference)) = map.get("$ref") {
                    let resolves = reference.strip_prefix('#').is_some_and(|pointer| root.pointer(pointer).is_some());
                    if !resolves {
                        dangling.push(reference.clone());
                    }
                }
                for child in map.values() {
                    walk(root, child, dangling);
                }
            }
            Value::Array(items) => items.iter().for_each(|item| walk(root, item, dangling)),
            _ => {}
        }
    }
    let mut dangling = Vec::new();
    walk(document, document, &mut dangling);
    dangling
}

/// The JSON pointer token for `key` (RFC 6901).
pub(crate) fn pointer_token(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// The component message key of `message` on `endpoint`: endpoint ids are
/// unique per API and message names per endpoint, so the pair is unique per
/// document even when endpoints reuse a message name.
#[must_use]
pub fn component_message_key(endpoint: &WebSocketEndpoint, message: &MessageSchema) -> String {
    format!("{}.{}", endpoint.id, message.name)
}

/// An endpoint's messages in export order: its own messages, then its
/// lifecycle `open`, `close`, and `keepalive` messages, which export as
/// ordinary messages.
#[must_use]
pub fn exported_messages(endpoint: &WebSocketEndpoint) -> Vec<&MessageSchema> {
    let lifecycle = &endpoint.lifecycle;
    endpoint
        .messages
        .iter()
        .chain(lifecycle.open.iter())
        .chain(lifecycle.close.iter())
        .chain(lifecycle.keepalive.iter())
        .collect()
}

struct Exporter<'a> {
    api: &'a WebSocketApi,
    diagnostics: Vec<ValidationDiagnostic>,
}

impl Exporter<'_> {
    fn diagnostic(&mut self, severity: Severity, endpoint: Option<&str>, message: String) {
        self.diagnostics.push(ValidationDiagnostic::new(
            severity,
            &self.api.name,
            endpoint,
            Feature::WebSocketDefinition,
            Output::AsyncApi,
            message,
        ));
    }

    fn error(&mut self, endpoint: Option<&str>, message: String) {
        self.diagnostic(Severity::Error, endpoint, message);
    }

    fn warn(&mut self, endpoint: Option<&str>, message: String) {
        self.diagnostic(Severity::Warning, endpoint, message);
    }

    fn document(&mut self) -> Value {
        let api = self.api;
        let mut info = Map::new();
        info.insert("title".into(), json!(api.name));
        info.insert("version".into(), json!(api.version.as_deref().unwrap_or("1.0.0")));
        if !api.description.is_empty() {
            info.insert("description".into(), json!(api.description));
        }
        if let Some(url) = &api.docs_url {
            info.insert("externalDocs".into(), json!({ "url": url }));
        }

        let mut components = Map::new();
        let scheme = self.security_scheme();
        let mut server = self.server();
        if let Some((name, scheme)) = scheme {
            server.insert("security".into(), json!([{ "$ref": format!("#/components/securitySchemes/{name}") }]));
            components.insert("securitySchemes".into(), json!({ name: scheme }));
        }

        let mut channels = Map::new();
        let mut operations = Map::new();
        let mut messages = Map::new();
        let mut schemas = Map::new();
        let mut endpoint_ids = BTreeSet::new();
        for endpoint in &api.endpoints {
            if !endpoint_ids.insert(endpoint.id.as_str()) {
                self.error(Some(&endpoint.id), "two endpoints share this id".into());
            }
            self.check_key(Some(&endpoint.id), "endpoint id", &endpoint.id);
            channels.insert(endpoint.id.clone(), self.channel(endpoint));
            self.endpoint_messages(endpoint, &mut operations, &mut messages, &mut schemas);
        }
        if !messages.is_empty() {
            components.insert("messages".into(), Value::Object(messages));
        }
        if !schemas.is_empty() {
            components.insert("schemas".into(), Value::Object(schemas));
        }

        let mut document = Map::new();
        document.insert("asyncapi".into(), json!(ASYNCAPI_VERSION));
        document.insert("info".into(), Value::Object(info));
        document.insert("defaultContentType".into(), json!("application/json"));
        document.insert("servers".into(), json!({ SERVER_KEY: server }));
        document.insert("channels".into(), Value::Object(channels));
        document.insert("operations".into(), Value::Object(operations));
        if !components.is_empty() {
            document.insert("components".into(), Value::Object(components));
        }
        Value::Object(document)
    }

    /// `servers.default`: the protocol, host, and base path of `base_url`.
    fn server(&mut self) -> Map<String, Value> {
        let base_url = &self.api.base_url;
        let mut server = Map::new();
        let Some((protocol, rest)) = base_url.split_once("://").filter(|(p, _)| *p == "ws" || *p == "wss") else {
            self.error(None, format!("base URL `{base_url}` is not a `ws://` or `wss://` URL"));
            return server;
        };
        let (host, pathname) = match rest.find('/') {
            Some(index) => rest.split_at(index),
            None => (rest, ""),
        };
        if host.is_empty() {
            self.error(None, format!("base URL `{base_url}` names no host"));
        }
        server.insert("host".into(), json!(host));
        server.insert("protocol".into(), json!(protocol));
        // A trailing `/` alone is the root; the endpoint address supplies the
        // path, so a base path is written only when it adds segments.
        let pathname = pathname.trim_end_matches('/');
        if !pathname.is_empty() {
            server.insert("pathname".into(), json!(pathname));
        }
        server
    }

    /// The component security scheme `auth` maps to, by name.
    fn security_scheme(&mut self) -> Option<(&'static str, Value)> {
        match &self.api.auth {
            AuthStrategy::None => None,
            AuthStrategy::ApiKey { header, value_prefix } => {
                let mut scheme = json!({ "type": "httpApiKey", "in": "header", "name": header });
                if let Some(prefix) = value_prefix {
                    scheme[X_VALUE_PREFIX] = json!(prefix);
                }
                Some(("apiKey", scheme))
            }
            AuthStrategy::BearerToken { header } => {
                let mut scheme = json!({ "type": "http", "scheme": "bearer" });
                if let Some(header) = header {
                    scheme[X_BEARER_HEADER] = json!(header);
                }
                Some(("bearer", scheme))
            }
            AuthStrategy::Basic { .. } => Some(("basic", json!({ "type": "http", "scheme": "basic" }))),
            AuthStrategy::ApiKeyParam { name, location } => {
                let location = match location {
                    ApiKeyLocation::Query => "query",
                    ApiKeyLocation::Cookie => "cookie",
                    _ => {
                        self.warn(None, "an API key location outside the supported subset; the document declares no security".into());
                        return None;
                    }
                };
                Some(("apiKey", json!({ "type": "httpApiKey", "in": location, "name": name })))
            }
            other => {
                self.warn(None, format!("auth `{}` has no AsyncAPI form in the supported subset; the document declares no security", auth_label(other)));
                None
            }
        }
    }

    fn channel(&mut self, endpoint: &WebSocketEndpoint) -> Value {
        let mut channel = Map::new();
        channel.insert("address".into(), json!(endpoint.path));
        if !endpoint.description.is_empty() {
            channel.insert("description".into(), json!(endpoint.description));
        }

        let mut channel_messages = Map::new();
        for message in exported_messages(endpoint) {
            let key = component_message_key(endpoint, message);
            channel_messages.insert(
                message.name.clone(),
                json!({ "$ref": format!("#/components/messages/{}", pointer_token(&key)) }),
            );
        }
        if !channel_messages.is_empty() {
            channel.insert("messages".into(), Value::Object(channel_messages));
        }

        let parameters = self.path_parameters(endpoint);
        if !parameters.is_empty() {
            channel.insert("parameters".into(), Value::Object(parameters));
        }

        let mut ws = Map::new();
        if let Some(query) = self.query_binding(endpoint) {
            ws.insert("query".into(), query);
        }
        if let Some(headers) = self.headers_binding(endpoint) {
            ws.insert("headers".into(), headers);
        }
        if !ws.is_empty() {
            ws.insert("bindingVersion".into(), json!(WS_BINDING_VERSION));
            channel.insert("bindings".into(), json!({ "ws": ws }));
        }

        channel.insert(X_ROLE.into(), json!(endpoint.role.to_string()));
        if let Some(heartbeat) = endpoint.runtime.as_ref().and_then(|hints| hints.heartbeat.as_ref()) {
            let mut hint = Map::new();
            hint.insert("interval_secs".into(), json!(heartbeat.interval_secs));
            if let Some(timeout) = heartbeat.timeout_secs {
                hint.insert("timeout_secs".into(), json!(timeout));
            }
            channel.insert(X_HEARTBEAT.into(), Value::Object(hint));
        }
        Value::Object(channel)
    }

    /// `channels.<id>.parameters`: one entry per address placeholder. An
    /// undeclared placeholder is `{}`; a declared one carries its type in
    /// [`X_PARAM`], which is how the reader tells the two apart.
    fn path_parameters(&mut self, endpoint: &WebSocketEndpoint) -> Map<String, Value> {
        let mut parameters = Map::new();
        if endpoint.params.path.is_empty() {
            for placeholder in placeholders(&endpoint.path) {
                parameters.insert(placeholder, json!({}));
            }
            return parameters;
        }
        for param in &endpoint.params.path {
            let mut object = Map::new();
            if let Some(description) = &param.description {
                object.insert("description".into(), json!(description));
            }
            let scalar = match &param.param_type {
                QueryParamType::String => "string",
                QueryParamType::Integer => "integer",
                QueryParamType::Number => "number",
                QueryParamType::Boolean => "boolean",
                _ => {
                    self.warn(Some(&endpoint.id), format!("path parameter `{}` is not a scalar; it exports as a string", param.name));
                    "string"
                }
            };
            let mut extension = param_extension(param, ParamStyle::Simple);
            extension.insert("type".into(), json!(scalar));
            object.insert(X_PARAM.into(), Value::Object(sorted(extension)));
            parameters.insert(param.name.clone(), Value::Object(object));
        }
        parameters
    }

    /// `bindings.ws.query`: an object schema with one property per query
    /// parameter, required ones listed in `required`.
    fn query_binding(&mut self, endpoint: &WebSocketEndpoint) -> Option<Value> {
        if endpoint.params.query.is_empty() {
            return None;
        }
        let mut properties = Map::new();
        let mut required = Vec::new();
        for param in &endpoint.params.query {
            let mut schema = param_type_schema(&param.param_type);
            if let Some(description) = &param.description {
                schema.insert("description".into(), json!(description));
            }
            if param.deprecated {
                schema.insert("deprecated".into(), json!(true));
            }
            let extension = param_extension(param, ParamStyle::Form);
            if !extension.is_empty() {
                schema.insert(X_PARAM.into(), Value::Object(sorted(extension)));
            }
            if param.required {
                required.push(json!(param.name));
            }
            properties.insert(param.name.clone(), Value::Object(schema));
        }
        let mut query = Map::new();
        query.insert("type".into(), json!("object"));
        query.insert("properties".into(), Value::Object(properties));
        if !required.is_empty() {
            query.insert("required".into(), Value::Array(required));
        }
        Some(Value::Object(query))
    }

    /// `bindings.ws.headers`: the API's constant headers, each a string
    /// `const`. Header and cookie parameters are not part of a WebSocket
    /// handshake the generator emits, so they are reported and left out.
    fn headers_binding(&mut self, endpoint: &WebSocketEndpoint) -> Option<Value> {
        for (location, params) in [("header", &endpoint.params.header), ("cookie", &endpoint.params.cookie)] {
            for param in params {
                self.warn(
                    Some(&endpoint.id),
                    format!("{location} parameter `{}` is not exported; WebSocket handshakes carry only constant headers", param.name),
                );
            }
        }
        if self.api.headers.is_empty() {
            return None;
        }
        let mut properties = Map::new();
        for (name, value) in &self.api.headers {
            properties.insert(name.clone(), json!({ "type": "string", "const": value }));
        }
        let required: Vec<Value> = self.api.headers.iter().map(|(name, _)| json!(name)).collect();
        Some(json!({ "type": "object", "properties": properties, "required": required }))
    }

    /// Adds `endpoint`'s component messages, payload schemas, and one
    /// operation per message and action.
    fn endpoint_messages(
        &mut self,
        endpoint: &WebSocketEndpoint,
        operations: &mut Map<String, Value>,
        messages: &mut Map<String, Value>,
        schemas: &mut Map<String, Value>,
    ) {
        let channel_ref = format!("#/channels/{}", pointer_token(&endpoint.id));
        let exported = exported_messages(endpoint);
        let mut names = BTreeSet::new();
        for message in &exported {
            if !names.insert(message.name.as_str()) {
                self.error(Some(&endpoint.id), format!("two messages share the name `{}`", message.name));
            }
            self.check_key(Some(&endpoint.id), "message name", &message.name);
            self.check_key(Some(&endpoint.id), "payload type name", &message.schema.type_name);
        }

        for message in &exported {
            let key = component_message_key(endpoint, message);
            let type_name = &message.schema.type_name;
            let mut component = Map::new();
            component.insert("name".into(), json!(message.name));
            if let Some(description) = &message.description {
                component.insert("description".into(), json!(description));
            }
            component.insert(
                "payload".into(),
                json!({ "$ref": format!("#/components/schemas/{}", pointer_token(type_name)) }),
            );
            if let Some(selectors) = &message.selectors {
                let selectors: Vec<Value> =
                    selectors.iter().map(|s| json!({ "field": s.field, "value": s.value })).collect();
                component.insert(X_SELECTORS.into(), Value::Array(selectors));
            }
            if message.fallback {
                component.insert(X_FALLBACK.into(), json!(true));
            }
            messages.insert(key, Value::Object(component));
            schemas.entry(type_name.clone()).or_insert_with(|| {
                json!({
                    "description": format!(
                        "Payload type `{type_name}`. Schematic WebSocket definitions declare no JSON Schema for payloads, so any JSON value is accepted."
                    )
                })
            });

            let message_ref = format!("{channel_ref}/messages/{}", pointer_token(&message.name));
            let actions: &[&str] = match message.direction {
                MessageDirection::Client => &["send"],
                MessageDirection::Server => &["receive"],
                MessageDirection::Bidirectional => &["send", "receive"],
            };
            for action in actions {
                let mut operation = Map::new();
                operation.insert("action".into(), json!(action));
                operation.insert("channel".into(), json!({ "$ref": channel_ref }));
                operation.insert("messages".into(), json!([{ "$ref": message_ref }]));
                if *action == "send"
                    && let Some(reply) = &message.responds_with
                {
                    if exported.iter().any(|candidate| &candidate.name == reply) {
                        operation.insert(
                            "reply".into(),
                            json!({
                                "channel": { "$ref": channel_ref },
                                "messages": [{ "$ref": format!("{channel_ref}/messages/{}", pointer_token(reply)) }],
                            }),
                        );
                    } else {
                        self.error(Some(&endpoint.id), format!("message `{}` responds with `{reply}`, which the endpoint does not declare", message.name));
                    }
                }
                operations.insert(format!("{}.{action}.{}", endpoint.id, message.name), Value::Object(operation));
            }
        }
    }

    /// AsyncAPI component, channel, and operation keys match
    /// `^[a-zA-Z0-9.\-_]+$`.
    fn check_key(&mut self, endpoint: Option<&str>, what: &str, key: &str) {
        let valid = !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
        if !valid {
            self.error(endpoint, format!("{what} `{key}` cannot be an AsyncAPI key (letters, digits, `.`, `-`, `_`)"));
        }
    }
}

fn auth_label(auth: &AuthStrategy) -> &'static str {
    match auth {
        AuthStrategy::None => "none",
        AuthStrategy::BearerToken { .. } => "bearer",
        AuthStrategy::ApiKey { .. } => "api-key",
        AuthStrategy::Basic { .. } => "basic",
        AuthStrategy::ApiKeyParam { .. } => "api-key-param",
        AuthStrategy::OAuth2(_) => "oauth2",
        _ => "unknown",
    }
}

/// The JSON Schema of a parameter type.
pub(crate) fn param_type_schema(param_type: &QueryParamType) -> Map<String, Value> {
    let value = match param_type {
        QueryParamType::String => json!({ "type": "string" }),
        QueryParamType::Integer => json!({ "type": "integer" }),
        QueryParamType::Number => json!({ "type": "number" }),
        QueryParamType::Boolean => json!({ "type": "boolean" }),
        QueryParamType::Array(items) => json!({ "type": "array", "items": param_type_schema(items) }),
        QueryParamType::Enum(values) => json!({ "type": "string", "enum": values }),
        _ => json!({}),
    };
    match value {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

/// The [`X_PARAM`] members of `param` that differ from its location's
/// defaults (`default_style`, not exploded, no Rust name, not secret).
fn param_extension(param: &ParamDef, default_style: ParamStyle) -> Map<String, Value> {
    let mut extension = Map::new();
    if param.style != default_style {
        extension.insert("style".into(), serde_json::to_value(param.style).unwrap_or(Value::Null));
    }
    if param.explode {
        extension.insert("explode".into(), json!(true));
    }
    if let Some(rust_name) = &param.rust_name {
        extension.insert("rust_name".into(), json!(rust_name));
    }
    if param.secret {
        extension.insert("secret".into(), json!(true));
    }
    if param.pager_managed {
        extension.insert("pager_managed".into(), json!(true));
    }
    // A path parameter has no schema to hold standard `deprecated`.
    if param.deprecated && default_style == ParamStyle::Simple {
        extension.insert("deprecated".into(), json!(true));
    }
    extension
}

fn sorted(map: Map<String, Value>) -> Map<String, Value> {
    map.into_iter().collect::<BTreeMap<_, _>>().into_iter().collect()
}

/// The `{name}` placeholders of an address, in order.
pub(crate) fn placeholders(address: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = address;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else { break };
        names.push(after[..end].to_string());
        rest = &after[end + 1..];
    }
    names
}

fn snake_case(name: &str) -> String {
    let mut out = String::new();
    let mut previous_lower = false;
    for c in name.chars() {
        let lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        if c.is_ascii_uppercase() {
            if previous_lower {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('_') {
            out.push('_');
        }
        previous_lower = lower;
    }
    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_are_read_in_order_and_unclosed_braces_end_the_scan() {
        assert_eq!(placeholders("/a/{x}/b/{y_z}"), ["x", "y_z"]);
        assert!(placeholders("/plain").is_empty());
        assert_eq!(placeholders("/{x}/{open"), ["x"]);
    }

    #[test]
    fn snake_case_splits_camel_case_names() {
        assert_eq!(snake_case("UnfoldedCircleCoreWs"), "unfolded_circle_core_ws");
        assert_eq!(snake_case("ElevenLabsTTS"), "eleven_labs_tts");
        assert_eq!(snake_case("a-b"), "a_b");
    }

    #[test]
    fn dangling_refs_finds_missing_and_remote_references() {
        let document = json!({
            "components": {"schemas": {"A": {}}},
            "x": [{"$ref": "#/components/schemas/A"}, {"$ref": "#/components/schemas/B"}],
            "y": {"$ref": "https://example.com/schema.json"},
        });
        assert_eq!(dangling_refs(&document), ["#/components/schemas/B", "https://example.com/schema.json"]);
    }
}
