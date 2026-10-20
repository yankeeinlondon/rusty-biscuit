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
human_review: true
human_review_items:
    - |-
        **A dependency this feature names has not been built yet: `2026-10-16-type-narrowing`.**

        The spec relies on it for two things. In Phase 3, the shell-approval check must read whether a value is guaranteed present inside a `when:` branch (the `TypeScope` type). Acceptance criterion 23 also needs its test matrix to pass. Today that spec is still a draft: nothing is implemented, and no `TypeScope` type exists in the code. Phase 2 (moving the engine into Darkmatter) does not need it. Phase 3 does, so the decision is cheapest now, before more work builds on the current plan.

        - **Option A: build `2026-10-16-type-narrowing` before Phase 3.** Pro: the spec is delivered as written, including criterion 23. Con: adds a separate fix to the critical path and delays Phase 3.
        - **Option B: keep going, and in Phase 3 keep today's approval behavior** (no narrowing-aware approval), marking criterion 23 and the narrowing paragraph of section 4 as deferred to the narrowing fix. Pro: no delay; nothing that works today changes. Con: the feature ends with one criterion open, and a follow-up must revisit the shell audit.
        - **Option C: fold the narrowing work into this feature.** Pro: one delivery. Con: makes an already large feature larger and mixes two independent concerns.

        **Recommendation: Option B.** The extraction's main promise is "no Claudine behavior change", and narrowing-aware approval is new behavior. Deferring it keeps this feature a pure move plus the `md` host, and the narrowing fix can then land against the single engine Darkmatter will own.
    - |-
        **Phase 4 needs your ruling on how strictly two configuration files are read.** These are the new global schema catalog and the schema-trigger files.

        The plan proposes, and asks you to confirm before those readers are changed:

        1. The global catalog is read strictly. A missing, `null`, wrongly typed, or empty entry for any global (`fm`, `ctx`, `current`, `env`, `err`, `timing`, and Claudine's `seq`, `loop`, `group`) is an error, as are duplicate keys and trailing garbage.
        2. In trigger files, an explicit `null` for any trigger field is an error, never treated as "absent".
        3. A non-string trigger `kind` (for example `kind: 5`) is an error rather than "this is not a trigger".

        This is not needed for Phase 2 or 3. Phase 4 cannot finish those readers without it, so answering early avoids a stall later.

        - **Option A: accept all three as proposed.** Pro: invalid configuration is always reported where it is written, never silently ignored. Con: a hand-edited file with a stray `null` that loads today could start failing; the repository's own files will be migrated in the same change.
        - **Option B: accept 1 and 3, but keep `null` meaning "absent" in triggers.** Pro: more forgiving for trigger authors. Con: `null` and absence become indistinguishable, which the repository's input-robustness rules discourage.
        - **Option C: make the catalog lenient (fall back to defaults).** Con: the catalog is the single source of every global's shape, so a silent fallback hides real breakage in the editor, the CLI, and runtime alike.

        **Recommendation: Option A.** It matches the repository's input-robustness rules, and the repository has no external users whose files would break.
message_to_agent: |-
    Phase 1 is done and the Stage 1 gate PASSED (all 30 production traces replay exactly through the prototype). Read the implementation log's Phase 1 sections before Phase 2. Key facts:

    - Traces: `claudine/cli/tests/l1/lifecycle_traces.rs` + `claudine/cli/tests/fixtures/lifecycle_traces/*.trace` are the immutable behavioral baseline (unix-only). Never edit a `.trace` to make Phase 2/3 pass; acceptance criterion 2 requires them unchanged.
    - The prototype (`darkmatter/lib/tests/l1/lifecycle_boundary/`) is TEST-ONLY. The spec forbids two production engines, so do not promote it to a second engine beside Claudine's. Phase 2 should move Claudine's real executor/catch/guard into `darkmatter::lifecycle` and use the prototype's API shapes (log section "Boundary prototype" and "Prototype verdict") as the target boundary. Once the real engine exists, retire or rewire `replay.rs` to drive it, keeping the 30 replays green.
    - Production departs from spec section 8 in eight places (D1–D8 in the log). The code is authoritative; preserve these. In particular, D5 (resume in `start` without a session aborts with no failure/finalize) and D7 (proxy target that fails to parse ends the run with no events) needed a distinct "aborted" reply, separate from "refused".
    - Untraced, so add a trace before relying on it: `retry` in `initialize` or from preflight-reached `blocked` (production hard-errors without finalize; the prototype runs refusal cleanup), `retry` from `start`, and a cancelled root run (exit 130).
    - `2026-10-16-type-narrowing` is not implemented (no `TypeScope`); see `human_review_items`.
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

