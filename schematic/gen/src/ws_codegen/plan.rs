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
    /// Correlated request-response: the endpoint declares
    /// `CorrelationHints`, and `pairs` are its messages linked by
    /// `responds_with` (never inferred from message names).
    Correlated {
        request_id_field: String,
        response_id_field: String,
        default_timeout_secs: u64,
        /// JSON type of the generated request ids.
        id_type: RequestIdType,
        pairs: Vec<CorrelatedPair>,
    },
}

/// One request message and the message its `responds_with` names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrelatedPair {
    pub request: String,
    pub request_schema: String,
    pub response: String,
    pub response_schema: String,
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
    /// The client handshake's credential policy in runtime terms, translated
    /// exactly as a REST client's.
    pub(crate) handshake_auth: crate::codegen::api_struct::auth::AuthConfig,
    /// Constant handshake headers, in declaration order.
    pub headers: Vec<(String, String)>,
    pub endpoints: Vec<EndpointPlan>,
    pub diagnostics: Vec<WsDiagnostic>,
}

/// The correlation of `endpoint`: its `CorrelationHints` and the
/// `responds_with` pairs of its messages. Without hints there is none;
/// [`validate_correlation`] reports a link declared without them.
fn lower_correlation(endpoint: &WebSocketEndpoint, id_type: RequestIdType) -> CorrelationStrategy {
    let Some(hints) = endpoint.runtime.as_ref().and_then(|runtime| runtime.correlation.as_ref()) else {
        return CorrelationStrategy::None;
    };
    let pairs = endpoint
        .messages
        .iter()
        .filter(|message| message.direction != MessageDirection::Server)
        .filter_map(|request| {
            let name = request.responds_with.as_deref()?;
            let response = endpoint
                .messages
                .iter()
                .find(|message| message.name == name && message.direction != MessageDirection::Client)?;
            Some(CorrelatedPair {
                request: request.name.clone(),
                request_schema: request.schema.full_path(),
                response: response.name.clone(),
                response_schema: response.schema.full_path(),
            })
        })
        .collect();
    CorrelationStrategy::Correlated {
        request_id_field: hints.request_id_field.clone(),
        response_id_field: hints.response_id_field.clone(),
        default_timeout_secs: hints.default_timeout_secs,
        id_type,
        pairs,
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

    if let Some(format) = api.runtime.as_ref().map(|runtime| runtime.frame_format)
        && matches!(format, FrameFormat::RawBinary | FrameFormat::Mixed)
    {
        diagnostics.push(WsDiagnostic::error(
            "runtime/frame_format",
            format!(
                "frame_format `{format}` is not supported: generated message enums encode and \
                 decode JSON, as `json_text` or `json_binary` frames"
            ),
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
        validate_message_names(endpoint, diagnostics);
        for (direction, enum_suffix) in inbound_directions(endpoint.role) {
            validate_selection(endpoint, direction, enum_suffix, diagnostics);
        }

        validate_correlation(endpoint, diagnostics);
    }

    let id_type = api.runtime.as_ref().map_or(RequestIdType::default(), |runtime| runtime.request_id_type);
    let correlates = api.endpoints.iter().any(|endpoint| {
        matches!(endpoint.role, EndpointRole::Client | EndpointRole::Both)
            && endpoint.messages.iter().any(|message| message.responds_with.is_some())
            && endpoint.runtime.as_ref().is_some_and(|runtime| runtime.correlation.is_some())
    });
    if id_type != RequestIdType::default() && !correlates {
        diagnostics.push(WsDiagnostic::warn(
            "runtime/request_id_type",
            format!("request_id_type `{id_type}` is declared, but no client endpoint correlates a request with its response"),
        ));
    }
}

/// Checks each `responds_with` link and the endpoint's `CorrelationHints`:
/// a link is set on a request (client or bidirectional) message, names a
/// message of the endpoint the client can receive, and needs the hints; the
/// hints need a non-empty field on each side and a link for a client to
/// consume.
fn validate_correlation(endpoint: &WebSocketEndpoint, diagnostics: &mut Vec<WsDiagnostic>) {
    let hints = endpoint.runtime.as_ref().and_then(|runtime| runtime.correlation.as_ref());
    let mut linked = false;
    for message in &endpoint.messages {
        let Some(response) = message.responds_with.as_deref().filter(|name| !name.is_empty()) else {
            continue;
        };
        let at = format!("endpoints/{}/messages/{}/responds_with", endpoint.id, message.name);
        if message.direction == MessageDirection::Server {
            diagnostics.push(WsDiagnostic::error(
                &at,
                "responds_with is set on a server message; only a client or bidirectional request names its response",
            ));
            continue;
        }
        match endpoint.messages.iter().find(|candidate| candidate.name == response) {
            None => diagnostics.push(WsDiagnostic::error(
                &at,
                format!("responds_with names `{response}`, which is not a message of endpoint `{}`", endpoint.id),
            )),
            Some(target) if target.direction == MessageDirection::Client => diagnostics.push(WsDiagnostic::error(
                &at,
                format!("responds_with names `{response}`, a client message the client never receives"),
            )),
            Some(_) => linked = true,
        }
        if hints.is_none() {
            diagnostics.push(WsDiagnostic::error(
                &at,
                "responds_with needs the endpoint's runtime.correlation hints to name the id fields",
            ));
        }
        if to_snake_case(&message.name) == "value" {
            diagnostics.push(WsDiagnostic::error(
                &at,
                "a request named `Value` would generate `request_value`, the untyped request method",
            ));
        }
    }

    let Some(hints) = hints else {
        return;
    };
    let at = format!("endpoints/{}/runtime/correlation", endpoint.id);
    if hints.request_id_field.is_empty() || hints.response_id_field.is_empty() {
        diagnostics.push(WsDiagnostic::error(
            &at,
            format!("Endpoint '{}' has correlation hints with empty field paths", endpoint.id),
        ));
    }
    if endpoint.role == EndpointRole::Host {
        diagnostics.push(WsDiagnostic::warn(
            &at,
            "correlation hints are declared on a host-only endpoint; only a generated client correlates requests",
        ));
    } else if !linked {
        diagnostics.push(WsDiagnostic::warn(
            &at,
            "correlation hints are declared, but no message links a response with responds_with",
        ));
    }
}

/// `PascalCase` → `snake_case`, as the generated method names spell it.
fn to_snake_case(name: &str) -> String {
    super::client::to_snake_case(name)
}

/// The message directions an endpoint receives, each with the suffix of the
/// enum it decodes into: a client receives `Server` messages, a host
/// receives `Client` messages, and `Bidirectional` messages are in both.
pub fn inbound_directions(role: EndpointRole) -> Vec<(MessageDirection, &'static str)> {
    let mut directions = Vec::new();
    if matches!(role, EndpointRole::Client | EndpointRole::Both) {
        directions.push((MessageDirection::Server, "ServerMessage"));
    }
    if matches!(role, EndpointRole::Host | EndpointRole::Both) {
        directions.push((MessageDirection::Client, "ClientMessage"));
    }
    directions
}

/// Whether `message` belongs to the enum of `direction` (`Client` or
/// `Server`); `Bidirectional` messages belong to both.
pub fn in_direction(message: &MessageSchema, direction: MessageDirection) -> bool {
    message.direction == direction || message.direction == MessageDirection::Bidirectional
}

/// Message names become enum variants: each is a Rust identifier and is
/// declared once per endpoint.
fn validate_message_names(endpoint: &WebSocketEndpoint, diagnostics: &mut Vec<WsDiagnostic>) {
    let mut seen = std::collections::BTreeSet::new();
    for message in &endpoint.messages {
        let at = format!("endpoints/{}/messages/{}", endpoint.id, message.name);
        if syn::parse_str::<syn::Ident>(&message.name).is_err() {
            diagnostics.push(WsDiagnostic::error(
                &at,
                "a message name becomes an enum variant and must be a Rust identifier",
            ));
        }
        if !seen.insert(message.name.as_str()) {
            diagnostics.push(WsDiagnostic::error(&at, "message name is declared more than once"));
        }
    }
}

/// Checks that every frame of an inbound direction selects at most one
/// message: at most one fallback, a fallback declares no selectors, every
/// other message declares selectors when the direction has more than one
/// message, and no two selector sets can match one frame unless one is a
/// strict subset of the other (the frame then selects the more specific
/// message).
fn validate_selection(
    endpoint: &WebSocketEndpoint,
    direction: MessageDirection,
    enum_suffix: &str,
    diagnostics: &mut Vec<WsDiagnostic>,
) {
    let inbound: Vec<&MessageSchema> = endpoint
        .messages
        .iter()
        .filter(|message| in_direction(message, direction))
        .collect();
    let at = |message: &MessageSchema| format!("endpoints/{}/messages/{}", endpoint.id, message.name);
    let enum_name = format!("{}{enum_suffix}", endpoint.id);

    let fallbacks: Vec<&str> = inbound
        .iter()
        .filter(|message| message.fallback)
        .map(|message| message.name.as_str())
        .collect();
    if fallbacks.len() > 1 {
        diagnostics.push(WsDiagnostic::error(
            format!("endpoints/{}/messages", endpoint.id),
            format!("`{enum_name}` has more than one fallback: {}", fallbacks.join(", ")),
        ));
    }

    let mut selected = Vec::new();
    for message in &inbound {
        let selectors = message.selectors.as_deref().unwrap_or_default();
        if message.fallback {
            if !selectors.is_empty() {
                diagnostics.push(WsDiagnostic::error(
                    format!("{}/selectors", at(message)),
                    "a fallback cannot also declare selectors",
                ));
            }
            continue;
        }
        if selectors.is_empty() {
            if inbound.len() > 1 {
                diagnostics.push(WsDiagnostic::error(
                    format!("{}/selectors", at(message)),
                    format!("`{enum_name}` has more than one message, so this message needs selectors or `fallback`"),
                ));
            }
            continue;
        }
        let mut fields = std::collections::BTreeMap::new();
        for selector in selectors {
            if fields.insert(selector.field.as_str(), selector.value.as_str()).is_some() {
                diagnostics.push(WsDiagnostic::error(
                    format!("{}/selectors", at(message)),
                    format!("selector field `{}` is declared more than once", selector.field),
                ));
            }
        }
        selected.push((*message, fields));
    }

    for (index, (first, first_fields)) in selected.iter().enumerate() {
        for (second, second_fields) in &selected[index + 1..] {
            let contradict = first_fields
                .iter()
                .any(|(field, value)| second_fields.get(field).is_some_and(|other| other != value));
            if contradict {
                continue;
            }
            let first_in_second = first_fields.iter().all(|(f, v)| second_fields.get(f) == Some(v));
            let second_in_first = second_fields.iter().all(|(f, v)| first_fields.get(f) == Some(v));
            let problem = match (first_in_second, second_in_first) {
                (true, true) => "declares the same selectors as",
                (false, false) => "has selectors that can match the same frame as",
                // A strict subset: the more specific message wins.
                _ => continue,
            };
            diagnostics.push(WsDiagnostic::error(
                format!("{}/selectors", at(second)),
                format!(
                    "in `{enum_name}`, `{}` {problem} `{}`; make one a strict subset of the other or give them a field with different values",
                    second.name, first.name
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
            let correlation = lower_correlation(ep, request_id_type);
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
        handshake_auth: crate::codegen::api_struct::auth::policy_config(
            &api.env_auth,
            &api.effective_auth_policy(),
            &api.default_env_mapping(),
        ),
        headers: api.headers.clone(),
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
                        "Query",
                        MessageDirection::Client,
                        Schema::new("ReqEnvelope"),
                    )
                    .responds_with("Reply"),
                    MessageSchema::new(
                        "Reply",
                        MessageDirection::Server,
                        Schema::new("RespEnvelope"),
                    )
                    .fallback(),
                    MessageSchema::new(
                        "EventEnvelope",
                        MessageDirection::Server,
                        Schema::new("EvtEnvelope"),
                    )
                    .matches("kind", "event"),
                ],
                runtime: Some(WebSocketEndpointHints {
                    correlation: Some(CorrelationHints {
                        request_id_field: "id".to_string(),
                        response_id_field: "req_id".to_string(),
                        default_timeout_secs: 30,
                    }),
                    ..WebSocketEndpointHints::default()
                }),
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
            frame_format: FrameFormat::JsonBinary,
            supports_reconnect: false,
            request_id_type: RequestIdType::String,
        });
        let plan = lower_to_plan(&api).unwrap();
        assert_eq!(plan.frame_format, FrameFormat::JsonBinary);
        assert!(!plan.supports_reconnect);
        assert_eq!(plan.request_id_type, RequestIdType::String);
    }

    #[test]
    fn lower_pairs_requests_through_responds_with_and_the_endpoint_hints() {
        let plan = lower_to_plan(&correlated_api()).unwrap();
        let CorrelationStrategy::Correlated {
            ref request_id_field,
            ref response_id_field,
            id_type,
            ref pairs,
            ..
        } = plan.endpoints[0].correlation
        else {
            panic!("expected a correlated endpoint");
        };
        assert_eq!((request_id_field.as_str(), response_id_field.as_str()), ("id", "req_id"));
        assert_eq!(id_type, RequestIdType::U64);
        assert_eq!(
            pairs,
            &[CorrelatedPair {
                request: "Query".to_string(),
                request_schema: "ReqEnvelope".to_string(),
                response: "Reply".to_string(),
                response_schema: "RespEnvelope".to_string(),
            }]
        );
        let findings: Vec<_> = plan.diagnostics.iter().filter(|d| d.severity != WsSeverity::Info).collect();
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn the_api_request_id_type_reaches_each_correlated_endpoint() {
        let mut api = correlated_api();
        api.runtime = Some(WebSocketRuntimeHints {
            request_id_type: RequestIdType::String,
            ..WebSocketRuntimeHints::default()
        });
        let plan = lower_to_plan(&api).unwrap();
        assert!(matches!(
            plan.endpoints[0].correlation,
            CorrelationStrategy::Correlated { id_type: RequestIdType::String, .. }
        ));
    }

    /// Every finding of lowering `api` at `severity`, as `path: message`.
    fn findings_of(api: &WebSocketApi, severity: WsSeverity) -> Vec<String> {
        let diagnostics = match lower_to_plan(api) {
            Ok(plan) => plan.diagnostics,
            Err(failure) => failure.diagnostics,
        };
        diagnostics
            .iter()
            .filter(|d| d.severity == severity)
            .map(|d| format!("{}: {}", d.path, d.message))
            .collect()
    }

    #[test]
    fn a_responds_with_link_without_endpoint_hints_is_an_error() {
        let mut api = correlated_api();
        api.endpoints[0].runtime = None;
        assert_eq!(
            findings_of(&api, WsSeverity::Error),
            ["endpoints/Main/messages/Query/responds_with: responds_with needs the endpoint's runtime.correlation hints to name the id fields"]
        );
    }

    #[test]
    fn a_link_on_a_server_message_or_to_an_unknown_or_client_message_is_an_error() {
        let mut api = correlated_api();
        let messages = &mut api.endpoints[0].messages;
        messages[1].responds_with = Some("Query".to_string());
        messages.push(client("Lookup").responds_with("Missing"));
        messages.push(client("Echoed").responds_with("Query"));
        assert_eq!(
            findings_of(&api, WsSeverity::Error),
            [
                "endpoints/Main/messages/Reply/responds_with: responds_with is set on a server message; only a client or bidirectional request names its response",
                "endpoints/Main/messages/Lookup/responds_with: responds_with names `Missing`, which is not a message of endpoint `Main`",
                "endpoints/Main/messages/Echoed/responds_with: responds_with names `Query`, a client message the client never receives",
            ]
        );
    }

    #[test]
    fn a_bidirectional_request_may_name_a_bidirectional_response() {
        let mut api = correlated_api();
        api.endpoints[0].messages.push(
            MessageSchema::new("Both", MessageDirection::Bidirectional, Schema::new("serde_json::Value"))
                .matches("kind", "both")
                .responds_with("Both"),
        );
        assert!(findings_of(&api, WsSeverity::Error).is_empty());
    }

    #[test]
    fn a_request_named_value_is_an_error() {
        let mut api = correlated_api();
        api.endpoints[0].messages[0].name = "Value".to_string();
        assert_eq!(
            findings_of(&api, WsSeverity::Error),
            ["endpoints/Main/messages/Value/responds_with: a request named `Value` would generate `request_value`, the untyped request method"]
        );
    }

    #[test]
    fn hints_that_nothing_consumes_warn() {
        let mut api = correlated_api();
        api.endpoints[0].messages[0].responds_with = None;
        assert_eq!(
            findings_of(&api, WsSeverity::Warning),
            ["endpoints/Main/runtime/correlation: correlation hints are declared, but no message links a response with responds_with"]
        );

        let mut host = correlated_api();
        host.endpoints[0].role = EndpointRole::Host;
        host.endpoints[0].messages[1].fallback = false;
        host.endpoints[0].messages[1].selectors = Some(vec![schematic_define::websocket::MessageSelector::new("kind", "resp")]);
        assert_eq!(
            findings_of(&host, WsSeverity::Warning),
            ["endpoints/Main/runtime/correlation: correlation hints are declared on a host-only endpoint; only a generated client correlates requests"]
        );
    }

    #[test]
    fn a_declared_string_id_type_that_nothing_consumes_warns_and_the_implicit_default_does_not() {
        let mut api = minimal_api();
        assert!(findings_of(&api, WsSeverity::Warning).is_empty());
        api.runtime = Some(WebSocketRuntimeHints {
            request_id_type: RequestIdType::String,
            ..WebSocketRuntimeHints::default()
        });
        assert_eq!(
            findings_of(&api, WsSeverity::Warning),
            ["runtime/request_id_type: request_id_type `string` is declared, but no client endpoint correlates a request with its response"]
        );
        // Consumed by a correlated client endpoint: no warning.
        let mut correlated = correlated_api();
        correlated.runtime = api.runtime.clone();
        assert!(findings_of(&correlated, WsSeverity::Warning).is_empty());
    }

    #[test]
    fn no_endpoint_correlates_without_hints_whatever_its_messages_are_named() {
        let mut api = correlated_api();
        api.endpoints[0].runtime = None;
        api.endpoints[0].messages[0].responds_with = None;
        let plan = lower_to_plan(&api).unwrap();
        assert!(matches!(plan.endpoints[0].correlation, CorrelationStrategy::None));
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
        assert_eq!(ep.client_messages, vec!["Query"]);
        assert_eq!(
            ep.server_messages,
            vec!["Reply", "EventEnvelope"]
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

    /// An endpoint of role `role` whose messages are `messages`.
    fn selection_api(role: EndpointRole, messages: Vec<MessageSchema>) -> WebSocketApi {
        let mut api = minimal_api();
        api.endpoints[0].role = role;
        api.endpoints[0].messages = messages;
        api
    }

    /// Every error of lowering `api` as `path: message`; empty when it lowers.
    fn selection_errors(api: &WebSocketApi) -> Vec<String> {
        match lower_to_plan(api) {
            Ok(_) => Vec::new(),
            Err(failure) => failure
                .diagnostics
                .iter()
                .filter(|d| d.severity == WsSeverity::Error)
                .map(|d| format!("{}: {}", d.path, d.message))
                .collect(),
        }
    }

    fn server(name: &str) -> MessageSchema {
        MessageSchema::new(name, MessageDirection::Server, Schema::new("serde_json::Value"))
    }

    fn client(name: &str) -> MessageSchema {
        MessageSchema::new(name, MessageDirection::Client, Schema::new("serde_json::Value"))
    }

    /// The control: a fallback, a generic selector, and a strict superset
    /// of it (Core's response/event envelope and `Pong` shape) lower.
    #[test]
    fn a_strict_subset_overlap_and_one_fallback_lower() {
        let api = selection_api(
            EndpointRole::Client,
            vec![
                server("Response").fallback(),
                server("Event").matches("kind", "event"),
                server("Challenge").matches("kind", "event").matches("msg", "auth_required"),
                server("Pong").matches("kind", "resp").matches("msg", "pong"),
            ],
        );
        assert_eq!(selection_errors(&api), Vec::<String>::new());
    }

    #[test]
    fn more_than_one_fallback_in_a_direction_is_an_error() {
        let api = selection_api(
            EndpointRole::Client,
            vec![server("A").fallback(), server("B").fallback()],
        );
        assert_eq!(
            selection_errors(&api),
            ["endpoints/Echo/messages: `EchoServerMessage` has more than one fallback: A, B"]
        );
    }

    #[test]
    fn identical_selectors_are_an_error() {
        let api = selection_api(
            EndpointRole::Client,
            vec![server("A").matches("kind", "resp"), server("B").matches("kind", "resp")],
        );
        let errors = selection_errors(&api);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("endpoints/Echo/messages/B/selectors:"), "{errors:?}");
        assert!(errors[0].contains("`B` declares the same selectors as `A`"), "{errors:?}");
    }

    /// `{kind=resp, a=1}` and `{kind=resp, b=2}` can both match one frame and
    /// neither contains the other: no rule picks one.
    #[test]
    fn incomparable_overlapping_selectors_are_an_error() {
        let api = selection_api(
            EndpointRole::Client,
            vec![
                server("A").matches("kind", "resp").matches("a", "1"),
                server("B").matches("kind", "resp").matches("b", "2"),
            ],
        );
        let errors = selection_errors(&api);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("can match the same frame as `A`"), "{errors:?}");
    }

    /// A shared field with different values never matches one frame.
    #[test]
    fn contradictory_selectors_never_conflict() {
        let api = selection_api(
            EndpointRole::Client,
            vec![
                server("A").matches("kind", "resp").matches("a", "1"),
                server("B").matches("kind", "event").matches("b", "2"),
            ],
        );
        assert_eq!(selection_errors(&api), Vec::<String>::new());
    }

    #[test]
    fn a_message_without_selectors_beside_others_is_an_error() {
        for missing in [server("B"), MessageSchema { selectors: Some(vec![]), ..server("B") }] {
            let api = selection_api(
                EndpointRole::Client,
                vec![server("A").matches("kind", "resp"), missing],
            );
            assert_eq!(
                selection_errors(&api),
                ["endpoints/Echo/messages/B/selectors: `EchoServerMessage` has more than one message, so this message needs selectors or `fallback`"]
            );
        }
    }

    /// A direction's only message needs no selector: it takes every frame.
    #[test]
    fn a_sole_inbound_message_needs_no_selector() {
        let api = selection_api(EndpointRole::Client, vec![server("Audio"), client("Text"), client("Close")]);
        assert_eq!(selection_errors(&api), Vec::<String>::new());
    }

    #[test]
    fn a_fallback_with_selectors_and_a_repeated_selector_field_are_errors() {
        let api = selection_api(
            EndpointRole::Client,
            vec![
                server("A").fallback().matches("kind", "resp"),
                server("B").matches("kind", "event").matches("kind", "other"),
            ],
        );
        assert_eq!(
            selection_errors(&api),
            [
                "endpoints/Echo/messages/A/selectors: a fallback cannot also declare selectors",
                "endpoints/Echo/messages/B/selectors: selector field `kind` is declared more than once",
            ]
        );
    }

    /// Selection is checked only where the endpoint receives: a client
    /// endpoint never decodes client messages, so two unselected client
    /// messages are fine, but a host receives them and must select one.
    #[test]
    fn selection_rules_apply_to_the_directions_the_role_receives() {
        let messages = vec![server("Reply"), client("A"), client("B")];
        assert_eq!(selection_errors(&selection_api(EndpointRole::Client, messages.clone())), Vec::<String>::new());
        let host_errors = selection_errors(&selection_api(EndpointRole::Host, messages.clone()));
        assert_eq!(host_errors.len(), 2, "{host_errors:?}");
        assert!(host_errors.iter().all(|e| e.contains("`EchoClientMessage` has more than one message")), "{host_errors:?}");
        assert_eq!(selection_errors(&selection_api(EndpointRole::Both, messages)).len(), 2);
    }

    /// A `Bidirectional` message belongs to both enums.
    #[test]
    fn a_bidirectional_message_is_checked_in_both_directions() {
        let shared = MessageSchema::new("Event", MessageDirection::Bidirectional, Schema::new("serde_json::Value"))
            .matches("kind", "event");
        let api = selection_api(
            EndpointRole::Both,
            vec![shared, server("Other").matches("kind", "event"), client("Request").matches("kind", "req")],
        );
        let errors = selection_errors(&api);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("in `EchoServerMessage`, `Other` declares the same selectors as `Event`"), "{errors:?}");
    }

    #[test]
    fn message_names_are_unique_identifiers() {
        let api = selection_api(
            EndpointRole::Client,
            vec![server("type").fallback(), client("Text"), client("Text")],
        );
        assert_eq!(
            selection_errors(&api),
            [
                "endpoints/Echo/messages/type: a message name becomes an enum variant and must be a Rust identifier",
                "endpoints/Echo/messages/Text: message name is declared more than once",
            ]
        );
    }

    #[test]
    fn raw_and_mixed_framing_are_named_errors() {
        for format in [FrameFormat::RawBinary, FrameFormat::Mixed] {
            let mut api = minimal_api();
            api.runtime = Some(WebSocketRuntimeHints { frame_format: format, ..WebSocketRuntimeHints::default() });
            let errors = selection_errors(&api);
            assert_eq!(errors.len(), 1, "{errors:?}");
            assert!(errors[0].starts_with(&format!("runtime/frame_format: frame_format `{format}` is not supported")), "{errors:?}");
        }
        for format in [FrameFormat::JsonText, FrameFormat::JsonBinary] {
            let mut api = minimal_api();
            api.runtime = Some(WebSocketRuntimeHints { frame_format: format, ..WebSocketRuntimeHints::default() });
            assert_eq!(selection_errors(&api), Vec::<String>::new());
        }
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
