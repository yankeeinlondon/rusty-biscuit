---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F3 (high): Unenforceable request rules do not consistently block generation"
  - "R3-F1 (high): Rebuilding free-form compositions loses generation errors"
observations: []
human_review: true
human_review_items:
  - |-
    Resolve the conflict between refusing Rust generation for request rules that cannot be checked and retaining the shipped OpenAI client's existing methods and pagination controls. Both requirements are currently in the specification. The existing generator accepts the OpenAI catalog with 113 unchecked-rule warnings; faithfully rebuilding additional compositions exposes further rules it cannot check. Generic composition evaluation is explicitly outside this feature's scope.

    Choose and record one change to the specification before another repair cycle:

    - Keep strict refusal and authorize the resulting reduction in OpenAI Rust generation, explicitly identifying which existing client methods and controls may be removed or excluded. OpenAPI and Postman export can remain available.
    - Keep the existing OpenAI client generation and authorize a precisely bounded exception for unchecked composition rules, with warnings and generated documentation. Specify whether the exception applies only to this catalog or is an explicit author choice for other APIs. Rebuilding must retain the rules and the information needed to disclose them.

    Review-4 treated successful catalog regeneration as a requirement that would need separate confirmation. This review recognizes that its loss also affects the specification's existing requirement to preserve client controls. Neither strict refusal nor preserving those controls is withdrawn here; the requested decision is which requirement to amend. A general composition evaluator would be a separate feature, not an assumed repair.
has_blocked_findings: true
blocked: true
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-23T22:10:24-07:00"
spec: 2026-10-20-type-constraints/spec.md
implemented: false
description: "A **feature** review of `2026-10-20-type-constraints/spec.md`"
feature: 2026-10-20-type-constraints/review-5.md
previous: 2026-10-20-type-constraints/review-4.md
---

# Type constraints review, iteration 5

The feature is **not production ready**. Both previous high findings remain partially fixed. No new finding, regression, or newly discovered sibling is reported. The earlier preservation, malformed-input, declared-property, nullable-value, and alias repairs remain effective for their enumerated instances.

Read the specification, implementation plan, original implementation log, all four previous reviews, and the repair log, including its iteration-4 Finding Ledger. Independently reran the public-validator sweep and the compiled definition-rebuilding reproduction. Completion flags and the repair's blocker labels were not used as evidence of correctness.

## Previous Finding Verification

The instance lists include the repair ledger's children. A fixed outcome concerns the listed instances; it does not claim that the separate composition finding is fixed.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Rebuilding imported definitions loses supported constraints | .1–.8: map values, free-form root, format, map enum, root const, flattened extras, occurrence const, inline/named items; .9: primitive, renamed, alias, parameter and form controls; .10–.11: baseline and export consumers | **verified fixed** | Inspected rule emission. Reran direct preservation/export, compiled `constraint_preservation`, historical baseline and schema-fidelity tests. The compiled acceptance compares the original rules with registered emitted definitions. | None of these instances. Composition loss remains separately under R3-F1. |
| R1-F2: Invalid constraints are accepted or discarded instead of rejected | .1–.4: stored components, bundle reload, every input location, endpoint metadata and numeric/null loader controls; .5–.7: dropped-keyword disclosure, duplicate pattern diagnostics, incompatible component rules | **verified fixed** | Inspected shared component and endpoint checks and their API-build/output consumers. L1 reran the field-shape matrices, contradictory imports, bundle/loader tests, disclosure tests and output rejection checks. | None. |
| R1-F3: Unenforceable request rules do not consistently block generation | .1–.2: root unions; .3: positional items; .4: other compositions; .5: supported traversal; .6: body/form/catalog consumers; .7: import disclosure; .8: other unsupported keywords | **partially fixed** | Fresh public-API sweep reproduces warnings at both entry points for .1/.2 and errors at the other unsupported positions. Shipped CLI succeeds with 113 unchecked-rule warnings. Supported controls and disclosure tests pass. | .1, .2 and .6 remain unchanged. Repair disposition is blocked on the conflicting catalog requirement discussed below; it is not a verified repair. |
| R1-F4: Pruning declared properties applies extra-property rules to valid fields | .1–.5: inactive membership, evaluator selection, pure map, active property, nested reference and multipart | **verified fixed** | Inspected membership-preserving pruning and runtime selection. L1 and freshly compiled clients accept declared fields while rejecting actual extra values with redacted paths. | None. |
| R1-F5: Nullable plans bypass explicit enum and const restrictions | .1–.6: generated/runtime enum and const, null alternatives and kind controls, rebuilt membership, export overlap and rejecting sibling | **verified fixed** | Inspected whole-value checks before nullable reference skipping. Runtime null tests, generated plans, rebuilt membership and nullable export tests pass. | None. |
| R2-F1: Rebuilding free-form schemas drops declared-property exemptions | .1–.7: seven rebuilding positions; .8–.9: active-property, actual-extra and nominal-name controls; .10: generated plans/preparation | **verified fixed** | Compiled acceptance runs all seven positions through `validate()` and `into_parts()` and compares the relevant direct/rebuilt plans. The additional composition reproduction confirms retained declared names beside extra-value checks. | None. |
| R2-F2: Rebuilding free-form schemas drops referenced constraints | .1–.7 and .11: ordinary references, direct items/extra values and adjacent rules; .8: nominal reference; .9: downstream decisions; .10: documentation; .13: nonrecursive/top-level controls; .14–.15: recursive alias and alias body | **verified fixed** | Compiled preservation and model-generation controls pass. Public import rejects the alias-body fixture before writing files. The fresh independent recursive-alias probe also rejects `RecursiveForest` before either definition file exists. Inspected `RuleSource::finish`, alias tracking and current import documentation. | None of these instances. Composition child .12 was assigned to R3-F1 in review-3 and remains there. |
| R3-F1: Rebuilding free-form compositions loses generation errors | .1–.7: six shapes at seven positions; .8: ordinary references; .9: downstream decisions; .10: direct planner/body/form; repair .A: supported wrappers; .B: generic compositions; .C: baseline input literal | **partially fixed** | Compiled acceptance retains both supported wrappers and enforces their referenced rules at all seven positions. Fresh compiled reproduction still loses all four generic shapes: direct validation reports 56 errors; rebuilt validation reports none. Inspected the baseline's repository-input literal inside its `openai` test; baselines pass. | Generic portions of .1–.7/.9, represented by repair child .B. Children .A/.C and the reference controls remain fixed. |

## Unblocked Findings

None. The two remaining findings share the product conflict below. This changes their repair disposition, not their priority or the required behavior of the current specification.

## Blocked Findings

### R1-F3 (high): Unenforceable request rules do not consistently block generation

**Authority — spec, “Supported schema traversal”:** “A constrained request beneath an unsupported composition or unresolved reference gets an actionable generation error.” “Definition errors versus unsupported rules” requires generation failure for supported request rules it cannot enforce, except the documented pattern/format exceptions. **Defect class:** unsupported traversal becomes warning-only when the composition is at a component root.

In schematic-gen, [composition_findings](../../gen/src/codegen/value_rules/mod.rs:710), which classifies request rules before generation, still selects `Warning` for component-root `anyOf` and `oneOf`. The reproduction loads the real preservation fixture, retains `CreateItem`, adds a multipart JSON entry point referencing the same component, and changes only that component's schema per case. Every row below was checked at both entry points.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F3.1 | Root component, JSON body and multipart JSON field | `anyOf: [{type: string, maxLength: 3}, {type: integer}]` | Warning at both | Generation error at both |
| R1-F3.2 | Same sites | Same members under `oneOf` | Warning at both | Generation error at both |
| R1-F3.3 | Positional array items, both entry points | `items: [constrained string]`; `prefixItems: [constrained string]` | Error for each at both | Clean |
| R1-F3.4 | Other unsupported compositions, both entry points | Nested `anyOf`, nested `oneOf`, two-member `allOf`, `not`, map-value union | Error for each at both | Clean |
| R1-F3.5 | Supported traversal, both entry points | Ordinary item object, nullable alternative, one-member `allOf` | Existing planner tests and compiled acceptance retain and enforce checks | Clean |
| R1-F3.6 | Public validator, shipped CLI, catalog expectations and output gate | Current OpenAI catalog | `validate --api openai` succeeds with 113 type-constraint warnings: `CreateConversation` 13, `CreateResponse` 50, `Getinputtokencounts` 50. Catalog test expects those counts; the Rust output gate only refuses errors. | Current spec requires refusal; catalog retention conflict needs resolution |
| R1-F3.7 | Import disclosure | Dropped parameter subschemas and positional parameter items | Disclosure and parse-error tests pass | Clean |
| R1-F3.8 | Other unsupported traversal, both entry points | `additionalItems`, `contains`, `unevaluatedItems`, `patternProperties`, `propertyNames`, `unevaluatedProperties`, `dependentSchemas`, schema `dependencies`, `if`, `then`, `else` | Error for every keyword at both | Clean |

These are not solely redundant enum checks: current catalog warnings include `minimum`/`maximum` on `BetaImageGenTool.output_compression` and `partial_images`, and `minItems` on tool inputs. The failure policy affects plausible provider requests. No runtime union evaluator is requested by this finding.

### R3-F1 (high): Rebuilding free-form compositions loses generation errors

**Authority — spec, “One source for model rules”:** “Imported Rust definitions must rebuild equivalent rules,” including inline schemas. “Supported schema traversal” also requires an actionable generation error beneath unsupported compositions. **Defect class:** rebuilding removes schema edges and their rules before the downstream refusal decision can see them.

In schematic-gen, [free_form_rules](../../gen/src/model_gen.rs:701), which recovers imported rules for generated model registration, still discards generic compositions. The code and its comments explicitly retain only nullable alternatives and one-member `allOf` wrappers.

The fresh reproduction copies the preservation fixture, adds the same four constrained compositions beside its existing wrapper controls at each of seven positions, runs public import and `run_import`, compiles the emitted definitions offline, assembles their public `module_group()`, and compares public Rust-validation results. Each failing shape is a constrained string (`maxLength: 11`) under `anyOf` with an integer, `oneOf` with an integer, `allOf` with another string, or `not`. The direct API reports **56 errors** across two request endpoints; the rebuilt API reports **zero**.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R3-F1.1 | Free-form field root, `Item.mixed_value` | Four generic compositions; nullable and single-member wrapper controls | Each generic property becomes `{}`; both wrappers survive | Retain enough structure for generation refusal; wrappers clean |
| R3-F1.2 | Nested property, `mixed_value.inner` | Same six shapes | Same | Same |
| R3-F1.3 | Free-form array items, `mixed_value.items` | Same six | Same | Same |
| R3-F1.4 | Free-form map values, `mixed_map.additionalProperties` | Same six | Same | Same |
| R3-F1.5 | Map-valued model field, `mixed_attributes.additionalProperties` | Same six | Same | Same |
| R3-F1.6 | Inline object array items, `mixed_entries.items` | Same six | Same | Same |
| R3-F1.7 | Flattened extra values, `MixedExtras.additionalProperties` | Same six | Same | Same |
| R3-F1.8 | Ordinary references at all seven positions | Unwrapped `owner` reference | Compiled acceptance enforces the referenced rule and matches relevant direct/rebuilt plans | Clean |
| R3-F1.9 | Import → emitted definition → registration → validator → request plans/output gate | All four generic shapes at all seven positions | Required errors disappear; no corresponding checks remain in generated plans | Preserve rules and the information needed for the specified refusal decision |
| R3-F1.10 | Direct planner, JSON body and multipart JSON controls | Generic shapes and both supported wrappers | Unsupported direct shapes error; supported wrappers produce checks | Clean |

Repair child R3-F1.A remains fixed: compiled acceptance checks refusal, valid values and nullable acceptance through `validate()`/`into_parts()` at all seven positions. Child R3-F1.C remains fixed: the baseline input literal is inside its test. Child R3-F1.B is unchanged. This finding does not expand to arbitrary composition evaluation or a new schema language.

### Why the remaining repairs are blocked

The iteration-4 repair log adds a concrete conflict with spec Goal 4: “Preserve the existing fluent setters, response decoding, and pagination controls.” Applying strict refusal to the shipped catalog stops regenerating its OpenAI Rust module. Existing callers in schematic-schema's [pagination tests](../../schema/tests/pagination.rs:300) use `list_batches().page_size(..).collect_all()`; streaming and constraint tests also consume this client. The generator validates a selected API before publishing its Rust output, so a negative test proves refusal but does not preserve that API's regeneration.

I therefore revise review-4's assessment that refusal plus negative expectations is a complete repair within the existing constraints. Keeping a stale generated client while its source cannot regenerate is not a completed regeneration/preservation contract. Removing or excluding public client methods changes scope, while introducing generic composition evaluation violates an explicit non-goal. The specification supplies no recommendation for choosing between those consequences and a warning exception. Either product change needs an explicit amendment; ordinary test expectation updates alone do not settle it.

This accepts the repair log's conflict analysis, **not** its labels as proof that the defects are fixed. The current refusal policy remains decided. Serialized-body checking, annotation-only formats and exact arithmetic remain accepted decisions and are not reopened. The frontmatter gives the author the two concrete changes to choose between; there is no interactive permission request in this run.

## Recurrence

- **R1-F3** is the unchanged component-root-union instance from reviews 1–4. Those reviews and repairs already enumerated the classifier, both request entry points, catalog expectations and downstream generation gate. No additional site is introduced here.
- **R3-F1** repeats review-3 and review-4's generic-composition instances at the same seven positions. The class originated as review-2's R2-F2 reference-preservation sibling. Earlier repairs fixed ordinary references and then supported wrappers, but left the four generic shapes and their downstream refusal decision unchanged.

The remaining boundary is fixed: four generic shapes, seven rebuilding positions, both request entry points and their downstream decisions. These are explicit acceptance failures, not new edge cases invented by repair machinery. Their high priority remains justified; repeated cycles do not turn them into polish.

## Observations

None added. Earlier observations are not carried as findings.

## Input Robustness Matrix

Default outcome: malformed supported constraint metadata produces a located error before output publication. Exceptions are absent optional rules, valid empty patterns, `uniqueItems: false`, arbitrary JSON data inside enum/const, supported preservation of unenforced imported patterns, and annotation-only formats. No new loader or validator was introduced by the latest repair.

| Validator or loader | Verified through the public result |
| --- | --- |
| Endpoint metadata/shared semantic checks | Rerun L1 field-shape matrices cover absent, null, wrong whole-field types, applicable invalid collection elements and empty values. All four parameter locations, body roots and form fields have wiring checks. |
| Stored component checks | Rerun malformed keyword, contradiction and type-applicability tests cover counts, numeric bounds, pattern/format, uniqueness, enum/const and item schemas, including API build, bundle reload and all output validators. |
| OpenAPI loader | Rerun JSON/YAML duplicate-key, invalid/trailing-content, syntax, null, exact-number and exclusive-bound cases. Existing CLI wiring checks a positive fixture and located invalid bounds with no files written. |
| Definition bundle loader | Rerun valid control, absent metadata, duplicate keys, wrong root and trailing content, with located failure results. |
| Alias rebuilding rejection | Existing alias-body test and fresh public recursive-alias reproduction reject before either file is written; nonrecursive and ordinary-reference controls still pass. |

Searched the changed constraint/import paths for `serde(default)`, `Option`, `filter_map`, `unwrap_or_default()` and `.ok()`. Optional metadata uses presence-aware deserializers. Reviewed the remaining loose conversions against their validators and exact-number failure paths; no additional malformed-input defect was established. Composition loss is reported above, rather than treated as a missing field-shape cell.

## Verification

**Level 1 is the appropriate level for this feature's requirements.** It changes definition loading, code generation, serialized request checking and mock transport behavior. It adds no terminal rendering, input-encoder or OS keyboard promise needing Level 2 or Level 3.

| Requirement | Strongest applicable evidence rerun |
| --- | --- |
| Fluent authoring, malformed rules and contradictions | L1 public builders, component/metadata matrices, loaders and output validation |
| Exact import/export and emitted definitions rebuilt through registration | L1 preservation and exact-number tests, offline compiled definition assembly and the fresh composition/alias probes |
| Unicode, pattern subset, exact numeric comparisons, deep equality, null, deterministic safe paths and aggregate errors | L1 runtime evaluator/report tests and generated fixture clients |
| Refusal through standalone validation, preparation, build, ordinary/full/raw/direct sends; no transport on failure | L1 fresh generated clients and loopback mocks; custom serialization, mutation, representation reuse and secrecy controls |
| Pager initial/default/changed effective sizes, bound conflicts and existing controls | L1 pager/definition tests, fresh generated fixtures and committed-client pagination tests |
| No response enforcement, runtime compatibility and regenerated artifacts | L1 generated fixture decoding, ABI/compiler checks, baseline/artifact checks and schematic-schema tests |

Executed checks:

- `just test` from schematic: 3,156 passed, 23 skipped; fresh-output compiler checks 17/17; runtime compile-fail check 1/1. Four unchanged runtime TLS tests exceeded five seconds; these are outside this feature's changes.
- `just test-schema`: 154/154 passed.
- `just check-tier-coverage schematic`: zero stranded tests.
- Shipped `validate --api openai`: exit 0, with the 113 type-constraint warnings described above.
- Three temporary nextest probes: policy sweep, compiled seven-position composition sweep and recursive-alias rejection all reproduced their asserted results.
- `git diff --check`: clean.

The probes assert the observed defects and clean controls; their passing status does not mean the feature meets the specification. Temporary test sources were removed and their module root restored byte-for-byte. No production code, test, fixture, generated output, lifecycle directory or Git commit was changed by this review. Lint and doctests were not rerun because this review makes documentation/frontmatter changes only; no new passing claim is made for them.

Detailed run records are `/tmp/type-constraints-review-5-tests.log`, `/tmp/type-constraints-review-5-schema.log`, `/tmp/type-constraints-review-5-probes.log`, `/tmp/type-constraints-review-5-openai.log` and `/tmp/type-constraints-review-5-tiers.log`. Cross-OS evidence is left to CI and does not affect this readiness decision.
