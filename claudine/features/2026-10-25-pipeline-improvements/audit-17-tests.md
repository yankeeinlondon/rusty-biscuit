---
kind: implementation-log
---

# Test audit: 2026-10-27-darkmatter-lifecycle-ownership

Added **one genuinely new test function/Nextest identity**, with gross and net
identity growth both **+1**. Strengthened existing tests: **0**. Slow tests
modified: **0**. No production behavior changed.

The new real-CLI test,
`compose_lifecycle::ignored_actions_warn_per_reached_occurrence_without_checking_their_calls`,
proves three contracts together: two reached `say` occurrences in `composed`
each warn, a third occurrence in `finalize` names that event, and an
unselected occurrence warns nothing. Each ignored operand contains a
provably invalid `length(42)` call. Composition still succeeds and writes
exactly the body, proving that an ignored operand is not call-checked or
evaluated. This catches warning deduplication by verb, incorrect event names,
and accidentally inspecting ignored operands. It took **0.381 seconds** in
the parallel run.

## Requirement coverage

| Acceptance criteria | Evidence examined and executed |
| --- | --- |
| 1–2: behavior baseline and preservation | Existing recorded `lifecycle_traces` replay through both the actual Claudine CLI and Darkmatter's `lifecycle_boundary::replay`. No trace or behavioral assertion was changed. Historical pre-extraction evidence is recorded in the implementation log; this audit checks the final integrated implementation. |
| 3–4: dependency direction and one implementation | `dependency_direction_guard`, both Claudine `lifecycle_one_implementation_guard` suites, and the lift-ready guard. Source inspection confirms the adapters delegate to Darkmatter's parser, executor, accounting, and policy. |
| 5–7: host names, registration, prototype host | `lifecycle_boundary::protocol_tests`, `publisher`, registry unit tests, parser rename tests, and schema parity. The prototype tests exercise the real shared registry and policy through their adapter. |
| 8–10: shell ownership, preflight, captured recovery rules | `lifecycle_run_protocol`, `md_host` unit tests, shared preflight/policy tests, and Claudine production traces. They assert shell relocking, failure stages, retry/resume/finalize rules, ticket rejection, and post-cancellation immutability. |
| 11 and 21: `md compose`, output, failure, cancellation | Real `compose_lifecycle` CLI tests cover successful order, preflight/missing-include failure, skip/stop, ignored actions, output channels, malformed root/lifecycle, initialization mutations, child isolation, broken output, and actual Ctrl-C. In-process `md_host` tests cover injected failing writers and cancellation at engine phases. The new test adds per-occurrence warnings and ignored invalid calls. |
| 12: passive surfaces | `lifecycle_passive_surfaces` uses filesystem and native-argument-recorder evidence plus a positive composition control. `lifecycle_passive_compose` covers the library API. Editor tests communicate through the LSP harness without effect services. |
| 13 and 15: editor profiles and host globals | `claudine_schema_discovery`, `lifecycle_event_views`, `lifecycle_host_profile`, `host_globals`, and host-catalog tests check diagnostics, completion, hover, unavailable globals, and broken-catalog reporting through the protocol harness. |
| 14: shared global catalog | `global_catalog`, host-catalog tests, and embedded baseline tests cover disk/embedded parity, transitive imports, `fm`, independent namespace roots, runtime `err`/`timing` shape, and catalog non-application. |
| 16–18: trigger spelling, flags, AND/OR grammar | Trigger migration, CLI flag, embedded schema, and `trigger_match_grammar` suites cover typed migration failures, selection controls, nested logical groups, vacuity, and rejected anonymous lists/`any`. |
| 19: spike scenarios | Both relocated `lifecycle_control_flow_spike` harnesses pass. Earlier pipeline audit steps preserved their unchanged models/scenarios under durable test fixtures; this audit preserves that repair rather than restoring the broken active-spec imports. |
| 20: documentation | Owned by the concurrent documentation agent. This test slice did not edit canonical docs or the completed spec. |
| 22: approval and tolerant parsing | `compose_lifecycle_approval`, schema fallback, parser/executor tests, and the new ignored-call test distinguish supported malformed/forbidden actions from opaque ignored actions, and exercise real native argument recording under exact/template approval. |
| 23: narrowing | Lifecycle host-profile/editor tests, guarded native-shell approval tests, and Claudine's unchanged guarded-shell traces preserve the shared typing and presence behavior. |

The engine, parser, matcher, and policy assertions are F1 transformation and
state-machine checks. Real binary execution, native argument recording, OS
signal delivery, and the LSP protocol harness prove command/protocol wiring.
This slice asserts no new terminal rendering or input-encoding behavior.
The unchanged detached-terminal integration evidence from S14 and S16 remains
applicable; no redundant L2 execution was added. No window was opened or
focused by this slice.

## Verification

From `darkmatter/`, using the canonical Nextest-backed recipe, with parallel
L1 execution:

```sh
BISCUIT_TEST_FILTER='test(lifecycle) | test(global_catalog) | test(trigger_match_grammar) | test(host_globals) | test(claudine_schema_discovery) | test(dependency_direction_guard)' just test
BISCUIT_TEST_FILTER='test(compose_lifecycle) | test(lifecycle_passive) | test(lifecycle_schema_fallback)' just test
BISCUIT_TEST_FILTER='test(trigger_spelling_guard) | test(trigger_selection_parity) | test(trigger_envelope_claim) | test(schema_trigger_rename) | test(trigger_input_matrix) | test(policy_verbs)' just test
```

From `claudine/`:

```sh
BISCUIT_TEST_FILTER='test(lifecycle_traces) | test(lifecycle_one_implementation_guard) | test(lifecycle_lift_ready_guard) | test(lifecycle_control_flow_spike) | test(lifecycle_renamed_events) | test(host_catalog) | test(claudine_schema_parity) | test(preflight::checks)' just test
```

Results: **571**, **58**, **160**, and **137** passing observations in the
command order above. The second run repeats 57 tests from the first, so
**869 distinct L1 tests passed**. Maxima were **1.999**, **0.591**, **1.920**,
and **2.344 seconds**; zero tests
exceeded five seconds. Logs are `/tmp/pipeline-s17-darkmatter.log`,
`/tmp/pipeline-s17-new-cli.log`, `/tmp/pipeline-s17-trigger.log`, and
`/tmp/pipeline-s17-claudine.log`.
`git diff --check` passed. The first draft of the new fixture used a boolean
`when`; the parser correctly rejected it, and the final fixture uses the
required expression string. That setup failure is not evidence of a product
defect or a red-before production regression. The trigger spelling source
guard also caught a newly added canonical-doc sentence spelling the retired
kind; the documentation agent rephrased that sentence, and the complete
160-test trigger/policy scope then passed without weakening the guard.

## Limits and recorded departures

The existing implementation/review records architectural departures that this
test audit does not silently approve or redesign: Claudine shares the engine's
policy/accounting but retains direct-call drivers rather than routing every
run through `Run`; checks classified `BeforeRun` fire no lifecycle events;
some preflight/final composition work is repeated. `md` uses passive
report-only `--shell`, not an implemented `compose --dry-run` flag. Its error
classification and cancellation/output limits remain as documented. Passing
tests prove these current boundaries, not every literal requirement of the
original snapshot. The concurrent review/doc agents were informed.

The later CLI policy refactor also superseded the snapshot's trigger-disable
flag: current commands use `--disable schema-triggers`, and reject both older
`--no-…` spellings. The trigger flag tests assert this integrated behavior;
this audit does not restore an obsolete flag.

All measurements here are from this macOS host. No cross-OS run is claimed;
the new test uses the portable isolated CLI fixture and contains no shell,
path comparison, native platform branch, network, or audio operation.
