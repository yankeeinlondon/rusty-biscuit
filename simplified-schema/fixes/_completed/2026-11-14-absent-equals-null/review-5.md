---
$schema: feature-review.yaml
ready: false
findings:
    - "R5-F1 (medium): Union alternatives that cannot contain a key bypass default checks"
implemented: true
implemented_by: claude/opus
log: /Volumes/coding/wt/rusty-biscuit/fix-sequences/simplified-schema/fixes/2026-11-14-absent-equals-null/log.md
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-11-15T09:43:33-08:00"
spec: 2026-11-14-absent-equals-null/spec.md
description: "A **fix** review of `2026-11-14-absent-equals-null/spec.md`"
fix: 2026-11-14-absent-equals-null/review-5.md
previous: 2026-11-14-absent-equals-null/review-4.md
next: 2026-11-14-absent-equals-null/review-6.md
---

# Review 5: Absent equals null

The fix is **not production ready**. Review 4's listed cases are repaired.
One newly found sibling remains: an independent union's string shorthand,
or another alternative that cannot describe an object, lets an invalid
default inside an object escape its required-child check. An alternative
that forbids the key through `additionalProperties: false` does the same. The default is
then repaired by child defaults, or converted by soft validation. R5-F1
contains the complete bounded sweep. No finding needs a human decision.

## Previous Finding Verification

Read reviews 1–4, both original implementation records, and the repair's
`log.md` Finding Ledgers. No earlier review has a Human rulings section;
there is no previously blocked work or pending ruling to reopen. The
author's spec decisions, including the one-arm default policy and passive
preparation, remain the authority.

The table checks every child of the latest default-check ledger, including
its documentation and host-consumer additions. Test names without a prefix
are in SimplifiedSchema's
[independent-union tests](../../lib/tests/l1/pattern_defaults.rs:877).
Their shared helper asserts the public `Unprepared(InvalidDefault)` variant
and property path through **both** validation modes. Raw structural cases
exercise both `anyOf` and `oneOf`. Both feature builds ran those tests.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Pattern-keyed objects skip default preparation | All 19 ledger children | verified fixed | Full library builds pass selector, conflict, recursion and position controls; Darkmatter passes the `then`/`else` reporting regressions | No original instance |
| R1-F2: Lifecycle validation accepts inputs the parser rejects | All 23 ledger children | verified fixed | Shared authored checks remain wired into both hosts; full host suites exercise numeric/null flags, placement, bundles, nesting, overlays, completed provenance and parser depth | None |
| R2-F1: Dictionary selectors still skip or split default preparation | All 20 ledger children | verified fixed | `pattern_defaults::{entry_defaults,combined_selectors,raw_schemas}` pass in both builds; merged selection includes literal, matching-pattern and per-node catch-all definitions | No original instance |
| R3-F1: Used defaults omit constraints from other applicable definitions | All 18 ledger children | verified fixed | `complete_default_checks` and union-policy tests pass in both builds; nullable wrappers and declaration-check equivalence remain covered; corpus timings remain below 5 s | No original instance |
| R3-F2: Failure guidance omits runtime dictionary-default conflicts | All 11 ledger children | verified fixed | Read validation, diagnostics, global constraints, both defaults skills, and error/outcome/prepared/default-plan Rust docs: they distinguish declaration-time and per-key conflicts | None |
| R4-F1: Used defaults skip constraints in independent unions | All 22 ledger children, detailed below | verified fixed | Original object-only cases and controls pass in both builds; the new reproduction changes an alternative's parent type | New sibling R5-F1, not a partial repair of an original reproduction |
| R4-F1.1: SimplifiedSchema named alternatives | Overlapping prefix/suffix fixture | verified fixed | `simplified_named_alternatives_refuse_a_default_neither_accepts`, path `x-settings.options` | None of this object-only case |
| R4-F1.2: Raw overlapping patterns | Both union keywords | verified fixed | `overlapping_raw_patterns_refuse_the_default` | None |
| R4-F1.3: Literal plus pattern | Both union keywords | verified fixed | `a_literal_and_a_matching_patterns_union_refuse_the_default` | None |
| R4-F1.4: Literals across `allOf` | Both union keywords | verified fixed | `a_literal_merged_across_all_of_refuses_the_default` | None |
| R4-F1.5: Pattern plus catch-all | Both union keywords | verified fixed | `a_pattern_and_another_members_catch_all_union_refuse_the_default` | None |
| R4-F1.6: Separate catch-alls | Both union keywords | verified fixed | `separate_catch_alls_across_all_of_refuse_the_default` | None |
| R4-F1.7: Local references | Both union keywords; `{}` | verified fixed | `reference_targets_across_all_of_refuse_the_default`, path `options` | None |
| R4-F1.8: Array items | Both union keywords; `[{}]` | verified fixed | `joined_array_items_refuse_the_default`, path `0.options` | None |
| R4-F1.9: Tuple positions | Existing first object and empty tuple | verified fixed | `joined_tuple_positions_refuse_the_default_and_insert_no_position` | None |
| R4-F1.10: Common root default | Both union keywords | verified fixed | `a_common_default_beside_a_root_union_is_refused` | None of this object-only case |
| R4-F1.11: Two independent unions | Default on one union, constraints on another | verified fixed | `a_default_on_one_union_is_judged_against_another_independent_union` | None of this object-only case |
| R4-F1.12: Scalar default | Text `"3"` versus number/Boolean property constraints | verified fixed | `a_scalar_default_beside_a_union_is_refused_without_conversion` | None of the original alternatives |
| R4-F1.13: Public default preparation | Compile plan and prepare directly | verified fixed | `default_preparation_refuses_before_completing_children` | None of original fixture |
| R4-F1.14: Public strict validation | Direct `Prepared` | verified fixed | `prepared_verdicts_refuse_the_default`, strict | None of original fixture |
| R4-F1.15: Public soft validation | Direct `Prepared` | verified fixed | Same test, soft | None of original fixture |
| R4-F1.16: Property-local union | Union inside defaulted property's definition | verified fixed | `a_union_inside_the_properties_own_definition_refuses_the_default` | None |
| R4-F1.17: Direct conjunctions | All original R3-F1 cases | verified fixed | `complete_default_checks` | None |
| R4-F1.18: One-arm policy | Genuine sibling alternatives | verified fixed | `required_defaults::unions`; code skips conflicting chosen members | None; accepted policy preserved |
| R4-F1.19: Nullable wrappers | Original null/object selector fixture | verified fixed | `complete_default_checks::nullable_selector_values_still_refuse_the_default` | Direct null case remains clean; referenced null is in R5-F1 |
| R4-F1.20: Positive controls | Optional child, authored `{}`, written false, repeat stability, key absent from one object arm | verified fixed | `authored_values_beside_an_independent_union_are_still_completed`, both `an_independent_union_member_*` controls, and original shape/combined-selector controls | None |
| R4-F1.21: Comments and guidance | Module/helper docs, validation, unions, diagnostics and defaults skill | verified fixed | Read all listed sites against the original examples; they describe the repaired object-only behavior | Qualify the newly exposed object/key admission distinction with R5-F1 |
| R4-F1.22: Host consumers | Darkmatter completion and Claudine composition | verified fixed | Darkmatter `EffectiveSchema::complete` delegates to `prepare_defaults`; Claudine uses that path; no independent host default-validity walk; full suites exercise consumers | New library sibling reaches the same consumers |
| R4-F2: Claudine L1 tests exceed the timing budget | 20 flagged rows, 132 control rows, shared-work row | verified fixed | 12,170 passed; all 152 original cases found by exact name; flagged cases 1.126–3.425 s; one control at 5.017 s cleared at 2.562 s alone with matching features | None; no speculative test rewrite needed |

Every flagged timing child is checked below against the final canonical package run. The other 132 original rows also executed; their maximum was 5.017 s, cleared by the same-feature isolated measurement described under verification.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R4-F2.54: `sequence_groups::a_parallel_group_commits_one_named_result` | Original flagged CLI workflow | verified fixed | 1.956 s in final canonical package run | None |
| R4-F2.56: `sequence_groups::group_variables_reach_members_and_do_not_leak_to_the_next_step` | Original flagged CLI workflow | verified fixed | 1.879 s in final canonical package run | None |
| R4-F2.57: `sequence_groups::outputs_chain_through_root_sequence_tasks` | Original flagged CLI workflow | verified fixed | 1.914 s in final canonical package run | None |
| R4-F2.58: `sequence_groups::parallel_group_members_keep_separate_provider_sessions` | Original flagged CLI workflow | verified fixed | 1.867 s in final canonical package run | None |
| R4-F2.62: `sequence_initialize_include_preflight::existing_include::a_dry_run_whose_include_exists_still_succeeds_without_side_effects` | Original flagged CLI workflow | verified fixed | 1.322 s in final canonical package run | None |
| R4-F2.63: `sequence_initialize_include_preflight::existing_include::a_sequence_whose_include_exists_runs_unchanged` | Original flagged CLI workflow | verified fixed | 1.426 s in final canonical package run | None |
| R4-F2.64: `sequence_jit::a_body_shell_expansion_reads_the_filesystem_at_its_turn` | Original flagged CLI workflow | verified fixed | 1.376 s in final canonical package run | None |
| R4-F2.66: `sequence_jit::a_later_step_composes_against_the_edit_an_earlier_step_made` | Original flagged CLI workflow | verified fixed | 1.350 s in final canonical package run | None |
| R4-F2.69: `sequence_jit::a_set_from_an_earlier_step_is_composed_into_a_later_step` | Original flagged CLI workflow | verified fixed | 1.951 s in final canonical package run | None |
| R4-F2.72: `sequence_jit::an_authored_state_mutation_does_not_override_the_sequence_binding` | Original flagged CLI workflow | verified fixed | 1.965 s in final canonical package run | None |
| R4-F2.97: `tts_prewarm::playa_dry_run_prepares_nothing_and_leaves_no_audio_artifacts` | Original flagged CLI workflow | verified fixed | 3.425 s in final canonical package run | None |
| R4-F2.104: `wrap_compose_validation::predicates_operands_and_proxy_with_values_are_rejected_before_launch` | Original flagged CLI workflow | verified fixed | 2.350 s in final canonical package run | None |
| R4-F2.118: `wrap_inline_compose::inline_compose_uses_a_caller_supplied_prompt_without_persisting_it` | Original flagged CLI workflow | verified fixed | 1.755 s in final canonical package run | None |
| R4-F2.119: `wrap_inline_compose::inline_compose_without_an_eager_prompt_fails_before_launch_naming_it` | Original flagged CLI workflow | verified fixed | 2.054 s in final canonical package run | None |
| R4-F2.126: `wrap_sequence_composition::sequence_dry_run_fail_fast_on_composition_error` | Original flagged CLI workflow | verified fixed | 2.069 s in final canonical package run | None |
| R4-F2.128: `wrap_sequence_composition::sequence_dry_run_quiet_and_silent_are_no_op` | Original flagged CLI workflow | verified fixed | 1.703 s in final canonical package run | None |
| R4-F2.129: `wrap_sequence_composition::sequence_dry_run_single_invalid_renders_state_per_step` | Original flagged CLI workflow | verified fixed | 1.126 s in final canonical package run | None |
| R4-F2.130: `wrap_sequence_composition::sequence_dry_run_single_not_installed_renders_state_per_step` | Original flagged CLI workflow | verified fixed | 1.189 s in final canonical package run | None |
| R4-F2.131: `wrap_sequence_composition::sequence_dry_run_zero_installed_list_renders_state_per_step` | Original flagged CLI workflow | verified fixed | 1.142 s in final canonical package run | None |
| R4-F2.132: `wrap_sequence_composition::sequence_live_auto_selectable_launches_provider` | Original flagged CLI workflow | verified fixed | 1.436 s in final canonical package run | None |
| R4-F2.controls: previously cleared cases | Remaining 132 named rows in review 4 | verified fixed | All executed; no result at or above 10 s; lone 5.017 s case is 2.562 s alone with matching features | None |
| R4-F2.shared: shared operation cost | Composition/schema/subprocess work reached by the cases | verified fixed | All original cases meet the timing budget in the final package run; no remaining measured operation requiring a speculative rewrite | None |

## Unblocked Findings

### R5-F1 (medium): Union alternatives that cannot contain a key bypass default checks

**Authority: spec, “Validation and default preparation.”** “An object supplied
by a default must already satisfy its complete required-child contract;
defaults do not repair an invalid default object.” Also: “Preserve strict
checking of a used default against the complete property definition, without
conversion or file access.” A schema combining common object settings with
a string shorthand is a realistic input. This repository already uses
scalar-or-object shorthand unions, for example Claudine's
[sequence step types](../../../claudine/schemas/sequence-types.yaml:22).
The reproduction edits the existing selector fixture's second alternative
to `string`; it does not introduce new syntax or conditional evaluation.

**Defect class:** an alternative with no definition for a key is treated as
accepting that key's default even when the alternative cannot describe the
object containing the key, or expressly forbids that key.

In the simplified-schema package,
[`Fill::accepts`](../../lib/src/defaults.rs:1139) accepts an empty selection
for an alternative unless
[`admits_only_null`](../../lib/src/defaults.rs:1301) recognizes a direct
null-only schema. String, number, Boolean and array alternatives also cannot
hold an object key, but are counted as accepting. A local reference to a
null-only schema escapes even that existing exclusion. The selected default
therefore reaches child completion or conversion without the required
strict refusal.

The same decision accepts an object alternative with
`additionalProperties: false` that does not name the key. The compiler's
[`members`](../../lib/src/defaults.rs:614) keeps only object-valued catch-all
schemas, dropping this Boolean denial, so the alternative again has an empty
selection. Direct and locally referenced closed alternatives both let the
invalid default be repaired. In contrast, `properties: { options: false }`
and `patternProperties: { '^options$': false }` retain their denials and
correctly refuse it. This is a key-admission distinction, without choosing
an arm by validating the unfinished parent.

This ordinary SimplifiedSchema document reproduces it:

```yaml
types:
  Parent:
    options: object(default({}))
  Nested:
    - options: "{ enabled: boolean(required; default(true)) }"
    - string
$schema:
  '<starting::x->': Parent
  '<ending::-settings>': Nested
```

For input `{ "x-settings": {} }`, strict validation succeeds, borrows the
original input, and reports defaults for both `x-settings.options` and
`x-settings.options.enabled`. Soft validation succeeds with:

```json
{ "x-settings": { "options": { "enabled": true } } }
```

Both must report `Unprepared(InvalidDefault)` at `x-settings.options` before
filling `enabled`. The containing value is an object, so the `string`
alternative cannot supply a valid alternative contract for its `options`.
An authored `options: {}` is different and may receive its child's default.

A compact raw JSON Schema reproduction, with input `{}`, is:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": { "options": { "type": "object", "default": {} } },
  "anyOf": [
    {
      "type": "object",
      "properties": {
        "options": {
          "type": "object",
          "required": ["enabled"],
          "properties": { "enabled": { "type": "boolean", "default": true } }
        }
      }
    },
    { "type": "string" }
  ]
}
```

A plain validator rejects `{ "options": {} }` and accepts
`{ "options": { "enabled": true } }`. Public `prepare_defaults` returns
the latter and two default reports; both public `Prepared` verdicts and
both facade verdicts accept it. This confirms the defect before any host
adapter. The same issue lets a raw default `"3"` be converted to number `3`
when its independently imposed union is an object requiring numeric
`timeout`, or a string shorthand. Strict gives a data mismatch; soft
succeeds. Both should instead give schema `InvalidDefault` at `timeout`.

The sweep used the previous repair's parent/default and required-child
fixture, changing only the second alternative. Every raw structural route
below was run with both union keywords, both validation modes, and each of
`string`, `number`, `boolean`, `array`, `null`, and `object` as that
alternative's type. It ran in feature-enabled and minimal builds.
Each of those ten routes was also run with a direct or locally referenced
closed object as the second alternative, and with explicit false property
and pattern definitions as controls, in both modes and both builds.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R5-F1.1 | SimplifiedSchema named union, overlapping prefix/suffix selectors | Second object alternative changed to `string`; `x-settings: {}` | Both modes accept parent and required-child defaults | `InvalidDefault` at `x-settings.options` |
| R5-F1.2 | Raw overlapping patterns | Same typed alternatives | Both modes accept parent plus required child for the four non-null non-object types | Refuse parent default |
| R5-F1.3 | Raw literal plus pattern | Same alternatives | Same acceptance | Same refusal |
| R5-F1.4 | Literals across `allOf` | Same alternatives | Same acceptance | Same refusal |
| R5-F1.5 | Pattern plus another member's catch-all | Same alternatives | Same acceptance | Same refusal |
| R5-F1.6 | Separate catch-alls | Same alternatives | Same acceptance | Same refusal |
| R5-F1.7 | Local references joining parent and union | Input `{}` | Both accept `options.enabled = true` | `InvalidDefault` at `options` |
| R5-F1.8 | Joined array item definitions | Input `[{}]` | Both accept completed first object | `InvalidDefault` at `0.options` |
| R5-F1.9 | Joined tuple position definitions | Existing first object | Both accept completed first object | Same refusal; retain no inserted positions |
| R5-F1.10 | Common root default beside union | Input `{}` | Both accept repaired default | `InvalidDefault` at `options` |
| R5-F1.11 | Two independent unions | First has parent-default/string arms; second has required-child/scalar arms | Both accept repaired default | Same refusal; preserve independent grouping |
| R5-F1.12 | Alternative's static shape | Type array excluding objects; scalar `const`; scalar-only `enum`; false schema; local references to string and null types | All six accept repaired default | Exclude alternatives that cannot hold the containing object; refuse default |
| R5-F1.13 | Scalar default under the same decision | Common `timeout.default = "3"`; independent object-number/string union | Strict data mismatch; soft accepts `3` | Schema `InvalidDefault` in both modes, before conversion |
| R5-F1.14 | Public `DefaultPlan::compile` → `prepare_defaults` | Raw common-root object/string fixture | Returns repaired default and two reports | Refuse before child completion |
| R5-F1.15 | Public `Prepared::validate_strict` | Same fixture | Accepts borrowed input and two reports | `Unprepared(InvalidDefault)` |
| R5-F1.16 | Public `Prepared::validate_soft` | Same fixture | Accepts completed copy and two reports | Same refusal |
| R5-F1.17 | Property-local union control | Entire union inside property's own definition | Existing public regressions refuse default, both keywords/modes | Clean; retain |
| R5-F1.18 | Direct conjunction controls | Every R3-F1 fixture | Refuse invalid default | Clean; retain |
| R5-F1.19 | Independent object-only union controls | Every R4-F1 fixture | Refuse invalid default | Clean; retain |
| R5-F1.20 | Default's own union controls | Number-default/string and nested loose/strict alternatives | Existing one-arm tests pass | Clean; do not check a default against every sibling arm |
| R5-F1.21 | Direct null alternative | All ten raw routes, both keywords/modes | Refuse invalid default | Clean; retain, unlike the referenced-null sibling in .12 |
| R5-F1.22 | Property value shapes | Absent/null `options`; authored `{}`; authored `enabled: false`; numeric, empty text and array values | Absent/null incorrectly get parent and child; authored object gets child only; false preserved; other types refused at `/options` | Refuse only the used invalid parent default; retain authored behavior |
| R5-F1.23 | Object-admitting alternative controls | Direct object; type array containing object; local reference to object | Default check allows completion; final `oneOf` still rejects overlapping matches where applicable | Clean; an object arm that omits the key remains a valid unconstrained alternative |
| R5-F1.24 | Comments/docs/skills describing alternative admission | Module and helper Rust docs; validation/unions/diagnostics topics; defaults skill | Say an alternative without the property accepts, without the containing-object distinction | Update with the repaired admission rule; preserve one-arm policy and runtime-conflict guidance |
| R5-F1.25 | Downstream decisions | Facade strict/soft, Darkmatter completion, Claudine composition | First two reproduced; latter two delegate to the same preparation with no independent validity check | Repair shared library once; hosts inherit it |
| R5-F1.26 | Boolean catch-all admission, every structural route .2–.11 | Direct and local-reference object alternative with `additionalProperties: false`; both keywords and modes | All 40 route/keyword/representation cases accept the repaired default in each build | Retain the selected key's explicit denial and refuse the invalid default |
| R5-F1.27 | Boolean property/pattern controls, every structural route .2–.11 | Alternative expressly declares `options: false`, or a matching false pattern; both keywords and modes | All 40 route/keyword/representation cases refuse with `InvalidDefault` in each build | Clean; retain existing Boolean definition checks |

**Recommended repair:** distinguish an alternative that permits the
containing object and its key but says nothing about the key's value from
an alternative that cannot contain that object or forbids the key. Apply the distinction consistently when
recording union choices and when admitting an empty member selection,
including the listed static type/constant/enum/Boolean forms and local
references. Preserve `additionalProperties: false` as a denial when selecting
the key; literal and pattern Boolean definitions already work. Retain unknown
or open object-admitting alternatives as unconstrained controls. Do not validate the unfinished parent to choose an arm: that would
make defaults part of matching and could reject an authored object whose
child default should complete it. No general conditional evaluator, new
syntax, per-host walk or arbitrary schema satisfiability layer is requested.
Keep the repair at the existing structural admission/check boundary and use
the tables above as its finite regression scope.

## Blocked Findings

None. R5-F1 is an ordinary repair within the spec. No pending human review
or cross-OS evidence blocks it.

## Recurrence

R5-F1 is a **newly found sibling** of review-4.md's R4-F1, and the same
default-check class as review-3.md's R3-F1. Their listed reproductions are
verified fixed. This follows the selected-entry/default-check path from
review-2.md's R2-F1 and the selector preparation work in review-1.md's R1-F1;
their original instances remain closed too. Review 4 swept independent unions whose alternatives both
describe objects and retained a direct null-only control. It did not sweep
scalar/array alternatives without the key, the static forms that express
the same exclusion, closed alternatives that forbid the key, or local references to those alternatives. Those are
now included across every supported structural route and public consumer.

Successive repairs have expanded machinery for an increasingly unusual
intersection of selectors, defaults and union constraints. That is a sign
the requirement risks having no practical boundary. This finding is
**medium**, rather than extending the earlier high priority: this exact
combination of independent defaults and parent/key admission is not named in an acceptance example, and no
shipped fixture was shown to combine these features. The bounded static
admission repair above is sufficient; it does not ask for general parent
arm matching or handling the deliberately excluded conditional keywords.

## Verification and input robustness

All behavior in this spec is schema/data processing. L1 public results and
hermetic CLI/fake-service tests are the appropriate level. No requirement
asserts terminal glyph rendering, keyboard encoding, mouse behavior or
browser interaction, so L2/L3 evidence is not missing.

| User-facing requirement | Strongest relevant verification | Result |
| --- | --- | --- |
| Presence-only `required`; optional and required defaults; null/falsey values; borrowed/owned reports; unchanged input; repeat stability | L1 `required_presence`, `required_defaults` and default-engine tests | Existing contract cases pass |
| Existing/missing/null parents, array/tuple objects, recursive/imported types, pending values, union conflicts and invalid defaults | L1 parent/nesting/pending/union/pattern suites and direct public probes | R5-F1 is the remaining used-default defect |
| Raw schemas versus plain exported JSON Schema; phase projections, triggers, cached/uncached preparation and artifact identity | L1 raw/export/phase/matcher/envelope/artifact tests | Pass |
| Envelope metadata/carriers, Claudine inheritance and keyed payloads, default-sensitive key counts | L1 envelope/config field and loader matrices | Pass |
| Lifecycle flag conversion, mandatory targets, placement, bundles, generated verbs, `set`, completed authored provenance | L1 schema/parser parity and fake-service execution | Pass |
| `md schema validate`, composition, whole YAML, DMLS; declared-but-unwritten parent/child behavior and no source writes | L1 host/CLI/editor suites and absent-property contract tests | Pass |
| `not-empty` per supported type, unsupported-type errors, list bound equivalence, empty YAML interpolation | L1 `not_empty_values` and host frontmatter tests | Pass |

The field/loader matrix remains applicable to the original schema/config
reader changes. Default outcome: malformed input is refused with a located
public error. Exceptions are the spec's null-as-unset fields, permitted soft
conversions and preserved raw-schema defaults; required presence is checked
separately. Read the public-result coverage in `required_defaults::input_shapes`,
`envelope_null_carriers`, `input_robustness`, host lifecycle parity, Claudine
keyed/config matrices and JSON5/YAML CLI wiring. Their controls, field edits,
wrong whole/element types, empty values, duplicate keys, invalid/trailing
content and format-specific syntax remain selected by the executed suites.

Searched the changed readers for `serde(default)`, `Option`, filtered
elements, `unwrap_or_default()` and `.ok()`. Claudine's typed null-as-unset
adapters intentionally implement the spec's inheritance exception; wrong
types still fail typed deserialization. The default engine's filtered
schema arrays are followed by schema-validator compilation. Pattern compile
errors are also refused by that validator. No additional silent field-loss
defect was found. The new alternative-admission issue is R5-F1, with all
alternative shapes and property shapes listed in its table; it is not a
new file loader.

Review commands on macOS:

| Command | Result and practical limit |
| --- | --- |
| `simplified-schema/`: `just test` | 2,325 feature-enabled and 1,966 minimal tests passed; none at or above 5 s |
| `darkmatter/`: `just test` | 16,730 assertions passed, 10 recipe skips; one 5.184 s result cleared below |
| `claudine/`: final sequential `just test` | 12,170 assertions passed, 15 recipe skips; one 5.017 s result cleared below; all 152 original timing cases executed |
| Same-feature isolated timing checks, nextest `-j 1` | Six Claudine cases passed at 1.952–2.562 s; Darkmatter case passed at 1.432 s |
| `claudine/`: `just test-cli tts_prewarm::` | All 26 tests passed, none at or above 5 s, after restoring the fixture build |
| Public probes, default and minimal library builds | Reproduced every R5-F1 raw structural route, shape controls, direct preparation/validation and plain-validator contrast; confirmation also checked the SimplifiedSchema strict verdict and scalar `oneOf` case |
| `just lint` in all three areas | Exit 0 each, including Claudine transport guards and Darkmatter Zed WASM check |
| `just check-tier-coverage simplified-schema darkmatter claudine` | Zero stranded tests; consolidated L1 module declarations and CI feature selection also read directly |
| Review metadata and whitespace checks | YAML frontmatter parses; inventory/readiness agree; previous implementation flags preserved; next link and iteration updated; `git diff --check` clean |

Slow results were assessed using the required loaded-host rule. Sniff reports
16 cores. Load samples were 37.24–75.09 during the first runs, 63.41 during
the sequential Claudine run and 34.69 at its end. Each valid package-run
result below is under ten seconds, and its isolated result with the same
features is under five seconds. The final isolated runs were sequential;
no test was renamed, ignored, retried by nextest or given a longer deadline.

| Test | Package timing (s) | Isolated timing with matching features (s) | Assessment |
| --- | --- | --- | --- |
| Darkmatter `markdown::compose::shell_expansion::probe::tests::installed_shells::injection_shaped_names_never_execute` | 5.184 | 1.432 (`effects-instrumentation,work-counters`) | Cleared |
| Claudine `claudine_schemas::cancelled_conditional_depth_is_the_parsers_alone_to_limit` | 5.900 | 2.150 | Cleared |
| Claudine `claudine_schemas::group_cancelled_rejects_forbidden_actions_in_the_deepest_conditionals` | 5.113 | 1.952 | Cleared |
| Claudine `claudine_schemas::nested_group_cancelled_rejects_forbidden_actions_in_the_deepest_conditionals` | 5.226 | 1.976 | Cleared |
| Claudine `claudine_schemas::shared_payload_properties_have_identical_types` | 6.410 | 2.379 | Cleared |
| Claudine `claudine_schemas::task_cancelled_rejects_forbidden_actions_in_the_deepest_conditionals` | 5.176 | 1.990 | Cleared |
| Claudine `container_schema_recursion::deep_container_deferral_does_not_hide_literal_or_shape_errors_in_either_phase` | 5.327; 5.021 in interrupted rerun; 5.017 in final run | 2.562 | Cleared |

The Claudine isolation run used the same area package selection and
`claudine/work-counters`, `claudine-cli/test-fixtures` features as the full
recipe, filtering these six exact test names with `-j 1`.

Two earlier Claudine runs were interrupted by my verification orchestration:
Claudine's `lint-transport` and the tier listing each run nextest builds
without the CLI recorder/helper features, replacing executables that the
ongoing suite then spawned. The first recorded ten recorder failures,
including a 13.793 s timeout waiting for a recorder absent from that build;
the second recorded six fixture-handshake failures. These were failed
verification attempts, not passing implementation evidence. After all
build-producing auxiliary checks finished, the final canonical run rebuilt
the correct feature graph and passed all 12,170 tests, including those
fixtures. The tables use that complete run for the previous finding's
verdict, and retain the library timing observations and isolated checks
above. No implementation or test behavior was changed to make them pass.


Public probes were temporary examples using the crate's public API, copied
from the existing repair fixtures and run with default features and
`--no-default-features`. They were removed after execution. No production
source or permanent test was changed by this review. Cross-OS proof remains
with CI and does not create a finding.

## Implementation record corrections

Corrected `implementation.md` in place: its summary still said the original
R4-F1 object-only cases were broken. They now pass; the new mixed-type
alternative is a separate sibling. Added a dated note naming both builds
and the direct public probes. Also narrowed the repair log's blanket
completion-report claim that all independent-union defaults are strictly
checked; the original fixtures are fixed but the mixed-type or key-forbidding
member is incorrectly accepted. These are factual record corrections, not findings
or human-review items. The spec body is unchanged.
