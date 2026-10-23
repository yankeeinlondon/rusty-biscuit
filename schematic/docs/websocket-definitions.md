---
kind: canonical-documentation
---

# WebSocket definitions

A `WebSocketApi` describes a WebSocket API the way a `RestApi` describes an
HTTP one: a base URL, authentication, and endpoints. `schematic-gen` turns it
into a module of `schematic-schema` with a client per endpoint and, where the
endpoint asks for it, a host (server). This page covers authoring the
definition in `schematic-define`.

Everything a handshake shares with an HTTP request uses the REST vocabulary:
`AuthStrategy`, `AuthPolicy`, `EnvMapping`, constant headers, and
`EndpointParams` of `ParamDef`s. There is no WebSocket-only parameter type.

## A complete definition

```rust
use schematic_define::websocket::*;
use schematic_define::{
    ApiKeyEnv, AuthStrategy, EndpointParams, EnvList, EnvMapping, ParamDef, QueryParamType, Schema,
};

let api = WebSocketApi {
    name: "ExampleWs".to_string(),
    description: "Example streaming API".to_string(),
    base_url: "wss://stream.example.com".to_string(),
    docs_url: None,
    auth: AuthStrategy::ApiKey { header: "X-Api-Key".to_string(), value_prefix: None },
    auth_policy: None,
    env_auth: vec!["EXAMPLE_API_KEY".to_string()],
    env_mapping: Some(EnvMapping {
        api_key: Some(ApiKeyEnv {
            names: EnvList::single("EXAMPLE_API_KEY"),
            header: "X-Api-Key".to_string(),
        }),
        ..EnvMapping::default()
    }),
    headers: vec![("X-Client".to_string(), "schematic".to_string())],
    version: None,
    endpoints: vec![WebSocketEndpoint {
        id: "Rooms".to_string(),
        path: "/rooms/{room}".to_string(),
        description: "Room events".to_string(),
        params: EndpointParams {
            path: vec![ParamDef::path("room", QueryParamType::String)],
            query: vec![ParamDef::new("since", QueryParamType::Integer)],
            ..EndpointParams::default()
        },
        role: EndpointRole::Client,
        messages: vec![
            MessageSchema::new("Ping", MessageDirection::Client, Schema::new("PingMessage"))
                .matches("type", "ping")
                .responds_with("Pong"),
            MessageSchema::new("Pong", MessageDirection::Server, Schema::new("PongMessage"))
                .matches("type", "pong"),
            MessageSchema::new("Event", MessageDirection::Server, Schema::new("EventMessage"))
                .fallback(),
        ],
        ..WebSocketEndpoint::default()
    }],
    runtime: None,
};
```

The five shipped definitions in `schematic-definitions` (ElevenLabs, Samsung
remote, and the Unfolded Circle Core, Dock, and Integration APIs) follow this
shape and are checked by its `ws_conformance` tests.

## Roles and directions

An endpoint's `role` says which side `schematic-gen` emits:

| `role` | Serialized | Generated |
|---|---|---|
| `EndpointRole::Client` (the default) | `"client"` | a client that dials the endpoint |
| `EndpointRole::Host` | `"host"` | a host that accepts connections |
| `EndpointRole::Both` | `"both"` | both |

The role is always declared, never inferred from the messages. A message's
`direction` is **client-relative** whatever the role: `Client` messages are
sent by the client and received by a host, `Server` messages the reverse, and
`Bidirectional` messages flow both ways. Use `Bidirectional` only for a
message that really travels both ways, such as the Integration API's event
envelope.

```rust
use schematic_define::websocket::EndpointRole;

// The Integration driver hosts the channel; the remote connects to it.
let role = EndpointRole::Both;
```

## Selecting an inbound message

An inbound frame is matched to one declared message by its **selectors**:
equality checks on existing top-level wire fields, written with `.matches`.
All of a message's selectors must hold. One message per direction may be the
`.fallback()`, which receives frames that match no selector; a fallback
declares no selectors.

```rust
use schematic_define::websocket::{MessageDirection, MessageSchema};
use schematic_define::Schema;

let pong = MessageSchema::new("Pong", MessageDirection::Server, Schema::new("PongMessage"))
    .matches("kind", "resp")
    .matches("msg", "pong");
let response = MessageSchema::new("Response", MessageDirection::Server, Schema::new("Envelope"))
    .fallback();
```

A direction with a single inbound message needs no selector. Selector values
are JSON strings; no nested field path is supported.

**Planned:** the generated client decodes each frame as the message its
selectors choose, and generation rejects a direction with more than one
fallback, conflicting selectors, or a non-fallback message without a selector
when the direction has several messages. Today the selectors are carried
through generation unchanged and the generated client still delivers
`serde_json::Value` events.

## Linking a request to its response

`responds_with` on a request message names the message, in the same
endpoint, that answers it. Nothing is inferred from message names.

```rust
use schematic_define::websocket::{MessageDirection, MessageSchema};
use schematic_define::Schema;

let ping = MessageSchema::new("Ping", MessageDirection::Client, Schema::new("PingMessage"))
    .responds_with("Pong");
```

A link needs the endpoint's `CorrelationHints`, which name the request id
field and the response field that echoes it.

**Planned:** the generator pairs requests and responses only through these
links and hints; today it still infers correlation from messages named
`RequestEnvelope`/`ResponseEnvelope` when an endpoint declares no hints.

## Handshake parameters

`params` is the REST `EndpointParams` container:

- **path:** each `{placeholder}` in `path` is a required connect argument.
  With an empty `params.path` every placeholder is a string; otherwise
  `params.path` must name exactly the placeholders, each required, `simple`
  style, and a string, integer, number, or boolean. A substituted value is
  percent-encoded as one segment, so `a/b c?d` becomes `a%2Fb%20c%3Fd`.
- **query:** required values are fields of the endpoint's
  `{Endpoint}ConnectionParams` struct, optional ones are `Option` fields and
  are left out of the URL when unset. Scalars and arrays follow the REST
  query styles: `form` with `explode: true` repeats the name
  (`tag=a&tag=b`), `form` with `explode: false` joins with `,`, and
  `spaceDelimited`/`pipeDelimited` join with `%20`/`%7C`.
- **header** and **cookie** parameters are not generated for a handshake and
  are an error; put constant handshake headers on the API's `headers`.

The handshake URL is `base_url` followed by the substituted path, so a base
path such as `wss://host/proxy` stays a prefix. The endpoint path must start
with `/` and may not carry a query, a fragment, or an origin of its own:

```text
[ERROR] severity=error api=Example endpoint=Live feature=websocket-definition output=rust: endpoints/Live/path: path `/live?debug=1` embeds a query or fragment; declare query parameters in `params.query`
```

## Authentication and environment

`auth`, `auth_policy`, `env_auth`, `env_mapping`, and `headers` mean what
they mean on a `RestApi`. `effective_auth_policy()` and
`default_env_mapping()` return the explicit value when one is set and the
REST derivation otherwise. Shipped definitions set `env_mapping` explicitly.

A challenge exchange after the upgrade (an `auth_required` message the
client must answer) is declared as `AuthFlowHints` on the endpoint, never
only in prose:

```rust
use schematic_define::websocket::AuthFlowHints;

let flow = AuthFlowHints {
    challenge_message: "auth_required".to_string(),
    auth_request_schema: "AuthMessage".to_string(),
    success_indicator: Some("authentication".to_string()),
    failure_indicator: None,
};
```

## Runtime hints and heartbeats

`WebSocketEndpointHints` carries `correlation`, `auth_flow`, and
`heartbeat`; `WebSocketRuntimeHints` on the API carries the frame format,
reconnect support, and request id type. Every declared duration is at least
one second: a `0` for `default_timeout_secs`, `interval_secs`, or
`timeout_secs` is a generation error.

Heartbeats are caller-managed. The generated client does not send them; its
rustdoc states each declared heartbeat so the caller knows to:

```text
/// ## Heartbeat
///
/// The server expects the `Heartbeat` keepalive message (`send_heartbeat`) every 20 s, and expects the reply within 5 s. This client does not send it: schedule it from the caller.
```

A declared heartbeat produces no generation warning.

## The JSON form

`WebSocketApi` serializes with serde. Absent and explicit `null` differ:

| Field | Absent | `null` |
|---|---|---|
| `endpoint.role` | `client` | error |
| `message.responds_with`, `message.selectors` | no declaration | error |
| `message.fallback` | `false` | error |
| `endpoint.runtime`, `api.runtime`, `auth_policy`, `env_mapping`, and each hint object | no declaration | error |
| `correlation.default_timeout_secs` | `30` | error |
| `endpoint.params`, `api.headers`, selector `field`/`value` | error | error |

An explicit empty value is kept as declared (`"selectors": []`,
`"runtime": {}`). A struct-valued field must be a JSON object: an array in
its place is an error, as are duplicate keys and trailing content. Empty
strings and zero durations load, and generation reports them as errors at
their definition path (for example
`endpoints/Live/runtime/heartbeat/interval_secs`).
