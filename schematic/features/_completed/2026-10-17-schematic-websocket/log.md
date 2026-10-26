---
implementation_1: "2026-10-23T01:58:27-07:00"
---

# Implementation Log: Schematic WebSocket

## Implementation of Review Findings #1

> **started at:** 2026-10-23T01:58:27-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-17-schematic-websocket/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review `implemented` flag confirmed `false` before repairs began

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| -------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1    | Challenge endpoints report Ready without completing authentication | fixed | all required children fixed or clean; `just test` 2770+14+1 passed |
| R1-F1.1  | Core `connect_core_ws`: setup required before Ready | fixed | `connect_core_ws` removed; `ws_session::challenge_connect_is_not_ready_until_setup_completes` and `ws_codegen::only_challenge_flow_endpoints_get_a_setup_connect` pass |
| R1-F1.2  | Core `connect_core_integrations`: setup required before Ready | fixed | removed by the same generator branch; test caller moved to `_with_setup`; generated output checked by `just check-drift` |
| R1-F1.3  | Core `connect_core_profiles`: setup required before Ready | fixed | same as R1-F1.2 |
| R1-F1.4  | Core `connect_core_events`: setup required before Ready | fixed | same as R1-F1.2 |
| R1-F1.5  | Dock `connect_dock_root`: setup required before Ready | fixed | `connect_dock_root` removed; Dock decode tests use `connect_dock_root_with_setup` with `\|_\| async { Ok(()) }` |
| R1-F1.6  | Integration `connect_integration`: setup required before Ready | fixed | `connect_integration` removed from generated output |
| R1-F1.7  | All six `_with_setup` methods: rejection yields `SetupRejected` | clean | `setup_rejection_on_connect_is_returned_without_a_second_attempt` and fresh-output rejection test pass |
| R1-F1.8  | Shared attempt/recovery: Ready only after callback success | clean | supervisor retains the one `WsSession`; every redial reuses `session.setup`; `setup_runs_before_ready_on_every_connection` passes |
| R1-F1.9  | ElevenLabs and Samsung connects: no challenge prerequisite | clean | no `AuthFlowHints`; plain connect kept, dead setup parameter removed; `plain_connect_is_kept_only_without_a_challenge_flow` asserts both |
| R1-F1.10 | Ready consumers (transport send, typed/value request) imply setup completion | fixed | `send` gate unchanged; Ready is now reachable on a challenge endpoint only after callback success (private open method requires `SetupFactory`) |
| R1-F2    | Typed Dock messages reject recorded replies and discard event data | fixed | all required children fixed or clean; `just test-schema` 152 passed; full `just test` passed |
| R1-F2.1  | Dock `ResponseEnvelope` via `next_event()` on unedited `systemInfoMsg` | fixed | `ws_runtime::dock_root_decodes_every_server_message_variant` sends verbatim `systemInfoMsg`; asserts `req_id`, code, name, model, version |
| R1-F2.2  | Dock typed `events()` on same response | fixed | `ws_runtime::dock_root_value_and_typed_streams_keep_recorded_frames` asserts the same via `events()` |
| R1-F2.3  | Dock typed `request_request_envelope` returns result data | fixed | `ws_correlation::dock_get_sysinfo_request_uses_the_recorded_wire_shape` returns typed result data |
| R1-F2.4  | Dock `EventEnvelope` retains top-level `ir_code` | fixed | both Dock runtime tests assert `data["ir_code"] == "4;0x10;12;0"`; unit test via `parse_data` |
| R1-F2.5  | Dock `AuthRequired` challenge fields preserved | clean | verbatim challenge asserted equal to full struct (model, revision, version) |
| R1-F2.6  | Dock `Authentication` outcome preserved | clean | `Authentication` code 200 asserted; unit test round-trips 401 |
| R1-F2.7  | Dock typed `Auth` output wire shape | clean | `recorded_authentication_frames_round_trip` asserts `{"type":"auth","token":"1234"}` |
| R1-F2.8  | Dock typed command request output uses recorded `type:dock,command` shape | fixed | correlation test asserts received frame `{"type":"dock","id":1,"command":"get_sysinfo"}` |
| R1-F2.9  | Dock `next_value_event()` preserves exact JSON | clean | now covered by test: `next_value_event()` returns both recorded frames exactly |
| R1-F2.10 | Core, Integration, ElevenLabs, Samsung typed receivers | fixed | no test edits a recorded frame; payload assertions added for Core, Integration, and Samsung all-variant tests (previously names only) |
| R1-F1.11 | Fresh-output `session_ws_fixture` (declares `auth_flow`) child tests used the plain connect | fixed | both moved to `connect_feed_with_setup`; `slow_ws_session_setup_and_recovery_run` asserts no `connect_feed(` |
| R1-F1.12 | Host emitter: any connect path reaching Ready | clean | `host.rs` has no connect path; only `has_setup: false` in a test fixture |
| R1-F1.13 | Repo consumers of the six removed methods | clean | repo-wide `rg` found only schematic tests and docs; all reverse dependents compile (`cargo check --all-targets`) |
| R1-F1.14 | `AuthFlowHints` rustdoc claimed generated challenge handling | fixed | `define/src/websocket/hints.rs` now states the caller-supplied setup contract |
| R1-F2.11 | Core and Integration `AuthRequired` dropped recorded `msg_data` | fixed | `msg_data: Option<Box<RawValue>>` added; `auth_required_retains_msg_data` unit tests in both |
| R1-F2.12 | Dock `Auth.responds_with` generated an uncorrelatable `request_auth` | fixed | link removed (the auth reply carries no `req_id`); regenerated; conformance tests pass |

Observations (nonblocking; carried by the follow-up pass, not repaired here unless a repair touches them): opening-write redial coverage, verbose Info CLI test, Dock setup credential closure, TLS/Dock default address, verification outside review, `AuthFlowHints` doc wording.

### Progress

- starting the work on 'R1-F1 Challenge endpoints report Ready without completing authentication' at 01:59:02
        - defect class: an optional setup path allows a required protocol prerequisite to be skipped while publishing the state that promises it completed
        - swept: every generator connect emitter, private open method, supervisor recovery path, host emitter, fresh-output fixtures, and every repo consumer of the six plain connect names
        - completed: an endpoint with `AuthFlowHints` now gets only `connect_{x}_with_setup`; its private open method requires a `SetupFactory`; endpoints without a challenge flow (ElevenLabs, Samsung) keep the plain connect and lose the dead setup parameter
        - discovered: fresh-output `session_ws_fixture` declares `auth_flow` but two child tests used the plain connect (new row R1-F1.11); fixed
        - discovered: `AuthFlowHints` rustdoc claimed generated challenge handling (review observation); corrected in the same change (R1-F1.14)
        - homelab and other crates do not call the removed methods; only schematic schema tests and `schematic/docs/websocket-definitions.md` did; about 35 transport-only test call sites now pass the shared `ws_mock::no_setup` helper
        - new tests: `ws_session::challenge_connect_is_not_ready_until_setup_completes`, `ws_codegen::client::tests::plain_connect_is_kept_only_without_a_challenge_flow`; extended `only_challenge_flow_endpoints_get_a_setup_connect` and `slow_ws_session_setup_and_recovery_run`
        - focused checks: schematic-gen ws filter 120 passed; `just test-schema` 150 passed; `just check-drift` 9 passed; `just lint` and `just lint-schema` clean
- work completed for 'R1-F1 Challenge endpoints report Ready without completing authentication' at 02:10:35
- starting the work on 'R1-F2 Typed Dock messages reject recorded replies and discard event data' at 02:10:35
        - defect class: a typed projection is wired to payload models that do not represent the protocol's recorded envelope or retain its application data
        - swept: all six Dock messages (both directions), every `DockWs*` consumer repo-wide (none outside schematic), the generated Dock module, and the typed-decode fixture tests of Core, Integration, ElevenLabs, and Samsung
        - completed: Dock types realigned with `UCD2-asyncapi.yaml`: `DockWsEnvelopeType` is `Dock | Event | AuthRequired | Auth`; request is `{type:dock, id, command, ...params}` with selector `type=dock`; reply and event keep top-level fields in a flattened `data` map; `msg_data` and the `parse_payload` helpers are removed in favor of `parse_data::<T>()`; `DockWsKnownMessage` now lists the 25 recorded names
        - discovered (R1-F2.11): Core and Integration `AuthRequired` dropped the recorded `msg_data`; field added, and `PartialEq`/`Eq`/`Hash` derives dropped (no struct literal outside schematic)
        - discovered (R1-F2.12): Dock `Auth.responds_with("Authentication")` generated a `request_auth` that could only time out, because `authResponseMsg` carries no `req_id`; the link is removed and the generated method is gone
        - departure: the brief assumed the Auth correlation stays; it was removed under R1-F2.12
        - deferred (nonblocking, not spec code): `.claude/skills/unfolded-circle/dock-api.md` (1158 lines) describes the Dock protocol with `kind`/`payload`/`req`/`resp`/`get_status`, which is probably where the wrong names came from; it needs a rewrite against `UCD2-asyncapi.yaml`
        - new or rewritten tests: `ws_runtime::dock_root_decodes_every_server_message_variant`, `ws_runtime::dock_root_value_and_typed_streams_keep_recorded_frames`, `ws_correlation::dock_get_sysinfo_request_uses_the_recorded_wire_shape`, recorded-example round-trip unit tests in `dock_ws/types.rs`, `auth_required_retains_msg_data` in Core and Integration; payload assertions added to the Core, Integration, and Samsung all-variant tests
        - focused checks: schematic-definitions filter 37 passed; `just test-schema` 152 passed; schematic-gen ws filter 120 passed; `just check-drift` 9 passed; `just lint` clean; `cargo check --all-targets -p unfolded-integration-helper` passed
- work completed for 'R1-F2 Typed Dock messages reject recorded replies and discard event data' at 02:28:51
- starting the final reconciliation and area gates at 02:28:51
        - reconciled every ledger row against the subagent evidence and the combined diff; spot-checked the generator change (`generate_connect_method`) and the new Dock types
        - all eight reverse dependents of schematic crates (sniff, biscuit-speaks, homelab, arcam-amp-integration, unfolded-integration-helper, eversolo-integration, sony-receiver-integration, model-citizen) compile with `cargo check --all-targets`
        - no cross-OS risk: the changes touch no paths, `cfg` blocks, or process handling, so no Windows cross-check was run
- work completed for the final reconciliation and area gates at 02:34:21

### Completion Report

- **outcome:** complete
- **elapsed:** 2026-10-23T01:58:27-07:00 to 2026-10-23T02:34:21-07:00 (about 35 minutes)
- **parent findings:** total 2 = 2 fixed + 0 already fixed + 0 blocked + 0 deferred + 0 unresolved + 0 disputed
- **instance rows:** 26 (20 from the review, 6 discovered): 17 fixed, 9 clean, none open
- verified repairs:
        - R1-F1: an endpoint with a challenge flow now has only `connect_{x}_with_setup`, and its private open method requires the callback, so `Ready` (and the `send` gate behind it) implies setup completed; endpoints without a challenge flow keep their plain connect
        - R1-F2: Dock envelopes match `UCD2-asyncapi.yaml` (`type:dock`, `command`, top-level data kept in a flattened map); recorded replies and events decode unedited through `next_event()`, `events()`, and the correlated typed request with their data readable; Core and Integration `AuthRequired` keep `msg_data`; the uncorrelatable Dock `request_auth` is gone
- open finding IDs: none
- nonblocking follow-ups (not spec findings):
        - `.claude/skills/unfolded-circle/dock-api.md` describes an outdated Dock wire format (`kind`, `payload`, `req`/`resp`, `get_status`); deferred because it is a 1158-line knowledge-base rewrite outside the spec's code; next action: rewrite it against `UCD2-asyncapi.yaml`
        - the review's observations remain for the follow-up pass; the `AuthFlowHints` doc observation was fixed as R1-F1.14
- changed files:
        - generator: `schematic/gen/src/ws_codegen/client.rs` (R1-F1)
        - definitions: `schematic/define/src/websocket/hints.rs` (R1-F1.14); `schematic/definitions/src/unfolded_circle/{mod.rs,dock_ws/mod.rs,dock_ws/types.rs,core_ws/types.rs,integration_ws/types.rs}` (R1-F2)
        - regenerated: `schematic/schema/src/{unfolded_circle_core_ws,unfolded_circle_dock_ws,unfolded_circle_integration_ws,elevenlabs_ws,samsung_smart_tv_remote_ws}.rs`
        - tests: `schematic/gen/tests/l1/{ws_codegen,fresh_output_compile}.rs`; `schematic/definitions/tests/ws_definition_json.rs`; `schematic/schema/tests/{ws_mock/mod,ws_session,ws_runtime,ws_correlation,ws_handshake,ws_idle}.rs`
        - docs: `schematic/docs/websocket-definitions.md`; `.claude/skills/schematic-define/schematic/websocket.md`
- final verification (macOS, after the last change):
        - `just test` in `schematic/`: exit 0; 2770 passed (20 skipped), 14 slow fresh-output passed, 1 compile-fail passed (R1-F1, R1-F2)
        - `just lint` in `schematic/`: exit 0, no warnings (R1-F1, R1-F2)
        - `just check-drift`: 9 passed (regenerated output matches the generator)
        - `just check-tier-coverage schematic`: 0 stranded tests (new tests are selected by a running tier)
        - `cargo check --all-targets` of the eight reverse dependents: passed (R1-F1.13, R1-F2 consumers)

## Implementation of Review Findings #2

> **started at:** 2026-10-23T02:39:46-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-17-schematic-websocket/review-2.md'
- this is iteration 2 of the review-to-implement cycle

## Follow-up Pass for Review #2

- the review found in '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-17-schematic-websocket/review-2.md' indicated that the specification is **production ready**
- the specification can be found at '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-17-schematic-websocket/spec.md'
- the review's `findings` list is empty, so there is nothing to fix; its `observations` list has one entry, which goes to the backlog unrepaired, as this pass requires

### Finding Ledger

| ID | Finding | Disposition | Evidence or reason |
| --- | --- | --- | --- |
| R2-O1 | Dock protocol skill contains an older wire format | observation, carried to backlog | `.claude/skills/unfolded-circle/dock-api.md` (1158 lines) still shows `kind`, nested `payload`, and `get_status` where `specs/unfolded-circle/UCD2-asyncapi.yaml` and the repaired Dock definition use `type`, top-level fields, and `get_sysinfo`; this is older knowledge-base content outside the spec's code, and observations are not repaired in this pass |

- code changes: none, so no focused tests, `just test`, `just lint`, or cross-OS check were needed; the area gates recorded in review 2 (`just test`, `just test-schema`, `just lint-schema`, `just check-tier-coverage schematic`) still describe the current tree
- backlog entry: `schematic/fixes/_unscheduled/schematic-websocket-follow-ups/spec.md` (new), with a section for R2-O1
- set `implemented: true` and `implemented_by: claude/opus` on `review-2.md`
- work completed at 2026-10-23T02:42:00-07:00
