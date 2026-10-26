---
$schema: feature-review.yaml
ready: true
findings:
  - "R3-F1 (low): Flattened allOf rebuilding repeats constraint disclosures"
observations: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-24T17:09:29-07:00"
spec: 2026-10-20-type-constraints/spec.md
implemented: true
implemented_by: claude/opus
description: "A **feature** review of `2026-10-20-type-constraints/spec.md`"
feature: 2026-10-20-type-constraints/review-6.md
previous: 2026-10-20-type-constraints/review-5.md
---

# Type constraints review, iteration 6

The feature is **production ready** under this review's severity rules. The author resolved the previous product conflict through the specification's October 24 API-level unchecked-composition opt-in. Strict refusal is now the default at component roots as well as nested positions; the explicit opt-in preserves OpenAI generation while disclosing unchecked rules. Rebuilt definitions retain the previously missing compositions.

One low finding remains: flattened composed models can report the same underlying rule at two schema positions. This is a newly exposed sibling recorded by the repair as R3-F1.13, not an unchanged loss of constraints. It does not justify another blocking repair cycle or a new schema transformation layer.

Review basis: the specification and plan, earlier reviews 1–5, the original implementation log, the latest repair's complete Finding Ledger, current source and generated artifacts. Completion flags were not treated as verification. The working tree already contained the repair; this review changes only review metadata and this report.

## Previous Finding Verification

The required-instance column includes the repair ledger's additional children. Earlier findings already closed in review 5 are included to make their regression checks explicit.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Rebuilding imported definitions loses supported constraints | .1–.8: map values, free-form root, format, map enum, root const, flattened extras, occurrence const, inline/named items; .9: primitive, renamed, alias, parameter and form controls; .10–.11: baseline and export consumers | **verified fixed** | Read rule rebuilding and reran direct preservation/export, schema-fidelity and historical-baseline tests. Fresh compiled `constraint_preservation` compares registered emitted definitions with the imported rules, including large integers and decimal divisors. | None of these instances. Composition disclosures are assessed separately below. |
| R1-F2: Invalid constraints are accepted or discarded instead of rejected | .1–.4: components, bundle reload, input locations, endpoint metadata and loaders; .5–.7: dropped-keyword disclosure, duplicate pattern diagnostics and incompatible component rules | **verified fixed** | Reran public builder, component/metadata shape matrices, loader, contradictory-import, bundle and output-validation tests. Reviewed the shared validators and their consumers. | None. |
| R1-F3: Unenforceable request rules do not consistently block generation | .1–.4: root unions, positional items and other compositions; .5: supported traversal; .6: public validator/catalog/output gate; .7: import disclosure; .8: other unsupported keywords; .9: stored opt-in; .10: diagnostics/rustdoc; .11: unresolved references and definition errors; .12: OpenAI overlay/regeneration | **verified fixed** | Newly unblocked by the spec's “Unchecked-composition opt-in,” ruled October 24. Reran the body/multipart error-versus-warning matrix, stricter-blocker precedence tests, public output refusal/publication controls, metadata and bundle round trips, emitted builder-call test, catalog counts and generated-client tests. Inspected the hand-written OpenAI overlay. | None. The emitted opt-in has source-level assertion coverage plus independently compiled builder usage; the compiled preservation fixture tests both policies on its assembled API. |
| R1-F4: Pruning declared properties applies extra-property rules to valid fields | .1–.5: inactive membership, evaluator selection, pure map, active property, nested reference and multipart | **verified fixed** | Read membership-preserving pruning and runtime selection. Fresh compiled requests accept declared names, reject actual extra values, and retain redacted error paths. | None. |
| R1-F5: Nullable plans bypass explicit enum and const restrictions | .1–.6: generated/runtime enum and const, nullable alternatives and kind controls, rebuilt membership and export overlap/rejecting sibling | **verified fixed** | Reran runtime null/whole-value checks, generated-plan tests, rebuilt membership and nullable export cases; inspected evaluation order. | None. |
| R2-F1: Rebuilding free-form schemas drops declared-property exemptions | .1–.7: seven rebuilding positions; .8–.9: active-property, actual-extra and nominal controls; .10: plans/preparation | **verified fixed** | Fresh compiled preservation test checks every position through `validate()` and `into_parts()`, then compares the relevant direct/rebuilt plans. | None. |
| R2-F2: Rebuilding free-form schemas drops referenced constraints | .1–.7/.11: references, direct items/extra values and adjacent rules; .8–.10: nominal reference, consumers/docs; .13–.15: nonrecursive controls, recursive alias and alias body | **verified fixed** | Fresh compiled preservation reaches referenced models through ordinary properties, items, extra values and supported wrappers. Alias rejection and CLI no-publication tests pass. Read alias-cycle tracking and `RuleSource::finish`. | None. Composition child .12 remains assigned to R3-F1. |
| R3-F1: Rebuilding free-form compositions loses generation errors | .1–.7: four generic shapes and supported wrappers at seven positions; .8/.10: reference/direct-planner controls; .9: downstream decisions; .A–.C: wrappers, generic retention, baseline input; .11–.12: nominal unions/root allOf; .13: flattened-member duplicates; .14: pinned importer behavior | **partially fixed** | Newly unblocked by the October 24 ruling. Fresh compiled preservation reports the same 62 errors without the opt-in and the same 62 warnings with it, preserves 62 disclosure lines, and compares active plans. Nominal composition tests pass. Independently compared public direct OpenAI import with the shipped rebuilt API; confirmed .13's extra warnings. Read the existing importer pins for .14. | Only .13: repeated disclosures from flattened member copies. Reduced to **low** because the original lost-rule failures are repaired and the remaining difference is diagnostic polish. See the instance table below. |

Repair-discovered consumers R5-X1 and R5-X2 are also checked: the schema-fidelity classifier tests and OpenAI extension round-trip pass. The documented omission of the opt-in from OpenAPI extension metadata is outside the spec's required bundle/emitted-definition storage paths and is not reopened.

## Unblocked Findings

### R3-F1 (low): Flattened allOf rebuilding repeats constraint disclosures

**Authority — spec, “Unchecked-composition opt-in,” preservation bullet:** the rebuilt API must produce “the same errors (without the opt-in) or the same warnings and disclosures (with it) as the direct import.” **Defect class:** rebuilding a composed model retains both a source composition and Schemars' flattened copies of member rules, creating additional diagnostic locations for the same underlying constraint.

In schematic-gen, [RuleSource::alias_attribute](../../gen/src/model_gen.rs:567), which preserves imported rules on emitted composed models, adds the source `allOf` while Schemars also merges flattened members into root properties. A rule can consequently be reported once beneath the retained composition and again at its copied property. When the composed model is reached directly, the copied property may instead receive a valid runtime check: all members of an `allOf` apply to the same value. I found no invalid-request acceptance or valid-request rejection caused by these copies.

The reproduction uses the vendored OpenAI document unchanged: public `OpenApiImport::build()` versus schematic-definitions' public `openai::define_openai_api()`, followed by public `validate_rust_client()`. It enables the explicit policy on the direct API and compares shared endpoint IDs. A temporary example performed the comparison and was removed after execution. Exact message differences due to renamed references, union spelling, or the first composition named are not counted as lost constraints.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R3-F1.1–.7 | All seven free-form rebuilding positions from the earlier finding | Generic anyOf, oneOf, multi-member allOf and not; nullable/single-member wrappers | Compiled fixture retains the rules and the expected diagnostics/plans | Clean |
| R3-F1.8/.10/.A | Reference, direct body/multipart and supported-wrapper controls | Ordinary references and supported traversal, both policies | Supported rules remain enforced; generic rules are rejected or disclosed | Clean |
| R3-F1.9/.B/.C | Import → emission → registration → validation/plans/docs; baseline reader | Fixture's complete composition set and repository-input literal | 62 matching errors or warnings, 62 matching disclosure lines; baseline passes | Clean |
| R3-F1.11 | Nominal component-root union rebuilding | MixedUnion fixture, plus shipped unions | Member rules survive registration; fixture contributes the expected findings | Clean |
| R3-F1.12 | Root allOf rebuilding | MixedAll fixture with referenced and inline members | Both members' rules survive; direct/rebuilt fixture diagnostics agree | Clean for rule retention |
| R3-F1.13 | Flattened member copies → validator and request rustdoc | OpenAI CreateChatCompletion and CreateResponse through CreateModelResponseProperties | 64 → 72 and 254 → 262 warnings respectively; copied temperature/top-p/top-logprobs rules appear at additional property paths | One disclosure per underlying rule, or an explicit documented limitation on diagnostic equality |
| R3-F1.14 | Existing importer overlay pins | CreateRealtimeCall's session field; API version | Shipped session remains a string field, unlike the fresh import's JSON field; its 25 → 0 warning difference follows that earlier pinned choice | Clean under the existing decision; no new rebuilding loss |

The public comparison covered all shared endpoints. Other warning-count differences were not established beyond the two duplicate cases and the pinned realtime field. The compiled fixture covers the seven free-form positions and the inline-member/root-union controls independently of the catalog.

**Recommended repair:** document that flattened composed models can repeat disclosures and can check logically applicable flattened member rules, and record this narrowly bounded departure from exact diagnostic equality in the implementation log. The current [import documentation](../../docs/io/import-openapi.md:681) already explains the copied checks; make the warning-duplication consequence explicit. Removing the copies with a new registration-time schema transform is disproportionate to this polish issue. No author decision or runtime composition evaluator is needed to finish that documentation repair.

## Blocked Findings

None. The October 24 author ruling resolved the previous conflict. The remaining documentation polish does not require human approval.

## Recurrence

R3-F1 belongs to the preservation class first reported as R1-F1 in review 1, extended by R2-F2 in review 2, and enumerated as R3-F1 in reviews 3–5. The previous missing-composition instances are now repaired. Remaining child .13 is a **sibling introduced by the latest repair's root-allOf retention**, already discovered in its ledger: that repair should also have compared flattened referenced members and their validator/rustdoc consumers. It did not remove their duplicated copies. This review carries that bounded list forward and lowers its priority to reflect its actual harm; it does not reopen the accepted opt-in or request additional composition evaluation.

R1-F3 is now fixed under the amended specification and is not carried as a recurring open finding.

## Input Robustness Matrix

Default outcome: malformed supported constraint metadata produces a located error before output publication. Existing exceptions remain absent optional metadata, valid empty patterns, `uniqueItems: false`, enum/const JSON data, preserved unenforced imported patterns and annotation-only formats. The new Boolean opt-in adds one exception: absence means false; explicit false is valid and omitted when serialized; true enables only composition warnings.

| Validator or loader | Public-result evidence |
| --- | --- |
| New API opt-in Boolean | `definition_serde` checks absent, true, false, explicit null, string, number and array. Null and wrong types fail; Boolean values round-trip. `definition_bundles` checks its loader wiring and malformed values. Collection-element shapes do not apply to this scalar. |
| Shared endpoint/component rule validators | Reran existing absent/null/type/element/empty matrices and wiring across parameters, bodies and forms. Contradictions remain errors even inside opted-in compositions. |
| OpenAPI JSON/YAML loaders | Reran duplicate-key, trailing/invalid-content, syntax, null, exclusive-bound and exact-number tests, plus the CLI positive/located-error/no-written-files checks. The repair adds no loader. |
| Definition bundle loader | Reran positive round trips, absent optional metadata, duplicate keys, malformed values, wrong roots and trailing content. |
| Composition traversal/refusal classifier | Reran both-policy body/multipart matrix for root/nested unions, multi-member allOf, not and map values; positional items and all 11 other untraversed-keyword controls remain errors, including inside compositions. Repeated references retain the stricter reason. |

Searched the changed paths for `serde(default)`, `Option`, `filter_map`, `unwrap_or_default()` and `.ok()`. The new Boolean's default is intentional, and its typed deserializer rejects null. Reviewed loose schema conversions with their preceding validation and the rebuilding contracts. No additional malformed-input finding was established.

## Verification

Level 1 is appropriate for these requirements: they concern Rust APIs, file loading, generation, serialized values and mock HTTP transport. No requirement here depends on real-terminal rendering or keyboard encoding, so Level 2/3 tests would not add relevant proof.

| User-facing requirement | Verification level and evidence |
| --- | --- |
| Fluent metadata, default refusal and explicit opt-in | L1 builders/serialization/bundles, generator policy matrices and output publication tests |
| Import/export and rebuilt definitions preserve value rules | L1 direct fixture meaning comparison and fresh emitted Rust compiled and assembled through registration |
| Generated requests validate what is sent and refuse before transport | L1 fresh generated clients against loopback mock HTTP, checking build/await/full/direct/raw paths, serialization, mutation and zero sends |
| Safe paths, exact arithmetic, null/equality/uniqueness semantics | L1 runtime evaluator/report tests and generated-client secret/map-key assertions |
| Text bodies, forms and multipart JSON | L1 freshly compiled clients asserting rejection before encoding and valid transmitted data |
| Pager defaults, overrides and changes during traversal | L1 runtime pager-rule tests plus generated-client pagination regressions |
| Existing response decoding and client controls | L1 generated-schema tests and fresh-output guards; structurally decodable out-of-bound replies remain accepted |
| Composition warnings and input documentation | L1 diagnostic/rustdoc assertions plus public direct/rebuilt OpenAI comparison; duplication limitation recorded above |

New tests are in declared targets: schematic-gen's `l1` root imports the modified modules, and schematic-define's integration targets compile with the OpenAPI feature used by the area recipe. The fresh-compilation tests are explicitly selected by `test-fresh-output`, which `just test` invokes; their `slow_` names do not strand them. The tier-coverage command was also run. Cross-OS execution evidence is left to CI and does not determine this review's readiness.

Commands completed on this host:

- `just test` from `schematic/`: exit 0; 3,171 main L1 tests, 17 fresh-output guards and one runtime compile-fail guard passed. The main run reported 23 skips; the fresh-output guards are separately selected by the recipe, not inferred from those skips.
- `just test-schema` from `schematic/`: exit 0; all 154 generated-client tests passed.
- `just check-tier-coverage schematic` from the repository root: exit 0; zero stranded tests.
- Public direct-import/rebuilt-catalog comparison: exit 0; reproduced the two eight-warning increases described above. Temporary source was removed.

Lint and doctests were not rerun for this review's Markdown-only changes; their earlier repair-log results are not presented as independently rerun evidence. Several existing Cargo-compilation guards and unrelated TLS tests exceed five seconds; this review does not establish a new test-speed regression from the repair.

## Observations

None.
