# WebSocket definitions

`WebSocketApi` / `WebSocketEndpoint` / `MessageSchema` in
`schematic_define::websocket`. Full rules and examples:
`schematic/docs/websocket-definitions.md`.

## Contract

- **Shared vocabulary.** Handshake params are REST `EndpointParams` of
  `ParamDef`s (`params.path` typed placeholders, `params.query`); header and
  cookie params are a generation error. Auth is `auth` + optional
  `auth_policy` + `env_auth` + explicit `env_mapping` + constant `headers`,
  with REST meaning (`effective_auth_policy()`, `default_env_mapping()`).
  `ConnectionParam`/`ParamType` no longer exist.
- **Role is declared.** `WebSocketEndpoint.role`: `Client` (default,
  serialized `"client"`), `Host`, `Both`. The generator emits a host for
  `Host`/`Both` only; it never infers a role from `Bidirectional` messages.
- **Directions are client-relative** for every role. `Bidirectional` only for
  a message that truly flows both ways.
- **Selectors.** `.matches(field, value)` = top-level string equality, all must
  hold; `.fallback()` = the direction's catch-all (no selectors). Declare
  them on the documented wire values (Core/Integration `kind`+`msg`, Dock
  `type`[+`msg`], Samsung `event`/`method`). `import-asyncapi` reads
  `x-schematic-selectors` when present, else derives them from payload
  top-level string `const`s (through `$ref`/`allOf`). AsyncAPI round trip:
  see `gen.md` § AsyncAPI.
- **Response link.** `.responds_with("Name")` on a request; pair it with the
  endpoint's `CorrelationHints` (no API-level fallback). Correlation exists
  only with both; message names are never inspected. API
  `request_id_type` `String` → JSON-string ids, default `U64` → numbers.
  Link only when request and answer both carry the id (Dock `auth` →
  `authentication` carries none, so no link; it runs in setup).
- **Payload types model the recorded frame.** A typed variant keeps only
  what its type models: test decoding with unedited recorded frames and
  assert payload content. Top-level data goes in a
  `#[serde(flatten)] Map<String, Value>` (Dock `params`/`data`); `RawValue`
  cannot be flattened.
- **Challenge flows** are `AuthFlowHints`, never prose. Heartbeats are
  caller-managed `HeartbeatHints`, documented in the client rustdoc, no
  warning. Durations must be ≥ 1 s.
- **Opening message.** `lifecycle.open` is a `connect_*` argument, sent
  first on every connection (no `send_{open}` helper). If its payload also
  carries the API key, bind the wire field with
  `lifecycle.open_credential_field` (ElevenLabs `xi-api-key`): filled from
  the handshake credential when absent/null/empty, conflict →
  `InvalidCredential`. Never infer it from the payload shape.
- Build messages with `MessageSchema::new(name, direction, schema)
  .describe(..)`; build endpoints with `..WebSocketEndpoint::default()`.

## Generation checks (`ws_codegen::plan::validate_ws_api`)

Errors at a definition path (`endpoints/{id}/…`): path not starting with
`/`, own origin, or embedded `?`/`#`; path param without placeholder (or
placeholder missing from a non-empty `params.path`); non-`simple`/optional
path param; duplicate/enum/JSON/unsupported-style query param; empty
`responds_with`, selector field, auth-flow name/indicator,
`open_credential_field` (also an Error without `lifecycle.open`); zero timeout or
interval; a message name that is not an identifier or repeats; `raw_binary`
/`mixed` frame format. Per direction the role **receives** (client: `Server`
+`Bidirectional`; host: `Client`+`Bidirectional`): >1 fallback, fallback
with selectors, unselected message beside others, repeated selector field,
overlapping selectors unless one set is a strict subset of the other (the
more specific wins). Correlation Errors: `responds_with` on a server
message, naming no message / a client message, without endpoint hints, or
on a request named `Value`. Warnings: hints with no link, hints on a
`Host`-only endpoint, a declared `string` id type nothing consumes.

## Generated surface (`ws_codegen/{client,host,shared,supervisor}.rs`)

- `{Ep}ServerMessage` / `{Ep}ClientMessage`: `Debug` only; payload-only
  `Serialize` where the endpoint sends that direction, `decode(WsInboundFrame)`
  where it receives it (most specific selectors → fallback/sole message →
  `WsError::Decode { frame: WsRawFrame, reason }`). Empty direction = empty
  enum. Payloads are decoded from the raw frame (RawValue-safe).
- Client (`Client`/`Both` only): `send(ClientMessage)`, `send_value(Value)`,
  `next_event()` typed, `next_value_event()`, `events()`/`value_events()`
  (`WsEventStream`, holds the transport) — one shared queue.
  Correlated: `request_{snake(request)}(payload) -> Result<ResponsePayload>`
  per pair + `request_value(Value)`; client-allocated ids (never reset by a
  redial); only the declared response with the same id completes a typed
  request (`selected_message`), other same-id frames stay events; a
  `PendingGuard` frees the slot on send failure/timeout/cancel; loss →
  `Disconnected`, queued commands discarded, nothing replayed.
- Handshake credential: `auth_config()` is the REST translation
  (`codegen/api_struct/auth.rs::policy_config`); connect selects one
  credential via `schematic_runtime::AuthConfig::select_handshake`
  (options `credential` → `credentials`/`api_key` → first non-empty env →
  anonymous), places it with `HandshakeCredential::apply`, and keeps the
  resulting `WsHandshake` for redials. `auth_method()` reports the source.
  Non-101 → `WsError::HandshakeRejected{status, headers}` (redacted via
  `secret_names()`); redirects never followed; 401/403/3xx on a redial
  closes. Caller headers may not name the credential header or an upgrade
  header. No `schematic_define::Headers` in WS output; no WS TLS opt-out.
  A self-signed `wss://` cert (Samsung TV :8002) fails as `WsError::Handshake`;
  `ws://` (UC Core/Dock) is cleartext and never warned about.
- Supervisor (`supervisor.rs` → `ws_shared::connect_session`, shared by every
  endpoint client): connect = ONE inline attempt (dial → opening write →
  setup) under `handshake_timeout`, then a spawned supervisor. `Ready` only
  after setup. `AuthFlowHints` endpoints get ONLY
  `connect_{ep}_with_setup(.., setup: Fn({Ep}Setup) -> Fut)` (no plain
  `connect_{ep}`; already-authenticated callers pass `|_| async { Ok(()) }`);
  `{Ep}Setup = WsSetupSession<ClientMsg, ServerMsg>` (slot + revocation
  watch; invalid after its attempt; `credential().expose_secret()`).
  `WsSetupError::Rejected` → `WsError::SetupRejected`, terminal; other →
  `WsError::Setup`. `max_attempts` = consecutive failed redials, reset on a
  completed setup. `receive_timeout` = inbound idle (any frame) after
  setup → `ReceiveTimeout` event + reconnect policy. Pongs flushed on ping.
  Full event queue: one held event, reading paused, everything else
  (close, timer, writes, state) keeps running. `send` while not `Ready` →
  `Disconnected`; `close()` waits for `Closed`. Tests:
  `schema/tests/ws_session.rs`, `ws_idle.rs`; CI-visible: the fresh-output
  WS session test in `gen/tests/l1/fresh_output_compile.rs`.
- Host (`Host`/`Both`): `{Api}EventHub`, `HostOutbound` (typed fan-out per
  endpoint), `WsConnectionContext::credential_verified()`, `WsHandler` with
  `handle_message`/`on_authenticated` (per-endpoint
  `handle_{ep}_message`/`on_{ep}_authenticated` when >1 host endpoint, routed
  by path, unknown → 404) and `on_error`. No protocol envelope is generated:
  UC drivers send their `authentication` greeting from `on_authenticated`.

## JSON form

Absent ≠ `null`: optional fields read absent as "no declaration" (role →
`client`, fallback → `false`, `default_timeout_secs` → 30) and reject `null`.
Struct-valued fields must be objects (`serde_presence::map_only` /
`non_null_map`), duplicates and trailing content are errors. Matrix tests:
`schematic-definitions/tests/ws_definition_json.rs`; conformance of the five
shipped definitions: `tests/ws_conformance.rs`.
