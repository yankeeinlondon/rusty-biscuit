---
$schema: feature-review.yaml
ready: true
findings: []
observations:
    - Dock protocol skill contains an older wire format
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-23T02:34:42-07:00"
spec: 2026-10-17-schematic-websocket/spec.md
implemented: true
description: "A **feature** review of `2026-10-17-schematic-websocket/spec.md`"
feature: 2026-10-17-schematic-websocket/review-2.md
previous: 2026-10-17-schematic-websocket/review-1.md
implemented_by: claude/opus
---

# Review outcome

The feature is **production ready within the specification's scope**. Both previous high findings are verified fixed, including every child instance in the repair's Finding Ledger. There are no new findings, partial repairs, remaining defects carried from review 1, or blocked findings.

This review read the specification's inline Design, review 1, the original implementation log, and the repair log before checking the implementation. The accepted decisions in review 1 remain accepted: challenge authentication belongs to an explicit caller callback, message selection uses declared wire fields, application heartbeats remain caller-managed, and live-device testing and transport redesign remain outside this work. No new design choice requires human approval.

## Previous Finding Verification

The required-instance list comes from `log.md` under “Finding Ledger,” including the six instances discovered during repair. “Verified fixed” for a previously clean child means its required behavior remains verified; it does not imply that child needed a change. Links below identify implementation and test evidence, rather than relying on the ledger's completion claims.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1 — Challenge endpoints report Ready without completing authentication | R1-F1.1–R1-F1.14, individually checked below | verified fixed | The generator emits only setup-taking connects for challenge endpoints; all six generated private open methods require a setup factory. Initial connect and recovery await that factory before Ready. Loopback and fresh-output checks passed. | Nothing |
| R1-F1.1 — Core main connect | `connect_core_ws` bypass | verified fixed | [Generated Core client](../../schema/src/unfolded_circle_core_ws.rs) exposes only `connect_core_ws_with_setup`; `open_core_ws` requires setup and stores `Some(setup)`. `challenge_connect_is_not_ready_until_setup_completes` keeps connect pending until the peer permits authentication to finish. | Nothing |
| R1-F1.2 — Core integrations connect | `connect_core_integrations` bypass | verified fixed | Same generated module: only `connect_core_integrations_with_setup`; its private opener requires setup and stores `Some(setup)`. No plain connect remains. | Nothing |
| R1-F1.3 — Core profiles connect | `connect_core_profiles` bypass | verified fixed | Same generated module: only `connect_core_profiles_with_setup`; its private opener requires setup and stores `Some(setup)`. No plain connect remains. | Nothing |
| R1-F1.4 — Core events connect | `connect_core_events` bypass | verified fixed | Same generated module: only `connect_core_events_with_setup`; its private opener requires setup and stores `Some(setup)`. No plain connect remains. | Nothing |
| R1-F1.5 — Dock connect | `connect_dock_root` bypass | verified fixed | [Generated Dock client](../../schema/src/unfolded_circle_dock_ws.rs) exposes only `connect_dock_root_with_setup`; its private opener requires setup and stores `Some(setup)`. | Nothing |
| R1-F1.6 — Integration connect | `connect_integration` bypass | verified fixed | [Generated Integration client](../../schema/src/unfolded_circle_integration_ws.rs) exposes only `connect_integration_with_setup`; its private opener requires setup and stores `Some(setup)`. | Nothing |
| R1-F1.7 — Six setup-taking methods reject failed setup | All four Core methods, Dock, Integration | verified fixed | Checked each generated opener's required callback and shared connection entry point. In [the supervisor template](../../gen/src/ws_codegen/supervisor.rs), `attempt` maps callback rejection to `SetupRejected` before returning a stream; no endpoint-specific readiness branch exists. The Core loopback rejection test and fresh-output rejection case passed. | Nothing |
| R1-F1.8 — Shared initial attempt and recovery | Success, rejection, transient failure, stall | verified fixed | [Session tests](../../schema/tests/ws_session.rs) passed for setup on every connection, terminal initial/redial rejection, transient failures consuming the budget, timeout/cancellation, and closing during stalled redial setup. Both Ready transitions follow successful `run_attempt`. | Nothing |
| R1-F1.9 — Non-challenge connects | Both ElevenLabs endpoints and both Samsung endpoints | verified fixed | Inspected all four generated connects: plain methods remain, with `setup: None`. `plain_connect_is_kept_only_without_a_challenge_flow` and `only_challenge_flow_endpoints_get_a_setup_connect` passed. ElevenLabs opening replay tests passed. | Nothing |
| R1-F1.10 — Ready consumers | Transport send and typed/value correlated requests | verified fixed | The transport's Ready/connection-number gate remains intact. Each challenge client now reaches that gate only after required setup returns successfully. Tests for disconnected sends, pending-request failure, and queued commands not crossing a redial passed. | Nothing |
| R1-F1.11 — Fresh-output challenge fixture | Two child tests previously using plain connect | verified fixed | [Fresh-output tests](../../gen/tests/l1/fresh_output_compile.rs) use `connect_feed_with_setup`; the fixture assertion excludes `connect_feed(`. `slow_ws_session_setup_and_recovery_run` passed, compiling and executing the generated setup/recovery surface. | Nothing |
| R1-F1.12 — Host emitter | Any host path publishing client Ready | verified fixed | [Host emitter](../../gen/src/ws_codegen/host.rs) accepts server connections; it has no client connect/Ready path. Its typed host behavior is exercised by the passing fresh-output host checks. | Nothing |
| R1-F1.13 — Repository consumers | Callers of the six removed plain connects | verified fixed | Text search across schematic, homelab, sniff, biscuit-speaks, and model-citizen found no remaining calls to those exact plain methods. Current schema callers compile and run through setup-taking methods; generated-output drift checks passed. No downstream production caller of a removed name was identified. | Nothing |
| R1-F1.14 — Authentication hint documentation | Claim that hints automate challenge handling | verified fixed | [AuthFlowHints documentation](../../define/src/websocket/hints.rs) describes the caller-supplied setup contract. Generated method docs and the WebSocket topic page now require the callback. | Nothing |
| R1-F2 — Typed Dock messages reject recorded replies and discard event data | R1-F2.1–R1-F2.12, individually checked below | verified fixed | Recorded Dock frames now decode unchanged, top-level data remains accessible, and command output matches the recorded shape. The repair also preserves Core/Integration challenge payloads and removes an impossible Dock auth correlation. | Nothing |
| R1-F2.1 — Dock single-event reply | Unmodified `systemInfoMsg`, `type:dock` | verified fixed | [Runtime test](../../schema/tests/ws_runtime.rs) `dock_root_decodes_every_server_message_variant` sends the recorded reply unchanged; asserts the response variant, request ID, status, name, model, version, and IR-learning flag. Passed. | Nothing |
| R1-F2.2 — Dock typed event stream | Same recorded reply through `events()` | verified fixed | `dock_root_value_and_typed_streams_keep_recorded_frames` checks the reply's typed result fields through `events()`. Passed. Both receive methods use the same decoder. | Nothing |
| R1-F2.3 — Dock correlated typed reply | Recorded reply with only allocated `req_id` substituted | verified fixed | [Correlation test](../../schema/tests/ws_correlation.rs) `dock_get_sysinfo_request_uses_the_recorded_wire_shape` returns typed response data and checks the exact outgoing command. Passed. | Nothing |
| R1-F2.4 — Dock IR event data | Recorded `ir_receive` through typed receive and stream | verified fixed | Both Dock runtime tests assert `data["ir_code"]`; the [payload unit test](../../definitions/src/unfolded_circle/dock_ws/types.rs) also round-trips the frame and reads it through `parse_data`. Passed. | Nothing |
| R1-F2.5 — Dock challenge | Recorded `auth_required` | verified fixed | All-variant runtime test checks model, revision, version, and discriminator; the recorded authentication round-trip unit test passed. | Nothing |
| R1-F2.6 — Dock authentication outcome | Authentication success and failure | verified fixed | Runtime test retains success code 200; payload unit test round-trips code 401 without modifying the protocol frame. Passed. | Nothing |
| R1-F2.7 — Dock auth output | `type:auth,token` | verified fixed | `recorded_authentication_frames_round_trip` checks exact serialized request JSON. Generated outbound enum serialization writes its payload directly. | Nothing |
| R1-F2.8 — Dock command output | `type:dock,id,command`, top-level parameters | verified fixed | Correlation test asserts the exact allocated-ID `get_sysinfo` request. `recorded_requests_round_trip_with_top_level_params` checks both system-information and IR-send requests. Passed. | Nothing |
| R1-F2.9 — Dock value receiver | Original reply and event JSON | verified fixed | `dock_root_value_and_typed_streams_keep_recorded_frames` compares both returned values with the original frames before testing typed stream delivery. Passed. | Nothing |
| R1-F2.10 — Other four APIs | Core, Integration, both ElevenLabs endpoints, both Samsung endpoints | verified fixed | All six corresponding all-variant runtime tests passed. Inspected payload assertions, including Core event/result data, Integration driver data, ElevenLabs response fields, and Samsung channel data. | Nothing |
| R1-F2.11 — Core and Integration challenge data | Both `AuthRequired` projections | verified fixed | Both payload structs now retain `msg_data: Option<Box<RawValue>>`. Both `auth_required_retains_msg_data` unit tests and the runtime challenge-payload assertions passed. | Nothing |
| R1-F2.12 — Uncorrelatable Dock auth request | Auth declaration and generated `request_auth` | verified fixed | [Dock definition](../../definitions/src/unfolded_circle/dock_ws/mod.rs) no longer declares an auth response link. The generated client has no `request_auth`; the command-response link and correlation remain. Definition conformance and drift checks passed. | Nothing |

The repair's additional instances are resolved extensions of the previous findings' sweeps, not newly outstanding siblings. No previous finding was blocked, and no prerequisite or accepted decision needed to be reopened.

## Unblocked Findings

None.

## Blocked Findings

None.

## Requirement verification and test levels

All behavior added by this specification is library, generation, or loopback-network behavior. Level 1 is appropriate; no requirement here needs terminal rendering or operating-system keyboard input. Test counts below describe executed checks, not substitutes for requirement evidence.

| Requirement | Strongest relevant verification checked | Level and assessment |
| --- | --- | --- |
| Correct URLs and parameter encoding | Definition/conformance and lowering tests; generated clients dial all Core channel paths; fresh-output parameter tests inspect received path/query values. | L1; appropriate |
| Repeated recovery and consecutive-failure budget | Loopback tests cover repeated successful redials, two refused redials, reset after success, zero attempts, unlimited attempts, and no reconnect policy. | L1; appropriate |
| Opening replay and challenge setup before Ready | Loopback tests inspect first opening frames across connections, credential binding/conflicts, blocked setup, rejection, cancellation, and recovery; fresh output compiles and executes the same contract. | L1; appropriate |
| Bearer host compilation and token handling | `slow_bearer_ws_host_compiles_and_checks_tokens` builds generated output and exercises valid/missing/wrong credentials. | L1 fresh-output check; appropriate |
| Credential precedence, placement, anonymous operation, redirects, and redaction | Shipped-client handshake tests and `slow_ws_handshake_credentials_and_correlated_requests_run` check actual loopback upgrade requests and rejection behavior, including terminal redial 401/403/redirects. | L1; appropriate |
| Idle timeout and control frames | Loopback tests cover silence, disabled timeout, inbound ping/pong activity, outbound activity not resetting the timer, full queues, stalled writes, and prompt close. | L1; appropriate |
| Diagnostics | `ws_validation` tests inspect shared severities and writer behavior; lowering tests cover invalid links, selectors, roles, parameters, framing, and exact duplicate boundary segments. | L1; appropriate |
| Typed messages, selectors, error frames, and shared receive queue | All-variant tests for all five definitions, exact Dock payload checks, malformed text/binary cases, overlap/fallback cases, compile-fail misuse checks, and fresh-output typed client/host execution. | L1; appropriate |
| Explicit response links and ID types | Numeric-ID shipped-client tests; string-ID fresh-output tests; wrong selected response remains an event; timeout/cancellation/disconnect clean up pending entries. | L1; appropriate |
| Shared vocabulary, normalized definitions, and serialized roles | `ws_conformance`, definition JSON matrix, shared-type checks, and compile-fail checks for removed vocabulary. | L1; appropriate |
| Generic host and post-authentication hook | Fresh-output multi-endpoint host tests check custom/absent hook messages, rejection, hook write failure, typed dispatch, and broadcast routing. | L1; appropriate |
| Consumers, harness, and generated artifacts | Current generated schema compiles/tests; removed-method search is clean; loopback harness tests pass; Rust/OpenAPI/Postman artifact-drift tests pass; generated-schema lint passes. Earlier dependent-build evidence is not represented as a new full dependent test run. | L1 plus static inspection; no newly introduced consumer break found |

The earlier observation that opening-write failure specifically during redial has indirect coverage remains a verification limit, not a new finding: the initial opening failure is exercised directly, and redials use the identical attempt function and tested failure-budget path. No stronger device-level guarantee is inferred from these tests.

## Input robustness matrix

The relevant configuration input is the public serialized `WebSocketApi` definition. Payload frames are protocol data, not configuration. The repair adds no new definition-file loader; it changes existing payload models and the generated connect surface.

Checked and reran `definitions/tests/ws_definition_json.rs`: the positive Dock control loads; absence uses explicitly documented defaults or is rejected for required fields; explicit null and wrong types are rejected for the new decision-bearing declarations; mixed/all-invalid collection elements are rejected; explicit empty declarations are retained for generation validation; duplicate keys, trailing garbage, concatenated documents, and truncated JSON fail. All five definitions round-trip without loss. The generator tests then check the declarations whose empty values are semantically invalid.

The source scan for `serde(default)`, `Option`, `filter_map`, `unwrap_or_default`, and `.ok()` found no additional defect in the changed definition validators. Selector absence is deliberately distinct from an explicit empty selector list, and the new optional declaration deserializers reject null. Dock's optional response ID/status support the recorded pong, which carries neither; those are payload fields rather than configuration defaults. Existing AsyncAPI document-loader permissiveness was not expanded into a new configuration-hardening requirement for this review.

## Recurrence

No finding recurs. Both previous defect classes are repaired across their ledger instances; no unchanged instance, repair regression, or newly outstanding sibling was established.

## Observations

**Dock protocol skill contains an older wire format.** The existing [Dock protocol reference](../../../.claude/skills/unfolded-circle/dock-api.md:186) still illustrates `kind`, nested `payload`, and `get_status`, whereas the recorded UCD2 examples and repaired definitions use `type`, top-level fields, and `get_sysinfo`. The repair log already records this older reference as deferred. This is preexisting reference content, not a remaining runtime defect introduced by this specification. A separately scoped reference update would reduce the chance of reintroducing the old model.

## Executed checks and limits

On the current macOS host:

- `just test` in `schematic`: 2,770 regular tests passed, 20 skipped; all 14 fresh-output checks passed; the runtime compile-fail check passed. The artifact-drift checks ran in this command and passed.
- `just test-schema`: all 152 tests passed, none skipped, including every WebSocket runtime, setup, correlation, handshake, idle-timeout, and mock-harness test.
- `just lint-schema`: passed with warnings denied.
- `just check-tier-coverage schematic`: zero stranded tests. The generator's consolidated L1 target declares the changed test modules; its fresh-output recipe explicitly runs the slow compilation checks. Schema test files are automatically discovered and appeared in the executed run.

The 20 skips in the area run are not counted as proof; the WebSocket behavior relied upon above ran through schema tests and explicit fresh-output checks. This review did not repeat cross-OS or live-provider testing, run full dependent-area suites, or run terminal tests unrelated to this specification. Cross-OS evidence remains CI/CD work as requested.

Only review metadata and this report were edited by this review. Review 1's implementation claims were preserved; its next-review link was added. The specification now records iteration 2 and `completed: true`. No commit, push, CI change, provider call, or lifecycle move was performed.
