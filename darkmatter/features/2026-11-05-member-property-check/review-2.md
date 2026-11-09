---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-09T04:30:45-08:00
spec: 2026-11-05-member-property-check/spec.md
implemented: false
description: "A **feature** review of `2026-11-05-member-property-check/spec.md`"
feature: 2026-11-05-member-property-check/review-2.md
previous: 2026-11-05-member-property-check/review-1.md
---

# Review 2: Member Property Check

The feature is **production ready** under the specification's contract. All four previous findings are verified fixed, including every child instance in the repair ledger. No new findings or human decisions are required.

I read the specification, plan, original implementation record, previous review, and repair log before checking the implementation. The previous review contains no `Human rulings` section. Its informational review items are not pending prerequisites, and accepted decisions are preserved.

## Previous Finding Verification

The following table checks the repair log's entire `Finding Ledger`, including clean controls and the additional sibling sites found during repair. Test names below refer to the declared Level 1 modules in the darkmatter library, darkmatter-cli, and darkmatter-lsp packages. Every cited member test was selected by the passing `just test member` run; documentation rows were checked directly.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1 — DMLS omits shell ternary conditions and value branches | R1-F1.1–R1-F1.9 below | verified fixed | Shared shell-piece inventory, passive editor analysis, source projection, and agreement tests checked | Nothing |
| R1-F1.1 — Shell condition | Composition/editor each report one typo | verified fixed | `undeclared_member::library_and_editor_agree::shell_condition`; checks code, message, severity, and position with clean spelling control | Nothing |
| R1-F1.2 — Shell then value | Composition/editor each report one typo | verified fixed | Agreement test `shell_then` | Nothing |
| R1-F1.3 — Shell else value | Composition/editor each report one typo | verified fixed | Agreement test `shell_else` | Nothing |
| R1-F1.4 — Shell placeholder | One warning per placeholder, no duplicate | verified fixed | Agreement tests `shell_placeholder`, `shell_member_inside_piece_placeholder` | Nothing |
| R1-F1.5 — Frontmatter interpolation and expression-typed values | Preserve existing checks | verified fixed | Agreement test `frontmatter_interpolation`; editor `reports_on_every_surface` includes expression-typed values | Nothing |
| R1-F1.6 — Body, conditions, callbacks, region bodies | Preserve shared checking on equivalent sites | verified fixed | Existing agreement rows for body, block/file conditions, map/filter callbacks and region body; `file_condition_beside_placeholder` | Nothing |
| R1-F1.7 — Lifecycle exclusion | Lifecycle remains call-only | verified fixed | `undeclared_member::lifecycle_unchanged`; frontmatter checker excludes lifecycle stacks | Nothing |
| R1-F1.8 — Unselected shell branches | Editor checks both; runtime stays silent on unselected ordinary reads | verified fixed | `shell_ternary_pieces_in_the_editor` asserts both editor findings and no composition findings | Nothing |
| R1-F1.9 — Current guidance | Remove editor omission | verified fixed | Expression topic, DMLS diagnostic reference, and darkmatter compose/DMLS skills now describe condition and both value branches | Nothing |
| R1-F2 — Evaluation failures discard already-observed member warnings | R1-F2.1–R1-F2.20 below | verified fixed | Warning-carrying interpolation failures, report transfer before propagation, and shell-stage warning ownership inspected; all public failure reproductions pass | Nothing |
| R1-F2.1 — Body pass | Earlier evaluated site survives another interpolation failing | verified fixed | `compose_member_warnings::ac8_same_stage_body_later_site` | Nothing |
| R1-F2.2 — Body expression | Warning survives failure within one expression | verified fixed | `ac8_same_stage_body_expression` | Nothing |
| R1-F2.3 — Mixed frontmatter scalar | Preserve earlier interpolation warning | verified fixed | `ac8_same_stage_frontmatter_mixed_scalar` | Nothing |
| R1-F2.4 — Whole-value frontmatter | Preserve warning within failing whole-value expression | verified fixed | `ac8_same_stage_frontmatter_whole_value` | Nothing |
| R1-F2.5 — Page-block conditions | Preserve warning for outer and nested conditions | verified fixed | `ac8_same_stage_block_condition`, `ac8_same_stage_nested_block_condition`; shared condition evaluator records observations before propagating error | Nothing |
| R1-F2.6 — Collection gate | Preserve warning on failing region condition | verified fixed | `ac8_same_stage_collection_gate`; collection condition transfers observations before returning error | Nothing |
| R1-F2.7 — File condition | Preserve warning before failed data read | verified fixed | `ac8_same_stage_file_condition`; transclusion gate transfers observations first | Nothing |
| R1-F2.8 — Code condition | Same result for code transclusion | verified fixed | `ac8_same_stage_code_condition`; same transclusion gate | Nothing |
| R1-F2.9 — Shell condition | Preserve warning in failing ternary condition | verified fixed | `ac8_same_stage_shell_condition`; failing-piece handler drains reached observations | Nothing |
| R1-F2.10 — Shell then value | Preserve warning in failing then value | verified fixed | `ac8_same_stage_shell_then_value` | Nothing |
| R1-F2.11 — Shell else value | Preserve warning in failing else value | verified fixed | `ac8_same_stage_shell_else_value` | Nothing |
| R1-F2.12 — Map callback | Preserve existing warning on failure | verified fixed | `ac8_same_stage_map_callback` | Nothing |
| R1-F2.13 — Filter callback | Preserve existing warning on failure | verified fixed | `ac8_same_stage_filter_callback` | Nothing |
| R1-F2.14 — Filter option | Preserve existing warning on failure | verified fixed | `ac8_same_stage_filter_option` | Nothing |
| R1-F2.15 — Sort option | Preserve existing warning on failure | verified fixed | `ac8_same_stage_sort_option` | Nothing |
| R1-F2.16 — While option | Preserve existing warning on failure | verified fixed | `ac8_same_stage_while_option` | Nothing |
| R1-F2.17 — Until option | Preserve existing warning on failure | verified fixed | `ac8_same_stage_until_option` | Nothing |
| R1-F2.18 — Later pipeline stage | Preserve completed-stage warning | verified fixed | `ac8_later_failure` checks completed frontmatter and page-block stages before body failure | Nothing |
| R1-F2.19 — Later preparation/execution and sibling containers | Preserve shell warnings; preserve frontmatter key/item and directive-target warnings; retain denial | verified fixed | `ac8_same_stage_shell_later_preparation`, `ac8_same_stage_shell_later_execution`, `ac8_same_stage_frontmatter_later_key`, `ac8_same_stage_frontmatter_later_item`, `ac8_same_stage_directive_target`, `ac8_same_stage_denial_still_blocks_first` | Nothing |
| R1-F2.20 — Structured CLI failure | Warning and original error in JSON; empty stderr | verified fixed | CLI `a_failed_composition_keeps_the_earlier_member_warning` checks exit 1, exact warning location, original missing-file error, and empty stderr; `a_reference_check_failure_reports_no_member_warning` pins the earlier discovery boundary | Nothing |
| R1-F3 — Interpolation hides other member reads in a shell expression | R1-F3.1–R1-F3.7 below | verified fixed | Authored-expression matching preserves selector identity for in-string placeholders; editor and composition use the shared piece inventory | Nothing |
| R1-F3.1 — Placeholder beside condition read | Report ordinary typo at authored position | verified fixed | `a_placeholder_does_not_hide_a_shell_piece_member`; editor agreement `shell_condition_beside_placeholder` | Nothing |
| R1-F3.2 — Placeholder beside then-value read | Same result for then branch | verified fixed | Same library matrix; agreement `shell_then_beside_placeholder` | Nothing |
| R1-F3.3 — Placeholder beside else-value read | Same result for else branch | verified fixed | Same library matrix; agreement `shell_else_beside_placeholder` | Nothing |
| R1-F3.4 — Literal controls | Literal filler warns; corrected member stays clean | verified fixed | Library matrix crosses literal/placeholder fillers with typo/correct member spellings | Nothing |
| R1-F3.5 — Read inside placeholder | Preserve one independently located warning | verified fixed | Existing placeholder tests and `shell_member_inside_piece_placeholder` agreement | Nothing |
| R1-F3.6 — Unselected value branch | No runtime warning | verified fixed | Library unselected-branch cases and editor shell-ternary test retain reachability difference | Nothing |
| R1-F3.7 — Denial and directive-condition sibling | Placeholder cannot bypass member denial; check other condition consumers | verified fixed | `a_placeholder_does_not_bypass_member_denial`; `a_placeholder_does_not_hide_a_condition_member` covers file conditions and block/collection controls; code directives use the same transclusion condition path; shared CLI policy wiring separately tested | Nothing |
| R1-F4 — Loading-data skill guidance still says member typos are silent | R1-F4.1–R1-F4.2 below | verified fixed | Loading-data guidance now invokes the shared member rules | Nothing |
| R1-F4.1 — Obsolete sentence | Remove contradiction | verified fixed | Darkmatter expression skill's typed-literal-returns bullet says typed element typos warn and unknown/computed contracts remain silent | Nothing |
| R1-F4.2 — Documentation siblings | No remaining contradictory current guidance | verified fixed | Checked expression/loading-data topics, interpolation docs, policy reference, DMLS reference and darkmatter skill guidance | Nothing |

The failure matrix uses the public composition result, checks the original missing-file error and exactly one warning at its authored position, and runs a successful control for each row. It does not merely assert an internal accumulator. The shell and directive placeholder cases likewise check public results and corrected spelling controls. There are no partial repairs, newly unblocked findings, or newly discovered siblings left open.

## Unblocked Findings

None.

## Blocked Findings

None. No repair waits on a human decision.

## Requirement coverage

All requirements here need **Level 1** evidence: expression classification, in-process composition, language-server messages, and hermetic CLI subprocesses. The feature makes no terminal-emulator, keyboard, mouse, or browser behavior promise requiring Level 2 or Level 3.

| Spec acceptance criterion | Evidence checked and executed | Assessment |
| --- | --- | --- |
| 1 — Typed nonempty/empty data, computed contracts, per-site deduplication | Library `ac1_rows_and_empty`, editor `ac1_editor`, typed-contract classifier cases | Level 1; satisfied |
| 2 — Equivalent accesses, optional members, function and literal bases | Public classifier access/function tests and composition `ac2_forms` | Level 1; satisfied; host test handler panics if passive analysis executes it |
| 3 — Pending schema declarations and unknown pending constraints | Library pending-template tests, editor held-resolution and refresh tests, dynamic-schema cases | Level 1; satisfied; composition resolves before observing members |
| 4 — Type/absence tables, references, narrowing and invalidation | Classifier table, projection and scope tests | Level 1; satisfied; conservative unknown/open/scalar alternatives retained |
| 5 — Paths, literal dotted keys, computed keys, array elements, no cascade | Public classifier path/key tests and composition `ac5_paths` | Level 1; satisfied |
| 6 — Lexical identity, shadowing and source-document deduplication | Classifier scope tests, composition `ac6_identity` and repeated region-body cases | Level 1; satisfied |
| 7 — Surfaces, agreement, runtime/editor reachability and authored positions | Per-surface library/editor agreement, shell-piece repairs, body edits, multiline and Unicode position tests | Level 1; satisfied |
| 8 — Advisory default, selective/all denial, structured output and failure retention | CLI member suite; library failure matrix, denial, discovery and existing observation tests | Level 1; satisfied |
| 9 — Current docs and skill guidance | Expression topic, DMLS diagnostics, policy reference and darkmatter skill reviewed against implementation | Satisfied |

The robustness matrix does not apply: these changes concern a text grammar and diagnostic propagation, not a new or changed configuration/file-format loader. Existing schema and data loaders remain responsible for their formats.

## Boundaries checked

The repair keeps the recommended narrow boundary from R1-F3: an in-string placeholder can be joined to the authored expression when its replacement does not introduce the enclosing quote or a backslash. It does not diagnose arbitrary generated expression syntax. This limit is explicit in the implementation and current docs; it does not require a new escaping framework.

The CLI still checks references before composition. That discovery pass emits no member warnings; a failure there precedes a composition observation. The CLI regression test distinguishes this from a failed transclusion condition that composition actually evaluates and whose warning it must retain. Keeping discovery-only candidates out of reports follows the spec's explicit passivity/reporting boundary; changing the CLI's earlier error source is unnecessary for this repair.

Shell preparation already prepares both branches before choosing one. A failing piece preserves its own observations; successful unselected ordinary branches remain silent. The change does not introduce unconditional reporting of successful unselected branches. A failing transclusion condition keeps its original evaluation error alongside observed warnings rather than replacing that error with a denial error. Both choices preserve the existing evaluation order and need no human prerequisite.

## Verification performed

- `just test member` in `darkmatter/`: **207 passed**, 2.137 seconds for execution.
- `just test failure` in `darkmatter/`: **438 passed**, 5.303 seconds for execution.
- `just test shell` in `darkmatter/`: **1,232 passed**, 8.582 seconds for execution.
- No selected individual test reached five seconds. These filtered runs overlap; their counts are not a distinct-test total.
- `just check-tier-coverage darkmatter`: passed, zero stranded tests. The member modules are declared by their packages' consolidated `l1` targets and actually ran.
- The compiler emitted an existing linker warning about the large unwind section; it did not prevent compilation or test execution.

This review did not rerun the full area suite or lint: the repair log records those implementation gates, and this review changes only review documents and requested metadata. No formatting command, commit, lifecycle-directory move, or runtime-code edit was performed. Cross-OS evidence remains the CI/CD responsibility and is not a readiness blocker.

## Recurrence

None. All prior defect classes are verified repaired; no repeated finding is carried forward.
