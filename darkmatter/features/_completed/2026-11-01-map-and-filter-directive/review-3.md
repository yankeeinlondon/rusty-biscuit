---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-28T15:25:48-07:00
spec: 2026-11-01-map-and-filter-directive/spec.md
implemented: false
description: 'A **feature** review of `2026-11-01-map-and-filter-directive/spec.md`'
feature: 2026-11-01-map-and-filter-directive/review-3.md
previous: 2026-11-01-map-and-filter-directive/review-2.md
---

# Review 3 — Map and Filter Directives

The feature is **production ready** under the specification's contract. The remaining shell-block command-location defect is verified fixed for both directives. The previous partial repair is now complete; no new sibling, regression, or other actionable finding was identified. No decision is needed before repair or release through this review cycle.

## Previous Finding Verification

Read both previous reviews, the original implementation record, and both repair sections of `log.md`, including their `### Finding Ledger` tables. Neither earlier review contains a `## Human rulings` section. Informational implementation decisions were not treated as author rulings. The settled pipeline and discovery decisions in the specification remain in force.

Every test cited below ran in this review. The command-site checks were also inspected for their assertions and independently reproduced through the built `md` CLI. All rows have nothing remaining; grouped rows identify every required child explicitly.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1 — Selection guards lose refinements across destructured and object bindings | R1-F1.1–R1-F1.7 | verified fixed | `ItemFrame::enter` aliases destructured fields to the original item identity plus member path; `Subject::local` and `LexicalTypes::shape_below` preserve that identity. Composition, passive analysis, and DMLS controls below pass. | Nothing |
| R1-F1.1 — Destructured filter body, object guard | Additional filter, while, until | verified fixed | All three `scope_schedule::destructured_filter_body_with_an_object_guard` rows render the members. | Nothing |
| R1-F1.2 — Object filter body, destructured guard | All three selection/stopping slots | verified fixed | All three `object_filter_body_with_a_destructured_guard` rows pass. | Nothing |
| R1-F1.3 — Destructured map result after object guard | All three slots; result identity isolation | verified fixed | `destructured_map_result_with_an_object_guard` rows pass; the mapped-result field and shadowed-inner-field negative controls still reject unsupported narrowing. | Nothing |
| R1-F1.4 — Differently named object parameters | All three slots | verified fixed | `object_filter_body_with_a_differently_named_object_guard` rows pass. | Nothing |
| R1-F1.5 — Both callbacks destructure | Map/filter, all three slots | verified fixed | Both `*_with_a_destructured_guard` groups pass. | Nothing |
| R1-F1.6 — Guarded file field in later callbacks | Sort, stopping, mapping reads | verified fixed | `a_file_field_checked_through_an_object_parameter_is_read_through_a_destructured_one` passes with uncertain calls denied. | Nothing |
| R1-F1.7 — Editor and passive call checker | Shared identity and shadowing isolation | verified fixed | Library `collection_analysis::a_guard_fact_crosses_destructured_and_object_spellings_in_the_shared_scopes` and DMLS `collection_editor::guard_facts_cross_destructured_and_object_spellings_in_editor_diagnostics` pass. | Nothing |
| R1-F2 — Runtime diagnostic projections drop authored locations or iteration identity | Original R1-F2.1–R1-F2.16 and every second-repair ledger row below | verified fixed | Public typed failures and shared diagnostic records preserve authored locations and original item paths. The previously incomplete R1-F2.14 now locates the command. | Nothing; this completes the partial repair reported in review 2 |
| R1-F2.1 — Visit reservation | Map and filter | verified fixed | Both `provenance::{map,filter}::item_visit_reservation` rows retain the authored opener and original item. | Nothing |
| R1-F2.2 — Authored-body copy | Map and filter | verified fixed | Both `authored_body_copy` rows retain the copied site's authored line and item. | Nothing |
| R1-F2.3 — Interpolation insertion budget | Map and filter | verified fixed | Both `interpolation_insertion` rows retain source, span, and path. | Nothing |
| R1-F2.4 — Ordinary shell output budget | Map and filter | verified fixed | Both `shell_output_insertion` rows locate the authored insertion site. | Nothing |
| R1-F2.5 — Whole shell-block output budget | Map and filter; second-repair clean control | verified fixed | Both `shell_block_output_insertion` rows still locate line 7, the block opener. Output charging uses the replacement edit's block range independently of the last command entered. | Nothing |
| R1-F2.6 — Child output budget | Map and filter | verified fixed | Both `child_output_insertion` rows retain source/path; the child-failure test also joins parent and child paths. | Nothing |
| R1-F2.7 — Cached child replay | Replay charge and applicable parent iteration | verified fixed | Both `cached_child_replay` rows and `budget::a_cached_child_is_charged_again_for_every_insertion_and_every_visit` pass. | Nothing |
| R1-F2.8 — Repeated interpolation failure | Map and filter | verified fixed | Both `repeated_body_interpolation` rows and CLI location parity pass. | Nothing |
| R1-F2.9 — Repeated page-block gate | Map and filter | verified fixed | Both `repeated_page_block_gate` rows locate the authored gate and original item; CLI gate parity passes. | Nothing |
| R1-F2.10 — Nested collection gate | Map and filter | verified fixed | Both `nested_collection_gate` rows retain the enclosing item's identity and authored gate. | Nothing |
| R1-F2.11 — Transclusion target expression | Map and filter | verified fixed | Both `repeated_transclusion_target` rows pass. | Nothing |
| R1-F2.12 — Transclusion condition | Map and filter | verified fixed | Both `repeated_transclusion_condition` rows pass. | Nothing |
| R1-F2.13 — Ordinary shell argument validation | First/later directive, both kinds; second-repair clean control | verified fixed | `shell_argument_validation` and `shell_later_argument_validation` assert directive lines 7/8, column 1, exact command-start byte, and `data[0]`. | Nothing |
| R1-F2.14 — First shell-block command validation | Null argument, both kinds | verified fixed | `shell_block_argument_validation` now asserts command line 8 rather than opener line 7, column 1, authored byte start, bounded span, and `data[0]`. Independent CLI reproductions agree. | Nothing |
| R1-F2.14a — Later shell-block command validation | Null argument after `echo first`, both kinds | verified fixed | `shell_block_later_command_validation` asserts command line 9 and exact start. Independent CLI reproductions agree. | Nothing |
| R1-F2.14b — CLI command-location wiring | Report-json for both kinds; validation launches nothing | verified fixed | `collection_blocks::report_locates_a_later_shell_block_command_in_each_kind` checks line 9, column, file, exact byte start, bounded span, typed path, and an empty native recorder log. | Nothing |
| R1-F2.15 — Callback failure control | Both kinds, nested paths, CLI | verified fixed | `callback_failure`, `nested_failures_report_every_enclosing_position`, and CLI report/terminal parity pass. | Nothing |
| R1-F2.16 — Shared library diagnostic projection | Typed records, serialization, CLI; second-repair clean control | verified fixed | `the_shared_record_carries_typed_frames_and_the_authored_span` and per-row record assertions pass. The stage supplies the location; the CLI serializes the shared record without parsing message text. | Nothing |
| R1-F2.X1 — Claudine wrapped-failure classification | Preserve underlying classification | verified fixed | Inspected Claudine's `compose_failed_code`, which matches `without_iteration()`. The command-location repair changes no downstream API. Claudine tests were not rerun. | Nothing |
| R1-F2.X2 — Claudine shell-failure routing | Preserve shell code and iteration context | verified fixed | Inspected wrapped-failure routing and classification; the existing wrapper retains context while its inner error determines the code. No new decision or routing change is introduced. | Nothing |
| R1-F2.X3 — First/later execution failures | Both kinds after all commands are prepared | verified fixed | Both `shell_block_earlier_command_execution` and `shell_block_later_command_execution` groups locate lines 8/9 respectively. The execution loop re-enters each prepared command's offset. | Nothing |
| R1-F2.X4 — Command offsets after changed line geometry | Multiline earlier argument, CRLF/multibyte text, blockquote prefix; both kinds | verified fixed | `shell_block_after_multiline_argument_validation`, `shell_block_crlf_command_validation`, and `shell_block_quoted_command_validation` check authored starts and columns. `authored_commands` projects current physical line starts back to authored command starts. | Nothing |
| R1-F3 — Static collection rejection discards earlier recovery warnings | R1-F3.1–R1-F3.7 | verified fixed | `analyze_collection_blocks_report` retains the recovered prefix and filters it before the first rejection; the call precheck consumes it. | Nothing |
| R1-F3.1 / R1-F3.2 — Recovery before static source rejection | Map recovery and filter recovery | verified fixed | Both `pipeline::recovery_prefix` source-rejection rows and the CLI recovery-prefix report test retain warning plus error. | Nothing |
| R1-F3.3 / R1-F3.4 — Recovery before runtime pattern failure | Map/filter clean controls | verified fixed | Both runtime-pattern rows retain the earlier warning and failure. | Nothing |
| R1-F3.5 — Recovery alone | Both kinds, exactly one warning | verified fixed | Both recovery-alone rows succeed and retain one warning. | Nothing |
| R1-F3.6 — Other static rejection branches | Pattern, sort-key, protected name after map/filter recovery | verified fixed | All six generated recovery-prefix cases pass. | Nothing |
| R1-F3.7 — Authored-order prefix filtering | Keep earlier warnings, omit later warnings, retain first rejection | verified fixed | `a_recovered_source_after_a_static_rejection_is_not_reported` and `an_unrecovered_source_is_the_failure_before_a_later_rejection` pass. | Nothing |
| R1-F4 — Expression reference retains obsolete lookup and file-guard behavior | R1-F4.1–R1-F4.3 | verified fixed | Inspected the expression reference, interpolation page, and skill: bare lookup prefers the nearest local; both file predicates establish true-side file suitability; the five-prefix table includes `local`. Shipped-document corpus tests pass. | Nothing |

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No open finding repeats an earlier defect class, so `recurrence` is false. R1-F2 was the carried, unchanged partial repair from review 2; it is now verified fixed. The extra execution and line-geometry controls in the repair ledger verify the same bounded command-provenance class. They do not leave a newly discovered sibling for another cycle.

## Spec Review and Verification

Reviewed the specification, implementation plan, repair inventory, shared collection parser and executor, passive scope analysis, binding identity, body-origin projection, shell-block template preparation and execution, diagnostic projection, test registration, and current directive documentation. Checked the specification's requirement families against the public composition, CLI, and editor tests. No additional finding with specification, realistic regression, or realistic defect authority was identified.

The important repair boundary is in the darkmatter library's [shell-block stage](../../lib/src/markdown/compose/shell_blocks/mod.rs). Template construction, argument preparation, and sequential execution enter the current command's offset. The [pipeline adapter](../../lib/src/markdown/compose/pipeline/phases.rs) projects that offset through retained body provenance. A failure of the block itself retains the opener; insertion of the combined output is charged separately at the block replacement range. No CLI message parsing or new diagnostic hierarchy was added.

Independent reproductions used the built `target/debug/md compose <temporary-document> --output report-json`, with temporary shell whitelist files. Each document declared `data: any[](required)`, supplied `[null]`, and placed `echo {{ i }}` inside a repeated shell block. Tested both a first command and a command after `echo first`, separately for map and filter:

| Directive | Failing command | Observed structured result |
| --- | --- | --- |
| Map | First | `dm.shell.failure`, line 8, column 1, exact authored command-start byte, `data[0]` |
| Map | Later | Same code/path, line 9 and the later command's exact byte start |
| Filter | First | Same code/path, line 8 and the first command's exact byte start |
| Filter | Later | Same code/path, line 9 and the later command's exact byte start |

All four exited with status 1. These checks independently reproduce the previous review's failing inputs and confirm their corrected public result. The native-recorder CLI test additionally proves argument validation executes neither command.

| User-facing requirement family | Verification present and run | Required level and assessment |
| --- | --- | --- |
| Grammar, both arrows, option validation, patterns, typed sources and absence | `collection_blocks::{grammar,types_values}`, expression/parser tests, CLI controls | L1; appropriate |
| Map transformation versus filter retention, local lookup, shadowing and guard identity | `scope_schedule`, `collection_analysis`, expression scope tests, DMLS `collection_editor` | L1; appropriate |
| Selection, stable ordering, stopping and evaluation timing | Controlled-effect tests in `collection_effects` and `collection_blocks::pipeline`, exact-output tests | L1; appropriate |
| Markdown boundaries, whitespace, inserted data, independent operations and frontmatter-only requests | Grammar/pipeline corpus, CRLF and container controls, operation-subset and effect tests | L1; appropriate |
| Dynamic child discovery, explicit child parameters and request scope | `collection_discovery`, reference/preflight and cache tests, normal CLI composition | L1; appropriate |
| Condition-blind shell authorization, typed arguments and cache controls | Public preflight tests, native argument-recorder CLI cases, shell-block regressions | L1; appropriate |
| Shared budgets, cached insertion charges and fatal nested exhaustion | `collection_blocks::budget`, nested fallback and byte-boundary controls | L1; appropriate |
| Authored diagnostic sites, original positions, nested paths and report agreement | `collection_blocks::provenance`, report tests, CLI parity, independent CLI reproductions | L1; appropriate; previously failing command-location instance is repaired |
| Passive editor diagnostics, completion and hover | DMLS collection tests and effect tripwire | L1; appropriate |
| Shipped examples and normal invocation | Passive document corpus, frozen executable map/filter fixtures, CLI collection tests | L1; appropriate |

There is no keyboard-input, terminal-emulator layout, or browser-interaction requirement needing L2/L3 evidence. The tests are compiled through the declared consolidated L1 targets: library `main.rs` includes `collection_blocks`, which includes `provenance`; CLI `main.rs` includes `collection_blocks`; DMLS `main.rs` includes `collection_editor`. Their names are selected by the live L1 recipe, and the tier audit found no stranded tests.

Verification executed for this review:

- Darkmatter `just test collection`: **488 passed**, 13,060 outside the filter skipped; nextest reported 2.607 seconds.
- Full Darkmatter `just test`: **13,538 passed**, 10 skipped; nextest reported 66.611 seconds. No test failure or slow-test report appeared.
- Darkmatter `just lint`: **passed**, including library, CLI, DMLS, Zed CLI, and Zed WASM checks.
- Root `just check-tier-coverage darkmatter`: **passed**, zero stranded tests.
- Four independent CLI command-location reproductions: **passed**.

Local transcripts are `/tmp/darkmatter-review3-collection.log`, `/tmp/darkmatter-review3-test.log`, and `/tmp/darkmatter-review3-lint.log`. The durable evidence is the named assertions, inspected implementation, observed results, and counts recorded here. Cross-OS checks were not rerun; they remain part of normal CI and are not a readiness condition for this review. No production source, tests, dependencies, or formatting were changed, and no commit was made.

## Documentation and Input Robustness

Read [map](../../docs/inline/map.md) and [filter](../../docs/inline/filter.md) in full. Both declare `kind: canonical-documentation`, have no stale Planned notice, and document the shipped grammar, source declarations, scope, options, limits, approval behavior, and failure policy. Map examples bind the transformed result; filter examples retain the original item. The passive documentation corpus and executable fixtures pass. The shell-block and report-json references now describe the failing command's authored location consistently with the implementation. No documentation repair remains from R1-F4.

The input robustness matrix does not apply: the feature extends a Markdown/expression text grammar and runtime processing, not a configuration or file-format loader. Existing frontmatter loading is not redefined. Positive limit types and CLI validation cover invalid limit values through their existing public controls. No new human-only verification or product decision is required.
