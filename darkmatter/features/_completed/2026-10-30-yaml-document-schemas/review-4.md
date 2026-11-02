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
created: '2026-11-01T01:16:47-07:00'
spec: 2026-10-30-yaml-document-schemas/spec.md
implemented: false
description: 'A **feature** review of `2026-10-30-yaml-document-schemas/spec.md`'
feature: 2026-10-30-yaml-document-schemas/review-4.md
previous: 2026-10-30-yaml-document-schemas/review-3.md
---

# Review 4

The feature is **production ready** within the specification's scope. R3-F1 is verified fixed across all eleven instances in the repair's Finding Ledger. No new finding, partial repair, regression, or newly found sibling remains.

This review checked the specification and its closed design rulings, earlier reviews, the repair ledger, implementation paths, and public-result tests. No earlier review contains a Human rulings section. Previously accepted decisions remain accepted, including gated bare-name lookup, compiled admission for direct library loads and registry candidates, saved-file Markdown trigger discovery, and the explicit embedded-resolution exclusions. No new author decision is required.

## Previous Finding Verification

The required instances below include the repair ledger's added comment, malformed-input, and sibling-classifier rows. Passing tests are evidence from this review's runs, rather than acceptance of the ledger's completion claims.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R3-F1: Unclaimed YAML in schema folders receives false trigger errors | R3-F1.1–R3-F1.11 | verified fixed | All rows below checked through implementation inspection and the library, CLI, provider, protocol, and trigger tests. | None. |
| R3-F1.1: Whole-YAML selector | Absent, null, non-string, empty, and ordinary unclaimed kinds; inside/outside schema folders; absent, valid, and missing fallback | verified fixed | `yaml_document_selection::a_kind_that_is_not_a_string_claims_nothing` and `an_unclaimed_kind_falls_back_to_the_root_schema_or_to_no_schema` pass. The selector reads an authored string only and preserves fallback outcomes. | None. |
| R3-F1.2: Shipped CLI | Empty-value null kind inside `schemas/`, with no fallback and a missing fallback | verified fixed | `schema_validate_yaml::a_null_kind_in_a_schema_root_claims_nothing` passes: exit 0, null schema, no problems; then exit 2 and the missing-reference error alone. | None. |
| R3-F1.3: Editor classification without a schema | All thirteen kind shapes, both folder locations | verified fixed | `an_unclaimed_kind_without_a_root_schema_is_not_validated_inside_or_outside_a_schema_folder` passes: NoSchema and zero diagnostics. `envelope_overlay` now checks the actual trigger claim before strict parsing. | None. |
| R3-F1.4: Editor classification with failed fallback | Same shapes and locations, missing referenced schema | verified fixed | `an_unclaimed_kind_with_a_missing_root_schema_reports_only_the_reference_error` passes: one preparation error starting at `./missing.yaml`. | None. |
| R3-F1.5: Editor classification with selected fallback | Same shapes and locations, inline owner contract | verified fixed | `an_unclaimed_kind_with_an_inline_root_schema_is_judged_by_it_alone` passes: validated with no diagnostics. | None. |
| R3-F1.6: Location and unclaimed-string controls | Outside `schemas/`; absent, empty-string, and ordinary-string kinds inside it; all three fallback outcomes | verified fixed | Each of the three provider matrices runs both paths and includes these controls, checking the public outcome and diagnostics. | None. |
| R3-F1.7: Final protocol publication | Open clean control, insert `kind:`, add missing reference, replace it with valid inline contract | verified fixed | `yaml_document_session::an_unfinished_kind_in_a_schema_root_publishes_only_whole_document_results` passes: clean → clean → exactly one preparation error at line 1, character 9 → clean. | None. |
| R3-F1.8: Genuine trigger checks and strict discovery | Valid trigger; malformed claim; invalid glob, vacuous match, count bounds, retired spelling, edited checkout definition | verified fixed | Trigger suite passes; kind-semantic and checkout-definition tests pass. The strict public parser still rejects null/non-string kinds for saved-file discovery; the new predicate changes only whether the editor applies it. | None; the accepted discovery boundary is preserved. |
| R3-F1.9: Nearby explanation | Overlay and diagnostic comments, editor documentation and skill | verified fixed | Comments in `overlay/mod.rs` and `diagnostics/frontmatter.rs` describe the claim gate. The editor topic page and darkmatter skill distinguish whole-document selection from strict saved-file discovery. | None. |
| R3-F1.10: Unloadable YAML reports once | Claimed trigger, null kind, numeric kind, each with incomplete sequence | verified fixed | `an_unloadable_file_in_a_schema_folder_reports_its_parse_problem_once` passes: one envelope error for the claimed trigger, one YAML load error for each unclaimed case. | None. |
| R3-F1.11: Sibling classifiers and consumers | Standalone claim scanner, registry boundaries/classifier, all TriggerError/TriggerWarning producers and diagnostic readers | verified fixed | Source sweep finds the trigger-state constructors only behind the new claim gate. Standalone whole-YAML authoring requires a claimed schema kind; registry lookup chooses context, not document validity. Saved-file registry classification retains its accepted discovery contract. Predicate tests include both spellings, malformed claims, BOM/CRLF, and unclaimed controls. | None. |

Earlier findings were already closed in reviews 2 and 3. Their regressions remain covered in the current runs:

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: DMLS suppresses required fallback validation | R1-F1.1–R1-F1.7 | verified fixed | Missing-data protocol test retains the required-property error and declaration/data assistance; invalid declaration and named-type failures are published once. | None. |
| R1-F2: Unsaved schema sources do not consistently refresh YAML validation | R1-F2.1–R1-F2.18 | verified fixed | Protocol tests cover unsaved definitions, imports inside/outside roots, explicit YAML and JSON targets, missing sources, absent root folders, save/watch changes, and close restoration. Shared source-buffer reads remain wired into preparation. | None within the accepted YAML scope. |
| R1-F3: Standalone YAML loses bare-name schema lookup roots | R1-F3.1–R1-F3.8 | verified fixed | Library, CLI, and editor parity tests pass for the existing gated policy and explicit paths outside it. | None. As corrected in review 2, the original recommendation was withdrawn; no broader lookup behavior is required. |
| R1-F4: YAML schema preparation still runs on the editor request thread | R1-F4.1–R1-F4.8 | verified fixed | Worker-thread observation tests and data-only-edit work-counter tests pass for kind, inline, and referenced contracts, including imports and reference facts. | None. |
| R2-F1: Secondary envelope checks override the selected YAML contract | R2-F1.1–R2-F1.22 | verified fixed | Metadata-data matrices, all three edited checkout definitions, public library/CLI results, declaration assistance and single-publication protocol tests pass. Direct-load admission remains compiled by design. | None. R3-F1 was the separately identified sibling; its complete required list is verified above. |

## Unblocked Findings

None.

## Blocked Findings

None. No finding waits on an author decision or human-only verification.

## Recurrence

No recurring finding is reported. The previously recurring secondary-classifier defect is closed by R3-F1's verified repair. The review found neither an unchanged failing instance nor another missed sibling within the specification's boundary.

## Input Robustness Matrix

The default for invalid document structure is a located load error; invalid schema declarations or selections produce preparation errors; data rejected by the selected contract produces validation problems. No path executes expressions or writes converted data. CLI exits distinguish these outcomes as 3, 2, and 1 respectively.

The intentional exceptions are: absent/null/non-string `kind` claims nothing; an unclaimed string falls through unless registry failures prevent that conclusion; an absent fallback permits NoSchema; present invalid fallback, including null, is an error; scalar/sequence/empty roots are unvalidated; optional ordinary properties retain nullable behavior; `non-null` permits absence and rejects null before conversion. Empty data is judged by its type.

| Decision point | Public-result evidence and shapes checked |
|---|---|
| Shared whole-YAML loader | `yaml_document_loading`: valid mapping control, duplicate/non-string keys, malformed content, unsupported numbers, second document, accepted start/end markers, BOM/CRLF and original spans, scalar/sequence/empty roots. CLI wiring distinguishes a document load error from a referenced schema's load error. |
| Authored `kind` selection | `yaml_document_selection`, `yaml_kind_registry`, and the repaired editor matrices: absent, both null spellings, numbers, booleans, single/mixed/all-numeric/empty arrays, nonempty/empty mappings, empty and unclaimed strings; exact case; healthy, duplicate, reserved, and incomplete registry outcomes. |
| Fallback `$schema` declaration | Selection tests cover absence, null, number, boolean, empty string, invalid mixed/all-invalid union elements, and valid inline mapping, root union, YAML reference, and JSON target. The key is removed only from the validation copy. |
| Envelope metadata and declaration carriers | `envelope_admission`, `yaml_schema_envelope`, and whole-YAML wiring tests cover controls, absent and explicit-null metadata, wrong carriers and elements, empty shapes, compiled closure, softly converted metadata, and unchanged schema carriers. Fallback fields named `description`, `categories`, and `types` are judged only by their selected contract. |
| Shared language validators | `non_null_properties`, `closed_roots`, `declaration_narrowing`, and `claimed_values` cover missing/null/wrong/empty values, required finite string claims, bad root-union arms, references/imports, closure after layering, and serialization/export preservation. These validators are tested once, with controls and invalid examples proving their use by the shipped definitions. |

The code-smell search covered whole-YAML loading, selection, registry and semantic checks, standalone admission/parsing, and editor contract preparation: `serde(default)`, optional fields, filtered elements, `.ok()`, and `unwrap_or_default()`. No load-bearing invalid field was found silently defaulted or discarded. The inspected occurrences concern optional cache/location state, already-reported loading failures, dependency hashes, or values already admitted by the loader. The strict trigger parser's different null policy remains an explicit, accepted saved-file discovery contract.

## Requirements and Verification Level

All requirements in this feature need **Level 1** verification. They concern parsing, passive validation, CLI output and exits, and editor protocol results. No requirement concerns terminal-emulator rendering, physical key events, or browser behavior; Level 2/3 evidence is therefore unnecessary.

| Requirement group | Strongest relevant verification |
|---|---|
| Shipped definitions, self-validation, source-copy edits, repository corpus | L1 public registry/validation API and passive shipped-artifact corpus tests; CLI and protocol checkout-copy tests. |
| Five roots, conflicts, failure reservation, incomplete registry, context isolation | L1 public library tests and normal CLI/protocol tests, including separate workspace folders. |
| Both routes, precedence, no baseline/triggers, soft conversion, passive strings | L1 public validation results and hermetic CLI invocation; assignments, baseline options, unvalidated records and mixed-batch exit precedence are asserted. |
| Diagnostics, locations, completion, hover, declaration assistance | L1 editor-provider tests and normal open/change/completion/hover protocol requests, including Unicode/CRLF, missing data and single error publication. |
| Unsaved edits, dependency creation/deletion, route changes and stale-state removal | L1 protocol tests over fixture-owned files and open buffers, including restoration on close and watch-event refresh. |
| Preparation outside dispatch and reuse | L1 observed worker thread identities and work counters for data-only edits, rather than timing assumptions. |
| Language additions and direct-load envelope admission | L1 SimplifiedSchema public APIs under default and minimal features; import, serialization, JSON export, subtype and layering tests. |

The relevant test modules are declared by their consolidated L1 targets. `just check-tier-coverage darkmatter` reports **zero stranded tests**. No test or production source was changed for this review.

## Executed Verification

- Darkmatter `just test yaml_`: **354 passed**.
- Darkmatter `just test trigger`: **595 passed**.
- Darkmatter `just test`: **13,050 passed, 10 skipped**; no slow-test or failure report.
- SimplifiedSchema `just test`: **1,400 default-feature tests and 1,212 minimal-feature tests passed**.

Cross-OS proof remains CI's responsibility and is not a readiness blocker. Existing accepted decisions do not require another confirmation. This review changes only the review report and the requested previous-review/spec metadata; it runs no formatter, creates no commit, and opens no terminal or browser window.
