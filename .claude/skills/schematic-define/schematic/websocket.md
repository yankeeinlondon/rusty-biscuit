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
  `type`[+`msg`], Samsung `event`/`method`).
- **Response link.** `.responds_with("Name")` on a request; pair it with the
  endpoint's `CorrelationHints` (no API-level fallback).
- **Challenge flows** are `AuthFlowHints`, never prose. Heartbeats are
  caller-managed `HeartbeatHints`, documented in the client rustdoc, no
  warning. Durations must be ≥ 1 s.
- Build messages with `MessageSchema::new(name, direction, schema)
  .describe(..)`; build endpoints with `..WebSocketEndpoint::default()`.

## Generation checks (`ws_codegen::plan::validate_ws_api`)

Errors at a definition path (`endpoints/{id}/…`): path not starting with
`/`, own origin, or embedded `?`/`#`; path param without placeholder (or
placeholder missing from a non-empty `params.path`); non-`simple`/optional
path param; duplicate/enum/JSON/unsupported-style query param; empty
`responds_with`, selector field, auth-flow name/indicator; zero timeout or
interval. Selector-overlap/fallback-count enforcement and typed decoding are
**planned**, as is correlation driven only by `responds_with`.

## JSON form

Absent ≠ `null`: optional fields read absent as "no declaration" (role →
`client`, fallback → `false`, `default_timeout_secs` → 30) and reject `null`.
Struct-valued fields must be objects (`serde_presence::map_only` /
`non_null_map`), duplicates and trailing content are errors. Matrix tests:
`schematic-definitions/tests/ws_definition_json.rs`; conformance of the five
shipped definitions: `tests/ws_conformance.rs`.
