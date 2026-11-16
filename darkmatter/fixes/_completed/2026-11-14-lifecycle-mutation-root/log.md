---
kind: log
spec: 2026-11-14-lifecycle-mutation-root
implementation_1: "2026-11-15T16:34:52-08:00"
---

# Review-to-implement log

## Implementation of Review Findings #1

> **started at:** 2026-11-15T16:34:52-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sequences/darkmatter/fixes/2026-11-14-lifecycle-mutation-root/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs began
- host load at start: 65.61 / 54.31 / 56.37 (16 cores); timing work uses the loaded-host rule

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Referenced groups and internal task adapters retain the caller's authoring path | fixed | all child rows verified; see Progress |
| R1-F1.1 | Referenced group event stacks: relative `events.log` lands in `groups/` | fixed | lib `a_referenced_groups_events_gate_and_member_write_beside_the_group` (failed before); CLI `referenced_containers_write_beside_themselves_…` extended (failed before) |
| R1-F1.2 | Referenced group loop gate: relative target lands in `groups/` | fixed | same two tests; gate line lands in `groups/` |
| R1-F1.3 | Group selected from a catalog: relative event target lands beside the catalog | fixed | lib `a_catalog_selected_groups_events_write_beside_the_catalog` (failed before); CLI `cataloged` step |
| R1-F1.4 | Foreign referenced group: absolute in-root `initialize` write succeeds | fixed | CLI `a_foreign_referenced_group_writes_absolute_in_root_targets` (exit 1 before; now writes initialize/leg/finalize) |
| R1-F1.5 | Internal task adapter primary stack writes beside the task | fixed | lib `a_task_files_setup_primary_and_teardown_write_beside_the_task_file` (failed before) |
| R1-F1.6 | Referenced sequence events and gate (control) | clean | lib `a_referenced_sequences_events_gate_and_task_write_beside_the_sequence` passes before and after |
| R1-F1.7 | Referenced group member (control) | clean | member assertions in the group tests |
| R1-F1.8 | Inline group events and member (control) | clean | lib `an_inline_groups_events_and_member_write_beside_the_containing_sequence`; CLI inline assertions |
| R1-F1.9 | Inline sequence origin (control) | clean | existing CLI `inline-sequence` assertion still passes; preflight `origin_path` invariant |
| R1-F1.10 | Internal task adapter setup stack writes beside the task | fixed | same adapter stage test (setup) |
| R1-F1.11 | Internal task adapter teardown stack writes beside the task | fixed | same adapter stage test (teardown) |
| R1-F2 | Claudine runs file actions after launch repository discovery fails | fixed | all child rows verified; see Progress |
| R1-F2.1 | Staged initialization refuses before write on failed discovery | fixed | CLI `compose_with_initialize_stops_before_the_staged_boot`; single-site bypass probe fails with marker written |
| R1-F2.2 | Wrapper pipeline `start` refuses before write | fixed | CLI `compose_without_initialize_stops_before_start`; pipeline site routed through shared check (harness check also covers it) |
| R1-F2.3 | Inline composition refuses before write | fixed | CLI `inline_compose_stops_before_initialize_and_start`; staged-site bypass fails it |
| R1-F2.4 | Loop preparation refuses before write | fixed | CLI `a_looping_compose_stops_before_its_first_pass`; single-site bypass fails it |
| R1-F2.5 | Harness loop start/gate refuses before write | fixed | `HarnessLoopState::new` uses shared check when the guard has a lifecycle; catches compose start when pipeline check is bypassed |
| R1-F2.6 | Root sequence engine refuses before write | fixed | CLI `a_sequence_stops_before_initialize_and_its_task`; single-site bypass fails it |
| R1-F2.7 | Task engine refuses before write | fixed | `run_step_task` uses shared check → `StepOutcome::Failed`; only reachable after R1-F2.6 (sole caller `execute_sequence`) |
| R1-F2.8 | Step-guard engine validates authority before subsequent execution | fixed | `root_step_guard` uses shared check (`CompositionError`); same reachability as R1-F2.7 |
| R1-F2.9 | Darkmatter CLI host (control) | clean | no Darkmatter code changed; review reproduced exit 1 with launch-resolution diagnostic and no marker |
| R1-F2.x | Reconcile duplicate follow-up record `2026-11-15-launch-discovery-failure-reporting` | fixed | `2026-11-15-launch-discovery-failure-reporting` set `status: abandoned` with completion note; its whole scope was the lifecycle-run check |
| R1-F3 | Root sequence events and loop gates omit refusal diagnostics | fixed | all child rows verified; see Progress |
| R1-F3.1 | Root `initialize` refusal names target/root | fixed | CLI `lifecycle_refusal_reports` row: exit 1, `sequence failed:` names target and root |
| R1-F3.2 | Root `start` refusal reported | fixed | same suite, `start` row |
| R1-F3.3 | Root `success` refusal reported | fixed | same suite, `success` row |
| R1-F3.4 | Root `finalize` refusal reported | fixed | same suite, `finalize` row |
| R1-F3.5 | Root `failure` secondary refusal reported, child error not repeated | fixed | same suite: child refusal reported by step, root `failure` refusal as `sequence failed:`, each once (exact-equality dedupe) |
| R1-F3.6 | Document loop gate refusal reported; outcome policy preserved | fixed | CLI rows for `ensure_file` and fake-editor `edit`: exit 0, one warning, nothing written; lib `gate_action_errors` |
| R1-F3.7 | Root sequence loop gate refusal reported | fixed | same, root sequence loop gate |
| R1-F3.8 | Referenced sequence gate refusal reported | fixed | same, referenced sequence gate |
| R1-F3.9 | Referenced group gate refusal reported | fixed | same, referenced group gate |
| R1-F3.10 | Gate positive control (in-root write) | clean | in-root gate write tested on all four owners |
| R1-F3.11 | Gate `no_error` suppression control | clean | `no_error` prints nothing, tested on all four owners |
| R1-F3.12 | Gate evaluation-error control | clean | division by zero: exit 1, evaluation error, no `action failed` line, all four owners |
| R1-F3.13 | Referenced sequence event control | clean | nested `initialize` reported by its step; nested `failure` secondary now also shown |
| R1-F3.14 | Foreign referenced group event refusal (repeat after R1-F1) | clean | after R1-F1: foreign referenced group `initialize` reports target beside the group and the root |
| R1-F3.15 | Canceled file actions invariant | clean | cancellation refuses `edit` before dispatch as an evaluation error; root reporter skips cancelled runs; existing Darkmatter `cleanup_events_edit_on_a_healthy_run_and_refuse_after_a_cancellation` |
| R1-F4 | L1 tests exceed the timing limit under concurrent execution | fixed | all child rows verified in canonical package runs; see Final Verification |
| R1-F4.1 | `a_foreign_sequence_cannot_write_beside_itself_from_initialize_or_a_task` | fixed | split into `…_from_initialize` / `…_from_a_task`: 0.21 / 0.48 s filtered (load 15); full `claudine` `just test` 12239 pass, none ≥5 s at load ~90–98 |
| R1-F4.2 | `every_pass_and_gate_of_a_document_loop_writes_beside_the_document` | fixed | 0.81 s filtered (load 15); full `claudine` `just test` 12239 pass, none ≥5 s at load ~90–98 |
| R1-F4.3 | `referenced_containers_write_beside_themselves_and_inline_ones_beside_their_container` | fixed | 0.65 s filtered (load 15); full `claudine` `just test` 12239 pass, none ≥5 s at load ~90–98 |
| R1-F4.4 | `ctx_per_run::a_sequence_task_observes_an_earlier_tasks_staging` | fixed | module max 1.01 s filtered (load 15); full `claudine` `just test` 12239 pass, none ≥5 s at load ~90–98 |
| R1-F4.5 | `embedded_yaml_action_initialize_bundle::wrong_type_one_element` | fixed | 0.6 s CPU alone; `claudine_schemas` filter max 1.0 s; full `claudine` `just test` 12239 pass, none ≥5 s at load ~90–98 |
| R1-F4.6 | `embedded_yaml_action_initialize_bundle::wrong_type_every_element` | fixed | same as R1-F4.5 |
| R1-F4.7 | `git_graph::gathered_graph_lays_out_sparse_lanes_at_56` | fixed | `git_graph` filter max 1.35 s (load 36); full `worktree` `just test` 1506 pass, none ≥5 s at load ~106 |
| R1-F4.8 | `git_graph::gathered_graph_lays_out_sparse_lanes_at_120` | fixed | same as R1-F4.7 |
| R1-F4.9 | `git_graph::the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60` | fixed | same as R1-F4.7 |
| R1-F4.10 | `diagnostics_then_point_requests_reuse_one_snapshot` | fixed | 1.16 s (1.0 s CPU) after operand-walker memoization; full `darkmatter` `just test` 16785 pass, none ≥5 s |
| R1-F4.11 | Other eleven slow mutation cases (cleared by review) | clean | every `lifecycle_mutation` test ≤0.84 s filtered; full `claudine` `just test` 12239 pass, none ≥5 s at load ~90–98 |
| R1-F4.12 | Darkmatter executor/compose-lifecycle/passive cases (control) | clean | full `darkmatter` `just test` 16785 pass, none ≥5 s |
| R1-F4.13 | Optimized differential and base-schema families (cleared) | clean | darkmatter schema filter 2007 tests none slow; full `darkmatter` run none ≥5 s |
| R1-F5 | Containment diagnostics truncate the target or effective root | fixed | all child rows verified; see Progress |
| R1-F5.1 | `md compose` short effect control | clean | `compose_lifecycle::a_prompt_outside_the_launch_scope_cannot_write_beside_itself` passes with an exact root match |
| R1-F5.2 | `md compose` long effect JSON keeps full target/root | fixed | darkmatter CLI `compose_lifecycle_refusals::an_effect_refusal_names_the_whole_long_root_{on_the_terminal,in_json}` |
| R1-F5.3 | `md compose` frontmatter writer keeps full target/root | fixed | `a_frontmatter_writer_refusal_names_the_whole_long_root_in_json` |
| R1-F5.4 | `md compose` editor keeps full target/root | fixed | `an_edit_refusal_names_the_whole_long_target_and_root_{on_the_terminal,in_json}` (fake editor never launched) |
| R1-F5.5 | Claudine composition effect/frontmatter keeps full detail | fixed | claudine `lifecycle_refusal_reports::a_long_root_survives_in_a_composition_{effect,frontmatter_writer}_refusal` |
| R1-F5.6 | Claudine composition editor keeps full detail | fixed | `a_long_target_and_root_survive_in_a_composition_edit_refusal`; lib `a_long_edit_refusal_keeps_its_complete_text_beside_the_concise_err_msg` |
| R1-F5.7 | Claudine nested sequence report control | clean | nested sequence/group rows pass with the tightened `names_root` (full `(root:<root>)`) |
| R1-F5.8 | Other effect writers share the repaired projection | fixed | shared point `concise_and_full` via `resolve_effect_path` → `from_error_or_action_with`; every-writer refusal test passes; long-path rows cover effect, frontmatter, edit families |

### Progress

- starting the work on 'R1-F1 referenced groups and internal task adapters retain the caller's authoring path' at 16:35:29
        - defect class: a document handoff replaces part of the resolution context while retaining the caller's source identity, so actions use the wrong authoring base or reject a valid target
        - one derivation now pairs `source_path` with that document's file-resolution context (`TaskExecution::authoring_context` / `in_authoring_document`), keeping the launch magic scope; it covers events, loop gate, loop cap (`open_loop`), container proxy, group `variables`, and task setup/primary/teardown/late values
        - sibling sites fixed beyond the review's list: `group.rs` `open_loop` (ran on the caller's stack), `sequence.rs` `open_loop` and container proxy (caller's context with the sequence's path), `resolve_value` for group `variables` (task path paired with group context)
        - clean sibling sites: CLI `task_run.rs` step stacks and step guard, `iterate.rs`/`container.rs` proxy loops, root sequence `mod.rs`, `looping/engine.rs`, `looping/container.rs`
        - decision: a derivation failure is now a typed `CompositionError` (fails the step) rather than a silent fallback; with no invocation at all, the caller's context is paired with the authoring path, so `for_source(source_path)` still resolves beside the authoring document (alternative: invent a context; rejected as duplicate of the executor's own derivation)
        - tests: new lib `task/tests/authoring_documents.rs` (6 tests, ~0.1 s each; 3 failed before the fix), CLI `referenced_containers_…` extended, new CLI `a_foreign_referenced_group_writes_absolute_in_root_targets` (both failed before the fix)
        - docs: `claudine/docs/topics/flow-control/lifecycle.md` "Relative paths start at the authoring document" now names catalogs, `kind: task` files, events, and loop gate
        - verification: lib `composition::sequence::` 621 pass; `--features work-counters` `task::` 236 pass; CLI `group lifecycle_mutation` 309 pass; `lifecycle_mutation` 19 pass; `just lint` clean; tier coverage 0 stranded
        - observed under load: existing `lifecycle_traces::sequence_nested_group` 5.8 s in the package-filtered run, 2.37 s alone (folded into R1-F4)
- work completed for 'R1-F1' at 17:04:00
- starting the work on 'R1-F2 Claudine runs file actions after launch repository discovery fails' at 17:04:00
        - defect class: a failed observation is projected as repository absence, and every downstream boundary consumer accepts the fallback without checking the retained failure
        - shared check `claudine::invocation_context::lifecycle_mutation_root` raises `CompositionError::LaunchRepositoryUnreadable` (`io.read_failed`) on a failed launch discovery; all seven CLI effect-engine sites (staged boot, pipeline, document loop, harness loop, sequence root, task, step guard) use it; no production `launch_magic_scope()` consumer remains in `cli/src`
        - only the discovery failure trips it (`InvocationContext::launch_discovery_failure()`); a failed package-topology probe of a found repository does not, since the boundary is known
        - decision: any `compose`/`inline-compose`/`sequence` run is refused on failed discovery (matches `md compose`); `InvocationContext::capture` is unchanged so non-lifecycle commands are unaffected; a direct wrapper's memory-file harness with an empty lifecycle passes `None` and is exempt (it runs no actions)
        - tests: lib `invocation_context::tests::lifecycle_mutation_root_refuses_a_failed_launch_discovery_and_only_that`; five `macro_rules!` CLI rows in `lifecycle_mutation_root.rs` with a hand-written `UnreadableLaunch` fixture (no `git` spawns); all five fail with the check disabled
        - docs: Claudine lifecycle, side-effects, `use-claudine` lifecycle pages, and `.claude/skills/claudine/SKILL.md`
        - verification: `test-library invocation_context` 54 pass, `composition::error diagnostics` 268, `guard error catalog` 864; `test-cli lifecycle_mutation` 24 pass (longest 1.78 s), `harness_orch:: wrap::composition:: wrap::sequence:: compose::prep` 361, `guard` 148, `launch_trigger_schemas` 57; `just lint` clean; no SLOW markers
- work completed for 'R1-F2' at 17:28:38
- starting the work on 'R1-F3 root sequence events and loop gates omit refusal diagnostics' at 17:28:38
        - defect class: an executor records a refused action, but the host's outcome/report projection does not publish the stored diagnostic
        - shared reporter `claudine::composition::lifecycle::report` (`warn_tolerated_action_error`, `report_container_failure`; text chosen once in `actionable_text`) and a default `LifecycleEmitter::emit_error`
        - consumers routed through it: root sequence events (`RootRun::report_recorded`, also on the refused-recovery abort path), `run_loop_gate` (serves all four gate owners), document events other than `initialize`/`start` (harness, `pipeline.rs`/`preflight.rs` catch loops, break-then-finalize, document-loop catch loop); cancellation cleanup in `error_routing.rs` de-duplicated
        - found beyond the review's list: silent document `success`/`failure`/`finalize`/`blocked` refusals, and nested container `TaskOutcome.secondary_errors` that were never shown (now carried on `StepOutcome::Failed`)
        - removed the foreign-sequence `failure:` workaround; fixture asserts the report directly
        - decisions: `initialize`/`start` stay silent because their action error is the run's error; repair kept in Claudine rather than Darkmatter's `LifecycleRunGuard` (a guard-level report would change `md` and double-report containers)
        - tests: new `claudine/cli/tests/l1/lifecycle_refusal_reports.rs` (31 macro rows), lib `looping/engine/tests/gate_action_errors.rs` (4); with the reporter disabled 8 CLI tests and the library test went red
        - verification: `test-cli lifecycle_refusal_reports lifecycle_mutation` 55 pass; `test-cli lifecycle loop sequence cancel harness_orch edit blocked preflight proxy` 1963 pass, none slow; `test-library looping lifecycle sequence` 1824 pass, none slow; `just lint` clean; tier coverage 0 stranded
        - an earlier narrower run under load ~257 showed 11 existing tests at 5.0–7.1 s; alone `lifecycle_traces::sequence_nested_group` 4.0 s, `sequence_prompt_property::…runs_each_step_inline` 3.6 s (carried to R1-F4)
- work completed for 'R1-F3' at 18:00:02
- starting the work on 'R1-F5 containment diagnostics truncate the target or effective root' at 18:00:02
        - defect class: a notification-sized summary replaces the complete actionable error when the failure is projected to terminal and machine output
        - lifecycle failures carry the full one-line message beside the 240-character `msg` (`LifecycleErrorInfo::full_msg: Option<Box<str>>` read through `actionable_msg()`; `DiagnosticSnapshot::full_message`, `#[serde(skip)]`); neither is serialized or visible as `err.*`
        - full text now used by: darkmatter `report_diagnostics::lifecycle_failure_diagnostics`, `md compose` status line, `run_guard` cancellation-cleanup warnings, `cancellation.rs` group-leg prefix; claudine `report::actionable_text`, pipeline/loop/proxy setup errors, `LifecycleInitializeFailed.reason`, `error_routing.rs`/`loop_control.rs`/`container.rs` warnings, step-failure message, `CompositionError::lifecycle_evaluation`, `RestoredDiagnostic` Display
        - kept compact: `err.msg`, `say`/`notify`/`message` templates, tracing fields
        - decision: no output-schema change (diagnostic `message` carries the full text); alternatives rejected: raising the limit (breaks speech/notification budget), parsing error text, a `DiagnosticCatalog` method; `Box<str>` keeps the error type under clippy `large-error-threshold`
        - restored full-path CLI assertions (darkmatter `compose_lifecycle`, claudine `names_root`)
        - verification: darkmatter `just test lifecycle report_diagnostics compose_lifecycle output_commands envelope compose_edit_action` 1155 pass, `guard` 277, `docs` 21, post-`Box` rerun 414; claudine `test-library lifecycle diagnostics sequence looping harness` 2052, `guard` 173; `test-cli guard` 148, `lifecycle_refusal_reports lifecycle_mutation lifecycle_edit_action` 135; `just lint` clean in both; no SLOW markers
- work completed for 'R1-F5' at 18:22:07
- starting the work on 'R1-F4 L1 tests exceed the timing limit under concurrent execution' at 18:22:07
        - defect class: tests pass their assertions but exceed the L1 execution budget when they compete for shared resources; isolated success was used to dismiss a failing concurrent run
        - root cause (profiled with the macOS Time Profiler): CPU-bound, unoptimized Simplified Schema code (~90% of each claudine process and schema-matrix cell) and the SVG stack (git-graph tests), stretched past 10 s by host load 50–130 on 16 cores; dmls operand walker recomputed node markers per item with linear site/action scans
        - fixes: root `Cargo.toml` `[profile.test.package.*] opt-level = 3` for `simplified-schema`, `unsafe-libyaml`, `usvg`, `simplecss`, `roxmltree`, `svgtypes`, `tiny-skia`, `tiny-skia-path`, `read-fonts`, `skrifa` (per-process CPU 2.4–2.9× lower; `opt-level = 1` measured insufficient); `simplified-schema/lib/src/json_shape.rs` caches compiled patterns per translation; `darkmatter/lib/src/markdown/schemas/bound_operands.rs` memoizes node markers and indexes sites/actions by path (dmls snapshot test 3.2 → 1.0 s CPU); claudine `common/mutation_paths.rs` writes `.git` directly (no `git init` spawns); R1-F4.1 split into `…_from_initialize` and `…_from_a_task` with no assertions dropped
        - decision: a test-profile optimization was chosen over per-test trimming because the cost was in every spawn's CPU, not any one fixture; trade-off is a slower simplified-schema rebuild (~7 → ~12 s) and a probable full-workspace CI scope for the `Cargo.toml` change; follows existing `[profile.test.package.*]` precedent in the root manifest, so kept in the ledger only
        - note: the `bound_operands.rs` indexing overlaps work planned by the open `2026-10-25-pipeline-improvements` spec ("Index operand sites and reuse immutable marker analysis")
        - filtered-run timings (load 15–68): every listed row 0.2–1.4 s; `lifecycle_traces::sequence_nested_group` 2.4 → 1.16 s; `sequence_prompt_property::…runs_each_step_inline` 3.6 → 0.85 s; twelve 5.0–7.1 s claudine-cli tests from run `1794793650` now 0.5–1.8 s
        - docs: `.claude/skills/rust-testing/test-speed.md` cause 9 (load-stretched CPU-bound spawns, `xctrace` recipe)
        - `just lint` clean in `simplified-schema`, `darkmatter`, `claudine`; tier coverage clean; full-area runs follow in Final Verification
- work completed for 'R1-F4' at 20:00:40

### Final Verification

All runs below were made after the last code change, with no subagent running.

| Command | Result | Verifies |
| --- | --- | --- |
| `cd darkmatter && just test` | 16785 passed, 10 skipped; `L1 run recorded: … none at or above 5.0s` | R1-F4.10, R1-F4.12, R1-F4.13, R1-F5, no regressions |
| `cd claudine && just test` | 12239 passed, 15 skipped; none at or above 5.0 s (load 89–98 on 16 cores) | R1-F1, R1-F2, R1-F3, R1-F4.1–R1-F4.6, R1-F4.11, R1-F5 |
| `cd worktree && just test` | 1506 passed, 13 skipped; none at or above 5.0 s (load ~106) | R1-F4.7–R1-F4.9 |
| `cd simplified-schema && just test` | 2332 + 1973 passed; none at or above 5.0 s | R1-F4 `json_shape` pattern cache |
| `just lint` in `darkmatter`, `claudine`, `simplified-schema`, `worktree` | all exit 0 | all findings |
| `just check-tier-coverage darkmatter` / `claudine` | exit 0, nothing stranded | new test files placed in declared binaries |
| `just publish-skills` in `claudine` and `darkmatter` | exit 0; `use-claudine`, `use-darkmatter`, `use-dmls` published | caller-skill drift |
| `just cross-check claudine --os windows` / `darkmatter --os windows` | **not run**: `build-win` storage preflight refused (44.5 / 41.6 GiB free, 50 GiB required) before any test | environment blocker, not a test failure; `os_failing` not set. Windows evidence falls to the nightly CI leg or a rerun once `build-win` has headroom (`just windows-sweep` / storage-strategy skill) |

### Completion Report

- **outcome: complete** — elapsed 3 h 34 min (16:34:52 → 20:09:11)
- parent findings: total 5 = fixed 5 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - R1-F1 fixed (11 rows: 7 fixed, 4 clean controls)
        - R1-F2 fixed (9 rows + follow-up reconciliation: 8 fixed, 1 clean control, record reconciled)
        - R1-F3 fixed (15 rows: 9 fixed, 6 clean controls)
        - R1-F4 fixed (13 rows: 10 fixed, 3 clean)
        - R1-F5 fixed (8 rows: 6 fixed, 2 clean controls)
- open finding/child IDs: none
- verified repairs (summary)
        - one authoring-document handoff pairs `source_path` with its own file-resolution context for referenced groups, catalog groups, referenced sequences, and `kind: task` adapter stages, keeping the launch boundary
        - one shared launch-discovery check (`invocation_context::lifecycle_mutation_root`) refuses every Claudine lifecycle host before file actions on a failed discovery; the duplicate follow-up record `2026-11-15-launch-discovery-failure-reporting` is `abandoned` with a completion note
        - one shared Claudine reporter publishes stored refusals for root sequence events, every loop gate owner, tolerated document events, and nested secondary failures, without changing exit policy, `no_error`, or cancellation
        - full containment text travels beside the 240-character `err.msg` and feeds terminal/JSON reports in both hosts; notifications and speech stay compact
        - CPU-bound test slowness addressed at its cause (test-profile optimization of schema and SVG crates, pattern cache, operand-walker memoization, spawn-free `.git` fixtures); every listed test is now ≤1.4 s filtered and none is slow in the full package runs
- residual notes (not open findings)
        - Windows cross-check could not run (rig storage); CI's nightly Windows leg remains the evidence source
        - the root `Cargo.toml` test-profile change likely widens CI scope for the next push
        - `bound_operands.rs` indexing overlaps work planned in `2026-10-25-pipeline-improvements`; that spec should treat the item as landed
- changed files: see `git status` at 20:09; new files `claudine/lib/src/composition/lifecycle/report.rs`, `claudine/cli/tests/l1/lifecycle_refusal_reports.rs`, `claudine/lib/src/composition/looping/engine/tests/gate_action_errors.rs`, `claudine/lib/src/composition/sequence/task/tests/authoring_documents.rs`, `darkmatter/cli/tests/l1/compose_lifecycle_refusals.rs`; modified production code in `claudine/lib`, `claudine/cli`, `darkmatter/lib`, `darkmatter/cli`, `simplified-schema/lib/src/json_shape.rs`, and root `Cargo.toml`; docs in `claudine/docs/`, `darkmatter/docs/`, `.claude/skills/{claudine,darkmatter,rust-testing}`
