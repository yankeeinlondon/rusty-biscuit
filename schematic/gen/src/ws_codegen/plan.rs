//! WS runtime plan: lowering `WebSocketApi` into concrete generation decisions.

use schematic_define::AuthStrategy;
use schematic_define::websocket::{
    AuthFlowHints, FrameFormat, HeartbeatHints, MessageDirection, ParamType, RequestIdType,
    WebSocketApi, WebSocketEndpoint,
};

use crate::errors::GeneratorError;
use crate::parser::extract_path_params;

/// Diagnostic emitted during WS validation/lowering.
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
}

/// Role of the generated code for an endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointRole {
    /// Outbound client connecting to a WS server.
    Client,
    /// Inbound host acting as a WS server.
    Host,
    /// Both client and host roles (bidirectional APIs).
    Both,
}

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

/// Typed plan for a required path parameter.
#[derive(Debug, Clone)]
pub struct PathParamPlan {
    pub name: String,
    pub param_type: ParamType,
    pub description: Option<String>,
}

/// Typed plan for a query parameter.
#[derive(Debug, Clone)]
pub struct QueryParamPlan {
    pub name: String,
    pub param_type: ParamType,
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

/// Infer the endpoint role from message directions.
fn infer_role(endpoint: &WebSocketEndpoint) -> EndpointRole {
    let all_bidi = !endpoint.messages.is_empty()
        && endpoint
            .messages
            .iter()
            .all(|m| m.direction == MessageDirection::Bidirectional);

    if all_bidi {
        EndpointRole::Both
    } else {
        EndpointRole::Client
    }
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
pub fn validate_ws_api(
    api: &WebSocketApi,
    diagnostics: &mut Vec<WsDiagnostic>,
) -> Result<(), GeneratorError> {
    if api.name.is_empty() {
        return Err(GeneratorError::ParseError(
            "WebSocket API name cannot be empty".to_string(),
        ));
    }

    if api.base_url.is_empty() {
        return Err(GeneratorError::ParseError(format!(
            "WebSocket API '{}' has empty base_url",
            api.name
        )));
    }

    if api.endpoints.is_empty() {
        diagnostics.push(WsDiagnostic::warn(
            &api.name,
            "No endpoints defined; generated client will be empty",
        ));
    }

    for endpoint in &api.endpoints {
        if endpoint.id.is_empty() {
            return Err(GeneratorError::ParseError(format!(
                "WebSocket API '{}' has endpoint with empty id",
                api.name
            )));
        }

        if endpoint.messages.is_empty() {
            diagnostics.push(WsDiagnostic::warn(
                format!("endpoints/{}", endpoint.id),
                "Endpoint has no messages defined",
            ));
        }

        // Check correlation hint consistency
        if let Some(ref runtime) = endpoint.runtime
            && let Some(ref corr) = runtime.correlation
            && (corr.request_id_field.is_empty() || corr.response_id_field.is_empty())
        {
            return Err(GeneratorError::ParseError(format!(
                "Endpoint '{}' has correlation hints with empty field paths",
                endpoint.id
            )));
        }
    }

    Ok(())
}

/// Lower a `WebSocketApi` definition into a `WsRuntimePlan`.
pub fn lower_to_plan(api: &WebSocketApi) -> Result<WsRuntimePlan, GeneratorError> {
    let mut diagnostics = Vec::new();

    validate_ws_api(api, &mut diagnostics)?;

    let runtime = api.runtime.as_ref();
    let frame_format = runtime.map_or(FrameFormat::default(), |r| r.frame_format);
    let request_id_type = runtime.map_or(RequestIdType::default(), |r| r.request_id_type);
    let supports_reconnect = runtime.is_none_or(|r| r.supports_reconnect);

    let auth = resolve_auth(api, &mut diagnostics);

    let endpoints: Vec<EndpointPlan> = api
        .endpoints
        .iter()
        .map(|ep| {
            let role = infer_role(ep);
            let correlation = infer_correlation(ep, &mut diagnostics);
            let heartbeat = ep
                .runtime
                .as_ref()
                .and_then(|r| r.heartbeat.clone());
            let path_placeholders: Vec<String> = extract_path_params(&ep.path)
                .into_iter()
                .map(std::string::ToString::to_string)
                .collect();

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

            let mut seen_connection_params: std::collections::BTreeMap<
                &str,
                &schematic_define::websocket::ConnectionParam,
            > = std::collections::BTreeMap::new();
            for param in &ep.connection_params {
                if let Some(existing) = seen_connection_params.get(param.name.as_str())
                    && (existing.param_type != param.param_type
                        || existing.required != param.required)
                {
                    diagnostics.push(WsDiagnostic::warn(
                        format!("endpoints/{}/connection_params/{}", ep.id, param.name),
                        format!(
                            "ConnectionParam '{}' is defined multiple times with conflicting type/required metadata.",
                            param.name
                        ),
                    ));
                }
                seen_connection_params.insert(param.name.as_str(), param);
            }

            let mut path_params = Vec::new();
            for placeholder in &path_placeholders {
                let typed_param = ep.connection_params.iter().find(|p| p.name == *placeholder);
                match typed_param {
                    Some(param) => {
                        if !param.required {
                            diagnostics.push(WsDiagnostic::warn(
                                format!("endpoints/{}/path/{}", ep.id, placeholder),
                                format!(
                                    "Path placeholder '{}' is optional in connection_params; generated connect arg remains required.",
                                    placeholder
                                ),
                            ));
                        }

                        path_params.push(PathParamPlan {
                            name: placeholder.clone(),
                            param_type: param.param_type,
                            description: param.description.clone(),
                        });
                    }
                    None => {
                        diagnostics.push(WsDiagnostic::error(
                            format!("endpoints/{}/path/{}", ep.id, placeholder),
                            format!(
                                "Path placeholder '{}' has no typed ConnectionParam entry.",
                                placeholder
                            ),
                        ));
                    }
                }
            }

            let query_params: Vec<QueryParamPlan> = ep
                .connection_params
                .iter()
                .filter(|p| !path_placeholders.iter().any(|name| name == &p.name))
                .map(|p| QueryParamPlan {
                    name: p.name.clone(),
                    param_type: p.param_type,
                    required: p.required,
                    description: p.description.clone(),
                })
                .collect();

            if ep.path.contains('{') && path_params.len() < path_placeholders.len() {
                diagnostics.push(WsDiagnostic::warn(
                    format!("endpoints/{}/path", ep.id),
                    "Unknown placeholders remain after WS path planning; runtime URL may still contain `{...}`."
                        .to_string(),
                ));
            }

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
        ConnectionLifecycle, CorrelationHints, MessageSchema, ParamType, WebSocketEndpointHints,
        WebSocketRuntimeHints,
    };

    fn minimal_api() -> WebSocketApi {
        WebSocketApi {
            name: "TestApi".to_string(),
            description: "Test".to_string(),
            base_url: "wss://test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            env_auth: vec![],
            version: None,
            endpoints: vec![WebSocketEndpoint {
                id: "Echo".to_string(),
                path: "/echo".to_string(),
                description: "Echo endpoint".to_string(),
                connection_params: vec![],
                lifecycle: ConnectionLifecycle::default(),
                messages: vec![MessageSchema {
                    name: "Msg".to_string(),
                    direction: MessageDirection::Bidirectional,
                    schema: Schema::new("EchoMsg"),
                    description: None,
                }],
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
            env_auth: vec!["TEST_KEY".to_string()],
            version: None,
            endpoints: vec![WebSocketEndpoint {
                id: "Main".to_string(),
                path: "/ws".to_string(),
                description: "Main endpoint".to_string(),
                connection_params: vec![],
                lifecycle: ConnectionLifecycle::default(),
                messages: vec![
                    MessageSchema {
                        name: "RequestEnvelope".to_string(),
                        direction: MessageDirection::Client,
                        schema: Schema::new("ReqEnvelope"),
                        description: None,
                    },
                    MessageSchema {
                        name: "ResponseEnvelope".to_string(),
                        direction: MessageDirection::Server,
                        schema: Schema::new("RespEnvelope"),
                        description: None,
                    },
                    MessageSchema {
                        name: "EventEnvelope".to_string(),
                        direction: MessageDirection::Server,
                        schema: Schema::new("EvtEnvelope"),
                        description: None,
                    },
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
    fn lower_classifies_path_and_query_connection_params() {
        let mut api = minimal_api();
        api.endpoints[0].path = "/ws/{channel_id}".to_string();
        api.endpoints[0].connection_params = vec![
            schematic_define::websocket::ConnectionParam {
                name: "channel_id".to_string(),
                param_type: ParamType::String,
                required: true,
                description: Some("Channel identifier".to_string()),
            },
            schematic_define::websocket::ConnectionParam {
                name: "limit".to_string(),
                param_type: ParamType::Integer,
                required: false,
                description: Some("Optional limit".to_string()),
            },
        ];

        let plan = lower_to_plan(&api).unwrap();
        let ep = &plan.endpoints[0];
        assert_eq!(ep.path_params.len(), 1);
        assert_eq!(ep.path_params[0].name, "channel_id");
        assert_eq!(ep.query_params.len(), 1);
        assert_eq!(ep.query_params[0].name, "limit");
    }

    #[test]
    fn lower_bidirectional_role() {
        let plan = lower_to_plan(&minimal_api()).unwrap();
        assert_eq!(plan.endpoints[0].role, EndpointRole::Both);
    }

    #[test]
    fn validate_rejects_empty_name() {
        let mut api = minimal_api();
        api.name = String::new();
        assert!(lower_to_plan(&api).is_err());
    }

    #[test]
    fn validate_rejects_empty_base_url() {
        let mut api = minimal_api();
        api.base_url = String::new();
        assert!(lower_to_plan(&api).is_err());
    }

    #[test]
    fn validate_rejects_empty_endpoint_id() {
        let mut api = minimal_api();
        api.endpoints[0].id = String::new();
        assert!(lower_to_plan(&api).is_err());
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
        assert!(lower_to_plan(&api).is_err());
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
            define_elevenlabs_websocket_api, define_unfolded_circle_core_ws_api,
            define_unfolded_circle_dock_ws_api, define_unfolded_circle_integration_ws_api,
        };

        let apis = vec![
            define_elevenlabs_websocket_api(),
            define_unfolded_circle_core_ws_api(),
            define_unfolded_circle_dock_ws_api(),
            define_unfolded_circle_integration_ws_api(),
        ];

        for api in &apis {
            let plan = lower_to_plan(api).unwrap();
            assert!(!plan.api_name.is_empty());
            assert!(!plan.endpoints.is_empty());
        }
    }
}
