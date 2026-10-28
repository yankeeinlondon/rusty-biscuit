---
$schema: feature-review.yaml
ready: false
findings:
  - 'R1-F2 (high): Runtime diagnostic projections still locate shell-block command failures at the opener'
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-28T06:20:28-07:00
spec: 2026-11-01-map-and-filter-directive/spec.md
implemented: true
description: 'A **feature** review of `2026-11-01-map-and-filter-directive/spec.md`'
feature: 2026-11-01-map-and-filter-directive/review-2.md
previous: 2026-11-01-map-and-filter-directive/review-1.md
next: 2026-11-01-map-and-filter-directive/review-3.md
log: '/Volumes/coding/wt/rusty-biscuit/feat-map-and-filter/darkmatter/features/2026-11-01-map-and-filter-directive/log.md'
implemented_by: codex/gpt-6.1-sol
---

# Review 2 — Map and Filter Directives

The feature is **not production ready**. Three previous findings are verified fixed. The diagnostic repair is partially fixed: failures in repeated shell blocks now retain their iteration, but their structured location identifies the block opener rather than the failing command. This is an outstanding instance of R1-F2, not a newly discovered sibling. No finding needs a human decision.

## Previous Finding Verification

Read `review-1.md`, the original implementation log, and the repair log's `### Finding Ledger` before verification. There is no `## Human rulings` section in the earlier review. Its informational decisions and the repair's implementation choices are not author rulings that supersede the specification.

The table checks every ledger child, including clean controls. Test names below refer to the darkmatter library's declared L1 integration suite unless a package is named explicitly. All these tests ran in this review; their passing status alone does not establish that their assertions enforce the required behavior.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1 — Selection guards lose refinements across destructured and object bindings | R1-F1.1–R1-F1.7 | verified fixed | Shared subject/member identity inspected; composition, passive analysis, and DMLS tests below pass | Nothing |
| R1-F1.1 — Destructured filter body, object guard | Additional filter, while, until | verified fixed | `scope_schedule::destructured_filter_body_with_an_object_guard` tests render both members in every slot | Nothing |
| R1-F1.2 — Object filter body, destructured guard | All three selection/stopping slots | verified fixed | `object_filter_body_with_a_destructured_guard` tests render both members | Nothing |
| R1-F1.3 — Destructured map result after object guard | All three slots and mapped field type | verified fixed | `destructured_map_result_with_an_object_guard` tests pass; mapped-result negative control rejects transferring facts by name | Nothing |
| R1-F1.4 — Differently named object parameters | All three slots | verified fixed | `object_filter_body_with_a_differently_named_object_guard` tests pass | Nothing |
| R1-F1.5 — Both callbacks destructure | Filter and map, all three slots | verified fixed | Both `*_with_a_destructured_guard` test modules pass | Nothing |
| R1-F1.6 — Guarded file field in later callbacks | Sort, while/until, mapping expression | verified fixed | `a_file_field_checked_through_an_object_parameter_is_read_through_a_destructured_one` passes with uncertain calls denied | Nothing |
| R1-F1.7 — Editor and passive call checker | Shared identity, shadowing isolation | verified fixed | `collection_analysis::a_guard_fact_crosses_destructured_and_object_spellings_in_the_shared_scopes` and DMLS `collection_editor::guard_facts_cross_destructured_and_object_spellings_in_editor_diagnostics` pass; `TypedLocal::aliasing_field` and `Subject::local` use the same member path | Nothing |
| R1-F2 — Runtime diagnostic projections drop authored locations or iteration identity | R1-F2.1–R1-F2.16 | partially fixed | Located budget errors, iteration wrappers, library report projection, and CLI parity inspected and tested | R1-F2.14 still identifies the shell-block opener; its test asserts that weaker location |
| R1-F2.1 — Visit reservation | Map and filter | verified fixed | `provenance::{map,filter}::item_visit_reservation`; CLI visit-limit parity | Nothing |
| R1-F2.2 — Authored-body copy | Map and filter | verified fixed | Both `authored_body_copy` rows assert authored line and original item | Nothing |
| R1-F2.3 — Interpolation insertion budget | Map and filter | verified fixed | Both `interpolation_insertion` rows assert located exhaustion and path | Nothing |
| R1-F2.4 — Ordinary shell output budget | Map and filter | verified fixed | Both `shell_output_insertion` rows assert authored shell site and path | Nothing |
| R1-F2.5 — Shell-block output budget | Map and filter | verified fixed | Both `shell_block_output_insertion` rows identify the block whose combined output is inserted | Nothing; the block opener is appropriate for this whole-block output charge |
| R1-F2.6 — Child output budget | Map and filter | verified fixed | Both `child_output_insertion` rows and CLI insertion parity pass | Nothing |
| R1-F2.7 — Cached child replay | Located replay and applicable parent iteration | verified fixed | Both `cached_child_replay` rows and `budget::a_cached_child_is_charged_again_for_every_insertion_and_every_visit` pass; replay reservation uses the including site's iteration | Nothing |
| R1-F2.8 — Repeated interpolation failure | Map and filter | verified fixed | Both `repeated_body_interpolation` rows; CLI interpolation parity | Nothing |
| R1-F2.9 — Repeated page-block gate | Map and filter | verified fixed | Both `repeated_page_block_gate` rows; CLI gate parity; structured location is authored line 7 | Nothing |
| R1-F2.10 — Nested collection gate | Map and filter | verified fixed | Both `nested_collection_gate` rows retain the enclosing item and authored gate | Nothing |
| R1-F2.11 — Transclusion target expression | Map and filter | verified fixed | Both `repeated_transclusion_target` rows retain authored target and item | Nothing |
| R1-F2.12 — Transclusion condition | Map and filter | verified fixed | Both `repeated_transclusion_condition` rows retain authored condition and item | Nothing |
| R1-F2.13 — Ordinary shell argument validation | Null item, map and filter | verified fixed | Both `shell_argument_validation` rows; independent CLI controls locate the failing ordinary directive, including after another directive | Nothing for the required directive-site location |
| R1-F2.14 — Shell-block argument validation | Null argument at the authored command site, map and filter | partially fixed | Both tests pass but explicitly expect the opener. Independent CLI runs locate line 7 for failures on command lines 8 and 9; `data[0]` now survives | Carry the failing command's authored location into the structured source/span and assert it |
| R1-F2.15 — Callback failure control | Map/filter, nested paths, CLI | verified fixed | Both `callback_failure` rows and `nested_failures_report_every_enclosing_position` pass | Nothing |
| R1-F2.16 — Library diagnostic projection | Typed span/path, serialization, CLI parity | verified fixed | `the_shared_record_carries_typed_frames_and_the_authored_span` and CLI `report_locates_*` tests pass; CLI no longer repairs the path separately | Nothing in the projection itself; the shell stage supplies the wrong location in R1-F2.14 |
| R1-F3 — Static collection rejection discards earlier recovery warnings | R1-F3.1–R1-F3.7 | verified fixed | `analyze_collection_blocks_report` returns the recovered prefix; the call precheck feeds its authored-order filtering | Nothing |
| R1-F3.1 — Map recovery then static source rejection | Earlier map warning plus error | verified fixed | `pipeline::recovery_prefix` map case and CLI recovery-prefix test pass | Nothing |
| R1-F3.2 — Filter recovery then static source rejection | Earlier filter warning plus error | verified fixed | Matching filter case and CLI test pass | Nothing |
| R1-F3.3 — Map recovery then runtime pattern failure | Clean prefix control | verified fixed | Runtime-pattern map row retains both diagnostics | Nothing |
| R1-F3.4 — Filter recovery then runtime pattern failure | Clean prefix control | verified fixed | Runtime-pattern filter row retains both diagnostics | Nothing |
| R1-F3.5 — Recovery alone | Both directive families; exactly one warning | verified fixed | Recovery-alone rows succeed with one recovery warning | Nothing |
| R1-F3.6 — Other static rejection branches | Pattern, sort key, protected name, each after map/filter recovery | verified fixed | All six generated prefix cases pass | Nothing |
| R1-F3.7 — Authored-order prefix filtering | Earlier retained; later omitted; first rejection preserved | verified fixed | `a_recovered_source_after_a_static_rejection_is_not_reported` and `an_unrecovered_source_is_the_failure_before_a_later_rejection` pass | Nothing |
| R1-F4 — Expression reference retains obsolete lookup and file-guard behavior | R1-F4.1–R1-F4.3 | verified fixed | Expression reference, interpolation page, and skill updates inspected | Nothing |
| R1-F4.1 — Bare-name lookup prose | Nearest local before document property | verified fixed | Lookup prose, flowchart, and local namespace examples agree with bindings | Nothing |
| R1-F4.2 — File predicate prose | Both predicates imply file suitability only on true side | verified fixed | Guard and registration tables corrected; independent later-read caveat retained | Nothing |
| R1-F4.3 — Reserved-root table | `local` included | verified fixed | Five-prefix table includes `local`; passive documentation corpus passes | Nothing |
| R1-F2.X1 — Claudine wrapped-failure classification (repair sweep) | Classify the underlying failure | verified fixed | Claudine `compose_failed_code` reads `without_iteration()`; inspected directly | Nothing; Claudine's tests were not rerun in this review |
| R1-F2.X2 — Claudine shell-failure routing (repair sweep) | Keep the shell code and iteration context | verified fixed | Wrapped errors remain `ComposeFailed` and classification preserves `composition.shell_expansion`; inspected directly | Nothing; this internal routing choice does not require another author decision |

## Unblocked Findings

### R1-F2 (high): Runtime diagnostic projections still locate shell-block command failures at the opener

**Authority — spec, R9:** “Library-owned typed diagnostics preserve the underlying error, authored file, line/column, expression span, and nested iteration path.” Previous review **R1-F2.14** specifically requires the authored command site for shell-block argument validation. A normal repeated shell block with a null argument instead reports the opener's line and span. The command location survives in message text, but consumers cannot use that text to recover structured provenance under the same contract.

The defect class is replacing a command failure's location with its enclosing block's location. The repair fixed iteration identity but deliberately retained the broader location. That choice is recorded in the repair log, not an author ruling, and leaves the required instance incomplete.

In the darkmatter library, [the shell-block stage](../../lib/src/markdown/compose/shell_blocks/mod.rs) enters only `pair.span.start` into the site hook. [The pipeline adapter](../../lib/src/markdown/compose/pipeline/phases.rs) then uses that last-entered offset to locate every failure at the opener. The stage already knows individual command lines and physical spans; this does not need a new diagnostic system.

Reproduce through the shipped `md compose doc.md --output report-json` command, in a temporary directory with `.darkmatter-shell-whitelist` containing:

```text
exact echo first
template echo {{ string }}
```

Use this document (replace map/end-map with filter/end-filter for the other mode):

```md
---
$schema:
  data: any[](required)
data: [null]
---
::map data
::shell-block
echo first
echo {{ i }}
::end-block
::end-map
```

The report's `dm.shell.failure` has `source.line: 7`, a span covering `::shell-block`, and the correct `data[0]` path. The failing command is on line **9**. Removing `echo first` moves the failing command to line **8**, but the structured location stays on line **7**. These were separate CLI reproductions for each directive kind, using temporary policy files; no persistent approvals were changed.

The sweep covers ordinary directives, first/later commands in a block, the whole-block output charge, and the library/CLI projection. Each runtime row below was run for **both map and filter**. The output-charge control was checked through the public composition tests.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F2.13 | Ordinary shell argument validation, clean control | Null item in a lone directive and after an approved ordinary directive | Correct failing directive line (7 or 8), file, and `data[0]` | Keep the failing directive site and item |
| R1-F2.14 | Shell-block argument validation | Null item in first command and in second command after `echo first` | Structured source/span identify opener line 7 in both cases; path is correct | Identify command line 8 or 9, respectively, and preserve the item's path |
| R1-F2.5 | Whole shell-block output budget, clean control | Output exceeds remaining byte allowance | Located block insertion site and item | The block site is appropriate for insertion of its combined output |
| R1-F2.16 | Shared diagnostic projection | Project the shell-block failure to library records and report-json | Projection faithfully carries the opener location supplied by the pipeline | Preserve the command location supplied by the repaired stage, without CLI reconstruction |

Recommend carrying the current command's body-relative span through the existing shell-block failure boundary and projecting it through `BodyOrigin`. Retain the opener for failures of the block itself and its combined-output charge. Reuse the existing command spans/line metadata; do not parse the rendered message or add another error hierarchy. Tighten `provenance::shell_block_argument_validation` to expect the command rather than line 7, and add a later-command control through the public API plus one CLI wiring case. The current test's explicit opener assertion proves the implementation choice, not the required command location.

## Blocked Findings

None. The remaining repair fits the specification and needs no author ruling.

## Recurrence

R1-F2 repeats the provenance-loss class from `review-1.md`, specifically **the unchanged R1-F2.14 instance**. It is a partial repair, not a regression introduced by the repair or a sibling missed by the first review. The earlier sweep already named shell-block command validation. This review checks both first and later commands and their ordinary-shell and output-budget controls, so the next repair has a bounded target. The specification explicitly requires authored diagnostic provenance; this is not an attempt to protect arbitrary synthetic payloads.

## Spec Review and Verification

Reviewed the specification and implementation plan against the shared parser, collection type scopes, immutable bindings, structural executor, retained body provenance, budget charging, child discovery/cache integration, shell authorization, diagnostic projection, editor adapters, tests, and current documentation. No additional finding with spec, regression, or realistic-defect authority was identified.

- `just test collection` in darkmatter: **473 passed**, 13,060 tests outside the filter skipped.
- Full darkmatter `just test`: **13,523 passed**, 10 skipped; nextest reported 71.415 seconds.
- `just check-tier-coverage darkmatter`: **0 stranded tests**. Collection tests are declared by the consolidated L1 targets and selected by the live tier.
- Independent shipped-CLI shell reproductions confirmed the remaining instance and clean controls. The first attempt at the later-command control used an invalid plain whitelist line and stopped at approval; rerunning with `exact echo first` reached and confirmed the command-location defect.
- The build emitted the existing macOS linker warning about the large unwind table; no test failed. No production source was changed, no formatting command was run, and no commit was made. This review did not rerun lint or cross-OS checks.

| User-facing requirement family | Verification reviewed/run | Required level and assessment |
| --- | --- | --- |
| Grammar, arrows, patterns, source declarations, absence, map/filter body values | Shared parser/analysis/composition corpus; normal CLI invocation | L1; appropriate |
| Local lookup, guard identity, shadowing and result isolation | Composition controls, passive analysis, DMLS diagnostics/completion/hover | L1; appropriate |
| Selection, stable sorting, stopping, laziness and document-wide stages | Controlled in-process effect counters and output tests | L1; appropriate |
| Inserted data, Markdown containers, whitespace and enabled operations | Exact-output and provenance tests, including corpus examples | L1; appropriate |
| Transclusion, dynamic discovery, child parameters and shell authorization | Public preflight/composition tests and CLI argument-recorder cases | L1; appropriate |
| Shared limits, cached insertions, nested paths and report agreement | Budget boundaries, public failure records, CLI parity | L1; appropriate, except the command-location assertions identified in R1-F2 |
| Passive editor behavior | In-process language-server requests and effect tripwire | L1; appropriate |
| Documentation grammar and command wiring | Passive shipped-document corpus and stable CLI fixtures | L1; appropriate |

The feature has no keyboard-input, emulator-layout, or browser-interaction requirement needing L2/L3 evidence. Cross-OS proof remains part of normal CI and does not block this review's readiness assessment.

The input robustness matrix does not apply: this feature adds a text grammar and typed expression consumers, not a configuration/file-format loader. Existing frontmatter parsing is not redefined. Positive CLI limit validation already covers zero, negative, overflowing, and unlimited-value rejection.

Read both [map documentation](../../docs/inline/map.md) and [filter documentation](../../docs/inline/filter.md) in full. Both declare `kind: canonical-documentation`, have no stale Planned notice, and consistently distinguish a map's transformed body binding from a filter's retained original item. Examples and documentation corpus tests agree with the delivered grammar. Their diagnostic contract remains subject to the command-location repair above. The expression reference now correctly documents nearest-local lookup, reserved `local`, and both true-side file guards.

Previously recorded implementation decisions do not need reaffirmation. None creates a new human-only prerequisite for the remaining repair.
