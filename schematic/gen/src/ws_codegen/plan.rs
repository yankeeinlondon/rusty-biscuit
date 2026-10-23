//! WS runtime plan: lowering `WebSocketApi` into concrete generation decisions.

use schematic_define::websocket::{
    AuthFlowHints, FrameFormat, HeartbeatHints, MessageDirection, MessageSchema, RequestIdType,
    WebSocketApi, WebSocketEndpoint,
};
use schematic_define::{AuthStrategy, ParamDef, ParamStyle, QueryParamType};

use crate::codegen::request_structs::query_params::{QueryForm, QueryScalar, query_shape};
use crate::diagnostics::{Feature, Output, Severity, ValidationDiagnostic};
use crate::parser::extract_path_params;

pub use schematic_define::websocket::EndpointRole;

/// Diagnostic emitted during WS validation/lowering.
///
/// `path` locates the finding inside the definition (`endpoints/{id}/...`,
/// `base_url`, `auth`, ...). Reported through the shared format with
/// [`WsDiagnostic::to_validation_diagnostic`].
#[derive(Debug, Clone)]
pub struct WsDiagnostic {
    pub severity: WsSeverity,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsSeverity {
    Info,
    Warning,
    Error,
}

impl WsDiagnostic {
    pub fn info(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: WsSeverity::Info,
            path: path.into(),
            message: message.into(),
        }
    }

    pub fn warn(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: WsSeverity::Warning,
            path: path.into(),
            message: message.into(),
        }
    }

    pub fn error(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: WsSeverity::Error,
            path: path.into(),
            message: message.into(),
        }
    }

    /// Maps this finding of API `api` onto the shared Rust-output format.
    ///
    /// `None` for [`WsSeverity::Info`]: the shared format has no info level,
    /// so info notes are printed only by a verbose run. A path under
    /// `endpoints/{id}` names that endpoint.
    pub fn to_validation_diagnostic(&self, api: &str) -> Option<ValidationDiagnostic> {
        let severity = match self.severity {
            WsSeverity::Info => return None,
            WsSeverity::Warning => Severity::Warning,
            WsSeverity::Error => Severity::Error,
        };
        let endpoint = self
            .path
            .strip_prefix("endpoints/")
            .map(|rest| rest.split('/').next().unwrap_or(rest));
        Some(ValidationDiagnostic::new(
            severity,
            api,
            endpoint,
            Feature::WebSocketDefinition,
            Output::Rust,
            format!("{}: {}", self.path, self.message),
        ))
    }
}

/// A WebSocket definition that cannot be lowered: at least one of
/// `diagnostics` is an [`WsSeverity::Error`]. Every other finding of the
/// same run is kept beside it.
#[derive(Debug, Clone)]
pub struct WsLoweringError {
    pub api_name: String,
    pub diagnostics: Vec<WsDiagnostic>,
}

impl WsLoweringError {
    /// The warnings and errors in the shared format; info notes are dropped.
    pub fn validation_diagnostics(&self) -> Vec<ValidationDiagnostic> {
        self.diagnostics
            .iter()
            .filter_map(|d| d.to_validation_diagnostic(&self.api_name))
            .collect()
    }
}

impl std::fmt::Display for WsLoweringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let errors: Vec<String> = self
            .diagnostics
            .iter()
            .filter(|d| d.severity == WsSeverity::Error)
            .map(|d| format!("{}: {}", d.path, d.message))
            .collect();
        write!(
            f,
            "WebSocket API '{}' cannot be lowered: {}",
            self.api_name,
            errors.join("; ")
        )
    }
}

impl std::error::Error for WsLoweringError {}

/// Resolved correlation strategy for an endpoint.
#[derive(Debug, Clone)]
pub enum CorrelationStrategy {
    /// No request-response correlation. Fire-and-forget send + event stream.
    None,
    /// Correlated request-response using envelope ID fields.
    Correlated {
        request_id_field: String,
        response_id_field: String,
        default_timeout_secs: u64,
    },
}

/// Resolved auth strategy for the WS connection.
#[derive(Debug, Clone)]
pub enum WsAuthStrategy {
    /// No authentication required.
    None,
    /// Header-based auth during handshake (Bearer, API key, Basic).
    HeaderBased {
        strategy: AuthStrategy,
        env_auth: Vec<String>,
    },
    /// Message-based auth after connection established.
    MessageBased {
        header_strategy: AuthStrategy,
        env_auth: Vec<String>,
        flow: AuthFlowHints,
    },
}

/// Lifecycle message metadata for generated helper methods.
#[derive(Debug, Clone)]
pub struct LifecycleMessagePlan {
    pub name: String,
    pub schema_type: String,
}

/// Value type of one handshake parameter, or of each element of an array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsScalar {
    String,
    Integer,
    Number,
    Boolean,
}

/// How a query value is written, following the REST query style rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsQueryForm {
    /// One `name=value` pair.
    Single,
    /// One pair per array element (`form`, `explode: true`).
    Exploded,
    /// One pair whose percent-encoded elements are joined by this literal
    /// (`,` for `form`, `%20` for `spaceDelimited`, `%7C` for `pipeDelimited`).
    Delimited(&'static str),
}

/// Typed plan for a required path parameter.
#[derive(Debug, Clone)]
pub struct PathParamPlan {
    pub name: String,
    pub scalar: WsScalar,
    pub description: Option<String>,
}

/// Typed plan for a query parameter.
#[derive(Debug, Clone)]
pub struct QueryParamPlan {
    pub name: String,
    pub scalar: WsScalar,
    pub form: WsQueryForm,
    pub required: bool,
    pub description: Option<String>,
}

/// Plan for a single WS endpoint's code generation.
#[derive(Debug, Clone)]
pub struct EndpointPlan {
    pub id: String,
    pub path: String,
    pub description: String,
    pub role: EndpointRole,
    pub correlation: CorrelationStrategy,
    pub heartbeat: Option<HeartbeatHints>,
    pub client_messages: Vec<String>,
    pub server_messages: Vec<String>,
    pub bidirectional_messages: Vec<String>,
    /// Every declared message with its selectors, fallback flag, and
    /// response link, in declaration order.
    pub messages: Vec<MessageSchema>,
    pub lifecycle_open: Option<LifecycleMessagePlan>,
    pub lifecycle_close: Option<LifecycleMessagePlan>,
    pub lifecycle_keepalive: Option<LifecycleMessagePlan>,
    pub has_lifecycle_open: bool,
    pub has_lifecycle_close: bool,
    pub has_lifecycle_keepalive: bool,
    pub path_params: Vec<PathParamPlan>,
    pub query_params: Vec<QueryParamPlan>,
}

/// Complete plan for generating a WS API runtime.
#[derive(Debug, Clone)]
pub struct WsRuntimePlan {
    pub api_name: String,
    pub description: String,
    pub base_url: String,
    pub docs_url: Option<String>,
    pub frame_format: FrameFormat,
    pub request_id_type: RequestIdType,
    pub supports_reconnect: bool,
    pub auth: WsAuthStrategy,
    pub endpoints: Vec<EndpointPlan>,
    pub diagnostics: Vec<WsDiagnostic>,
}

/// Infer correlation strategy from endpoint hints and message schemas.
fn infer_correlation(
    endpoint: &WebSocketEndpoint,
    diagnostics: &mut Vec<WsDiagnostic>,
) -> CorrelationStrategy {
    // Use explicit hints if provided
    if let Some(ref runtime) = endpoint.runtime
        && let Some(ref corr) = runtime.correlation
    {
        return CorrelationStrategy::Correlated {
            request_id_field: corr.request_id_field.clone(),
            response_id_field: corr.response_id_field.clone(),
            default_timeout_secs: corr.default_timeout_secs,
        };
    }

    // Attempt to infer from message schema names
    let has_request_envelope = endpoint
        .messages
        .iter()
        .any(|m| m.name == "RequestEnvelope" && m.direction != MessageDirection::Server);
    let has_response_envelope = endpoint
        .messages
        .iter()
        .any(|m| m.name == "ResponseEnvelope" && m.direction != MessageDirection::Client);

    if has_request_envelope && has_response_envelope {
        diagnostics.push(WsDiagnostic::info(
            format!("endpoints/{}", endpoint.id),
            "Inferred correlation from RequestEnvelope/ResponseEnvelope pattern (id/req_id)",
        ));
        CorrelationStrategy::Correlated {
            request_id_field: "id".to_string(),
            response_id_field: "req_id".to_string(),
            default_timeout_secs: 30,
        }
    } else {
        CorrelationStrategy::None
    }
}

/// Resolve the WS auth strategy from API definition.
fn resolve_auth(api: &WebSocketApi, diagnostics: &mut Vec<WsDiagnostic>) -> WsAuthStrategy {
    let header_strategy = api.auth.clone();

    // Check if any endpoint has auth flow hints
    let auth_flow = api.endpoints.iter().find_map(|ep| {
        ep.runtime
            .as_ref()
            .and_then(|r| r.auth_flow.as_ref().cloned())
    });

    // Check if any endpoint has auth messages
    let has_auth_messages = api.endpoints.iter().any(|ep| {
        ep.messages
            .iter()
            .any(|m| m.name == "Auth" || m.name == "AuthRequired")
    });

    match (&header_strategy, auth_flow, has_auth_messages) {
        (AuthStrategy::None, None, false) => WsAuthStrategy::None,
        (AuthStrategy::None, None, true) => {
            diagnostics.push(WsDiagnostic::warn(
                "auth",
                "Auth messages found but no AuthStrategy or AuthFlowHints configured",
            ));
            WsAuthStrategy::None
        }
        (strategy, Some(flow), _) => WsAuthStrategy::MessageBased {
            header_strategy: strategy.clone(),
            env_auth: api.env_auth.clone(),
            flow,
        },
        (strategy, None, true) => {
            diagnostics.push(WsDiagnostic::info(
                "auth",
                "Auth messages detected; using header-based auth. Add AuthFlowHints for message-based auth flow.",
            ));
            WsAuthStrategy::HeaderBased {
                strategy: strategy.clone(),
                env_auth: api.env_auth.clone(),
            }
        }
        (strategy, None, false) => WsAuthStrategy::HeaderBased {
            strategy: strategy.clone(),
            env_auth: api.env_auth.clone(),
        },
    }
}

fn lifecycle_to_plan(
    message: Option<&schematic_define::websocket::MessageSchema>,
) -> Option<LifecycleMessagePlan> {
    message.map(|m| LifecycleMessagePlan {
        name: m.name.clone(),
        schema_type: m.schema.full_path(),
    })
}

/// Validate a WebSocket API definition before lowering.
///
/// Every finding is pushed onto `diagnostics`; nothing stops at the first
/// error, so one run reports them all.
pub fn validate_ws_api(api: &WebSocketApi, diagnostics: &mut Vec<WsDiagnostic>) {
    if api.name.is_empty() {
        diagnostics.push(WsDiagnostic::error(
            "name",
            "WebSocket API name cannot be empty",
        ));
    }

    if api.base_url.is_empty() {
        diagnostics.push(WsDiagnostic::error(
            "base_url",
            format!("WebSocket API '{}' has empty base_url", api.name),
        ));
    }

    if api.endpoints.is_empty() {
        diagnostics.push(WsDiagnostic::warn(
            &api.name,
            "No endpoints defined; generated client will be empty",
        ));
    }

    for endpoint in &api.endpoints {
        if endpoint.id.is_empty() {
            diagnostics.push(WsDiagnostic::error(
                "endpoints",
                format!(
                    "WebSocket API '{}' has an endpoint with an empty id (path `{}`)",
                    api.name, endpoint.path
                ),
            ));
        }

        if endpoint.messages.is_empty() {
            diagnostics.push(WsDiagnostic::warn(
                format!("endpoints/{}", endpoint.id),
                "Endpoint has no messages defined",
            ));
        }

        if let Some(segment) = duplicated_boundary_segment(&api.base_url, &endpoint.path) {
            diagnostics.push(WsDiagnostic::warn(
                format!("endpoints/{}/path", endpoint.id),
                format!(
                    "base_url `{}` ends with segment `{segment}` and path `{}` starts with it; \
                     the composed URL repeats `/{segment}/{segment}`",
                    api.base_url, endpoint.path
                ),
            ));
        }

        validate_endpoint_path(endpoint, diagnostics);
        validate_endpoint_fields(endpoint, diagnostics);

        // Check correlation hint consistency
        if let Some(ref runtime) = endpoint.runtime
            && let Some(ref corr) = runtime.correlation
            && (corr.request_id_field.is_empty() || corr.response_id_field.is_empty())
        {
            diagnostics.push(WsDiagnostic::error(
                format!("endpoints/{}/runtime/correlation", endpoint.id),
                format!(
                    "Endpoint '{}' has correlation hints with empty field paths",
                    endpoint.id
                ),
            ));
        }
    }
}

/// The segment repeated where `base_url`'s path meets `endpoint_path`: the
/// last segment of the base path when it is exactly the endpoint path's
/// first segment.
///
/// Whole segments only — `/api` + `/apikeys` shares a prefix, not a segment,
/// and is `None`.
fn duplicated_boundary_segment<'a>(base_url: &'a str, endpoint_path: &str) -> Option<&'a str> {
    let after_scheme = base_url.split_once("://").map_or(base_url, |(_, rest)| rest);
    let base_path = after_scheme
        .find('/')
        .map_or("", |index| &after_scheme[index..]);
    let base_path = base_path.split(['?', '#']).next().unwrap_or_default();
    let last = base_path.split('/').rfind(|segment| !segment.is_empty())?;
    let first = endpoint_path
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .split('/')
        .find(|segment| !segment.is_empty())?;
    (last == first).then_some(last)
}

/// Checks that an endpoint path only extends the base URL's path: it starts
/// with `/`, carries no query or fragment, and names no origin of its own.
fn validate_endpoint_path(endpoint: &WebSocketEndpoint, diagnostics: &mut Vec<WsDiagnostic>) {
    let at = format!("endpoints/{}/path", endpoint.id);
    let path = endpoint.path.as_str();
    if path.contains("://") || path.starts_with("//") {
        diagnostics.push(WsDiagnostic::error(
            &at,
            format!("path `{path}` names its own origin; endpoints are always relative to base_url"),
        ));
    } else if !path.is_empty() && !path.starts_with('/') {
        diagnostics.push(WsDiagnostic::error(
            &at,
            format!("path `{path}` must start with `/`; appended to base_url it would change the host or a segment"),
        ));
    }
    if path.contains(['?', '#']) {
        diagnostics.push(WsDiagnostic::error(
            &at,
            format!("path `{path}` embeds a query or fragment; declare query parameters in `params.query`"),
        ));
    }
}

fn ws_scalar(param_type: &QueryParamType) -> Option<WsScalar> {
    match param_type {
        QueryParamType::String => Some(WsScalar::String),
        QueryParamType::Integer => Some(WsScalar::Integer),
        QueryParamType::Number => Some(WsScalar::Number),
        QueryParamType::Boolean => Some(WsScalar::Boolean),
        _ => None,
    }
}

/// Lowers an endpoint's [`schematic_define::EndpointParams`]: typed path
/// placeholders and query parameters. Every declaration the handshake cannot
/// send is an error, never silently dropped.
fn lower_params(
    ep: &WebSocketEndpoint,
    diagnostics: &mut Vec<WsDiagnostic>,
) -> (Vec<PathParamPlan>, Vec<QueryParamPlan>) {
    let base = format!("endpoints/{}/params", ep.id);
    for (location, params) in [("header", &ep.params.header), ("cookie", &ep.params.cookie)] {
        for param in params {
            diagnostics.push(WsDiagnostic::error(
                format!("{base}/{location}/{}", param.name),
                format!("{location} parameters are not generated for a WebSocket handshake; declare constant headers on the API"),
            ));
        }
    }

    let placeholders: Vec<String> = extract_path_params(&ep.path)
        .into_iter()
        .map(str::to_string)
        .collect();
    let mut path_params = Vec::new();
    if ep.params.path.is_empty() {
        // As for REST: no typed declarations keep every placeholder a string.
        path_params.extend(placeholders.iter().map(|name| PathParamPlan {
            name: name.clone(),
            scalar: WsScalar::String,
            description: None,
        }));
    } else {
        for placeholder in &placeholders {
            if !ep.params.path.iter().any(|param| &param.name == placeholder) {
                diagnostics.push(WsDiagnostic::error(
                    format!("endpoints/{}/path/{placeholder}", ep.id),
                    format!("path placeholder `{{{placeholder}}}` has no entry in `params.path`"),
                ));
            }
        }
        for param in &ep.params.path {
            let at = format!("{base}/path/{}", param.name);
            if !placeholders.contains(&param.name) {
                diagnostics.push(WsDiagnostic::error(
                    &at,
                    format!("path parameter `{}` has no `{{{}}}` placeholder in `{}`", param.name, param.name, ep.path),
                ));
                continue;
            }
            if !param.required || param.style != ParamStyle::Simple || param.explode {
                diagnostics.push(WsDiagnostic::error(
                    &at,
                    "a path parameter must be required, `simple` style, and not exploded",
                ));
            }
            match ws_scalar(&param.param_type) {
                Some(scalar) => path_params.push(PathParamPlan {
                    name: param.name.clone(),
                    scalar,
                    description: param.description.clone(),
                }),
                None => diagnostics.push(WsDiagnostic::error(
                    &at,
                    "a path parameter must be a string, integer, number, or boolean",
                )),
            }
        }
    }

    let mut query_params = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for param in &ep.params.query {
        let at = format!("{base}/query/{}", param.name);
        if !seen.insert(param.name.as_str()) {
            diagnostics.push(WsDiagnostic::error(&at, "query parameter is declared more than once"));
            continue;
        }
        if let Some(plan) = lower_query_param(param, &at, diagnostics) {
            query_params.push(plan);
        }
    }
    (path_params, query_params)
}

/// One query parameter's plan, using the REST query style rules
/// ([`query_shape`]). Enum values are not generated for a handshake.
fn lower_query_param(
    param: &ParamDef,
    at: &str,
    diagnostics: &mut Vec<WsDiagnostic>,
) -> Option<QueryParamPlan> {
    let shape = match query_shape(param, "") {
        Ok(shape) => shape,
        Err(declared) => {
            diagnostics.push(WsDiagnostic::error(
                at,
                format!("unsupported query parameter style: {declared}"),
            ));
            return None;
        }
    };
    let scalar = match shape.scalar {
        QueryScalar::String => WsScalar::String,
        QueryScalar::Integer => WsScalar::Integer,
        QueryScalar::Number => WsScalar::Number,
        QueryScalar::Boolean => WsScalar::Boolean,
        QueryScalar::Enum { .. } => {
            diagnostics.push(WsDiagnostic::error(
                at,
                "enum query parameters are not generated for a WebSocket handshake; declare a string",
            ));
            return None;
        }
    };
    let form = match shape.form {
        QueryForm::Single => WsQueryForm::Single,
        QueryForm::Exploded => WsQueryForm::Exploded,
        QueryForm::Delimited("Space") => WsQueryForm::Delimited("%20"),
        QueryForm::Delimited("Pipe") => WsQueryForm::Delimited("%7C"),
        QueryForm::Delimited(_) => WsQueryForm::Delimited(","),
    };
    Some(QueryParamPlan {
        name: param.name.clone(),
        scalar,
        form,
        required: param.required,
        description: param.description.clone(),
    })
}

/// Field-level checks on declared metadata: names and indicators are
/// non-empty, and every declared duration is at least one second.
fn validate_endpoint_fields(endpoint: &WebSocketEndpoint, diagnostics: &mut Vec<WsDiagnostic>) {
    let base = format!("endpoints/{}", endpoint.id);
    for message in &endpoint.messages {
        let at = format!("{base}/messages/{}", message.name);
        if message.responds_with.as_deref() == Some("") {
            diagnostics.push(WsDiagnostic::error(
                format!("{at}/responds_with"),
                "responds_with names no message",
            ));
        }
        for (index, selector) in message.selectors.iter().flatten().enumerate() {
            if selector.field.is_empty() {
                diagnostics.push(WsDiagnostic::error(
                    format!("{at}/selectors/{index}/field"),
                    "a selector must name a wire field",
                ));
            }
        }
    }
    let Some(runtime) = endpoint.runtime.as_ref() else {
        return;
    };
    if let Some(correlation) = &runtime.correlation
        && correlation.default_timeout_secs == 0
    {
        diagnostics.push(WsDiagnostic::error(
            format!("{base}/runtime/correlation/default_timeout_secs"),
            "default_timeout_secs must be at least 1",
        ));
    }
    if let Some(flow) = &runtime.auth_flow {
        let required = [
            ("challenge_message", Some(&flow.challenge_message)),
            ("auth_request_schema", Some(&flow.auth_request_schema)),
            ("success_indicator", flow.success_indicator.as_ref()),
            ("failure_indicator", flow.failure_indicator.as_ref()),
        ];
        for (field, value) in required {
            if value.is_some_and(String::is_empty) {
                diagnostics.push(WsDiagnostic::error(
                    format!("{base}/runtime/auth_flow/{field}"),
                    format!("{field} cannot be empty"),
                ));
            }
        }
    }
    if let Some(heartbeat) = &runtime.heartbeat {
        if heartbeat.interval_secs == 0 {
            diagnostics.push(WsDiagnostic::error(
                format!("{base}/runtime/heartbeat/interval_secs"),
                "interval_secs must be at least 1",
            ));
        }
        if heartbeat.timeout_secs == Some(0) {
            diagnostics.push(WsDiagnostic::error(
                format!("{base}/runtime/heartbeat/timeout_secs"),
                "timeout_secs must be at least 1",
            ));
        }
    }
}

/// Lower a `WebSocketApi` definition into a `WsRuntimePlan`.
///
/// ## Errors
///
/// [`WsLoweringError`] when any validation or lowering finding is an error.
/// It carries every finding of the run, not only the first error; a returned
/// plan holds only warnings and info notes.
pub fn lower_to_plan(api: &WebSocketApi) -> Result<WsRuntimePlan, WsLoweringError> {
    let mut diagnostics = Vec::new();

    validate_ws_api(api, &mut diagnostics);

    let runtime = api.runtime.as_ref();
    let frame_format = runtime.map_or(FrameFormat::default(), |r| r.frame_format);
    let request_id_type = runtime.map_or(RequestIdType::default(), |r| r.request_id_type);
    let supports_reconnect = runtime.is_none_or(|r| r.supports_reconnect);

    let auth = resolve_auth(api, &mut diagnostics);

    let endpoints: Vec<EndpointPlan> = api
        .endpoints
        .iter()
        .map(|ep| {
            let role = ep.role;
            let correlation = infer_correlation(ep, &mut diagnostics);
            let heartbeat = ep
                .runtime
                .as_ref()
                .and_then(|r| r.heartbeat.clone());

            let client_messages: Vec<String> = ep
                .messages
                .iter()
                .filter(|m| m.direction == MessageDirection::Client)
                .map(|m| m.name.clone())
                .collect();

            let server_messages: Vec<String> = ep
                .messages
                .iter()
                .filter(|m| m.direction == MessageDirection::Server)
                .map(|m| m.name.clone())
                .collect();

            let bidirectional_messages: Vec<String> = ep
                .messages
                .iter()
                .filter(|m| m.direction == MessageDirection::Bidirectional)
                .map(|m| m.name.clone())
                .collect();

            let (path_params, query_params) = lower_params(ep, &mut diagnostics);

            EndpointPlan {
                id: ep.id.clone(),
                path: ep.path.clone(),
                description: ep.description.clone(),
                role,
                correlation,
                heartbeat,
                client_messages,
                server_messages,
                bidirectional_messages,
                messages: ep.messages.clone(),
                lifecycle_open: lifecycle_to_plan(ep.lifecycle.open.as_ref()),
                lifecycle_close: lifecycle_to_plan(ep.lifecycle.close.as_ref()),
                lifecycle_keepalive: lifecycle_to_plan(ep.lifecycle.keepalive.as_ref()),
                has_lifecycle_open: ep.lifecycle.open.is_some(),
                has_lifecycle_close: ep.lifecycle.close.is_some(),
                has_lifecycle_keepalive: ep.lifecycle.keepalive.is_some(),
                path_params,
                query_params,
            }
        })
        .collect();

    diagnostics.push(WsDiagnostic::info(
        "ws_runtime/options",
        "TODO: WsClientOptions.receive_timeout is not yet enforced by generated runtime.",
    ));

    if diagnostics.iter().any(|d| d.severity == WsSeverity::Error) {
        return Err(WsLoweringError {
            api_name: api.name.clone(),
            diagnostics,
        });
    }

    Ok(WsRuntimePlan {
        api_name: api.name.clone(),
        description: api.description.clone(),
        base_url: api.base_url.clone(),
        docs_url: api.docs_url.clone(),
        frame_format,
        request_id_type,
        supports_reconnect,
        auth,
        endpoints,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::Schema;
    use schematic_define::websocket::{
        AuthFlowHints, ConnectionLifecycle, CorrelationHints, MessageSchema,
        WebSocketEndpointHints, WebSocketRuntimeHints,
    };

    fn minimal_api() -> WebSocketApi {
        WebSocketApi {
            name: "TestApi".to_string(),
            description: "Test".to_string(),
            base_url: "wss://test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_mapping: None,
            headers: vec![],
            version: None,
            endpoints: vec![WebSocketEndpoint {
                id: "Echo".to_string(),
                path: "/echo".to_string(),
                description: "Echo endpoint".to_string(),
                params: schematic_define::EndpointParams::default(),
                role: EndpointRole::Both,
                lifecycle: ConnectionLifecycle::default(),
                messages: vec![MessageSchema::new(
                    "Msg",
                    MessageDirection::Bidirectional,
                    Schema::new("EchoMsg"),
                )],
                runtime: None,
            }],
            runtime: None,
        }
    }

    fn correlated_api() -> WebSocketApi {
        WebSocketApi {
            name: "CorrelatedApi".to_string(),
            description: "Test".to_string(),
            base_url: "wss://test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::ApiKey {
                header: "API-KEY".to_string(),
                value_prefix: None,
            },
            auth_policy: None,
            env_auth: vec!["TEST_KEY".to_string()],
            env_mapping: None,
            headers: vec![],
            version: None,
            endpoints: vec![WebSocketEndpoint {
                id: "Main".to_string(),
                path: "/ws".to_string(),
                description: "Main endpoint".to_string(),
                params: schematic_define::EndpointParams::default(),
                role: EndpointRole::Client,
                lifecycle: ConnectionLifecycle::default(),
                messages: vec![
                    MessageSchema::new(
                        "RequestEnvelope",
                        MessageDirection::Client,
                        Schema::new("ReqEnvelope"),
                    ),
                    MessageSchema::new(
                        "ResponseEnvelope",
                        MessageDirection::Server,
                        Schema::new("RespEnvelope"),
                    ),
                    MessageSchema::new(
                        "EventEnvelope",
                        MessageDirection::Server,
                        Schema::new("EvtEnvelope"),
                    ),
                ],
                runtime: None,
            }],
            runtime: None,
        }
    }

    #[test]
    fn lower_minimal_api() {
        let plan = lower_to_plan(&minimal_api()).unwrap();
        assert_eq!(plan.api_name, "TestApi");
        assert_eq!(plan.frame_format, FrameFormat::JsonText);
        assert!(plan.supports_reconnect);
        assert_eq!(plan.endpoints.len(), 1);
        assert_eq!(plan.endpoints[0].id, "Echo");
        assert!(plan.endpoints[0].path_params.is_empty());
        assert!(plan.endpoints[0].query_params.is_empty());
        assert!(matches!(plan.auth, WsAuthStrategy::None));
    }

    #[test]
    fn lowering_keeps_selectors_fallbacks_and_links() {
        use schematic_definitions::define_unfolded_circle_core_ws_api;
        let api = define_unfolded_circle_core_ws_api();
        let plan = lower_to_plan(&api).unwrap();
        for (planned, declared) in plan.endpoints.iter().zip(&api.endpoints) {
            assert_eq!(planned.messages, declared.messages);
        }
        let pong = plan.endpoints[0].messages.iter().find(|m| m.name == "Pong").unwrap();
        assert_eq!(pong.selectors.as_ref().map(Vec::len), Some(2));
        let ping = plan.endpoints[0].messages.iter().find(|m| m.name == "Ping").unwrap();
        assert_eq!(ping.responds_with.as_deref(), Some("Pong"));
    }

    #[test]
    fn lower_with_runtime_hints() {
        let mut api = minimal_api();
        api.runtime = Some(WebSocketRuntimeHints {
            frame_format: FrameFormat::Mixed,
            supports_reconnect: false,
            request_id_type: RequestIdType::String,
        });
        let plan = lower_to_plan(&api).unwrap();
        assert_eq!(plan.frame_format, FrameFormat::Mixed);
        assert!(!plan.supports_reconnect);
        assert_eq!(plan.request_id_type, RequestIdType::String);
    }

    #[test]
    fn lower_infers_correlation_from_envelope_pattern() {
        let plan = lower_to_plan(&correlated_api()).unwrap();
        let ep = &plan.endpoints[0];
        assert!(matches!(
            ep.correlation,
            CorrelationStrategy::Correlated { .. }
        ));
        if let CorrelationStrategy::Correlated {
            ref request_id_field,
            ref response_id_field,
            ..
        } = ep.correlation
        {
            assert_eq!(request_id_field, "id");
            assert_eq!(response_id_field, "req_id");
        }
    }

    #[test]
    fn lower_uses_explicit_correlation_hints() {
        let mut api = minimal_api();
        api.endpoints[0].runtime = Some(WebSocketEndpointHints {
            correlation: Some(CorrelationHints {
                request_id_field: "msg_id".to_string(),
                response_id_field: "ref_id".to_string(),
                default_timeout_secs: 60,
            }),
            auth_flow: None,
            heartbeat: None,
        });
        let plan = lower_to_plan(&api).unwrap();
        if let CorrelationStrategy::Correlated {
            ref request_id_field,
            default_timeout_secs,
            ..
        } = plan.endpoints[0].correlation
        {
            assert_eq!(request_id_field, "msg_id");
            assert_eq!(default_timeout_secs, 60);
        } else {
            panic!("Expected Correlated strategy");
        }
    }

    #[test]
    fn lower_resolves_header_auth() {
        let plan = lower_to_plan(&correlated_api()).unwrap();
        assert!(matches!(plan.auth, WsAuthStrategy::HeaderBased { .. }));
    }

    #[test]
    fn lower_categorizes_messages_by_direction() {
        let plan = lower_to_plan(&correlated_api()).unwrap();
        let ep = &plan.endpoints[0];
        assert_eq!(ep.client_messages, vec!["RequestEnvelope"]);
        assert_eq!(
            ep.server_messages,
            vec!["ResponseEnvelope", "EventEnvelope"]
        );
        assert!(ep.bidirectional_messages.is_empty());
    }

    #[test]
    fn lower_types_path_and_query_params_from_endpoint_params() {
        use schematic_define::{EndpointParams, ParamDef, ParamStyle, QueryParamType};
        let mut api = minimal_api();
        api.endpoints[0].path = "/ws/{channel_id}".to_string();
        api.endpoints[0].params = EndpointParams {
            path: vec![ParamDef::path("channel_id", QueryParamType::Integer).describes("Channel")],
            query: vec![
                ParamDef::new("limit", QueryParamType::Number),
                ParamDef { explode: true, ..ParamDef::new("tag", QueryParamType::Array(Box::new(QueryParamType::String))) },
                ParamDef::new("ids", QueryParamType::Array(Box::new(QueryParamType::Integer))).required(),
                ParamDef { style: ParamStyle::PipeDelimited, ..ParamDef::new("pipe", QueryParamType::Array(Box::new(QueryParamType::Boolean))) },
            ],
            ..EndpointParams::default()
        };

        let plan = lower_to_plan(&api).unwrap();
        let ep = &plan.endpoints[0];
        assert_eq!(ep.path_params.len(), 1);
        assert_eq!(ep.path_params[0].name, "channel_id");
        assert_eq!(ep.path_params[0].scalar, WsScalar::Integer);
        assert_eq!(ep.path_params[0].description.as_deref(), Some("Channel"));
        let query: Vec<(&str, WsScalar, WsQueryForm, bool)> = ep
            .query_params
            .iter()
            .map(|p| (p.name.as_str(), p.scalar, p.form, p.required))
            .collect();
        assert_eq!(
            query,
            [
                ("limit", WsScalar::Number, WsQueryForm::Single, false),
                ("tag", WsScalar::String, WsQueryForm::Exploded, false),
                ("ids", WsScalar::Integer, WsQueryForm::Delimited(","), true),
                ("pipe", WsScalar::Boolean, WsQueryForm::Delimited("%7C"), false),
            ]
        );
    }

    #[test]
    fn untyped_placeholders_are_required_strings() {
        let mut api = minimal_api();
        api.endpoints[0].path = "/rooms/{room}/{seat}".to_string();
        let plan = lower_to_plan(&api).unwrap();
        let names: Vec<(&str, WsScalar)> = plan.endpoints[0]
            .path_params
            .iter()
            .map(|p| (p.name.as_str(), p.scalar))
            .collect();
        assert_eq!(names, [("room", WsScalar::String), ("seat", WsScalar::String)]);
    }

    #[test]
    fn endpoint_role_comes_from_the_definition_not_message_directions() {
        let mut api = minimal_api();
        for role in [EndpointRole::Client, EndpointRole::Host, EndpointRole::Both] {
            api.endpoints[0].role = role;
            assert_eq!(lower_to_plan(&api).unwrap().endpoints[0].role, role);
        }
        // All-bidirectional messages no longer imply a host.
        api.endpoints[0].role = EndpointRole::default();
        assert_eq!(lower_to_plan(&api).unwrap().endpoints[0].role, EndpointRole::Client);
    }

    #[test]
    fn unsendable_handshake_params_are_errors() {
        use schematic_define::{EndpointParams, ParamDef, ParamStyle, QueryParamType};
        let mut api = minimal_api();
        api.endpoints[0].path = "/ws/{id}/{other}".to_string();
        api.endpoints[0].params = EndpointParams {
            path: vec![
                ParamDef::path("id", QueryParamType::Json),
                ParamDef { style: ParamStyle::Form, ..ParamDef::path("other", QueryParamType::String) },
                ParamDef::path("stray", QueryParamType::String),
            ],
            query: vec![
                ParamDef::new("dup", QueryParamType::String),
                ParamDef::new("dup", QueryParamType::String),
                ParamDef { style: ParamStyle::DeepObject, ..ParamDef::new("deep", QueryParamType::String) },
                ParamDef::new("kind", QueryParamType::Enum(vec!["a".into()])),
                ParamDef::new("blob", QueryParamType::Json),
            ],
            header: vec![ParamDef::new("X-Trace", QueryParamType::String)],
            cookie: vec![ParamDef::new("session", QueryParamType::String)],
        };
        assert_eq!(
            errors_of(&api),
            [
                "endpoints/Echo/params/header/X-Trace",
                "endpoints/Echo/params/cookie/session",
                "endpoints/Echo/params/path/id",
                "endpoints/Echo/params/path/other",
                "endpoints/Echo/params/path/stray",
                "endpoints/Echo/params/query/dup",
                "endpoints/Echo/params/query/deep",
                "endpoints/Echo/params/query/kind",
                "endpoints/Echo/params/query/blob",
            ]
        );
    }

    #[test]
    fn a_placeholder_without_its_typed_declaration_is_an_error() {
        use schematic_define::{EndpointParams, ParamDef, QueryParamType};
        let mut api = minimal_api();
        api.endpoints[0].path = "/ws/{id}/{missing}".to_string();
        api.endpoints[0].params = EndpointParams {
            path: vec![ParamDef::path("id", QueryParamType::String)],
            ..EndpointParams::default()
        };
        assert_eq!(errors_of(&api), ["endpoints/Echo/path/missing"]);
    }

    #[test]
    fn endpoint_paths_cannot_carry_a_query_or_an_origin() {
        let cases = [
            ("/ws?debug=1", 1),
            ("/ws#frag", 1),
            ("wss://evil.example/ws", 1),
            ("//evil.example/ws", 1),
            ("ws", 1),
            ("ws://evil.example/ws?x", 2),
        ];
        for (path, count) in cases {
            let mut api = minimal_api();
            api.endpoints[0].path = path.to_string();
            let errors = errors_of(&api);
            assert_eq!(errors.len(), count, "{path}: {errors:?}");
            assert!(errors.iter().all(|e| e == "endpoints/Echo/path"), "{path}: {errors:?}");
        }
        for path in ["/", "/ws", "/v1/{id}/stream", ""] {
            let mut api = minimal_api();
            api.endpoints[0].path = path.to_string();
            assert!(lower_to_plan(&api).is_ok(), "{path}");
        }
    }

    #[test]
    fn declared_names_indicators_and_durations_are_checked() {
        let mut api = minimal_api();
        let message = &mut api.endpoints[0].messages[0];
        message.responds_with = Some(String::new());
        message.selectors = Some(vec![schematic_define::websocket::MessageSelector::new("", "x")]);
        api.endpoints[0].runtime = Some(WebSocketEndpointHints {
            correlation: Some(CorrelationHints {
                request_id_field: "id".to_string(),
                response_id_field: "id".to_string(),
                default_timeout_secs: 0,
            }),
            auth_flow: Some(AuthFlowHints {
                challenge_message: String::new(),
                auth_request_schema: String::new(),
                success_indicator: Some(String::new()),
                failure_indicator: Some(String::new()),
            }),
            heartbeat: Some(HeartbeatHints {
                interval_secs: 0,
                timeout_secs: Some(0),
            }),
        });
        assert_eq!(
            errors_of(&api),
            [
                "endpoints/Echo/messages/Msg/responds_with",
                "endpoints/Echo/messages/Msg/selectors/0/field",
                "endpoints/Echo/runtime/correlation/default_timeout_secs",
                "endpoints/Echo/runtime/auth_flow/challenge_message",
                "endpoints/Echo/runtime/auth_flow/auth_request_schema",
                "endpoints/Echo/runtime/auth_flow/success_indicator",
                "endpoints/Echo/runtime/auth_flow/failure_indicator",
                "endpoints/Echo/runtime/heartbeat/interval_secs",
                "endpoints/Echo/runtime/heartbeat/timeout_secs",
            ]
        );
        // Control: the smallest valid values are accepted.
        let mut api = minimal_api();
        api.endpoints[0].runtime = Some(WebSocketEndpointHints {
            correlation: Some(CorrelationHints {
                request_id_field: "id".to_string(),
                response_id_field: "id".to_string(),
                default_timeout_secs: 1,
            }),
            auth_flow: None,
            heartbeat: Some(HeartbeatHints { interval_secs: 1, timeout_secs: Some(1) }),
        });
        assert!(lower_to_plan(&api).is_ok());
    }

    #[test]
    fn heartbeat_hints_lower_without_any_warning() {
        let mut api = minimal_api();
        api.endpoints[0].runtime = Some(WebSocketEndpointHints {
            heartbeat: Some(HeartbeatHints { interval_secs: 20, timeout_secs: Some(5) }),
            ..WebSocketEndpointHints::default()
        });
        let plan = lower_to_plan(&api).unwrap();
        assert_eq!(plan.endpoints[0].heartbeat.as_ref().map(|h| h.interval_secs), Some(20));
        let findings: Vec<_> = plan
            .diagnostics
            .iter()
            .filter(|d| d.severity != WsSeverity::Info)
            .collect();
        assert!(findings.is_empty(), "{findings:?}");
    }

    fn errors_of(api: &WebSocketApi) -> Vec<String> {
        lower_to_plan(api)
            .expect_err("lowering must fail")
            .diagnostics
            .iter()
            .filter(|d| d.severity == WsSeverity::Error)
            .map(|d| d.path.clone())
            .collect()
    }

    #[test]
    fn validate_rejects_empty_name() {
        let mut api = minimal_api();
        api.name = String::new();
        assert_eq!(errors_of(&api), ["name"]);
    }

    #[test]
    fn validate_rejects_empty_base_url() {
        let mut api = minimal_api();
        api.base_url = String::new();
        assert_eq!(errors_of(&api), ["base_url"]);
    }

    #[test]
    fn validate_rejects_empty_endpoint_id() {
        let mut api = minimal_api();
        api.endpoints[0].id = String::new();
        assert_eq!(errors_of(&api), ["endpoints"]);
    }

    #[test]
    fn validate_rejects_empty_correlation_fields() {
        let mut api = minimal_api();
        api.endpoints[0].runtime = Some(WebSocketEndpointHints {
            correlation: Some(CorrelationHints {
                request_id_field: String::new(),
                response_id_field: "id".to_string(),
                default_timeout_secs: 30,
            }),
            auth_flow: None,
            heartbeat: None,
        });
        assert_eq!(errors_of(&api), ["endpoints/Echo/runtime/correlation"]);
    }

    #[test]
    fn lowering_failure_keeps_every_finding_not_only_the_first_error() {
        let mut api = minimal_api();
        api.name = String::new();
        api.base_url = String::new();
        api.endpoints[0].messages.clear();
        api.endpoints[0].path = "/ws?room=1".to_string();

        let failure = lower_to_plan(&api).expect_err("lowering must fail");
        let errors: Vec<&str> = failure
            .diagnostics
            .iter()
            .filter(|d| d.severity == WsSeverity::Error)
            .map(|d| d.path.as_str())
            .collect();
        assert_eq!(errors, ["name", "base_url", "endpoints/Echo/path"]);
        assert!(
            failure
                .diagnostics
                .iter()
                .any(|d| d.severity == WsSeverity::Warning && d.path == "endpoints/Echo"),
            "the warning for the message-less endpoint is kept: {:?}",
            failure.diagnostics
        );
    }

    #[test]
    fn diagnostic_maps_onto_the_shared_format() {
        let error = WsDiagnostic::error("endpoints/Echo/path/room", "no typed entry");
        let mapped = error.to_validation_diagnostic("TestApi").expect("errors map");
        assert_eq!(
            mapped.to_string(),
            "severity=error api=TestApi endpoint=Echo feature=websocket-definition \
             output=rust: endpoints/Echo/path/room: no typed entry"
        );

        let warning = WsDiagnostic::warn("auth", "careful");
        let mapped = warning.to_validation_diagnostic("TestApi").expect("warnings map");
        assert_eq!(mapped.severity, Severity::Warning);
        assert_eq!(mapped.endpoint, None);

        assert!(
            WsDiagnostic::info("ws_runtime/options", "note")
                .to_validation_diagnostic("TestApi")
                .is_none(),
            "the shared format has no info level"
        );
    }

    #[test]
    fn duplicated_boundary_segment_matches_whole_segments_only() {
        let cases = [
            ("ws://remote.local/ws", "/ws", Some("ws")),
            ("ws://remote.local/ws/", "/ws/sub", Some("ws")),
            ("https://h/api", "/api/items", Some("api")),
            ("https://h/v1/api", "/api", Some("api")),
            ("https://h/api", "/apikeys", None),
            ("https://h/apikeys", "/api", None),
            ("https://h/api", "/v1/api", None),
            ("ws://remote.local", "/ws", None),
            ("ws://remote.local/", "/ws", None),
            ("ws://remote.local/ws", "/", None),
            ("ws://host:8080/ws", "/{id}", None),
        ];
        for (base, path, expected) in cases {
            assert_eq!(
                duplicated_boundary_segment(base, path),
                expected,
                "{base} + {path}"
            );
        }
    }

    #[test]
    fn validate_warns_on_empty_endpoints() {
        let mut api = minimal_api();
        api.endpoints.clear();
        let plan = lower_to_plan(&api).unwrap();
        assert!(
            plan.diagnostics
                .iter()
                .any(|d| d.severity == WsSeverity::Warning)
        );
    }

    #[test]
    fn lower_all_known_ws_apis() {
        // Verify lowering works for all currently defined WS APIs
        use schematic_definitions::{
            define_elevenlabs_websocket_api, define_samsung_smart_tv_remote_ws_api,
            define_unfolded_circle_core_ws_api, define_unfolded_circle_dock_ws_api,
            define_unfolded_circle_integration_ws_api,
        };

        let apis = vec![
            define_elevenlabs_websocket_api(),
            define_unfolded_circle_core_ws_api(),
            define_unfolded_circle_dock_ws_api(),
            define_unfolded_circle_integration_ws_api(),
            define_samsung_smart_tv_remote_ws_api(),
        ];

        for api in &apis {
            let plan = lower_to_plan(api).unwrap();
            assert!(!plan.api_name.is_empty());
            assert!(!plan.endpoints.is_empty());
        }
    }
}
