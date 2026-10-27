//! AsyncAPI import pipeline.
//!
//! Reads an AsyncAPI 2.x or 3.0 document into a `WebSocketApi` and a
//! `ModelCatalog`, and persists both, with a report, through the staged
//! publication every generator output uses
//! ([`MutationPlan`](crate::output::publish::MutationPlan)).
//!
//! The version decides how directions are read. A 3.0 document is read from
//! the client application's point of view, as the
//! [exporter](crate::export::asyncapi) writes it: `send` is a client
//! message, `receive` a server message, both is bidirectional, and the
//! endpoint role (`x-schematic-role`) never changes that. A 2.x document's
//! `publish`/`subscribe` follow `--ws-role`, as they always have.
//!
//! Every malformed standard field is a located ERROR and the import fails
//! with nothing written; a malformed Schematic extension is a WARN and reads
//! as absent. See `schematic/docs/io/asyncapi.md`.

mod extensions;
mod reader;
mod v3;

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use schematic_define::models::{ModelCatalog, ModelDef, PrimitiveType, TypeAlias, TypeRef};
use schematic_define::openapi::import::{DiagnosticSeverity, OpenApiDiagnostic};
use schematic_define::websocket::{
    ConnectionLifecycle, EndpointRole, MessageDirection, MessageSchema, MessageSelector, WebSocketApi,
    WebSocketEndpoint, WebSocketEndpointHints,
};
use schematic_define::{ApiKeyLocation, AuthStrategy, EndpointParams, ParamDef, ParamStyle, QueryParamType, Schema};
use serde_json::{Map, Value};

use crate::errors::GeneratorError;
use crate::export::asyncapi::{
    X_BEARER_HEADER, X_FALLBACK, X_HEARTBEAT, X_PARAM, X_ROLE, X_SELECTORS, X_VALUE_PREFIX, placeholders,
};
use crate::output::prepare_export_output;
use reader::{Reader, child};

/// Supported generation role for AsyncAPI websocket mappings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsRole {
    /// Generate messages from a websocket client perspective.
    Client,
    /// Generate messages from a websocket server perspective.
    Server,
    /// Generate bidirectional message directions.
    Both,
}

/// Options for AsyncAPI import.
#[derive(Debug)]
pub struct AsyncImportOptions {
    /// Path to AsyncAPI input document (YAML or JSON).
    pub input: String,
    /// Override API name.
    pub api_name: Option<String>,
    /// Override module path.
    pub module_path: Option<String>,
    /// The endpoint role of a channel without `x-schematic-role`. For a 2.x
    /// document it also maps `publish`/`subscribe` to directions.
    pub ws_role: WsRole,
    /// Output directory for the persisted definition, models, and report.
    pub output: String,
    /// Import and report without writing anything.
    pub dry_run: bool,
    /// Fail when warnings or errors are emitted.
    pub strict: bool,
    /// Print diagnostics and import internals.
    pub verbose: bool,
}

/// The AsyncAPI major version a document declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncApiVersion {
    /// 2.x: `servers.*.url`, `channels.<address>.publish`/`subscribe`.
    V2,
    /// 3.0: `servers.*.host`/`pathname`, `channels.*.address`, `operations`.
    V3,
}

/// Result of a successful AsyncAPI import.
#[derive(Debug)]
pub struct AsyncImportResult {
    /// Imported websocket API name.
    pub api_name: String,
    /// The module path the outputs are named by.
    pub module_path: String,
    /// The version the document declared.
    pub version: AsyncApiVersion,
    /// Endpoint count extracted from channels.
    pub endpoint_count: usize,
    /// Total message schemas extracted from all channels.
    pub message_count: usize,
    /// Imported model count from components/schemas.
    pub model_count: usize,
    /// INFO and WARN diagnostics; an ERROR fails the import instead.
    pub diagnostics: Vec<OpenApiDiagnostic>,
    /// Imported websocket API model.
    pub ws_api: WebSocketApi,
    /// Imported model catalog.
    pub models: ModelCatalog,
    /// The files written, in order; empty for a dry run.
    pub outputs: Vec<PathBuf>,
}

/// The persisted definition's file name: the `WebSocketApi` as JSON.
#[must_use]
pub fn definition_file_name(module_path: &str) -> String {
    format!("{module_path}.definition.json")
}

/// The persisted model catalog's file name: the `ModelCatalog` as JSON.
#[must_use]
pub fn models_file_name(module_path: &str) -> String {
    format!("{module_path}.models.json")
}

/// The import report's file name.
#[must_use]
pub fn report_file_name(module_path: &str) -> String {
    format!("{module_path}_asyncapi_import.txt")
}

/// Reads back the definition and model catalog an import persisted in `dir`.
///
/// ## Errors
///
/// [`GeneratorError::ParseError`] when either file is missing or does not
/// deserialize.
pub fn read_import_output(dir: &Path, module_path: &str) -> Result<(WebSocketApi, ModelCatalog), GeneratorError> {
    let read = |name: String| {
        let path = dir.join(name);
        fs::read_to_string(&path)
            .map_err(|e| GeneratorError::ParseError(format!("Failed to read '{}': {e}", path.display())))
            .map(|text| (path, text))
    };
    let (path, text) = read(definition_file_name(module_path))?;
    let api = serde_json::from_str(&text)
        .map_err(|e| GeneratorError::ParseError(format!("'{}' is not a WebSocket definition: {e}", path.display())))?;
    let (path, text) = read(models_file_name(module_path))?;
    let models = serde_json::from_str(&text)
        .map_err(|e| GeneratorError::ParseError(format!("'{}' is not a model catalog: {e}", path.display())))?;
    Ok((api, models))
}

/// Imports the AsyncAPI document at `options.input` and, unless it is a dry
/// run, persists the definition, model catalog, and report under
/// `options.output`.
///
/// ## Errors
///
/// [`GeneratorError::ParseError`] when the file cannot be read or parsed,
/// any ERROR diagnostic is reported, or strict mode rejects a WARN;
/// [`GeneratorError::ConfigError`] for a module path that is not a file-name
/// component; [`GeneratorError::UnsafeOutput`] or
/// [`GeneratorError::PublicationFailed`] when the outputs cannot be written.
/// Nothing is written unless the import succeeds.
pub fn run_import_asyncapi(options: &AsyncImportOptions) -> Result<AsyncImportResult, GeneratorError> {
    let content = fs::read_to_string(&options.input)
        .map_err(|e| GeneratorError::ParseError(format!("Failed to read '{}': {}", options.input, e)))?;
    let mut result = import_asyncapi_text(&content, options)?;
    if !options.dry_run {
        result.outputs = persist(&result, Path::new(&options.output))?;
    }
    Ok(result)
}

/// Imports an AsyncAPI document from its text; writes nothing.
/// `options.input` names the document in error messages only.
///
/// ## Errors
///
/// As [`run_import_asyncapi`], less the output errors.
pub fn import_asyncapi_text(content: &str, options: &AsyncImportOptions) -> Result<AsyncImportResult, GeneratorError> {
    let doc = reader::parse_document(content, &options.input)?;
    let mut import = Import::new(&doc, options.ws_role);
    let mapped = import.document(options);
    let diagnostics = import.reader.diagnostics;

    if options.verbose {
        for diag in &diagnostics {
            eprintln!("[{}] {} - {}", severity_label(diag.severity), diag.location, diag.message);
        }
    }
    if diagnostics.iter().any(OpenApiDiagnostic::is_error) {
        return Err(GeneratorError::ParseError(format!(
            "AsyncAPI import of '{}' failed ({} error(s)); nothing was written:{}",
            options.input,
            diagnostics.iter().filter(|d| d.is_error()).count(),
            diagnostics
                .iter()
                .map(|d| format!("\n  [{}] {}: {}", severity_label(d.severity), d.location, d.message))
                .collect::<String>()
        )));
    }
    if options.strict && diagnostics.iter().any(OpenApiDiagnostic::is_warn) {
        return Err(GeneratorError::ParseError(format!(
            "Strict mode rejected AsyncAPI import ({} warning(s), 0 error(s))",
            diagnostics.iter().filter(|d| d.is_warn()).count()
        )));
    }
    let Some(mapped) = mapped else {
        unreachable!("an unmapped document always reports an error");
    };
    let module_path = mapped.module_path;
    if module_path.is_empty() || !module_path.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(GeneratorError::ConfigError(format!(
            "module path `{module_path}` names the output files and must be letters, digits, and `_`"
        )));
    }
    let ws_api = mapped.ws_api;
    Ok(AsyncImportResult {
        api_name: ws_api.name.clone(),
        module_path,
        version: mapped.version,
        endpoint_count: ws_api.endpoints.len(),
        message_count: ws_api.endpoints.iter().map(|ep| ep.messages.len()).sum(),
        model_count: mapped.models.types.len(),
        diagnostics,
        ws_api,
        models: mapped.models,
        outputs: Vec::new(),
    })
}

fn severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Info => "INFO",
        DiagnosticSeverity::Warn => "WARN",
        DiagnosticSeverity::Error => "ERROR",
    }
}

/// Plans and publishes the three outputs; a conflict fails before any write.
fn persist(result: &AsyncImportResult, output: &Path) -> Result<Vec<PathBuf>, GeneratorError> {
    let serialize = |value: Result<String, serde_json::Error>| {
        value
            .map(|text| text + "\n")
            .map_err(|e| GeneratorError::CodeGenError(format!("Failed to serialize the AsyncAPI import: {e}")))
    };
    let module = &result.module_path;
    let files = vec![
        (definition_file_name(module), serialize(serde_json::to_string_pretty(&result.ws_api))?),
        (models_file_name(module), serialize(serde_json::to_string_pretty(&result.models))?),
        (report_file_name(module), import_report(result)),
    ];
    // Import outputs are never deleted: a partial set of names is planned.
    let plan = prepare_export_output(output, files, false, |_| false)?;
    let report = plan.publish()?;
    Ok(report.applied.iter().map(|mutation| mutation.path().to_path_buf()).collect())
}

fn import_report(result: &AsyncImportResult) -> String {
    let mut content = String::new();
    content.push_str(&format!("api_name={}\n", result.api_name));
    content.push_str(&format!("endpoint_count={}\n", result.endpoint_count));
    content.push_str(&format!("message_count={}\n", result.message_count));
    content.push_str(&format!("model_count={}\n", result.model_count));
    content.push_str("diagnostics:\n");
    if result.diagnostics.is_empty() {
        content.push_str("  - none\n");
    }
    for diag in &result.diagnostics {
        content.push_str(&format!("  - [{}] {}: {}\n", diag.severity, diag.location, diag.message));
    }
    content
}

/// What a document maps to before the outcome is decided.
struct Mapped {
    version: AsyncApiVersion,
    module_path: String,
    ws_api: WebSocketApi,
    models: ModelCatalog,
}

/// The `components` maps a document's references resolve into.
#[derive(Clone, Copy, Default)]
struct Components<'a> {
    messages: Option<&'a Map<String, Value>>,
    schemas: Option<&'a Map<String, Value>>,
    security_schemes: Option<&'a Map<String, Value>>,
}

/// One message as read from its message object.
struct MessageParts {
    name: String,
    type_name: String,
    description: Option<String>,
    selectors: Option<Vec<MessageSelector>>,
    fallback: bool,
}

impl MessageParts {
    fn into_schema(self, direction: MessageDirection) -> MessageSchema {
        let mut schema = MessageSchema::new(self.name, direction, Schema::new(self.type_name));
        schema.description = self.description;
        schema.selectors = self.selectors;
        schema.fallback = self.fallback;
        schema
    }
}

/// The state of one import.
struct Import<'a> {
    doc: &'a Value,
    reader: Reader,
    version: AsyncApiVersion,
    components: Components<'a>,
    ws_role: WsRole,
    /// Payload schema names messages referenced; the catalog lists each,
    /// even one no component schema declares.
    payload_refs: BTreeSet<String>,
    /// Constant handshake headers, from every channel's headers binding.
    headers: Vec<(String, String)>,
}

impl<'a> Import<'a> {
    fn new(doc: &'a Value, ws_role: WsRole) -> Self {
        Self {
            doc,
            reader: Reader::default(),
            version: AsyncApiVersion::V3,
            components: Components::default(),
            ws_role,
            payload_refs: BTreeSet::new(),
            headers: Vec::new(),
        }
    }

    fn document(&mut self, options: &AsyncImportOptions) -> Option<Mapped> {
        let Some(root) = self.doc.as_object() else {
            self.reader.error("#", "Expected an AsyncAPI document object");
            return None;
        };
        let version = self.reader.required_string(root.get("asyncapi"), "#/asyncapi")?;
        self.version = match parse_version(version) {
            Some(version) => version,
            None => {
                self.reader.error("#/asyncapi", format!("Unsupported AsyncAPI version '{version}'; supported: 2.x and 3.0.x"));
                return None;
            }
        };

        let components = self.reader.object(root.get("components"), "#/components");
        let component = |reader: &mut Reader, key: &str| {
            components.and_then(|components| reader.object(components.get(key), &child("#/components", key)))
        };
        self.components = Components {
            messages: component(&mut self.reader, "messages"),
            schemas: component(&mut self.reader, "schemas"),
            security_schemes: component(&mut self.reader, "securitySchemes"),
        };

        let info = self.reader.required_object(root.get("info"), "#/info");
        let title = match &options.api_name {
            Some(name) => {
                info.map(|info| self.reader.string(info.get("title"), "#/info/title"));
                Some(name.clone())
            }
            None => info.and_then(|info| self.reader.required_string(info.get("title"), "#/info/title")).map(to_pascal_case),
        };
        let api_version = info.and_then(|info| self.reader.required_string(info.get("version"), "#/info/version"));
        let description = info.and_then(|info| self.reader.string(info.get("description"), "#/info/description"));
        let (docs_holder, docs_at) = match self.version {
            AsyncApiVersion::V2 => (Some(root), "#/externalDocs"),
            AsyncApiVersion::V3 => (info, "#/info/externalDocs"),
        };
        let docs_url = docs_holder
            .and_then(|holder| self.reader.object(holder.get("externalDocs"), docs_at))
            .and_then(|docs| self.reader.required_string(docs.get("url"), &child(docs_at, "url")))
            .map(String::from);

        let (base_url, auth) = self.server(root);

        let endpoints = match self.reader.required_object(root.get("channels"), "#/channels") {
            Some(channels) => match self.version {
                AsyncApiVersion::V2 => self.v2_channels(channels),
                AsyncApiVersion::V3 => self.v3_channels(channels, root),
            },
            None => Vec::new(),
        };
        if endpoints.is_empty() {
            self.reader.warn("#/channels", "No channels were found in AsyncAPI document");
        }

        let api_name = title?;
        let module_path = options.module_path.clone().unwrap_or_else(|| to_snake_case(&api_name));
        let models = self.models();
        let env_auth = default_env_auth(&module_path, &auth);
        let env_mapping = schematic_define::RestApi::legacy_env_mapping_for(&auth, &env_auth);
        Some(Mapped {
            version: self.version,
            module_path,
            ws_api: WebSocketApi {
                name: api_name,
                description: description.unwrap_or_default().to_string(),
                base_url,
                docs_url,
                auth,
                auth_policy: None,
                env_auth,
                env_mapping: Some(env_mapping),
                headers: std::mem::take(&mut self.headers),
                version: api_version.map(String::from),
                endpoints,
                runtime: None,
            },
            models,
        })
    }

    /// The base URL and auth of the first server. Security comes from what
    /// the server applies, never from the first declared component scheme.
    fn server(&mut self, root: &Map<String, Value>) -> (String, AuthStrategy) {
        let fallback = || ("ws://localhost".to_string(), AuthStrategy::None);
        let servers = self.reader.object(root.get("servers"), "#/servers");
        let Some((name, server)) = servers.and_then(|servers| servers.iter().next()) else {
            if servers.is_some() || root.get("servers").is_none() {
                self.reader.warn("#/servers", "No servers entry found; defaulting base_url to ws://localhost");
            }
            return fallback();
        };
        let count = servers.map_or(0, Map::len);
        if count > 1 {
            self.reader.info("#/servers", format!("Using server '{name}'; {} other server(s) are not imported", count - 1));
        }
        let at = child("#/servers", name);
        let Some(server) = self.reader.required_object(Some(server), &at) else { return fallback() };

        let base_url = match self.version {
            AsyncApiVersion::V2 => {
                let protocol = self.reader.string(server.get("protocol"), &child(&at, "protocol"));
                match self.reader.required_string(server.get("url"), &child(&at, "url")) {
                    Some(url) if url.starts_with("ws://") || url.starts_with("wss://") => url.to_string(),
                    Some(url) => {
                        let scheme = protocol.filter(|p| *p == "ws" || *p == "wss").unwrap_or("ws");
                        format!("{scheme}://{}", url.trim_start_matches("//"))
                    }
                    None => String::new(),
                }
            }
            AsyncApiVersion::V3 => {
                let host = self.reader.required_string(server.get("host"), &child(&at, "host"));
                let protocol_at = child(&at, "protocol");
                let protocol = match self.reader.required_string(server.get("protocol"), &protocol_at) {
                    Some(protocol @ ("ws" | "wss")) => Some(protocol),
                    Some(other) => {
                        self.reader.error(&protocol_at, format!("Protocol '{other}' is not `ws` or `wss`"));
                        None
                    }
                    None => None,
                };
                let pathname_at = child(&at, "pathname");
                let pathname = match self.reader.string(server.get("pathname"), &pathname_at) {
                    Some(path) if !path.starts_with('/') => {
                        self.reader.error(&pathname_at, "A pathname must start with `/`");
                        ""
                    }
                    Some(path) => path.trim_end_matches('/'),
                    None => "",
                };
                format!("{}://{}{pathname}", protocol.unwrap_or("ws"), host.unwrap_or_default())
            }
        };
        (base_url, self.applied_security(server, &at))
    }

    fn applied_security(&mut self, server: &Map<String, Value>, at: &str) -> AuthStrategy {
        let at = child(at, "security");
        let Some(items) = self.reader.array(server.get("security"), &at) else {
            if server.get("security").is_none() {
                self.reader.info(&at, "The server applies no security; the API is anonymous");
            }
            return AuthStrategy::None;
        };
        let Some(first) = items.first() else { return AuthStrategy::None };
        if items.len() > 1 {
            self.reader.info(&at, format!("Using the first applied security entry; {} other(s) are not imported", items.len() - 1));
        }
        let first_at = child(&at, "0");
        let Some(entry) = self.reader.required_object(Some(first), &first_at) else { return AuthStrategy::None };
        let schemes = self.components.security_schemes;
        let resolved = match self.version {
            // 2.x applies a requirement object `{scheme_name: [scopes]}`.
            AsyncApiVersion::V2 => match entry.keys().next() {
                Some(name) => match schemes.and_then(|schemes| schemes.get(name)) {
                    Some(scheme) => Some((child("#/components/securitySchemes", name), scheme)),
                    None => {
                        self.reader.error(&first_at, format!("Security scheme '{name}' is not declared in components.securitySchemes"));
                        None
                    }
                },
                None => {
                    self.reader.error(&first_at, "Expected a security requirement naming a scheme");
                    None
                }
            },
            // 3.0 applies a scheme object or a reference to one.
            AsyncApiVersion::V3 => {
                if entry.contains_key("$ref") {
                    self.reader
                        .local_ref(first, "#/components/securitySchemes/", schemes, &first_at)
                        .map(|(name, scheme)| (child("#/components/securitySchemes", &name), scheme))
                } else {
                    Some((first_at.clone(), first))
                }
            }
        };
        match resolved {
            Some((scheme_at, scheme)) => self.security_scheme(scheme, &scheme_at),
            None => AuthStrategy::None,
        }
    }

    fn security_scheme(&mut self, scheme: &Value, at: &str) -> AuthStrategy {
        let Some(scheme) = self.reader.required_object(Some(scheme), at) else { return AuthStrategy::None };
        let Some(scheme_type) = self.reader.required_string(scheme.get("type"), &child(at, "type")) else {
            return AuthStrategy::None;
        };
        match scheme_type {
            "httpApiKey" | "apiKey" => {
                let in_at = child(at, "in");
                // A 2.x `apiKey` puts the key in the user or password of the
                // connection; only a named header form (the legacy reading)
                // or an `httpApiKey` maps to an API key.
                let location = match scheme_type {
                    "httpApiKey" => self.reader.required_string(scheme.get("in"), &in_at),
                    _ => match self.reader.string(scheme.get("in"), &in_at) {
                        Some("user" | "password") => {
                            self.reader.warn(at, "An API key in the connection's user or password is not supported; falling back to AuthStrategy::None");
                            return AuthStrategy::None;
                        }
                        other => other.or(Some("header")),
                    },
                };
                let Some(name) = self.reader.required_string(scheme.get("name"), &child(at, "name")).map(String::from) else {
                    return AuthStrategy::None;
                };
                match location {
                    Some("header") => AuthStrategy::ApiKey {
                        header: name,
                        value_prefix: extensions::scheme_string(&mut self.reader, scheme.get(X_VALUE_PREFIX), &child(at, X_VALUE_PREFIX)),
                    },
                    Some("query") => AuthStrategy::ApiKeyParam { name, location: ApiKeyLocation::Query },
                    Some("cookie") => AuthStrategy::ApiKeyParam { name, location: ApiKeyLocation::Cookie },
                    Some(other) => {
                        self.reader.error(&in_at, format!("API key location '{other}' is not `header`, `query`, or `cookie`"));
                        AuthStrategy::None
                    }
                    None => AuthStrategy::None,
                }
            }
            "http" => {
                let inner = self
                    .reader
                    .required_string(scheme.get("scheme"), &child(at, "scheme"))
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                match inner.as_str() {
                    // AsyncAPI names no Basic credential variables.
                    "basic" => AuthStrategy::Basic {
                        username_env: schematic_define::EnvList::new(vec![]),
                        password_env: schematic_define::EnvList::new(vec![]),
                    },
                    "bearer" => AuthStrategy::BearerToken {
                        header: extensions::scheme_string(&mut self.reader, scheme.get(X_BEARER_HEADER), &child(at, X_BEARER_HEADER)),
                    },
                    "" => AuthStrategy::None,
                    other => {
                        self.reader.warn(at, format!("Unsupported HTTP auth scheme '{other}'; falling back to AuthStrategy::None"));
                        AuthStrategy::None
                    }
                }
            }
            other => {
                self.reader.warn(at, format!("Unsupported security scheme type '{other}'; falling back to AuthStrategy::None"));
                AuthStrategy::None
            }
        }
    }

    fn v2_channels(&mut self, channels: &'a Map<String, Value>) -> Vec<WebSocketEndpoint> {
        let mut endpoints = Vec::new();
        for (path, channel) in channels {
            let at = child("#/channels", path);
            let Some(channel) = self.reader.required_object(Some(channel), &at) else { continue };
            let mut messages = Vec::new();
            for operation_name in ["publish", "subscribe"] {
                let op_at = child(&at, operation_name);
                let Some(operation) = self.reader.object(channel.get(operation_name), &op_at) else { continue };
                let direction = map_direction(operation_name, self.ws_role);
                let Some(message) = operation.get("message") else {
                    self.reader.warn(&op_at, "Operation has no message definition");
                    continue;
                };
                let message_at = child(&op_at, "message");
                let variants: Vec<(String, &Value)> = match message.get("oneOf") {
                    Some(one_of) => self
                        .reader
                        .array(Some(one_of), &child(&message_at, "oneOf"))
                        .map(|items| {
                            items.iter().enumerate().map(|(i, v)| (child(&child(&message_at, "oneOf"), &i.to_string()), v)).collect()
                        })
                        .unwrap_or_default(),
                    None => vec![(message_at, message)],
                };
                for (variant_at, variant) in variants {
                    let default_name = format!(
                        "{}{}",
                        to_pascal_case(&channel_id_from_path(path)),
                        to_pascal_case(operation_name)
                    );
                    if let Some(parts) = self.message(variant, &variant_at, &default_name) {
                        messages.push(parts.into_schema(direction));
                    }
                }
            }
            let params = self.channel_params(channel, &at, path);
            let mut endpoint = WebSocketEndpoint {
                id: channel_id_from_path(path),
                path: path.clone(),
                description: self.reader.string(channel.get("description"), &child(&at, "description")).unwrap_or_default().to_string(),
                params,
                role: extensions::role(&mut self.reader, channel.get(X_ROLE), &child(&at, X_ROLE), endpoint_role(self.ws_role)),
                lifecycle: ConnectionLifecycle::default(),
                messages: dedup_messages(messages),
                runtime: None,
            };
            self.finish_endpoint(&mut endpoint, channel, &at);
            endpoints.push(endpoint);
        }
        endpoints
    }

    /// Reads the heartbeat hint and settles fallback markers.
    fn finish_endpoint(&mut self, endpoint: &mut WebSocketEndpoint, channel: &Map<String, Value>, at: &str) {
        if let Some(heartbeat) = extensions::heartbeat(&mut self.reader, channel.get(X_HEARTBEAT), &child(at, X_HEARTBEAT)) {
            endpoint.runtime = Some(WebSocketEndpointHints { heartbeat: Some(heartbeat), ..WebSocketEndpointHints::default() });
        }
        for message in &mut endpoint.messages {
            if message.fallback && message.selectors.as_ref().is_some_and(|s| !s.is_empty()) {
                self.reader.warn(
                    &child(&child(at, "messages"), &message.name),
                    "A fallback message declares selectors; it is not imported as the fallback",
                );
                message.fallback = false;
            }
        }
        // At most one fallback per direction; more is never a silent pick.
        for (direction, label) in [(MessageDirection::Client, "client"), (MessageDirection::Server, "server")] {
            let receives = |message: &MessageSchema| {
                message.direction == direction || message.direction == MessageDirection::Bidirectional
            };
            let fallbacks: Vec<&str> =
                endpoint.messages.iter().filter(|m| m.fallback && receives(m)).map(|m| m.name.as_str()).collect();
            if fallbacks.len() > 1 {
                self.reader.warn(
                    at,
                    format!("{} {label} messages are marked as the fallback ({}); none is imported as the fallback", fallbacks.len(), fallbacks.join(", ")),
                );
                for message in endpoint.messages.iter_mut().filter(|m| receives(m)) {
                    message.fallback = false;
                }
            }
        }
    }

    /// Reads one message: a `$ref` into `components.messages` or an inline
    /// message object. `default_name` names an inline message without `name`.
    fn message(&mut self, message: &'a Value, at: &str, default_name: &str) -> Option<MessageParts> {
        let (object, at, key_name) = if message.get("$ref").is_some() {
            let (key, target) = self.reader.local_ref(message, "#/components/messages/", self.components.messages, at)?;
            let target_at = child("#/components/messages", &key);
            let object = self.reader.required_object(Some(target), &target_at)?;
            (object, target_at, Some(key))
        } else {
            (self.reader.required_object(Some(message), at)?, at.to_string(), None)
        };
        let declared_name = self.reader.nonempty_string(object.get("name"), &child(&at, "name"));
        let name = match (self.version, key_name) {
            // 2.x names a referenced message by its component key.
            (AsyncApiVersion::V2, Some(key)) => to_pascal_case(&key),
            (AsyncApiVersion::V2, None) => declared_name.map_or_else(|| default_name.to_string(), to_pascal_case),
            // 3.0 names it by `name`, else by its key in the channel.
            (AsyncApiVersion::V3, _) => declared_name.map_or_else(|| default_name.to_string(), String::from),
        };
        let description = self.reader.string(object.get("description"), &child(&at, "description")).map(String::from);
        let payload = object.get("payload");
        let type_name = self.payload_type(payload, &child(&at, "payload"));
        let selectors = match extensions::selectors(&mut self.reader, object.get(X_SELECTORS), &child(&at, X_SELECTORS)) {
            Some(declared) => Some(declared),
            None => {
                let (inferred, ignored) = payload_selectors(payload, self.components.schemas.unwrap_or(&Map::new()));
                if !ignored.is_empty() {
                    self.reader.info(
                        &child(&at, "payload"),
                        format!("Non-string `const` properties are not selectors: {}", ignored.into_iter().collect::<Vec<_>>().join(", ")),
                    );
                }
                (!inferred.is_empty()).then_some(inferred)
            }
        };
        let fallback = extensions::fallback(&mut self.reader, object.get(X_FALLBACK), &child(&at, X_FALLBACK));
        Some(MessageParts { name, type_name, description, selectors, fallback })
    }

    /// The payload's type name: the component schema it references, or
    /// `JsonValue` for an inline or absent schema.
    fn payload_type(&mut self, payload: Option<&Value>, at: &str) -> String {
        let fallback = "JsonValue".to_string();
        let Some(payload) = payload else {
            match self.version {
                AsyncApiVersion::V2 => self.reader.info(at, "Message payload not defined; defaulting to JsonValue"),
                AsyncApiVersion::V3 => self.reader.warn(at, "Message payload not defined; defaulting to JsonValue"),
            }
            return fallback;
        };
        let Some(object) = self.reader.object(Some(payload), at) else { return fallback };
        if object.contains_key("$ref") {
            let Some((name, _)) = self.reader.local_ref(payload, "#/components/schemas/", self.components.schemas, at) else {
                return fallback;
            };
            let name = match self.version {
                AsyncApiVersion::V2 => to_pascal_case(&name),
                AsyncApiVersion::V3 => name,
            };
            self.payload_refs.insert(name.clone());
            return name;
        }
        if object.is_empty() {
            return fallback;
        }
        if ["oneOf", "anyOf", "allOf"].iter().any(|key| object.contains_key(*key)) {
            self.reader.warn(at, "Combinator payload (oneOf/anyOf/allOf) mapped to JsonValue");
        } else {
            self.reader.info(at, "Inline payload schema mapped to JsonValue");
        }
        fallback
    }

    /// The handshake parameters of a channel: typed path parameters from
    /// `parameters`, query parameters from the WebSocket binding's `query`
    /// schema, and constant headers from its `headers` schema, which join
    /// the API's headers.
    fn channel_params(&mut self, channel: &'a Map<String, Value>, at: &str, address: &str) -> EndpointParams {
        let mut params = EndpointParams { path: self.path_params(channel, at, address), ..EndpointParams::default() };
        let bindings_at = child(at, "bindings");
        let Some(bindings) = self.reader.object(channel.get("bindings"), &bindings_at) else { return params };
        let ws_at = child(&bindings_at, "ws");
        let Some(ws) = self.reader.object(bindings.get("ws"), &ws_at) else { return params };
        if let Some((properties, required, query_at)) = self.object_schema(ws.get("query"), &child(&ws_at, "query")) {
            for (name, property) in properties {
                let property_at = child(&child(&query_at, "properties"), name);
                if let Some(param) = self.query_param(name, property, &property_at, required.contains(name)) {
                    params.query.push(param);
                }
            }
        }
        if let Some((properties, _, headers_at)) = self.object_schema(ws.get("headers"), &child(&ws_at, "headers")) {
            for (name, property) in properties {
                let property_at = child(&child(&headers_at, "properties"), name);
                self.header(name, property, &property_at);
            }
        }
        params
    }

    /// An object schema's properties and required names.
    fn object_schema(
        &mut self,
        schema: Option<&'a Value>,
        at: &str,
    ) -> Option<(&'a Map<String, Value>, BTreeSet<String>, String)> {
        let schema = self.reader.object(schema, at)?;
        match self.reader.string(schema.get("type"), &child(at, "type")) {
            None | Some("object") => {}
            Some(other) => self.reader.error(&child(at, "type"), format!("Expected an object schema, found type '{other}'")),
        }
        let properties = self.reader.object(schema.get("properties"), &child(at, "properties"))?;
        let mut required = BTreeSet::new();
        if let Some(names) = self.reader.array(schema.get("required"), &child(at, "required")) {
            for (index, name) in names.iter().enumerate() {
                if let Some(name) = self.reader.string(Some(name), &child(&child(at, "required"), &index.to_string())) {
                    required.insert(name.to_string());
                }
            }
        }
        Some((properties, required, at.to_string()))
    }

    fn path_params(&mut self, channel: &Map<String, Value>, at: &str, address: &str) -> Vec<ParamDef> {
        let parameters_at = child(at, "parameters");
        let Some(parameters) = self.reader.object(channel.get("parameters"), &parameters_at) else { return Vec::new() };
        let placeholders = placeholders(address);
        let mut read = BTreeMap::new();
        let mut declared = false;
        for (name, parameter) in parameters {
            let param_at = child(&parameters_at, name);
            let Some(object) = self.reader.required_object(Some(parameter), &param_at) else { continue };
            if object.contains_key("$ref") {
                self.reader.error(&param_at, "Parameter references are not supported; declare the parameter on the channel");
                continue;
            }
            if !placeholders.contains(name) {
                self.reader.warn(&param_at, format!("Parameter '{name}' has no `{{{name}}}` placeholder in the address; it is not imported"));
                continue;
            }
            let description = self.reader.string(object.get("description"), &child(&param_at, "description")).map(String::from);
            let extension = extensions::param(&mut self.reader, object.get(X_PARAM), &child(&param_at, X_PARAM));
            // A 2.x parameter may carry a schema; its scalar type is read.
            let schema_type = match self.version {
                AsyncApiVersion::V2 => {
                    let schema_at = child(&param_at, "schema");
                    let schema = self.reader.object(object.get("schema"), &schema_at);
                    match schema.and_then(|schema| self.reader.string(schema.get("type"), &child(&schema_at, "type"))) {
                        Some("integer") => Some(QueryParamType::Integer),
                        Some("number") => Some(QueryParamType::Number),
                        Some("boolean") => Some(QueryParamType::Boolean),
                        _ => None,
                    }
                }
                AsyncApiVersion::V3 => None,
            };
            declared |= extension.is_some() || description.is_some() || schema_type.is_some();
            read.insert(name.clone(), (description, extension, schema_type));
        }
        // Without any typed or described parameter every placeholder stays an
        // untyped string, as it was declared.
        if !declared {
            return Vec::new();
        }
        let mut params = Vec::new();
        for name in placeholders {
            let (description, extension, schema_type) = read.remove(&name).unwrap_or_default();
            let extension = extension.unwrap_or_default();
            let param_type = extension.param_type.or(schema_type).unwrap_or(QueryParamType::String);
            let mut param = ParamDef::path(name, param_type);
            param.description = description;
            param.style = extension.style.unwrap_or(ParamStyle::Simple);
            param.explode = extension.explode;
            param.rust_name = extension.rust_name;
            param.secret = extension.secret;
            param.pager_managed = extension.pager_managed;
            param.deprecated = extension.deprecated;
            params.push(param);
        }
        params
    }

    fn query_param(&mut self, name: &str, property: &Value, at: &str, required: bool) -> Option<ParamDef> {
        let object = self.reader.required_object(Some(property), at)?;
        let param_type = self.param_type(property, at)?;
        let mut param = ParamDef::new(name, param_type);
        param.required = required;
        param.description = self.reader.string(object.get("description"), &child(at, "description")).map(String::from);
        param.deprecated = match object.get("deprecated") {
            None => false,
            Some(Value::Bool(flag)) => *flag,
            Some(other) => {
                self.reader.error(&child(at, "deprecated"), format!("Expected a boolean, found {}", reader::kind(other)));
                false
            }
        };
        if let Some(extension) = extensions::param(&mut self.reader, object.get(X_PARAM), &child(at, X_PARAM)) {
            if extension.param_type.is_some() {
                self.reader.info(&child(&child(at, X_PARAM), "type"), "A query parameter's type comes from its schema; this is ignored");
            }
            param.style = extension.style.unwrap_or(ParamStyle::Form);
            param.explode = extension.explode;
            param.rust_name = extension.rust_name;
            param.secret = extension.secret;
            param.pager_managed = extension.pager_managed;
        }
        Some(param)
    }

    /// The parameter type a property schema declares: `enum` is an
    /// enumeration, `type` a scalar or an array of `items`, and a schema
    /// with neither is any JSON value.
    fn param_type(&mut self, schema: &Value, at: &str) -> Option<QueryParamType> {
        let object = self.reader.required_object(Some(schema), at)?;
        if let Some(values) = self.reader.array(object.get("enum"), &child(at, "enum")) {
            let mut names = Vec::new();
            for (index, value) in values.iter().enumerate() {
                names.push(self.reader.string(Some(value), &child(&child(at, "enum"), &index.to_string()))?.to_string());
            }
            return Some(QueryParamType::Enum(names));
        }
        let type_at = child(at, "type");
        Some(match self.reader.string(object.get("type"), &type_at) {
            None if object.get("type").is_none() => QueryParamType::Json,
            None => return None,
            Some("string") => QueryParamType::String,
            Some("integer") => QueryParamType::Integer,
            Some("number") => QueryParamType::Number,
            Some("boolean") => QueryParamType::Boolean,
            Some("array") => {
                let items_at = child(at, "items");
                let Some(items) = object.get("items") else {
                    self.reader.error(&items_at, "An array parameter must declare `items`");
                    return None;
                };
                QueryParamType::Array(Box::new(self.param_type(items, &items_at)?))
            }
            Some("object") => {
                self.reader.info(&type_at, "An object parameter is imported as any JSON value");
                QueryParamType::Json
            }
            Some(other) => {
                self.reader.error(&type_at, format!("Unsupported parameter type '{other}'"));
                return None;
            }
        })
    }

    /// A handshake header: a string `const` (or one-value `enum`) joins the
    /// API's constant headers; anything else cannot be sent and is dropped.
    fn header(&mut self, name: &str, property: &Value, at: &str) {
        let Some(object) = self.reader.required_object(Some(property), at) else { return };
        let constant = match (object.get("const"), object.get("enum")) {
            (Some(Value::String(value)), _) => Some(value.clone()),
            (None, Some(Value::Array(values))) if values.len() == 1 => values[0].as_str().map(String::from),
            _ => None,
        };
        let Some(value) = constant else {
            self.reader.warn(at, format!("Header '{name}' is not a constant string; WebSocket handshakes send only constant headers, so it is not imported"));
            return;
        };
        match self.headers.iter().find(|(existing, _)| existing.eq_ignore_ascii_case(name)) {
            Some((_, existing)) if *existing != value => self.reader.warn(
                at,
                format!("Header '{name}' is '{value}' here and '{existing}' on another channel; the first value is kept"),
            ),
            Some(_) => {}
            None => self.headers.push((name.to_string(), value)),
        }
    }

    /// The model catalog: the `JsonValue` fallback, every component schema,
    /// and every referenced payload name.
    fn models(&self) -> ModelCatalog {
        let mut seen = HashSet::new();
        let mut models = vec![ModelDef::Alias(TypeAlias {
            name: "JsonValue".to_string(),
            description: Some("Fallback type for unknown or inline payload schemas".to_string()),
            target: TypeRef::Primitive(PrimitiveType::Json),
            discriminator: None,
        })];
        seen.insert("JsonValue".to_string());
        for (name, schema) in self.components.schemas.into_iter().flatten() {
            let model_name = match self.version {
                AsyncApiVersion::V2 => to_pascal_case(name),
                AsyncApiVersion::V3 => name.clone(),
            };
            if seen.insert(model_name.clone()) {
                models.push(ModelDef::Alias(TypeAlias {
                    name: model_name,
                    description: schema.get("description").and_then(Value::as_str).map(String::from),
                    target: TypeRef::Primitive(PrimitiveType::Json),
                    discriminator: None,
                }));
            }
        }
        for name in &self.payload_refs {
            if seen.insert(name.clone()) {
                models.push(ModelDef::Alias(TypeAlias {
                    name: name.clone(),
                    description: Some("Referenced payload schema".to_string()),
                    target: TypeRef::Primitive(PrimitiveType::Json),
                    discriminator: None,
                }));
            }
        }
        ModelCatalog { module_path: None, types: models }
    }
}

/// `2.x.y` or `3.0.y`; anything else is unsupported.
fn parse_version(version: &str) -> Option<AsyncApiVersion> {
    let parts: Vec<&str> = version.split('.').collect();
    let numeric = parts.len() == 3 && parts.iter().all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
    match (numeric, parts.first(), parts.get(1)) {
        (true, Some(&"2"), _) => Some(AsyncApiVersion::V2),
        (true, Some(&"3"), Some(&"0")) => Some(AsyncApiVersion::V3),
        _ => None,
    }
}

/// Selectors for a message: every top-level payload property whose schema is
/// a string `const` (for example `kind: {const: resp}`), following `$ref`
/// into `components/schemas` and merging `allOf` parts. A property declared
/// without `const` in one part keeps a `const` another part gives it.
///
/// Also returns the properties whose `const` is not a string: a selector
/// matches strings only, so they select nothing.
fn payload_selectors(
    payload: Option<&Value>,
    component_schemas: &Map<String, Value>,
) -> (Vec<MessageSelector>, BTreeSet<String>) {
    let mut consts = BTreeMap::new();
    let mut ignored = BTreeSet::new();
    collect_payload_consts(payload, component_schemas, &mut consts, &mut ignored, 0);
    let selectors = consts
        .into_iter()
        .filter_map(|(field, value)| value.map(|value| MessageSelector::new(field, value)))
        .collect();
    (selectors, ignored)
}

fn collect_payload_consts(
    schema: Option<&Value>,
    component_schemas: &Map<String, Value>,
    consts: &mut BTreeMap<String, Option<String>>,
    ignored: &mut BTreeSet<String>,
    depth: usize,
) {
    // A bound on `$ref` chains, so a cyclic reference ends.
    const MAX_DEPTH: usize = 16;
    let Some(schema) = schema else { return };
    if depth > MAX_DEPTH {
        return;
    }
    if let Some(name) = schema.get("$ref").and_then(Value::as_str).and_then(parse_ref_name) {
        collect_payload_consts(component_schemas.get(&name), component_schemas, consts, ignored, depth + 1);
    }
    for part in schema.get("allOf").and_then(Value::as_array).into_iter().flatten() {
        collect_payload_consts(Some(part), component_schemas, consts, ignored, depth + 1);
    }
    for (field, property) in schema.get("properties").and_then(Value::as_object).into_iter().flatten() {
        match property.get("const") {
            Some(Value::String(value)) => {
                consts.insert(field.clone(), Some(value.clone()));
            }
            other => {
                if other.is_some() {
                    ignored.insert(field.clone());
                }
                consts.entry(field.clone()).or_insert(None);
            }
        }
    }
}

fn default_env_auth(module_path: &str, auth: &AuthStrategy) -> Vec<String> {
    // Basic carries its own environment lists; `env_auth` is never its source.
    if matches!(auth, AuthStrategy::None | AuthStrategy::Basic { .. }) {
        return vec![];
    }
    let upper = module_path.replace('-', "_").to_ascii_uppercase();
    vec![format!("{}_API_KEY", upper)]
}

fn parse_ref_name(reference: &str) -> Option<String> {
    reference.rsplit('/').next().map(String::from)
}

fn channel_id_from_path(path: &str) -> String {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return "RootChannel".to_string();
    }

    let mut id = String::new();
    for part in trimmed.split('/') {
        let sanitized = part.trim_matches('{').trim_matches('}');
        id.push_str(&to_pascal_case(sanitized));
    }

    if id.is_empty() {
        "RootChannel".to_string()
    } else {
        format!("{}Channel", id)
    }
}

/// The endpoint role the CLI's `--ws-role` selects: `server` generates a host.
fn endpoint_role(role: WsRole) -> EndpointRole {
    match role {
        WsRole::Client => EndpointRole::Client,
        WsRole::Server => EndpointRole::Host,
        WsRole::Both => EndpointRole::Both,
    }
}

/// The 2.x direction of a `publish` or `subscribe` message for `role`.
fn map_direction(operation_name: &str, role: WsRole) -> MessageDirection {
    match role {
        WsRole::Both => MessageDirection::Bidirectional,
        WsRole::Client => {
            if operation_name == "publish" {
                MessageDirection::Client
            } else {
                MessageDirection::Server
            }
        }
        WsRole::Server => {
            if operation_name == "publish" {
                MessageDirection::Server
            } else {
                MessageDirection::Client
            }
        }
    }
}

fn dedup_messages(messages: Vec<MessageSchema>) -> Vec<MessageSchema> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for message in messages {
        let key = format!(
            "{}:{}:{}",
            message.name, message.direction, message.schema.type_name
        );
        if seen.insert(key) {
            deduped.push(message);
        }
    }

    deduped
}

fn to_pascal_case(input: &str) -> String {
    let mut out = String::new();
    let mut upper_next = true;

    for c in input.chars() {
        if c.is_ascii_alphanumeric() {
            if upper_next {
                out.push(c.to_ascii_uppercase());
                upper_next = false;
            } else {
                out.push(c);
            }
        } else {
            upper_next = true;
        }
    }

    if out.is_empty() {
        "Unnamed".to_string()
    } else {
        out
    }
}

fn to_snake_case(input: &str) -> String {
    let mut out = String::new();
    for (i, c) in input.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_selectors_keep_string_consts_across_parts_and_end_on_cycles() {
        let schemas: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "base": {"properties": {"kind": {"const": "resp"}, "msg": {"type": "string"}}},
            "loop": {"$ref": "#/components/schemas/loop"},
        }))
        .unwrap();
        let payload = serde_json::json!({"allOf": [
            {"properties": {"msg": {"const": "pong"}}},
            {"$ref": "#/components/schemas/base"},
            {"$ref": "#/components/schemas/loop"},
            {"properties": {"code": {"const": 200}}},
        ]});
        let (selectors, ignored) = payload_selectors(Some(&payload), &schemas);
        assert_eq!(selectors, [MessageSelector::new("kind", "resp"), MessageSelector::new("msg", "pong")]);
        assert_eq!(ignored.into_iter().collect::<Vec<_>>(), ["code"]);
        assert!(payload_selectors(None, &schemas).0.is_empty());
    }

    #[test]
    fn versions_dispatch_on_major_and_reject_everything_else() {
        assert_eq!(parse_version("2.2.0"), Some(AsyncApiVersion::V2));
        assert_eq!(parse_version("2.6.0"), Some(AsyncApiVersion::V2));
        assert_eq!(parse_version("3.0.0"), Some(AsyncApiVersion::V3));
        for unsupported in ["3.1.0", "1.2.0", "3.0", "3.0.x", "", "4.0.0", "v3.0.0"] {
            assert_eq!(parse_version(unsupported), None, "{unsupported}");
        }
    }

    fn options(dir: &Path, module: &str) -> AsyncImportOptions {
        AsyncImportOptions {
            input: dir.join("in.yaml").to_string_lossy().to_string(),
            api_name: None,
            module_path: Some(module.to_string()),
            ws_role: WsRole::Client,
            output: dir.to_string_lossy().to_string(),
            dry_run: true,
            strict: false,
            verbose: false,
        }
    }

    #[test]
    fn imports_minimal_asyncapi_document() {
        let yaml = r#"
asyncapi: "2.2.0"
info:
  title: test-ws
  version: "1.0.0"
servers:
  local:
    url: ws://localhost:5555
channels:
  /events:
    subscribe:
      message:
        $ref: '#/components/messages/TestEvent'
components:
  messages:
    TestEvent:
      payload:
        $ref: '#/components/schemas/TestEventPayload'
  schemas:
    TestEventPayload:
      type: object
"#;
        let temp_dir = tempfile::TempDir::new().unwrap();
        let result = import_asyncapi_text(yaml, &options(temp_dir.path(), "test_ws")).unwrap();
        assert_eq!(result.version, AsyncApiVersion::V2);
        assert_eq!(result.endpoint_count, 1);
        assert_eq!(result.ws_api.base_url, "ws://localhost:5555");
        assert!(result.message_count >= 1);
        assert!(result.model_count >= 1);
    }

    #[test]
    fn maps_http_api_key_auth_scheme() {
        let yaml = r#"
asyncapi: "2.2.0"
info:
  title: auth-ws
  version: "1.0.0"
servers:
  local:
    url: ws://localhost:5555
    security:
      - apiKeyAuth: []
channels:
  /ws:
    publish:
      message:
        name: Ping
        payload:
          type: object
components:
  securitySchemes:
    apiKeyAuth:
      type: apiKey
      in: header
      name: API-KEY
"#;
        let temp_dir = tempfile::TempDir::new().unwrap();
        let result = import_asyncapi_text(yaml, &options(temp_dir.path(), "auth_ws")).unwrap();
        assert!(matches!(
            result.ws_api.auth,
            AuthStrategy::ApiKey { ref header, .. } if header == "API-KEY"
        ));
    }
}
