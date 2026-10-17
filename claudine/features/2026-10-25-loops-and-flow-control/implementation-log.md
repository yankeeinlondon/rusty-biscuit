---
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-25-loops-and-flow-control/spec.md
plan: claudine/features/2026-10-25-loops-and-flow-control/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - claudine/cli/tests/l1/loop_config_matrix.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/features/2026-10-25-loops-and-flow-control/spike/loop_flow_spike.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/dependency-audit.md
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/path-inventory.md
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/coverage-inventory.md
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/matrix-contracts.md
    - claudine/features/2026-10-25-loops-and-flow-control/spike/README.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/commands/wrap/sequence/task_run/tests.rs
    - claudine/cli/tests/l1/compose_caller_file_provenance.rs
    - claudine/cli/tests/l1/compose_schema_cli.rs
    - claudine/cli/tests/l1/error_guards/transport-allow.toml
    - claudine/cli/tests/l1/handoff_owners.rs
    - claudine/cli/tests/l1/lifecycle_directive_matrix.rs
    - claudine/cli/tests/l1/lifecycle_downgrade_outcome.rs
    - claudine/cli/tests/l1/loop_cli.rs
    - claudine/cli/tests/l1/loop_config_matrix.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/sequence_cli.rs
    - claudine/cli/tests/l1/sequence_errors_cli.rs
    - claudine/cli/tests/l1/sequence_grammar_matrix.rs
    - claudine/cli/tests/l1/sequence_groups.rs
    - claudine/cli/tests/l1/wrap_sigint.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/lifecycle/action_shape.rs
    - claudine/lib/src/composition/lifecycle/actions.rs
    - claudine/lib/src/composition/lifecycle/context.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/parse.rs
    - claudine/lib/src/composition/lifecycle/signatures.rs
    - claudine/lib/src/composition/lifecycle/source_map.rs
    - claudine/lib/src/composition/lifecycle/tests/break_prep.rs
    - claudine/lib/src/composition/lifecycle/tests/grammar.rs
    - claudine/lib/src/composition/lifecycle/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/validate.rs
    - claudine/lib/src/composition/looping/config.rs
    - claudine/lib/src/composition/looping/config/tests.rs
    - claudine/lib/src/composition/looping/dsl.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/looping/engine/tests/gate_ambient.rs
    - claudine/lib/src/composition/looping/engine/tests/iteration_actions.rs
    - claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs
    - claudine/lib/src/composition/looping/engine/tests/mod.rs
    - claudine/lib/src/composition/looping/engine/tests/post_checked_timing.rs
    - claudine/lib/src/composition/looping/engine/tests/rate_limits.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/sequence/data.rs
    - claudine/lib/src/composition/sequence/grammar.rs
    - claudine/lib/src/composition/sequence/mod.rs
    - claudine/lib/src/composition/sequence/normalize.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/reserved.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/src/invocation_context/document_tests.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/schemas/action.yaml
    - claudine/schemas/claudine.yaml
    - claudine/schemas/lifecycle.yaml
    - darkmatter/cli/tests/l1/compose_remote_caching.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/path_function_contracts.rs
docs_updated_during_phase_2:
    - claudine/docs/pipeline.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/flow-control/flow-control-reference.md
    - claudine/docs/topics/flow-control/flow-control.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/looping.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/frontmatter-properties.md
    - claudine/docs/topics/schemas.md
    - darkmatter/docs/schemas/expression-functions.yaml
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/remote-url-references.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/darkmatter/compose.md
packages:
    - claudine
    - claudine-cli
    - darkmatter
    - darkmatter-cli
    - dmls
human_review: false
human_review_items: []
message_to_agent: |-
  Phase 2 landed the grammar; Phases 3 and 4 land the behavior. Every author ruling is recorded in the log's Phase 2 section; none is open.

  1. **"Grammar ahead of behavior" refusals.** Each valid-but-unbuilt construct is refused at exactly one site; remove it in the change that lands the behavior:
     - container `loop:` → `sequence/grammar.rs::container_loop` (`ContainerLoopNotYetSupported`), Phase 3;
     - task `when:` → `sequence/grammar.rs::task_when` (`TaskWhenNotYetSupported`), Phase 4. The guard is validated but **not stored** on the preflight task; Phase 4's "Task guard binding" adds that field;
     - lifecycle `break`/`prep` → `lifecycle/executor.rs::resolve_control` (`LifecycleDirectiveNotYetSupported`), Phase 4.
  2. **Loop configuration API** (`composition/looping/config.rs`, types in `types.rs`): `parse_loop_block(value, LoopOwner, &LoopLocation) -> LoopBlock { config, gate }`, `resolve_templated_loop_max` (a templated `max` stays in `LoopConfig::max_template` until resolved once; the engine refuses an unresolved one), `check_loop_config_keys`. `LoopInvalid` is located (`source_path`, `property`).
  3. **Loop-level fail-fast is gone from the grammar** (`loop.fail_fast` is `RemovedGrammar`), but the engine still honors `CLAUDINE_FAIL_FAST`; tests that relied on authored `fail_fast` set that variable. Phase 3's state machine removes loop-level fail-fast; group `fail_fast` stays.
  4. **`on_rate_limit` is document-only** (author ruling). Keep document recovery unchanged in the new engine.
  5. **Test scope.** Run only the tests for what you change while working; the area's full `just test`/`just lint` runs once at the end of the phase, by the orchestrator, with no subagent running. New robustness matrices follow `prompts/_input-robustness.md`: field shapes once per validator, in-process through the public result; document shapes once per loader; spawn the CLI only to prove wiring. The 303 existing subprocess matrix cells (`loop_config_matrix`, `lifecycle_directive_matrix`, `sequence_grammar_matrix`) may be folded into that form when you touch them; do not grow them per field and per format.
  6. **Schema edits reach Darkmatter.** `darkmatter-cli`'s `schema_validate` tests and `dmls`'s `nested_span_tests` read `claudine/schemas/`; run them when a schema changes.


---

# Implementation Log for 2026-10-25-loops-and-flow-control (6 phases)

## Phase 1

Phase 1 started 2026-10-16. All four dependency specs report
`implemented: true` (2026-10-20-lifecycle-grammar,
2026-10-21-claudine-schemas, 2026-10-23-shell-template-approval,
2026-10-24-sequence-containers); 2026-10-26-task-inputs-and-providers is not
implemented, so task inputs stay `params:`.

### Wave 1: inventories

Three read-only inventories ran in parallel. Their reports are the Phase 1
deliverables, and later phases read them:

- `phase-1/dependency-audit.md`: delivered behavior with file:line evidence,
  the seams, the cause of the looping-document defect, the `on_rate_limit`
  inventory, every loose loop verb's signature, and a proposed owner for each
  shared file (§7).
- `phase-1/path-inventory.md`: every Darkmatter path function's return type,
  missing-target and URL behavior, and errors; the Phase 2 path fixture table
  (§5); and the smallest change sites (§6).
- `phase-1/coverage-inventory.md`: existing tests, test registration rules,
  where the prompt tests live, how route drift derives its hash, which formats
  each entry point accepts, and the acceptance-criterion → test → task map
  (§6).

**Dependency gate: PASS.** Lifecycle grammar (node tree, last-item rule,
planned `break`/`prep` in the schemas and the `PLANNED` parity list), shell
template approval (exact versus template, `ShellPreApprovals` on
`PreflightGraph`), and sequence containers (`ContainerRunner`, `cancelled`,
group `fail_fast`, `seq`/`group`/`loop` roots) are all in code. Task
`setup:`/`teardown:` is rejected grammar; only dead plumbing remains.
`break` and `prep` are parsed nowhere. The things the plan says this feature
removes are still present, as expected: loop `fail_fast`, predictive
`is_last` (`looping/engine.rs:284-306`), and the action applied after the
last pass (`engine.rs:496-522`).

Baseline caveat: the sequence-containers review fixes are uncommitted in this
worktree (20 modified claudine files and 3 untracked L1 tests). Phase 2 must
start from that tree and must not stash or reset it.

The defect in spec §3 is a routing gap. The loop engine is reached only from
`prepare_and_run_active_document` → `execute_loop_or_single`. A `prompt:`
task goes `WrapperPromptRunner::run` (`cli/wrap/sequence/task_run.rs:620`) →
`execute_composition_request_inner`, which runs a single attempt and never
reads `loop:`. A document reached by `proxy` inside a step does loop. The
shipped `review-loop.md` → `implement-plan.md` route hits the defect today.

### Compatibility ruling: `on_rate_limit` (plan rule 8)

Inventory: `loop.on_rate_limit` (`pause`/`abort`/`continue`, default
`pause`) and `--on-rate-limit` act only in the document loop, between
iterations, when the provider stream reports throttling. `abort` exits 75.
No shipped prompt or frozen fixture uses it. A container has no provider
stream, so the key cannot mean anything on a group or sequence loop.

The author has not ruled; this session is non-interactive. Recommendation,
recorded as a human review item: keep it for documents only, reject it with a
typed error on container `loop:` blocks, and apply the policy only after the
condition and cap decide another pass will run. That replaces today's
predicted-last-pass skip. **Until the author rules, current behavior is
preserved**, and Phase 2 must not remove or extend the key.

### Other rulings and findings

- **Empty collections** (plan rule 9): `gate: []` and `action: []` are
  explicit no-ops. `gate: null`, `gate: {}`, `action: ""`, and `loop: {}`
  are errors. `prep.with: {}` is an explicit empty overlay; `with: null` is
  an error, as for `proxy.with`. Today `gate: null` and `gate: {}` parse as an
  empty gate, so Phase 2 tightens them.
- **Duplicate keys**: YAML frontmatter and standalone YAML already reject
  them. JSON, JSON5, JSONL, and NDJSON are last-wins today. Phase 2 adds a
  duplicate-detecting read for executable task sources, before the
  `serde_json::Value` conversion.
- **Unclosed frontmatter fence** reads as empty frontmatter (Darkmatter). It
  is kept: it is Markdown grammar, which this feature does not change.
- **`implemented` guard**: characterized with `md compose`
  (`phase-1/matrix-contracts.md`, review metadata). A quoted `'true'` counts
  as implemented under `!= true`. The spec's semantics are kept.
- **`review_iterations` presence**: `frontmatter()` returns null for absent
  and for explicit null. The migrated prompt uses the existing
  `has_key(frontmatter(spec), 'review_iterations')` to tell them apart; no
  new read is needed.
- **`file_exists` swallows errors**: permission probe errors, invalid
  references, missing home or vault, and failed fetches all read `false`
  (`darkmatter/.../functions/mod.rs:1619-1622`). Spec §10 says such errors
  never become missing-file success. Recorded as a human review item for
  Phase 2.
- **`dirname` expectation changes**: preserving the absolute anchor reverses
  outputs pinned by `darkmatter/lib/src/markdown/compose/tests/schema.rs:565,
  795, 874`, `claudine/cli/tests/l1/compose_caller_file_provenance.rs:86`, and
  possibly `darkmatter/cli/tests/l1/compose_schema_file_rewrite.rs:146`.
  The spec requires the change; Phase 2 updates these on purpose and logs it.
- **`review-loop.md` agents**: the live prompt takes `review_agent` and
  `repair_agent`; the spec's version takes one `agent`. Recorded for the
  author before Phase 5.
- **`commit.md`** already ends `initialize` with `skip`, so spec §9's edit
  has landed. Only its acceptance test (AC12) remains.
- **Windows recorder gap**: `launch_recorder` is Unix-only, and no portable
  prompt recorder exists. New prompt-recording CLI tests are `cfg(unix)`, as
  existing loop tests are, until a portable recorder lands.
- **Stale comment noticed, left as is** (no behavior change in this phase):
  `StackExecutionContext::scope` (`lifecycle/executor.rs:579`) still
  mentions task `setup:`/`teardown:` stacks. The owner of that file fixes it
  in Phase 2.

### Wave 2: control-flow spike

Ran once and passed 6/6. It drove the real `ContainerRunner` and
`LifecycleRunGuard` with fake bodies, as a temporary in-crate test that was
removed afterwards. The archive and the interface contracts are in
`spike/README.md` and `spike/loop_flow_spike.rs`. Summary:

1. A break settles a container through the existing guard (`defuse()`, then
   `finalize` once). No new guard state is needed, and Drop synthesizes no
   terminal event.
2. A failed `finalize` (`Finished { failed: true }` with a primary error)
   discards the pending break.
3. Group pass settlement is a pure declaration-order fold, run after every
   leg settles. A handled leg failure keeps the break; `fail_fast` failure
   wins.
4. `prep` is a returning **action** through an injected runner, like
   `shell_runner`, not a `StackControl`. The executor already continues the
   list and after a conditional branch.
5. **New seam needed:** `RuntimeState` cannot restore its mutation layer.
   `replace_sequence_scope` restores outputs only. Add a pass-checkpoint
   restore that keeps `proxy_chain`.
6. Call ancestry can reuse `enter_container_proxy`'s RAII guard: repeated
   calls are legal, active-ancestor calls are refused, and branches are
   isolated. Cancellation uses `current().child()`. **No deadline carrier
   exists**; add an absolute-instant `CallDeadline` that a child can only
   tighten.

### Matrix contracts

`phase-1/matrix-contracts.md` expands every column: each loose verb from its
real signature, each format's duplicate-key and trailing-content behavior,
and the measured review-metadata decisions. Each cell is marked covered,
Phase N, or N/A.

Implemented the parameterized traversal for the one format that carries a
loop field today, YAML frontmatter on a document, as
`claudine/cli/tests/l1/loop_config_matrix.rs`. It is registered `cfg(unix)`
in `cli/tests/l1/main.rs` because it uses the Unix-only `launch_recorder`.
It has a `matrix!` macro with one `#[test]` per cell: 36 cells, each one
textual edit of the counting control, asserting exit status, launch count,
prompt text, that a rejection names the document, and that no audio spool
exists. All 36 pass, the slowest at about 1.6 s.

Deviation: the plan asks for one traversal per supported format. The other
formats (standalone YAML, JSON, JSON5, JSONL/NDJSON) carry only container
and task fields that do not exist until the Phase 2 grammar, so their
traversals are written in Phase 2 against the contract table. The probe run
measured 18 cells that differ from the contract today. They are not
registered, so `just test` stays green, and each is listed with its current
result in the contract:

- **11 unlocated errors**: `LoopInvalid` messages carry no document path.
  The cells are `loop: null`, `loop: {}`, a missing condition, null
  `until`/`while`/`max`/`action`/`actions`, both conditions together, both
  aliases together, and `action: ""`. Phase 2 makes the loop error located.
- **Accepted when they should be rejected**: `fail_fast: true`,
  `gate: null`, and `gate: {}`.
- **Rejected when it should resolve**: templated `max: "{{ cap }}"`.
- **Rejected but unlocated**: `fail_fast: null` and `on_rate_limit: null`.
- **Wrong exit status**: `max: 2` with the condition unmet exits 0, because
  the action runs after the last pass. The loop engine fixes this in Phase 3.

### Acceptance mapping

Every criterion (1–14) is mapped to an observable assertion, a proposed test
file, and its owning plan task in `phase-1/coverage-inventory.md` §6. That
table is the Phase 1 acceptance mapping.

### Verification (macOS)

- `just test` in `claudine/`: 9892 passed, 0 failed, 9 skipped, 27 slow. All
  slow tests are pre-existing. No `loop_config_matrix` test is slow; the
  longest takes about 2.7 s under full-suite load.
- `just lint` in `claudine/`: first run failed on `clippy::unnecessary_to_owned`
  in the new matrix macro's control row. Fixed, then clean.
- `cargo nextest run -p claudine-cli --test l1 -E 'test(loop_config_matrix)'`
  after the fix: 36/36 passed.
- `just check-tier-coverage claudine`: nothing stranded. The new tests carry
  no tier marker and run in L1.
- Not run: Darkmatter tests (no Darkmatter code changed) and other OS rigs.
  The only new test is `cfg(unix)` subprocess composition with no path or
  OS-specific logic, so Linux and WSL CI cover it. The temporary spike module
  was removed and `lifecycle/tests/mod.rs` is back to its prior content.

Requirement-to-test mapping for this phase: Phase 1 changes no behavior. Its
one test deliverable is the robustness-matrix traversal: 36 cells, each
named in `phase-1/matrix-contracts.md`. The 18 cells that differ from the
contract today are listed there with their measured current result and the
phase that changes them. The spike tests were a one-time experiment and are
archived, not registered.

Phase 1 status: complete, ready for review. Phase 2 is blocked on the
`on_rate_limit` ruling and, for the path work, on the `file_exists` ruling
(see `human_review_items`).

## Phase 2

Phase 2 started 2026-10-16 (claude/opus). The worktree was clean at start:
the sequence-containers review fixes the Phase 1 log calls uncommitted were
committed before this phase (`ec331c303` and earlier), so no stash or reset
question arose.

### Rulings taken for this phase

The session is non-interactive and the author's answers to the Phase 1
human-review items had not arrived. Each item that blocks Phase 2 is decided
the conservative way below and listed again for the author.

- **`loop.on_rate_limit`: kept for documents, rejected on container loops**
  (Phase 1 option A). Document behavior is unchanged. A group or sequence
  `loop:` naming the key gets a typed, located error. This can be relaxed
  later without breaking anyone; accepting it now could not be undone.
- **`file_exists` reports probe errors** (Phase 1 option A). Spec §10 is
  explicit: permission errors, invalid references, and remote-fetch failures
  are never turned into missing-file success, and existence checks keep their
  missing-file behavior. A genuinely absent file still answers `false`.
- **Grammar ahead of behavior.** Phase 2 parses and validates container
  `loop:`, task `when:`, `break`, and `prep`, but their behavior arrives in
  Phases 3 and 4. Between those phases a valid construct is refused with a
  typed "not yet supported" error rather than accepted and ignored, because
  an ignored `when:` would run a guarded task and an ignored container loop
  would reproduce the run-once defect. The refusal is removed by the change
  that lands each behavior.

### Author rulings (2026-10-17)

The author confirmed or settled every open decision before Phase 3:

1. **`loop.on_rate_limit`:** documents only; a group or sequence `loop:` naming it is a located error. Confirms the Phase 2 ruling.
2. **`file_exists()` probe failures are errors**; a genuinely absent file (or a directory) is `false`. Confirms the Phase 2 ruling.
3. **`file_exists()` takes `string | null`.** `null` (an optional reference never supplied) is `false`; any other non-string argument is a type error. Implemented in this phase.
4. **`absolute()` on a recursive `%` reference:** an existing match returns its path; no match is a `not_found` error (path-inventory row 6b).
5. **A `break` reason may interpolate** (`Expr`, like `error`'s reason), departing from the plan's `Option<String>`.
6. **A malformed `CLAUDINE_MAX_ITERATIONS` fails only runs that would use it**; under `--max-iterations` it is never read. Implemented in this phase.
7. **The review-loop migration is deferred.** `prompts/review-loop.md` was retired; `just fix` stays the loop. Phase 5 skips "Review prompt migration" and acceptance criterion 11 (see the Phase 5 note below). The one-agent-or-two question is moot.

### What Phase 2 delivered

- **Loop configuration** (`looping/config.rs`): one parser for every owner (`parse_loop_block` with `LoopOwner` and a located `LoopLocation`), strict shapes, did-you-mean for unknown keys, `fail_fast` as removed grammar, `on_rate_limit` document-only, templated `max` resolved once.
- **Lifecycle directives:** `break` (reason, code `0..=255`) and `prep` (target, `with`) parse everywhere but `cancelled`; non-final `prep` is permitted; both are refused at run time in `executor.rs::resolve_control` until Phase 4. `gate: null` and `gate: {}` are parser errors.
- **Sequence and group grammar:** container `loop:` on group objects, sequence roots, and inline nested sequences, validated then refused (`ContainerLoopNotYetSupported`); task `when:` validated then refused (`TaskWhenNotYetSupported`); `TaskLoopMisplaced` for a `loop` on any other task; `WhenMisplaced` at a sequence root or on a group object; JSON-family duplicate keys rejected before the lossy value conversion (`sequence/data.rs`, `DuplicateScan`).
- **Schemas:** `claudine/schemas/{action,lifecycle,claudine}.yaml` carry the new keys, with planned markers for run-time refusal.
- **Darkmatter path contracts:** `absolute` builds a missing target's first candidate and rejects HTTP(S); `dirname` and the file-index family keep an absolute input's anchor; `file_exists` reports probe failures.
- **Robustness matrices:** `loop_config_matrix` (YAML frontmatter), `lifecycle_directive_matrix`, and `sequence_grammar_matrix` (YAML frontmatter, standalone YAML, JSON, JSON5, JSONL, NDJSON), each a control row plus one-edit cells asserting the public result.

### Deviations

- `break` reason is an expression (ruling 5).
- The task guard is validated but not stored on the preflight task; Phase 4 adds the field.
- `file_exists`'s permission-error row is Unix-only; Windows would need an ACL fixture.
- The matrices were written before the robustness rule changed (field shapes once per validator, in process); they spawn one process per cell, and 23 NDJSON cells repeat the JSONL ones through the same loader.

### Process note

Phase 2's first run took over three hours instead of 30–40 minutes. Three parallel subagents each ran the full claudine suite (about 10,000 tests) on one host, and its load average reached about 200; timing tests then failed or timed out, and each failure triggered another full rerun. Every timed-out test passed when rerun alone. `prompts/_headless-orchestration.md` and the `rust-testing` skill now require subagents to run only their slice's tests, with the full suite once per phase by the orchestrator.

### Verification (macOS, 2026-10-17)

Run one at a time, with no other build or test on the host:

- `just test` in `claudine/`: 10,226 passed, 9 skipped.
- `just lint` in `claudine/`: clean (the only warning is the macOS linker's unwind-table notice).
- `just check-tier-coverage claudine`: 0 stranded.
- `just test` in `darkmatter/`: 10,127 passed, 10 skipped, after two test fixes for Phase 2's schema changes: `darkmatter-cli`'s `schema_validate` test now expects the union message for `loop.max: 0` (the field is a count or a `{{ cap }}` template), and `dmls`'s `lifecycle_inventory_matches_the_authored_claudine_schema` now treats `fail_fast` as removed grammar the parser recognizes but the schema omits.
- `just lint` in `darkmatter/`: clean.
- Darkmatter's other dependents (biscuit-icon, biscuit-speaks, messenger, playa, research, sniff) were not run: none uses the changed expression functions, and Phase 2's only Darkmatter API change is additive.
- Not run: native Windows, Linux, and WSL2 for the final tree. The path-contract work was cross-checked on native Windows in its own worktree; CI covers the rest.

Phase 2 status: complete, ready for review. Phase 3 starts from `phase: 3` in the plan.

### Note for Phase 5: the review loop changed after this spec closed

Recorded 2026-10-17, outside this feature's own work, so the Phase 5 "Review
prompt migration" task does not undo it.

- **`prompts/review-loop.md` was deleted on purpose.** It had never run, and
  the loop in use is `just fix` (`just/review.just`). Spec §11 and acceptance
  criterion 11 describe migrating a file that no longer exists; settle with
  the author whether Phase 5 recreates it as a looping sequence or migrates
  `just fix` instead, before starting that task.
- **A ready review now gets one follow-up pass.** `implement-suggestions.md`
  fixes the review's small medium and low findings, records the rest and
  every observation in the area's `fixes/_unscheduled/<spec>-follow-ups/`
  backlog, and marks the review `implemented`; no review follows it.
  `just fix` runs that pass after a ready review and on a rerun whose ready
  review is still open. Review frontmatter `findings` now lists every
  priority (`<ID> (<priority>): <title>`); only a high finding makes `ready`
  false.
- **This departs from spec §11**, whose loop breaks on `ready` before any
  repair, and from criterion 11's "first review ready, no commit step
  launches". The departure is intended: whatever replaces `just fix` must
  keep the follow-up pass. `claudine/cli/tests/prompts/review_fix_rehearsal.rs`
  is the behavioral contract to carry over.
