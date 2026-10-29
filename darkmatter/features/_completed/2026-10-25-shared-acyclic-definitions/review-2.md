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
created: "2026-10-27T00:34:53-07:00"
spec: 2026-10-25-shared-acyclic-definitions/spec.md
implemented: false
description: "A **feature** review of `2026-10-25-shared-acyclic-definitions/spec.md`"
feature: 2026-10-25-shared-acyclic-definitions/review-2.md
previous: 2026-10-25-shared-acyclic-definitions/review-1.md
---

# Review 2

The feature is **production ready**. The previous finding is verified fixed,
including every instance in the repair ledger. No new findings or observations
remain. This assessment covers the current implementation and uncommitted repairs
in the session's worktree.

## Previous Finding Verification

The previous review contained one finding, R1-F1. The repair ledger adds five
compatibility, documentation, and consumer checks to its ten failing instances.
All fifteen rows were checked; the added rows describe repaired or clean sites,
not defects newly discovered by this review.

The test names below are in Darkmatter's
[shared acyclic consumer tests](../../lib/tests/l1/shared_acyclic_consumers.rs).
The focused `just test shared_acyclic` run executed all of them successfully.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Blanket exclusions leave compatible structured definitions unshared | R1-F1.1–R1-F1.15, detailed below | verified fixed | Inspected resolver selection, lowering, catalog propagation, baseline merging, and consumer changes; all 57 focused sharing tests pass. | Nothing |
| R1-F1.1: Disk catalog sharing | Two `entry` uses within `group`; declaration consumers retain their meaning | verified fixed | `a_disk_catalog_declaration_stores_a_repeated_type_once_like_ordinary_resolution`: ordinary and catalog controls each store one definition; description, members, and lowered references are checked. Disk declarations now use automatic sharing. | Nothing |
| R1-F1.2: Embedded catalog sharing | Same declaration and virtual library as the disk case | verified fixed | `an_embedded_catalog_declaration_selects_what_the_disk_catalog_selects`: one stored definition and equal lowered disk/embedded results. Embedded declarations use automatic sharing. | Nothing |
| R1-F1.3: Matched file in an object | `{ path: FILE }` | verified fixed | `a_matched_file_in_a_repeated_object_is_shared`: plain and matched controls store one definition; validation, both runtime phases, and suggestions match the inline form. | Nothing |
| R1-F1.4: Matched file in a nested list | `{ inner: { path: FILE }[] }` | verified fixed | `a_matched_file_in_a_nested_list_is_shared`: the same representation, validation, phase, and suggestion checks pass. | Nothing |
| R1-F1.5: Matched file in a dictionary | `{ <pattern::^n>: FILE }` | verified fixed | `a_matched_file_in_a_dictionary_is_shared`: one definition and unchanged behavior, including suggestions at the dictionary property. | Nothing |
| R1-F1.6: Matched file in a tuple | `tuple([FILE, number])` | verified fixed | `a_matched_file_in_a_tuple_is_shared`: one definition and unchanged validation, phases, and suggestions. | Nothing |
| R1-F1.7: Matched file list in a union | `FILE[] \| { n: number }` | verified fixed | `a_matched_file_list_in_a_union_is_shared`: one definition and unchanged validation, phases, and suggestions. | Nothing |
| R1-F1.8: Matched file in a list of objects | `{ path: FILE }[]` | verified fixed | `a_matched_file_in_a_list_of_objects_is_shared`: one definition and unchanged validation, phases, and suggestions. | Nothing |
| R1-F1.9: Matched file in a tuple spread | `tuple([string, ...FILE[]])` | verified fixed | `a_matched_file_in_a_tuple_spread_is_shared`: one definition and unchanged validation, phases, and suggestions. | Nothing |
| R1-F1.10: Catalog and baseline consumers | Carry definitions into baseline validation and effective-schema merges | verified fixed | `a_shared_catalog_baseline_merges_into_document_schemas_and_validates_as_inlined` checks equal-body reuse, different-body renaming, resolving references, and validation failures against an inline control. `the_shipped_baseline_shares_its_lifecycle_types_and_validates_as_its_inlined_form` checks shipped lifecycle verdicts and located diagnostics. | Nothing |
| R1-F1.11: Root-union compatibility | Five original container shapes, plus recursive globs reached through shared types | verified fixed | `a_root_union_arm_keeps_a_suggestion_only_match_through_a_repeated_type` preserves all five suggestion-only cases. `a_root_union_arm_enforces_a_recursive_glob_reached_through_a_shared_type` checks seven cases, including shared values used outside and inside a recursive body. | Nothing |
| R1-F1.12: Documentation and skill corrections | Remove the catalog-wide and file-match-wide exclusions | verified fixed | Inspected `definition.md`, the darkmatter schema skill, Claudine schema docs, and changed source contracts. They describe catalog sharing, contextual match lowering, and the definition-free baseline API boundary. | Nothing |
| R1-F1.13: File-alternative consumer | Shared acyclic file-list unions behave like inline unions; recursive files remain recognized | verified fixed | `a_shared_type_offers_the_file_alternatives_of_its_inlined_value` checks matched lists, eager lists, and a recursive-file control. The consumer distinguishes cycle membership from definition storage. | Nothing |
| R1-F1.14: Unused baseline definitions | Drop definitions reached only by overridden properties; retain those still used | verified fixed | `a_baseline_definition_reached_only_by_overridden_properties_is_dropped_by_the_merge` checks both outcomes. Reviewed transitive reference traversal in `retain_reached_definitions`; baseline and root-union corpus checks pass. | Nothing |
| R1-F1.15: Remaining compatibility exclusions | Nullable values, operands/deferred annotations, union markers, suggestions, optional file arms, and cyclic union arms | verified fixed | These are clean compatibility cases rather than missing repairs. Inspected `is_shareable` and per-use fallback; `context_dependent_union_bodies_stay_inlined`, conflict/default tests, and the focused consumer tests preserve the required behavior. | Nothing |

## Unblocked Findings

None.

## Blocked Findings

None.

## Observations

None.

## Recurrence

No recurring finding remains. R1-F1 is closed after checking its full ledger;
this review found no unchanged instance, repair regression, or missed sibling.

## Implementation assessment

The repair removes the two broad sharing exclusions at their source. Disk and
embedded catalogs now select definitions under the ordinary resolver rules.
Their baseline projection carries the definition table into lowering, validation,
and composition. Darkmatter's existing public
[`darkmatter_base_schema()`](../../lib/src/markdown/schemas/mod.rs:235) API
retains its definition-free result through a cached inline projection; the
shared JSON baseline retains the smaller representation. The cache identity
includes the definitions used to lower the baseline.

File-match compatibility is decided during root-union lowering, where the
relevant context is available. Shared acyclic bodies keep their inline
suggestion behavior, while references reaching recursive bodies retain the
existing enforced matching. The additional copies have separate deterministic
keys, and the shared bodies remain unchanged. Inspection covered objects,
pattern dictionaries, arrays, union alternatives, tuple slots, tuple spreads,
and references between definitions. The focused tests include positive
controls and files outside the glob, so a passing result demonstrates the
distinction rather than merely successful schema compilation.

The original selection and fallback requirements remain covered: authored-site
counting, namespace identity, primitive exclusions, local bounds, key exclusions,
typed conflicts, nullable/optional-file behavior, callable rejection, deterministic
output, and the import-depth guard. The metadata library and eight-file bundle
retain independent trigger behavior and validate through Claudine's launch path.
The fixed byte budget remains 1,000,000 bytes; the recorded maximum of 768,262
bytes gives 960,327.5 bytes with 25% headroom, rounded up as the spec requires.

The repair's recorded decisions about contextual match copies and the cached
inline baseline projection include alternatives and reasoning. They implement
the accepted contract and require no new human decision. No additional
performance or ergonomics change is necessary for readiness.

## Verification and limits

| Requirement | Appropriate verification level and evidence |
| --- | --- |
| Selection, counting, constraints, fallback, cycles, depth, and deterministic representation | L1 public resolver results: `just test shared_acyclic`, 57 passed. Assertions check stored definitions and uses as well as accepted/rejected instances. |
| Consumer parity: presence, launch/completion rules, coercion, defaults, eager files, deferred values, diagnostics, and imported dependency refresh | L1 public consumer results in the same 57-test run, comparing shared and inline controls. Existing diagnostic cases cover nested list items and missing required values. |
| CLI validation and DMLS diagnostics, completion, hover, navigation, and refresh at both uses | L1 shipped command and language-server protocol paths: `just test shared`, 164 passed, including the shared schema CLI and editor cases. These are semantic request/response requirements; no new keyboard or terminal-rendering behavior needs L2/L3. |
| Catalogs, baseline propagation, root-union matching and merging, shipped artifacts, frozen output | L1 `just test recursive_schema_golden shipped_schema_corpus root_union baseline catalog`, 608 passed. This includes all 17 golden tests and the shipped-schema corpus. No golden blessing was enabled. |
| Claudine metadata, independent triggers, disk/embedded bundle inventory and export, launch verdicts, deep lifecycle/profile cases, and fixed budget | L1 `just test claudine_schemas`, 799 passed. Reviewed metadata parity assertions through reached definitions and passive-versus-launch cases. |
| Tests are compiled and selected | The sharing modules are declared in the library's consolidated L1 binary; CLI/editor modules are declared in their L1 binaries and executed above. `just check-tier-coverage darkmatter` reports zero stranded tests. |

The frozen corpus `inputs/` and `manifest.yaml` have no diff from the
pre-feature implementation commit's parent. Changed golden outputs reflect
definition sharing; the behavior tests above provide separate semantic evidence.
New repair tests ran below five seconds. The implementation log records matched
size/timing samples and the DMLS allocation comparison; this review did not
repeat performance measurements.

The repair log reports both areas' complete L1, L2, and lint gates passing.
This review independently reran the focused L1 checks listed above, rather than
claiming another full-area or terminal run. The feature adds no emulator,
keyboard, browser, or focus behavior. Cross-platform execution remains CI's
responsibility and does not block this review.

The Input Robustness Matrix does not apply: the feature changes resolution of
a text grammar and its schema representation, not configuration field validation
or a document loader. Catalog sharing reuses the existing catalog loader.

Only this review and the requested review/spec frontmatter were changed during
the review. No implementation repairs, commits, or formatting were performed.
