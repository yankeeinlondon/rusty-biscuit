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
created: 2026-11-09T15:14:23-08:00
spec: 2026-11-08-template-types/spec.md
implemented: false
description: 'A **feature** review of `2026-11-08-template-types/spec.md`'
feature: 2026-11-08-template-types/review-3.md
previous: 2026-11-08-template-types/review-2.md
---

# Review 3: Template types

The feature is **production ready** under the specification and its accepted decisions. No new findings were identified. The previous finding is verified fixed across all 18 instances in the repair ledger, including the additional sites discovered during repair. No finding remains blocked or unresolved.

This review read the specification, both earlier reviews, the implementation log, and the repair log's Finding Ledgers. It checked the shared file selector and its callers, the new buffer-presence hook, type substitution, passive pending-type views, read admission and observations, schema-file validation, editor behavior, and shell completion. Neither earlier review contains a Human rulings section. The specification's rulings and the earlier reviews' accepted decisions are preserved; advisory implementation choices already recorded there are not submitted for confirmation again.

## Previous Finding Verification

The repair ledger is an inventory of required instances, not proof of completion. The checks below used the implementation and executable assertions that distinguish the selected buffer's schema from the later disk file's schema.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R2-F1: File lookup skips an earlier unsaved schema buffer | R2-F1.1–R2-F1.18: all 14 original rows and all four repair-ledger additions | **verified fixed** | The shared selector calls biscuit-file's per-candidate presence hook. Admission and containment precede buffer lookup; a buffer match stops further selection. Public resolution, dependencies, strict validation, Darkmatter grants, editor transitions, and the graph regression pass. Each child is accounted for below. | Nothing. This completes the previously reported sibling of R1-F2; it is not a partial repair or a new sibling finding. |

The following table accounts for every R2-F1 ledger child. Unless otherwise stated, the evidence is in simplified-schema's `template_type_lazy_admission` L1 module; all 24 tests in that module ran within the passing template-type selection.

| Child ID | Required instance | Verification evidence and result |
| --- | --- | --- |
| R2-F1.1 | Returned implicit-relative import; first buffer before later disk file | `a_returned_import_selects_an_open_buffer_at_its_first_candidate_over_a_later_file`: imports and searched locations contain only the first candidate; positive and negative strict validation identify its `name` schema. |
| R2-F1.2 | Only the first candidate's directory granted | `a_returned_import_of_an_open_buffer_never_asks_about_the_later_candidate`: resolution succeeds and admission/content records contain only the local file. Darkmatter's `an_unsaved_buffer_at_the_first_candidate_needs_a_grant_only_for_itself` independently passes through the real grant host. |
| R2-F1.3 | Same local grant with no fallback disk file | `a_returned_import_of_an_open_buffer_with_no_later_file_never_probes_further`: succeeds without a later probe or approval request. |
| R2-F1.4 | Transitive returned import | `a_transitive_returned_import_selects_the_open_buffer_its_file_names`: records the outer import and local buffer, never asks about the fallback, accepts the nested `name` object and rejects `age`. |
| R2-F1.5 | Static named import using the same selector | `a_static_import_selects_an_open_buffer_at_its_first_candidate_over_a_later_file`: imports and observations name the buffer; strict validation accepts `name` and rejects `age`. |
| R2-F1.6 | Returned example reference | `a_returned_example_selects_an_open_buffer_at_its_first_candidate_over_a_later_file`: the public result's `examples` identifies the local buffer and the gate asks only about that location. |
| R2-F1.7 | Static example reference | `a_static_example_selects_an_open_buffer_at_its_first_candidate_over_a_later_file`: `examples` and `searched` identify only the local buffer. |
| R2-F1.8 | Root schema reference | `a_root_schema_reference_selects_an_open_buffer_at_its_first_candidate_over_a_later_file`: `referenced_files` and `searched` identify the first candidate; exported properties contain `name`, not `age`. |
| R2-F1.9 | Public selection and editor projection | `the_public_selection_finds_an_open_buffer_at_its_first_candidate_and_probes_no_further` returns the buffer and a one-location attempt list. DMLS's `a_new_unsaved_file_at_a_returned_import_first_candidate_is_its_file` passes through editor requests: the buffer admits the value and is never saved. |
| R2-F1.10 | Final validation decision | `assert_buffer_person` builds `Prepared` and strict-validates both objects: `name` succeeds and `age` fails. This is called by the returned, static, bare-root, and edited-buffer cases, not merely inferred from selected paths. |
| R2-F1.11 | Explicit-relative control | `an_explicit_returned_import_selects_its_open_buffer`: selects its sole candidate and asks only about it. |
| R2-F1.12 | Bare-name schema-root control | `a_returned_bare_name_import_stops_at_an_open_buffer_in_the_first_root`: the first root's grant suffices; the later disk definition never wins; strict validation identifies the buffer's schema. |
| R2-F1.13 | Buffer-only match with every disk candidate absent | `a_returned_import_found_only_as_an_open_buffer_records_no_probe_after_it`: import identity, asked/observed probes, and both searched sets stop at the buffer. |
| R2-F1.14 | Edited buffer over an existing first disk file | `a_returned_import_reads_the_unsaved_edit_of_its_first_file`: the first disk candidate is selected, then its buffer supplies the edited definition; validation identifies the buffer rather than either disk copy. |
| R2-F1.15 | DMLS Markdown link graph | `graph::arena::tests::test_an_unsaved_buffer_at_a_middle_candidate_outranks_a_later_existing_file` passes. The fixture asserts three planned candidates and a document-node target rather than the later disk-file target. `arena::locate` uses the same presence hook after its existing first-indexed-candidate shortcut. |
| R2-F1.16 | Consumers of attempted locations | Code inspection: DMLS `overlay/references.rs::probes` enumerates `selection.attempted`; the schema selector records its probes into observations, including refusal paths. Rows .1, .3 and .13 assert the shortened public lists. No consumer independently resumes the fallback search. |
| R2-F1.17 | Other file resolvers and providers | Code inspection: DMLS `context::resolve_reference`, Darkmatter `markdown/fs.rs::resolve_entry_in_context`, simplified-schema `file_context::resolve_document_file_ref`, and `schema/source.rs::resolve_target` use disk resolution without an after-search open-buffer fallback. They do not contain this ordering defect. |
| R2-F1.18 | Kind registry and trigger discovery | Code inspection: the kind registry collects claims and groups duplicate claims; trigger discovery collects root entries through its source abstraction and applies filename shadowing. These are discovery/collection rules, not the disk-search-then-buffer-fallback algorithm repaired here. Existing kind/trigger restrictions remain intact. |

Earlier findings R1-F1 (tuple example approval and defining-file bases), R1-F2 (approval of unused search candidates), and R1-F3 (nested grouped-union substitution) had no remaining instances after review 2. They remain **verified fixed**: this review reran their import/refusal, lazy-admission, nested-group static-equivalence, CLI, and editor regressions. The selector repair retains lazy admission; example relocation still descends through tuple elements and spreads; substitution still processes grouped unions recursively. No previously fixed instance was observed to regress.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No recurrent finding is raised in this review. R2-F1's relationship to R1-F2 remains as recorded in review 2, but its complete repair inventory is now verified fixed. There is no new sibling to carry forward.

## Spec coverage and test rigor

All behavior required here concerns parsing, public library results, command results, grant decisions, completion results, or editor protocol state. **Level 1 is appropriate for every requirement.** The specification explicitly requires no focused terminal or browser window. Shell completion is tested at the completion request/result boundary; this feature does not promise terminal key encoding or visual styling.

| User-facing requirement | Evidence checked | Verification level and result |
| --- | --- | --- |
| Allowed and refused positions; string/list results; union flattening; arrays and suffix merging; nullable presence | `template_types_passive`, `template_type_results`, and grammar/refusal assertions; static-equivalence helpers compare exported schemas and positive/negative strict validation | L1; appropriate and passing |
| Literal brace text remains literal; malformed and empty results fail; passive query/export and unresolved validation remain passive | Passive/result/refusal suites, including pending requiredness, subtype/narrowing behavior and export round trips | L1; appropriate and passing |
| Defining-file imports/examples; cycles; refusal of reachable active slots; introduced pattern restrictions | `template_type_imports`, `template_type_refusals`, and lazy-admission tests; exhaustive tuple traversal inspected | L1; appropriate and passing |
| Grants before probe, buffer or memo use; distinct refusal codes; dependencies on success and failure | Library read-gate tests, Darkmatter template-type tests, selector and host implementation; buffer selection evidence above | L1; appropriate and passing |
| Pending trigger matching; replacement prevents evaluation; excluded schema layers remain slot-free | `dynamic_schema_slot_template_triggers` and broader slot/trigger tests | L1; appropriate and passing |
| Schema-file and document validation resolve before judging; assignments use resolved types; refusals exit unsuccessfully; prompted/granted/refused reads | CLI template-type tests and `dynamic_schema_slot_schema_files`; approving/denying handler assertions exercise the existing approval boundary | L1 library/CLI; appropriate and passing |
| Composition, cleanup and dependent decisions use resolved types; failures do not become valid/clean results | `dynamic_schema_slot_surfaces`; resolution and lowering guards inspected | L1; appropriate and passing |
| Cache observations, changed imports, missing candidates, and grant rechecks | Darkmatter template-type cache tests plus session/read-observation implementation | L1; appropriate and passing |
| Editor pending, resolved, failed and stale states; cancellation; unsaved edits and missing imports | `schema_slots_editor`, including the newly repaired unsaved first candidate and nested-group transition | L1 protocol; appropriate and passing |
| Claudine names remain passive; static values stay available; template values use non-prompting grants and fail silently | `completion_committed_prompt_schema` across compose, inline-compose and sequence; completion implementation inspected | L1 binary/in-process; appropriate and passing |
| Static schema preservation and minimal-feature support | Static-equivalence controls, shipped-corpus assertions in the minimal-feature run, and syntax-only resolver checks | L1; appropriate and passing |

The configuration input robustness matrix does not apply: this feature changes a type/template text grammar and file selection, not a configuration loader or its field contract. Returned-value tests nevertheless exercise the public result for supported strings/lists and rejected null/scalar/object, empty and malformed forms. No additional configuration-field contract is inferred from these grammar inputs.

The terminology sweep found no old schema-slot names in the inspected production code and affected skills. Remaining ordinary phrases about a cursor's value position describe a different concept. Current documentation describes the per-candidate buffer rule and the disk-only recursive-reference limitation; the latter was already recorded as an accepted implementation choice and is not reopened.

## Executed verification

- simplified-schema `just test template_type`: **164 passed**; the recipe's additional no-default-features run: **1,567 passed**.
- biscuit-file `just test detailed_resolution`: **22 passed**; its additional syntax-only nextest target: **6 passed, 104 skipped**.
- Darkmatter `just test dynamic_schema_slot_template`: **41 passed**.
- Darkmatter `just test dynamic_schema_slot`: **262 passed**, including schema-file, composition, cleanup, reference-check and trigger tests.
- Darkmatter `just test schema_slots_editor`: **54 passed**.
- Darkmatter `just test test_an_unsaved_buffer_at_a_middle_candidate`: **1 passed**.
- Claudine `just test-cli completion_committed_prompt_schema`: **13 passed**.
- `just check-tier-coverage` for simplified-schema, biscuit-file and Darkmatter: **zero stranded tests**. Relevant integration modules are declared in their L1 targets; the executed test names confirm selection.

Initial concurrent recipe runs passed their tests but collided while writing the shared JUnit report, which prevented slow-test recording. The affected Darkmatter and Claudine runs were repeated sequentially; reports then recorded successfully, with no selected L1 test at or above five seconds. This was a review-run orchestration collision, not a feature failure. No production source or test was changed for this review.

Full lint and cross-platform execution were not repeated. Their earlier results and the unavailable remote-host check are recorded in the repair log; neither is represented here as fresh execution evidence. Cross-platform proof is outside this review's readiness criteria. No new human decision or human-only verification is required.
