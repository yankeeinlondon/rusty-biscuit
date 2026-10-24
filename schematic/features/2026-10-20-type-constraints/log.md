---
implementation_1: "2026-10-23T16:37:57-07:00"
implementation_2: "2026-10-23T19:33:13-07:00"
implementation_3: "2026-10-23T21:17:04-07:00"
implementation_4: "2026-10-23T22:04:48-07:00"
---

# Type constraints — review implementation log

## Implementation of Review Findings #1

> **started at:** 2026-10-23T16:37:57-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-20-type-constraints/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs began
- the working tree already carried uncommitted edits (docs, plan, spec, implementation log, `runtime/src/endpoint.rs`) before this cycle; they are left in place

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Rebuilding imported definitions loses supported constraints | fixed | all children F1.1–F1.8 fixed, F1.9 clean; slow compiled rebuild test asserts each key and zero generator errors |
| R1-F1.1 | Map-valued property `additionalProperties.maxLength` survives rebuild | fixed | `schema:Item/properties/attributes/additionalProperties` in `constraint_preservation::slow_definitions_out_rebuilds_every_rule_through_registration` (importer types maps as `Value`, so rebuilt as `extend("additionalProperties"=…)`); generator map path covered by `model_gen::tests::definitions_out_rebuilds_rules_wherever_the_rust_type_places_them` |
| R1-F1.2 | Free-form JSON property `maxLength` survives rebuild | fixed | `schema:Item/properties/free_value` untyped `maxLength`, same slow test |
| R1-F1.3 | String `format: uuid` annotation survives rebuild (not enforced) | fixed | `schema:Item/properties/identifier` keeps `format: uuid`; planner has no format check (annotation only) |
| R1-F1.4 | Map field whole-value `enum` survives rebuild | fixed | `schema:Item/properties/palette` whole-value `enum`, same slow test |
| R1-F1.5 | Struct root `const` survives rebuild | fixed | `schema:Settings` container `extend` const, same slow test |
| R1-F1.6 | Struct flattened `additionalProperties.maxLength` survives rebuild | fixed | `schema:Extras/additionalProperties` container `extend`, same slow test |
| R1-F1.7 | Named struct field with adjacent `const` survives rebuild | fixed | `schema:Item/properties/owner` (one-member `allOf` + `enum` under Schemars OpenAPI 3.0); `Owner` component unchanged; planner and `unenforceable_rules` walk one-member `allOf`; `value_rules::tests::a_one_member_all_of_is_its_reference_and_keeps_the_rule_beside_it` |
| R1-F1.8 | Inline object array item `const` survives rebuild | fixed | `schema:Item/properties/entries/items` and `schema:Item/properties/owners/items`; importer never names inline objects, so the free-form path carries it |
| R1-F1.9 | Controls (primitive, renamed, alias, primitive-array items, inline roots, param/form metadata) keep working | clean | existing preservation keys and export/re-import meaning comparison still pass; full `just test` green |
| R1-F2   | Invalid constraints are accepted or discarded instead of rejected | fixed | all children fixed or clean, including discovered F2.5–F2.7 |
| R1-F2.1 | Stored component malformed rules (null, wrong type, negative, contradictory, `multipleOf: 0`, empty enum, `items: 7`, invalid pattern) give located errors at API build / generator entry, incl. bundle reload | fixed | `schema_rule_problems` wired into `constraint_problems`/`validate_api`; `value_constraints::component_field_shapes_per_validator` (47 cells), `a_reloaded_bundle_refuses_a_malformed_component_rule_of_each_family`, gen `output_validation::a_malformed_component_rule_blocks_every_output` |
| R1-F2.2 | Imported contradictory supported rules rejected (query/path/header/cookie/body/form) without silently erasing valid siblings | fixed | import errors at the keyword pointer in both modes; `contradictory_item_lengths_fail_the_import_in_either_mode` (real fixture), `contradictory_rules_fail_the_import_at_every_input_and_component`, `an_inapplicable_rule_is_removed_without_its_siblings` |
| R1-F2.3 | Authored/serialized endpoint metadata route (control) | clean | `serialized_field_shapes_per_validator` passes; one added cell rejects an invalid unenforced pattern |
| R1-F2.4 | Imported numeric/null keyword loader (control) | clean | existing `loader_*` tests pass; added `loader_empty_enum_fails_with_its_path` |
| R1-F3   | Unenforceable request rules do not consistently block generation | blocked | F3.3 fixed and F3.6 verified; F3.1/F3.2 need an author decision (OpenAI `CreateResponse` would stop generating) |
| R1-F3.1 | Component-root `anyOf` member rule → generation error | blocked | forcing error fails OpenAI generation with 42 errors on `CreateResponse`; spec has no exception and union evaluation is a non-goal; author must choose tag-selected member checks, a documented exception, or dropping the endpoint's generation; warning and pinning tests unchanged |
| R1-F3.2 | Component-root `oneOf` member rule → generation error | blocked | same code path and decision as R1-F3.1 |
| R1-F3.3 | Positional `items: [...]` rule → generation error | fixed | positional `items`, `prefixItems`, and 10 other unwalked keywords are generation errors; `registered_model_rules` tuple tests, `output_validation::a_request_rule_in_positional_items_blocks_the_rust_client` |
| R1-F3.4 | Nested `oneOf`, root `allOf`, root `not` (control) | clean | existing nested `oneOf`/root `allOf`/root `not` error tests pass (one-member `allOf` now walked per R1-F1.7; two-member still errors) |
| R1-F3.5 | Ordinary array item object and nullable `anyOf` (control) | clean | new control test: no findings, plans contain `.items(..max_length(3))` and `.max_length(5)` |
| R1-F3.6 | JSON body and multipart JSON field share the classification policy | fixed | identical errors at `body (...)` and `body.meta (...)` in new `value_rules` parity tests |
| R1-F4   | Pruning declared properties applies extra-property rules to valid fields | fixed | all children fixed or clean |
| R1-F4.1 | Inactive declared property retained as membership when extra-value plan is active | fixed | `prune` keeps declared names while a `values` plan survives; `pruning_keeps_declared_names_while_an_extra_value_plan_survives`; fresh-output `a_declared_property_is_never_checked_by_the_extra_value_plan` red before, green after |
| R1-F4.2 | Runtime evaluator selects declared plan first (control invariant) | clean | runtime evaluator unchanged; runtime `validate` tests 37/37 |
| R1-F4.3 | Pure map (control) | clean | existing map tests pass; extra keys fail at `body[[redacted]]` |
| R1-F4.4 | Declared property with active rules (control) | clean | active declared properties prune as before; nested-body tests pass |
| R1-F4.5 | Nested object via reference and multipart JSON field | fixed | `CreateHolder` (named `$ref`) and `UploadMixed` (multipart JSON) in the fresh-output child test; `an_unchecked_declared_property_stays_out_of_the_extra_value_plan_at_every_level` |
| R1-F5   | Nullable plans bypass explicit enum and const restrictions | fixed | all children fixed or clean, including discovered F5.6 |
| R1-F5.1 | Nullable type with `enum: ["ok"]` rejects null | fixed | `value_rules::tests::a_nullable_type_keeps_its_enum_on_the_plan_null_is_checked_against` + runtime test below |
| R1-F5.2 | Nullable `Plan` with independent enum or const checks null | fixed | `validate_eval::null_is_data_for_an_enum_or_const_with_or_without_a_nullable_type` |
| R1-F5.3 | Nullable alternative wrapping constrained member still admits null (control) | clean | `validate_eval::a_null_alternative_admits_null_past_its_constrained_member` |
| R1-F5.4 | Non-nullable null members / string-only rules receiving null (control) | clean | `WITH_NULL`/`WITHOUT_NULL`/`LENGTH_ONLY` assertions retained |
| R1-F5.5 | Definitions rebuild of optional enum/const fields keeps absent vs present-null distinct | fixed | `admit_null` removed; `model_gen::tests::rebuilt_enum_and_const_keep_the_source_null_membership`; OpenAI definitions re-imported |
| R1-F5.6 | Exporter folds an overlapping member beside a null alternative into `allOf`, narrowing null (found by sweep) | fixed | `enums_and_constants_keep_their_value_domain`, `a_folded_null_alternative_keeps_admitting_null_and_a_rejecting_sibling_keeps_rejecting_it`; sibling: null alternative beside a null-rejecting keyword no longer exports `nullable` |
| R1-F2.5 | JSON Schema subschema keywords dropped silently in imported components/bodies/forms (found by sweep) | fixed | located "not preserved" warning; `dropped_subschema_keywords_in_components_bodies_and_forms_are_reported_not_preserved` |
| R1-F2.6 | Duplicate gen "invalid pattern" warning beside the definition error (found by sweep) | fixed | `output_validation::an_invalid_or_unenforceable_pattern_is_reported_once` (red before: 2 diagnostics) |
| R1-F2.7 | Component rule inapplicable to its declared `type` accepted (found by sweep) | fixed | `value_constraints` 15 new cells, `a_model_rule_inapplicable_to_its_type_fails_api_build`, `a_component_rule_inapplicable_to_its_type_is_removed_with_a_warning` |
| R1-F3.7 | OpenAPI importer drops parameter subschema keywords without a warning (found by sweep) | fixed | `dropped_subschema_keywords_on_a_parameter_are_reported_not_preserved`, `positional_items_on_a_parameter_are_a_parse_error` |
| R1-F1.10 | Consumer: historical `definition_baseline::openai` stripping did not know the new rebuild placements | fixed | normalization resets field `format` to the Schemars-implied value and drops rule-only nested objects; all 167 registry hashes match the snapshot with no exceptions |
| R1-F1.11 | Consumer: `schema_fidelity` counted rule-only free-form positions exported intact as lost | fixed | classifier treats an untyped rule-only source as kept when every rule exports; `every_registry_exports_without_losing_a_schema_constraint` passes |
| R1-O1   | Observation: allowed exclusions and optional measurements remain deferred (nonblocking) | deferred | nonblocking observation; carried to the backlog by the follow-up pass, not repaired here |

### Progress

- starting the work on 'R1-F5 nullable plans bypass enum/const' at 16:38:51
        - defect class: nullability that makes a value's _type_ nullable was treated as permission for null to skip, or be added to, an independently declared `enum`/`const`
        - runtime: `Plan::nullable()` now means "null skips this plan's reference" (the null alternative); the plan's own `enum`/`const` still apply to null; length/number/array rules never see null
        - generator lowering already keeps a type's enum on the nullable node and puts a null alternative's member behind `Plan::reference`; no change; regenerating into a temp dir matched committed `schema/src` and all 15 `openapi/*.json` exports byte-for-byte
        - R1-F5.5: removed `admit_null` from `gen/src/model_gen.rs`; rebuilt `enum`/`const` copy the source's null membership exactly (OAS 3.0.3 `nullable` does not add null to an `enum`)
        - sweep found a sibling in the OpenAPI exporter: `nullable_body` added null to every nullable enum; now null is added only to an enum merged from an `anyOf` member beside a null alternative and only when the outer schema has no `enum`/`const` of its own
        - sweep found a second exporter sibling: `merge_member`'s overlap case pushes the member into `allOf`, where a member enum without null rejects null beside a null alternative (added as R1-F5.6)
        - committed `definitions/src/openai/types.rs` was built with the old `admit_null`; a dry re-import shows 102 dropped `null` members, all on optional fields checked; refresh scheduled after R1-F1 (which also changes rebuild output)
        - tests: `validate_eval::null_is_data_for_an_enum_or_const_with_or_without_a_nullable_type`, `validate_eval::a_null_alternative_admits_null_past_its_constrained_member`, `value_rules::tests::a_nullable_type_keeps_its_enum_on_the_plan_null_is_checked_against`, `model_gen::tests::rebuilt_enum_and_const_keep_the_source_null_membership`, exporter assertions in `values_and_constants_export_as_enums`
        - scoped checks: runtime `validate` 37 passed; gen lib `model_gen|value_rules` 33 passed; gen l1 `definitions_out_constraints|constraint_preservation` 4 passed; define openapi/export 261 passed; `just migration-check` 40 passed; `just lint` clean
- work completed for 'R1-F5 nullable plans bypass enum/const' at 16:53:59 (R1-F5.6 still open)
- starting the work on 'R1-F4 pruning declared properties' at 16:53:59
        - defect class: removing an apparently inactive schema node changes the classification of values handled by another rule (a dropped declared property becomes an "extra" map entry)
        - `prune` now prunes `values` first; while it survives, every declared property stays (empty `Plan::new()` when it checks nothing); without a values plan pruning is unchanged
        - siblings checked: dead component `$ref`/`items` pruning, inline references, `emit_static` (body, multipart JSON, parameters, text), name dedupe, `whole_array_rules` — none classify by membership; no `patternProperties` lowering exists
        - regression: fixture APIs `Mixed`/`Holder`/`UploadMixed` with child test `a_declared_property_is_never_checked_by_the_extra_value_plan`, red without the fix and green with it; extra keys still fail at `body[[redacted]]`, `body.inner[[redacted]]`, `body.mixed[[redacted]]`
        - unit tests `pruning_keeps_declared_names_while_an_extra_value_plan_survives`, `an_unchecked_declared_property_stays_out_of_the_extra_value_plan_at_every_level`
        - checks: gen lib `value_rules` 9 passed; `just test-fresh-output` 17/17 passed; runtime `validate` 37 passed; clippy clean; temp regeneration identical to committed `schema/src`
- work completed for 'R1-F4 pruning declared properties' at 17:05:11
- starting the work on 'R1-F3 unenforceable request rules' at 17:05:11
        - defect class: the check for whether a request rule is reachable by generated checks treats some unwalked schema positions as supported, or downgrades them to a warning, so a client is generated that silently skips a supported rule
        - R1-F3.3 fixed: a rule beneath a positional `items` list, `prefixItems`, `additionalItems`, `contains`, `unevaluatedItems`, `patternProperties`, `propertyNames`, `unevaluatedProperties`, `dependentSchemas`, schema-form `dependencies`, or `if`/`then`/`else` is now a Rust-output `type-constraint` error; only an `items` object is walked
        - comment drift: the walk claimed OpenAPI 3.0 settings rewrite `prefixItems` as an `items` list; a registered tuple shows Schemars writes `prefixItems`, so tuple element rules were never walked; comment replaced (code assumed correct)
        - unresolved `$ref` / untraversed keyword hint now reads "remove the rule or restructure the schema"
        - R1-F3.1/F3.2 **blocked**: forcing union-member severity to error fails OpenAI Rust generation with exactly 42 errors, all on `CreateResponse` (`BetaTool` 30, `BetaToolChoiceParam` 9, `BetaTextResponseFormatConfiguration` 3); none are Rust-type guaranteed
                - the only route that keeps OpenAI generatable is checking the member selected by its `type` tag (all three unions have non-overlapping tag values): new union evaluation in runtime and generator, likely an ABI bump, and union evaluation is a spec non-goal
                - author must choose: approve tag-selected member checks, record a documented spec exception, or accept that OpenAI `CreateResponse` stops generating
                - the warning and its pinning tests (`unenforced_rules_are_findings_with_their_severity`, `UNCHECKED_VALUE_RULES = 42`) remain until the decision
        - F3.6 parity: positional and nested-`anyOf` rules give identical errors at `body (...)` and `body.meta (...)`
        - sweep found an importer sibling (added as R1-F3.7): the OpenAPI importer's warning list for keywords parameter metadata cannot carry omits an `items` list, `additionalItems`, `if`/`then`/`else`, `dependentSchemas`, `unevaluated*`, so those rules are dropped silently
        - shipped output: all APIs generate (0 errors, the same 42 warnings); Rust client, OpenAPI, Postman output byte-identical to committed copies
        - tests: `registered_model_rules` 7/7, define doctests 175, define `constraint` 13/13, gen lib `value_rules` 12/12, gen l1 `output_validation::` + `definitions_out_constraints` 23/23; new `a_request_rule_in_positional_items_blocks_the_rust_client`
- work completed for 'R1-F3 unenforceable request rules' at 17:25:48 (R1-F3.1/F3.2 blocked; R1-F3.7 handed to R1-F2's import sweep)
- starting the work on 'R1-F2 invalid constraints accepted or discarded' at 17:25:48
        - defect class: whether a value rule is checked depended on how it entered the definition; rules stored in components or imported were read loosely, so a malformed value counted as absent and a contradiction was dropped instead of rejected
        - new shared validator `schema_rule_problems` walks every component and its subschemas (never entering data keywords); `constraint_problems` and `validate_api` include components, so API build, bundle load, and all three output validators (Rust, OpenAPI, Postman) reject malformed/contradictory component rules located by component and JSON pointer
        - `Constraints::from_schema` now returns `Result`; `ConstraintProblem.endpoint` became `site: ConstraintSite` with an `endpoint()` accessor; `Display` text unchanged
        - `[` (unclosed class) is now an invalid pattern; `check` rejects an invalid pattern even when marked unenforced
        - import: invalid or contradictory supported rules in query/path/header/cookie/body/form/component fail the import at the keyword's pointer in permissive and strict mode; a rule that only mismatches its input's type is removed one keyword at a time with a warning, so valid siblings survive; an empty source `enum` (erased by `openapiv3`) is refused at its pointer
        - R1-F3.7: source capture records 12 dropped JSON Schema subschema keywords and warns "not preserved" for each on a parameter; a positional `items` list is already a parse error
        - sibling readers: gen `schema_node` and `model_gen::rule_keywords` read strictly and rely on the upstream validator (`ensure_no_errors` before codegen); export `exact_number` runs on validated definitions; presence-only readers (`unenforceable_rules::report`, `is_enum_value_set`) are blocked upstream
        - tests: `value_constraints::component_field_shapes_per_validator` (47 cells), `a_contradictory_model_rule_fails_api_build_located_by_component_and_pointer`, `a_reloaded_bundle_refuses_a_malformed_component_rule_of_each_family`, gen `output_validation::a_malformed_component_rule_blocks_every_output`, import `contradictory_item_lengths_fail_the_import_in_either_mode` (real fixture, error at `…/parameters/3/schema/items/minLength`), `contradictory_rules_fail_the_import_at_every_input_and_component`, `an_inapplicable_rule_is_removed_without_its_siblings`, `invalid_pattern_fails_the_import_at_its_pointer`, `loader_empty_enum_fails_with_its_path`, `dropped_subschema_keywords_on_a_parameter_are_reported_not_preserved`, `positional_items_on_a_parameter_are_a_parse_error`
        - checks: define (openapi) 948 passed + doctests; gen filtered 197 passed; lint clean; temp generation of all APIs identical to committed `schema/src`; no committed definition holds an invalid rule
        - red: `import_overlay::reimporting_the_vendored_openai_document_reproduces_the_committed_definition` — the committed OpenAI definition still carries the old `admit_null` members (from R1-F5); refresh via `just import-openai` after R1-F1
        - open follow-ups found (added as R1-F2.5–F2.7): 3.1 subschema keywords inside imported _components_ are still dropped unwarned; gen still emits a duplicate "invalid pattern" warning beside the new error; a component rule inapplicable to its declared `type` is not rejected
        - two define tests ran over 5 s (`bundle_json_matrix`, `shipped_openai_query_schema_gate_and_reimports`); checked in the phase gate
- work completed for 'R1-F2 invalid constraints accepted or discarded' at 18:01:28 (R1-F2.5–F2.7 open)
- starting the work on 'R1-F1 rebuild loses supported constraints' at 18:01:28
        - defect class: rebuilding schemas filtered supported rules according to the emitted Rust type instead of preserving their meaning
        - rules are now placed according to the emitted Rust type: field attributes, container `extend` (model root, flattened extras), map values and array items (over the Schemars slot schema, with a `$ref` base for models), free-form `Value` fields (untyped nested `properties`/`items`/`additionalProperties`), and `format` (kept as an annotation, replacing the Rust-implied one)
        - discovery: the OpenAPI importer types maps and inline objects as `serde_json::Value`, so F1.1/F1.4/F1.8 travel the free-form path; no inline object gets a model name; generator map-value rebuilding has unit coverage only
        - discovery: Schemars' OpenAPI 3.0 settings turn an occurrence rule beside a model `$ref` into a one-member `allOf`; `unenforceable_rules` and the value-rule planner now treat a one-member `allOf` as its member (matching generator validation); `registered_model_rules` contract changed from "one-member `allOf` blocks" to "walked through"; two-member `allOf` still blocks
        - fixture `constraint_preservation.yaml` gained `attributes`, `free_value`, `identifier`, `palette`, required `owner` (`$ref` + `const`), `settings`, `extras`, `entries`, `owners`, components `Owner`/`Settings`/`Extras`
        - slow rebuild test asserts each key, rule-map equality with direct import, unchanged `Owner` component, and zero `validate_rust_client` errors; red with the `allOf` handling disabled
        - meaning comparator now walks `additionalProperties` and compares `format` (except Schemars' `int64`/`double`)
        - remaining excluded gaps: rules inside composition members of free-form fields (generic composition is a non-goal); `$ref` targets beneath free-form values are not restated
        - temp OpenAI re-import: 665 changed lines in `types.rs` (164 `format`, 25 `properties`, 14 `items`, plus R1-F5 null removals) and 8 pre-existing stale lines in `mod.rs`; the rebuilt module compiles, registers, validates with 0 errors
        - scoped checks: gen lib `model_gen`/`value_rules`, gen l1 `constraint_preservation`/`definitions_out_constraints` (slow included), define `constraint_preservation`, `registered_model_rules`, `composition`, `value_constraints`, `openapi_constraint_export` all passed; clippy clean
- work completed for 'R1-F1 rebuild loses supported constraints' at 18:37:34
- starting the work on 'sweep follow-ups R1-F5.6, R1-F2.5, R1-F2.6, R1-F2.7' at 18:37:34
        - R1-F5.6 fixed: an overlapping member folded beside a null alternative now exports as `allOf: [{anyOf: [member, null]}]`; sibling fixed: a null alternative beside a null-rejecting `type`/`enum`/`const` no longer exports `nullable: true` (it widened the domain); tests `enums_and_constants_keep_their_value_domain`, `a_folded_null_alternative_keeps_admitting_null_and_a_rejecting_sibling_keeps_rejecting_it`
        - R1-F2.5 fixed by a located warning (preservation impossible: components are stored from the `openapiv3` parse, which has no field for these keywords); strict mode fails on it; test `dropped_subschema_keywords_in_components_bodies_and_forms_are_reported_not_preserved`
        - R1-F2.6 fixed: gen no longer re-warns invalid patterns or unmarked out-of-subset endpoint patterns that `constraint_problems` already rejects; test `an_invalid_or_unenforceable_pattern_is_reported_once` (red before: 2 diagnostics)
        - R1-F2.7 fixed: `schema_rule_problems` rejects a rule applicable to none of a declared `type`'s names (no declared type → no check); imported component mismatches are removed with a warning, matching the existing parameter policy (departure recorded); tests: 15 new `value_constraints` matrix cells, `a_model_rule_inapplicable_to_its_type_fails_api_build`, `a_component_rule_inapplicable_to_its_type_is_removed_with_a_warning`
        - checks: define filtered 272 + wider 462 + lib 509 passed; gen lib 45 passed; gen l1 27 passed; `just lint` clean; all 15 APIs generate byte-identical to `schema/src`; `cargo check -p schematic-definitions --all-targets` clean
- work completed for 'sweep follow-ups R1-F5.6, R1-F2.5, R1-F2.6, R1-F2.7' at 18:58:07
- reconciliation (orchestrator):
        - re-imported OpenAI definitions with the `import-openai` command; restored committed `definitions/src/openai/mod.rs` afterwards because its two re-import differences (`info.version`, realtime `session` JSON part) are deliberately pinned in `import_overlay` as outside this feature; only `types.rs` adopts the new rebuild output
        - regenerated Rust clients, OpenAPI, Postman, and AsyncAPI with the `generate` recipe's commands; only OpenAI artifacts changed (more requests now carry `validate()` because free-form fields carry rebuilt rules)
        - first full run failed `schema_fidelity::every_registry_exports_without_losing_a_schema_constraint` (25 rule-only free-form positions counted as "lost" though exported intact); fixed the classifier (R1-F1.11)
        - second full run failed `definition_baseline::openai` (167 registry hashes); a subagent extended the historical value-rule stripping to undo exactly the rebuild's additions (R1-F1.10)
        - noted: `ConstraintProblem` display text now reads `… \`keyword\` reason` (the colon after the keyword was dropped); no doc or test quotes the old form
        - cross-OS check skipped: the changes are JSON-schema logic, code generation, and tests with no path, process, or OS-specific code
- work completed for 'reconciliation and phase gates' at 19:22:17

### Completion Report

- **outcome: partial** — elapsed 2 h 44 min (16:37:57 → 19:22:17)
- parent findings: total 5 = fixed 4 (R1-F1, R1-F2, R1-F4, R1-F5) + already fixed 0 + blocked 1 (R1-F3) + deferred 0 + unresolved 0 + disputed 0
        - observation R1-O1 is nonblocking and deferred to the backlog pass; it is not counted as a finding
- verified repairs:
        - R1-F1: rebuilt definitions keep map-value, free-form, format, whole-value, model-root, flattened-extra, occurrence (`$ref` + rule), and array-item rules; proved through compile + `module_group()` + `validate_rust_client`
        - R1-F2: one shared validator for stored component rules at API build, bundle load, and every output; contradictory imported rules fail at their pointer without erasing valid siblings; sibling gaps F2.5–F2.7 and F3.7 closed
        - R1-F3: rules beneath positional `items`, `prefixItems`, and ten other unwalked keywords now fail generation; JSON body and multipart JSON parity verified
        - R1-F4: declared properties stay in the plan while an extra-value plan survives; compiled regression red before, green after
        - R1-F5: a nullable type no longer bypasses its own `enum`/`const`; null alternatives still admit null; rebuild and export keep the source's null membership
- open items:
        - **R1-F3.1, R1-F3.2 — blocked (author decision):** making component-root `anyOf`/`oneOf` member rules generation errors makes OpenAI Rust generation fail with 42 errors, all on `CreateResponse` (`BetaTool` 30, `BetaToolChoiceParam` 9, `BetaTextResponseFormatConfiguration` 3). Next action: the author picks one of (a) tag-selected member checks (all three unions have non-overlapping `type` tags; new runtime/generator support, likely ABI bump), (b) a documented spec exception for union-member rules, or (c) accept that `CreateResponse` stops generating. The warning and its pinning tests (`unenforced_rules_are_findings_with_their_severity`, `UNCHECKED_VALUE_RULES = 42`) flip once decided
- changed files (this cycle):
        - runtime: `runtime/src/validate/{eval.rs,plan.rs}`, `runtime/tests/l1/validate_eval.rs` (R1-F5)
        - gen: `gen/src/model_gen.rs` (R1-F1, R1-F5.5), `gen/src/codegen/value_rules/{mod.rs,tests.rs}` (R1-F1.7, R1-F2.6, R1-F3, R1-F4, R1-F5), `gen/src/validation.rs` (R1-F2), `gen/tests/l1/{constraint_preservation.rs,definitions_out_constraints.rs,fresh_output_compile.rs,output_validation.rs,definition_baseline.rs}`
        - define: `define/src/constraints/{check.rs,mod.rs,pattern.rs,registered.rs,schema.rs}`, `define/src/{builders.rs,registration.rs}`, `define/src/openapi/import/{builder.rs,resolver.rs,normalize.rs,mappings/mod.rs,mappings/schema.rs}`, `define/src/openapi/export/conversion.rs`, `define/tests/{constraint_preservation.rs,openapi_constraint_export.rs,openapi_constraint_import.rs,registered_model_rules.rs,value_constraints.rs,support/constraint_meaning.rs,fixtures/constraint_preservation.yaml}`
        - definitions and artifacts: `definitions/src/openai/types.rs`, `definitions/tests/schema_fidelity.rs`, `schema/src/openai/requests.rs`, `openapi/openai.json`, `postman/openai.postman_collection.json`
        - docs and skills: `docs/{value-constraints.md,typed-registration.md}`, `docs/generation/{request-side.md,validation.md}`, `docs/io/{definition-bundles.md,export-openapi.md,import-openapi.md}`, `docs/runtime/validation.md`, `.claude/skills/schematic-define/{SKILL.md,schematic/gen.md,schematic/openapi.md}`
- final verification (macOS, run after every change, no subagents running):
        - `just test` (schematic): 3,146 passed, 23 skipped; fresh-output 17/17; runtime compile-fail 1/1 — all findings
        - `just lint` (schematic): exit 0
        - `just test-schema`: 154/154 passed — committed OpenAI client after regeneration (R1-F1, R1-F5)
        - `just check-tier-coverage schematic`: 0 stranded
        - `git diff --check`: clean
        - six main-suite tests exceed 5 s (`extension_matrix_yaml`, `import_overlay::reimporting_the_vendored_openai_document_reproduces_the_committed_definition`, four runtime TLS `builder::` tests) plus the dedicated slow feature-selection test; the review recorded six slow main-suite tests before this cycle

## Implementation of Review Findings #2

> **started at:** 2026-10-23T19:33:13-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-20-type-constraints/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs began

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| -------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F3    | Unenforceable request rules do not consistently block generation | blocked | F3.3–F3.5, F3.7, F3.8 clean; F3.1, F3.2, F3.6 blocked on one author decision, now backed by new evidence (see R2-F2.12): the spec's error policy, applied faithfully, stops OpenAI Rust generation on 27 endpoints, not the 42 rules on one endpoint the review measured |
| R1-F3.1  | Root-component `anyOf` member rule → generation error at JSON body and multipart field | blocked | flipping severity alone fails OpenAI `CreateResponse` (42 errors); faithful free-form rebuild adds 343 errors on 26 more endpoints. Each remaining route is a design choice: tag-selected (discriminated) union checks, a documented spec exception for union-member rules, or giving up OpenAI Rust generation. Warning policy unchanged until the author rules |
| R1-F3.2  | Root-component `oneOf` member rule → generation error at JSON body and multipart field | blocked | same code path (`composition_findings`) and same decision as R1-F3.1 |
| R1-F3.3  | Positional `items`/`prefixItems` (control, repaired in iteration 1) | clean | `a_rule_in_positional_items_is_a_generation_error_for_body_and_form_field_alike`, `output_validation::a_request_rule_in_positional_items_blocks_the_rust_client`, `registered_model_rules::rules_in_positional_items_are_located` pass in the final `just test` |
| R1-F3.4  | Nested union, two-member `allOf`, `not`, union in map values (control) | clean | `rules_beneath_one_of_all_of_and_not_are_located_but_a_one_member_all_of_is_walked_through` and the `value_rules` error tests pass |
| R1-F3.5  | Supported traversal controls (item object, null alternative, one-member `allOf`) | clean | `a_one_member_all_of_is_its_reference_and_keeps_the_rule_beside_it` and the iteration-1 control test pass |
| R1-F3.6  | Consumers: body/form policy and shipped-catalog expectation (`UNCHECKED_VALUE_RULES = 42`) | blocked | the expectation follows R1-F3.1/F3.2; `live_definitions_have_no_embedded_queries_or_validation_gaps` passes with the unchanged 42 after the R2-F2 adoption (same 57 warnings line for line) |
| R1-F3.7  | Import disclosure (control, repaired in iteration 1) | clean | `dropped_subschema_keywords_on_a_parameter_are_reported_not_preserved`, `..._in_components_bodies_and_forms_...`, `positional_items_on_a_parameter_are_a_parse_error` pass |
| R1-F3.8  | Other unsupported subschema keywords (control) | clean | `rules_beneath_untraversed_keywords_are_located` passes |
| R2-F1    | Rebuilding free-form schemas drops declared-property exemptions | fixed | every child fixed or clean; mutation (reverting the keep condition) fails the slow test and three unit tests |
| R2-F1.1  | Free-form field root mixed object: declared label accepted | fixed | unit `a_kept_extra_value_rule_keeps_every_declared_name_at_each_free_form_position`; compiled child `every_free_form_position_exempts_its_declared_names_from_the_extra_value_rule` (`validate()` + `into_parts()`) |
| R2-F1.2  | Nested `free_value.inner` mixed object | fixed | same two tests, path `body.mixed_value.inner` |
| R2-F1.3  | Free-form `items` mixed object | fixed | same, `body.mixed_value[0]` |
| R2-F1.4  | Free-form `additionalProperties` mixed object | fixed | same, `mixed_map` |
| R2-F1.5  | Map-valued model field mixed object | fixed | same, `mixed_attributes` |
| R2-F1.6  | Inline object array item mixed object | fixed | same, `mixed_entries` |
| R2-F1.7  | Flattened extra-properties value mixed object | fixed | unit test checks `MixedExtras` container `extend`; child test checks `mixed_extras` |
| R2-F1.8  | Active declared property and actual extra key still rejected (control) | clean | child test: `code: "long"` rejected at `{at}.code`, `extra: "long"` at `{at}[[redacted]]`, through both entry points |
| R2-F1.9  | Nominal struct's own declared names accepted (control) | clean | `extras`/`mixed_extras` accept `id: "long declared id"` |
| R2-F1.10 | Downstream classification and preparation keep membership | fixed | slow test compares direct and rebuilt generated plans: both hold six mixed-object plans and seven `Property::new("label", &Plan::new())` entries (rebuilt was `(0, 0)` before the fix); runtime acceptance proven on the compiled client |
| R2-F2    | Rebuilding free-form schemas drops referenced constraints | blocked | every review-required child (F2.1–F2.10) fixed or clean and adopted into the OpenAI definitions; discovered sibling F2.12 blocked with R1-F3, F2.14–F2.15 deferred |
| R2-F2.1  | Referenced property in free-form field root rejected by `maxLength` | fixed | unit `a_free_form_reference_to_a_model_with_rules_is_written_through_the_generator_at_each_position`; compiled child `every_free_form_position_reaches_a_referenced_model_s_rule` rejects `{"owner":{"name":"longer than ten"}}` via `validate()` and `into_parts()`, accepts `short` |
| R2-F2.2  | Referenced property in `free_value.inner` | fixed | same two tests |
| R2-F2.3  | Referenced property in free-form array item | fixed | same two tests |
| R2-F2.4  | Referenced property in free-form map value | fixed | same two tests |
| R2-F2.5  | Referenced property in map-valued model field | fixed | same two tests |
| R2-F2.6  | Referenced property in inline object array item | fixed | same two tests |
| R2-F2.7  | Referenced property in flattened extra value | fixed | same two tests (`schema_with` on the flattened field) |
| R2-F2.8  | Nominal typed reference control | clean | unit test asserts `Item.owner` still emits `extend("const"=…)`; slow test's rule-map comparison and unchanged `Owner` component pass |
| R2-F2.9  | Downstream: closure check, generator validation, generated validation/preparation enforce rule | fixed | slow test: rebuilt API has `Owner`, every `$ref` resolves, `validate_rust_client` 0 errors, rebuilt and direct plans each reference `Owner` 7 times; OpenAI adopted with 0 errors |
| R2-F2.10 | Documented exclusion updated | fixed | `docs/io/import-openapi.md` and `.claude/skills/schematic-define/schematic/openapi.md` now state references are rebuilt (target registered) and compositions are not |
| R2-F2.11 | Discovered: `$ref` directly as free-form `items`/`additionalProperties`, `$ref` beside a rule or `nullable` | fixed | unit test, `linked_value` fixture positions, compiled child test, `a_rebuilt_free_form_reference_gives_the_direct_import_s_findings_and_plans` |
| R2-F2.12 | Discovered: compositions (`anyOf`/`oneOf`/`allOf`/`not`) beneath free-form values are dropped by the rebuild | blocked | preserving them makes rebuilt OpenAI fail Rust validation with 343 `type-constraint` errors on 26 endpoints (the direct import already reports them); depends on the same author decision as R1-F3.1/F3.2; even the transparent-only variant adds 71 union warnings and 114 `schema_with` helpers; backed out and documented as current behavior |
| R2-F2.13 | Discovered: reference to a plain alias, and a reference at the top of a free-form value | fixed | OpenAI `ChunkingStrategyRequestParam` sweep; unit assertions in `model_gen::tests` |
| R2-F2.14 | Discovered: plain alias that refers back to itself through a free-form value | deferred | a plain alias is not a component after rebuild, so there is nothing for a recursive reference to point to; needs a design for registering aliases; no live definition has one; documented in `docs/io/import-openapi.md` |
| R2-F2.15 | Discovered: JSON request body typed as a plain alias over inline objects (`Forest = Vec<Value>`) | deferred | `body_constraints` lowers to `Constraints`, which cannot hold nested properties or references, so only `maxItems` survives; needs a body-schema carrier on the endpoint; no live definition has one; documented in `docs/io/import-openapi.md` |

### Progress

- starting the work on 'R2-F1 free-form rebuild drops declared names' at 19:39:20
        - defect class: discarding unconstrained declared property names while rebuilding a schema changes which values an extra-property (`additionalProperties`) rule constrains
        - reproduced: re-importing the review's mixed object (`label: {type: string}`, `code: {maxLength: 2}`, `additionalProperties: {maxLength: 3}`) at all seven positions emitted `properties` without `label` everywhere; the existing unit test `definitions_out_rebuilds_rules_wherever_the_rust_type_places_them` encoded the defect (it expected `attributes`' unconstrained `bare` to vanish beside a kept extra-value rule) and now expects `"bare":{}`
        - fix (`gen/src/model_gen.rs` `free_form_rules`): the `additionalProperties` rules are rebuilt first; while they are non-empty every declared property name is kept, as `{}` when it has no rules (a Boolean property schema included). Without a kept extra-value rule (absent, rule-free, or the Boolean `true`/`false` forms, which are not rules) a rule-free property is still left out, so no noise is emitted. `$ref` handling unchanged (R2-F2); a later `$ref` restatement slots into the per-property rebuild
        - all seven positions route through `free_form_rules`: the importer types every inline object as `Primitive(Json)` (`map_schema_to_type_ref_inner`), so `free_value`, its nested/items/extra values, a map-valued field, inline array items (`child` with base `{}`), and a struct's flattened extra value (`model_attribute` -> `child("additionalProperties", Json)`) all reach it
        - sibling sites checked: `RuleSource::rules` `Map`/`KeyedMap` branch (clean: the importer never produces `TypeRef::Map`, and a Rust map has no declared names); `child` (clean: delegates to `rules`; skipping an empty child drops no name); `model_attribute` (clean: a nominal struct's declared names are Rust fields Schemars writes, `extend` replaces only `additionalProperties`); `field_attribute`, `base_schema`, `own_rules` (clean: no property membership); generator `prune` (control: rebuilt plans keep `Property::new("label", &Plan::new())` beside `values`)
        - unit tests (`model_gen::tests`): `a_kept_extra_value_rule_keeps_every_declared_name_at_each_free_form_position` (exact emitted `extend` at R2-F1.1–F1.7), `a_declared_name_without_rules_is_left_out_when_no_extra_value_rule_is_kept` (absent, rule-free, `false`, `true` extra schemas; empty result; Boolean property kept)
        - fixture: added `Item.mixed_value` (root, `inner`, `items`), `mixed_map`, `mixed_attributes`, `mixed_entries`, `mixed_extras` and component `MixedExtras` to `define/tests/fixtures/constraint_preservation.yaml`; define consumers unchanged and green
        - e2e (`slow_definitions_out_rebuilds_every_rule_through_registration`, still one fresh-crate compile): child `every_free_form_position_exempts_its_declared_names_from_the_extra_value_rule` runs `validate()` and `into_parts()` on the compiled client at all seven positions: mixed object accepted; `code: "long"` refused at `{at}.code`; `extra: "long"` refused at `{at}[[redacted]]`; `Extras`/`MixedExtras` `id: "long declared id"` accepted. The outer test then generates value-rule plans for the direct and rebuilt APIs and requires both to hold the six mixed-object plans and seven `label` entries (whole-module equality is not possible: `const` vs one-value `enum` and `Option` null alternatives are spelled differently)
        - mutation check: reverting the one-line keep condition fails the slow test (rebuilt `(0, 0)` vs `(6, 7)`) and three `model_gen` unit tests; restored
        - docs: `docs/io/import-openapi.md` rebuild table row states the kept-membership rule; `free_form_rules` doc comment updated
        - checks: `cargo nextest run -p schematic-gen --lib -E 'test(model_gen) | test(value_rules)'` 42/42; `cargo nextest run -p schematic-gen --test l1 -E 'test(constraint_preservation) | test(definitions_out) | test(definition_baseline)'` 36/36; `just test-fresh-output` 17/17; `cargo nextest run -p schematic-define --all-features -E 'binary(constraint_preservation) | binary(/openapi_constraint/)'` 43/43; `just check-tier-coverage schematic` 0 stranded; `just lint` clean
        - OpenAI: temp re-import with and without the fix produced byte-identical output; `types.rs` equals the committed file (0 changed lines). `mod.rs` differs from the committed file in pre-existing ways unrelated to this fix (`endpoint_183` `session` form field now `FormField::json_desc`, and an added `.version("2.3.0")`)
- work completed for 'R2-F1 free-form rebuild drops declared names' at 19:49:13
- starting the work on 'R2-F2 free-form rebuild drops reference edges' at 19:50:56
        - defect class: schema reconstruction omits reference edges (and composition edges), so supported rules become unreachable in the rebuilt definition without any generation diagnostic
        - reproduced: `free_form_rules` (`gen/src/model_gen.rs`) copied own rules and inline children only; a `$ref` beneath a `serde_json::Value` field and any `allOf`/`anyOf`/`oneOf`/`not` were dropped, so `owner: {$ref: Owner}` lost `Owner.name.maxLength: 10` and a constrained union produced no generation error on the rebuilt API while the direct import errored
        - design: kept `extend` for rule sets without a model reference (unchanged output); a rule set that references a model from inside a free-form value is written by a generated `#[schemars(schema_with = "Model::field_rules_schema_{field}")]` associated function that starts from `generator.subschema_for::<FieldType>()` and writes each keyword over it, each reference being `generator.subschema_for::<Target>()`. Chosen over a bare `$ref` in `extend` plus extra registration in `Registration::register`: a type reached only as a nested Schemars subschema never runs its own `register`, so its targets would dangle; `subschema_for` adds the target to the definitions wherever the containing type is reached, Schemars ends recursion at the component, and no second rule source exists (the source component is still the only input). `schema_with` hides `Option` from Schemars (field would become required), so `Option` fields also get `#[serde(default)]` (same deserialization; Schemars treats a defaulted field as optional, matching the `Option` case for both contracts). Flattened extra values that reference a model move `additionalProperties` from the container `extend` to a `schema_with` on the flattened field (Schemars' `flatten` merges it)
        - edges kept: `$ref` to a model whose component (transitively, fixed point over references; `format` is not a rule) holds a rule, as an internal marker turned into `subschema_for` at emission, with sibling rules and `nullable: true`; a `$ref` to a plain alias also carries the alias' rules (alias stack ends self-recursion; the inner occurrence then carries only the Rust type's schema — documented); an unresolved `$ref` is kept literally only beside rules (same diagnostic as direct); compositions with a rule beneath a member keep every member in place, null-only members copied verbatim, `not` likewise. A rule-free reference or composition is left out (no churn, same pruned plans)
        - bugs found while sweeping OpenAI and fixed: an occurrence's own `$ref` to a plain alias was merged into the alias' schema and treated as a self edge (`ChunkingStrategyRequestParam`), and a reference at the top of a free-form value's rules was emitted as a literal key; the alias branch of `RuleSource::rules` now drops the occurrence's `$ref`, and `write_rules` copies the generator's reference object onto the schema
        - sibling sites: free-form root property, nested property, items, `additionalProperties` value, map-valued field (`HashMap<String, Value>`), inline object items (`Vec<Value>`), flattened extra values, `$ref` directly as free-form `items`/`additionalProperties`, `$ref` with sibling rules, top-level reference of a free-form value, plain alias of a free-form value, compositions (`anyOf`/`oneOf`/`allOf`/`not`) — all fixed. Discovered, not fixed: a plain-alias JSON request body whose items are inline objects (`Forest = Vec<Value>`) keeps only `max_items` in `request_constraints`; nested `properties`/references are dropped because `Constraints` cannot carry them (reproduced with `/tmp/r2f2-alias/spec.yaml`; no OpenAI instance) — needs a design decision
        - unit tests (`model_gen::tests`): `a_free_form_reference_to_a_model_with_rules_is_written_through_the_generator_at_each_position` (exact emitted attributes/helpers at all positions, `serde(default)` on `Option`, alias rules, alias recursion stop, unresolved ref, rule-free model omitted, nominal `Item.owner` control keeps `extend`), `a_rebuilt_free_form_composition_gives_the_direct_import_s_findings_and_plans` (two-member `anyOf`, referenced member, `oneOf`, `not`, two-member `allOf` give identical errors; nullable `anyOf`, one-member `allOf`, nullable `$ref` give identical plans reaching `Owner`; rule-free composition gives nothing)
        - fixture: `owner: {$ref: Owner}` added to all seven `mixed_*`/`MixedExtras` objects; new `Item.linked_value` (`$ref` items and extra values, `pinned` beside `const`, nullable `anyOf`, one-member `allOf`)
        - e2e (`slow_definitions_out_rebuilds_every_rule_through_registration`, still one fresh-crate compile): new child `every_free_form_position_reaches_a_referenced_model_s_rule` rejects `{"name":"longer than ten"}` at all seven positions plus four `linked_value` positions with `maxLength` at the exact path through `validate()` and `into_parts()`, accepts `short`/`null`; outer asserts rebuilt registers `Owner`, every rebuilt `$ref` resolves (`referenced_component`), `validate_rust_client` has no errors, and both routes' plans (component indexes normalized) contain the `owner` reference 7 times and the `linked_value` plans once. Mutation (never inserting the reference) fails the slow test with rebuilt `(0, 7, 0)`; restored
        - docs: `docs/io/import-openapi.md` rebuild table row and the former "Not rebuilt" exclusion rewritten (references, compositions, remaining recursive-alias limit); `.claude/skills/schematic-define/schematic/openapi.md`; `RuleSource`/`free_form_rules`/`assemble_model_tokens_with` doc comments
        - checks: `cargo nextest run -p schematic-gen --lib -E 'test(model_gen) | test(value_rules)'` 44/44; `cargo nextest run -p schematic-gen --test l1 -E 'test(constraint_preservation) | test(definitions_out) | test(definition_baseline)'` 36/36; `just test-fresh-output` 17/17; `cargo nextest run -p schematic-define --all-features -E 'binary(constraint_preservation) | binary(/openapi_constraint/) | binary(registered_model_rules)'` 50/50; `just lint` clean
        - OpenAI temp re-import (`/tmp/r2f2-openai`): `types.rs` differs from the committed file by 272 `schema_with` helpers and kept compositions; swapped in, `cargo check -p schematic-definitions` passes; `schematic-gen validate --api openai` goes from 0 to 343 type-constraint errors, 335 identical to the direct import's errors and 8 that the direct import reports as root-union warnings (`ChunkingStrategyRequestParam` is a plain alias, so its union is inline in the rebuilt property) — the R1-F3.1/F3.2 policy question; committed file restored (`git diff --stat definitions/` unchanged). Consequence: `import_overlay::reimporting_the_vendored_openai_document_reproduces_the_committed_definition` now fails until `just import-openai` is run, and running it makes OpenAI Rust generation fail on those errors; left for the orchestrator/author
- work completed for 'R2-F2 free-form rebuild drops reference edges' at 20:26:28
- orchestrator check after R2-F2 at 20:29:26
        - swapped the temp re-imported OpenAI `types.rs` in and ran `schematic-gen validate --api openai`: 345 errors (343 `type-constraint`), on 26 OpenAI endpoints (`CreateEvalRun` 102, `CreateFineTuningJob` 61, `CreateRealtimeClientSecret` 21, `CreateEval` 19, ...); committed file restored afterwards
        - nearly all are request rules beneath property-level `anyOf`/`oneOf` in inline objects (`model: anyOf [string, enum]`, tagged unions such as `network_policy`, `data_source`, `grader`); the direct import already reports them, but the committed rebuild silently dropped compositions inside free-form values, so OpenAI only generated because of that drop
        - consequence: preserving composition edges faithfully (the R2-F2 sweep sibling, R2-F2.12) and applying the spec's generation-error policy (R1-F3) together make the OpenAI Rust client non-generatable across 27 endpoints; the review's premise of 42 rules on one endpoint was measured on the lossy rebuild
        - decision: the `$ref` edge repair (the review's R2-F2) is kept; composition-edge preservation is backed out and recorded **blocked** with R1-F3.1/F3.2 on one author decision (union evaluation vs a documented exception vs dropping OpenAI generation); the next subagent measures `$ref`-only OpenAI impact before adoption
- starting the work on 'R2-F2 composition back-out and OpenAI adoption' at 20:30:52
        - backed out composition-edge preservation (R2-F2.12) in `gen/src/model_gen.rs` `free_form_rules`: `allOf`/`anyOf`/`oneOf`/`not` beneath a free-form value are again left out with everything beneath them (a `$ref` inside a member too); `admits_only_null` import dropped. Kept: `$ref` edges (R2-F2.1–F2.11), alias fixes (R2-F2.13), R2-F1 declared-name membership. `ruled_components` still counts rules/references beneath compositions (over-approximation: an extra reference only registers a component, while skipping compositions would drop a nominal union's typed reach); documented on `RuleSource::ruled`
        - tests: removed `a_rebuilt_free_form_composition_gives_the_direct_import_s_findings_and_plans`; added `a_rebuilt_free_form_reference_gives_the_direct_import_s_findings_and_plans` (reference, nullable reference, reference beside a rule: identical findings and plans, reaching `Owner`); `referencing_catalog`'s `Choice` (plain alias with a `oneOf`) replaced by `Holder` (plain alias of a free-form value holding `owner`), still pinning that the alias' own `$ref` is not an edge; fixture `Item.linked_value` lost `maybe` (nullable `anyOf`) and `wrapped` (one-member `allOf`), and the slow e2e child/outer `LINKED` plans with them
        - measured a variant keeping only the transparent compositions (`Option` `anyOf`, one-member `allOf`, no `$ref` beside): OpenAI 0 errors, but 133 `schema_with` helpers (vs 19), 162 components changed (272 added `anyOf`, many `format`-only), type-constraint warnings 42 -> 113 (+71 union-member warnings on CreateConversation 13, CreateResponse 50, Getinputtokencounts 50), and `definition_baseline::openai` cannot undo it from schema shape (a rebuilt `anyOf [{$ref}, null]` on a free-form field equals Schemars' own `Option<Model>`, 228 of which are historical). Treated as doubt: backed out entirely. Mutation check on the variant (disabling the `anyOf` branch) failed the slow e2e, so the fixture did cover it
        - docs: `docs/io/import-openapi.md` (rebuild table row, "references are rebuilt and compositions are not" list, YAML example of a dropped `Option` `anyOf`, `learning_rate_multiplier` example, "Also not rebuilt" list now also records the plain-alias JSON request body limit, R2-F2.15, beside the recursive-alias limit, R2-F2.14); `.claude/skills/schematic-define/schematic/openapi.md`; `RuleSource`/`free_form_rules`/`ruled` doc comments
        - OpenAI: temp re-import `types.rs` vs pre-step working tree: +399/-30 lines, 19 `schema_with` helpers (22 distinct referenced models); `mod.rs` differs only by the pinned `.version("2.3.0")` and `endpoint_183` `session` form field. Swapped in: `validate --api openai` 0 errors; warnings 57 (15 query-parameter, 42 type-constraint), line-identical to the pre-step definitions; restored, `git diff --stat definitions/` unchanged
        - adopted: `import-openai` command run for real, working-tree `mod.rs` restored from backup (only `types.rs` adopts); `generate` recipe commands rerun (0 errors): changed `openapi/openai.json`, `schema/src/openai/requests.rs`, `postman/openai.postman_collection.json` (vs HEAD now 2255 / 406 / 6 lines, were 1651 / 288 / 6); AsyncAPI unchanged
        - `definition_baseline::openai` failed after adoption (19 component hashes, 9 components registered earlier): extended `rule_only` to treat a rebuilt reference leaf (`$ref`, `nullable: true` beside it) beneath an untyped field as a rule slot, and added `rebuild_reached` + `in_historical_order`: only components whose reference count the stripping lowered, and what they reach, may sit at their historical slot; every other entry keeps its order slot by slot
        - checks: `cargo nextest run -p schematic-gen --lib -E 'test(model_gen) | test(value_rules)'` 44/44; `--test l1 -E 'test(constraint_preservation) | test(definitions_out) | test(definition_baseline) | test(output_validation)'` 59/59; `just test-fresh-output` 17/17; `schematic-define --all-features` constraint binaries 50/50; `cargo nextest run -p schematic-definitions` 657/657; `-p schematic-gen --test l1 -E 'test(import_overlay)'` 7/7; `just test-schema` 154/154; `just lint` clean
        - open: R2-F2.12 composition preservation stays blocked with R1-F3.1/F3.2 on the author decision; rules beneath free-form compositions (incl. `Option` `anyOf`) are not in rebuilt definitions
- work completed for 'R2-F2 composition back-out and OpenAI adoption' at 21:02:18
- reconciliation (orchestrator) at 21:09:01
        - inspected the combined diff of `gen/src/model_gen.rs` (`free_form_rules` membership rule, `schema_with` helper emission, alias fixes) and a generated helper in `definitions/src/openai/types.rs` (`AssistantToolsFileSearch::field_rules_schema_file_search` writes `ranking_options` through `generator.subschema_for::<FileSearchRankingOptions>()`)
        - reviewed `definition_baseline`'s new `rebuild_reached`/`in_historical_order`: only components whose reference count the stripping lowered, and what they reach, may return to their historical slot; every other entry is still compared slot by slot
        - **departure to record for the author:** the spec says rebuilding rules must not change registration order. Nine OpenAI components now register earlier, because a rebuilt free-form reference reaches them first, the same way a typed field would. Names, Rust paths, and collision handling are unchanged. Keeping the old order would mean hiding the reference from Schemars, which reintroduces R2-F2
        - optional-field side effect: a field written through `schema_with` also gets `#[serde(default)]`, so its schema stays optional; deserialization is unchanged
        - cross-OS check skipped: the changes are schema rebuilding, code generation, and tests; the generated helpers are plain Rust with no path, process, or OS-specific code
- work completed for 'reconciliation and phase gates' at 21:09:01

### Completion Report

- **outcome: partial** — elapsed 1 h 37 min (19:33:13 → 21:09:01)
- parent findings: total 3 = fixed 1 (R2-F1) + already fixed 0 + blocked 2 (R1-F3, R2-F2) + deferred 0 + unresolved 0 + disputed 0
        - R2-F2 is counted blocked, not fixed: every review-required child (R2-F2.1–F2.10) is fixed and adopted, but the discovered sibling R2-F2.12 is blocked and R2-F2.14–F2.15 are deferred
- verified repairs:
        - R2-F1: rebuilt free-form schemas keep every declared property name whenever an extra-property rule survives at the same level, at all seven positions; compiled requests accept declared labels and still reject active-property and extra-key violations with named and redacted paths
        - R2-F2: references beneath free-form values are rebuilt through generated `schema_with` helpers, so the referenced component is registered wherever the containing type is reached and its rules are enforced at all seven positions plus direct `items`/`additionalProperties` references; adopted into the OpenAI definitions (19 helpers, 0 validation errors, warnings unchanged) and regenerated artifacts
- open items:
        - **R1-F3.1, R1-F3.2, R1-F3.6 — blocked (author decision, new evidence):** making rules beneath unions generation errors stops OpenAI Rust generation. Root unions alone give 42 errors on `CreateResponse`, and a faithful free-form rebuild (R2-F2.12) adds 343 errors on 26 more endpoints (`CreateEvalRun` 102, `CreateFineTuningJob` 61, ...). The review's "42 on one endpoint" was measured on a rebuild that dropped those compositions. Next action: the author chooses (a) discriminated-union checks (select the member by its required tag property; covers `CreateResponse`'s three unions and many property-level ones, but not `anyOf [string, enum]` or overlapping-array `oneOf`s), (b) a documented spec exception that keeps union-member rules as labeled warnings, or (c) giving up OpenAI Rust generation
        - **R2-F2.12 — blocked:** composition edges beneath free-form values are left out of rebuilt definitions, as documented in `docs/io/import-openapi.md`. Next action: preserve them once (a) or (b) above is decided
        - **R2-F2.14 — deferred:** a recursive plain alias through a free-form value keeps rules only at its first level; needs a way to register aliases; no live instance
        - **R2-F2.15 — deferred:** a JSON body typed as a plain alias over inline objects keeps only its own array rules, because `Constraints` has no nested-schema carrier; needs an endpoint body-schema carrier; no live instance
- changed files (this cycle):
        - gen: `gen/src/model_gen.rs` (R2-F1, R2-F2), `gen/tests/l1/constraint_preservation.rs` (R2-F1, R2-F2), `gen/tests/l1/definition_baseline.rs` (R2-F2 adoption)
        - define: `define/tests/fixtures/constraint_preservation.yaml` (R2-F1, R2-F2)
        - definitions and artifacts: `definitions/src/openai/types.rs`, `schema/src/openai/requests.rs`, `openapi/openai.json`, `postman/openai.postman_collection.json` (R2-F2 adoption)
        - docs and skills: `docs/io/import-openapi.md`, `.claude/skills/schematic-define/schematic/openapi.md` (R2-F1, R2-F2)
- final verification (macOS, after every change, no subagents running):
        - `just test` (schematic): 3,150 passed, 23 skipped; fresh-output 17/17 (includes the slow compiled round trip for R2-F1/R2-F2); runtime compile-fail 1/1. All findings
        - `just lint` (schematic): exit 0
        - `just test-schema`: 154/154. Covers the regenerated OpenAI client (R2-F2)
        - `just check-tier-coverage schematic`: 0 stranded
        - `git diff --check`: clean
        - four runtime TLS `builder::` tests ran over 5 s. They are the same TLS tests listed as slow before this cycle and untouched by it

## Implementation of Review Findings #3

> **started at:** 2026-10-23T21:17:04-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-20-type-constraints/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs began

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| -------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F3    | Unenforceable request rules do not consistently block generation | blocked | F3.3–F3.5, F3.7, F3.8 clean; F3.1, F3.2, F3.6 blocked: the one-line severity change is unambiguous, but with it Rust generation over `--api all` aborts on OpenAI (42 errors on `CreateResponse`, 113 after this cycle's wrapper rebuild) and three of the area's own L1 tests fail; every way to keep them green is either deleting OpenAI Rust generation or a route review-3 rules out (union evaluator, removing endpoints, warning exception), so the author must choose |
| R1-F3.1  | Root-component `anyOf` member rule → generation error at JSON body and multipart field | blocked | experiment at 21:19 (see Progress): flip → `validate --api openai` exit 1; `artifact_drift::generated_rust_artifacts_are_up_to_date`, `fresh_output_compile::slow_real_apis_compile_offline`, `live_definitions_have_no_embedded_queries_or_validation_gaps` fail; reverted. Severity unchanged (warning) pending the author's choice |
| R1-F3.2  | Root-component `oneOf` member rule → generation error at JSON body and multipart field | blocked | same line (`composition_findings`), same experiment and decision as R1-F3.1 |
| R1-F3.3  | Positional `items`/`prefixItems` (control) | clean | positional-items error tests (`a_rule_in_positional_items_is_a_generation_error_for_body_and_form_field_alike`, `output_validation::a_request_rule_in_positional_items_blocks_the_rust_client`) pass in the final `just test` (3,156 passed) |
| R1-F3.4  | Nested unions, two-member `allOf`, `not`, map-value union (control) | clean | `rules_beneath_one_of_all_of_and_not_are_located_but_a_one_member_all_of_is_walked_through` and the `value_rules` error tests pass; `value_rules` code unchanged this cycle except the reverted experiment |
| R1-F3.5  | Supported traversal: item object, nullable alternative, one-member `allOf` (control) | clean | direct controls pass; the rebuilt route now also keeps nullable alternatives and one-member `allOf` (R3-F1.A) |
| R1-F3.6  | Consumers: JSON body, multipart field, shipped-catalog expectation (`UNCHECKED_VALUE_RULES`) | blocked | follows R1-F3.1/F3.2. `UNCHECKED_VALUE_RULES` kept truthful under the current policy: now `CreateConversation` 13, `CreateResponse` 50, `Getinputtokencounts` 50 (113 warnings, all union members reached through rebuilt nullable references) |
| R1-F3.7  | Import disclosure of dropped subschemas (control) | clean | import disclosure tests pass in the final `just test` |
| R1-F3.8  | Other untraversed keywords (control) | clean | `rules_beneath_untraversed_keywords_are_located` passes in the final `just test` |
| R2-F2    | Rebuilding free-form schemas drops referenced constraints (remaining alias children) | fixed | remaining children F2.14/F2.15 now fail the `--definitions-out` import with a located error instead of losing the rule (the bounded rejection review-3 offered); controls clean; OpenAI re-import unaffected (byte-identical `types.rs`) |
| R2-F2.8  | Nominal typed reference `Item.owner` (control) | clean | `a_free_form_reference_to_a_model_with_rules_is_written_through_the_generator_at_each_position` still asserts `Item.owner` emits its `extend` const, and assembly returns `Ok` |
| R2-F2.9  | Downstream: rebuilt alias schemas → plans → preparation diagnose or preserve | fixed | a lost alias rule is now a generation error, so no rebuilt definition missing it reaches plans or preparation; public-path test `definitions_out_constraints::definitions_out_rejects_an_alias_body_rule_it_cannot_rebuild_and_writes_nothing` (in-process `run_import`, neither file written) |
| R2-F2.13 | Nonrecursive alias and top-level reference (control) | clean | nonrecursive `Tag` alias and top-level reference keep rules; `plain_alias_body_rules_become_constraints_but_nominal_bodies_do_not` (`Slug`) also asserts `finish()` is `Ok` |
| R2-F2.14 | Recursive plain alias through a free-form field: preserve or explicitly reject | fixed | rejected: `a_ruled_plain_alias_reached_again_through_a_free_form_value_fails_the_rebuild` (`RecursiveForest` via `Grove.trees`; error names alias, `$ref`, and the fix); a rule-free recursive alias still rebuilds. Sibling: an alias chain past `MAX_ALIAS_DEPTH` with rules is also reported |
| R2-F2.15 | JSON body naming a plain array alias over inline objects: preserve or explicitly reject | fixed | rejected: `a_plain_alias_body_with_a_rule_beneath_its_items_object_fails_at_the_rule` (pointer `/items/properties/label/maxLength`) and the `$ref`-item sibling; control `a_plain_alias_body_of_constrained_scalars_keeps_its_item_rules_as_constraints` |
| R3-F1    | Rebuilding free-form compositions loses supported traversal and generation errors | blocked | the spec-named supported wrappers are fixed and adopted (R3-F1.A); the four generic shapes (R3-F1.B) are blocked on the R1-F3 decision: retaining them gives 343 Rust errors on 26 OpenAI endpoints (iteration 2 measurement), which fails the same three L1 tests |
| R3-F1.1  | Free-form field root (`Item.mixed_value`): six composition shapes | blocked | nullable alternative and one-member `allOf` fixed (R3-F1.A evidence, `free_value`); generic `anyOf`/`oneOf`/two-member `allOf`/`not` blocked (R3-F1.B) |
| R3-F1.2  | Nested property (`mixed_value.inner`): six shapes | blocked | as R3-F1.1 at `free_value.properties.inner` |
| R3-F1.3  | Free-form items (`mixed_value.items`): six shapes | blocked | as R3-F1.1 at `free_value.items` |
| R3-F1.4  | Free-form extra values (`mixed_map.additionalProperties`): six shapes | blocked | as R3-F1.1 at `free_map` |
| R3-F1.5  | Map-valued model field (`mixed_attributes.additionalProperties`): six shapes | blocked | as R3-F1.1 at `attributes` |
| R3-F1.6  | Inline object array items (`mixed_entries.items`): six shapes | blocked | as R3-F1.1 at `entries` |
| R3-F1.7  | Flattened extra-value schema (`MixedExtras.additionalProperties`): six shapes | blocked | as R3-F1.1 at `struct_rules` on `Extras`/`MixedExtras` |
| R3-F1.8  | Unwrapped ordinary reference at the seven positions (control) | clean | slow e2e still finds the plain `owner` reference 7 times in both routes; `linked_value` plans unchanged |
| R3-F1.9  | Downstream: imported → emitted → registered → validator and generated plan | blocked | fixed for the supported wrappers (emitted `schema_with` helper asserted, rebuilt API 0 errors, identical direct/rebuilt findings and plans); generic shapes blocked (R3-F1.B) |
| R3-F1.10 | Direct planner, JSON body and multipart controls | clean | direct planner untouched; new controls confirm the direct import still errors for generic `anyOf`, `oneOf`, two-member `allOf`, `not` |
| R3-F1.A  | Discovered split: nullable alternative and one-member `allOf` rebuilt at all seven positions | fixed | `free_form_rules` keeps both in source shape; unit `transparent_wrappers_are_rebuilt_at_each_free_form_position_and_other_compositions_are_left_out`; compiled child refuses `{"name":"longer than ten"}` under both probes at all seven positions via `validate()`/`into_parts()`, accepts `short`/`null`; outer comparison requires 7+7 probe plans reaching `Owner` in both routes; mutation → rebuilt `[0, 7, 7, 0, 0]` vs `[6, 7, 7, 7, 7]`; adopted into OpenAI (0 errors, 133 helpers) |
| R3-F1.B  | Discovered split: generic `anyOf`, `oneOf`, two-member `allOf`, `not` retain structure so generation fails | blocked | depends on the R1-F3 author decision; preserving them aborts OpenAI Rust generation (343 errors, 26 endpoints); still left out, as `docs/io/import-openapi.md` states |
| R3-F1.C  | Discovered: baseline normalization read the vendored OpenAI document through a helper path literal (test-input index widens it to the whole binary) | fixed | literal moved into the `openai` `#[test]`; `definition_baseline` 31/31 |
| OBS-1    | Observation: registration order normalized by the baseline test (not repaired here) | deferred | observation, not repaired in a fix cycle; the baseline test changed this cycle (narrow wrapper normalization) still normalizes only order, and the departure stays recorded |

### Progress

- starting the work on 'R1-F3 root-union severity (policy experiment)' at 21:19:10
        - defect class: whether a request rule blocks Rust generation depends on where the unenforced rule sits (root-component union member vs any other unsupported position)
        - sibling sites checked: `composition_findings` (`gen/src/codegen/value_rules/mod.rs`) is the only place that downgrades a finding; `unenforceable_rules` feeds both the JSON body and the multipart JSON field through it, so one severity decision covers both entry points; positional items, nested unions, two-member `allOf`, `not`, map-value unions and the eleven untraversed keywords already give errors (controls F3.3, F3.4, F3.8)
        - experiment: made every union-member finding `Severity::Error` (one line), then ran `schematic-gen validate --api openai` → exit 1, 42 distinct `type-constraint` errors, all on `CreateResponse` (members of `BetaTextResponseFormatConfiguration`, `BetaToolChoiceParam`, `BetaTool`; 36 of 42 are the members' `type` tag `enum`s, typed `String` in Rust)
        - with the flip, `cargo nextest run -p schematic-gen --test l1 -E 'test(/^artifact_drift::/) | test(=fresh_output_compile::slow_real_apis_compile_offline) | test(live_definitions_have_no_embedded_queries_or_validation_gaps)'` → 3 of 11 fail: `artifact_drift::generated_rust_artifacts_are_up_to_date` and `fresh_output_compile::slow_real_apis_compile_offline` fail with `GeneratorError::Validation { output: Rust }` because Rust generation runs over `--api all` as one batch, and the catalog test fails on the new errors; `just generate` and `just check-drift` fail the same way
        - consequence: the flip cannot land with a green `just test` unless OpenAI's Rust client stops being generated, or one of the routes review-3 itself rules out is taken (a union evaluator, removing endpoints, a warning exception). Choosing between deleting the OpenAI Rust client from generation and those routes is a product decision, not an implementation ambiguity
        - experiment reverted (`gen/src/codegen/value_rules/mod.rs` restored from `/tmp/r3-vr-mod.rs.bak`; one `if union_member { Severity::Warning }` remains)
- work completed for 'R1-F3 root-union severity (policy experiment)' at 21:21:05
- starting the work on 'R3-F1 rebuild transparent composition wrappers' at 21:22:16
        - defect class: rebuilding an imported schema removed schema edges (transparent composition wrappers) before the generator could enforce the rules they reach; part A only (transparent wrappers), generic compositions stay left out (blocked on the author decision with R1-F3)
        - repair (`gen/src/model_gen.rs` `free_form_rules`): a two-member `anyOf` with exactly one `admits_only_null` member (same predicate as the direct planner's `schema_node`, imported from `schematic_define::constraints`) is kept in source shape, the non-null member rebuilt recursively and the null-only member copied verbatim; a one-member `allOf` without a sibling `$ref` (the planner follows the `$ref` instead) is kept with its member rebuilt. A wrapper whose member rebuilds to no rule is left out (no churn). Nested wrappers recurse. `RuleSource`/`ruled`/`free_form_rules` doc comments updated. Source shape chosen over "inner rules + `nullable: true`" so the direct and rebuilt plans are token-identical (proved by the unit and e2e plan comparisons)
        - sibling sites: all seven free-form positions go through `free_form_rules` (root, nested property, items, extra values via `rules`/`slot`; map-valued field and inline items via `rules` Array/Map → `slot`; flattened extras via `struct_rules` → `slot`) — fixed by the one branch; `$ref` directly under items/additionalProperties (`linked_value`) unchanged and still green; plain-alias branch (`rules`/`free_form_reference`) reaches `free_form_rules` for the alias target, so an alias holding a wrapper is covered by the same branch; `ruled` already counts beneath compositions (no change); `MODEL_REFERENCE` inside the `anyOf`/`allOf` arrays is emitted through `schema_value_tokens` bindings (`contains_model_reference` walks arrays) — no change needed
        - importer side: a property whose schema is an inline `anyOf [$ref X, null]` is typed `TypeRef::AnyOf` → `serde_json::Value` (`define/src/openapi/import/mappings/schema.rs` `map_schema_to_type_ref_inner`; `model_gen::type_ref_to_tokens`), not `Option<X>`, so it is a free-form route and is fixed by this branch (unit-tested as `wrapped`); a property that is a bare `$ref` stays nominal (`Option<X>`), unaffected
        - fixture: `nullable_probe` (`anyOf [{$ref: Owner}, {type: string, nullable: true, enum: [null]}]`) and `allof_probe` (`allOf [{$ref: Owner}]`) beside each of the seven inline `owner` references; header comment updated
        - unit tests: new `transparent_wrappers_are_rebuilt_at_each_free_form_position_and_other_compositions_are_left_out` (exact rebuilt rules at R3-F1.1–F1.7 incl. `struct_rules` for flattened extras, the importer-typed `wrapped` field, null-first order, nested `anyOf[allOf[ref]]`, inline-rule member, sibling `const`; controls generic `anyOf`, both-null `anyOf`, three-member `anyOf`, `oneOf` with null, two-member `allOf`, `not`, wrappers of a rule-free model, `allOf` beside a `$ref` all give `{}`; exact emitted `schema_with` helper with `anyOf` binding); `a_rebuilt_free_form_reference_gives_the_direct_import_s_findings_and_plans` extended with nullable alternative, nullable alternative as the value, one-member `allOf` beside `const`, nested wrappers (identical findings and plans), and controls (generic `anyOf`, `oneOf`, two-member `allOf`, `not`: rebuilt rules empty, direct import reports an error)
        - e2e (`slow_definitions_out_rebuilds_every_rule_through_registration`, no new compile): child `every_free_form_position_reaches_a_referenced_model_s_rule` now refuses `{"name":"longer than ten"}` under `owner`, `nullable_probe`, `allof_probe` at all seven positions with `maxLength` at `{at}.{probe}.name` via `validate()` and `into_parts()`, accepts `short` under all three and `null` under `nullable_probe`; outer plan comparison now requires 7 `nullable_probe` and 7 `allof_probe` plans reaching `COMPONENT_OWNER` in both routes (and the 6 mixed-object plans now include both probes)
        - note: the compiled child runs on the direct API's client; the rebuilt route is proven by the outer plan comparison, and the by-meaning child (`constraint_meaning.rs`) compares own rules only (references to object models are not part of its map), so it does not see lost wrappers
        - mutation check: `&& false` on both new branches → slow test FAILS at the outer plan comparison, rebuilt counts `[0, 7, 7, 0, 0]` vs `[6, 7, 7, 7, 7]`; restored from `/tmp/r3f1-model_gen.rs.bak` (verified no `&& false` left)
        - OpenAI temp re-import (`/tmp/r3f1-openai`): `types.rs` +2171/-3 lines vs working tree, `schema_with` helpers 19 → 133, 222 `anyOf` + 2 `allOf` added; `mod.rs` differs only in the known pre-existing `endpoint_183` `session` form field and `.version("2.3.0")`. Swapped in: `validate --api openai` exit 0, 0 errors; warnings: query-parameter 15 (unchanged), type-constraint 42 → 113 (CreateConversation 0→13, CreateResponse 42→50, Getinputtokencounts 0→50), every one a root-union-member warning ("a union whose members are not checked")
        - adopted: `just import-openai` (types.rs identical to the temp re-import), `mod.rs` restored from `/tmp/r3f1-mod.rs.worktree` (now equal to HEAD); `just generate` exit 0, 0 errors; changed `openapi/openai.json`, `schema/src/openai/requests.rs`, `postman/openai.postman_collection.json` (AsyncAPI unchanged). Side effect: Postman example bodies for format-only nullable fields changed from `{}` to `"string"` (e.g. `finished_at`, `expires_at`), because the rebuilt member keeps only `format` (type stripped, as for any free-form value)
        - `definition_baseline::openai` failed after adoption (78 component hashes, 58 slot shifts): a rebuilt `anyOf [{$ref}, null]` registers byte-identical to Schemars' `Option<Model>` (98 such top-level wrappers). Normalization extended narrowly: `rule_slot`/`rebuilt_wrapper` accept a nullable alternative or one-member `allOf` of rule slots beneath an untyped field (unambiguous: Schemars writes nothing there) and on array items; at a component's top-level property only when the importer types that field as free-form (`free_form_fields`: imports the vendored document, peels `Option`, `Json`/`Unknown`/`OneOf`/`AnyOf`/`AllOf`; an `allOf` model inherits its named members' fields because Schemars merges flattened members). Gate check: stripping wrappers on every property instead gives 135 hash differences, so Schemars' own `Option<Model>` is still compared strictly; restored
        - `output_validation::UNCHECKED_VALUE_RULES` set to `[("OpenAI","CreateConversation",13),("OpenAI","CreateResponse",50),("OpenAI","Getinputtokencounts",50)]` (still warnings; description unchanged and still true)
        - docs: `docs/io/import-openapi.md` (intro sentence, rebuild table row, "references and the two transparent wrappers are rebuilt, other compositions are not", new "Transparent wrappers" bullet with a kept YAML example, "Other compositions" bullet); `.claude/skills/schematic-define/schematic/openapi.md`; `.claude/skills/schematic-define/schematic/gen.md` (pinned warning counts)
        - checks: `cargo nextest run -p schematic-gen --lib -E 'test(model_gen) | test(value_rules)'` 45/45; `cargo nextest run -p schematic-gen --test l1 -E 'test(constraint_preservation) | test(definitions_out) | test(definition_baseline) | test(output_validation) | test(import_overlay) | test(/^artifact_drift::/)'` 74/74; `just test-fresh-output` 17/17; `cargo nextest run -p schematic-define --all-features -E 'binary(constraint_preservation) | binary(/openapi_constraint/) | binary(registered_model_rules)'` 50/50; `cargo nextest run -p schematic-definitions` 657/657; `just test-schema` 154/154; `just lint` exit 0, no warnings
        - not done (blocked, unchanged): generic compositions beneath free-form values (two-member `anyOf` without a null-only member, `oneOf`, `allOf` of 2+, `not`) stay left out, so the rebuilt route still neither checks nor reports their rules; depends on the author decision shared with R1-F3
- work completed for 'R3-F1 rebuild transparent composition wrappers' at 21:43:53
- orchestrator check after R3-F1 at 21:45:11
        - verified the claimed counts: `UNCHECKED_VALUE_RULES` now lists three OpenAI endpoints (13/50/50), still warnings; the 71 new warnings are root-union members reached through the newly rebuilt nullable references, the same R1-F3 policy question
        - fixed a test-input placement defect in the subagent's baseline change: the vendored `specs/openai/openapi.yaml` path literal sat in the helper `value_rule_source`, which CI's test-input index maps to the whole `l1` binary; the literal now sits in the `openai` `#[test]` and reaches `free_form_fields` through `assert_baseline_from`/`historical_differences_from`; the helper is gone
        - `cargo nextest run -p schematic-gen --test l1 -E 'test(definition_baseline)'` → 31/31
- starting the work on 'R2-F2 alias rebuild limits' at 21:45:43
        - defect class: rebuilding an imported definition with `--definitions-out` silently dropped value rules the rebuilt Rust form cannot carry (a ruled plain alias reached again through a free-form value; a JSON body typed as a plain alias whose rules reach past root/`items` rules). Repair is a focused rejection, not alias registration or a body-schema carrier
        - repair (`gen/src/model_gen.rs`): `RuleSource` gains a `problems` collector and `finish() -> Result<(), GeneratorError>` (`CodeGenError` listing each problem); `assemble_model_tokens_with` now returns `Result` and calls `finish` (models assembled by a new private `assemble_models`; `assemble_model_tokens` unchanged), so `generate_models_with` fails before writing `types.rs`
        - R2-F2.14: `free_form_reference` records a problem when a ruled plain alias is referenced while already in `expanding` (names the alias, the `$ref`, the alias being rebuilt, and the fix: give the item/object schema a component name). A recursive alias without rules is not `ruled` and still rebuilds
        - R2-F2.15: `body_constraints(endpoint, reference)` round-trips the body rules through `Constraints::from_schema`/`to_schema` and records a problem at the first rule not carried (new `uncarried_rule`, JSON pointer under `#/components/schemas/{alias}`, `$schematicModel` shown as `$ref`); scalar and `items` rules `Constraints` holds still pass. `definition_gen::generate_definition_module` calls `rules.finish()` before writing `mod.rs`; new `definition_gen::check_alias_bodies` runs in `import_pipeline` before either file is written
        - sibling sites: `rules` `MAX_ALIAS_DEPTH` branch (alias chain >32 deep) now records a problem when the alias or occurrence has rules — fixed (unreachable for compilable Rust; cheap); `base_schema` depth branch is reached only after `rules` hit the same branch — covered; `field_rules`/`struct_rules`/`model_attribute` reach the recursion only through `free_form_reference` — covered; nominal references (`Item.owner`) and nonrecursive alias references unchanged — clean
        - tests: `model_gen::tests` `a_plain_alias_body_with_a_rule_beneath_its_items_object_fails_at_the_rule` (Forest, pointer `/items/properties/label/maxLength`), `a_plain_alias_body_reaching_a_model_with_rules_through_a_free_form_item_fails_at_the_reference` (`/items/$ref`), `a_plain_alias_body_of_constrained_scalars_keeps_its_item_rules_as_constraints`, `a_ruled_plain_alias_reached_again_through_a_free_form_value_fails_the_rebuild` (RecursiveForest via `Grove.trees`, plus unruled-recursion control), `plain_alias_body_rules_become_constraints_but_nominal_bodies_do_not` now also asserts `finish` is Ok; `referencing_catalog` drops its `Tree` self-reference (it asserted the old silent truncation) and `a_free_form_reference_to_a_model_with_rules_is_written_through_the_generator_at_each_position` keeps the Tag alias, top-level reference, and `Item.owner` `extend` controls (R2-F2.13, R2-F2.8) with assembly now returning Ok. Public path: `l1 definitions_out_constraints::definitions_out_rejects_an_alias_body_rule_it_cannot_rebuild_and_writes_nothing` (in-process `run_import`, asserts located message and no `types.rs`/`mod.rs`)
        - docs: `docs/io/import-openapi.md` "Also not rebuilt" list replaced by the two rejected shapes, sample error, and the naming workaround with YAML; `.claude/skills/schematic-define/schematic/openapi.md` limit line updated
        - OpenAI re-import to /tmp/r2f2-openai (justfile `import-openai` args): succeeds (258 endpoints, 1394 models, 0 errors); `types.rs` byte-identical to committed `definitions/src/openai/types.rs`; `mod.rs` differs from the committed one in pre-existing, unrelated ways (endpoint_183 `session` form field as `json_desc`, `.version("2.3.0")`) that this change cannot produce (it emits no new tokens)
        - `cargo nextest run -p schematic-gen --lib -E 'test(model_gen) | test(value_rules)'` → 49/49; `--lib -E 'test(definition_gen) | test(import_pipeline)'` → 20/20
        - `cargo nextest run -p schematic-gen --test l1 -E 'test(constraint_preservation) | test(definitions_out) | test(import_overlay) | test(definition_baseline) | test(module_naming) | test(openapi_import_test)'` → 58/58
        - `just lint` (schematic/) → exit 0, no warnings
- work completed for 'R2-F2 alias rebuild limits' at 21:52:36
- reconciliation and phase gates (orchestrator) at 21:57:20
        - inspected the combined `free_form_rules` change (two-member `anyOf` with exactly one `admits_only_null` member kept with the null member verbatim; one-member `allOf` kept only without a `$ref` beside it, matching the planner's precedence) and the `RuleSource::problems`/`finish()` rejection path, which fails `generate_models_with`, `generate_definition_module`, and `import_pipeline` (`check_alias_bodies`) before either file is written
        - `assemble_model_tokens_with` now returns `Result`; `assemble_model_tokens` keeps its signature
        - the OpenAI `mod.rs` re-import differences (`endpoint_183` form field, `.version`) are the same pre-existing ones iteration 2 recorded; the working-tree `mod.rs` was restored and is unchanged by this cycle
        - cross-OS check skipped: schema rebuilding, code generation, and tests only; no path, process, or OS-specific code
- work completed for 'reconciliation and phase gates' at 21:57:25

### Completion Report

- **outcome: blocked**: elapsed 40 min (21:17:04 → 21:57:25). Every piece of work that does not depend on the author's decision is done. Everything still open depends on that one decision
- parent findings: total 3 = fixed 1 (R2-F2) + already fixed 0 + blocked 2 (R1-F3, R3-F1) + deferred 0 + unresolved 0 + disputed 0
        - R3-F1 is counted as blocked, not fixed. Its spec-named supported wrappers (R3-F1.A) are fixed and adopted, but the generic shapes (R3-F1.B) are open
        - OBS-1 is an observation, not a parent finding
- verified repairs:
        - R3-F1.A: rebuilt free-form values now keep nullable alternatives and one-member `allOf`, in their source shape, at all seven positions. A reference wrapped in either one keeps its model's rules. Compiled requests refuse violations at the exact path and accept `null` where the wrapper allows it. The direct and rebuilt plans are identical. The change is adopted into the OpenAI definitions: 0 errors and 133 `schema_with` helpers, with all artifacts regenerated
        - R2-F2.14/F2.15: when a recursive plain alias or a plain-alias JSON body would lose a rule, the `--definitions-out` rebuild now fails with a located error and writes nothing. No live definition is affected
        - R1-F3: an experiment showed what the required policy costs. The severity line itself was left unchanged
- why R1-F3 and R3-F1.B stay blocked, despite review-3 saying no author decision is needed:
        - the implementation itself is clear. The problem is its consequence: Rust generation runs over `--api all` as one batch, so a single OpenAI error stops every client from generating
        - making the 42 (now 113) root-union warnings into errors, or keeping the generic compositions (343 errors), fails three of the area's own L1 tests. The task forbids finishing with macOS failures
        - review-3 rules out every route that keeps those tests green while generating OpenAI: a union evaluator, removing endpoints, or a warning exception. The only remaining route is dropping the OpenAI Rust client from generation, and that is a product decision for the author
- open items:
        - **R1-F3.1, R1-F3.2, R1-F3.6: blocked.** Next action: the author picks one of the following, and the next cycle implements it, including the catalog expectation:
                - (a) stop generating the OpenAI Rust client
                - (b) discriminated-union member selection
                - (c) a documented spec exception for union-member rules
        - **R3-F1.1–F1.7, R3-F1.9, via R3-F1.B: blocked.** Generic `anyOf`, `oneOf`, two-member `allOf`, and `not` beneath free-form values are still left out of rebuilt definitions. Next action: retain them once that decision allows OpenAI to carry the resulting errors
- changed files (this cycle):
        - gen: `gen/src/model_gen.rs` (R3-F1.A, R2-F2.14/F2.15), `gen/src/definition_gen.rs` and `gen/src/import_pipeline.rs` (R2-F2.15, no-write-on-failure)
        - gen tests: `gen/tests/l1/constraint_preservation.rs` (R3-F1.A), `gen/tests/l1/definition_baseline.rs` (R3-F1.A adoption, R3-F1.C), `gen/tests/l1/output_validation.rs` (R1-F3.6 truthful counts), `gen/tests/l1/definitions_out_constraints.rs` (R2-F2.9/F2.15)
        - define: `define/tests/fixtures/constraint_preservation.yaml` (R3-F1 probes at seven positions)
        - definitions and artifacts: `definitions/src/openai/types.rs`, `schema/src/openai/requests.rs`, `openapi/openai.json`, `postman/openai.postman_collection.json` (R3-F1.A adoption)
        - docs and skills: `docs/io/import-openapi.md`, `.claude/skills/schematic-define/schematic/openapi.md`, `.claude/skills/schematic-define/schematic/gen.md` (R3-F1.A, R2-F2)
- final verification (macOS, after every change, no subagents running):
        - `just test` (schematic): 3,156 passed, 23 skipped. Fresh-output 17/17 (includes the compiled rebuild for R3-F1.A). Runtime compile-fail 1/1. Covers all findings
        - `just lint` (schematic): exit 0
        - `just test-schema`: 154/154 (regenerated OpenAI client, R3-F1.A)
        - `just check-tier-coverage schematic`: 0 stranded
        - `git diff --check`: clean
        - the four runtime TLS `builder::` tests ran over 5 s. They are the same TLS tests recorded as slow in earlier cycles, and this cycle did not touch them

## Implementation of Review Findings #4

> **started at:** 2026-10-23T22:04:48-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-20-type-constraints/review-4.md'
- this is iteration 4 of the review-to-implement cycle 
- the review's `implemented` flag was confirmed `false` before repairs began

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| -------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F3    | Unenforceable request rules do not consistently block generation | blocked | F3.3–F3.5, F3.7, F3.8 clean (review-4 re-verified); F3.1, F3.2, F3.6 blocked on a spec conflict only the author can resolve (see Progress 22:05–22:09): under the refusal policy the OpenAI Rust client cannot be generated by any route, and removing it conflicts with spec Goal 4 |
| R1-F3.1  | Root-component `anyOf` member rule → generation error at JSON body and multipart field | blocked | flip experiment 22:08 on current code: `validate --api openai` exit 1, 113 distinct `type-constraint` errors (`CreateConversation` 13, `CreateResponse` 50, `Getinputtokencounts` 50); reverted, `cmp` with backup identical |
| R1-F3.2  | Root-component `oneOf` member rule → generation error at both entry points | blocked | same severity branch (`composition_findings`), same experiment and decision as R1-F3.1 |
| R1-F3.3  | Positional `items`/`prefixItems` (control) | clean | review-4 independent probe: error at both entry points; code unchanged this cycle |
| R1-F3.4  | Nested unions, two-member `allOf`, `not`, map-value union (control) | clean | review-4 independent probe: error for each at both entry points; code unchanged |
| R1-F3.5  | Supported traversal: item object, nullable alternative, one-member `allOf` (control) | clean | review-4 reran planner and compiled preservation tests; code unchanged |
| R1-F3.6  | Consumers: public validator, shipped `validate --api openai`, catalog expectation `UNCHECKED_VALUE_RULES` | blocked | follows R1-F3.1/F3.2; expectation still truthful for the current (warning) policy: 13/50/50 |
| R1-F3.7  | Import disclosure (control) | clean | review-4 reran disclosure/parse-error tests; code unchanged |
| R1-F3.8  | Other untraversed keywords (control) | clean | review-4 probe: error for all eleven keywords at both entry points; code unchanged |
| R3-F1    | Rebuilding free-form compositions loses generation errors | blocked | R3-F1.A, .C fixed (review-4 verified); R3-F1.8, .10 clean; R3-F1.1–.7 and .9 (generic portion, R3-F1.B) blocked on the same decision as R1-F3: retaining the four generic shapes makes the rebuilt OpenAI definitions report the errors the direct import reports, and OpenAI Rust generation stops |
| R3-F1.1  | Free-form field root `Item.mixed_value`: four generic shapes | blocked | wrappers fixed (R3-F1.A); generic shapes → R3-F1.B |
| R3-F1.2  | Nested property `mixed_value.inner` | blocked | as R3-F1.1 |
| R3-F1.3  | Free-form items `mixed_value.items` | blocked | as R3-F1.1 |
| R3-F1.4  | Free-form extra values `mixed_map.additionalProperties` | blocked | as R3-F1.1 |
| R3-F1.5  | Map-valued model field `mixed_attributes.additionalProperties` | blocked | as R3-F1.1 |
| R3-F1.6  | Inline object array items `mixed_entries.items` | blocked | as R3-F1.1 |
| R3-F1.7  | Flattened extra values `MixedExtras.additionalProperties` | blocked | as R3-F1.1 |
| R3-F1.8  | Ordinary-reference controls at all seven positions | clean | review-4 compiled acceptance and plan comparisons pass; code unchanged |
| R3-F1.9  | Downstream: imported → emitted → registered → validator and plans | blocked | wrapper portion fixed; generic portion → R3-F1.B |
| R3-F1.10 | Direct planner, JSON body and multipart controls | clean | review-4: direct unsupported compositions error, wrappers produce checks; code unchanged |
| R3-F1.A  | Supported wrappers rebuilt at all seven positions | already fixed | fixed in iteration 3; review-4 verified through `validate()`/`into_parts()` and plan comparison |
| R3-F1.B  | Generic `anyOf`, `oneOf`, multi-member `allOf`, `not` retain enough structure for refusal | blocked | the code repair is small (keep the composition with its members rebuilt in `free_form_rules`), but landing it is the same decision: iteration 2 measured 343 Rust errors on 26 OpenAI endpoints with retention |
| R3-F1.C  | Baseline repository-input literal inside the `openai` `#[test]` | already fixed | fixed in iteration 3; review-4 verified |

### Progress

- starting the work on 'R1-F3 and R3-F1 shared blocker re-assessment' at 22:05:10
        - defect class (R1-F3): where an unenforceable request rule sits decides whether generation fails or only warns
        - defect class (R3-F1): rebuilding a definition removes schema edges before the generator's refusal decision can see the rules beneath them
        - both findings end in the same place: once the spec's policy holds, OpenAI's Rust output reports generation errors. So one decision governs both, and one investigation covers both. No per-finding subagent was launched, because no code repair can land until that decision is made. That is also why the per-finding repair loop was not run
        - review-4's argument was checked against the spec: "Generation must fail for a supported request rule it cannot enforce" and "A constrained request beneath an unsupported composition ... gets an actionable generation error" are decided. The review says the remaining repair is refusal plus matching test expectations, and that a negative generation test can prove the refusal
        - new evidence 1, no narrower route exists: a classification of the vendored `specs/openai/openapi.yaml` request bodies (script `/tmp/r4/an.py`) finds rule-bearing compositions at 270 discriminated `oneOf` sites, 207 untagged `oneOf` sites over 35 endpoints, 100 untagged `anyOf` sites over 30 endpoints, and 14 multi-member `allOf` sites over 7 endpoints, including `createChatCompletion` and `createResponse`. A discriminated-union traversal, the only exact extension that stays short of generic union evaluation, would leave more than 300 untagged sites as errors. So under the policy, no implementation keeps OpenAI's Rust client
        - new evidence 2, the current-code flip: with every union-member finding made `Severity::Error`, `cargo run -p schematic-gen -- validate --api openai` exits 1 with 113 distinct errors (`CreateConversation` 13, `CreateResponse` 50, `Getinputtokencounts` 50; log `/tmp/r4/flip.log`). Retaining the generic compositions adds the 343 errors iteration 2 measured. The file was restored from `/tmp/r4/vr-mod.rs.bak`, and `cmp` confirms it is identical
        - new evidence 3, the conflict inside the spec: the one implementation that satisfies the policy and keeps `just test` green stops generating the OpenAI Rust client (`schema/src/openai`, 258 endpoints). It also has to drop or rewrite the schema tests that use that client (`pagination.rs` cursor pager, `streaming_providers.rs`, `value_rules.rs`, two compile-fail fixtures) and the gen tests (`fresh_output_compile`, `artifact_drift`, `output_validation`). That conflicts with spec Goal 4, "Preserve the existing fluent setters, response decoding, and pagination controls" (OpenAI's `list_batches().page_size(..).collect_all()` is one of those controls). Review-4 itself states that it "does not request ... deleting the committed OpenAI client", and that if all-catalog generation is mandatory "the author must revise the specification". Whether the client must be kept is a product requirement the spec does not settle, and both of its outcomes are a spec-level choice
        - conclusion: R1-F3.1/.2/.6 and R3-F1.B stay **blocked**. This is not a reading of the policy as undecided. The decided policy and Goal 4 cannot both hold for the shipped OpenAI catalog, and only the author can say which one gives way. No production or test code was changed this cycle
        - author decision needed, with what each option means for the next cycle:
                - (a) stop generating the OpenAI Rust client: flip the severity (one line in `composition_findings`), keep generic compositions in `free_form_rules`, re-import OpenAI, exclude OpenAI from Rust generation (keeping its OpenAPI and Postman outputs), delete `schema/src/openai` and its feature, add negative generation tests for the JSON body and multipart entry points, and port or drop the OpenAI-based pagination, streaming, and compile-fail tests
                - (c) revise the spec: allow a documented exception, for example warning-only for a union whose members are not checked, or per-API acknowledgement of unchecked rules. The current behavior then becomes the specified behavior, and R3-F1.B gets the same exception
                - (b) discriminated-union selection is not enough on its own, as evidence 1 shows
- work completed for 'R1-F3 and R3-F1 shared blocker re-assessment' at 22:09:43

### Completion Report

- **outcome: blocked**: elapsed about 6 min (22:04:48 → 22:10:12). Every remaining child depends on one author decision. No other work is open
- parent findings: total 2 = fixed 0 + already fixed 0 + blocked 2 (R1-F3, R3-F1) + deferred 0 + unresolved 0 + disputed 0
        - child rows: R1-F3.3/.4/.5/.7/.8, R3-F1.8, and R3-F1.10 are clean. R3-F1.A and R3-F1.C are already fixed (fixed in iteration 3, verified by review-4). R1-F3.1/.2/.6, R3-F1.1–.7, R3-F1.9, and R3-F1.B are blocked
- why blocked although review-4 says there is no blocker: this cycle found new evidence (Progress, evidence 1–3) that no implementation satisfies both the refusal policy and spec Goal 4 for the shipped OpenAI catalog. The only policy-compliant route deletes the OpenAI Rust client, and review-4 explicitly does not request that. The reviewer recognized the same fork ("the author must revise the specification") but did not take into account that Goal 4 already ties the spec to keeping existing setters and pagination controls
- open items, all with the same next action:
        - **R1-F3.1, R1-F3.2, R1-F3.6** (root-union severity and catalog expectation) and **R3-F1.1–F1.7, R3-F1.9 via R3-F1.B** (keep generic compositions when rebuilding)
        - next required action: the author chooses (a) stop generating the OpenAI Rust client, or (c) revise the spec with a documented exception. The next cycle then implements that choice, using the implementation outline in Progress. Human review is recommended: four review cycles have now returned this same finding
- changed files (this cycle): `features/2026-10-20-type-constraints/log.md` and the frontmatter of `review-4.md`. No production, test, fixture, generated, or documentation file changed. The temporary severity flip in `gen/src/codegen/value_rules/mod.rs` was reverted, and `cmp` with the backup confirms it is identical
- verification:
        - `cargo run -p schematic-gen -- validate --api openai` with the flip applied: exit 1, 113 errors (R1-F3.1/.2 blocker evidence). After the revert, the code is the same tree review-4 validated (exit 0, 113 warnings)
        - `python3 -I /tmp/r4/an.py` over the vendored OpenAI document: the composition counts in evidence 1 (R1-F3, R3-F1.B)
        - `just lint` (schematic): exit 0
        - `just test` was not rerun: no source changed since review-4's run on this tree (3,156 passed, 23 skipped; fresh-output 17/17)
        - cross-OS check: not applicable, because no code changed

## Implementation of Review Findings #5

> **started at:** 2026-10-24T15:10:37-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-20-type-constraints/review-5.md'
- this is iteration 5 of the review-to-implement cycle 
- the review's `implemented` flag was confirmed `false` before repairs began
- the blocker recorded in iterations 4 and 5 is removed by the author's 2026-10-24 ruling (spec, "Unchecked-composition opt-in", commit `a891d01fd`): an explicit, API-level opt-in turns covered composition rules into disclosed warnings; without it they are generation errors. Both findings are therefore unblocked this cycle

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| -------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F3    | Unenforceable request rules do not consistently block generation | pending | |
| R1-F3.1  | Component-root `anyOf` rule → error at JSON body and multipart JSON field without the opt-in; disclosed warning with it | pending | |
| R1-F3.2  | Component-root `oneOf` rule → same as R1-F3.1 | pending | |
| R1-F3.3  | Positional `items`/`prefixItems` stay errors even with the opt-in (control) | pending | |
| R1-F3.4  | Nested unions, multi-member `allOf`, `not`, map-value union → error without opt-in, warning with it | pending | |
| R1-F3.5  | Supported traversal (item object, nullable alternative, one-member `allOf`) still checked (control) | pending | |
| R1-F3.6  | Consumers: public validator, shipped `validate --api openai`, catalog expectations, output gate | pending | |
| R1-F3.7  | Import disclosure (control) | pending | |
| R1-F3.8  | Other unsupported keywords stay errors even with the opt-in (control) | pending | |
| R1-F3.9  | Opt-in metadata: absent by default, fluent `RestApi` builder, serialized, bundle and emitted-definition round trip | pending | |
| R1-F3.10 | Opt-in warnings name API, endpoint, path, keyword, composition; generated rustdoc discloses unenforced rules | pending | |
| R1-F3.11 | Opt-in never relaxes unresolved references or definition errors (malformed/contradictory) | pending | |
| R1-F3.12 | OpenAI overlay `openai_pagination.rs` sets the opt-in; import never overwrites it; client regenerates | pending | |
| R3-F1    | Rebuilding free-form compositions loses generation errors | pending | |
| R3-F1.1  | Free-form field root `Item.mixed_value`: four generic shapes kept | pending | |
| R3-F1.2  | Nested property `mixed_value.inner` | pending | |
| R3-F1.3  | Free-form items `mixed_value.items` | pending | |
| R3-F1.4  | Free-form map values `mixed_map.additionalProperties` | pending | |
| R3-F1.5  | Map-valued model field `mixed_attributes.additionalProperties` | pending | |
| R3-F1.6  | Inline object array items `mixed_entries.items` | pending | |
| R3-F1.7  | Flattened extra values `MixedExtras.additionalProperties` | pending | |
| R3-F1.8  | Ordinary-reference controls at all seven positions | pending | |
| R3-F1.9  | Downstream: rebuilt API reports the same errors (no opt-in) / warnings (opt-in) as direct import | pending | |
| R3-F1.10 | Direct planner, JSON body and multipart controls | pending | |
| R3-F1.A  | Supported wrappers rebuilt at all seven positions | pending | |
| R3-F1.B  | Generic `anyOf`, `oneOf`, multi-member `allOf`, `not` retained when rebuilding | pending | |
| R3-F1.C  | Baseline repository-input literal inside its test | pending | |

### Progress

- starting the work on 'R1-F3: Unenforceable request rules do not consistently block generation' at 15:12:26
        - defect class: whether an unenforceable composition rule fails generation depends on where the composition sits (component root versus anywhere else) rather than on an explicit, stored author choice
        - ordering: R1-F3 lands first because R3-F1's retention adds composition rules to the OpenAI catalog, which only stays generable once the opt-in exists
        - builder name chosen (the plan did not fix one): `RestApiBuilder::accept_unchecked_compositions()`, stored as `RestApi::unchecked_compositions: bool` (omitted when false)
        - delegated to a foreground subagent; it appends its own sub-bullets below
