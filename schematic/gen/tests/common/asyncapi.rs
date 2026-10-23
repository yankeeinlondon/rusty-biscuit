//! AsyncAPI fixtures and the supported-subset projection the round trip is
//! measured against.

use schematic_define::websocket::{
    ConnectionLifecycle, EndpointRole, HeartbeatHints, MessageDirection, MessageSchema, WebSocketApi,
    WebSocketEndpoint, WebSocketEndpointHints,
};
use schematic_define::{AuthStrategy, EndpointParams, EnvList, ParamDef, ParamStyle, QueryParamType, RestApi, Schema};
use schematic_gen::asyncapi_import::{AsyncImportOptions, AsyncImportResult, WsRole, import_asyncapi_text};
use schematic_gen::export::asyncapi::{export_asyncapi, exported_messages};

pub fn message(name: &str, direction: MessageDirection, payload: &str) -> MessageSchema {
    MessageSchema::new(name, direction, Schema::new(payload)).describe(format!("The {name} message"))
}

/// One endpoint per role, overlapping payloads told apart only by
/// selectors, a fallback per inbound direction, a bidirectional message, a
/// reply, heartbeat hints with and without a timeout, typed and untyped path
/// placeholders, every query parameter shape, a constant header, and a base
/// path in the base URL.
pub fn fixture_api() -> WebSocketApi {
    use MessageDirection::{Bidirectional, Client, Server};
    let mut tags = ParamDef::new("tags", QueryParamType::Array(Box::new(QueryParamType::String)));
    tags.explode = true;
    let mut pipes = ParamDef::new("ids", QueryParamType::Array(Box::new(QueryParamType::Integer)));
    pipes.style = ParamStyle::PipeDelimited;
    let feed = WebSocketEndpoint {
        id: "Feed".into(),
        path: "/feed/{room}".into(),
        description: "Room feed".into(),
        params: EndpointParams {
            path: vec![ParamDef::path("room", QueryParamType::Integer).describes("Room number")],
            query: vec![
                ParamDef::new("limit", QueryParamType::Integer).required().describes("Most events per frame"),
                tags,
                pipes,
                ParamDef::new("mode", QueryParamType::Enum(vec!["live".into(), "replay".into()])),
                ParamDef::new("filter", QueryParamType::Json),
                ParamDef::new("ratio", QueryParamType::Number),
                ParamDef::new("verbose", QueryParamType::Boolean).deprecated(),
                ParamDef::new("token", QueryParamType::String).secret().rust_name("auth_token"),
            ],
            ..EndpointParams::default()
        },
        role: EndpointRole::Client,
        lifecycle: ConnectionLifecycle::default(),
        messages: vec![
            message("Subscribe", Client, "SubscribeFrame").matches("type", "subscribe").responds_with("Subscribed"),
            message("Subscribed", Server, "AckFrame").matches("type", "ack").matches("msg", "subscribed"),
            // Same payload type: only the selectors tell these two apart.
            message("Update", Server, "FeedEvent").matches("type", "event").matches("msg", "update"),
            message("Delete", Server, "FeedEvent").matches("type", "event").matches("msg", "delete"),
            message("Envelope", Server, "FeedEnvelope").fallback(),
            message("Ping", Bidirectional, "PingFrame").matches("type", "ping").matches("note", ""),
        ],
        runtime: Some(WebSocketEndpointHints {
            heartbeat: Some(HeartbeatHints { interval_secs: 30, timeout_secs: Some(10) }),
            ..WebSocketEndpointHints::default()
        }),
    };
    let host = WebSocketEndpoint {
        id: "Host".into(),
        path: "/host".into(),
        description: String::new(),
        role: EndpointRole::Host,
        messages: vec![
            message("Command", Client, "HostCommand").matches("op", "run"),
            message("Other", Client, "HostCommand").fallback(),
            message("Event", Server, "HostEvent"),
        ],
        runtime: Some(WebSocketEndpointHints {
            heartbeat: Some(HeartbeatHints { interval_secs: 15, timeout_secs: None }),
            ..WebSocketEndpointHints::default()
        }),
        ..WebSocketEndpoint::default()
    };
    // An explicitly empty selector list, distinct from none declared.
    let mut data = message("Data", Bidirectional, "DataFrame");
    data.selectors = Some(vec![]);
    let both = WebSocketEndpoint {
        id: "Both".into(),
        path: "/both/{session}".into(),
        description: "Untyped placeholder".into(),
        role: EndpointRole::Both,
        messages: vec![data],
        ..WebSocketEndpoint::default()
    };
    WebSocketApi {
        name: "FixtureStream".into(),
        description: "A fixture stream".into(),
        base_url: "wss://stream.example.com:8443/api/v2".into(),
        docs_url: Some("https://stream.example.com/docs".into()),
        auth: AuthStrategy::ApiKey { header: "X-Key".into(), value_prefix: Some("Token ".into()) },
        auth_policy: None,
        env_auth: vec!["FIXTURE_STREAM_API_KEY".into()],
        env_mapping: None,
        headers: vec![("X-Protocol".into(), "2".into()), ("X-Client".into(), "schematic".into())],
        version: Some("2.1.0".into()),
        endpoints: vec![feed, host, both],
        runtime: None,
    }
}

/// Import options for an in-memory import named `module`.
pub fn options(module: &str, role: WsRole) -> AsyncImportOptions {
    AsyncImportOptions {
        input: format!("{module}.asyncapi"),
        api_name: None,
        module_path: Some(module.to_string()),
        ws_role: role,
        output: String::new(),
        dry_run: true,
        strict: false,
        verbose: false,
    }
}

/// Exports `api` and imports the document again from `format` text.
pub fn round_trip(api: &WebSocketApi, module: &str, yaml: bool) -> AsyncImportResult {
    let document = export_asyncapi(api).expect("the definition exports").document;
    let text = if yaml {
        biscuit_file::serde_yaml_ng::to_string(&document).unwrap()
    } else {
        serde_json::to_string_pretty(&document).unwrap()
    };
    import_asyncapi_text(&text, &options(module, WsRole::Client)).unwrap_or_else(|e| panic!("{module}: {e}"))
}

/// Endpoints by id, messages by name, query parameters by name, headers by
/// name: AsyncAPI maps are unordered, and dispatch never depends on order.
pub fn canonical(mut api: WebSocketApi) -> WebSocketApi {
    api.headers.sort();
    api.endpoints.sort_by(|a, b| a.id.cmp(&b.id));
    for endpoint in &mut api.endpoints {
        endpoint.messages.sort_by(|a, b| a.name.cmp(&b.name));
        endpoint.params.query.sort_by(|a, b| a.name.cmp(&b.name));
    }
    api
}

/// What a supported-subset round trip keeps of `api`, imported under
/// `module`. Each rule is one documented difference, named by
/// [`subset_differences`].
pub fn subset_projection(api: &WebSocketApi, module: &str) -> WebSocketApi {
    let mut projected = api.clone();
    if let AuthStrategy::Basic { username_env, password_env } = &mut projected.auth {
        *username_env = EnvList::new(vec![]);
        *password_env = EnvList::new(vec![]);
    }
    projected.auth_policy = None;
    projected.env_auth = match projected.auth {
        AuthStrategy::None | AuthStrategy::Basic { .. } => vec![],
        _ => vec![format!("{}_API_KEY", module.to_ascii_uppercase())],
    };
    projected.env_mapping = Some(RestApi::legacy_env_mapping_for(&projected.auth, &projected.env_auth));
    projected.version = Some(api.version.clone().unwrap_or_else(|| "1.0.0".into()));
    projected.runtime = None;
    for endpoint in &mut projected.endpoints {
        let mut messages: Vec<MessageSchema> = exported_messages(endpoint).into_iter().cloned().collect();
        for message in &mut messages {
            message.schema.module_path = None;
        }
        endpoint.messages = messages;
        endpoint.lifecycle = ConnectionLifecycle::default();
        endpoint.params.header.clear();
        endpoint.params.cookie.clear();
        endpoint.runtime = endpoint
            .runtime
            .as_ref()
            .and_then(|hints| hints.heartbeat.clone())
            .map(|heartbeat| WebSocketEndpointHints { heartbeat: Some(heartbeat), ..WebSocketEndpointHints::default() });
    }
    canonical(projected)
}

/// The projection rules that change `api`: the fields outside the supported
/// subset it declares.
pub fn subset_differences(api: &WebSocketApi, module: &str) -> Vec<&'static str> {
    let projected = subset_projection(api, module);
    let mut differences = Vec::new();
    let mut note = |condition: bool, label: &'static str| {
        if condition {
            differences.push(label);
        }
    };
    note(projected.env_auth != api.env_auth || projected.env_mapping != api.env_mapping, "env metadata");
    note(projected.auth != api.auth, "basic credential variables");
    note(api.version.is_none(), "version defaults to 1.0.0");
    note(api.auth_policy.is_some(), "auth policy");
    note(api.runtime.is_some(), "API runtime hints");
    let endpoints = &api.endpoints;
    note(
        endpoints.iter().any(|e| e.lifecycle != ConnectionLifecycle::default()),
        "lifecycle messages become ordinary messages",
    );
    let hints = |pick: fn(&WebSocketEndpointHints) -> bool| endpoints.iter().any(|e| e.runtime.as_ref().is_some_and(pick));
    note(hints(|h| h.correlation.is_some()), "correlation hints");
    note(hints(|h| h.auth_flow.is_some()), "auth flow hints");
    note(endpoints.iter().any(|e| !e.params.header.is_empty() || !e.params.cookie.is_empty()), "header and cookie parameters");
    note(
        endpoints.iter().flat_map(|e| &e.messages).any(|m| m.schema.module_path.is_some()),
        "payload module paths",
    );
    differences
}
