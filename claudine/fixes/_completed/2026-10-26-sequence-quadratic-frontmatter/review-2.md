---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-26T21:00:34-07:00"
spec: 2026-10-26-sequence-quadratic-frontmatter/spec.md
implemented: false
description: "A **fix** review of `2026-10-26-sequence-quadratic-frontmatter/spec.md`"
fix: 2026-10-26-sequence-quadratic-frontmatter/review-2.md
previous: 2026-10-26-sequence-quadratic-frontmatter/review-1.md
---

# Review 2

The fix is **production ready**. R1-F1 is verified fixed, including every
instance in the repair's Finding Ledger. No new findings or observations
remain, and no human decision is required.

This review checked the specification, plan, first review, original
implementation log, repair log, current implementation, and relevant tests.
Completion flags were not treated as proof. The repair's additional sites
are included below rather than treated as previously verified work.

## Previous Finding Verification

All executable evidence below is Level 1. Test names refer to the Claudine
library unless explicitly labeled Darkmatter or CLI. The focused library run
selected and passed every named Claudine library regression below.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Shell assignments still copy the enclosing sequence | All 21 ledger children below, including clean controls and consumers added during repair | **verified fixed** | Inspected the state handoff, container preparation, memoization, freshness checks, and indexed consumers; ran the focused library, CLI, and Darkmatter tests described below. | Nothing |
| R1-F1.1: Conditional sequence-event assignment | Sequence-container `success`, 10/55/100 steps | **verified fixed** | `a_conditional_shell_set_value_copies_bounded_state_per_step` passes with linear copied-node increments and both approved branches per step. The recorder clones the shared step view. | Nothing |
| R1-F1.2: Plain sequence-event control | Same event, plain assignment | **verified fixed** | `a_plain_shell_set_value_copies_no_state_in_a_sequence_or_group_event` and `a_plain_shell_set_value_never_materializes_the_step_state` pass; zero state nodes and every assignment command retained. This instance remains clean. | Nothing |
| R1-F1.3: Conditional group-event assignment | Group-container `success`, one shell leg per group | **verified fixed** | The conditional scaling test exercises its group variant, checks linear copied nodes, both branches, and every leg command. | Nothing |
| R1-F1.4: Plain group-event control | Same group event, plain assignment | **verified fixed** | The plain scaling test exercises its group variant at 10/100 steps with zero assignment-state copies. This instance remains clean. | Nothing |
| R1-F1.5: Conditional runtime assignment | Public shell-value execution adapter and condition lookup | **verified fixed** | `a_set_value_copies_the_same_state_beside_10_and_100_sequence_entries` checks equal copied work and `yes`; `a_conditional_value_reads_an_earlier_write_over_the_document_value` checks overlay precedence. Darkmatter's `a_ternary_reads_a_shared_view_base_and_its_overlay_write` also passes. | Nothing |
| R1-F1.6: Plain runtime assignment | Same adapter, plain command | **verified fixed** | The runtime scaling test exercises the plain form, checks equal copied work, and asserts its result. The adapter clones the view rather than collecting its entries. | Nothing |
| R1-F1.7: Retired task stacks | Authored `setup` and `teardown` | **verified fixed** | `nested_sequence_and_removed_task_keys` passes; the grammar rejects these fields before they reach the legacy collector. This instance remains clean. | Nothing |
| R1-F1.8: Ordinary shell commands | Ten placeholders per step at 10/100 steps | **verified fixed** | `shell_preflight_parses_the_same_expressions_per_step_at_10_and_100_steps` passes, including exact first-step bytes and placeholder counts. This instance remains clean. | Nothing |
| R1-F1.9: Approval consumers | Both conditional branches and their interpolated bytes | **verified fixed** | The conditional scaling test asserts both commands per step; Darkmatter's shared-view ternary test checks `commands()` with the overlay value interpolated. This instance remains clean. | Nothing |
| R1-F1.10: Container speech preparation | Nested container start and phrase classification | **verified fixed** | `a_speaking_container_start_copies_the_same_state_beside_10_and_100_siblings` passes; it checks linear work, every child execution, and the prepared phrase `Release starts`. Inputs and expression state share the base. | Nothing |
| R1-F1.11: Templated loop cap | Container `open_loop` | **verified fixed** | `a_looping_container_copies_the_same_state_beside_10_and_100_siblings` and `a_container_loop_reads_runtime_writes_through_its_view` pass. Cap lookup layers runtime writes over a shared view. | Nothing |
| R1-F1.12: Loop-pass state | Gate, condition, and action staging | **verified fixed** | The same loop tests check linear copied work, two passes per container, and later conditions seeing action/lifecycle writes. The gate and staging use the view directly. | Nothing |
| R1-F1.13: Root speech preparation | CLI root initialization and start | **verified fixed** | Inspected the CLI root's `PrewarmInputs::shared` call: submission is per root event/pass rather than per child. This instance remains clean. | Nothing |
| R1-F1.14: Action staging and completion projection | Container action state; CLI prompt completion map | **verified fixed** | `ActionStaging` clones the shared view for container actions; loop tests above pass. The CLI completion projection is per prompt launch, explicitly excluded by the spec's “Prompt launch cost beyond preflight.” | Nothing within scope |
| R1-F1.15: Preflight speech walk | Nested group/sequence phrase submissions | **verified fixed** | `nested_container_submissions_share_the_sequence_frontmatter` passes. `Readable` computes hidden template keys once for the common document; submissions clone its view. | Nothing |
| R1-F1.16: Nested source parsing | Nested sequence/group authoring documents | **verified fixed** | `nested_source` tests pass: one parse at 10/100 siblings, changed events after a rewrite, and a fresh error after a broken rewrite. Both preparation functions use the invocation's content-compared memo. | Nothing |
| R1-F1.17: Referenced document copies | Task/group/catalog documents and sequence references | **verified fixed** | `referenced_documents` tests pass: one catalog parse, no per-entry document copy, offset selection, ordered commands/events, changed catalog events, and failure after removal. `load_data_document` returns an `Arc`; task/group selection borrows entries, and sequence planning uses the loaded value. | Nothing |
| R1-F1.18: Committed output snapshots | Per-child runtime result snapshots | **verified fixed** | Confirmed this copies runtime results rather than authored definitions. The specification excludes the separate runtime snapshot work under “Track 5's other runtime items: C2–C4.” This instance remains outside scope. | Nothing within scope |
| R1-F1.19: Group source freshness | Files read at each group start | **verified fixed** | All `group_freshness` tests pass: exactly two reads per group at 10/100 groups, own-file changes detected, unrelated changes do not re-prepare this group. Capture logging includes the origin and subtree references. | Nothing |
| R1-F1.20: Graph lookups | Prompt lookup, nested command/template membership, and shell-mode settlement | **verified fixed** | `prompt_document_finds_every_reached_document_by_path` passes. Inspected each consumer: prompt/command lookups use indexes; candidate lookups retain discovery order within the authored-text bucket. Current append sites maintain all indexes. Approval/reread suites also pass. | Nothing |
| R1-F1.21: Memo parse identity | Referenced data, referenced Markdown, and data container sources | **verified fixed** | All three `parse_memo` tests pass; text-loader tests check values, mapping order, errors, and independence from current disk content. Data parsing consumes the compared bytes; Markdown entries use the parser's retained text. | Nothing |

The repair added R1-F1.10–R1-F1.21 while sweeping the original defect class.
They are repaired siblings, not partial repairs or new findings from this
review. No previous finding is blocked, regressed, or unresolved.

## Unblocked Findings

None.

## Blocked Findings

None.

## Requirement coverage and verification

The contract concerns ownership, bounded analysis, approval decisions, runtime
state, and filesystem visibility. **Level 1 is the appropriate verification
level** for these requirements. No terminal rendering or keyboard-encoding
behavior is introduced, so Level 2 or Level 3 captures are not needed to prove
the changed behavior.

| Specification requirement | Strongest applicable evidence and assessment |
| --- | --- |
| Shell expression work stays constant per step | Counted Level 1 scaling tests pass at 10/100 steps, with exact-byte and template-shape assertions. Conditional and plain assignment controls also pass. |
| Leg views do not copy enclosing definitions | Counted Level 1 `leg_views` tests pass and assert every leg runs. Inspected production cell construction. The original allocation measurements report 288-byte cells at 10/100/1,000 legs; these measurements probe the same view clone and mutex construction. |
| Prompt preflight allocations per task differ by no more than 10% at 100/1,000 tasks | Counted Level 1 preflight regressions pass. The allocation table records 439.3 KB/task versus 425.9 KB/task, a 3.1% difference. The fix's implementation log and the pipeline spike report both retain the required before/after measurements. Allocation benches were not rerun during this review. |
| Group-start state and private sibling writes remain intact | Level 1 sibling-isolation, group atomicity, mutation, retry, and nested-container tests pass in the focused library run. |
| Disk writes reach the next root step | The CLI Level 1 `a_leg_write_reaches_disk_for_the_next_step_but_not_its_sibling` test passes. It exercises the root runner's reread rather than only a library fixture. |
| Approval inventory, freshness, and reread behavior remain correct | Level 1 preflight, approval, source-rewrite, catalog-rewrite, and container CLI tests pass. Frozen children remain the execution graph; fresh events are checked against the invocation's original membership. |
| Required package gates and test placement | The repair log records final full Claudine L1/L2/lint and Darkmatter L1/lint passes. This review reran focused tests and the Claudine tier audit; feature-gated counted tests are enabled by the normal L1 recipe and CI metadata. Full gates were not repeated. |

Commands run during this review:

- In `claudine/`, `just test-library --features work-counters composition::sequence composition::lifecycle composition::looping runtime_state`: **1,512 passed**, 10.953 seconds total, no selected test exceeded five seconds.
- In `claudine/`, `just test-cli a_leg_write_reaches_disk_for_the_next_step_but_not_its_sibling sequence_containers`: **23 passed**, 8.394 seconds total. One existing test took 5.129 seconds under concurrent suite load; a focused timing check of `container_shells_use_event_time_values_and_lexical_group_outcomes` passed in **1.799 seconds**. No assertion, timeout, or tier was changed.
- At the repository root, `cargo nextest run -p darkmatter --features work-counters -E 'test(shell_result_values) | test(frontmatter_interpolation) | test(fm_namespace) | test(lookup_parity)'`: **135 passed**, 1.556 seconds total.
- At the repository root, `just check-tier-coverage claudine`: **passed, zero stranded tests**.

The review did not change implementation code, run formatting, or create a
commit. Cross-OS evidence remains CI's responsibility and does not block this
review.

## Input robustness

The repair moves data-file parsing behind a text-taking entry point so the
memo compares the bytes actually parsed. It does not add a configuration
field validator or change any field's absent/null/type/empty semantics.
Accordingly, the field-shape matrix does not expand for this change.

For document loading, inspected all three data branches: YAML, shared
JSON/JSON5, and shared JSONL/NDJSON. The file and text paths use the same
underlying parsers; duplicate-key checks, authored mapping order, and
line-delimited record handling remain in place. Passing public loader tests
cover positive objects, explicit null data, nested values, malformed syntax,
duplicate keys, JSON5 comments/trailing commas, and blank JSONL records.
The new parity test additionally checks matching public values, orders, and
error causes. The memo tests verify that held data bytes win over a different
disk revision, while Markdown is keyed by its own parser's retained text.
No new permissive default or silently filtered load-bearing field was found
in these changed paths.

## Recurrence

No finding recurs. R1-F1's defect class is verified fixed across the full
repair ledger, and this review adds no sibling instance.

## Observations

None.
