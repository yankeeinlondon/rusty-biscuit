---
spec: /Volumes/coding/wt/rusty-biscuit/feat-work-reduction/claudine/features/2026-10-25-pipeline-work-reduction/spec.md
plan: claudine/features/2026-10-25-pipeline-work-reduction/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1:
    - claudine/lib/tests/l1/host_catalog.rs
    - darkmatter/cli/src/commands/render.rs
    - darkmatter/lib/benches/pipeline_work_support/mod.rs
    - darkmatter/lib/benches/schema_validation.rs
docs_updated_during_phase_1:
    - claudine/features/2026-10-25-pipeline-work-reduction/spike-report.md
docs_created_during_phase_1:
    - claudine/features/2026-10-27-shared-launch-trigger-discovery/spec.md
    - claudine/features/2026-10-27-shell-site-placeholder-parse-once/spec.md
skills_files_updated_during_phase_1:
    - .claude/skills/rust-testing/performance-testing.md
packages:
    - claudine
    - darkmatter
    - darkmatter-cli
    - claudine-cli
    - content-policy
    - test-toolkit
source_files_during_phase_2:
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/looping/container.rs
    - claudine/lib/src/composition/looping/expression.rs
    - claudine/lib/src/composition/looping/expression/tests.rs
    - claudine/lib/src/composition/looping/expression/tests/loop_leaf_reads.rs
    - claudine/lib/src/composition/runtime_state.rs
    - claudine/lib/src/composition/runtime_state/tests.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/templates.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/sequence/task/tests/runtime_projections.rs
    - claudine/lib/src/dispatch/expression.rs
    - claudine/lib/src/dispatch/expression/tests.rs
    - claudine/lib/src/dispatch/expression/tests/hook_leaf_reads.rs
    - claudine/lib/src/work_counters.rs
    - darkmatter/justfile
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/benches/pipeline_work_support/mod.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/content_policy_duration_parity.rs
    - darkmatter/lib/tests/l1/schemas_named_import_errors.rs
    - darkmatter/lib/tests/l1/schemas_named_type_memo.rs
docs_updated_during_phase_2:
    - claudine/docs/topics/configuring-actions.md
    - claudine/docs/topics/flow-control/looping.md
    - claudine/docs/topics/flow-control/sequences.md
    - content-policy/docs/topics/policy-lifecycle.md
    - darkmatter/docs/topics/schemas/definition.md
docs_created_during_phase_2:
    - claudine/features/2026-10-25-pipeline-work-reduction/completion-walk-design.md
skills_files_updated_during_phase_2:
    - .claude/skills/claudine/SKILL.md
decisions_for_review:
    - |-
      The saved timing baseline (`pipeline-work-pre`) was taken with the computer under moderate load, not on a quiet one. The plan asked for a quiet host; the spike report suggested a load average of about 2. Other worktrees kept compiling, so after waiting 30 minutes I ran the pass once the 1-minute load fell under 8, the bench recipe's own limit (half of 16 cores). It began at 7.7 and ended at 12.3. Every anchor's Criterion interval was within about ±1% of its median.
      Alternatives: keep waiting for a load near 2, which could take hours and was not guaranteed; or take the pass anyway and record the load, as I did.
      If the reviewer wants a quieter baseline, rerun the two `just _bench … --save-baseline pipeline-work-pre` commands in the log on a quiet host before phase 5's comparison. Nothing else depends on it.
    - |-
      I classified placeholder parsing (D8) as qualifying and wrote its own spec (`2026-10-27-shell-site-placeholder-parse-once`). The decision depends on which repeated parses count as D8's. While preparing one sequence shell command, Claudine parses each placeholder five times, once per analysis pass. Removing four of the five is 6.4% of anchor A6's time (over the 5% line); the bytes are 3.2%. Counting only the two repeats that Darkmatter's isolated template code shows gives 3.2% of time, which would close D8 as "measured, no change".
      I chose the wider count because all five passes run inside one site preparation, which is what D8 describes ("placeholders reparsed across analysis passes"). The time share uses the count × unit-cost method the spike used, with Criterion medians from the moderately loaded pass above.
      If the reviewer prefers the narrower count, delete that spec's directory and change D8's row in `spike-report.md` → "Current outcome (2026-10-27)" to "measured, no change".
    - |-
      Two baseline gates were already failing at the starting revision. I fixed both instead of only recording them, because the plan requires these gates to pass and the fixes are mechanical and outside the measured code.
      (1) Darkmatter lint: `darkmatter/cli/src/commands/render.rs::run_render` takes 8 arguments; I added `#[allow(clippy::too_many_arguments)]`, as five other commands in that crate already do.
      (2) The test-toolkit archive-path guard flagged three compile-time manifest paths: a Claudine L1 test (`host_catalog.rs`) and two Darkmatter benches (`pipeline_work_support/mod.rs`, `schema_validation.rs`). I spelled them `biscuit_test_harness::manifest_dir!()`, the guard's remedy.
      Alternatives: leave them failing and report them, or add allowlist entries for the two benches (they never run from an archive). Undoing either fix is a one-line revert per file.
    - |-
      Darkmatter's normal test run now turns on its work counters. The plan's regression tests for the named-type memo (and, in phase 3, the validator-cache items) count work, and those counts compile out without the `work-counters` feature. Claudine already enabled it; Darkmatter did not, so CI would never have run those tests. I added `work-counters` to Darkmatter's `just test` recipe and to its CI test feature lists in `darkmatter/lib/Cargo.toml`.
      Alternatives: run the counter tests only through a separate opt-in command (CI would skip them), or prove the memo behavior without counting.
      To undo: revert the two-line changes in `darkmatter/justfile` and `darkmatter/lib/Cargo.toml`; the one counting test would then compile out.
    - |-
      Hook and loop leaf reads (C2/C4) cover expressions and loop conditions, but not loop values read inside lifecycle actions or the document body. Those still receive the whole `loop` object, built eagerly in `claudine/lib/src/composition/lifecycle/bindings.rs`. The spike's cost ("`loop` built twice per pass") came from the condition's reads, which are fixed. Making the lifecycle binding lazy changes how Claudine hands globals to Darkmatter, which is a larger change than the plan's two named files.
      Alternatives: make that binding lazy now (wider change across the Darkmatter host-globals seam), or leave it, as I did, and say so in the docs.
      To redo: give `EnclosingLoop` a per-member lookup and add a counter test like `loop_leaf_reads::work_counts`.
    - |-
      In a loop condition, `loop.last_output.<key>` is still absent, as before, while lifecycle templates resolve it through the full object. I kept the old condition behavior because changing it is a behavior change, not a work reduction.
      To align them later: let `LoopAmbient::member_named` descend into `last_output`, and add equivalence cases.
    - |-
      C3 also stopped a task with no `when:` guard from building the full `seq` value (all outputs) just to skip guard evaluation. It was the main remaining per-leg copy and its result was unused. It is a one-line early return in `TaskExecution::guard()`.
      Alternatives: leave it for a later item. To undo: remove that return.
    - |-
      C3's output-copy tests assert that copies do not grow with history size (the same count at 10 and 100 outputs) rather than pinning exact numbers (currently 12 and 30), so later lifecycle changes don't break them for unrelated reasons.
      To pin them: add `assert_eq!` on those two values in `runtime_projections.rs`.
    - |-
      The named-type memo drops a path's entry when a read or parse fails, rather than keeping the last good table that can no longer be served. The memo then never holds a table that differs from the file's last read. Cost: restoring a broken file to its previous good text parses once instead of zero times.
      Alternative: keep the old entry (it is never served, since the hash must match). To switch: remove the `remove` call on the failure path in `load_named_types`.
    - |-
      A failing import file is parsed twice per schema resolution: discovery ignores the load error and expansion re-reads and reports it (`darkmatter/lib/src/markdown/schemas/resolve/graph.rs`). This was true before and is outside the memo change, so I left it. It only costs work on the error path.
    - |-
      The completion-walk design (X2) chose exact per-alternative masks over a simpler "prune on the union" rule, and keys shared walks by the exact root spelling rather than the canonical folder. It also notes that the larger saving may be per-file canonicalization inside biscuit-file, which needs an API change and its own measurement. These are recorded in `completion-walk-design.md` for the follow-up spec to decide.
    - |-
      Group branches (C1): the command-line path is checked through a test-only file seam, not through the work counters. Claudine's command-line test build in CI does not turn on the counters, so a test-build-only environment variable (`CLAUDINE_TEST_GROUP_BRANCHES`) makes each finished group write its branch counts (declared legs, cap, built, peak, live) to a file. It follows the existing speech pre-warm test seam. The library tests use a counter that tracks how many branches are alive and the most that ever were.
      Alternative: give the command-line crate its own counter feature. That means changing its manifest, justfile and CI feature lists, removing the seam, and rewriting the two command-line tests.
      Also: a group container's own private state (one per container, not per leg) is not counted as a branch. And each new branch still copies the whole group-start snapshot. Sharing one copy between branches would save more bytes but needs a change to how runtime state is stored.
    - |-
      Provider prediction (C7): how the counter is counted changed. A document hint that has to be evaluated now counts two state builds per task, shared by `agent` and `model`; before, it counted one, plus one for each document key. Literal hints count zero, which is the goal.
      Left alone deliberately: a task's dependencies are tracked for the task, not per key. So when `agent` reads a value written at runtime, a literal `model` in the same document is also decided at launch. This is conservative and was already the behavior. Splitting dependencies by key would change prediction results, so it belongs in its own small fix (about 30 lines plus tests).
    - |-
      Union-arm validators (D1), four choices:
      (a) Validators for arms of a property-level union are compiled without the caller's origin folders. Inside an arm compiled on its own, a `match()` rule cannot tell which top-level property it belongs to. So, as before, a caller-supplied value's `match()` inside a property union is judged from the document's folder, while ordinary validation judges it from the caller's folder. That difference predates this change. Fixing it means tagging the arm with its top-level property (about half a day).
      (b) A small behavior fix: arms of a root-level union now use the caller's origin folders, so whether an arm accepts a value agrees with what validation decided. To undo it, pass empty origins.
      (c) The public free function `rewrite_eager_file_values` now uses the process-wide validator cache instead of compiling fresh on every call. Going back to a per-call cache is a one-line change.
      (d) Each arm carries all of the root's shared definitions, not only the ones it reaches, so editing an unrelated definition recompiles every arm. Pruning to the reachable ones can come later.
    - |-
      Preparation summary (D2): the summary reports only whether a schema applied (`has_schema`). The plan said "presence and timing metadata", but the command line already gets timings, matched triggers and cache state by wrapping the call in its existing timing recorder. Copying them into the summary would make every caller pay for recording them. The summary type is marked non-exhaustive, so a field can be added later without breaking callers. The new call returns a pair (report, summary) rather than a named struct.
    - |-
      Deferred positions (D4): which part of the source a problem points at (its key, its value, or the enclosing mapping) is now worked out from the finished problem's code and property, not from the validator's raw error. The alternative was a hidden field on the public `ValidationProblem` struct, which other code builds directly. The results are the same today, and the tests compare every case with the old eager path.
      Not done: composition's earlier pre-shell report still builds one position map when it has problems that composition later sets aside. Avoiding that means carrying the deferred map through composition's filtering, a medium-sized change.
    - |-
      Alphanumeric checks (D13): the check lives in `simplified/convert.rs`, not `resolve.rs` as the plan said, so that file was changed. To avoid editing the validator cache while another agent was working in it, a compile is counted after checking the cache first. If two threads miss on the same constraints at the same moment, the count can be one too high. Results are unaffected. For an exact count, expose the cache's existing lookup-or-compile function (which reports whether it compiled) and use its flag, a one-line change in each file.
    - |-
      I did not repair the one timing slowdown that reproduced: D5 (a lifecycle stack of 1,000 conditionals) ran 6.8% slower than the saved baseline on a second, isolated run. Its counted work, allocations, bytes and peak are identical to the spike's figures, and no code it runs is in this feature's diff, so the slowdown is host noise on a memory-heavy case, not something this change caused. The plan only asks to resolve regressions attributable to the change.
      Alternatives: investigate D5 now (outside this feature's items, and the plan forbids repeated measurement campaigns), or open a spec for it.
      If the reviewer wants it chased: rerun `cargo bench -p darkmatter --bench pipeline_work -- --baseline pipeline-work-pre D5/conditionals/1000` on a quiet host first; nothing else depends on it.
    - |-
      The timing comparison ran under moderate load (1-minute load 7.2 falling to 6.1), similar to or slightly lighter than the baseline pass (7.7 rising to 12.3). I did not wait for a quiet host, because the baseline itself was not quiet and matching the conditions matters more than an absolute quiet. The large changes (A5 -99.5%, A1 -24%, A4 -2.8%) are far outside Criterion's intervals; small ones (under about 3%) should not be read as real.
      If the reviewer wants cleaner numbers: save a new baseline and compare on a quiet host at the parent revision and this branch; the log's commands are reusable.
    - |-
      The documentation pass fixed two things beyond this feature's own changes: a broken code fence in `darkmatter/docs/topics/schemas/definition.md` (Candidate Linting) that predates the feature, and a sentence in `tools/test-toolkit/README.md` that said a library never declares the counting allocator (Claudine's unit-test binary does). It also corrected `claudine/docs/cli/sequence.md`, whose example said a plain `model` is always predicted as written; when `agent` reads a runtime-written key, both are decided at launch. Each is a small, Markdown-only revert if unwanted.
    - |-
      The before/after anchor figures are recorded only in the implementation log, as the spec's acceptance item 7 asks. I did not add an "after" section to `spike-report.md`, which records the spike and the pre-implementation refresh. If the reviewer wants the report to carry the outcome too, copy the log's "Anchor comparison" table into a new report section.
source_files_during_phase_3:
    - claudine/cli/tests/l1/group_branch_lifetime.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/providers.rs
    - claudine/lib/src/composition/sequence/preflight/providers/tests.rs
    - claudine/lib/src/composition/sequence/preflight/providers/tests/literal_hints.rs
    - claudine/lib/src/composition/sequence/task/container_regressions.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/group/branch.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/sequence/task/tests/branch_admission.rs
    - claudine/lib/src/composition/sequence/task/tests/pipeline_spike.rs
    - claudine/lib/src/pipeline_spike/tests.rs
    - claudine/lib/src/work_counters.rs
    - claudine/lib/tests/l1/context_construction_guard.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_validate_summary.rs
    - darkmatter/lib/src/markdown/schemas/bound_operands.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/positions.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/tests/lazy_positions.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/work_counters.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schemas_alphanumeric_check_cache.rs
    - darkmatter/lib/tests/l1/schemas_rewrite_arm_validators.rs
    - darkmatter/lib/tests/l1/schemas_validation_summary.rs
docs_updated_during_phase_3:
    - claudine/docs/cli/sequence.md
    - claudine/docs/topics/flow-control/sequences.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/topics/schemas/definition.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_4:
    - claudine/cli/tests/l1/compose_named_type_import.rs
    - claudine/cli/tests/l1/main.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5:
    - claudine/docs/cli/sequence.md
    - claudine/docs/topics/flow-control/sequences.md
    - darkmatter/docs/topics/schemas/definition.md
    - tools/test-toolkit/README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/SKILL.md
source_code:
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/tests/l1/compose_named_type_import.rs
    - claudine/cli/tests/l1/group_branch_lifetime.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/looping/container.rs
    - claudine/lib/src/composition/looping/expression.rs
    - claudine/lib/src/composition/looping/expression/tests.rs
    - claudine/lib/src/composition/looping/expression/tests/loop_leaf_reads.rs
    - claudine/lib/src/composition/runtime_state.rs
    - claudine/lib/src/composition/runtime_state/tests.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/providers.rs
    - claudine/lib/src/composition/sequence/preflight/providers/tests.rs
    - claudine/lib/src/composition/sequence/preflight/providers/tests/literal_hints.rs
    - claudine/lib/src/composition/sequence/task/container_regressions.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/group/branch.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/templates.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/sequence/task/tests/branch_admission.rs
    - claudine/lib/src/composition/sequence/task/tests/pipeline_spike.rs
    - claudine/lib/src/composition/sequence/task/tests/runtime_projections.rs
    - claudine/lib/src/dispatch/expression.rs
    - claudine/lib/src/dispatch/expression/tests.rs
    - claudine/lib/src/dispatch/expression/tests/hook_leaf_reads.rs
    - claudine/lib/src/pipeline_spike/tests.rs
    - claudine/lib/src/work_counters.rs
    - claudine/lib/tests/l1/context_construction_guard.rs
    - claudine/lib/tests/l1/host_catalog.rs
    - darkmatter/cli/src/commands/render.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_validate_summary.rs
    - darkmatter/justfile
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/benches/pipeline_work_support/mod.rs
    - darkmatter/lib/benches/schema_validation.rs
    - darkmatter/lib/src/markdown/schemas/bound_operands.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/positions.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/tests/lazy_positions.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/work_counters.rs
    - darkmatter/lib/tests/l1/content_policy_duration_parity.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schemas_alphanumeric_check_cache.rs
    - darkmatter/lib/tests/l1/schemas_named_import_errors.rs
    - darkmatter/lib/tests/l1/schemas_named_type_memo.rs
    - darkmatter/lib/tests/l1/schemas_rewrite_arm_validators.rs
    - darkmatter/lib/tests/l1/schemas_validation_summary.rs
documentation:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/rust-testing/performance-testing.md
    - claudine/docs/cli/sequence.md
    - claudine/docs/topics/configuring-actions.md
    - claudine/docs/topics/flow-control/looping.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/features/2026-10-25-pipeline-work-reduction/completion-walk-design.md
    - claudine/features/2026-10-25-pipeline-work-reduction/spike-report.md
    - claudine/features/2026-10-27-shared-launch-trigger-discovery/spec.md
    - claudine/features/2026-10-27-shell-site-placeholder-parse-once/spec.md
    - content-policy/docs/topics/policy-lifecycle.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/topics/schemas/definition.md
    - tools/test-toolkit/README.md
completed_phase: 5
implemented: true
---
---

# Implementation Log for 2026-10-25-pipeline-work-reduction (5 phases)

## Phase 1

### Prerequisites

- Revision: `d7ea96a1f` (branch `feat/work-reduction`); the only uncommitted file at start was this log (empty).
- Host: macOS development host, 16 cores; load average 4.4–5.4 at start (another worktree was running `claudine compose` review jobs).
- Both dependency chains are in that revision:
  - `2026-10-26-dmls-frontmatter-typing-allocation` (last planning commit `aa8f41143`);
  - `2026-10-26-sequence-quadratic-frontmatter` (Phase 4 close `7d67c5473`);
  - `2026-10-25-fix-now` (`98c86432c`) → `2026-10-25-host-function-catalog-leak` (`d7310764d`) → `2026-10-25-schema-diagnostic-provenance` (`dcb13ab9e`) → `2026-10-25-shared-acyclic-definitions` (cycle 2 close `df6ea2d9d`).
- X1 is delivered: `darkmatter/dmls/src/overlay/analysis.rs` keeps prepared `AlternativeMatcher`s keyed by `alternative_fingerprint` across snapshots. As the spec's re-baseline decision says, the A3 hover reference is now the post-sharing 66,277 B (verb) / 69,943 B (expression), replacing 58,763 / 105,025.

### Tooling inventory (existing; nothing recreated)

- Counters: `darkmatter::work_counters` and `claudine::work_counters` behind each lib's `work-counters` feature (`pub mod work_counters` is `#[cfg(feature = "work-counters")]`). `note_work!` expands to a `#[cfg(feature = "work-counters")]` statement, so a default build has no counter state or calls. DMLS enables Darkmatter's feature only as a dev-dependency.
- Counting allocator (`test_toolkit::alloc_count::CountingAllocator`) is declared only in final binaries: `darkmatter/lib/benches/pipeline_work_alloc.rs`, `claudine/lib/benches/pipeline_work_alloc.rs`, `darkmatter/dmls/benches/pipeline_work_a3.rs`, and Claudine's lib unit-test binary under `cfg(all(test, feature = "work-counters"))` for the crate-private `pipeline_spike_*` scenarios (C1, C3, A4 group step, C6, C7 per task).
- Timing targets `darkmatter/lib/benches/pipeline_work.rs` and `claudine/lib/benches/pipeline_work.rs` declare no global allocator (System).
- Fixtures: `darkmatter/lib/benches/fixtures`, `pipeline_work_support/`, `tests/fixtures/recursive_schema_frozen`.
- Report rows: `spike-report.md` has a row for every C1–C8, D1–D13, P1, X1–X4.
- Docs: `tools/test-toolkit/README.md` (`alloc_count`) matches the tooling. The `rust-testing` skill's `performance-testing.md` named only bench targets as allocator owners; it left out the Claudine unit-test-binary exception and how to run those scenarios. I added that paragraph (the only omission repaired).

### Baseline gates (before any phase-1 change)

| Gate | Result |
| ---- | ------ |
| `darkmatter/ just test` | 12,908 passed, 10 skipped |
| `darkmatter/ just lint` | **failed (already on `main`)**: `clippy::too_many_arguments` on `darkmatter/cli/src/commands/render.rs::run_render` (8 arguments), which arrived with the `main` merge `d7ea96a1f`. Fixed with the crate's existing `#[allow(clippy::too_many_arguments)]` idiom (used on five other commands); no behavior change. Lint then passed |
| `claudine/ just test` | 11,709 passed, 15 skipped |
| `claudine/ just lint` | passed |
| root `just test test-toolkit` | **1 failure (already present)**: `archive_path_guard::no_archive_executed_target_bakes_in_a_producer_path` reported three `env!("CARGO_MANIFEST_DIR")` sites: `claudine/lib/tests/l1/host_catalog.rs:331`, `darkmatter/lib/benches/pipeline_work_support/mod.rs:757`, `darkmatter/lib/benches/schema_validation.rs:338`. Fixed by spelling each as `biscuit_test_harness::manifest_dir!()`, the guard's remedy (no allowlist entry), and dropping the now-unused `Path` import in `schema_validation.rs`. Then 477 passed, 4 skipped |
| root `just _lint test-toolkit` | passed |

### Wave 2: count and allocation refresh

Toolchain `rustc 1.98.1 (48a229cea 2026-09-01)`, `cargo 1.98.1 (797e8a9bc 2026-08-05)`, `bench` profile (release settings) for the bench targets and `--release` for the crate-private scenarios. Revision `d7ea96a1f` plus this phase's uncommitted edits (the lint attribute, the three `manifest_dir!()` spellings, the skill paragraph); none of them touches measured code. The host was **loaded** for every count run: another worktree was building and testing `biscuit-terminal`, and other agents were running. Counts and bytes do not depend on load. Times printed by these runs are counting-allocator observations under load and are not used for any verdict.

Commands (repository root, `BENCH_NONINTERACTIVE=1 BENCH_YES=1` because the host-state preflight refuses a loaded host in a non-interactive session):

| Command | Load before → after |
| ------- | ------------------- |
| `just _bench darkmatter --features work-counters --bench pipeline_work_alloc` | 15.41 → 27.15 |
| `just _bench claudine --features work-counters --bench pipeline_work_alloc` | 27.15 → 36.25 |
| `just _bench dmls --bench pipeline_work_a3` | 36.25 → 32.09 |
| `claudine/`: `just test-library --release --features work-counters --run-ignored only --no-capture pipeline_spike_c6` | 17.64 |
| `claudine/`: `just test-library --release --features work-counters --run-ignored only --no-capture pipeline_spike_a4` | 18.99 |

Harness accounting needed no repair. Each scenario resets the peak (`reset_peak`) immediately before it is measured, and the group-step scenario joins its workers before the final snapshot.

Results that changed from the report's last figures:

- **A3** (DMLS edit loop) reproduces the post-sharing figures recorded by `2026-10-25-shared-acyclic-definitions`: `a3x1` warm total 52,848,942 B (diagnostics 37,090,371 B; overlay 14,968,060 B), cold 345,580,121 B; `a3x10` warm 98,051,049 B, cold 390,782,228 B. Hovers are verb 66,277 B and expression 69,943 B. DMLS's own counters are zero in every warm loop.
- **A2** (Claudine bundle, warm / cold bytes): `no_trigger_match` 3,130,659 / 54,362,491; `document_events` 3,209,451 / 56,000,826; `nested_sequence_group` 3,223,037 / 60,422,122; `implement_plan` 4,667,124 / 331,148,774. The cold rows fell from 334–810 MB with sharing.
- **A1**: corpus validate warm 88,826,897 B; with the runtime baseline warm 93,888,707 B, cold 180,609,812 B (was 202.6 MB).
- **A4**: preflight 91,614,330 B (peak 22,457,613; `PredictionStateBuild:98`); group step 624,837,390 B (peak 149,752,757; `RuntimeSnapshot:615`, `RuntimeStateBranch:200`, `LiveFrontmatterView:200`). Total **716,451,720 B** (was 843.4 MB).
- **A6**: 10 / 100 steps → `ExpressionParse` 1,000 / 10,000 (of them `DeferredRootScanParse` 200 / 2,000); bytes 3,200,966 / 26,709,585.
- **C7** per-task bytes 43.80 MB / 100 tasks and 424.59 MB / 1,000 tasks (`PredictionStateBuild` 2 per task), the same as the post-sequence-fix rows.
- **D1** (1,000 × 3-item eager list): normalize only 162,444,780 B with 6,000 `RewriteArmCompile`; isolated rewrite compiles 2 validators per item at every size.
- **D2**: validate then presence check 159,053,226 B vs validate only 88,805,226 B at 1,000 documents (the second preparation is 44.2%).
- **D4**: positions cost 88,805,226 − 73,834,949 = 14,970,277 B of A1 validation (16.9%); schema-free 6,450,188 vs 1,434,911 B.
- **D12**: `edit_and_reuse` 10 paths → `NamedTypeTableParse` 1; 300 paths → 300 (the memo still clears past 256).
- **D13**: `suggested_imports` 1 / 10 / 100 → `AlphanumericCheckCompile` 2 / 11 / 101.
- Closed items (C5, C8, D5, D6, D7, D9, D10, D11, X3, X4) reproduce their spike counts; their verdicts do not change.

### Wave 2: timing baseline `pipeline-work-pre`

- Waited for a quiet host; other worktrees kept the load high. After 30 minutes the pass began at a 1-minute load of 7.70 (5-minute 8.20), which passes the `_bench` preflight (limit 8, half the cores) but is above the "about 2" the spike report asked for.
- Commands, from the repository root (both benches pre-built with `--no-run` so nothing compiled during timing):
  - `just _bench darkmatter --bench pipeline_work -- --save-baseline pipeline-work-pre`: load 7.70 → 7.03, 12:48:26–12:52:14;
  - `just _bench claudine --bench pipeline_work -- --save-baseline pipeline-work-pre`: load 7.03 → 12.29, 12:52:14–12:52:51.
- Default features, so no counters; system allocator; `bench` profile; toolchain as above. Criterion's intervals were narrow: every anchor case was within about ±1% of its median (for example A6 8.1460 / **8.2117** / 8.2866 ms; A4 35.132 / **35.349** / 35.599 ms; A1 warm 23.535 / **23.755** / 23.909 ms). So the pass was not repeated. The full table is in `spike-report.md` → "Current outcome (2026-10-27)".
- DMLS A3 has no Criterion target; its elapsed times come from the counting allocator under load and are recorded as instrumented observations only.

### Wave 2: conditional items (Decision 4)

- **C6, measured, no change.** `pipeline_spike_c6`: `PrewarmStateBuild` is still 1 per phrase, but classification builds over the shared frontmatter view (`EffectiveStateBuilder::with_shared_frontmatter`), so a phrase costs 15 KB at both 10 KB and 200 KB of frontmatter. Before, it was 53 KB and 757 KB. 20 phrases over 200 KB: 301,844 B (was 15.2 MB). Peak 13 KB. The largest real phrase count is 6 (`prompts/_pr/push.md`), so clause 2 would need about 660 phrases. A4 estimate 0.1%.
- **D3, qualifies (clause 1, estimated bytes 5.8%).** The share rose from 5.0% because A4's total fell from 843.4 MB to 716.5 MB. Created `claudine/features/2026-10-27-shared-launch-trigger-discovery/spec.md`.
- **D8, qualifies (clause 1, time 6.4%).**
  - A6 parses each placeholder 10 times per step. To attribute those parses, I added a temporary `D8_PARSE_SITES` backtrace print at `Parser::with_mode`'s counter, ran `cargo bench -p claudine --features work-counters --bench pipeline_work_alloc --profile dev -- A6` once, and **removed it** (`git diff darkmatter/lib/src` is empty).
  - The tally: exactly one parse per placeholder at each of ten sites. Five are passes inside Claudine's `SequencePreflight::prepare_site` (`parse_authored`'s typing, `approval_diagnostics`, `first_target_identity_root`, `project_types`, `mark_late_holes`). The other five are two frontmatter `value_roots` scans, `reject_removed_sequence_reads`, and two interpolations.
  - Removable: 4 × 1,000 = 4,000 parses × 132.18 ns = 0.529 ms of 8.2117 ms. Bytes 3.2%.
  - Created `claudine/features/2026-10-27-shell-site-placeholder-parse-once/spec.md`.
- Every other item keeps its outcome (report section "Every other item").

### Phase 1 gates (after every edit)

| Gate | Result |
| ---- | ------ |
| `claudine/ just test` | 11,709 passed, 15 skipped |
| `claudine/ just lint` | passed |
| `darkmatter/ just test` | 12,908 passed (2 slow), 10 skipped |
| `darkmatter/ just lint` | passed |
| root `just test test-toolkit` / `just _lint test-toolkit` | 477 passed, 4 skipped / passed |
| `cargo check -p darkmatter --benches --features work-counters` | no warnings |

- **Requirement-to-test mapping.** This phase adds no behavior, so it adds no behavior test. Its source edits are a lint attribute, a test's path spelling, and two bench path spellings. The existing tests that cover them:
  - `claudine/lib/tests/l1/host_catalog.rs::claudine_ships_no_host_function_catalog_and_no_facts_artifact` (run alone and in the suite);
  - `tools/test-toolkit/tests/archive_path_guard.rs::no_archive_executed_target_bakes_in_a_producer_path`, now passing;
  - the Darkmatter benches, which compiled and ran in the count and timing passes above.
  The input-robustness matrix does not apply: no reader or loader changed.
- **Dependent packages.** None retested beyond the two areas: the edits change no public symbol or behavior (the `run_render` attribute is lint-only).
- **Cross-OS.** No cross-OS check: `manifest_dir!()` is the repository's portable spelling and nothing OS-specific changed.
- No pre-existing failures remain. The two found at the start (Darkmatter lint, test-toolkit archive guard) are fixed above.
- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `packages` = `claudine`, `darkmatter`, `darkmatter-cli`, `test-toolkit`; `change_class` = `package`. The gates could not be skipped, and all four packages' gates ran and passed. `test-toolkit` is in scope because its archive guard reads the edited files; none of its own files changed.

## Phase 2

- Start revision `41f78bc0b` (branch `feat/work-reduction`), clean tree. Host macOS, 16 cores.
- Coordinator wiring before the wave: Darkmatter's L1 recipe (`darkmatter/justfile` `test`, local and CI branches) and `darkmatter/lib/Cargo.toml` `[package.metadata.ci.tests] features`/`local-features` now enable `work-counters`, as Claudine's already do. Without it Darkmatter's count regressions (D12 here; D1/D13 in phase 3) compile out in CI's per-package run.
- Wave 3 ran as five parallel subagents with disjoint files: C3 (runtime projections), C2+C4 (hook and loop leaf reads), D12 (named-type memo), P1 (duration parity matrix), X2 (completion-walk design). Each ran only tests scoped to its change; the area gates ran once afterwards.

### C3: narrow runtime-state projections

- `RuntimeState` gains owned projections, each taken under one lock that is released before returning: `mutations()`, `mutation_keys()`, `mutations_and_outputs()`, `mutations_and_output_count()`, `outputs_from(start)`, `sequence_scope()`. `replace_sequence_scope` now returns the scope it replaced, so a nested sequence moves the enclosing outputs out and back instead of copying them. New counter `Work::RuntimeOutputsCopy` counts output entries copied by a snapshot or an outputs projection.
- Converted callers: task `mod.rs` (`verify_preflight_approvals`, `mutation_snapshot`, `mutation_delta`, `sequence_binding`), `templates.rs`, `group.rs` (live-frontmatter extend, parent `set_batch` reads, `member_state`/`member_state_from`), `sequence.rs` (scope entry/exit, event/step/gate reads, pass result), `looping/container.rs` (`PassCheckpoint::take`, `container_state`), `lifecycle/executor.rs` `bindings()`, CLI `wrap/sequence/container.rs` (`RootRun::record_pass`, root frontmatter extend) and `task_run.rs`.
- Kept on a full `snapshot()` because they need the whole state: the group seed (`group.rs:71`) and group-start base (`group.rs:444`), and every `layered_set_overrides` input (task `layered_overrides`, CLI `iterate.rs`, `container.rs`, `compose/prep.rs`, `harness_orch/prompt.rs`).
- A4 group step (`pipeline_spike_a4`, release, `work-counters`): `RuntimeSnapshot` 615 → **2**; bytes 624,837,390 → **192,153,235**; peak 149.75 MB → 147.05 MB (the peak is the 200 per-leg branches, which C1 removes in phase 3).
- Tests: `runtime_state::tests::{field_projections_take_no_full_snapshot_and_copy_only_the_outputs_they_return, projections_equal_the_matching_fields_of_a_full_snapshot, multi_field_projections_are_coherent_under_a_concurrent_writer, replace_sequence_scope_moves_the_enclosing_scope_out_and_back}`; `sequence::task::tests::runtime_projections::{a_shell_task_reads_runtime_state_without_a_snapshot_or_an_output_copy, a_nested_sequence_moves_the_enclosing_outputs_instead_of_copying_them, a_looping_sequence_copies_only_its_own_passes, a_group_takes_two_full_snapshots_at_any_leg_count}` (counter tests under `work-counters`). Retry clearing, pass restoration, and lexical restoration are proved by the existing `container_scope_restore_and_retry_keep_native_results_and_mutations`, `nested_sequences::*`, `container_group_contract::retry_restarts_legs_without_intermediate_finalize_or_duplicate_result`, `container_regressions::inner_group_and_sequence_roots_are_lexical_and_restored_after_return`, and CLI `loop_retry_passes::*`.
- Focused runs: library 1,074 passed (`runtime_state:: runtime_projections composition::sequence:: composition::looping:: composition::lifecycle::executor composition::lifecycle::bindings`); CLI 221 passed (loop, sequence container/group, task guard, provider, JIT modules).
- Comment drift fixed: the doc comment above task `sequence_binding` described `late_values`; it moved to `late_values` and `sequence_binding` got its own.

### C2 and C4: hook and loop leaf reads

- C2 (`dispatch/expression.rs`): `EvaluationLookup::get` sends `hook.<path>` to `hook_member`. `tool_input`, `tool_response`, and `extra` are walked by borrow and only the leaf is cloned; `os`/`hardware`/`git`/`project` build only their own small group (same absent/null rules); string fields resolve directly; anything else, including `env`, is absent and never falls through to another root. Bare `hook` still builds the whole payload.
- C4 (`looping/expression.rs`): `LoopAmbient::member_named` builds only the named member for `loop.<member>`. Bare `loop` and bracketed `loop['count']` still build the whole object. The module doc said a missing member is `null`; the code returns absent (an expression shows it as `null`); the doc now says so.
- Nothing is cached; every lookup borrows the live event or loop ambient.
- Tests: `dispatch::expression::tests::hook_leaf_reads::{hook_leaf_reads_equal_the_whole_payload_projection (58 paths), hook_leaf_reads_equal_the_projection_when_optional_members_are_missing, hook_env_stays_absent_and_a_missing_member_reads_no_other_root, hook_leaf_reads_see_each_event_current_values, work_counts::hook_leaf_reads_never_build_the_payload_beside_a_large_response, work_counts::a_whole_hook_read_builds_the_payload_once}`; `looping::expression::tests::loop_leaf_reads::{loop_leaf_reads_equal_the_whole_object_member, loop_leaf_reads_see_each_pass_current_values, work_counts::loop_leaf_reads_never_build_the_object_beside_a_large_result, work_counts::a_whole_loop_read_builds_the_object_once}`. Large sibling 1 MB; zero materializations for leaf reads, exactly one for a whole-root read.
- Focused runs: 60 (`dispatch::expression looping::expression`), 460 (`dispatch:: composition::looping pipeline_spike`), CLI loop modules 42, all passed.
- Remaining gap (decision recorded): `loop.*` reads in lifecycle actions and the body go through `lifecycle/bindings.rs` `EnclosingLoop::value`, which still supplies `loop` eagerly. Only condition reads, the ones the spike counted twice per pass, are leaf reads now.

### D12: latest named-type table per path

- `darkmatter/lib/src/markdown/schemas/resolve.rs`: `FILE_NAMED_TYPES` is now `Mutex<HashMap<PathBuf, FileTable { hash, table: Arc<NamedTypes> }>>` keyed by the same resolved path value as before. `FILE_NAMED_TYPES_CAPACITY` (256) and `FileRevision` are gone. Every use still reads, records `observed::read`, checks UTF-8, and hashes with `biscuit_hash::xx_hash_bytes`; a matching hash serves the stored `Arc`, otherwise it parses and replaces the entry. A failed read/UTF-8/parse removes the entry and returns the error, so no old table is ever served. Held `Arc`s keep their revision.
- Count regression `schemas_named_type_memo::named_type_memo_reuses_every_path_past_256_files_and_reparses_only_an_edit`: 300 files → 300 parses cold, 0 on reuse, 1 after one edit, 0 again (was 300 on reuse). Behavior tests in the same module: malformed after good (located `SchemaDocument` error, never the old table), malformed cold then repaired, deleted file (cold and warm, then recreated), a schema resolved before an edit keeps its revision. Unit tests `resolve::named_type_file_memo::{a_held_table_keeps_its_revision_after_the_entry_is_replaced, a_failed_read_or_parse_drops_the_entry}`.
- Loader matrix: existing coverage proves duplicate keys, trailing content, second document, `types:` null, root union, whole-file reference, malformed YAML, valid-YAML-not-schema, tagged control, and the shared envelope validator cells (`schema_enhancements_matrix/envelope.rs`). New in `schemas_named_import_errors`: invalid UTF-8, JSON-compatible YAML plus pure `$schema` control, an invalid sibling entry failing an import of a valid one (tagged and pure), and an empty `types` mapping reporting the imported name as unknown. CLI-boundary cells remain phase 4's.
- Code-smell grep over the changed loader: none in `load_named_types`/`read_named_types`/`named_types_from_text`. Noted outside the memo: `simplified/standalone.rs:254` `filter_map(as_str)` affects only an error message's key list; `resolve/graph.rs:262–270` ignores a namespace load error during discovery and re-reports it at expansion, so a failing import file parses twice per resolution (pre-existing, unchanged).
- Focused runs: 138 passed (`named_type|named_import|resolve::tests|shared_type_tables`), DMLS `named_type` 20 passed.
- Docs: `darkmatter/docs/topics/schemas/definition.md` cache-lifetime paragraph rewritten; the stale "past 256 revisions" bench comment in `darkmatter/lib/benches/pipeline_work_support/mod.rs` corrected.

### P1: duration lexical parity matrix

- New `darkmatter/lib/tests/l1/content_policy_duration_parity.rs` (registered in `l1/main.rs`): one roster of 114 rows (309 cases after expanding `d`/`wk`/`mo`/`yr`), five rules in eight forms (`Evergreen`, `TimeSensitive`, `ValidFor` default/property/inline-date baseline, `ValidUntil` date/property, `FileChanged`). Exhaustive matches over `DurationUnit` and `Rule` fail compilation when either grows.
- Tests: `roster_covers_every_rule_form_and_unit`, `parser_meets_the_roster` (`Policy::from_declaration`), `editor_fragments_meet_the_roster` (the 8 `short_form` patterns from `EDITOR_SCHEMA`), `shipped_schema_through_darkmatter_meets_the_roster` (one document validated once through `DarkmatterSchemas` with the baseline, ~0.28 s). 6 passed with the layout gate.
- 11 intentional differences (editor accepts, parser rejects): counts over `u32::MAX`, impossible calendar dates, an inline date plus duration out of range, a path template in `FileChanged`. No genuine disagreement found. Production code unchanged.
- Docs: `content-policy/docs/topics/policy-lifecycle.md` gains a rule-spelling table and states that the parser is authoritative and editor patterns are lexical only, checked by the parity test.

### X2: completion-walk candidate design

- Delivered `completion-walk-design.md` beside this log (design only, no code). One walk per identical (exact root spelling, walk settings) key per request; per-directory alternative masks so a directory is pruned only when no alternative wants it; authored-order merge and existing dedup unchanged; request-owned listings dropped at return; next request walks fresh. It records the spike's timing sample and notes that per-file canonicalization inside `lists_file` is a separate, possibly larger saving that needs a biscuit-file API change.

### Phase 2 gates (after every subagent finished; run sequentially)

| Gate | Result |
| ---- | ------ |
| `claudine/ just test` | 11,727 passed, 15 skipped (205 s; Phase 1 was 11,709) |
| `claudine/ just lint` | passed |
| `just check-tier-coverage claudine` | 0 stranded |
| `darkmatter/ just test` (now with `work-counters`) | 12,923 passed (1 slow), 10 skipped. Phase 1 was 12,908; the +15 are exactly the D12 and P1 tests, so the counter-gated D12 test is compiled and ran |
| `darkmatter/ just lint` | passed |
| `just check-tier-coverage darkmatter` | 0 stranded |
| root `just test test-toolkit` / `just _lint test-toolkit` | 477 passed, 4 skipped / passed |
| Darkmatter dependents that resolve schemas: `cargo nextest run -p research -p messenger -p messenger-cli -E 'test(/schema\|frontmatter\|validat/)'` | 153 passed |

- No failures, no reruns, nothing load-sensitive. Claudine's dependents (`claudine-contract`, `claudine-cli`) are in its area run; Darkmatter's CLI and DMLS are in its area run. Other Darkmatter dependents (`biscuit-icon-cli`, `biscuit-terminal-cli`, `sniff-cli`, `biscuit-speaks-cli`, `playa-cli`) use no schema resolution, the only Darkmatter production code changed.
- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `packages` = `claudine`, `claudine-cli`, `darkmatter`, `test-toolkit`, `dmls`; `change_class` = `package`. Gates could not be skipped; all ran.
- Cross-OS: no `just cross-check`. No path comparison, path identity, `cfg(windows)` code, or process handling changed; D12 keys the memo by the same resolved path value as before. CI's Windows/WSL2 nightly covers the rest.
- Subagents ran `rustfmt` only on the new test files they created; no existing file was reformatted (`git diff --stat` shows no churn).

### Requirement-to-test mapping (phase 2)

| Requirement | Tests |
| ----------- | ----- |
| C3 projections take no full snapshot | `runtime_state::tests::field_projections_take_no_full_snapshot_and_copy_only_the_outputs_they_return`, `runtime_projections::*` (4) |
| C3 values equal full snapshot, coherent multi-field reads | `projections_equal_the_matching_fields_of_a_full_snapshot`, `multi_field_projections_are_coherent_under_a_concurrent_writer` |
| C3 retry clearing, pass restoration, lexical restoration, mutation order | `replace_sequence_scope_moves_the_enclosing_scope_out_and_back` plus existing container/nested-sequence/retry tests listed above |
| C2 zero payload builds for leaf reads; whole-root still builds | `hook_leaf_reads::work_counts::*` |
| C2 leaf = whole-root projection (scalars, nested, arrays, null, absent, reserved/unknown, large leaf); `hook.env.*` absent; no fall-through; fresh per event | `hook_leaf_reads::*` (4 behavior tests) |
| C4 zero loop-object builds for leaf reads; whole-root still builds | `loop_leaf_reads::work_counts::*` |
| C4 leaf = whole-object member; fresh per pass | `loop_leaf_reads::{loop_leaf_reads_equal_the_whole_object_member, loop_leaf_reads_see_each_pass_current_values}` |
| D12 >256 paths no unrelated reparse; edit reparses once | `schemas_named_type_memo::named_type_memo_reuses_every_path_past_256_files_and_reparses_only_an_edit` |
| D12 malformed, deletion, recovery, held old revision | `schemas_named_type_memo::*` (4), `resolve::named_type_file_memo::*` (2) |
| D12 loader matrix (missing cells) | `schemas_named_import_errors::*` (4 new) plus existing cells listed above |
| P1 parity matrix | `content_policy_duration_parity::*` (4) |
| X2 | design document, no tests (by plan) |

## Phase 3

- Start: branch `feat/work-reduction` at `41f78bc0b`, with Phase 2's changes still uncommitted (36 files). I hashed those files at the start so Phase 3's changes could be told apart from Phase 2's. Host: macOS, 16 cores.
- Wave 4 ran as four parallel foreground subagents, each owning separate files: C1, C7, D1 and D2. Wave 5 then ran D4 and D13 in parallel. Each subagent ran only tests scoped to its own change. I ran the area gates once, after both waves, with no subagent running.

### C1: admit private branches

- `run_parallel` (`claudine/lib/src/composition/sequence/task/group.rs`) keeps one immutable group-start seed: the existing full snapshot plus the live-frontmatter view, taken after eager validation and context capture. Each leg's `LegBranch` (`task/group/branch.rs`: an `Arc<RuntimeState>` built from the seed, plus its frontmatter cell) is created inside the worker after admission. It is dropped as soon as `run_guarded` returns the leg's outcome (result plus mutation delta), before that worker admits another leg. Never-admitted legs build nothing. A retry builds fresh branches from the retry's starting state.
- The CLI needed no source change. It runs groups through the library's `run_parallel`; `open_leg_run`/`open_group_run` do the context capture, which stays eager.
- Instrumentation:
  - `work_counters::Gauge::RuntimeStateBranch` (`enter`/`leave`/`live`/`peak`/`reset_peak`), compiled only with `work-counters`.
  - A `test-fixtures` seam, `CLAUDINE_TEST_GROUP_BRANCHES=<path>`, appends `{group, legs, cap, built, peak, live}` for each settled attempt. It exists because the CLI's CI build has no `work-counters`. The seam's one env read has an allowlist entry in `context_construction_guard.rs`.
- Tests:
  - `sequence::task::tests::branch_admission::*` covers capped, uncapped, delayed-leg seed isolation, nested, retried, interrupted, `fail_fast` with never-admitted legs, and tolerant declaration-order fold.
  - CLI: `group_branch_lifetime::{capped_group_branches_are_bounded_by_the_cap_and_built_once_per_leg, a_delayed_leg_reads_the_group_start_state_and_nested_legs_build_their_own}`.
  - Counter assertions were added to `container_regressions::sibling_failure_cancels_opted_container_descendants_and_preserves_other_legs`.
  - `runtime_projections::a_group_takes_two_full_snapshots_at_any_leg_count` and `leg_views::*` are unchanged and green.
  - Existing tests that cover the other invariants: `container_group_contract::{retry_restarts_legs_without_intermediate_finalize_or_duplicate_result, default_fail_fast_withholds_mutations_outputs_and_unadmitted_legs, tolerated_failed_legs_merge_mutations_and_publish_native_named_results, group_finalize_error_discards_leg_mutations_and_result, concurrent_legs_obey_cap_and_merge_declaration_order_without_shared_state}` and `parallel_groups::{a_sibling_mutation_is_invisible_until_the_group_completes, an_interrupt_fans_out_and_commits_neither_outputs_nor_mutations, a_contested_key_resolves_to_the_later_declared_task_and_warns}`.
  - The tests synchronize with condvars, never sleeps.
- Peak memory (ignored spike tests, debug build). The "after" figures are debug and the "before" figures release, so read the comparison as indicative only. Phase 5 re-measures in release.
  - A4 group step: 147.05 MB → 6.89 MB.
  - C1, cap 4, 10/100/1,000 legs: 3.69/22.60/212.2 MB → 2.35/2.73/6.77 MB.
  - Bytes and construction counts are unchanged; there is still one branch per admitted leg.
- Coordinator edit: `branch.rs` imports `RuntimeState` through a `crate::` path instead of four `super::` hops.

### C7: classify provider hints

- `predict` (`composition/sequence/preflight/providers.rs`) checks each hint for `{{ … }}`/`$( … )` before building anything:
  - A literal document, `with:` or setter hint is predicted as written and builds no state.
  - A document hint that reads an expression lazily builds the layered state (setters over `with:` over the document) and, only when it actually evaluates, the early-binding state. Each is built at most once per task, and `agent` and `model` share them.
- The caller's redundant copies are gone. `predict_provider` (`preflight/mod.rs`) borrows the prompt frontmatter, the step view and the context, and `HintLayers::document` is `&FrontmatterMap`.
- Tests: `preflight::providers::tests::literal_hints::*` (4 behavior tests and 4 counted tests):
  - a literal and the equivalent expression give the same selection;
  - precedence holds in both directions at every layer;
  - a runtime-written text key stays decided at launch;
  - late values defer without evaluation;
  - the literal path builds 0 `PredictionStateBuild`/`PredictionWithStateNodes`;
  - an expression builds 2 fresh states per task;
  - late and failing expressions keep their analysis;
  - in the preflight walk at 10 and 50 tasks, literal tasks build 0 and expression tasks 2 each.
  Existing `providers/tests.rs` and `preflight/tests/scaling.rs` pass unchanged.
- Bench `pipeline_work_alloc -- C7` (release), bytes and `PredictionStateBuild`:

  | Tasks | Bytes | `PredictionStateBuild` |
  | ----- | ----- | ---------------------- |
  | 10 | 5.64 → 2.22 MB | 20 → 0 |
  | 100 | 43.80 → 9.68 MB | 200 → 0 |
  | 1,000 | 424.59 → 83.35 MB | 2,000 → 0 |

- Spike test `pipeline_spike/tests.rs` no longer measures the "caller copy", which no longer exists.

### D1: reuse arm validators

- `rewrite.rs` builds an `ArmValidators` once per rewrite invocation. Each eligible arm's validator comes from the shared `ValidatorCache` once and is held for that call, memoized by arm address, so there is no per-item hashing or lookup.
  - Property-level arms are compiled whole with the root `$defs`, so no `$ref` dangles. They use the document context and no caller origins.
  - Root-union arms reuse the effective schema's wrapped arms with its fallback and caller origins, so they are cache hits shared with validation.
- `EffectiveSchema::normalize_frontmatter` calls the new `rewrite::rewrite_for_effective`. This is a one-call edit in `mod.rs`.
- The new `ValidatorCache::validator_for_reporting_compile` (`pub(crate)`) reports misses, so `RewriteArmCompile` counts only real compiles.
- Tests: `darkmatter/lib/tests/l1/schemas_rewrite_arm_validators.rs` (6):
  - a cold rewrite compiles 2 at 10, 100 and 1,000 items;
  - a warm rewrite, and a second request, compile 0;
  - an edited arm recompiles 1 and judges with its new body;
  - an arm that accepts only through a shared `$defs` definition is selected;
  - root arms get separate entries per origin context;
  - creating, removing and re-judging a file changes the verdict each time (nothing frozen);
  - zero or several accepting arms leave the item unchanged.
- Bench `-- D1` (release; "before" is the spike table):
  - isolated, 1,000 items: 557,007 allocations / 53.8 MB / 49.9 ms → 239,105 / 26.2 MB / 34.8 ms;
  - A1 normalize-only: 162.4 → 88.0 MB;
  - A1 prepare + validate + normalize: 345.2 → 272.1 MB.

### D2: return preparation summary

- New `DarkmatterSchemas::validate_summarized` returns `(ValidationReport, PreparationSummary { has_schema })`. `PreparationSummary` is `#[non_exhaustive]`.
- The private `validate_prepared(source, Option<SchemaPhase>)` is now the only place document-level reports are produced. `validate`, `validate_for_phase` and `validate_timed` delegate to it.
- `md schema validate` (`darkmatter/cli/src/commands/schema/validate.rs`):
  - its display-only `effective_for` call is gone;
  - the "has / doesn't have a schema definition" wording comes from the summary;
  - with assignments, it keeps the pre-assignment coercion preparation and the final one.
- Tests:
  - Library, `darkmatter/lib/tests/l1/schemas_validation_summary.rs` (9): no schema with the baseline disabled; baseline only; explicit `$schema`; trigger; preparation error; pending shell value; phase delegation; with assignments, 2 preparations, including a setter that switches the trigger from alpha to beta and one that turns a no-schema document into a schema document. Without assignments there is 1 preparation.
  - CLI, `darkmatter/cli/tests/l1/schema_validate_summary.rs` (2 tests, 3 spawns).
  - Existing output/format/exit/quiet/verbose/timing tests in `schema_validate*.rs` and `schema_triggers.rs` pass unchanged.
  - The preparation counts are asserted in the library, because the CLI is not built with `work-counters`.

### D4: deferred source positions

- Problems are now built without a location. `ProblemPositions::locate_all` (`positions.rs`) locates them once, at the end of `validate_instance`, after arm selection, narrowing, excusing deferred templates and operand binding have settled the final list.
- `validate_prepared` passes `ProblemPositions::deferred(source)`, so `positions_for` runs only when a report has problems, and once per report.
- Public entry points that take a caller-built `&PositionMap` behave as before.
- Which token a problem points at is now derived from the finished problem: `MissingRequired` points at the mapping, an offending property at its key, and anything else at the value. This is equivalent to the old per-error rule.
- DMLS and Claudine already pass the empty map, so they are unaffected.
- Tests: `darkmatter::markdown::schemas::tests::lazy_positions` (6 counted tests, 4 parity tests):
  - 0 maps for: a valid baseline-only document, a document with no schema, a pending shell value, a preparation failure, and a root union a later arm accepts;
  - 1 map for several problems, both unphased and at the Launch phase;
  - lines and columns equal the eager path's for: multiple problems, CRLF, alias fallback, a discriminated root union, assigned values, built-in code, unparsed frontmatter, and shared imported definitions.
  - Existing `schemas::tests::positions` and the CLI `(at line …, column …)` tests pass.
- Bench, release:
  - `-- A1` warm validation: 88,826,897 → 75,050,255 B, with `PositionMapBuild` 100 per 1,000 documents (the invalid ones only);
  - `-- D4`: positions now cost 1.12 MB, down from 14.97 MB.
- Remaining: compose's pre-shell report still builds one map when it holds problems that compose later defers.

### D13: cached alphanumeric checks

- The check lives in `darkmatter/lib/src/markdown/schemas/simplified/convert.rs::check_alphanumeric_values`, not in `resolve.rs` as the plan said.
- The new `alphanumeric_check_validator` hashes the lowered check schema: the options' pattern plus every string constraint, without the default or suggestion values. It then uses `SHARED_VALIDATOR_CACHE.cached_structural` on a hit, or `structural_validator_hashed` on a miss, and counts `AlphanumericCheckCompile` only on a miss.
- Each value is still judged on every call, and error construction is unchanged.
- Tests: `darkmatter/lib/tests/l1/schemas_alphanumeric_check_cache.rs` (4):
  - 1/10/100 imports compile once, and a warm repeat compiles 0;
  - different options compile separately and judge differently (`rb-c` with and without `allow-dashes`);
  - a rejected default stays rejected with an identical diagnostic, and a valid default under the same constraints compiles 0;
  - an inline type keeps its property name in the diagnostic.
  - The subagent also ran the diagnostic assertions against the old direct-compile code: they pass there, and the count test fails there (2 vs 1).
- Bench `-- D13`: `AlphanumericCheckCompile` is 1 at 1, 10 and 100 imports (was 101 at 100).

### Phase 3 gates (run sequentially, after every subagent finished)

| Gate | Result |
| ---- | ------ |
| `darkmatter/ just test` | 12,954 passed, 10 skipped (66 s; Phase 2 was 12,923) |
| `darkmatter/ just lint` | passed |
| `just check-tier-coverage darkmatter` | 0 stranded |
| `claudine/ just test` | 11,745 passed (1 slow), 15 skipped (242 s; Phase 2 was 11,727) |
| `claudine/ just lint` | passed |
| `just check-tier-coverage claudine` | 0 stranded |
| Darkmatter dependents that resolve schemas: `cargo nextest run -p research -p messenger -p messenger-cli -E 'test(/schema\|frontmatter\|validat/)'` | 153 passed |

- No failures and no reruns. Subagents reported SLOW tests during their parallel focused runs (`claudine_schemas::*`, `container_schema_recursion::*`, `sequence_prompt_property::sequence_with_prompt_property_runs_each_step_inline`). I attribute them to the concurrent agents' load: the sequential gate run reported one slow test, and that output did not name it. It passed.
- Claudine's dependents (`claudine-cli`, `claudine-contract`) ran in its area run. Darkmatter's CLI and DMLS ran in its area run. The other Darkmatter dependents (`biscuit-icon-cli`, `biscuit-terminal-cli`, `sniff-cli`, `biscuit-speaks-cli`, `playa-cli`) do not use schema validation, rewrite or alphanumeric checks.
- root `just test test-toolkit` / `just _lint test-toolkit`: 477 passed (1 slow), 4 skipped / passed. The scope calculator includes test-toolkit because its archive guard reads the changed files.
- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `packages` = `claudine`, `claudine-cli`, `darkmatter`, `darkmatter-cli`, `test-toolkit`, `dmls`; `change_class` = `package`. The gates could not be skipped, and all of them ran and passed. The tree still holds Phase 2's uncommitted changes too, so this scope is the union of both phases.
- Cross-OS: no `just cross-check`. Nothing touched path comparison, `cfg(windows)` code, or process handling. The C1 seam writes a JSON line to a path given by a `test-fixtures` env var, and its CLI test uses portable `side_effect` tasks.
- Subagents ran `rustfmt` only on new test files they created. No `cargo fmt`.

### Requirement-to-test mapping (phase 3)

| Requirement | Tests |
| ----------- | ----- |
| C1 peak branches ≤ active cap; one construction per admitted leg per attempt | `branch_admission::{a_capped_group_keeps_at_most_cap_branches_alive_and_builds_one_per_leg, an_uncapped_group_admits_every_leg_and_builds_one_branch_each, a_nested_group_bounds_live_branches_by_its_active_caps, a_retried_group_rebuilds_its_legs_from_the_seed}`; CLI `group_branch_lifetime::*` |
| C1 delayed leg sees group-start state; sibling isolation | `branch_admission::a_delayed_leg_reads_the_group_start_seed_after_its_sibling_released_its_branch`, CLI `group_branch_lifetime::a_delayed_leg_reads_the_group_start_state_and_nested_legs_build_their_own`, `parallel_groups::a_sibling_mutation_is_invisible_until_the_group_completes` |
| C1 interruption, fail-fast, never-admitted cancellation | `branch_admission::{an_interrupt_stops_admission_and_releases_the_admitted_branch, a_failure_stops_admission_and_unadmitted_legs_build_no_branch}`, `container_regressions::sibling_failure_cancels_…`, `container_group_contract::default_fail_fast_withholds_…` |
| C1 atomic/tolerant outcomes, declaration-order fold | `branch_admission::a_tolerant_group_folds_every_admitted_leg_in_declaration_order` plus the existing `container_group_contract`/`parallel_groups` tests above |
| C7 literal path: zero prediction state; expression path: fresh state | `literal_hints::counted::*` |
| C7 literal ≡ expression selection; precedence; runtime-written and late deferral; failures | `literal_hints::{a_literal_and_an_equivalent_expression_predict_the_same_selection, precedence_holds_between_literals_and_expressions_at_every_layer, a_runtime_written_text_key_keeps_its_hint_decided_at_launch, a_late_document_value_is_decided_at_launch_without_evaluation}`, `counted::late_and_failing_document_expressions_keep_their_analysis` |
| D1 compiles per distinct arm and context, not per item; warm 0 | `schemas_rewrite_arm_validators::a_cold_rewrite_compiles_each_arm_once_whatever_the_batch_size` |
| D1 changed body, shared defs, origins, file create/remove, exactly one arm | the other 5 `schemas_rewrite_arm_validators::*` tests |
| D2 one final preparation; two with assignments, including a trigger switch | `schemas_validation_summary::{assignments_prepare_twice_and_the_final_report_follows_the_assigned_trigger, an_assignment_that_selects_the_first_schema_turns_no_schema_into_a_schema}` and the 1-preparation cases |
| D2 no-schema, baseline-only, explicit, trigger, error, pending, formats, exits | `schemas_validation_summary::*`, CLI `schema_validate_summary::*`, existing `schema_validate*.rs` |
| D4 zero maps when valid or schema-free; one per multi-problem report | `lazy_positions::counts::*` |
| D4 spans, aliases, shared definitions, original vs composed sources unchanged | `lazy_positions::{several_problems_locate_as_the_eager_map_does, crlf_aliases_and_root_unions_locate_as_the_eager_map_does, original_constructed_and_unparsed_sources_locate_as_the_eager_map_does, shared_definitions_locate_each_use_as_the_eager_map_does}` |
| D13 one compile for equal constraints; different options apart; no verdict cache; diagnostics unchanged | `schemas_alphanumeric_check_cache::*` (4) |

- Input robustness matrix: no reader or loader changed in this phase. D1, D4 and D13 change validator and position lifetimes, not document parsing. Phase 4 owns the D12 loader cells.

## Phase 4

- Start revision `51b2e810c` (branch `feat/work-reduction`), clean tree. Host macOS, 16 cores. No subagents: the only code change was one CLI test file, and every gate ran once, sequentially, from this session.

### D12 loader: CLI-boundary cells

- New `claudine/cli/tests/l1/compose_named_type_import.rs` (registered in `l1/main.rs` under `#[cfg(feature = "test-fixtures")]`, which the area's `just test` enables). It uses `CliProcessFixture` and the compiled `claudine-fake-goose` with `PROVIDER_MARKER`, so it runs on every OS. The document is `$schema: { title: name@./types.yaml }` importing a tagged table `name: string(required; min(10))`.
  - `a_valid_named_type_import_launches_the_provider_once` — control: success, one launch.
  - `a_value_the_imported_type_rejects_stops_the_launch` — proves the import is load-bearing at the CLI: `title: short` fails, the report names `title`, no launch. The imported type is `required` because an optional rejected value is dropped with a warning and the launch goes ahead (the existing drop-and-warn rule; first draft of this test found it).
  - `a_malformed_imported_file_is_a_located_error_and_launches_nothing` (`types: [`) and `a_duplicate_key_in_the_imported_file_is_a_located_error_and_launches_nothing` — failure, stderr names `types.yaml` and says `not valid YAML`, no launch.
- The remaining loader cells stay in-process, where phase 2 left them complete: document shapes (duplicate key, trailing content, second document, invalid UTF-8, malformed YAML, JSON-compatible YAML, not-a-schema) in `darkmatter/lib/tests/l1/schemas_named_import_errors.rs`; the shared envelope validator's field shapes (`types` absent, null, number, list, every element wrong, `{}`, duplicate) in `schema_enhancements_matrix/envelope.rs`; mixed/all-invalid entries and empty-map-is-unknown-name in `schemas_named_import_errors`; cold and warm malformed/deleted/repaired reads in `schemas_named_type_memo`.
- Code-smell grep re-run over `load_named_types`, `read_named_types`, `named_types_from_text` (`darkmatter/lib/src/markdown/schemas/resolve.rs`): no `serde(default)`, `unwrap_or_default`, `.ok()`, or `filter_map`. Nothing in the loader changed after phase 2.

### Coverage audit (item-to-test map)

| Item | Removed-work regression | Behavior coverage |
| ---- | ----------------------- | ----------------- |
| C1 | `branch_admission::{a_capped_group_keeps_at_most_cap_branches_alive_and_builds_one_per_leg, an_uncapped_group_admits_every_leg_and_builds_one_branch_each, a_nested_group_bounds_live_branches_by_its_active_caps}`; CLI `group_branch_lifetime::*` | retries, interruption, fail-fast, never-admitted, delayed-leg seed, tolerant fold: `branch_admission::*` (8) plus the existing `container_group_contract`, `parallel_groups`, `container_regressions` tests listed under phase 3 |
| C2 | `hook_leaf_reads::work_counts::*` | `hook_leaf_reads::*` (58-path equivalence, missing members, `hook.env` absent, per-event freshness) |
| C3 | `runtime_state::tests::field_projections_take_no_full_snapshot_…`, `runtime_projections::*` (4) | projection equality, concurrent coherence, scope move, plus existing retry/nested/lexical tests (phase 2 list) |
| C4 | `loop_leaf_reads::work_counts::*` | `loop_leaf_reads::*` |
| C7 | `literal_hints::counted::*` | `literal_hints::*` (equivalence, precedence, runtime-written and late deferral, failures) |
| D1 | `schemas_rewrite_arm_validators::a_cold_rewrite_compiles_each_arm_once_whatever_the_batch_size` | the other 5 `schemas_rewrite_arm_validators::*` |
| D2 | `schemas_validation_summary::*` preparation counts | `schemas_validation_summary::*`, CLI `schema_validate_summary::*`, existing `schema_validate*.rs` |
| D4 | `lazy_positions::counts::*` | `lazy_positions::*` (4 eager-map equivalence tests) |
| D12 | `schemas_named_type_memo::named_type_memo_reuses_every_path_past_256_files_and_reparses_only_an_edit` | `schemas_named_type_memo::*`, `resolve::named_type_file_memo::*`, loader matrix above, CLI `compose_named_type_import::*` (new) |
| D13 | `schemas_alphanumeric_check_cache::*` | same module (diagnostics, different options apart, no verdict cache) |
| P1 | lexical matrix, not a counter (by spec) | `content_policy_duration_parity::*` |

- Registration: every module above ran in this phase's gate runs (checked in the nextest output, for example `branch_admission` 8, `group_branch_lifetime` 2, `runtime_projections` 4, `literal_hints::counted` 4, `compose_named_type_import` 4), and `just check-tier-coverage` reports 0 stranded for both areas. Counter features come from the area recipes: Claudine `claudine --features work-counters`, Darkmatter `work-counters` (phase 2 wiring).
- Unchanged public behavior (spec acceptance 10) is carried by the existing integration suites, which all passed: diagnostics and file resolution (`compose_schema_cli`, `launch_trigger_schemas`, Darkmatter `schema_validate*`), approval membership and lifecycle ordering (sequence preflight and container tests), retries (`loop_retry_passes`, `container_group_contract`), plus the L2 captures below. No attributable regression found; no repair needed.

### Phase 4 gates (run sequentially, no concurrent work)

| Gate | Result |
| ---- | ------ |
| `claudine/ just test` | 11,749 passed, 15 skipped, 0 slow (215 s; phase 3 was 11,745, +4 new) |
| `claudine/ just lint` | passed (only the existing macOS linker `__eh_frame` note) |
| `just check-tier-coverage claudine` | 0 stranded |
| `claudine/ just test-l2` | claudine-cli 289 passed; claudine-gen 3 passed |
| `darkmatter/ just test` (with `work-counters`, includes DMLS) | 12,954 passed, 10 skipped (80 s) |
| `darkmatter/ just lint` | passed |
| `just check-tier-coverage darkmatter` | 0 stranded |
| `darkmatter/ just test-l2` | 18 + 69 + 3 passed |
| root `just test test-toolkit` / `just _lint test-toolkit` | 477 passed, 4 skipped / passed |
| Default build, separate invocation: `cargo build -p darkmatter -p darkmatter-cli -p claudine -p claudine-cli --message-format=json` | lib artifacts' features: `darkmatter` = `[default]`, `claudine` = `[]`; no `work-counters`. Control `cargo check -p claudine --features work-counters` reports it, so the check can see it |
| Benches with their required features: `cargo check -p darkmatter --benches --features work-counters`, `cargo check -p claudine --benches --features work-counters`, `cargo check -p dmls --benches` | clean |

- No failures, no reruns, nothing load-sensitive. L2 runs use detached tmux (and kitty's `--keep-focus` where present); no window took focus.
- Headless browser checks not run: nothing in this work renders to a browser.
- Dependents: this phase changed only a Claudine CLI test file, so no dependent package needed retesting; phase 3's dependent runs still stand.
- Environments that actually ran: macOS only. No `just cross-check`: the new test uses the portable fake-goose pattern and no path comparison, `cfg(windows)` code, or process handling changed. Linux and macOS are proven by pull-request CI; Windows and WSL2 by the nightly.
- No `cargo fmt`; the new file was written by hand.
- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`, after every edit including this log and the plan): `packages` = `claudine-cli`, `test-toolkit`, `darkmatter`; `change_class` = `package`. The gates could not be skipped, and all of them ran and passed; the Darkmatter and test-toolkit gates above cover the two packages beyond `claudine-cli`.

### Requirement-to-test mapping (phase 4)

| Requirement | Tests |
| ----------- | ----- |
| D12 loader wired at the CLI: control launches | `compose_named_type_import::a_valid_named_type_import_launches_the_provider_once` |
| imported constraint is load-bearing at the CLI | `compose_named_type_import::a_value_the_imported_type_rejects_stops_the_launch` |
| malformed imported file: located error, no launch | `compose_named_type_import::a_malformed_imported_file_is_a_located_error_and_launches_nothing` |
| duplicate key in imported file: located error, no launch | `compose_named_type_import::a_duplicate_key_in_the_imported_file_is_a_located_error_and_launches_nothing` |
| default builds uninstrumented | default-build artifact feature check above |

## Phase 5

- Start revision `51b2e810c` (branch `feat/work-reduction`). Phase 4's two test changes (`claudine/cli/tests/l1/compose_named_type_import.rs`, its `l1/main.rs` line) and its log/plan edits were still uncommitted, so the working tree is phase 4 plus phase 5. Phase 5 changed no Rust source. Host: macOS, 16 cores, `rustc 1.98.1 (48a229cea 2026-09-01)`, `bench` profile.
- One documentation subagent ran (Markdown only, no builds) while the timing pass ran. Every measurement ran from this session, one at a time, with no build, test, or other measurement running alongside.

### Anchor comparison

Commands (repository root; every bench target was pre-built with `--no-run` first, so nothing compiled during timing; `BENCH_NONINTERACTIVE=1 BENCH_YES=1`):

| Command | Load (1-min) before → after |
| ------- | --------------------------- |
| `just _bench darkmatter --bench pipeline_work -- --baseline pipeline-work-pre` | 7.22 → 7.10 (14:56–15:00) |
| `just _bench claudine --bench pipeline_work -- --baseline pipeline-work-pre` | 7.10 → 6.14 (15:00–15:01) |
| `just _bench darkmatter --features work-counters --bench pipeline_work_alloc` | 6.05 → 5.88 |
| `just _bench claudine --features work-counters --bench pipeline_work_alloc` | 5.88 → 5.89 |
| `just _bench dmls --bench pipeline_work_a3` | 5.89 |
| `claudine/`: `just test-library --release --features work-counters --run-ignored only --no-capture pipeline_spike_{a4,c1,c3}` | 4.44 → 12.34 (the build raised it; counts and bytes do not depend on load) |

The saved baseline ran at 7.70 → 12.29, so the comparison host was about as loaded as, or slightly less loaded than, the baseline host. That favors the "after" figures a little; the intervals below are what Criterion reported, and no percentage is claimed beyond them. Times are system allocator, default features. Bytes come from the counting benches (warm unless marked).

| Anchor | Before: median (interval) | After: median (interval) | Criterion change (interval, p) | Bytes before → after | Direct work |
| ------ | ------------------------- | ------------------------ | ------------------------------ | -------------------- | ----------- |
| A1 corpus validate, 1,000, warm | 23.755 ms (23.535–23.909) | 17.820 ms (17.722–17.956) | −24.4% (−25.4…−23.5), p<0.05 | 88,826,897 → 74,976,218 | `PositionMapBuild` 100 (D4: maps only for documents with problems) |
| A1 + runtime baseline, warm / cold | 27.808 / 47.181 ms | 22.865 (22.742–22.992) / 41.928 ms (41.652–42.170) | −18.2% / −12.3% (−13.8…−11.0) | 93,888,707 → 80,038,028 / 180,609,812 → 166,761,765 | as above |
| A2 `implement_plan`, warm / cold | 2.3622 / 65.746 ms | 2.3520 (2.3350–2.3793) / 64.379 ms (63.907–65.026) | +1.1% p=0.61 / −1.4% p=0.08: no change | 4,667,124 → 4,667,124 / 331,148,774 → 331,143,333 | unchanged |
| A2 other bundles, warm bytes | — | — | within noise except `document_events-warm` (see regressions) | `no_trigger_match` 3,130,659 → 3,129,617; `document_events` 3,209,451 → 3,193,230; `nested_sequence_group` 3,223,037 → 3,196,526 | unchanged counts |
| A3 `a3x1`, warm / cold | instrumented 19.2 / 73.0 ms | instrumented 17.9 / 69.6 ms | no Criterion target | 52,848,942 → 52,848,942 / 345,580,121 → 345,582,753 | DMLS counters 0 in every warm loop; hovers 66,277 / 69,943 B (unchanged) |
| A3 `a3x10`, warm / cold | instrumented 42.1 / 95.1 ms | instrumented 38.8 / 91.2 ms | no Criterion target | 98,051,049 → 98,051,049 / 390,782,228 → 390,782,228 | unchanged |
| A4 sequence preflight (50 steps, 200 legs) | 35.349 ms (35.132–35.599) | 34.425 ms (34.150–34.683) | −2.8% (−3.6…−2.1), p<0.05 | 91,614,330 → 74,802,037 | `PredictionStateBuild` 98 → 0 (C7) |
| A4 group step (200 legs, cap 4) | — (no timing target) | — | — | 624,837,390 → 192,122,867; peak 149,752,757 → 7,115,564 | `RuntimeSnapshot` 615 → 2, `RuntimeStateBranch` 200 (C1, C3) |
| A4 total | — | — | — | **716,451,720 → 266,924,904 (−62.7%)** | |
| A5 event dispatch (100 events) | 130.90 ms (129.78–132.01) | 674.67 µs (669.56–679.47) | −99.48% (−99.49…−99.47), p<0.05 | 1,550,137,044 → 1,315,052 | C2/C4 leaf reads: no whole-payload `hook`/`loop` materialization |
| A6 shell preparation (100 × 10) | 8.2117 ms (8.1460–8.2866) | 8.1907 ms (8.0992–8.3324) | +0.9% (−0.4…+2.6), p=0.27: no change | 26,709,585 → 26,709,593 | `ExpressionParse` 10,000, unchanged (D8 is its own spec, not built here) |

C1 peaks re-measured in **release** (phase 3's were debug): A4 group step 7.12 MB peak (was 149.75 MB); C1 cap 4 at 10/100/1,000 legs, 0 KB frontmatter payload: 2.45/2.93/6.85 MB (was 3.69/22.60/212.2 MB). One construction per admitted leg (`RuntimeStateBranch` 10/100/1,000), two full snapshots per group at any leg count.

Item cases with a Criterion row (same pass): C2 `hook.tool_name` at 1 MB −99.985%; C4 `loop.count` at 1 MB −99.991%; C5 `evaluate_condition` −41.1% (C4 also serves its condition read); C7 `build_preflight_graph/tasks=100` −47.4%; D1 isolated rewrite −27…−29%, normalize only −25.1%; D2 `validate_only` −23…−25% and `validate_then_presence` −13…−15% (the bench still calls both, which is the library contract; `md schema validate` itself now prepares once); D4 `schema_free_validate` −89.1%, `a1_validate` −21.1%; D12 `edit_and_reuse/300` −8.6% (`NamedTypeTableParse` 300 → 1); D13 `suggested_imports/100` −79.6% (`AlphanumericCheckCompile` 101 → 1). Closed items C8, D3, D6, D7, D9, D10, D11, X4 are within noise or a few percent either way with unchanged counts; X3 −5.2%.

### Regressions checked

Five cases reported a slowdown in the full pass. I re-ran only those cases once against `pipeline-work-pre` (`cargo bench -p darkmatter --bench pipeline_work -- --baseline pipeline-work-pre <case>`, one invocation per case, because the bench pre-filter is a plain substring; load 5.48 → 4.42). I then judged each by its direct work:

| Case | Full pass | Re-run | Direct work vs. before | Verdict |
| ---- | --------- | ------ | ---------------------- | ------- |
| A2 `document_events-warm` | +3.6% (p=0.01) | +0.9% (p=0.25) | counts equal, bytes −16 KB | not reproducible |
| D4 `a1_no_positions_control` | +7.4% | −0.05% (p=0.92) | 73,834,949 → 73,830,949 B, same counts | not reproducible |
| D12 `edit_and_reuse/10` | +6.1% | +0.3% (p=0.69) | `NamedTypeTableParse` 1 → 1 | not reproducible |
| D8 `template_holes/1,10,50` | +1…+5% | +1.7…+2.7% | `ExpressionParse` 3 per hole, unchanged; template code not in this diff | not attributable |
| D5 `conditionals/1000` | +9.6% | +6.8% (+5.5…+8.1) | visits 1,504,497, allocations 184,680, bytes 58,716,764, peak 3,045,203: identical to the spike's row; lifecycle call-check code not in this diff (`git diff d7ea96a1f --stat -- darkmatter/lib/src` touches only `markdown/schemas/` and `work_counters.rs`) | reproducible but not attributable: same work, untouched code |

- No repair was made. D5's quadratic case allocates 58.7 MB per run and is the most sensitive to a loaded host's memory bandwidth. A real D5 change belongs to its own measurement, which this plan excludes (no repeated campaigns).
- The A2 re-run's first attempt went through `just _bench` with a `|`-joined filter. `just` passed it unquoted, so the shell read `|` as a pipe and Criterion panicked on a closed stdout. Nothing was saved (`--baseline` only reads the baseline). The direct `cargo bench` runs above replaced it. For later readers: pass one case per invocation, or call `cargo bench` directly.

### Documentation reconciliation

A documentation-only subagent checked every page this feature touched against the code. Fixes:

- `claudine/docs/cli/sequence.md`:
  - a YAML example's closing fence carried prose on the same line, so the block never closed; split it.
  - The example said `model: big  # predicted as written`, which is wrong when `agent` reads a runtime-written key: dependencies are tracked per task (`providers.rs::defer_runtime_written`), so both hints move to launch. The text and comment now say so (phase 3 recorded this rule as a decision; the doc had drifted from it).
- `claudine/docs/topics/flow-control/sequences.md`: added a Mermaid flowchart of group admission (seed once → admit next leg → build branch → run → keep result and writes → drop branch → next; never-admitted legs build nothing).
- `tools/test-toolkit/README.md`: said "a library never does" declare the counting allocator; Claudine's lib unit-test binary does under `cfg(all(test, feature = "work-counters"))`. Corrected.
- `.claude/skills/claudine/SKILL.md`: the same per-task `agent`/`model` rule in the provider-prediction paragraph.
- `darkmatter/docs/topics/schemas/definition.md`: a second fence with trailing prose (Candidate Linting). It predates this feature, but it broke the fence pairing of everything after it on a page this feature edits.

Checks that needed no change: C2/C3/C4/D1/D2/D4/D12/D13/P1 text matches the code; no `docs/` page or README names a feature or fix snapshot; no current doc describes the X2 shared completion walk (the existing "completion walks the glob" lines describe today's behavior, so nothing needs a **planned** marker); `performance-testing.md` already lists the four allocator owners correctly; no README overview claim changed; no crate was added, so no dependency doc changed.

### Phase 5 gates (sequential, nothing else running)

| Gate | Result |
| ---- | ------ |
| `claudine/ just test` | 11,749 passed, 15 skipped (210 s) |
| `claudine/ just lint` | passed (only the existing macOS linker `__eh_frame` note) |
| `just check-tier-coverage claudine` | 0 stranded |
| `darkmatter/ just test` (with `work-counters`, includes DMLS) | 12,954 passed (2 slow), 10 skipped (108 s); run because `definition.md` changed and the scope calculator lists `darkmatter` |
| root `just test test-toolkit` | 477 passed (4 slow), 4 skipped; run because its README changed |

- No failures and no reruns.
- No `just test-l2`, `darkmatter/ just lint`, or dependent-package runs: phase 5 changed no Rust source, so phase 4's L2, lint and dependent results still apply to identical code.
- No cross-OS check: no code changed.
- Scope calculator after every phase-5 edit (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `packages` = `claudine-cli`, `test-toolkit`, `darkmatter`; `change_class` = `package`. Not skippable; the gates above ran. `claudine-cli` is there because of phase 4's still-uncommitted test file.

### Closure audit

Spec acceptance list against the evidence:

| # | Criterion | Evidence | State |
| - | --------- | -------- | ----- |
| 1 | Tooling compiles away by default | phase 4 default-build artifact check (`darkmatter` = `[default]`, `claudine` = `[]`); no Rust changed since | met |
| 2 | Every C1–C8, D1–D13, X1–X4 has a report row, limitations kept | `spike-report.md` → "Current outcome (2026-10-27)" (phase 1) | met |
| 3 | `pipeline-work-pre` saved and compared on the same host, features, profile, workloads; revision recorded | phase 1 (saved at `d7ea96a1f`), phase 5 (compared at `51b2e810c` plus phase 4 test-only edits) | met; both passes were under moderate load (recorded above) |
| 4 | `just test` and `just lint` pass in darkmatter, claudine, test-toolkit; counters have empty disabled expansions | phase 4 and phase 5 gates; `note_work!` expands to a `cfg`-gated statement | met |
| 5 | Each implemented item has a removed-work L1 regression and behavior coverage | phase 4 → "Coverage audit (item-to-test map)"; every module ran in this phase's gate | met |
| 6 | D12, D13, P1 implemented | phase 2 (D12, P1), phase 3 (D13) | met |
| 7 | Log records each anchor's before/after timing, bytes, uncertainty; reproducible attributable regressions resolved | "Anchor comparison" and "Regressions checked" above | met; D5's reproducible slowdown has unchanged work and untouched code, so it is not attributable |
| 8 | test, test-l2, lint pass in every touched area including DMLS; default builds proven separately; L1 under 5 s; no focus taken | phase 4 (all of these), phase 5 L1 gates | met |
| 9 | C6, D3, D8 terminally classified; qualifying items have dated active specs; X2 design delivered; no item awaits a ruling | C6 measured, no change; D3 → `2026-10-27-shared-launch-trigger-discovery`; D8 → `2026-10-27-shell-site-placeholder-parse-once` (both exist); `completion-walk-design.md` marked "candidate design, not implemented"; spec `needs_rulings: false` | met |
| 10 | Public behavior unchanged except the reductions | existing integration suites and L2 (phase 4) green | met, with the recorded departures below |

Scope checks:

- No excluded optimization was built. `git diff d7ea96a1f --name-only` touches nothing in `claudine/cli/src/completion/` (X2), `claudine/lib/src/composition/schema/` (D3 trigger discovery), or Darkmatter's template code (D8).
- No X1 cache was duplicated: DMLS source is not in the diff.
- No formatting churn: `git diff -w` differs from `git diff` by 18 changed lines across 55 Rust files, all re-indentation inside rewritten blocks. No `cargo fmt` was run in any phase.
- No unrelated production change. The production edits outside the item surfaces are the phase-1 lint attribute (`render.rs`) and the `manifest_dir!()` path spellings, both recorded as decisions.

Departures from the snapshot spec and plan (the spec is left as written; each is in `decisions_for_review`):

- D2's summary carries only `has_schema`, not the plan's "presence and timing metadata".
- D13 lives in `simplified/convert.rs`, not `resolve.rs`.
- D1 aligns root-level union-arm origins with validation (a small behavior fix). Property-level arms keep the document folder for `match()`, as before.
- C3 added an early return for tasks with no `when:`.
- C7 counts state builds per task, not per key.
- Lifecycle-action and body `loop.*` reads stay eager (C4 covers conditions only).
- C1's CLI evidence is a `test-fixtures` seam, not counters.

**Implementation complete, ready for review.** Evidence limitations: performance figures come from one macOS host under moderate load (1-minute load about 6–12) in both passes; A3 has only instrumented elapsed times; Windows, Linux and WSL2 evidence comes from CI, not this phase.
