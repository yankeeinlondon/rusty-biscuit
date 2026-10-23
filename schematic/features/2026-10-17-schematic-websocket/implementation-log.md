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
packages:
    - schematic-define
    - schematic-definitions
    - schematic-gen
    - schematic-schema
    - homelab
    - unfolded-integration-helper
    - arcam-amp-integration
    - eversolo-integration
    - sony-receiver-integration
    - schematic-runtime
human_review: false
source_files_during_phase_2:
    - schematic/definitions/src/unfolded_circle/core_ws/mod.rs
    - schematic/gen/src/diagnostics.rs
    - schematic/gen/src/output/ws_modules.rs
    - schematic/gen/src/pipeline.rs
    - schematic/gen/src/ws_codegen/plan.rs
    - schematic/gen/src/ws_codegen/shared.rs
    - schematic/gen/tests/common/fresh_output.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
    - schematic/gen/tests/l1/main.rs
    - schematic/gen/tests/l1/ws_validation.rs
    - schematic/schema/src/unfolded_circle_core_ws.rs
    - schematic/schema/tests/ws_mock/mod.rs
    - schematic/schema/tests/ws_mock_harness.rs
    - schematic/schema/tests/ws_runtime.rs
docs_updated_during_phase_2:
    - schematic/docs/generation/validation.md
    - schematic/docs/generation/generated-clients.md
    - schematic/docs/generation/workflow.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/schematic-define/schematic/gen.md
source_files_during_phase_3:
    - schematic/define/src/lib.rs
    - schematic/define/src/params/param_def.rs
    - schematic/define/src/prelude.rs
    - schematic/define/src/serde_presence.rs
    - schematic/define/src/websocket/hints.rs
    - schematic/define/src/websocket/lifecycle.rs
    - schematic/define/src/websocket/mod.rs
    - schematic/define/src/websocket/params.rs
    - schematic/define/tests/builders.rs
    - schematic/definitions/src/elevenlabs/mod.rs
    - schematic/definitions/src/samsung_smart_tv/remote_ws/mod.rs
    - schematic/definitions/src/unfolded_circle/core_ws/mod.rs
    - schematic/definitions/src/unfolded_circle/dock_ws/mod.rs
    - schematic/definitions/src/unfolded_circle/integration_ws/mod.rs
    - schematic/definitions/tests/ws_conformance.rs
    - schematic/definitions/tests/ws_definition_json.rs
    - schematic/gen/src/asyncapi_import.rs
    - schematic/gen/src/ws_codegen/client.rs
    - schematic/gen/src/ws_codegen/host.rs
    - schematic/gen/src/ws_codegen/plan.rs
    - schematic/gen/tests/fixtures/asyncapi/UCR-integration-asyncapi.json
    - schematic/gen/tests/l1/asyncapi_import.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
    - schematic/gen/tests/l1/main.rs
    - schematic/gen/tests/l1/ws_codegen.rs
    - schematic/gen/tests/l1/ws_validation.rs
    - schematic/schema/src/elevenlabs_ws.rs
    - schematic/schema/src/samsung_smart_tv_remote_ws.rs
    - schematic/schema/src/unfolded_circle_core_ws.rs
    - schematic/schema/src/unfolded_circle_dock_ws.rs
    - schematic/schema/src/unfolded_circle_integration_ws.rs
    - schematic/schema/tests/ws_runtime.rs
docs_updated_during_phase_3:
    - schematic/README.md
    - schematic/docs/definition-builders.md
    - schematic/docs/generation/generated-clients.md
    - schematic/docs/generation/validation.md
docs_created_during_phase_3:
    - schematic/docs/websocket-definitions.md
skills_files_updated_during_phase_3:
    - .claude/skills/schematic-define/SKILL.md
    - .claude/skills/schematic-define/schematic/gen.md
    - .claude/skills/schematic-define/schematic/websocket.md
    - .claude/skills/unfolded-circle/core-api.md
source_files_during_phase_4:
    - homelab/arcam-amp-integration/src/handler.rs
    - homelab/eversolo-integration/src/handler.rs
    - homelab/lib/src/samsung_tv.rs
    - homelab/sony-receiver-integration/src/handler.rs
    - homelab/unfolded-integration-helper/Cargo.toml
    - homelab/unfolded-integration-helper/src/envelope.rs
    - homelab/unfolded-integration-helper/src/lib.rs
    - homelab/unfolded-integration-helper/src/subscriptions.rs
    - homelab/unfolded-integration-helper/src/test_fixtures.rs
    - schematic/definitions/src/unfolded_circle/dock_ws/types.rs
    - schematic/definitions/src/unfolded_circle/integration_ws/types.rs
    - schematic/gen/src/asyncapi_import.rs
    - schematic/gen/src/ws_codegen/client.rs
    - schematic/gen/src/ws_codegen/host.rs
    - schematic/gen/src/ws_codegen/plan.rs
    - schematic/gen/src/ws_codegen/shared.rs
    - schematic/gen/tests/l1/asyncapi_import.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
    - schematic/gen/tests/l1/ws_codegen.rs
    - schematic/schema/src/elevenlabs_ws.rs
    - schematic/schema/src/samsung_smart_tv_remote_ws.rs
    - schematic/schema/src/unfolded_circle_core_ws.rs
    - schematic/schema/src/unfolded_circle_dock_ws.rs
    - schematic/schema/src/unfolded_circle_integration_ws.rs
    - schematic/schema/src/ws_shared.rs
    - schematic/schema/tests/ws_runtime.rs
docs_updated_during_phase_4:
    - homelab/eversolo-integration/README.md
    - homelab/unfolded-integration-helper/README.md
    - schematic/README.md
    - schematic/docs/generation/generated-clients.md
    - schematic/docs/generation/validation.md
    - schematic/docs/websocket-definitions.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/schematic-define/schematic/websocket.md
source_files_during_phase_5:
    - schematic/gen/src/codegen/api_struct/auth.rs
    - schematic/gen/src/codegen/api_struct/mod.rs
    - schematic/gen/src/ws_codegen/client.rs
    - schematic/gen/src/ws_codegen/host.rs
    - schematic/gen/src/ws_codegen/plan.rs
    - schematic/gen/src/ws_codegen/shared.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
    - schematic/gen/tests/l1/ws_codegen.rs
    - schematic/gen/tests/l1/ws_validation.rs
    - schematic/runtime/src/client.rs
    - schematic/runtime/src/credentials.rs
    - schematic/runtime/src/lib.rs
    - schematic/runtime/tests/l1/handshake_credentials.rs
    - schematic/runtime/tests/l1/main.rs
    - schematic/schema/src/elevenlabs_ws.rs
    - schematic/schema/src/samsung_smart_tv_remote_ws.rs
    - schematic/schema/src/unfolded_circle_core_ws.rs
    - schematic/schema/src/unfolded_circle_dock_ws.rs
    - schematic/schema/src/unfolded_circle_integration_ws.rs
    - schematic/schema/src/ws_shared.rs
    - schematic/schema/tests/ws_correlation.rs
    - schematic/schema/tests/ws_handshake.rs
    - schematic/schema/tests/ws_mock/mod.rs
    - schematic/schema/tests/ws_mock_harness.rs
    - schematic/schema/tests/ws_runtime.rs
docs_updated_during_phase_5:
    - schematic/README.md
    - schematic/docs/dependencies.md
    - schematic/docs/generation/generated-clients.md
    - schematic/docs/generation/validation.md
    - schematic/docs/headers/README.md
    - schematic/docs/runtime/credentials.md
    - schematic/docs/websocket-definitions.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/schematic-define/SKILL.md
    - .claude/skills/schematic-define/schematic/headers.md
    - .claude/skills/schematic-define/schematic/websocket.md
source_files_during_phase_6:
    - schematic/define/src/websocket/lifecycle.rs
    - schematic/define/src/websocket/mod.rs
    - schematic/definitions/src/elevenlabs/mod.rs
    - schematic/definitions/tests/ws_definition_json.rs
    - schematic/gen/src/ws_codegen/client.rs
    - schematic/gen/src/ws_codegen/host.rs
    - schematic/gen/src/ws_codegen/mod.rs
    - schematic/gen/src/ws_codegen/plan.rs
    - schematic/gen/src/ws_codegen/shared.rs
    - schematic/gen/src/ws_codegen/supervisor.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
    - schematic/gen/tests/l1/ws_codegen.rs
    - schematic/gen/tests/l1/ws_validation.rs
    - schematic/runtime/src/credentials.rs
    - schematic/runtime/tests/l1/handshake_credentials.rs
    - schematic/schema/src/elevenlabs_ws.rs
    - schematic/schema/src/samsung_smart_tv_remote_ws.rs
    - schematic/schema/src/unfolded_circle_core_ws.rs
    - schematic/schema/src/unfolded_circle_dock_ws.rs
    - schematic/schema/src/unfolded_circle_integration_ws.rs
    - schematic/schema/src/ws_shared.rs
    - schematic/schema/tests/ws_idle.rs
    - schematic/schema/tests/ws_mock/mod.rs
    - schematic/schema/tests/ws_mock_harness.rs
    - schematic/schema/tests/ws_runtime.rs
    - schematic/schema/tests/ws_session.rs
docs_updated_during_phase_6:
    - schematic/README.md
    - schematic/docs/generation/generated-clients.md
    - schematic/docs/generation/validation.md
    - schematic/docs/runtime/credentials.md
    - schematic/docs/websocket-definitions.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/schematic-define/schematic/websocket.md
source_files_during_phase_7:
    - schematic/gen/tests/l1/openapi_import_test.rs
    - schematic/schema/tests/ws_handshake.rs
docs_updated_during_phase_7:
    - schematic/docs/websocket-definitions.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
    - .claude/skills/homelab/SKILL.md
    - .claude/skills/os/SKILL.md
    - .claude/skills/os/windows.md
    - .claude/skills/schematic-define/schematic/websocket.md
    - .claude/skills/unfolded-circle/api-clients.md
source_code:
    - homelab/arcam-amp-integration/src/handler.rs
    - homelab/eversolo-integration/src/handler.rs
    - homelab/lib/src/samsung_tv.rs
    - homelab/sony-receiver-integration/src/handler.rs
    - homelab/unfolded-integration-helper/Cargo.toml
    - homelab/unfolded-integration-helper/src/envelope.rs
    - homelab/unfolded-integration-helper/src/lib.rs
    - homelab/unfolded-integration-helper/src/subscriptions.rs
    - homelab/unfolded-integration-helper/src/test_fixtures.rs
    - schematic/define/src/lib.rs
    - schematic/define/src/params/param_def.rs
    - schematic/define/src/prelude.rs
    - schematic/define/src/serde_presence.rs
    - schematic/define/src/websocket/hints.rs
    - schematic/define/src/websocket/lifecycle.rs
    - schematic/define/src/websocket/mod.rs
    - schematic/define/src/websocket/params.rs
    - schematic/define/tests/builders.rs
    - schematic/definitions/src/elevenlabs/mod.rs
    - schematic/definitions/src/samsung_smart_tv/remote_ws/mod.rs
    - schematic/definitions/src/unfolded_circle/core_ws/mod.rs
    - schematic/definitions/src/unfolded_circle/dock_ws/mod.rs
    - schematic/definitions/src/unfolded_circle/dock_ws/types.rs
    - schematic/definitions/src/unfolded_circle/integration_ws/mod.rs
    - schematic/definitions/src/unfolded_circle/integration_ws/types.rs
    - schematic/definitions/tests/ws_conformance.rs
    - schematic/definitions/tests/ws_definition_json.rs
    - schematic/gen/src/asyncapi_import.rs
    - schematic/gen/src/codegen/api_struct/auth.rs
    - schematic/gen/src/codegen/api_struct/mod.rs
    - schematic/gen/src/diagnostics.rs
    - schematic/gen/src/output/ws_modules.rs
    - schematic/gen/src/pipeline.rs
    - schematic/gen/src/ws_codegen/client.rs
    - schematic/gen/src/ws_codegen/host.rs
    - schematic/gen/src/ws_codegen/mod.rs
    - schematic/gen/src/ws_codegen/plan.rs
    - schematic/gen/src/ws_codegen/shared.rs
    - schematic/gen/src/ws_codegen/supervisor.rs
    - schematic/gen/tests/common/fresh_output.rs
    - schematic/gen/tests/fixtures/asyncapi/UCR-integration-asyncapi.json
    - schematic/gen/tests/l1/asyncapi_import.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
    - schematic/gen/tests/l1/main.rs
    - schematic/gen/tests/l1/openapi_import_test.rs
    - schematic/gen/tests/l1/ws_codegen.rs
    - schematic/gen/tests/l1/ws_validation.rs
    - schematic/runtime/src/client.rs
    - schematic/runtime/src/credentials.rs
    - schematic/runtime/src/lib.rs
    - schematic/runtime/tests/l1/handshake_credentials.rs
    - schematic/runtime/tests/l1/main.rs
    - schematic/schema/src/elevenlabs_ws.rs
    - schematic/schema/src/samsung_smart_tv_remote_ws.rs
    - schematic/schema/src/unfolded_circle_core_ws.rs
    - schematic/schema/src/unfolded_circle_dock_ws.rs
    - schematic/schema/src/unfolded_circle_integration_ws.rs
    - schematic/schema/src/ws_shared.rs
    - schematic/schema/tests/ws_correlation.rs
    - schematic/schema/tests/ws_handshake.rs
    - schematic/schema/tests/ws_idle.rs
    - schematic/schema/tests/ws_mock/mod.rs
    - schematic/schema/tests/ws_mock_harness.rs
    - schematic/schema/tests/ws_runtime.rs
    - schematic/schema/tests/ws_session.rs
documentation:
    - .claude/skills/homelab/SKILL.md
    - .claude/skills/os/SKILL.md
    - .claude/skills/os/windows.md
    - .claude/skills/schematic-define/SKILL.md
    - .claude/skills/schematic-define/schematic/gen.md
    - .claude/skills/schematic-define/schematic/headers.md
    - .claude/skills/schematic-define/schematic/websocket.md
    - .claude/skills/unfolded-circle/api-clients.md
    - .claude/skills/unfolded-circle/core-api.md
    - homelab/eversolo-integration/README.md
    - homelab/unfolded-integration-helper/README.md
    - schematic/README.md
    - schematic/docs/definition-builders.md
    - schematic/docs/dependencies.md
    - schematic/docs/generation/generated-clients.md
    - schematic/docs/generation/validation.md
    - schematic/docs/generation/workflow.md
    - schematic/docs/headers/README.md
    - schematic/docs/runtime/credentials.md
    - schematic/docs/websocket-definitions.md
    - schematic/features/2026-10-17-schematic-websocket/setup-spike/Cargo.toml.txt
    - schematic/features/2026-10-17-schematic-websocket/setup-spike/lib.rs.txt
    - schematic/features/2026-10-17-schematic-websocket/setup-spike/setup_tests.rs.txt
completed_phase: 7
implemented: true
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

## Phase 2

Started 2026-10-22. The working tree was clean at the start (the unrelated
discriminated-union work noted in Phase 1 had been committed), so the
uncommitted tree is exactly this phase's changes.

### What changed

- **Diagnostics (R9).** `ws_codegen/plan.rs`: `validate_ws_api` no longer
  returns at the first structural problem; every finding (empty name,
  `base_url`, endpoint id, correlation fields, unresolved path placeholder) is
  pushed as a `WsDiagnostic`. `lower_to_plan` now returns
  `Result<WsRuntimePlan, WsLoweringError>`; the error carries **every**
  finding of the run, and a returned plan holds only warnings and info.
  `WsDiagnostic::to_validation_diagnostic(api)` maps onto the shared
  `ValidationDiagnostic` (Warning/Error 1:1, `Output::Rust`, new
  `Feature::WebSocketDefinition` = `websocket-definition`, endpoint taken from
  an `endpoints/{id}/…` path, message prefixed with the definition path); Info
  maps to `None`.
- `output/ws_modules.rs`: the `Err(_) => return None` swallow is gone.
  `generate_ws_definition_modules` collects every shipped definition's
  lowering failure and returns `GeneratorError::Validation { output: Rust }`,
  so `prepare_rust_output` (and `generate_and_write_all`) write nothing, as
  for any other Rust validation error. New public helpers:
  `shipped_ws_apis`, `validate_ws_apis` / `validate_ws_definition_modules`
  (returns `WsValidation { diagnostics, notes }`), and `render_ws_api_module`
  (renders any definition's module without the re-export/`define_api`, used
  by the fresh-output compile test).
- `pipeline.rs`: the Rust output's diagnostics for `generate` and `validate`
  are the REST batch's plus the shipped WebSocket definitions'
  (`rust_diagnostics`), so they print and block through the unchanged shared
  `publish`. Info notes print as `[INFO] severity=info …` only when
  `verbose > 0`.
- **URL (R1).** Core `base_url` is now the origin `ws://remote.local`; the four
  channel paths are unchanged. New Warning in `validate_ws_api`: the last
  segment of the `base_url` path equal to the endpoint path's first segment
  (`duplicated_boundary_segment`, whole segments only). Never an Error.
- **Bearer host (R4).** Removed the trailing `;` after the final `Err(...)` in
  `generate_host_validate_bearer_header` (`ws_codegen/shared.rs`).
- **Harness (R16).** `schema/tests/ws_mock/mod.rs`: `MockWsServer` (loopback
  port 0, scripts `Accept(steps)` / `Refuse` / `Redirect` / `Stall` /
  `DropTcp`, steps `Send` / `Receive(n)` / `Wait(Barrier)` / `Stall` / `Close`
  / `Drop`, last script repeats), `RecordedRequest` (path, query, lowercased
  headers, `target()`), connection count, frame log, event-driven
  `wait_for_*` (watch channel, no polling), deterministic `shutdown()`
  (listener closed and all connection tasks joined when it returns; `Drop`
  aborts), and `FailingStream` + `FailSwitch` (writes fail with `BrokenPipe`
  once armed; used with `tokio_tungstenite::client_async`).
- Regenerated with `just generate` once: only
  `schema/src/unfolded_circle_core_ws.rs` changed (base URL literal and its
  module doc line). No OpenAPI/Postman change (WS has none).

### Adopted defaults and deviations

- adopted default: one shared feature id, `websocket-definition`, for every
  lowered-WS finding, with the definition path leading the message (source:
  Phase 1 "Refreshed contracts" diagnostics row, "add WS Feature
  variant(s)"). Alternative: a separate id per WS rule; rejected as
  speculative until a consumer filters on it.
- adopted default: Info notes are printed by `generate`/`validate` only with
  `-v` (source: Phase 1 adopted mapping; the shared spec has no verbosity rule
  for info, and the shared `Severity` gains no `Info`). Besides the
  receive-timeout TODO there are two other pre-existing infos (inferred
  correlation, header-based auth with auth messages); they follow the same
  rule.
- The WS diagnostics are added to the **Rust output of every run**, including
  `validate --api <rest-api>`, because every Rust run writes the five WS
  modules; a WS error therefore fails the Rust output of any run. Shipped
  definitions currently report no warning or error.
- R9's example Error ("duplicate message name or unresolved `responds_with`")
  does not exist until Phase 3/4; the test uses the existing Error for an
  unresolved path placeholder instead.
- Test Toolkit not used in `schematic-schema` tests: that crate's manifest is
  generated by `schematic-gen` and has its own lockfile outside the
  workspace; adding `test-toolkit` would change the generated manifest and
  lockfile for one env guard. The area's existing `tests/common::EnvGuard`
  (restores on drop, `#[serial]`) isolates the two Core credential variables
  instead. The harness itself reads no environment.
- The R4 host test runs inside the fresh crate (`nextest_run` of a child test
  file) rather than in `schematic-schema`, because no shipped definition uses
  `BearerToken`; the host exists only in the rendered fixture module.
- R4 compile test is `slow_bearer_ws_host_compiles_and_checks_tokens` in
  `fresh_output_compile` (author ruling, plan rule 10): it is `slow_` because
  every test in that module drives Cargo and is selected by
  `just test-fresh-output`, which `just test` invokes.
- `FreshCrate::add_module` (test support) writes a rendered module into the
  fresh crate and declares it in `lib.rs`; `FreshCrate::generate(&[], …)`
  is used so only the shared runtime and WS modules are built.
- Dock URL (`ws://dock.local/`, missing the documented `:946`) is asserted as
  it stands with a comment; still new scope (Phase 1 finding 5).

### Requirement-to-test map

| Requirement | Boundary / rung | Test(s) | Regression proof |
|---|---|---|---|
| R1 definition | crate API, F1 | `schematic-definitions` `unfolded_circle::core_ws::tests::core_ws_has_expected_endpoints` (base URL + exact `(id, path)` list) | — |
| R1 composed URLs, all five | gen public API, F1 | `l1 ws_validation::every_shipped_endpoint_composes_its_documented_url`, `every_shipped_module_is_generated` (generated Core literal) | failed before the definition fix (`ws://remote.local/ws/ws`, …) |
| R1 Warning / no false positives | gen public API, F1 | `ws_validation::an_exactly_duplicated_boundary_segment_warns_without_blocking`, `a_legitimately_repeated_segment_is_only_a_warning`, `a_substring_or_prefix_overlap_emits_nothing` (`/api`+`/apikeys`, `/apikeys`+`/api`, `/api`+`/v1/api`, no base path); unit `duplicated_boundary_segment_matches_whole_segments_only` | — |
| R1 shipped clean | gen public API | `ws_validation::shipped_definitions_report_no_warning_or_error` | failed before the fix with the Core `/ws/ws` warning |
| R1 request target | generated client ↔ loopback server, F3 | `schematic-schema` `ws_runtime::core_endpoints_dial_their_documented_channel_paths` (all four Core endpoints from the default base URL; targets `/ws`, `/intg`, `/profiles`, `/events`; no API-key header with credentials cleared) | fails against the pre-fix generated module (`HEAD` copy) |
| R4 compile | generated crate, Cargo, F3 | `l1 fresh_output_compile::slow_bearer_ws_host_compiles_and_checks_tokens` (`cargo check` of the rendered Bearer host) | with the `;` restored the child build fails `E0308` (expected `Result`, found `()`) |
| R4 401 vs upgrade | generated host ↔ tungstenite client over loopback, F3 | child tests `bearer_host_rejects_a_missing_token_with_401`, `…_wrong_token_with_401`, `bearer_host_upgrades_a_matching_token` (101 + echoed frame), asserted present in the child report | same build failure |
| R9 Error in shared format, writer semantics | gen public API | `ws_validation::a_lowering_error_reaches_the_shared_format_and_blocks_the_rust_output` (exact rendered lines, `has_errors`, `render_ws_api_module` → `Validation { output: Rust }` with all findings) | before, the module was dropped with no diagnostic |
| R9 Warning does not fail | gen public API | `an_exactly_duplicated_boundary_segment_warns_without_blocking` (render `Ok`) | — |
| R9 no swallowed `Err` | unit | `plan::tests::lowering_failure_keeps_every_finding_not_only_the_first_error`, `validate_rejects_*` (now assert the exact error path) | previously the first error returned alone |
| R9 mapping | unit | `plan::tests::diagnostic_maps_onto_the_shared_format`; `diagnostics::tests::display_and_serialized_ids_agree_for_every_variant` (now lists `WebSocketDefinition`, `GraphQlOperation`) | — |
| R9 info verbosity | CLI wiring (one spawn pair) | `ws_validation::info_notes_stay_out_of_the_shared_diagnostics`, `the_validate_command_prints_info_notes_only_when_verbose` | — |
| R16 harness | loopback, L1 | `schematic-schema` `ws_mock_harness::*` (10): request record, frames in order, drop after N frames, refuse 403 / redirect 307 with `Location` / last script repeats / connection count, TCP drop, stall, barrier, deterministic shutdown (client sees end, listener closed), 8 concurrent servers on distinct ports with isolated records, armed `FailingStream` fails the next write | — |

Input robustness matrix: not applicable to this phase; no reader of a file
format or configuration changed (the definition readers change in Phase 3).

Tier placement: every new gen test is in the declared `l1` binary
(`ws_validation` added to `tests/l1/main.rs`); the Bearer test is `slow_` in
`fresh_output_compile`, selected by `just test-fresh-output`. The schema tests
are ordinary per-file test targets of `schematic-schema` (no
`autotests = false`); `tests/ws_mock/mod.rs` is a module directory, not a
target. As recorded in Phase 1, CI does not run `schematic-schema` tests;
their evidence is local.

### Gates and verification (Phase 2 close)

- **Gate-skip check:** `git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` →
  `packages: [schematic-definitions, schematic-gen, test-toolkit, darkmatter,
  schematic-define]`, `change_class: package`. Gates were run. Per file: the
  skill edit selects a `darkmatter` docs test, `generated-clients.md` a
  `schematic-define` test, and every schematic source the `test-toolkit`
  `archive-path-guard` lint companion.
- `just test` (schematic): **pass** — L1 2696 passed, 16 skipped;
  `test-fresh-output` 10/10 (includes the new
  `slow_bearer_ws_host_compiles_and_checks_tokens`, 53 s pass total);
  `test-compile-fail` 1/1. No SLOW L1 test this run.
- `just lint` and `just lint-schema`: **pass** (after allowing
  `clippy::result_large_err` in the two new schema test files, as
  `ws_runtime.rs` already does for tungstenite's large `Error`).
- `just test-schema`: **pass**, 76/76; the one SLOW (> 60 s) is the
  pre-existing trybuild `compile_fail::slow_generated_pager_surface_…`.
- `just check-drift`: **pass**, 9/9 (`artifact_drift::*` +
  `slow_real_apis_compile_offline`); regeneration left the tree drift-free.
- `just check-tier-coverage schematic`: 0 stranded.
- `test-toolkit` `archive_path_guard`: **failed first** on
  `ws_validation.rs` using `env!("CARGO_BIN_EXE_schematic-gen")`, which an
  archived CI run cannot resolve; replaced with
  `biscuit_test_harness::bin_exe!("schematic-gen")`; now 2/2.
- darkmatter tests that read skills (`shipped_schema_corpus`,
  `current_root_documentation_contract`, `current_root_migration_guard`):
  11/11.
- Dependents: no dependent references a changed symbol (`rg` for `core_ws`,
  `UnfoldedCircleCoreWs`, `lower_to_plan`, `ws_modules` in `homelab`,
  `sniff`, `biscuit-speaks`, `model-citizen`: none). The only public change
  they can see is the Core default base-URL value. `cargo check -p homelab -p
  unfolded-integration-helper -p sniff -p biscuit-speaks -p model-citizen`:
  pass. Their test suites were not run (no symbol they use changed).
- **Cross-OS (native Windows, `build-win-native`, via
  `scripts/cross-check.sh` native mode):**
  - `schematic-schema --manifest-path schematic/schema/Cargo.toml --test
    ws_mock_harness --test ws_runtime`: 15/15 pass.
    `shutdown_ends_open_connections_and_closes_the_listener` takes 2.1 s
    there (a refused loopback connect retries for about 2 s on Windows; already
    recorded in the `os` skill, `windows.md`); under the 5 s budget.
  - `schematic-gen --test l1 slow_bearer_ws_host`: 1/1 pass (144 s, first
    child build on that host).
  - Linux and WSL2 were not run; the harness uses only loopback TCP and
    tokio, with no Unix-only API.
- Skills: `.claude/skills/schematic-define/schematic/gen.md` (WS validation on
  the Rust output, fixture helpers). The `os` skill already held the one OS fact
  met here.
- No commits, no CI runs, no device or provider contact, no stale-file
  deletion.

Phase 2 checkpoint met: R1, R4, R9 and R16 pass through public boundaries
(definition, gen public API, compiled generated host, generated client against
the loopback server); every WS module compiles in the fresh-output guard and
no lowering diagnostic is dropped. The supervisor behavior (R2, R3, R8)
remains pending for Phase 6.

## Phase 3

Started 2026-10-22. The Phase 2 changes were still uncommitted when this phase
began, so the working tree holds both phases; the `*_during_phase_3`
frontmatter lists only files this phase touched (Phase 2's files that this
phase edited again are listed too). No subagents were used: Waves 5–7 touch
the same few files in sequence.

### What changed

- **Vocabulary (R12, `schematic-define`).** `websocket/params.rs`
  (`ConnectionParam`, `ParamType`) is deleted, with its re-exports in
  `lib.rs`/`prelude.rs`. `WebSocketEndpoint.connection_params` is replaced by
  `params: EndpointParams` (REST location containers, typed `params.path`)
  and gains `role: EndpointRole` (`client`/`host`/`both`, default `Client`).
  `WebSocketApi` gains `auth_policy: Option<AuthPolicy>`,
  `env_mapping: Option<EnvMapping>`, `headers: Vec<(String, String)>`, plus
  `effective_auth_policy()` and `default_env_mapping()` (REST derivation via
  `RestApi::legacy_env_mapping_for`). `MessageSchema` gains
  `responds_with: Option<String>`, `selectors: Option<Vec<MessageSelector>>`,
  `fallback: bool`, and the builder `new/describe/matches/fallback/responds_with`.
  `MessageSelector { field, value }` (string equality on a top-level field).
- **Presence-aware serde.** Every new or changed optional field uses
  `serde_presence::non_null` (absent = none, `null` = error). Finding during
  the matrix: serde's derived structs also accept a JSON array
  (`"runtime": []` loaded as default hints). New
  `serde_presence::map_only` / `non_null_map` accept only an object; used for
  `params`, both `runtime`s, `auth_policy`, `env_mapping`, each hint object,
  and (through a hand-written `Deserialize`) `MessageSelector`.
- **Definitions (R13).** All five rewritten onto the builder:
  - Core: 13 messages with `kind`/`msg` selectors, `ResponseEnvelope`
    fallback, pairs Auth→Authentication, Ping→Pong, Version→VersionInfo,
    System→SystemInfo, RequestEnvelope→ResponseEnvelope; explicit
    `CorrelationHints{id, req_id, 30}` and `AuthFlowHints` (was prose only);
    explicit `EnvMapping.api_key` (API-KEY). The skill's `get_version`/
    `get_system` were wrong: the official Core AsyncAPI uses `version`/
    `system` (skill fixed).
  - Dock: selectors on the official UCD2 wire values (`type=req|event|
    auth_required|auth`, `type=auth,msg=authentication`), explicit
    correlation, `failure_indicator` `authentication_failed` removed (the
    spec has no such message; failure is a `code` on `authentication`).
  - Integration: role `Both`; true directions (RequestEnvelope/Auth `Client`,
    ResponseEnvelope/AuthRequired/Authentication `Server`, EventEnvelope the
    only `Bidirectional`); explicit correlation, auth flow, env mapping. The
    contortion test now asserts the true directions.
  - Samsung: `name`/`token` as query `ParamDef`s (`token` marked secret);
    ChannelEnvelope `Server` + fallback (was `Bidirectional`); `event`/`method`
    selectors; API runtime hints removed (no correlation evidence, so the
    explicit `RequestIdType::String` was unconsumed).
  - ElevenLabs: typed `params.path` `voice_id` + query `ParamDef`s; explicit
    `EnvMapping` like its REST sibling. One inbound message per endpoint, so
    no selectors.
- **Planning (`schematic-gen`).** `plan::EndpointRole` is now a re-export of
  the definition's role; `infer_role` is gone. Params lower through
  `lower_params`: header/cookie params are errors; an empty `params.path`
  keeps every placeholder a required string (as REST); otherwise names must
  equal the placeholders and each must be a required `simple` scalar. Query
  params use the REST `query_shape` style rules (single, exploded,
  comma/space/pipe-delimited); enum/JSON/unsupported styles and duplicates
  are errors. `validate_endpoint_path` rejects a path not starting with `/`,
  with its own origin, or with `?`/`#`. `validate_endpoint_fields` rejects
  empty `responds_with`/selector field/auth-flow names and indicators and
  zero durations. `EndpointPlan.messages` keeps the full declared messages.
- **Client emitter.** Query values are percent-encoded per element and
  delimiters written in wire form; exploded arrays repeat the name; unset
  optionals are omitted. Client rustdoc states each declared heartbeat
  (interval, keepalive message and `send_*` helper, reply timeout).
- **AsyncAPI import.** Construction only: `params` empty, explicit
  `env_mapping` (REST derivation), empty `headers`, and the endpoint role from
  `--ws-role` (`client`→Client, `server`→Host, `both`→Both). Direction
  mapping and document reading are unchanged.
- Regenerated once with `just generate`: only the five `*_ws.rs` modules
  changed (query assembly, auth-flow doc lines, doc spacing).

### Adopted defaults and deviations

- adopted default: an untyped placeholder (empty `params.path`) is a required
  string, as for REST endpoints (source: spec "Parameters and URL assembly",
  "reuse the settled REST location containers and typed path parameters").
  Before, an untyped placeholder was a WS Error; the Phase 2 R9 test now uses
  an embedded-query path as its Error example.
- adopted default: `headers` and `params` are required JSON keys (absent →
  error), as `RestApi.headers` and the old `connection_params` were;
  explicit `[]`/empty containers are valid (source: matrix "absent must never
  read as an empty collection").
- adopted default: selector values are JSON strings only (source: matrix
  "`selector.value` … against declared scalar contract"; every shipped wire
  discriminator is a string). A non-string value is a load error.
- adopted default: AsyncAPI `--ws-role server` → `EndpointRole::Host`
  (source: spec "set the role explicitly where known"). Before, `server`
  inverted directions but still generated a client.
- adopted default: header and cookie handshake params are generation errors
  rather than generated (source: plan "Reject … unsupported styles"; adding
  them would be new scope).
- adopted default: `HeartbeatHints` are declared on no shipped endpoint — no
  protocol documents an application heartbeat interval (Core `ping` is an
  ordinary request; Integration keepalive is control frames). The R13
  heartbeat evidence uses a fixture.
- adopted default: Dock `env_mapping` is the explicit empty mapping its
  `AuthStrategy::None` derives; `env_auth [UCR_DOCK_TOKEN]` stays for the
  setup flow (Phase 1 finding 7; Phase 6 decides the binding).
- adopted default: client-direction selectors are declared where the wire
  value is documented and top-level; Samsung `AppLaunchCommand`/
  `InstalledAppsCommand` share `method=ms.channel.emit` and differ only in
  nested `params.event`, so they declare none (outbound-only, no host).
- Deferred to Phase 4 per the plan: selector enforcement in `validate_ws_api`
  (fallback count, overlap, missing selector, `"selectors": []` when
  required). The conformance test checks those properties for the shipped
  five today.
- Deferred to Phase 5: R11 link/hint Errors and Warnings, removal of
  envelope-name inference, presence for `request_id_type`.
- Finding (Phase 4): Dock payload types cannot decode the documented auth
  frames (`DockWsEnvelopeType` lacks `auth_required`/`auth`; see
  `message_to_agent`).
- Serde errors locate a bad field by line/column, not by field path (same as
  REST definitions); a field-path error would need `serde_path_to_error`, a
  new crate, which is not authorized.
- `docs/research/samsung-design.md` still shows `ConnectionParam` code: it is
  a research snapshot, left as written.
- Process slip, corrected: one `git rm --cached` briefly staged the deletion
  of `websocket/params.rs`; it was unstaged immediately (`git restore
  --staged`). Nothing is staged.

### Requirement-to-test map

| Requirement | Boundary / rung | Test(s) |
|---|---|---|
| R12 old names gone | public API, compile-fail doctests | `schematic-define` `websocket` module docs: four `compile_fail` blocks (`websocket::ConnectionParam`, `websocket::ParamType`, root `ConnectionParam`, root `ParamType`) |
| R12 shared parameter serde | unit | `websocket::tests::shared_parameter_types_are_serializable` (`ParamDef`, `QueryParamType`, `ParamStyle`, `EndpointParams`) |
| R12 role names / auth & env derivation | unit | `endpoint_role_names_are_stable`, `derived_policy_and_env_mapping_follow_rest`, `explicit_policy_and_env_mapping_replace_the_derived_ones` |
| R12 round trip | crate API | `ws_definition_json::every_shipped_definition_round_trips_twice_without_loss` (write/read/write/read, byte-identical) |
| R12 five definitions lower | gen API | `plan::tests::lower_all_known_ws_apis`, `ws_validation::shipped_definitions_report_no_warning_or_error`, `every_shipped_endpoint_composes_its_documented_url` |
| R13 roles + true directions | crate API | `ws_conformance::roles_and_directions_match_the_expected_table`; `integration_ws::tests::integration_ws_declares_both_roles_and_true_directions` (replaces the all-`Bidirectional` assertion) |
| R13 selectors / links / flows / env | crate API | `ws_conformance::{every_inbound_direction_can_choose_one_message, every_response_link_resolves_and_is_correlated, challenge_flows_are_declared_as_hints_not_prose, environment_mapping_is_explicit_and_matches_the_handshake_strategy, path_parameters_type_every_placeholder, no_protocol_declares_a_heartbeat_it_does_not_document}` |
| R13 `Both` keeps host | gen API | `ws_codegen::a_both_role_with_one_way_messages_keeps_its_host_and_client`, `integration_ws_has_both_role` (host module non-empty), `plan::tests::endpoint_role_comes_from_the_definition_not_message_directions` |
| R13 heartbeat rustdoc, no warning | gen API | `ws_codegen::a_declared_heartbeat_is_documented_on_the_client_without_a_warning`, `plan::tests::heartbeat_hints_lower_without_any_warning` |
| Plan metadata intact | unit | `plan::tests::lowering_keeps_selectors_fallbacks_and_links` |
| Param/URL rules | unit | `plan::tests::{lower_types_path_and_query_params_from_endpoint_params, untyped_placeholders_are_required_strings, unsendable_handshake_params_are_errors, a_placeholder_without_its_typed_declaration_is_an_error, endpoint_paths_cannot_carry_a_query_or_an_origin}` |
| Path `/`, space, `?`; omitted optional; base-path prefix | generated client ↔ loopback mock (shipped ElevenLabs), F3 | `schematic-schema` `ws_runtime::elevenlabs_handshake_encodes_one_path_segment_and_omits_unset_queries` (local evidence only; CI does not run schema tests) |
| Repeated and delimited query values | compiled fixture client ↔ loopback server, F3, CI-visible | `fresh_output_compile::slow_ws_handshake_parameters_reach_the_server_in_their_declared_form` (child tests `path_value_is_one_encoded_segment_and_unset_options_are_omitted`, `array_values_follow_their_declared_style`) |
| Field-level ERR cells (empty strings, zero BOUND) | gen validator | `plan::tests::declared_names_indicators_and_durations_are_checked` (all nine paths + a minimum-value control) |
| AsyncAPI JSON/YAML construction | gen API on the official Integration AsyncAPI and its JSON conversion | `asyncapi_import::{yaml_and_json_forms_import_the_same_definition_for_every_cli_role, cli_role_keeps_its_direction_mapping, imported_definitions_lower_with_their_declared_role}` |
| Struct fields reject arrays | unit | `serde_presence::tests::map_fields_accept_only_objects` |
| Doc examples compile | doctest | `WebSocketDefinitionsDoc` (`cfg(doctest)` include of `docs/websocket-definitions.md`, 5 blocks) |

### Input robustness matrix (definition JSON)

One loader (serde_json over the derived `WebSocketApi`), tested once in
`schematic-definitions/tests/ws_definition_json.rs` from the shipped Dock
definition plus an added heartbeat and API runtime hints, one edit per cell:

- control: `control_loads_with_every_decision_bearing_field`
- absent: `absent_optional_fields_take_their_documented_meaning` (role →
  Client, responds_with/selectors/hints/runtime/env_mapping → none, fallback
  → false, default_timeout → 30, indicator/timeout → none, runtime leaves →
  defaults) and `required_fields_cannot_be_omitted` (params, selector
  field/value, correlation fields, auth-flow names, interval, headers,
  env_auth)
- explicit null: `explicit_null_is_rejected_and_never_read_as_absent`
  (27 cells)
- wrong type, one/every malformed element, `{}`/`[]` for objects:
  `wrong_types_and_malformed_elements_are_rejected` (51 cells)
- empty: `explicit_empty_values_are_kept_as_declared` (`selectors: []`,
  `runtime: {}`, `env_mapping: {}`, `headers: []` kept; empty strings and a
  zero interval load and are rejected by the validator test above)
- exact values: `declared_values_are_read_exactly`
- duplicate keys (10 fields) and trailing/truncated documents:
  `duplicate_keys_and_trailing_content_are_rejected`

Nested `EnvMapping`, `AuthPolicy`, and `EndpointParams` leaves keep their
upstream readers and tests (`define/tests/definition_serde.rs::
parameter_collections_reject_malformed_json_matrix` for `EndpointParams`);
this phase adds only their WS-level absent/null/wrong-type/empty cells.
Smell grep over the changed readers: no `#[serde(default)]` remains on a
load-bearing `Option` without `non_null`/`non_null_map`; the remaining
`#[serde(default)]`s are `role` (documented default), `fallback`, and the
pre-existing `WebSocketRuntimeHints` leaves whose null is still rejected by
their own types; no `filter_map`/`unwrap_or_default`/`.ok()` on a load path.
The AsyncAPI reader is unchanged; its regression runs on a real document in
both formats.

### Gates and verification (Phase 3 close)

- Gate-skip check: `git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` →
  `packages: [schematic-define, schematic-definitions, schematic-gen,
  test-toolkit, darkmatter]`, `change_class: package`. Gates were run.
- `just lint`: pass (after factoring two type aliases in
  `ws_conformance.rs` for `clippy::type_complexity`). `just lint-schema`:
  pass.
- `just test`: pass — L1 2721 passed, 17 skipped; `test-fresh-output` 11/11
  (includes the new `slow_ws_handshake_parameters_…`, 118 s for the pass);
  `test-compile-fail` 1/1.
- `just test-schema`: pass, 77/77. The one LEAK
  (`artificial_analysis_client::list_image_editing_models_…`, wiremock) and
  the SLOW trybuild `compile_fail::slow_generated_pager_surface_…` are
  pre-existing and unrelated.
- `just check-drift`: pass, 9/9. `just check-tier-coverage schematic`: 0
  stranded.
- Doctests: `schematic-define` 191 after fixing the stale `prelude.rs`
  WebSocket example (it failed first: missing new fields);
  `schematic-definitions` 44 passed.
- `test-toolkit` `archive_path_guard`: 1/1. darkmatter skill/doc corpus
  tests (`shipped_schema_corpus`, `current_root_documentation_contract`,
  `current_root_migration_guard`): 12/12.
- Dependents: no non-schematic crate names a changed definition type.
  `cargo check -p homelab -p unfolded-integration-helper -p
  arcam-amp-integration -p eversolo-integration -p sony-receiver-integration
  -p sniff -p biscuit-speaks -p model-citizen -p schematic-oauth`: pass.
  `homelab` Samsung tests (the only consumer of a regenerated symbol,
  `RemoteControlConnectionParams`): 12/12.
- Cross-OS: not run. The phase adds no path, process, or OS-specific code;
  the new runtime tests use loopback TCP like the Phase 2 harness that
  passed on native Windows, and file reads use `manifest_dir!()` joins.
- Docs: new `docs/websocket-definitions.md` (compiled through
  `cfg(doctest)`), `validation.md` WebSocket error list, `generated-clients.md`
  fresh-output list, `definition-builders.md` Default rationale for
  `EndpointRole`, README link. Skills: new `schematic-define/schematic/
  websocket.md` (+ table row), `gen.md` pointer, `unfolded-circle/core-api.md`
  request names.
- No commits, no CI runs, no device or provider contact, no stale-file
  deletion.

Phase 3 checkpoint met: R12–R13 pass through public boundaries; the
Integration host is still generated (role `Both`); the five definitions
survive JSON write/read/write/read without losing roles, selectors,
fallbacks, links, or hints. R10 typed selection and R11 link execution remain
pending for Phases 4 and 5.

## Phase 4

Started 2026-10-22. Phases 2 and 3 are committed; the working tree holds only
this phase's changes. The generator work (Waves 8–9 host template) was done
by the orchestrator in sequence; three subagents then ran in parallel on
disjoint files: the homelab consumer migration, the `schematic-schema` runtime
tests, and the `schematic-gen` fresh-output fixtures. Each ran only scoped
tests; the area gates ran once afterwards.

### What changed

- **Message enums (R10, `ws_codegen/client.rs`).** Every endpoint gets
  `{Ep}ServerMessage` and `{Ep}ClientMessage`, one variant per `MessageSchema`
  named after it and holding the payload type its `Schema` names
  (`Bidirectional` in both; lifecycle messages are not variants). Only
  `Debug` is derived. A direction gets a payload-only `serde::Serialize` impl
  where the endpoint *sends* it (client → ClientMessage, host → ServerMessage)
  and `decode(WsInboundFrame)` where it *receives* it, so payloads need only
  the serde capability their direction uses; empty directions are
  uninhabited enums. `decode` tries selectors most-specific-first (selector
  count descending; validation guarantees any two matching sets are in a
  strict-subset chain), then the fallback (or a direction's sole unselected
  message), else `WsError::Decode`. Payloads decode from the **raw** frame,
  not the parsed `Value`, because `serde_json::from_value` cannot produce the
  `Box<RawValue>` fields the Unfolded Circle envelopes use.
- **Shared runtime (`ws_codegen/shared.rs`).** New `WsRawFrame`
  (`Text`/`Binary`, exactly as received), `WsInboundFrame` (raw + parsed
  value; `parse`, `matches`, `decode_payload`, `unmatched`), `WsError::Decode
  { frame, reason }`, and `WsEventStream<T>`. The unused `WsEncode`/`WsDecode`
  blanket traits are removed (no caller in the repo).
- **Client surface.** `send({Ep}ClientMessage)`, `send_value(Value)`,
  `next_event()` typed, `next_value_event()`, `events()`/`value_events()`. The
  event queue now carries `WsInboundFrame`, so typed and value receivers read
  one bounded queue and each message is delivered once. Non-JSON frames are
  `Decode` errors that keep the frame; the connection stays open. The generic
  `send_typed` is gone (a private `send_payload` serves lifecycle helpers and
  `request`). The API struct and endpoint clients are now emitted only for
  `Client`/`Both` endpoints (before, `Host` endpoints got a client too, which
  contradicted the documented role table). `request(Value)` is unchanged
  (Phase 5).
- **Defect fixed on the way:** `events()` used to move the receiver out and
  drop the transport handle, whose `Drop` aborts the supervisor — the stream
  ended the connection it was reading. `WsEventStream` now holds the
  transport; the fresh-output child test checks the stream keeps the
  connection and that dropping it ends the connection.
- **Host (R14, `ws_codegen/host.rs`).** `{ApiBase}EventHub` (for Integration
  `UnfoldedCircleIntegrationEventHub`); `HostOutbound` trait implemented by
  each host endpoint's ServerMessage so typed `send_to_connection`,
  `broadcast`, `broadcast_to_subscribers` reach only connections of that
  endpoint; `_value` escape hatches reach any connection. Outbound queues
  carry encoded frames, so fan-out needs no `Clone` payload. `WsHandler`:
  `handle_message(ClientMessage) -> Option<ServerMessage>`, default
  `on_authenticated(ctx) -> Option<ServerMessage>` (None), default
  `on_error(WsError, ctx)`. Multi-endpoint hosts get
  `handle_{ep}_message`/`on_{ep}_authenticated` and route the upgrade by
  request path (`{x}` matches one segment, unknown path → 404); a
  single-endpoint host still accepts any path. The hook's message is written
  directly on the socket before the writer task starts, so it precedes every
  reply or broadcast; an encode or write failure removes the connection,
  closes it, and calls `on_error`. Undecodable inbound frames go to
  `on_error` and the connection continues. The hard-coded Unfolded Circle
  `authentication_success_response` is gone; `WsConnectionContext` gains
  `credential_verified()` (the upgrade presented a configured credential that
  matched). Host auth helpers became `check_upgrade_credential(request) ->
  Result<bool, ErrorResponse>`, called from the routing callback.
- **Validation (`ws_codegen/plan.rs`).** Per direction the role receives
  (client: `Server`+`Bidirectional`; host: `Client`+`Bidirectional`):
  >1 fallback, fallback with selectors, a selector-less message beside
  others, a repeated selector field, identical selectors, and incomparable
  overlap are Errors (strict-subset overlap allowed). Message names must be
  Rust identifiers and unique per endpoint. `raw_binary`/`mixed` frame
  formats are a named Error at `runtime/frame_format`.
- **Definitions.** Dock auth payloads now match the UCD2 AsyncAPI:
  `DockWsEnvelopeType` gains `auth_required`/`auth` (snake_case),
  `DockWsAuthRequired { type, model?, revision?, version? }`,
  `DockWsAuthMessage { type, token }`, `DockWsAuthenticationMessage` gains
  `code`. Integration: `IntegrationWsAuthMessage` gains `id` (AsyncAPI
  `authRequestMsg` is a `commonReq`), `IntegrationWsAuthenticationMessage`
  gains `req_id`/`code` (`authMsg` is a `commonResp`), and
  `IntegrationWsEventEnvelope.cat`/`ts` are omitted from the wire when absent
  (drivers built events without them; typed serialization would otherwise
  add `null`s).
- **Consumers (R15 partial).** `homelab/lib/src/samsung_tv.rs` sends typed
  `RemoteControlClientMessage`/`ArtModeClientMessage` (no `send_value` needed).
  The three integration handlers keep their JSON routing in a private
  `route(Value) -> Option<Value>` and adapt at the typed boundary through
  `unfolded-integration-helper` helpers `client_message_value` and
  `server_message`; `on_authenticated` calls the shared
  `authentication_greeting`, which returns
  `{"kind":"resp","req_id":0,"msg":"authentication","code":200,"msg_data":{}}`
  exactly when `!credential_verified()` — the condition under which the old
  generated host sent it, and what the Integration AsyncAPI requires of a
  driver that does not require authentication. Mid-request pushes and
  subscriber broadcasts use the `_value` escape hatches (they are the helper
  crate's JSON builders). The helper crate enables serde_json's `raw_value`
  feature (`to_raw_value`); no crate added.
- Regenerated twice with `just generate` (after the generator change, and
  after the Integration payload fix); only the five `*_ws.rs` modules and
  `ws_shared.rs` changed.

### Adopted defaults and deviations

- adopted default: enum derives are `Debug` only; `Serialize` and `decode`
  are emitted per direction by what the role sends/receives (source: spec
  R10 "Derives must respect payload capabilities"). Payloads must implement
  `Debug`; every shipped payload does.
- adopted default: inbound JSON is decoded from either frame kind; the frame
  kind is kept in `WsRawFrame` so a binary decode error retains bytes, and
  outbound frames follow `frame_format` (source: R10 "JSON text and JSON
  binary framing remain distinct … Binary decode errors retain bytes").
  Rejecting a text frame on a `json_binary` endpoint was rejected as a
  behavior change for real devices.
- adopted default: a direction's sole message without selectors takes every
  frame (source: R10 Error list applies only "in a direction with more than
  one inbound message").
- adopted default: a fallback that declares selectors is an Error, and a
  repeated selector field on one message is an Error (source: plan rule 4
  "A fallback cannot also declare selectors"; the repeat rule keeps the
  subset comparison well-defined).
- adopted default: selection rules are checked only for directions the role
  receives (source: R10 "an inbound frame is matched"; Samsung's
  outbound-only `AppLaunchCommand`/`InstalledAppsCommand` share a wire value
  and stay valid).
- adopted default: single host endpoint → `handle_message`/`on_authenticated`;
  several → endpoint-specific names (source: spec item 14 and R14
  "Multi-endpoint hosts use endpoint-specific hook methods").
- adopted default: "authentication" for the hook = an accepted upgrade,
  including header-credential validation where the host configures one
  (source: R14 "failed authentication never invokes it"; the generated host
  performs no message-based auth). Hook-write failure is "reported" through
  the new `on_error` handler method.
- adopted default: typed hub broadcasts return `Result<usize, WsError>` (an
  encode failure is reported, not silently 0); value broadcasts keep
  `usize`.
- adopted default: the Integration greeting is sent as the `ResponseEnvelope`
  variant, byte-identical to the old envelope (the `Authentication` payload
  now also carries `req_id`/`code`; either variant is valid).
- deviation (scope): the Dock/Integration payload-type corrections above are
  outside the plan's file list but required by R10 ("decoding every inbound
  message type … from a recorded fixture frame") and by the consumer
  migration keeping its wire bytes.
- adopted default (found by the gate): AsyncAPI import now derives each
  message's selectors from its payload's top-level string `const`
  properties, through `$ref` and `allOf` (a non-`const` redeclaration keeps
  an earlier `const`; `$ref` chains are bounded at depth 16). Without it the
  new selection Errors rejected every imported multi-message definition and
  `asyncapi_import::imported_definitions_lower_with_their_declared_role`
  failed (source: plan rule 11 "Keep AsyncAPI import working by adapting its
  typed construction"). The official Integration AsyncAPI imports and lowers
  for the client, server, and both roles. Tests:
  `asyncapi_import::selectors_are_read_from_the_payloads_string_consts`,
  `asyncapi_import::tests::payload_selectors_keep_string_consts_across_parts_and_end_on_cycles`;
  the Phase 3 assertion "imports declare no selectors" now asserts every
  imported message selects on `kind`.
- Subagent process note: the homelab subagent ran `rustfmt --edition 2024`
  on the five `.rs` files it edited (they were rustfmt-clean at HEAD, so
  only its own lines were reflowed). No `cargo fmt` was run.

### Findings for review (not fixed; new scope or later phases)

1. **Dock request/response envelopes do not match UCD2.** The official
   `commonReq`/`commonResp` use `type: dock|auth` and requests carry
   `command`, not `msg`; `req_id`/`code` are optional. The definition models
   `type=req|resp` with required `req_id`/`code`, so the documented
   `systemInfoMsg` example (`{"type":"dock","req_id":2,"msg":"get_sysinfo",
   …}`) is selected as the `ResponseEnvelope` fallback and fails to decode
   (`unknown variant dock`). The Dock runtime test sends that frame with
   `"type":"resp"` and a comment. Remodeling the Dock protocol is new scope;
   Dock has no consumer.
2. **Generated clients never send the env-mapped credential (Phase 5).**
   `connect_*` calls `self.headers.clone().build()`, and `Headers::build()`
   never reads `env_mapping` (only `from_env()` does), so e.g. ElevenLabs'
   `xi-api-key` from `ELEVEN_LABS_API_KEY` is never attached. Present since
   the first WS generator commit; it is R5/R6 credential resolution, Phase 5.
   The fresh-output Lobby routing test calls `.from_env()` explicitly.
3. Samsung unauthorized/error/installed-apps frames are not recorded in the
   repo; the runtime tests use the definition's documented shapes.

### Requirement-to-test map

| Requirement | Boundary / rung | Test(s) |
|---|---|---|
| R10 selector Errors (each) | gen plan API | `plan::tests::{more_than_one_fallback_in_a_direction_is_an_error, identical_selectors_are_an_error, incomparable_overlapping_selectors_are_an_error, a_message_without_selectors_beside_others_is_an_error (None and Some([])), a_fallback_with_selectors_and_a_repeated_selector_field_are_errors, a_bidirectional_message_is_checked_in_both_directions, selection_rules_apply_to_the_directions_the_role_receives}`; controls `a_strict_subset_overlap_and_one_fallback_lower`, `contradictory_selectors_never_conflict`, `a_sole_inbound_message_needs_no_selector` |
| R10 named framing diagnostic; variant names | gen plan API | `plan::tests::raw_and_mixed_framing_are_named_errors` (+ json_text/json_binary controls), `message_names_are_unique_identifiers` |
| R10 generated variant names/directions | gen output (syn over shipped modules) | `ws_codegen::{core_message_enums_name_one_variant_per_declared_message, a_bidirectional_integration_message_is_a_variant_of_both_directions, elevenlabs_client_enums_hold_the_declared_client_messages, every_shipped_client_sends_its_typed_enum_and_values_only_through_send_value, core_decode_tries_the_most_specific_selectors_before_generic_ones_and_the_fallback}` |
| R10 every inbound type of all five, variant asserted | generated client ↔ loopback mock, F3 (local; CI does not run schema tests) | `schematic-schema` `ws_runtime::{core_ws_decodes_every_server_message_variant, dock_root_decodes_every_server_message_variant, integration_decodes_every_server_message_variant, samsung_remote_control_decodes_every_server_message_variant, samsung_art_mode_decodes_every_server_message_variant, elevenlabs_text_to_speech_decodes_every_server_message_variant, elevenlabs_multi_context_decodes_every_server_message_variant}` |
| R10 overlap (Core pong vs response), fallback | as above | `ws_runtime::core_ws_selects_the_most_specific_message_before_the_fallback`; CI-visible: fresh-output child `every_server_message_is_selected_from_its_frame` |
| R10 unmatched → error, connection usable | compiled fixture ↔ loopback, F3, CI-visible | child `an_unmatched_frame_without_a_fallback_is_a_decode_error_and_the_connection_stays_usable`, `an_endpoint_without_server_messages_reports_every_frame_as_undecodable` |
| R10 malformed-then-valid, text and bytes retained | both | `ws_runtime::{core_ws_reports_a_non_json_text_frame_and_keeps_decoding, core_ws_reports_a_non_json_binary_frame_with_its_bytes, core_ws_reports_a_selected_payload_that_does_not_decode}`; child `malformed_text_and_binary_frames_are_decode_errors_that_keep_the_frame` |
| R10 payload-only send, `send_value` | both | `ws_runtime::core_ws_typed_send_writes_the_payload_alone`; child `send_writes_the_payload_alone_and_send_value_writes_the_value_verbatim` |
| R10 one shared queue; `events()` | both | `ws_runtime::{core_ws_typed_and_value_receivers_share_one_queue, core_ws_events_stream_keeps_the_connection_open}`; child `typed_and_value_receives_share_one_queue_and_the_event_stream_keeps_the_connection_open` |
| R10 JSON binary distinct | compiled fixture | child `a_json_binary_api_sends_and_receives_binary_frames` |
| R10 wrong send does not compile | compile-fail (E0308 per file) | `WS_MISUSES` in `slow_typed_ws_clients_and_a_multi_endpoint_host_compile_and_run`: server enum to `send`, bare payload to `send`, another endpoint's ClientMessage |
| R10 empty / non-Clone payload compile | compiled fixture | same slow test: `ws_payloads` without `Clone`; `ListenClientMessage`/`ShoutServerMessage` uninhabited, matched in `ws_send_control` |
| R14 no UC assumptions; hub name; hooks per endpoint | rendered module text | parent assertions in the slow test (`ChatFixtureWs`), `ws_codegen::{the_integration_host_is_named_for_its_api_and_sends_no_protocol_envelope, the_integration_handler_takes_and_returns_typed_messages}`; `shared::tests::host_common_auth_helpers_produces_tokens` |
| R14 hook once, first, before replies; None sends nothing; rejected auth never runs it; write failure closes + reported | compiled host ↔ client, F3, CI-visible | children `the_room_hook_message_is_the_first_frame_and_runs_once_per_connection`, `a_hook_returning_none_sends_nothing_before_the_reply`, `a_rejected_key_never_runs_the_hook`, `a_hook_message_that_fails_to_write_closes_the_connection_and_reaches_on_error` |
| R14 typed host input/output, routing, per-endpoint fan-out | same | children `room_messages_arrive_typed_and_an_undecodable_frame_reaches_on_error_without_closing`, `connections_route_by_path_and_unknown_paths_are_rejected_with_404`, `typed_broadcasts_reach_their_endpoint_only_and_value_broadcasts_reach_every_connection`; guard `ws_child_test_lists_name_every_child_test` |
| Bearer host still 401/401/101 | compiled host | `slow_bearer_ws_host_compiles_and_checks_tokens` |
| Dock/Integration documented frames | unit | `dock_ws::types::tests::documented_authentication_frames_decode`, `integration_ws::types::tests::{documented_authentication_frames_keep_their_ids_and_code, absent_event_category_and_timestamp_stay_off_the_wire}` |
| Consumer wire bytes kept | crate tests | `unfolded-integration-helper` `envelope` tests (greeting bytes, `server_message` round trip byte-equal), arcam `an_auth_request_keeps_its_id_and_is_answered_as_unknown`, each crate's `on_authenticated_sends_authentication_without_a_verified_credential`; Samsung homelab tests 12/12 |

Input robustness matrix: not applicable this phase — no definition or
configuration reader changed (selectors, fallbacks and roles are read by the
Phase 3 loader and its matrix). The Dock/Integration changes are wire
payload types, which the matrix excludes.

### Gates and verification (Phase 4 close)

- Gate-skip check not applicable: the phase changed source in
  `schematic-gen`, `schematic-definitions`, `schematic-schema`, and five
  homelab packages, so every gate was run.
- `just lint` (schematic area): pass — after splitting the importer's two
  component maps into a `Components` struct (`clippy::too_many_arguments`).
  `just lint-schema`: pass.
- `just test`: first run failed one test,
  `asyncapi_import::imported_definitions_lower_with_their_declared_role`
  (imported definitions had no selectors; fixed by deriving them, see
  adopted defaults). Second run: pass — L1 2746 passed, 18 skipped;
  `test-fresh-output` 12/12 (123 s, includes the new
  `slow_typed_ws_clients_and_a_multi_endpoint_host_compile_and_run`);
  `test-compile-fail` 1/1. The importer refactor after that run changed no
  behavior; its 7 tests and the lint were rerun and pass.
- `just test-schema`: 91/91 (`ws_runtime` 20, `ws_mock_harness` 10).
- `just check-drift`: 9/9. `just check-tier-coverage schematic`: 0 stranded.
- `just doctest`: pass (define 173 + 2 ignored — the two `rust,ignore`
  generated-surface examples in `docs/websocket-definitions.md`, which need
  `schematic-schema`; definitions 18; gen 44; oauth 19; runtime 3).
- Dependents, serially: `unfolded-integration-helper` 32/32,
  `arcam-amp-integration` 23 passed (2 tier-skipped), `eversolo-integration`
  33 (3 skipped), `sony-receiver-integration` 28 (3 skipped), `homelab`
  Samsung tests 12/12; clippy `-D warnings --all-targets` over the five
  homelab packages clean; `cargo check -p sniff -p biscuit-speaks -p
  model-citizen` pass (they use no WS module).
- Removed-name grep (`UnfoldedCircleEventHub`, `send_typed`,
  `authentication_success_response`, `WsEncode`, `WsDecode`) over `.rs`/`.md`
  outside specs and reviews: only the new absence assertions remain.
- Cross-OS: not run. The phase adds no path, process, or OS-specific code;
  new tests use loopback TCP on `127.0.0.1:0` like the Phase 2 harness that
  passed on native Windows. The host's path routing compares URL paths, not
  filesystem paths.
- Docs: `schematic/README.md` (typed quick start),
  `docs/websocket-definitions.md` (selection rules + flowchart, the
  generated messages/client/host section, AsyncAPI selector derivation; the
  **Planned** selector marker removed), `docs/generation/validation.md`
  (new Errors), `docs/generation/generated-clients.md` (fresh-output list),
  homelab `eversolo-integration/README.md` and
  `unfolded-integration-helper/README.md`. Skill:
  `.claude/skills/schematic-define/schematic/websocket.md`.
- No commits, no CI runs, no device or provider contact, no stale-file
  deletion.

Phase 4 checkpoint met: R10 and R14 pass through public boundaries
(selector-chosen variants, not merely deserializable ones; protocol-neutral
host names and hooks); consumer source compiles against the typed surfaces.
R11 typed correlation and credential resolution remain for Phase 5.

## Phase 5

Started 2026-10-22. Phase 4 was still uncommitted when this phase began, so
its tree was snapshotted (`git stash create`, no stash-stack entry) and the
`*_during_phase_5` lists are the diff against that snapshot. The generator,
runtime, and planner work was done by the orchestrator; two subagents then
wrote tests in parallel on disjoint files (`schematic-schema` mock-server
tests; `schematic-gen` fresh-output fixtures), each running only scoped
tests. The area gates ran once afterwards.

### What changed

- **Runtime (pre-authorized minimal function, `runtime/src/credentials.rs`).**
  `AuthConfig::select_handshake(per_call, client) -> HandshakeCredential`
  selects one credential with the REST precedence (per-call, client, first
  non-empty env variable, none) for a handshake another transport sends;
  `HandshakeCredential::{auth_method, apply(&mut Url, &mut HeaderMap)}`
  places it with the REST placement code (`place_credential` now takes the
  URL and headers instead of the private `Assembled`; behavior unchanged),
  refusing a request that already sets the credential's header, query
  value, or cookie. `AuthConfig::secret_names()` is the redaction set.
  `Debug` of the selection never shows the value. No new crate, no new
  error kind.
- **Handshake auth (R5/R6/R7, `ws_codegen/{client,shared}.rs`).** The WS
  API struct drops the public `headers: schematic_define::Headers` field.
  It gains `auth_config()` (the REST translation: `api_struct/auth.rs`
  `policy_config`/`policy_tokens`, now `pub(crate)` and taking `env_auth`
  instead of a `RestApi`), `credentials(Credential)`, `api_key(key)` (only
  when a non-Basic placement is accepted), and `header(name, value)`; the
  definition's constant `headers` are now emitted (they were ignored before).
  `WsClientOptions.credential` is the per-connect source. `connect_*`
  selects once and builds a `WsHandshake` (URL + headers with the credential
  placed, kept privately; redacted `Debug`), which the supervisor reuses for
  every redial, so an env change needs a new explicit connect. `dial` moved
  from each endpoint client into `WsHandshake`. Endpoint clients expose
  `auth_method()`. Handshake failures are no longer `WsError::Transport`:
  any non-101 answer is `HandshakeRejected { status, headers }` (headers
  redacted through `SecretNames::redact_headers`, so `Set-Cookie` and a
  `Location` URL's secrets are hidden), other failures are
  `Handshake(String)` with the URL replaced by its redacted form; new
  `InvalidCredential`, `InvalidUrl`. A 401, 403 or 3xx on a redial closes
  the supervisor. Caller headers may not name an upgrade-managed header or
  the API's credential header. The unresolved-placeholder check now runs
  before the query is appended, so its message cannot carry a query value.
  This fixes the Phase 4 finding (generated clients never read their env
  mapping).
- **Correlation (R11, `ws_codegen/plan.rs`, `client.rs`, `shared.rs`).**
  `infer_correlation` (envelope-name and `id`/`req_id` inference) is gone;
  `lower_correlation` uses only endpoint `CorrelationHints` and
  `responds_with` pairs, carried as `CorrelationStrategy::Correlated {
  .., id_type, pairs: Vec<CorrelatedPair> }`. New validation (Errors):
  link on a server message, naming no message, naming a client message,
  link without hints, a linked request named `Value`; (Warnings): hints with
  no link, hints on a `Host`-only endpoint, a `string` id type no client
  endpoint consumes. The generated client has one `request_{snake(name)}`
  per pair (request payload → response payload) and `request_value`. Ids
  come from the existing per-connect counter (never reset by redials), JSON
  number for `U64` and string for `String`. The pending table is a
  `std::sync::Mutex<HashMap<String, PendingRequest>>`; a `PendingGuard`
  removes the entry on every exit (send failure, timeout, cancellation);
  `answer_pending` completes a request only when the id matches with the
  declared JSON type and the frame's `selected_message` is the declared
  response (`None` = any frame, for `request_value`). On connection loss the
  supervisor drops every pending entry (`Disconnected`) and discards queued
  writer commands, so nothing is replayed on the replacement. Each message
  enum's `decode` is now driven by a public `selected_message(&frame)`.
- **Regenerated** with `just generate`: only the five `*_ws.rs` and
  `ws_shared.rs` changed (no OpenAPI/Postman change). No WS diagnostic is
  emitted for the shipped definitions. Consumers needed no edit: homelab
  never touched `headers` or `request`; `schema/tests/ws_runtime.rs` moved
  to `api_key(..)` and `request_value(..)`.

### Adopted defaults and deviations

- adopted default: typed request method name is `request_{snake(request
  message)}`, uniformly, e.g. Core's `request_request_envelope` (source:
  spec Design "Emit a typed request operation for each declared
  request/response pair"; plan rule 3 names `request_value`). A request named
  `Value` is an Error so the names cannot collide.
- adopted default: `request_value` completes on the first frame with its id,
  whatever message it is (source: spec "retain `request_value` as the
  explicit value escape hatch" — it has no declared response to check).
- adopted default: the per-connect credential lives on `WsClientOptions`
  (`credential`, redacted `Debug`); the client-level one is
  `credentials`/`api_key` on the API struct, mirroring REST (source: R5
  "per-connect-call value, client-level value"; REST `CallOptions`).
- adopted default: "competing explicit sources" are a caller header named
  like a credential placement header (`InvalidHeader`) and a request that
  already sets the credential's query value or cookie (`InvalidCredential`);
  a cookie key merges with other caller cookies (source: spec Design
  "Headers and redaction"; R5 cookie merge). Unlike REST's
  `AuthBearingDefaultHeader`, built-in secret names such as `Cookie` are not
  refused outright, since R5 requires merging with existing cookies.
- adopted default: upgrade-managed headers = `Host`, `Connection`,
  `Upgrade`, `Sec-WebSocket-Key`, `Sec-WebSocket-Version`,
  `Sec-WebSocket-Accept`, `Sec-WebSocket-Extensions`; `Sec-WebSocket-Protocol`
  stays settable (subprotocol negotiation is the caller's).
- adopted default: a programmatic `header` replaces a constant header of the
  same name (source: spec "constant headers and programmatic overrides using
  shared precedence").
- adopted default: terminal redial statuses are 401, 403 and every 3xx;
  other non-101 statuses (e.g. 500/503) keep using the reconnect budget
  (source: R5 "A 401, 403, or refused redirect during a redial closes the
  supervisor … Transient dial failures use the reconnect budget").
- adopted default: the "explicitly declared `RequestIdType`" Warning fires
  for a non-default `string`; an explicit `u64` is indistinguishable from the
  default (the field is not optional) and never warns (source: R11 "An
  implicit default ID type on an uncorrelated endpoint produces no Warning").
- adopted default: commands queued when a connection is lost are dropped
  (not only request commands), implementing "never replay commands" for
  correlation now; returning `Disconnected` from a send while disconnected
  stays Phase 6 (source: spec Design "Reconnect and shutdown").
- open issue (plan rule 11, item 16): **no WS TLS opt-in.** The runtime's
  `danger_accept_invalid_certificates` configures reqwest only; the runtime
  builds no rustls `ClientConfig` to pass to tokio-tungstenite, and writing a
  custom certificate verifier is a new capability. No WS-only switch was
  added; a Samsung TV's self-signed `wss://` certificate is expected to fail
  the handshake (unverified, no device contacted). Documented in
  `docs/websocket-definitions.md`.
- observation (not changed): `schematic_define::Headers` now has no
  generated consumer (REST never used it); removing it is new scope.
- observation (not changed): `resolve_auth` in `plan.rs` still detects auth
  messages by the names `Auth`/`AuthRequired` for a Warning/Info only; it
  does not affect correlation and R11 does not name it.
- Subagent note: the schema test subagent saw nextest report `LEAK` once on
  each of two different tests in 2 of 4 runs; both passed alone and in the
  gate runs below (no LEAK in the gate). Recorded as load/timing noise.

### Requirement-to-test map

| Requirement | Boundary / rung | Test(s) |
|---|---|---|
| R5 runtime selection: precedence, first non-empty env, None ignores env, one placement each (header/query/Basic), cookie merge, second source refused, wrong kind refused, Debug/secret names | `schematic-runtime` public API, F1 | `runtime` L1 `handshake_credentials::{per_call_wins_over_client_and_environment_and_client_over_environment, the_first_non_empty_variable_wins_and_none_set_is_anonymous, an_api_accepting_no_credential_ignores_the_environment, each_placement_writes_exactly_one_credential, a_cookie_key_is_merged_with_existing_cookies, a_second_explicit_source_in_the_header_or_query_is_refused, a_credential_kind_the_api_does_not_accept_is_refused_before_reading_the_environment, the_selection_debug_and_secret_names_never_show_the_value}` |
| R5 precedence with sentinels at every level, one credential seen by the server, `auth_method` | generated Core client ↔ loopback mock, F3 (local) | `schematic-schema` `ws_handshake::core_credential_precedence_per_call_then_client_then_env` |
| R5 placement per strategy (header, query, cookie merge without duplicate, Basic env pair and client pair, Bearer), precedence | compiled fixtures ↔ loopback, F3, **CI-visible** | fresh-output `slow_ws_handshake_credentials_and_correlated_requests_run` children: `a_per_call_credential_wins_over_the_client_and_the_environment`, `a_client_credential_wins_over_the_environment`, `the_first_non_empty_env_variable_supplies_the_header_key`, `an_env_query_key_is_one_query_pair_and_nothing_else`, `a_cookie_key_merges_into_an_existing_cookie_header_and_never_duplicates_its_name`, `basic_credentials_from_the_env_lists_are_one_authorization_header`, `a_client_basic_credential_wins_over_the_env_lists`, `a_bearer_key_is_sent_as_an_access_token`; guard `ws_child_test_lists_name_every_child_test` |
| R5 None policy attaches no env credential | F3 local | `ws_handshake::dock_without_auth_policy_ignores_its_env_token`; runtime `an_api_accepting_no_credential_ignores_the_environment` |
| R5 retained selection on redials (env changed after connect) | F3 local | `ws_handshake::redial_reuses_the_credential_selected_at_connect` |
| R5/R7 initial 401/403 not retried, exactly one upgrade | F3 local + CI-visible | `ws_handshake::initial_401_and_403_are_returned_without_a_second_attempt`; child `a_401_upgrade_is_rejected_once_and_never_retried` |
| R5 redial 401/403/redirect terminal, one rejected upgrade | F3 local | `ws_handshake::{redial_401_is_terminal, redial_403_is_terminal, redial_redirect_is_terminal}` |
| R5/R7 explicit-source conflicts and managed headers refused before any request | F3 local | `ws_handshake::invalid_headers_and_credential_kinds_fail_before_dialing` |
| R6 anonymous connect, `Anonymous` reported | F3 local + CI-visible | `ws_handshake::core_without_any_credential_connects_anonymously`; child `nothing_resolved_connects_anonymously_and_reports_it` |
| R7 redirect not followed, second origin zero requests, redacted Location | F3 local + CI-visible | `ws_handshake::redirect_is_not_followed_and_its_location_is_redacted`; child `a_302_upgrade_is_not_followed_and_its_location_is_redacted` |
| R7 Set-Cookie / secret headers in a rejection | F3 local | `ws_handshake::refusal_headers_redact_set_cookie_and_secret_values` (mock gained `Script::RefuseWith`; harness test `ws_mock_harness::refuses_with_scripted_response_headers_in_order`) |
| R7 query sentinel: refused port, stalled handshake, 302; options `Debug` | F3 local | `ws_handshake::query_secret_is_absent_from_every_dial_error` |
| R11 each Error and Warning; implicit default no Warning; control | gen plan API, F1 | `plan::tests::{lower_pairs_requests_through_responds_with_and_the_endpoint_hints, the_api_request_id_type_reaches_each_correlated_endpoint, a_responds_with_link_without_endpoint_hints_is_an_error, a_link_on_a_server_message_or_to_an_unknown_or_client_message_is_an_error, a_bidirectional_request_may_name_a_bidirectional_response, a_request_named_value_is_an_error, hints_that_nothing_consumes_warn, a_declared_string_id_type_that_nothing_consumes_warns_and_the_implicit_default_does_not, no_endpoint_correlates_without_hints_whatever_its_messages_are_named}` |
| R11 no envelope literal in the planner; rename invariance; names alone don't correlate; typed signatures | gen output | `ws_codegen::{the_planner_names_no_envelope_message, renaming_the_envelope_messages_changes_no_correlation, envelope_names_alone_correlate_nothing, core_requests_are_typed_per_declared_pair}` |
| R11 id JSON type (U64 number / String string), typed response returned | F3 local + CI-visible | `ws_correlation::request_ping_sends_a_numeric_id_and_returns_the_pong`; child `string_request_ids_go_out_as_json_strings_and_return_the_declared_response` |
| R11 same-id other message stays an event | both | `ws_correlation::{same_id_with_another_message_is_an_event_not_the_answer, same_id_with_another_message_alone_times_the_request_out}`; child `a_same_id_frame_of_another_message_is_an_event_not_the_answer` |
| R11 capacity reclaimed (timeout, cancel, serialize failure) under limit 1, with `BackpressureFull` control | both | `ws_correlation::pending_capacity_is_reclaimed_after_timeout_cancel_and_serialize_failure`; children `a_timed_out_request_frees_its_pending_slot`, `a_cancelled_request_frees_its_pending_slot` |
| R11 disconnect → `Disconnected`, not re-sent, ids not reset | F3 local | `ws_correlation::disconnect_fails_in_flight_request_without_resending_it` (existing `ws_runtime::ws_disconnect_fails_pending_requests` also passes) |
| R11 concurrent requests, `request_value` | F3 local | `ws_correlation::{concurrent_requests_receive_their_own_answers_out_of_order, request_value_returns_the_first_frame_with_its_id}` |

Send failure: the guard makes it the same exit path as cancellation; a
supervisor-ended send (`Disconnected`) is exercised by
`ws_runtime::ws_disconnect_fails_pending_requests`. An injected failing
writer for the request path was not added (the `FailingStream` harness is
for the opening write in Phase 6).

Input robustness matrix: not applicable — no file format or configuration
reader changed. The one new input read is an environment variable in the
runtime selection; its shapes (unset → next variable, empty → skipped,
set → selected, none set → anonymous) are covered by
`the_first_non_empty_variable_wins_and_none_set_is_anonymous` and the
precedence tests.

### Gates and verification (Phase 5 close)

- Gate-skip check not applicable: the phase changed source in
  `schematic-runtime`, `schematic-gen`, and `schematic-schema`, so every gate
  was run.
- `just lint` (schematic area): pass. `just lint-schema`: pass.
- `just test`: pass — L1 2766 passed, 19 skipped; `test-fresh-output` 13/13
  (71 s, includes the new `slow_ws_handshake_credentials_and_correlated_requests_run`);
  `test-compile-fail` 1/1.
- `just test-schema`: 111/111 (`ws_handshake` 12, `ws_correlation` 7,
  `ws_runtime` 20, `ws_mock_harness` 11, others unchanged).
- `just check-drift`: 9/9. `just check-tier-coverage schematic` (root): 0
  stranded. `just doctest`: pass (define 173 + 4 ignored — the
  `rust,ignore` generated-surface examples of `docs/websocket-definitions.md`,
  two of them new; definitions 18; gen 44 + 2 ignored; oauth 19; runtime 3+3).
- Dependents, serially: `unfolded-integration-helper` 32/32,
  `arcam-amp-integration` 23 (2 skipped), `eversolo-integration` 33 (3
  skipped), `sony-receiver-integration` 28 (3 skipped), `homelab` Samsung
  tests 12/12; clippy `-D warnings --all-targets` over the five homelab
  packages clean; `cargo check -p sniff -p biscuit-speaks -p model-citizen`
  pass.
- Cross-OS: not run. No path, process, or OS-specific code; URL handling is
  the `url` crate the REST runtime already uses on every OS, and the tests
  use loopback `127.0.0.1:0` like the harness that passed on native Windows
  in Phase 2.
- Docs: `docs/websocket-definitions.md` (handshake credential section with a
  precedence flowchart, correlation section with a sequence diagram; the
  **Planned** correlation marker removed), `docs/generation/validation.md`,
  `docs/generation/generated-clients.md`, `docs/headers/README.md` (WS
  section rewritten; it described the removed `Headers` field),
  `docs/runtime/credentials.md` (handshake selection), `docs/dependencies.md`,
  `README.md` quick start. Skills: `schematic-define` `SKILL.md`,
  `schematic/websocket.md`, `schematic/headers.md`.
- No commits, no CI runs, no device or provider contact, no stale-file
  deletion.

Phase 5 checkpoint met: R5, R6, R7, and R11 pass through mock-observed
handshakes and typed request results; no transport formatting leaks a
credential URL; one selected credential is retained for recovery. The old
receive-timeout Info diagnostic is retained for Phase 6.

## Phase 6

Started 2026-10-23. Phase 5 was committed (`2b81e4a72`), so the working tree
is exactly this phase's changes. The orchestrator implemented Wave 13
(runtime accessor, definition field, generator, regeneration, docs). Three
subagents then wrote tests on disjoint files at the same time, each running
only its own target: `schema/tests/ws_session.rs` (R2/R3), `schema/tests/
ws_idle.rs` plus mock additions (R8, backpressure), and the CI-visible
fresh-output test. The area gates ran once afterwards with no subagent
running.

### What changed

- **One shared supervisor (`gen/src/ws_codegen/supervisor.rs`, new; emitted
  into `ws_shared.rs`).** The per-endpoint inline supervisor loop in
  `client.rs` is gone; every endpoint client calls
  `ws_shared::connect_session(WsSession, options)`. `WsSession` keeps the
  `WsHandshake` (now holding the `HandshakeCredential`), the serialized
  opening `Message`, the setup factory, the correlation config, and the
  frame kind, and is reused for every redial.
  - **Attempt** = dial → opening write → setup callback → reclaim, all inside
    one `tokio::time::timeout(handshake_timeout)`; expiry is
    `WsError::HandshakeTimeout(Duration)` (was `u64` seconds and covered the
    dial only). The explicit connect makes one attempt inline (dropping the
    connect future leaves nothing running); the supervisor is spawned after
    the first `Ready`. State is `Authenticating` while the opening/setup
    runs; `Ready` and the budget reset only after setup.
  - **R2:** `recover` counts consecutive failed redials from zero for each
    loss, so a redial that completes setup resets the count; `Some(0)` never
    redials, `None` unlimited. A 401/403/3xx or `SetupRejected` is terminal
    (`WsError::is_terminal`).
  - **R3:** `connect_*` takes `opening: {OpenPayload}` for an endpoint with
    `lifecycle.open`; `opening_message` serializes it once. The open
    message's `send_*` helper was removed (it could only send a duplicate).
    Endpoints with `AuthFlowHints` also get
    `connect_{ep}_with_setup(.., setup: Fn({Ep}Setup) -> Fut)`, with
    `pub type {Ep}Setup = WsSetupSession<{Ep}ClientMessage,
    {Ep}ServerMessage>`. The session implements the Phase 1 spike's slot +
    revocation design (`SetupGuard`, `WsSetupChannel`), with typed
    `send`/`next_message`, `send_value`/`next_value`, and `credential()`.
    `WsSetupError::{Rejected, Transient, Closed, Io, Codec, SessionInvalid}`;
    only `Rejected` is terminal (`WsError::SetupRejected`), the rest are
    `WsError::Setup`. Opening write failure is `WsError::OpeningWrite`.
  - **R8:** `serve` races, in a biased `select!`, shutdown, the idle
    deadline (reset by every inbound frame, ping/pong included, never by
    writes; starts after setup), delivery of a held event, a cancel-safe
    `drive_write` (`poll_ready`/`start_send`/`poll_flush`), the writer queue,
    and reads. A ping sets `flush`, so the queued pong is written at once.
    Expiry delivers `WsError::ReceiveTimeout(d)` and applies the reconnect
    policy.
  - **Backpressure:** events are `try_send`; a full queue holds one event
    and pauses reading, while close, timer, writes, state and pending
    wakeups carry on. Error events go through the same path; the terminal
    one is awaited after `Closed` is published, so it precedes the end of
    the stream.
  - **Shutdown:** `close()` sets a `watch` and waits for `Closed`
    (interrupting backoff, dial or setup; a ready connection gets a Close
    frame within a 500 ms grace); `WriterCommand::Close` was removed. Drop
    still aborts the supervisor. On every exit pending requests fail
    `Disconnected`.
  - **Send while disconnected:** `WsTransportHandle::send` returns
    `Disconnected` unless `Ready`. Fix found by the idle tests: a send that
    was blocked on a full outbound queue at the moment of loss was written to
    the replacement connection after `discard_queued` freed room (a replay).
    Queued commands are now tagged with the connection number; `send`
    re-checks number and state after `reserve()`, the supervisor bumps the
    number before publishing `Ready` and drops commands of an earlier
    connection.
- **Opening credential binding (plan rule 6).** New definition field
  `ConnectionLifecycle.open_credential_field: Option<String>`
  (`define/src/websocket/lifecycle.rs`; `default` + `non_null` +
  `skip_serializing_if`, so absent ≠ null). ElevenLabs `TextToSpeech`
  declares `xi-api-key`; `MultiContextTextToSpeech` binds nothing. Plan-time
  Errors: an empty field; a field without `lifecycle.open`.
  `opening_message` fills an absent/null/empty field from the selected key,
  keeps an equal value, refuses a different value with `InvalidCredential`
  before dialing (message names the field only); with no credential
  selected a caller value is sent as given.
- **Runtime (pre-authorized minimal transport-neutral extension, logged per
  plan rule 11).** `HandshakeCredential::expose_secret() -> Option<&str>`:
  the selected key/token without prefix or `Bearer ` scheme; `None` for
  anonymous or Basic. `HandshakeCredential` now stores the unplaced secret
  next to the `Resolved` (private `resolve_env_secret`,
  `Credential::single_secret`); REST resolution is unchanged.
- **Planner.** `EndpointPlan` gains `open_credential_field` and `has_setup`
  (`auth_flow` declared). The "receive_timeout is not yet enforced" Info and
  the `_receive_timeout` TODO in `client.rs` are removed (the behavior now
  exists).
- **Removed `WsError` variants:** `AuthRejected`, `AuthTimeout` (never
  constructed anywhere; replaced by `SetupRejected` and the attempt
  timeout). Added: `SetupRejected`, `Setup`, `OpeningWrite`,
  `ReceiveTimeout`.
- **Regenerated** with `just generate`: only the five `*_ws.rs` and
  `ws_shared.rs` changed. Consumers: `schema/tests/ws_runtime.rs` passes the
  new opening arguments; homelab (Samsung clients, integration hosts) needed
  no change — no homelab code uses an opening message, a setup callback, or
  a removed `WsError` variant.

### Adopted defaults and deviations

- adopted default: the opening credential binding is declared in the
  definition as `ConnectionLifecycle.open_credential_field` (source: plan
  rule 6 "bind each applicable protocol's field explicitly; never infer";
  the spec names no carrier). Alternatives: a trait on the payload type
  (every opening type would have to implement it), or generator special-casing
  ElevenLabs (rejected: not definition-driven). The field is additive and
  serialized, so the AsyncAPI exporter can carry it; review item.
- adopted default: an empty string in the bound field counts as absent and
  is filled (source: plan rule 6 "fill an absent declared auth field").
- adopted default: the initial connect makes exactly one attempt (no retry
  before the first `Ready`), so the "shared pre-Ready deadline" of plan rule
  7 is that attempt's `handshake_timeout`; no separate connect-deadline
  option was added, and nothing carries over to redials (source: R2 "the
  initial dial is separate from the reconnect budget"; Phase 1 spike).
  `handshake_timeout` keeps its name and now bounds the whole attempt.
- adopted default: `connect_{ep}_with_setup` is emitted only for endpoints
  declaring `AuthFlowHints`; the plain `connect_{ep}` stays on those
  endpoints too (Core also accepts its key in the handshake) (source: spec
  R3 "an endpoint with a challenge flow takes a typed setup callback").
- adopted default: `Authenticating` (existing variant) is the state while
  the opening and setup run (source: spec state diagram "Setup").
- adopted default: with the event queue full, the idle timer keeps running
  while reading is paused (time not reading counts as silence), and a
  transient error raised while an event is already held is dropped (state
  still shows it) (source: spec "a full inbound event queue must not prevent
  the supervisor from noticing timeout"; "preserve bounded queues").
  Documented in `docs/websocket-definitions.md`; review item.
- adopted default: manual close grace for the Close frame is a fixed 500 ms.
- adopted default: the setup session's credential accessor is the runtime's
  `HandshakeCredential` itself (redacted `Debug`, `expose_secret()`), not a
  WS-only wrapper (source: Phase 5 handoff).
- observation (not changed): Dock's policy is `AuthStrategy::None`, so its
  handshake selection is anonymous and its setup session exposes no secret;
  a Dock setup callback takes its token from its own closure. Reading
  `UCR_DOCK_TOKEN` for a setup-only credential would need a new
  runtime selection rule (new scope).
- observation: the CLI test `the_validate_command_prints_info_notes_only_when_verbose`
  depended on the retired receive-timeout Info; no shipped WebSocket
  definition emits an Info now, so it is renamed
  `the_validate_command_keeps_info_notes_out_of_its_default_output` and
  checks only the quiet run plus the absence of the retired note. The
  verbose-printing rule is still covered in-process by
  `ws_validation::info_notes_stay_out_of_the_shared_diagnostics` (now driven
  by the auth-messages-without-flow Info). Coverage loss: the spawned CLI no
  longer proves a verbose run prints an Info.
- gap: an opening-write failure on a **redial** is not tested; the only
  deterministic trigger (`max_write_buffer_size` below the frame size) also
  fails the initial connect, and the options cannot inject a failing
  transport. The redial runs the same `run_attempt`; a failed attempt using
  the budget is proven by the refused-redial and transient-setup tests.

### Requirement-to-test map

| Requirement | Boundary / rung | Test(s) |
|---|---|---|
| R2 five drop/redial cycles with budget 2 stay alive; two consecutive refused redials close; `Some(0)`; `None` unlimited; reset after setup; no policy | generated client ↔ loopback mock, F3 local | `ws_session::{repeated_losses_with_successful_redials_never_exhaust_the_budget, two_consecutive_refused_redials_exhaust_a_budget_of_two, zero_max_attempts_closes_on_loss_without_redialing, unlimited_max_attempts_survives_consecutive_failures, budget_resets_after_each_successful_redial, no_reconnect_policy_closes_on_loss}` |
| R2/R3 CI-visible | fresh-output child crate, F3, **CI-visible** | `slow_ws_session_setup_and_recovery_run` children: `the_opening_frame_comes_first_on_the_initial_connection_and_on_a_redial`, `an_opening_key_that_differs_from_the_credential_fails_the_connect_before_dialing`, `setup_runs_with_the_handshake_key_before_ready_on_the_initial_connection_and_on_a_redial`, `a_rejected_setup_fails_the_connect_with_setup_rejected_after_one_connection`, `the_reconnect_budget_counts_consecutive_failed_redials_and_resets_after_each_setup`, `a_silent_server_times_out_the_receive_and_the_client_redials`; guard `ws_child_test_lists_name_every_child_test` |
| R3 opening first on three connections, before Ready | F3 local | `ws_session::opening_message_is_written_first_on_every_connection` |
| R3 opening credential binding cells (absent, equal, different, anonymous + caller value, unbound endpoint) | F3 local | `ws_session::{opening_without_a_key_field_is_filled_from_the_credential, opening_with_the_same_key_keeps_one_field, opening_with_a_different_key_is_refused_before_dialing, anonymous_opening_keeps_the_callers_key, multi_context_opening_is_sent_unchanged}` |
| R3 deterministic opening-write failure | F3 local | `ws_session::opening_write_failure_fails_the_connect_without_redialing` |
| R3 setup before Ready on every dial; rejection terminal (initial, redial); transient uses budget; stall/cancel drops the stream and invalidates a leaked session | F3 local | `ws_session::{setup_runs_before_ready_on_every_connection, setup_rejection_on_connect_is_returned_without_a_second_attempt, setup_rejection_on_redial_closes_the_client, transient_setup_failures_on_redials_use_the_budget, setup_cancellation_drops_stream_and_invalidates_session}` |
| Shutdown: manual close/drop never redial; close interrupts backoff and stalled setup; pre-Ready attempt stall = one attempt; no inherited lifetime deadline | F3 local | `ws_session::{manual_close_never_redials, close_during_backoff_returns_promptly, close_during_stalled_redial_setup_returns_promptly, dropping_the_client_during_backoff_never_redials, stalled_initial_upgrade_times_out_after_one_attempt, ready_connection_and_later_redials_have_no_inherited_deadline}` |
| Disconnected sends fail at once; nothing replayed | F3 local | `ws_session::{send_while_disconnected_fails_immediately, messages_from_a_lost_connection_are_not_replayed}`; `ws_idle::{disconnect_fails_pending_and_discards_queued_commands, a_send_waiting_for_room_at_disconnect_is_not_written_to_the_replacement, a_request_waiting_for_room_at_disconnect_is_not_replayed}` (the last two failed before the connection-tag fix) |
| R8 silence → `ReceiveTimeout` + redial; `None` unchanged; pings keep alive with pongs observed at the server; outbound does not reset | F3 local | `ws_idle::{silence_after_a_frame_times_out_and_redials, no_receive_timeout_keeps_a_silent_connection, server_pings_keep_the_connection_alive_and_are_ponged, outbound_sends_do_not_reset_the_idle_timer}` |
| Backpressure: full event queue and stalled writes keep timeout, state, request wakeup and close responsive | F3 local | `ws_idle::{a_full_event_queue_still_times_out_publishes_state_and_closes, stalled_writes_do_not_block_close, stalled_writes_do_not_block_the_idle_timeout_and_pending_request_fails}` |
| Mock heartbeat step (pings, pongs recorded, reading on/off) | harness | `ws_mock_harness` (two new tests) |
| R8 Info diagnostic removed | gen plan / output | `plan::tests::no_info_note_reports_an_unenforced_receive_timeout`; `ws_validation::receive_timeout_is_enforced_so_no_shipped_api_has_a_note_about_it`; `ws_codegen::the_receive_timeout_is_enforced_in_the_shared_supervisor` |
| Generated surface: opening argument + binding, no open helper; setup connect only with a challenge flow | gen output | `ws_codegen::{elevenlabs_connect_takes_the_opening_payload_and_binds_its_credential_field, only_challenge_flow_endpoints_get_a_setup_connect}` |
| `open_credential_field` validation | gen plan API, F1 | `plan::tests::{an_open_credential_field_binds_a_declared_open_message, only_an_endpoint_with_a_challenge_flow_takes_a_setup_callback}` |
| `expose_secret` per source (per-call, client, env, prefix/scheme stripped, anonymous, Basic, Basic env) | `schematic-runtime` public API, F1 | `runtime` L1 `handshake_credentials::{the_exposed_secret_is_the_selected_key_or_token_without_its_placement, a_basic_pair_from_the_environment_exposes_no_secret}` |

### Input robustness matrix (`lifecycle.open_credential_field`)

One new load-bearing field on the existing definition loader (serde_json
over the derived `WebSocketApi`), so only field shapes are added; document
shapes (duplicate keys, trailing content) are already covered once for this
loader by `duplicate_keys_and_trailing_content_are_rejected`. Default: an
invalid shape fails to load. Test:
`schematic-definitions/tests/ws_definition_json.rs::the_open_credential_field_has_one_outcome_per_shape`
from a control (Dock document plus an open message and the binding):
control → read exactly and round-trips; absent → `None`, not written back;
null, number, bool, `[]`, `{}`, `["…"]` → rejected; `""` → loads as
declared, rejected by generation (`an_open_credential_field_binds_a_declared_open_message`).
Smell grep over the new reader: the field uses `default` with `non_null`
(null is not absent); no `filter_map`/`unwrap_or_default`/`.ok()` on its
path.

### Gates and verification (Phase 6 close)

- Gate-skip check not applicable: the phase changed source in
  `schematic-define`, `schematic-definitions`, `schematic-gen`,
  `schematic-runtime`, and `schematic-schema`; every gate was run, once,
  after all subagent work was merged and with no subagent running.
- `just lint` (schematic area): pass (rerun after the connection-tag fix).
  `just lint-schema`: pass.
- `just test`: pass — L1 2776 passed, 20 skipped; `test-fresh-output` 14/14
  (74 s, includes the new `slow_ws_session_setup_and_recovery_run`);
  `test-compile-fail` 1/1.
- `just test-schema`: 149/149 (`ws_session` 26 and `ws_idle` 10 new,
  `ws_mock_harness` +2; the rest unchanged). Subagent runs: `ws_session` 3×
  green (slowest 0.73 s), `ws_idle` 3× green after the fix (slowest
  ~1.4 s), fresh-output child crate 6/6. One subagent run reported "1
  leaky" on a passing `ws_idle` test; not seen in the gate run.
- `just check-drift`: 9/9. `just check-tier-coverage schematic` (root): 0
  stranded. `just doctest`: pass (define 173 + 6 ignored — the
  `rust,ignore` generated-surface examples of `docs/websocket-definitions.md`,
  two of them new; definitions 18; gen 44 + 2 ignored; oauth 19; runtime
  3+3).
- Dependents, serially: `unfolded-integration-helper` 32/32,
  `arcam-amp-integration` 23 (2 skipped), `eversolo-integration` 33 (3
  skipped), `sony-receiver-integration` 28 (3 skipped), `homelab` Samsung
  tests 12/12; clippy `-D warnings --all-targets` over the five homelab
  packages clean; `cargo check -p sniff -p biscuit-speaks -p model-citizen`
  pass.
- Cross-OS: attempted for the generated crate, whose loopback timing tests
  are the new OS-sensitive surface. `just cross-check schematic-schema --os
  windows` refuses ("the plan resolves no single windows-latest build for
  schematic-schema"), and with `--all-features` Windows cargo refuses
  ("cannot specify features for packages outside of workspace"):
  `schematic/schema` has its own manifest, so the cross-check tool cannot
  run it. No test failed; **Windows, Linux and WSL2 are not verified** for
  the new `ws_session`/`ws_idle` tests. Phase 7 ("Verify Environments",
  which names schema's separate manifest) owns this evidence; `os_failing`
  is not set because nothing failed. The generator change itself is
  OS-neutral tokio/tungstenite code with no path or process handling.
- Docs: `docs/websocket-definitions.md` (new "Connecting, setup, and
  recovery" section with a state diagram: opening message and credential
  binding, setup callbacks, timeouts table, reconnect budget, backpressure;
  lifecycle-helper sentence corrected), `docs/generation/validation.md`
  (new Errors; the Info example no longer names the retired note),
  `docs/generation/generated-clients.md` (fresh-output test list),
  `docs/runtime/credentials.md` (`expose_secret`), `README.md` (quick start
  passes the opening payload; link to the new section). Skill:
  `schematic-define/schematic/websocket.md` (opening binding, supervisor
  contract, test locations).
- Comment drift pass over changed symbols: `ws_codegen/mod.rs` listed
  `codec` and `routing` modules that do not exist (drift; the code is
  correct) — replaced by the real list including `supervisor`.
  `WsTransportHandle`, `WsClientOptions.{handshake_timeout,
  receive_timeout}`, `ReconnectPolicy.max_attempts`, `WsHandshake::dial`,
  `discard_queued`, `WsConnectionState::Authenticating`, and
  `ConnectionLifecycle.open` docs were rewritten for the new behavior.
- No commits, no CI runs, no device or provider contact, no stale-file
  deletion.

Phase 6 checkpoint met: R2, R3, and R8 pass; the state machine is exercised
through observable frames, states, events, callback runs, and request
completion. No accepted connection is reported `Ready` before its opening
message and setup complete.

## Phase 7

Started 2026-10-23. Phase 6 is not committed, so the working tree holds the
Phase 6 and Phase 7 changes together; this phase's own edits are listed in
its frontmatter. No subagents were used: the phase is verification, and the
remote rigs carried the cross-OS load while the local gates ran.

### Dependents (Wave 16, Verify Dependents)

Run serially on macOS, after the Phase 6 changes and before any Phase 7 fix:

| Package(s) | Command | Result |
|---|---|---|
| `homelab`, `homelab-cli`, `homelab-server`, `unfolded-integration-helper`, `arcam-amp-integration`, `eversolo-integration`, `sony-receiver-integration` (all seven, every R15 package plus cli/server builds) | `just test homelab` (root) | 252 passed, 8 skipped (tier-filtered). The root recipe runs the Rust L1 only; the homelab Vue frontend suite (no WebSocket code) was not part of it |
| `model-citizen` (153 tests listed, under the 200 limit) | `just test model-citizen` (root) | 170 passed |
| `sniff` (2141 tests) | `cargo nextest run -p sniff -E 'test(/remote\|github\|gitlab\|gitea\|bitbucket\|envelope/)'`; sniff uses only REST modules (`github`, `gitlab`, `gitea`, `bitbucket`) and `schematic_runtime` from `schematic-schema` | 76 passed |
| `biscuit-speaks` (441 tests) | `cargo nextest run -p biscuit-speaks -E 'test(/elevenlabs/)'`; it uses only the REST `schematic_schema::elevenlabs` module | 48 passed |

All four builds compile against the regenerated `schematic-schema` and the
changed `schematic-runtime` (`HandshakeCredential` now stores the unplaced
secret; REST resolution unchanged). No dependent needed an edit.

Removed-name check (R15): `git grep` for `UnfoldedCircleEventHub`,
`ConnectionParam`, `ParamType`, `AuthRejected`, `AuthTimeout` outside
`features/`, `reviews/`, `plans/` and the generated `schema/src` finds only
the negative assertion in `gen/tests/l1/ws_codegen.rs:629`; nothing in
`schema/src`. The `{Ep}ConnectionParams` structs are the generated parameter
types, not the removed `ConnectionParam`. Absence is enforced by tests, not
only by this grep: `ConnectionParam`/`ParamType` no longer exist (four
`compile_fail` doctests in `schematic-define`), the hub name is asserted by
`ws_codegen` (line 629), and
`ws_codegen::every_shipped_client_sends_its_typed_enum_and_values_only_through_send_value`
asserts that `send` takes only the typed enum and that `Value` goes only
through `send_value`. Historical research notes (`docs/research/samsung-design.md`)
still show `ParamType` as a design-time sketch; research notes are not
behavior docs and were left alone.

### Fixes made this phase (two test defects found by cross-OS runs)

Both are test defects; the production code was correct on every OS.

- **`schema/tests/ws_handshake.rs::query_secret_is_absent_from_every_dial_error`
  failed on native Windows** (`HandshakeTimeout(2s)` where `Handshake` was
  expected, `tests\ws_handshake.rs:389`). Case (a) dials a just-freed
  loopback port to get a refusal. Windows answers a closed port's RST by
  retrying the SYN and reports the refusal only after about 2 s, so the
  shared 2 s `handshake_timeout` fired first. The test predates this phase
  (Phase 5) and had not run on native Windows before. Fix: case (a) uses its
  own 10 s budget; a refusal still ends it at once on macOS/Linux, and after
  ~2.26 s on Windows (measured in the rerun). Fix attempt 1 of 3 passed. The
  trap is recorded in the `os` skill (`windows.md`, "A refused loopback
  connect takes about 2 seconds", plus a router row in `SKILL.md`).
- **`gen/tests/l1/openapi_import_test.rs::a_referenced_separable_union_generates_a_multi_shape_success`
  failed on Linux.** It picked "the API module" as the first `.rs` entry of
  `read_dir` not named `lib.rs`/`shared.rs`/`prelude.rs`, but
  `generate_and_write` also writes the shipped WebSocket modules and
  `ws_shared.rs`; ext4's hash order returned `samsung_smart_tv_remote_ws.rs`
  first. The `os` skill already documents this `read_dir` trap
  (`linux.md`), so no skill entry was added. Fix: collect the candidates,
  exclude `ws_shared.rs` and `*_ws.rs`, and require exactly one. The test
  came from an earlier REST feature, but its hidden dependence on the
  WebSocket output makes it this change's to repair; other `read_dir`
  readers in `gen/tests` use no `.find(` over the directory.

### Documentation and skills audit (Wave 16, Audit Documentation)

Plan checklist against `docs/websocket-definitions.md` and `README.md`:
typed and value calls ("The generated messages, client, and host"),
parameters ("Handshake parameters"), auth ("Authentication and
environment", "The handshake credential"), setup callbacks ("Setup
callbacks for challenge flows"), correlation ("Linking a request to its
response"), host hooks (`on_authenticated`, `on_error`, routing), caller
heartbeats ("Runtime hints and heartbeats"), and lifecycle/state flow
(Mermaid state diagram in "Connecting, setup, and recovery"); all were
present from earlier phases. **Missing: cleartext local-device transport.**
Added to `docs/websocket-definitions.md` (authentication bullets): a
self-signed `wss://` certificate (Samsung TV :8002) fails the dial as
`WsError::Handshake` (verified in `ws_shared.rs::handshake_error`: every
non-HTTP tungstenite error maps there), and a `ws://` base URL (Unfolded
Circle Core and Dock) is cleartext, sends any credential unencrypted, and
emits no warning (plan rule 9: no unsolicited warning). This writes down
the open TLS issue logged in Phase 5; it adds no behavior.

Skills:
- `.claude/skills/schematic-define/schematic/websocket.md`: the same
  TLS/cleartext line.
- `.claude/skills/unfolded-circle/api-clients.md`: drift fixed — the
  Integration WebSocket link named `unfolded_circle_dock_ws` and pointed at
  the nonexistent `unfolded_circle_doc_ws.rs`; now
  `unfolded_circle_integration_ws.rs`.
- `.claude/skills/homelab/SKILL.md`: drift fixed — the dependency row said
  `homelab` lib uses only REST clients from `schematic-schema`; it also uses
  the Samsung remote WebSocket client (`samsung_tv.rs`), and the four
  integration crates use the typed `unfolded_circle_integration_ws` messages
  and host.
- `.claude/skills/os/windows.md`, `.claude/skills/os/SKILL.md`: the Windows
  refused-connect trap above.
- `docs/dependencies.md`: no crate was added or removed this phase; no edit.

Checks: `git grep` finds no feature name or `{date}-{name}` in
`schematic/docs`, `schematic/README.md`, or the touched skills, and no
**planned** marker in the WebSocket docs (everything described is built).
Comment drift: a sweep of `gen/src/ws_codegen/*.rs`,
`runtime/src/credentials.rs`, and `define/src/websocket/*.rs` for retired
behavior (`TODO`, "not enforced", `AuthRejected`/`AuthTimeout`,
`WriterCommand::Close`, opening `send_*`, lifetime counters) found nothing
beyond the drift Phase 6 already fixed; the one hit ("Ids come from one
counter per connect, never reset by a redial", `client.rs`) is correct.
Input robustness: this phase adds no loader or field. The matrices for
every field added by this feature are recorded in Phases 1, 3, 5, and 6
(`open_credential_field` in Phase 6), each tested at the crate's public API.

### Environments (Wave 16, Verify Environments)

Hosts: `BUILD_LINUX=build-linux`, `BUILD_WIN=build-win-native`,
`BUILD_WSL=build-win` (all declared and reachable). Every remote run used
`./scripts/cross-check.sh`, which ships the local tree (uncommitted Phase 6
and 7 changes included) as a throwaway commit. Phase 6's attempt failed
because `just cross-check schematic-schema` cannot resolve a crate outside
the workspace; the `os` skill's documented native form for the generated
crate (`--all-features --manifest-path schematic/schema/Cargo.toml`) works,
and that is what ran here. No CI run was triggered.

| Package / scope | macOS (local) | Linux | Native Windows | WSL2 |
|---|---|---|---|---|
| `schematic-schema` (own manifest; all 149 incl. `ws_session` 26, `ws_idle` 10, `ws_handshake`, `ws_correlation`, `ws_runtime`, `ws_mock_harness`, compile-fail) | `just test-schema` 149/149 | 149/149 | run 1: 148/149 (`query_secret_is_absent_from_every_dial_error`, fixed above); run 2: 149/149 | 149/149 (with the fix) |
| `schematic-gen` L1 | in `just test` | 842/842 after the `read_dir` fix (run 1: 841/842) | 840/840 (archive) | 842/842 (archive) |
| WebSocket fresh-output (`slow_bearer_ws_host…`, `slow_typed_ws_clients…`, `slow_ws_handshake_credentials…`, `slow_ws_handshake_parameters…`, `slow_ws_session_setup_and_recovery_run`) | `just test` fresh-output pass 14/14 | 5/5 (native) | 5/5 (native) | **not verified** (not run: WSL2 follows Linux code paths and Linux passed; the archive mode cannot select `slow_` tests) |
| `schematic-runtime` L1 | in `just test` | 461/461 | not run | not run |
| `homelab` lib | `just test homelab` | 49/49 | not run | not run |
| `unfolded-integration-helper` | `just test homelab` | 32/32 | not run | not run |
| `arcam-amp-` / `eversolo-` / `sony-receiver-integration`, `homelab-cli`, `homelab-server`, `sniff`, `biscuit-speaks`, `model-citizen` | see Dependents | not run | not run | not run |

**Not verified**, with reason: the dependents other than `homelab` lib and
`unfolded-integration-helper` on Linux, and every dependent on native
Windows and WSL2. Their WebSocket-facing code did not change in Phase 6
(no homelab edit was needed) and they hold no OS-specific code on the
changed surface. The pull request's CI proves Linux and macOS for each
changed package; the nightly proves native Windows and WSL2 and fixes
forward. `schematic-runtime` was not run on Windows/WSL2 here: its change
(`expose_secret`, storing the unplaced secret) is string handling with no
OS surface. Determinism: no flaky test was seen on any host; one REST test
(`github_client::anonymous_call_sends_no_credential_despite_client_and_env`)
was reported `LEAK` once in the local `test-schema` run and passed (not
WebSocket code, not seen elsewhere).

### Acceptance matrix (R1–R17)

"Local" means `schematic-schema` tests (own manifest; CI's workspace run does
not execute them, but `just test-schema` and the cross-checks below did).
"CI-visible" means a child test of a `fresh_output_compile` `slow_` test that
`just test` runs in its fresh-output pass. Every row's tests passed in this
phase's gate runs (macOS) unless stated.

| Req | Evidence (exact tests) | Status |
|---|---|---|
| R1 URLs | `schematic-definitions` `core_ws::tests::core_ws_has_expected_endpoints`; gen `ws_validation::{every_shipped_endpoint_composes_its_documented_url, shipped_definitions_report_no_warning_or_error, an_exactly_duplicated_boundary_segment_warns_without_blocking, a_legitimately_repeated_segment_is_only_a_warning, a_substring_or_prefix_overlap_emits_nothing}`; schema `ws_runtime::core_endpoints_dial_their_documented_channel_paths` | pass |
| R2 consecutive-failure budget | schema `ws_session::{repeated_losses_with_successful_redials_never_exhaust_the_budget, two_consecutive_refused_redials_exhaust_a_budget_of_two, zero_max_attempts_closes_on_loss_without_redialing, unlimited_max_attempts_survives_consecutive_failures, budget_resets_after_each_successful_redial, no_reconnect_policy_closes_on_loss}`; CI-visible child `the_reconnect_budget_counts_consecutive_failed_redials_and_resets_after_each_setup` | pass |
| R3 opening + setup on every dial | schema `ws_session::{opening_message_is_written_first_on_every_connection, opening_without_a_key_field_is_filled_from_the_credential, opening_with_the_same_key_keeps_one_field, opening_with_a_different_key_is_refused_before_dialing, anonymous_opening_keeps_the_callers_key, multi_context_opening_is_sent_unchanged, opening_write_failure_fails_the_connect_without_redialing, setup_runs_before_ready_on_every_connection, setup_rejection_on_connect_is_returned_without_a_second_attempt, setup_rejection_on_redial_closes_the_client, transient_setup_failures_on_redials_use_the_budget, setup_cancellation_drops_stream_and_invalidates_session}` + shutdown/deadline tests; CI-visible children `the_opening_frame_comes_first_on_the_initial_connection_and_on_a_redial`, `setup_runs_with_the_handshake_key_before_ready_on_the_initial_connection_and_on_a_redial`, `a_rejected_setup_fails_the_connect_with_setup_rejected_after_one_connection`, `an_opening_key_that_differs_from_the_credential_fails_the_connect_before_dialing` | pass; **gap**: opening-write failure on a *redial* untested (no deterministic trigger; see Phase 6) |
| R4 Bearer host compiles | gen `fresh_output_compile::slow_bearer_ws_host_compiles_and_checks_tokens` (+ children `bearer_host_rejects_a_missing_token_with_401`, `…_wrong_token_with_401`, `bearer_host_upgrades_a_matching_token`) | pass (CI-visible) |
| R5 credential precedence | runtime L1 `handshake_credentials::*`; schema `ws_handshake::{core_credential_precedence_per_call_then_client_then_env, dock_without_auth_policy_ignores_its_env_token, redial_reuses_the_credential_selected_at_connect, initial_401_and_403_are_returned_without_a_second_attempt, redial_401_is_terminal, redial_403_is_terminal, redial_redirect_is_terminal, invalid_headers_and_credential_kinds_fail_before_dialing}`; CI-visible `slow_ws_handshake_credentials_and_correlated_requests_run` children | pass |
| R6 anonymous | schema `ws_handshake::core_without_any_credential_connects_anonymously`; child `nothing_resolved_connects_anonymously_and_reports_it`; runtime `an_api_accepting_no_credential_ignores_the_environment` | pass |
| R7 no leaks, no redirect | schema `ws_handshake::{redirect_is_not_followed_and_its_location_is_redacted, refusal_headers_redact_set_cookie_and_secret_values, query_secret_is_absent_from_every_dial_error}`; child `a_302_upgrade_is_not_followed_and_its_location_is_redacted` | pass (the query-secret test was fixed this phase for native Windows, see below) |
| R8 receive timeout | schema `ws_idle::{silence_after_a_frame_times_out_and_redials, no_receive_timeout_keeps_a_silent_connection, server_pings_keep_the_connection_alive_and_are_ponged, outbound_sends_do_not_reset_the_idle_timer, a_full_event_queue_still_times_out_publishes_state_and_closes, stalled_writes_do_not_block_close, stalled_writes_do_not_block_the_idle_timeout_and_pending_request_fails}`; child `a_silent_server_times_out_the_receive_and_the_client_redials`; gen `plan::tests::no_info_note_reports_an_unenforced_receive_timeout` | pass |
| R9 diagnostics | gen `ws_validation::{a_lowering_error_reaches_the_shared_format_and_blocks_the_rust_output, info_notes_stay_out_of_the_shared_diagnostics, the_validate_command_keeps_info_notes_out_of_its_default_output}`; `plan::tests::{lowering_failure_keeps_every_finding_not_only_the_first_error, diagnostic_maps_onto_the_shared_format}`; `diagnostics::tests::display_and_serialized_ids_agree_for_every_variant` | pass; **coverage note**: the spawned CLI no longer proves a verbose run prints an Info (no shipped definition emits one); the rule is covered in-process |
| R10 typed messages | gen `plan::tests::{more_than_one_fallback_in_a_direction_is_an_error, identical_selectors_are_an_error, incomparable_overlapping_selectors_are_an_error, raw_and_mixed_framing_are_named_errors}`, `ws_codegen::{core_message_enums_name_one_variant_per_declared_message, every_shipped_client_sends_its_typed_enum_and_values_only_through_send_value}`; schema `ws_runtime::{core_ws_decodes_every_server_message_variant, core_ws_selects_the_most_specific_message_before_the_fallback, core_ws_typed_send_writes_the_payload_alone, core_ws_typed_and_value_receivers_share_one_queue}` (and per-API decode tests); CI-visible `slow_typed_ws_clients_and_a_multi_endpoint_host_compile_and_run` (incl. `WS_MISUSES` compile-fail E0308 cases) | pass |
| R11 `responds_with` | gen `plan::tests::lower_pairs_requests_through_responds_with_and_the_endpoint_hints` and siblings, `ws_codegen::{the_planner_names_no_envelope_message, renaming_the_envelope_messages_changes_no_correlation}`; schema `ws_correlation::*` (7); CI-visible children `string_request_ids_go_out_as_json_strings_and_return_the_declared_response`, `a_same_id_frame_of_another_message_is_an_event_not_the_answer` | pass |
| R12 shared vocabulary | define `compile_fail` doctests (4) for `ConnectionParam`/`ParamType`; `websocket::tests::{shared_parameter_types_are_serializable, derived_policy_and_env_mapping_follow_rest}`; definitions `ws_definition_json::every_shipped_definition_round_trips_twice_without_loss` | pass |
| R13 five definitions normalized | definitions `ws_conformance::{roles_and_directions_match_the_expected_table, every_inbound_direction_can_choose_one_message, every_response_link_resolves_and_is_correlated, challenge_flows_are_declared_as_hints_not_prose}`; `integration_ws::tests::integration_ws_declares_both_roles_and_true_directions`; gen `ws_codegen::a_declared_heartbeat_is_documented_on_the_client_without_a_warning`, `plan::tests::heartbeat_hints_lower_without_any_warning` | pass |
| R14 protocol-neutral host | gen `ws_codegen::the_integration_host_is_named_for_its_api_and_sends_no_protocol_envelope`; CI-visible host children (`the_room_hook_message_is_the_first_frame_and_runs_once_per_connection`, `connections_route_by_path_and_unknown_paths_are_rejected_with_404`, …) | pass |
| R15 dependents | "Dependents" table above; removed-name absence enforced by tests listed there | pass on macOS; Linux evidence below |
| R16 mock harness | schema `ws_mock_harness::*` (12) | pass on macOS, Linux, native Windows, WSL2 (below) |
| R17 generated output/docs/skills | `just lint-schema` pass; `just check-drift` 9/9 (nextest; see note); manual doc/skill pass below | pass |

### Ledger rows (parent D20)

| Row | Defect | Closed by |
|---|---|---|
| 3 | Core URL `ws://remote.local/ws/ws` | R1 tests above; `ws_runtime::core_endpoints_dial_their_documented_channel_paths` failed before the definition fix |
| 9 | reconnect counter never reset; opening never re-sent | R2/R3 `ws_session` tests and CI-visible children above |
| 10 | dormant `E0308` in the Bearer host | `slow_bearer_ws_host_compiles_and_checks_tokens` (child build fails `E0308` with the `;` restored) |
| 11 | env auth never applied | `ws_handshake::core_credential_precedence_per_call_then_client_then_env`, `redial_reuses_the_credential_selected_at_connect`; runtime `handshake_credentials` |
| 12 | diagnostics discarded | `ws_validation::a_lowering_error_reaches_the_shared_format_and_blocks_the_rust_output`, `plan::tests::lowering_failure_keeps_every_finding_not_only_the_first_error` |

All five rows are closed; none is an accepted gap.

### Gates (Wave 17, Record Acceptance)

Run from `schematic/`, after every fix, with no subagent running:

- `just test`: pass — L1 2776 passed, 20 skipped (tier-filtered `slow_`
  tests, run by the next pass, and three pre-existing `#[ignore]`d
  `e2e_generation` tests that this feature does not own); fresh-output 14/14
  (includes all five WebSocket `slow_` tests); compile-fail 1/1. Run twice:
  before and after the `openapi_import_test` fix.
- `just test-schema`: 149/149 (before the `ws_handshake` fix; the fixed test
  was rerun alone on macOS and passed, and the whole crate passed with the
  fix on Windows and WSL2).
- `just doctest`: pass (define 173 + 6 ignored `rust,ignore` generated-surface
  examples; definitions 18; gen 44 + 2 ignored; oauth 19; runtime 3 + 3).
- `just lint`: pass (after removing a needless borrow in my test fix);
  `just lint-schema`: pass.
- Drift: the plan's `cargo nextest run -p schematic-gen --test l1
  --run-ignored only -E 'test(artifact_drift)'` now selects **0 tests**: the
  `artifact_drift` tests are no longer `#[ignore]`d, and `just check-drift`
  now runs them through nextest (`--no-tests=fail`), which removes the
  plan's reason to avoid it. `just check-drift`: 9/9, clean.
- `just check-tier-coverage schematic` (root): 0 stranded.
- Gate-skip calculator (`affected_scope.py` over the working tree):
  `packages: [schematic-define, schematic-definitions, schematic-gen,
  schematic-runtime, test-toolkit, darkmatter]`, `change_class: package`,
  so no gate was skipped. (The tree includes the uncommitted Phase 6 work;
  `test-toolkit`/`darkmatter` are pulled in by the skill edits.)

### Outstanding items for review

- gap (R3): opening-write failure on a **redial** is untested (no
  deterministic trigger without a transport seam; Phase 6).
- coverage note (R9): the spawned CLI no longer proves that a verbose run
  prints an Info note; covered in-process only (Phase 6).
- observation: Dock's setup session exposes no secret (policy `None`); a
  Dock setup callback brings its token from its own closure (Phase 6).
- open issue (Phase 5, plan rule 11): no WebSocket TLS opt-in; a Samsung TV's
  self-signed `wss://` certificate is expected to fail the handshake
  (unverified, no device contacted). Now documented.
- not verified: dependents on native Windows/WSL2, most dependents on Linux,
  WSL2 fresh-output (see Environments).
- adopted defaults from all phases are recorded in each phase's section
  above; none was changed this phase.

No commit, push, CI run, device or provider contact, stale-file deletion,
`just complete`, or lifecycle move was made.

Phase 7 checkpoint met: R15 and R17 pass, earlier checkpoints remain green,
drift is clean, and the requirement/evidence matrix above lists every gap.
Status: **implementation complete, ready for review**.
