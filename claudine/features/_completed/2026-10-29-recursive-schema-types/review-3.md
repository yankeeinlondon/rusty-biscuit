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
created: '2026-10-24T00:09:13-07:00'
spec: 2026-10-29-recursive-schema-types/spec.md
implemented: false
description: 'A **feature** review of `2026-10-29-recursive-schema-types/spec.md`'
feature: 2026-10-29-recursive-schema-types/review-3.md
previous: 2026-10-29-recursive-schema-types/review-2.md
---

# Review 3

The feature is **production ready** under the specification's acceptance criteria. R2-F1 is verified fixed across all 12 repair-ledger instances. The repair restores acyclic behavior without losing the recursive cases fixed in the first review. No new findings, observations, or human decisions remain.

## Previous Finding Verification

Reviewed `review-2.md`, `review-1.md`, the implementation log, and both repair Finding Ledgers in `log.md`. Completion flags were checked against source and fresh test results. The rows below enumerate every instance in the second repair ledger, including its two discovered consumers. No earlier finding was still open at the end of review 2; the four earlier findings are also rechecked below because this repair touches their implementation.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R2-F1: Recursive match repair changes acyclic schema validation and arm selection | R2-F1.1–R2-F1.12 below | **verified fixed** | All required sites inspected; the five public assembly-route tests, phase checks, caller tests, golden corpus, and recursive controls passed in this review. | Nothing. |
| R2-F1.1: Document root union, inline arm | Six acyclic shapes: property union, named union, inline object, dictionary, tuple, and list of a union | **verified fixed** | `schema_cache_validity::acyclic_match_in_an_inline_document_arm_only_suggests` passed. Each shape retains a file fragment, emits no match keyword, accepts the existing outside-glob file, and accepts it in both phases. | Nothing. |
| R2-F1.2: Document root union, referenced simplified arm | Same six shapes | **verified fixed** | `acyclic_match_in_a_referenced_simplified_arm_only_suggests` passed with the same public verdict and phase assertions. | Nothing. |
| R2-F1.3: Referenced root-union file, inline arm | Same six shapes | **verified fixed** | `acyclic_match_in_an_inline_arm_of_a_referenced_union_file_only_suggests` passed with the same assertions. | Nothing. |
| R2-F1.4: Referenced root-union file, file arm | Same six shapes | **verified fixed** | `acyclic_match_in_a_file_arm_of_a_referenced_union_file_only_suggests` passed with the same assertions. | Nothing. |
| R2-F1.5: Mixed simplified/raw root union | Same six shapes beside a raw boolean arm | **verified fixed** | `acyclic_match_beside_a_raw_arm_only_suggests` passed. The simplified arm accepts the outside-glob file; the raw boolean arm cannot account for that acceptance. This route has no simplified phase projection. | Nothing. |
| R2-F1.6: Launch and Completion re-lowering | Both phases across the four wholly simplified routes | **verified fixed** | All four route tests call `EffectiveSchema::validate_for_phase` for both phases. Inspected `union_to_root_schema`: it uses the restored direct-file attachment; new file-fragment emission is restricted to recursive definition copies. | Nothing. |
| R2-F1.7: Claudine caller arm admission | Existing caller file inside/outside a property-union alternative's glob; downstream `plan` candidates | **verified fixed** | `a_caller_file_outside_a_property_union_alternative_glob_rules_no_arm_out` passed: both `**/one/*.md` and `**/two/*.md` remain available. Inspected the shared `file_alternatives` result through `root_union_match_groups`, `admits_arm_files`, and `supplied_file_arms`. | Nothing. |
| R2-F1.8: Direct and acyclic named-file controls | Direct file and an acyclic name that inlines to a direct file, including assembly and caller admission | **verified fixed** | Direct/named Darkmatter and Claudine controls passed. All four resolver attachment calls use `attach_root_union_match`, which applies the restored direct attachment before its early return; phase lowering uses that same attachment helper. Thus the assembly routes do not bypass the existing rule. | Nothing. |
| R2-F1.9: Recursive, raw explicit-keyword, and single-schema controls | Recursive references across assembly routes, both phases, context/cache behavior, raw local reference, and suggestion-only shared definitions | **verified fixed** | All `match_below_a_recursive_reference_*` tests, the recursive phase/context tests, and `an_explicit_match_keyword_behind_a_raw_local_reference_is_enforced` passed. `a_reference_inside_an_acyclic_inline_object_is_enforced_but_its_file_sibling_is_not` also passed, checking both sides of the restored boundary within one arm. | Nothing. |
| R2-F1.10: Documentation and skill policy | Schema validation page, definition page, Darkmatter schema skill, implementation log, and affected Rust documentation | **verified fixed** | Inspected the updated descriptions and examples against the converter and file helpers: direct arm file properties and recursive definition copies enforce; the six other acyclic placements only suggest. The implementation log records withdrawal of the broader policy. | Nothing. |
| R2-F1.11: Discovered caller-completion sibling | Claudine eager classification, completion target, and partial-file relaxation for acyclic file unions | **verified fixed** | `caller_files_through_references_are_classified_and_completed` and `a_partial_through_a_reference_keeps_its_root_union_arm_applicable` passed. Their acyclic rows expect no pending-file completion; recursive rows still complete. All three helpers consume the restricted `file_alternatives` result. | Nothing. This was an additional consumer repaired in the second cycle, not an open partial repair. |
| R2-F1.12: Discovered Darkmatter caller classifier consumer | Reference-following classification of acyclic simplified and raw schemas | **verified fixed** | The acyclic resolver inlines its types, so it cannot enter the classifier's new `$ref` branch. The golden tests and acyclic named caller-file control passed. Inspected the raw-reference branch: it retains caller origin without adding a schema assertion. | Nothing. No new restriction on raw schemas was introduced. |
| R1-F1: Inline trigger definitions share an identity and corrupt editor results | Trigger/trigger and document/trigger separation; completion, hover, positioned validation; file/embedded controls; root-union re-keying; namespace documentation | **verified fixed** | Recursive merge tests and DMLS's `same_named_inline_trigger_definitions_keep_their_own_bodies` passed. Inspected trigger-owned namespaces and their use by payload assembly. Root-union merge and golden tests passed. | Nothing; no regression. |
| R1-F2: Recursive references lose root-union file match rules | Four resolver attachment calls, mixed arms, phase re-lowering, shared bodies, context/cache selection, Claudine arm admission and construction | **verified fixed** | Recursive match, phase, context, raw-control, and Claudine caller tests passed. Inspected copy collection, reference redirects, and copy lowering: recursive copies remain enforcing while shared bodies remain suggestion-only. | Nothing. The acyclic regression introduced by its repair is now fixed as R2-F1. |
| R1-F3: Caller file overrides lose their launch-directory meaning through references | Eager/lazy scalar/list values and controls; classification, projection, materialization, recheck; match copies, termination, and Claudine completion/late verdict | **verified fixed** | All eight `recursive_schema_caller_files` tests passed, as did match-copy/reference-cycle classifiers, `glob_consumers`, and Claudine supplied-file tests. Cases distinguish launch and document folders and preserve ambiguous/object values. | Nothing; no regression. |
| R1-F4: CLI timing reports omit assignment preparation and misreport cache hits | Assignment success/error, presence lookup, no assignment, discovery/early failure, library/editor controls, and assignment-triggered re-resolution | **verified fixed** | `schema_validate_timings`, library timing, and DMLS timing tests passed. Inspected `validate_one`: assignment preparation is absorbed before handling its result; validation and presence lookup share a recording; assignments still precede authoritative re-resolution. | Nothing; no regression. |

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No finding recurs in this review. Review 2's recurrence was a regression introduced while fixing review 1's recursive file matching; this repair removes that regression and preserves the original recursive behavior. The two additional consumers recorded in the repair ledger are checked above and have no remaining defect.

The repair's recorded engineering choices require no additional approval. Restricting new enforcement to recursive definitions follows the spec's acyclic compatibility requirement. Retaining caller-directory meaning through raw local references adds no validation constraint and does not reopen an accepted policy.

## Observations

None.

## Verification

Fresh runs in this review:

```sh
cd darkmatter
just test recursive_schema schema_recursive schema_cache_validity schema_validation_timings schema_validate_timings validation_timing recursive_call_check coercion_translator_conformance file_match glob_consumers
# 298 passed

just test match_copy reference_cycle assignment trigger_payload_load_errors schema_triggers schemas_union_typed_consumers golden frozen snapshot union
# 745 passed; selections overlap the preceding run

cd ../claudine
just test supplied caller_file root_union schema_completion
# 149 passed

cd ..
just check-tier-coverage darkmatter
just check-tier-coverage claudine
# zero stranded tests in each area
```

Inspected the declared library, CLI, and DMLS L1 modules. The new compatibility tests are in the already-declared `schema_cache_validity` module; the caller-file integration file is declared in the library's L1 target. The caller helper tests compile as library unit tests. None needs terminal focus or browser interaction.

The repair log records full package-area L1 and lint passes for both Darkmatter and Claudine. Those are implementation evidence, not fresh full-suite or lint runs by this reviewer. This review changed only its report and the requested spec/previous-review metadata. No production source or test was changed, and no formatter was run.

| User-facing requirement | Required verification level | Evidence assessed |
|---|---|---|
| Acyclic output and verdict compatibility | L1 | Golden corpus plus six acyclic shapes across five assembly routes; direct/named controls; Launch/Completion and caller admission checks |
| Supported recursion, finite size, graph errors, identities and merge scope | L1 | Recursive type/merge tests, frozen fixture size and depth invariance, raw/mixed schemas, embedded namespaces, cycle and import error tests |
| Deep positioned validation and presence | L1 library/CLI/editor protocol | Recursive reporting, presence, CLI validation and editor diagnostic tests, including invalid nodes beyond the former unroll |
| Cleanup, conversion, rewriting and type-level call checks | L1 | Recursive walker suites, conversion beyond the former cutoff, translator conformance, recursive call-check and union-policy tests |
| Lazy editor completion and hover at levels 1–3 | L1 editor protocol | `recursive_schema_editor`, including cursor-directed lookup and distinct same-named trigger types |
| Normal invocation and caller-file meaning | L1 public API/CLI | Recursive compose/validate/about fixtures, repeated normalization, eager/lazy scalar/list caller cases, arm selection and completion |
| Timing output, settings and cache correctness | L1 library/CLI/editor protocol | Timing suites, assignment-triggered resolution, threshold precedence/errors, cache hit/miss/context tests, changed imports and nearer candidates |
| Generator portability and performance evidence | L1 plus recorded release measurements | Generator/frozen tests passed; implementation log separates I/O, resolution and compilation and records cold/warm benchmark results and each waste item's disposition |

These requirements concern schema results and editor protocol responses, rather than keyboard encoding or terminal layout. L2/L3 testing is not required for them. Cross-OS execution evidence remains a CI responsibility and does not block readiness.

### Input robustness

The new configuration field is DMLS's `schema.slow_validation_ms`; the simplified schema type syntax is a text grammar, and generated JSON Schema annotations are writer output, so neither needs a configuration-shape matrix. This repair introduces no configuration loader.

Invalid threshold values use the existing configuration diagnostic and preserve the lower settings layer. The exceptions are absence (default 200 or the file layer), zero (log every validation), and null for the entire editor settings object (restore file settings). Explicit null for the field is invalid. Fresh public `ConfigState` tests cover these outcomes, valid overrides, negative and fractional numbers, and wrong-type strings. The field is a scalar `u64`; collection-element cases do not apply. The same deserializer rejects empty strings, arrays, objects, and booleans rather than converting them to absence. Existing TOML/editor loaders are reused, so no new duplicate-key, trailing-content, or format-syntax decision was added by this feature.

Inspected the changed readers for load-bearing defaults, null-collapsing options, filtered invalid elements, and discarded parse errors. The default on the settings struct implements the specified absent-field behavior; the new scalar does not use an `Option` that would collapse null into absence. No additional defect was identified.
