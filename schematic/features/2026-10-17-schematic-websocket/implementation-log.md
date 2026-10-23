---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-17-schematic-websocket/spec.md"
plan: "schematic/features/2026-10-17-schematic-websocket/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - schematic/features/2026-10-17-schematic-websocket/setup-spike/lib.rs.txt
    - schematic/features/2026-10-17-schematic-websocket/setup-spike/setup_tests.rs.txt
    - schematic/features/2026-10-17-schematic-websocket/setup-spike/Cargo.toml.txt
skills_files_updated_during_phase_1: []
packages: []
human_review: false
message_to_agent: |-
    Phase 1 changed no source. Read the Phase 1 log sections "Refreshed contracts",
    "Protocol map", "Findings for review", "Verification map" and "Setup spike"
    before starting; they supersede several plan assumptions:
    - The shared diagnostic format (gen/src/diagnostics.rs ValidationDiagnostic) has
      only Warning/Error and no WebSocket Feature variant. Adopted mapping: add WS
      Feature variant(s), use Output::Rust, map Warning/Error 1:1, print the single
      Info (plan.rs:470 receive-timeout TODO) only under the generator's verbose flag.
    - The fresh-output harness (gen/tests/common/fresh_output.rs) accepts only
      &[&RestApi]; WS modules come from the fixed WS_DEFINITION_MODULES list. R4's
      Bearer-host compile test needs a hook that renders a fixture WebSocketApi.
    - artifact_drift is ordinary L1 and `just check-drift` now uses nextest; use it.
    - CI never runs schematic/schema tests (ws_runtime.rs, compile_fail.rs); mock-server
      evidence is local (`just test-schema`, `just cross-check`). Do not change CI.
    - A peer close does not make the next local write fail on macOS (first write
      returns Ok): the harness needs an injectable failing writer for R3.
    - Selector overlap: adopted strict-subset specificity (Findings item 1), not the
      plan's conservative rule, because the conservative rule rejects Core, Dock and
      Integration as documented.
    - Core keeps four endpoints including /events (documented in
      specs/unfolded-circle/UCR-core-asyncapi.yaml:488). Dock's missing :946 port is
      logged as new scope, not fixed.
    - The working tree contains unrelated uncommitted changes (discriminated-union
      OpenAPI import work); do not touch or revert them.
---

# Implementation Log for 2026-10-17-schematic-websocket (7 phases)

## Phase 1

Started 2026-10-22.

### Readiness and authorization

- **Prerequisite (plan rule 1): met.** `2026-10-16-schematic-response-model-graphql`
  has `implemented: true`, `completed: true`, `review_iterations: 2`; its
  `review-2.md` (2026-10-22) is `ready: true` with no findings and no human
  review items. Its implementation is committed (`74f9a9833` and earlier).
  The earlier chain (`2026-10-11-schematic-generation-integrity`,
  `2026-10-12-schematic-runtime-envelope`, `2026-10-14-schematic-definition-api`,
  `2026-10-15-schematic-definition-vocabulary`) precedes it.
- **Authorization (plan rule 2).** The spec's pre-authorizations were confirmed
  by the author on 2026-10-21 (spec "Pre-authorizations"); starting this run is
  that confirmation. Phase 1 used none of them (no regeneration, no dependent
  edits, no public API removal, no runtime change, no dev-dependency). The "Not
  pre-authorized" list stays in force.
- **Pre-existing user changes preserved.** The working tree already carried
  uncommitted, unrelated schematic changes (discriminated-union OpenAPI import
  work: `define/src/openapi/import/mappings/*`, `gen/tests/l1/discriminated_unions.rs`,
  `definitions/src/openai/*`, docs and skill edits, the graphql feature's
  `log.md`/review files, `fixes/_unscheduled/`). Phase 1 does not touch them.
  Consequence: the uncommitted tree is **not** exactly this phase's changes, so
  the gate-skip calculator cannot be used to skip gates; both gates are run.

### Adopted defaults (spec "Planning-time details", items 1–21)

Each line: item → adopted value; alternative considered; why it held.

1. **Response link** → `MessageSchema.responds_with: Option<String>` on the
   request message, naming one answering message in the same endpoint
   (source: item 1; plan rule 3). Alt: a separate endpoint-level pair table or
   many-to-many links. Held: one request has one declared response in every
   shipped protocol; the field travels with the message it describes.
2. **Correlation without hints** → active only with endpoint-level
   `CorrelationHints`; `responds_with` without hints is an Error; hints with no
   pair Warn (source: item 2). Alt: API-level fallback hints. Held: the spec
   forbids an API-level fallback (R11).
3. **Diagnostics** → map `WsDiagnostic` onto the generation-integrity format
   and reuse its writer/validation; no WS-specific format (source: item 3).
   Confirmed available — see "Refreshed contracts".
4. **Double `/ws`** → Core `base_url` becomes the origin `ws://remote.local`;
   endpoints keep `/ws`, `/intg`, `/profiles` (source: item 4; evidence: the
   official Core AsyncAPI `0.45.0-beta`, already verified in the spec).
5. **Reconnect counter** → reset after a successful dial *and* complete setup
   (opening write + callback); incremented per failed dial/setup;
   `max_attempts` bounds consecutive failures; `Some(0)` = no redials,
   `None` = unlimited (source: item 5, R2, plan rule 7).
6. **Opening/setup order** → opening payload written first, then the setup
   callback; both before `Ready` (source: item 6; plan rule 5). Ownership and
   cancellation: see "Setup spike".
7. **Missing credential** → anonymous upgrade, `AuthMethod::Anonymous`
   (Decided, D56).
8. **Cross-origin** → no redirect is followed; 3xx is an error with redacted
   `Location` (Decided, D56).
9. **`receive_timeout`** → inbound idle timer over every frame kind, reset on
   each inbound frame, starts after setup; expiry → `ReceiveTimeout` event then
   reconnect policy (source: item 9).
10. **Enum naming** → `{Endpoint}ServerMessage` / `{Endpoint}ClientMessage`,
    variants named by `MessageSchema.name`, `Bidirectional` in both
    (source: item 10; selection Decided D57-19).
11. **Untyped surface** → `send_value`, `next_value_event`, `value_events`,
    `request_value` (source: item 11; plan rule 3). Alt: `send_raw` — rejected
    because D45 gives that name REST semantics.
12. **`WebSocketApi` field set** → add `env_mapping`, constant `headers`
    (REST header type), the retained `auth_policy`; `env_auth` follows REST
    (source: item 12). Exact upstream shapes: see "Refreshed contracts".
13. **`ParamType` → `QueryParamType`** → `String`/`Integer`/`Boolean` by name,
    `Float` → `Number` (source: item 13); typed path parameters per upstream.
14. **Host post-auth** → endpoint-specific `on_authenticated` hook returning
    `Option<{Endpoint}ServerMessage>`, default `None` (source: item 14; R14
    requires endpoint-specific methods for multi-endpoint hosts).
15. **Event hub** → `{ApiName}EventHub` (source: item 15).
16. **TLS for `wss://`** → pass through the runtime builder's opt-in setting
    if one exists; add no WebSocket-only switch (source: item 16). Finding:
    see "Protocol map".
17. **Samsung correlation** → declared only with documented request/response
    ID evidence (source: item 17; plan rule 9). Finding: see "Protocol map".
18. **Test levels** → every mock-server test is L1; no L2 (source: item 18).
19. **Mock server** → built on `tokio-tungstenite` server side in
    `schematic-schema` test support; source-included by `schematic-gen` tests
    only where useful; no generated-crate dependency cycle; no new crate
    (source: item 19).
20. **Timing** → before first `Ready` each attempt gets
    `min(remaining deadline, per-attempt timeout)` covering dial, upgrade,
    opening write and callback; no deadline renewal on redial; after `Ready`
    only the per-attempt timeout (Decided D56-3; plan rule 7).
21. **ElevenLabs scope** → normalize the `WebSocketApi` declaration (params,
    auth, env mapping); payload structs stay ordinary Serde types resolved by
    the shared schema mechanism; no wire change (source: item 21).

Plan-level proposed choices also adopted (plan rules 3–6, 9, 10):

- **Roles** → `Client`, `Host`, `Both`; absent = `Client`; message directions
  stay client-relative (plan rule 3; spec "Host selection and directions").
- **Selector overlap rule** → a selector is a conjunction of equality checks
  on existing wire fields using the existing field-path syntax. Two messages in
  one direction conflict unless some shared field carries different values in
  their predicate sets; identical predicates are the minimal conflict. A
  fallback may not also declare selectors. No declaration-order resolution and
  no invented nested paths (plan rule 4). Recorded for review: this is
  conservative — `{type: a}` and `{type: a, sub: b}` conflict.
- **Opening credentials** → a caller-supplied opening auth field that
  conflicts with the selected shared credential is rejected; an absent
  declared auth field is filled from it; an equal value is one source. Fields
  are bound explicitly per protocol, never inferred from payload shape
  (plan rule 6). Per-protocol binding: see "Protocol map".
- **Heartbeats** → caller-managed; rustdoc only; no warning (Decided D57-21).
- **Compile test (R4)** → in `schematic-gen`'s `fresh_output_compile` module,
  run by `just test-fresh-output` (invoked by `just test`), no `slow_` prefix
  (author ruling 2026-10-21, plan rule 10).

### Refreshed contracts (plan rule 1; code read 2026-10-22, code wins over specs)

Paths relative to `schematic/`.

| Contract | Final shape in the tree | WS verdict |
|---|---|---|
| Runtime package | crate `schematic-runtime` at `runtime/` | use verbatim |
| Runtime trait | `EndpointSpec: Send + 'static` (`runtime/src/endpoint.rs:30`): `Api`, `Success`, `Failure`, `Decoder`, `ENDPOINT_ID`, `METHOD`, `ERRORS`, `ADDRESS`, `into_parts() -> RequestParts` | HTTP-shaped; a WS handshake does not implement it. Generated WS code does not depend on the runtime today (`gen/src/ws_codegen/client.rs:579-603`) |
| `ParamDef` | `define/src/params/param_def.rs:25-55`; fields `name, rust_name, secret, pager_managed, deprecated, required, description, param_type, explode, style`; **no default-value field** | reuse. "Defaults preserved" is vacuous: neither `ConnectionParam` nor `ParamDef` carries a default |
| `QueryParamType` | `:81-96`, `#[non_exhaustive]`: `String, Integer, Number, Boolean, Array, Enum, Json` | `Float` → `Number` |
| `ParamStyle` | `:126-148`: `Form, Simple, SpaceDelimited, PipeDelimited, DeepObject` | reuse |
| serde on the three | **present** — `Serialize` + `Deserialize` derived on `ParamDef`, `QueryParamType`, `ParamStyle` | R12 asserts only; no predecessor defect |
| Location containers | `EndpointParams { path, query, header, cookie }` (`define/src/params/mod.rs:83-96`); only `path` is `#[serde(default)]` | reuse for `WebSocketEndpoint` |
| Typed path params | `ParamDef::path(name, ty)` (`param_def.rs:160`, required, `Simple`); `EndpointBuilder::path_param` | reuse |
| Constant headers | `Vec<(String, String)>` on `RestApi.headers` / `Endpoint.headers` (`define/src/types.rs:145`); `Headers` builder is runtime-only, no serde | WS `headers: Vec<(String, String)>` |
| `AuthStrategy` | `define/src/auth.rs:77`, `#[non_exhaustive]`: `None, BearerToken{header}, ApiKey{header, value_prefix}, Basic{username_env, password_env}, ApiKeyParam{name, location}, OAuth2` | already used by WS |
| `AuthPolicy` | `auth.rs:271` `{ explicit: Vec<AuthMethod>, env_fallback: Option<EnvAuthStrategy> }`; `AuthPolicy::from_auth_strategy`; `RestApi::effective_auth_policy()` (`types.rs:283`); set via `RestApiBuilder::auth_policy` | WS adds `auth_policy: Option<AuthPolicy>` + `effective_auth_policy()` mirroring REST |
| `EnvMapping` | `define/src/headers/env.rs:149`: `bearer_token, basic_user, basic_pass: Option<EnvList>`, `api_key: Option<ApiKeyEnv{names, header}>`, three `oauth_*`; `EnvList` ordered | WS adds `env_mapping: Option<EnvMapping>` |
| `env_auth` | **not retired**: still `RestApi.env_auth: Vec<String>` (`types.rs:138`), the ordered chain for token/key strategies, "never a Basic source"; `env_auth[0]`-as-password removed; `legacy_env_mapping_for` (`types.rs:302`) | WS keeps `env_auth` with REST semantics |
| Definition builders | REST: typestate `RestApi::builder(&DefinitionContext, ..)`; typed registration `DefinitionContext::register::<T>()` | WS keeps struct-literal construction (no WS builder exists; adding one is new scope) |
| Typed descriptors for `MessageSchema` | **not extended**; `Schema{type_name, module_path}`; definition-API review 1 kept WS `Schema` unchanged | keep name-based resolution; gap logged (spec Design note) |
| Credential resolution | `AuthConfig::resolve` (`runtime/src/credentials.rs:301`), `resolve_required`, `Resolved`, `CallAuth`, `Credential::wire_value`, `place_credential` (`runtime/src/client.rs:1378`) are **crate-private**; public: `Credential`, `CredentialKind`, `CredentialPlacement{Bearer, ApiKeyHeader, ApiKeyQuery, ApiKeyCookie, Basic}`, `EnvFallback`, `AuthConfig` | Phase 5 uses the pre-authorized minimal extraction: a public resolve wrapper returning placement + value + `AuthMethod`, and the header/query/cookie placement half of `place_credential` over `(&mut Url, &mut HeaderMap)` |
| `AuthMethod` | `runtime/src/meta.rs:22`: `PerCall, ClientLevel, EnvVar(String), Anonymous` | reuse |
| Redaction | `runtime/src/redact.rs`: `REDACTED`, `BUILTIN_SECRET_HEADERS`, `BUILTIN_SECRET_QUERY_NAMES`, `SecretNames::{builtin, with_name(s), redact_headers, redact_query, redact_url -> RedactedUrl, redact_url_reference}` — public, transport-neutral | reuse as-is |
| Handshake rejection error | **missing**: `TransportErrorKind` (`runtime/src/error.rs:50`) has `CrossOriginRejected, InvalidArgument, InvalidUrl, Connect, Tls, Timeout, DeadlineExceeded, InvalidClient, Other`; no status+headers kind | open issue (plan rule 11): the WS error type carries status + headers redacted through `SecretNames::redact_headers`; no new runtime error kind |
| Redirect policy | private `is_followable_redirect`, `MAX_REDIRECTS`, `OriginPolicy` | WS follows no redirect (D56); nothing to reuse |
| Deadline timing | public `Deadline::{after, remaining, is_expired, expires_at}`; the `min(remaining, timeout)` helper `Expiration::start` (`deadline.rs:106`) is **crate-private** | Phase 5/6: expose one tiny public helper under the runtime pre-authorization, or compute `min` from `Deadline::remaining()`; log which |
| Duration zero rule | none upstream: `Client::timeout` accepts 0; `CorrelationHints.default_timeout_secs` (default 30) and `HeartbeatHints` have no zero validation | **adopted BOUND rule**: definition-level `default_timeout_secs`, `heartbeat.interval_secs`, `heartbeat.timeout_secs` must be ≥ 1 (0 is a field-addressed Error: a zero interval/timeout has no meaning); absent `default_timeout_secs` keeps its documented default 30 |
| TLS opt-in | `ClientBuilder::danger_accept_invalid_certificates` (`runtime/src/client.rs:382`), `TransportBuilder::danger_accept_invalid_certificates` (`transport.rs:57`) — reqwest only; runtime has no tungstenite dependency | see Samsung TLS finding |
| Diagnostics format | `ValidationDiagnostic{severity, api, endpoint, feature, output, message}` (`gen/src/diagnostics.rs:211`); `Severity{Warning, Error}` (**no `Info`**); `Feature` has no WS variants; `Output{Rust, OpenApi, Postman}`; `pipeline.rs::publish` prints and blocks on Error; two-phase `MutationPlan`/`publish()` (`output/publish.rs`) | **adopted mapping (Phase 2)**: add WS `Feature` variant(s) to the shared enum (extending, not forking, the format); WS outputs use `Output::Rust`; `WsSeverity::Warning/Error` map 1:1; the only `Info` (`plan.rs:470`, receive-timeout TODO) is printed only under the generator's verbose flag and disappears in Phase 6. The shared `Severity` gains no `Info` |
| Fresh-output compile check | `gen/tests/common/fresh_output.rs` `FreshCrate::generate(&[&RestApi], Definitions, lockfile)`, `add_test_file`, `cargo_check(_tests)`, `nextest_run`; shared child target `<target>/schematic-fresh-output`; tests named `slow_*` under `just test-fresh-output` | **gap**: WS modules come only from the fixed `WS_DEFINITION_MODULES` (`output/ws_modules.rs:16`); Phase 2 needs a hook that renders a fixture `WebSocketApi` into the fresh crate (test-support/generator-internal change, not a new contract) |

Missing contracts and their classification:

- None of the missing pieces is **blocking**: each is covered by the runtime
  pre-authorization (credential resolution, timing helper), by plan rule 11's
  "log and proceed without it" (handshake rejection kind → WS error type), or
  is an extension of the existing shared format (WS `Feature` variant;
  verbose-only `Info`). The predecessor-owned serde derives are present.

### Protocol map (per-endpoint expected table)

Composed URL = `base_url + path` (`format!("{}{}", ..)` in generated code).

| API / endpoint | base_url today | path | composed today | expected | role |
|---|---|---|---|---|---|
| Core `CoreWs` | `ws://remote.local/ws` (`definitions/src/unfolded_circle/core_ws/mod.rs:182`) | `/ws` (:144) | `…/ws/ws` | `ws://remote.local/ws` | Client |
| Core `CoreIntegrations` | same | `/intg` (:153) | `…/ws/intg` | `ws://remote.local/intg` | Client |
| Core `CoreProfiles` | same | `/profiles` (:162) | `…/ws/profiles` | `ws://remote.local/profiles` | Client |
| Core `CoreEvents` | same | `/events` (:171) | `…/ws/events` | `ws://remote.local/events` — the repo's copy `specs/unfolded-circle/UCR-core-asyncapi.yaml:488` documents `/events` as a fourth channel | Client |
| Dock `DockRoot` | `ws://dock.local` (`dock_ws/mod.rs:80`) | `/` (:87) | `ws://dock.local/` | official `specs/unfolded-circle/UCD2-asyncapi.yaml:80` server is `ws://uc-dock.local:946`, channel `/` | Client |
| Integration `Integration` | `ws://remote.local` (`integration_ws/mod.rs:96`) | `/intg` (:106) | `ws://remote.local/intg` | the driver is the server; a client URL is the driver's own mDNS-advertised address | **Both** (host used in production by four homelab crates; client unused) |
| Samsung `RemoteControl` | `wss://192.168.1.1:8002` (`samsung_smart_tv/remote_ws/mod.rs:205`) | `/api/v2/channels/samsung.remote.control` (:143) | correct, plus `?name=…&token=…` | same | Client |
| Samsung `ArtMode` | same | `/api/v2/channels/com.samsung.art-app` (:173) | correct | same | Client |
| ElevenLabs `TextToSpeech` | `wss://api.elevenlabs.io` (`elevenlabs/mod.rs:201`) | `/v1/text-to-speech/{voice_id}/stream-input` (:215) | correct | same | Client |
| ElevenLabs `MultiContextTextToSpeech` | same | `/v1/text-to-speech/{voice_id}/multi-stream-input` (:320) | correct | same | Client |

Parameters, auth, lifecycle and hints:

| API | params | auth / env | opening payload auth field | auth flow | correlation | heartbeat |
|---|---|---|---|---|---|---|
| Core | none | `ApiKey{header:"API-KEY"}`; `env_auth` `UCR_CORE_API_KEY, UNFOLDED_CIRCLE_API_KEY` (:184-188) | none (no lifecycle open) | **prose only** (module doc :29-34) → needs `AuthFlowHints` | name-inferred; wire `id`/`req_id` u64 → explicit `CorrelationHints{id, req_id}` | app ping/pong exists, no `HeartbeatHints` declared |
| Dock | none | `AuthStrategy::None` but `env_auth ["UCR_DOCK_TOKEN"]` (:82-83), never applied | none | `AuthFlowHints` declared (:94-99): `auth_required` → `DockWsAuthMessage{token}`; success `authentication`, failure `authentication_failed` — the token travels in a setup message, so it is bound in the setup callback | name-inferred `id`/`req_id` → explicit hints | none |
| Integration | none | `ApiKey{header:"auth-token"}`, `env_auth ["UCR_INTEGRATION_TOKEN"]`; the host uses that variable as its accepted-token list (`gen/src/ws_codegen/shared.rs:422-434`) | none | prose only → `AuthFlowHints` | name-inferred → explicit hints | none |
| Samsung | `name` (String, required, query), `token` (String, optional, query; built-in secret) | `AuthStrategy::None`, no env | none | TV-side approval prompt, not a message challenge → no `AuthFlowHints` | **none** (see finding) | none |
| ElevenLabs | `voice_id` path (required) + optional query (`model_id`, `language_code`, `output_format`, `apply_text_normalization`: String; `enable_logging`, `enable_ssml_parsing`, `sync_alignment`, `auto_mode`: Boolean; `inactivity_timeout`, `seed`: Integer); MultiContext: `voice_id`, `model_id`, `output_format` | `ApiKey{header:"xi-api-key"}`, `env_auth ELEVEN_LABS_API_KEY, ELEVENLABS_API_KEY` (:203-207); REST sibling already has explicit `EnvMapping` (:159-168) | **`TtsInitMessage.xi_api_key`**, wire name `"xi-api-key"` (`elevenlabs/types/websocket.rs:15-16`); `MultiContextInitMessage` has none | none | none | none |

Messages and selectors (directions client-relative; "fallback" = proposed
per-direction fallback):

- **Core** (13 messages, shared by all four endpoints, :46-133; selectors from
  type docs and `.claude/skills/unfolded-circle/core-api.md`): Client —
  `RequestEnvelope{kind=req}`, `Auth{kind=req,msg=auth}`,
  `Ping{kind=req,msg=ping}`, `Version{msg=version}`, `System{msg=system}`;
  Server — `ResponseEnvelope{kind=resp}` (fallback candidate),
  `EventEnvelope{kind=event}`, `AuthRequired{kind=event,msg=auth_required}`,
  `Authentication{kind=resp,msg=authentication}`, `Pong{kind=resp,msg=pong}`,
  `VersionInfo{kind=resp,msg=version_info}`,
  `SystemInfo{kind=resp,msg=system_info}`, `Result{kind=resp,msg=result}`.
  Pairs: Ping→Pong, Version→VersionInfo, System→SystemInfo,
  Auth→Authentication, RequestEnvelope→ResponseEnvelope. Name discrepancy to
  verify in Phase 3: the skill names the requests `get_version`/`get_system`,
  the types say `version`/`system`.
- **Dock** (6): selector field is `type` (Rust `type_name`, renamed) plus
  `msg`: `RequestEnvelope{type=req}` C, `ResponseEnvelope{type=resp}` S
  (fallback candidate), `EventEnvelope{type=event}` S,
  `AuthRequired{msg=auth_required}` S, `Auth{msg=auth}` C,
  `Authentication{msg=authentication}` S.
- **Integration** (6, all declared `Bidirectional` today): true directions —
  `RequestEnvelope{kind=req}` C, `ResponseEnvelope{kind=resp}` S (fallback),
  `EventEnvelope{kind=event}` **truly Bidirectional** (driver sends
  `entity_change`/`device_state`; remote sends `abort_driver_setup`/connect
  events), `AuthRequired{kind=event,msg=auth_required}` S,
  `Auth{kind=req,msg=auth}` C, `Authentication{kind=resp,msg=authentication}` S.
  Tests asserting the contortion: `definitions/src/unfolded_circle/integration_ws/mod.rs:137-144`,
  `gen/tests/l1/ws_codegen.rs:92-95` (`integration_ws_has_both_role`),
  `gen/src/ws_codegen/plan.rs:688`; heuristic `infer_role` (`plan.rs:156-168`).
- **Samsung**: RC — Client `RemoteControlCommand{method=ms.remote.control}`,
  `AppLaunchCommand`/`InstalledAppsCommand` (`method=ms.channel.emit` +
  nested `params.event` — outbound only, so no inbound selector needed);
  Server `ChannelConnectEvent{event=ms.channel.connect}`,
  `ChannelUnauthorizedEvent{event=ms.channel.unauthorized}`,
  `ChannelErrorEvent{event=ms.error}`, `ChannelEnvelope` (declared
  Bidirectional; true use is the Server catch-all → fallback). Art —
  `ArtModeCommand` C, `ArtModeEvent{event=d2d_service_message}` S,
  `ChannelConnectEvent` S, `ChannelEnvelope` fallback.
- **ElevenLabs**: TTS — `TextChunk` C, `AudioChunk` S; Multi — `TextChunk`,
  `CloseContext` C, `AudioChunk` S. One inbound message per endpoint → no
  selector required. Outbound `TtsTextMessage`/`TtsCloseMessage` are
  identical on the wire (`{text}`); selectors are inbound-only so this is not
  a conflict.

Consumers (non-generated): `homelab/lib/src/samsung_tv.rs` (Samsung clients,
`WsClientOptions`, `WsError`, untyped `send(serde_json::to_value(..))`,
`close()`); `homelab/{arcam-amp,eversolo,sony-receiver}-integration/src/handler.rs`
(`WsHandler`, `WsConnectionContext`, `UnfoldedCircleEventHub::new()`,
`handle_message(Value, ctx) -> Option<Value>`) and their `src/main.rs`
(`UnfoldedCircleIntegrationWsHost::{new_event_hub, serve_addr_with_hub}`);
`homelab/unfolded-integration-helper/src/{subscriptions,envelope,setup}.rs`;
`homelab/arcam-amp-integration/src/responses.rs:94` builds its own
authentication response. `ConnectionParam`/`ParamType` are used only in
`define`, the Samsung and ElevenLabs definitions, and the generator.
`sniff`, `biscuit-speaks` (REST modules of `schematic-schema` only) and
`model-citizen` (`schematic-definitions` only) use no WS module.

### Findings for review

1. **Selector overlap rule vs shipped protocols (review item; adopted
   deviation from plan rule 4's proposed rule).** Core, Dock and Integration
   each have two generic inbound envelopes (`kind=resp`, `kind=event`) plus
   specific messages that extend those predicates (`{kind=event,
   msg=auth_required}`). Only one fallback per direction is allowed, so one
   generic envelope must carry a selector, and under the plan's conservative
   rule `{kind=event}` and `{kind=event, msg=auth_required}` conflict — the
   rule rejects all three Unfolded Circle definitions. Options considered:
   (a) keep the conservative rule and drop the generic `EventEnvelope`
   (loses every undocumented event; unknown events would hit the
   `ResponseEnvelope` fallback and fail to decode); (b) merge the two generic
   envelopes into one fallback payload type (changes payload types and their
   consumers); (c) **adopted**: a *strict-subset specificity* rule — when one
   message's predicate set is a strict subset of another's, the frame matching
   both selects the more specific (superset) message; every other overlap
   (identical sets, or partially overlapping sets with no contradictory
   equality and no subset relation) remains an Error. It never uses
   declaration order, is decided statically at plan time, and keeps the
   serialized selector shape (`[{field, value}]`) unchanged, so a later author
   ruling changes only validator/matcher code, not the shape the exporter
   reads. Not blocking under the run's test (shape-stable, revisable in
   review). Phase 4 must test both the subset case (Core pong vs
   `ResponseEnvelope`) and an incomparable-overlap Error.
2. **Samsung correlation: none.** `docs/research/samsung-design.md:133`
   declares `request_id_type: RequestIdType::String` with no request/response
   pairing; the only ids are REST device info and the optional
   `SamsungRemoteConnectData.id`. `homelab/docs/samsung/samsung-s95-tcp-api.md:515-519`
   puts `id`/`request_id` inside a double-encoded `data` string and matches
   the reply by event name only (:541). → No `CorrelationHints`; remove the
   explicit API-level `RequestIdType` (plan rule 9, item 17).
3. **Samsung `wss://` TLS (item 16).** The generated WS `dial` uses
   `connect_async_with_config` with no connector and `WsClientOptions` has no
   TLS field; `schema/Cargo.toml` enables `rustls-tls-native-roots`.
   `homelab/lib/src/samsung_tv.rs:396-408` applies
   `danger_accept_invalid_certificates()` to its REST client only. The
   runtime opt-in exists but configures reqwest only. A TV's self-signed
   certificate on 8002 is therefore expected to fail the WS handshake
   (**unverified**; no device contacted). Phase 5 decides whether passing the
   existing runtime opt-in to the WS dial is possible without a new
   third-party crate; otherwise it logs the gap and adds no WS-only switch.
4. **ElevenLabs.** Confirmed: the definition uses
   `schematic_define::WebSocketApi` (`definitions/src/elevenlabs/mod.rs:195`);
   payload types are plain Serde + schemars structs without
   `schematic_define` (`elevenlabs/types/websocket.rs:1-2`) and are not
   registered in `module_group` (REST only). The opening payload's
   `xi-api-key` field is the one auth-bearing opening payload: per the
   adopted rule it is filled from the selected shared credential when absent
   and rejected when it conflicts (Phase 6 binds it explicitly).
5. **Dock URL (new scope, left undone).** The official UCD2 AsyncAPI server is
   `ws://uc-dock.local:946`; the definition has `ws://dock.local` (no port).
   The spec's URL fix (R1) covers Core only. Logged for the author; Phase 2's
   all-five composed-URL test asserts the current value with a comment
   pointing at this finding.
6. **Integration client URL (new scope, left undone).** The integration
   driver is the WebSocket server and the remote dials it; the client-side
   `base_url ws://remote.local` does not describe a reachable client target.
   Role `Both` keeps today's host and client output; the client side has no
   consumer. No change scheduled.
7. **Dock env auth.** `AuthStrategy::None` with `env_auth ["UCR_DOCK_TOKEN"]`:
   the token belongs to the setup message (`DockWsAuthMessage.token`), not the
   handshake. Phase 3 should express it as `EnvMapping` with an explicit
   policy; since `None` attaches no handshake credential (R5), the setup
   callback reads the resolved credential through the session's redacted
   credential facility.

### Verification map

- Recipes (`schematic/justfile`): `test` = `_test_all` over
  `schematic-define --all-features; schematic-gen; schematic-definitions;
  schematic-oauth; schematic-runtime` + `test-fresh-output`
  (`-p schematic-gen --test l1 -E 'test(/(^|::)slow_/)'`) +
  `test-compile-fail` (`-p schematic-runtime --test l1`, `slow_`);
  `test-schema` = `cargo nextest run --manifest-path schema/Cargo.toml`;
  `lint` = clippy `-D warnings --all-targets` per package; `lint-schema`;
  `doctest`; `check-drift` = nextest over `artifact_drift::` +
  `fresh_output_compile::slow_real_apis_compile_offline`; regeneration via
  `just generate` / `just generate-one <api>`.
- **`artifact_drift` is ordinary L1, not ignored**, and `check-drift` now uses
  nextest. The plan's Phase 7 command (`--run-ignored only`) and its caution
  against `just check-drift` are outdated; later phases should run
  `just check-drift`.
- **CI never runs `schematic/schema` tests**: CI only does `cargo fetch
  --manifest-path schematic/schema/Cargo.toml` (`.github/workflows/ci.yml:745`);
  nothing runs `test-schema` or `schema/tests/compile_fail.rs`. The spec puts
  the mock-server tests (R2, R3, R5–R8, R10, R14, R16) in `schematic-schema`,
  so their evidence is local (`just test-schema`, plus `just cross-check` on
  the rigs) only. Changing CI is outside this run's authorization; recorded
  for the author.
- `schematic/schema` is excluded from the root workspace, has its own
  `Cargo.lock`, no `autotests = false`; dev-deps `serial_test`, `trybuild`,
  `wiremock`; regular deps include `tokio 1.43` and `tokio-tungstenite 0.26`
  (`connect`, `rustls-tls-native-roots`). Existing `tests/ws_runtime.rs` (4
  tests, loopback port 0, inline `accept_async`); no shared mock server
  exists. `tests/common/mod.rs` has an `EnvGuard::clear/set/remove` that
  needs `#[serial]`.
- `schematic-gen`: `autotests = false`, binaries `l1` and `level2`; no
  tokio/tungstenite dev-deps, so gen L1 stays in-process; anything that
  compiles is `slow_` under `test-fresh-output`. WS tests:
  `gen/tests/l1/ws_codegen.rs` (25), unit tests in `ws_codegen/{plan,host,
  shared,client}.rs`; `output/ws_modules.rs` has none. The Bearer unit test
  `host_validate_bearer_header_produces_tokens` asserts substrings only.
- Compile-fail patterns: trybuild in `runtime/tests/l1/compile_fail.rs`
  (`slow_`), trybuild in `schema/tests/compile_fail.rs`, and the
  diagnostic-based `slow_generated_types_reject_misuse_at_compile_time` in
  `fresh_output_compile.rs`.
- `schematic-define`/`schematic-definitions`: flat per-file `tests/*.rs`;
  WS unit tests in `define/src/websocket/mod.rs:238+`.
- AsyncAPI importer: **no fixture files**, two inline-YAML unit tests
  (`gen/src/asyncapi_import.rs:778,825`); JSON parsing (:218-226) is untested;
  real inputs exist at `specs/unfolded-circle/{UCR-core,UCR-integration,UCD2}-asyncapi.yaml`.
  It builds `connection_params: vec![]` (:387) and never a `ConnectionParam`;
  role selection `WsRole{Client, Server, Both}` via `--ws-role` (default
  `client`). Phase 3's "JSON/YAML imported-fixture regressions" need a JSON
  fixture created from a real YAML spec.
- Dependents: `schematic-define` ← oauth, gen, definitions, schema;
  `schematic-definitions` ← gen, schema, `model-citizen/lib`;
  `schematic-runtime` ← schema; `schematic-schema` ← `sniff/lib`,
  `biscuit-speaks/lib`, `homelab/lib`, integration crates. All five R15 homelab
  crates are covered by `homelab`'s `just test`
  (`_test_all "homelab homelab-cli homelab-server unfolded-integration-helper
  arcam-amp-integration eversolo-integration sony-receiver-integration"`).
- Test Toolkit: `test_toolkit::EnvGuard::set_safe` (restores on drop),
  `init_test_tracing`, `trace_phase!`.

### Input robustness matrix — finalized inventory

The plan's matrix stands with these substitutions from the refresh:

- **BOUND** = integer seconds ≥ 1; `0` is a field-addressed Error for
  `correlation.default_timeout_secs`, `heartbeat.interval_secs`,
  `heartbeat.timeout_secs`. Absent `default_timeout_secs` → DEFAULT 30
  (upstream serde default). Implementing this changes the existing hint
  readers (they accept 0 today) — Phase 3 owns that change and its tests.
- Presence: today every `Option` hint with `#[serde(default)]` merges absent
  and `null` (`define/src/websocket/hints.rs`, `mod.rs`); the matrix's
  "explicit null → ERR, distinct from absent" cells require presence-aware
  deserialization for each new/changed field (`role`, `responds_with`,
  `selectors`, `fallback`, `runtime`, `correlation`, `auth_flow`,
  `heartbeat`, indicator and timeout leaves).
- New load-bearing leaves found during inventory, each needing its own column
  before its reader changes (Phase 3): `WebSocketApi.env_mapping`,
  `WebSocketApi.headers`, `WebSocketApi.auth_policy`, and
  `WebSocketEndpoint.params` (`EndpointParams`). Their nested shapes are owned
  by upstream readers (`EnvMapping`, `AuthPolicy`, `EndpointParams` with its
  required `query/header/cookie`); Phase 3 links those upstream matrix tests
  and adds only the WS-level absent/null/wrong-type/empty cells.
- Duplicate-key and trailing-content cells must be built from raw text (the
  definition loader is `serde_json`, which rejects trailing content and, by
  default, takes the last duplicate for structs only when not
  `deny_unknown_fields`… Phase 3 must verify duplicate-key behavior against
  the actual loader before asserting ERR, and add rejection if it is
  last-wins).
- The AsyncAPI reader is unchanged by this phase; the definition matrix does
  not cover it.

### Setup spike (Wave 2)

Disposable prototype built outside the repo (`/tmp/ws-setup-spike`, offline,
same tokio 1.52.1 / tokio-tungstenite 0.26.2 / futures-util / serde versions as
`schematic/schema`, no new crate, no tokio-util). Its source is preserved as
non-compiled reference text in `setup-spike/` beside this log
(`lib.rs.txt`, `setup_tests.rs.txt`, `Cargo.toml.txt`) so Phase 6 does not
repeat the spike. 10 tests, all passing on five consecutive runs
(0.53–0.54 s per run; longest test ~150 ms).

**Signatures (adopted for Phase 6; names provisional until emitted):**

```rust
pub async fn connect_with_setup<F, Fut>(
    &self, /* endpoint params */, opening: {Endpoint}Opening, setup: F,
) -> Result<{Endpoint}Client, WsError>
where
    F: Fn(SetupSession) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), SetupError>> + Send + 'static;

impl SetupSession {
    pub fn credential(&self) -> Option<&RedactedCredential>; // Debug prints [redacted]
    pub async fn send(&mut self, msg: &{Endpoint}ClientMessage) -> Result<(), SetupError>;
    pub async fn next_message(&mut self) -> Result<{Endpoint}ServerMessage, SetupError>; // skips control frames
    // value counterparts: send_value / next_value
}

pub enum SetupError { Rejected { reason }, Transient { reason }, Closed, Io, Codec, SessionInvalid }
// only Rejected is terminal
```

- The opening payload is taken **by value** (the spike's `Option<&O>` made the
  connect future non-`'static` and `Send` only for `O: Sync`). Its serialized
  form is retained privately (`Arc<str>`) and re-sent first on every dial.
- **Ownership/transfer:** the stream lives in `Arc<std::sync::Mutex<Slot>>`
  with `Slot = Idle(Box<Transport>) | Busy | Revoked`, shared by the session
  and a supervisor-owned `SlotGuard`. Each session operation checks the stream
  out (no lock across `.await`), runs its I/O in `select!` against a revocation
  `watch`, and checks it back in only while still `Busy`. On callback `Ok` the
  supervisor calls `guard.reclaim()` (swaps in `Revoked`, returns the stream)
  and only then publishes `Ready` and resets the counter. `SlotGuard::drop`
  (timeout, close, error, drop) revokes, drops an idle stream at once and
  aborts in-flight session I/O — no detached task can keep the stream. A
  session leaked into a user task returns `SessionInvalid`; an `Ok` while a
  detached operation holds the stream is `SessionLeaked`, never a silent
  hand-back. Box the `Idle` transport (clippy `large_enum_variant`).
- Rejected alternatives: `&mut SetupSession` callback (needs HRTB
  `for<'a> Fn(&'a mut S) -> BoxFuture<'a, _>`, breaks the spec's
  `|session| async {..}`); callback returning the session (awkward, wrong
  session can be returned, does not stop escape); plain
  `Arc<tokio::Mutex<Option<Stream>>>` (a detached task holding the guard across
  `recv` blocks reclamation).
- **Timing:** one `tokio::time::timeout` per attempt covers dial, upgrade,
  opening write and callback, with budget `min(remaining deadline,
  per_attempt)` before first `Ready` and `per_attempt` after. The initial
  attempt runs inline; the supervisor task is spawned only after the first
  `Ready`, so dropping the connect future leaves no task.
- **Shutdown:** close and drop signal a `watch<bool>` (sender drop = shutdown)
  raced against the connected loop, backoff sleep and attempt; close returns
  in < 500 ms during a 60 s backoff and during a stalled 60 s setup.
- **Bounds vs consumers:** `Fut: Send + 'static` (redials run in
  `tokio::spawn`), `F: Sync` (supervisor holds `&F` across awaits), connect
  future `Send`. Fits `homelab/lib/src/samsung_tv.rs:229-299`, called from axum
  handlers (`homelab/server/src/handlers/samsung_tv.rs:311-319,367`) and the
  CLI (`homelab/cli/src/main.rs:1980`). A plain
  `async fn(SetupSession) -> Result<(), SetupError>` satisfies the factory
  bound; closures need `move` plus cloned `Arc`s. Integration crates use only
  the host and are unaffected.
- **Surprise (affects Phase 2/6 test design):** on macOS loopback, after the
  peer drops TCP or sends Close then drops, the *first* client write returns
  `Ok(())`; only later writes fail (`EPIPE`). A peer close does not reliably
  fail the next opening write, so R3's open-write-failure case needs the
  injectable failing writer (plan Wave 3). Linux may differ (`ECONNRESET`).
- On `Rejected` the prototype drops the stream without a Close frame; Phase 6
  may add a bounded graceful close.

**Retained regression test: none in Phase 1.** A test of the prototype would
not exercise shipped code. The useful regression is recorded for Phase 6:
`setup_cancellation_drops_stream_and_invalidates_session` in
`schematic/schema/tests/ws_runtime.rs`, built on the Phase 2 harness —
a timed-out stalled callback and a close during a stalled redial setup both
leave the server observing EOF, and a session moved out of the callback
returns `SessionInvalid`. Cases (a), (c), (d) are already required acceptance
tests (R2, R3).

No performance spike was run (plan: correctness risks only).

### Gates and verification (Phase 1 close)

- **Behavior changes: none.** Phase 1 is readiness, inventory and a disposable
  spike; no source, test, generated output, doc page or skill changed. There
  is therefore no requirement-to-test mapping to report for this phase and no
  targeted test was added (the one useful regression is scheduled for Phase 6;
  see "Setup spike"). The input-robustness matrix is inventoried above;
  its tests land with the readers in Phase 3.
- **Gate-skip check:** `git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` →
  `packages: [schematic-define, schematic-definitions, schematic-gen,
  test-toolkit, darkmatter]`, `change_class: package` — non-empty because of
  the pre-existing unrelated changes in the tree. (Restricted to this phase's
  files: `packages: []`, `change_class: documentation`.) Gates were run.
- `just test` (schematic): **pass** — 2684 passed, 15 skipped (L1);
  `test-fresh-output` 9/9; `test-compile-fail` 1/1. Four pre-existing
  `schematic-runtime::l1 builder::*` TLS tests
  (`accepting_invalid_certificates_reaches_a_self_signed_server_that_the_default_rejects`,
  `an_added_der_root_…`, `an_added_pem_root_…`,
  `without_the_added_root_the_handshake_fails_as_a_tls_error`) were flagged
  SLOW (> 5 s) in the parallel run; rerun alone they took 3.1–3.9 s and
  passed. Recorded as **load-sensitive** (code untouched by this phase).
- `just lint` (schematic): **pass**.
- Dependent-package tests: not run — no package source changed.
- Cross-OS check: not run — no OS-sensitive change.
- Skills: no update; nothing about package behavior changed. Repo facts found
  (CI does not run `schematic/schema` tests; `artifact_drift` is plain L1) are
  recorded here for the author rather than written into a skill.
- No commits, no regeneration, no live device/provider contact, no CI runs.

Phase 1 checkpoint met: refreshed contracts, readiness/authorization state,
the endpoint expectation table, the matrix inventory and a feasible setup API
are recorded; no prerequisite is missing in a way that blocks Phase 2.
