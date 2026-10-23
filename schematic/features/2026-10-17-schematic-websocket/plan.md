---
total_phases: 7
created: 2026-10-11
phase: 1
agent: codex/gpt-6.1-sol
yolo: "true"
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
---

# Schematic WebSocket Implementation Plan

## Work Summary and Success Criteria

Implement `2026-10-17-schematic-websocket` by making WebSocket definitions reuse the settled REST vocabulary, emitting typed protocol surfaces, applying shared credential and redaction rules, and repairing connection recovery. Preserve separate REST and WebSocket definition types and the existing transport architecture. Normalize all five definitions and migrate their consumers alongside breaking changes.

Success means R1–R17 have observable acceptance evidence: correct handshake targets; recoverable sessions that finish setup before `Ready`; consecutive-failure reconnect budgets; enforced inbound idle timeouts; one retained credential selection; terminal authentication and redirect rejection; credential-free diagnostics; selector-based typed messages and explicitly linked correlation; protocol-neutral hosts; and passing dependent builds. Generated output, documentation, skills, and comments must describe the resulting behavior. Completion is **implementation complete, ready for review**, with OS evidence and any outstanding gaps recorded honestly.

This plan is prepared against the 2026-10-11 draft. The preceding `2026-10-16-schematic-response-model-graphql` is currently marked `implemented: false`, and this specification has `refresh_required: true`. Phase 1 establishes readiness before source implementation; names inherited from preceding specifications are provisional until that refresh. Planning this document does not authorize implementation or the specification's pending pre-authorizations.

Each phase depends on the previous phase's checkpoint. Waves have globally unique numeric indexes. Tasks in a wave may be delegated concurrently only when their file ownership is disjoint; a singleton wave is sequential. A wave is the execution step for its grouped tasks. One integration owner handles shared manifests, module registration, generated output, and checkpoint runs; agents never regenerate the same output concurrently. No subagents are needed to prepare this plan.

At every phase that changes generator output, regenerate through the generator, migrate affected consumers, run the relevant tests and drift check, and update affected behavior documentation before declaring the phase green. Do not hand-edit generated source, run `cargo fmt`, commit, push, contact devices/providers, trigger speculative CI, delete stale generated files, run `just complete`, or move the feature to `_completed`.

## Phase 1: Resolve Contracts and Prove Setup Feasibility

### Necessary Rules

1. **Refresh prerequisite.** Confirm the preceding feature has landed and completed its first human review. Refresh this spec and plan from those outcomes before implementation. Record the final shared runtime package, `EndpointSpec`, `ParamDef`, location containers, typed path parameters, header types, `AuthPolicy`, `EnvMapping`, serde support, diagnostic format, writer behavior, and isolated compile-check interface. Missing REST parameter serde support is a predecessor defect; do not add those derives here. Missing shared diagnostics contracts cannot be replaced with a WebSocket-only design.
2. **Authorization — granted.** The author confirmed the specification's listed pre-authorizations on 2026-10-21; log each use. The spec's "Not pre-authorized" list still applies. A missing required upstream contract is still a prerequisite blocker under rule 1.
3. **Adopted defaults.** Use `responds_with: Option<String>` with endpoint-only correlation hints; `{Endpoint}ServerMessage` / `{Endpoint}ClientMessage`; `send_value`, `next_value_event`, `value_events`, and `request_value`; `{ApiName}EventHub`; endpoint-specific authentication hooks defaulting to no message. Log planning items 1–21 with adopted values, alternatives, and any upstream replacement. Roles are `Client`, `Host`, `Both`, with absence defined as `Client`; message directions remain client-relative.
4. **Selector ambiguity.** Proposed planning choice: selectors are conjunctions of equality checks on existing wire fields, using the existing field-path syntax. Reject identical predicates and predicate sets that can simultaneously match because they contain no contradictory equality on a shared field. Do not use declaration order or invent nested paths. A fallback cannot also declare selectors. Record this conservative overlap rule for review; if it rejects a shipped protocol definition, resolve the modeling conflict before emission rather than quietly choosing a variant.
5. **Setup contract.** Proposed planning choice: send the retained typed opening payload first, then invoke a reusable callback factory that creates a new future for each dial. Its typed session exclusively owns setup access to that connection, uses the same message codecs, and exposes the resolved credential through a private/redacted credential facility. Completion transfers transport ownership to the supervisor; timeout, close, or drop cancels the future and invalidates the session without a detached task. Callback rejection is terminal; transient failure consumes the reconnect budget. `Ready` and counter reset occur only after all required setup completes.
6. **Opening credentials.** Proposed planning choice: reject a caller-supplied opening auth field that conflicts with the selected shared credential; fill an absent declared auth field from that credential. An equal value is one source, not another selection. Bind each applicable protocol's field explicitly; never infer secret fields from payload shape or derive an independent credential from them. With no resolved credential, permit anonymous upgrade and report a later protocol rejection. Log how ElevenLabs and any other auth-bearing opening payload obey this rule.
7. **Timing and recovery.** Until first `Ready`, dial, upgrade, opening write, and callback share `min(remaining deadline, per-attempt timeout)`. No deadline renewal on redial. After first `Ready`, recovery has only the per-attempt timeout. `Some(0)` disables redials, `Some(n)` allows at most `n` consecutive failed redials, `None` has no attempt limit. Clear `Ready` immediately on loss; never replay pending or queued application commands. Manual close/drop interrupts every state.
8. **Auth and metadata.** Resolve once per explicit connect, retain that selection on redials, attach only the selected method, and expose only `AuthMethod`. No credential means anonymous. Never follow any upgrade redirect; redact `Location`, `Set-Cookie`, query credentials, private opening storage, and Schematic-controlled errors/metadata using the runtime boundary. Reject competing explicit credential sources and transport-managed upgrade-header overrides. No WebSocket-specific anonymous or insecure-TLS switch.
9. **Heartbeat and Samsung.** Caller-managed application heartbeats are settled, produce rustdoc and no warning, and retain metadata. Samsung correlation requires documented request/response ID evidence; an optional envelope `id` alone proves no pair. Otherwise remove the unused explicit ID declaration. Test inherited opt-in TLS with a local TLS fixture only if supported; actual device requirements remain unverified. Do not add an unsolicited cleartext warning or contact a device to settle it.
10. **Compile-test policy.** The spec suggests `slow_` for a slow isolated compile test; the current `rust-testing` guide says an L1 test over five seconds fails its speed contract and a prefix does not fix it. Ruled 2026-10-21: place the R4 compile test in the existing `fresh_output_compile` module of `schematic-gen`, which runs in the separate `just test-fresh-output` pass that `just test` already invokes; give it no `slow_` prefix. Reuse that facility's build caching while keeping generated source fresh. Do not hide, skip, weaken, or report the check as passing without executing it.
11. **Scope boundaries.** Keep AsyncAPI import working by adapting its typed construction to new definitions; do not add the exporter, round-trip export tests, new file formats, automatic heartbeats, REST changes, or a transport redesign. Preserve selectors, roles, and heartbeat metadata for the following exporter feature. The minimal transport-neutral `schematic-runtime` function is pre-authorized; any other shared-runtime capability, a newly required third-party crate, or stale-output deletion is logged as an open issue and the run proceeds without it where possible.

### Wave 1 — Contract Inventory (parallel reads)

- [x] **Refresh Contracts**
  - Verify review outcomes and prerequisite readiness; read the relevant shared definitions/runtime/generation code after the refresh.
  - Create `implementation-log.md` when implementation begins, recording authorization evidence, chosen interfaces, all adopted defaults, deviations, and unresolved prerequisites as they occur.
  - Identify existing user changes and preserve them. Use the `sniff` skill for repository/host discovery and the `os` skill for eventual environment evidence.
- [x] **Map Protocols**
  - Inventory Core, Dock, integration, Samsung, and ElevenLabs endpoints, lifecycle payloads, selectors/fallbacks, wire framing, auth fields, hints, roles, and consumers in a per-endpoint expected table.
  - Use the already verified Core origin and `/ws`, `/intg`, `/profiles` paths; do not schedule another discovery spike for that known defect.
  - Read Samsung protocol notes for correlation evidence and inspect the current TLS adapter; record the ordinary-Serde ElevenLabs payload finding.
- [x] **Map Verification**
  - Read `rust-testing`, `just`, Test Toolkit, existing `schema/tests/ws_runtime.rs`, consolidated generator targets, importer fixtures, and predecessor compile-check support.
  - Inventory exact affected packages and executable checks, including schema's separate manifest and ignored drift tests. Establish baseline results without starting device or terminal tests.
  - Finalize the input matrix below against the refreshed shapes; add a separate column for each newly discovered load-bearing leaf before touching its reader.

### Input Robustness Matrix

This applies to changed definition deserialization contracts, not application wire-message payloads, generated Rust writers, or Markdown. The JSON definition representation has one column per new or changed decision-bearing field below. Shared auth/parameter/header readers retain their upstream matrices; link their exact tests in the log and extend those matrices only for a newly read leaf. No manifest or lockfile reader is added.

Codes specify public outcomes: **ERR** = a field-addressed public load/generation error and shared-writer error behavior; **NONE** = absence explicitly means no declaration; **CLIENT** = role defaults to client; **EMPTY** = explicit empty collection is retained, then normal validation runs; **FALSE** = no fallback; **PAIR** = valid declared request/response pair; **SELECT** = the intended selected variant. `N/A` means a scalar has no collection elements; substitute a malformed nested value where the field is structured.

| JSON definition shape | `endpoint.role` | `message.responds_with` | `message.selectors` | `message.fallback` |
|---|---|---|---|---|
| Unedited generated fixture | Declared host/client output | PAIR | SELECT | Declared fallback receives unmatched frame |
| Absent | CLIENT | NONE; no inferred pair | NONE; ERR if required for multiple inbound messages | FALSE |
| Explicit null | ERR | ERR; distinct from absent | ERR | ERR |
| Wrong whole-field type | ERR | ERR | ERR | ERR |
| One malformed element | N/A | N/A | ERR | N/A |
| Every element malformed | N/A | N/A | ERR | N/A |
| Empty value/collection | ERR for empty string | ERR for empty name | EMPTY; ERR when selectors required | N/A; `{}`/`[]` are ERR |
| Duplicate key | ERR | ERR | ERR | ERR |
| Trailing/invalid document | ERR | ERR | ERR | ERR |

Selector leaf cells apply independently to each selector object; omission never supplies an invented wire field or comparison value.

| JSON definition shape | `selector.field` | `selector.value` | `endpoint.runtime.correlation` | `endpoint.runtime.auth_flow` | `endpoint.runtime.heartbeat` |
|---|---|---|---|---|---|
| Unedited generated fixture | Intended wire field | Exact comparison | Typed linked request completes | Setup callback completes before Ready | Metadata and rustdoc retained; no warning |
| Absent | ERR | ERR | NONE; link then ERR | NONE; no required challenge callback | NONE; no declared heartbeat |
| Explicit null | ERR | ERR; null matching is not introduced | ERR | ERR | ERR |
| Wrong whole-field type | ERR | ERR against declared scalar contract | ERR | ERR | ERR |
| One malformed element/leaf | N/A | N/A | ERR | ERR | ERR |
| Every element/leaf malformed | N/A | N/A | ERR | ERR | ERR |
| Empty value/object | ERR for empty field | Valid empty scalar only if its declared type permits it | ERR for incomplete `{}` | ERR for incomplete `{}` | ERR for incomplete `{}` |
| Duplicate key | ERR | ERR | ERR | ERR | ERR |
| Trailing/invalid document | ERR | ERR | ERR | ERR | ERR |

The following JSON leaf tables complete the structured fields above. **BOUND** means a valid value within the refreshed shared duration contract; Phase 1 must substitute the exact zero/minimum rule before implementation. **DEFAULT** means the existing documented implicit default is retained; explicit null is never that default. **WARN** means an explicitly unused declaration produces the R11 warning. Correlation and auth-flow leaf rows assume their containing object is present.

| JSON definition shape | `correlation.request_id_field` | `correlation.response_id_field` | `correlation.default_timeout_secs` | `auth_flow.challenge_message` | `auth_flow.auth_request_schema` |
|---|---|---|---|---|---|
| Unedited generated fixture | Matching request ID inserted | Matching linked response completes | Request uses declared timeout | Callback receives declared challenge type | Typed authentication request resolves |
| Absent | ERR | ERR | DEFAULT only if upstream defines one; otherwise ERR | ERR | ERR |
| Explicit null | ERR | ERR | ERR | ERR | ERR |
| Wrong whole-field type | ERR | ERR | ERR | ERR | ERR |
| One malformed element | N/A | N/A | N/A | N/A | N/A |
| Every element malformed | N/A | N/A | N/A | N/A | N/A |
| Empty value | ERR | ERR | Zero obeys BOUND | ERR | ERR |
| Duplicate key | ERR | ERR | ERR | ERR | ERR |
| Trailing/invalid document | ERR | ERR | ERR | ERR | ERR |

| JSON definition shape | `auth_flow.success_indicator` | `auth_flow.failure_indicator` | `heartbeat.interval_secs` | `heartbeat.timeout_secs` |
|---|---|---|---|---|
| Unedited generated fixture | Declared success metadata retained | Declared rejection metadata retained | Rustdoc states interval | Timeout metadata retained |
| Absent | NONE | NONE | ERR | NONE |
| Explicit null | ERR | ERR | ERR | ERR |
| Wrong whole-field type | ERR | ERR | ERR | ERR |
| One malformed element | N/A | N/A | N/A | N/A |
| Every element malformed | N/A | N/A | N/A | N/A |
| Empty value | ERR for empty indicator | ERR for empty indicator | Zero obeys BOUND | Zero obeys BOUND |
| Duplicate key | ERR | ERR | ERR | ERR |
| Trailing/invalid document | ERR | ERR | ERR | ERR |

Apply the runtime leaf columns independently at each API/endpoint scope that the refreshed definition exposes. API-level correlation is not introduced.

| JSON definition shape | `api.runtime` | `endpoint.runtime` | `runtime.frame_format` | `runtime.supports_reconnect` | `runtime.request_id_type` |
|---|---|---|---|---|---|
| Unedited generated fixture | Effective API hints retained | Effective endpoint hints retained | Declared framing observed | Recovery follows declared support | String/U64 observed by server |
| Absent | NONE; documented effective defaults | NONE; inherit only supported API hints | DEFAULT | DEFAULT | DEFAULT; no unused-declaration warning |
| Explicit null | ERR | ERR | ERR | ERR | ERR |
| Wrong whole-field type | ERR | ERR | ERR | ERR | ERR |
| One malformed element/leaf | ERR | ERR | N/A | N/A | N/A |
| Every element/leaf malformed | ERR | ERR | N/A | N/A | N/A |
| Empty value/object | Explicit defaults only; keep presence | Explicit defaults only; keep presence | ERR for empty string | N/A; objects/arrays are ERR | ERR for empty string |
| Duplicate key | ERR | ERR | ERR | ERR | ERR |
| Trailing/invalid document | ERR | ERR | ERR | ERR | ERR |

The `frame_format`, `supports_reconnect`, and `request_id_type` columns apply only where those leaves exist; absent endpoint hint leaves cannot create an API-level field. An explicit unconsumed ID declaration produces WARN, even when equal to the default. The public result must prove these decisions, including failure paths, rather than merely show serde accepted a value. Optional indicator/timeouts deliberately distinguish absent from null; document the changed serialization contract and omit absent fields on serialization rather than emitting forbidden nulls.
Use one data-driven test per supported definition format that starts with a real generated definition fixture, proves the positive public result, then makes one textual edit per cell. Preserve raw document text for duplicate-key and trailing-content cases; parsing into `Value` first can erase duplicates. Reject malformed elements as a whole; do not filter them out. Walk the full row and column after any review finding. Search changed readers for `#[serde(default)]`, `Option<T>` that merges absent/null, `filter_map(..as_str())`, `unwrap_or_default()`, and `.ok()` masking failure. Use presence-aware deserialization where needed; narrowly fix changed contracts rather than hardening unrelated REST readers.

The existing AsyncAPI reader supports JSON and YAML. Its parsing/field interpretation is outside this change: update only construction of the new Rust definition shape, retain existing diagnostics, and run real imported-fixture regressions in both formats. If implementation must change a read-side AsyncAPI field, first add a separate JSON and YAML matrix with a column for every affected input leaf, explicit spec-compatible absence/null/empty outcomes, duplicate rejection where forbidden, and public import/generation assertions. Do not claim the definition matrix covers the importer.

### Wave 2 — Setup Spike (sequential, once)

- [x] **Prove Setup**
  - Build a disposable, local prototype of a reusable typed callback factory, exclusive per-attempt session ownership, stream transfer, and cancellation during setup/backoff. Use existing Tokio/tungstenite dependencies and a minimal fixture type.
  - Prove a deadline covers opening plus callback, callback rejection is distinguishable from transient failure, and closing cannot leave a task owning the stream. Check the required `Send`/lifetime bounds against current consumer patterns.
  - Output the callback/session signatures and ownership decision in the log; retain only a useful regression test. Do not repeat this spike in later phases or turn it into an architectural rewrite.
  - No performance spike is scheduled: the spec names correctness risks rather than a cost needing measurement. Wider benchmarking requires an author ruling.

**Checkpoint:** refreshed contracts, explicit readiness/authorization state, endpoint expectation table, matrix inventory, and a feasible setup API are recorded. No dependent source work begins with a missing prerequisite.

## Phase 2: Establish Harness, Diagnostics, and Defect Coverage

### Wave 3 — Independent Foundations (parallel)

- [x] **Build Harness**
  - Add reusable test support under `schematic/schema/tests` using TCP `127.0.0.1:0` and tungstenite; source-include test-only support in generator tests only where useful, without a generated-crate dependency cycle.
  - Record path/query/headers and connection counts; script accept, rejection, redirect, stall, frame exchange, and drop. Provide barriers, deterministic shutdown, and an injectable failing writer for reliable opening-write failure.
  - Use Test Toolkit tracing/environment restoration, isolate every candidate credential variable, and register modules in existing nextest targets. Prove request capture, scripted drop, concurrent ephemeral-port safety, and teardown.
- [x] **Surface Diagnostics**
  - Change `ws_codegen/plan.rs` and `output/ws_modules.rs` to preserve all lowering diagnostics and failures, mapping API/path/severity/message into the inherited diagnostic mechanism.
  - Reuse shared per-output validation and writer behavior. Prove Warning permits output, Error has inherited write semantics, verbosity governs Info, and lowering errors cannot disappear.
  - Keep the receive-timeout-not-enforced diagnostic until Phase 6 implements the behavior.

### Wave 4 — Known Defects (sequential integration)

- [x] **Repair Templates**
  - Set Core's base URL to its origin and keep each documented channel path. Add exact path-segment overlap Warning; cover legitimate repeated segments and `/api` versus `/apikeys` without substring false positives.
  - Remove the Bearer host's erroneous semicolon in `ws_codegen/shared.rs`; generate its fixture into an isolated fresh-output compile check.
  - Assert all five definitions' composed URLs and actual mock request targets. Run missing/wrong/matching Bearer tests against the compiled generated host, verifying 401 versus successful upgrade.
- [x] **Integrate Foundations**
  - Regenerate once after the wave; record actual compile-test execution, update affected URL/auth docs, and run focused harness, diagnostic, URL, Bearer, schema, and drift checks.

**Checkpoint:** R1, R4, R9, and R16 pass through public boundaries; generated modules compile and no diagnostic is silently dropped. Existing supervisor behavior remains explicitly pending.

## Phase 3: Adopt Shared Vocabulary and Normalize Definitions

### Wave 5 — Definition Contracts (sequential)

- [x] **Replace Vocabulary**
  - Update `define/src/websocket/{mod,lifecycle,hints,params}.rs` to use settled REST auth policy, env mapping, headers, and location-aware parameters. Map old Float to shared Number and preserve typed path parameters, defaults, and styles.
  - Add serializable endpoint roles, selectors/fallbacks, and `responds_with`; make role selection explicit before correcting message directions. Remove public `ConnectionParam`/`ParamType` and migrate every source user in this phase.
  - Add serde/presence and public matrix tests, role/metadata round trips, shared parameter serde assertions, and compile-fail checks for removed names using the existing compile-fail pattern. Do not add predecessor-owned serde derives.

### Wave 6 — Definition Adoption (parallel after Wave 5)

- [x] **Normalize Protocols**
  - Migrate all five definitions using the Phase 1 endpoint table: honest directions, required roles, wire selectors/fallbacks, explicit links/hints, env mappings, headers, auth flows, and heartbeat metadata.
  - Keep ElevenLabs payload structs as ordinary Serde types resolved through the shared schema mechanism. Set Samsung correlation only with documented pair evidence; remove an unconsumed explicit ID declaration otherwise.
  - Add mechanical conformance coverage for every endpoint, including a `Both` fixture with one-way directions and retained host output.
- [x] **Adapt Planning**
  - Migrate lowering and URL generation to the new containers and role contracts; adapt AsyncAPI import construction while preserving both JSON/YAML fixture behavior and its existing CLI role selection.
  - Reject missing placeholders, embedded endpoint query strings, origin-changing endpoints, and unsupported styles. Preserve base-path prefixes; encode `/`, spaces, and `?` inside one path segment, optional query omission, and repeated query style.
  - Keep new metadata intact in plans. Update client rustdoc generation with each declared heartbeat interval/message and no heartbeat warning.

### Wave 7 — Vocabulary Integration (sequential)

- [x] **Integrate Vocabulary**
  - Resolve shared fixtures/module registration, regenerate, migrate consumers of removed definition symbols, and update definition README/topic pages and skill contracts.
  - Run all-five lowering/conformance, importer JSON/YAML, serialization, parameter, role, compile-fail, schema, and drift checks.

**Checkpoint:** R12–R13 pass; required hosts still exist; the new definitions survive serialization without losing decision-bearing metadata. R11 link execution and R10 typed runtime behavior are still pending.

## Phase 4: Emit Typed Messages and Protocol-Neutral Hosts

### Wave 8 — Typed Codecs (sequential shared contract)

- [ ] **Emit Messages**
  - Emit per-endpoint client/server enums with payload-only serialization, client-relative directions, lifecycle exclusion, and derives limited to payload capabilities. Empty directions compile as uninhabited enums.
  - Select by declared predicates before decoding, enforce overlap/fallback/selector errors, and retain original text or bytes in decode errors. JSON text/binary remain distinct; unsupported raw/mixed framing yields a named diagnostic.
  - Change primary `send`, `next_event`, and `events` to typed messages; add explicit value counterparts consuming the same bounded queue exactly once. Malformed messages yield an error and leave the connection usable.
  - Add wrong-send compile-fail coverage, empty/non-Clone payload compile fixtures, every recorded inbound type for all five definitions, overlapping Core pong/response cases, fallback, unmatched, and malformed-followed-by-valid cases.

### Wave 9 — Host and Consumers (parallel with disjoint ownership)

- [ ] **Generalize Hosts**
  - Use `{ApiName}EventHub`, remove fixed Unfolded Circle envelopes, and emit endpoint-specific typed `on_authenticated` hooks with default None. Host input uses client enums; output uses server enums.
  - Prove one hook invocation after successful auth and before application traffic, no invocation on auth failure, exact custom output/no extra frame, and connection closure plus reporting on hook-write failure.
  - Compile/test a non-Unfolded and multi-endpoint host fixture; its generated source contains no Unfolded Circle assumptions.
- [ ] **Migrate Consumers**
  - Update `homelab/lib/src/samsung_tv.rs` and the four named integration packages for typed messages, value escape hatches where intentionally needed, and the finalized hub/hook API. Implement protocol-specific auth-success messages in integration handlers.
  - Coordinate the host contract before editing; do not concurrently edit templates, manifests, or generated files. Build against regenerated output in Wave 10.

### Wave 10 — Typed Integration (sequential)

- [ ] **Integrate Surfaces**
  - Regenerate, finish host consumer integration, and update public README/topic/skill examples with typed and value operations and queue ownership.
  - Run typed/host fixture tests, schema tests, affected consumer builds, `just lint-schema`, and drift checks.

**Checkpoint:** R10 and R14 pass; consumer source compiles with typed public surfaces and protocol-neutral host names. Definitions select the intended variants, not simply any deserializable payload.

## Phase 5: Apply Shared Handshake Auth and Typed Correlation

### Wave 11 — Independent Runtime Features (parallel only after file split)

- [ ] **Apply Credentials**
  - Reuse shared resolution, auth policy, header merging, redaction, errors, timing, and TLS adaptation. If needed, extract the smallest transport-neutral helper inside the existing runtime and log it; never copy credential policy into generated text.
  - Resolve per-call, client, ordered first-nonempty env, then anonymous once per connect. Cover Bearer/custom header, API-key header, query/cookie, Basic pair, None policy, cookie merge, auth accessor, retained selection on redials, and explicit-source conflicts.
  - Reject upgrade-header overrides; preserve structured rejection status/redacted headers. Refuse all redirects without a second request. Test sentinels in URL/options/private storage Debug and Display, refused/stalled dials, Location and Set-Cookie, and a second origin receiving zero requests.
- [ ] **Implement Correlation**
  - Use only endpoint hints plus `responds_with`, with all R11 Error/Warning cases. Remove envelope-name and hard-coded field inference; renaming messages changes no behavior.
  - Emit typed request methods returning the linked response payload and retain `request_value`. Insert runtime-owned unique String/U64 IDs, never resetting across redials; only the declared response variant with the same ID completes a request.
  - Remove pending entries on serialize/send failure, timeout, cancellation, and disconnect; wake loss with `Disconnected` and never replay commands. Prove wrong-type same-ID frames remain events and capacity is reclaimed under a small pending limit.

`client.rs` currently contains both features. Before parallel execution, the integration owner assigns disjoint emitter helpers/files consistent with existing generator structure. If that would require unnecessary refactoring, run the two tasks sequentially in this wave instead.

### Wave 12 — Runtime Integration (sequential)

- [ ] **Integrate Runtime**
  - Regenerate and migrate changed request/credential call sites in the same phase. Run auth placement/precedence/rejection/redaction, request ID/type/rename/capacity, schema, consumer, and drift checks.
  - Verify initial and redial 401/403/redirect errors are terminal; retain the old receive-timeout diagnostic until Phase 6. Update auth/correlation docs and any runtime dependency documentation touched by actual changes.

**Checkpoint:** R5, R6, R7, and R11 pass through mock-observed handshakes and typed request results. No raw transport formatting leaks credential URLs; one selected credential is retained for recovery.

## Phase 6: Complete Session Setup, Recovery, and Idle Enforcement

### Wave 13 — Supervisor Implementation (sequential owner)

- [ ] **Complete Recovery**
  - Implement the proven setup contract: typed retained opening payload, credential-field binding/conflict handling, and callback on initial dial and every redial. Publish Ready/reset budget only after setup succeeds; explicit opening helpers do not replace or duplicate the retained opening payload.
  - Implement consecutive-failure counting, terminal setup/handshake rejection, shared attempt timing, and shutdown cancellation during backoff/dial/setup. Discard commands from lost connections; disconnected sends fail immediately and pending requests wake independently of event consumers.
  - Enforce inbound idle timeout after setup, resetting on every frame including ping/pong but never outbound activity. Flush automatic pongs. Bounded queue backpressure/slow writes cannot block timeout or close; terminal events precede eventual stream exhaustion.
  - Remove the unenforced-timeout TODO/Info only when the behavior is present. Regenerate and adapt opening/setup consumer calls, including callbacks required by declared challenge flows.

### Wave 14 — Recovery Proof (parallel disjoint test modules)

- [ ] **Verify Sessions**
  - Prove five successful drop/redial cycles with budget two; exactly two consecutive failed redials close; cover zero/unlimited budgets and reset after successful complete setup.
  - Observe the opening frame first on three connections; force deterministic opening-write failure. Prove callback success/redial runs before Ready, terminal rejection produces one exchange without fallback, and transient/setup-stall failures obey budget and timeout.
  - Prove manual close/drop never redials, close interrupts a stalled recovery, pre-Ready deadline expiry prevents further attempts, and a ready socket has no inherited lifetime deadline.
- [ ] **Verify Backpressure**
  - Prove inbound silence yields `ReceiveTimeout` and recovery, None preserves behavior, pings prevent expiry with pongs observed at the server, and outbound activity does not reset silence.
  - Fill the event queue and stall writes; assert state publication, request wakeup, timeout, and manual close remain responsive. Assert disconnect clears pending/queued commands and only opening/setup repeats.
  - Use server barriers and generous bounded timing tolerances; no sleeps used as synchronization, sub-100-ms timing assumptions, unbounded queues, or real devices.

### Wave 15 — Recovery Integration (sequential)

- [ ] **Integrate Recovery**
  - Run the complete schema runtime suite plus generator/definition tests and affected consumers. Regenerate if templates changed after Wave 13; run schema lint and drift checks.
  - Update lifecycle, callback, deadline, reconnect, idle, heartbeat, and shutdown rustdoc/topic/skill contracts and inspect changed comments for drift.

**Checkpoint:** R2, R3, and R8 pass; the state machine is exercised through observable frames, state/events, callback behavior, and request completion. No accepted connection is reported Ready before protocol setup.

## Phase 7: Verify Dependents and Close Review Evidence

### Wave 16 — Final Evidence (parallel disjoint work)

- [ ] **Verify Dependents**
  - Run root `just test {area}` for `homelab`, `sniff`, `biscuit-speaks`, and `model-citizen`, plus the appropriate nested integration-area recipes identified in Phase 1. Record exact package coverage so every R15 library/integration package is actually exercised.
  - Build each migrated homelab package, including `unfolded-integration-helper`; use minimal repairs only for real breaks. `homelab/cli` and `homelab/server` need no WebSocket migration but remain covered as dependent builds require.
  - Check removed definition/hub names and old primary untyped call patterns in non-generated code; exclude deliberate negative fixtures and historical specs from absence assertions.
- [ ] **Verify Environments**
  - Load `os` and gather scoped macOS, Linux, native Windows, and WSL2 build/test evidence using documented non-interactive methods. Reuse qualifying passing evidence per environment rather than rerunning it without cause.
  - Include schema's separate manifest and the named consumers in evidence coverage; ordinary workspace tests do not cover schema. Use no speculative CI runs or broadened matrix cells.
  - Record unreachable environments as **not verified**, with the reason and outstanding requirements. Full acceptance cannot be described as passing until its required coverage exists. Add any newly learned OS trap to the `os` skill.
- [ ] **Audit Documentation**
  - Finish README and `schematic/docs` topic pages with compact examples of typed/value calls, parameters, auth, setup callbacks, correlation, host hooks, caller heartbeats, and cleartext local-device transport behavior. Explain lifecycle/state flow with Mermaid where useful.
  - Update affected `schematic-define`, Schematic, `homelab`, and `unfolded-circle` skill entries and dependency pages only for actual changes. Docs never name/link the feature; unbuilt decided behavior remains marked planned.
  - Review every behavior-changed symbol's docs/comments, assuming code is correct when incidental comment drift is found; log files and resolution. Verify the input matrix is complete and tests assert public outcomes.

### Wave 17 — Acceptance Gate (sequential)

- [ ] **Record Acceptance**
  - From `schematic`, run `just test`, `just test-schema`, `just doctest`, `just lint`, and `just lint-schema`. Use nextest for unit/integration tests; the repository's doctest recipe is the documented exception. Do not run `cargo fmt`.
  - Run ignored drift explicitly through nextest: `cargo nextest run -p schematic-gen --test l1 --run-ignored only -E 'test(artifact_drift)'`. Do not use `just check-drift` while it invokes consolidated `cargo test`. Record fresh-output compile and compile-fail test results and confirm no acceptance test is silently ignored or unregistered.
  - Map R1–R17 to exact tests/manual evidence and each of ledger rows 3, 9, 10, 11, 12 to its closing test or explicitly documented accepted gap. Include every default/deviation, runtime extraction, TLS/Samsung finding, authorization, OS gap, and determinism issue in `implementation-log.md`.
  - Report **implementation complete, ready for review** only with the verified scope stated precisely. Leave the feature active and record any missing acceptance evidence as outstanding; do not move lifecycle directories or create commits/PRs.

**Checkpoint:** R15 and R17 pass, all earlier checkpoints remain green, generated drift is clean, and the review log contains a complete requirement/evidence matrix with no unreported gaps.
