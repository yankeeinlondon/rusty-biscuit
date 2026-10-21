---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/features/2026-10-27-darkmatter-lifecycle-ownership/spec.md"
plan: "claudine/features/2026-10-27-darkmatter-lifecycle-ownership/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - claudine/cli/tests/l1/lifecycle_traces.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/fixtures/lifecycle_traces/
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/mod.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/engine.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/profiles.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/publisher.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/protocol_tests.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/replay.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - claudine-cli
    - darkmatter
    - claudine
human_review: false
source_files_during_phase_2:
    - claudine/cli/Cargo.toml
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/prep_target.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/pipeline/tests.rs
    - claudine/cli/src/commands/wrap/composition/preflight.rs
    - claudine/cli/src/commands/wrap/composition/prep_context.rs
    - claudine/cli/src/commands/wrap/composition/runner.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/composition/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/coordinator_adoption.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/lifecycle_ordering.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_evaluation.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_routing.rs
    - claudine/cli/src/commands/wrap/launch_plan.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/commands/wrap/wrapper_stages.rs
    - claudine/cli/src/main.rs
    - claudine/cli/src/output/error_walker/tests.rs
    - claudine/cli/tests/fixtures/lifecycle_traces/guarded-shell-null.md
    - claudine/cli/tests/fixtures/lifecycle_traces/guarded-shell-null.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/guarded-shell-present.md
    - claudine/cli/tests/fixtures/lifecycle_traces/guarded-shell-present.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/retry-in-initialize.md
    - claudine/cli/tests/fixtures/lifecycle_traces/retry-in-initialize.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/retry-in-preflight-blocked.md
    - claudine/cli/tests/fixtures/lifecycle_traces/retry-in-preflight-blocked.trace
    - claudine/cli/tests/fixtures/lifecycle_traces/retry-in-start.md
    - claudine/cli/tests/fixtures/lifecycle_traces/retry-in-start.trace
    - claudine/cli/tests/l1/composition_seams.rs
    - claudine/cli/tests/l1/error_guards.rs
    - claudine/cli/tests/l1/error_guards/transport-allow.toml
    - claudine/cli/tests/l1/lifecycle_traces.rs
    - claudine/lib/Cargo.toml
    - claudine/lib/src/composition/coordinator/handoff.rs
    - claudine/lib/src/composition/error/lifecycle_authoring.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/frontmatter_excerpt.rs
    - claudine/lib/src/composition/lifecycle/audio.rs
    - claudine/lib/src/composition/lifecycle/binding_checks.rs
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/bindings/rename.rs
    - claudine/lib/src/composition/lifecycle/bindings/tests.rs
    - claudine/lib/src/composition/lifecycle/cancellation.rs
    - claudine/lib/src/composition/lifecycle/context.rs
    - claudine/lib/src/composition/lifecycle/context/tests.rs
    - claudine/lib/src/composition/lifecycle/control.rs
    - claudine/lib/src/composition/lifecycle/effects.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/runtime.rs
    - claudine/lib/src/composition/lifecycle/tests/container_runtime.rs
    - claudine/lib/src/composition/lifecycle/tests/grammar.rs
    - claudine/lib/src/composition/lifecycle/tests/mod.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/diagnostics/error_kind.rs
    - claudine/lib/src/diagnostics/mod.rs
    - claudine/lib/src/diagnostics/restored.rs
    - claudine/lib/src/diagnostics/restored/tests.rs
    - claudine/lib/src/diagnostics/snapshot.rs
    - claudine/lib/src/harness/runtime.rs
    - claudine/lib/src/harness/timeout.rs
    - claudine/lib/src/invocation_context.rs
    - claudine/lib/src/mcp/import.rs
    - claudine/lib/src/reporting/error.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/lifecycle_control_flow_spike.rs
    - claudine/lib/tests/l1/lifecycle_lift_ready_guard.rs
    - claudine/lib/tests/l1/path_lookup_guard.rs
    - claudine/lib/tests/l1/type_scope_ownership_guard.rs
    - darkmatter/lib/src/lib.rs
    - darkmatter/lib/src/lifecycle/action_shape.rs
    - darkmatter/lib/src/lifecycle/actions.rs
    - darkmatter/lib/src/lifecycle/actions/tests.rs
    - darkmatter/lib/src/lifecycle/authored_order.rs
    - darkmatter/lib/src/lifecycle/cancellation.rs
    - darkmatter/lib/src/lifecycle/cancellation/tests.rs
    - darkmatter/lib/src/lifecycle/context.rs
    - darkmatter/lib/src/lifecycle/context/tests.rs
    - darkmatter/lib/src/lifecycle/control.rs
    - darkmatter/lib/src/lifecycle/control/tests.rs
    - darkmatter/lib/src/lifecycle/duration.rs
    - darkmatter/lib/src/lifecycle/effects.rs
    - darkmatter/lib/src/lifecycle/effects/tests.rs
    - darkmatter/lib/src/lifecycle/envelope.rs
    - darkmatter/lib/src/lifecycle/envelope/tests.rs
    - darkmatter/lib/src/lifecycle/error.rs
    - darkmatter/lib/src/lifecycle/executor.rs
    - darkmatter/lib/src/lifecycle/executor/host.rs
    - darkmatter/lib/src/lifecycle/executor/outcome.rs
    - darkmatter/lib/src/lifecycle/executor/shell.rs
    - darkmatter/lib/src/lifecycle/executor/tests.rs
    - darkmatter/lib/src/lifecycle/mod.rs
    - darkmatter/lib/src/lifecycle/parse.rs
    - darkmatter/lib/src/lifecycle/run_guard.rs
    - darkmatter/lib/src/lifecycle/run_guard/tests.rs
    - darkmatter/lib/src/lifecycle/run.rs
    - darkmatter/lib/src/lifecycle/run/protocol.rs
    - darkmatter/lib/src/lifecycle/runtime.rs
    - darkmatter/lib/src/lifecycle/runtime/tests.rs
    - darkmatter/lib/src/lifecycle/services.rs
    - darkmatter/lib/src/lifecycle/services/tests.rs
    - darkmatter/lib/src/lifecycle/shell_template.rs
    - darkmatter/lib/src/lifecycle/signatures.rs
    - darkmatter/lib/src/lifecycle/source_map.rs
    - darkmatter/lib/src/lifecycle/tests.rs
    - darkmatter/lib/src/lifecycle/validate.rs
    - darkmatter/lib/src/markdown/compose/context/merge.rs
    - darkmatter/lib/tests/l1/dependency_direction_guard.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/engine.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/mod.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/profiles.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/protocol_tests.rs
    - darkmatter/lib/tests/l1/lifecycle_boundary/replay.rs
    - darkmatter/lib/tests/l1/lifecycle_run_protocol.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/path_lookup_guard.rs
docs_updated_during_phase_2:
    - claudine/docs/topics/error-architecture.md
    - claudine/docs/topics/frontmatter-properties.md
    - claudine/docs/topics/signal-handling.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/cli-commands.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/validations-and-handlers.md
    - .claude/skills/darkmatter/lifecycle.md
    - .claude/skills/darkmatter/SKILL.md
message_to_agent: |-
    Phase 2 is done: the lifecycle parser, executor, run accounting, catch protocol, control admission, services, and a gated request/reply `Run` live in `darkmatter::lifecycle`; Claudine adapts them and re-exports every old path. Read the Phase 2 log sections (Entry gate, Wave 4, design, Waves 5a–5d, Wave 6, checkpoint) before Phase 3. Key facts:

    - Baseline revision is now `d48d7dcf0`. Traces: 35 scenarios in `claudine/cli/tests/fixtures/lifecycle_traces/` (Phase 1's 30 plus `guarded-shell-{present,null}` and `retry-in-{initialize,preflight-blocked,start}`); all replay through the production `Run` in `darkmatter/lib/tests/l1/lifecycle_boundary/replay.rs`. Never edit a `.trace`.
    - One engine: `LifecycleRunGuard` (used by every Claudine driver today) and `run::Run` (request/reply, used only by tests so far) share `run_guard::RunAccounting`, `decide_lifecycle_transition`, `LifecycleCatchProtocol`, and `decide_control`. Phase 3 "Driver migration" moves Claudine's drivers onto `Run` and then deletes what is superseded; do not add a third state machine.
    - Claudine adapters: `ClaudineLifecycleEffects` (TTS/messaging config), `StackExecutionContext` as the `LifecycleHost`, `ClaudineRunHost` for the guard, `*Ext` extension traits for the old constructors (a file calling `LifecycleRunGuard::new`, `LifecycleCatchProtocol::new`, `LifecycleErrorInfo::from_*`, `DiagnosticSnapshot::from_*` needs the matching trait import), and `CompositionError` twins of `LifecycleError` joined by one `From` and a Display-parity test (fold them in Phase 3's delete task).
    - Still in Claudine and due in Phase 3: the lifecycle pre-flight walk (`composition/preflight.rs`, the type-narrowing decision sites), the binding catalog's availability rules (read through `LifecycleHost`), `binding_checks.rs`, `LIFECYCLE_HOST_GLOBALS` (a fixed list in `shell_template.rs`, to come from the registry).
    - Open, untraced divergences to trace BEFORE wiring drivers onto `Run`: `resume` in a preflight-reached `blocked` (production `LifecycleResumeWithoutSession`, `Run` `SetupRecoveryUnsupported`, which a protocol test asserts); `initialize` resume/hand-off rows in `Run`; a cancelled root run (exit 130).
    - Guards now scan `darkmatter/lib/src/lifecycle/` from Claudine (`type_scope_ownership_guard`, `composition_seams`); new Darkmatter files there must be added to the `source-inputs` of `claudine/lib/Cargo.toml` and `claudine/cli/Cargo.toml`.
    - `dmls::l1 searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back` is a pre-existing load-sensitive flake (fails at `d48d7dcf0` too); rerun it alone.
    - Windows: `claudine` and the Phase 2 Darkmatter tests pass natively; 9 `claudine-cli` tests (`implement_plan_commit_message::*`, two `*_quoted_empty_argument*`) fail on Windows at `HEAD` too (see "Cross-OS check (Phase 2)"); the spec has `os_failing: windows`. They are not caused by this feature; do not "fix" them by weakening their assertions.
---

# Implementation Log for 2026-10-27-darkmatter-lifecycle-ownership (6 phases)

## Phase 1

### Baseline inventory (Wave 1)

- **Starting revision:** `f3381eb5cb98102e26159cef5b690b5a5939ba27` on `feat/pipeline` (2026-10-20). This is the behavioral baseline for every trace captured in this phase.
- **Prerequisite gate — passed.** `2026-10-26-task-inputs-and-providers` closed in `2db75c81b` and sits in `_completed`; its transitive prerequisites `2026-10-25-loops-and-flow-control`, `2026-10-24-sequence-containers`, `2026-10-20-lifecycle-grammar`, `2026-10-23-shell-template-approval`, `2026-10-21-claudine-schemas`, and `2026-10-18-fm-namespace` are all in `claudine/features/_completed/`. The running code exposes the flat grammar (`when`/`then`/`else`), `cancelled` and container lifecycles, `seq`/`loop`/`group` roots, `break`/`prep`/task `when:`, task `with:`/per-task providers, schema action unions, and typed shell templates (all exercised by the existing L1 suites listed below).
- **Finding — `2026-10-16-type-narrowing` has NOT landed.** The spec lists it in `depends-on` (section 4's `TypeScope` placeholder audit and acceptance criterion 23). It is still `status: draft-spec`, `implemented: false` in `claudine/fixes/`, and no `TypeScope` type exists in `darkmatter/lib/src` or `claudine/lib/src`. It is not a transitive prerequisite of the task-inputs gate the plan names, so Phase 1 proceeds; Phase 3 (preflight `TypeScope` reads) and criterion 23 cannot be satisfied until it lands. Raised for human review.
- **Lifecycle code at the baseline:** about 50,000 lines across `claudine/lib/src/composition/lifecycle/` (10,787 in its top-level `.rs` files), `claudine/lib/src/composition/looping/`, and `claudine/cli/src/commands/wrap/harness_orch/loop_control/`.

#### Lifecycle caller inventory (production paths that drive the engine)

| Entry symbol | Callers (excluding tests) | Path(s) |
|---|---|---|
| `LifecycleRunGuard::new` | `cli/.../wrap/composition/pipeline.rs`, `cli/.../wrap/wrapper_stages.rs`, `cli/.../compose/prep.rs`, `cli/.../wrap/sequence/container.rs`, `lib/.../looping/engine.rs`, `lib/.../sequence/task/{group,sequence}.rs` | direct + inline compose, wrapper stages, `prep`, root sequence container, loop engine, nested group/sequence |
| `LifecycleCatchProtocol::new` | `cli/.../wrap/composition/{pipeline,preflight}.rs`, `cli/.../loop_control/error_routing.rs`, `lib/.../looping/engine.rs` | compose preflight catch, harness error routing, loop catch |
| `decide_lifecycle_transition` | `cli/.../wrap/composition/{pipeline,preflight}.rs`, `cli/.../harness_orch/loop_control{,/control_dispatch}.rs`, `lib/.../lifecycle/runtime.rs` | terminal recovery decisions |
| `drive_terminal_recovery` | `cli/.../harness_orch/loop_control{,/control_dispatch}.rs` | retry/resume/proxy dispatch (before finalize) |
| `decide_control` | `lib/.../lifecycle/{control,runtime}.rs` | retry/resume admission |
| `run_event_stack` | `cli/.../wrap/composition/preflight.rs`, `cli/.../loop_control/{error_routing,lifecycle_events}.rs`, `lib/.../lifecycle/runtime.rs`, `lib/.../looping/engine.rs` | every event stack run |
| `ContainerRunner` | `cli/.../wrap/sequence/container.rs`, `lib/.../looping/container.rs`, `lib/.../sequence/task/{group,sequence}.rs` | group/sequence container events |
| `execute_loop_with_lifecycle` | `cli/.../compose/loop_run.rs`, `lib/.../looping/engine.rs` | document loops |

Task paths (`cli/src/commands/wrap/sequence/task_run.rs`: `run_step_task`, `open_member_run`, `open_leg_run`, `run_looping_document`) and recovery paths (proxy adoption in `loop_control`, retry/resume reread in `wrap/composition/pipeline.rs`) reach the engine through the symbols above.

#### Acceptance-criterion ownership map

| AC | Owner phase / task | Evidence (fixture or test) |
|---|---|---|
| 1 Stage 1 gate | P1 trace capture + prototype + verdict | `claudine/cli/tests/l1/lifecycle_traces.rs`, `darkmatter/lib/tests/l1/lifecycle_boundary/`, verdict below |
| 2 No Claudine behavior change | P2 bridge, P3 parity | existing Claudine lib/CLI suites unchanged + Stage 1 trace replay |
| 3 Dependency direction | P2 Claudine bridge | dependency guard over Darkmatter lib/CLI/DMLS manifests and imports |
| 4 One implementation | P3 driver migration | removed-symbol source guard |
| 5 Host names | P3 host registry | registry + parse tests (`composed:`/`failed:` in Claudine docs) |
| 6 Registration checks | P3 host registry (prototyped P1) | registry unit tests; prototype `protocol_tests.rs` |
| 7 Prototype host | P1 boundary prototype | `lifecycle_boundary/publisher.rs` |
| 8 Shell invariants | P3 preflight integration, P5 invocation | both-host shell tests; trace `shell-*` scenarios |
| 9 Preflight host checks | P3 preflight integration | `blocked-preflight` trace + existing approval-order tests |
| 10 Section 8 behavior | P2 engine, P3 parity | Stage 1 traces (`blocked-refused-handoff`, `resume-incompatible`, `skip`, `retry-in-finalize`, …) under the new owner |
| 11 `md compose` events | P4 compose host, P5 invocation | darkmatter CLI L1 |
| 12 Passive surfaces | P5 editor projection | recording fake services |
| 13 Editor typing | P5 editor + schema projection | DMLS tests |
| 14 Global catalog | P4 global catalog | catalog tests + matrix |
| 15 Host globals | P4 global catalog, P5 editor | host catalog + BindingView parity |
| 16 Trigger kind | P4 trigger migration | trigger scan/DMLS tests + matrix |
| 17 Trigger flag | P4 trigger migration | CLI flag tests |
| 18 AND/OR | P4 trigger migration | matcher tests + migrated-file selection parity |
| 19 Spike tests | P2 bridge, P6 final | both `lifecycle_control_flow_spike` tests |
| 20 Docs | P6 documentation | Stage 5 pages |
| 21 Boundaries / output failures | P4 compose host, P5 invocation | darkmatter CLI L1 with failing writers / fake cancellation |
| 22 Approval / tolerant parsing | P3 preflight, P4 compose host, P5 schema projection | approval + ignored-action tests |
| 23 Narrowing survives | blocked on `2026-10-16-type-narrowing` | — (see finding above) |

### Trace capture (Wave 2)

- **Boundary:** the whole `claudine` binary (F3, L1 via `CliProcessFixture`), driven by `claudine compose` / `claudine sequence` with the Unix `launch_recorder` stub provider. These are production paths, not the historical model.
- **Files:** `claudine/cli/tests/l1/lifecycle_traces.rs` (declared `#[cfg(unix)] mod lifecycle_traces;` in `claudine/cli/tests/l1/main.rs`), fixtures in `claudine/cli/tests/fixtures/lifecycle_traces/` (30 `<id>.md` + `<id>.trace`, plus `target`/`step` helper documents).
- **Trace channel:** every lifecycle stack appends to one `trace.log` with the `append_line` side effect (`event <name>`, `err <code | uncoded: msg | none>`, `state <k>=<v>`); lifecycle `shell` actions append `shell <event>` only when shells are unlocked; the stub provider appends `work <fresh|resume> <status>`; sequence `shell:` tasks append `task <name> …`; the test appends `exit <code>`. Container subjects are prefixed (`seq:`, group/leg names, `target:`, `step:`). Each `.trace` carries `# given:` host facts used by the replay.
- **Tests:** one `#[test]` per scenario from `macro_rules!` (fixture `include_str!` literals inside each test fn for CI test-input mapping), plus `every_trace_fixture_has_exactly_one_test`. 31 tests, each 0.02–2.0 s. A mutation check (`skip.trace` edited to `exit 7`) failed as expected and was restored byte-identical.
- **Scenarios (30):** success; blocked-preflight; blocked-refused-handoff; catch-{initialize,start,blocked,success,failure,finalize,loop}; downgrade-{success,blocked}; finalize-error; retry-{admitted,exhausted,in-finalize}; resume-{admitted,no-session,incompatible}; proxy-adopted; proxy-target-bootstrap-failed; loop-break; cancelled-{with-handler,without-handler,not-opted-in}; skip; sequence-nested-group; shell-locked-blocked; shell-unlocked-blocked-after-start; shell-relocked-after-proxy. Cancellation uses deterministic sibling-failure cancellation (`fail_fast` group, marker-file synchronization, no real signals); "without handler" is a nested container that opted in with `cancelled: []` but has no stack.
- **Coverage gap:** the module is Unix-only, like its siblings `loop_break` and `lifecycle_directive_matrix`, because `launch_recorder` is Unix-only and the shells are POSIX. Native Windows and WSL2 parity of these traces is not covered by Phase 1. The darkmatter replay reads the trace text on every OS.

#### Departures from spec section 8 (the code wins; fixtures keep observed behavior)

| # | Scenario | Spec reading | Observed at `f3381eb5c` |
|---|---|---|---|
| D1 | catch-initialize | pre-composition failure → `blocked` | an evaluation error in `initialize` fires `failure` then `finalize` (setup catch goes to the failed outcome alternative) |
| D2 | catch-blocked | catch-only cleanup, unspecified step | a raising guard in `blocked` continues to `failure` then `finalize`; the later error replaces the earlier |
| D3 | catch-success | only explicit `error:` covered | an evaluation error in `success` does **not** run `failure`; `finalize` sees the error; exit 1 (explicit `error:` does downgrade) |
| D4 | retry-in-finalize | "finalize runs once when the lifecycle ends" | finalize runs for attempt 1, then the retried attempt runs its own `start`…`finalize`; never a second finalize for the old attempt |
| D5 | resume-no-session | refused recovery still finalizes | `resume` in `start` before launch raises `ResumeWithoutSession`, exits 1, runs neither `failure` nor `finalize` |
| D6 | resume-incompatible | — (routing matches) | routing is `start` → `failure` → `finalize`; `err` is `LifecycleResumeIncompatible`, whose code falls through to the generic `composition.failed` (`compose_failed_code` / render default) |
| D7 | proxy-target-bootstrap-failed | refused handoff after launch → `failure` | a target that fails to parse (shell in its `initialize`) ends the run, exit 1, with no further events: no source `finalize`, no target events |
| D8 | shell-locked-blocked | refused before execution | a locked shell in early `blocked` raises like an evaluation error → `failure`; a locked shell in early `finalize` ends `finalize` silently, exit 1 |

Also observed: catch-loop (raising gate guard after `finalize`) runs no catch event and exits 1. Current Claudine docs (`claudine/docs/topics/flow-control/lifecycle.md` evaluation-error rules, `flow-control-reference.md` resume/proxy sections) already describe this behavior, so no doc correction was needed in this phase. The spec is left unchanged as the decision record. Section 8's text should be corrected in the Darkmatter lifecycle docs written in Phase 6.

### Boundary prototype (Wave 2)

- **Location:** test-only module `darkmatter/lib/tests/l1/lifecycle_boundary/` (`engine.rs`, `profiles.rs`, `publisher.rs`, `protocol_tests.rs`, `replay.rs`), declared in `darkmatter/lib/tests/l1/main.rs`. There is no production module, Cargo change, or export, because the spec forbids two production engines. `engine.rs` uses Darkmatter's real `BindingView`/`Availability` and `is_reserved_namespace`. Claudine names appear only in `profiles.rs::claudine_profile()`.
- **API decided (input to Phase 2):**
  - descriptors are `EventDescriptor { name, position, role, globals: BindingView, actions }` over `CoreEvent` × `Position` (`before-composition`/`after-composition`/`after-finalize`) × `Role` (`notification`/`terminal-outcome`/`iteration-gate`);
  - `Registry` with `rename`, `register`, `register_outcome_pair(OutcomePair::new(s, f).downgrade(from, to))`, `declare_core_actions`, `set_cancellation_error`, typed `RegistrationError`, and immutability while a run holds it;
  - owned `Request { ticket(run, document, attempt, iteration, generation), kind }` with kinds `EvaluateEvent` (mode `NormalRecovery` / `FatalEvaluationCleanup` / `RefusalCleanup` / `CancellationCleanup`), `Prepare{entry}`, `PerformOperation{entry}`, `AwaitCompletion`, `Recover`, and `Done`;
  - typed replies `Event(EventOutcome)`, `Prepared(Ready{epoch} | Failed(HostFailure{stage, entry, error}))`, `Operation(Started | Completed{outcome, session} | NotStarted)`, and `Recovery(Admitted | Exhausted | Refused | Adopted | Aborted)`;
  - a single shell owner, `run.shell_permission(&ticket)`;
  - `cancel()` invalidates outstanding generations before cleanup.
- **Non-agent host (`publisher.rs`):** registers a before-composition notification `configure`, renames `composed` → `ready`, and pairs `published`/`rejected` with downgrade `published → rejected`. It proves notification order, a `set` visible later, typed failure as `err`, unavailable `err` failing with the view's reason (never `fm.err`), refused shell in `initialize` and early cleanup, and outcome selection/downgrade.
- **Protocol tests:** registration (duplicates, Darkmatter names, reserved roots, fixed events, second name, role/position, outcome-pair completeness, downgrade target, immutability); the core-only `md` profile (no pair; only `failed` holds the slot; write failure and `composed` error → `finalize` without `failed`); rename diagnostics; request binding and rising generations; stale/duplicate/foreign/wrong-kind/late replies leaving the run unchanged (`run == before`); cancellation with and without `cancelled`, during `initialize`, late replies after cancel, cancel during cleanup; the shell owner from `initialize` to proxy adoption; all seven failure-stage rows; downgrade; skip; retry/resume/proxy finalize exceptions; recovery in `finalize`; `SetupRecoveryUnsupported`; the iteration gate.
- **Criterion 6 note:** "handler emitting an event" and "out-of-position emission" cannot be expressed against this API. It has no emit operation, and replies name outcomes rather than events. Phase 3 must keep that property (or add the checked emission seam and test it).

### Prototype verdict (Wave 3) — **PASS**

- `darkmatter/lib/tests/l1/lifecycle_boundary/replay.rs` replays all 30 production traces through the prototype with a scripted Claudine-profile host (one `#[test]` per trace, `include_str!` of each `.trace` inside its test, and a guard `every_recorded_trace_has_a_replay` that reads the directory via `manifest_dir!()`). **30/30 reproduce exactly**, line for line including `exit`.
- **Gate criteria:** the registry API was sufficient for the Claudine profile and the publisher host. Shell permission has one owner (the host writes `shell <event>` only after `shell_permission` returns `Ok`). No reply mutates engine accounting directly (terminal slot, finalize, shell lock, and generation are engine-internal). The Claudine coordinator and provider harness were not moved: budgets, sessions, target parsing, loop `until`/`max`, and container scheduling stay in the scripted host.
- **Engine changes the traces forced (none keyed on Claudine names):** (1) a new `RecoveryReply::Aborted(HostFailure)` ends the run without further events (D5, D7; only stage `Setup` for proxy and `RecoveryRefused` for retry/resume, anything else is `StageNotApplicable`); (2) `composed` may request `resume` (host answers `Aborted` without a session); (3) `Registry::set_cancellation_error` (`composition.cancelled` in the Claudine profile) is selected when cancellation cleanup starts. Two prototype cancellation tests gained the `error composition.cancelled` transcript line. A probe that made `Aborted` finalize failed exactly the D5/D7 replays, and the engine was restored.
- **How D1–D8 are expressed:** D1 is an engine rule (the outcome pair's failed alternative catches setup errors). D2/D3 are role-keyed engine rules (setup versus terminal outcome). D4 is an engine rule. D5/D7 use the host's typed `Aborted` reply. D6 is a host fact (stage + code). D8 is the host turning `ShellRefusal::Locked` into its uncoded evaluation error with Claudine's wording. The cancellation `err` is a profile setting.
- **Open, not covered by any trace (Phase 2 must add a trace before relying on it):** production's `retry` in `initialize` (and possibly recovery from `blocked`) is a hard error without `finalize`, while the prototype runs refusal cleanup; `retry` from `start` is unverified; a cancelled root run (exit 130) is not traced. The historical `spike/model.rs` and `spike/scenarios.rs` are untouched.

### Gates and test evidence

- **Baseline test identities:** `cargo nextest list` at this revision: `claudine` lib 5,499 tests (`/tmp`-only listing, not committed); `claudine-cli --features test-fixtures` 4,649 tests, of which 31 are the new `lifecycle_traces` tests (4,618 pre-existing). Both spike tests (`claudine/lib/tests/l1/lifecycle_control_flow_spike.rs`, `darkmatter/lib/tests/l1/lifecycle_control_flow_spike.rs`) pass unchanged.
- `cd claudine && just test`: **10,479 run, 10,479 passed** (12 slow, 9 skipped), 224.8 s. No pre-existing failures. Phase 1 adds only tests, so this is also the baseline result for the existing suite.
- `cd claudine && just lint`: pass. `just check-tier-coverage claudine`: pass, 0 stranded.
- `cargo nextest run -p darkmatter --features effects-instrumentation --test l1` (the only Darkmatter binary this phase touches): **1,693 passed**. `cd darkmatter && just lint`: pass. `just check-tier-coverage darkmatter`: 0 stranded.
- **Dependents:** no public API changed (test-only additions), so no downstream package tests apply.
- **Cross-OS:** no `just cross-check` run. The changes are test-only. The Darkmatter replay is portable: it trims `\r`, and the repository's `.gitattributes` forces `eol=lf`. The Claudine trace tests are `#[cfg(unix)]` by necessity (see the trace capture coverage gap).
- **Scope calculator:** not used to skip anything. Source files changed, so both gates ran.

#### Requirement → test mapping

| Requirement (plan Phase 1) | Tests |
|---|---|
| Baseline suite + spike tests | `just test` (claudine), both `lifecycle_control_flow_spike` tests |
| Traces through production paths, every Stage 1 scenario | `claudine-cli::l1 lifecycle_traces::<scenario>` ×30 + `every_trace_fixture_has_exactly_one_test` |
| Prototype: before-composition notification, rename, outcome pair, `set`, typed failure, unavailable `err`, refused `initialize` shell | `darkmatter::l1 lifecycle_boundary::publisher::*` (6) |
| Registration validation, owned requests, stale/duplicate/foreign/wrong-kind/cancelled replies, cleanup modes, cancellation, shell owner, failure stages | `darkmatter::l1 lifecycle_boundary::protocol_tests::*` (26) |
| Prototype verdict: replay every trace | `darkmatter::l1 lifecycle_boundary::replay::<scenario>` ×30 + `every_recorded_trace_has_a_replay` |

The Input Robustness Matrix does not apply to Phase 1: no catalog or trigger loader was changed. Its proposed outcomes still need the author rulings the plan requires before Phase 4 (see `human_review_items`).


## Phase 2

- **Starting revision:** `563326f84` on `feat/pipeline` (2026-10-20), Phase 1 committed. **Superseded:** Phase 2 was paused here, before any extraction code, so `2026-10-16-type-narrowing` could be built first (the author's ruling on the former Phase 1 review item, 2026-10-20). Phase 2 resumes from the revision that closes that fix; the plan's Phase 2 entry gate records it and refreshes the Phase 1 baseline.
- **Criterion 23 is no longer blocked:** owner P3 preflight integration (the audit reads `TypeScope`), with evidence from P5 invocation coverage (a guarded lifecycle shell under `md compose` behaves as under Claudine) and the narrowing conformance matrix passing unedited after every phase.

### Extraction design (decided before Wave 4)

- **Placement:** a new `darkmatter::lifecycle` module in `darkmatter/lib/src/lifecycle/`. No new crate: Darkmatter already depends on every crate the moved code uses (`biscuit-terminal`, `biscuit-file`, `indexmap`, `serde_json`, `thiserror`, `tracing`).
- **Errors:** the parser and model raise `darkmatter::lifecycle::LifecycleError`, which holds the lifecycle authoring variants moved out of Claudine's `CompositionError` (same fields, same `Display` text). Claudine keeps its same-named `CompositionError` variants for now, because its diagnostic code catalog, block renderer, and every existing test assertion are keyed on them, and maps each `LifecycleError` variant onto its twin with one exhaustive `From` impl. A Claudine test checks that the two `Display` texts agree for every variant. This keeps the error *type* in two places while the parser exists once; Phase 3 (the "delete superseded" task) can fold the twins together.
- **Existing tests stay where their assertions are typed.** Claudine's lifecycle tests assert on `CompositionError` variants through `claudine::composition::parse_lifecycle_config` and friends. They stay in Claudine, unchanged, and now exercise the moved parser through the bridge, which is the strongest evidence for acceptance criterion 2. Moving them would have meant renaming the error type inside each assertion. Tests that live inside a moved file and assert only on moved types move with it. Darkmatter gets new tests against its own public API.
- **Re-exports:** Claudine re-exports every moved item at its old path (`claudine::composition::{LifecycleSignal, LifecycleConfig, …}`), so callers and tests keep compiling. The re-exports delegate to the one implementation; there is no copy.

### Entry gate (resumed 2026-10-21)

#### Prerequisite revision

- `2026-10-16-type-narrowing` is closed: `implemented: true` on its plan and
  spec, a recorded Phase 6 close (documentation and final verification, all
  gates green), and `review-1.md` with `ready: true`, no findings, and
  `human_review: false`.
- Its closing commits: `3c05cfe60` (structural type narrowing),
  `2a58d0617` (scoped shell template refinements), `e3eb8e7ef` (guard scopes
  through composition consumers), `1b10bb732` and `d91e916af` (phase closes),
  then the review-to-implement repairs `13cbc87c6`, `8fd0addf1`, `3ec767019`,
  and `86d97771f`. The later strict-equality fix (`b59ba28e0`, `455eb05e4`,
  `96ccc207b`, `faef73cda`) also landed in between.
- **New starting revision: `d48d7dcf0`** on `feat/pipeline` (2026-10-21). It
  replaces Phase 1's `f3381eb5c` as this feature's behavioral baseline.

#### Baseline replay

All results at `d48d7dcf0`, before any extraction code:

- `cd claudine && just test`: **10,606 run, 10,606 passed** (30 slow, 9
  skipped), 288.8 s. This includes the 31 `lifecycle_traces` tests. All 30
  traces pass unedited, so the prerequisite changed no recorded trace and none
  was recaptured.
- Darkmatter `lifecycle_boundary` (6 publisher + 26 protocol + 31 replay) and
  its `lifecycle_control_flow_spike` tests: **78 passed**
  (`cargo nextest run -p darkmatter --features effects-instrumentation --test l1 -E 'test(lifecycle_boundary) | test(lifecycle_control_flow_spike)'`).
- Claudine `lifecycle_control_flow_spike`: **4 passed**.

These counts replace Phase 1's (10,479 / 1,693-binary).

#### Inventory refresh

What the fix added or changed in the lifecycle code
(`git diff f3381eb5c..d48d7dcf0`), with its owner under the spec's ownership
table:

| Code | What it does | Owner |
|---|---|---|
| `lifecycle/actions.rs::walk_scoped`, `action_writes` | the lifecycle walk's `TypeScope` extension (`when:` → `then:`/`else:` via `assume_at`, `join`), and the writes that end a fact | **Darkmatter**: moves with the parser slice in Wave 4 |
| `lifecycle/shell_template.rs` (`lifecycle_declared_types`, `placeholder_guard`, `typed_template`, `retime_guarded`, `HoleTiming::Guarded` sites) | typed lifecycle templates and the presence decision | **Darkmatter**: names only Darkmatter types; `actions.rs::ShellAction.template` holds a `LifecycleShellTemplate`, so it moves with the slice |
| `composition/preflight.rs` (`resolve_shell_text`, `guarded_template`, `sequence_lifecycle_template`, `missing_value`) | the scope-carrying lifecycle pre-flight walk | Darkmatter per the table, but **Phase 3** ("Preflight integration"); stays in Claudine this phase |
| `sequence/preflight/mod.rs` (`site_types`, `prepare_site`, `record_prompt_reference`, `resolve_approval_time`) | the sequence task `when:` extension | **Claudine** (stays) |
| `executor/tests/{guarded_shell_sites,scope_site_conformance}.rs`, `sequence/preflight/tests.rs` | the narrowing conformance matrix and guarded-site tests | tests stay with their assertions; paths updated only if a file moves |
| guarded-hole filling at a reached action | `LifecycleShellTemplate` filled through Darkmatter's shared template API (`fill.rs`) by the executor | **Darkmatter**, with the executor (Wave 5) |

The executor itself (`executor.rs`) did not change in the fix.

Files the fix's guard and allowlists name (`claudine/lib/tests/l1/type_scope_ownership_guard.rs`, which scans `claudine/lib/src` only):

- `composition/preflight.rs`: `resolve_lifecycle_shell_commands_in_sequence` (`Unguarded` ×1), decision sites `resolve_shell_text`, `guarded_template`, `sequence_lifecycle_template`, missing-value constructor `missing_value`
- `composition/sequence/preflight/mod.rs`: `site_types` (`Unguarded` ×1, `Assume` ×1), `prepare_site` (`DeclaredTypes` ×1), `record_prompt_reference` (`DeclaredTypes` ×1), decision sites `prepare_site`, `resolve_approval_time`
- `composition/lifecycle/shell_template.rs`: `lifecycle_declared_types` (`DeclaredTypes` ×1), `placeholder_guard` (`PresenceLookup` ×2)
- `composition/lifecycle/actions.rs`: `walk_scoped` (`Assume` ×2)

When `actions.rs` and `shell_template.rs` move, their four entries leave
Claudine's guard (which only scans Claudine source) and join Darkmatter's own
`darkmatter/lib/tests/l1/type_scope_ownership_guard.rs` allowlist with the
same function, count, and reason. That is the "path only" change for a guard
whose scan root is per crate.

#### Guarded-shell trace

- Two new scenarios, `guarded-shell-present` and `guarded-shell-null`
  (`claudine/cli/tests/fixtures/lifecycle_traces/`), driven like the others
  but with a `.darkmatter-shell-whitelist` holding only
  `template trace-shell start {{ string }}`, so the guarded shell is approved
  by shape and runs as its template (the `--yolo` first draft approved the
  resolved bytes, so the null case ran `hello` from pre-flight. That is
  exact-mode behavior, not the guarded hole). Template mode refuses `>>`, so
  the trace test now installs a `trace-shell` helper in the fixture `bin` for
  these two scenarios' commands.
- Present: `event initialize`, `event start`, `shell start hello`,
  `work fresh 0`, `event success`, `event finalize`, `err none`, `exit 0`.
- Null (`set: { note: null }` inside the selected `then:`): `event start`,
  `state note=`, then **no shell**; the action fails with "reads 'note',
  which is missing" (`MissingValue`), routed `failure` → `finalize` with
  `err composition.failed`, exit 1 (the same routing as `catch-start`).
- Darkmatter replay (`lifecycle_boundary/replay.rs`): a `{"template_shell": key}`
  step that checks `shell_permission`, then fills the placeholder from the
  host's live state; a `null` is the host fact "missing value" raised with
  Claudine's generic code. No engine rule is keyed on names. A truthy
  string guard condition and `null`-as-empty rendering were added to the
  replay vocabulary. **33 trace tests and 32 replays pass.**
- No trace was recaptured; Wave 4 starts from these 32 traces.

### Wave 4 — Model extraction (done)

- **Moved** to `darkmatter/lib/src/lifecycle/` (history-preserving renames):
  `parse.rs`, `action_shape.rs`, `actions.rs` (+ `actions/tests.rs`),
  `signatures.rs`, `validate.rs`, `source_map.rs`, `shell_template.rs`, and
  `composition/authored_order.rs`. **New:** `lifecycle/mod.rs` (`LifecycleSignal`,
  `LifecycleConfig`, `LifecycleStacks`, `LIFECYCLE_EVENT_KEYS`,
  `LifecycleParseOptions`, the neutral parse entry points, and the four
  `LifecycleSignal` methods that used to live in Claudine's `audio.rs`, because
  Rust allows inherent methods only on a crate's own types), `lifecycle/error.rs`
  (`LifecycleError`, `ActionExprError`), and `lifecycle/tests.rs`. Darkmatter's
  `json_type_name` became `pub(crate)` so the parser reuses it rather than a
  copy.
- **Errors:** `LifecycleError` has the 26 authoring variants the moved parser
  raises, with the same names, fields, and `Display` text as their
  `CompositionError` twins. Claudine keeps the twins (its diagnostic catalog,
  renderer, and test assertions are keyed on them) and maps with one exhaustive
  `From` impl in `claudine/lib/src/composition/error/lifecycle_authoring.rs`.
  Parity tests there: `every_lifecycle_error_converts_to_its_same_named_twin_with_the_same_display`
  and `a_typed_cause_survives_the_conversion`. `ActionExprError` moved and is
  re-exported at its old path.
- **Claudine bridge (for the slice):** Claudine's `lifecycle/mod.rs` re-exports
  every moved item at its old path (`actions` and `shell_template` as module
  re-exports) and keeps its public wrappers' signatures (`parse_lifecycle_config`,
  `parse_loop_gate`, `parse_task_action_stack`, `parse_single_action`, …),
  which pass Claudine's Playa effect catalog and `UnknownVerbPolicy::Reject`
  and map errors. **Two validators stay in Claudine** in the new
  `lifecycle/binding_checks.rs`: `validate_no_err_in_no_error_events` and
  `find_removed_loop_read` read Claudine's binding catalog (`err` and `loop`
  availability rules are host policy until Phase 3's registry), so they use
  Darkmatter's walkers. `LifecycleErrNotAvailable` therefore stays a
  Claudine-only error.
- **Unknown verbs:** `LifecycleParseOptions.unknown_verbs: UnknownVerbPolicy`
  (`Reject`, the default and Claudine's, or `Ignore`). Under `Ignore` an
  unknown verb is `LifecycleActionKind::Ignored(IgnoredAction { verb, path })`,
  and its operand is never parsed, evaluated, or scanned for shells, spans, or
  `err` reads; known verbs and `when`/`then`/`else` structure still validate.
  Claudine's executor treats `Ignored` as a commented no-op (its profile never
  produces one). Tests: `darkmatter` `lifecycle::tests::{reject_refuses_an_unknown_verb_with_the_closed_grammar_error,
  reject_is_the_default_policy, ignore_keeps_an_unknown_verb_as_an_opaque_action_with_its_name_and_path,
  ignore_never_interprets_an_unknown_verbs_operand, ignore_still_rejects_a_malformed_known_verb,
  ignore_still_validates_when_then_else_structure}`.
- **Guards (path-only updates):**
  - `claudine/lib/tests/l1/lifecycle_lift_ready_guard.rs` reads the six slice
    files at their Darkmatter paths. Its assertions and forbidden list are
    unchanged, and `claudine/lib/Cargo.toml` lists them as CI `source-inputs`.
  - `claudine/lib/tests/l1/type_scope_ownership_guard.rs`: Darkmatter's own
    guard only reviews `TypeScope::unguarded`, so moving the three entries
    there would have dropped the `DeclaredTypes`/`Assume`/`PresenceLookup`
    reviews. Instead Claudine's guard now also scans
    `darkmatter/lib/src/lifecycle/`, where Claudine's lifecycle walk now lives,
    and keeps the three entries with their function, count, and reason
    unchanged and their paths now `darkmatter/lib/src/lifecycle/{actions,shell_template}.rs`.
    All 7 guard tests pass.
  - `claudine/cli/tests/l1/error_guards/transport-allow.toml`: the
    `actions.rs::type_with_value` entry was removed because that guard scans
    Claudine source only and fails on a stale entry. The code is unchanged, but
    no error-text guard covers it while it lives in Darkmatter.
- Scoped results: darkmatter `--lib` lifecycle 43/43; darkmatter `l1` lifecycle +
  guard 91/91; claudine `--lib` lifecycle/error/looping/preflight 1331/1331;
  claudine `l1` lifecycle + guard 41/41 (7/7 after the guard repair);
  claudine-cli `l1` traces/directive matrix/guarded shells 94/94; doctests 6 + 1;
  `just lint` darkmatter and claudine: pass.

### Engine and service extraction design (Waves 5–6)

Decided before Wave 5, from reading the code at `d48d7dcf0`:

- **What Phase 2 moves is the running implementation**, not the prototype.
  The prototype stays test-only. The executor (`StackExecutionContext` and its
  action dispatch), the run guard (`LifecycleRunGuard`: initialize/start/
  terminal/finalize accounting, the one terminal slot, shell lock, proxy
  reset), the catch protocol and transition decision (`runtime.rs`), control
  admission (`control.rs`), and the `err`/`timing` envelope (`context.rs`)
  move to `darkmatter::lifecycle`. Each Claudine dependency becomes a neutral
  service seam that Claudine implements. Rewiring every driver onto a
  request/reply `Run` is Phase 3's "Driver migration": doing it here would
  leave the prototype-shaped `Run` and the moved guard as two engines for the
  length of the phase, which the spec forbids. Recovery and cleanup modes
  become named types where today's code passes booleans.
- **Services** (`darkmatter::lifecycle` traits, Claudine adapters): effects and
  communication (the moved emitter trait has no messaging or TTS parameters,
  and Claudine's adapter holds `RuntimeMessagingSettings` and `GlobalSettings`
  and keeps its existing configuration-carrying emitter trait, so the current
  test doubles and their assertions stay); shell running (the `ShellRunner`
  trait moves, while `SystemShellRunner`, `DocumentShellRunner`, and structured
  execution stay with Claudine's process code); runtime-state writes, loop
  values and the `break` slot, the `prep` runner, and cancellation probes, each
  through a trait. A missing required service is a typed error.
- **Error envelope:** the neutral envelope (code, category, disposition,
  origin, severity, detail, message, one-level cause, source location) is
  Darkmatter data. Claudine's effective-diagnostic selection projects into it
  (constructors that take `ClaudineError`, `HarnessError`, `CompositionError`,
  or `Diagnostic` stay in Claudine as extension traits), so rendering, machine
  output, and `err` keep reading one value.

### Wave 5a — Service adapters (done)

- **Error envelope** moved to `darkmatter/lib/src/lifecycle/envelope.rs`
  (`DiagnosticSnapshot`, `DiagnosticCause`, `DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION`,
  `concise_message`/`concise_message_with_suffix`/`CONCISE_MESSAGE_MAX_CHARS`,
  the `DiagnosticCatalog` service) and `lifecycle/context.rs`
  (`LifecycleErrorInfo`, `LifecycleCause`, `LifecycleTiming`,
  `LifecycleEvaluationReason`); `LifecycleExprError` moved to
  `lifecycle/error.rs`. All are re-exported at their old Claudine paths. Field
  names, serde attributes, `Display`, and `to_value` output are unchanged; the
  one `to_value` projection lives in Darkmatter.
- **Catalog-dependent constructors** stay in Claudine as extension traits:
  `DiagnosticSnapshotExt` (`from_diagnostic`, `select`, `from_code`),
  `DiagnosticCauseExt`, `LifecycleErrorInfoExt` (`from_claudine_error`,
  `from_harness_error`, `from_composition_error`, `from_proxy_commit_error`,
  `from_error_or_action`, `from_expr_error`, `from_action_failure`), and
  `CancellationCauseExt::error`. Each delegates to a Darkmatter constructor
  that takes a `&dyn DiagnosticCatalog` (`*_with`), passing
  `ClaudineDiagnostics`, so the projection logic exists once. Call sites gained
  only a trait import.
- **`frontmatter_excerpt`** is now `Option<SourceExcerpt>`: an opaque,
  never-serialized `Arc<dyn SourceAppendix>` with value equality. Claudine's
  `FrontmatterExcerpt` stays in Claudine (its line location reads Claudine's
  property paths) and implements `SourceAppendix`; `RestoredDiagnostic`
  renders it unchanged.
- **`concise_message`** hygiene moved with the envelope; Claudine's
  `harness::runtime::sanitize_message` delegates to it, so provider failures
  and typed errors still share one rule.
- **Effects:** `darkmatter::lifecycle::effects::LifecycleEffects` (text only:
  `stdout`, `stderr(signal, …)`, `info`, `warn`, `success`, `message`,
  `speech`, `effect`, `notification`), `dispatch_communication`, and the render
  helpers `render_prose_line`/`render_status_line`. Claudine's
  `LifecycleEmitter` and `DefaultLifecycleEmitter` keep their signatures and
  now render through those helpers. `lifecycle/effects.rs`'s
  `ClaudineLifecycleEffects` adapter resolves the TTS configuration (once per
  adapter) and the messaging route; the executor's `emit_communication` and
  `on_error` warnings and the run guard's `emit_signal` all go through it.
- **Services:** `LifecycleServices { effects, diagnostics }` returns
  `MissingServiceError { service: LifecycleService }` for an absent service.
  Nothing in Claudine constructs it yet; it is the bundle the moved executor
  takes.
- **Cancellation:** moved, not wrapped. `CancellationToken`,
  `CancellationScope`, `CleanupScope`, `current()`, and `CancellationCause`
  live in `darkmatter::lifecycle::cancellation` with the same thread-locals;
  the probes are `cause_with(interrupted)`/`requested_with(interrupted)`, and
  Claudine's `cancellation::{cause, requested}` pass `crate::interrupt::interrupted`.
  The probe is an argument rather than a registered global, so no
  initialization order can drop a user interrupt.
- No guard needed a path update: the error guards, lift-ready guard, and
  type-scope guard scan nothing that moved.

### Wave 5b — Executor extraction (done)

- **Moved** to `darkmatter/lib/src/lifecycle/executor.rs` (engine) with
  `executor/{outcome,shell,host}.rs`: the whole action dispatch (ordered
  nodes, `when`/`then`/`else`, `set` snapshot semantics and shell values,
  deferred spans, event-time interpolation, control directives, `no_error`,
  `on_error`, `proxy`/`retry` `with:`, `break`, `prep`, side effects through
  `EffectEngine`, template-mode shells), `StackControl`,
  `LifecycleEventOutcome`, `ActionLocation`, `ShellRunError`, `ShellRunner`,
  `ShellValueRequest`, `execute_set_shell_values`, `DisabledShellRunner`,
  `compose_request`, `evaluate_operand_in`, `is_template_operand`. All are
  re-exported at their old Claudine paths (`ActionLocation` from
  `coordinator/handoff.rs`, `compose_request` from `composition/mod.rs`).
- **Darkmatter API:** `StackInputs` (per-event neutral inputs) and
  `StackExecutor::{new(inputs, LifecycleServices) -> Result<_, MissingServiceError>,
  with_services}`, with `execute_event`, `execute_action_stack`,
  `evaluate_guard`, `dispatch_task_side_effect`, `eval_expr`,
  `resolve_string_value`, `resolution_context`. Effects and diagnostics are
  both required; a missing one is a `MissingServiceError` at construction.
- **Host seam** `LifecycleHost`: `bindings` (the whole availability decision
  stays Claudine's binding catalog), `name_coercion_keys`,
  `resolution_context`, `capture_overlay_context`, `persisted_data`,
  `commit_set`, `in_loop`/`record_break`, `prep` (`PrepRequest` →
  `PrepError::{Unavailable, Failed}`), `user_blacklist`, `interrupted` (fed to
  `cancellation::requested_with`), `is_known_effect`, and `failure`, which
  classifies a `LifecycleRunFailure` (seven variants mirroring the
  `CompositionError` twins the executor raised) in the host's catalog.
- **`ShellRunner::for_child_environment`** now takes Darkmatter's
  `ShellChildEnvironment`; the one Claudine call site passes
  `env_baseline().shell_child_environment()` and Claudine's runners rebuild an
  `EnvBaseline` from its entries.
- **Claudine adapter:** `StackExecutionContext` keeps every field, so its 45
  construction sites and every test are unchanged. It implements
  `LifecycleHost`, builds one `ClaudineLifecycleEffects` per run (the TTS
  configuration is now resolved once per run rather than once per action;
  settings are immutable, so the result is the same), and delegates every
  dispatch method. `SystemShellRunner`, `DocumentShellRunner`, structured
  execution, `TaskGuardFailure` (parsed in the adapter, evaluated by
  `evaluate_guard`), `PrepCall`/`PrepRunner`, and `LoopBreak` stay in Claudine.
- **Departure from the plan sketch:** call sites were not rewritten onto the
  Darkmatter type; the adapter is the "constructor/helper" and keeps the
  driver rewiring for Phase 3. The `capture_proxy_with_fallback` test probe
  stays in Claudine behind `LifecycleHost::capture_overlay_context`.
- **Guards:** `claudine/cli/tests/l1/composition_seams.rs` now also scans
  `darkmatter/lib/src/lifecycle/` (root-anchored join); its
  `lifecycle::executor::{interpolate_text_in, early_binding_context}` entries
  are unchanged because the module identity is the same. The type-scope guard
  already scanned that directory. The four executor files are `source-inputs`
  of `claudine` and `claudine-cli` (`RealWorkspaceTestInputTests` 7/7).
  Test edits: `executor/tests/mod.rs` gained imports only.

### Wave 5c — Run guard, catch protocol, control admission (done)

- **Moved** to `darkmatter/lib/src/lifecycle/`: `control.rs` (`ControlDispatch`,
  `decide_control`, `control_budget_for`, `compute_backoff_delay`,
  `MAX_PROXY_HOPS`, `proxy_path_identity`, `proxy_handoff_allowed`,
  `parse_delay`); `runtime.rs` (`LifecycleTransition{Input,Error,Abort,Decision}`,
  `decide_lifecycle_transition`, `LifecycleCatch{Execution,Step,Result,State,Protocol}`,
  `ContainerEventDecision`, `ContainerEventResult`, `ContainerRunner`); and
  `run_guard.rs` (`LifecycleRunGuard` with its accounting, terminal slot,
  provider-launched fact, shell lock, resets, `redesignate_terminal_to_failure`,
  `run_event_stack`/`execute_event`, `emit_*_once`, `defuse`, run-prepared
  context, `Drop`, and `cancel_if_requested`). **New:** `duration.rs`
  (`parse_duration_secs`, `DurationSyntaxError`), the one `{number}{unit}`
  grammar; Claudine's `harness::parse_timeout{,_allow_zero}` map its error to
  `HarnessError::InvalidTimeout` with the same `detail` text.
- **Darkmatter API:** `LifecycleRunHost` (`source_path`, `prepared_context`,
  `effects`, `diagnostics`, `interrupted`, `enter_terminal_event` →
  `TerminalScope`, `failure`); `GuardedEventStack::run_guarded(GuardedEvent {
  signal, config, shell_runner, error })`, where the guard passes
  `DisabledShellRunner` until `start` and the cancellation as `err`;
  `LifecycleRunGuard::with_host`/`host`; `LifecycleCatchProtocol::with_catalog`;
  `ContainerRunner<'a, H>::mode`; `LifecycleRunFailure::CancellationRecovery`;
  `EvaluationMode { NormalRecovery, FatalEvaluationCleanup, RefusalCleanup,
  CancellationCleanup }`, carried as `LifecycleCatchStep::mode` and used for
  cancellation cleanup in the guard and `ContainerRunner` (the refused-recovery
  and explicit-`error` warnings are one `cleanup_warnings` for both).
- **Claudine adapters:** `LifecycleRunGuard<'a>` and `ContainerRunner<'a>` are
  aliases over `ClaudineRunHost` (runtime context, emitter, one
  `ClaudineLifecycleEffects` per run, `ClaudineDiagnostics`,
  `TerminalLifecycleScope`, `executor::classify_run_failure`, now shared with
  `StackExecutionContext::failure`). `LifecycleRunGuardExt` keeps `new(config,
  &ctx, emitter)`, `context()`, and `emitter()`; `LifecycleCatchProtocolExt`
  keeps `new(origin, state, prior, outcome)`. Call sites gained only those trait
  imports. `StackExecutionContext` implements `GuardedEventStack` through
  `with_signal` (a new event starts a new `current` memo) and `with_error`.
  `LifecycleRuntimeContext`, `resolve_proxy_target` (it returns `HarnessError`),
  and `IterationSummarySignals` stay in Claudine: the last is provider facts the
  CLI loop consumes, not an engine input, so the engine no longer imports
  `stream::summary`. Everything is re-exported at its old path.
- **Behavior notes:** the drop safety net's `emit_*` now reuses the run's one
  effects adapter instead of building one per emission (settings are
  immutable, so the TTS configuration is the same); `cancel_if_requested` warns
  through that adapter, whose terminal is the runtime context's (every caller
  builds its event context from the same one).
- **Guards:** `path_lookup_guard`'s `proxy_handoff_allowed` exception
  (`std::fs::canonicalize`, 2, same-file verdict) moved from Claudine's guard to
  `darkmatter/lib/tests/l1/path_lookup_guard.rs` with item, operation, count,
  and reason unchanged; Claudine's guard scans only `claudine/lib`, and
  Darkmatter's already scans the new home. `control.rs`, `run_guard.rs`, and
  `runtime.rs` are `source-inputs` of `claudine` and `claudine-cli`. No other
  guard needed an edit.
- **Tests:** the spike imports the moved types from `darkmatter::lifecycle::…`
  plus `LifecycleCatchProtocolExt`/`LifecycleRunGuardExt`; assertions unchanged.
  New Darkmatter tests with a fake host: `run_guard::tests` (6),
  `runtime::tests` (3), `control::tests` (3).
- Scoped results: darkmatter `--lib` lifecycle 86/86; darkmatter `l1` lifecycle
  84/84 (+ guard 62/62); claudine `--lib` 1720/1720; claudine `l1` 47/47;
  claudine-cli `l1` 1426/1426; claudine-cli bin 1370/1370; doctests 10 + 1;
  `just lint` darkmatter and claudine: pass.

### Wave 5d — Gated request/reply `Run` (done)

- **New:** `darkmatter/lib/src/lifecycle/run.rs` (`Run`, `RunSettings`,
  `RunSnapshot`) and `run/protocol.rs` (`Ticket`, `RunId`, `Request`,
  `RequestKind` {`EvaluateEvent(EventRequest)`, `Prepare`, `PerformOperation`,
  `AwaitCompletion`, `Recover(RecoveryRequest)`, `Done(Finish)`}, `Reply`
  {`Event(LifecycleEventOutcome)`, `Gate { outcome, iterate }`, `Prepared`,
  `Operation`, `Recovery`}, `Preparation`, `OperationReport`,
  `CompletionVerdict`, `RecoveryReply` {`Budget { ceiling }`, `Adopted`,
  `Refused`, `Aborted`}, `HostFailure { stage, entry, error }`,
  `FailureStage`, `EntryReason`, `ApprovalEpoch`, `SessionAvailability`,
  `RunError`, `ReplyError`, `CancelError`, `ShellRefusal`, `AuthoredEvents`,
  `TranscriptEntry`). `Run::{start, pending, reply, answer, cancel,
  shell_permission, finish, selected_error, terminal, accounting, snapshot,
  transcript}`. The cancellation error is a `RunSettings` value; the catalog
  that classifies an explicit `error(...)` is the other. Events are the fixed
  `LifecycleSignal` set; nothing is keyed on a host string.
- **One copy of the logic:** the guard's six flags moved into
  `run_guard::RunAccounting` (record once, the one terminal slot, redesignate,
  provider launched, defuse, both resets, the owed safety-net signal);
  `LifecycleRunGuard` holds one and delegates with an unchanged API, and `Run`
  holds one. Routing is `decide_lifecycle_transition` (catch trigger, control
  admission through `decide_control`, the finish verdict) and
  `LifecycleCatchProtocol` (setup catch, cleanup modes, evaluation
  precedence); modes are `EvaluationMode`. The explicit-error default text is
  one `runtime::explicit_error`, used by the catch protocol and the success
  downgrade. Engine rules that are new code: request sequencing, the stage
  table that picks `blocked`/`failure` for a host failure, which events admit
  which recovery, and ticket/cancellation checks.
- **Host facts, not engine state:** document state and `set`, stacks, the
  retry/resume ceiling (`RecoveryReply::Budget`, which the run feeds to
  `decide_control`; it replaces the prototype's `Admitted`/`Exhausted`),
  proxy targets and path identity, the `loop` verdict, and `err` binding
  availability. A resume without a session is now the run's decision
  (`RunError::Aborted(ResumeWithoutSession)`) once the host's budget admits
  it, as in production; the host no longer answers `Aborted` for it.
- **Replay:** `lifecycle_boundary/replay.rs` drives the production `Run`.
  **32/32 traces reproduce line for line**, plus
  `every_recorded_trace_has_a_replay`; no trace or fixture changed.
- **Protocol tests:** 17 prototype tests ported to
  `darkmatter/lib/tests/l1/lifecycle_run_protocol.rs` (the downgrade test
  split in two, so 18), plus
  `a_resume_without_a_session_ends_the_run_without_finalize` and
  `concurrent_runs_share_no_state` (20). Facts that could not be ported: a
  request's stack and state, `set` mutations, and the `goal` state of an
  adopted document (host-owned now); the adopted document's rename check and
  per-event action types (registry phase; kept as prototype tests
  `an_adopted_document_is_split_by_the_registry` and
  `registered_action_types_refine_shell_permission`).
- **Finding (untraced):** the prototype let `failure` recover after an
  explicit `error(...)` in a preflight `blocked`. Production's catch protocol
  re-designates the slot and runs `failure` as `RefusalCleanup`, so its
  recovery is not honored; the ported test is now
  `an_explicit_error_in_preflight_blocked_is_caught_by_failure_as_cleanup`.
  Two smaller production rules replace prototype behavior no test asserted:
  an action error in a terminal-phase event leaves the selected error alone,
  and a failure during cancellation cleanup does not replace the cancellation.
- **Prototype retired:** the engine's cancellation, retry/resume, iteration,
  host-refusal, and await-completion paths and their types, the Ticket
  accessors, and `Registry::set_cancellation_error`. **Kept** (registry
  phase): `engine.rs` registry and its minimal run, `profiles.rs`,
  `publisher.rs`, and the registration, rename, `md`-profile, `err`-view, and
  action-type tests in `protocol_tests.rs`; `mod.rs` says which is which.
- **Claudine:** no driver uses `Run`; behavior unchanged. `run.rs` and
  `run/protocol.rs` joined the `source-inputs` of `claudine` and
  `claudine-cli` (their guards scan `darkmatter/lib/src/lifecycle/`).
- Scoped results: darkmatter `--lib` lifecycle 86/86; darkmatter `l1`
  lifecycle 89/89; claudine `--lib` lifecycle/looping/sequence 1550/1550;
  claudine `l1` lifecycle/guard/spike 47/47; claudine-cli `l1`
  `lifecycle_traces` 33/33; `just lint` darkmatter and claudine: pass.

### Wave 6 — Claudine bridge (done)

- **Dependency-direction guard (criterion 3):**
  `darkmatter/lib/tests/l1/dependency_direction_guard.rs` (declared in
  `tests/l1/main.rs`). From `darkmatter`, `darkmatter-cli`, and `dmls`, it
  walks every dependency table (`dependencies`, `dev-dependencies`,
  `build-dependencies`, `target.*`), resolves `workspace = true` through the
  root `[workspace.dependencies]` (none exists today), follows `path`
  dependencies to their manifests, and fails on a Claudine package name
  (`claudine`, `claudine-cli`, `claudine-contract`, `claudine-gen`,
  `claudine-catalog-types`, `rendezvous-*`, including a `package =` rename) or a
  manifest under `claudine/`. A transitively reached package contributes only
  the tables Cargo builds for a dependency (not its `dev-dependencies`). It
  also scans the three crates' production sources (`source_scan`, comments,
  literals, and test code blanked) for a Claudine crate in a path, a `use`, or
  an `extern crate`. Self-tests inject a Claudine dependency (a renamed
  `cfg(unix)` path dependency one hop away, and a renamed workspace entry) and
  a `use claudine::…;`, and check that each is reported. The real walk asserts
  that it reached `biscuit-file/lib/Cargo.toml`. It uses the existing `toml`
  dev-dependency; no dependency was added. The manifests and `source_scan.rs`
  are `include_str!` literals, so CI's test-input index maps a manifest edit to
  the guard. 4 tests, 0.5 s in total (the source scan).
- **Untraced recovery routes, now traced** (`claudine/cli/tests/l1/lifecycle_traces.rs`,
  35 scenarios; Phase 1 called them open):
  - `retry-in-initialize`: `event initialize`, `exit 1`. The retry raises
    `LifecycleSetupPhaseRecoveryUnsupported` (`initialize`, `retry`) and the
    run ends with no catch event and no `finalize`.
  - `retry-in-preflight-blocked` (shell audit denied): `event initialize`,
    `event blocked`, `err composition.shell_approval`, `event finalize`,
    `err composition.shell_approval`, `exit 1`. `finalize` runs reading the
    blocking error; afterwards the retry becomes
    `LifecycleSetupPhaseRecoveryUnsupported` (`blocked`, `retry`), which is
    the reported error.
  - `retry-in-start` (before launch): `event initialize`, `event start`,
    `event start`, `work fresh 0`, `event success`, `event finalize`,
    `err none`, `exit 0`. The retry re-runs preflight (attempt 2) without
    `initialize` or `finalize`. The second `start`'s retry finds the budget
    spent and the run continues to launch.
  - Against spec section 8 and the prototype: the prototype ran refusal
    cleanup (`finalize`) after a retry in `initialize`; production does not.
- **`Run` changed** (`run.rs::admits_recovery`/`request_recovery`; new
  request-sequencing code, not Claudine knowledge). All three replays failed
  first: the run finalized after an `initialize` retry, showed
  `SetupRecoveryUnsupported` to a preflight `blocked`'s `finalize`, and refused
  a `start` retry. Now `start` admits every recovery, an `initialize` retry
  ends the run without `finalize`, and a preflight `blocked`'s `finalize`
  reads the blocking error (`fire_reading`), while the selected error is still
  `SetupRecoveryUnsupported`, so `blocked_reached_from_preflight_cannot_recover`
  passes unchanged. `initialize` also admits resume (refused without a
  session) and hand-off, following Claudine's `initialize` dispatch in
  `pipeline.rs`. Those two rows are **untraced**. The
  `EventRequest::error` doc was updated. New protocol test
  `setup_phase_recovery_follows_production`. **35/35 replays** reproduce, and
  `every_recorded_trace_has_a_replay` passes.
- **Still divergent (untraced, unchanged):** production reports `resume` in a
  preflight `blocked` as `LifecycleResumeWithoutSession`. `Run` reports
  `SetupRecoveryUnsupported`, which an existing protocol test asserts.
- Scoped results: darkmatter `l1` guard + lifecycle 97/97; darkmatter `--lib`
  lifecycle 86/86; claudine-cli `l1` `lifecycle_traces` 36/36; `just lint`
  darkmatter and claudine: pass; `just check-tier-coverage darkmatter`: pass.

### Phase 2 checkpoint and gates

All gates ran once, sequentially, after every subagent finished and with
nothing else running.

- `cd claudine && just test`: **10,587 run, 10,587 passed** (49 slow, 9
  skipped), 331.7 s. Against the entry gate's 10,606: 26 tests moved to
  Darkmatter with their files (`actions/tests.rs` 21, `shell_template.rs` 3,
  `authored_order.rs` 2) and 7 were added (2 guarded-shell traces, 3
  retry-route traces, 2 error-parity tests): 10,606 − 26 + 7 = 10,587. All 35
  `lifecycle_traces` pass, as do the type-narrowing conformance matrix
  (`executor/tests/scope_site_conformance.rs`, `sequence/preflight/tests.rs`)
  and `type_scope_ownership_guard`, with their assertions unedited.
- `cd claudine && just lint`: pass. `just check-tier-coverage claudine`: 0 stranded.
- `cd darkmatter && just test`: **10,889 run, 10,889 passed** (10 skipped)
  on the third run. The first two runs each failed one test,
  `dmls::l1 searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`
  ("the watch set changed, so it is registered again", left 3, right 2). It is
  **pre-existing and load-sensitive**: it passed alone 3/3, and in a
  temporary worktree at `HEAD` (`d48d7dcf0`, without this phase's changes,
  separate target dir) the DMLS binary failed it in 2 of 7 runs. Nothing in
  this phase touches DMLS. The worktree was removed afterward.
- `cd darkmatter && just lint`: pass. `just check-tier-coverage darkmatter`: 0 stranded.
- **Dependents.** Darkmatter's public API changes are additive (the new
  `lifecycle` module, plus `compose_request`, which moved into it), and
  Claudine's old lifecycle paths are re-exported.
  `cargo check --all-targets` of every direct dependent of `darkmatter` or
  `claudine` (`biscuit-icon-cli`, `biscuit-speaks-cli`, `messenger`,
  `darkmatter-cli`, `claudine-gen`, `playa-cli`, `messenger-cli`, `dmls`,
  `sniff-cli`, `research`, `claudine-contract`): clean.
  `cargo nextest run -p claudine-contract`: 48/48. `claudine-cli`, `dmls`, and
  `darkmatter-cli` ran inside the two area suites.
- **Spike tests (criterion 19):** both `lifecycle_control_flow_spike` tests
  pass. Claudine's imports point at `darkmatter::lifecycle` for moved types;
  `spike/model.rs` and `spike/scenarios.rs` did not move.

#### Checkpoint criteria

| Criterion | Evidence |
|---|---|
| 2 (no Claudine behavior change) | every pre-existing Claudine assertion passes unedited (only imports and include paths changed in tests); 35/35 traces unchanged; 35/35 replays through the production `Run` |
| 3 (dependency direction) | `darkmatter::l1 dependency_direction_guard` (manifests of lib, CLI, DMLS plus path-dependency closure; sources; self-tests) |
| 10 (section 8 behavior under the new owner) | the D1–D8 trace rows replay through Darkmatter's `Run`, and Claudine runs them through Darkmatter's `LifecycleRunGuard`/`decide_lifecycle_transition`/`LifecycleCatchProtocol` |
| 19 (spike tests) | both spike tests pass; imports repointed |
| "no moved module imports a Claudine type" | `dependency_direction_guard` source scan; `lifecycle_lift_ready_guard` over the slice |

#### Requirement → test mapping (Phase 2)

| Requirement | Tests |
|---|---|
| Entry gate: guarded hole present / nulled at a reached shell | `claudine-cli::l1 lifecycle_traces::{guarded_shell_present, guarded_shell_null}`; `darkmatter::l1 lifecycle_boundary::replay::{guarded_shell_present, guarded_shell_null}` |
| Model extraction: one parser, same errors | Claudine's unchanged lifecycle suites through the wrappers; `composition::error::lifecycle_authoring::{every_lifecycle_error_converts_to_its_same_named_twin_with_the_same_display, a_typed_cause_survives_the_conversion}` |
| Opaque ignored action (`md` profile) | `darkmatter lifecycle::tests::{reject_*, ignore_*}` (6) |
| Engine extraction: executor, run guard, catch, admission, modes | `darkmatter lifecycle::{executor,run_guard,runtime,control}::tests` (fake hosts), plus every Claudine lifecycle/looping/sequence/CLI test through the adapters |
| Gated request/reply: stale/duplicate/foreign/wrong-kind/late replies, cancellation invalidation, per-subject isolation | `darkmatter::l1 lifecycle_run_protocol` (21, including `concurrent_runs_share_no_state`, `setup_phase_recovery_follows_production`) |
| Guarded hole filled by the moved executor, `MissingValue` before launch | `guarded_shell_null` trace; `guarded_shell_values` CLI tests; `executor/tests/guarded_shell_sites.rs` |
| Services: missing service is typed; envelope and render parity | `darkmatter lifecycle::{services,envelope,context,effects,cancellation}::tests` |
| Untraced recovery routes | `lifecycle_traces::{retry_in_initialize, retry_in_preflight_blocked, retry_in_start}` + replays |
| Dependency guard | `darkmatter::l1 dependency_direction_guard` (4) |

Input Robustness Matrix: not applicable to Phase 2. No catalog or trigger
loader changed; the lifecycle text grammar is excluded by the plan.

#### Decisions and departures recorded this phase

- **Request/reply driving is built, but Claudine's drivers do not use it
  yet.** The plan's Wave 5 asks for the gated API, and it exists as
  `darkmatter::lifecycle::run::Run`, sharing `RunAccounting` and the routing
  functions with `LifecycleRunGuard`, so there is one engine. Claudine's
  drivers still call the guard directly; rewiring them is Phase 3's "Driver
  migration", as the plan schedules it.
- **Error twins:** `LifecycleError` (Darkmatter) and Claudine's same-named
  `CompositionError` variants coexist, joined by one exhaustive `From` and a
  `Display`-parity test. Phase 3's "delete superseded" task can fold them.
- **Kept in Claudine, behind seams:** the binding catalog's availability
  rules (`err`, `loop`, and the `seq`/`group` roots, read through
  `LifecycleHost`), `validate_no_err_in_no_error_events` and
  `find_removed_loop_read` (`binding_checks.rs`), `LoopBreak`/`LoopBreakSlot`,
  `PrepRunner`, `RuntimeState`, `FrontmatterExcerpt`, `IterationSummarySignals`,
  `resolve_proxy_target` (returns `HarnessError`), process and shell execution,
  and the lifecycle pre-flight walk (`composition/preflight.rs`, Phase 3).
- **Guard coverage moves:** `type_scope_ownership_guard` and
  `composition_seams` now also scan `darkmatter/lib/src/lifecycle/`; the
  `path_lookup_guard` exception for `proxy_handoff_allowed` moved to
  Darkmatter's `path_lookup_guard` unchanged. The `transport-allow.toml` entry
  for `actions.rs::type_with_value` was removed, so no error-text guard covers
  that site in Darkmatter.
- **Open, untraced divergence:** production reports a `resume` in a
  preflight-reached `blocked` as `LifecycleResumeWithoutSession`, while `Run`
  reports `SetupRecoveryUnsupported` (an existing protocol test asserts the
  latter). `initialize` resume/hand-off rows in `Run` follow `pipeline.rs`
  but are untraced. A cancelled root run (exit 130) is still untraced.

#### Cross-OS check (Phase 2)

Moving code between crates carries the classic `-D warnings` risk of an import
or item used only under one `cfg`. Claudine's executor adapter keeps the
`cfg(windows)`/`cfg(not(windows))` system-shell launch code. So the phase ran
the native Windows rig (`build-win-native`) through `just cross-check`:

- `claudine` (lib, all L1): **5,519/5,519 passed**.
- `darkmatter`, filtered to `lifecycle dependency_direction path_lookup_guard type_scope`
  (every test this phase added or moved, plus the guards it edited): **228/228 passed**.
- `claudine-cli` (all L1 and bin): **3,673 passed, 9 failed**. All 9 are
  **pre-existing**: from a temporary worktree at `HEAD` (`d48d7dcf0`, without
  this phase's changes; removed afterward), the same rig ran exactly these
  tests and **all 9 failed the same way** (0/9 passed). None involves code
  this phase changed; the system-shell launch code is byte-identical to
  `HEAD`. All 9 tests arrived on 2026-10-20 with the type-narrowing review
  repairs and strict-equality fix (`13cbc87c6`, `455eb05e4`), after the
  last Windows evidence for this area:
  - `lifecycle_shell_templates::a_lifecycle_exact_key_keeps_a_quoted_empty_argument`
    and `sequence_shell_templates::sequence_task_exact_keys_keep_quoted_empty_arguments`:
    an exact-approved command runs its resolved bytes through the system
    shell, and `cmd.exe` passes `''` literally (`["-m", "", "-s", "''"]`
    instead of `""`). How an exact approval's empty argument should be
    parsed on Windows is the open author item that the type-narrowing review
    (`review-1.md`) records ("Earlier exact-approval handling of an empty
    argument remains an author item").
  - `implement_plan_commit_message::*` (7): the recorder installed as `git`
    records an extra `["--exec-path"]` first. `gix-path` runs
    `git --exec-path` on Windows to find the system Git configuration unless
    `EXEPATH` names a Git for Windows layout. The fixture's `git` is the
    recorder, so the probe lands in the recording, which the module doc says
    must fail the exact comparison. The `worktree` crate's workaround
    (`name_installation_for_gix`) needs a real launcher layout and would not
    apply here. Setting `EXEPATH` in the fixture would hide a real Windows
    `git` call that the test exists to catch.
- **No code fix was attempted.** The first family needs the author's
  exact-approval decision; the second needs either a production change to
  how Claudine's in-process Git discovery reads system configuration on
  Windows or a ruling on what the test may record. Neither belongs to this
  feature, and a test-side workaround would weaken a passing assertion.
  The spec carries `os_failing: windows` so the pipeline routes them for
  Windows-side repair.
