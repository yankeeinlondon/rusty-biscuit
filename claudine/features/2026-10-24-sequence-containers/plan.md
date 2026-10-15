---
total_phases: 9
created: 2026-10-12
phase: 2
agent: claude/sonnet
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - claudine
    - claudine-cli
    - messenger
    - messenger-cli
    - darkmatter-cli
    - darkmatter

source_files_during_phase_2:
    - messenger/lib/src/research/refresh/runner.rs
    - messenger/lib/src/research/refresh/state.rs
    - messenger/lib/src/research/refresh/prepare.rs
    - messenger/lib/src/research/refresh/check.rs
    - messenger/lib/src/research/refresh/promote.rs
    - messenger/lib/src/research/refresh/select.rs
    - messenger/lib/src/research/refresh/cleanup.rs
    - messenger/lib/src/research/refresh/mod.rs
    - messenger/cli/src/research.rs
    - messenger/cli/src/research_lifecycle.rs
    - messenger/justfile
    - messenger/lib/tests/research_refresh.rs
    - messenger/cli/tests/research_lifecycle_cli.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/mod.rs
    - claudine/cli/src/args.rs
    - claudine/cli/src/main.rs
    - claudine/cli/src/telemetry.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/exec/termination/windows.rs
    - claudine/cli/src/commands/wrap/exec/spawn/captured.rs
    - claudine/cli/src/commands/wrap/exec/spawn/inherited.rs
    - claudine/cli/src/commands/wrap/exec/spawn/retained.rs
    - claudine/cli/src/commands/wrap/exec/spawn/semantic.rs
    - claudine/cli/src/commands/wrap/exec/wiring/session.rs
    - claudine/cli/src/completion/engine/tests.rs
    - claudine/cli/tests/l1/sequence_errors_cli.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/completion_ownership.rs
    - claudine/cli/tests/l1/provider_tail_ownership.rs
    - claudine/cli/tests/common/owned_value_options.rs
    - claudine/cli/tests/l1/error_guards/transport-allow.toml
docs_updated_during_phase_2:
    - claudine/docs/cli/index.md
    - claudine/docs/cli/sequence.md
    - claudine/README.md
    - claudine/docs/topics/completions/shell-completions.md
    - claudine/docs/dependencies.md
    - docs/dependencies.md
    - messenger/cli/README.md
    - messenger/docs/user-guide.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/messenger/research-contract.md
    - .claude/skills/messenger/SKILL.md
    - .claude/skills/claudine/cli-commands.md
    - .claude/skills/claudine/timeline.md
source_files_during_phase_3:
    - claudine/cli/src/argv/partition.rs
    - claudine/cli/src/argv/partition/tests.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/sequence/jit.rs
    - claudine/cli/src/commands/wrap/sequence/review.rs
    - claudine/cli/src/commands/wrap/sequence/review/tests.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/commands/wrap/sequence/task_run/tests.rs
    - claudine/cli/src/completion/engine/tests.rs
    - claudine/cli/src/completion/schema_completion/tests.rs
    - claudine/cli/tests/common/owned_value_options.rs
    - claudine/cli/tests/fixtures/frozen_prompts/prompts/review-loop.md
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/completion_ownership.rs
    - claudine/cli/tests/l1/composition_outputs.rs
    - claudine/cli/tests/l1/ctx_per_run.rs
    - claudine/cli/tests/l1/document_environment_routes.rs
    - claudine/cli/tests/l1/handoff_owners.rs
    - claudine/cli/tests/l1/launch_trigger_schemas.rs
    - claudine/cli/tests/l1/lifecycle_downgrade_outcome.rs
    - claudine/cli/tests/l1/lifecycle_mixed_span_validation.rs
    - claudine/cli/tests/l1/loop_cli.rs
    - claudine/cli/tests/l1/provider_tail_notice.rs
    - claudine/cli/tests/l1/sequence_cli.rs
    - claudine/cli/tests/l1/sequence_ctrl_c_windows.rs
    - claudine/cli/tests/l1/sequence_errors_cli.rs
    - claudine/cli/tests/l1/sequence_groups.rs
    - claudine/cli/tests/l1/sequence_jit.rs
    - claudine/cli/tests/l1/sequence_perf.rs
    - claudine/cli/tests/l1/sequence_shell_templates.rs
    - claudine/cli/tests/l1/sequence_sources_cli.rs
    - claudine/cli/tests/l1/tts_prewarm.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/lifecycle/actions.rs
    - claudine/lib/src/composition/lifecycle/audio.rs
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/parse.rs
    - claudine/lib/src/composition/lifecycle/prewarm/sequence/tests.rs
    - claudine/lib/src/composition/lifecycle/runtime.rs
    - claudine/lib/src/composition/lifecycle/tests/grammar.rs
    - claudine/lib/src/composition/lifecycle/tests/nested_span.rs
    - claudine/lib/src/composition/lifecycle/tests/parse_config.rs
    - claudine/lib/src/composition/sequence/formal.rs
    - claudine/lib/src/composition/sequence/grammar.rs
    - claudine/lib/src/composition/sequence/mod.rs
    - claudine/lib/src/composition/sequence/model.rs
    - claudine/lib/src/composition/sequence/normalize.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/runtime_fixture.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/reserved.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/lifecycle_lift_ready_guard.rs
    - claudine/schemas/action.yaml
    - claudine/schemas/claudine.yaml
    - claudine/schemas/lifecycle.yaml
    - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs

docs_updated_during_phase_3:
    - claudine/README.md
    - claudine/docs/cli/sequence.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/features/2026-10-24-sequence-containers/implementation-log.md
    - claudine/features/2026-10-24-sequence-containers/plan.md
    - claudine/schemas/README.md

docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_4: []
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
---

# Plan: Sequence containers

## Summary and definition of done

The spec makes groups and sequences first-class executable primitives with
the same lifecycle events as a document. The work falls into six strands:

1. **Grammar**: remove `execution:`, task `setup:`/`teardown:`, non-group
   `fail_fast`, `--fail-fast`; add inline/by-reference nested `sequence:`
   tasks, container lifecycle events, and the `cancelled` event.
2. **Bindings**: replace the injected `state`/`previous`/`next`/`outputs`/…
   globals with the reserved `seq` and `group` roots (innermost-only), with
   typed hard-rename errors.
3. **Runtime**: one container runner for groups (always concurrent) and
   sequences (ordered, nestable) driving the section 2 event order, failure
   propagation, `retry`, atomic group results, native array/dictionary results.
4. **Cancellation**: sibling-failure opt-in cancellation, Ctrl-C ending
   execution (exit `130`), `finalize` always running except after
   `skip`/`proxy`/`retry`/`resume`.
5. **Preflight**: transitive nested walk, cycle detection by active chain,
   late-value shell approval with the new roots.
6. **Budget ledger removal** in `claudine-cli` and Messenger, with Messenger
   gaining its own OS-lock-backed run protection.

**Done means** (all must hold):

- [ ] every acceptance criterion 1-23 in `spec.md` has a passing test named
  after it (L1 unless the spec says L2; criterion 7 is L2)
- [ ] `just test` and `just lint` are green in `claudine/` and `messenger/`;
  `just test-l2` is green in `claudine/` for the affected integration tests
- [ ] the criterion 15 `rg` search returns no hit outside the allowed paths
- [ ] `shipped_prompt_contract`, `shipped_prompt_route_drift`, and the
  shipped-prompt lifecycle walker pass after the migration sweep
- [ ] docs, README, and the `claudine` and `messenger` skills describe the new
  behavior; no `docs/` page names a feature or fix
- [ ] no new CI cell is added

Terminal state for the implementing agent: "implementation complete, ready for
review". The agent does not move the feature to `_completed`.

## Conventions used by this plan

- Wave = tasks that can run concurrently in separate subagents. A wave starts
  only after the previous wave's checkpoint passes. Concurrent subagents in a
  wave must own disjoint files; the "Files" note on each task names its
  ownership.
- Every task that changes behavior also updates the `///`/`//!` docs and
  inline comments of the symbols it touches (CLAUDE.md "Authoring discipline").
- Tests use `CliProcessFixture` with private CWD and home, stub providers,
  `PLAYA_DRY_RUN` with a private spool, and never take window focus. Load the
  `rust-testing` and `os` skills before writing tests or touching path code.
- Do not run `cargo fmt`. Do not commit; the commit step is separate.
- Sub-agents are told the whole session tree is non-interactive.

---

## Phase 1: Preconditions, rulings, and spikes

Goal: confirm the dependency specs are in the tree, settle the open
interpretation points, and run the two read-only spikes that de-risk the
runtime phases. No product code changes in this phase.

### Necessary Rules

The spec says "Open questions: None", but these points are underdetermined
for an implementer. Each needs an author ruling before the phase named in
brackets begins. The default shown is what the plan assumes if the author
makes no other ruling.

1. **[Phase 3-5] Dependency specs may not be landed.** `depends-on` names
   `2026-10-20-lifecycle-grammar` and `2026-10-23-shell-template-approval`;
   the spec leans on `loop.*` roots and the late-value approval grammar and
   `2026-10-18-fm-namespace` for the reservation of `seq`/`loop`/`group`.
   Default: Task 1.1 verifies each by reading the tree. If a dependency is not
   implemented, stop and report; this plan does not absorb that work.
2. **[Phase 3] `params:` spelling.** The spec says tests spell it `params:`
   until `2026-10-26-task-inputs-and-providers` lands. Default: use `params:`
   in all fixtures, and treat the spec's `with:` examples as `params:`.
   Container `retry.with` values enter runtime state under the container's
   `params:` binding without a per-key schema (no schema exists yet);
   "validated afresh" means the existing value-shape validation only.
3. **[Phase 5] Outcome of a non-cancellable leg whose group failed.** Section
   4 says it "runs to completion, its outcome is recorded in `group.legs`".
   Default: record its **own** outcome (`succeeded` or `failed`), not
   `cancelled`; only legs cancelled by opt-in or never admitted are
   `cancelled`.
4. **[Phase 5] A skipped container as a sequence step or group leg.** A
   `skip` from `initialize` makes the container `skipped`: appends nothing to
   a sequence and maps to `null` in a group dictionary (section 8). Default as
   stated; no ruling needed beyond confirming `group.legs.<leg>.outcome` is
   `skipped`.
5. **[Phase 6] Windows Ctrl-C and exit `130`.** Criterion 7 is an L2 test
   asserting exit `130`. Default: assert `130` on Unix and the Windows value
   already produced by the existing signal ladder (see
   `2026-07-11-signal-assurance-and-handling` docs and the `os` skill),
   recorded in one table in `docs/`. If Windows cannot deliver a console
   interrupt to a test child without focus, the test is skipped there with an
   explicit, reported skip, not silently.
6. **[Phase 2] Messenger fleet-lock implementation.** The spec requires an
   "OS-backed fleet lock" but names no crate. Default: use whatever advisory
   file-lock crate already exists in the workspace dependency graph (Task 1.2
   finds it) so no new dependency is added; if none exists, adding one needs
   author approval.
7. **[Phase 2] How "recovered explicitly" is invoked.** Default: a new
   explicit flag on the existing `messenger research` cleanup/resume path
   (not a new top-level command) that marks a lockless `active` record
   `interrupted`; status commands only *report* a stale run.
8. **[Phase 2] Old ledger-era run records.** Default: `exhausted` stays a
   readable status that is resumable like `interrupted` but is never
   promotable and is never shown as successful; an unreadable `budget.json`
   in a run directory is ignored (not deleted) with a one-line note.
9. **[Phase 5] Wall-clock `timeout` on a container.** The spec treats
   `timeout` as a failure but does not say whether containers carry their own
   `timeout`. Default: containers have none; a leg's own `timeout` applies to
   that leg only. Flag if the author wants container-level timeouts.

Wider measurement is out of scope: no performance study, and no extra OS
runs beyond those the acceptance criteria name.

### Spikes

Each spike runs once, on this host only (macOS), read-only, and writes its
findings into `implementation-log.md` in this feature directory as a short
"Spike result" entry. A spike that answers its question by reading code does
not need a prototype.

- [x] **Spike A: runtime-state branching.** Question: can the existing
  runtime state (`lib/src/composition/runtime_state.rs`) be cloned/branched
  per leg in memory and merged back in declaration order, including for a
  nested sequence leg, without touching disk? Output: the exact branch/merge
  API to use in Phase 5, and whether `group.rs`'s current merge handles a
  nested-sequence leg or needs a new merge type.
- [x] **Spike B: guard and runtime re-entry.** Question: does
  `LifecycleRunGuard` (`lifecycle/mod.rs`) plus `lifecycle/runtime.rs` already
  model `retry`/`resume` without `finalize`, and what must change to add
  `cancelled` as a terminal event with the section 4 action set? Output: a
  list of enum variants, tables, and call sites to change in Phase 3 and 6.

### Tasks

#### Wave 1 (concurrent, read-only)

- [x] **1.1 Dependency check.**
  - Confirm in the tree: lifecycle-grammar's `loop.*` roots and event grammar,
    shell-template-approval's late-value classification and first-task
    approval, fm-namespace's reserved `seq`/`loop`/`group`.
  - Record in `implementation-log.md` which are implemented and which are
    spec-only. Report blockers per Rule 1.
  - Files: none changed except the log.
- [x] **1.2 Messenger lock inventory.**
  - Find an existing advisory-lock crate in the workspace (`cargo tree`,
    `docs/dependencies.md`) and the current `fleet.lock`/liveness code in
    `messenger/lib/src/research/refresh/{state,prepare,cleanup,promote}.rs`.
  - Output: the lock crate choice and the list of ledger readers to delete.
- [x] **1.3 Spike A** (above).
- [x] **1.4 Spike B** (above).
- [x] **1.5 Baseline.** Run `just test` and `just lint` in `claudine/` and
  `messenger/` and record pre-existing failures so later failures are
  attributable. The working tree already has unrelated uncommitted changes;
  note them and do not touch them.

#### Wave 2 (author gate)

- [x] **1.6 Rulings.** Present the nine rulings with their defaults and the
  spike/dependency findings to the author. With `yolo: true`, proceed on the
  defaults and record each as "assumed" in `implementation-log.md`.

**Checkpoint 1:** dependency status recorded; spike results recorded; rulings
recorded; baseline recorded.

---

## Phase 2: Budget ledger removal and Messenger run protection

Goal: spec section 10 and requirement 8. Independent of the container work,
so it may run in a separate worktree concurrently with Phases 3-6; it is
placed first because it deletes code the later phases would otherwise
edit around (`sequence.rs` options, completion tables, exit codes).

Messenger's replacement protection must exist **before** the ledger path is
deleted, so Wave 1 builds it and Wave 2 deletes the ledger.

### Input robustness matrix: Messenger `run.json`

Applies because Messenger reads `run.json` (and tolerates an old
`budget.json`). Load-bearing fields: `status`, `resumptions`,
`stage results`, `run_id`/`platform`. Outcome per shape (a single test walks
the matrix from a real fixture record produced by the current code; a control
row proves the unedited record loads):

| Shape | `status` | `resumptions` | stage results | `invocations` |
| --- | --- | --- | --- | --- |
| absent | typed `RunRecordInvalid`; never default `active` | typed error; never `0` | typed error; never "no stages done" | legacy migration: `0` |
| explicit null | same typed error as absent, distinguishable message | typed error | typed error | typed error; never zero |
| wrong type, whole field | typed error | typed error | typed error | typed error |
| wrong type, one element | n/a | n/a | typed error naming the element; never dropped | n/a (scalar) |
| wrong type, every element | n/a | n/a | typed error; never read as empty | n/a (scalar) |
| empty | `""` is an unknown-status typed error | n/a | `[]` is "no stages", distinct from absent | typed error for string or collection |
| duplicate key | typed parse error | typed parse error | typed parse error | typed parse error |
| trailing content | rejected | rejected | rejected | rejected |

The runner also reads `invocations` (JSON): absence explicitly migrates a
ledger-era record to zero; `null`, wrong scalar or collection types, negative
values, duplicate keys, and trailing content are typed errors. Zero admits the
first launch when below the positive `limits.max_invocations`; the exact cap
refuses without rewriting the record. Resuming preserves the count.

Plus the ledger-era cases: `status: "exhausted"` loads (Ruling 8), never maps
to success, and is refused for promotion; an unknown status string is a typed
error, not coerced.

Smells to grep before declaring done: `#[serde(default)]` on those fields,
`unwrap_or_default()`, `.ok()` on a run-record parse.

### Tasks

#### Wave 1: replacement protection (messenger only)

- [x] **2.1 Runner lock.**
  - Add the Messenger-owned runner holding an OS-backed fleet lock for the
    whole execution (one research run at a time), per Ruling 6.
  - Record `finished`, `interrupted`, `stopped` in the existing `run.json`.
  - Keep: per-refresh cap on agent runs, run-record validation, stage checks,
    resume limit, preparation lock, refusal to promote incomplete research.
  - Files: `messenger/lib/src/research/refresh/{state,prepare}.rs`, new
    runner module if needed.
- [x] **2.2 Stale-run recovery and refusals.**
  - Cleanup and promotion are refused while a run is live.
  - An `active` record whose runner no longer holds the lock is reported as
    stale and recovered explicitly (Ruling 7), never silently.
  - Files: `cleanup.rs`, `promote.rs`, `check.rs`, `select.rs`.
- [x] **2.3 Run-record matrix test** (above) and the criterion 22 tests: live
  runner refuses second run; interrupted runner recorded `interrupted`; stale
  `active` recovered explicitly; ledger-era `exhausted` readable or actionable
  migration error; cleanup/promotion refused during a live run.
  - Files: `messenger/lib/tests/research_refresh.rs`, `state` tests.

**Checkpoint 2a:** `just test` and `just lint` pass in `messenger/` with the
ledger still present.

#### Wave 2: ledger deletion

Subagents own disjoint areas.

- [x] **2.4 Messenger ledger removal.**
  - Remove `LedgerView` and `budget.json` reads, the `exhausted` derivation
    from the ledger, `RefreshError::Ledger` and its `grant`/`resume`
    guidance, the ledger-lock check in cleanup, the ledger line in
    `messenger research runs`, the "within the same budget" wording, and the
    `claudine budget init … --exclusive-lock …` and `--budget-ledger`
    emission in `prepare.rs`.
  - Files: `messenger/lib/src/research/refresh/*`,
    `messenger/cli/src/research_lifecycle.rs`.
- [x] **2.5 Claudine CLI ledger removal.**
  - Delete `cli/src/budget/`, `cli/src/commands/budget.rs`, the `budget`
    subcommand registration, the `sequence --budget-ledger` option and
    plumbing, `BUDGET_EXHAUSTED_EXIT_CODE`/`BUDGET_BLOCKED_EXIT_CODE`, the
    ledger hooks in `commands/wrap/harness_orch/loop_control.rs`, and the
    ledger comments in `wrap/exec/termination/windows.rs`.
  - Delete `tests/l1/sequence_budget.rs`; edit `completion_ownership.rs`,
    `tests/common/owned_value_options.rs`, and `completion/engine/tests.rs`
    to drop ledger cases.
  - Files: `claudine/cli/**`.
- [x] **2.6 Completions.** Remove `budget` and `--budget-ledger` from the
  completion engine, owned-value tables, and every generated completion
  script (regenerate through the repo's own recipe, not by hand).
- [x] **2.7 Docs and skills.**
  - Delete `claudine/docs/cli/budget.md`; edit `docs/cli/sequence.md`,
    `claudine/README.md`, `docs/topics/completions/shell-completions.md`
    (only the `--budget-ledger` mention), both `dependencies.md` files,
    `messenger/cli/README.md`, `messenger/docs/user-guide.md`, and the
    `claudine` and `messenger` skills.
  - Remove exit codes `76`/`77` from every exit-code table.
  - Add a dated entry to `.claude/skills/claudine/timeline.md`.
- [x] **2.8 Unused dependencies.** Check `claudine-cli` and `messenger` for
  crates now unused (keep `sysinfo` where it still has a user); remove and
  update dependency docs.

**Checkpoint 2b (criterion 15, 22):**
- [x] `claudine budget init x.json` is an unknown-subcommand usage error;
  `claudine sequence --budget-ledger x.json run.md` is an unknown-option
  usage error (L1 tests).
- [x] `rg -n -i 'budget_ledger|BudgetLedger|budget-ledger|claudine budget'`
  outside `**/features/**`, `**/fixes/**`, `timeline.md` returns nothing.
- [x] `just test` and `just lint` pass in `claudine/` and `messenger/`;
  `messenger research prepare` emits no `--budget-ledger` and no
  `claudine budget init` step.

---

## Phase 3: Grammar, schemas, and the lift-ready lifecycle slice

Goal: parse and validate everything the spec adds or removes, before any
runtime change. After this phase the new documents parse and fail validation
correctly but do not yet run as containers.

### Input robustness matrix: authored sequence/group/lifecycle YAML

Applies: this phase adds parsing of YAML frontmatter and `kind: group`,
`kind: group-catalog`, `kind: sequence` files. Load-bearing fields:
`sequence:` (on a task), `tasks:` (group), `fail_fast`, `cancelled:`, leg/step
`name`, group `variables`, and the removed keys `execution`, `setup`,
`teardown`. A single table-driven test per file form (Markdown frontmatter,
`kind: sequence`, `kind: group`/`group-catalog`) walks every cell starting
from a real fixture that already runs; a control row proves the unedited
fixture is accepted.

| Shape | `sequence:` (task) | `tasks:` | `fail_fast` | `cancelled:` | `name` | `variables` |
| --- | --- | --- | --- | --- | --- | --- |
| absent | task has no container executable (normal) | typed error: group needs ≥ 1 leg | `true` | no opt-in | typed error (leg); step name per existing rule | none |
| explicit null | typed error (not absent) | typed error | typed error | typed error; not an empty opt-in stack | typed error | typed error |
| wrong type, whole | typed error (must be list or file ref string) | typed error | typed error (non-boolean) | typed error | typed error | typed error |
| wrong type, one element | typed error naming index | typed error naming index | n/a | typed error naming index | n/a | n/a: values may be arbitrary JSON |
| wrong type, every element | typed error; never `[]` | typed error | n/a | typed error | n/a | n/a: values may be arbitrary JSON |
| empty | `[]` is a valid empty sequence returning `[]` (spec section 1) | typed error (groups need a leg) | n/a | `[]` is a valid opt-in with an empty stack (spec section 1) | `""` typed error | `{}` valid |
| duplicate key | typed error | typed error | typed error | typed error | typed error | typed error |
| trailing/invalid content | document rejected | rejected | rejected | rejected | rejected | rejected |

Rows that differ from the example of "absent = empty" are deliberate:
`cancelled: []` is **not** absent (it opts in) and an empty `sequence: []` is
**not** an error. Also assert: combining two exclusive executables
(`prompt`/`shell`/`side_effect`/`group`/`task`/`sequence`) is a typed error;
`execution:` of any value, task `setup:`/`teardown:`, a sequence-document
`fail_fast`, container event keys on a non-inline-sequence task, duplicate or
missing leg names, reserved step keys, and reserved group variable names are
typed errors that name the replacement.

Smells to grep before declaring done: `#[serde(default)]` on `tasks`,
`fail_fast`, `cancelled`; `Option<T>` where null must differ from absent;
`filter_map(.. as_str())`, `unwrap_or_default()`, `.ok()` on these parses.

### Tasks

#### Wave 1 (disjoint files, concurrent)

- [x] **3.1 Lifecycle event model.**
  - Add `cancelled` to the event enum and the container event table in the
    lift-ready slice: `lifecycle/parse.rs`, `action_shape.rs`, `actions.rs`,
    `validate.rs`, `signatures.rs`.
  - `cancelled` accepts communication, expression (`set`, side-effect verbs,
    expression functions), `shell`, and `stop`; `retry`, `resume`, `proxy`,
    `prep`, `skip`, `error`, `break`, `defer` are typed parse errors.
  - A `shell` action in `cancelled` is rejected for use before `start` at run
    time (runtime phase enforces); parsing allows it.
  - Prerequisite: Spike B output.
  - Files: `claudine/lib/src/composition/lifecycle/{parse,action_shape,actions,validate,signatures}.rs`.
- [x] **3.2 Lift-ready guard test (criterion 14).**
  - Add a test asserting the five modules above import no Claudine runtime
    types (settings, provider layer, messaging, TTS/Playa). Make it fail on a
    deliberate temporary import before keeping it.
  - Files: new test only.
- [x] **3.3 Sequence/group grammar.**
  - Add the inline `sequence:` executable (list or file reference with
    `-> offset` and `::operator` suffixes) to task parsing; make it exclusive
    with the other executables.
  - Remove `execution:`; add the two typed errors (parallel: omit the key;
    serial: sequence replacement with nested-sequence spelling).
  - Make `fail_fast` valid on group objects only; typed errors for sequence
    `fail_fast` and for `--fail-fast` (CLI flag, also stop exporting
    `CLAUDINE_FAIL_FAST`).
  - Reject task `setup:`/`teardown:` with the section 6 replacement text.
  - Container event keys allowed on a task only when its executable is an
    inline `sequence:` list; typed error otherwise.
  - Require unique leg names; reserved step keys; reserved group variable
    names (`legs`, `succeeded`, `failed`, `cancelled`).
  - Files: `lib/src/composition/sequence/{grammar,model,normalize,reserved,source,formal}.rs`,
    `sequence/task/{mod,group}.rs`, `cli/src/commands/sequence.rs` (flag only).

#### Wave 2 (depends on Wave 1)

- [x] **3.4 Schemas and completions (requirement 5).**
  - Add `cancelled` to Claudine's schema partials on every primitive; drop
    the document-level `fail_fast` and the generated sequence overlays from
    `darkmatter/docs/schemas/claudine.yaml` payload; update group, task, and
    lifecycle partials and editor-facing completions for nested containers.
  - Do **not** write `sequence.yaml` (belongs to the task-inputs feature).
  - Note: `darkmatter` files carry unrelated uncommitted changes in the
    working tree; edit only the lines this task needs.
  - Files: `darkmatter/docs/schemas/claudine.yaml`, schema partials,
    `claudine/cli/src/completion/schema_completion/*`.
- [x] **3.5 Grammar tests.** Implement the matrix tests above and the
  criterion 2 tests (each removed key raises its typed error before anything
  runs).

**Checkpoint 3:**
- [x] matrix tests, criterion 2 and 14 tests pass
- [x] `just test` and `just lint` pass in `claudine/`; schema-related
  `darkmatter` tests pass (`just test darkmatter` from the repo root)

---

## Phase 4: `seq` and `group` roots, and the hard rename

Goal: the binding catalog resolves `seq`, `group`, and `loop`
innermost-only; old names raise typed errors. Runtime values are produced by
Phase 5, so this phase builds the catalog, validation, and error plumbing
against fixtures.

### Tasks

#### Wave 1

- [x] **4.1 Binding catalog.**
  - In `lifecycle/bindings.rs`: define `seq.*` and `group.*` contents per
    spec section 7, read-only runtime roots (not assignable through `set`,
    not persisted by inline-compose), and the availability table additions
    for `cancelled` and container events (also update `lifecycle.md`).
  - Innermost-only: shadow the complete root; restore on exit; absent group
    keeps the typed outside-group error; `fm.seq`/`fm.loop`/`fm.group` read
    frontmatter.
  - Within a container's own events the root names the container itself, with
    step-position fields `null` and `seq.outputs`/`seq.run_id` set.
  - Standalone `compose`/`inline-compose` keep `seq.outputs` only.
  - `{{ seq }}` string shorthand to `seq.name`, also for `previous`/`next`.
  - Files: `lifecycle/bindings*.rs`, Darkmatter's shared binding view only
    if the interface requires it (check first; prefer Claudine-side).
- [x] **4.2 Group result values.** Build `group.legs`/`succeeded`/`failed`/
  `cancelled` data types and the empty-vs-populated timing per event
  (empty in `initialize`/`start`/`blocked`; absent inside a leg except
  variables).

#### Wave 2

- [x] **4.3 Hard-rename errors.**
  - Typed error naming the replacement for each old name in the section 7
    table. Implementation must walk **parsed** root reads, including static
    bracket access, across body, conditions, lifecycle operands, and shell
    discovery. No text search.
  - Suppress the error when the reading document declares or is supplied a
    property of that name (including `null`); `fm.previous` is never a
    renamed root; `ctx.state` and quoted strings are never flagged.
  - Audit and migrate implicit `fm.outputs`/`fm.state` access.
- [x] **4.4 Remove generated overlays.** Delete the code that injects
  `state`, `previous`, `next`, `outputs`, `sequence_id`, `previous_state`,
  `next_state`, `step`, `total_steps`, root `is_first`/`is_last` into
  frontmatter, in lockstep with Phase 5's `seq` production (the removal and
  the producer land in one change set; coordinate with Phase 5 Wave 1).
- [x] **4.5 Binding tests** for criteria 9 (the parts testable without a
  runner), 10, 11, and the step-key reserved-name error.

**Checkpoint 4:** `just test`/`just lint` in `claudine/`; the rename-error
test walks every row of the section 7 table plus the "declares its own
`previous`" control.

---

## Phase 5: Container runtime

Goal: groups and sequences run through the section 2 event order, with
nesting, failure propagation, `retry`, atomic results, and native results.
Cancellation (Phase 6) is stubbed to "not reachable" here.

### Tasks

#### Wave 1 (foundation; sequential internally, one subagent)

- [ ] **5.1 Container runner.**
  - A single runner driving groups and sequences through `initialize` →
    preparation → `start`/`blocked` → children → terminal event →
    `finalize`, using `LifecycleRunGuard` so each primitive finalizes at most
    once.
  - Root sequence: `initialize` before static preflight; nested: `start`
    after just-in-time preparation (re-read source, evaluate group
    `variables`).
  - `skip` from `initialize` ends with no further events; `proxy` hands off
    and skips own `finalize`; `retry` re-enters without `finalize`;
    `resume` raises the existing typed runtime error.
  - A body-only step (no executable) composes the document body and runs no
    lifecycle events of its own; root events belong to the sequence
    (criterion 12).
  - `--dry-run` fires no lifecycle events; preserve today's recursive dry-run
    behavior.
  - Files: `cli/src/commands/wrap/sequence/{mod,iterate,task_run,jit}.rs`,
    `lib/src/composition/lifecycle/runtime.rs`.
- [ ] **5.2 Results model.** Native array (sequence) / dictionary (group)
  results through the task result, accumulator, and expression adapters;
  never flattened or JSON-encoded. Publication only after `finalize`
  completes; a `finalize` error publishes nothing (criterion 13).

#### Wave 2 (concurrent after Wave 1; disjoint files)

- [ ] **5.3 Sequence nesting.** Inline and by-reference nested sequences,
  `seq.*` production per step (`id` uniqueness suffixing, `index`, `count`,
  neighbors without recursive embedding, `run_id` shared across nested
  sequences), and restoration of the enclosing bindings.
  - Files: `sequence/*`, `task_run.rs`.
- [ ] **5.4 Group runner.** Concurrent legs only, admitted in declaration
  order under `max_parallel`; each leg gets a branched in-memory runtime
  state (per Spike A); declaration-order merge; no interactivity; keep
  write-back collision rejection and output rendering.
  - `fail_fast: true`: leg failure fails the group once every leg settles;
    unadmitted legs never start and record `cancelled` without events;
    no leg mutations or output published on failure. `fail_fast: false`:
    always succeeds on leg failures, merges failed-leg mutations in order.
  - `group.legs` and summary lists populated for terminal events and
    `finalize`.
  - Files: `sequence/task/group.rs`, group runner in the CLI.
- [ ] **5.5 Failure propagation.** A failure travels task document `failure`
  → enclosing container `failure` → … → root exit `1`, each `finalize` on
  the way, innermost first. A sequence never proceeds past an unhandled
  failure. Timeouts (`timeout`, `step_timeout`) are failures through
  `failure` recovery and never fire `cancelled`.
- [ ] **5.6 Container `retry`.** `max_attempts` counts additional attempts
  and survives re-entry; clear `seq.outputs` and group outcome records
  between attempts while keeping frontmatter/runtime mutations; `retry.with`
  evaluated once, entering runtime state (Ruling 2), never written back;
  nested-retry publication rule (single result after `finalize`);
  exhaustion keeps the failed outcome and publishes nothing.

#### Wave 3 (tests; may be split by criterion)

- [ ] **5.7 Tests:** criteria 1, 3, 4, 5 (the non-cancel parts), 6, 8, 9,
  11, 12, 13, 16, 18, 19 (non-cancel parts), 20, 23. Use deterministic
  fixture signals (files/barriers), not sleeps, for the concurrent legs in
  criterion 20. Add event-recording assertions that a `retry`/`resume`
  attempt's terminal event is followed directly by the next attempt's events.

**Checkpoint 5:**
- [ ] criteria above pass on macOS; `just test` and `just lint` in `claudine/`
- [ ] nothing in the production path still reads the removed overlay names

---

## Phase 6: Cancellation and Ctrl-C

Goal: section 4. Depends on Phase 5's runner and the Ruling 5 decision.

### Tasks

#### Wave 1

- [ ] **6.1 Cancel call.** Add the host-neutral "cancel this primitive with
  this cause" call between Claudine's runtime and the lift-ready event model;
  the cause is a structured diagnostic (originating group and leg for
  sibling failure; interrupt for Ctrl-C). First observed failure is the
  recorded cause.
- [ ] **6.2 Sibling cancellation.** On a `fail_fast: true` group failure,
  stop admission, cancel only running legs that declared `cancelled`
  (terminate the child process, or cancel a container leg's running children
  and all descendants), wait for descendants to settle before the owner's
  terminal events; fire `cancelled` then `finalize`; outcome `cancelled`;
  `err` set. Non-opted legs run to completion (Ruling 3).
- [ ] **6.3 Ctrl-C.** Interrupt every running primitive through the existing
  signal ladder; run declared `cancelled` actions then `finalize` for every
  primitive whose lifecycle was entered (children before parents); exit
  `130`; a second Ctrl-C keeps force-exit. Entered-but-not-started primitives
  get cleanup with shells disabled; unadmitted primitives fire nothing.
- [ ] **6.4 Cancellation is terminal.** Recovery directives in a following
  `finalize` raise the typed cancellation-recovery error; cleanup errors are
  retained as secondary diagnostics; a cleanup-shell timeout is a cleanup
  error, not a new cancellation.

#### Wave 2 (tests)

- [ ] **6.5** Criterion 5 (cancel parts), 7 (L2: a sequence containing a
  group with one cancellable and one non-cancellable running leg; assert no
  later step starts, the non-cancellable leg does not complete, declared
  `cancelled` actions precede `finalize`, `finalize` runs innermost first,
  exit `130`), and 19 (opt-in nested container cancels all descendants;
  cleanup errors preserve cancellation; recovery directive launches no work).
  - L2 rules: no window focus; use the harness's non-focusing spawn mode; see
    the `biscuit-test-harness` and `os` skills; follow Ruling 5 for Windows.

**Checkpoint 6:** `just test`, `just test-l2`, `just lint` in `claudine/`.

---

## Phase 7: Preflight and approval

Goal: requirement 3 and criteria 18 (cycles) and 21.

### Tasks

#### Wave 1 (disjoint files)

- [ ] **7.1 Transitive walk.** `sequence/preflight/{mod,shape}.rs` walk every
  nested container, validate container events, freeze discovered task
  graphs, and retain transitive write-back collision checks (including
  nested same-target inline writes).
- [ ] **7.2 Cycle detection.** Active-reference-chain detection across
  sequence, task, group, and catalog references; the error prints the
  complete chain; reuse of the same sequence twice and distinct groups in
  one catalog are legal. References keep request-scoped `biscuit-file`
  resolution and authoring-file origin (use `FileReference`).
- [ ] **7.3 Shell approval positions.** Late roots become `seq.outputs`,
  `loop.*`, `group.*`; container terminal-event commands and commands
  reading `group.*` are typed templates; first-task resolved-value approval
  follows the execution position (group-first: all legs; nested-sequence
  first: its own first task recursively); a reread or proxy target
  introducing an unapproved command is refused after approval closes;
  concurrent descendants never prompt. `err`, `timing`, `current.*` remain
  illegal in sequence shell commands.

#### Wave 2

- [ ] **7.4 Tests:** criterion 1 (cycle), 18, 21, and the refusal after
  approval closes.

**Checkpoint 7:** `just test` and `just lint` in `claudine/`.

---

## Phase 8: Migration sweep, docs, and skills

Goal: requirement 6, 7, and criterion 17.

### Tasks

#### Wave 1 (disjoint by directory, concurrent)

- [ ] **8.1 Classify hits.** Search the whole repository for `state`,
  `previous`, `next`, `outputs`, `sequence_id`, `previous_state`,
  `next_state`, `step`, `total_steps`, root `is_first`/`is_last`,
  `execution:`, `fail_fast` outside groups, `--fail-fast`, `setup:`,
  `teardown:`. Produce a classification table in `implementation-log.md`:
  migrate / preserve (authored property, unrelated code) / historical
  snapshot (`features/`, `fixes/`, `timeline.md`). Audit `fm.outputs` and
  `fm.state`. Do not rely on text search alone to decide behavior; confirm by
  reading each hit.
- [ ] **8.2 Migrate prompts, fixtures, tests** classified "migrate". If a
  prompt on the route-drift route changes, re-derive the fixture under
  `claudine/cli/tests/fixtures/` and refresh its hash.
- [ ] **8.3 Docs.** Update `sequences.md`, `lifecycle.md` (event table,
  availability table, state machine), `flow-control.md`,
  `flow-control-reference.md`, `frontmatter-properties.md`,
  `composition.md`, `timeouts.md`, with Mermaid diagrams for container event
  order and failure propagation. Audience is a developer new to the repo:
  lead with what the reader can do and give a compact example per rule.
  Document: that `--dry-run` runs shell work with real effects; the
  isolation limit (in-memory only, not the filesystem); atomic group
  results are not a filesystem transaction; planned-vs-built markers.
  No page names or links to a feature or fix.
- [ ] **8.4 Skills.** Update the `claudine` skill (and `messenger` skill if
  not already done in Phase 2); add the dated `timeline.md` entry for this
  feature; load the `drift` review afterward.

**Checkpoint 8 (criterion 17):** `shipped_prompt_contract`,
`shipped_prompt_route_drift`, and the shipped-prompt lifecycle walker pass;
the repo-wide search finds no remaining use of the removed names as runtime
globals or task grammar.

---

## Phase 9: Final verification

### Tasks

#### Wave 1 (concurrent)

- [ ] **9.1 Full local gates.** `just test`, `just test-l2`, `just lint` in
  `claudine/`; `just test`, `just lint` in `messenger/`; `just test darkmatter`
  from the repo root for the schema changes. Report any failure with output.
- [ ] **9.2 Criteria audit.** Walk criteria 1-23 against test names; list any
  criterion with no test.
- [ ] **9.3 Code smell greps** from both robustness matrices
  (`#[serde(default)]`, `unwrap_or_default()`, `.ok()`, `filter_map(.. as_str())`
  on the load-bearing parses).
- [ ] **9.4 Comment/doc drift pass** over every changed symbol (CLAUDE.md
  "Code Comment Quality"); fix drift in favor of the code and record each
  instance in `implementation-log.md`.
- [ ] **9.5 CI impact check.** Run `just ci-local --plan` and confirm no new
  cell was introduced.
- [ ] **9.6 Other-OS evidence.** Per the `os` skill, record which of
  macOS/Linux/Windows/WSL2 were exercised locally and which rely on CI
  (pull requests prove Linux and macOS; Windows is found after merge).

**Checkpoint 9:** all gates green; `status` in `spec.md` is left for the
author; the agent reports "implementation complete, ready for review" and
does not move the feature to `_completed` or run `just complete`.
