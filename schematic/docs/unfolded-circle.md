# Unfolded Circle API Client Design (Schematic)

## Goal

Design and implement typed Rust clients for Unfolded Circle:

1. Core REST API
2. Core WebSocket API
3. Dock WebSocket API
4. Integration WebSocket API

using Schematic (`schematic-define`, `schematic-definitions`, `schematic-gen`, `schematic-schema`) as the source of truth for API definitions and generated clients.

## Inputs Used

- Context skill: `homelab/docs/unfolded-circle/SKILL.md`
- API YAML definitions:
  - Core REST OpenAPI: `core-api/rest/UCR-core-openapi.yaml` (OpenAPI `3.0.3`, API version `0.44.4`)
  - Core WS AsyncAPI: `core-api/websocket/UCR-core-asyncapi.yaml` (AsyncAPI `2.2.0`, API version `0.34.0-beta`)
  - Dock AsyncAPI: `dock-api/UCD2-asyncapi.yaml` (AsyncAPI `2.2.0`, API version `0.8.0-beta`)
  - Integration AsyncAPI: `integration-api/UCR-integration-asyncapi.yaml` (AsyncAPI `2.2.0`, API version `0.14.0-beta`)

## Current Schematic Capabilities and Gaps

### What exists now

- OpenAPI import/export pipeline exists (`schematic-gen import`, `schematic_define::openapi::*`).
- REST codegen pipeline is mature and outputs strongly typed `reqwest` clients.
- `WebSocketApi` primitives exist in `schematic-define` and are already used in definitions (e.g. ElevenLabs WS definitions).

### Gaps to close for this project

- No AsyncAPI import pipeline today.
- No WebSocket runtime/codegen in `schematic-gen` / `schematic-schema` yet.
- No generated WS client abstraction for request-response correlation, auth handshake fallback, reconnect, subscriptions.

## Proposed Target Architecture

### Repository placement

- `schematic/definitions/src/unfolded_circle/`
  - `core_rest/mod.rs`
  - `core_rest/types.rs`
  - `core_ws/mod.rs`
  - `core_ws/types.rs`
  - `dock_ws/mod.rs`
  - `dock_ws/types.rs`
  - `integration_ws/mod.rs`
  - `integration_ws/types.rs`
- `schematic/gen/src/`
  - Extend import pipeline for AsyncAPI
  - Add WS codegen pipeline
- `schematic/schema/src/`
  - Generated output modules:
    - `unfolded_circle_core_rest.rs`
    - `unfolded_circle_core_ws.rs`
    - `unfolded_circle_dock_ws.rs`
    - `unfolded_circle_integration_ws.rs`

### Runtime split

- REST runtime (existing): `reqwest`, same as other generated clients.
- WS runtime (new): `tokio-tungstenite` + `futures` with:
  - JSON frame encode/decode
  - request-id correlation map (`id` / `req_id`)
  - typed inbound event stream
  - auth handshake strategy (header-first, message fallback)
  - keepalive/ping support

## API-by-API Design

### 1) Core REST API client

### Spec facts driving design

- `134` paths, `314` operations, `312` schemas.
- Path domains (count by prefix): `system`, `cfg`, `remotes`, `ir`, `intg`, `docks`, `activities`, `profiles`, `auth`, `pub`.
- Non-JSON success responses exist:
  - `text/plain`, `text/csv`
  - `application/octet-stream`
  - `audio/wav`, `image/png`, `image/jpg`
- Multipart upload endpoints exist (`file` form field):
  - `/resources/{type}`
  - `/intg/install`
  - `/ir/codes/custom/{codeSetId}`
  - `/system/backup/restore`
  - `/system/install/{customComponent}`

### Auth model

The description documents three auth methods (Basic, Bearer, Cookie session), but security schemes in the OpenAPI model expose `basicAuth` and `cookieAuth`. Design for all three:

- `CoreRestAuth::Basic { user, pass }`
- `CoreRestAuth::Bearer { token }`
- `CoreRestAuth::CookieSession` with login helper:
  - `POST /pub/login`
  - persistent cookie jar (`reqwest` cookie store)

Use Schematic `Headers` + variant builder to inject bearer/basic programmatically and bypass env when needed.

### Generation path

1. Import OpenAPI into initial model/types (`schematic-gen import`).
2. Normalize endpoint response types:
   - Map binary endpoints to `ApiResponse::Binary`.
   - Map plain text and CSV endpoints to `ApiResponse::Text`.
3. Normalize multipart endpoints to `ApiRequest::form_data([FormField::file("file")])`.
4. Add curated env mapping:
   - `UCR_CORE_API_KEY`, `UNFOLDED_CIRCLE_API_KEY` (bearer)
   - `UCR_CORE_USER`, `UCR_CORE_PASSWORD` (basic)

### Client surface

- Single typed client: `UnfoldedCircleCoreRest`.
- Domain convenience methods on top of generated request enum:
  - `system()`, `cfg()`, `integrations()`, `docks()` helpers (thin wrappers).
- Streaming binary download helpers for backup/resource endpoints.

### 2) Core WebSocket API client

### Spec facts driving design

- Channels: `/ws`, `/intg`, `/profiles`, `/events`
- Message set size: ~`150` publish + `92` subscribe refs (unique).
- Envelope shape:
  - requests: `{ kind: "req", id, msg, msg_data? }`
  - responses: `{ kind: "resp", req_id, msg, code, msg_data? }`
  - events: `{ kind: "event", msg, cat?, ts?, msg_data }`
- Supports header auth (`API-KEY`), basic auth, or message-based auth (`auth_required` -> `auth` -> `authentication`).

### Client responsibilities

- Connection bootstrap:
  - URL selection (`ws://` / `wss://`)
  - optional header auth (`API-KEY`)
  - fallback to message auth when `auth_required` arrives
- Request-response RPC:
  - monotonic `id` generator
  - pending map keyed by `req_id`
  - timeout + cancellation
- Event stream:
  - typed enum for async events
  - filter by channel/category
- Optional reconnect:
  - backoff
  - resubscribe to event channels after reconnect

### Generated API shape

- `UnfoldedCircleCoreWsClient`
- `CoreWsCommand` enum (typed request messages)
- `CoreWsResponse` enum (typed response messages)
- `CoreWsEvent` enum (typed events)
- Channel-oriented sub-clients:
  - `core_ws.ws.*`
  - `core_ws.integrations.*`
  - `core_ws.profiles.*`
  - `core_ws.events.*`

### 3) Dock WebSocket API client

### Spec facts driving design

- Single channel: `/`
- Message counts: `15` publish, `12` subscribe
- Envelope differs from Core/Integration:
  - uses `type` + `msg` instead of `kind`
  - responses use `req_id`, `code`, optional `reboot`
- Auth is explicit post-connect:
  - receives `auth_required`
  - send `auth { token }`
  - receive `authentication` result

### Client responsibilities

- Dedicated envelope codec for Dock schema.
- Auth handshake mandatory unless only issuing allowed unauthenticated calls (`get_sysinfo`).
- Strongly typed IR and port-control operations:
  - `ir_send`, `ir_stop`
  - `get/set_port_mode`, `get/set_port_trigger`
  - device config commands
- Event stream for:
  - `ir_receive*`
  - `port_mode_event`

### Generated API shape

- `UnfoldedCircleDockWsClient`
- `DockCommand`, `DockResponse`, `DockEvent` enums
- Convenience APIs:
  - `dock.ir.*`
  - `dock.ports.*`
  - `dock.system.*`

### 4) Integration WebSocket API client

### Spec facts driving design

- Channel: `/intg`
- Message counts: `21` publish, `18` subscribe
- Envelope is Core-like (`kind=req/resp/event`).
- Authentication via `auth-token` header or `auth` message.
- Critical topology detail from spec: integration driver acts as **server**, UC Remote acts as **client**.

### Design implication

Implement two complementary roles:

1. `IntegrationWsClient` (outbound) for test tools/simulators.
2. `IntegrationWsHost` (inbound server) for real driver implementations.

Both share schema/message types and dispatcher logic.

### Host runtime requirements

- `tokio-tungstenite` server accept loop.
- Per-connection auth gate.
- Request dispatcher trait:
  - `handle_get_driver_version`
  - `handle_get_available_entities`
  - `handle_entity_command`
  - etc.
- Event push API to connected remotes:
  - `entity_change`, `entity_available`, `assistant_event`, etc.

## AsyncAPI Support Plan in Schematic

### A) Import pipeline

Add `schematic-gen import-asyncapi`:

- `--input <yaml/json>`
- `--api-name`
- `--module-path`
- `--ws-role client|server|both` (important for Integration API)
- `--dry-run`
- `--strict`

Importer outputs:

- `WebSocketApi` definition
- `ModelCatalog` for schemas/messages
- diagnostics list (unsupported constructs, ambiguous oneOf, external refs)

### B) Generator pipeline

Add WS codegen phases analogous to REST:

1. Validate WebSocketApi/model catalog
2. Generate message enums + serde models
3. Generate transport client/host
4. Generate dispatch traits and builders
5. Validate with `syn`
6. Format with `prettyplease`

### C) Shared generated WS support module

Add `schematic/schema/src/ws_shared.rs`:

- `WsError`
- `PendingRequestMap`
- `ReconnectPolicy`
- `AuthStrategyWs`
- `MessageRouter`
- serde helpers for tagged envelopes

## Mapping Rules (AsyncAPI -> Schematic)

- Channel path -> `WebSocketEndpoint.path`
- Message refs from `publish/subscribe.oneOf` -> `MessageSchema`
- `#/components/schemas/*` -> generated Rust models in `types.rs`
- Common envelope schemas (`commonReq/commonResp/commonEvent`) -> generated base structs
- Message `const` fields (`msg`, `kind`, `type`) drive enum discriminants
- Header security scheme mapping:
  - `httpApiKey` -> `AuthStrategy::ApiKey { header }`
  - `http basic` -> `AuthStrategy::Basic`

## Versioning and Drift Control

All four APIs are pre-1.0 (`0.x`), so breaking changes are expected.

Drift strategy:

1. Pin YAML snapshots in-repo under `schematic/specs/unfolded-circle/`.
2. Regenerate in CI and diff generated files.
3. Fail CI on:
   - added/removed message names
   - changed required fields
   - response/content-type mapping regressions
4. Keep changelog section in this doc with spec version bumps.

## Testing Strategy

### Unit tests

- Importer tests:
  - OpenAPI and AsyncAPI parse coverage
  - mapping diagnostics
- Codegen snapshot tests:
  - generated modules compile
  - generated method names stable

### Integration tests

- REST: `wiremock` for auth flows, multipart upload, binary/text responses.
- WS (client): mock WS server scripts for auth-required and correlation behavior.
- WS (host): simulated UC Remote client for Integration API request/response/event loops.

### Workspace commands

- From `schematic/`:
  - `just lint`
  - `just test`
  - `just build`
- Full regeneration check:
  - `just -f schematic/justfile generate`
  - compile generated crate(s)

## Implementation Phases

1. **Phase 1: Core REST MVP**
   - Import + normalize REST API.
   - Generate `UnfoldedCircleCoreRest`.
   - Cover auth + binary/multipart endpoints.
2. **Phase 2: AsyncAPI Importer**
   - Build `import-asyncapi` and diagnostics.
   - Produce `WebSocketApi` + models for Core WS, Dock, Integration.
3. **Phase 3: WS Client Generator**
   - Generate Core WS + Dock WS clients.
   - Implement auth handshake + request correlation runtime.
4. **Phase 4: Integration Host Generator**
   - Generate server-side dispatcher and host runtime.
   - Add simulator client for testing.
5. **Phase 5: Hardening**
   - Reconnect behavior, backpressure, structured tracing, docs/examples.

## Open Risks / Decisions

1. Core REST bearer auth is documented in description but not modeled in security schemes; importer must allow a manual auth override.
2. Core WS message surface is large; generated enums may be heavy. Consider feature-gating modules per channel.
3. Integration API role inversion (driver-as-server) must be first-class; a client-only generator is insufficient.
4. All APIs are `0.x`; regeneration and compatibility tests are mandatory, not optional.

## Deliverables

1. New Unfolded Circle API definition modules in `schematic-definitions`.
2. OpenAPI + AsyncAPI import support in `schematic-gen`.
3. Generated REST + WS clients in `schematic-schema`.
4. WS shared runtime primitives.
5. CI regeneration/compatibility checks for spec drift.
