---
$schema: feature-review.yaml
ready: false
findings:
    - "R4-F1 (high): Used defaults skip constraints in independent unions"
    - "R4-F2 (high): Claudine L1 tests exceed the timing budget"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-11-15T08:55:12-08:00"
spec: 2026-11-14-absent-equals-null/spec.md
implemented: true
log: /Volumes/coding/wt/rusty-biscuit/fix-sequences/simplified-schema/fixes/2026-11-14-absent-equals-null/log.md
implemented_by: claude/opus
description: "A **fix** review of `2026-11-14-absent-equals-null/spec.md`"
fix: 2026-11-14-absent-equals-null/review-4.md
previous: 2026-11-14-absent-equals-null/review-3.md
next: 2026-11-14-absent-equals-null/review-5.md
---

# Review 4: Absent equals null

The fix is **not production ready**. Both findings from review 3 are verified
fixed for every instance in the repair ledger. A new sibling remains: a
default can still be repaired by its required child's default when another
applicable definition contains a union. The same omission lets soft validation
convert a schema default that must be checked strictly. This is one finding,
R4-F1, with a bounded repair in the shared default engine. A separate timing
finding, R4-F2, records Claudine tests that exceed the review's ten-second
ceiling even on a loaded host. Neither finding needs a human decision.

## Previous Finding Verification

Read all three earlier reviews, the original implementation records, and the
repair's `log.md` Finding Ledgers. None of the earlier reviews contains a
Human rulings section. Earlier findings were already closed against their
listed instances; they remain included here for continuity. Completion flags
and ledger dispositions were checked against code, executed public results,
and the tests themselves.

The following table includes all 18 R3-F1 children and all 11 R3-F2 children,
including the repair's additional cases. Unless a different module is named,
default-check test names below are in SimplifiedSchema's
[pattern-default tests](../../lib/tests/l1/pattern_defaults.rs:634).

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Pattern-keyed objects skip default preparation | R1-F1.1–R1-F1.19, closed in review 2 | verified fixed | All selector, nested/defaulted-parent, conflict, positional and recursive regressions pass in both library builds; Darkmatter's full suite passes the conditional `then`/`else` default-report regressions | No original instance; the union-check sibling is R4-F1 |
| R1-F2: Lifecycle validation accepts inputs the parser rejects | R1-F2.1–R1-F2.23, closed in review 2 | verified fixed | Shared authored-placement/flag checks remain wired into both hosts; full host suites exercise their agreement, nesting, overlays, completed defaults and parser-only depth boundary | None |
| R2-F1: Dictionary selectors still skip or split default preparation | R2-F1.1–R2-F1.20, closed in review 3 | verified fixed | Both library suites pass entry-default, merged-selector, required-name, required-null, conflict and one-pass-stability tests; selection still merges literal, matching-pattern and applicable catch-all definitions | No listed instance; R4-F1 concerns validity across an additional union |
| R3-F1: Used defaults omit constraints from other applicable definitions | R3-F1.1–R3-F1.18 | verified fixed | Every ledger child checked below, including the later null-wrapper and performance repairs | R4-F1 is a newly found sibling, not a failed original reproduction |
| R3-F1.1: SimplifiedSchema overlapping selectors | Required child added to the existing prefix/suffix fixture | verified fixed | `overlapping_simplified_patterns_refuse_a_default_missing_another_selectors_required_child` checks `InvalidDefault` at `x-settings.options` in both modes | None of this direct-conjunction case |
| R3-F1.2: Raw overlapping patterns | Same required-child fixture | verified fixed | `overlapping_raw_patterns_refuse_the_default` | None |
| R3-F1.3: Literal plus pattern | Same default and required-child constraint | verified fixed | `a_literal_property_and_a_matching_pattern_refuse_the_default` | None |
| R3-F1.4: Literal properties across `allOf` | Same constraint split | verified fixed | `a_literal_merged_across_all_of_members_refuses_the_default` | None |
| R3-F1.5: Pattern plus another member's catch-all | Same constraint split | verified fixed | `a_pattern_and_another_members_catch_all_refuse_the_default` | None |
| R3-F1.6: Separate catch-alls | Same constraint split | verified fixed | `catch_alls_of_separate_all_of_members_refuse_the_default` | None |
| R3-F1.7: Local references | Default and required child in referenced `allOf` members | verified fixed | `reference_targets_joined_by_all_of_refuse_the_default`, at `options` | None |
| R3-F1.8: Array items | Existing `[{}]`, joined item schemas | verified fixed | `array_items_joined_by_all_of_refuse_the_default`, at `0.options` | None |
| R3-F1.9: Tuple positions | Existing first object; empty-tuple control | verified fixed | `tuple_positions_joined_by_all_of_refuse_the_default_and_insert_no_position` checks refusal and preserves `[]` | None |
| R3-F1.10: Split scalar type | Default `"3"` and `type: number` in separate literal `allOf` definitions | verified fixed | `a_scalar_default_is_checked_against_the_type_another_member_declares` checks schema `InvalidDefault`, not a data mismatch or conversion | None of this direct-conjunction case |
| R3-F1.11: Complete definition control | Required child and parent default in one definition | verified fixed | `a_complete_definition_in_one_schema_refuses_the_default`, SimplifiedSchema and raw controls | None |
| R3-F1.12: Property-local `allOf` control | Entire conjunction inside the property's definition | verified fixed | `all_of_inside_the_properties_own_definition_refuses_the_default` | None |
| R3-F1.13: Optional-child control | Valid parent default, optional child from another selector | verified fixed | `combined_selectors::overlapping_simplified_patterns_complete_parent_and_child_in_one_pass`, including repeat stability | None |
| R3-F1.14: Union policy controls | One-arm defaults, conflicts, nested alternatives and conjunctions within an arm | verified fixed | `required_defaults::unions`, including `a_nested_default_on_one_arm_is_not_checked_against_the_other_arm` and `a_default_is_checked_against_every_definition_along_its_arm` | None of the listed cases; an independent union is R4-F1 |
| R3-F1.15: Value shapes | Absent/null parent; authored empty object; false; wrong scalar/container | verified fixed | `options_shapes_beneath_the_required_child_fixture` and the absent-parent test exercise the public failure variant and paths in both modes | None |
| R3-F1.16: Comments and guidance | Module/check docs, validation topic and defaults skill | verified fixed | Read the updated contract and the repaired example; direct conjunctive constraints are now checked before child completion | The new union omission violates that same contract; covered by R4-F1 |
| R3-F1.17: Nullable wrappers | Matching selectors under null/object wrappers | verified fixed | `nullable_selector_values_still_refuse_the_default`; null-only wrappers do not introduce an independent choice | None |
| R3-F1.18: Compilation cost | Shipped-schema corpus test and declaration-check equivalence | verified fixed | `check_declarations_decides_reference_loops_as_compilation_does` passes in both builds; Darkmatter's corpus test takes 3.161 s in the full area run and 3.601 s alone | None; full area run has no tests at or above 5 s |
| R3-F2: Failure guidance omits runtime dictionary-default conflicts | R3-F2.1–R3-F2.11 | verified fixed | Every required guidance site checked below against the existing runtime overlap-conflict regressions | None |
| R3-F2.1: Validation failure guidance | Include per-key conflicts in `Unprepared` | verified fixed | [validation.md](../../docs/topics/validation.md:413) names the exception and links to default application | None |
| R3-F2.2: Diagnostic catalog | Qualify conflict timing | verified fixed | [diagnostics.md](../../docs/topics/diagnostics.md:416) distinguishes preparation-time and per-key validation-time conflicts | None |
| R3-F2.3: Main repo skill | Qualify lowering/preparation conflict detection | verified fixed | [SKILL.md](../../../.claude/skills/simplified-schema/SKILL.md:62) includes the dictionary exception | None |
| R3-F2.4: Sibling guidance sweep | Other docs, skills and Rust API comments | verified fixed | Searched current guidance in all three areas; remaining unconditional compile-time statements describe union declarations, not per-key overlap | None |
| R3-F2.5: Default-rule heading | Remove blanket pre-value timing claim | verified fixed | [validation.md](../../docs/topics/validation.md:306) now says “One default per property” | None |
| R3-F2.6: Global constraints topic | Add runtime exception | verified fixed | [global-type-constraints.md](../../docs/topics/global-type-constraints.md:59) qualifies the conflict rule | None |
| R3-F2.7: Defaults skill error list | Include per-key `Fill::fill` conflicts | verified fixed | [defaults.md](../../../.claude/skills/simplified-schema/defaults.md:103) describes both timings | None |
| R3-F2.8: Schema error API docs | Explain per-key detection | verified fixed | [errors.rs](../../lib/src/errors.rs:948) carries the exception | None |
| R3-F2.9: Validation failure API docs | Include selector-conflict cause | verified fixed | [outcome.rs](../../lib/src/outcome.rs:62) names runtime selector conflicts | None |
| R3-F2.10: Strict/soft API docs | Include conflict and recursive-default errors | verified fixed | [prepared.rs](../../lib/src/prepared.rs:281) documents both methods consistently | None |
| R3-F2.11: Default-plan API docs | Separate compile-time and per-key conflict checks | verified fixed | [defaults.rs](../../lib/src/defaults.rs:271) says unnamed selected keys are checked during preparation | None |

## Unblocked Findings

### R4-F1 (high): Used defaults skip constraints in independent unions

**Authority: spec.** “An object supplied by a default must already satisfy
its complete required-child contract; defaults do not repair an invalid
default object,” and “Preserve strict checking of a used default against the
complete property definition, without conversion or file access.” Both are
in “Validation and default preparation.” A settings object with alternative
required fields reaches the defect through ordinary SimplifiedSchema; raw
schemas also permit a numeric-text default to be converted rather than
strictly refused. These are explicitly named acceptance requirements.

**Defect class:** the default checker drops an independent union's entire
constraint when deciding which definitions apply together, while subsequent
child completion and conversion still use that union.

In the simplified-schema package,
[`within`](../../lib/src/defaults.rs:211) requires every recorded union choice
on a definition's route to occur on a default's declaring route.
[`Fill::check`](../../lib/src/defaults.rs:1092) skips a definition otherwise.
This correctly separates sibling alternatives of the *same* union. However,
an independently imposed union is also skipped when the default is outside
it, or on a different independent union's route. Its alternatives must remain
a group that constrains the property; dropping the group is not equivalent
to preserving alternatives.

[`Fill::fill`](../../lib/src/defaults.rs:1050) then descends into all selected
child locations. A default `{}` that fails every alternative can acquire the
required child before the final validator sees it. Copy the existing
parent/required-child selector fixture and give `Nested` two alternatives:

```yaml
types:
  Parent:
    options: object(default({}))
  Nested:
    - options: "{ enabled: boolean(required; default(true)) }"
    - options: "{ mode: string(required) }"
$schema:
  '<starting::x->': Parent
  '<ending::-settings>': Nested
```

Through the public simplified-schema API:

```rust
let schema = simplified_schema::SimplifiedSchema::try_from(text).unwrap();
let input = serde_json::json!({ "x-settings": {} });
let accepted = schema.validate_soft(&input).unwrap();
assert_eq!(accepted.value["x-settings"]["options"]["enabled"], true);
// Actual: success, after applying options = {} and enabled = true.
// Required: Unprepared(InvalidDefault) at x-settings.options.
```

Strict validation also succeeds, borrowing the original value and reporting
both defaults. A plain validator on the exported schema confirms that
`{ "x-settings": { "options": {} } }` is invalid and the version containing
`enabled: true` is valid. The preparation step is repairing the invalid
default, rather than merely completing an authored object. An authored
`options: {}` may still receive its child's default, as the spec allows.

A scalar counterpart uses this raw schema and input `{}`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": { "timeout": { "default": "3" } },
  "anyOf": [
    { "properties": { "timeout": { "type": "number" } } },
    { "properties": { "timeout": { "type": "boolean" } } }
  ]
}
```

Strict returns a data mismatch; soft **succeeds with `timeout: 3`** and a
default report containing the original text `"3"`. The default satisfies
neither alternative strictly, so both modes should instead report schema
`InvalidDefault` at `timeout`. Replacing `anyOf` with `oneOf` gives the same
soft success and an ordinary union failure in strict mode.

The bounded sweep reused the existing parent/required-child fixture at every
structural route supported by the compiler. In raw cases, `Parent` supplies
`options: { type: object, default: {} }`; `Nested` is an `anyOf` or `oneOf`
between an object requiring `options.enabled` (Boolean, default true) and an
object requiring `options.mode` (string, no default). Both alternatives
therefore reject the uncompleted default. Every affected raw route was run
with **both** union keywords and **both** validation modes. The named-type
case uses the SimplifiedSchema text above. The same results reproduce in the
minimal library build.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R4-F1.1 | SimplifiedSchema overlapping prefix/suffix selectors; resolved named union | Existing `x-settings: {}`, defaulted `options` | Both modes accept after inserting parent and required child | `Unprepared(InvalidDefault)` at `x-settings.options` |
| R4-F1.2 | Raw overlapping `patternProperties` | Parent under `^x-`, independent union under `-settings$` | Both accept repaired default | Same refusal |
| R4-F1.3 | Raw literal plus matching pattern | Literal parent, union from the matching pattern | Both accept repaired default | Same refusal |
| R4-F1.4 | Literal properties across `allOf` | Parent and union in different members | Both accept repaired default | Same refusal |
| R4-F1.5 | Pattern plus another member's catch-all | Parent from pattern, union from catch-all | Both accept repaired default | Same refusal |
| R4-F1.6 | Separate catch-alls across `allOf` | Parent and union describe the same key | Both accept repaired default | Same refusal |
| R4-F1.7 | Local `$ref` targets across `allOf` | References to parent and union; input `{}` | Both accept with `options.enabled = true` | `InvalidDefault` at `options` |
| R4-F1.8 | Joined array `items` | Existing `[{}]`; parent and union item schemas | Both accept repaired default | `InvalidDefault` at `0.options` |
| R4-F1.9 | Joined tuple `prefixItems` | Existing first object; parent and union positional schemas | Both accept repaired default | Same refusal; retain the no-inserted-position rule |
| R4-F1.10 | Common root `properties` beside a union | Common `options` default; required-child alternatives on the root | Both accept repaired default | `InvalidDefault` at `options` |
| R4-F1.11 | Two independent unions joined by `allOf` | Default on one union's arm, required-child alternatives in the other union | Both accept repaired default | Same refusal; preserve each union's grouping |
| R4-F1.12 | Common scalar default beside a union | Text `"3"` default versus number/Boolean alternatives | Strict data refusal; soft converts and accepts `3` | Schema `InvalidDefault` at `timeout` in both modes |
| R4-F1.13 | Public `DefaultPlan::compile` → `prepare_defaults` | Raw common-root object fixture, no host or final validator | Returns completed `options.enabled = true` and two reports | Refuse the parent default before descending |
| R4-F1.14 | Public `Prepared::validate_strict` | Same raw common-root fixture | Accepts borrowed `{}`, two default reports | `Unprepared(InvalidDefault)` |
| R4-F1.15 | Public `Prepared::validate_soft` | Same raw common-root fixture | Accepts completed copy, two reports | Same refusal |
| R4-F1.16 | Complete union inside the property's own definition | `properties.options` contains both its default and the union | Both modes refuse `InvalidDefault` | Clean control; retain |
| R4-F1.17 | Direct conjunctions without an independent union | All original R3-F1 selector/reference/positional/scalar fixtures | Both modes refuse `InvalidDefault` | Clean controls; retain |
| R4-F1.18 | Genuine alternatives on the default's own union | Number-default/string example and nested loose/strict example | Existing `required_defaults::unions` public-result tests pass | Clean; do not check a default separately against every alternative |
| R4-F1.19 | Null-only optional wrappers | Original nullable-selector fixture | Both modes refuse the invalid default | Clean; retain the wrapper repair |
| R4-F1.20 | Valid default and authored values | Optional-child control; authored empty object; supplied `enabled: false` | Parent/optional child complete; authored object gets only child default; false is unchanged | Clean; preserve completion and existing repeat-stability controls |

Followed the result to its consumers: SimplifiedSchema uses `Prepared`, whose
two public verdicts are reproduced above. Darkmatter's
[`EffectiveSchema::complete`](../../../darkmatter/lib/src/markdown/schemas/mod.rs:1844)
delegates to the same `prepare_defaults`; its validation and composition
reports do not add an independent default-validity check. Claudine uses that
effective-schema path. Repair the library once; no host defaulting walk or
host-specific union rule is needed.

**Recommended repair:** retain the logical grouping of independently imposed
union constraints when compiling the selected property's default check.
Judge each such group as a union before completing children or converting
the value. Do not replace the current filter with “check every alternative”:
that breaks the spec's accepted one-arm default policy. Keep the work bounded
to the already supported structural routes in the table, reuse structural
validators and their cache, and retain the original conflict, recursive,
null-wrapper, stability and performance controls. No evaluation of raw
conditional keywords, new schema syntax, or arbitrary input protection layer
is requested. Add public-result tests for these sibling routes and review the
associated code comments and guidance against the repaired contract.

### R4-F2 (high): Claudine L1 tests exceed the timing budget

**Authority: spec and required verification.** The spec's “Verification”
section requires `just test` in all three areas and passing package checks.
The review's “A Slow Test Is a Failing Test” rule explicitly makes an L1 test
at ten seconds or longer a failure even on a loaded host. The canonical
Claudine area run has 20 such tests. All are existing, ordinary fixture
workflows; no newly constructed input is needed.

**Defect class:** existing Level 1 workflows exceed their execution budget
when run together in the package, despite passing their functional assertions.

The canonical `claudine/` `just test` invocation ran 12,170 tests and reported
152 at or above five seconds. Twenty took 10.140–15.426 seconds. The host has
16 physical/logical cores (`sniff cpu --json`); recorded `uptime` load averages
rose from 70.31 / 35.84 / 30.30 to 184.28 / 122.96 / 74.59 during the run.
This establishes heavy load, but does not waive the explicit ten-second cap.
The package gate is red even though nextest exits successfully.

The sweep includes every slow test in that run, not only the first sequence
case. Every one was selected for a separate-process remeasurement with
nextest `-j 1`, retaining `work-counters` and CLI `test-fixtures`, plus the
generator package where applicable. The table records package and isolated
timings, and distinguishes required repairs from cleared load-sensitive
controls. A successful isolated measurement cannot clear a package timing
of ten seconds or longer.

Representative sources show that these are ordinary CLI paths:
[`outputs_chain_through_root_sequence_tasks`](../../../claudine/cli/tests/l1/sequence_groups.rs:314)
in claudine-cli exercises sequential task outputs;
[`sequence_dry_run_fail_fast_on_composition_error`](../../../claudine/cli/tests/l1/wrap_sequence_composition.rs:303)
uses one dry-run command and verifies that a missing schema property prevents
provider launch; and
[`playa_dry_run_prepares_nothing_and_leaves_no_audio_artifacts`](../../../claudine/cli/tests/l1/tts_prewarm.rs:169)
uses a frozen prompt, fake provider and private directories to check that dry
run creates no audio artifacts. Their package durations are 13.514, 14.207
and 15.426 seconds respectively. This review does not claim that the new
default compiler caused all the measured contention.

All 152 slow rows are accounted for: **20 require repair; 132 are cleared controls**. All functional assertions passed. Isolated times range from 0.329 to 4.289 seconds.

| Child ID | Site | Shape tested | Observed seconds: package / alone | Expected result / disposition |
| --- | --- | --- | --- | --- |
| R4-F2.1 | `claudine::l1 claudine_schemas::nested_group_cancelled_rejects_forbidden_actions_in_the_deepest_conditionals` | Shipped lifecycle schema fixture | 5.210 / 2.632 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.2 | `claudine::l1 claudine_schemas::nested_task_cancelled_rejects_forbidden_actions_in_the_deepest_conditionals` | Shipped lifecycle schema fixture | 5.169 / 2.066 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.3 | `claudine::l1 claudine_schemas::shared_payload_properties_have_identical_types` | Shipped lifecycle schema fixture | 6.463 / 2.749 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.4 | `claudine::l1 claudine_schemas::task_cancelled_rejects_forbidden_actions` | Shipped lifecycle schema fixture | 5.048 / 2.671 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.5 | `claudine::l1 claudine_schemas::task_cancelled_rejects_forbidden_actions_in_the_deepest_conditionals` | Shipped lifecycle schema fixture | 5.538 / 3.401 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.6 | `claudine::l1 claudine_schemas_embedded::disk_bundle_resolves_its_relative_imports_from_disk` | Embedded/disk schema fixture | 5.178 / 4.289 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.7 | `claudine::l1 claudine_schemas_embedded::embedded_json_trigger_combinator::control` | Embedded/disk schema fixture | 9.300 / 2.225 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.8 | `claudine::l1 claudine_schemas_embedded::embedded_json_trigger_combinator::duplicate` | Embedded/disk schema fixture | 9.079 / 1.763 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.9 | `claudine::l1 claudine_schemas_embedded::embedded_json_trigger_combinator::trailing` | Embedded/disk schema fixture | 9.015 / 1.743 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.10 | `claudine::l1 claudine_schemas_embedded::embedded_json_trigger_condition_operand::control` | Embedded/disk schema fixture | 5.094 / 0.928 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.11 | `claudine::l1 claudine_schemas_embedded::embedded_json_trigger_kind::control` | Embedded/disk schema fixture | 5.463 / 0.955 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.12 | `claudine::l1 claudine_schemas_embedded::embedded_payloads_apply_without_disk_schemas` | Embedded/disk schema fixture | 5.086 / 1.737 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.13 | `claudine::l1 claudine_schemas_embedded::embedded_yaml_action_initialize_bundle::wrong_type_one_element` | Embedded/disk schema fixture | 5.423 / 1.643 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.14 | `claudine::l1 claudine_schemas_embedded::embedded_yaml_library_types::wrong_type_one_element` | Embedded/disk schema fixture | 5.142 / 1.070 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.15 | `claudine::l1 claudine_schemas_embedded::embedded_yaml_trigger_combinator::control` | Embedded/disk schema fixture | 9.311 / 1.836 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.16 | `claudine::l1 claudine_schemas_embedded::embedded_yaml_trigger_combinator::duplicate` | Embedded/disk schema fixture | 8.498 / 1.781 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.17 | `claudine::l1 claudine_schemas_embedded::embedded_yaml_trigger_combinator::trailing` | Embedded/disk schema fixture | 8.321 / 1.780 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.18 | `claudine::l1 claudine_schemas_embedded::embedded_yaml_trigger_kind::control` | Embedded/disk schema fixture | 5.197 / 0.928 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.19 | `claudine::l1 claudine_schemas_embedded::embedded_yaml_trigger_payload::control` | Embedded/disk schema fixture | 5.841 / 0.944 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.20 | `claudine::l1 claudine_schemas_embedded::passive_parity_agent_empty_list` | Embedded/disk schema fixture | 5.234 / 0.968 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.21 | `claudine::lib composition::preflight::tests::a_saved_choice_answers_the_next_run_without_a_prompt_and_is_not_written_twice` | Saved schema-choice fixture | 7.787 / 1.455 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.22 | `claudine::lib composition::preflight::tests::each_choice_at_a_resolved_site_saves_its_row` | Saved schema-choice fixture | 5.275 / 0.865 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.23 | `claudine::lib composition::preflight::tests::each_choice_at_a_template_site_saves_its_row` | Saved schema-choice fixture | 5.382 / 1.010 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.24 | `claudine-cli::l1 compose_schema_cli::a_required_property_violation_stops_before_lifecycle_actions_under_existing_recovery_policy` | Hermetic CLI fixture | 5.027 / 1.165 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.25 | `claudine-cli::l1 compose_schema_cli::inline_compose_shell_expanded_value_violating_schema_aborts_without_launching_provider` | Hermetic CLI fixture | 5.731 / 1.139 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.26 | `claudine-cli::l1 contamination_probes::an_exported_launch_identity_does_not_change_the_result` | Hermetic CLI fixture | 5.259 / 0.861 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.27 | `claudine-cli::l1 contamination_probes::exported_claudine_application_variables_do_not_change_the_result` | Hermetic CLI fixture | 5.489 / 1.091 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.28 | `claudine-cli::l1 error_guards::production_sources_pass_every_scan_backed_guard` | Hermetic CLI fixture | 8.155 / 1.797 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.29 | `claudine-cli::l1 handoff_owners::sequence_task_proxy_after_a_retry_runs_target_within_the_step` | Hermetic CLI fixture | 5.265 / 0.948 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.30 | `claudine-cli::l1 handoff_owners::sequence_task_target_keeps_the_tasks_operation_and_environment` | Hermetic CLI fixture | 6.234 / 1.123 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.31 | `claudine-cli::l1 implement_plan_commit_message::a_clean_tree_with_a_passed_commit_message_runs_neither_commit_route` | Hermetic CLI fixture | 8.755 / 1.327 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.32 | `claudine-cli::l1 implement_plan_commit_message::a_clean_tree_with_the_empty_default_runs_neither_commit_route` | Hermetic CLI fixture | 8.020 / 1.307 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.33 | `claudine-cli::l1 implement_plan_commit_message::a_null_commit_message_takes_the_just_commit_branch` | Hermetic CLI fixture | 9.195 / 1.356 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.34 | `claudine-cli::l1 implement_plan_commit_message::a_passed_commit_message_reaches_git_as_one_argument` | Hermetic CLI fixture | 8.001 / 1.376 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.35 | `claudine-cli::l1 implement_plan_commit_message::an_absent_commit_message_passes_pre_flight_and_its_false_guard_launches_nothing` | Hermetic CLI fixture | 9.040 / 1.356 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.36 | `claudine-cli::l1 implement_plan_commit_message::an_absent_commit_message_without_a_template_rule_is_refused_at_pre_flight` | Hermetic CLI fixture | 5.575 / 0.746 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.37 | `claudine-cli::l1 implement_plan_commit_message::the_frozen_prompt_with_its_empty_default_takes_the_just_commit_branch` | Hermetic CLI fixture | 9.180 / 1.360 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.38 | `claudine-cli::l1 inline_completion_lifecycle::a_conditional_required_property_resolved_to_null_fails_the_inline_launch` | Hermetic CLI fixture | 6.936 / 1.116 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.39 | `claudine-cli::l1 inline_completion_lifecycle::a_direct_compose_run_fails_completion_when_a_start_effect_invalidates_a_property` | Hermetic CLI fixture | 5.411 / 0.744 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.40 | `claudine-cli::l1 inline_completion_lifecycle::a_duplicate_owned_key_is_refused_and_rolled_back` | Hermetic CLI fixture | 5.666 / 0.913 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.41 | `claudine-cli::l1 inline_completion_lifecycle::a_failed_inline_verdict_fires_failure_with_the_typed_err_then_finalize` | Hermetic CLI fixture | 5.834 / 0.955 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.42 | `claudine-cli::l1 inline_completion_lifecycle::a_failed_provider_attempt_hands_the_retry_a_restored_document` | Hermetic CLI fixture | 5.683 / 1.069 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.43 | `claudine-cli::l1 inline_completion_lifecycle::a_failed_rollback_reports_the_typed_cause_and_keeps_the_initiating_error` | Hermetic CLI fixture | 5.989 / 0.907 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.44 | `claudine-cli::l1 inline_completion_lifecycle::a_metadata_only_retry_recovers_a_completion_schema_failure` | Hermetic CLI fixture | 6.019 / 1.233 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.45 | `claudine-cli::l1 inline_completion_lifecycle::a_missing_completion_property_fails_the_run_and_keeps_the_written_artifact` | Hermetic CLI fixture | 5.178 / 0.924 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.46 | `claudine-cli::l1 sequence_errors_cli::a_group_with_no_tasks_is_rejected` | Hermetic CLI fixture | 5.691 / 0.923 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.47 | `claudine-cli::l1 sequence_errors_cli::a_unitless_shell_timeout_is_rejected` | Hermetic CLI fixture | 5.858 / 0.939 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.48 | `claudine-cli::l1 sequence_errors_cli::max_parallel_below_one_is_rejected` | Hermetic CLI fixture | 5.910 / 0.952 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.49 | `claudine-cli::l1 sequence_errors_cli::removed_parallel_execution_names_the_implicit_concurrency_replacement` | Hermetic CLI fixture | 5.856 / 0.940 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.50 | `claudine-cli::l1 sequence_errors_cli::removed_serial_execution_names_the_sequence_replacement` | Hermetic CLI fixture | 5.802 / 0.962 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.51 | `claudine-cli::l1 sequence_frozen_graph::root_retry_runs_the_discovered_children_of_a_changed_reference` | Hermetic CLI fixture | 5.506 / 0.837 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.52 | `claudine-cli::l1 sequence_grammar_matrix::yaml_frontmatter::root_loop_runs` | Hermetic CLI fixture | 5.138 / 0.581 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.53 | `claudine-cli::l1 sequence_groups::a_failed_parallel_member_lets_its_siblings_finish` | Hermetic CLI fixture | 5.037 / 1.467 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.54 | `claudine-cli::l1 sequence_groups::a_parallel_group_commits_one_named_result` | Hermetic CLI fixture | 11.168 / 1.008 | **Repair:** package must stay below 10 s even under load |
| R4-F2.55 | `claudine-cli::l1 sequence_groups::bounded_group_frames_share_one_left_edge` | Hermetic CLI fixture | 7.665 / 0.802 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.56 | `claudine-cli::l1 sequence_groups::group_variables_reach_members_and_do_not_leak_to_the_next_step` | Hermetic CLI fixture | 12.349 / 0.957 | **Repair:** package must stay below 10 s even under load |
| R4-F2.57 | `claudine-cli::l1 sequence_groups::outputs_chain_through_root_sequence_tasks` | Hermetic CLI fixture | 13.514 / 0.999 | **Repair:** package must stay below 10 s even under load |
| R4-F2.58 | `claudine-cli::l1 sequence_groups::parallel_group_members_keep_separate_provider_sessions` | Hermetic CLI fixture | 12.638 / 1.006 | **Repair:** package must stay below 10 s even under load |
| R4-F2.59 | `claudine-cli::l1 sequence_groups::parallel_members_do_not_depend_on_the_default_thread_stack` | Hermetic CLI fixture | 9.316 / 0.952 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.60 | `claudine-cli::l1 sequence_groups::parallel_prompt_task_splits_data_and_status_across_channels` | Hermetic CLI fixture | 8.933 / 0.763 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.61 | `claudine-cli::l1 sequence_initialize_include_preflight::a_prompt_whose_initialize_would_create_its_include_fails_sequence_preflight` | Hermetic CLI fixture | 6.074 / 0.369 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.62 | `claudine-cli::l1 sequence_initialize_include_preflight::existing_include::a_dry_run_whose_include_exists_still_succeeds_without_side_effects` | Hermetic CLI fixture | 10.406 / 0.569 | **Repair:** package must stay below 10 s even under load |
| R4-F2.63 | `claudine-cli::l1 sequence_initialize_include_preflight::existing_include::a_sequence_whose_include_exists_runs_unchanged` | Hermetic CLI fixture | 11.260 / 0.709 | **Repair:** package must stay below 10 s even under load |
| R4-F2.64 | `claudine-cli::l1 sequence_jit::a_body_shell_expansion_reads_the_filesystem_at_its_turn` | Hermetic CLI fixture | 11.022 / 0.749 | **Repair:** package must stay below 10 s even under load |
| R4-F2.65 | `claudine-cli::l1 sequence_jit::a_failing_shell_task_fails_its_step` | Hermetic CLI fixture | 6.995 / 0.418 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.66 | `claudine-cli::l1 sequence_jit::a_later_step_composes_against_the_edit_an_earlier_step_made` | Hermetic CLI fixture | 10.705 / 0.760 | **Repair:** package must stay below 10 s even under load |
| R4-F2.67 | `claudine-cli::l1 sequence_jit::a_later_step_composes_the_previous_steps_output` | Hermetic CLI fixture | 8.059 / 0.621 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.68 | `claudine-cli::l1 sequence_jit::a_mid_run_edit_to_the_sequence_list_does_not_change_the_plan` | Hermetic CLI fixture | 8.082 / 0.614 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.69 | `claudine-cli::l1 sequence_jit::a_set_from_an_earlier_step_is_composed_into_a_later_step` | Hermetic CLI fixture | 13.240 / 1.093 | **Repair:** package must stay below 10 s even under load |
| R4-F2.70 | `claudine-cli::l1 sequence_jit::a_shell_task_step_replaces_the_body_and_contributes_its_stdout` | Hermetic CLI fixture | 6.715 / 0.574 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.71 | `claudine-cli::l1 sequence_jit::a_step_failure_stops_the_run_when_fail_fast_is_true` | Hermetic CLI fixture | 6.127 / 0.499 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.72 | `claudine-cli::l1 sequence_jit::an_authored_state_mutation_does_not_override_the_sequence_binding` | Hermetic CLI fixture | 11.350 / 1.083 | **Repair:** package must stay below 10 s even under load |
| R4-F2.73 | `claudine-cli::l1 sequence_jit::an_interrupt_between_steps_exits_130_with_a_partial_summary` | Hermetic CLI fixture | 7.115 / 0.506 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.74 | `claudine-cli::l1 sequence_jit::dry_run_composes_every_step_against_the_initial_state` | Hermetic CLI fixture | 6.817 / 0.447 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.75 | `claudine-cli::l1 sequence_jit::dry_run_composes_every_step_and_launches_nothing` | Hermetic CLI fixture | 7.535 / 0.605 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.76 | `claudine-cli::l1 sequence_magic_reference::sequence_magic_reference_follows_launch_scope_and_relative_stays_source_anchored::explicit_relative_resolves_beside_the_source_document` | Hermetic CLI fixture | 6.354 / 0.584 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.77 | `claudine-cli::l1 sequence_magic_reference::sequence_magic_reference_follows_launch_scope_and_relative_stays_source_anchored::nested_magic_resolves_from_the_launch_tree` | Hermetic CLI fixture | 7.058 / 0.866 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.78 | `claudine-cli::l1 sequence_magic_reference::sequence_resolves_external_file_via_magic_reference` | Hermetic CLI fixture | 7.065 / 0.857 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.79 | `claudine-cli::l1 sequence_magic_reference::sequence_resolves_external_file_via_relative_path` | Hermetic CLI fixture | 5.931 / 0.680 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.80 | `claudine-cli::l1 sequence_overlay_pty::pty_sequence_authored_row_projection_satisfies_required_property` | Hermetic CLI fixture | 8.145 / 1.924 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.81 | `claudine-cli::l1 sequence_overlay_pty::pty_sequence_auto_selectable_skips_review_and_launches` | Hermetic CLI fixture | 6.540 / 0.780 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.82 | `claudine-cli::l1 sequence_overlay_pty::pty_sequence_invalid_agent_shows_preprompt_before_review` | Hermetic CLI fixture | 5.652 / 0.799 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.83 | `claudine-cli::l1 sequence_overlay_pty::pty_sequence_prompt_dedupes_and_launches_all_steps` | Hermetic CLI fixture | 7.772 / 1.414 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.84 | `claudine-cli::l1 sequence_overlay_pty::pty_sequence_status_report_honors_setter_supplied_required` | Hermetic CLI fixture | 6.901 / 1.916 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.85 | `claudine-cli::l1 sequence_task_providers::tasks_name_their_providers_in_a_looping_sequence_with_no_document_agent` | Hermetic CLI fixture | 5.222 / 1.230 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.86 | `claudine-cli::l1 tts_prewarm::a_blocked_preparation_does_not_delay_any_sequence_step` | Hermetic CLI fixture | 7.660 / 1.254 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.87 | `claudine-cli::l1 tts_prewarm::a_blocked_preparation_does_not_delay_the_provider_launch` | Hermetic CLI fixture | 5.675 / 0.762 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.88 | `claudine-cli::l1 tts_prewarm::a_mixed_sequence_prepares_prompts_with_their_playback_configuration` | Hermetic CLI fixture | 5.439 / 0.917 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.89 | `claudine-cli::l1 tts_prewarm::a_prompt_documents_env_and_ctx_phrases_are_prepared_resolved` | Hermetic CLI fixture | 6.722 / 1.000 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.90 | `claudine-cli::l1 tts_prewarm::a_retry_prepares_its_new_snapshot_without_repeating_a_candidate` | Hermetic CLI fixture | 5.565 / 0.867 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.91 | `claudine-cli::l1 tts_prewarm::a_three_prompt_sequence_submits_every_document_before_step_one` | Hermetic CLI fixture | 8.174 / 1.255 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.92 | `claudine-cli::l1 tts_prewarm::every_accepted_playa_dry_run_spelling_prepares_nothing::lowercase_true` | Hermetic CLI fixture | 5.124 / 0.761 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.93 | `claudine-cli::l1 tts_prewarm::every_accepted_playa_dry_run_spelling_prepares_nothing::one` | Hermetic CLI fixture | 5.326 / 0.755 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.94 | `claudine-cli::l1 tts_prewarm::every_accepted_playa_dry_run_spelling_prepares_nothing::uppercase_true` | Hermetic CLI fixture | 5.318 / 0.753 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.95 | `claudine-cli::l1 tts_prewarm::every_accepted_playa_dry_run_spelling_prepares_nothing::yes` | Hermetic CLI fixture | 5.236 / 0.753 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.96 | `claudine-cli::l1 tts_prewarm::initialize_changes_are_discovered_by_the_reread_without_repeating_a_candidate` | Hermetic CLI fixture | 5.819 / 0.752 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.97 | `claudine-cli::l1 tts_prewarm::playa_dry_run_prepares_nothing_and_leaves_no_audio_artifacts` | Hermetic CLI fixture | 15.426 / 1.710 | **Repair:** package must stay below 10 s even under load |
| R4-F2.98 | `claudine-cli::l1 with_file_provenance::a_proxy_from_a_sequence_task_resolves_from_the_router` | Hermetic CLI fixture | 5.457 / 0.682 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.99 | `claudine-cli::l1 with_file_provenance::a_retry_with_file_resolves_from_the_retried_document` | Hermetic CLI fixture | 6.808 / 0.896 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.100 | `claudine-cli::l1 wrap_basics::wrapper_dry_run_forwards_the_tail_after_the_user_separator_without_it` | Hermetic CLI fixture | 5.455 / 0.602 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.101 | `claudine-cli::l1 wrap_compose_validation::a_template_text_value_is_sent_verbatim_at_event_time` | Hermetic CLI fixture | 6.992 / 0.721 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.102 | `claudine-cli::l1 wrap_compose_validation::compose_dry_run_quiet_and_silent_are_no_op` | Hermetic CLI fixture | 7.614 / 0.623 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.103 | `claudine-cli::l1 wrap_compose_validation::compose_rejects_the_pre_fix_incident_before_any_provider_starts` | Hermetic CLI fixture | 7.520 / 0.752 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.104 | `claudine-cli::l1 wrap_compose_validation::predicates_operands_and_proxy_with_values_are_rejected_before_launch` | Hermetic CLI fixture | 11.318 / 1.148 | **Repair:** package must stay below 10 s even under load |
| R4-F2.105 | `claudine-cli::l1 wrap_compose_validation::reentry_refuses_a_defect_introduced_by_the_previous_attempt` | Hermetic CLI fixture | 6.746 / 0.828 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.106 | `claudine-cli::l1 wrap_compose_validation::sequence_document_with_a_nested_span_starts_no_step` | Hermetic CLI fixture | 5.204 / 0.491 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.107 | `claudine-cli::l1 wrap_compose_validation::sequence_referencing_the_incident_in_step_two_starts_no_step` | Hermetic CLI fixture | 9.315 / 0.944 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.108 | `claudine-cli::l1 wrap_compose_validation::synthesized_mixed_and_ordinary_frontmatter_values_still_launch` | Hermetic CLI fixture | 6.637 / 0.739 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.109 | `claudine-cli::l1 wrap_incomplete_subagents::a_later_success_for_the_same_task_id_clears_the_stop` | Hermetic CLI fixture | 6.464 / 0.759 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.110 | `claudine-cli::l1 wrap_incomplete_subagents::a_started_task_cut_off_by_a_step_timeout_is_reported_as_timed_out` | Hermetic CLI fixture | 5.805 / 2.474 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.111 | `claudine-cli::l1 wrap_incomplete_subagents::completed_subagents_still_fire_the_success_stack` | Hermetic CLI fixture | 6.073 / 0.739 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.112 | `claudine-cli::l1 wrap_inline_compose::inline_compose_delivers_the_native_document_path_and_file_aware_guardrails` | Hermetic CLI fixture | 8.342 / 0.960 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.113 | `claudine-cli::l1 wrap_inline_compose::inline_compose_dry_run_quiet_and_silent_are_no_op` | Hermetic CLI fixture | 6.905 / 0.709 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.114 | `claudine-cli::l1 wrap_inline_compose::inline_compose_dry_run_schema_error_to_stderr_with_clean_stdout` | Hermetic CLI fixture | 6.537 / 0.808 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.115 | `claudine-cli::l1 wrap_inline_compose::inline_compose_keeps_agent_frontmatter_and_refuses_a_malformed_document` | Hermetic CLI fixture | 7.137 / 1.037 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.116 | `claudine-cli::l1 wrap_inline_compose::inline_compose_keeps_agent_written_frontmatter_across_runs` | Hermetic CLI fixture | 7.370 / 0.955 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.117 | `claudine-cli::l1 wrap_inline_compose::inline_compose_resolves_env_agent_in_prompt_template` | Hermetic CLI fixture | 7.355 / 0.786 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.118 | `claudine-cli::l1 wrap_inline_compose::inline_compose_uses_a_caller_supplied_prompt_without_persisting_it` | Hermetic CLI fixture | 10.760 / 0.973 | **Repair:** package must stay below 10 s even under load |
| R4-F2.119 | `claudine-cli::l1 wrap_inline_compose::inline_compose_without_an_eager_prompt_fails_before_launch_naming_it` | Hermetic CLI fixture | 13.525 / 1.070 | **Repair:** package must stay below 10 s even under load |
| R4-F2.120 | `claudine-cli::l1 wrap_opencode::opencode_unknown_model_surfaces_its_cause_as_the_failure_error` | Hermetic CLI fixture | 5.378 / 0.477 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.121 | `claudine-cli::l1 wrap_opencode_models::inline_compose_claude_dry_run_does_not_call_opencode_models` | Hermetic CLI fixture | 5.061 / 0.375 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.122 | `claudine-cli::l1 wrap_opencode_models::sequence_claude_dry_run_does_not_call_opencode_models` | Hermetic CLI fixture | 5.087 / 0.396 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.123 | `claudine-cli::l1 wrap_opencode_models::sequence_opencode_dry_run_with_env_model_skips_opencode_models_call` | Hermetic CLI fixture | 7.225 / 0.522 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.124 | `claudine-cli::l1 wrap_perf::inline_compose_perf_stdout_matches_non_perf` | Hermetic CLI fixture | 8.904 / 0.964 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.125 | `claudine-cli::l1 wrap_sequence_composition::sequence_dry_run_concatenates_bodies_with_dividers` | Hermetic CLI fixture | 5.785 / 0.447 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.126 | `claudine-cli::l1 wrap_sequence_composition::sequence_dry_run_fail_fast_on_composition_error` | Hermetic CLI fixture | 14.207 / 0.968 | **Repair:** package must stay below 10 s even under load |
| R4-F2.127 | `claudine-cli::l1 wrap_sequence_composition::sequence_dry_run_no_agent_renders_state_per_step` | Hermetic CLI fixture | 5.306 / 0.422 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.128 | `claudine-cli::l1 wrap_sequence_composition::sequence_dry_run_quiet_and_silent_are_no_op` | Hermetic CLI fixture | 11.944 / 0.844 | **Repair:** package must stay below 10 s even under load |
| R4-F2.129 | `claudine-cli::l1 wrap_sequence_composition::sequence_dry_run_single_invalid_renders_state_per_step` | Hermetic CLI fixture | 10.388 / 0.559 | **Repair:** package must stay below 10 s even under load |
| R4-F2.130 | `claudine-cli::l1 wrap_sequence_composition::sequence_dry_run_single_not_installed_renders_state_per_step` | Hermetic CLI fixture | 10.402 / 0.553 | **Repair:** package must stay below 10 s even under load |
| R4-F2.131 | `claudine-cli::l1 wrap_sequence_composition::sequence_dry_run_zero_installed_list_renders_state_per_step` | Hermetic CLI fixture | 10.140 / 0.560 | **Repair:** package must stay below 10 s even under load |
| R4-F2.132 | `claudine-cli::l1 wrap_sequence_composition::sequence_live_auto_selectable_launches_provider` | Hermetic CLI fixture | 10.358 / 0.758 | **Repair:** package must stay below 10 s even under load |
| R4-F2.133 | `claudine-cli::l1 wrap_sequence_composition::sequence_live_no_agent_aborts_without_launching_provider` | Hermetic CLI fixture | 7.239 / 0.346 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.134 | `claudine-cli::l1 wrap_sequence_composition::sequence_live_shell_only_no_agent_runs_headless` | Hermetic CLI fixture | 7.478 / 0.418 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.135 | `claudine-cli::l1 wrap_sequence_composition::sequence_live_silent_does_not_suppress_agent_resolution_abort` | Hermetic CLI fixture | 9.548 / 0.481 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.136 | `claudine-cli::l1 wrap_sequence_composition::sequence_live_single_invalid_aborts_without_launching_provider` | Hermetic CLI fixture | 8.557 / 0.482 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.137 | `claudine-cli::l1 wrap_sequence_composition::sequence_live_single_not_installed_aborts_without_launching_provider` | Hermetic CLI fixture | 9.305 / 0.472 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.138 | `claudine-cli::l1 wrap_sequence_composition::sequence_live_zero_installed_list_aborts_without_launching_provider` | Hermetic CLI fixture | 9.349 / 0.480 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.139 | `claudine-cli::l1 wrap_sequence_composition::sequence_preflight_rejects_blacklisted_lifecycle_shell_before_launch` | Hermetic CLI fixture | 8.452 / 0.489 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.140 | `claudine-cli::l1 wrap_sequence_composition::sequence_yolo_approves_lifecycle_shell_during_preflight` | Hermetic CLI fixture | 8.784 / 0.702 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.141 | `claudine-gen::l1 drift::committed_catalog_matches_regenerated_inputs` | Generated-file CLI fixture | 5.207 / 0.329 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.142 | `claudine-gen::l1 drift::committed_data_matches_regenerated_inputs` | Generated-file CLI fixture | 8.879 / 0.816 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.143 | `claudine-gen::l1 drift::committed_steering_catalog_matches_regenerated_inputs` | Generated-file CLI fixture | 5.479 / 0.550 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.144 | `claudine-gen::l1 generate_ux::clean_area_generates_nothing_and_exits_zero` | Generated-file CLI fixture | 8.318 / 1.027 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.145 | `claudine-gen::l1 generate_ux::clean_check_report_summary_matches_phase_1_snapshot` | Generated-file CLI fixture | 6.305 / 0.675 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.146 | `claudine-gen::l1 generate_ux::dry_run_reports_and_writes_nothing` | Generated-file CLI fixture | 6.769 / 1.073 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.147 | `claudine-gen::l1 generate_ux::non_tty_drift_is_report_only_and_exits_nonzero` | Generated-file CLI fixture | 6.132 / 1.261 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.148 | `claudine-gen::l1 generate_ux::roster_alias_rename_drifts_the_darkmatter_name_table` | Generated-file CLI fixture | 5.040 / 2.634 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.149 | `claudine-gen::l1 generate_ux::roster_alias_rename_is_regenerated_until_check_converges` | Generated-file CLI fixture | 5.553 / 1.568 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.150 | `claudine-gen::l1 generate_ux::steering_activation_policy_grant_is_written_and_checks_clean` | Generated-file CLI fixture | 6.480 / 1.243 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.151 | `claudine-gen::l1 generate_ux::steering_activation_policy_refuses_an_expected_loss_grant` | Generated-file CLI fixture | 5.022 / 1.007 | Clean: loaded run <10 s; alone ≤5 s |
| R4-F2.152 | `claudine-gen::l1 generate_ux::yes_writes_all_drift_and_check_reconverges` | Generated-file CLI fixture | 5.905 / 1.255 | Clean: loaded run <10 s; alone ≤5 s |

**Recommended repair:** profile the shared composition/schema and subprocess
work reached by the listed workflows, remove repeated setup or compilation
within an operation, and verify the affected tests both alone and in the
canonical package run. Keep each workflow's existing behavioral assertions.
The isolated results show load sensitivity, not its precise cause; start with
the shared work rather than prescribing twenty separate test rewrites.
If a matrix test performs repeated launches, split it into bounded named rows
with a coverage guard only where that preserves the behavior being proved.
Do not hide the cost through a tier rename, retries, skips, dropped cases or
larger time limits. The repair scope is the failing rows in the table; the
cleared rows are controls, not requests to rewrite all 152 tests.

## Blocked Findings

None. The spec already requires strict checking before child completion and
already decides that union alternatives remain alternatives. R4-F1 has a
repair within those decisions; R4-F2 requires the stated test budget, not an
author decision about that budget. No item warrants a pipeline stop or a
separate post-cycle human review.

## Recurrence

R4-F1 is a **newly found sibling** of review-3.md's R3-F1, in the broader
default-selection class previously reported as R2-F1 and R1-F1. Every prior
required reproduction remains fixed. Review 3 swept direct conjunctions and
retained clean union controls, but its repair's new route filter did not
preserve the constraint imposed by an independent union. The sweep should
have combined each selector/reference/positional route with an independently
imposed union, including a common default outside the union and two unions
joined together. This review carries those routes together in one instance
list, plus the scalar and public preparation/verdict consumers.

Repeatedly extending the compiler's route machinery risks an open-ended
requirement. The boundary here remains explicit: the existing structural
selectors and union keywords, and the spec's named prohibition on repairing
an invalid parent default or converting a used default. Those acceptance
requirements make this high priority; this is not a request to support raw
conditional default semantics or speculative schema constructs.

R4-F2 is a newly observed sibling of the test-cost issue recorded as repair
row R3-F1.18. That row's shipped-schema corpus case remains fixed, as its
individual and full-area measurements show. The earlier runs did not show
these twenty Claudine cases exceeding ten seconds. This review sweeps every
slow result from its full area run, including the isolated controls; it does
not attribute an unmeasured code regression to the previous repair.

## Verification and input robustness

All requirements here concern data validation, passive loading, conversion,
composition or execution with fake services. Level 1 public APIs, hermetic
subprocesses and editor-protocol sessions are appropriate. The spec requires
no terminal rendering, mouse behavior or physical keyboard encoding, so there
is no missing Level 2 or Level 3 evidence.

| User-facing requirement | Verification level and evidence | Assessment |
| --- | --- | --- |
| Missing/null/required table; strict borrowing; default reports; supplied falsey values; repeat stability | L1 `required_presence`, `required_defaults`, `pattern_defaults`, default-engine tests | Existing cases pass; new complete-definition gap is R4-F1 |
| Existing/defaulted parents, named/imported/recursive types, unions, arrays and tuple objects | L1 parent/nesting/union suites, all repair-ledger regressions, public review probes | Original cases pass; independent union constraints are R4-F1 |
| Raw schemas, passive default checking, exported annotations, preparation/cache parity | L1 raw-input, plain-validator, envelope, artifact and identity tests; direct `Prepared`/`prepare_defaults` probes | Shared default-check omission reproduced; no separate host implementation |
| Removed constraint, extended `not-empty`, trigger presence, envelope null carriers and phases | L1 grammar, per-type, matcher, envelope and host phase tests | Pass |
| Declared but unwritten properties, parent/child requirements, source preservation and no failure publication | L1 absent-property contract, composition/frontmatter/setter tests and hermetic CLI tests | Pass |
| Lifecycle flags, edit operands, duplicate placements, bundles, `set`, authored provenance and schema/parser agreement | L1 shared parser tests, both hosts' agreement suites, overlay regressions and fake execution | Previous finding remains fixed |
| Claudine inheritance, keyed configuration, completed key counts, whole-YAML validation and DMLS | L1 public configuration, composition, loader and editor tests | Existing matrices checked; see command results below |

For file/configuration readers, the default invalid-shape outcome remains a
located error with no successful publication; the existing initialization
order is preserved. Exceptions are the spec's absent/null inheritance,
default replacement, allowed empty values and positional omission rules.
The default checker is shared across formats: its value-shape tests must not
be multiplied into separate CLI processes for every file extension.

For the validator column newly implicated by R4-F1, the same settings fixture
was tested with absent and explicit-null `options` (both wrongly accept the
repaired parent), an authored `{}` (correctly receives only the child default),
`{ enabled: false }` (correctly preserved), and a number, empty text and empty
list (correctly refused at `/x-settings/options`). Removing the child's
`required` marker is the positive default control. The scalar counterpart
above separately proves that the skipped type constraint reaches soft
conversion. Collection-element rows do not apply to these object/scalar
validators.

Existing loader tests cover YAML duplicate keys, invalid/trailing content,
second documents and null spellings; JSON file syntax and numeric admission;
Markdown frontmatter admission; and Claudine's legacy JSON5 syntax. Their
public-result matrices and representative CLI wiring are exercised by the
area suites. Reviewed the requested `serde(default)`, `Option`, `filter_map`,
`unwrap_or_default` and `.ok()` sites: nullable configuration overrides have
the intentional absent-equals-null contract; default compilation reads
schema metadata whose shape is judged by preparation/lowering; invalid
patterns fail validator compilation. No additional realistic silent fallback
was established.

Review runs on macOS:

| Command | Result and practical limit |
| --- | --- |
| `simplified-schema/`: `just test` | 2,307 feature-enabled and 1,948 minimal tests passed; none at or above 5 s |
| `darkmatter/`: `just test` | 16,730 passed, 10 recipe skips; none at or above 5 s |
| `claudine/`: `just test` | 12,170 functional assertions passed, 15 recipe skips; **timing gate fails**: 152 slow tests, including 20 at or above 10 s (R4-F2) |
| Exact slow-test filter, nextest `-j 1`, same package features | All 152 remeasured at 0.329–4.289 s; 132 clear the loaded-host allowance, while the 20 package results at or above 10 s remain failures |
| Focused Darkmatter public-contract tests | 158 passed functionally; two slow results assessed below, then full area gate above completed without slow tests |
| Public review probes, default and minimal library builds | Reproduced every affected R4-F1 structural route with `anyOf` and `oneOf`, object/scalar shape controls, exported plain-validator checks, and direct `Prepared`/`prepare_defaults` results; temporary sources removed |
| `just lint` in SimplifiedSchema, Darkmatter and Claudine | Exit 0 in each area, including the recipes' auxiliary checks |
| `just check-tier-coverage simplified-schema darkmatter claudine` | Zero stranded tests |
| Review metadata checks and `git diff --check` | Pass; previous implementation flags preserved, next link and iteration updated |

The initial focused Darkmatter run measured
`shipped_schema_corpus::loadable_shipped_schemas_resolve_and_lower` at
5.408 s and `base_schema_end_to_end::base_style_schema_agrees_with_the_runtime_style_parser`
at 7.864 s. Follow-up host readings were 16.76 / 21.16 / 25.79 on 16 cores;
the individual tests took 3.601 s and 2.669 s. In the full canonical area
run they took 3.161 s and 4.277 s respectively, with no test at or above 5 s.
The corpus performance repair therefore remains verified; Claudine's
separate ten-second failures are not cleared by these results or by the
Claudine isolated rerun.

Tests added by the implementation are declared in the consolidated L1
targets, including `mod pattern_defaults` in SimplifiedSchema's test root;
their names stay in L1, and their work-counter/terminal feature requirements
are enabled by the recipes and CI metadata. No permanent test or production
source was changed by this review. Cross-OS proof remains with CI and does
not contribute a review finding.

## Implementation record corrections

Corrected `implementation.md`'s stale statement that review 3's original
conjunctive required-child cases remained unrepaired. Its dated note names
the checked regressions and distinguishes the new independent-union sibling.
Also corrected the repair log's blanket claim that its route filter checks
every definition applying together: the filter deliberately omits an
independent union, and the public reproductions above show that final
validation does not compensate for the omission. Both corrections state what
was wrong, what the code does and how it was verified. Neither is a separate
finding or human-review item. The repair ledger's parent count was also
corrected from 17 to 18 to include its already-listed performance child.
The spec body was not changed.
