---
$schema: feature-review.yaml
ready: false
findings:
  - 'R2-F1 (high): Recursive match repair changes acyclic schema validation and arm selection'
observations: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: '2026-10-23T23:38:26-07:00'
spec: 2026-10-29-recursive-schema-types/spec.md
implemented: true
log: /Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/features/2026-10-29-recursive-schema-types/log.md
implemented_by: claude/opus
description: 'A **feature** review of `2026-10-29-recursive-schema-types/spec.md`'
feature: 2026-10-29-recursive-schema-types/review-2.md
previous: 2026-10-29-recursive-schema-types/review-1.md
next: 2026-10-29-recursive-schema-types/review-3.md
---

# Review 2

The feature is **not production ready**. All four original findings are verified fixed, including the additional instances recorded in the repair ledger. One new high finding remains: the file-match repair changes validation and arm selection for schemas without recursion, contrary to the specification's compatibility requirement. This is a regression introduced by the repair, not a partial repair of the original recursive cases. No finding requires a human decision.

## Previous Finding Verification

Read `review-1.md`, the repair log's **Finding Ledger**, and the implementation log before reviewing new work. The table includes every child row in that ledger; “verified fixed” includes controls checked and found clean. Evidence below comes from inspected implementation and tests executed in this review, rather than the ledger's completion flags.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: Inline trigger definitions share an identity and corrupt editor results | R1-F1.1 trigger/trigger lookup; .2 document/trigger lookup; .3 DMLS completion; .4 hover and positioned validation; .5 referenced-file control; .6 embedded control; .7 root-union identities and table absorption; .8 namespace invariant documentation | **verified fixed** | Darkmatter's `recursive_schema_merge::{inline_trigger_payloads_keep_their_own_same_named_definitions,a_document_node_and_trigger_nodes_keep_separate_bodies,referenced_payload_files_keep_their_own_same_named_definitions,embedded_inline_payloads_keep_their_own_same_named_definitions}` passed. DMLS's `recursive_schema_editor::same_named_inline_trigger_definitions_keep_their_own_bodies` passed, checking distinct completion keys, number hover with description, and a deep type error. Root-union identity/re-keying tests passed. Inspected `InlineOwner::Trigger`, its use in trigger assembly, and the corrected `DefNamespace` invariant: each trigger path now gives its inline payload a separate typed identity. | Nothing from R1-F1. |
| R1-F2: Recursive references lose root-union file match rules | R1-F2.1 inline arm; .2 referenced arm; .3 referenced union's inline and file arms; .4 mixed raw arm; .5 direct-file control; .6 acyclic named-file control; .7 raw explicit-keyword control; .8 phase re-lowering; .9 context/cache classification and unchanged shared bodies; .10 Claudine caller arm admission; .11 Claudine arm construction with definitions | **verified fixed** | All corresponding `schema_cache_validity::match_below_a_recursive_reference_*` tests passed, as did `phase_projections_of_a_root_union_keep_match_below_a_recursive_reference`, `match_below_a_reference_keeps_context_and_leaves_shared_uses_suggestion_only`, and the three direct/named/raw controls. Inspected all four resolver attachment calls, converter re-lowering, definition-copy allocation, and keyword origin selection. Claudine's `an_existing_caller_file_outside_a_glob_reached_through_a_reference_rules_its_arm_out` passed; its public result demonstrates that the reconstructed recursive arm retains its definition table. | Nothing from the original recursive instances. R2-F1 separately reports the repair's additional change to acyclic behavior. |
| R1-F3: Caller file overrides lose their launch-directory meaning through references | R1-F3.1 recursive eager scalar; .2 lazy scalar; .3 eager list; .4 lazy list; .5 direct scalar controls; .6 acyclic named control; .7 direct list controls; .8 classification, projection, materialization and recheck consumers; .9 match-copy classification and termination; .10 Claudine classification, completion, per-arm relaxation and late verdict | **verified fixed** | All eight public composition tests in `recursive_schema_caller_files` passed, including launch/document folders containing different files, eager/lazy scalar/list cases, ambiguity, and an unchanged supplied object. Inspected the reference-following classifier and materializer and their projection/recheck callers. Unit tests `a_file_outside_a_match_copy_reached_through_a_reference_is_still_eager` and `a_reference_cycle_ends_caller_file_classification` establish the additional classifier branches; these were also included in the final targeted verification. Claudine's `caller_files_through_references_are_classified_and_completed`, `a_partial_through_a_reference_keeps_its_root_union_arm_applicable`, and `late_verdict_reads_a_settled_arms_file_through_a_reference` passed. | Nothing from R1-F3. |
| R1-F4: CLI timing reports omit assignment preparation and misreport cache hits | R1-F4.1 cold assignment preparation and its error return; .2 presence lookup; .3 no-assignment, discovery failure and early failure controls; .4 library recorder controls; .5 DMLS recording controls; .6 assignment-triggered re-resolution | **verified fixed** | All eight `schema_validate_timings` tests passed, including `a_cold_assignment_request_reports_its_compile_miss`, failing preparation, failing discovery, and assignment-triggered schema changes. Library `schema_validation_timings` and DMLS `validation_timing_log` passed. Inspected `validate_one`: preparation is recorded and absorbed before its result is inspected, and validation plus presence lookup share a recording. Post-assignment resolution remains in place. | Nothing from R1-F4. |

There are no earlier reviews in this directory before `review-1.md`, and no carried blocked findings.

## Unblocked Findings

### R2-F1 (high): Recursive match repair changes acyclic schema validation and arm selection

**Authority — spec:** Under **Decided**, “every schema that does not recurse lowers to byte-identical JSON Schema; inlining stays the default.” Design 2 repeats the acyclic compatibility requirement. Design 7 also requires preserving each consumer's existing union policy. **Defect class:** applying recursive file-match enforcement to previously suggestion-only acyclic shapes adds a rejecting constraint and changes which root-union arms remain available.

In the `darkmatter` package, [attach_root_union_match](../../../darkmatter/lib/src/markdown/schemas/simplified/convert.rs:585) now lowers the entire arm in enforcing mode. The mode reaches every nested file atom, including acyclic property unions, inline objects, dictionaries, and tuples. [file_fragment](../../../darkmatter/lib/src/markdown/schemas/simplified/convert.rs:1789) consequently emits a match keyword where the checked-in pre-repair converter emitted none. The same mode is used by [union_to_root_schema](../../../darkmatter/lib/src/markdown/schemas/simplified/convert.rs:324) during phase re-lowering. No recursive definition is needed to reach the regression.

The repair log explicitly records this as a policy change and a decision for the author to confirm. It is not an accepted decision in the previous review or an exception in the specification. The recursive fix does not require changing these existing acyclic outcomes; a human approval step is unnecessary to restore the stated contract.

**Reproduction.** Use the existing `schema_cache_validity` fixture layout: `a/notes/x.md` exists, and open the following as `b/doc.md` through `Markdown::try_from`, then call the public `DarkmatterSchemas::validate` API:

```yaml
---
$schema:
  - spec: "{ path: file(match('./notes/*.md')) }"
  - spec: number
spec:
  path: ../a/notes/x.md
---
```

This schema has no recursion. Previously the nested glob supplied suggestions and the document passed; now it fails because the existing file is outside `b/notes`. An author grouping a file property inside an object is a realistic input. The scalar alternative `spec: "file(match('./notes/*.md')) | number"` has the same regression with `spec: ../a/notes/x.md`.

**Sweep and evidence.** A temporary L1 probe used the public resolver/validation result with six acyclic shapes: a property union, an acyclic name bound to that union, an inline object, a pattern dictionary, a tuple slot, and a list of a file/number union. For each shape, the unedited control document in `a` passed; changing only its file value to an existing file outside the document's glob caused rejection. Each shape was checked through all five assembly routes below: **30 rejecting combinations**. Direct file and acyclic named-file controls were checked through the same five routes and retained their existing rejection behavior.

For the previous-output comparison, inspected the checked-in pre-repair attachment helper: it consults `file_match_groups` only for a top-level direct file atom, which excludes all six shapes. The probe also validated each previous-equivalent JSON schema through a referenced raw schema file: removing only the newly added match keyword restored acceptance in every affected combination. This comparison preserves all other generated constraints. Launch and Completion were additionally called through `EffectiveSchema::validate_for_phase` for every route retaining a simplified projection; both reject. The mixed raw route has no simplified phase projection.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R2-F1.1 | Document root union, inline-arm attachment in Darkmatter `resolve_root_union` | All six acyclic shapes above | Inside passes; outside fails; new keyword changes serialized output | Preserve the prior output and acceptance for these suggestion-only shapes |
| R2-F1.2 | Document root union, referenced simplified-file arm | Same six shapes in the arm file | Same regression | Same prior output and acceptance |
| R2-F1.3 | Referenced root-union file, inline-arm attachment in `resolve_standalone_root_union` | Same six shapes | Same regression | Same prior output and acceptance |
| R2-F1.4 | Referenced root-union file, referenced-file-arm attachment | Same six shapes in the referenced arm | Same regression | Same prior output and acceptance |
| R2-F1.5 | Mixed simplified/raw root union | Same six shapes beside a raw boolean-property arm | Same regression; raw arm cannot admit the value | Preserve the simplified arm's prior output and acceptance |
| R2-F1.6 | Launch and Completion re-lowering through Darkmatter `union_to_root_schema` | All six shapes in the four wholly simplified assembly routes | Both phases reject the outside file | Preserve the existing acyclic phase verdict |
| R2-F1.7 | Claudine caller arm admission through `root_union_match_groups` → `admits_arm_files` → `supplied_file_arms` → unresolved caller-file candidates | Existing public-result test's acyclic `file(match(...)) | number` property; caller supplies `./other/x.md` and a partial `plan` | First arm is discarded; only `**/two/*.md` remains for `plan` | Keep both prior candidates, `**/one/*.md` and `**/two/*.md`, because this acyclic property's glob did not previously constrain admission |
| R2-F1.8 | Direct file and acyclic named direct-file controls at all five assembly routes, plus Claudine admission | Top-level `file(match(...))`, directly and through an inlined name | Clean: inside passes; outside rejects/selects the other arm | Keep the already enforced rule |
| R2-F1.9 | Recursive reference, raw explicit keyword, and single-schema controls | Original R1-F2 reproductions; a raw local reference with an authored keyword; the same recursive type outside a root union | Clean: recursive/root-union and raw explicit rules reject outside files; a single schema stays suggestion-only | Preserve all three contracts |

For R2-F1.7, the executed Claudine test is `composition::schema::supplied::tests::an_existing_caller_file_outside_a_glob_reached_through_a_reference_rules_its_arm_out`. Its property-union row now asserts the changed result. The pre-repair `root_union_match_groups` excluded property unions, so that caller could not discard this arm by its glob. The downstream public `unresolved_supplied_files` result demonstrates the user-visible loss of the first arm's file candidates. The direct and acyclic named-file rows are clean controls; the recursive row correctly verifies the original repair.

**Smallest repair:** retain the old attachment/admission behavior for acyclic shapes, while following recursive references into their enforcing copies. Apply that distinction at all five attachment/re-lowering sites and the caller admission helper; keep the successful R1-F2 recursive, context, and cache cases. Add compact compatibility fixtures for the six acyclic shapes and make the two new tests that currently assert the broader policy preserve the prior acyclic result. Update the affected schema documentation and skill text alongside the behavior. This needs no new schema syntax, protection layer, or broad synthetic matrix in the permanent suite.

## Blocked Findings

None.

## Recurrence

R2-F1 is a regression introduced by the repair for **review-1.md, R1-F2: Recursive references lose root-union file match rules**. Both concern preservation of file-selection semantics during lowering and the arm decisions that read the result. The original missing recursive enforcement is fixed; this is neither an unchanged original instance nor a previously missed recursive sibling. The repair expanded the policy to acyclic inputs while sweeping references. Its sweep should also have checked the existing acyclic output/verdict at property unions, inline objects, dictionary values, tuple slots, array items, all root-union assembly routes, phase re-lowering, and Claudine caller admission. This review includes those sites and clean direct-file controls.

The requirement has a finite boundary: preserve the specified acyclic behavior and support the new recursive references. No additional escaping rules or open-ended input-hardening machinery is warranted.

## Observations

None.

## Verification

The focused existing suites passed:

```sh
cd darkmatter
just test recursive_schema schema_recursive schema_cache_validity schema_validation_timings schema_validate_timings validation_timing recursive_call_check coercion_translator_conformance file_match glob_consumers
# 293 passed

just test match_copy reference_cycle assignment trigger_payload_load_errors schema_triggers
# 90 passed

cd ../claudine
just test supplied caller_file root_union schema_completion
# 148 passed
```

The second Darkmatter selection includes the additional classifier tests named in the previous-finding table and their assignment/trigger controls. Both package areas' tier-coverage audits reported zero stranded tests. The new caller-file file is declared by `lib/tests/l1/main.rs`; the changed library, CLI and DMLS tests belong to their declared L1 targets. No new tests depend on terminal or browser focus.

The temporary compatibility probe confirmed R2-F1 and was removed; no production code or permanent test was changed by this review. Final existing-suite verification was repeated after removing it. Lint was not rerun for the resulting Markdown-only review changes; the repair log's lint results were inspected but are not claimed as a new run.

| Spec requirement | Appropriate verification | Evidence assessed |
|---|---|---|
| Acyclic compatibility | L1 serialized-output and public verdict checks | Frozen golden tests pass, but do not include the six acyclic match combinations; R2-F1 supplies the missing compatibility check |
| Recursive graph admission, finite schema size, namespaces and merge scope | L1 | `recursive_schema_types`, merge, frozen-fixture, raw/mixed-arm and golden tests; repaired trigger identities checked through the library and editor |
| Deep positioned validation, presence, phase timing and passive invocation | L1 library/CLI | Recursive validation/reporting/phase tests and `schema_recursive`; failures identify the deep node, and normal composition uses the recursive types |
| Cleanup, conversion beyond the former cutoff, rewriting and call checking | L1 public API | Recursive clean/conversion/rewrite tests, translator conformance and recursive call-check tests |
| Lazy completion, hover and schema diagnostics | L1 editor protocol | `recursive_schema_editor` sessions cover levels 1–3, unopened definitions, empty insertion sites, definition navigation and positioned diagnostics |
| Stage logging, cache state, configuration and request context | L1 library/CLI/editor protocol | Timings, cache-validity, glob-consumer and DMLS logging tests, including repaired assignment preparation and recursive match context |
| Generator portability and performance evidence | L1 portability checks; recorded release measurements | Unrelated-CWD generator tests and frozen size checks pass; implementation log records I/O, resolution and compilation separately, plus cold/warm benchmark preparation and measured before/after results |

These contracts do not involve keyboard encoding or terminal layout; no L2/L3 verification is needed. Cross-OS execution evidence remains a CI responsibility and does not block this review.

The input-robustness matrix applies to the new DMLS integer setting, `schema.slow_validation_ms`, rather than to the simplified text grammar or generated match annotations. Its invalid-value outcome is a configuration diagnostic while the lower settings layer remains effective. Existing public `ConfigState` tests distinguish absence/default 200, explicit zero, a valid editor override, whole-settings null restoring the file layer, and explicit field null, negative/fractional numbers, and strings producing configuration errors. The scalar uses the shared unsigned-integer deserializer; no new document loader or permissive collection filtering was added. Inspected the default and null-handling paths and the changed parsing code for silent defaults, filtered elements, and discarded parse errors. No additional configuration finding was identified.
