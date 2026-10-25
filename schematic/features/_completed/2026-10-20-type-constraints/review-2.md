---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F3 (high): Unenforceable request rules do not consistently block generation"
  - "R2-F1 (high): Rebuilding free-form schemas drops declared-property exemptions"
  - "R2-F2 (high): Rebuilding free-form schemas drops referenced constraints"
observations: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-23T19:23:00-07:00"
spec: 2026-10-20-type-constraints/spec.md
implemented: false
description: "A **feature** review of `2026-10-20-type-constraints/spec.md`"
feature: 2026-10-20-type-constraints/review-2.md
previous: 2026-10-20-type-constraints/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-20-type-constraints/log.md
implemented_by: claude/opus
next: 2026-10-20-type-constraints/review-3.md
---

# Review of type constraints, iteration 2

The feature is **not ready for production**. One previous high finding is partially repaired, and two new high findings affect rebuilt imported definitions. The original required instances of the other four findings are fixed. The new findings distinguish additional broken positions from those original repairs.

The specification, plan, implementation logs, previous review, and repair's Finding Ledger were read before verification. The working tree already contained the implementation and repairs. This review changes review documents and specification frontmatter only; temporary reproduction tests were removed after execution.

## Previous Finding Verification

Every child in the repair ledger is included below, including the additional instances the repair discovered. “Verified fixed” applies to those enumerated instances, not every possible sibling. The prior review's observation is not carried as a finding.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Rebuilding imported definitions loses supported constraints | R1-F1.1–R1-F1.8: map values, free-form root rule, format, map enum, model const, flattened extra-value rule, occurrence const beside a reference, inline/named array-item const. R1-F1.9: primitive/renamed/alias/array/parameter/form controls. Repair ledger R1-F1.10–R1-F1.11: baseline and export-fidelity consumers. | **verified fixed** | Inspected field/container/child rule emission in schematic-gen's `model_gen.rs`. Reran `slow_definitions_out_rebuilds_every_rule_through_registration`: compiles emitted definitions, assembles them, compares the listed positions, and validates the rebuilt API. Full L1 also passed baseline, definitions-out, and schema-fidelity tests. | No original instance remains. Newly found reference loss in other free-form positions is R2-F2. |
| R1-F2: Invalid constraints are accepted or discarded instead of rejected | R1-F2.1: malformed stored components and bundle reload; R1-F2.2: contradictory query/path/header/cookie/body/form/component imports; R1-F2.3–R1-F2.4: endpoint metadata and numeric/null loader controls. Repair ledger R1-F2.5–R1-F2.7: dropped-keyword disclosure, duplicate pattern diagnostics, incompatible component rules. | **verified fixed** | Inspected shared component validation and its API-build, bundle, and three output-validator consumers. Full L1 passed component/endpoint shape matrices, bundle reload, all-input contradiction, loader, dropped-keyword, and output-validation tests. Invalid patterns now produce errors rather than unsupported-pattern warnings. | None of the required instances remains. |
| R1-F3: Unenforceable request rules do not consistently block generation | R1-F3.1–R1-F3.2: root `anyOf`/`oneOf`; R1-F3.3: positional items; R1-F3.4: nested union/`allOf`/`not`; R1-F3.5: ordinary items/nullable alternative; R1-F3.6: body/form parity; repair ledger R1-F3.7: importer disclosure. | **partially fixed** | Positional items now fail, including Schemars' `prefixItems`. Existing tests and an additional public-validator sweep confirm errors for unsupported nested traversal at both body/form entry points. Root unions still warn. The shipped-catalog test permits exactly 42 unchecked rules on OpenAI `CreateResponse`. | R1-F3.1 and R1-F3.2 remain open. The ledger's author-decision blocker is not supported by the specification; see below. |
| R1-F4: Pruning declared properties applies extra-property rules to valid fields | R1-F4.1: inactive declared property; R1-F4.2–R1-F4.4: runtime membership, pure map, active property controls; R1-F4.5: referenced nested object and multipart JSON. | **verified fixed** | Inspected corrected plan pruning. Reran the fresh-output fixture containing `a_declared_property_is_never_checked_by_the_extra_value_plan`: root, referenced nested, and multipart objects pass while actual extra entries still fail with redacted paths. Runtime map/property and generator pruning tests passed. | No original instance remains. An earlier rebuilding stage contains a new sibling, R2-F1. |
| R1-F5: Nullable plans bypass explicit enum and const restrictions | R1-F5.1–R1-F5.2: nullable generated/runtime enum and const; R1-F5.3–R1-F5.4: null alternative, null-member and kind-only controls; R1-F5.5: rebuild null membership; repair ledger R1-F5.6: overlapping export and null-rejecting sibling. | **verified fixed** | Runtime checks its own whole-value assertions before skipping a nullable reference. Reran null enum/const tests, null-alternative controls, generator nullable-plan tests, rebuild membership tests, and public export tests including `a_folded_null_alternative_keeps_admitting_null_and_a_rejecting_sibling_keeps_rejecting_it`. Inspected removal of `admit_null` and refreshed source memberships. | None of the required instances remains. |

## Unblocked Findings

### R1-F3 (high): Unenforceable request rules do not consistently block generation

**Authority — spec, “Supported schema traversal”:** “A constrained request beneath an unsupported composition or unresolved reference gets an actionable generation error.” “Definition errors versus unsupported rules” permits pattern/format exceptions, not a root-union exception. **Defect class:** unsupported traversal is downgraded to a warning based on schema position, permitting clients with unenforced supported rules.

In schematic-gen, [composition_findings](../../gen/src/codegen/value_rules/mod.rs:710), which classifies request rules for generation, still assigns `Warning` to unsupported unions at component roots. This is the unchanged instance from review 1. A public `validate_rust_client` probe moved `maxLength: 3` between positions, using both a JSON body and a multipart JSON field.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F3.1 | Root union component, JSON body and multipart field | `anyOf: [{type: string, maxLength: 3}, {type: integer}]` | Warning at both entry points | Generation error at both entry points |
| R1-F3.2 | Root union component, JSON body and multipart field | Same rule under `oneOf` | Warning at both entry points | Generation error at both entry points |
| R1-F3.3 | Positional array traversal | Same rule under `items: [...]` and `prefixItems: [...]` | Error at both entry points | Clean, repaired |
| R1-F3.4 | Unsupported compositions | Same rule under nested-property union, two-member `allOf`, `not`, and union in map values | Error at both entry points | Clean |
| R1-F3.5 | Supported traversal controls | Ordinary item object; constrained member beside null alternative; one-member reference `allOf` | Plans retain rules without unsupported-traversal findings | Clean |
| R1-F3.6 | Diagnostic consumers | Body/form policy and shipped-catalog validator | Same warning policy; catalog accepts 42 OpenAI `CreateResponse` omissions | Reject unsupported independent rules; update the catalog expectation |
| R1-F3.7 | Import disclosure | Parameter subschema keywords and positional items | Dropped subschemas disclosed; positional items fail parsing | Clean, repaired |
| R1-F3.8 | Other unsupported subschema positions | `additionalItems`, `contains`, `unevaluatedItems`, `patternProperties`, `propertyNames`, `unevaluatedProperties`, `dependentSchemas`, schema-form `dependencies`, `if`, `then`, `else` | Same rule produces an error at both entry points for every keyword | Clean |

The repair log marks the first two rows blocked because rejecting them prevents OpenAI `CreateResponse` generation. Review 1 already discussed this consequence and explicitly rejected the warning exception as unauthorized. No new author ruling or changed prerequisite exists. The specification chooses generation failure and excludes generic union evaluation. Apply that failure policy; this finding does not request a union evaluator, ABI change, or speculative protection layer. A later author change to that contract would be a separate decision. No author input is required to implement the current contract.

### R2-F1 (high): Rebuilding free-form schemas drops declared-property exemptions

**Authority — spec, “One source for model rules”:** “Imported Rust definitions must rebuild equivalent rules,” including inline schemas; direct import is insufficient if emitted definitions and assembly lose the rule. “Supported schema traversal” requires object properties and map values. **Defect class:** discarding unconstrained declared names changes which values an extra-property rule constrains.

In schematic-gen, [free_form_rules](../../gen/src/model_gen.rs:478), which rebuilds imported schemas for fields represented by `serde_json::Value`, discards a property when recursively collected rules are empty. That removes its declared-name exemption from `additionalProperties`. The repaired final plan pruner cannot restore a name already lost during rebuilding.

The reproduction extends the real preservation fixture with this mixed object at each position below:

```yaml
properties:
  label: {type: string}
  code: {type: string, maxLength: 2}
additionalProperties: {type: string, maxLength: 3}
```

Submit `{"label":"long declared label","code":"ok"}`. Public import, `run_import` with `definitions_out`, compilation, and `module_group()` produce the rebuilt API. Clients freshly generated and compiled from direct and rebuilt APIs run the same requests through both `validate()` and `EndpointSpec::into_parts()`. The direct client accepts every case; the rebuilt client rejects every case with a redacted extra-key `maxLength` violation.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F1.1 | Free-form field root | Mixed object at `Item.free_value` | Valid label rejected by validation and preparation | Accept |
| R2-F1.2 | Recursive property rebuilding | Mixed object at `free_value.inner` | Valid nested label rejected | Accept |
| R2-F1.3 | Recursive item rebuilding | Mixed object under free-form `items` | Valid array member rejected | Accept |
| R2-F1.4 | Recursive map-value rebuilding | Mixed object under free-form `additionalProperties` | Valid map value rejected | Accept |
| R2-F1.5 | Map-valued model field | Mixed object as `attributes` map value | Valid map value rejected | Accept |
| R2-F1.6 | Inline object array item | Mixed object as `entries.items` | Valid array member rejected | Accept |
| R2-F1.7 | Flattened extra-properties value | Mixed object as a nominal struct's extra value | Valid extra object rejected because its declared label is lost | Accept |
| R2-F1.8 | Active declared property and actual extra key | `code: "long"`; undeclared `extra: "long"` | Direct/rebuilt clients reject both; code has a named path and extra key is redacted | Clean controls |
| R2-F1.9 | Nominal struct's own declared names | Existing `Extras.id: "long declared id"` beside constrained extras | Direct/rebuilt clients accept | Clean; Rust-derived membership survives |
| R2-F1.10 | Downstream classification and preparation | Rebuilt component → generated plan → runtime object lookup → `into_parts()` | Missing name becomes an extra key and prevents preparation | Preserve membership before generation |

The deciding runtime consumer is schematic-runtime's [object evaluator](../../runtime/src/validate/eval.rs:211): it selects a declared plan by name, otherwise the extra-value plan, and derives the displayed path from that decision. Its behavior is correct with complete membership. Schematic-gen's [field/container/child rebuilding](../../gen/src/model_gen.rs:298) feeds the same lossy helper for all seven failing positions. Alias and optional dispatch forward to these branches; they provide no alternate membership source. Ordinary nominal properties and the repaired final-plan pruning are clean controls.

Retain declared names wherever extra-value rules depend on them, even when child rule sets are empty. Add actual request acceptance/refusal to the compiling round-trip fixture: its keyword-only comparison does not record unconstrained membership. This is a small rebuilding correction, not general JSON Schema evaluation.

### R2-F2 (high): Rebuilding free-form schemas drops referenced constraints

**Authority — spec, “Supported schema traversal”:** “Validation must reach root values, object properties, array items, map values, local component references, and recursive models.” “One source for model rules” also requires equivalent rebuilt rules for inline schemas and nested-only components. **Defect class:** reconstruction omits reference edges, making supported rules unreachable without a generation diagnostic.

Schematic-gen's [free_form_rules](../../gen/src/model_gen.rs:478) copies direct rules and inline children but never preserves or resolves `$ref`. A free-form object's property `owner: {$ref: "#/components/schemas/Owner"}` loses `Owner.properties.name.maxLength: 10`. This is ordinary supported object traversal; no union is involved. The new documentation records the limitation, but no specification or author ruling authorizes this exclusion.

The compiled reproduction uses the same fixture and positions as R2-F1, adds that referenced property, and submits `{"owner":{"name":"longer than ten"}}`. The existing `Owner` component remains registered with its rule after rebuilding. The missing edge, rather than missing registration or malformed metadata, prevents traversal. Both `validate()` and `into_parts()` reject every direct-import control and accept every rebuilt counterpart.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F2.1 | Free-form field root | Referenced property inside `Item.free_value` | Invalid referenced name accepted | Reject with `maxLength` |
| R2-F2.2 | Recursive property rebuilding | Referenced property inside `free_value.inner` | Invalid referenced name accepted | Reject |
| R2-F2.3 | Recursive item rebuilding | Referenced property in free-form array item | Invalid referenced name accepted | Reject |
| R2-F2.4 | Recursive map-value rebuilding | Referenced property in free-form map value | Invalid referenced name accepted | Reject |
| R2-F2.5 | Map-valued model field | Referenced property in `attributes` map value | Invalid referenced name accepted | Reject |
| R2-F2.6 | Inline object array item | Referenced property in `entries.items` | Invalid referenced name accepted | Reject |
| R2-F2.7 | Flattened extra-properties value | Referenced property in a nominal struct's extra value | Invalid referenced name accepted | Reject |
| R2-F2.8 | Nominal reference control | Existing typed `Item.owner` and occurrence constant | Compiled preservation checks retain occurrence and target rules; target stays separately registered | Clean |
| R2-F2.9 | Downstream acceptance decisions | API closure check, Rust generator validation, generated validation/preparation | No dangling reference remains to diagnose; generation succeeds and violating data passes | Preserve supported traversal and enforce its rules |

Preserve the reference and required registration, or rebuild its supported meaning while retaining recursion. Keep compilation bounded by reusing registered components as specified. Do not expand arbitrary unions or create another editable rule source. A compiled fixture with an ordinary referenced property protects this case without a synthetic payload matrix. Update the newly documented exclusion alongside the repair.

## Blocked Findings

None. R1-F3 remains actionable under the existing specification. Its repair-log label does not establish a new design dependency.

## Recurrence

- **R1-F3 repeats review-1.md, R1-F3:** unchanged root-component union severity policy. Positional-item repairs are real; the root-union instances were deliberately left unchanged. This sweep covers both entry points, all listed unsupported subschema keywords, supported controls, and the live catalog consumer.
- **R2-F1 repeats review-1.md, R1-F4's classification class:** a new sibling introduced in rebuilding machinery used to repair R1-F1. Final-plan pruning is fixed. That repair should also have checked earlier pruning in `free_form_rules`, since membership is lost before the corrected plan pruner runs. All seven rebuilding positions and downstream classification/preparation decisions are listed above.
- **R2-F2 repeats review-1.md, R1-F1's preservation class:** a sibling the earlier sweep missed, now visible in the repaired rebuilding path. Named-field and inline-item tests protected direct occurrence rules but missed references nested inside free-form values. All seven positions now have direct-versus-rebuilt compiled reproductions.

These cases have a finite boundary: declared membership and ordinary local reference edges under the traversal the specification names. They do not ask for generic composition evaluation or a stronger validation guarantee.

## Input Robustness Matrix

The JSON bundle/OpenAPI loaders, endpoint constraint deserializer, and stored-component validator were inspected separately. The default for malformed supported metadata is a located error before output. Exceptions remain absent optional keywords, empty patterns, `uniqueItems: false`, valid unsupported imported patterns preserved with a warning, annotation-only formats, and whole JSON values including null inside valid enum/const data.

| Validator or loader | Evidence inspected and rerun |
| --- | --- |
| Endpoint metadata | `serialized_field_shapes_per_validator`: absent/control, null, wrong type, empty and contradictory values through the public bundle result; enum members stay JSON data |
| Stored component rules | `component_field_shapes_per_validator`: counts, numeric/exact limits, pattern/format, uniqueness, enum/const, items, nested location, and type-compatibility cases; `a_reloaded_bundle_refuses_a_malformed_component_rule_of_each_family` proves public bundle wiring |
| API and output consumers | Registered contradictory/mistyped models fail API build; edited components fail Rust, OpenAPI, and Postman validation in `a_malformed_component_rule_blocks_every_output` |
| OpenAPI document loader | Control/absence, YAML/JSON duplicate keys, null, empty enum, wrong-type length, infinite/large-exponent bounds, Boolean exclusives, and out-of-range flow-style bound tests |
| Bundle document loader | Existing duplicate-key, wrong-root, and trailing-content checks; CLI wiring checks locations and writes nothing |
| Import routing | Contradictions fail at query/path/header/cookie/body/form/component locations in permissive and strict modes; incompatible rules are disclosed individually without erasing valid siblings |

The requested smell search found presence-aware optional deserializers; defaulted Booleans whose absence means false; strict schema reads guarded by shared validation; numeric conversion `.ok()` branches choosing exact-text preservation or refusal; and type-name filtering outside the supported keyword-value reader. None reproduces a remaining R1-F2 instance. The two new findings concern schema writers/projections, not new configuration shape cells.

## Verification

All feature requirements need **Level 1**: in-process APIs, hermetic CLI wiring, compiled generated requests, and local mock HTTP servers. No new terminal rendering, keyboard, browser, or live-provider behavior requires Level 2 or Level 3.

| Requirement | Appropriate level and evidence |
| --- | --- |
| Unicode lengths, patterns, exact arithmetic, deep equality, deterministic redacted violations | Level 1: runtime decimal/evaluator/report/pattern and definition pattern-table tests |
| Authored/serialized metadata rejection | Level 1: public API-build/bundle/import/output checks and robustness cases above |
| Import/export and emitted definitions through compilation/registration | Level 1: preservation fixtures and compiling guard; direct/rebuilt request probes expose R2-F1/R2-F2 |
| Unsupported traversal blocks generation | Level 1: public-validator sweep and catalog tests; R1-F3 identifies the remaining failure |
| Validation, build, await, full, direct/raw send, mutation, serialization and zero sends on refusal | Level 1: fresh-output fixture/mock-server tests; new probes exercise actual `into_parts()` for rebuilt requests |
| Effective pager sizes, defaults, traversal changes and conflicting bounds | Level 1: runtime pager rules, generated pager fixtures, and definition conflict tests |
| Replies outside value bounds still decode | Level 1: committed-schema `value_rules` test rerun against its mock server |
| Tests compiled and selected | Generator modules declared by consolidated `l1/main.rs`; declared define/schema targets; tier audit reports zero stranded tests |

Executed on this macOS host:

- `just test` in `schematic/`: **3,146 passed**, 23 skipped; fresh-output guard **17 passed**; runtime compile-fail guard **1 passed**.
- `just test-schema value_rules`: **1 passed**, proving response decoding on committed clients.
- `just check-tier-coverage schematic`: **0 stranded tests**.
- Additional public root-union/unsupported-position sweep: **1 passed**, asserting the warning/error results at both body/form sites.
- Additional compiled direct/rebuilt sweep: **1 passed**, with seven membership cases, seven reference cases, both `validate()` and `into_parts()`, and active-property/extra-key/nominal controls. Assertions establish defects rather than accepting them as the intended contract.

Temporary probes used the existing offline fresh-output harness and declared test modules. Recipe argument quoting rejected the targeted filter, so the equivalent nextest command was used. Initial probe iterations needed a borrow correction and removal of inherited assertions for positions intentionally changed by the fixture edit; the final complete sweep passed. Reproduction sources remain at `/tmp/type-constraints-review-2-preservation-probe.rs` and `/tmp/type-constraints-review-2-union-probe.rs` on this host; repository test sources were restored.

The full run reported 11 slow main-suite tests. This loaded local run alone does not establish a new constraint-feature defect; the temporary compiling probe uses the existing separate compilation guard. No formatter, production-code fix, commit, live-provider call, or lifecycle move was performed. Cross-OS evidence remains CI work and does not determine readiness here.

## Observations

None added in this iteration.
