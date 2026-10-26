---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-23T01:50:07-07:00"
spec: 2026-10-17-schematic-websocket/spec.md
feature: 2026-10-17-schematic-websocket/review-1.md
implemented: true
description: "A **feature** review of `2026-10-17-schematic-websocket/spec.md`"
ready: false
findings:
    - "R1-F1 (high): Challenge endpoints report Ready without completing authentication"
    - "R1-F2 (high): Typed Dock messages reject recorded replies and discard event data"
observations:
    - Opening-write failure during redial has indirect coverage
    - Verbose informational diagnostics have no positive CLI test
    - Dock setup credentials require a caller closure
    - Local-device TLS and Dock default address remain limitations
    - Some verification remains outside this review
    - Authentication hint documentation overstates automation
human_in_the_loop: null
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
recurrence: false
log: /Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-17-schematic-websocket/log.md
implemented_by: claude/opus
next: 2026-10-17-schematic-websocket/review-2.md
---

# Review outcome

The feature is **not production ready**. Two high findings affect ordinary use: a challenge-authenticated connection can announce readiness without authenticating, and the new typed Dock receiver cannot return a recorded command reply or preserve the data in a recorded IR event. Neither repair requires an author decision.

Two independent reviewers inspected the specification and implementation log for decisions and deferred work before the technical review began. Neither found a blocking design choice. Course 2 applies: adopt the defaults below and continue. This is the first implementation review; there are no previous findings to carry or close.

The review compared the implementation with the requirements and inline Design, using `e1050946d` as the implementation's preceding checkpoint. It inspected definition changes, lowering and diagnostics, generated client and host templates, shared credential handling, generated output, consumer migrations, and tests. No production code was changed. Temporary loopback probes were removed after execution.

## Decisions adopted for author review

These choices are accepted for this review unless a finding explicitly identifies a conflict with a requirement. An author can reverse an adopted default later through an ordinary change; none requires the unattended pipeline to pause. Sources below refer to sections of the specification and phases of the implementation log, rather than reopening settled author rulings.

| Decision and classification | Adopted behavior, alternatives, and reason |
| --- | --- |
| Separate definition types — decided | Keep REST and WebSocket definitions separate, sharing parameters, authentication policy, environment mapping, and headers. Full unification is explicitly excluded. |
| Core URLs — decided/defaulted | Use `ws://remote.local` plus the declared `/ws`, `/intg`, `/profiles`, and `/events` paths. Preserve a base-path prefix when joining. Fixing the base rather than stripping endpoint paths matches the documented channel model. Warn only for exact repeated boundary segments. |
| Roles and directions — defaulted | Serialize `Client`, `Host`, and `Both`; absence means `Client`. Directions are always relative to the client. Integration retains `Both`, preserving its generated host despite accurate one-way messages. AsyncAPI server imports select `Host`. This replaces inference from bidirectional messages. |
| Parameters — decided/defaulted | Use REST location containers and parameter types; float becomes `Number`. Undeclared path placeholders remain required strings, as in REST; explicit path declarations must cover them. Unsupported header/cookie parameter declarations and enum query shapes fail generation rather than disappear. Constant headers and credential cookies remain supported separately. |
| Definition JSON — defaulted | `headers` and endpoint `params` are required; new optional declarations reject explicit null, while documented absence supplies defaults. Empty declarations survive loading and are validated during generation. Errors use Serde line/column information instead of introducing a new location dependency. |
| Response links — decided/defaulted | One request names one response through `responds_with: Option<String>` and endpoint correlation hints. No name-based inference or many-to-many relation. Typed methods use `request_{request_name}`; `request_value` accepts the first frame with its matching ID regardless of selected message. This preserves an explicit untyped escape hatch. |
| Correlation ownership — decided | Allocate IDs across redials, honor string/numeric ID type, remove pending entries on every exit, wake callers on disconnect, and never replay application requests. An unused nondefault string-ID declaration warns; the stored default numeric value cannot distinguish an explicit declaration from an implicit default. |
| Selector specificity — defaulted, revising the plan | A strict superset of matching selectors wins: `{kind:event,msg:auth_required}` precedes `{kind:event}`. Identical and incomparable overlapping predicates fail. This preserves both specific and generic protocol events without declaration-order decoding, merged payload types, or a new wire tag. Selector values are strings; repeated fields and fallback-with-selectors fail. Only received directions need selection validation; a sole unselected message accepts every frame. |
| Typed enum surface — decided/defaulted | Use endpoint `ServerMessage` and `ClientMessage` enums with declared message names as variants. Require `Debug`, and generate serialization/decoding only for the role that needs it; do not add `Clone` or `Eq` requirements. Emit payload bytes alone. Inbound JSON accepts text or binary and keeps the original frame for errors; outbound framing follows the declaration. Raw and mixed framing fail with named diagnostics. |
| Explicit value operations — defaulted | Keep `send_value`, `next_value_event`, `value_events`, and `request_value`. Typed and value receive operations consume one queue. Avoid `send_raw`, whose REST meaning includes a response envelope. |
| Credential resolution — decided/defaulted | Select once per explicit connect: per-call credential on options, then API-level credential, then the first nonempty environment candidate, then anonymous. Retain the selection across redials. Shared runtime selection and placement replace generated resolver copies. An unresolved credential permits anonymous upgrade. |
| Headers and origin — decided/defaulted | Programmatic headers replace constants of the same name; reject competing explicit credential placements and transport-managed upgrade headers. Allow a caller subprotocol header. Merge credential cookies. Never follow redirects; upgrade 401, 403, and any 3xx during recovery are terminal. Other transient failures consume the reconnect budget. |
| Opening credential binding — defaulted | Add serialized `ConnectionLifecycle.open_credential_field`, explicitly binding ElevenLabs `xi-api-key`. Fill absent/null/empty payload fields from the selected credential, preserve an equal value, and reject a conflict before dialing. With no selected credential, preserve the caller's payload. Alternatives were a payload trait or provider-specific inference; explicit metadata avoids both. |
| Setup session ownership — defaulted | Write the retained opening first, run an owned typed callback second, then publish readiness. A revocable stream slot invalidates escaped sessions after completion, cancellation, timeout, or shutdown. A borrowed async callback has more difficult lifetime requirements; an ordinary shared async mutex would let escaped operations delay reclamation. Retaining the plain connect route is recorded but its readiness behavior is rejected in R1-F1. |
| Credential access during setup — defaulted | Reuse the runtime's redacted `HandshakeCredential` and explicit `expose_secret()` accessor, instead of a second WebSocket credential wrapper. Store the selected unplaced secret privately for protocols that also carry it in setup. |
| Recovery and timing — decided/defaulted | The initial connect makes one attempt. Existing `handshake_timeout` bounds upgrade, opening, and callback together; there is no newly exposed overall connect deadline. Each later attempt gets its own bound. Count consecutive failed redials, resetting only after complete setup; zero disables recovery and no maximum means unlimited recovery. Reuse `Authenticating` for setup. |
| Bounded queues and close — defaulted | A full event queue pauses reading while writes, idle timeout, state changes, pending-request failure, and close remain active. A transient error may be dropped when another event is already held. Time not reading counts toward silence. Keep bounded storage rather than add an unlimited buffer. Close-frame grace is 500 ms; close and drop terminate recovery/setup. |
| Generic host — defaulted | Name the hub from its API. An optional endpoint-specific post-upgrade hook supplies the first server message; one-endpoint hosts use shorter hook/handler names. A successful upgrade is the host hook's authentication boundary; no generated message-auth engine is added. Typed broadcast returns a result and count; value broadcast retains a count. Integration handlers supply the prior greeting as a typed response with unchanged wire bytes. |
| Diagnostics — defaulted | Reuse the shared writer/validation format with feature name `websocket-definition`, retaining every lowering diagnostic. Info notes are separate and verbose-only. Every Rust generation run validates the shipped WebSocket modules it writes, even when the requested API is REST. No independent WebSocket write policy is introduced. |
| Import compatibility — defaulted | Derive selectors from existing top-level string constants through references/compositions, with bounded traversal. This keeps multi-message imports usable after selector validation. Do not extend the AsyncAPI exporter here. |
| Payload registration and shared helpers — allowed deferral | Keep WebSocket name-based `Schema` resolution because prerequisite typed registration did not extend to message schemas. ElevenLabs payloads remain ordinary Serde types. Leave `ws_shared` generated and retain the partial-generation byte comparison; moving all helpers into runtime was a candidate, not a requirement. |
| Heartbeats and Samsung correlation — decided/defaulted | Application heartbeats remain caller-managed and documented, with metadata retained and no warning. Do not invent heartbeat intervals for shipped definitions. Leave Samsung uncorrelated because repository protocol notes do not establish response-ID matching; remove its dead explicit ID declaration. |
| Testing architecture — defaulted | Use loopback L1 tests and the existing `tokio-tungstenite` helper, shared into generator test support without a generated-crate dependency cycle or a new third-party crate. Fresh-output child crates prove compilation and behavior. Keep their `slow_` tests selected by the live fresh-output recipe. Existing schema environment isolation is retained instead of expanding the generated dependency manifest. |
| Allowed boundaries and deferrals | AsyncAPI export, including selectors/roles/heartbeat metadata, belongs to the following work. No live device/provider tests, new cross-origin policy, automatic application heartbeat, speculative TLS verifier, or transport redesign. TLS and Dock setup-token limitations are recorded below. Deferring the recorded Dock decoding defect is not accepted because typed decoding is required here; see R1-F2. |

All pre-authorizations are treated as confirmed. The review performed no commit, push, CI change/run, real provider/device call, stale-output deletion, or lifecycle move. Incomplete OS evidence is recorded rather than used to block readiness.

## Findings

### R1-F1 (high): Challenge endpoints report Ready without completing authentication

**Authority — spec:** Requirement R3 says an endpoint with a challenge flow takes a typed setup callback on every dial before reporting `Ready`, and explicitly defines readiness as “the protocol session is usable, not only that the upgrade completed.” The Design state flow repeats that completing the upgrade does not prove application readiness.

**Defect class:** An optional setup path allows a required protocol prerequisite to be skipped while publishing the state that promises it completed.

In `schematic-gen`, [the plain connect emitter](../../gen/src/ws_codegen/client.rs:720) always passes `None` for setup, including endpoints with `AuthFlowHints`. The shared [connection entry point](../../gen/src/ws_codegen/supervisor.rs:440) then publishes `Ready`. Recovery uses the same absent callback, so the bypass persists after reconnects. A resolved handshake key does not prove the server accepted it: the declared protocols can still send `auth_required`; Dock has no handshake credential at all.

Reproduction used the existing loopback helper and the challenge frames already used by `schema/tests/ws_runtime.rs`. For each affected endpoint, the server sent its challenge and waited for an answer. Plain connect returned successfully, `next_value_event()` delivered the challenge, the server had received no application frame, and `state()` was `Ready`. As a control, the corresponding setup method read the same challenge and returned `WsSetupError::Rejected`; every connect returned `SetupRejected` rather than a ready client.

| Child ID | Site in generated `schematic-schema` | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Core `connect_core_ws` | Core `auth_required`, no callback | Ready, unanswered challenge | Required setup before Ready |
| R1-F1.2 | Core `connect_core_integrations` | Same Core challenge | Ready, unanswered challenge | Same |
| R1-F1.3 | Core `connect_core_profiles` | Same Core challenge | Ready, unanswered challenge | Same |
| R1-F1.4 | Core `connect_core_events` | Same Core challenge | Ready, unanswered challenge | Same |
| R1-F1.5 | Dock `connect_dock_root` | Recorded Dock `type:auth_required` | Ready, unanswered challenge | Same |
| R1-F1.6 | Integration `connect_integration` | Recorded Integration challenge | Ready, unanswered challenge | Same |
| R1-F1.7 | All six corresponding `_with_setup` methods | Same challenge, callback rejects | Clean: `SetupRejected` on every endpoint | No Ready client on rejection |
| R1-F1.8 | Shared attempt and recovery | Setup-present initial/redial success, rejection, transient failure, stall | Clean: existing `ws_session` tests verify callback ordering and terminal rejection | Ready only after callback success; transient failures use budget |
| R1-F1.9 | ElevenLabs and Samsung connect methods | No declared challenge callback | Clean by declaration: neither API declares `AuthFlowHints`; ElevenLabs opening is separately enforced | No challenge prerequisite to enforce |
| R1-F1.10 | State consumers: transport send and typed/value request methods | Ready from one of the six bypasses | `WsTransportHandle::send` gates only on Ready and the connection number, so unauthenticated application traffic is permitted | The readiness gate must imply completion of required setup |

The final row follows readiness into the decisions that accept outgoing messages; it is not merely a misleading status label. The existing connection-number guard correctly prevents sending queued commands on a different connection, but cannot compensate for an incorrect Ready state.

Require setup for endpoints declaring the challenge flow, or provide an explicit API contract for bypassing it that does not falsely promise protocol readiness. Requiring the callback is the smaller change consistent with this spec; a caller that already established authentication can supply an explicit successful callback. Regenerate output and adjust convenience examples/tests to use the intended route. Do not add automatic protocol inference.

### R1-F2 (high): Typed Dock messages reject recorded replies and discard event data

**Authority — spec and regression:** Requirement R10 requires decoding every inbound message type of all five definitions from recorded fixture frames, asserting the selected variant. The new typed primary receiver also regresses a realistic frame that the preceding implementation returned intact: at checkpoint `e1050946d`, `next_event()` and `events()` returned the parsed JSON value. The Dock examples below are present in `schematic/specs/unfolded-circle/UCD2-asyncapi.yaml`, not invented adversarial inputs.

**Defect class:** A new typed projection is wired to payload models that do not represent the protocol's recorded envelope or retain its application data.

In `schematic-definitions`, [the Dock envelope types](../../definitions/src/unfolded_circle/dock_ws/types.rs:75) do not accept `type:"dock"` and only retain a nested `msg_data` payload. The actual system-information response uses `type:"dock"` and top-level result fields; an IR event carries top-level `ir_code`. In `schematic-schema`, [the new Dock decode test](../../schema/tests/ws_runtime.rs:613) changes the recorded response to `type:"resp"` before sending it and checks only the event name for the IR example. Those assertions conceal both failures.

Reproduction sent the unmodified `systemInfoMsg` example through the loopback server. Both `next_event()` and `events()` returned `WsError::Decode`: `selected message ResponseEnvelope did not decode: unknown variant dock`. A correlated typed request received the same error when the server echoed its allocated ID. Sending the recorded `irReceiveEvent` selected `EventEnvelope`, but the returned payload had no `ir_code` field and `msg_data` was absent; the actual received code could no longer be read. The value receiver returned both original frames intact.

The sweep checked all Dock message families and their downstream projections. It also inspected the existing all-variant loopback tests for the other four APIs, which passed during this review; no second recorded-envelope mismatch was identified there. This does not claim live-provider fidelity beyond the repository fixtures.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F2.1 | Dock selector and `ResponseEnvelope` decoder, through `next_event()` | Unedited recorded `systemInfoMsg`, `type:dock`, `req_id:2` | Fallback selects the response; payload decode rejects `dock` | Return the response variant with accessible result data |
| R1-F2.2 | Dock typed `events()` | Same recorded system-information response | Same decode error | Same result as typed single-event receive |
| R1-F2.3 | Dock typed `request_request_envelope` | Same response with only `req_id` changed to the allocated ID | Correlation accepts the ID/selected response, then typed payload decode fails | Return its declared response payload with result data |
| R1-F2.4 | Dock `EventEnvelope` through typed receive | Recorded `type:event,msg:ir_receive,ir_code:"4;0x10;12;0"` | Variant selected, actual IR code discarded | Retain the code for application use |
| R1-F2.5 | Dock `AuthRequired` | Recorded `type:auth_required` challenge | Clean: variant and model/revision/version preserved in existing loopback test | Challenge fields available |
| R1-F2.6 | Dock `Authentication` | `type:auth,msg:authentication,code:200` | Clean: variant and outcome preserved in existing loopback test | Authentication outcome available |
| R1-F2.7 | Dock typed `Auth` output | `type:auth,token` model and its serialized shape | Clean by the payload struct's explicit wire names, matching the checked-in auth request example | Emit type and token without a wrapper |
| R1-F2.8 | Dock typed command request output | `get_sysinfo` request through new typed request method | Wire frame has `type:req,msg:get_sysinfo`; recorded command schema instead uses `type:dock,command` | Represent the protocol command in the typed surface |
| R1-F2.9 | Dock `next_value_event()` | Unmodified response and IR event | Clean: exact JSON values preserved | Explicit escape hatch preserves data |
| R1-F2.10 | Core, Integration, ElevenLabs, Samsung typed receivers | Existing per-endpoint all-variant loopback frames | Clean for the tested fixture sets; all tests passed | Select and decode the declared variants |

The old command/envelope models predate this feature. Their newly generated typed send/request/receive routes now make those limitations part of the primary API; the finding is bounded to making those routes work with the recorded messages required by R10. It is not a request to remodel every Dock operation. Preserve top-level application data in the general envelope or use appropriate concrete payloads, align command/response discriminators and command field names, and test the recorded frames unchanged. Assert payload content as well as variant names. A generic retained data map/raw payload is a reasonable smaller repair than a large catalog of new command types.

## Observations

**Opening-write failure during redial has indirect coverage.** The implementation log correctly admits that no test makes the initial opening succeed and a later opening write fail. The initial write-error test does exercise the shipped error mapping, while every redial calls that same `run_attempt`; transient-setup and refused-redial tests exercise the common budget branch. This is a coverage limitation, but the code invariant and existing branch checks do not justify demanding a new transport-injection layer solely for this combination. Preserve the limitation in the acceptance record instead of calling it directly tested.

**Verbose informational diagnostics have no positive CLI test.** No shipped definition now emits an Info note, so the spawned CLI test proves only quiet behavior. Fixture-based in-process tests exercise the Info mapping, and the shared pipeline prints notes under verbosity. The feature does not introduce a new CLI verbosity mechanism. The lost positive CLI assertion is recorded, without requiring a synthetic production catalog entry.

**Dock setup credentials require a caller closure.** The Dock handshake declares no authentication, so shared selection is anonymous and the setup session has no token to expose. Callers supply their message-auth token from a closure. The generated module documentation still lists `UCR_DOCK_TOKEN`, which can be read as more automatic than the actual behavior. A separate setup-only credential resolver is new scope; clarify this limitation rather than adding one in the repair cycle.

**Local-device TLS and Dock default address remain limitations.** The current runtime invalid-certificate option configures reqwest and cannot simply configure the WebSocket TLS connector. No new verifier or WebSocket-only switch was added, as permitted by the spec. Samsung self-signed TLS remains unverified against a device. The Dock default URL still lacks the checked-in protocol's port 946. Its existing address defect is outside the Core-only URL repair. Cleartext `ws://` credentials are documented; no unsolicited warning was added.

**Some verification remains outside this review.** The log identifies incomplete dependent/runtime results on Windows/WSL2 and incomplete Linux dependent coverage. OS proof is explicitly excluded from this review's readiness decision. This review reran the scoped local checks below, not every dependent test or every lint/doc gate. The schema's existing slow compile-fail test took 7.735 seconds here; it is a Cargo-compilation test explicitly named for the fresh/slow resource, not a newly slow WebSocket network test.

**Authentication hint documentation overstates automation.** In `schematic-define`, the `AuthFlowHints` type documentation still says the generator produces challenge/response handling in the state machine. The implemented contract is caller-provided setup, as the updated topic documentation explains. This old description should be narrowed when that documentation is next edited; it is not an additional behavior requirement.

## Verification and requirement coverage

All networking below is L1: a local server and generated public Rust API are sufficient to exercise these contracts. There is no terminal rendering, keyboard, browser, or focus-dependent requirement in this feature, so no L2/L3 mismatch applies.

| Requirements | Strongest relevant evidence inspected or rerun | Assessment |
| --- | --- | --- |
| Correct URLs; explicit shared vocabulary and normalized definitions | Definition conformance/JSON tests; lowering URL tests; generated-client path/query loopback tests | L1; passed for the required Core repair; preexisting Dock address noted above |
| Reconnect budget, replay, setup, timeout, cancellation and close | Generated `ws_session`/`ws_idle` tests plus fresh-output session child crate | L1 through real loopback transport; callback route passes; plain route fails R1-F1; redial opening-write combination indirect |
| Bearer host compilation and protocol-neutral host behavior | Fresh-output Bearer host and multi-endpoint host child crates, including missing/wrong/matching credential checks and hooks | L1 compilation and loopback; passed |
| Credentials, anonymous upgrade, placement, redaction, no redirects | Runtime handshake tests; generated `ws_handshake`; fresh-output credentials fixture | L1 public results and wire observations; passed |
| Diagnostics | Lowering/shared writer tests, Info in-process test, quiet spawned CLI check | L1; passed, positive verbose wiring limitation recorded |
| Typed messages and declared correlation | All-variant schema tests, correlation loopback tests, fresh-output typed compilation/misuse fixtures | L1; existing tests pass, but recorded Dock data exposes R1-F2 |
| Dependents, docs, skills and generated output | Consumer edits and implementation-log gate record; local drift check; topic/skill inspection | No independent all-dependent rerun; drift passed; no evidence of hand-edited generated output |
| Focus-safe harness | Harness L1 tests, port-0 loopback scripts, deterministic shutdown | Passed; no windows opened |

Executed locally on this review tree:

- `just test-schema`: **149 passed**, including WebSocket runtime/handshake/correlation/session/idle and mock-harness binaries.
- `cargo nextest run -p schematic-gen -p schematic-define -p schematic-definitions -p schematic-runtime -E 'test(ws_) | test(websocket) | test(handshake_credentials)'`: **202 passed**, including all five WebSocket fresh-output guards; unrelated tests filtered out.
- `cargo nextest run -p schematic-definitions --test ws_definition_json --test ws_conformance`: **17 passed**.
- `just check-drift`: **9 passed**, including generated Rust consistency and fresh compilation.
- `just check-tier-coverage schematic`: **0 stranded tests**.
- Temporary `schematic-schema` public-API probe binary: **10 passed**, asserting the two reported failure classes and their controls. These are successful reproductions of defects, not ten additional acceptance tests. The file was removed after use.

The input robustness matrix applies to definition JSON, not to WebSocket message payloads as configuration. The JSON tests exercise a real shipped definition with one field edited: absent, explicit null, wrong whole-field/element types, empty values, duplicate keys, invalid/trailing documents, and the opening-credential binding. The review also searched the changed readers for `serde(default)`, `Option`, `filter_map`, `unwrap_or_default`, and `.ok()`; the documented defaults and transport/environment parsing explain the relevant uses. No additional configuration-shape finding was established.

Only the two titled high findings block readiness. No finding is blocked on human review. The specification's review iteration is now 1; it has not been marked completed.
