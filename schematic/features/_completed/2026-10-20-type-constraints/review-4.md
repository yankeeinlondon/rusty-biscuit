---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F3 (high): Unenforceable request rules do not consistently block generation"
  - "R3-F1 (high): Rebuilding free-form compositions loses generation errors"
observations: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-23T21:58:00-07:00"
spec: 2026-10-20-type-constraints/spec.md
implemented: false
log: /Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-20-type-constraints/log.md
implemented_by: claude/opus
description: "A **feature** review of `2026-10-20-type-constraints/spec.md`"
feature: 2026-10-20-type-constraints/review-4.md
previous: 2026-10-20-type-constraints/review-3.md
next: 2026-10-20-type-constraints/review-5.md
---

# Review of type constraints, iteration 4

The feature is **not ready for production**. The alias rejection repair is verified fixed, and supported nullable and one-member composition wrappers now survive rebuilding. Two previously reported high findings remain: root-component unions still produce warnings where the specification requires errors, and rebuilding still removes unsupported compositions before generation can report their rules. No new finding or newly discovered sibling is added.

Read the specification, plan, implementation log, all three earlier reviews, and the repair log's third Finding Ledger before verification. The repair log's blocked dispositions were checked against the implementation and the specification; they are not author rulings. Existing decisions about serialized-body validation, exact arithmetic, annotation-only formats, and excluding generic union evaluation remain in force.

## Previous Finding Verification

Required instances include the repair ledger's child rows. Previously fixed instances were checked through their implementation and the rerun acceptance tests. The remaining composition instances are carried under R3-F1, rather than counted again under the older reference finding.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Rebuilding imported definitions loses supported constraints | R1-F1.1–.8: map values, free-form rules, format, map enum, root const, flattened extras, occurrence const, inline/named items; .9: primitive/renamed/alias/parameter/form controls; .10–.11: baseline and export-fidelity consumers | **verified fixed** | Inspected rule emission; reran the compiled `constraint_preservation` acceptance test, direct preservation/export tests, historical baseline, and schema-fidelity tests. The enumerated original rules remain present. | None of the original instances. The previously identified composition sibling remains R3-F1. |
| R1-F2: Invalid constraints are accepted or discarded instead of rejected | R1-F2.1–.4: component and endpoint shape validators, bundle reload, all input locations, numeric/null loader controls; .5–.7: dropped-subschema disclosure, duplicate pattern diagnostics, incompatible component rules | **verified fixed** | Shared component/metadata validation remains wired into API build, bundle loading, and all output validators. Rerun shape matrices, contradictory imports, loader tests, disclosure tests, and located output errors pass. | None. |
| R1-F3: Unenforceable request rules do not consistently block generation | R1-F3.1–.2: root `anyOf`/`oneOf`; .3: positional items; .4: nested compositions/map values; .5: supported traversal; .6: JSON body, multipart, catalog; .7: import disclosure; .8: other unsupported keywords | **partially fixed** | Independent public-validator sweep confirms root unions still warn at both input entry points. Positional items and the other unsupported positions error. Shipped CLI validation succeeds with 113 unchecked-rule warnings. | R1-F3.1, .2, and .6 are unchanged. No new prerequisite or author ruling changes the required policy. |
| R1-F4: Pruning declared properties applies extra-property rules to valid fields | R1-F4.1–.5: inactive membership, evaluator selection, pure map, active property, nested reference and multipart | **verified fixed** | Inspected membership-preserving pruning and runtime property selection. Rerun fresh-output tests accept declared fields and refuse actual extra entries with redacted paths. | None. |
| R1-F5: Nullable plans bypass explicit enum and const restrictions | R1-F5.1–.6: generated/runtime enum and const; null alternatives and kind controls; rebuilt membership; nullable export overlap and rejecting sibling | **verified fixed** | Own whole-value checks still precede nullable reference skipping. Runtime null tests, generator nullable tests, rebuilt enum/const tests, and OpenAPI nullable export tests pass. | None. |
| R2-F1: Rebuilding free-form schemas drops declared-property exemptions | R2-F1.1–.7: seven rebuilding positions; .8–.9: active-property, actual-extra and nominal-name controls; .10: generated plans/preparation | **verified fixed** | Rerun compiled acceptance checks all seven positions through `validate()` and `into_parts()`; direct/rebuilt generated-plan comparisons retain membership. Independent composition probe also confirms dropped compositions leave declared names as empty schemas beside extra-value rules. | None. |
| R2-F2: Rebuilding free-form schemas drops referenced constraints — remaining alias instances | Original .1–.7 and .11: ordinary references at all free-form positions and direct item/extra-value references; .8: nominal reference; .9: downstream refusal; .10: documentation; .13: nonrecursive/top-level controls; remaining .14–.15: recursive alias and alias body | **verified fixed** | Inspected `RuleSource::finish`, alias recursion detection, body-rule comparison, and import preflight. Rerun tests reject an unrepresentable alias body with its endpoint and rule pointer, writing neither file. An independent public `run_import` probe over the preservation fixture rejects `RecursiveForest` before either file is written. Scalar/item-only aliases and ordinary/nominal reference controls pass. Import documentation now describes the rejection and workaround. | None of the remaining alias instances. The earlier composition child .12 was assigned to R3-F1 in review-3 and remains there; this outcome does not claim it is fixed. |
| R3-F1: Rebuilding free-form compositions loses supported traversal and generation errors | R3-F1.1–.7: six shapes at seven positions; .8: ordinary-reference controls; .9: downstream decisions; .10: direct planner/body/multipart controls; repair .A: supported wrappers; .B: generic compositions; .C: baseline repository-input literal | **partially fixed** | Compiled acceptance verifies both supported wrappers at all seven positions, including refusal, null acceptance, and corresponding generated plans. Independent compilation confirms wrapper structure survives but all four generic shapes still become empty schemas at those positions; direct validation gives 56 errors, rebuilt validation zero. Inspected .C's input literal inside the `openai` test; baseline tests pass. | Only .1–.7 and .9's generic-composition portions, recorded as repair child .B. Supported wrapper child .A and baseline-input child .C are fixed. |

## Unblocked Findings

### R1-F3 (high): Unenforceable request rules do not consistently block generation

**Authority — spec, “Supported schema traversal”:** “A constrained request beneath an unsupported composition or unresolved reference gets an actionable generation error.” “Definition errors versus unsupported rules” also states: “Generation must fail for a supported request rule it cannot enforce,” with pattern and format exceptions only. **Defect class:** the position of an unsupported composition changes refusal into a warning, allowing generation without the required check.

In schematic-gen, [composition_findings](../../gen/src/codegen/value_rules/mod.rs:710), which classifies unreachable request rules, still selects `Severity::Warning` for a component-root `anyOf` or `oneOf`. This is the same remaining instance reported in reviews 1–3.

The independent public-API reproduction loads the real preservation fixture, retains its `CreateItem` JSON request, adds a multipart JSON entry point using the same component, and replaces only that component's schema for each case. Both entry points receive the same classification.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F3.1 | Root component, JSON body and multipart JSON field | `anyOf: [{type: string, maxLength: 3}, {type: integer}]` | Warning at both entry points | Generation error |
| R1-F3.2 | Root component, both entry points | Same members under `oneOf` | Warning at both | Generation error |
| R1-F3.3 | Positional items, both entry points | Constrained member beneath `items: [...]` and `prefixItems: [...]` | Error at both | Clean |
| R1-F3.4 | Other unsupported compositions, both entry points | Nested `anyOf`, nested `oneOf`, two-member `allOf`, `not`, map-value union | Error for each at both | Clean |
| R1-F3.5 | Supported traversal, both entry points | Ordinary item object, nullable alternative, one-member `allOf` | Rerun planner tests retain plans; compiled preservation test enforces wrapped references | Clean |
| R1-F3.6 | Generation consumers and shipped catalog | Public Rust validator; shipped `validate --api openai`; catalog expectation | CLI exits successfully with 113 type-constraint warnings: `CreateConversation` 13, `CreateResponse` 50, `Getinputtokencounts` 50. Catalog test accepts these counts. | Refuse unsupported constrained requests consistently; tests must reflect that policy |
| R1-F3.7 | Import disclosure | Dropped parameter subschemas and positional parameter items | Disclosure/parse-error tests pass | Clean |
| R1-F3.8 | Other unsupported traversal, both entry points | `additionalItems`, `contains`, `unevaluatedItems`, `patternProperties`, `propertyNames`, `unevaluatedProperties`, `dependentSchemas`, schema `dependencies`, `if`, `then`, `else` | Error for every keyword at both | Clean |

The latest repair increases the visible warnings from 42 to 113 by restoring supported reference traversal. That is progress in preservation, not a new authorization for warning-only generation. The repair log correctly identifies that applying the specified error policy makes existing all-catalog generation tests fail. Those tests currently require generation of clients the spec requires refusing; their green result cannot settle the policy in the opposite direction.

The requested repair is consistent refusal and corresponding test expectations. It does not request union evaluation, endpoint removal, or deleting the committed OpenAI client. A negative generation test can verify the required refusal. If keeping all-catalog Rust generation successful is an additional mandatory product requirement, the author must revise the specification to resolve that conflict; no supplied author ruling does so. This review neither invents that exception nor treats existing warning expectations as an accepted decision.

### R3-F1 (high): Rebuilding free-form compositions loses generation errors

**Authority — spec, “One source for model rules”:** “Imported Rust definitions must rebuild equivalent rules,” including inline schemas. “Supported schema traversal” requires an actionable generation error for constrained requests beneath unsupported compositions. **Defect class:** rebuilding deletes schema edges before the generator's refusal decision can see their rules.

In schematic-gen, [free_form_rules](../../gen/src/model_gen.rs:701), which rebuilds rules for imported free-form values, now retains the two supported wrappers. It still intentionally removes generic `anyOf`, `oneOf`, multi-member `allOf`, and `not`, including their reachable rules. The original supported-wrapper defect is fixed; the title is narrowed to the remaining refusal defect.

The independent compiled reproduction starts with the real preservation fixture. At each of its seven existing probe positions it adds a constrained string under each generic composition, alongside the unchanged nullable and one-member-reference controls. It runs public import, `run_import` with definitions output, offline compilation, public `module_group()` assembly, then public Rust validation of both direct and rebuilt APIs. There are **56 direct errors and zero rebuilt errors**: four lost rules at seven positions, reached by two request endpoints.

For every failing row, the four shapes are `anyOf: [constrained string, integer]`, `oneOf: [constrained string, integer]`, `allOf: [constrained string, string]`, and `not: constrained string`, with `maxLength: 11` on the string.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R3-F1.1 | Free-form field root, `Item.mixed_value` | Four generic shapes; two supported wrapper controls | Four generic properties rebuild as `{}`; both wrappers survive | Preserve enough unsupported structure to refuse generation; wrappers clean |
| R3-F1.2 | Nested property, `mixed_value.inner` | Same six shapes | Same | Same |
| R3-F1.3 | Free-form items, `mixed_value.items` | Same six | Same | Same |
| R3-F1.4 | Free-form extra values, `mixed_map.additionalProperties` | Same six | Same | Same |
| R3-F1.5 | Map-valued model field, `mixed_attributes.additionalProperties` | Same six | Same | Same |
| R3-F1.6 | Inline object array items, `mixed_entries.items` | Same six | Same | Same |
| R3-F1.7 | Flattened extra values, `MixedExtras.additionalProperties` | Same six | Same | Same |
| R3-F1.8 | Ordinary-reference controls at all seven positions | Unwrapped `owner` reference | Compiled acceptance and generated-plan comparisons pass | Clean |
| R3-F1.9 | Downstream generation decisions | Imported schema → emitted definition → registered API → Rust validator and request plans | Required errors disappear with the four generic schemas; no nested rule remains to compile into a plan | Retain the information needed for the existing refusal policy |
| R3-F1.10 | Direct planner, JSON body and multipart controls | Generic compositions and supported wrappers | Direct unsupported compositions error; supported wrappers produce checks | Clean |

Repair ledger child R3-F1.A is verified fixed: the compiled acceptance test exercises `validate()` and `into_parts()` for both wrappers at all seven positions, and compares the relevant direct/rebuilt plans. Child R3-F1.B remains open. This is preservation and diagnosis work, not a request for generic composition evaluation or a larger runtime validation model. The specification explicitly requires refusal, so documenting the silent loss alone does not satisfy this acceptance criterion.

## Blocked Findings

None under the supplied specification. The repair log raises a real consequence for catalog generation, but offers no changed prerequisite or author ruling that supersedes the required refusal policy. The existing policy is decided, rather than a default awaiting confirmation. Human review alone would not change readiness.

## Recurrence

- **R1-F3** is the unchanged root-union policy instance from review-1.md, review-2.md, and review-3.md. Positional and other unsupported-traversal repairs remain verified. Newly visible OpenAI warnings are reached through the repaired wrappers; they use the same unchanged severity branch.
- **R3-F1** repeats review-3.md's generic-composition instances at the same seven sites. It was originally enumerated as a sibling of review-2.md's R2-F2 reference-preservation class. The latest repair swept and fixed the supported-wrapper portion but deliberately left all four generic compositions out. This review adds no sibling or new syntax requirement.

The boundary remains the existing six shapes and seven rebuilding positions. No new evaluator, escaping layer, or synthetic payload matrix is requested. The remaining failures are explicit preservation/refusal acceptance criteria, rather than new edges introduced by repair machinery.

## Input Robustness Matrix

Default outcome: malformed supported constraint metadata produces a located error before generation writes output. Exceptions remain absent optional metadata, empty patterns, `uniqueItems: false`, imported valid patterns outside the enforced subset preserved with warnings, annotation-only formats, and arbitrary JSON data inside enum/const values.

| Validator or loader | Public-result verification |
| --- | --- |
| Endpoint constraint metadata and semantic validation | Full L1 reruns the shared field-shape matrices: absent, null, wrong whole-field types, applicable collection-element shapes, and empty values. Old snapshot absence remains compatible; explicit null is rejected where required. Valid enum/const JSON data is retained. |
| Component constraint validation | Full L1 reruns malformed keyword, contradiction, type applicability, API-build, bundle-reload, and all-output rejection tests. Numeric/exact text, counts, pattern/format, uniqueness, enum/const and items validators have controls and invalid shapes. |
| OpenAPI loader | Full L1 reruns JSON/YAML duplicate keys, invalid/trailing content, format syntax, malformed keyword shapes, exclusive bounds, and exact-number cases. CLI wiring checks a valid fixture and located invalid bounds with no output. |
| Bundle loader | Full L1 reruns valid control, omitted metadata, duplicate-key, wrong-root, trailing-content, and located-error cases. |
| New alias rejection | Public `run_import` checks both alias-body and recursive-alias failures before either output file is written. Scalar/item-only and rule-free recursion controls pass. These are schema traversal failures after parsing, not extra document-shape loaders. |

Searched constraint parsing and rebuilding for `serde(default)`, `Option`, `filter_map`, `unwrap_or_default()`, and `.ok()`. Optional constraint fields use presence-aware deserialization. Loose reads in planner/rebuild code are behind the shared validator; source schema traversal deliberately excludes compositions as reported above. No additional malformed-input finding was established. The existing validators were not multiplied into a field-by-format test matrix.

## Verification

**Level 1 is appropriate for every requirement in this feature.** It changes library authoring, format loading, schema preservation, code generation, request preparation, mock sends, and pagination. It adds no real-terminal rendering or keyboard-input promise requiring Level 2 or Level 3.

| User-facing requirement | Verification level and evidence |
| --- | --- |
| Fluent constraint authoring and invalid-definition rejection | L1 builders, registered-model, metadata/component matrices, bundle and output-validator tests |
| Exact import/export and rebuilt definitions | L1 direct preservation, numeric/export tests, offline compiled definition assembly, and independent composition/alias probes |
| All violations, deterministic safe paths, Unicode, patterns, exact numbers, null and deep equality | L1 runtime evaluator/report tests plus generated fixture requests |
| Request refusal before transport through standalone validation, preparation, build, ordinary/full/raw/direct sends | L1 freshly compiled clients and mock transport; mutation, serialized-data reuse, secrecy and unchanged valid-body controls |
| Pager effective-size checks and unchanged public controls | L1 pager rules, definition conflicts and generated GitHub pagination refusal |
| No response-value enforcement | L1 generated response outside its declared rules still decodes |
| Tests are compiled and selected | Declared generator `l1` target, live fresh-output recipe, tier audit with zero stranded tests; temporary probes declared in that target during execution and removed afterwards |

Executed on this host:

- `just test` in `schematic/`: **3,156 passed**, 23 skipped; fresh-output checks **17 passed**; runtime compile-fail check **1 passed**.
- `just test-schema --test value_rules --test pagination`: **16 passed**.
- `just check-tier-coverage schematic`: **zero stranded tests**.
- Temporary public policy and compiled composition probes: **2 passed**, checking 20 policy shapes at both body/form entry points and four generic compositions plus wrapper controls at seven rebuilt positions.
- Temporary public recursive-alias rejection probe: **1 passed**, located refusal and neither definition file written.
- `cargo run --quiet -p schematic-gen -- validate --api openai`: **exit 0**, confirming the 113 type-constraint warnings and successful Rust validation reported above.

Probe assertions confirm the observed behavior, including the defects; their success is not evidence that those defects satisfy the spec. Temporary probe source is saved at `/tmp/type-constraints-review-4-probe.rs`, with outputs in `/tmp/type-constraints-review-4-*.log` and the independently rebuilt API in `/tmp/type-constraints-review-4-rebuilt.json`. Production code was not changed. No formatter, commit, push, live-provider call, CI change, or lifecycle move was performed. Existing unrelated TLS/compilation test durations are not new findings about this feature. Cross-OS evidence is left to CI and does not affect readiness.

## Observations

None added in this iteration. Earlier observations are not carried as findings.
