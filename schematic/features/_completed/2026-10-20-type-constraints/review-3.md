---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F3 (high): Unenforceable request rules do not consistently block generation"
  - "R2-F2 (medium): Rebuilding free-form schemas drops referenced constraints"
  - "R3-F1 (high): Rebuilding free-form compositions loses supported traversal and generation errors"
observations:
  - "Registration order changes are normalized by the baseline test"
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-23T21:09:25-07:00"
spec: 2026-10-20-type-constraints/spec.md
implemented: false
description: "A **feature** review of `2026-10-20-type-constraints/spec.md`"
feature: 2026-10-20-type-constraints/review-3.md
previous: 2026-10-20-type-constraints/review-2.md
log: /Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-20-type-constraints/log.md
implemented_by: claude/opus
next: 2026-10-20-type-constraints/review-4.md
---

# Type constraints review, iteration 3

The feature is **not production ready**. Root-component unions still permit unenforced rules, and rebuilding free-form compositions loses both supported checks and required generation errors. The declared-property repair and the original ordinary-reference repair are effective. Two unusual alias cases remain nonblocking.

The specification, plan, both earlier reviews, implementation log, and repair log's Finding Ledger were read. Findings below distinguish unchanged defects from newly enumerated siblings. No accepted author decision is reopened.

## Previous Finding Verification

“Required instances” includes the repair ledger's additional children. A successful test is evidence only for the positions and decisions its assertions inspect.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Rebuilding imported definitions loses supported constraints | R1-F1.1–R1-F1.8: map values, free-form root, format, map enum, root const, flattened extra-value rule, occurrence const, inline/named array items; .9: primitive/renamed/alias/parameter/form controls; .10–.11: baseline and export consumers | **verified fixed** | Reran the compiling `constraint_preservation::slow_definitions_out_rebuilds_every_rule_through_registration`, model-generation tests, definition baselines, and schema-fidelity tests. Inspected field/container/child emission. All enumerated original placements remain preserved. | None of these instances. Newly enumerated composition placements are R3-F1. |
| R1-F2: Invalid constraints are accepted or discarded instead of rejected | R1-F2.1: stored components/bundle reload; .2: every imported input location; .3–.4: endpoint metadata and numeric/null loader controls; .5–.7: dropped-keyword disclosure, duplicate pattern reporting, incompatible component rules | **verified fixed** | Full L1 reran field-shape matrices, API-build and bundle wiring, contradictory imports, output validation, and loader tests. Inspected shared component validation and its consumers. | None of these instances. |
| R1-F3: Unenforceable request rules do not consistently block generation | R1-F3.1–.2: root unions; .3: positional items; .4: nested compositions; .5: supported traversal controls; .6: body/form/catalog consumers; .7: import disclosure; .8: other unsupported subschema positions | **partially fixed** | Public `validate_rust_client` sweep reproduced warnings for root `anyOf`/`oneOf` at both JSON-body and multipart-field entry points. All other unsupported-position probes produced errors. Existing supported-control and disclosure tests passed. Catalog still expects 42 omissions. | Original .1, .2, and .6 remain under R1-F3 below. No author ruling changes the required error policy. |
| R1-F4: Pruning declared properties applies extra-property rules to valid fields | R1-F4.1: inactive property; .2–.4: runtime membership, pure map, active-property controls; .5: referenced nested object and multipart JSON | **verified fixed** | Reran pruning, runtime evaluator, and fresh-output tests. Plans retain declared names while an extra-value plan survives; generated requests accept declared fields and reject actual extra values. | None of these original instances. |
| R1-F5: Nullable plans bypass explicit enum and const restrictions | R1-F5.1–.2: generated/runtime enum and const; .3–.4: null-alternative, null-member, kind-only controls; .5: rebuilt membership; .6: export overlap and null-rejecting sibling | **verified fixed** | Reran runtime null tests, generator nullable-plan tests, rebuilt enum/const tests, and nullable OpenAPI export tests. Own enum/const assertions precede nullable-reference skipping. | None of these original instances. R3-F1 concerns losing entire wrappers during rebuilding, rather than incorrect evaluation of a retained nullable plan. |
| R2-F1: Rebuilding free-form schemas drops declared-property exemptions | R2-F1.1–.7: all seven free-form positions; .8–.9: active-property, actual-extra, nominal-name controls; .10: generated plans and preparation | **verified fixed** | Inspected `free_form_rules`' extra-value-dependent membership retention. The compiled acceptance test ran all seven positions through `validate()` and `into_parts()`, with named and redacted-path refusal controls. Its direct/rebuilt plan assertions passed. | None. |
| R2-F2: Rebuilding free-form schemas drops referenced constraints | R2-F2.1–.7: seven ordinary-reference positions; .8: nominal reference; .9: closure/generation/preparation; .10: documentation; .11: direct items/extra-value references and adjacent rule/nullability; .12: compositions; .13: plain-alias and top-level references; .14–.15: recursive alias and alias request body | **partially fixed** | Original .1–.9 and added .11/.13 pass the compiled acceptance and model-generation tests; generated helpers register referenced models. .10 now accurately documents ordinary references and exclusions. Independently compiled edited fixtures confirm .12 and .14–.15 still lose rules. | Original ordinary-reference instances are fixed. Newly enumerated composition sibling .12 is R3-F1; remaining alias children .14–.15 stay under R2-F2 below, reduced to medium because they require unusual alias schemas with no shipped occurrence. |

## Unblocked Findings

### R1-F3 (high): Unenforceable request rules do not consistently block generation

**Authority — spec, “Supported schema traversal”:** “A constrained request beneath an unsupported composition or unresolved reference gets an actionable generation error.” “Definition errors versus unsupported rules” allows pattern/format exceptions, not a root-union exception. **Defect class:** diagnostic severity depends on schema position, allowing supported request rules to go unenforced.

In schematic-gen, [composition_findings](../../gen/src/codegen/value_rules/mod.rs:710), which decides whether request constraints block Rust generation, still assigns warnings to root-component unions. This is unchanged from both earlier reviews. A public-validator reproduction starts with the real preservation fixture, changes the request component's schema, and uses that component as both a JSON body and a multipart JSON field.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F3.1 | Root component, both input entry points | `anyOf: [{type: string, maxLength: 3}, {type: integer}]` | Warning | Generation error |
| R1-F3.2 | Root component, both input entry points | Same rule under `oneOf` | Warning | Generation error |
| R1-F3.3 | Positional array traversal | `items: [...]`, `prefixItems: [...]` | Error at both entry points | Clean |
| R1-F3.4 | Unsupported compositions | Nested `anyOf` and `oneOf`, two-member `allOf`, `not`, map-value union | Error at both entry points | Clean |
| R1-F3.5 | Supported traversal | Ordinary item object, nullable alternative, one-member `allOf` | Existing tests retain plans without unsupported-traversal findings | Clean |
| R1-F3.6 | Generation policy consumers | JSON body, multipart field, shipped-catalog expectation | Both entry points warn; catalog accepts 42 omissions on OpenAI `CreateResponse` | Reject these rules consistently; update the catalog expectation |
| R1-F3.7 | Import disclosure | Dropped parameter subschemas and positional items | Disclosure/parse-error tests pass | Clean |
| R1-F3.8 | Other unsupported traversal | `additionalItems`, `contains`, `unevaluatedItems`, `patternProperties`, `propertyNames`, `unevaluatedProperties`, `dependentSchemas`, schema `dependencies`, `if`, `then`, `else` | Error for every keyword at both entry points | Clean |

The repair log now reports that faithfully preserving free-form compositions exposes failures on additional OpenAI endpoints. That is useful new scope evidence, but does not change the specified outcome: fail generation for unsupported constrained requests. Implementing a union evaluator, removing endpoints, or granting a warning exception is unnecessary to implement that policy and is not requested here. The severity change remains reversible; there is no blocking author decision under this run's rules.

### R2-F2 (medium): Rebuilding free-form schemas drops referenced constraints

**Authority — spec, “One source for model rules”:** “Imported Rust definitions must rebuild equivalent rules,” explicitly including aliases and inline schemas; “Supported schema traversal” includes recursive models. **Defect class:** plain aliases lose nested rule reachability when rebuilt as inline Rust container types.

The original seven ordinary-reference instances are fixed. The remaining instances are the repair ledger's R2-F2.14 and R2-F2.15. In schematic-gen, [free_form_reference](../../gen/src/model_gen.rs:655) stops recursive alias expansion, while [body_constraints](../../gen/src/model_gen.rs:510) lowers an alias body to metadata that cannot carry nested object schemas. [Definition generation](../../gen/src/definition_gen.rs:272) applies the latter to JSON bodies.

A compiled reproduction extends the preservation fixture with `Forest`, an array alias whose inline object items have `label.maxLength: 11`, and `RecursiveForest`, whose item objects also reference that alias through `children`. It imports, emits definitions, compiles them, and reads the API assembled by their public `module_group()`.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F2.14 | Recursive plain alias reached through an object field | Array items carry `label.maxLength: 11` and `children: {$ref: RecursiveForest}` | Initial expansions keep rules, then a deeper `children.items` becomes `{}`; no recursive component remains | Preserve recursive reachability, or explicitly reject the unsupported rebuilding case |
| R2-F2.15 | JSON body naming a plain array alias | `Forest`, `maxItems: 3`, item objects with constrained `label` | Rebuilt endpoint is `Json(Array(Primitive(Json)))`; metadata keeps only `maxItems: 3`, with `items: None`; nested rule disappears | Preserve nested rule reachability, or explicitly reject this case |
| R2-F2.13 | Nonrecursive alias/free-form reference controls | Plain scalar alias and top-level reference | Model-generation tests retain rules; compiled fixture retains ordinary references | Clean |
| R2-F2.8 | Nominal typed model reference | Existing `Item.owner` | Target and occurrence rules remain separate and present | Clean |
| R2-F2.9 | Downstream decisions | Rebuilt alias schemas → generated plans → request preparation | Missing deeper schemas leave no rule for validation or preparation to enforce | Diagnose or preserve the missing traversal |

No shipped definition exercises the two failing alias shapes. The current import documentation already states these limitations. A general alias-registration design or new body-schema carrier would be large relative to the demonstrated harm; prefer keeping this bounded limitation documented or adding a focused rejection, rather than expanding the feature's public schema model in this repair cycle. These cases do not block readiness.

### R3-F1 (high): Rebuilding free-form compositions loses supported traversal and generation errors

**Authority — spec, “Supported schema traversal”:** “Nullable wrappers and the equivalent nullable alternatives emitted by Schemars must remain supported”; unsupported constrained compositions must produce an actionable generation error. “One source for model rules” requires equivalent rebuilt rules for inline schemas. **Defect class:** rebuilding removes schema edges before the generator can enforce or reject their reachable rules. This is a newly enumerated sibling of R2-F2, recorded as R2-F2.12 in its repair ledger.

In schematic-gen, [free_form_rules](../../gen/src/model_gen.rs:616), which rebuilds imported fields represented by free-form JSON, copies properties, items, extra values, and references, but discards every composition. This includes supported nullable `anyOf` and one-member `allOf`, not only generic unions. A direct reference now works; wrapping that same reference loses it.

The reproduction copies the real preservation fixture and adds these properties beside each of its seven mixed-object `owner` references:

```yaml
nullable_probe:
  anyOf:
    - {$ref: '#/components/schemas/Owner'}
    - {type: string, nullable: true, enum: [null]}
allof_probe:
  allOf: [{$ref: '#/components/schemas/Owner'}]
```

`Owner.name` has `maxLength: 10`. It also adds a string `maxLength: 11` under generic `anyOf`, `oneOf`, two-member `allOf`, and `not`. Public import retains every probe; `run_import`, compilation, and `module_group()` rebuild them as empty schemas. Direct generation retains supported reference plans; rebuilt generation emits empty property plans. For the generic compositions, public Rust validation reports **56 direct-import errors and zero rebuilt errors** across the two existing endpoints that use `Item`.

Each failing row below was tested with all six shapes: nullable alternative, one-member reference wrapper, generic `anyOf`, generic `oneOf`, two-member `allOf`, and `not`.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R3-F1.1 | Free-form field root, `Item.mixed_value` | Six wrapper/composition properties | All become `{}` | Preserve supported wrapper checks; retain enough unsupported structure to fail generation |
| R3-F1.2 | Nested property, `mixed_value.inner` | Same six | All become `{}` | Same |
| R3-F1.3 | Free-form items, `mixed_value.items` | Same six | All become `{}` | Same |
| R3-F1.4 | Free-form extra values, `mixed_map.additionalProperties` | Same six | All become `{}` | Same |
| R3-F1.5 | Map-valued model field, `mixed_attributes.additionalProperties` | Same six | All become `{}` | Same |
| R3-F1.6 | Inline object array items, `mixed_entries.items` | Same six | All become `{}` | Same |
| R3-F1.7 | Flattened extra-value schema, `MixedExtras.additionalProperties` | Same six | All become `{}` | Same |
| R3-F1.8 | Ordinary reference controls at those seven positions | Unwrapped `owner: {$ref: Owner}` | Rebuilt target references and generated checks survive; existing compiled acceptance test passes | Clean |
| R3-F1.9 | Downstream generation decisions | Imported schema → emitted schema → registered API → validator and generated plan | Supported checks disappear; unsupported-rule errors disappear | Preserve the traversal or the required refusal |
| R3-F1.10 | Direct planner, JSON body and multipart controls | Supported null alternative and one-member `allOf`; generic nested compositions | Existing tests enforce supported shapes and reject unsupported ones | Clean; loss occurs before this consumer |

The repair log documents and deliberately backs out composition preservation because it changes OpenAI diagnostics and the historical baseline. Neither consequence authorizes dropping a supported nullable wrapper. The spec explicitly names that case, so this finding stays high despite its recurrence. This asks for preservation and the existing failure policy, not arbitrary union evaluation. Preserve nullable/transparent edges and disclose unsupported constrained compositions before their rules can disappear. Do not broaden the evaluator to make the catalog pass.

## Blocked Findings

None. More affected OpenAI endpoints increase the consequences of the selected generation-error policy; they do not supply a new author ruling or make its implementation ambiguous. Alias limitations are bounded and nonblocking, not pending human decisions.

## Recurrence

- **R1-F3** repeats review-1.md and review-2.md's finding of the same ID: unchanged root-union severity policy. The repair did fix positional arrays; that repair is not being reopened.
- **R3-F1** is a sibling of review-2.md's R2-F2 and review-1.md's R1-F1 preservation class. The reference repair should have swept transparent wrappers around the same reference and the decisions that consume their rules. It instead explicitly excluded every composition. This review enumerates all seven rebuilding positions and all six relevant composition shapes, plus the clean direct consumers.
- **R2-F2's remaining alias children** extend the same preservation class. They are reduced to medium because they require unusual non-nominal recursive/container aliases with no shipped instance; adding more alias machinery would cost more than their current harm warrants.

The supported nullable-wrapper requirement has a clear boundary. Generic union evaluation remains excluded. Further synthetic composition variants should not generate a new evaluator requirement or an expanding payload matrix.

## Input Robustness Matrix

The default for malformed supported constraint metadata is a located error before generation publishes output. Exceptions are absent optional metadata; empty patterns; `uniqueItems: false`; valid imported patterns outside the enforced subset, preserved with warnings; annotation-only formats; and arbitrary JSON data, including null, within valid enum/const values.

| Shared validator or loader | Shapes and public wiring checked |
| --- | --- |
| Endpoint constraint deserializer and semantic validation | Reran `serialized_field_shapes_per_validator`, old-snapshot absence, explicit-null rejection, wrong whole-field types, empty values, contradictory bounds, and enum/const JSON-data controls through bundle loading. Each validator family has control and invalid cases. |
| Stored component constraint validation | Reran `component_field_shapes_per_validator`, API-build rejection, malformed bundle reload, and every-output rejection. Counts, numbers/exact text, pattern/format, uniqueness, enum/const, and items have applicable shape cases and located errors. Mixed enum members are valid JSON data, not silently filtered string entries. |
| OpenAPI loader | Reran JSON/YAML duplicate-key, trailing/invalid content, null, wrong-type keyword, empty enum, Boolean exclusive-bound, numeric range/precision, and format-syntax tests; CLI control and invalid-bound wiring verify paths and no output. |
| Bundle loader | Reran control, omitted new metadata, duplicate-key, wrong-root, trailing-content, and located error tests. |
| New rebuilding logic | Composition and alias findings concern already parsed schema traversal. The compiled probes above check the resulting public assembled API, followed by generator diagnostics/plans. |

Inspected `serde(default)`, `Option`, `filter_map`, `unwrap_or_default()`, and `.ok()` sites in constraint parsing/rebuilding. Optional metadata uses presence-aware deserialization; rule parsing reports malformed values rather than treating them as absent. The rebuilding omissions described above are explicit traversal exclusions, not an additional missing/null coercion finding. No new malformed-field class was established.

## Verification

All feature requirements are library, generation, serialization, or mock-transport behavior; **Level 1 is appropriate**. This feature adds no terminal rendering, key handling, browser, or OS-input promise requiring Level 2 or Level 3.

| Requirement | Strongest relevant evidence inspected and rerun |
| --- | --- |
| Rule authoring and definition rejection | L1 public builders, registered-model schemas, serialized bundles, import/output validation matrices |
| Direct import/export and exact-number preservation | L1 `constraint_preservation`, OpenAPI constraint import/export, normalization tests |
| Emitted definitions and registration | L1 offline compilation and assembly; seven-position ordinary-reference and membership controls pass; new probes expose composition/alias omissions |
| Request refusal, unchanged valid data, safe deterministic errors | L1 runtime evaluator/report tests and freshly compiled clients with mock transport, including `validate`, `into_parts`, build, ordinary await, full/raw/direct sends, mutation and serialization cases |
| Pager bounds and later size changes | L1 runtime pager rules, definition conflicts, generated GitHub pagination tests |
| Response constraints do not add enforcement | L1 generated-client `a_reply_outside_its_declared_value_rules_still_decodes` |
| Test compilation and tier selection | Declared integration targets, consolidated generator `l1/main.rs`, live fresh-output recipe, and tier audit |

Executed on this macOS host:

- `just test` in `schematic/`: **3,150 passed**, 23 skipped; fresh-output guards **17 passed**; runtime compile-fail guard **1 passed**.
- `just test-schema --test value_rules --test pagination`: **16 passed**.
- `just check-tier-coverage schematic`: **zero stranded tests**.
- Temporary public policy sweep: **one passed**, checking 20 schema positions/shapes at both JSON-body and multipart entry points.
- Temporary compiled composition sweep: **one passed**, checking 42 rebuilt positions and downstream diagnostics/plans.
- Temporary compiled alias sweep: **one passed**, confirming both ledger limitations through emitted definitions and public assembly.

The probes assert the observed defects, not the desired specification behavior. One initial composition-probe assertion incorrectly expected dropped property names to disappear; the repaired membership logic correctly retains them as `{}` beside extra-value rules. The final assertion checks those empty plans and passed. Temporary tests were compiled through the declared generator integration target and removed afterwards. Their source and detailed outputs remain under `/tmp/type-constraints-review-3-*` on this host.

No formatter, production fix, commit, live-provider call, CI change, or lifecycle move was performed. Cross-OS evidence is left to CI and does not affect this readiness judgment. Slow tests reported by the loaded full run include existing TLS and compilation guards; no new speed finding is asserted from that run alone.

## Observations

### Registration order changes are normalized by the baseline test

The repair log records nine OpenAI components being reached earlier through the new generated schema helpers. In schematic-gen, [in_historical_order](../../gen/tests/l1/definition_baseline.rs:442) restores the affected entries to historical positions before comparison. Consequently, that passing baseline is evidence of normalized schema compatibility, not evidence that registration order stayed unchanged as the spec requested. The log accurately discloses the departure; no wrong endpoint behavior, component name, Rust path, or collision outcome was demonstrated. Keep this distinction visible when assessing the baseline rather than adding a registration redesign solely to restore historical ordering.
