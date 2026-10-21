---
total_phases: 6
created: 2026-10-12
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
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
---

# Plan: Darkmatter lifecycle ownership

## Work and successful completion

Move Claudine's lifecycle grammar, neutral action definitions, executor, catch
routing, terminal slot, shell permission, and finalize accounting into
`darkmatter::lifecycle`. Keep provider execution, coordinator decisions, audio,
messaging configuration, and container scheduling in Claudine behind neutral
services and typed work replies. Then enable root lifecycle events in `md
compose`, make the embedded global catalog authoritative, and migrate schema
triggers to their final names and AND/OR semantics.

Success means Claudine preserves its prerequisite-revision behavior, proven by
unchanged test assertions and identical ordered traces; Darkmatter owns one
engine with no dependency on Claudine; `md` implements the specified effects,
output, failure, and cancellation contracts; passive tools remain passive;
runtime and editor typing share catalog and registry definitions; and migrated
triggers select the same documents. All 22 specification acceptance criteria
must have recorded evidence. Documentation and skills must describe the shipped
behavior, and scoped tests and lint must pass.

This is an implementation plan, not authorization to implement or commit now.
Implementation finishes ready for review; the author closes the feature.

Waves are globally numbered. Tasks in one wave may run concurrently only where
file ownership is disjoint; every wave waits for the preceding checkpoint.
The implementation lead integrates shared files. Individual tasks list any
additional dependencies. Use subagents for the explicitly parallel waves, with
bounded briefs stating that the entire session tree is non-interactive.

## Phase 1: Establish contracts and baseline

### Necessary Rules

- **Prerequisite gate:** start extraction only after
  `2026-10-26-task-inputs-and-providers` and its transitive prerequisites have
  landed. Record the actual revision, prerequisite evidence, and existing test
  inventory in `implementation-log.md`. The planning checkout still exposes the
  older lifecycle API; it is not the behavioral baseline for this extraction.
- **Authority:** the finalized specification governs the feature. Historical
  supporting documents and the existing spike are evidence, not alternate
  designs. If prerequisite-revision traces contradict section 8, preserve the
  running behavior, record the discrepancy and departure in the implementation
  log, and correct current docs. Do not silently rewrite the specification.
- **Ownership:** use `darkmatter::lifecycle` inside the existing library. A new
  crate needs a demonstrated dependency/build constraint and a logged reason.
  Hosts reply with facts; they never mutate engine accounting through callbacks.
  Provider/session facts and projected errors must replace Claudine-specific
  imports, including production-summary conversion currently in `runtime.rs`.
- **Known conclusions:** the historical spike already answers the request/reply
  architecture question. Do not repeat it or invent a performance study: the
  specification names no unresolved performance threshold or OS-specific cost.
  The Phase 1 prototype below answers the remaining production-equivalence and
  API-boundary question once, before extraction.
- **State boundaries:** distinguish run, iteration, attempt, approval epoch, and
  request generation. One captured `ComposeRequest` governs context and policy;
  recovery rereads do not recapture the process environment. Keep lazy `current`
  observations and captured `ctx` behavior distinct.
- **Schema scope:** retain three typed nesting levels and existing deeper runtime
  validation. No recursive-schema work, new expression predicates, scheduler,
  provider hooks, or redesigned cancellation/container semantics belongs here.
- **Input rulings (decided by the author, 2026-10-20):** the specification does
  not settle every malformed catalog and trigger-envelope shape, so the author
  ruled: the global and host catalogs load strictly (a missing, `null`, wrongly
  typed, or empty declaration, a duplicate key, or trailing garbage is a typed
  error, never a fallback); an explicit `null` in any trigger field is an error,
  never treated as absent; and a non-string trigger `kind` is an error, not a
  non-trigger classification. The matrix below encodes these rulings; no
  further ruling is needed. Do not invent new supported file formats.
- **Execution:** no new speculative CI cells, interactive commands, audible
  playback, window focus, `cargo fmt`, commits, or lifecycle-directory moves.
  During implementation use package-area nextest recipes and the Test Toolkit.

### Wave 1 — sequential prerequisite work

- [x] **Baseline inventory**
  - Verify prerequisite behavior: flat grammar, namespaces, subject/container
    lifecycle, loops and flow control, task inputs/provider selection, schema
    action unions, and approved typed shell templates.
  - Record revision and test identities; map every acceptance criterion to an
    owner, fixture, and validation task. Inventory lifecycle callers in direct,
    inline, task, group, sequence, loop, and recovery paths.
  - Run the baseline Claudine L1 suite and both existing control-flow spike tests;
    record existing failures separately. Never weaken assertions to obtain parity.

### Wave 2 — parallel evidence and interface work

- [x] **Trace capture**
  - From the gated baseline, use fake services to record events, action order,
    state snapshots, selected errors, and shell-lock changes for every Stage 1
    scenario: success; both blocked routes; catch failures in each catch event;
    both terminal downgrades; finalize errors; admitted/refused retry and resume,
    including incompatible resume; adopted proxy and failed target bootstrap;
    loop/break; cancellation with/without handler; skip; nested sequence/group.
  - Add immutable fixtures and L1 assertions through production paths, not only
    the historical model. Keep audio silent and provider work deterministic.

- [x] **Boundary prototype**
  - Build an isolated deterministic non-agent host using proposed neutral
    descriptors and synchronous typed requests/replies. Register a
    before-composition notification, one renamed core event, and a complete
    success/failure outcome pair with an explicit downgrade target.
  - Exercise mutation, typed failure, unavailable `err`, initialization shell
    refusal, notification ordering, and outcome selection without Claudine types.
  - Define stable owned request payloads, failure stage/entry reason, named cleanup
    modes, service seams, approval-authority access, and registration validation.
    Probe stale, duplicate, foreign, wrong-kind, and cancelled-generation replies.
  - Coordinate with trace capture on scenario IDs; use separate fixture/code files.

### Wave 3 — sequential gate

- [x] **Prototype verdict**
  - Replay every captured trace through the proposed request/reply boundary and
    compare all recorded facts. Confirm one shell-permission owner and ergonomic
    service adapters without moving the provider harness or coordinator.
  - Record pass/fail and API decisions before Phase 2. On failure, record the
    concrete blocker and stop extraction. The historical model/scenario files
    remain in place; this spike is not rescheduled later.

**Checkpoint:** baseline and trace identities are fixed, the prototype passes,
and the gate verdict is reviewable. Acceptance criteria 1 and 7 originate here.

## Phase 2: Extract the neutral implementation

### Entry gate — sequential, before Wave 4

Phase 1 fixed its baseline before `2026-10-16-type-narrowing` was built. The
author ruled that the fix lands first, so Phase 2 opens by moving the baseline
onto the revision that closes it. These tasks change no production code.

- [x] **Prerequisite revision**
  - Confirm `2026-10-16-type-narrowing` has `implemented: true` on its plan and
    spec and a recorded final-phase close. If it does not, record that, set
    `human_review: true`, and stop the phase without extraction work.
  - Record the new starting revision and the fix's closing commits in the log
    as this feature's prerequisite evidence.
- [x] **Baseline replay**
  - At that revision, run `just test` in `claudine/` (it includes the 30
    `lifecycle_traces` tests), the Darkmatter `lifecycle_boundary` replays and
    protocol tests, and both `lifecycle_control_flow_spike` tests. Record the
    new test counts in place of Phase 1's.
  - Every trace must pass unedited. A trace that now differs was changed by the
    prerequisite, not by this feature: confirm that cause, recapture that trace
    and its replay, and log the before/after lines as a baseline change before
    any extraction. Never recapture a trace after Wave 4 starts.
- [x] **Inventory refresh**
  - Inventory the lifecycle code the fix added or changed: the scope-carrying
    lifecycle preflight walk, typed lifecycle templates, sequence task `when`
    scopes, guarded-hole (`HoleTiming::Guarded`) filling at a reached action,
    and any lifecycle site kind in its conformance matrix. Assign each to an
    owner using the spec's ownership table: the lifecycle walk's `TypeScope`
    extension moves to Darkmatter, the sequence task `when` extension stays in
    Claudine. Update the caller inventory and file list Waves 4–6 move.
  - List every file path the fix's guard test and allowlists name, so moves in
    this phase update those paths only.
- [x] **Guarded-shell trace**
  - Add a trace scenario and its replay for a lifecycle `shell` with a
    `{{ … }}` hole under a `when:` guard: once with the value present (the
    command runs with it), and once with the value `set` to `null` before the
    branch runs (the action fails with `MissingValue`, nothing runs, and the
    routing that follows is recorded). Express the failure in the replay as a
    host fact, not an engine rule keyed on names.

### Wave 4 — sequential foundational extraction

- [x] **Model extraction**
  - Move parser, action shapes, actions, signatures, validation, source maps, and
    their tests from `claudine/lib/src/composition/lifecycle/` to Darkmatter.
    Preserve typed diagnostics, source locations, mutation/snapshot semantics,
    and deferred operand identity.
  - Introduce registry/profile inputs rather than another manual name/root list.
    Keep neutral shapes for every existing verb; service support is host-specific.
  - Preserve Claudine's closed unknown-verb behavior and define an opaque ignored
    action representation for `md`, retaining name/span without interpreting
    unsupported operands or nested payloads. Shared control structure still validates.

### Wave 5 — parallel engine and service work

- [x] **Engine extraction**
  - Move executor, run guard, catch protocol, terminal selection, control admission,
    finalize accounting, and recovery modes into the gated request/reply API.
  - Reject invalid replies without state mutation; invalidate normal work before
    cancellation cleanup; prevent late replies from output, outcomes, shell unlock,
    and repeated finalize. Isolate mutable state per concurrent subject.
  - Preserve every failure-stage row and finalize/re-entry exception from baseline
    traces, including source-finalize suppression on accepted proxy handoff.
  - Carry the guarded-hole check with the executor: a reached shell still fills
    guarded holes through Darkmatter's shared template API and fails with
    `MissingValue` before launch. Do not add a second guard walk or fact source;
    the preflight walk itself moves in Phase 3.

- [x] **Service adapters**
  - Move neutral effect/emitter interfaces and the full neutral error envelope;
    retain TTS/Playa, messaging settings, provider facts, and effective-diagnostic
    projection in Claudine adapters. Missing required services are typed errors.
  - Preserve existing publication/drain behavior and diagnostic selection across
    rendering, machine output, and `err`; terminal output uses `TerminalRenderable`.
  - Use `FileReference` and captured resolution/fetch policy; grant no new authority.
  - Work in adapter files and agreed interfaces, leaving engine files to the other task.

### Wave 6 — sequential adapter integration

- [x] **Claudine bridge**
  - Repoint imports and drive the moved engine through adapters. Transitional
    exports may ease import migration but must delegate to the sole implementation.
  - Move tests with assertions intact; repoint both historical spike test imports
    without moving `spike/model.rs` or `spike/scenarios.rs`.
  - Add dependency guards covering Darkmatter lib, CLI, and DMLS manifests/imports.
  - Keep `2026-10-16-type-narrowing`'s guard test, allowlists, and conformance
    matrix green with assertions unedited; a moved file changes only its path
    there, and each allowlist entry keeps its reason.

**Checkpoint:** run both areas' L1 suites and focused trace replay. Every existing
Claudine assertion and Stage 1 trace (as re-baselined by the entry gate) passes
unchanged, as do the type-narrowing conformance matrix and guard test; no moved
module imports a Claudine type. Covers criteria 2, 3, 10, and 19.

## Phase 3: Install registry and unified preflight

### Wave 7 — parallel registry and preparation work

- [ ] **Host registry**
  - Register Darkmatter's five events and Claudine's `start`/`blocked` renames,
    success/failure outcome pair, downgrade targets, and post-finalize loop gate.
  - Validate collisions, fixed-name guarantees, complete outcome alternatives,
    positions/roles, immutable run registration, and ordered notifications.
    Reject alias names with typed replacement guidance before effects.
  - Derive deferred lifecycle keys, event binding views, and passive descriptors
    from the registry. Handlers cannot emit events or acquire roles by naming them.

- [ ] **Preflight integration**
  - Add Claudine agent resolution, launch-schema validation, and lifecycle-shell
    audit as Darkmatter preflight checks. Sequence approval inventory invokes the
    same collection per task, condition-blind and in the existing order.
  - Bootstrap frontmatter without shells, run initialize before body discovery,
    then reuse the approved graph and remote runtime for full composition.
  - Defer only active-host operands; root stacks retain identity when child
    frontmatter merges. Initialization mutations feed the first composition.
  - Audit lifecycle-only shells even when body-shell execution is disabled. Share
    approval authority but keep compose-shell permission distinct from lifecycle
    permission; lifecycle unlock is only at composed and relocks on adoption.
  - Refuse pre-unlock shells in dead branches, catch routes, and whole-value `set`
    substitutions. Check exact approval/typed templates at execution after mutations.

### Wave 8 — sequential orchestration integration

- [ ] **Driver migration**
  - Wire direct/inline composition and document/task/group/sequence/loop runs to
    the engine. Keep container scheduling, provider processes, session/budget
    decisions, proxy coordination, subprocess shutdown, and drains in Claudine.
  - Preserve retry/resume reread and reaudit without initialize; loop re-entry
    uses the approved plan; proxy targets bootstrap fresh with shells locked.
  - Delete superseded parser, executor, guard, catch, and admission implementations;
    add a source guard for removed symbols without rejecting legitimate adapters.

- [ ] **Parity verification**
  - Replay traces and unchanged library/CLI assertions; cover every section-8
    failure stage and cleanup mode, concurrent isolation, cancellation generations,
    shell/template invariants, approval prompt order, and no post-launch prompts.

**Checkpoint:** one production implementation drives all Claudine subjects,
with unchanged outcomes and diagnostics. Covers criteria 2, 4–10, 19, 21, and 22.
Freeze shared registry and catalog-consumption interfaces before the next wave.

## Phase 4: Add hosts and migrate schema authority

### Wave 9 — parallel implementation with explicit file ownership

- [ ] **Compose host**
  - Implement `md compose` as the core-only host with no terminal-outcome pair.
    Own CLI lifecycle driver files; consume agreed schema APIs without editing
    catalog, trigger, or shared baseline files owned by the other tasks.
  - Implement supported expressions, shell, terminal communication, and flow;
    only stdout actions use stdout, all other terminal verbs use stderr. Write
    the document after composed and before finalize; `stop` ends only its stack.
  - Skip reached unsupported actions with one warning each naming action/event;
    never evaluate their operands, conditions/payloads, or collect hidden shells.
    Unselected branches do not warn. Supported malformed/forbidden actions fail.
  - Implement skip/no-output/exit-0; initialize errors to failed/finalize; composed
    errors and output failures to finalize/nonzero without failed; cleanup error
    replacement without recursive cleanup; cancellation/exit-130 and second-interrupt
    force exit. Stop underlying pending work and suppress subsequent document output.
  - Dry runs and report-only `--shell` run no stacks/effects/prompts. Keep ordinary
    compose API, read/render, schema validation, DMLS, and child transclusions passive.

- [ ] **Global catalog**
  - Own global/partial YAML, embedding/loading, baseline switching, and host-global
    catalog files. Copy the running embedded baseline's definitions to `fm` first;
    then remove the old embedded `darkmatter/docs/schemas/darkmatter.yaml`.
  - Resolve one embedded transitive bundle independent of launch directory/network.
    Preserve baseline overrides, merge precedence, and disable-baseline behavior.
  - Share ctx/current type; declare err and timing with runtime fields and optional
    integer/null behavior; replace tracking. Claudine declares seq/loop/group once.
  - Explicitly register catalogs and exclude them/types-only partials from automatic
    document-schema discovery, including rediscovery via another path.
  - Feed runtime, schema CLI, and DMLS from resolved declarations; per-view availability
    remains separate from shape, including unavailable err and independent fm.ctx.
  - Implement catalog matrix outcomes, per the input rulings, after parity tests.

- [ ] **Trigger migration**
  - Own `darkmatter/lib/src/markdown/schemas/triggers/`, trigger-facing CLI options,
    and trigger files. Integrate CLI shared argument files through the lead.
  - Rename kind/flag without aliases; explicitly diagnose old kind, flag, and `any`
    with replacement guidance. DMLS retains discovery-scope incomplete-validation errors.
  - Change top-level lists to AND and add explicit group OR; preserve mapping AND,
    `$path`, none/all/min-match nesting, and whole-expression vacuity checks.
    Reject anonymous nested lists. Migrate legacy OR lists to group.
  - Capture existing trigger match/defeat results before rewriting; migrate active
    files/examples/completions, excluding historical/completed specs. Move placeholder
    `schemas/memory.yaml` outside every schema root, using `git mv` during implementation.
  - Implement the trigger robustness matrix through public scan/validation results.

**Checkpoint:** each strand's focused L1 tests pass independently. Baseline validation
assertions remain unchanged; migrated triggers preserve selection. Covers criteria
11, 12, 14–18, 21, and 22. Shared files are integrated once before Phase 5.

## Phase 5: Prove passive tooling and public behavior

### Wave 10 — parallel validation strands

- [ ] **Schema projection**
  - Publish Darkmatter-owned neutral action definitions and md-profile baseline
    unions; Claudine references them and remains closed to unknown verbs.
  - Use suggest for baseline action names. The unknown-action branch excludes
    every known md verb, so forbidden shell and malformed info cannot escape typing.
  - Validate shared conditional/bundle structure and existing depth limits; derive
    event-name/action/binding parity from descriptors, not a second manual list.

- [ ] **Editor projection**
  - After the relevant Phase 4 interfaces are integrated, update DMLS overlay,
    diagnostics, hover, and completion for host names and resolved global catalogs.
  - Prove composed typed/start untyped under baseline; say accepted but unsuggested;
    info suggested and malformed long form rejected; Claudine start typed/composed
    rejected with guidance; host globals' availability agrees with BindingView.
  - Use passive recording services to prove tooling cannot execute stacks, fetch,
    mutate, or accidentally resolve ignored operands.

- [ ] **Invocation coverage**
  - Through each area's `CliProcessFixture`, prove md success/failure/cancellation,
    output channel/order, skip/stop, initialization-created includes, root/child
    identity, unsupported warnings, malformed supported actions, broken writers,
    shell-only audits, report/dry modes, and plain host-only keys.
  - Use failing writers and fake cancellation for engine cases; use real signal/PTY
    coverage only where it answers a distinct invocation-path question.
  - Complete all matrix fixture sweeps and registry/schema/catalog parity tests;
    rerun unchanged Claudine traces and tests after schema integration.

**Checkpoint:** criteria 5–7 and 11–18, 21–22 have passing public-surface evidence.
Fix drift in production rather than normalizing new behavior into old assertions.

## Phase 6: Document and validate the finished change

### Wave 11 — parallel documentation

- [ ] **Lifecycle documentation**
  - Write Darkmatter lifecycle index and error-handling pages with compact examples
    and Mermaid flows for host driving, failure routing, cancellation, and recovery.
  - Update Claudine lifecycle docs to describe the host, preserving authored names
    and linking to Darkmatter docs. Explain section-8 exceptions explicitly.
  - Update both READMEs and lifecycle portions of both skills; review comments on
    every behavior-changing symbol for drift.

- [ ] **Schema documentation**
  - Update schema definition, authoring, targeting, CLI reference, completions, and
    schema skill topics for catalogs, trigger renames, AND/group OR, and tooling.
  - Change old-baseline transclusions to the fm partial; remove planned/implementation
    caveats only for shipped behavior. Docs never name this feature/spec directory.
  - Update dependency docs only if manifests change; coordinate shared README/skill
    files with the lifecycle documentation task.

### Wave 12 — sequential completion audit

- [ ] **Final verification**
  - Run `just test` and `just lint` in both areas, including DMLS and the two spike
    tests in their actual recipe/target coverage. Run affected `just test-l2` only
    for terminal subjects; no terminal/browser focus and no audible playback.
  - Scope downstream compile checks to actual changed public API consumers. Use
    existing cross-platform coverage for macOS, Linux, native Windows, and WSL2;
    load the os skill before OS/path changes or claims about unavailable testing.
    Record executed evidence and any remaining environment gap precisely.
  - Check dependency/source guards, old-name exception allowlists, removed baseline
    references, artifact corpus, matrix coverage, and all 22 acceptance criteria.
    Use frozen prompt fixtures for normal CI behavior tests; prompt tests remain opt-in.
  - Record commands, results, trace equality, prerequisite revision, departures,
    and review-ready status in the implementation log. Do not change CI scope,
    commit, or move the feature into `_completed` as part of this task.

**Checkpoint:** all acceptance evidence is accounted for, docs reflect the code,
and the implementation is complete and ready for review. Criterion 20 closes here.

### Input Robustness Matrix

This applies to YAML catalog and trigger configuration readers changed here, not
to the lifecycle text grammar, renderers, writers, or Markdown bodies. `.yaml`
and `.yml` are one supported YAML format. The matrices describe configuration
values, not the document values being validated; optional document fields retain
their existing absence/null semantics.

Each column is a separate load-bearing field. `E` means a typed, field-located
load failure surfaced by public catalog lookup or trigger scan/validation, never
an empty/successful result. `N/A` means the shape cannot apply to that scalar;
for a collection substituted there, assert E. Null and absence have separate
outcomes even when both are rejected. The outcomes below follow the author's
input rulings and are final.

Global catalog (strict declaration contract):

| Shape | kind | $schema | fm | ctx | current | env | err | timing |
|---|---|---|---|---|---|---|---|---|
| Positive control | schema catalog | resolves | baseline | captured shape | same context shape | object | error shape | runtime durations |
| Absent | E: catalog kind required | E: declarations required | E: required root | E | E | E | E | E |
| Explicit null | E: null kind | E: null declarations | E: null definition | E | E | E | E | E |
| Wrong whole type | E | E | E | E | E | E | E | E |
| One wrong element/member | N/A | E: malformed declaration | E: invalid nested field | E | E | E | E | E |
| Every element/member wrong | N/A | E | E | E | E | E | E | E |
| Empty value/container | E | E: no globals | E: missing baseline shape | E: missing context shape | E | E: empty type expression | E: missing error shape | E: missing durations |
| Duplicate key | E | E | E | E | E | E | E | E |
| Invalid/trailing YAML | E | E | E | E | E | E | E | E |

Host catalog uses the same kind/$schema envelope outcomes, independently tested,
and these separate required declaration columns:

| Shape | seq | loop | group |
|---|---|---|---|
| Positive control | sequence hover/runtime shape | loop hover/runtime shape | group hover/runtime shape |
| Absent | E: required declaration | E | E |
| Explicit null | E: null declaration | E | E |
| Wrong whole type | E | E | E |
| One wrong member | E | E | E |
| Every member wrong | E | E | E |
| Empty definition | E: missing subject shape | E | E |
| Duplicate key | E | E | E |
| Invalid/trailing YAML | E | E | E |

Trigger envelope and condition fields (strict explicit-null handling;
absence of kind retains ordinary non-trigger classification):

| Shape | kind | match | $schema payload | group | all | none | min-match | min-match.count | min-match.of | $path | property condition |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Positive control | schema-trigger discovered | selects expected docs | applies expected payload | OR selection | AND selection | exclusion selection | threshold selection | threshold changes selection | candidates change selection | path selects docs | type/presence selects docs |
| Absent | non-trigger classification | E: required | matcher-only trigger, no payload | no group node | no all node | no none node | no threshold node | E: required | E: required | no path predicate | no property predicate |
| Explicit null | E: null kind, not ordinary data | E: null match | E: null payload | E | E | E | E | E | E | E | E |
| Wrong whole type | E: non-string kind | E | E | E | E | E | E | E | E | E | E |
| One wrong element/member | N/A | E | E in mapping/reference list | E | E | E | E | N/A | E | E | E in supported long form |
| Every element/member wrong | N/A | E | E | E | E | E | E | N/A | E | E | E |
| Empty value/container | non-trigger empty string; E for collection | E: empty/vacuous | preserve explicit empty schema if currently supported; no constraints | E: empty list | E: empty list | E: empty list | E: missing count/of | E: empty scalar | E: empty list | E: empty/invalid pattern | E: empty type/invalid long form |
| Duplicate key | E | E | E | E | E | E | E | E | E | E | E |
| Invalid/trailing YAML | E | E | E | E | E | E | E | E | E | E | E |

An omitted optional node removes that predicate, not an empty predicate collection;
validate the remaining expression's vacuity. Old `trigger-schema` and `any` are
replacement-guidance failures, tested in addition to shape rows. Count must be a
positive integer no larger than the nonempty candidate list. Preserve supported
property long-form fields and payload forms from the prerequisite revision; if
loader inventory finds additional load-bearing fields (for example long-form
condition constraints), give each its own column with these eight shape outcomes
before implementation, rather than collapsing them into one parser test.

- [ ] **Matrix evidence**
  - Capture valid fixtures from the actual public tooling at the prerequisite
    revision (schema export/show and normal trigger discovery), including each
    supported payload/condition form. Where no exporter exists, take the actual
    shipped file and prove it through the normal tool before editing it.
  - Use one parameterized YAML-format matrix sweep per reader family: unedited
    positive control, then one source edit per cell. Assert public globals/baseline,
    hover/availability, selected trigger/payload, or scoped incomplete-validation
    diagnostics. Never assert only parser return values.
  - Test duplicate keys and trailing garbage from source bytes, before constructing
    a map that could erase them. Exercise embedded catalog failures via the public
    catalog build/load boundary and resolved metadata rather than inventing a CLI.
  - Review `#[serde(default)]`, absence/null-collapsing `Option<T>`, string-filtering
    `filter_map`, `unwrap_or_default()`, and discarded `.ok()` errors on these fields.
    A finding in one cell reports every other cell in its row and column.
