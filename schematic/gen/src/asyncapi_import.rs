//! AsyncAPI import pipeline.
//!
//! Transforms AsyncAPI documents into `WebSocketApi` and `ModelCatalog`
//! definitions suitable for staged WebSocket code generation.

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::Path;

use biscuit_file::Yaml;
use schematic_define::models::{ModelCatalog, ModelDef, PrimitiveType, TypeAlias, TypeRef};
use schematic_define::openapi::import::{DiagnosticSeverity, OpenApiDiagnostic};
use schematic_define::websocket::{
    ConnectionLifecycle, MessageDirection, MessageSchema, WebSocketApi, WebSocketEndpoint,
};
use schematic_define::{AuthStrategy, Schema};
use serde_json::{Map, Value};

use crate::errors::GeneratorError;

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
    /// Role mapping for publish/subscribe semantics.
    pub ws_role: WsRole,
    /// Output directory for import report artifacts.
    pub output: String,
    /// Print generated artifact summary only.
    pub dry_run: bool,
    /// Fail when warnings or errors are emitted.
    pub strict: bool,
    /// Print diagnostics and import internals.
    pub verbose: bool,
}

/// Result of a successful AsyncAPI import.
#[derive(Debug)]
pub struct AsyncImportResult {
    /// Imported websocket API name.
    pub api_name: String,
    /// Endpoint count extracted from channels.
    pub endpoint_count: usize,
    /// Total message schemas extracted from all channels.
    pub message_count: usize,
    /// Imported model count from components/schemas.
    pub model_count: usize,
    /// Diagnostics produced during mapping.
    pub diagnostics: Vec<OpenApiDiagnostic>,
    /// Imported websocket API model.
    pub ws_api: WebSocketApi,
    /// Imported model catalog.
    pub models: ModelCatalog,
}

/// Run AsyncAPI import and emit a lightweight report artifact.
///
/// ## Errors
///
/// Returns `GeneratorError` when file parsing fails, strict mode rejects
/// diagnostics, or output writing fails.
pub fn run_import_asyncapi(
    options: &AsyncImportOptions,
) -> Result<AsyncImportResult, GeneratorError> {
    let doc = parse_input_document(&options.input)?;

    let mut diagnostics = Vec::new();
    validate_asyncapi_document(&doc, &mut diagnostics);

    let info = doc
        .get("info")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let api_name = options.api_name.clone().unwrap_or_else(|| {
        info.get("title")
            .and_then(Value::as_str)
            .map(to_pascal_case)
            .unwrap_or_else(|| "ImportedAsyncApi".to_string())
    });

    let module_path = options
        .module_path
        .clone()
        .unwrap_or_else(|| to_snake_case(&api_name));

    let base_url = infer_base_url(&doc, &mut diagnostics);
    let auth = infer_auth_strategy(&doc, &mut diagnostics);

    let channels = doc
        .get("channels")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let components = doc
        .get("components")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let component_messages = components
        .get("messages")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let component_schemas = components
        .get("schemas")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut payload_refs = BTreeSet::new();
    let endpoints = map_channels_to_endpoints(
        &channels,
        &component_messages,
        &mut payload_refs,
        options.ws_role,
        &mut diagnostics,
    );

    if endpoints.is_empty() {
        diagnostics.push(OpenApiDiagnostic::warn(
            "#/channels".to_string(),
            "No channels were found in AsyncAPI document".to_string(),
        ));
    }

    let models = map_component_schemas(&component_schemas, &payload_refs);

    if options.strict && diagnostics.iter().any(|d| d.is_warn() || d.is_error()) {
        let warn_count = diagnostics.iter().filter(|d| d.is_warn()).count();
        let error_count = diagnostics.iter().filter(|d| d.is_error()).count();
        return Err(GeneratorError::ParseError(format!(
            "Strict mode rejected AsyncAPI import ({} warning(s), {} error(s))",
            warn_count, error_count
        )));
    }

    let env_auth = default_env_auth(&module_path, &auth);
    let ws_api = WebSocketApi {
        name: api_name.clone(),
        description: info
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("Imported AsyncAPI WebSocket API")
            .to_string(),
        base_url,
        docs_url: None,
        auth,
        env_auth,
        version: None,
        endpoints,
        runtime: None,
    };

    let endpoint_count = ws_api.endpoints.len();
    let message_count = ws_api.endpoints.iter().map(|ep| ep.messages.len()).sum();
    let model_count = models.types.len();

    if options.verbose {
        for diag in &diagnostics {
            let level = match diag.severity {
                DiagnosticSeverity::Info => "INFO",
                DiagnosticSeverity::Warn => "WARN",
                DiagnosticSeverity::Error => "ERROR",
            };
            eprintln!("[{}] {} - {}", level, diag.location, diag.message);
        }
    }

    if !options.dry_run {
        write_import_report(
            Path::new(&options.output),
            &module_path,
            &api_name,
            endpoint_count,
            message_count,
            model_count,
            &diagnostics,
        )?;
    }

    Ok(AsyncImportResult {
        api_name,
        endpoint_count,
        message_count,
        model_count,
        diagnostics,
        ws_api,
        models,
    })
}

fn parse_input_document(path: &str) -> Result<Value, GeneratorError> {
    let content = fs::read_to_string(path)
        .map_err(|e| GeneratorError::ParseError(format!("Failed to read '{}': {}", path, e)))?;

    let trimmed = content.trim_start();
    if trimmed.starts_with('{') {
        serde_json::from_str::<Value>(&content).map_err(|e| {
            GeneratorError::ParseError(format!("Failed to parse JSON AsyncAPI '{}': {}", path, e))
        })
    } else {
        let yaml = Yaml::from_str(&content).map_err(|e| {
            GeneratorError::ParseError(format!("Failed to parse YAML AsyncAPI '{}': {}", path, e))
        })?;
        yaml.as_json().map_err(|e| {
            GeneratorError::ParseError(format!("Failed to convert YAML AsyncAPI '{}': {}", path, e))
        })
    }
}

fn validate_asyncapi_document(doc: &Value, diagnostics: &mut Vec<OpenApiDiagnostic>) {
    let version = doc.get("asyncapi").and_then(Value::as_str);
    if version.is_none() {
        diagnostics.push(OpenApiDiagnostic::error(
            "#/asyncapi".to_string(),
            "Missing required 'asyncapi' version field".to_string(),
        ));
    }

    if doc.get("channels").and_then(Value::as_object).is_none() {
        diagnostics.push(OpenApiDiagnostic::error(
            "#/channels".to_string(),
            "Missing required 'channels' object".to_string(),
        ));
    }
}

fn infer_base_url(doc: &Value, diagnostics: &mut Vec<OpenApiDiagnostic>) -> String {
    let servers = doc
        .get("servers")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    if let Some((_name, server)) = servers.into_iter().next()
        && let Some(url) = server.get("url").and_then(Value::as_str)
    {
        let normalized = if url.starts_with("ws://") || url.starts_with("wss://") {
            url.to_string()
        } else {
            format!("ws://{}", url.trim_start_matches("//"))
        };
        return normalized;
    }

    diagnostics.push(OpenApiDiagnostic::warn(
        "#/servers".to_string(),
        "No servers entry found; defaulting base_url to ws://localhost".to_string(),
    ));
    "ws://localhost".to_string()
}

fn infer_auth_strategy(doc: &Value, diagnostics: &mut Vec<OpenApiDiagnostic>) -> AuthStrategy {
    let schemes = doc
        .get("components")
        .and_then(Value::as_object)
        .and_then(|c| c.get("securitySchemes"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    for (name, scheme) in schemes {
        let scheme_type = scheme.get("type").and_then(Value::as_str).unwrap_or("");
        match scheme_type {
            "httpApiKey" | "apiKey" => {
                let header = scheme
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("X-API-Key")
                    .to_string();
                return AuthStrategy::ApiKey {
                    header,
                    value_prefix: None,
                };
            }
            "http" => {
                let inner_scheme = scheme
                    .get("scheme")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if inner_scheme == "basic" {
                    return AuthStrategy::Basic;
                }
                if inner_scheme == "bearer" {
                    return AuthStrategy::BearerToken { header: None };
                }

                diagnostics.push(OpenApiDiagnostic::warn(
                    format!("#/components/securitySchemes/{}", name),
                    format!(
                        "Unsupported HTTP auth scheme '{}'; falling back to AuthStrategy::None",
                        inner_scheme
                    ),
                ));
            }
            "" => {
                diagnostics.push(OpenApiDiagnostic::warn(
                    format!("#/components/securitySchemes/{}", name),
                    "Security scheme missing 'type'; falling back to AuthStrategy::None"
                        .to_string(),
                ));
            }
            other => {
                diagnostics.push(OpenApiDiagnostic::warn(
                    format!("#/components/securitySchemes/{}", name),
                    format!(
                        "Unsupported security scheme type '{}'; falling back to AuthStrategy::None",
                        other
                    ),
                ));
            }
        }
    }

    AuthStrategy::None
}

fn map_channels_to_endpoints(
    channels: &Map<String, Value>,
    component_messages: &Map<String, Value>,
    payload_refs: &mut BTreeSet<String>,
    role: WsRole,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Vec<WebSocketEndpoint> {
    channels
        .iter()
        .map(|(path, channel)| {
            let mut messages = Vec::new();

            if let Some(op) = channel.get("publish") {
                messages.extend(resolve_operation_messages(
                    op,
                    "publish",
                    path,
                    component_messages,
                    payload_refs,
                    role,
                    diagnostics,
                ));
            }

            if let Some(op) = channel.get("subscribe") {
                messages.extend(resolve_operation_messages(
                    op,
                    "subscribe",
                    path,
                    component_messages,
                    payload_refs,
                    role,
                    diagnostics,
                ));
            }

            WebSocketEndpoint {
                id: channel_id_from_path(path),
                path: path.clone(),
                description: channel
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("Imported AsyncAPI channel")
                    .to_string(),
                connection_params: vec![],
                lifecycle: ConnectionLifecycle::default(),
                messages: dedup_messages(messages),
                runtime: None,
            }
        })
        .collect()
}

fn resolve_operation_messages(
    operation: &Value,
    operation_name: &str,
    channel_path: &str,
    component_messages: &Map<String, Value>,
    payload_refs: &mut BTreeSet<String>,
    role: WsRole,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> Vec<MessageSchema> {
    let direction = map_direction(operation_name, role);

    let Some(message) = operation.get("message") else {
        diagnostics.push(OpenApiDiagnostic::warn(
            format!("#/channels/{}/{}", channel_path, operation_name),
            "Operation has no message definition".to_string(),
        ));
        return Vec::new();
    };

    let message_variants = expand_message_variants(message);
    let mut result = Vec::new();

    for variant in message_variants {
        let (message_name, payload_name, description) = resolve_message_variant(
            variant,
            channel_path,
            operation_name,
            component_messages,
            payload_refs,
            diagnostics,
        );

        result.push(MessageSchema {
            name: message_name,
            direction,
            schema: Schema::new(payload_name),
            description,
        });
    }

    result
}

fn expand_message_variants(message: &Value) -> Vec<&Value> {
    if let Some(one_of) = message.get("oneOf").and_then(Value::as_array) {
        return one_of.iter().collect();
    }
    vec![message]
}

fn resolve_message_variant(
    variant: &Value,
    channel_path: &str,
    operation_name: &str,
    component_messages: &Map<String, Value>,
    payload_refs: &mut BTreeSet<String>,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> (String, String, Option<String>) {
    if let Some(message_ref) = variant.get("$ref").and_then(Value::as_str) {
        let Some(message_name) = parse_ref_name(message_ref) else {
            diagnostics.push(OpenApiDiagnostic::error(
                format!("#/channels/{}/{}", channel_path, operation_name),
                format!("Invalid message reference '{}'", message_ref),
            ));
            return (
                "UnknownMessage".to_string(),
                "JsonValue".to_string(),
                Some("Unable to resolve message reference".to_string()),
            );
        };

        if let Some(message_def) = component_messages.get(&message_name) {
            let schema_name = resolve_payload_schema_name(
                message_def.get("payload"),
                format!(
                    "#/components/messages/{}/payload",
                    escape_json_pointer(&message_name)
                ),
                payload_refs,
                diagnostics,
            );

            let description = message_def
                .get("description")
                .and_then(Value::as_str)
                .map(String::from);
            return (to_pascal_case(&message_name), schema_name, description);
        }

        diagnostics.push(OpenApiDiagnostic::error(
            format!("#/channels/{}/{}", channel_path, operation_name),
            format!("Unresolved message reference '{}'", message_ref),
        ));
        return (
            to_pascal_case(&message_name),
            "JsonValue".to_string(),
            Some("Unresolved message reference".to_string()),
        );
    }

    let inline_name = variant
        .get("name")
        .and_then(Value::as_str)
        .map(to_pascal_case)
        .unwrap_or_else(|| {
            format!(
                "{}{}",
                to_pascal_case(&channel_id_from_path(channel_path)),
                to_pascal_case(operation_name)
            )
        });

    let schema_name = resolve_payload_schema_name(
        variant.get("payload"),
        format!("#/channels/{}/{}", channel_path, operation_name),
        payload_refs,
        diagnostics,
    );

    let description = variant
        .get("description")
        .and_then(Value::as_str)
        .map(String::from);

    (inline_name, schema_name, description)
}

fn resolve_payload_schema_name(
    payload: Option<&Value>,
    location: String,
    payload_refs: &mut BTreeSet<String>,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> String {
    let Some(payload) = payload else {
        diagnostics.push(OpenApiDiagnostic::info(
            location,
            "Message payload not defined; defaulting to JsonValue".to_string(),
        ));
        return "JsonValue".to_string();
    };

    if let Some(schema_ref) = payload.get("$ref").and_then(Value::as_str) {
        if let Some(name) = parse_ref_name(schema_ref) {
            let normalized = to_pascal_case(&name);
            payload_refs.insert(normalized.clone());
            return normalized;
        }
        diagnostics.push(OpenApiDiagnostic::warn(
            location,
            format!(
                "Invalid schema reference '{}'; defaulting to JsonValue",
                schema_ref
            ),
        ));
        return "JsonValue".to_string();
    }

    if payload.get("oneOf").is_some()
        || payload.get("anyOf").is_some()
        || payload.get("allOf").is_some()
    {
        diagnostics.push(OpenApiDiagnostic::warn(
            location,
            "Combinator payload (oneOf/anyOf/allOf) mapped to JsonValue".to_string(),
        ));
        return "JsonValue".to_string();
    }

    diagnostics.push(OpenApiDiagnostic::info(
        location,
        "Inline payload schema mapped to JsonValue".to_string(),
    ));
    "JsonValue".to_string()
}

fn map_component_schemas(
    component_schemas: &Map<String, Value>,
    payload_refs: &BTreeSet<String>,
) -> ModelCatalog {
    let mut seen = HashSet::new();
    let mut models = Vec::new();

    models.push(ModelDef::Alias(TypeAlias {
        name: "JsonValue".to_string(),
        description: Some("Fallback type for unknown or inline payload schemas".to_string()),
        target: TypeRef::Primitive(PrimitiveType::Json),
    }));
    seen.insert("JsonValue".to_string());

    for (name, schema) in component_schemas {
        let model_name = to_pascal_case(name);
        if seen.insert(model_name.clone()) {
            models.push(ModelDef::Alias(TypeAlias {
                name: model_name,
                description: schema
                    .get("description")
                    .and_then(Value::as_str)
                    .map(String::from),
                target: TypeRef::Primitive(PrimitiveType::Json),
            }));
        }
    }

    for name in payload_refs {
        if seen.insert(name.clone()) {
            models.push(ModelDef::Alias(TypeAlias {
                name: name.clone(),
                description: Some("Referenced payload schema".to_string()),
                target: TypeRef::Primitive(PrimitiveType::Json),
            }));
        }
    }

    ModelCatalog {
        module_path: None,
        types: models,
    }
}

fn write_import_report(
    output_dir: &Path,
    module_path: &str,
    api_name: &str,
    endpoint_count: usize,
    message_count: usize,
    model_count: usize,
    diagnostics: &[OpenApiDiagnostic],
) -> Result<(), GeneratorError> {
    if !output_dir.exists() {
        fs::create_dir_all(output_dir).map_err(|e| GeneratorError::WriteError {
            path: output_dir.display().to_string(),
            source: e,
        })?;
    }

    let report_path = output_dir.join(format!("{}_asyncapi_import.txt", module_path));
    let mut content = String::new();
    content.push_str(&format!("api_name={}\n", api_name));
    content.push_str(&format!("endpoint_count={}\n", endpoint_count));
    content.push_str(&format!("message_count={}\n", message_count));
    content.push_str(&format!("model_count={}\n", model_count));
    content.push_str("diagnostics:\n");

    if diagnostics.is_empty() {
        content.push_str("  - none\n");
    } else {
        for diag in diagnostics {
            content.push_str(&format!(
                "  - [{}] {}: {}\n",
                diag.severity, diag.location, diag.message
            ));
        }
    }

    fs::write(&report_path, content).map_err(|e| GeneratorError::WriteError {
        path: report_path.display().to_string(),
        source: e,
    })
}

fn default_env_auth(module_path: &str, auth: &AuthStrategy) -> Vec<String> {
    if matches!(auth, AuthStrategy::None) {
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

fn escape_json_pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
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
        let spec_path = temp_dir.path().join("test-asyncapi.yaml");
        std::fs::write(&spec_path, yaml).unwrap();

        let options = AsyncImportOptions {
            input: spec_path.to_string_lossy().to_string(),
            api_name: None,
            module_path: Some("test_ws".to_string()),
            ws_role: WsRole::Client,
            output: temp_dir.path().to_string_lossy().to_string(),
            dry_run: true,
            strict: false,
            verbose: false,
        };

        let result = run_import_asyncapi(&options).unwrap();
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
        let spec_path = temp_dir.path().join("auth-asyncapi.yaml");
        std::fs::write(&spec_path, yaml).unwrap();

        let options = AsyncImportOptions {
            input: spec_path.to_string_lossy().to_string(),
            api_name: None,
            module_path: Some("auth_ws".to_string()),
            ws_role: WsRole::Client,
            output: temp_dir.path().to_string_lossy().to_string(),
            dry_run: true,
            strict: false,
            verbose: false,
        };

        let result = run_import_asyncapi(&options).unwrap();
        assert!(matches!(
            result.ws_api.auth,
            AuthStrategy::ApiKey { ref header, .. } if header == "API-KEY"
        ));
    }
}
