---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): Rebuilding imported definitions loses supported constraints"
  - "R1-F2 (high): Invalid constraints are accepted or discarded instead of rejected"
  - "R1-F3 (high): Unenforceable request rules do not consistently block generation"
  - "R1-F4 (high): Pruning declared properties applies extra-property rules to valid fields"
  - "R1-F5 (medium): Nullable plans bypass explicit enum and const restrictions"
observations:
  - "Allowed exclusions and optional measurements remain deferred"
human_in_the_loop: null
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-23T16:26:45-07:00"
spec: 2026-10-20-type-constraints/spec.md
implemented: false
description: "A **feature** review of `2026-10-20-type-constraints/spec.md`"
feature: 2026-10-20-type-constraints/review-1.md
next: 2026-10-20-type-constraints/review-2.md
log: /Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-20-type-constraints/log.md
implemented_by: claude/opus
---

# Review of type constraints

The feature is **not ready for production**. Four high-priority findings violate the specification's preservation, definition-validation, generation-failure, or request-validation contracts. One medium-priority finding concerns an unusual nullable schema. No finding requires a human decision before repair.

Two independent reviewers examined the specification and implementation log first. Neither found a blocking decision under this review's rules. The three questions in the spec already have author rulings. This review therefore follows course 2: record choices, then review the implementation. This is the first implementation review; there are no earlier findings to carry forward.

## Decisions adopted for author review

These choices do not need confirmation to continue. Where an implementation choice contradicts an explicit requirement, the finding below identifies the necessary repair rather than presenting the contradiction as an authorized default. Sources are the specification's named sections and the implementation log's phase-specific decision and review-agenda sections.

| Choice adopted or recorded | Alternative and reason |
| --- | --- |
| Validate serialized JSON and send that same representation; parameters keep typed checks. Standalone validation, optional call build, and await are separate preparations. | Typed model traversal can disagree with Serde; model-owned validation would duplicate rules. This follows the author's ruling. The temporary JSON tree changes object-key order, widens `f32`, cannot carry every `i128`/`u128`, and collapses overlapping flattened keys; these representation effects are recorded in the log, rather than promised away. |
| Formats remain annotations, with no format-checker dependencies. | Heuristics can reject legitimate provider inputs. Numeric-width formats do not need repeated warning messages, but field documentation can still identify them as annotations. Losing source formats during definitions rebuilding is separately included in R1-F1. |
| Use exact decimal text and `num-bigint`, with a normalized decimal exponent bound of ±10,000. | Floating-point tolerance would change equality and divisibility. Enabling arbitrary-precision JSON alone would not prevent OpenAPI parser narrowing. Unsupported numeric limits must fail with a location. |
| Use `x-schematic-exact` to preserve limits that an ordinary JSON number cannot hold exactly. Reuse the existing YAML scalar visitor and the existing OpenAPI library. | A new parser or YAML dependency was unnecessary. Oversized integer bounds in YAML flow mappings can fail with a location rather than silently narrow; the spec permits a located failure. |
| Keep registered schemas authoritative; use metadata for parameters, inline roots, form fields, and rebuilding imports. Do not add an editable constraint list to registered fields. | Two editable sources could disagree. The body builder method is `request_constraints`, avoiding confusion with parameter constraints. |
| Expose `Result<(), Violations>` from constrained generated requests; preserve existing transport and argument error shapes. | Structured details are available from the request without changing every send result. Shared runtime helpers avoid validation dependencies in generated crates. |
| Keep plans static and patterns in separate lazy static objects; reparse numeric limit text and use pairwise array uniqueness checks. | Speculative numeric caches and faster uniqueness machinery add maintenance without measured need. Regex shorthand classes are translated to ASCII meanings while Unicode matching remains enabled. |
| Preserve valid unsupported imported patterns with `pattern_unenforced`; reject unsupported authored patterns. A later `.pattern()` clears the imported marker. | Rejecting every imported pattern would exceed the chosen subset policy. Invalid source patterns still require accurate classification; R1-F2 records a counterexample. |
| Serialize bundle numeric limits as strings, use existing snake_case keys, and omit absent metadata. Reject empty format names and incompatible authored rules. | JSON-number limits could lose precision. Boxing metadata and large error details avoids the observed stack and enum-size problems while retaining plain builder arguments. |
| Reject constrained WebSocket handshake inputs rather than implement new WebSocket checks. | WebSocket validation is explicitly outside scope; silently accepting unchecked metadata would be misleading. |
| Keep pager checks after choosing each effective size, using a shared numeric evaluator without leaking static strings. Adopted parameters carry no bounds or exactly the strategy's bounds, and no other assertions. | Original-request validation cannot check future pager settings. The existing pager error wording and controls remain. OpenAI's imported minimum-zero size parameters are explicitly aligned to its existing minimum-one strategy. |
| Skip rules genuinely guaranteed by Rust types, such as integer-width bounds and ordinary enum/tag values. | Rechecking them adds serialized-tree work without protection. This does not authorize skipping independently authored rules on arbitrary serialized JSON. |
| Report unsupported component patterns once per API, with each affected request documenting the omission. Use Rust names for parameter paths and serialized names for body/form paths. | Repeating the same component diagnostic for every endpoint is noisy. Map keys remain redacted. |
| Do not reserve `validate` as a setter name; setters use `with_*`, and a Rust field can share a method name. Raise the actual runtime ABI from 7 to 8 once and regenerate immediately. | Neither an extra name restriction nor a second ABI bump was necessary. |
| Check path, query, header, cookie, then text and JSON body values; serialization failures remain preparation errors. | The implementation can aggregate value violations without inventing violations for a failed serializer. The log also records following `$ref` alone at an unusual level that additionally contains nullable `anyOf`; this combination is not produced by supported Schemars output. |
| Emit constrained typed path parameters without also migrating the previously omitted description-only path parameters. | Migrating all 222 omitted OpenAI parameters would change unrelated generated request ergonomics. No new behavior is required for those unconstrained parameters here. |
| Use Schemars `extend` attributes to rebuild supported imported field rules; field-local rules take precedence over alias rules. | Native attributes cannot express all supported keywords and can silently drop mismatches. The exclusions for maps, free-form JSON, named values, and formats are implementation gaps, not spec-authorized deferrals: see R1-F1. Widening enum/constant null membership also needs care: see R1-F5. |
| Record the choice to downgrade union-component rules to warnings, which keeps OpenAI `CreateResponse` generatable with 42 unchecked rules. | The spec explicitly requires failure when supported request rules cannot be enforced. A warning is useful disclosure but does not satisfy that requirement; see R1-F3. This is reversible implementation policy, not a blocking author decision. |
| Record permissive import's removal of invalid/unrepresentable input rules with warnings; strict mode rejects warnings. | Preserving a usable partial import may help exploration, but removing contradictory supported constraints violates the required rejection contract: see R1-F2. Unsupported metadata outside this feature's contract remains a separate limitation. |
| Use wiremock's received-request count to establish zero sends; regenerate artifacts through the existing recipes. | A second transport harness was unnecessary. Existing metadata redaction, response decoding, and mock-provider isolation remain the contracts to verify. |
| Omit the optional request-validation timing sample and generated-plan compile-time measurement. | The spec does not require either measurement. Optimize only when a measured cost justifies it. |

## Findings

### R1-F1 (high): Rebuilding imported definitions loses supported constraints

**Authority — spec, “One source for model rules”:** “Imported Rust definitions must rebuild equivalent rules through Schemars attributes where expressible and explicit schema metadata where attributes cannot express them.” The same section says direct import succeeding is insufficient when `--definitions-out` and assembly lose the rule. **Defect class:** rebuilding schemas filters supported rules according to the emitted Rust type instead of preserving their meaning.

In schematic-gen, [RuleSource::rules](../../gen/src/model_gen.rs:322) returns an empty rule set for maps, free-form JSON, and non-alias named types, and [rule_keywords](../../gen/src/model_gen.rs:388) deletes every format annotation. [Struct generation](../../gen/src/model_gen.rs:159) supplies rules to ordinary fields but not to the flattened extra-properties field or the model root.

Reproduction: extend the real `constraint_preservation.yaml` fixture's `Item.properties` with `attributes: {type: object, additionalProperties: {type: string, maxLength: 3}}`, `free_value: {maxLength: 3}`, and `identifier: {type: string, format: uuid}`. Import through `OpenApiImport`, run public `run_import` with `definitions_out`, compile the emitted module in the existing fresh-output harness, call its `module_group()`, and inspect the registered `Item` schema. All three source keywords exist after direct import and disappear after rebuilding. The existing renamed `display-name.maxLength` survives as a positive control.

| Child ID | Site / shape | Observed result | Expected result |
| --- | --- | --- | --- |
| R1-F1.1 | Map-valued property, `additionalProperties.maxLength: 3` | Rule disappears after rebuilding | Preserve map-value rule |
| R1-F1.2 | Free-form JSON property, `maxLength: 3` | Rule disappears after rebuilding | Preserve rule, applying it when the submitted value is a string |
| R1-F1.3 | String property's source `format: uuid` | Annotation disappears after rebuilding | Preserve annotation without enforcing it |
| R1-F1.4 | Map field's whole-value enum | Direct import retains the enum; compiled rebuild loses it | Preserve whole-value rule |
| R1-F1.5 | Struct root `const` | Direct import retains the constant; compiled rebuild loses it | Preserve model-root rule |
| R1-F1.6 | Struct's flattened `additionalProperties.maxLength` | Direct import retains the rule; compiled rebuild loses it | Preserve extra-value rule |
| R1-F1.7 | Named struct field with adjacent `const` | Direct import retains the constant; compiled rebuild loses it | Preserve occurrence-specific rule |
| R1-F1.8 | Inline object array item's `const` | Direct import retains the constant; compiled rebuild loses it | Preserve item rule when the importer assigns the object a model name |
| R1-F1.9 | Primitive field, renamed field, scalar alias, primitive-array item rules, inline body roots, parameter/form metadata | Existing round-trip fixture and compiling acceptance retain these rules | Clean controls; keep these working |

A second compiled rebuild probe swept non-alias named fields and array items, flattened `additionalProperties`, and whole-model constants, confirming the additional losses listed above. The enum and const readers converge on the same whole-value metadata path; preserve both rather than fixing only the demonstrated constant. Generic composition evaluation and binary-content interpretation are excluded; preservation of supported assertions in otherwise supported schema positions is not. Add these positions to the compiled rebuild fixture rather than merely asserting the emitted attribute text. Use explicit schema metadata where an attribute cannot represent the source rule.

### R1-F2 (high): Invalid constraints are accepted or discarded instead of rejected

**Authority — spec, “Definition errors versus unsupported rules”:** “Reject malformed keyword values, negative lengths or counts, nonpositive `multipleOf`, incompatible authored types, empty enums, contradictory intervals, and lower bounds above upper bounds.” “One source for model rules” also requires validation at API build and generator entry, including serialized definitions. **Defect class:** validation depends on how a rule entered the definition, so stored component rules escape checks and permissive imports erase invalid rules.

In schematic-define, [constraint_problems](../../define/src/constraints/check.rs:54) visits endpoint metadata only. Components store arbitrary JSON, and [Constraints::from_schema](../../define/src/constraints/schema.rs:279) reads malformed values as absence. Schematic-gen's validator consumes that filtered result without validating the original component. A typo such as `maxLength: "3"` therefore generates an unchecked request. A nonpositive divisor and an impossible interval also reach generated plans. In addition, the importer's [drop_unpreservable_rules](../../define/src/openapi/import/mappings/mod.rs:186) removes the entire input rule set when one supported rule is contradictory.

Public-API probes constructed a request component, bundled and reloaded it through `DefinitionBundle`, then called `validate_rust_client`. Controls and every malformed component shape below were accepted without an error. For import, changing only the real fixture's `query.tags.items.minLength` from 2 to 20, above `maxLength: 12`, produced a successful import with a warning and removed **all** `tags` constraints, including its valid count, uniqueness, and pattern rules.

| Child ID | Site / shape | Observed result | Expected result |
| --- | --- | --- | --- |
| R1-F2.1 | Stored component via bundle creation and JSON reload, then Rust generation validation | All malformed cases accepted; most produce no diagnostic | Located definition/generation error |
| R1-F2.2 | Imported query-array item with contradictory lengths | Warning; entire parameter constraints removed | Reject invalid supported constraints; do not erase valid siblings |
| R1-F2.3 | Authored endpoint metadata and serialized endpoint metadata | Shared `check` and non-null deserializers reject invalid shapes; existing `value_constraints` matrix exercises the public bundle result | Clean route; reuse its semantics for components |
| R1-F2.4 | Imported numeric/null keyword loader | Existing `openapi_constraint_import` tests reject null and unrepresentable numeric limits at their pointer | Clean loader route; retain these failures |

The default outcome should be a located error for invalid metadata. Exceptions are absent optional keywords, valid empty pattern, `uniqueItems: false`, and arbitrary JSON values (including null) inside valid `enum`/`const` data. Component field-shape probes covered:

| Shared reader | Control / absence | Explicit null | Wrong whole-field type | Empty / semantic invalid |
| --- | --- | --- | --- | --- |
| Count/length | `maxLength: 3` and omitted keyword accepted, correctly | Accepted incorrectly | `"3"` accepted incorrectly | Negative count and `minLength: 4, maxLength: 3` accepted incorrectly |
| Numeric bounds/divisor | Ordinary valid numeric rules covered by existing runtime tests | `maximum: null` accepted incorrectly | `maximum: "3"` accepted incorrectly | `multipleOf: 0` accepted incorrectly |
| Pattern/format | Empty pattern accepted, correctly | `pattern: null` accepted incorrectly | Numeric pattern/format accepted incorrectly | `pattern: "["` only warns and is called unsupported rather than invalid |
| Uniqueness | True/false handled by the Boolean comparison | Accepted incorrectly | `"yes"` accepted incorrectly | No collection-element cases: this keyword is Boolean |
| Allowed values | Well-formed arrays are data, not lists of strings to filter | `enum: null` accepted incorrectly | `enum: "x"` accepted incorrectly | Empty enum accepted incorrectly |
| Item schema | An object is traversed; absent items impose no item rules | No item schema is produced | `items: 7` accepted incorrectly | Empty schema is allowed; positional item arrays are R1-F3 |

`enum` and `const` members are whole JSON values, so mixed JSON member types are not inherently malformed and must not be filtered. Bundle document-level duplicate keys and trailing content already have public loader tests; this finding concerns field validation after loading, not a request for another parser. Check each keyword wired to these readers, nested and referenced components, request roots and JSON form fields, and both API build and all requested output validators. Reject contradictory imported query/path/header/cookie/body/form rules consistently rather than repairing only the demonstrated query case.

### R1-F3 (high): Unenforceable request rules do not consistently block generation

**Authority — spec, “Supported schema traversal”:** “A constrained request beneath an unsupported composition or unresolved reference gets an actionable generation error.” “Definition errors versus unsupported rules” requires generation failure except for the documented pattern and format exceptions. **Defect class:** unsupported traversal is either downgraded to a warning or missed entirely, allowing generation with unenforced supported assertions.

In schematic-gen, [composition_findings](../../gen/src/codegen/value_rules/mod.rs:714) makes component-root `oneOf`/`anyOf` member rules warnings. In schematic-define, the [schema walk](../../define/src/constraints/registered.rs:156) enters positional `items` arrays without marking the traversal unsupported; schematic-gen only compiles an `items` object. The rule is consequently neither enforced nor diagnosed. These are one class: the generated client has no implementation of a supported assertion, yet generation succeeds.

The public generator validator was exercised with the same `maxLength: 3` rule moved between positions. A freshly generated crate also compiled with an unchecked union and positional array; neither produced a rule plan. No union evaluator is required to fix the finding: failing generation is the specified fallback. The OpenAI catalog impact explains the implementer's choice, but does not make a warning equivalent to the required failure.

| Child ID | Site / shape | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F3.1 | Component-root `anyOf` member | Warning, generation permitted | Actionable generation error |
| R1-F3.2 | Component-root `oneOf` member | Warning, generation permitted | Actionable generation error |
| R1-F3.3 | Positional `items: [{type: string, maxLength: 3}]` | No diagnostic or emitted plan | Error explaining unsupported positional traversal |
| R1-F3.4 | Nested-property `oneOf`, root `allOf`, root `not` | Generation error | Clean |
| R1-F3.5 | Ordinary array item object and two-member nullable `anyOf` | Accepted supported traversal | Clean; preserve their existing enforcement |
| R1-F3.6 | JSON body versus multipart JSON field | Both are explicitly passed to `unenforceable_rules` by the same generator loop | Same classification policy must apply at both entry points |

Retain the exclusion for constraints truly guaranteed by a Rust type. For unsupported independent rules, update the diagnostic expectations rather than pinning them as permitted warnings. Positional arrays are an unusual source shape; that instance alone would be medium priority. Root unions affect shipped OpenAI generation and violate the explicit acceptance contract, making the class high priority.

### R1-F4 (high): Pruning declared properties applies extra-property rules to valid fields

**Authority — spec, “Supported schema traversal” and “Constraints and their meaning”:** validation must reach “object properties” and “map values,” with the declared JSON Schema value semantics. **Defect class:** removing an apparently inactive schema node changes the classification of values handled by another rule.

In schematic-gen, [prune](../../gen/src/codegen/value_rules/mod.rs:975) removes declared properties without active constraints. In schematic-runtime, the [object evaluator](../../runtime/src/validate/eval.rs:218) uses that surviving property list to decide whether a key is an extra map entry. Thus an unconstrained declared property receives the `additionalProperties` rule after pruning. A valid request is refused before sending, and its named field is reported as a redacted arbitrary key.

Reproduction through freshly generated, compiled request code:

```json
{
  "type": "object",
  "properties": {"label": {"type": "string"}},
  "additionalProperties": {"type": "string", "maxLength": 3}
}
```

`MixedRequest::new(json!({"label": "long declared label"})).validate()` returns a `maxLength` violation. The expected result is success: the extra-property rule does not constrain `label`.

| Child ID | Site / shape | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F4.1 | Generated object with inactive declared property and active extra-value plan | Declared property removed, then incorrectly checked as an extra entry | Retain its declared-name exemption |
| R1-F4.2 | Runtime evaluator with the declared property retained | Selects declared-property plan before extra-value plan, even when that plan has no rules | Clean invariant; the runtime needs complete declaration membership |
| R1-F4.3 | Pure map with no declared properties | Every key uses extra-value rules and redacted paths; existing map tests verify this | Clean |
| R1-F4.4 | Declared property with active rules | Survives pruning and uses its own plan; existing generated nested-body and runtime property tests verify this | Clean |
| R1-F4.5 | Nested object through a named reference, and multipart JSON field | Additional freshly compiled request probes reject the same valid long declared label at both sites | Retain membership at every object level and both request entry points |

Preserve declared names whenever extra-property validation needs them, even if their plans do no checking. This needs a small plan-generation correction and a generated-request regression test, not a new validation subsystem. Check a genuinely extra key in the same fixture to prove the fix still enforces and redacts map-value failures.

### R1-F5 (medium): Nullable plans bypass explicit enum and const restrictions

**Authority — spec, “Constraints and their meaning”:** “`enum` and `const` apply to whole JSON values” and “Explicit JSON null follows the schema's nullability and applicable rules.” **Defect class:** type nullability is treated as unconditional permission to bypass independent whole-value assertions.

In schematic-runtime, [Walker::value](../../runtime/src/validate/eval.rs:182) immediately returns on null for every nullable plan. A generated request for `{"type": ["string", "null"], "enum": ["ok"]}` therefore accepts an explicit JSON null, although it is absent from the allowed set. Nullability permits the JSON kind; it does not insert null into an independently declared enum. The same issue affects `const`.

| Child ID | Site / shape | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F5.1 | Generated nullable type with root `enum: ["ok"]`, submitted null | Compiled request's `validate()` succeeds | Enum violation |
| R1-F5.2 | Nullable `Plan` with independent enum or const | Public runtime probes accept null against both excluding plans; early return precedes the checks | Check independent whole-value rules |
| R1-F5.3 | Nullable alternative wrapping a constrained non-null member | Null may bypass the non-null member's assertions | Clean intended behavior; preserve this distinction |
| R1-F5.4 | Non-nullable plan with null member in enum/const, and string-length-only plan receiving null | Existing runtime tests preserve null as data and apply string rules only to strings | Clean |
| R1-F5.5 | Definitions rebuilding of optional enum/const fields | `field_attribute` calls `admit_null`; the preservation comparator removes null members before comparison | Does not establish whole-schema equivalence; keep absent optional properties distinct from present null |

This requires an unusually restrictive nullable schema, so it does not independently block readiness. Preserve whether nullability comes from a separate alternative or from the type keyword when lowering plans. A focused test should compare allowed and excluded null membership directly; the current round-trip comparator intentionally normalizes that difference away.

## Verification

All required behavior here is library validation, code generation, or HTTP request preparation. Level 1 is appropriate; this feature adds no terminal rendering, keyboard, mouse, browser, or live-provider contract requiring higher levels.

| Requirement | Strongest evidence inspected or exercised |
| --- | --- |
| Exact arithmetic, Unicode lengths, patterns, deep equality, ordering and redaction | Runtime Level-1 `validate_decimal`, `validate_eval`, `validate_report`, and the pattern module tests |
| Authored/serialized metadata validation and definition round trips | Define Level-1 `value_constraints`, `openapi_constraint_import`, `openapi_constraint_export`, and `constraint_preservation`; missing positions are identified above |
| Fresh generated requests compile; build/await/full/direct-send/raw-send reject invalid arguments with zero requests; custom serialization is checked once per preparation | Generator's existing Level-1 fresh-output compiling/mock-server guard, plus temporary compiled probes for R1-F1, R1-F4, and R1-F5 |
| Effective page-size checks, including defaults and changes during traversal | Existing runtime `pager_rules`, generated pager tests, and definition conflict tests; shared helper remains at effective-size selection |
| Responses outside value constraints still decode | Committed-schema `value_rules::a_reply_outside_its_declared_value_rules_still_decodes`, independently rerun through `just test-schema value_rules` |
| Tests are compiled and selected | Temporary probes were explicitly declared in the existing `l1` binary, then removed; the repository suite runs the permanent tests through the area's live recipes |

Review probes exercised public bundle/import/generation APIs and compiled generated requests with the existing offline fresh-output harness. They asserted the observed defect to establish reproducibility, rather than silently treating that behavior as the intended contract. Temporary files and their module declaration were removed afterward. One initial probe failed to compile because of a missing borrow; another initially failed because its fixture edit did not match the fixture's indentation; a later sweep initially passed arguments to a helper that accepts environment pairs instead. Both probes were corrected and rerun before using their results.

Verification completed on this macOS host:

- `just test` from `schematic/`: **3,119 passed**, 23 skipped; its fresh-output step: **17 passed**; its runtime compile-fail step: **1 passed**.
- `just test-schema value_rules`: the committed-client response-decoding test passed; `just test-schema an_openai_speech_request` also passed the committed-client request-refusal test.
- `just check-tier-coverage schematic`: **0 stranded tests**. `git diff --check` passed.
- Public-API and compiled-output review probes reproduced the defects. Two final compiled sweep tests passed, covering the additional preservation sites, nested/reference/form property classification, valid property/map controls, and both nullable enum and constant behavior.
- The suite's six slow main-suite tests are reported without treating their existing runtime as a new feature defect. Fresh-output tests intentionally invoke Cargo under the existing dedicated recipe.
- No production implementation was changed. No cross-OS behavioral claim is made; cross-OS evidence is not a readiness condition for this review. No live provider calls or focus-changing terminal tests were needed.

## Observations

### Allowed exclusions and optional measurements remain deferred

The importer already loses some rules inside generic union branches collapsed to free-form JSON; generic composition support is excluded. Porting rules from the vendored Unfolded Circle document into its handwritten definition, repairing pre-existing unconstrained multipart serialization failures, migrating description-only path parameters, consolidating duplicate percent-decoding helpers, and changing unrelated recipe argument handling are separate work. A performance sample and broader numeric-literal preservation beyond the implemented limit-keyword route are recorded limitations; very high-precision enum/constant literals merit documentation rather than a speculative parser replacement in this cycle. None of these observations independently blocks readiness. This does not waive the supported map/free-form JSON preservation or generation-failure requirements reported above.
