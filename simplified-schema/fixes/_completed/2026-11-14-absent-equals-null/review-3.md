---
$schema: feature-review.yaml
ready: false
findings:
    - "R3-F1 (high): Used defaults omit constraints from other applicable definitions"
    - "R3-F2 (low): Failure guidance omits runtime dictionary-default conflicts"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-11-15T07:30:20-08:00"
spec: 2026-11-14-absent-equals-null/spec.md
implemented: true
description: "A **fix** review of `2026-11-14-absent-equals-null/spec.md`"
fix: 2026-11-14-absent-equals-null/review-3.md
previous: 2026-11-14-absent-equals-null/review-2.md
log: /Volumes/coding/wt/rusty-biscuit/fix-sequences/simplified-schema/fixes/2026-11-14-absent-equals-null/log.md
implemented_by: claude/opus
next: 2026-11-14-absent-equals-null/review-4.md
---

# Review 3: Absent equals null

The fix is **not production ready**. Review 2's finding is verified fixed
for all 20 instances in the repair ledger. A newly found sibling remains:
checking a used default can omit constraints supplied by another schema
that also describes the property. In particular, child defaults can repair
an invalid parent default, which the spec explicitly forbids. A separate
documentation finding covers the remaining blanket claims that every
default conflict is caught before validation. Neither finding needs a
human decision.

## Previous Finding Verification

Read both earlier reviews, the repair's `log.md` Finding Ledgers, and
`implementation-log.md` and `implementation.md` before verification.
Neither earlier review contains a Human rulings section. No earlier
finding remained open after review 2; the two original findings are
included below to make their status explicit.

Evidence below comes from source inspection and this review's executed
tests. The SimplifiedSchema test names refer to
[pattern_defaults.rs](../../lib/tests/l1/pattern_defaults.rs), unless
another module is named. The implementation flags and ledger dispositions
were not treated as proof.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Pattern-keyed objects skip default preparation | R1-F1.1–R1-F1.19, previously verified in review 2 | verified fixed | Selector, conflict, recursion, tuple and literal-precedence regressions pass; Darkmatter's conditional `then`/`else` reports pass in its full suite | No original instance remains; the new default-check sibling is R3-F1 |
| R1-F2: Lifecycle validation accepts inputs the parser rejects | R1-F2.1–R1-F2.23, previously verified in review 2 | verified fixed | Reviewed the shared authored-placement helpers and both hosts' agreement tests; Darkmatter lifecycle, composition and editor tests pass; Claudine schema/overlay tests pass | None |
| R2-F1: Dictionary selectors still skip or split default preparation | R2-F1.1–R2-F1.20 | verified fixed | Every ledger child checked below; both library feature builds pass | None of the required reproductions; R3-F1 is a newly found sibling |
| R2-F1.1: Literal property control | Null scalar default, absent named entry | verified fixed | `entry_defaults::{scalar_entry_takes_its_default,an_absent_entry}::literal` | None |
| R2-F1.2: Catch-all entry default | Existing null number | verified fixed | `entry_defaults::scalar_entry_takes_its_default::catch_all`; strict borrowing/report, soft copy, repeat stability | None |
| R2-F1.3: Prefix entry default | Same null value | verified fixed | `entry_defaults::scalar_entry_takes_its_default::prefix` | None |
| R2-F1.4: Suffix entry default | Same null value | verified fixed | `entry_defaults::scalar_entry_takes_its_default::suffix` | None |
| R2-F1.5: Regex entry default and scalar shapes | Null, zero, wrong types, numeric text, absent entry | verified fixed | `entry_defaults::{scalar_entry_takes_its_default,other_scalar_shapes_are_kept,an_absent_entry}::regular_expression`; shape tests also run for the other selectors | None |
| R2-F1.6: Raw pattern entry | Null, with/without its name in `required` | verified fixed | `a_raw_pattern_properties_entry_takes_its_default`, `a_raw_pattern_properties_entry_named_in_required_takes_its_default` | None |
| R2-F1.7: Raw catch-all entry | Null number | verified fixed | `a_raw_additional_properties_entry_takes_its_default` | None |
| R2-F1.8: Selected object defaults | Null parent, default `{}`, optional defaulted child; all five selectors | verified fixed | `entry_defaults::object_entry_takes_its_default_and_child::{literal,catch_all,prefix,suffix,regular_expression}` checks both modes and one-pass stability | None of those optional-child cases; adding a required-child constraint exposes R3-F1 |
| R2-F1.9: Used invalid defaults | Number default `'bad'`; all five selectors | verified fixed | `entry_defaults::used_invalid_default_is_refused::*` checks `Unprepared(InvalidDefault)` and its property path in both modes | None of those single-definition cases; split constraints are R3-F1 |
| R2-F1.10: Existing object traversal | Missing/null child and false control under literal, pattern and catch-all | verified fixed | `selectors::*`, `raw_schemas::*`, including literal precedence | None |
| R2-F1.11: Another member's catch-all | Literal name also described by another `allOf` member's catch-all, with/without nonmatching pattern | verified fixed | `combined_selectors::{another_members_catch_all_completes_a_literally_named_key,another_members_guarded_catch_all_completes_a_key_its_patterns_miss}` | None |
| R2-F1.12: Overlapping patterns | Parent default and optional child default split between SimplifiedSchema/raw selectors | verified fixed | Both `overlapping_*_patterns_complete_parent_and_child_in_one_pass` tests | None of those optional-child cases |
| R2-F1.13: Pattern plus another member's catch-all | Same one-pass parent/optional-child completion | verified fixed | `a_pattern_and_another_members_catch_all_complete_in_one_pass` | None |
| R2-F1.14: Required/null-default propagation | One selected definition requires `n`, another defaults it to null | verified fixed | `required_and_a_null_default_from_matching_patterns_insert_null` | None |
| R2-F1.15: Raw literal plus pattern | Entry default and distinct child defaults | verified fixed | `a_raw_literal_matching_a_pattern_takes_the_patterns_default`, `a_declared_property_matching_a_pattern_takes_both_schemas_defaults` | None |
| R2-F1.16: Literal definitions across `allOf` | Optional-child completion and required-null insertion | verified fixed | `a_literal_merged_across_all_of_members_completes_in_one_pass`, `required_and_a_null_default_from_a_literal_merged_across_all_of_insert_null` | None of those cases; required children inside a default object are R3-F1 |
| R2-F1.17: Overlapping default conflicts | Equal child defaults, unequal child/entry defaults; unused entry control | verified fixed | `raw_schemas::overlapping_patterns_with_*`, `overlapping_patterns_with_different_entry_defaults_conflict_for_a_null_entry` | None; accepted per-key conflict behavior preserved |
| R2-F1.18: Positional/recursive controls | Arrays, tuple objects, recursive/local references; no inserted positions | verified fixed | `required_defaults::{nesting,parents}` and `defaults::tests` pass | None of those cases |
| R2-F1.19: Pattern guidance | Repaired selector boundary in validation page and two skills | verified fixed | Read the changed pattern paragraphs; their entry-default and optional-child examples match the passing public results | Broader error-timing guidance outside those paragraphs is R3-F2 |
| R2-F1.20: Missing name listed only in raw `required` | Pattern/catch-all default supplies the named key; unnamed absence stays absent | verified fixed | `a_missing_required_name_takes_the_default_of_the_schemas_selecting_it`, `a_missing_name_no_schema_requires_stays_absent_beside_a_pattern`; `Fill::visit` includes required names | None |

## Unblocked Findings

### R3-F1 (high): Used defaults omit constraints from other applicable definitions

**Authority: spec.** Under “Validation and default preparation”: “An object
supplied by a default must already satisfy its complete required-child
contract; defaults do not repair an invalid default object,” and “Preserve
strict checking of a used default against the complete property definition,
without conversion or file access.” This fails by adding one `required`
marker to the existing combined-selector fixture; it requires no malformed
or adversarial data.

**Defect class:** checking a used default discards constraints from other
definitions that must hold together when those definitions do not repeat
the `default` annotation.

In SimplifiedSchema,
[`Compiler::declared`](../../lib/src/defaults.rs:490) retains a definition
for checking only when it finds a default beneath that definition.
[`Fill::check`](../../lib/src/defaults.rs:729) consequently checks that subset.
[`Fill::fill`](../../lib/src/defaults.rs:712) then completes children from
*all* selected definitions. A required child omitted from a parent default
can therefore be filled before the final validator sees it, turning an
invalid schema default into a successful validation.

Start with the fixture already used by
`combined_selectors::overlapping_simplified_patterns_complete_parent_and_child_in_one_pass`.
Add `required` to the second definition's Boolean child:

```yaml
types:
  Parent:
    options: "{ enabled: boolean }(default({}))"
  Nested:
    options: "{ enabled: boolean(required; default(true)) }"
$schema:
  '<starting::x->': Parent
  '<ending::-settings>': Nested
```

Through the public `SimplifiedSchema` API:

```rust
let schema = SimplifiedSchema::try_from(text).unwrap();
let input = serde_json::json!({ "x-settings": {} });
let result = schema.validate_soft(&input).unwrap();
// Observed: {"x-settings":{"options":{"enabled":true}}}.
// Expected: Unprepared(InvalidDefault), at x-settings.options.
```

Strict validation also succeeds and reports both insertions while borrowing
the input. The default `{}` lacks `enabled`, which the second matching
pattern requires in the effective `options` definition. Writing
`options: {}` explicitly remains valid: completing an authored object is
allowed. Supplying that object as a default has the stricter check above.

The sweep followed every existing structural route to the shared checker.
Raw cases use Draft 2020-12. `allOf` means every member's constraints must
hold; the reference and positional rows use that same parent/child fixture
with only the route changed. Both validation modes were checked in each
public-API probe.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R3-F1.1 | SimplifiedSchema overlapping prefix/suffix selectors and resolved named types | Existing `x-settings: {}`; required child added to `Nested` | Both modes accept and report parent plus child defaults | `Unprepared(InvalidDefault)` for the parent |
| R3-F1.2 | Raw overlapping regex patterns | Same parent/default/required-child definitions | Both accept after repairing the default | Same refusal before child completion |
| R3-F1.3 | Raw literal property plus matching pattern | Parent from `properties`, required-child constraint from matching pattern | Both accept after repair | Same refusal |
| R3-F1.4 | Raw literal properties in separate `allOf` members | Same definitions under literal `x-settings` | Both accept after repair | Same refusal |
| R3-F1.5 | Pattern plus catch-all in separate `allOf` members | Same definitions | Both accept after repair | Same refusal |
| R3-F1.6 | Catch-alls in separate `allOf` members | Same definitions | Both accept after repair | Same refusal |
| R3-F1.7 | Local `$ref` targets joined by `allOf` | Parent/default in one referenced object, required child in the other; input `{}` | Both accept with `options.enabled = true` | `InvalidDefault` at `options` |
| R3-F1.8 | Array `items` joined by `allOf` | Input `[{}]`; same object definitions for each item | Both accept with `0.options.enabled = true` | `InvalidDefault` at `0.options` |
| R3-F1.9 | Tuple `prefixItems` joined by `allOf` | Same existing first item | Both accept with `0.options.enabled = true` | Same refusal; no inserted position |
| R3-F1.10 | Scalar default split from its type across literal `allOf` definitions | `timeout.default = "3"` in one member; `timeout.type = number` in the other; input `{}` | Both return a data `TypeMismatch` at `/timeout` | Schema `InvalidDefault` at `timeout`, checked strictly |
| R3-F1.11 | Complete property definition in one schema | Same `{}` parent default with its required/defaulted child in the declaring definition | Both return `Unprepared(InvalidDefault)` | Clean control |
| R3-F1.12 | `allOf` inside the property's own definition | Same required/defaulted child, but the default's retained property definition contains the complete `allOf` | Both return `Unprepared(InvalidDefault)` | Clean control |
| R3-F1.13 | Valid default with optional children across selectors | Original fixture, before adding `required` | Parent and child complete in one pass; stable second validation | Clean; preserve R2-F1's repair |
| R3-F1.14 | Alternative union arms | Spec's number-default/string alternative; equal/different union defaults | Existing `required_defaults::unions` tests pass: one arm's default fills, conflicting declarations fail before use | Clean; preserve the author's union policy |

The field-shape sweep uses the edited SimplifiedSchema fixture once through
the shared validator: absent `options` and explicit null both wrongly
accept the repaired parent default; an authored empty mapping correctly
gets only the child default; `{ enabled: false }` stays unchanged; a native
number, empty text, and empty list are refused at `/x-settings/options`.
The existing `other_scalar_shapes_are_kept` matrix covers scalar absence,
null, zero, wrong containers and numeric-text conversion for every selector.
These are object/scalar value checks; no collection element is filtered.
The positive control is the unedited optional-child fixture in row 13.

**Recommended repair:** retain the constraints that must hold together in a
selected property's compiled default-check definition, and check the used
default against them before filling children. Reuse the structural-validator
cache and the shared preparation engine. Preserve alternatives: checking a
default against every independent union arm would incorrectly reject the
spec's valid number-default/string example. Keep the repair bounded to the
structural routes already supported; no conditional-keyword evaluator,
new syntax, or host default walk is needed. Add public-result regressions
for the affected routes and keep the clean controls above. Update the
related default-check comments and guidance with the repaired contract.

The repair log's assertion that constraints from a non-declaring selector
would produce “a refusal either way” is disproved by this successful
parent repair. That factual statement was corrected in place; its
correction is not an additional finding or a request for a ruling.

### R3-F2 (low): Failure guidance omits runtime dictionary-default conflicts

**Authority: spec.** “One implementation and consistent guidance” requires
the crate's docs and skills to describe the same contract. The accepted
per-key conflict check remains in the implementation, but the following
guidance still claims it cannot happen during validation:

- [Validation failures](../../docs/topics/validation.md:400) says a
  `Prepared` returns `Unprepared` only for invalid or recursive defaults
  because `Prepared::new` already refused conflicting defaults.
- [Diagnostic error catalog](../../docs/topics/diagnostics.md:416) says
  `ConflictingDefaults` is raised before any value is considered.
- [SimplifiedSchema skill](../../../.claude/skills/simplified-schema/SKILL.md:62)
  presents conflict detection at lowering/`Prepared::new` without the
  dictionary-selection exception.

A direct public `Prepared::new` call succeeds for this schema:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "patternProperties": {
    "^x-": { "default": 1 },
    "-settings$": { "default": 2 }
  }
}
```

Its subsequent `validate_soft({"x-settings": null})` returns
`Unprepared(ConflictingDefaults)`, as the existing overlap regression
requires. The applying-defaults paragraph and focused `defaults.md` skill
already describe this exception correctly. Union-specific compile-time
claims remain correct.

**Recommended repair:** qualify the three broad statements and link to the
existing per-key explanation. Retain the current conflict behavior; stronger
compile-time regex-overlap analysis is unnecessary and was not requested.

## Blocked Findings

None. Both findings can be repaired within the spec and existing author
decisions. No item meets the test for stopping for a human, and no separate
post-cycle human review item is warranted.

## Recurrence

R3-F1 is a **newly found sibling** of review-2.md's R2-F1, which itself
extended review-1.md's R1-F1. The original instance lists are verified fixed.
The earlier sweeps tested invalid defaults within one declaring definition
and combined selectors with optional children, but did not combine those
two concerns: a default must be checked before another selector's required
child can be supplied. They also missed the literal `allOf`, reference,
array and tuple counterparts listed above. This review sweeps those routes
together rather than reporting another individual selector omission.

The boundary is the spec's explicit prohibition on repairing an invalid
parent default. It is not an open-ended request to implement arbitrary
JSON Schema default semantics. The high priority follows that named
acceptance requirement and the ordinary one-marker edit to an existing
fixture. R3-F2 is newly identified guidance drift; it does not reopen the
accepted runtime conflict decision.

## Verification and input robustness

All user-facing requirements here concern data, conversion, passive schema
checks, file loading, composition, or execution using fake services. Level 1
public APIs and hermetic subprocesses are appropriate. The spec adds no
terminal-rendering, mouse, or keyboard requirement needing Level 2 or 3.

| Requirement | Strongest relevant evidence checked | Assessment |
| --- | --- | --- |
| Missing/null/required table, borrowing, reports, falsey values, repeat stability | L1 `required_presence`, `required_defaults`, selector matrix | Original cases pass |
| Existing/defaulted parents, named/imported/recursive types, unions, arrays and tuple objects | L1 parent/nesting/union suites plus new public-API probes | R2-F1 repaired; complete default validity remains R3-F1 |
| Raw schemas, exported annotations, cached/uncached preparation | L1 raw-schema, plain-validator, envelope-mode and artifact/identity tests | Existing cases pass; raw default checks share R3-F1 |
| Not-empty per type, trigger presence, envelopes and validation phases | L1 not-empty, matcher, envelope and host phase suites | Pass |
| Declared but unwritten properties; parent/child propagation; source preservation | L1 `absent_property_contract`, compose/frontmatter/setter tests, CLI wiring | Pass |
| Lifecycle flag conversion, operands, placements, bundles, set and authored origin | L1 shared parser tests, both hosts' agreement suites, composition/overlay regressions and fake execution | Previous finding remains fixed |
| CLI validation/composition, whole YAML, DMLS diagnostics | L1 CLI and editor-protocol tests, including default/error and response-contract fixtures | Pass for exercised cases |
| Claudine override inheritance, keyed configuration and publication on refusal | L1 config/wire/JSON5/CLI matrices and schema/completion tests | Existing cases pass |

For file/configuration inputs, invalid shapes default to a located refusal
and no publication of failed completed data. Exceptions are the specified
missing/null inheritance, default application and admitted empty values.
Initialization keeps its documented position before validation; no event
ordering change is requested.

The review-2 repair changed the compiled default plan, not a loader.
Field shapes are exercised once per shared validator in `required_defaults`,
`pattern_defaults`, the envelope matrix and Claudine's configuration matrix.
The new object-default shapes and their controls are listed under R3-F1.
Document-level coverage remains in admitted-YAML `input_robustness`, JSON
file-loader tests, Markdown frontmatter tests and legacy-JSON5 tests:
duplicates, trailing/invalid content, YAML null spellings and second
documents, and JSON5 syntax are checked at their public boundaries, with
representative CLI wiring. No new loader-specific matrix is needed.

Searched the changed readers for `serde(default)`, `Option`, `filter_map`,
`unwrap_or_default()` and `.ok()`. The default compiler's required-name
extraction reads schema metadata; the compiled validator checks its shape.
Invalid key patterns are left to schema compilation. The consequential
omission is the default check's discarded definitions, inventoried as R3-F1.
The intentional null-as-unset configuration readers are exercised by the
public override/configuration matrices.

Review runs on macOS:

| Command | Result |
| --- | --- |
| `simplified-schema/`: `just test` | 2,290 feature-enabled and 1,931 minimal tests passed; none at or above 5 s |
| `darkmatter/`: `just test` | 16,730 passed, 10 recipe skips; none at or above 5 s |
| `claudine/`: isolated final `just test` | 12,170 passed, 15 recipe skips; none at or above 5 s |
| `claudine/`: `just test-cli tts_prewarm` | 26 passed; none at or above 5 s |
| `just lint` in all three areas | Passed; includes all three library feature builds and Darkmatter's Zed WASM check |
| `just check-tier-coverage simplified-schema darkmatter claudine` | Zero stranded tests |
| Public-API probes | R3-F1 rows 1–12 and object shapes reproduced; direct `Prepared` runtime conflict reproduced; temporary example removed |
| `git diff --check`; review metadata, ledger inventory and local-link checks | Passed |

An initial Claudine run was invalidated by this review's concurrent
`just lint`: its `lint-transport` recipe runs a CLI test build without
`test-fixtures`, replacing the executable the feature-enabled tests spawn.
Ten speech-preparation tests then saw no fixture recorder, and the
three-prompt test waited 13.811 s for records that build cannot produce.
After all concurrent builds stopped, the isolated 26-test module passed in
2.573 s total, with no slow tests. The complete final suite then passed
without concurrent builds; the three-prompt test took 1.636 s there.
The invalidated run is not passing evidence or an
implementation performance finding.

After recording the final result, `just slow-tests fresh` returned 1
because this review file changed after the run. Its output names only
`review-3.md` and explicitly says that file is outside the directories
those tests cover. The final runs recorded zero slow tests; no production
or test source changed after them.

The changed test module is declared by the consolidated `l1` target, with
`work-counters,terminal` enabled by its manifest's CI metadata; its tests
have no excluded tier marker. Existing host agreement modules are likewise
registered in their declared targets. Cross-OS results belong to CI and do
not block this review. No production code or permanent test was changed
by the review.

## Implementation record corrections

Updated `implementation.md` in place: selected-entry and combined-selector
defaults no longer remain defective as it claimed; union conflicts and
runtime dictionary conflicts now have distinct descriptions. Added a dated
note with verification. Corrected the repair implementation log (`log.md`)
in place where it claimed a non-declaring constraint would always cause a
refusal; the dated correction gives the successful required-child
reproduction. These are factual record corrections, separate from the two
findings and from human-review decisions.
