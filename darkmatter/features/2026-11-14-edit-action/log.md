---
feature: 2026-11-14-edit-action
implementation_1: "2026-11-18T08:29:38-08:00"
implementation_2: "2026-11-18T09:03:43-08:00"
---

# Edit action review log

## Implementation of Review Findings #1

> **started at:** 2026-11-18T08:29:38-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sequences/darkmatter/features/2026-11-14-edit-action/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs began

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Recorded planner tests exceed the L1 time budget | fixed | Defect class: planner contract tests did repository-wide work (the planner's test-input scan of ~5,700 files) that no assertion needed. All children verified; `just test test-toolkit` twice: 569 passed, none ≥5 s (load up to 26.8 on 16 cores) |
| R1-F1.1 | `the_shipped_planner_emits_plans_the_guards_reader_accepts`: never ≥10 s in package run; ≤5 s alone | fixed | Split into documentation/full-scope/changed-file tests plus guard `every_shipped_plan_case_has_a_test`; documentation case uses `prompts/_docs.md` (still `change_class: documentation`, now asserted) to avoid the scan. Package run: 0.202/0.286/0.243 s, guard 0.010 s; alone 0.158/0.215/0.179 s |
| R1-F1.2 | `the_shipped_planner_omits_deletions_and_the_reader_refuses_an_unexpected_absence`: same budget, keep deletion/missing-file assertions | fixed | Unchanged code; its planner call does no scan and every spawn belongs to the real plan; recorded 15.5 s came from contention with the sibling's scan, removed under R1-F1.1. Package run 0.262 s; alone 0.182 s; assertions intact |
| R1-F1.3 | `archive_path_guard::every_allowlist_entry_still_names_a_live_site`: <10 s loaded, ≤5 s alone | clean | Package run 0.798 s at load 26.8 |
| R1-F1.4 | `archive_path_guard::no_archive_executed_target_bakes_in_a_producer_path`: same | clean | Package run 0.797 s at load 26.8 |
| R1-F1.5 | `kache_init_contracts::ensure_is_idempotent_on_a_second_run`: same | clean | Package run 4.527 s at load 26.8 (under 5 s) |
| R1-F1.6 | `kache_host_contracts::report_resolves_the_worktree_base_by_the_wt_contract`: same | clean | Nine split cases 0.563–0.737 s in package run at load 26.8 |
| R1-F2 | Sequence edit test describes an obsolete template limitation | fixed | Defect class: test/doc text claiming sequence-root values are not templates. Sweep of claudine, darkmatter, and both repo skills found no other false statement |
| R1-F2.1 | `claudine/cli/tests/l1/lifecycle_edit_action.rs` sequence-root test uses computed `fqn`, obsolete comment removed, assertions kept | fixed | Test now declares `fqn: "{{ ctx.cwd }}/specs/spec.md"` used by `when`, `warn`, `edit`, `info`; all assertions kept; `just test lifecycle_edit_action` 77 passed (0.611 s); full claudine `just test` passed |

### Progress

- starting the work on 'R1-F1' at 08:30:59
- R1-F1 diagnosis: profiled every shipped-planner invocation the two tests made, alone at load ~16 on 16 cores
        - `-- README.md` took 1.91 s; nearly all of it is `test_input_references` → `test_inputs.scan` reading ~5,700 workspace files (`_declarations`, `needs_tokens`, `_scan_file`) to find tests that read the path
        - `--all` 0.17 s, `-- tools/test-toolkit/src/archive_guard.rs` 0.12 s, the deletion invocation 0.12 s; each spawns `cargo metadata --no-deps --offline` and `target/debug/ci-build key` three times, all needed by the real plan
        - `archive_guard_scope` reads only the change list, so the README scan contributed nothing to any guard assertion
        - the interpreter probe is one `python3 --version` spawn per test; nextest runs each test in its own process, so a `OnceLock` would share nothing and was not added
- R1-F1.1: split `the_shipped_planner_emits_plans_the_guards_reader_accepts` into three `#[test]`s stamped by `shipped_plan_cases!`, plus the no-spawn guard `every_shipped_plan_case_has_a_test` pinning the generated name list
        - documentation case now names `prompts/_docs.md`: the planner classifies it `change_class: documentation` (now asserted) and leaves the guard unselected with the same reason, but internal prompt folders are never test inputs, so the repository-wide scan is skipped
        - path literals stay inside each `#[test]` body; the shared `shipped_plan` helper only runs the command
- R1-F1.2: deletion test left unchanged; its planner run does no scan (0.12 s) and every spawn it makes is the real planner's own, so its recorded 15.5 s was contention from the sibling's scan and the package run
- sibling sweep: `scripts/ci-rollup-tests.rs::the_real_planners_plan_rolls_up` (`-- claudine/lib/src/lib.rs`, source path, no scan) and both `scripts/ci-build-archive-tests.rs` planner tests (`--all`, no scan) are clean; `scripts/ci-push` tests run stub or fixture planners, not the checkout scan
- verification at load 11.0–11.7 on 16 cores
        - `cargo nextest run -p test-toolkit --test ci_workflow_contracts -E 'test(/shipped_plan/)'` → 5 passed in 0.282 s
        - alone: documentation 0.158 s, full scope 0.215 s, changed file 0.179 s, deletion 0.182 s, guard 0.007 s
        - `just lint` (tools) → clean
- work completed for 'R1-F1' at 08:35:32
- starting the work on 'R1-F2' at 08:36:18
- R1-F2.1: `lifecycle_edit_action::sequence_root_initialize_runs_the_motivating_workflow_before_any_task` now declares the root property `fqn: "{{ ctx.cwd }}/specs/spec.md"` and reads it in `when: file_exists(fqn)`, the `warn`, `edit.file`, and `info`, as the spec's motivating example does
        - removed the obsolete comment claiming a sequence document's own values are step data, not templates
        - every assertion kept: new-file branch (one launch, canonical target as last argument, saved bytes, success message, no `trace.txt`, zero provider launches) and existing-file branch (warning, still one launch, no trace)
- R1-F2 sibling sweep (claudine lib/cli/docs/use-skills, darkmatter docs/lib/cli/tests, `.claude/skills/{claudine,darkmatter}`) for "not templates", "step data", "own values", "spelled where", and `ctx.cwd +` / `file_exists(ctx.cwd` workarounds: no other false statement
        - remaining "step data" hits describe sequence step rows' undeclared keys (`seq.<key>`), and "data, not a template" hits describe stored literal tokens (`schemas.md`, `bound_operands.rs`) and proxy/retry `with:` overlays (`flow-control-reference.md`); all accurate
        - no caller-skill change, so `just publish-skills` is not needed
- verification
        - `just test lifecycle_edit_action` (claudine) → 77 passed; changed test 0.611 s
        - `just lint` (claudine) → exit 0; only the existing macOS linker `__eh_frame` warning
- work completed for 'R1-F2' at 08:43:14
- final orchestrator verification at 08:43–08:54 (after both repairs)
        - root `just test test-toolkit` (twice) → 569 passed, 4 skipped, `none at or above 5.0s` (load 9.8 and 26.8 on 16 cores)
        - `claudine` `just test` → 12651 passed, 15 skipped, exit 0; 3 tests recorded slow at load 44.7 on 16 cores (loaded-host rule applies)
                - `compose_initialize_staged_boot::initialize_shells_are_forbidden_even_with_yolo_across_entry_paths` 5.608 s in package, 1.628 s alone → cleared
                - `compose_initialize_staged_boot::the_reviewed_early_catch_shell_trigger_is_refused_on_every_entry_path` 7.125 s in package, 1.582 s alone → cleared
                - `level1_provider_overlay_home::every_activation_reason_leaves_the_launch_home_variables_unchanged` 8.097 s in package, 2.797 s alone → cleared
                - none of the three is touched by this cycle; all under 10 s loaded and ≤5 s alone
        - `just lint` in `claudine` and `tools` → exit 0
        - `just check-tier-coverage claudine` → 0 stranded
        - darkmatter source was not changed in this cycle (only feature records), so no darkmatter package gate was required
        - cross-OS check skipped: test-only changes with no path, process, or `cfg` code

### Completion Report

- **outcome:** complete
- **elapsed:** 2026-11-18T08:29:38-08:00 → 08:55 (about 25 minutes)
- **parent findings:** total 2 = fixed 2 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - instance rows: 2 fixed (R1-F1.1, R1-F1.2), 4 clean (R1-F1.3–R1-F1.6), 1 fixed (R1-F2.1)
- **verified repairs**
        - R1-F1: the multi-plan planner test is now three independently scheduled tests with a guard; its documentation-only case no longer triggers the planner's repository-wide test-input scan (≈2 s CPU alone, the contention source); deletion test unchanged and now 0.26 s in the package run
        - R1-F2: the Claudine sequence-root test exercises the motivating workflow's computed `fqn` property; obsolete comment removed
- **open findings:** none
- **decision recorded:** for R1-F1.1 the documentation case uses a path under the internal `prompts/` folder, which the planner classifies as documentation and never scans for test readers; alternatives rejected: a small temp workspace (the planner has no root option and needs `cargo metadata` and real CI policy), and speeding `test_inputs.scan` itself (production change that would remove ~0.9 s of ~3 s, not enough)
- **changed files**
        - `tools/test-toolkit/tests/ci_workflow_contracts.rs` (R1-F1)
        - `claudine/cli/tests/l1/lifecycle_edit_action.rs` (R1-F2)
        - `darkmatter/features/2026-11-14-edit-action/log.md`, `review-1.md` (records)
- **side effect:** the R1-F1 subagent rebuilt `target/debug/ci-build` (`--no-default-features --features build-tools`) while checking the planner's fallback; the planner uses that binary for build keys

## Implementation of Review Findings #2

> **started at:** 2026-11-18T09:03:43-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sequences/darkmatter/features/2026-11-14-edit-action/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs began

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R2-F1 | Whole-value edit operands bypass the approved scalar conversion | fixed | Defect class: scalar-to-text conversion applied to authored scalars but not to expression results. All nine children closed. Darkmatter `just test` 17268 passed, none ≥5 s; Claudine `just test` 12652 passed (3 slow tests cleared, see Completion Report); both `just lint` exit 0 |
| R2-F1.1 | Darkmatter short-form parser: convert constant numeric/Boolean expression results; keep property expressions deferred | fixed | `edit_file_problem` now refuses only a blank literal. `lifecycle_edit_schema_parity` ACCEPTED rows `{ edit: '{{ 5 }}' }` and `{ edit: '{{ true }}' }`; blank-constant REFUSED rows added; null/blank/list rows kept; 6/6 pass |
| R2-F1.2 | Darkmatter one-item positional-list parser: same conversion contract | fixed | Parity rows `['{{ 5 }}']`, `['{{ true }}']`, `['{{ fqn }}']` plus `a_number_or_boolean_property_target_passes_schema_validation_and_the_parser`, which stays deferred and accepted; pass |
| R2-F1.3 | Darkmatter long-form parser (`file`): same conversion contract | fixed | Parity rows `{ file: '{{ 5 }}' }`, `{ file: '{{ true }}' }` plus the property test above; pass |
| R2-F1.4 | Shared executor via `md compose`: numeric/Boolean properties in all three spellings bind as text, resolve, and launch | fixed | `dispatch_edit` binds the evaluated value through `edit_file_value` before the nonblank check. Six executor rows (`scalar_result_opens_its_text!`: number, Boolean, fractional, one-item list, long form, constant) pass. `a_null_collection_or_blank_value_is_an_unsuppressible_evaluation_failure` passes and shows no launch. CLI `compose_edit_action::a_numeric_property_target_opens_the_file_its_digits_spell`: `schema validate` 0, `compose` 0, one launch at `5` |
| R2-F1.5 | Shared executor via `claudine compose --goose`: same | fixed | `lifecycle_edit_action::compose_opens_the_file_a_numeric_property_spells`: one launch on `5`, bytes saved, no provider; 0.944 s; Claudine suite 78/78 |
| R2-F1.6 | `BoundOperands` target projection via `md schema validate`: accept convertible constants; keep deferring event-time values | fixed | `edit_target_problem` → `Option<&str>`, so `BoundOperands` refuses only blank (`mistyped: false`). The parity rows run through schema validation, and the CLI test's `md schema validate` exits 0 |
| R2-F1.7 | DMLS ordinary Darkmatter document: stays clean and passive | clean | DMLS projects the same `BoundOperands` rule, which the review observed already accepting these constants with no launch or filesystem change; the rule now accepts more, never less. `just test editor` 334/334 (includes `edit_action_editor` passivity tests) |
| R2-F1.8 | DMLS registered Claudine document: stays clean and passive | clean | Same rule and fixture as R2-F1.7 under the Claudine host profile; `just test editor` 334/334 |
| R2-F1.9 | Lifecycle guide and error guide describe whole-value numeric/Boolean results as converted | fixed | `docs/topics/lifecycle/index.md` states that a number or Boolean template result names its text, and that null/list/mapping/blank fail with no launch. `error-handling.md` "never suppressed" bullet now uses a list/null result. Also swept: `use-darkmatter/authoring.md` source, Claudine `flow-control/lifecycle.md`, repo `darkmatter` skill `lifecycle.md`; `claudine/docs/topics/schemas.md` and `use-claudine` made no false claim |
| R2-F1.10 | Sibling: Claudine-side `edit` operand type decision | clean | Claudine has none of its own; it uses Darkmatter's parser, executor, and `BoundOperands` (grep of `claudine/` for the strict-string messages finds nothing) |
| R2-F1.11 | Sibling: `prep_target` scalar refusal | clean | A separate action that the `edit` ruling does not cover. Authored `prep: 5` and `"{{ 5 }}"` both become `Expr::NumberLiteral` and are refused the same way, so authored and expression paths do not diverge |

### Progress

- starting the work on 'R2-F1: Whole-value edit operands bypass the approved scalar conversion' at 09:04:07
        - defect class: a scalar-to-text conversion approved for an action operand is applied to the authored value but not to the value an expression produces, so parser, schema projection, and runtime each keep their own strict-string refusal
        - dispatched to one foreground `rust-developer` subagent (single finding, serial)
        - subagent results (reported to the orchestrator, then checked against the diff):
                - parser: `edit_file_problem` now refuses only a blank literal; constant `{{ 5 }}`/`{{ true }}` are accepted in scalar, one-item list, and long-form `file` spellings
                - schema projection: `edit_target_problem` returns `Option<&str>`; its only remaining refusal (blank) is reported with `mistyped: false`
                - executor: `dispatch_edit` passes the evaluated value through the existing `edit_file_value` (`bind(&Shape::text(), …)`) before the nonblank check and path resolution; null, list, mapping, and blank are still refused with no launch
                - tests: six executor rows generated by the `scalar_result_opens_its_text!` macro; parity rows for constant number/Boolean in all three spellings, plus blank-constant refusals; a property-target parity test; one CLI wiring test each in `md compose` (`compose_edit_action::a_numeric_property_target_opens_the_file_its_digits_spell`) and Claudine (`lifecycle_edit_action::compose_opens_the_file_a_numeric_property_spells`)
                - docs: lifecycle guide and error guide corrected; `use-darkmatter/authoring.md` source, Claudine's `flow-control/lifecycle.md`, and the repo `darkmatter` skill's `lifecycle.md` updated (`just publish-skills` not run)
                - sibling `prep_target` checked: a separate action that the ruling does not cover, and it already treats authored and expression scalars the same (both are refused); left as is
                - no schema or preparation change, so `just shipped-schemas` was not needed; both freshness tests pass
        - orchestrator diff review: `RefusedTarget::mistyped` is still set by the other producers, so it is not dead code
- work completed for 'R2-F1: Whole-value edit operands bypass the approved scalar conversion' at 09:27:49

### Completion Report

- **outcome: complete.** Elapsed about 25 minutes (09:03:43 → 09:27)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - instance rows: 9 required rows (R2-F1.1–R2-F1.9; 7 fixed, 2 clean) plus 2 sibling rows found by the sweep (R2-F1.10–R2-F1.11, clean)
- verified repair: an `edit` target now passes through the shared scalar-to-text conversion whether it is written natively, as a constant expression, or produced by an expression at run time. Parser, `BoundOperands` schema projection, DMLS, and the runtime executor agree. Null, collection, mapping, and blank targets are still refused, and the editor does not launch for them
- open findings: none
- no decision needed the author's review, so `human_review` was not changed
- cross-OS: the change only converts a JSON scalar to text before the existing path resolution and adds no path or process code, so there is no notable cross-OS risk and `just cross-check` was not run; CI covers the other operating systems
- changed files:
        - `darkmatter/lib/src/lifecycle/action_shape.rs`, `darkmatter/lib/src/lifecycle/executor.rs`, `darkmatter/lib/src/markdown/schemas/bound_operands.rs` (R2-F1.1–R2-F1.6)
        - `darkmatter/lib/src/lifecycle/executor/tests.rs`, `darkmatter/lib/tests/l1/lifecycle_edit_schema_parity.rs`, `darkmatter/cli/tests/l1/compose_edit_action.rs`, `claudine/cli/tests/l1/lifecycle_edit_action.rs` (tests for R2-F1.1–R2-F1.6)
        - `darkmatter/docs/topics/lifecycle/index.md`, `darkmatter/docs/topics/lifecycle/error-handling.md`, `darkmatter/docs/use-skills/use-darkmatter/authoring.md`, `claudine/docs/topics/flow-control/lifecycle.md`, `.claude/skills/darkmatter/lifecycle.md` (R2-F1.9 and doc sweep; `just publish-skills` not run)
- final verification (orchestrator, after all changes; load average 27–63 on 16 cores):
        - darkmatter `just test`: 17268 passed, 10 skipped, none ≥5 s (all rows)
        - darkmatter `just lint`: exit 0
        - claudine `just test`: 12652 passed, 15 skipped, 3 slow tests in the package run, all unrelated to this change and each cleared under the loaded-host rule (under 10 s in the package run, ≤5 s alone at load about 27–31):
                - `compose_initialize_staged_boot::initialize_shells_are_forbidden_even_with_yolo_across_entry_paths`: 5.488 s in the package run, 1.267 s alone
                - `compose_initialize_staged_boot::the_reviewed_early_catch_shell_trigger_is_refused_on_every_entry_path`: 7.223 s in the package run, 1.340 s alone
                - `level1_provider_overlay_home::every_activation_reason_leaves_the_launch_home_variables_unchanged`: 8.280 s in the package run, 2.781 s alone
        - claudine `just lint`: exit 0
        - darkmatter and claudine `just test-l2 level2_edit_action`: 2/2 passed in each (0.93–1.12 s), no window focus
        - subagent focused runs: darkmatter `edit_action` 47/47, `lifecycle_edit_schema_parity` 6/6, `lifecycle::executor::tests` 59/59, `editor` 334/334, `lifecycle` 687/687, `shipped_artifact_is_fresh` 1/1; claudine `lifecycle_edit_action` 78/78, `shipped_artifact_is_fresh` 1/1, `lifecycle_lift_ready_guard` 11/11
