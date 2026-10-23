//! Every shipped WebSocket definition is declared on the shared vocabulary:
//! true client-relative directions and explicit roles (checked against the
//! table below, never inferred from `Bidirectional`), challenge flows as
//! `AuthFlowHints`, response links that resolve, correlation only where a
//! link exists, inbound selectors that can choose one message, and an
//! explicit environment mapping.

use schematic_define::websocket::{
    EndpointRole, MessageDirection, MessageSchema, WebSocketApi, WebSocketEndpoint,
};
use schematic_define::RestApi;
use schematic_definitions::{
    define_elevenlabs_websocket_api, define_samsung_smart_tv_remote_ws_api,
    define_unfolded_circle_core_ws_api, define_unfolded_circle_dock_ws_api,
    define_unfolded_circle_integration_ws_api,
};

fn shipped() -> Vec<WebSocketApi> {
    vec![
        define_elevenlabs_websocket_api(),
        define_unfolded_circle_core_ws_api(),
        define_unfolded_circle_dock_ws_api(),
        define_unfolded_circle_integration_ws_api(),
        define_samsung_smart_tv_remote_ws_api(),
    ]
}

use MessageDirection::{Bidirectional as B, Client as C, Server as S};

const CORE: &[(&str, MessageDirection)] = &[
    ("RequestEnvelope", C), ("ResponseEnvelope", S), ("EventEnvelope", S),
    ("AuthRequired", S), ("Auth", C), ("Authentication", S), ("Ping", C), ("Pong", S),
    ("Version", C), ("VersionInfo", S), ("System", C), ("SystemInfo", S), ("Result", S),
];

/// Messages of one endpoint in declaration order.
type Messages = &'static [(&'static str, MessageDirection)];

/// One endpoint's expectation, in comparable owned form.
type Row = (String, String, EndpointRole, Vec<(String, MessageDirection)>);

/// `(api, endpoint, role, messages in declaration order)`.
const EXPECTED: &[(&str, &str, EndpointRole, Messages)] = &[
    ("ElevenLabsTTS", "TextToSpeech", EndpointRole::Client, &[("TextChunk", C), ("AudioChunk", S)]),
    ("ElevenLabsTTS", "MultiContextTextToSpeech", EndpointRole::Client,
        &[("TextChunk", C), ("CloseContext", C), ("AudioChunk", S)]),
    ("UnfoldedCircleCoreWs", "CoreWs", EndpointRole::Client, CORE),
    ("UnfoldedCircleCoreWs", "CoreIntegrations", EndpointRole::Client, CORE),
    ("UnfoldedCircleCoreWs", "CoreProfiles", EndpointRole::Client, CORE),
    ("UnfoldedCircleCoreWs", "CoreEvents", EndpointRole::Client, CORE),
    ("UnfoldedCircleDockWs", "DockRoot", EndpointRole::Client, &[
        ("RequestEnvelope", C), ("ResponseEnvelope", S), ("EventEnvelope", S),
        ("AuthRequired", S), ("Auth", C), ("Authentication", S),
    ]),
    // The driver hosts the channel; the generated host is required by the
    // homelab integration drivers.
    ("UnfoldedCircleIntegrationWs", "Integration", EndpointRole::Both, &[
        ("RequestEnvelope", C), ("ResponseEnvelope", S), ("EventEnvelope", B),
        ("AuthRequired", S), ("Auth", C), ("Authentication", S),
    ]),
    ("SamsungSmartTvRemote", "RemoteControl", EndpointRole::Client, &[
        ("RemoteControlCommand", C), ("ChannelConnectEvent", S), ("ChannelUnauthorizedEvent", S),
        ("ChannelErrorEvent", S), ("ChannelEnvelope", S), ("AppLaunchCommand", C),
        ("InstalledAppsCommand", C),
    ]),
    ("SamsungSmartTvRemote", "ArtMode", EndpointRole::Client, &[
        ("ArtModeCommand", C), ("ArtModeEvent", S), ("ChannelConnectEvent", S),
        ("ChannelEnvelope", S),
    ]),
];

fn endpoints() -> Vec<(String, WebSocketEndpoint)> {
    shipped()
        .into_iter()
        .flat_map(|api| {
            let name = api.name.clone();
            api.endpoints.into_iter().map(move |ep| (name.clone(), ep))
        })
        .collect()
}

#[test]
fn roles_and_directions_match_the_expected_table() {
    let actual: Vec<Row> = endpoints()
        .into_iter()
        .map(|(api, ep)| {
            let messages = ep.messages.iter().map(|m| (m.name.clone(), m.direction)).collect();
            (api, ep.id, ep.role, messages)
        })
        .collect();
    let expected: Vec<Row> = EXPECTED
        .iter()
        .map(|(api, id, role, messages)| {
            let messages = messages.iter().map(|(name, d)| (name.to_string(), *d)).collect();
            (api.to_string(), id.to_string(), *role, messages)
        })
        .collect();
    assert_eq!(actual, expected);
}

/// Messages the client receives (`Server`/`Bidirectional`) or, for a
/// generated host, the messages it receives (`Client`/`Bidirectional`).
fn inbound(ep: &WebSocketEndpoint, to_client: bool) -> Vec<&MessageSchema> {
    ep.messages
        .iter()
        .filter(|m| match m.direction {
            B => true,
            S => to_client,
            C => !to_client,
        })
        .collect()
}

#[test]
fn every_inbound_direction_can_choose_one_message() {
    for (api, ep) in endpoints() {
        let mut directions = vec![true];
        if ep.role != EndpointRole::Client {
            directions.push(false);
        }
        for to_client in directions {
            let messages = inbound(&ep, to_client);
            let fallbacks: Vec<&str> =
                messages.iter().filter(|m| m.fallback).map(|m| m.name.as_str()).collect();
            assert!(fallbacks.len() <= 1, "{api}/{}: fallbacks {fallbacks:?}", ep.id);
            for message in &messages {
                let at = format!("{api}/{}/{}", ep.id, message.name);
                if message.fallback {
                    assert!(message.selectors.is_none(), "{at}: a fallback declares no selector");
                } else if messages.len() > 1 {
                    let selectors = message.selectors.as_deref().unwrap_or_default();
                    assert!(!selectors.is_empty(), "{at}: needs a selector");
                    assert!(selectors.iter().all(|s| !s.field.is_empty()), "{at}");
                }
            }
            // No two messages declare the same selector set.
            for (index, first) in messages.iter().enumerate() {
                for second in &messages[index + 1..] {
                    if first.selectors.is_some() {
                        assert_ne!(first.selectors, second.selectors, "{api}/{}", ep.id);
                    }
                }
            }
        }
    }
}

#[test]
fn every_response_link_resolves_and_is_correlated() {
    for (api, ep) in endpoints() {
        let links: Vec<&MessageSchema> =
            ep.messages.iter().filter(|m| m.responds_with.is_some()).collect();
        let correlation = ep.runtime.as_ref().and_then(|r| r.correlation.as_ref());
        assert_eq!(
            correlation.is_some(),
            !links.is_empty(),
            "{api}/{}: correlation hints exactly where a response link exists",
            ep.id
        );
        for request in links {
            let at = format!("{api}/{}/{}", ep.id, request.name);
            assert_ne!(request.direction, S, "{at}: a server message cannot be a request");
            let target = request.responds_with.as_deref().unwrap();
            let response = ep
                .messages
                .iter()
                .find(|m| m.name == target)
                .unwrap_or_else(|| panic!("{at}: `{target}` is not in the endpoint"));
            assert_ne!(response.direction, C, "{at}: the client cannot receive `{target}`");
        }
    }
}

#[test]
fn challenge_flows_are_declared_as_hints_not_prose() {
    for (api, ep) in endpoints() {
        let at = format!("{api}/{}", ep.id);
        let challenge = ep.messages.iter().find(|m| m.name == "AuthRequired");
        let flow = ep.runtime.as_ref().and_then(|r| r.auth_flow.as_ref());
        assert_eq!(challenge.is_some(), flow.is_some(), "{at}");
        let (Some(challenge), Some(flow)) = (challenge, flow) else { continue };
        let wire_values: Vec<&str> = challenge
            .selectors
            .iter()
            .flatten()
            .map(|s| s.value.as_str())
            .collect();
        assert!(wire_values.contains(&flow.challenge_message.as_str()), "{at}: {wire_values:?}");
        assert!(
            ep.messages
                .iter()
                .any(|m| m.direction != S && m.schema.type_name == flow.auth_request_schema),
            "{at}: `{}` is not a message the client sends",
            flow.auth_request_schema
        );
    }
}

#[test]
fn no_protocol_declares_a_heartbeat_it_does_not_document() {
    // None of the five documents an application heartbeat interval: the
    // Core `ping` is an ordinary request, and Integration keepalive uses
    // WebSocket control frames. A declared heartbeat would be invented.
    for (api, ep) in endpoints() {
        let heartbeat = ep.runtime.as_ref().and_then(|r| r.heartbeat.as_ref());
        assert!(heartbeat.is_none(), "{api}/{}", ep.id);
    }
}

#[test]
fn environment_mapping_is_explicit_and_matches_the_handshake_strategy() {
    for api in shipped() {
        let explicit = api.env_mapping.clone().unwrap_or_else(|| panic!("{}: implicit", api.name));
        assert_eq!(explicit, RestApi::legacy_env_mapping_for(&api.auth, &api.env_auth), "{}", api.name);
        assert!(api.headers.is_empty(), "{}", api.name);
    }
}

#[test]
fn path_parameters_type_every_placeholder() {
    for (api, ep) in endpoints() {
        let placeholders: Vec<&str> = ep
            .path
            .split('{')
            .skip(1)
            .filter_map(|rest| rest.split_once('}').map(|(name, _)| name))
            .collect();
        let declared: Vec<&str> = ep.params.path.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(declared, placeholders, "{api}/{}", ep.id);
        assert!(ep.params.header.is_empty() && ep.params.cookie.is_empty(), "{api}/{}", ep.id);
    }
}
