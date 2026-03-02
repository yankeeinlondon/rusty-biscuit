# Schematic WebSocket Runtime Design

## Status

Proposed design for first-class WebSocket runtime generation in the Schematic ecosystem.

Scope of this document:

- Review of `tokio-tungstenite` fit for Schematic
- Review of current `schematic-define` and `schematic-gen` WS primitives
- Detailed runtime architecture and generated API design
- Concrete implementation plan across `schematic-define`, `schematic-gen`, and `schematic-schema`

Non-goals for v1:

- Multiplexing multiple endpoints over one physical socket when the provider does not support it
- Durable event replay protocol (server-side feature)
- Guaranteeing exactly-once semantics across reconnects

---

## 1. Current State Review

## 1.1 `tokio-tungstenite` review

`tokio-tungstenite` (`0.28.x`) is the right transport layer for Schematic WS runtime because it provides:

- Async client and server handshakes (`connect_async*`, `accept_async*`, `accept_hdr_async*`)
- Stream/Sink based full-duplex API (`WebSocketStream`)
- Custom request handshake headers via `IntoClientRequest`
- Optional `WebSocketConfig` for read/write buffers and message limits
- Optional Nagle disable at connect (`connect_async_with_config(..., disable_nagle)`)
- TLS variants suitable for `wss://` (`rustls-tls-native-roots`, `rustls-tls-webpki-roots`, `native-tls`)

Operational caveats that must be designed in:

- `wss://` requires TLS feature selection at compile-time
- Reader can block forever without explicit receive timeout policy
- We must handle control frames (`Ping`, `Pong`, `Close`) predictably
- `tungstenite::Error` variants are nuanced (`ConnectionClosed`, `AlreadyClosed`, `WriteBufferFull`, protocol and TLS errors)

Conclusion:

- Use `tokio-tungstenite` as the transport foundation.
- Wrap it behind generated typed runtime APIs so end users rarely interact with tungstenite directly.

## 1.2 `schematic-define` review

Current WS primitives are strong for schema-level modeling:

- `WebSocketApi`
- `WebSocketEndpoint`
- `ConnectionParam`
- `MessageSchema`
- `ConnectionLifecycle`
- `MessageDirection`

Strengths:

- Captures endpoint paths, lifecycle messages, message directions, and auth strategy reuse from REST
- Adequate for manual definition and basic import artifacts

Gaps for runtime generation:

- No runtime-level metadata for correlation strategy (`id`/`req_id` mapping details)
- No transport framing metadata (text JSON vs binary vs mixed)
- No reconnect policy hints
- No explicit server-role definition model for generated host runtimes

## 1.3 `schematic-gen` review

Current state:

- AsyncAPI import exists (`import-asyncapi`) and emits `WebSocketApi` + `ModelCatalog`
- Generated output currently only emits WS definition helper modules
- No WS runtime codegen path exists yet

Strengths:

- Existing import CLI and diagnostics path can be extended
- Existing codegen architecture (module assembly, shared runtime module, validation/format/write flow) is reusable

Gaps:

- No WS model lowering stage (definition -> runtime plan)
- No WS shared runtime module generation
- No endpoint-specific WS client/host generation
- AsyncAPI importer currently maps many payloads to fallback JSON aliases; this is insufficient for strongly typed runtime clients

---

## 2. Design Goals

1. Generate typed WebSocket clients from `WebSocketApi` definitions with minimal handwritten runtime code.
2. Support both roles:
   1. Outbound client runtime (connects to WS server)
   2. Inbound host runtime (acts as WS server) for integration/server-role APIs
3. Preserve current Schematic ergonomics:
   1. `new()`/`with_base_url()` style constructors
   2. variant builder pattern
   3. shared error model style
4. Provide predictable runtime behavior under failure:
   1. timeouts
   2. reconnect policy
   3. bounded channels and backpressure
5. Keep runtime transport details encapsulated and testable.

---

## 3. Proposed Architecture

## 3.1 Package boundaries

- `schematic-define`
  - Remains source-of-truth for WS API definitions.
  - Gains optional WS runtime metadata structs (transport-agnostic, serializable).

- `schematic-gen`
  - Adds WS lowering + codegen pipeline.
  - Generates typed WS runtime modules and shared WS support module.

- `schematic-schema`
  - Receives generated WS clients/hosts plus `ws_shared.rs`.

## 3.2 Generated module layout (target)

`schematic/schema/src/`:

- `ws_shared.rs` (new, generated)
- `elevenlabs_ws.rs` (upgraded: typed runtime client + current helpers)
- `unfolded_circle_core_ws.rs` (upgraded)
- `unfolded_circle_dock_ws.rs` (upgraded)
- `unfolded_circle_integration_ws.rs` (upgraded; includes host role)

`lib.rs`/`prelude.rs` updates:

- Export generated WS runtime clients/hosts
- Keep definition helper exports for backwards compatibility in initial migration

---

## 4. Runtime Model

## 4.1 Core runtime types (generated into `ws_shared.rs`)

`WsError`:

- Transport errors (`tungstenite::Error`)
- Serialization/deserialization errors (`serde_json::Error`)
- Auth failures
- Timeout errors
- Disconnected/shutdown errors
- Protocol mismatch/correlation errors
- Backpressure errors (send queue full)

`WsClientOptions`:

- handshake timeout
- receive timeout
- request timeout default
- max pending requests
- outbound channel capacity
- inbound event channel capacity
- optional reconnect policy
- optional `WebSocketConfig`
- `disable_nagle`

`ReconnectPolicy`:

- enabled
- initial backoff
- max backoff
- multiplier
- jitter ratio
- max attempts (optional)

`WsConnectionState`:

- `Disconnected`
- `Connecting`
- `Authenticating`
- `Ready`
- `Closing`
- `Closed`

`WsTransportHandle`:

- writer command channel (`mpsc`)
- task join handles
- shutdown signal
- state watch channel

## 4.2 Concurrency model

Per active connection:

- One reader task:
  - Reads tungstenite frames
  - Converts frame -> decoded envelope/message
  - Routes to pending request map or event stream
  - Handles `Ping`/`Pong`/`Close`

- One writer task:
  - Consumes bounded outbound command queue
  - Serializes typed outbound messages to WS frames
  - Flushes close and shutdown semantics

- Optional keepalive task:
  - App-level heartbeat messages if endpoint lifecycle specifies keepalive
  - Not required when provider uses only WS protocol ping/pong

Shared state:

- `Arc<AtomicU64>` request id generator
- `Arc<Mutex<HashMap<u64, oneshot::Sender<...>>>>` pending requests
- `broadcast` or `mpsc` stream for events

Design constraints:

- All queues bounded
- Connection drop clears pending map with deterministic error (`WsError::Disconnected`)

## 4.3 Message framing and codec

v1 framing rules:

- Default outbound frame: `Message::Text` JSON for schema-defined messages
- Allow binary frame passthrough for APIs requiring raw binary payloads
- Inbound:
  - `Text` => JSON decode path
  - `Binary` => endpoint-configurable binary decode path or raw bytes event
  - Control frames handled internally

Generated codec traits:

- `WsEncode` for outbound types
- `WsDecode` for inbound types

Generated per-endpoint enums:

- `XxxWsOutbound`
- `XxxWsInbound`
- Optional split enums:
  - `XxxWsResponse`
  - `XxxWsEvent`

## 4.4 Correlation model (request-response)

Runtime supports envelope-based RPC correlation where available.

Mechanism:

1. Generate request id (`u64` monotonic)
2. Insert `oneshot::Sender` into pending map
3. Send outbound request with id field
4. Reader receives response with req_id field
5. Resolve and remove pending entry

Timeout/cancel semantics:

- Per call timeout override or default request timeout
- On timeout:
  - pending entry removed
  - late response discarded or surfaced as unmatched response metric

For APIs without request/response correlation:

- Runtime exposes fire-and-forget send + inbound event stream only

---

## 5. Auth Design

## 5.1 Auth source of truth

Reuse `AuthStrategy` + env mapping patterns from REST.

v1 auth methods:

- Header-based during handshake:
  - Bearer
  - API key header
  - Basic
- Message-based fallback:
  - when server emits auth challenge message (`auth_required` style)
  - generated/auth hook composes and sends auth message

## 5.2 Auth state machine

Flow:

1. Connect handshake with headers if configured
2. Enter `Authenticating`
3. If auth challenge arrives, send generated auth message
4. Wait for auth success/failure message or ready signal
5. Transition to `Ready`

Failure handling:

- auth timeout => `WsError::AuthTimeout`
- negative auth result => `WsError::AuthRejected`

For APIs with optional auth fallback (for example Unfolded Circle patterns), runtime keeps both strategies available.

---

## 6. Connection Lifecycle and Reconnect

## 6.1 Connect path

Generated client methods:

- `connect_<endpoint>(params, options)`
- `connect_with_url(url_override, ...)`

Connect implementation:

- Build endpoint URL from base URL + path template + connection params
- Build handshake request (`IntoClientRequest`)
- Apply auth headers and optional user-agent
- `connect_async_with_config` with optional tungstenite config and nodelay

## 6.2 Reconnect behavior

When enabled:

- Connection loop owns dial/session lifecycle
- Backoff with jitter
- On reconnect:
  - re-run auth flow
  - re-establish reader/writer tasks
  - publish state transitions

In-flight requests on disconnect:

- Fail immediately with `WsError::Disconnected`
- Caller decides retry policy

Re-subscription strategy:

- Generated endpoint-level hook `on_reconnect_resubscribe()` for APIs that require explicit subscribe commands

---

## 7. Generated API Surface

## 7.1 Client API shape

For each `WebSocketEndpoint`:

- Connect type: `XxxEndpointClient`
- Outbound methods:
  - `send(message)`
  - `request(message) -> Result<Response, WsError>` when correlation exists
- Inbound:
  - `next_event()` helper
  - `events()` stream adapter
- Lifecycle:
  - `close()`
  - `state()` watcher

Top-level API type per WS API:

- `XxxWs` with constructors:
  - `new()`
  - `with_base_url(...)`
  - variant builder style

## 7.2 Host API shape (server role)

For server-role APIs:

- `XxxWsHost` with:
  - `serve(listener, handler)`
  - `serve_addr(addr, handler)` convenience

Generated handler trait:

- Per operation typed method signatures
- Return typed response enums

Event push API:

- `ConnectionHandle::send_event(...)`
- `HostHandle::broadcast_event(...)`

Initial host target:

- `unfolded_circle_integration_ws`

---

## 8. Required Definition-Layer Enhancements (`schematic-define`)

Add optional runtime hints to WS primitives (non-breaking via defaults):

- `WebSocketApi.runtime: Option<WebSocketRuntimeHints>`
- `WebSocketEndpoint.runtime: Option<WebSocketEndpointHints>`

Suggested hint fields:

`WebSocketRuntimeHints`:

- `frame_format: JsonText | JsonBinary | RawBinary | Mixed`
- `supports_reconnect: bool`
- `request_id_type: U64 | String`

`WebSocketEndpointHints`:

- `correlation: Option<CorrelationHints>`
- `auth_flow: Option<AuthFlowHints>`
- `heartbeat: Option<HeartbeatHints>`

`CorrelationHints`:

- request id field path
- response id field path
- timeout defaults

`AuthFlowHints`:

- challenge message name
- auth request message schema name
- success/failure response identification rules

If hints are absent, generator falls back to conservative generic behavior.

---

## 9. Generator Design (`schematic-gen`)

## 9.1 New generation phases

1. WS validation phase
   - structural validation of endpoints/messages
   - detect correlation-capable envelopes
2. WS lowering phase
   - convert `WebSocketApi` + models into `WsRuntimePlan`
3. WS codegen phase
   - generate `ws_shared` runtime module
   - generate per-API WS client/host modules
4. Existing validation/format/write phase reuse

## 9.2 New codegen modules

Proposed `schematic/gen/src/ws_codegen/`:

- `shared.rs` (runtime shared types generator)
- `client.rs` (client type generation)
- `host.rs` (host generation)
- `codec.rs` (serialize/deserialize helpers)
- `routing.rs` (message routing and correlation)
- `docs.rs` (module docs)

## 9.3 AsyncAPI importer upgrades

Current importer heavily falls back to `JsonValue` aliases.

Required upgrades:

- Preserve object field structure for component schemas
- Better oneOf mapping for discriminated unions
- Capture message traits needed for correlation/auth hints
- Emit importer diagnostics when runtime-critical metadata is ambiguous

---

## 10. Dependency Strategy (`schematic-schema`)

Add runtime dependencies for generated WS clients:

- `tokio-tungstenite = { version = "0.28", features = ["rustls-tls-native-roots"] }`
- `futures-util = "0.3"`
- `tokio-stream = "0.1"` (if stream adapters are generated)

Keep existing dependencies:

- `tokio`
- `serde`, `serde_json`
- `thiserror`

Optional future feature gates:

- `ws-native-tls`
- `ws-rustls-webpki`
- `ws-host`

---

## 11. Testing Strategy

## 11.1 Unit tests (generator)

- Runtime plan extraction from `WebSocketApi`
- Correlation hint inference
- Auth-flow hint inference
- Generated syntax validation + snapshot tests

## 11.2 Runtime integration tests

Use tokio-based local WS test servers with `tokio-tungstenite::accept_async`:

- Happy-path connect/send/receive
- Correlated request-response timeout behavior
- Auth challenge fallback flow
- Ping/pong + close handling
- Reconnect and state transitions
- Host-mode request dispatch for integration APIs

## 11.3 Contract tests for known APIs

- ElevenLabs WS
- Unfolded Circle core/dock/integration WS

Each contract test validates:

- handshake headers
- expected envelope routing
- schema decode behavior

---

## 12. Rollout Plan

Phase 1:

- Introduce `ws_shared` runtime + generic client scaffolding
- Upgrade generated WS modules for helper + runtime coexistence

Phase 2:

- Correlated request-response generation
- Auth fallback flows

Phase 3:

- Reconnect support + resubscribe hooks
- Better AsyncAPI schema fidelity

Phase 4:

- Host runtime generation for server-role APIs
- End-to-end contract tests for integration host flow

---

## 13. Risks and Mitigations

Risk: oversized generated enums for large AsyncAPI surfaces.

- Mitigation: endpoint/channel-level feature flags or split modules.

Risk: ambiguous AsyncAPI oneOf payloads reducing type safety.

- Mitigation: diagnostics + explicit fallback wrappers (`UnknownMessage`, raw payload).

Risk: runtime memory growth from unbounded queues and pending maps.

- Mitigation: bounded channels, max pending requests, explicit overflow errors.

Risk: transport differences across providers (text vs binary payload conventions).

- Mitigation: per-endpoint framing hints with conservative defaults.

---

## 14. Acceptance Criteria

1. `schematic-gen` can generate compile-valid WS runtime modules for existing WS definitions.
2. Generated clients can:
   1. connect with auth headers
   2. send typed messages
   3. receive typed messages
   4. close cleanly
3. Correlation-enabled APIs support `request()` with timeout and cancellation semantics.
4. At least one server-role API (`unfolded_circle_integration_ws`) has generated host runtime and dispatcher trait.
5. `just test` passes for `schematic` workspace with new WS tests included.

---

## Appendix A: Recommended Initial Generated Interfaces

```rust
pub struct UnfoldedCircleCoreWs {
    // config, auth, shared options
}

impl UnfoldedCircleCoreWs {
    pub fn new() -> Self;
    pub fn with_base_url(base_url: impl Into<String>) -> Self;

    pub async fn connect_core_ws(
        &self,
        options: WsClientOptions,
    ) -> Result<CoreWsEndpointClient, WsError>;
}

pub struct CoreWsEndpointClient {
    // transport handle + typed routing state
}

impl CoreWsEndpointClient {
    pub async fn request(
        &self,
        req: CoreWsRequestEnvelope,
    ) -> Result<CoreWsResponseEnvelope, WsError>;

    pub async fn send(&self, msg: CoreWsRequestEnvelope) -> Result<(), WsError>;

    pub async fn next_event(&mut self) -> Option<Result<CoreWsEventEnvelope, WsError>>;

    pub async fn close(&self) -> Result<(), WsError>;
}
```

```rust
pub struct UnfoldedCircleIntegrationWsHost {
    // listener + config
}

#[async_trait::async_trait]
pub trait IntegrationWsHandler: Send + Sync + 'static {
    async fn handle_request(
        &self,
        req: IntegrationWsRequestEnvelope,
    ) -> Result<IntegrationWsResponseEnvelope, WsError>;
}
```

These interfaces are intentionally aligned with existing Schematic client ergonomics while exposing WS-specific lifecycle controls.
