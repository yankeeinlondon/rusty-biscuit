---
$schema: feature-review.yaml
ready: false
findings:
    - "R2-F1 (high): Dictionary selectors still skip or split default preparation"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-11-15T06:51:14-08:00"
spec: 2026-11-14-absent-equals-null/spec.md
implemented: true
description: "A **fix** review of `2026-11-14-absent-equals-null/spec.md`"
fix: 2026-11-14-absent-equals-null/review-2.md
previous: 2026-11-14-absent-equals-null/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/fix-sequences/simplified-schema/fixes/2026-11-14-absent-equals-null/log.md
implemented_by: claude/opus
next: 2026-11-14-absent-equals-null/review-3.md
---

# Review 2: Absent equals null

The fix is **not production ready**. Both previous findings are verified fixed
for every required instance in the repair ledger. The wider selector sweep
found a new sibling of the default-preparation defect: a dictionary entry's
own default is ignored, and schemas that describe the same entry are not
always combined. This is R2-F1, an ordinary repair within the spec. No finding
needs a human decision.

## Previous Finding Verification

Read `review-1.md`, the repair's `log.md` Finding Ledger, and the original
implementation records before checking the code. There is no earlier review
or Human rulings section carrying additional open work.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Pattern-keyed objects skip default preparation | R1-F1.1–R1-F1.19, including the repair's added controls and siblings | verified fixed | Every instance below checked against code and passing public-result tests; the conditional default reports now include inner actions | None of those instances. R2-F1 reports newly found siblings, not an unchanged reproduction |
| R1-F2: Lifecycle validation accepts inputs the parser rejects | R1-F2.1–R1-F2.23 | verified fixed | Shared placement helpers, both hosts' agreement tests, completed/authored controls, composition and proxy-overlay tests all checked and passed | None |

The following tables account for every ledger child, including entries the
implementer labeled clean. Test names refer to SimplifiedSchema's
[pattern-default tests](../../lib/tests/l1/pattern_defaults.rs), Darkmatter's
[lifecycle agreement tests](../../../darkmatter/lib/tests/l1/lifecycle_schema_parity.rs),
[edit agreement tests](../../../darkmatter/lib/tests/l1/lifecycle_edit_schema_parity.rs),
[composition tests](../../../darkmatter/lib/tests/l1/composition_defaults.rs),
and Claudine's [schema tests](../../../claudine/lib/tests/l1/claudine_schemas.rs).
All ran through their declared L1 targets in this review's full package runs.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1.1: Literal selector | Missing/null Boolean child; supplied false | verified fixed | `selectors::a_literal_property_child_is_completed`; strict borrowing, report, soft copy and stability assertions | None |
| R1-F1.2: Catch-all selector | Existing object with missing/null child | verified fixed | `selectors::a_catch_all_child_is_completed` | None |
| R1-F1.3: Prefix selector | Same child shapes under `<starting::x->` | verified fixed | `selectors::a_prefix_pattern_child_is_completed` | None |
| R1-F1.4: Suffix selector | Same child shapes under `<ending::-settings>` | verified fixed | `selectors::a_suffix_pattern_child_is_completed` | None |
| R1-F1.5: Regex selector | Same child shapes under `<pattern::^x->` | verified fixed | `selectors::a_regular_expression_pattern_child_is_completed` | None |
| R1-F1.6: Raw pattern selector | Missing/null child before validation | verified fixed | `raw_schemas::a_pattern_properties_child_is_prepared_before_it_is_judged` | None |
| R1-F1.7: Catch-all beside a pattern | Matching and nonmatching existing object keys | verified fixed | `raw_schemas::additional_properties_beside_a_pattern_completes_only_unmatched_keys` | None |
| R1-F1.8: Conflict beneath a pattern | Raw preparation and SimplifiedSchema export | verified fixed | Both pattern conflict tests in `conflicts`; error checked before any document needs a default | None |
| R1-F1.9: Conflict beneath other selectors | Literal and catch-all | verified fixed | `conflicts::a_union_conflict_beneath_a_literal_or_catch_all_is_refused_when_prepared` | None |
| R1-F1.10: Conditional true branch | Inner edit defaults under `then` | verified fixed | `a_lifecycle_action_inside_a_conditional_branch_reports_its_defaults`, true branch | None |
| R1-F1.11: Conditional false branch | Same edit under `else` | verified fixed | Same test, false branch; valid `then` control retained | None |
| R1-F1.12: Positional and recursive children | Arrays, tuple positions, recursive/local named types | verified fixed | `defaults::tests::tuple_positions_are_never_inserted_but_their_objects_are_completed`, recursive tests and `required_defaults::{parents,nesting}` | None |
| R1-F1.13: Skill limitation | Obsolete statement that patterns never complete children | verified fixed | Claudine skill now describes traversal; current nested-object probes agree | None of the former limitation; broader selector behavior is R2-F1 |
| R1-F1.14: Simplified literal precedence | Literal and matching sibling pattern | verified fixed | `selectors::a_literal_key_is_not_completed_by_a_sibling_pattern`; lowered exclusion pattern enforces precedence | None |
| R1-F1.15: Raw literal plus pattern | Both schemas' child defaults | verified fixed | `raw_schemas::a_declared_property_matching_a_pattern_takes_both_schemas_defaults`; additional direct-value probe also passed | None |
| R1-F1.16: Overlapping pattern conflicts | Equal defaults, different defaults, keys matching only one pattern | verified fixed | Both `raw_schemas::overlapping_patterns_with_*` tests; equality deduplicates and different insertions return `ConflictingDefaults` | None for those scalar-child cases; parent/required combinations are new siblings in R2-F1 |
| R1-F1.17: Value-dependent keywords | Conditional and annotation-dependent raw applicators | verified fixed | Compiler follows only the listed structural selectors; documented boundary remains explicit; SimplifiedSchema's key-count conditionals contain no defaults | None; no new requirement to interpret these applicators |
| R1-F1.18: Preparation cost | Formerly slow style-schema regression | verified fixed | `base_schema_end_to_end::base_style_schema_agrees_with_the_runtime_style_parser` took 3.136 s in the full Darkmatter run; caches are bounded and reset together | None |
| R1-F1.19: Completed edit control | Conditional completion must preserve authored parsing | verified fixed | `a_completed_cell_parses_with_its_authored_source_as_the_authored_cell_does`; equality assertion retained, unrelated side assertion narrowed | None |
| R1-F2.1: Untyped side-effect flag | Numeric `ensure_file.no_error` | verified fixed | `side_effect_numeric_flag_is_refused_by_both`, located at the inner flag | None |
| R1-F2.2: Untyped function flag | Numeric `abs.no_error` | verified fixed | `expression_function_numeric_flag_is_refused_by_both` | None |
| R1-F2.3: md conditional families | Authored false beside `then`, all three levels | verified fixed | `md_conditional_{1,2,3}_authored_false_flag_is_refused_by_both`; `else` control also passes | None |
| R1-F2.4: md initialize conditionals | Same placement at all three levels | verified fixed | `md_initialize_conditional_{1,2,3}_authored_false_flag_is_refused_by_both` | None |
| R1-F2.5: Claudine conditional families | Start, initialize, cancelled; false/true and nested controls | verified fixed | `an_authored_flag_beside_then_is_refused_in_every_family`; absent/null controls accepted | None |
| R1-F2.6: md duplicate flags | Edit, info, shell, generated side effect; root/body positions | verified fixed | Duplicate agreement rows plus `two_authored_flag_placements_fail_schema_validation_and_the_parser` | None |
| R1-F2.7: Claudine duplicate flags | Three event families; item/root/body; generated action | verified fixed | `duplicate_flags_are_refused_in_every_family`, `single_action_placement_rules_are_judged_alike_by_schema_and_parser` | None |
| R1-F2.8: Flag on set | Authored null/false at item/root; flagged multi-verb bundle control | verified fixed | `set_*_flag_is_refused_by_both`, `set_in_a_flagged_bundle_is_accepted_by_both` | None |
| R1-F2.9: md incomplete conditional | Bare `when`, nested and initialize | verified fixed | `bare_when*`, `initialize_bare_when*`, `nested_bare_when*`; DMLS response-contract fixture and tests passed | None |
| R1-F2.10: Claudine incomplete conditional | All three event families | verified fixed | `a_bare_when_is_refused_in_every_family` | None |
| R1-F2.11: Typed flag controls | md info/edit/generated long form; Claudine typed actions | verified fixed | Existing flag matrix and `a_typed_actions_numeric_flag_is_refused_by_both` | None |
| R1-F2.12: Shared flag binding | Absent, null, false, Boolean-like text; sibling/bundle paths | verified fixed | Side-effect shape rows, `every_no_error_placement_converts_text_and_refuses_a_number_in_both_checks`, edit acceptance matrix | None |
| R1-F2.13: Authored conditional controls | Absent/null versus authored true | verified fixed | Both hosts' conditional agreement tests | None |
| R1-F2.14: Completed provenance | Inner and enclosing false defaults on edit | verified fixed | Completed edit parser equality; `a_completed_document_is_judged_by_its_authored_flags` | None |
| R1-F2.15: Flag-shape column | Number, invalid text, empty text/list/mapping | verified fixed | All side-effect agreement rows assert schema and parser refusal; absent/null/false/text controls pass | None |
| R1-F2.16: Tests and guidance | Former gap-pinning tests and current docs/skills | verified fixed | Tests now assert agreement; changed topic pages and skills describe passive placement checks | None |
| R1-F2.17: Untyped deeper bodies | Both md event profiles below level three | verified fixed | `below_level_3!` rows and completed-document control; `untyped_bodies` follows the parser's list/dictionary shapes | None |
| R1-F2.18: Replaced/inherited stacks | Overrides, completed parent stacks, proxy overlay | verified fixed | Four composition regressions; Claudine `a_proxy_overlays_lifecycle_stack_is_judged_by_its_own_authored_flags` | None |
| R1-F2.19: Other placements | Root/body `when`, incomplete branches, sibling action, flag-only item, loop loose verbs | verified fixed | Agreement rows and Claudine `a_loops_loose_verbs_follow_the_placement_rules`; parser helpers preserve their original call positions | None |
| R1-F2.20: Unprovided verbs | Ignored long-form and duplicate flags on `say` | verified fixed | Both `unprovided_verb_*_left_unread_by_both` rows | None |
| R1-F2.21: Nesting limit | Parser's sixth-level refusal; schema has no depth limit | verified fixed | Existing Claudine depth contract and `a_sixth_conditional_level_is_the_parsers_alone_to_refuse` | None; existing scope preserved |
| R1-F2.22: Deep operand typing | md deliberately leaves deeper operands untyped | verified fixed | Types and docs retain the tolerant baseline; the new walker only adds the reviewed placement/flag checks | None; no broader operand-typing requirement introduced |
| R1-F2.23: Nested setters and merged mappings | Schema and parser read the same authored flag values | verified fixed | Reviewed `authored_view`, setter handling and parser consumers; no differing verdict established | None |

## Unblocked Findings

### R2-F1 (high): Dictionary selectors still skip or split default preparation

**Authority: spec.** “A default replaces a `null` value,” “One rule for both
schema forms,” “After checking that default, complete reachable child defaults
in the same preparation so the result is stable on a second validation,” and
“For a required null-admitting property, it inserts a missing key with `null`
and reports that insertion.” Ordinary dictionary settings reach the first
failure without overlapping patterns or unusual payloads. The overlap cases
below also violate the explicitly required one-pass stability.

**Defect class:** default preparation does not treat the schemas selecting an
existing dictionary key as one property definition, so it skips defaults on
the selected value and loses information needed to complete its children.

In SimplifiedSchema, [Compiler::scope](../../lib/src/defaults.rs:340) collects
defaults through literal `properties`, but compiles patterns and catch-alls
only as child locations. [Fill::visit](../../lib/src/defaults.rs:526) visits an
undeclared entry's value directly; it never applies that entry's own default.
With several selectors it visits each location independently against the
original input. It also skips every globally declared key before considering
another node's catch-all, although that catch-all can describe the key too.

This public-API example is an ordinary dictionary of numeric settings:

```rust
use serde_json::json;
use simplified_schema::SimplifiedSchema;

let schema = SimplifiedSchema::try_from(
    "$schema:\n  '<string>': number(default(3))\n",
).unwrap();
let input = json!({ "timeout": null });
let result = schema.validate_soft(&input);
// Observed: InvalidData / TypeMismatch at /timeout.
// Expected: timeout = 3, with one applied-default report.
```

Strict validation also incorrectly refuses it; it should accept the completed
candidate, return the original input borrowed, and report `timeout = 3`.
Changing the declaration to the literal `timeout` gives those results.
Prefix, suffix, regex, and raw JSON Schema dictionaries have the same defect.
An existing null object with an object default likewise fails instead of
receiving the object and its reachable child defaults. A used invalid default
is never checked: the entry gets a data-type error instead of `InvalidDefault`.

For the combination cases, these two valid declarations describe the same
settings object:

```yaml
types:
  Parent:
    options: "{ enabled: boolean }(default({}))"
  Nested:
    options: "{ enabled: boolean(default(true)) }"
$schema:
  '<starting::x->': Parent
  '<ending::-settings>': Nested
```

Soft validation of `{"x-settings": {}}` returns
`{"x-settings": {"options": {}}}`. A second validation adds `enabled: true`.
Both applicable definitions must describe the defaulted parent in the same
preparation. The same failure occurs with raw patterns, or a pattern in one
`allOf` member and a catch-all in another. Splitting `required: [n]` and
`n: {type: "null", default: null}` across the matching definitions instead
produces `MissingRequired`; the equivalent literal property's merged location
correctly inserts `n: null`.

The sweep checked the selectors, default validation, presence, both validation
modes, reports, and repeated completion. Raw examples used the Draft 2020-12
dialect and the same public facade; its preparation reaches the same
`DefaultPlan` as `Prepared` and hosts. Each row changes a selector or one value
in these control fixtures.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F1.1 | Literal `properties` | Existing null numeric entry with default 3 | Strict borrowed with report; soft 3; stable | Clean control |
| R2-F1.2 | `<string>` / catch-all | Same existing null entry | Both modes return `TypeMismatch`, no default | Strict accepts/reports; soft writes 3 |
| R2-F1.3 | `<starting::x->` | Same entry named `x-settings` | Same refusal | Same fill/report |
| R2-F1.4 | `<ending::-settings>` | Same entry | Same refusal | Same fill/report |
| R2-F1.5 | `<pattern::^x->` | Same entry | Same refusal | Same fill/report |
| R2-F1.6 | Raw `patternProperties` | Existing null entry; also with its name in `required` | Both modes reject null | Prepare 3 before judging; written key meets presence |
| R2-F1.7 | Raw `additionalProperties` | Existing null numeric entry | Both modes reject null | Prepare/report 3 |
| R2-F1.8 | Selected object defaults, all four SimplifiedSchema dictionary selectors | Existing null parent with `default({})` and optional defaulted Boolean child | Null parent refused; existing `{}` control completes the child | Fill parent and child together; no second-pass change |
| R2-F1.9 | Used-default checking, all four SimplifiedSchema dictionary selectors | Existing null numeric entry with `default('bad')` | Data `TypeMismatch`; default never checked | `ValidationFailure::Unprepared` with `InvalidDefault` |
| R2-F1.10 | Existing object traversal through literal, pattern and catch-all | Parent `{}`, child absent/null | Child completed and reported by both modes; original regressions pass | Clean control; preserve this repair |
| R2-F1.11 | Raw `allOf`: literal in one member, catch-all in another | `x-settings: {}`, catch-all declares defaulted `enabled`; tested with and without its own nonmatching `^y-` pattern | Both modes accept unchanged, report nothing | Complete `x-settings.enabled = true` from the applicable catch-all |
| R2-F1.12 | Two overlapping pattern locations, SimplifiedSchema and raw | Missing `options`, parent default in one definition, child default in the other | First soft call inserts `{}`; second inserts `enabled`; strict report omits the reachable child too | Parent and child completed/reported together; second call unchanged |
| R2-F1.13 | Raw pattern plus catch-all in separate `allOf` members | Same parent/child definitions and input | Same two-pass completion | Same one-pass result |
| R2-F1.14 | Required/default propagation between matching patterns | One definition requires null-admitting `n`; another declares `n` with null default | `MissingRequired` for absent `n` | Insert/report `n: null`; subsequent validation unchanged |
| R2-F1.15 | Raw literal plus matching pattern | Direct null scalar default; also distinct child defaults | Both definitions combined; fill/report correct | Clean control |
| R2-F1.16 | Merged literal location through `allOf` | Parent/child definitions and split required-null declarations from rows 12–14 | One-pass nested completion; required null inserted | Clean control |
| R2-F1.17 | Matching-pattern scalar-child conflicts | Equal/different child defaults from R1-F1.16 | Equal defaults reported once; unequal defaults refused | Clean control; preserve refusal rather than order-based choice |
| R2-F1.18 | Arrays, tuple objects, recursive/local references | Same ordinary defaulted child in existing values | Existing public-result and engine regressions pass; no positional insertion | Clean control |

The affected scalar's other shapes were checked through strict and soft
public results for literal, catch-all, prefix, suffix and regex selectors.
Apart from the null/default failure, their common outcome is correct:
`0` stays `0`; Boolean `false`, empty text, `[]` and `{}` are refused at the
entry; numeric text `"3"` is refused strictly and converted softly, without a
default report. An empty dictionary stays empty: a dictionary selector cannot
invent an absent key. The literal control inserts its specifically named
missing key. These are scalar fields, so collection-element shapes do not
apply. For the object-default cases, an absent dictionary entry stays absent,
an existing `{}` gets the child default, and the existing null entry is the
failing shape. Literal-parent controls complete parent and child together.

**Recommended repair:** use the ordinary property-default preparation rules
for each existing selected key, combining all schemas that describe that key
before deciding its default, required-null insertion, or child completion.
Include another node's applicable catch-all even when a node names the key
literally. Reuse the current compiled plan and caches; keep unknown keys
unknown and never enumerate keys from patterns. No second host walk or
regex-overlap analysis is needed: runtime selection already knows the actual
key. Preserve the existing per-key conflict refusal for overlapping selectors,
and add public-result regressions for the listed failing rows. Align the
pattern guidance with the repaired boundary.

## Blocked Findings

None. R2-F1 can be repaired within the spec without changing syntax, execution
order, exported assertions, or a public format.

## Recurrence

R2-F1 is a **sibling missed by the earlier sweep**, in the same structural
default-preparation class as review-1.md's R1-F1. It is not a regression of
the repaired existing-object child cases. The earlier sweep checked defaults
*beneath* selected objects but missed a default on the selected entry itself.
It also did not combine selectors with a newly defaulted parent, split
required/null-default declarations, or a literal key described by another
node's catch-all. The repair added independent selector visits, leaving those
combined-location cases incomplete. The instance table above includes all
selectors, affected decisions, and clean controls found in this sweep.

This remains a bounded property-default requirement: prepare the definitions
that select an existing key. It does not request arbitrary conditional
applicators, synthetic dictionary keys, or a stronger exported JSON Schema
guarantee.

## Verification and input robustness

These requirements concern values, passive validation, conversion, composition
and lifecycle parsing. Level 1 public APIs, hermetic CLI processes, fake
services and editor-protocol tests are the appropriate verification level.
The spec requires no terminal-rendering or keyboard-input behavior, so no
Level 2/3 terminal run is missing.

| User-facing requirement | Level 1 evidence checked | Assessment |
| --- | --- | --- |
| Missing/null/presence table, borrowing/copying, falsey values, reports and stability | `required_presence`, `required_defaults`, engine tests; selector public-API probes | Literal properties pass; R2-F1 covers dictionary omissions and combined selectors |
| Existing/defaulted parents, object arrays, named/imported/recursive types, unions and pending values | `required_defaults::{parents,nesting,unions,pending_templates}`, `pattern_defaults`, conditional composition reports | Original cases pass; combined-selector cases remain R2-F1 |
| Raw schemas and exported annotations/assertions | `required_defaults::{raw_schemas,raw_schema_inputs}`, plain-validator test, artifact and identity tests | Export behavior passes; raw preparation shares R2-F1 |
| Trigger presence, schema envelopes, phase presence and not-empty | Matcher, envelope-null, required-presence, not-empty and phase suites | Pass |
| Declared but unwritten values, parent/child presence, no source mutation | `absent_property_contract`, compose/frontmatter/setter suites | Pass |
| Lifecycle flag and operand coercion, placement, bundles, generated actions, completed provenance | Both hosts' parity tests, authored parser tests, fake editor/executor tests, composition/overlay tests | R1-F2 verified fixed |
| CLI schema validation, composition, whole YAML and editor diagnostics | Darkmatter CLI suites, whole-YAML suites and DMLS tests, including updated response contract | Pass for the exercised requirements; library selector defects remain shared |
| Claudine overrides, keyed config, key counts and failure publication | Config schema/wire/JSON5/CLI tests and completion/preparation tests | Pass |

For configuration/file inputs the default invalid-shape outcome remains a
located refusal, with no publication of failed completed data. Initialization
order remains the explicitly preserved exception to treating this as a ban
on every earlier lifecycle effect. Other exceptions are the spec's
missing/null inheritance, default application and admitted empty values.

No loader or configuration representation changed in the repair. The full
runs exercised the existing admitted-YAML, JSON-file, Markdown-frontmatter
and legacy-JSON5 loader matrices: duplicate keys, invalid/trailing content
and format-specific syntax remain covered at their public boundaries, with
representative CLI wiring. This review's new field-shape probes are expanded
under R2-F1 rather than repeated once per file format. The fixed lifecycle
flag-shape column is accounted for under R1-F2.11–R1-F2.15.

The smell search covered `serde(default)`, `Option`, `filter_map`,
`unwrap_or_default()` and `.ok()` in the changed readers. Pattern compilation
failures are left to validator compilation; required-name extraction reads
schema metadata, not user data. No additional silent wrong-type fallback was
established. The relevant loss is the selector's missing property preparation,
already inventoried under R2-F1.

Review runs on macOS, with full suites run one area at a time:

| Command | Result |
| --- | --- |
| `simplified-schema/`: `just test` | 2,251 feature-enabled and 1,892 minimal tests passed; none at or above 5 s |
| `darkmatter/`: `just test` | 16,730 passed, 10 recipe skips; none at or above 5 s |
| `claudine/`: `just test` | 12,170 passed, 15 recipe skips; none at or above 5 s |
| `just lint`, all three areas | Passed, including SimplifiedSchema's three feature builds and Darkmatter's Zed WASM check |
| `just check-tier-coverage simplified-schema darkmatter claudine` | Zero stranded tests |
| Public-API probes | Reproduced R2-F1 and its controls through strict/soft validation; temporary example sources removed afterward |
| `git diff --check` | Passed |

All new/changed test modules are registered in declared consolidated targets;
their names and features are selected by the recipes that ran. No production
code was modified during this review. Cross-OS evidence remains CI's work and
does not block readiness.

`just slow-tests fresh` subsequently returned 1 solely because this review
file changed after the last run; its output explicitly says the changed file
is outside the directories those tests cover. The runs themselves recorded
zero slow tests. No production or test source changed after those runs.

## Implementation record corrections

Corrected obsolete statements in `implementation-log.md` and `implementation.md`
that the repaired lifecycle gaps still pass schema validation. Both now carry
dated notes with the code/test verification. In `implementation.md`, also
narrowed the blanket default-filling statement to reflect the verified
selector defects, and qualified the statement about absent/null parents:
parents with their own defaults can be created. These record corrections are
not separate findings or human-review items.
