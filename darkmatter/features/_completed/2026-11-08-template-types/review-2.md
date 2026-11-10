---
$schema: feature-review.yaml
ready: false
findings:
  - 'R2-F1 (high): File lookup skips an earlier unsaved schema buffer'
human_review: true
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-09T14:36:14-08:00
spec: 2026-11-08-template-types/spec.md
implemented: true
description: 'A **feature** review of `2026-11-08-template-types/spec.md`'
feature: 2026-11-08-template-types/review-2.md
previous: 2026-11-08-template-types/review-1.md
next: 2026-11-08-template-types/review-3.md
log: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-08-template-types/log.md
implemented_by: claude/opus
human_review_items:
  - 'R2-F1: recursive (%) references still never select an unsaved buffer; only direct candidates consult open buffers'
  - 'R2-F1: biscuit-file records a host-declared present location as ProbeDisposition::Matched rather than a new variant'
  - 'R2-F1.15: the buffer-first repair was widened to the DMLS Markdown link graph (arena::locate), the same defect class'
---

# Review 2: Template types

The feature is **not production ready**. All three previous findings are verified fixed across their listed instances. One newly found lookup case remains: a schema file that exists only in an editor buffer can lose to a later file on disk, causing validation against the wrong schema or an unnecessary approval refusal. This can be repaired within the spec; no human decision is needed.

The review read the specification, previous review, implementation log, and the repair log's Finding Ledger before checking the repairs. It also examined type substitution, example relocation, file selection and admission, pending requiredness, schema-file validation, editor projections, completion, and their tests. The earlier review contains no pending Human rulings section. Its accepted scope choices are preserved, including resolving the whole effective template for completion and leaving contract-schema template types outside this feature.

## Previous Finding Verification

“Required instances” includes every child in the repair's Finding Ledger in `log.md`, including the extra instances discovered during repair. Completion claims were checked against implementation and executable assertions.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Tuple examples bypass read approval and lose their defining-file base | R1-F1.1–R1-F1.25: returned/imported/transitive examples; tuple elements and spreads; defining-file base; function/property refusal controls; final example loading; Darkmatter classification; static tuple imports; tuple pattern and reachable-slot checks | **verified fixed** | `rehome_atom_examples` now explicitly handles tuples, descending through each element and spread with the same gate and base. Public import/refusal tests and Darkmatter grant tests passed. Detailed evidence below covers all child rows. | None of its listed instances. |
| R1-F2: Import lookup requires grants for unused search candidates | R1-F2.1–R1-F2.10: direct/explicit/transitive imports and examples; static first-match control; missing first root; sibling fallback; implicit-relative fallback; observations before refusal | **verified fixed** | `select_schema_file_checked` uses the ordered selector with admission immediately before each probe; biscuit-file stops at the first disk match. All 11 lazy-admission tests and both lower-level admitted-resolution tests passed, as did the Darkmatter grant controls. | None of its listed disk-backed instances. R2-F1 is a newly found open-buffer sibling, not an incomplete repair of one of these rows. |
| R1-F3: Nested grouped union arms are never substituted | R1-F3.1–R1-F3.19: bare/array/grouped/union/object/pattern-key/tuple/named positions; final lowering and CLI result; constraints/markers/nullability; invariant errors; pending presence; template constraints; export; editor transition | **verified fixed** | `TypeSubstitution::atom` recursively substitutes each grouped union before its enclosing definition checks invariants. Public static-equivalence assertions, passive-query tests, CLI validation, and the DMLS pending-to-resolved test passed. | None. |

The following groups account for every ledger child. Test names below belong to simplified-schema's declared L1 integration target unless another package is named.

| Instances checked | Evidence inspected and rerun |
| --- | --- |
| R1-F1.1–.5, .8–.11, .14 | `template_type_imports::an_example_in_every_returned_or_imported_position_is_gated`: each returned/imported scalar, array, object, union, pattern-key and transitive pattern-key refuses the example. |
| R1-F1.6–.7 | `an_example_in_a_returned_tuple_position_is_gated_and_observed` and `refusing_an_example_in_a_returned_tuple_position_refuses_the_resolution`: both positions, inline and file-backed, grant-before-content order, observed bytes, refusal before opening. |
| R1-F1.12–.13, .17–.18, .23 | Imported tuple tests cover Pair, Rest, and recursive Tree; a valid inner example and invalid outer example prove the defining base. Static import results match; denied examples refuse. |
| R1-F1.15–.16 | `an_example_in_a_transitively_imported_tuple_position_is_gated_from_its_own_file`: both positions, granted and denied, with content observations. |
| R1-F1.19–.21 | `an_example_a_returned_definition_cannot_lower_is_never_read`, plus the exhaustive example traversal and final JSON loader: function signatures/property declarations refuse without consuming their examples; loadable tuple references are admitted and rehomed before lowering. No separate loader gate is required. |
| R1-F1.22 | Darkmatter `an_example_in_a_returned_tuple_needs_its_own_grant`: approval-required result with no content read when ungranted; granted result records the example. |
| R1-F1.24–.25 | `template_type_refusals` tests reject lookaround patterns in returned/imported tuple positions and reject active slots reached through tuple positions. |
| R1-F2.1–.6 | `template_type_lazy_admission` first-root, explicit, transitive, example, and static controls; assert selected dependencies and asked/observed paths. |
| R1-F2.7, .10 | Missing-first-root test refuses the second probe before content access, keeps the first missing probe, and succeeds after granting both roots. Darkmatter independently checks the grant outcome. |
| R1-F2.8–.9 | Sibling fallback and implicit-relative tests prove when later probes are and are not requested. Biscuit-file's `admitted_resolution_asks_only_about_candidates_the_search_reaches` and `admitted_resolution_stops_at_a_refused_candidate` check the shared resolver boundary. |
| R1-F3.1–.10, .12 | `template_type_results` static-equivalence tests check exported JSON and positive/negative strict validation for every listed position, including nested groups, pattern keys and tuple elements. |
| R1-F3.11 | `grouped_union_arm_in_a_named_type_used_twice_equals_static` asserts one evaluation and validation at both use sites. |
| R1-F3.13 | Darkmatter CLI `a_schema_file_with_a_template_type_in_a_nested_grouped_union_is_valid` checks successful command output. |
| R1-F3.14–.17 | Nested-group tests assert array limits, hoisted requiredness, nullable items, located non-null failures, pending requiredness through the public property view, and nested template-constraint substitution. |
| R1-F3.18 | `template_types_passive::export_round_trips_a_template_type_in_a_nested_grouped_union` checks passive export and recollection of its slot. |
| R1-F3.19 | DMLS `a_template_type_in_a_nested_grouped_union_is_pending_then_resolved` checks pending diagnostics, no claimed requiredness, completed resolution, and validation of the resolved item type. |

## Unblocked Findings

### R2-F1 (high): File lookup skips an earlier unsaved schema buffer

**Authority — spec, “Imports, reads, and resolution”:** newly discovered files “use the current request's source-buffer reader” and the editor “honors unsaved buffers.” **Defect:** when an implicit relative reference has a new, unsaved file at its first candidate and a disk file at the repository fallback, the public resolver selects the fallback. It publishes and validates the wrong schema. With only the local directory granted, it instead refuses an unnecessary fallback probe, even when that fallback file does not exist.

**Class:** ordered file selection checks open buffers only after the entire disk search misses, so an earlier buffer is not treated as a match before later candidate selection and admission.

In simplified-schema, [the non-bare selector](../../../simplified-schema/lib/src/resolve/select.rs:209) runs the whole `resolve_admitted` lookup and returns a disk match immediately; `buffers::first_open` runs only for a total disk miss. [The admission wrapper](../../../simplified-schema/lib/src/resolve/select.rs:240) likewise cannot stop when an admitted first candidate has an open buffer. The schema-root branch already checks buffers after each individual root, so it is clean.

Reproduced through `resolve_yaml_schema_with_roots`, returned `PendingSchema::resolve`, and `Prepared::validate_strict`, using the existing lazy-admission fixture with one added open buffer:

- Request directory: `<repo>/doc`; repository fallback: `<repo>`.
- Open, unsaved `<repo>/doc/lib/types.yaml` exports `Person` with `name: string`.
- Saved `<repo>/lib/types.yaml` exports `Person` with `age: number`.
- The template is `who: "{{ t }}"`; the host answers `Person@lib/types.yaml`.

Granting both directories returns `<repo>/lib/types.yaml` in `imports`. Strict validation rejects `{"who":{"name":"x"}}` and accepts `{"who":{"age":3}}`, the opposite of the open schema's contract. Granting only `<repo>/doc` returns the test host's refusal for `<repo>/lib/types.yaml`. Removing the fallback disk file still produces that refusal. A user creating a local type file in the editor beside an existing repository-level library can reach this without malformed or adversarial input.

The sweep follows the selected identity into imports, example dependencies, exported schema, validation, and editor reference selection. Each row used the same temporary fixture and open-buffer snapshot. Clean controls establish that the buffer is present and usable.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F1.1 | Returned named import | Implicit relative; first candidate buffer-only; fallback on disk; both granted | Imports fallback `lib/types.yaml` | Import first candidate's buffer |
| R2-F1.2 | Returned import admission | Same, grant only local folder | Refuses fallback probe | Select granted buffer; never request fallback |
| R2-F1.3 | Returned import admission with missing fallback | Remove fallback file; local grant only | Still refuses fallback probe | Select granted buffer without fallback probe |
| R2-F1.4 | Transitive returned import | Returned `Outer@./outer.yaml`; Outer imports `Person@lib/types.yaml` | Publishes fallback's `age` definition | Publish buffer's `name` definition |
| R2-F1.5 | Static named import through the shared selector | `Person@lib/types.yaml`, same snapshot | Imports fallback | Import first candidate's buffer |
| R2-F1.6 | Returned example | `string(example(lib/example.yaml))`; first buffer and later disk example | `examples` contains fallback file | Consume first candidate's buffer |
| R2-F1.7 | Static example through the final example loader | Same example reference and snapshot | `examples` contains fallback file | Consume first candidate's buffer |
| R2-F1.8 | Root schema reference | `$schema` value `lib/schema.yaml`; buffer declares `name`, fallback declares `age` | `referenced_files` and export identify fallback/`age` | Identify first buffer/`name` |
| R2-F1.9 | Public selection result consumed by DMLS reference projection | `select_schema_file("lib/schema.yaml", ...)`, same snapshot | `Found` names fallback | `Found` names first buffer |
| R2-F1.10 | Final validation decision | Resolve row .1, then strict-validate name/age objects | Rejects name; accepts age | Accept name; reject age |
| R2-F1.11 | Explicit relative control | `Person@./lib/types.yaml`, first candidate buffer-only | Uses buffer; clean | Use buffer |
| R2-F1.12 | Bare-name schema-root control | First schema root has buffer-only `types.yaml`; second has disk file; only first granted | Uses first buffer; clean | Stop at first root's buffer |
| R2-F1.13 | Missing-all-disk control | Implicit relative; buffer-only first candidate; all directories granted | Uses first buffer after probing fallbacks; selected identity is correct | Use first buffer, stopping before fallbacks |
| R2-F1.14 | Unsaved edit to an existing first file | First candidate exists on disk and has an edited buffer; local grant only | Publishes edited buffer's `name`; clean | Use edited buffer |

Imports in nested type positions and trigger payloads use the same import engine and selector, with no separate file-choice decision. Example relocation and final example loading both reach this selector; both are represented above. DMLS reference projection directly consumes the tested public selection result; row .9 is API evidence, not a claim of an additional language-server protocol reproduction.

**Recommended repair:** make an admitted open buffer participate in first-match selection at each candidate, before probing or admitting a later one. Preserve biscuit-file's candidate order, containment checks, typed failures, and grant-before-buffer-access rule. Keep static and returned references on the same selector. Extend the current resolver hook or candidate-selection seam rather than duplicating path-resolution rules or special-casing the example and import callers. Add public-API regressions for wrong selection, unnecessary refusal, and clean controls, asserting the resulting dependencies and validation behavior. No new syntax, approval policy, or persisted format is needed.

## Blocked Findings

None.

## Recurrence

R2-F1 is a **sibling missed by the earlier sweep** of `review-1.md`, R1-F2, “Import lookup requires grants for unused search candidates.” Both concern advancing past the first usable match and asking about unnecessary fallback locations. The prior repair correctly made disk probes lazy; its implicit-relative tests used files on disk and did not include a first candidate that exists only in an open buffer. The underlying buffer ordering problem also exists in the static selector, so this is not a regression introduced by that repair.

The sweep should also have combined open-buffer presence with the multi-candidate import/example lookup, then followed the selected path into dependencies and validation. This review includes those sites and the root/reference projections sharing the selector. The original R1-F2 children remain verified fixed; the new sibling has its own ID and complete repair inventory above. This is a bounded candidate-selection defect, not a request for broader defensive machinery.

## Verification and test rigor

All required behaviors are parsing, library results, command output, completion results, or editor-protocol behavior. Level 1 is appropriate: this feature has no terminal styling, terminal input encoding, or OS keyboard-event requirement. No terminal or browser window was opened.

| User-facing requirement | Strongest evidence reviewed or run | Level and assessment |
| --- | --- | --- |
| Allowed/refused positions, result shapes, literal braces, suffixes, unions, nullable presence, passive queries/export, authored locations | `template_types_passive`, `template_type_results`, `template_type_refusals`, `quote_the_slot`; public result and positive/negative static-equivalence assertions | L1; appropriate |
| Defining-file imports/examples, cycles, reachable-slot and pattern refusal, grants before memo/read, read observations | `template_type_imports`, `template_type_lazy_admission`, memo tests, Darkmatter template-type tests; additional sweep above | L1; R2-F1 exposes the remaining buffer-selection gap |
| Passive trigger matching and replacement; replaced properties do not evaluate | Darkmatter template-trigger tests and assembly implementation | L1; appropriate |
| Schema-file/document validation, every slot, grant refusals, prompted handler, assignment conversion, data-output exit status | CLI template-type suite and scripted approving/denying handler assertions | L1; appropriate; handler seam tests the approval decision without terminal UI assumptions |
| Composition, cleanup and phase decisions after resolution; invalidation and warm observations | Darkmatter template-type tests and slot session/cache implementation | L1; appropriate |
| Editor pending/resolved/failed/stale state, cancellation, unsaved edits, missing dependencies | DMLS slot suite, including repaired nested-union transition | L1 protocol; appropriate; existing-file edits pass, but new buffer-only first candidates need the R2-F1 regression |
| Claudine passive name/static-value completion, non-prompting dynamic values and silent fallback across composition modes | Committed-prompt completion suite and completion implementation | L1 binary/in-process; appropriate |
| Static and shipped-schema preservation, minimal-feature compilation | Simplified-schema minimal-feature suite and corpus assertions | L1; appropriate |

Review runs:

- simplified-schema `just test template_type`: **151 passed**; its additional minimal-feature run: **1,567 passed**.
- Darkmatter `just test dynamic_schema_slot_template`: **40 passed**.
- Darkmatter `just test schema_slots_editor`: **53 passed**.
- Claudine `just test-cli completion_committed_prompt_schema`: **13 passed**.
- biscuit-file `just test detailed_resolution`: **18 passed**, plus its syntax-only target **6 passed, 104 skipped**.
- `just check-tier-coverage` for simplified-schema, Darkmatter and biscuit-file: **zero stranded tests**. Relevant modules are included in the declared L1 targets and use ordinary L1 names.
- A temporary public-API test produced the R2-F1 instance results and intentionally failed to emit the collected evidence. Its source was restored byte-for-byte afterward. The failure is a review reproduction, not a failure of the existing committed tests. Temporary evidence is under `/tmp/template-review-buffer-sweep.log`; the reproduction details above are sufficient to recreate it.
- `git diff --check` passed before writing the review. Full lint and cross-platform execution were not repeated; the repair log records their earlier results. Cross-platform proof is outside this review's readiness criteria.

The input robustness matrix does not apply: this feature changes a type/template text grammar and resolver behavior, not a configuration loader or its field contract. Returned-definition shape tests reject null objects/scalars, malformed/empty definitions and mixed invalid array members through public results; supported string/list forms have positive controls. No configuration-field requirement was inferred from the template grammar.

No new decision requires a human stop or a second confirmation of an accepted choice. The remaining finding is unblocked and repairable within the specification.
