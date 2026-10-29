---
spec: /Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/fixes/2026-10-26-sequence-quadratic-frontmatter/spec.md
plan: claudine/fixes/2026-10-26-sequence-quadratic-frontmatter/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - claudine/lib/src/work_counters.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/reapproval.rs
    - claudine/lib/src/composition/sequence/preflight/providers.rs
    - claudine/lib/benches/pipeline_work_alloc.rs
    - darkmatter/lib/src/work_counters.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs
    - darkmatter/dmls/tests/l1/registry_preparation.rs
    - darkmatter/dmls/tests/l1/startup_preparation.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - claudine
    - darkmatter
    - dmls
    - claudine-cli
    - darkmatter-cli
human_review: false
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/context/shared_frontmatter.rs
    - darkmatter/lib/src/markdown/compose/context/effective_state.rs
    - darkmatter/lib/src/markdown/compose/context/mod.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
    - darkmatter/lib/src/lifecycle/executor.rs
    - darkmatter/lib/src/lifecycle/executor/host.rs
    - darkmatter/lib/src/lifecycle/executor/shell.rs
    - darkmatter/lib/src/lifecycle/executor/tests.rs
    - darkmatter/lib/src/lifecycle/md_host.rs
    - darkmatter/lib/src/lifecycle/md_host/tests.rs
    - darkmatter/lib/tests/l1/lifecycle_event_binding.rs
    - darkmatter/cli/src/commands/compose/lifecycle.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/binding_contract.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/mutation_visibility.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/set_shell_values.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/templates.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/types.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/frontmatter.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/compose/context/shared_frontmatter.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/assignment.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/mod.rs
    - darkmatter/lib/src/lifecycle/preflight/shell_walk.rs
    - darkmatter/lib/src/lifecycle/executor.rs
    - darkmatter/lib/src/lifecycle/executor/tests.rs
    - darkmatter/lib/src/lifecycle/md_host.rs
    - darkmatter/lib/src/lifecycle/md_host/tests.rs
    - darkmatter/lib/tests/l1/lifecycle_event_binding.rs
    - darkmatter/lib/tests/l1/shell_result_values.rs
    - darkmatter/cli/src/commands/compose/lifecycle.rs
    - claudine/lib/Cargo.toml
    - claudine/lib/src/work_counters.rs
    - claudine/lib/src/pipeline_spike/tests.rs
    - claudine/lib/src/composition/runtime_state.rs
    - claudine/lib/src/composition/sequence/expr.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/formal.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/reapproval.rs
    - claudine/lib/src/composition/sequence/preflight/providers.rs
    - claudine/lib/src/composition/sequence/preflight/runtime_fixture.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/tests/scaling.rs
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/bindings/rename.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
    - claudine/lib/src/composition/lifecycle/tests/audio_emission.rs
    - claudine/lib/src/composition/lifecycle/tests/break_prep.rs
    - claudine/lib/src/composition/lifecycle/tests/container_runtime.rs
    - claudine/lib/src/composition/lifecycle/tests/guard_runtime.rs
    - claudine/lib/src/composition/looping/container.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/looping/expression.rs
    - claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/sequence/task/tests/leg_views.rs
    - claudine/lib/tests/l1/agent_errors_fleet.rs
    - claudine/cli/src/commands/wrap/overlay.rs
    - claudine/cli/src/commands/wrap/harness_orch/types.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/active_state_wiring.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/pipeline/tests.rs
    - claudine/cli/src/commands/wrap/composition/preflight.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/composition/runner.rs
    - claudine/cli/src/commands/wrap/composition/tests.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/justfile
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/frontmatter.md
    - .claude/skills/darkmatter/compose.md
source_files_during_phase_4:
    - claudine/lib/src/composition/sequence/task/container_regressions.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/sequence/task/tests/pipeline_spike.rs
    - claudine/lib/src/pipeline_spike.rs
    - claudine/cli/tests/l1/sequence_containers.rs
docs_updated_during_phase_4:
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/features/2026-10-25-pipeline-work-reduction/spike-report.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/architecture.md
source_code:
    - claudine/lib/src/work_counters.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/reapproval.rs
    - claudine/lib/src/composition/sequence/preflight/providers.rs
    - claudine/lib/benches/pipeline_work_alloc.rs
    - darkmatter/lib/src/work_counters.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs
    - darkmatter/dmls/tests/l1/registry_preparation.rs
    - darkmatter/dmls/tests/l1/startup_preparation.rs
    - darkmatter/lib/src/markdown/compose/context/shared_frontmatter.rs
    - darkmatter/lib/src/markdown/compose/context/effective_state.rs
    - darkmatter/lib/src/markdown/compose/context/mod.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
    - darkmatter/lib/src/lifecycle/executor.rs
    - darkmatter/lib/src/lifecycle/executor/host.rs
    - darkmatter/lib/src/lifecycle/executor/shell.rs
    - darkmatter/lib/src/lifecycle/executor/tests.rs
    - darkmatter/lib/src/lifecycle/md_host.rs
    - darkmatter/lib/src/lifecycle/md_host/tests.rs
    - darkmatter/lib/tests/l1/lifecycle_event_binding.rs
    - darkmatter/cli/src/commands/compose/lifecycle.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/binding_contract.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/mutation_visibility.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/set_shell_values.rs
    - claudine/lib/src/composition/looping/engine.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/templates.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/types.rs
    - claudine/cli/src/commands/wrap/sequence/container.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/assignment.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/mod.rs
    - darkmatter/lib/src/lifecycle/preflight/shell_walk.rs
    - darkmatter/lib/tests/l1/shell_result_values.rs
    - claudine/lib/Cargo.toml
    - claudine/lib/src/pipeline_spike/tests.rs
    - claudine/lib/src/composition/runtime_state.rs
    - claudine/lib/src/composition/sequence/expr.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/formal.rs
    - claudine/lib/src/composition/sequence/preflight/runtime_fixture.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/tests/scaling.rs
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/bindings/rename.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
    - claudine/lib/src/composition/lifecycle/tests/audio_emission.rs
    - claudine/lib/src/composition/lifecycle/tests/break_prep.rs
    - claudine/lib/src/composition/lifecycle/tests/container_runtime.rs
    - claudine/lib/src/composition/lifecycle/tests/guard_runtime.rs
    - claudine/lib/src/composition/looping/container.rs
    - claudine/lib/src/composition/looping/expression.rs
    - claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/tests/leg_views.rs
    - claudine/lib/tests/l1/agent_errors_fleet.rs
    - claudine/cli/src/commands/wrap/overlay.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/active_state_wiring.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/pipeline/tests.rs
    - claudine/cli/src/commands/wrap/composition/preflight.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/composition/runner.rs
    - claudine/cli/src/commands/wrap/composition/tests.rs
    - claudine/justfile
    - claudine/lib/src/composition/sequence/task/container_regressions.rs
    - claudine/lib/src/composition/sequence/task/tests/pipeline_spike.rs
    - claudine/lib/src/pipeline_spike.rs
    - claudine/cli/tests/l1/sequence_containers.rs
documentation:
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/features/2026-10-25-pipeline-work-reduction/spike-report.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/frontmatter.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/claudine/architecture.md
completed_phase: 4
implemented: true
pipeline_refreshed:
    - 2026-10-25-pipeline-work-reduction
decisions_for_author: []
---

# Implementation Log for 2026-10-26-sequence-quadratic-frontmatter (4 phases)

## Phase 1

Wave 1, attribution spike. Before any repair, this phase measures the sequence-size
multipliers in prompt and shell preflight, adds the counters that confirm
them, and maps each one to a repair. The only production-visible change is
counter instrumentation compiled under `work-counters`; `note_work!`
expands to nothing without that feature.

### Scenarios and commands

All runs are on this macOS host in release builds, one scenario at a time.

| What | Command (repository root unless noted) |
| ---- | --------------------------------------- |
| Prompt preflight (C7 at 10/100/1,000 tasks, 100 KB prompt frontmatter) | `cargo bench -p claudine --features work-counters --bench pipeline_work_alloc -- C7` |
| Shell preflight (A6 at 10/100 steps, ten holes each) | `cargo bench -p claudine --features work-counters --bench pipeline_work_alloc -- A6` |
| A4 sequence preflight (49 distinct prompt documents and one 200-leg group) | `cargo bench -p claudine --features work-counters --bench pipeline_work_alloc -- A4` |
| New `with`-hint and `set`-value rows (10/100 steps) | `cargo bench -p claudine --features work-counters --bench pipeline_work_alloc -- SQ` |
| Group, container, and A4 group-step rows (C1, C3, A4) | in `claudine/`: `just test-library --release --features work-counters --run-ignored only --no-capture pipeline_spike_c1 pipeline_spike_c3 pipeline_spike_a4` |
| CPU samples | `sample <pid> 15 -f <file>` on the bench binary, then `rustfilt -i <file> -o <file>.dm` |

**Measurement boundary.** The bench's `preflight` closure covers source
resolution, `resolve_sequence_plan`, and `build_preflight_graph`. Context
capture and fixture writes are setup and lie outside the boundary. Rows
report the warm (last) run.

**Profiling repeat.** One C7 or A6 invocation runs for under 2 s, which is too
short for a 15 s sample. For the sampling run only, I wrapped the bench
`main` in a `TMP_SPIKE_REPEAT` loop: 12 repetitions for C7 and 100 for A6,
same inputs and sizes. The loop has since been removed and is not part of
the change. Sampling the wrapper shell's pid by mistake gave an empty first
C7 sample; the C7 rows below come from the second sample, of the binary's
own pid.

### Baseline (reproduces the spec)

| Row | 10 | 100 | 1,000 |
| --- | -: | --: | ----: |
| C7 prompt preflight: bytes | 5.83 MB | 63.04 MB | 2,330.0 MB |
| C7: bytes per task | 583 KB | 630 KB | 2,330 KB |
| C7: allocations | 17,353 | 371,831 | 27,095,864 |
| A6 shell preflight: `dm.ExpressionParse` | 1,900 | 109,000 | — |
| A6: bytes | 3.97 MB | 92.12 MB | — |
| C1: one leg's live-frontmatter copy (payload 0 KB) | 7.65 KB | 48.2 KB | 455.0 KB |
| C1: group-step peak (payload 0 KB, cap 4) | 3.82 MB | 27.75 MB | 669.5 MB |
| A4 preflight (50 steps, 200 legs) | bytes 115.9 MB, peak 22.5 MB | | |
| A4 group step (200 legs) | bytes 728.2 MB, peak 190.0 MB, `LiveFrontmatterCopy` 200 | | |

The spec's 63 MB / 2.33 GB prompt rows, its `10·S² + 90·S` shell count, and
its 455 KB per leg copy and 669 MB peak all reproduce exactly.

### CPU attribution

**C7, 1,000 tasks.** In the sample, `build_preflight_graph_with_context` is
2,257 samples, all of them under `Loader::walk_plan`:

| Callee of `walk_plan` | Samples | Share |
| --------------------- | ------: | ----: |
| `reapproval::early_binding_state`: caller clone 244; `EffectiveStateBuilder::build` 770 (its `RawTable` clone 219, `deep_merge` clone 217, value drops 329) | 1,014 | 45% |
| `Loader::step_frontmatter`'s `HashMap::from_iter` through `stored_text` → `decode_literal_tokens` (a deep copy of the whole document frontmatter) | 693 | 31% |
| `Loader::load_task` (`providers::predict` 69, `resolve_reference` 55, prompt frontmatter copy 32) | 174 | 8% |
| dropping the previous step's `EffectiveState` | 161 | 7% |
| dropping the previous step's `Arc<HashMap>` step frontmatter | 153 | 7% |
| `derived_deferred_keys` (strings scanned, none parsed) | 49 | 2% |

**A6, 100 steps.** `walk_plan` is 10,900 samples:

| Callee of `walk_plan` | Samples | Share |
| --------------------- | ------: | ----: |
| `derived_deferred_keys` over the whole step frontmatter | 6,372 | 58% |
| `Loader::load_task` → `prepare_site` (per-hole `compose_approval_value` 516, `owner_schema` clone 348, `canonical` 302, `DeclaredTypes::for_schema` 248, `project_types` 176, `parse_authored` 168) | 2,200 | 20% |
| `early_binding_state` | 1,098 | 10% |
| `step_frontmatter` decode | 654 | 6% |
| drops of the previous step's state and frontmatter | 352 | 3% |
| `reject_removed_sequence_reads` (once per preparation) | 90 | 1% |

`prepare_site`'s callees are per hole and per site, and they show no
sequence-size term in the counters (see below). Their constant factor, the
three parses per hole, is D8's and is out of scope.

### Counters added (`work-counters` only)

The new claudine kinds count **JSON nodes copied or scanned** (`value_nodes`)
rather than calls. A call count is the same at every sequence size; a node
count grows when the work visits other steps.

| Counter | Site |
| ------- | ---- |
| `PreflightStepFrontmatterNodes` | `walk_plan` root and per-step frontmatter builds; `load_sequence`'s referenced-document frontmatter; `prepare_nested_sequence` / `prepare_nested_group` (`state.data().clone()`) |
| `PreflightDeferredScanNodes` | the `derived_deferred_keys` scans at those same three walk sites |
| `EarlyBindingStateNodes` | `reapproval::early_binding_state`, for every caller |
| `PredictionWithStateNodes` | `providers::predict_key`'s `with` branch (`layers.step_frontmatter.clone()`) |
| `ShellValueStateNodes` | `Loader::record_resolved_bytes` for a `set` value (`(*self.step_frontmatter).clone()`) |
| `PromptReferenceCompare` | `record_prompt_reference`'s linear `prompt_references.contains` |
| `dm.DeferredRootScanParse` (Darkmatter) | each expression parsed by `derived_deferred_keys`'s `value_roots`; it is also counted as an `ExpressionParse` |

### Site / counter table (per step = total ÷ builds or tasks)

Walk sites build their frontmatter S + 1 times: once for the root, then once per step.

| Site | Scenario | 10 | 100 | 1,000 | Per step | Grows with S |
| ---- | -------- | -: | --: | ----: | -------- | ------------ |
| step frontmatter build (`PreflightStepFrontmatterNodes`) | C7 | 559 | 32,689 | 3,026,989 | 51 → 324 → 3,024 | **yes** |
| ″ | A6 | 790 | 34,810 | — | 72 → 345 | **yes** |
| `derived_deferred_keys` scan (`PreflightDeferredScanNodes`) | C7 / A6 | = build | = build | = build | same | **yes** |
| ″ parses (`dm.DeferredRootScanParse`) | A6 | 1,100 | 101,000 | — | 110 → 1,010 (= 10·S(S+1) total) | **yes** |
| other `ExpressionParse` (total − scan) | A6 | 800 | 8,000 | — | 80 → 80 | no (D8 constant) |
| early-binding state (`EarlyBindingStateNodes`) | C7 / A6 | = build | = build | = build | same | **yes** |
| `with` hint state copy (`PredictionWithStateNodes`) | SQ-with | 728 | 52,388 | — | 73 → 524 per task | **yes** |
| `set` value state copy (`ShellValueStateNodes`) | SQ-set | 928 | 72,388 | — | 93 → 724 per site | **yes** |
| prompt reference dedupe (`PromptReferenceCompare`) | C7 (one document, two distinct references) | 17 | 197 | 1,997 | 2 → 2 → 2 | no, bounded by distinct references |
| ″ | A4 (49 distinct documents) | 1,176 = 49·48/2 | | | ∝ distinct references | **yes**, with distinct prompt references |
| provider prediction state (`PredictionStateBuild`, C7's 231 KB) | C7 | 20 | 200 | 2,000 | 2 → 2 → 2 builds, prompt-document sized | no (C7 constant) |
| lifecycle parse (`dm.LifecycleConfigParse`) | SQ-set | 21 | 201 | — | 2 → 2 | no |
| nested preparation copy (`prepare_nested_*`) | C3 nested-sequence-of-3 | 65 nodes per run | | | size of the live state, which carries `sequence:` | **yes** (code reading; the fixture has a small document) |

### Allocation attribution (prompt bytes)

The C7 rows fit `bytes = c + a·tasks + b·nodes` exactly, with `nodes` =
`PreflightStepFrontmatterNodes`: c = 1.38 MB per preparation, a = 409 KB per
task, b = 634 B per copied node. A fourth size not used in the fit
(300 tasks: 278,089 nodes) predicts 300.5 MB; it measures 308.9 MB, within
2.7%.

- At 1,000 tasks the node term is 3,026,989 × 634 B = **1.92 GB of 2.33 GB
  (82%)**: each step's whole-frontmatter decode plus `early_binding_state`'s
  three copies and their drops. The CPU split above matches it.
- The per-task constant (409 MB at 1,000) includes C7's 231 KB of prediction
  state per task (`PredictionStateBuild`) and the per-task loading of the
  prompt document. Neither grows with S, so both stay with Track 5.
- After the repair, the model predicts 422.8 KB per task at 100 tasks and
  410.7 KB at 1,000: a ratio of 0.97, inside the ±10% criterion. The
  per-preparation intercept is kept, not subtracted.

No contributor is left unexplained: the shell `S²` term is entirely the
scan's parses, and the prompt `N²` bytes are the node term.

### Runtime container candidates (for Wave 3's "Isolate group views")

These sites were found by reading code; they copy live state that carries the
`sequence:` list. Group legs are confirmed by `LiveFrontmatterCopy` and C1
above. Wave 3 confirms the others with counters before it repairs them.

- `task/group.rs:471`: `Mutex::new(seed.clone())` for each declared leg
  (confirmed)
- `task/group.rs:74`, `task/group.rs:797`, `task/templates.rs:42`: copies of
  `stack.frontmatter` / `state.data()`
- `task/sequence.rs:37,108,111,157,192,300`: copies of the nested sequence's
  event, gate, and step frontmatter
- `preflight/mod.rs` `prepare_nested_sequence` / `prepare_nested_group`:
  `Arc::new(state.data().clone())`
- `preflight/reapproval.rs` `ShellApproval::verify`: copies
  `basis.frontmatter` whenever there are runtime writes, which happens at
  each re-checked task
- `ApprovalRecordOwned.frontmatter` holds one `Arc` per step, each to that
  step's own full copy, so retained memory grows as S² across a graph's
  approvals

### Source-to-repair map

| Site | Repair (phase) |
| ---- | -------------- |
| `Loader::step_frontmatter` per-step decode (`walk_plan`, `load_sequence`) | shared preparation seed (decoded once) plus a per-step overlay (Phase 2 view, Phase 3 shell owner) |
| `derived_deferred_keys` per step | derived keys computed once per preparation, then each step scans only its overlay against those keys (Phase 3 shell) |
| `early_binding_state` per step (caller clone, builder clone, `deep_merge`) | one base `EffectiveState` per preparation; each step's state shares that base under its overlay (Phase 2 Darkmatter view, Phase 3) |
| providers `with` hint `step_frontmatter.clone()` | classify against the shared view plus one entry (Phase 3 prompt) |
| `record_resolved_bytes` `set` value clone | `ResolvedShellValue::resolve` takes the shared view (Phase 3 shell) |
| `ApprovalRecordOwned.frontmatter` and `verify` | the record holds seed plus overlay; `verify` layers runtime writes over it instead of copying (Phase 3 shell) |
| `prompt_references.contains` | bucket references by `(path, position, in_group)` (Phase 3 prompt) |
| group legs and other runtime container copies | private deltas over the group-start seed (Phase 3 group) |
| `prepare_site` constants, C7 prediction state, three parses per hole | unchanged (constant per step; Track 5) |

### Design Decisions

1. **Node-weighted counters**
   - **decision:** Each new claudine counter adds the JSON node count of the
     map it copies or scans (`work_counters::value_nodes`). Darkmatter gets
     one call-count kind, `DeferredRootScanParse`, inside `value_roots`.
   - **alternatives considered:** one count per call, which is the same at
     every S and cannot show a multiplier; byte probes in production code,
     which would need allocator access the library does not have.
   - **why:** A per-step node count that stays flat from 10 to 100 steps is
     exactly the spec's criterion, so Phase 3 regressions can assert it
     without exact-byte assertions. `DeferredRootScanParse` separates the
     scan's parses from D8's constant three per hole within `ExpressionParse`.
   - **downsides:** A node count says nothing about string lengths. The
     allocation bench still has to prove the bytes (Wave 5).
2. **The shared state lives in Darkmatter's `EffectiveState`**
   - **decision:** Phase 2 gives `EffectiveState` an immutable, `Arc`-shared
     base map with an owned top-level overlay. A preparation builds one base
     state (one copy of the decoded document frontmatter plus caller setters),
     and each step derives its state as base plus overlay (`seq`), without a
     deep copy. Overlay entries shadow whole top-level values, as today's
     `HashMap::extend` does, and an overlay that writes `ctx` or `replace`
     goes through the same merge rules `build` applies. Lookups (`get`,
     `get_checked`, expression scope) read the overlay first. Consumers of
     `data()` move to a layered view; a whole map is materialized only at an
     existing serialization boundary.
   - **alternatives considered:** a Claudine-only fix that keeps building a
     full `EffectiveState` per step (three of the four copies happen inside
     `EffectiveStateBuilder::build`, so it cannot remove the multiplier);
     dropping `sequence` from the per-step map (rejected by plan decision 2,
     because authored data must stay readable); sharing values inside
     `serde_json::Value` (no `Arc` variant exists).
   - **why:** 82% of prompt bytes and 45% of prompt CPU come from
     `early_binding_state`'s copies. The skill's rule ("one request's context
     and resolution authorities") is kept, because the base state is built
     once from the same request context.
   - **downsides:** It changes a Darkmatter public type: `data()`'s about 15
     consumers across both crates need the layered view. Phase 2 must widen
     validation to Darkmatter.
3. **Preparation seed and derived keys in the Loader**
   - **decision:** `Loader` holds one preparation seed: the decoded authored
     frontmatter plus caller setters, its `derived_deferred_keys`, and the
     base state. It is rebuilt only when a preparation starts (`walk_plan`, a
     referenced `load_sequence` document, a nested preparation). Each step
     adds its overlay and computes `derived_deferred_keys(overlay, roots,
     &seed_derived)`. If an overlay key shadows a seed key, the seed's scan
     is redone for that step with the shadowed key replaced, so transitive
     reads stay exact. That path is rare, because `seq` is reserved.
   - **alternatives considered:** process-wide memoization (plan decision 3
     rejects it); hiding `sequence` from the scan, which would change derived
     keys for a document value that reads a step definition.
   - **why:** The A6 parse count then becomes `10·S` (one scan) plus `80·S`,
     the same per step at 10 and 100 steps.
   - **downsides:** A seed with a large inline definition is still scanned
     once per preparation, and the shadowing fallback is a second code path
     that needs its own test.
4. **Prompt-reference dedupe**
   - **decision:** Index recorded references by `(path, position, in_group)`
     and compare `with` maps only within that bucket. The order of
     `graph.prompt_references` is preserved.
   - **alternatives considered:** a `HashSet<PromptReference>`, which is
     impossible because `with` holds `serde_json::Map` and is not `Hash`;
     leaving it, since it costs CPU only.
   - **why:** It grows with the number of distinct prompt references: A4 does
     1,176 comparisons for 49 documents. That is sequence-size-dependent work,
     so it is in scope.
   - **downsides:** One more map in the graph builder.
5. **Profiling repeat is temporary**
   - **decision:** The `TMP_SPIKE_REPEAT` loop used for sampling was removed
     after the samples were taken. The bench keeps only the new `SQ` rows.
   - **alternatives considered:** a permanent repeat flag in the bench.
   - **why:** Nothing else needs it, so keeping it would add surface (Rule 2).
     The sampling method is recorded above.
   - **downsides:** Repeating the CPU sample means re-adding the loop by hand.

### Notes for later phases

- The ignored `pipeline_spike_c1_group_leg_state` takes 6.2 s in release, over
  the five-second budget. Wave 5 must split it into bounded cases or move it
  into the bench, as the plan requires.
- C3's nested-sequence row now reports `PreflightStepFrontmatterNodes` (65)
  from `prepare_nested_sequence`, which makes it a ready regression probe
  for the nested-preparation repair.

### Gates and verification

- `just lint` in `darkmatter/` (run twice: before and after the DMLS test
  fixes) and in `claudine/`: pass.
- `cargo clippy -p claudine -p darkmatter --features claudine/work-counters,darkmatter/work-counters --lib --benches -- -D warnings`: clean, so the feature-on build compiles too.
- `just test` in `darkmatter/`: 12,575 tests passed.
- `just test` in `claudine/`: 11,648 tests passed.
- Release bench rows (C7, A6, A4, SQ) and the ignored crate-private rows
  (C1, C3, A4 group step) ran with the feature on; the results are recorded
  above.
- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages` = biscuit-hash, claudine, darkmatter, darkmatter-cli, dmls,
  test-toolkit; `change_class` = package. The worktree also holds other
  fixes' uncommitted edits. Gates were run, not skipped.
- Downstream: the Darkmatter change is one `note_work!`, which compiles to
  nothing without `work-counters`. Only Claudine and DMLS enable that
  feature, and both were tested in full.
- Cross-OS: no paths, processes, or OS-specific code changed, so no
  cross-check was run.

**Load-sensitive DMLS tests, which failed before this phase.** In the first two
Darkmatter `just test` runs,
`registry_preparation::a_held_preparation_leaves_edits_cancellation_and_other_requests_running`
failed and passed alone. It still failed with this phase's Darkmatter edits
reverted, so this phase did not cause it. Cause: the test counted only
`request complete` lines, but a request whose elapsed time (which includes
the deliberately held preparation) crosses the 500 ms `SLOW_THRESHOLD` is
logged as `slow request` (WARN). The captured log showed exactly that for
request 502. After the fix,
`startup_preparation::a_request_overtakes_the_remaining_startup_probes_after_an_unchanged_configuration`
failed the same way, for request 900 behind a held startup probe. Both tests
now accept either completion event for their id. The assertion's intent, one
completion event that records the wait or the cancellation, is unchanged.

## Phase 2

Wave 2, shared foundation. This phase adds the shared frontmatter view and
moves Darkmatter's evaluation state and the lifecycle live cell onto it. The
preflight Loader's per-step construction, the provider and approval state
copies, and the counters that measure them are untouched: Phase 3 repairs
those against the interfaces below.

### What changed

- **`darkmatter::markdown::compose::SharedFrontmatter`** (new,
  `darkmatter/lib/src/markdown/compose/context/shared_frontmatter.rs`): an
  `Arc<Map<String, Value>>` base, an owned `Map` overlay of written keys, and
  a set of removed base keys. `clone()` copies only the overlay. API: `new`,
  `get`, `contains_key`, `insert`, `remove`, `get_mut` (copies one top-level
  value), `get_or_insert_with`, `len`, `is_empty`, `iter`, `keys`, `to_map`,
  `to_hash_map`, `shares_base_with`, `owned_len`, plus `From<Map>`,
  `From<HashMap>`, `FromIterator`, `Extend`, and layering-insensitive
  `PartialEq`.
- **`EffectiveState`** stores its data as a `SharedFrontmatter`;
  `data()` returns `&SharedFrontmatter`.
  `EffectiveStateBuilder::with_shared_frontmatter` is new. Without external
  state (or with an empty external object, which merges to the frontmatter
  itself under every strategy) `build` keeps the view's base and owns only
  `ctx` and the view's writes. With external state it materializes and merges
  exactly as before. `with_frontmatter(HashMap)` and `EffectiveState::new`
  keep their signatures.
- **Lifecycle live cell.** `StackInputs::live_frontmatter`,
  `MdStackContext::live_frontmatter`, and Claudine's
  `StackExecutionContext::live_frontmatter` are
  `Option<&Mutex<SharedFrontmatter>>` (`&Mutex<…>` for `md`). The executor's
  per-event `working` state, the `set` snapshot, `ShellValueRequest::state`,
  `LifecycleHost::commit_set`'s `prior_base`, and `StackExecutor::eval_expr` /
  `resolve_string_value` take the view. `build_state` builds each expression's
  state through `with_shared_frontmatter`, so an event no longer deep-copies
  the document per event or per expression. Claudine's `commit_set` copies
  only the written keys' prior values before calling
  `RuntimeState::set_batch`, whose `&Map` signature is unchanged.
- **Claudine runtime consumers.** `group.rs` (`run_parallel` live cells,
  the container's own live cell, `member_state_from`), `sequence.rs` (the
  referenced document's own live cell and each step's state), `templates.rs`
  (`prepare_primary_template`), and `looping/engine.rs` (the `initialize`
  live cell) build on the view. `live_frontmatter_snapshot()` returns a view
  that shares the cell's base, so `run_parallel`'s `Mutex::new(seed.clone())`
  per leg is no longer a deep copy.
- **Materialized boundaries kept**: `markdown_child_options` (a child's
  external state is an owned `Value`; it now filters `prologue`, `epilogue`,
  and `ctx` while copying rather than after); `prewarm_container` and the CLI
  container pre-warm (`to_map()` once per `initialize`/`start`); the
  completion instance in `loop_control.rs`; `md`'s effective-state readback;
  `prepare_nested_sequence` / `prepare_nested_group`
  (`Arc::new(state.data().to_hash_map())`, the same copy as before, which
  Phase 3 removes); `execute_shell_values_with`'s `HashMap` state.

### Stable interfaces and file ownership for Wave 3

| Wave 3 task | Owns | Interface to use |
| ----------- | ---- | ---------------- |
| Isolate group views | `sequence/task/group.rs`, `task/sequence.rs`, `task/templates.rs` | Live cells are already `Mutex<SharedFrontmatter>`; `seed.clone()` shares the base. Remaining work: make `LiveFrontmatterCopy` count actual deep copies (it now counts cheap view creations, see the note below), add a view counter, and repair the `sequence.rs` copies (`own_live.extend(event_frontmatter.clone())`, `base.clone()` per event/step/gate) and `group.rs:797`'s merges. |
| Linearize shell analysis | `preflight/mod.rs`, `preflight/reapproval.rs`, `preflight/runtime_fixture.rs` | Change `Loader::step_frontmatter` from `Arc<HashMap>` to `SharedFrontmatter` (seed built once per preparation, `seq` inserted per step), `ApprovalRecordOwned.frontmatter` to `SharedFrontmatter` (so `verify` layers runtime writes with `insert`), and `early_binding_state` to take `&SharedFrontmatter` and build with `with_shared_frontmatter`. `prepare_nested_*` can then take `state.data().clone()` without copying. |
| Linearize prompt preflight | `preflight/providers.rs` | `PredictionLayers::step_frontmatter: &SharedFrontmatter`; the `with` branch clones the view and inserts its entries. |

Darkmatter files are done; a Wave 3 task that needs a Darkmatter change
coordinates through one owner.

### Design Decisions

1. **One view type, in Darkmatter, keyed on top-level keys**
   - **decision:** `SharedFrontmatter` holds an `Arc` base, an owned
     overlay `Map`, and a removed-key set. A write replaces a whole top-level
     value; `get_mut` copies only the value it mutates. Darkmatter's
     `EffectiveState`, the lifecycle live cell, and Claudine all use this one
     type.
   - **alternatives considered:** a Claudine-only view (the deep copies happen
     inside Darkmatter's builder and executor, so it cannot remove them);
     `Arc<Map>` with whole-map copy-on-write (the plan rules it out: the first
     write would copy `sequence:`); a persistent map crate (a new dependency
     the plan rules out); nested sharing below the top level (`serde_json::Value`
     has no shared variant, and today's overlays shadow whole top-level values
     anyway).
   - **why:** Every measured copy (early-binding state, step state, leg
     cells, per-event `working`) is a top-level map copy, and every overlay the
     code applies (`extend`, `insert`, `set_frontmatter`) replaces whole
     top-level values, so a top-level overlay preserves all existing semantics.
   - **downsides:** `append_frontmatter` on a large base array still copies
     that one array. Darkmatter's public `data()` and three executor
     signatures changed (no consumer outside Darkmatter and Claudine).
2. **Overlay iteration order**
   - **decision:** base keys in base order (a written value in place), then
     new keys in first-write order; a removed and rewritten base key keeps its
     base position.
   - **alternatives considered:** tracking IndexMap positions exactly,
     including `Map::remove`'s swap-removal order.
   - **why:** `EffectiveState`'s map was a `HashMap` (no order at all), and
     nothing reads the live cell's key order. A deterministic order is
     strictly better and needs no position bookkeeping.
   - **downsides:** after a remove and re-insert, `to_map()`'s key order can
     differ from what a plain `Map` would have produced.
3. **Builder fast path only without a merge**
   - **decision:** `build` shares the view's base when the external state is
     absent, not an object, or empty; otherwise it materializes and merges as
     before.
   - **alternatives considered:** an overlay-aware deep merge.
   - **why:** Every hot path (Claudine preflight, group members, sequence
     steps, lifecycle expressions) builds without external state. The test
     `a_view_builds_the_same_state_as_its_materialized_map` proves both paths
     agree for every strategy, with and without `replace_parent_wins`.
   - **downsides:** transclusion children (which carry their parent's state
     as external state) still copy once per child, as before.
4. **The preflight Loader stays on `Arc<HashMap>` in this phase**
   - **decision:** Phase 2 does not change `Loader::step_frontmatter`,
     `ApprovalRecordOwned`, `PredictionLayers`, or `early_binding_state`; it
     records their target interface above.
   - **alternatives considered:** retyping them now.
   - **why:** Retyping alone removes the `with`-hint, `set`-value, and
     `verify` copies while their counters
     (`PredictionWithStateNodes`, `ShellValueStateNodes`) keep counting nodes
     of a view that is no longer copied, which would corrupt Phase 3's
     before/after evidence. Those files are Phase 3's shell and prompt owners',
     who change construction, counters, and regressions together.
   - **downsides:** `prepare_nested_*` keep their `to_hash_map()` copy until
     Phase 3.

### Note on `LiveFrontmatterCopy`

`run_parallel` still records `LiveFrontmatterCopy` once per leg cell, but the
cells are now views that share the group-start base, not deep copies. The
counter's value is unchanged (one per leg), so existing tests and the C1 rows
still read it; Phase 3's group task owns renaming or splitting it, as the plan
requires.

### Tests (requirement → test)

| Requirement | Test |
| ----------- | ---- |
| shared seed reads, no owned values after clone | `shared_frontmatter::tests::a_clone_reads_the_seed_without_owning_any_value` |
| a small write owns one key; `sequence` stays shared and readable | `…::a_small_write_owns_only_that_key_and_keeps_the_sequence_shared` |
| two views never observe each other's writes | `…::sibling_views_never_observe_each_others_writes` |
| explicit `null` override is present, not a removal | `…::an_explicit_null_is_present_and_shadows_the_base_value` |
| whole-value shadowing of a nested value | `…::a_write_replaces_a_whole_nested_value_rather_than_merging` |
| removal, re-insert, order | `…::a_removal_hides_the_base_key_and_a_rewrite_restores_its_place`, `…::removing_a_key_only_the_overlay_held_forgets_it`, `…::iteration_keeps_base_order_then_appends_new_keys` |
| `get_mut` copies only the mutated value | `…::get_mut_copies_only_the_mutated_value`, `…::get_or_insert_with_writes_only_an_absent_key` |
| equality across layering | `…::equality_ignores_how_entries_are_layered` |
| `EffectiveState` over a view shares the base; nested, null, `ctx`, `fm.ctx`, `replace` lookups | `effective_state::shared_frontmatter_tests::a_state_built_from_a_view_shares_its_base_and_owns_only_ctx_and_writes` |
| fast path and merge path agree (5 strategy/external/`replace` cases) | `effective_state::shared_frontmatter_tests::a_view_builds_the_same_state_as_its_materialized_map` |
| executor: a `set` in a private live cell keeps the seed's 50-step `sequence` shared, is read by the next action, and is invisible to a sibling cell | `lifecycle::executor::tests::private_live_cells_over_one_seed_share_its_definitions_and_keep_writes_apart` |
| runtime parity (sibling isolation, declaration-order merge, retries, atomic groups, live-cell mutation visibility) | existing Claudine executor, group, and sequence suites, run unchanged apart from the cell's type |

All are L1 unit tests at F1 (in-process, no I/O beyond the executor
fixture's temp dir); the view crosses no process, file, or terminal boundary.
No loader, parser, or configuration grammar changed, so the Input Robustness
Matrix does not apply.

### Gates and verification

- Focused: `cargo nextest run -p darkmatter --lib -E 'test(shared_frontmatter) | test(private_live_cells) | test(effective_state) | …'`: 63 passed, then the four new view tests alone: 4 passed.
- `just lint` in `darkmatter/` and in `claudine/`: pass.
- `cargo clippy -p claudine -p darkmatter --features claudine/work-counters,darkmatter/work-counters --lib --benches -- -D warnings`: clean.
- `just test` in `darkmatter/`: 12,589 passed, 10 skipped (no failures, no retries).
- `just test` in `claudine/` (directly changed, and Darkmatter's main dependent): 11,648 passed, 10 skipped.
- Other Darkmatter dependents (research, messenger, playa, sniff, biscuit-icon, biscuit-speaks, claudine-gen): a search found no use of `EffectiveState`, `StackInputs`, `live_frontmatter`, `ShellValueRequest`, `commit_set`, or `markdown_child_options`, so no further tests were run.
- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `packages` = claudine, claudine-cli, darkmatter, darkmatter-cli, test-toolkit; `change_class` = package. Gates were run, not skipped.
- Cross-OS: no paths, processes, or `cfg`-specific code changed, so no cross-check was run.
- Every new test runs in under 50 ms.

## Phase 3

Wave 3, the three repairs. I did them serially rather than with parallel
subagents. All three edit the one `claudine` crate, and two of them change
interfaces the other two compile against (the step view type and
`StackExecutionContext::frontmatter`), so concurrent builds in one worktree
would have broken each other.

### What changed

**Shell analysis (`preflight/mod.rs`, `reapproval.rs`, `runtime_fixture.rs`)**

- `Loader::step_frontmatter` is a `SharedFrontmatter`. A walk decodes the
  root steps' common frontmatter once (`seed_steps`: the document's
  frontmatter, stored literal tokens decoded, under the caller's values) into
  `step_seed`, and scans it for late reads once (`seed_derived`). Each step
  (`enter_step`) clones the seed and inserts its `seq` overlay, so it owns one
  key.
- `overlay_derived` gives a step's late keys. It scans only the overlay
  (inheriting `seed_derived`) when the overlay shadows no seed key and none of
  its keys is late. Otherwise it scans the whole view, so a transitive read
  through an overlay key stays exact.
- `ApprovalRecordOwned` and `ApprovalBasis` keep the step view.
  `PreflightApproval::verify` layers the runtime writes over a clone of the
  view rather than copying the map. `early_binding_state` takes
  `&SharedFrontmatter` and builds through `with_shared_frontmatter`.
- `prepare_nested_sequence` and `prepare_nested_group` take
  `state.data().clone()`, which shares the base, instead of `to_hash_map()`.
- A shell `set` value (`record_resolved_bytes`) passes
  `ResolvedShellValue::resolve` a closure (a Darkmatter change, below), so a
  plain command never materializes the step state.
- `load_sequence` passes the step view to `load_referenced_sequence` as the
  invoking document's values. It previously copied the whole step state for
  each referenced sequence. This site was **found in this phase**.
- `build_group` no longer clones its owner document, and it scans that
  document's frontmatter for retired sequence names once per walk
  (`checked_group_owners`). Each group still checks its own events, through
  the new `bindings::reject_removed_lifecycle_reads`, which is the lifecycle
  half of `reject_removed_sequence_reads`. This site was **found in this
  phase**: every group re-parsed every expression of the document that holds
  it, 110 parses per step at 100 one-leg groups against 20 per step after the
  repair.

**Prompt preflight (`providers.rs`, `mod.rs`)**

- `HintLayers::step_frontmatter` is `&SharedFrontmatter`.
  `classify_reads` takes a lookup and classifies over `read_closure(key)`:
  the hint and every key it reads, directly or through other keys. It
  previously took a copy of the whole step state plus the hint and scanned
  every key of it once per candidate. That cost grew with the sequence for
  any `with` hint that holds an expression (**found in this phase**; the
  Phase 1 SQ row used a literal hint, which returns before the scan).
- `record_prompt_reference` dedupes within a
  `(path, position, in_group)` bucket (`prompt_reference_index`). The order of
  `graph.prompt_references` is unchanged. `TaskPosition` derives `Hash`.

**Group and container views (`task/group.rs`, `task/sequence.rs`, `task/mod.rs`, `looping/`)**

- `StackExecutionContext::frontmatter` (Claudine), `StackInputs::frontmatter`,
  and `MdStackContext::frontmatter` (Darkmatter) are now `&SharedFrontmatter`.
  Before, every nested-sequence child step, container event, loop gate, and
  task guard materialized the whole state (the outer `sequence:` included)
  into a `Map` just to borrow it. Each of them now passes a view:
  `sequence.rs`'s per-child `state.data()...collect()`, the guard's copy in
  `task/mod.rs`, the executor's `SharedFrontmatter::from(inputs.frontmatter.clone())`
  in `evaluate_guard` and in the no-live-cell `working` state, and
  `group.rs`'s fallback container cell.
- A referenced sequence document's base (`sequence_frontmatter`) is decoded
  once into a view. Its children clone the view, where before each child
  copied the referenced document again. Its own live cell receives each
  event's state through `layer_onto`: the first event shares the view, and a
  later event writes only the values that changed.
- `prewarm_container` takes the view and materializes it only inside the
  pre-warm closure, which runs only when a phrase needs the event state.
  Before, every group copied the document twice to call it.
- `RuntimeState::set_batch` reads its prior values through
  `&dyn DocumentFrontmatter`, so a proxy, a retry, or a container `set` passes
  its view instead of a materialized map.
- `LoopExpressionLookup` and `SourceExpressionLookup` read the document
  through the new `sequence::expr::DocumentFrontmatter` trait, which
  `Map<String, Value>` and `SharedFrontmatter` implement. Their lookup rules
  are unchanged. A loop gate still copies its state once per pass, because
  `ActionStaging` stages actions onto an owned map. That copy is per pass,
  not per step.
- **Counters.** `LiveFrontmatterCopy` now counts the JSON nodes actually
  deep-copied into leg cells, which are the group-start view's owned values.
  The new `LiveFrontmatterView` counts one per leg cell. A leg that never ran
  therefore cannot pass as a leg that copied nothing.

**Darkmatter**

- `SharedFrontmatter::owned()` iterates the values a view owns, which is what
  a clone copies. The counters read it.
- `shell_expansion::template::value_read_roots`: the root keys one value's
  expressions read, which is exactly what `derived_deferred_keys` follows.
- `ResolvedShellValue::resolve` takes its state as
  `impl FnOnce() -> HashMap<String, Value>`, called only for a ternary's
  branches.
- `StackInputs::frontmatter` and `MdStackContext::frontmatter` are
  `&SharedFrontmatter`. `md`'s `ComposeHost` keeps one view and seeds its live
  cell from it, which also removes one copy per run.

**Prompt harness (Claudine CLI).** `MaterializedHarnessPrompt` gains a
`frontmatter_view`, taken from the fresh live cell (`view_of`), so building
each event's stack copies nothing. The CLI's one-shot root sites (`initialize`,
`blocked`, staged boot, the root sequence, `task_run`) build one view each.

**CI.** `claudine/lib/Cargo.toml` declares
`[package.metadata.ci.tests] features = ["work-counters"]`, so CI and
`just test` run the counted regressions instead of compiling them out.

### Sites deliberately left

| Site | Why |
| ---- | --- |
| `step_guard_frontmatter` (root step guard, CLI) | The root runner rereads the sequence document from disk before each step, by design (the next step reads the file again). The guard decodes that fresh read once, which is the same order of cost as the read itself; it is now moved into a view, not copied again. |
| Runtime `prompt_references.iter().find` (`task/mod.rs`) | Prompt launch after preflight is out of scope (spec). Each search is a path-and-position compare per distinct reference, beside a provider launch. |
| Loop-pass copies (`finish_container_pass`, `open_loop`, `ActionStaging`) | One copy per loop pass, which does not grow with the step count. |
| A ternary `set` value's state | It still materializes the step state once, because `FrontmatterSeedState` owns a `HashMap`. Plain commands, including the measured `SQ-set` shape, no longer do. |
| *(Note added in review 1, R1-F1)* | The ternary `set` row above is now repaired: `FrontmatterSeedState` holds a `SharedFrontmatter`, preflight passes the step view, and Darkmatter's `execute_set_shell_values` no longer copies the whole state for plain or conditional values. |
| *(Note added in review 1, R1-F1.10–R1-F1.14)* | The loop-pass row above was wrong: each copy is the size of the container's whole state, which holds the `sequence:` list, and a sequence can declare N looping containers, so the total grew as N². It is now repaired: `open_loop` and `finish_container_pass` read the shared view (runtime writes in its overlay), `ActionStaging` stages into a view clone (`ActionTarget`), and `prewarm_container`, the CLI root pre-warm, and the preflight pre-warm walk build `PrewarmInputs` over a view (`PrewarmInputs::shared`), so the "`to_map()` once per `initialize`/`start`" boundaries listed earlier no longer copy. The completion instance in `loop_control.rs` stays: it runs once per prompt launch, which the spec leaves out of scope, and the schema validation it feeds reads the whole instance anyway. |
| Referenced-document decode in `load_sequence` | One copy of the *referenced* document per reference, which is linear. |
| `classify_reads`'s document path (`raw_state`) | A prompt document's own state, constant in the sequence size (Track 5, C7). |
| *(Note added for R1-F1.16)* Referenced data documents in a nested preparation (`Loader::load_data_document`) | Each nested preparation's fresh `Loader` still loads (and clones from its per-walk cache) a `kind: task`/`group`/`group-catalog` file its own executable references. That is one parse of the *referenced* file per container, linear in the sibling count; it grows as N² only if N siblings reference one shared file whose size also grows with N (a catalog of N groups). Routing it through `NestedSources` would still clone the whole `Value` per preparation and would change the read-failure error, so it is left. |
| *(Note added for R1-F1.16)* `RuntimeState::snapshot` per child (`sequence_binding`, `mutation_snapshot`, `mutation_delta`, the step loop's `SequenceBinding::step`) | After the nested-source memo, the residual dominant caller at 1500 sibling containers (about a third of samples). Each child copies the enclosing container's `seq.outputs`, which grows by one result per completed sibling. These are runtime results, not definitions, so it is a different class from this fix; measured 0.19 / 0.35 / 0.72 / 1.76 s at 100 / 200 / 400 / 800 containers. |
| *(Note added for R1-F1.17)* | The R1-F1.16 row on referenced data documents and the "Referenced-document decode in `load_sequence`" row are now repaired. A `Loader` shares each parsed referenced document (`ReferencedDocument`, by `Arc`) instead of cloning it per reference; a catalog step borrows its one entry through a lazy name index; a referenced sequence file is no longer parsed a second time (`plan_referenced_sequence`); and nested preparations reuse the invocation's parse through `NestedSources` when the walk's own bytes are unchanged, with the retired-read check memoized per retired-name presence set. A missing file still fails in `resolve_reference`, and an unreadable one goes to `data::load_document` unmemoized, so the error text is unchanged. The whole-document copy for a whole-document sequence reference stays: it is that step's own definition. |

### Design Decisions

1. **Seed once, overlay per step, scan the overlay**
   - **decision:** A walk decodes and scans the root steps' common frontmatter
     once; each step's view is the seed plus its `seq` overlay, and its late
     keys come from scanning the overlay alone, with a whole-view rescan when
     the overlay shadows a seed key or adds a late key.
   - **alternatives considered:** cache `derived_deferred_keys` by content
     hash (still rescans or rehashes the seed per step); always scan the whole
     view (the `S²` term this fix removes); drop the fallback (wrong when a seed
     key reads a late overlay key).
   - **why:** It is Phase 1 decision 3 as written. The fallback is proved
     equal to a whole-view scan by
     `overlay_late_keys_match_a_whole_view_scan`, which includes the shadowing
     case and the case where a late overlay key makes a seed key late.
   - **downsides:** the root view and the step seed are two copies per walk:
     the root keeps the caller's `sequence` exclusion and raw tokens, which
     the steps do not.
2. **Classify a hint over what it reads**
   - **decision:** `classify_reads` builds the transitive read closure of the
     hint (`value_read_roots`, new in Darkmatter) and runs the unchanged
     classification over that sub-state.
   - **alternatives considered:** keep the whole-state copy and only share it
     (the per-candidate scans stay `O(keys × document)`); cache per-key
     classifications across tasks (the state differs per step).
   - **why:** a key's class depends only on the keys it reaches, so the
     closure gives the same `Circular`, `ShellSubstitution`, `LateValue`, and
     `Evaluable` results and the same dependency set. The existing provider
     tests and `a_with_hint_keeps_its_class_through_document_keys` cover each
     class through a document key.
   - **downsides:** one more Darkmatter public function.
3. **Retype the stack's base state to the view**
   - **decision:** `StackExecutionContext`, `StackInputs`, and
     `MdStackContext` borrow a `SharedFrontmatter`.
   - **alternatives considered:** keep `&Map` and patch individual hot sites
     (each nested child and container event would still materialize a `Map`
     to borrow); add a second optional view field (two sources of truth).
   - **why:** the spec requires that *running* step *k* never copies the other
     steps' definitions, and the `&Map` type forced exactly that copy at every
     nested-sequence child, container event, and guard. About 25 production
     construction sites and 25 test sites moved. Every one-shot site builds one
     view, and the per-event sites share one.
   - **downsides:** a Darkmatter public field type changed (consumers are
     Claudine and `md` only). Test helpers that return a context over a
     borrowed map leak one view per call (`leaked_view`, test-only; each test
     is its own process).
4. **One read trait for document lookups**
   - **decision:** `sequence::expr::DocumentFrontmatter` (top-level read plus
     whole-object materialization) is implemented for `Map` and
     `SharedFrontmatter`, and taken by `SourceExpressionLookup`,
     `LoopExpressionLookup`, `load_referenced_sequence`,
     `normalize_formal_plan`, and `RuntimeState::set_batch`.
   - **alternatives considered:** convert every caller to
     `SharedFrontmatter` (adds a copy where callers hold a `Map`); duplicate
     each function.
   - **why:** callers holding a `Map` coerce unchanged, and the walk and
     runtime pass their view.
   - **downsides:** a dynamic call per lookup, which is negligible beside an
     expression evaluation.
5. **A plain `set` command never materializes the state**
   - **decision:** `ResolvedShellValue::resolve` takes the state as a closure.
   - **alternatives considered:** move `FrontmatterSeedState` onto the view
     (it threads a `HashMap` through `transitively_shell_blocked_keys`
     and `fm` resolution for the whole compose pipeline, which is a much wider
     change); pass a borrowed view and materialize always.
   - **why:** only a ternary's branches read the state. The measured shape
     (`SQ-set`) is a plain command and now copies nothing
     (`a_plain_shell_set_value_never_materializes_the_step_state`).
   - **downsides:** a ternary `set` value in a step event still copies the step
     state once.
6. **The owner scan runs once per walk**
   - **decision:** `build_group` scans an owner document's frontmatter for
     retired names once per walk, and checks each group's own events every
     time.
   - **alternatives considered:** skip the owner scan for the invoking
     document because `walk_plan` checked it (that check reads a different
     map, with the caller's values merged in); cache the result by content.
   - **why:** the scan reads only the document and the walk's fixed caller
     values, so a second scan of the same owner cannot fail when the first
     passed. Error order is unchanged, because the first group of an owner
     still runs the full check.
   - **downsides:** one `HashSet<PathBuf>` on the loader.
7. **Counters say what was copied**
   - **decision:** `LiveFrontmatterCopy` counts deep-copied nodes per leg
     cell, and `LiveFrontmatterView` counts cells.
   - **alternatives considered:** rename the old counter only (the plan rules
     it out).
   - **why:** the plan requires that zero deep copies cannot read as zero legs
     run.
   - **downsides:** `LiveFrontmatterCopy`'s unit changed, so Phase 1's C1 rows
     (one per leg) are not comparable by value. Wave 5 re-measures.
8. **Counted regressions run in CI**
   - **decision:** declare `work-counters` as Claudine's CI test feature.
   - **alternatives considered:** leave the counted tests feature-gated and
     unscheduled (stranded); assert structure only (a structural test cannot
     show per-step work).
   - **why:** the plan specifies feature-gated counter tests, and the
     tier-placement rules require that a feature-gated test runs in a tier CI
     enables.
   - **downsides:** Claudine's L1 test binary uses the counting allocator and
     compiles the `note_work!` atomics in. Both cost a few percent at most.
     The counters stay compiled out of production builds.

### Tests (requirement → test)

All tests are L1 and in-process (F1, against temp-dir fixtures).
Darkmatter-side tests compare per-step work at different sizes. Each also
asserts the graph or outcome, so a run that skipped work cannot pass.

| Requirement | Test |
| ----------- | ---- |
| Shell `ExpressionParse` per step equal at 10 and 100 steps, ten holes (spec) | `preflight::scaling_tests::counted::shell_preflight_parses_the_same_expressions_per_step_at_10_and_100_steps` (also asserts the first command's exact bytes and ten placeholders per template) |
| Step copies, scans, and early-binding states linear in S | `…::counted::step_frontmatter_copies_scans_and_states_grow_linearly` (10/55/100 steps; equal increments) |
| `with` hint copies only what it reads; a literal copies nothing; predictions correct | `…::counted::a_with_hint_copies_only_what_it_reads_at_any_sequence_size` |
| Plain `set` value never materializes state; discovered commands unchanged | `…::counted::a_plain_shell_set_value_never_materializes_the_step_state` |
| Prompt-reference dedupe compares only possible duplicates and still dedupes | `…::counted::prompt_references_are_compared_only_with_possible_duplicates` |
| Groups parse the same per step at 10 and 100 (owner-scan multiplier) | `…::counted::groups_parse_the_same_expressions_per_step_at_10_and_100_steps`. Verified to fail (20 vs 110 per step) with the owner memo disabled. |
| Every root step's view shares the seed and owns only `seq`; `sequence` and document values readable | `…::every_root_step_state_shares_the_preparation_seed` |
| Referenced sequence operator reads enclosing values through the view | `…::a_referenced_sequence_operator_reads_the_enclosing_document_values` |
| Hint class preserved through document keys (late, shell, circular, static) | `…::a_with_hint_keeps_its_class_through_document_keys` |
| Overlay-only scan equals a whole-view scan, including the fallback cases | `…::overlay_late_keys_match_a_whole_view_scan` |
| Group legs: one view per leg, nodes copied per leg equal at 10 and 100 legs, every leg ran in declaration order | `task::tests::leg_views::leg_views_copy_the_same_per_leg_at_10_and_100_legs` |
| `value_read_roots` follows every expression and strips `fm.` | `shell_expansion::template::candidate::read_root_tests::read_roots_follow_every_expression_at_any_depth_and_strip_fm` (Darkmatter) |
| Sibling isolation, declaration-order merge, retries, atomic groups, approvals, reapproval, providers | existing suites, unchanged apart from the construction type of the stack's base state |

No loader, parser, or configuration grammar changed, so the Input Robustness
Matrix does not apply. The one configuration edit is the CI feature
declaration.

### Gates and verification

- Focused, during the work: `cargo nextest run -p claudine --lib --features work-counters -E 'test(/scaling_tests|leg_views|composition::sequence|composition::lifecycle|composition::looping|runtime_state/)'`: 1,485 passed. The new regressions alone (`just test-library --features work-counters scaling_tests leg_views`): 11 passed, each under 0.5 s.
- `just test` in `darkmatter/`: 12,590 passed, 10 skipped.
- `just lint` in `darkmatter/`: pass, after removing three `SharedFrontmatter::from(…clone())` conversions that the retype made redundant (`useless_conversion`).
- `just lint` in `claudine/`: pass, after three clippy fixes in my edits.
- `cargo clippy -p claudine -p darkmatter --features claudine/work-counters,darkmatter/work-counters --lib --tests --benches -- -D warnings`: clean.
- `just test` in `claudine/` (directly changed, and Darkmatter's dependent): first run, one failure from my change. `composition_seams::every_prepared_context_capture_belongs_to_the_invocation_owner` flagged the new test's direct `ComposeContext::capture_for_content`. I moved the test file under `preflight/tests/scaling.rs`, which the guard treats as a whole-file test module, as it does `preflight/tests.rs`. The guard then passed (19/19). Second run:
  `wrap_watchdog_startup_stall::{watchdog_startup_stall_terminates_without_a_wall_clock_timeout, watchdog_opencode_stall_before_first_step_is_bounded}`
  failed at 4.8 s with the host's load average at 73. Both passed alone (3.2 s) and touch none of this change, so I record them as **load-sensitive**. That run stopped at 9,461 of 11,652 on fail-fast, so I ran the whole suite once more without fail-fast: `just test --no-fail-fast`: 11,652 passed, 10 skipped.
- `claudine/justfile`'s `test` recipe lists its packages with explicit features and does not read the CI metadata, so the counted tests did not run in that suite. The recipe now passes `claudine --features work-counters` (local and CI branches), matching the declared CI feature. `just test-library --features work-counters` (the whole library with the counting allocator): 6,404 passed, 5 skipped.
- Other Darkmatter dependents: no package outside `darkmatter/` and `claudine/` names `StackInputs`, `MdStackContext`, `StackExecutionContext`, `ResolvedShellValue`, `set_batch`, or the two lookups, so no further tests were run.
- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `packages` = claudine, claudine-cli, darkmatter, darkmatter-cli, test-toolkit; `change_class` = package. Gates were run, not skipped. The worktree still holds Phase 2's uncommitted edits.
- Cross-OS: no paths, processes, or `cfg`-specific code changed. The only path code is the existing `canonical` helper reused for the owner memo key. No cross-check was run.
- Wave 3 checkpoint: every Phase 1 site in the source-to-repair map is repaired, except the rows Phase 1 assigned to Track 5. The three sites this phase found are repaired too. What remains is in "Sites deliberately left", each with its reason.

## Phase 4

Waves 4–6: behavioral verification, documentation, allocation evidence, and
final gates. I did it serially with no subagents, because the measurements must
run alone on the host and the test additions were small.

### Wave 4 — Behavioral verification

Five of the plan's six regression cases already had tests; I added the two
that were missing, or only partly covered.

| Plan case | Test | State |
| --------- | ---- | ----- |
| Sibling reads after another leg's private write | `parallel_groups::a_sibling_mutation_is_invisible_until_the_group_completes` (runtime `set`, gated, deterministic); `container_regressions::nested_group_legs_keep_memory_private_but_reread_shared_files_between_steps` | existing |
| Small live-frontmatter writes over a large inline definition | `container_regressions::small_leg_writes_stay_private_over_a_large_inline_definition` (**new**, L1, F1 in-process): 40 legs, `max_parallel: 4`, 80 KB of group variables in the inline definition; every leg `set_frontmatter`s the same document key in `setup`, then reads it back in `teardown`; asserts each leg read its own value and the authored `title`, the write reached the file, and the file's `sequence:` is unchanged | added |
| Disk write followed by the next step's reread | `sequence_containers::a_leg_write_reaches_disk_for_the_next_step_but_not_its_sibling` (**new**, L1, F3 through the `claudine sequence` binary, `CliProcessFixture`): a leg writes `sequence.md`; the sibling after it prints `initial`; the next root step prints `written` | added |
| Nested-step file visibility | `container_regressions::nested_group_legs_keep_memory_private_but_reread_shared_files_between_steps` | existing |
| Failed group publication | `container_group_contract::group_finalize_error_discards_leg_mutations_and_result`; `nested_sequences::failure_stops_nested_sequence_and_publishes_nothing`; `container_regressions::unhandled_nested_failure_finalizes_innermost_first_and_never_starts_later_children` | existing |
| Preparation after a changed-file reread keeps frozen membership | `group_preparation_contract::{new_child_*_is_refused, new_child_command_is_blocked_before_start_and_publishes_nothing, retry_rereads_source_without_repeating_initialize, inline_group_rereads_variables_and_lifecycle_after_initial_audit, external_*}`; CLI `root_retry_keeps_approval_closed_when_a_referenced_task_changes` | existing |

The in-process test needed `Fixture::mutating_its_directory()` (test-only), which roots
the effect engine at the fixture directory so a `set_frontmatter` may target
the sequence document; the default engine root is a separate temp dir.

**The new in-process test is load-bearing.** I temporarily made every leg share cell 0
(`live_cells.get(0)` in `group.rs`), and it failed on 3 of 3 runs (legs 0,
10, and 20 read a sibling's write). Then I restored the file.

The existing suites ran unchanged: `just test-library --features work-counters
composition::sequence composition::lifecycle composition::looping
runtime_state` passed all 1,486 tests.

### Documentation

- `claudine/docs/topics/flow-control/sequences.md`: a new "What a leg sees"
  subsection under Groups covers private in-memory state versus the shared
  file, nested steps, a Mermaid diagram, and the CLI test's example with its
  output. It also explains that a leg's view is not a copy. A new
  static-preflight bullet states that the walk reads the document's
  definitions once and checks each step against that shared state plus its
  `seq` row. Neither change promises new behavior; both describe how the
  view and the walk are formed.
- `.claude/skills/claudine/architecture.md` § Sequences: the preflight seed
  and overlay (`step_seed`, `overlay_derived`, `classify_reads`), the
  leg-cell view, `&SharedFrontmatter` on the stack context, and the counters
  and tests that guard them.
- No README change: no public behavior changed.

### Wave 5 — Allocation evidence

`pipeline_spike.rs`: the C1 and A4 `one-live-frontmatter-copy` probes
(`fixture.frontmatter.clone()`, a whole `Map`) are replaced by
`one-live-frontmatter-view`, which times `Mutex::new(view.clone())` over the
group-start view, exactly as `group.rs` builds a cell. The module doc's
reproduction command is now the nextest recipe. The bench
(`pipeline_work_alloc.rs`) already measures through the production preflight
path and has no view probe, so it is unchanged.

Commands, run one at a time on this macOS host (load average 7–25 while
they ran; times are context only):

```sh
cargo bench -p claudine --features work-counters --bench pipeline_work_alloc -- C7   # also A6, A4, SQ
# from claudine/
just test-library --release --features work-counters --run-ignored only --no-capture pipeline_spike_c1   # also _a4, _c3
```

| Row | Before (Phase 1) | After |
| --- | ---------------: | ----: |
| C7 prompt preflight, 100 tasks: bytes | 63.04 MB (630 KB/task) | 43,933,686 B (439.3 KB/task) |
| C7 prompt preflight, 1,000 tasks: bytes | 2,330.0 MB (2,330 KB/task) | 425,932,685 B (425.9 KB/task) |
| C7 per-task ratio 1,000 : 100 | 3.70 | **0.9695**: `abs(0.9695 − 1) = 0.031 ≤ 0.10` ✔ |
| C7 peak, 100 / 1,000 | — | 1.33 MB / 5.72 MB |
| C7 `PreflightStepFrontmatterNodes` 10/100/1,000 | 559 / 32,689 / 3,026,989 | 280 / 2,890 / 28,990 (29 per step) |
| C7 `PromptReferenceCompare` 10/100/1,000 | 17 / 197 / 1,997 | 8 / 98 / 998 |
| A6 `dm.ExpressionParse`, 10 / 100 steps | 1,900 / 109,000 | **1,000 / 10,000**, 100 per step at both ✔ |
| A6 `dm.DeferredRootScanParse` | 1,100 / 101,000 | 200 / 2,000 |
| A6 bytes | 3.97 MB / 92.12 MB | 3.19 MB / 26.66 MB |
| A4 preflight (50 steps, 200 legs) | 115.9 MB, peak 22.5 MB | 91.6 MB, peak 22.4 MB |
| A4 group step (200 legs) | 728.2 MB, peak 190.0 MB | 624.8 MB, peak 150.0 MB |
| A4 one leg cell | (a 100 KB+ map copy) | 288 B, 2 allocations |
| SQ-with 10 / 100 steps: bytes | — | 1.60 MB / 14.01 MB |
| SQ-set 10 / 100 steps: bytes | — | 1.52 MB / 13.68 MB |
| C1 one leg cell, 10 / 100 / 1,000 legs, payload 0 or 100 KB | 7.65 KB / 48.2 KB / 455.0 KB | **288 B at every size and payload** ✔ |
| C1 group-step peak, payload 0 KB, 10 / 100 / 1,000 legs | 3.82 / 27.75 / 669.5 MB | 3.69 / 22.60 / 212.2 MB |
| C1 group-step bytes, payload 0 KB | — | 8.92 / 56.50 / 532.9 MB |
| C1 counters per run | `LiveFrontmatterCopy` = legs (one per copy) | `LiveFrontmatterView` = legs; `LiveFrontmatterCopy` = legs (one node each: the group-start view owns no values in this fixture) |
| C3 nested-sequence-of-3 `PreflightStepFrontmatterNodes` | 65 | 41 |

**The three scaling criteria pass:**

1. Shell `ExpressionParse` per step is equal at 10 and 100 steps (100 and 100).
2. Per-leg view construction is constant at 10 and 100 legs. The view is
   288 B whatever the leg count or payload, and `leg_views` (L1) asserts equal
   copied nodes per leg.
3. Prompt preflight bytes per task differ by 3.1% between 100 and 1,000
   tasks, measured on full-preflight rows with nothing subtracted.

**The remaining runtime-branch costs** are reported separately, as the plan
asks. Each declared leg still gets a `RuntimeState::from_snapshot` branch
before admission: 103 KB per leg in C1 (`one-runtime-branch`), and 608 KB in
A4, where the branch carries 49 committed outputs. That accounts for C1's
remaining 212 MB peak at 1,000 legs and A4's 150 MB. These are Track 5's C1
and C3. Both are linear in the leg count.

Every measurement test finished in under 1 s (C1 0.18 s, A4 0.10 s, C3 0.03 s),
so none needed splitting. I also added the before/after table and the
commands to `claudine/features/2026-10-25-pipeline-work-reduction/spike-report.md`,
in a new dated section that leaves the historical rows untouched.

### Design Decisions

1. **The C1 and A4 cell probes time the cell, not a stand-in**
   - **decision:** Probe `Mutex::new(view.clone())` over a
     `SharedFrontmatter::from(fixture.frontmatter.clone())` view, which is
     what `Fixture::execute_step` seeds as the live cell and what `group.rs`
     clones per leg.
   - **alternatives considered:** Keep the `Map` clone as a historical probe
     (it would measure code the executor no longer runs; the plan rules it
     out); probe through the whole group run only (it cannot isolate one
     cell from the runtime branch).
   - **why:** The spec asks for a bytes probe of per-leg view size, and this
     is the exact construction.
   - **downsides:** In production the group-start view may own values that
     earlier events wrote, and a cell copies those (counted by
     `LiveFrontmatterCopy`). This fixture's view owns none, so the probe
     shows the floor.
2. **Disk-write regression through the CLI**
   - **decision:** Test "a leg's write reaches disk and the next step rereads
     it" with one `CliProcessFixture` test, and test sibling privacy of live
     writes in-process.
   - **alternatives considered:** In-process only (the fixture runs one step
     and cannot reread between root steps); a CLI matrix (adds spawns and
     no new coverage).
   - **why:** The root runner's reread lives in the CLI's sequence loop,
     which only the binary exercises.
   - **downsides:** One more process spawn, about 1 s.
3. **Bench unchanged**
   - **decision:** Leave `pipeline_work_alloc.rs` as it is.
   - **alternatives considered:** Add a view probe to the bench (a bench
     target cannot reach the crate-private fixture).
   - **why:** Its rows already run the production preflight entry points.
   - **downsides:** none.

4. **Review 1: shell assignments retain the full performance requirement**
   - **decision:** Conditional and plain shell assignments must read shared
     frontmatter through resolution and execution without materializing the
     enclosing sequence. The remaining conditional preflight copy and the
     runtime shell-value adapter's whole-state copy are part of this repair.
   - **alternatives considered:** Keep only the plain-command preflight
     optimization (leaves requested scope unfinished); remove `sequence` from
     the state (changes authored-data lookup); copy an inferred subset of
     keys (adds dependency analysis where the existing shared view suffices).
   - **why:** The spec explicitly forbids copying other steps' definitions.
     Review probes through public APIs reproduced sequence-size-dependent
     copies for conditional preflight and for both runtime assignment forms.
     Phase 3's wider-change cost does not justify narrowing that contract.
   - **downsides:** The repair must adapt Darkmatter's shell-value seed-state
     boundary as well as Claudine's preflight caller; it must preserve branch
     interpolation, approval inventories, and failure timing.

### Pipeline Refresh

- `2026-10-25-pipeline-work-reduction` (spec and plan, not started):
  **changed.** The plan's C1 task now names the runtime branch as the
  heavyweight eager allocation, because leg cells are already views, and
  records the counter semantics. Its C7 task names only the prediction-state
  builds, because the `with` copy is gone. A new decision 11 records both.
  The spec gains the Design Decision "What
  `2026-10-26-sequence-quadratic-frontmatter` already delivered". No outcome
  was dropped or narrowed.
- `2026-10-26-dmls-frontmatter-typing-allocation`: upstream of this fix (both
  worktrees start from it), so not downstream; unchanged.

### Wave 6 — Gates and verification

- `just lint` in `claudine/`: pass. The only warning is the existing
  `__eh_frame section too large` linker message.
- `cargo clippy -p claudine --features claudine/work-counters --lib --tests --benches -- -D warnings`:
  clean. This covers the feature-gated `pipeline_spike.rs` and `leg_views.rs`.
- `just test` in `claudine/`: 11,661 passed, 15 skipped, no failures (547 s).
  The two new tests took 0.38 s (in-process) and 3.09 s (CLI, under load).
  126 existing tests went over 5 s in that run. The host's load average
  was 33–44 from work outside this session, and this phase touches none of
  those tests. Rerun alone, the slowest took 0.52–2.49 s
  (`every_consumer_of_the_launch_document_reads_its_captured_map`,
  `frozen_cli_implement{,_plan}`,
  `cancelled_conditional_depth_is_the_parsers_alone_to_limit`), so they are
  recorded as **load-sensitive**.
- `just test-l2` in `claudine/`: 289 + 3 passed, through the recipe's
  harness.
- `just check-tier-coverage claudine`: nothing stranded.
- `just lint` and `just test` in `darkmatter/` (the calculator lists
  `darkmatter`; this phase changed no Darkmatter code): lint pass; 12,590
  passed, 10 skipped.
- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages` = claudine, claudine-cli, test-toolkit, darkmatter;
  `change_class` = package. The gates were run, not skipped.
- Both builds compile: ordinary (`just lint`, `just test-l2`) and with
  `work-counters` (`just test`, which enables it for `claudine`; clippy;
  benches). No production path depends on counters or on macOS sampling
  tools: `note_work!` expands to nothing without the feature, and `sample`
  and `rustfilt` were Phase 1 tools only.
- Cross-OS: this phase adds tests and docs only. The new CLI test uses the
  side-effect grammar and spawns no shell, so I ran no cross-check.
- The spec snapshot is unchanged apart from its `implemented` and
  `implemented_by` frontmatter, and the fix stays in its active directory.
  Final state: implementation complete, ready for review.
