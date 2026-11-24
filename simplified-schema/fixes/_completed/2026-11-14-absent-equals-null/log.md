---
fix: 2026-11-14-absent-equals-null
implementation_1: "2026-11-15T04:54:26-08:00"
implementation_2: "2026-11-15T07:04:30-08:00"
implementation_3: "2026-11-15T07:42:16-08:00"
implementation_4: "2026-11-15T09:16:40-08:00"
implementation_5: "2026-11-15T10:05:00-08:00"
---

# Absent equals null: review/fix log

## Implementation of Review Findings #1

> **started at:** 2026-11-15T04:54:26-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sequences/simplified-schema/fixes/2026-11-14-absent-equals-null/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs began
- the review lists no `human_in_the_loop` items and no blockers; both findings are high and cite spec authority

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Pattern-keyed objects skip default preparation | fixed | All 13 child rows verified below; scoped suites pass (simplified-schema 2251 + 1892 minimal; darkmatter filtered 2815; claudine filtered 2609 + 414) |
| R1-F1.1 | Literal `properties` child: default applied/reported | clean | `pattern_defaults::selectors::a_literal_property_child_is_completed` passes before and after the fix |
| R1-F1.2 | `<string>` / `additionalProperties` without sibling pattern: default applied | clean | `selectors::a_catch_all_child_is_completed` passes before and after |
| R1-F1.3 | `<starting::x->` prefix pattern: fill/report `true` for absent/null child | fixed | `selectors::a_prefix_pattern_child_is_completed` (failed before fix): strict borrowed + report, soft copy, null replaced, false kept, stable |
| R1-F1.4 | `<ending::-settings>` suffix pattern: fill/report `true` | fixed | `selectors::a_suffix_pattern_child_is_completed` (failed before fix) |
| R1-F1.5 | `<pattern::^x->` regex pattern: fill/report `true` | fixed | `selectors::a_regular_expression_pattern_child_is_completed` (failed before fix) |
| R1-F1.6 | Raw `patternProperties`: prepare default before judging (absent and null) | fixed | `raw_schemas::a_pattern_properties_child_is_prepared_before_it_is_judged` (failed before fix) |
| R1-F1.7 | Raw `additionalProperties` beside a pattern: complete the nonmatching object's child | fixed | `raw_schemas::additional_properties_beside_a_pattern_completes_only_unmatched_keys`: `other: {}` → `{n: 2}`; matching key untouched by catch-all |
| R1-F1.8 | Conflict detection beneath a pattern (raw and SimplifiedSchema export): compile-time error | fixed | `conflicts::a_union_conflict_beneath_a_pattern_is_refused_when_prepared` and `conflicts::a_raw_union_conflict_beneath_pattern_properties_fails_prepared_new` |
| R1-F1.9 | Conflict detection beneath literal/catch-all child: `ConflictingDefaults` | clean | `conflicts::a_union_conflict_beneath_a_literal_or_catch_all_is_refused_when_prepared` passes before and after |
| R1-F1.10 | Darkmatter `md-conditional-*` → `then`: report all reachable defaults | fixed | darkmatter `composition_defaults::a_lifecycle_action_inside_a_conditional_branch_reports_its_defaults`: report now includes `composed.0.then.0.no_error`, `.edit.create_new`, `.edit.no_error` |
| R1-F1.11 | Darkmatter `md-conditional-*` → `else`: report all reachable defaults | fixed | same test, `else` placement |
| R1-F1.12 | Array `items`, tuple `prefixItems`, recursive/local named types: completed, positional items not inserted | clean | `defaults::tests::tuple_positions_*`, `a_recursive_reference_*`, `required_defaults::parents::*` pass |
| R1-F1.13 | Claudine skill's recorded pattern-default limitation removed once repaired | fixed | `.claude/skills/claudine/SKILL.md` limitation sentence corrected; no other copy found in claudine/darkmatter/simplified-schema docs or skills |
| R1-F1.14 | Sibling: literal key also matched by a sibling SimplifiedSchema pattern is judged by the literal only | fixed | `selectors::a_literal_key_is_not_completed_by_a_sibling_pattern` |
| R1-F1.15 | Sibling: raw declared property that also matches a pattern takes both schemas' defaults (JSON Schema semantics) | fixed | `raw_schemas::a_declared_property_matching_a_pattern_takes_both_schemas_defaults` |
| R1-F1.16 | Sibling: two patterns matching one undeclared key with different defaults | fixed | `raw_schemas::overlapping_patterns_with_{equal_defaults_report_once,different_defaults_conflict_for_a_key_both_match}`; decision: raise `ConflictingDefaults` during preparation (only knowable per key) rather than first-wins, which the spec forbids ("never resolved by order") |
| R1-F1.17 | Sibling: value-dependent keywords (`if`/`then`/`else`, `dependentSchemas`, `not`, `unevaluated*`, `contains`) | clean | applicability depends on judging the value; SimplifiedSchema only lowers `if`/`then` for key-count bounds (no defaults); documented in `validation.md`. `allOf`/`anyOf`/`oneOf`/`$ref` inside patterns already followed (Darkmatter conditional reaches `md-loose-body` through `$ref`) |
| R1-F1.18 | Discovered: compile-time check over larger plan (181 → 422 locations) made `base_schema_end_to_end::base_style_schema_agrees_with_the_runtime_style_parser` 6.0 s | fixed | compiled-pattern cache and accepted-document cache (keyed by `biscuit_hash::xx_hash_json`, refusals never cached), both cleared by `reset_process_caches`; test now 2.67 s alone |
| R1-F1.19 | Discovered: `lifecycle_edit_schema_parity::a_completed_cell_parses_with_its_authored_source_as_the_authored_cell_does` side assertion relied on `else` never being completed | fixed | narrowed that assertion to `/initialize/0`; main equality check now covers the `else` placement with defaults filled |
| R1-F2 | Lifecycle validation accepts inputs the parser rejects | fixed | Every child row below verified; each agreement test asserts the schema verdict and the public parser on the same input; full gates green |
| R1-F2.1 | md `md-expression` side-effect operands: numeric `no_error` rejected by schema and parser | fixed | `side_effect_numeric_flag_is_refused_by_both` (located at `/composed/0/ensure_file/no_error`); `a_long_form_no_error_follows_the_flag_rule_whether_or_not_the_verb_is_typed` |
| R1-F2.2 | md `md-expression` function operands (`abs`): numeric `no_error` rejected by both | fixed | `expression_function_numeric_flag_is_refused_by_both` |
| R1-F2.3 | md `md-conditional-1/-2/-3`: authored `no_error: false` beside `then` rejected by both | fixed | `md_conditional_{1,2,3}_authored_false_flag_is_refused_by_both`, `conditional_flag_beside_else_is_refused_by_both`, `an_authored_flag_beside_then_fails_the_schema_and_the_parser` |
| R1-F2.4 | md `md-initialize-conditional-1/-2/-3`: same, rejected by both | fixed | `md_initialize_conditional_{1,2,3}_authored_false_flag_is_refused_by_both` |
| R1-F2.5 | Claudine `conditional`, `initialize-conditional`, `cancelled-conditional`: rejected by both | fixed | Claudine `an_authored_flag_beside_then_is_refused_in_every_family` (start/initialize/cancelled; false and true, nested; absent and null accepted) |
| R1-F2.6 | md short/long action categories and generated long form: duplicate authored flag placement rejected by both | fixed | `{edit,info,shell,side_effect,root_category,body_category}_duplicate_flags_are_refused_by_both`; `two_authored_flag_placements_fail_schema_validation_and_the_parser` |
| R1-F2.7 | Claudine category/long-form composition: duplicate authored flags rejected by both (all three event families) | fixed | Claudine `duplicate_flags_are_refused_in_every_family` (`info` as item, root, body); `ensure_file` duplicate row in `single_action_placement_rules_are_judged_alike_by_schema_and_parser` |
| R1-F2.8 | md `set` category beside `no_error: null`: rejected by both | fixed | `set_{null,false,root_null}_flag_is_refused_by_both`; parser refuses `set` beside `no_error: null` by key presence and the schema now matches; `set` inside a flagged bundle still accepted (implementation decision 11 kept) |
| R1-F2.9 | md `{ when: 'true' }` without branch/action: rejected by both | fixed | `bare_when_*`, `initialize_bare_when_*`, `nested_bare_when_*`; DMLS golden `a3_lifecycle_faults.json` now carries a located `dm.schema.constraint` |
| R1-F2.10 | Claudine incomplete conditional in `start`/`initialize`/`cancelled`: rejected by both | fixed | Claudine `a_bare_when_is_refused_in_every_family` |
| R1-F2.11 | Typed md `info`/`edit`/generated `ensure_file-long`, Claudine typed actions: numeric flag rejected (control) | clean | `every_no_error_placement_converts_text_and_refuses_a_number_in_both_checks`, `ensure_file-long` row, edit matrix rows, Claudine `a_typed_actions_numeric_flag_is_refused_by_both` |
| R1-F2.12 | Shared flag binder: omitted/null/false/`"yes"` accepted where applicable (control) | clean | `side_effect_{absent,null,false,boolean_like}_flag_is_accepted_by_both`, `null_sibling_beside_an_inner_flag_is_accepted_by_both`, edit `ACCEPTED` cells |
| R1-F2.13 | Conditional authored origin: absent/null accepted, true rejected (control) | clean | `an_authored_flag_beside_then_fails_the_schema_and_the_parser` (absent/null accepted, true/false refused) plus the Claudine family test |
| R1-F2.14 | Completed-default provenance `{ edit: { file: notes.md } }`: no duplicate reported (control) | clean | `a_completed_cell_parses_with_its_authored_source_*` passes; new `a_completed_document_is_judged_by_its_authored_flags` |
| R1-F2.15 | Flag-shape column on `ensure_file`: `1`, `"maybe"`, `""`, `[]`, `{}` rejected by schema as by parser | fixed | `ensure_file` rows: absent/null/false/`"yes"` accepted by both; `1`, `"maybe"`, `""`, `[]`, `{}` refused by both |
| R1-F2.16 | Gap-pinning tests turned into agreement regressions; docs describing these gaps as limitations updated | fixed | gap-pinning tests renamed to agreement tests (`an_authored_flag_beside_then_fails_the_schema_and_the_parser`, `two_authored_flag_placements_fail_schema_validation_and_the_parser`, `a_long_form_no_error_follows_the_flag_rule_whether_or_not_the_verb_is_typed`, `single_action_placement_rules_are_judged_alike_by_schema_and_parser`); docs updated: `darkmatter/docs/topics/lifecycle/index.md`, `darkmatter/docs/topics/schemas/definition.md`, `claudine/docs/topics/schemas.md`, `claudine/docs/topics/flow-control/lifecycle.md`, skills `claudine`, `darkmatter/schema.md`, `darkmatter/lifecycle.md`; no `docs/use-skills/` source changed |
| R1-F2.17 | Sibling: md actions below conditional level 3 (`md-loose-body`) escaped placement, branch-shape and flag checks | fixed | 20 `below_level_3!` agreement rows in `composed` and `initialize` (14 of 16 runnable rows failed before); `bound_operands::untyped_bodies` judges authored bodies with the same `placement_rule`, inheriting the enclosing conditional's provided verbs; shipped schema typing unchanged |
| R1-F2.18 | Sibling: composition's schema verdict read authored flags from `raw_source` although overrides/setters/inheritance had replaced values | fixed | `an_overridden_lifecycle_stack_is_not_judged_by_the_flags_it_replaced` and `an_overriding_lifecycle_stacks_own_placement_is_refused` (both failed before), `a_completed_lifecycle_stack_keeps_its_authored_verdict_through_composition`, `an_inherited_completed_lifecycle_stack_is_not_judged_again_as_authored`; Claudine `a_proxy_overlays_lifecycle_stack_is_judged_by_its_own_authored_flags` shows its overlay path was already aligned |
| R1-F2.19 | Sibling: other branch shapes (`when` beside root/body dictionary, `else` without `then`, `then` without `when`, sibling action beside `then`, flag-only item, loop loose-verb duplicates and `set` + flag) | fixed | `when_beside_a_{root,body}_dictionary_is_refused_by_both`, Claudine `a_loops_loose_verbs_follow_the_placement_rules`, and matrix rows |
| R1-F2.20 | Sibling: unprovided verbs (`say`) stay permissive | clean | `unprovided_verb_*_left_unread_by_both` |
| R1-F2.21 | Sibling: conditional depth limit (parser allows five levels, refuses the sixth) | clean | decision: neither md nor Claudine schema encodes it, matching Claudine's existing `ac5_depths_and_stack_shapes`/`container_schema_recursion`; pinned by `a_sixth_conditional_level_is_the_parsers_alone_to_refuse`. Alternative (schema refuses depth 6) rejected: a nesting rule, not a flag or branch-shape rule, and would change Claudine's established contract |
| R1-F2.22 | Sibling: below level 3, long-form field names, operand types and `shell` in `initialize` | clean | documented contract of md's deliberately tolerant baseline (operands of side effects are untyped at every level, a pre-existing design the review preserved); the spec's flag/placement parity now holds at every depth; `definition.md` "Placement rules at every depth" states the boundary |
| R1-F2.23 | Sibling: nested-path setter flag and deep-merged parent mapping in composition | clean | md's parser reads the document as written, so the composition verdict and the parser treat these flags identically; no schema/parser disagreement |

### Progress

- starting the work on 'R1-F1: Pattern-keyed objects skip default preparation' at 04:54:55
        - defect class: the shared default compiler omitted a structural child selector (key-pattern selection), so runtime completion and compile-time conflict detection both missed whole subtrees
        - `simplified-schema/lib/src/defaults.rs`: pattern locations compiled; literal properties merge matching raw patterns at compile time; a node's catch-all applies only to keys none of its patterns match; regexes compiled once with `fancy_regex` (the engine that handles the lowered literal-exclusion lookaheads)
        - new regressions in `simplified-schema/lib/tests/l1/pattern_defaults.rs` (registered in `tests/l1/main.rs`); with the fix reverted 11 of 14 tests and the Darkmatter conditional test failed, only the three controls passed
        - discovered perf regression in the compile-time check fixed with two caches (R1-F1.18)
        - no `PREPARATION_REVISION` bump or artifact regeneration needed: lowered JSON is unchanged; the default plan is runtime-only
        - docs: `simplified-schema/docs/topics/validation.md`, `.claude/skills/simplified-schema/defaults.md`, `.claude/skills/claudine/SKILL.md`
- work completed for 'R1-F1: Pattern-keyed objects skip default preparation' at 05:21:32
- starting the work on 'R1-F2: Lifecycle validation accepts inputs the parser rejects' at 05:21:32
        - defect class: passive schema admission did not run the authored flag-placement and branch-shape rules that execution parsing applies, and md's provided-but-operand-untyped verbs bypassed the flag check
        - first subagent pass: the parser's inline checks in `darkmatter/lib/src/lifecycle/parse.rs` became shared `PlacementRule` helpers (`item_keys_rule`, `conditional_keys_rule`, `guarded_keys_rule`, `dictionary_keys_rule`, `sibling_flag`, `empty_bundle_rule`, `set_flag_rule`, `duplicate_flag_rule`, `loop_concerns`) called by the parser at its original points and composed by `placement_rule()` for passive use
        - `darkmatter/lib/src/markdown/schemas/bound_operands.rs` records each action dictionary (a type declaring `no_error` beside `when`, or beside `while`/`until`) and `report_placements` adds a located problem when the schema reported nothing inside it
        - long-form inner-flag rules apply to typed operand sites and to every side effect and expression function; unprovided verbs stay unread, as the parser ignores them
        - authored origin: value-based entry points judge input as written; `DarkmatterSchemas::validate(&Markdown)` reads authored values from `Frontmatter::raw_source` (new `Frontmatter::authored_values`); new `EffectiveSchema::validate_completed(completed, authored, phase)`; Claudine `post_shell_validate` passes `authored_lifecycle_values`, `evaluate_completion` passes `Value::Null`
        - decisions: rejected a new SimplifiedSchema keyword (language change and artifact churn), extra JSON Schema alternatives (cannot see authorship; review advised against), and thread-local provenance (action at a distance)
        - siblings fixed: `when` beside a root/body dictionary, `else` without `then`, `then` without `when`, a sibling action beside `then`, flag-only items, loop loose-verb duplicates and `set` + `no_error`
        - `composition_defaults::a_lifecycle_action_inside_a_conditional_branch_reports_its_defaults` used an invalid `{when, else}`; it now writes `then: [stop]`
        - DMLS golden `a3_lifecycle_faults.json` regenerated with `DMLS_UPDATE_GOLDEN=1` (intended new located `dm.schema.constraint` for a bare `when`)
        - two drifted YAML comments corrected (`darkmatter/schemas/partials/md-lifecycle.yaml`, `claudine/schemas/lifecycle.yaml`); root `just shipped-schemas` ran once, regenerating Darkmatter and Claudine `shipped.json`; no generated lifecycle region or `PREPARATION_REVISION` changed
        - orchestrator review found an open sibling: md conditionals nested below level 3 use `md-loose-body` (“checked only by the lifecycle parser”), so the same disagreements remain there; a follow-up subagent was dispatched (R1-F2.17), along with a check of the proxy-overlay path alignment the first pass did not reproduce (R1-F2.18)
        - follow-up subagent pass (R1-F2.17, R1-F2.18):
                - `bound_operands::untyped_bodies` walks `then`/`else` bodies below level 3 the way the parser does and applies the shared `placement_rule`; a rejected alternative was a recursive md schema (changes every document's prepared baseline and completion)
                - composition now calls crate-internal `validate_with_authored` via `authored_view` and `FilledDefault` (carried across both passes); public `validate(&Markdown)` judges the frontmatter as given; two tests that completed in place now use `validate_completed`
                - root `just shipped-schemas` ran again (description strings in `md-lifecycle.yaml`); six recursive-schema goldens re-blessed with `DARKMATTER_BLESS_RECURSIVE_GOLDEN=1` (descriptions only); no `PREPARATION_REVISION` bump
                - not established as a disagreement: a Claudine `with:` value read from completed frontmatter would carry filled flags, but its parser and `validate_completed` would count them the same way
- work completed for 'R1-F2: Lifecycle validation accepts inputs the parser rejects' at 06:50:42

### Completion Report

- **outcome: complete**; elapsed 1 h 57 min (04:54:26 → 06:51 local)
- parent findings: total 2 = 2 fixed + 0 already fixed + 0 blocked + 0 unresolved + 0 disputed
- instance rows: R1-F1 has 19 children (14 fixed, 5 clean); R1-F2 has 23 children (17 fixed, 6 clean); none open
- verified repairs:
        - R1-F1: the default plan compiles `patternProperties` and the catch-all beside them, so key-pattern objects get runtime completion and compile-time `ConflictingDefaults`; the Darkmatter conditional `then`/`else` now reports its inner action defaults
        - R1-F2: the parser's placement and branch-shape rules are shared helpers used by both the parser and passive schema validation, at every conditional depth, judged on authored values; md's untyped side effects and expression functions get the flag check
- open finding/child IDs: none
- no human-review item was raised; every decision is recorded in its ledger row
- cross-OS: the changes are pure in-process logic (regex matching, JSON walks), not path or process code, so no `just cross-check` run was required; CI proves Linux/Windows/WSL2
- changed files:
        - simplified-schema: `lib/src/defaults.rs`, `lib/tests/l1/pattern_defaults.rs` (new), `lib/tests/l1/main.rs`, `docs/topics/validation.md` (R1-F1)
        - darkmatter: `lib/src/lifecycle/{parse.rs,mod.rs,signatures.rs}`, `lib/src/markdown/{frontmatter.rs,schemas/bound_operands.rs,schemas/mod.rs}`, `lib/src/markdown/compose/{schema_validation.rs,pipeline/mod.rs}`, `schemas/partials/md-lifecycle.yaml`, `lib/generated/shipped.json`, recursive-schema goldens, `dmls/tests/fixtures/response_contracts/a3_lifecycle_faults.json`, tests `lib/tests/l1/{lifecycle_schema_parity.rs,lifecycle_edit_schema_parity.rs,composition_defaults.rs}`, docs `docs/topics/lifecycle/index.md`, `docs/topics/schemas/definition.md` (R1-F1.10/11, R1-F2)
        - claudine: `lib/src/composition/{schema/mod.rs,completion.rs}`, `schemas/lifecycle.yaml`, `lib/generated/shipped.json`, tests `lib/tests/l1/{claudine_schemas.rs,claudine_schemas_embedded.rs}`, docs `docs/topics/schemas.md`, `docs/topics/flow-control/lifecycle.md` (R1-F2)
        - skills: `.claude/skills/simplified-schema/defaults.md`, `.claude/skills/claudine/SKILL.md`, `.claude/skills/darkmatter/{schema.md,lifecycle.md}`
- final verification (macOS, run by the orchestrator after all subagents finished, no subagent running):

| Command | Result | Verifies |
| --- | --- | --- |
| `simplified-schema/`: `just test` | 2,251 + 1,892 passed; none at or above 5 s | R1-F1 |
| `simplified-schema/`: `just lint` | exit 0 | R1-F1 |
| `darkmatter/`: `just test` | 16,730 passed, 10 recipe skips; none at or above 5 s | R1-F1.10/11, R1-F2 |
| `darkmatter/`: `just lint` | exit 0, including the Zed WASM check | R1-F2 |
| `claudine/`: `just test` | 12,170 passed, 15 recipe skips; none at or above 5 s (load average up to 36.9) | R1-F2 |
| `claudine/`: `just lint` | exit 0 | R1-F2 |
| `just check-tier-coverage` for all three areas | 0 stranded tests each | all new tests |
| `just slow-tests fresh` | exit 0 | slow-test gate |
| `git diff --check` | clean | all |

## Implementation of Review Findings #2

> **started at:** 2026-11-15T07:04:30-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sequences/simplified-schema/fixes/2026-11-14-absent-equals-null/review-2.md'
- this is iteration 2 of the review-to-implement cycle 
- the review's `implemented` flag was confirmed `false` before repairs began
- the review lists one high finding (R2-F1) citing spec authority, no `human_in_the_loop` items and no blockers; previous findings R1-F1 and R1-F2 are verified fixed by the reviewer
- R2-F1 is a recurrence (sibling missed by the R1-F1 sweep); its `## Recurrence` section is the minimum scope

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R2-F1 | Dictionary selectors still skip or split default preparation | fixed | one per-key selection routine (`Fill::select`/`Fill::fill`) applies the ordinary property rules to every schema selecting an existing key; all child rows below |
| R2-F1.1 | Literal `properties`: existing null numeric entry with default 3 (control) | clean | `pattern_defaults::entry_defaults::scalar_entry_takes_its_default::literal`, `pattern_defaults::entry_defaults::an_absent_entry::literal` |
| R2-F1.2 | `<string>` / catch-all: existing null entry → strict accepts/reports 3; soft writes 3 | fixed | `pattern_defaults::entry_defaults::scalar_entry_takes_its_default::catch_all` |
| R2-F1.3 | `<starting::x->`: same fill/report | fixed | `pattern_defaults::entry_defaults::scalar_entry_takes_its_default::prefix` |
| R2-F1.4 | `<ending::-settings>`: same fill/report | fixed | `pattern_defaults::entry_defaults::scalar_entry_takes_its_default::suffix` |
| R2-F1.5 | `<pattern::^x->`: same fill/report | fixed | `pattern_defaults::entry_defaults::scalar_entry_takes_its_default::regular_expression`; scalar shape matrix: `pattern_defaults::entry_defaults::other_scalar_shapes_are_kept::{literal,catch_all,prefix,suffix,regular_expression}`, `pattern_defaults::entry_defaults::an_absent_entry::{literal,catch_all,prefix,suffix,regular_expression}` |
| R2-F1.6 | Raw `patternProperties`: existing null entry (also listed in `required`) → prepare 3 before judging; written key meets presence | fixed | `pattern_defaults::entry_defaults::a_raw_pattern_properties_entry_takes_its_default`, `pattern_defaults::entry_defaults::a_raw_pattern_properties_entry_named_in_required_takes_its_default` |
| R2-F1.7 | Raw `additionalProperties`: existing null numeric entry → prepare/report 3 | fixed | `pattern_defaults::entry_defaults::a_raw_additional_properties_entry_takes_its_default` |
| R2-F1.8 | Selected object defaults, all four SimplifiedSchema dictionary selectors: null parent with `default({})` + defaulted child → fill both in one pass | fixed | `pattern_defaults::entry_defaults::object_entry_takes_its_default_and_child::{literal,catch_all,prefix,suffix,regular_expression}` (null parent filled with child in one pass; existing `{}` gets the child; absent entry stays absent in `an_absent_entry`) |
| R2-F1.9 | Used-default checking, all four dictionary selectors: `default('bad')` → `Unprepared` with `InvalidDefault` | fixed | `pattern_defaults::entry_defaults::used_invalid_default_is_refused::{literal,catch_all,prefix,suffix,regular_expression}` (`Unprepared`/`InvalidDefault` at `x-settings`, both modes) |
| R2-F1.10 | Existing object traversal through literal, pattern and catch-all (control; preserve repair) | clean | `pattern_defaults::selectors::*` (all five selectors), `pattern_defaults::raw_schemas::*`, `pattern_defaults::selectors::a_literal_key_is_not_completed_by_a_sibling_pattern` unchanged and passing |
| R2-F1.11 | Raw `allOf`: literal in one member, catch-all in another (with/without nonmatching `^y-` pattern) → complete `x-settings.enabled = true` | fixed | `pattern_defaults::combined_selectors::another_members_catch_all_completes_a_literally_named_key`, `pattern_defaults::combined_selectors::another_members_guarded_catch_all_completes_a_key_its_patterns_miss` |
| R2-F1.12 | Two overlapping pattern locations (SimplifiedSchema and raw): parent default in one, child default in other → one-pass completion and report | fixed | `pattern_defaults::combined_selectors::overlapping_simplified_patterns_complete_parent_and_child_in_one_pass`, `pattern_defaults::combined_selectors::overlapping_raw_patterns_complete_parent_and_child_in_one_pass` |
| R2-F1.13 | Raw pattern plus catch-all in separate `allOf` members: same one-pass result | fixed | `pattern_defaults::combined_selectors::a_pattern_and_another_members_catch_all_complete_in_one_pass` |
| R2-F1.14 | Required/default propagation between matching patterns: insert/report `n: null`; stable | fixed | `pattern_defaults::combined_selectors::required_and_a_null_default_from_matching_patterns_insert_null` |
| R2-F1.15 | Raw literal plus matching pattern (control) | clean | `pattern_defaults::entry_defaults::a_raw_literal_matching_a_pattern_takes_the_patterns_default`, `pattern_defaults::raw_schemas::a_declared_property_matching_a_pattern_takes_both_schemas_defaults` |
| R2-F1.16 | Merged literal location through `allOf` (control) | clean | `pattern_defaults::combined_selectors::a_literal_merged_across_all_of_members_completes_in_one_pass`, `pattern_defaults::combined_selectors::required_and_a_null_default_from_a_literal_merged_across_all_of_insert_null` |
| R2-F1.17 | Matching-pattern scalar-child conflicts: equal reported once, unequal refused (control; preserve) | clean | `pattern_defaults::raw_schemas::overlapping_patterns_with_{equal_defaults_report_once,different_defaults_conflict_for_a_key_both_match}`, `pattern_defaults::combined_selectors::overlapping_patterns_with_different_entry_defaults_conflict_for_a_null_entry` (per-key refusal, unused conflict not judged); conflict detection moved from `finish` into `Fill::fill` |
| R2-F1.18 | Arrays, tuple objects, recursive/local references (control) | clean | `required_defaults::nesting::{array_items_are_filled_in_ascending_index_order,a_recursive_reference_is_filled_at_every_depth_the_value_has}`, `required_defaults::parents::{list_items_have_their_null_children_filled_by_index,positional_items_are_never_inserted}`, unit `defaults::tests::{tuple_positions_are_never_inserted_but_their_objects_are_completed,a_default_that_recreates_its_own_parent_is_refused,a_recursive_reference_compiles_once_and_follows_the_value}` |
| R2-F1.19 | Pattern guidance (docs/skills) aligned with repaired boundary | fixed | `simplified-schema/docs/topics/validation.md` (pattern bullet rewritten with example, conflict paragraph), `.claude/skills/simplified-schema/defaults.md` (key-selection bullet), `.claude/skills/claudine/SKILL.md` (pattern-keyed map sentence), `defaults.rs` module and `prepare_defaults` docs; no Darkmatter doc described the old boundary |
| R2-F1.20 | Sibling found by orchestrator review: a raw `required` name that only a pattern or catch-all describes, missing from the value → its selecting schema's default is inserted and reported (a default meets `required` for a missing key); a non-required absent key stays absent | fixed | `pattern_defaults::entry_defaults::a_missing_required_name_takes_the_default_of_the_schemas_selecting_it` (pattern and catch-all), control `pattern_defaults::entry_defaults::a_missing_name_no_schema_requires_stays_absent_beside_a_pattern`; `Fill::visit` now names `required` entries alongside literal properties |

### Progress

- starting the work on 'R2-F1: Dictionary selectors still skip or split default preparation' at 07:04:55
        - defect class: default preparation did not treat every schema selecting an existing object key (literal `properties`, matching `patternProperties` / `<string>` / `<starting::…>` / `<ending::…>` / `<pattern::…>`, and each node's applicable `additionalProperties`, across `allOf` members and merged locations) as that key's one property definition
        - cause confirmed in `defaults.rs`: `Fill::visit` only visited an undeclared entry's value (never `fill`ed the entry itself), visited each selecting location independently against the original input, and skipped every globally declared key before considering another node's catch-all
        - design: one per-key routine for literal and dictionary keys alike; `visit` now takes the set of locations describing a value, `Fill::select` gathers every `Entry` (child location + one `Declared`) selecting the key across those locations (a location's literal `Property`, else its matching `KeyPattern`s and the `CatchAll`s whose own node neither names nor matches the key), and `Fill::fill` applies the ordinary rules to the merged selection (one default, `required` from any location, every declaring definition checked, children of all selected definitions completed in the same pass)
        - compile time: a literal name's definitions now also include another node's applicable catch-all (`Scope::catches` rule, per node), so its conflicts stay compile-time; patterns and catch-alls each carry their own `Entry` so a dictionary entry's own default exists; `Scope::{additional,guarded}` replaced by `catch_alls`
        - per-key conflict refusal moved from `finish` (two insertions at one path, now impossible by construction) into `Fill::fill` (different defaults among the key's selected definitions, only when the default is needed); `finish` no longer fails and its unit test was removed; recursion guard keyed on the `default` keyword pointers being expanded instead of `(location, index)`
        - absent dictionary entries are never prepared: only literal names and keys present in the value are visited
        - sweep: every `DefaultPlan`/`prepare_defaults` consumer (`Prepared`, Darkmatter `EffectiveSchema`, Claudine through Darkmatter) goes through this engine; Darkmatter's `fragments_below`, `bound_operands` walk and `schema_declares_property` select schema fragments for judging/projection, apply the per-node catch-all rule correctly and decide no default or presence, so they are outside the class; no new ledger rows
        - `PREPARATION_REVISION` not bumped: the default plan is runtime-only and `check_declarations` only refuses; no lowered or prepared artifact changed (shipped artifact tests pass)
        - tests: 38 new public-result tests in `lib/tests/l1/pattern_defaults.rs` (`entry_defaults` with a `per_selector!` macro giving one test per selector per case, `combined_selectors`); the R1 work is uncommitted so the pre-fix engine could not be restored from Git, and failure before the fix is established from the review's reproductions, whose inputs the tests use unchanged, and from the removed code paths above
        - docs/skills aligned (R2-F1.19)
        - verification: `simplified-schema/`: `just test defaults` (134 passed with features; minimal build ran its whole suite, 1929 passed), `just lint` exit 0; `cargo nextest run -p darkmatter --test l1 -E 'test(composition_defaults) | test(effective_default_preparation) | test(effective_schema_layering) | test(schema_language_ownership_guard) | test(shipped)'` 130 passed; `cargo nextest run -p claudine --test l1 -E 'test(claudine_schemas)'` 805 passed
        - orchestrator reconciliation (07:13–07:29):
                - inspected the combined `defaults.rs` diff; confirmed tests use the review's fixtures and expected values (`options_complete_in_one_pass`, `required_null_is_inserted`)
                - the subagent left one point open: a raw `required` name declared only through a matching pattern or catch-all, missing from the value, was reported `MissingRequired` instead of taking the selecting schema's default; it is a sibling of the same class (the key is named, not invented from a pattern) and the spec's "A default also meets `required` for a missing key" applies; fixed by naming `required` entries in `Fill::visit` (R2-F1.20), with docs updated in `validation.md` and the repo skill
                - decision recorded (unchanged behavior): a used default is checked against the definitions that declare it, as the literal path already did at HEAD. Correction, 2026-11-15 (review 3): the original claim that a non-declaring selector's requirement causes a refusal either way was false. If that selector requires a child with its own default, preparation fills it and both validation modes accept the invalid parent default. Verified by adding `required` to `Nested.options.enabled` in the existing overlapping-selector fixture and by raw `allOf`, pattern, catch-all, reference, array, and tuple public-API probes. R2-F1.16's optional-child control remains clean; the new required-child case is R3-F1.
                - caller skills (`docs/use-skills/`) checked: none describes pattern-default behavior, so no `just publish-skills` run was needed
- work completed for 'R2-F1: Dictionary selectors still skip or split default preparation' at 07:29:48

### Completion Report

- **outcome: complete**; elapsed 25 min (07:04:30 → 07:30 local)
- parent findings: total 1 = 1 fixed + 0 already fixed + 0 blocked + 0 unresolved + 0 disputed
- instance rows: R2-F1 has 20 children (13 fixed, 7 clean); none open
- verified repairs:
        - R2-F1: every schema selecting an existing (or literally/`required`-named) key is merged into that key's one property definition, so dictionary entries get their own defaults, used-default checks, required-null insertion and one-pass child completion; another node's catch-all describes literally named keys; per-key conflict refusal is preserved; no key is created from a pattern
- open finding/child IDs: none
- no human-review item was raised
- cross-OS: in-process JSON/regex logic only, no path or process code; no `just cross-check` run required
- changed files (this iteration):
        - `simplified-schema/lib/src/defaults.rs` (R2-F1)
        - `simplified-schema/lib/tests/l1/pattern_defaults.rs` (R2-F1.1–R2-F1.18, R2-F1.20)
        - `simplified-schema/docs/topics/validation.md`, `.claude/skills/simplified-schema/defaults.md`, `.claude/skills/claudine/SKILL.md` (R2-F1.19, R2-F1.20)
- final verification (macOS, run by the orchestrator after the subagent finished, nothing else running):

| Command | Result | Verifies |
| --- | --- | --- |
| `simplified-schema/`: `just test` | 2,290 + 1,931 (minimal) passed; none at or above 5 s | R2-F1 |
| `darkmatter/`: `just test` | 16,730 passed, 10 recipe skips; none at or above 5 s | host consumers of the default engine |
| `claudine/`: `just test` | 12,170 passed, 15 recipe skips; none at or above 5 s (load average 47.7) | host consumers |
| `just lint` in all three areas | exit 0 each | all |
| `just check-tier-coverage simplified-schema darkmatter claudine` | 0 stranded | new tests |
| `just slow-tests fresh` | exit 0 | slow-test gate |
| `git diff --check` | clean | all |

## Implementation of Review Findings #3

> **started at:** 2026-11-15T07:42:16-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sequences/simplified-schema/fixes/2026-11-14-absent-equals-null/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs began
- the review lists no `human_in_the_loop` items and no blockers; R3-F1 (high) cites the spec's "Validation and default preparation" sentences; R3-F2 (low) cites "One implementation and consistent guidance"

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R3-F1 | Used defaults omit constraints from other applicable definitions | fixed | All 18 child rows verified (14 fixed, 4 clean controls); focused suites and lint pass; orchestrator full-suite gates below. Count corrected 2026-11-15 (review 4): the earlier total of 17 omitted the later-added performance child R3-F1.18. |
| R3-F1.1 | SimplifiedSchema overlapping prefix/suffix selectors with required child added to `Nested`: `Unprepared(InvalidDefault)` at `x-settings.options` in both modes | fixed | `pattern_defaults::complete_default_checks::overlapping_simplified_patterns_refuse_a_default_missing_another_selectors_required_child`: failed first (both modes `Ok`), passes after |
| R3-F1.2 | Raw overlapping regex patterns: same refusal before child completion | fixed | `complete_default_checks::overlapping_raw_patterns_refuse_the_default` fail → pass |
| R3-F1.3 | Raw literal property plus matching pattern: same refusal | fixed | `complete_default_checks::a_literal_property_and_a_matching_pattern_refuse_the_default` fail → pass |
| R3-F1.4 | Raw literal properties in separate `allOf` members: same refusal | fixed | `complete_default_checks::a_literal_merged_across_all_of_members_refuses_the_default` fail → pass |
| R3-F1.5 | Pattern plus catch-all in separate `allOf` members: same refusal | fixed | `complete_default_checks::a_pattern_and_another_members_catch_all_refuse_the_default` fail → pass |
| R3-F1.6 | Catch-alls in separate `allOf` members: same refusal | fixed | `complete_default_checks::catch_alls_of_separate_all_of_members_refuse_the_default` fail → pass |
| R3-F1.7 | Local `$ref` targets joined by `allOf`, input `{}`: `InvalidDefault` at `options` | fixed | `complete_default_checks::reference_targets_joined_by_all_of_refuse_the_default` (at `options`) fail → pass |
| R3-F1.8 | Array `items` joined by `allOf`, input `[{}]`: `InvalidDefault` at `0.options` | fixed | `complete_default_checks::array_items_joined_by_all_of_refuse_the_default` (at `0.options`) fail → pass |
| R3-F1.9 | Tuple `prefixItems` joined by `allOf`: same refusal, no inserted position | fixed | `complete_default_checks::tuple_positions_joined_by_all_of_refuse_the_default_and_insert_no_position` fail → pass; `[]` stays empty |
| R3-F1.10 | Scalar default `"3"` split from `type: number` across literal `allOf` members: schema `InvalidDefault` at `timeout`, checked strictly | fixed | `complete_default_checks::a_scalar_default_is_checked_against_the_type_another_member_declares`: `InvalidDefault` at `timeout`, not `TypeMismatch`; fail → pass |
| R3-F1.11 | Clean control: complete definition in one schema refuses with `InvalidDefault` | clean | `complete_default_checks::a_complete_definition_in_one_schema_refuses_the_default` (SimplifiedSchema and raw) passes with old and new check |
| R3-F1.12 | Clean control: `allOf` inside the property's own definition refuses | clean | `complete_default_checks::all_of_inside_the_properties_own_definition_refuses_the_default` passes with old and new check |
| R3-F1.13 | Clean control: optional-child fixture completes in one pass and is stable | clean | `combined_selectors::overlapping_simplified_patterns_complete_parent_and_child_in_one_pass` still passes including stable second soft pass; all R2-F1 tests pass |
| R3-F1.14 | Clean control: union arms keep the number-default/string policy; conflicting arm defaults fail | clean | all `required_defaults::unions` tests pass (spec number-default/string example); added `a_nested_default_on_one_arm_is_not_checked_against_the_other_arm` and `a_default_is_checked_against_every_definition_along_its_arm` |
| R3-F1.15 | Field shapes through the edited fixture: absent/null `options` refused; `{}` gets child default; `{enabled: false}` unchanged; number/empty text/empty list refused at `/x-settings/options` | fixed | `complete_default_checks::options_shapes_beneath_the_required_child_fixture` fail → pass: absent/null refused `InvalidDefault`; authored `{}` gets only `enabled = true`; `{enabled:false}` kept; `5`, `""`, `[]` refused at `/x-settings/options` |
| R3-F1.17 | Discovered sibling: selector values lowered to nullable `anyOf[{type:null}, T]` must still be checked together (a null-only member is not an alternative) | fixed | `complete_default_checks::nullable_selector_values_still_refuse_the_default` fails if wrapper counted as an arm; passes |
| R3-F1.18 | Discovered during gates: `darkmatter::l1 shipped_schema_corpus::loadable_shipped_schemas_resolve_and_lower` slow alone (6.975 s) after the arm-aware compiler; must be ≤5 s | fixed | 7.073 s → 2.412/2.453/2.464 s alone (load ~7); compiler time over the test's 509 lowered schemas 5.1 s → 0.65 s (`check_declarations`) and 1.6 s (full arm-aware compile); compiled plans byte-identical before/after on all 509 (58 MB dump); new unit test `check_declarations_decides_reference_loops_as_compilation_does` |
| R3-F1.16 | Default-check comments and guidance updated with the repaired contract | fixed | default-check contract rewritten in `defaults.rs` module/fn docs, `docs/topics/validation.md`, `.claude/skills/simplified-schema/defaults.md`; no spec links |
| R3-F2 | Failure guidance omits runtime dictionary-default conflicts | fixed | All 4 required rows fixed plus 7 discovered sites (R3-F2.5–R3-F2.11); `just lint` passes; anchors checked |
| R3-F2.1 | `docs/topics/validation.md` (~line 400): qualify `Unprepared` causes to include runtime dictionary conflicts | fixed | `docs/topics/validation.md:417-424` now lists the per-key selector `ConflictingDefaults` among `Unprepared` causes and links `#applying-defaults` |
| R3-F2.2 | `docs/topics/diagnostics.md` (~line 416): qualify "raised before any value is considered" | fixed | `docs/topics/diagnostics.md:416` `ConflictingDefaults` row adds the per-key exception reported inside `ValidationFailure::Unprepared`, links `./validation.md#applying-defaults` |
| R3-F2.3 | `.claude/skills/simplified-schema/SKILL.md` (~line 62): add the dictionary-selection exception | fixed | `.claude/skills/simplified-schema/SKILL.md:62-65` adds the dictionary-selection exception, pointing to `defaults.md` |
| R3-F2.4 | Sibling sweep of other docs/skills/rustdoc claiming all conflicts are compile-time | fixed | sweep of simplified-schema docs/src/README, darkmatter and claudine docs and skills: accurate union-specific and non-blanket claims left (validation.md:59/74/352, unions.md:66-71/480, convert.rs, types.rs:192, darkmatter schemas/mod.rs:1602/1890, definition.md:1879); 7 inaccurate sites fixed as R3-F2.5–11; no `docs/use-skills/` change, so no `just publish-skills` |
| R3-F2.5 | `validation.md:306` bullet title "decided before any value" contradicted its own rule | fixed | retitled "One default per property." |
| R3-F2.6 | `docs/topics/global-type-constraints.md:62-65` blanket "schema error before any value is judged" | fixed | adds the pattern/catch-all per-key exception |
| R3-F2.7 | `.claude/skills/simplified-schema/defaults.md:96-98` Errors list `ConflictingDefaults` as compile-time only | fixed | adds the per-key `Fill::fill` case |
| R3-F2.8 | rustdoc `SchemaError::ConflictingDefaults` (`lib/src/errors.rs:948-954`) | fixed | comment-only: explains the per-key runtime detection |
| R3-F2.9 | rustdoc `ValidationFailure::Unprepared` (`lib/src/outcome.rs:64-68`) | fixed | comment-only: causes include selector conflicts |
| R3-F2.10 | rustdoc `validate_strict`/`validate_soft` Errors (`lib/src/prepared.rs:283-305`) | fixed | comment-only: adds `RecursiveDefault` and selector conflicts, links `prepare_defaults` |
| R3-F2.11 | rustdoc `DefaultPlan::compile` Errors (`lib/src/defaults.rs:271-275`) | fixed | comment-only: pattern/catch-all keys are checked later by `prepare_defaults` |

### Work Log

- starting the work on 'R3-F1: Used defaults omit constraints from other applicable definitions' at 07:42:51
        - defect class: a used default is checked against only the subset of applicable conjunctive definitions that repeat the `default` annotation, while child completion afterward uses every applicable definition
        - delegated to one foreground `rust-developer` subagent (serial; no other subagent running)
        - design decision (ledger only, no human-review item): each reached schema node records its union "arm" — the `(anyOf/oneOf pointer, member index)` choices on its route; `$ref`, `allOf`, selectors and array positions keep the arm, only a union member adds one; a used default is checked against every selected definition whose arm is contained in a declaring definition's arm, i.e. every definition that necessarily holds wherever the declaring one holds
                - a union with at most one non-null-admitting member adds no choice (SimplifiedSchema's optional wrappers), found as R3-F1.17
                - recursion stays finite: meeting the same union again replaces its earlier choice; location memo keyed by `(pointer, arm)`
                - alternatives rejected: checking every definition (breaks the spec's one-arm default example; tried and confirmed), excluding only conflicting choices (wrongly rejects on independent unions), exhaustive arm search (exponential; nullable wrappers make it vacuous)
                - trade-off: constraints of unions off the default's own route are not applied to the default; final validation still judges the completed value
        - sibling sweep: `check_declarations`, `prepared.rs`, Darkmatter's `EffectiveSchema::default_plan`/`default_problem`, and `resolve.rs`/`convert.rs` default lifting only call the shared engine or map its errors; none checks a default itself — clean
        - the "old check" fail evidence came from a temporarily restricted copy of `defaults.rs` (restored and `cmp`-verified); that throwaway run used `--no-fail-fast` once
        - subagent scoped checks: `simplified-schema/` `just test defaults` 152 + 1,947 minimal passed; `just lint` exit 0; `darkmatter/` `just test composition_defaults` 23 passed, `just test lifecycle` 630 passed, layering/ownership filters 12 passed; nothing at or above 5 s
- work completed for 'R3-F1: Used defaults omit constraints from other applicable definitions' at 07:54:25
- starting the work on 'R3-F2: Failure guidance omits runtime dictionary-default conflicts' at 07:54:25
        - defect class: guidance states that every default conflict is refused when a schema is prepared, omitting the per-key runtime conflict among dictionary selectors
        - delegated to one foreground `Documenter` subagent
        - linked explanation: the "One default per property" bullet under `## Applying defaults` in `validation.md` (anchor `#applying-defaults`; the bullet has no own anchor)
        - `cargo doc` with `-D rustdoc::broken_intra_doc_links` shows no warnings in edited files; it fails on links already broken in `triggers/matcher.rs` and `yaml_documents/` (outside this fix, not changed; `just lint` does not gate on them)
- work completed for 'R3-F2: Failure guidance omits runtime dictionary-default conflicts' at 07:57:17
- starting final verification gates at 07:57:17
        - `simplified-schema/` `just test`: 2,306 feature-enabled + 1,947 minimal passed; none at or above 5 s
        - `darkmatter/` `just test` (load average 35.8 on 16 cores): 16,730 passed, 10 skips, **2 slow** — red
                - `darkmatter-lsp::l1 claudine_schema_discovery::removed_sequence_keys_are_reported_at_their_paths`: 5.215 s in package, 2.632 s alone — cleared as load-sensitive (loaded host, under 10 s, ≤5 s alone)
                - `darkmatter::l1 shipped_schema_corpus::loadable_shipped_schemas_resolve_and_lower`: 8.105 s in package, 6.975 s alone — slow alone; history shows it at 5.6–12.9 s on earlier commits, but review 2/3 runs recorded it under 5 s, so the arm-aware default compiler is a suspect; delegated a foreground performance investigation (added as ledger row R3-F1.18)
        - performance subagent (foreground `rust-developer`) result for R3-F1.18:
                - root cause: arm-keyed scopes — one ~540 KB types-only import document compiled 9,535 scopes with 62,976 distinct arms (714 without arms); SipHash over string pointers and arm vectors dominated (`sample`); trimming arms to default-reaching unions saved only 4 scopes, dropped
                - repair in `simplified-schema/lib/src/defaults.rs`: `check_declarations` compiles without arms (arms matter only in `Fill::check`), falling back to the full arm-aware compile when the arm-free pass refuses or `Compiler::loops()` finds a reference loop, so verdicts and errors are always the arm-aware ones; the arm-aware compile interns pointers/arms as `u32` ids, caches per-pointer links and single expansions, and uses a small id hasher
                - test itself: no redundant work (second resolve already hits `CHECKED`); no coverage removed
                - scoped checks: `simplified-schema/` `just test defaults|pattern_defaults|required_defaults` pass (153/68/66 + 1,948 minimal), `just lint` exit 0; `darkmatter/` `just test shipped_schema_corpus` 13, `composition_defaults` 23, `recursive_schema_golden` 17, `lifecycle` 630 passed; nothing slow
                - `.claude/skills/simplified-schema/defaults.md` notes the arm-free declaration check
- rerunning all final gates after the performance repair
        - rerun `simplified-schema/` `just test`: 2,307 + 1,948 minimal passed; none at or above 5 s
        - rerun `darkmatter/` `just test` (load average 56 on 16 cores): 16,730 passed, 10 skips; `shipped_schema_corpus::loadable_shipped_schemas_resolve_and_lower` no longer slow; 2 other tests over 5 s, both under the 10 s loaded allowance and cleared alone
                - `markdown::compose::shell_expansion::probe::tests::installed_shells::injection_shaped_names_never_execute`: 5.428 s in package, 1.266 s alone
                - `base_schema_end_to_end::base_style_schema_agrees_with_the_runtime_style_parser`: 5.188 s in package, 2.685 s alone
        - `claudine/` `just test` (load average 51–59): 12,170 passed, 15 skips; 15 tests at 5.1–8.1 s, all under 10 s; re-measured serially (`-j 1`) at 0.63–1.45 s each — cleared as load-sensitive
        - `just lint` exit 0 in all three areas; `just check-tier-coverage simplified-schema darkmatter claudine` 0 stranded; `git diff --check` clean; `just slow-tests fresh` exit 0
- final verification completed at 08:54:40

### Completion Report

- **outcome: complete**; elapsed 1 h 13 min (07:42:16 → 08:55 local)
- parent findings: total 2 = 2 fixed + 0 already fixed + 0 blocked + 0 unresolved + 0 disputed
- instance rows:
        - R3-F1: 18 children — 14 fixed (R3-F1.1–R3-F1.10, R3-F1.15–R3-F1.18), 4 clean controls (R3-F1.11–R3-F1.14); none open
        - R3-F2: 11 children — 11 fixed (R3-F2.1–R3-F2.11, R3-F2.4 being the sweep); none open
- verified repairs:
        - R3-F1: a used default is checked strictly against selected definitions on its declaring union route (other selectors, `allOf` members, `$ref` targets, array/tuple positions), before children are completed; definitions reached through another union are skipped, and a null-only member is not an alternative. Correction, 2026-11-15 (review 4): the original claim that this includes every definition applying together was too broad. An independent union is another constraint, but its members are omitted from this check; public `SimplifiedSchema`, `Prepared`, and `prepare_defaults` probes accept a parent default `{}` after filling the child that union requires. All original ledger cases pass; this newly found sibling is R4-F1. The slow-compile regression R3-F1.18 is repaired.
        - R3-F2: docs, skills and rustdoc now say that differing defaults from patterns or catch-alls selecting one unnamed key are refused per key, at validation, as `Unprepared(ConflictingDefaults)`, only when the default is needed; every other conflict is refused at preparation
- open finding/child IDs: none
- no human-review item was raised; the union-arm design choice is recorded in the R3-F1 work log only
- cross-OS: in-process JSON/hash logic only, no path, process, or `cfg` code; no `just cross-check` run required
- pre-existing, unrelated: `cargo doc -D rustdoc::broken_intra_doc_links` fails on links in `triggers/matcher.rs` and `yaml_documents/`; not gated by `just lint`, not changed here
- changed files (this iteration):
        - `simplified-schema/lib/src/defaults.rs` (R3-F1, R3-F1.18, R3-F2.11)
        - `simplified-schema/lib/tests/l1/pattern_defaults.rs` (R3-F1.1–R3-F1.12, R3-F1.15, R3-F1.17)
        - `simplified-schema/lib/tests/l1/required_defaults.rs` (R3-F1.14)
        - `simplified-schema/lib/src/errors.rs`, `outcome.rs`, `prepared.rs` (R3-F2.8–R3-F2.10, comment-only)
        - `simplified-schema/docs/topics/validation.md` (R3-F1.16, R3-F2.1, R3-F2.5)
        - `simplified-schema/docs/topics/diagnostics.md` (R3-F2.2), `simplified-schema/docs/topics/global-type-constraints.md` (R3-F2.6)
        - `.claude/skills/simplified-schema/SKILL.md` (R3-F2.3), `.claude/skills/simplified-schema/defaults.md` (R3-F1.16, R3-F1.18, R3-F2.7)
- final verification (macOS, run by the orchestrator after every subagent finished):

| Command | Result | Verifies |
| --- | --- | --- |
| `simplified-schema/`: `just test` | 2,307 + 1,948 (minimal) passed; none at or above 5 s | R3-F1, R3-F2 rustdoc |
| `darkmatter/`: `just test` | 16,730 passed, 10 recipe skips; 2 load-sensitive tests over 5 s cleared alone (1.3 s, 2.7 s) | host consumers, R3-F1.18 |
| `claudine/`: `just test` | 12,170 passed, 15 recipe skips; 15 load-sensitive tests over 5 s (max 8.1 s) cleared alone (≤1.5 s) | host consumers |
| `just lint` in all three areas | exit 0 each | all |
| `just check-tier-coverage simplified-schema darkmatter claudine` | 0 stranded | new tests |
| `just slow-tests fresh` | exit 0 | slow-test gate |
| `git diff --check` | clean | all |

## Implementation of Review Findings #4

> **started at:** 2026-11-15T09:16:40-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sequences/simplified-schema/fixes/2026-11-14-absent-equals-null/review-4.md'
- this is iteration 4 of the review-to-implement cycle 
- the review's `implemented` flag was confirmed `false` before repairs began
- the review lists no `human_in_the_loop` items and no blockers; both findings are high and cite spec authority ("Validation and default preparation"; "Verification" + slow-test rule)
- `recurrence: true`: R4-F1 is a sibling of R3-F1/R2-F1/R1-F1; the Recurrence section's minimum scope is every route in the R4-F1 table combined with an independent union, plus two unions joined together

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R4-F1 | Used defaults skip constraints in independent unions: a used default must be judged against every independently imposed union as a group before child completion or conversion | fixed | all 22 children verified; 18 new public-result tests (14 failed before fix, 18 pass after); scoped simplified-schema 387 + 232 + minimal 359, darkmatter 1658, claudine 901 passed; full area gates below |
| R4-F1.1 | SimplifiedSchema overlapping prefix/suffix selectors with resolved named union `Nested` → `Unprepared(InvalidDefault)` at `x-settings.options`, strict and soft | fixed | pattern_defaults::independent_union_checks::simplified_named_alternatives_refuse_a_default_neither_accepts (strict + soft, `x-settings.options`) |
| R4-F1.2 | Raw overlapping `patternProperties`, union under `-settings$` (`anyOf` and `oneOf`) → same refusal | fixed | pattern_defaults::independent_union_checks::overlapping_raw_patterns_refuse_the_default (`anyOf` and `oneOf`) |
| R4-F1.3 | Raw literal plus matching pattern, union from the pattern → same refusal | fixed | pattern_defaults::independent_union_checks::a_literal_and_a_matching_patterns_union_refuse_the_default |
| R4-F1.4 | Literal properties across `allOf`, parent and union in different members → same refusal | fixed | pattern_defaults::independent_union_checks::a_literal_merged_across_all_of_refuses_the_default |
| R4-F1.5 | Pattern plus another member's catch-all, union from catch-all → same refusal | fixed | pattern_defaults::independent_union_checks::a_pattern_and_another_members_catch_all_union_refuse_the_default |
| R4-F1.6 | Separate catch-alls across `allOf` → same refusal | fixed | pattern_defaults::independent_union_checks::separate_catch_alls_across_all_of_refuse_the_default |
| R4-F1.7 | Local `$ref` targets across `allOf`, input `{}` → `InvalidDefault` at `options` | fixed | pattern_defaults::independent_union_checks::reference_targets_across_all_of_refuse_the_default (path `options`) |
| R4-F1.8 | Joined array `items`, input `[{}]` → `InvalidDefault` at `0.options` | fixed | pattern_defaults::independent_union_checks::joined_array_items_refuse_the_default (path `0.options`) |
| R4-F1.9 | Joined tuple `prefixItems` → same refusal; no inserted position | fixed | pattern_defaults::independent_union_checks::joined_tuple_positions_refuse_the_default_and_insert_no_position |
| R4-F1.10 | Common root `properties` default beside a root union → `InvalidDefault` at `options` | fixed | pattern_defaults::independent_union_checks::a_common_default_beside_a_root_union_is_refused |
| R4-F1.11 | Two independent unions joined by `allOf` → refusal; each union's grouping preserved | fixed | pattern_defaults::independent_union_checks::a_default_on_one_union_is_judged_against_another_independent_union |
| R4-F1.12 | Common scalar default `"3"` beside number/boolean union → schema `InvalidDefault` at `timeout` in both modes (no soft conversion) | fixed | pattern_defaults::independent_union_checks::a_scalar_default_beside_a_union_is_refused_without_conversion (`InvalidDefault` at `timeout` both modes, `anyOf`/`oneOf`) |
| R4-F1.13 | Public `DefaultPlan::compile` → `prepare_defaults` on raw common-root fixture → refuses parent default before descending | fixed | pattern_defaults::independent_union_checks::default_preparation_refuses_before_completing_children (public `DefaultPlan::compile` + `prepare_defaults`) |
| R4-F1.14 | Public `Prepared::validate_strict` on same fixture → `Unprepared(InvalidDefault)` | fixed | pattern_defaults::independent_union_checks::prepared_verdicts_refuse_the_default (strict) |
| R4-F1.15 | Public `Prepared::validate_soft` on same fixture → same refusal | fixed | pattern_defaults::independent_union_checks::prepared_verdicts_refuse_the_default (soft) |
| R4-F1.16 | Control: complete union inside the property's own definition → refuses (retain) | clean | pattern_defaults::independent_union_checks::a_union_inside_the_properties_own_definition_refuses_the_default passes before and after (the union is part of the definition's own structural validator) |
| R4-F1.17 | Control: R3-F1 direct-conjunction fixtures → refuse (retain) | clean | all `complete_default_checks` (R3-F1) tests pass unchanged |
| R4-F1.18 | Control: one-arm default policy on the default's own union (`required_defaults::unions`) → still passes; no per-alternative check | clean | all `required_defaults::unions` tests pass unchanged: definitions on a sibling member of the declaring arm's own union are still skipped (`conflicts`) |
| R4-F1.19 | Control: null-only optional wrappers → still refuse | clean | `complete_default_checks::nullable_selector_values_still_refuse_the_default` passes; a null-only member is never an accepting alternative |
| R4-F1.20 | Control: valid default + optional child; authored `{}` gets child default; `enabled: false` unchanged; repeat stability | clean | `options_shapes_beneath_the_required_child_fixture` + new pattern_defaults::independent_union_checks::authored_values_beside_an_independent_union_are_still_completed (authored `{}` gets only `enabled: true`; `false` kept; repeat soft stable); new positive controls `an_independent_union_member_accepting_the_default_lets_it_apply`, `an_independent_union_member_without_the_key_lets_the_default_apply` |
| R4-F1.21 | Comments/docs/skills describing the default-check contract updated to the repaired rule | fixed | rustdoc (module, `Entry::definitions`, `Declared::arms`, `chosen`, `conflicts`, `check`, `accepts`, `verdict`); `docs/topics/validation.md`, `docs/topics/language/unions.md`, `docs/topics/diagnostics.md`; `.claude/skills/simplified-schema/defaults.md`; `SKILL.md` states no such rule; area has no `docs/use-skills` |
| R4-F1.22 | Host consumers (Darkmatter `EffectiveSchema::complete`, Claudine) inherit the library repair; no host walk | clean | read `EffectiveSchema::complete` (darkmatter/lib/src/markdown/schemas/mod.rs:1844): delegates only to `prepare_defaults`; no host default-validity walk in darkmatter or claudine; no shipped schema broke (darkmatter/claudine scoped schema suites pass); runtime-only change, no `PREPARATION_REVISION`/artifact change |
| R4-F2 | Claudine L1 tests exceed the timing budget: every claudine `just test` L1 test must stay below 10 s in the package run (loaded) and ≤5 s alone | already fixed | canonical `claudine/` `just test` after the R4-F1 change (09:27–09:32, load 12→55 on 16 cores): 12,170 passed, 15 recipe skips, `L1 run recorded: none at or above 5.0s`; the 20 repair rows took 1.000–2.843 s in the package run; slowest test in the run 3.704 s; no code or test change made |
| R4-F2.54 | claudine-cli `a_parallel_group_commits_one_named_result` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.663 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.56 | claudine-cli `group_variables_reach_members_and_do_not_leak_to_the_next_step` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.649 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.57 | claudine-cli `outputs_chain_through_root_sequence_tasks` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.628 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.58 | claudine-cli `parallel_group_members_keep_separate_provider_sessions` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.761 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.62 | claudine-cli `existing_include::a_dry_run_whose_include_exists_still_succeeds_without_side_effects` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.044 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.63 | claudine-cli `existing_include::a_sequence_whose_include_exists_runs_unchanged` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.193 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.64 | claudine-cli `a_body_shell_expansion_reads_the_filesystem_at_its_turn` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.218 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.66 | claudine-cli `a_later_step_composes_against_the_edit_an_earlier_step_made` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.203 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.69 | claudine-cli `a_set_from_an_earlier_step_is_composed_into_a_later_step` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.752 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.72 | claudine-cli `an_authored_state_mutation_does_not_override_the_sequence_binding` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.804 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.97 | claudine-cli `playa_dry_run_prepares_nothing_and_leaves_no_audio_artifacts` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 2.843 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.104 | claudine-cli `predicates_operands_and_proxy_with_values_are_rejected_before_launch` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.914 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.118 | claudine-cli `inline_compose_uses_a_caller_supplied_prompt_without_persisting_it` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.474 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.119 | claudine-cli `inline_compose_without_an_eager_prompt_fails_before_launch_naming_it` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.742 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.126 | claudine-cli `sequence_dry_run_fail_fast_on_composition_error` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.728 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.128 | claudine-cli `sequence_dry_run_quiet_and_silent_are_no_op` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.490 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.129 | claudine-cli `sequence_dry_run_single_invalid_renders_state_per_step` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.006 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.130 | claudine-cli `sequence_dry_run_single_not_installed_renders_state_per_step` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.008 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.131 | claudine-cli `sequence_dry_run_zero_installed_list_renders_state_per_step` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.000 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.132 | claudine-cli `sequence_live_auto_selectable_launches_provider` (review: ≥10 s in package) → <10 s in canonical package run | already fixed | 1.263 s in the canonical package run (<10 s and ≤5 s); no repair needed |
| R4-F2.controls | 132 cleared control rows (R4-F2.1–R4-F2.152 excluding the 20 above) → remain <10 s loaded / ≤5 s alone in the final run | clean | same run: no test of the 12,170 at or above 5 s, so every cleared control row is under 5 s in the package run |
| R4-F2.shared | Shared composition/schema/subprocess work reached by those workflows: remove repeated setup or compilation within one operation where found | clean | not reproduced, so no profiling target: each flagged workflow is one CLI spawn taking 1.0–1.9 s in the package (playa dry run 2.8 s) and 0.4–1.7 s alone per the review, consistent per spawn; the review's 10–15 s readings were taken at load 184 on 16 cores (11× core count), a contention level the canonical run does not produce; no repeated per-operation work was indicated, so no speculative rewrite was made |

### Work Log

- starting the work on 'R4-F1: Used defaults skip constraints in independent unions' at 09:19:30
        - defect class: when deciding which schema definitions a used default must satisfy, a definition reached through a union that the default's own arm does not choose is dropped outright, instead of its union being judged as a group of alternatives (at least one must accept the default)
        - delegated to one foreground `rust-developer` subagent (serial, the only one running); its log items are recorded below when it returns
        - 09:18:32 subagent loaded the rust, rust-testing and simplified-schema skills; read review-4 R4-F1 and `defaults.rs`
        - 09:21:33 reproduced: 18 new `pattern_defaults::independent_union_checks` tests, 14 failing (every R4-F1.1–R4-F1.15 route, `anyOf` and `oneOf`), 4 controls passing
        - 09:22:36 repaired: `within` replaced by `chosen`/`conflicts` and a recursive `Fill::accepts`; 18/18 new tests pass, 387/387 in the scoped default filter
                - definitions within the declaring arm must all accept the default; definitions on a sibling member of a union the arm chooses are skipped (one-arm policy kept)
                - each union the arm makes no choice in is a group: at least one member must accept, judged recursively with that member added (nested unions judged within their member); a member defining nothing for the key accepts; a null-only member never does
                - each definition is validated at most once per check (per-check verdict map over the existing `OnceLock` validators); member count read from the union array in the plan's schema, no new compiler state
                - decision: an unresolvable union pointer accepts (lenient); alternative of refusing would turn a plan-internal inconsistency into a user-facing `InvalidDefault` with no actionable location; ledger only
        - 09:22:54 minimal build, same filter: 359/359
        - 09:24:15 darkmatter scoped schema/default/lifecycle filter: 1658/1658, no slow tests; R4-F1.22 confirmed by reading
        - 09:26:04 claudine `claudine_schemas | shipped`: 901/901
        - 09:26:34 `just lint` (simplified-schema) exit 0; docs updated (R4-F1.21)
        - orchestrator reviewed the `check`/`accepts`/`verdict` diff against the design and the spec's one-arm policy
- work completed for 'R4-F1: Used defaults skip constraints in independent unions' at 09:27:54
- starting the work on 'R4-F2: Claudine L1 tests exceed the timing budget' at 09:28:02
        - defect class: Level 1 workflows that spawn the CLI pass their assertions but exceed the 10 s loaded-host ceiling in the package run because of CPU/process contention or repeated work per operation
        - first step: measure the canonical `claudine/` `just test` on the current host (uptime: 12.06 19.95 32.20) to see which rows still fail
        - 09:32 canonical `claudine/` `just test` (after the R4-F1 change): 12,170 passed, 15 skips, `L1 run recorded: 12170 test(s), none at or above 5.0s`; 4 min 42 s wall; load 12.4 at start, 43.9–54.9 at end on 16 cores
                - the 20 rows the review marked for repair took 1.000–2.843 s in this package run; the slowest test in the run took 3.704 s
                - every row listed in the review is under 5 s in the package run, so none needs a solo re-measurement under the slow-test rule
        - decision: no test or production change for R4-F2. The review's 10.1–15.4 s readings were taken at load 184 on 16 cores and the same tests take ≤2.9 s in the canonical package run; the gate it requires (no L1 test ≥10 s in the package run, none over 5 s on a quiet host) passes. Rewriting twenty single-spawn tests would be a speculative change with no measured target. Ledger only: this is an outcome measurement, not an author decision
- work completed for 'R4-F2: Claudine L1 tests exceed the timing budget' at 09:34:00
- final verification (orchestrator, after the subagent finished, nothing else running) completed at 09:40:30
        - `darkmatter/` `just test`: one test over 5 s, `markdown::compose::shell_expansion::probe::tests::installed_shells::injection_shaped_names_never_execute`, 5.420 s in the package run at load 30.7–38.0 on 16 cores (under the 10 s loaded allowance), 2.179 s alone at load 11.0; cleared as load-sensitive (same test and pattern as iteration 3)

### Completion Report

- **outcome: complete**; elapsed 24 min (09:16:40 → 09:41 local)
- parent findings: total 2 = 1 fixed (R4-F1) + 1 already fixed (R4-F2) + 0 blocked + 0 unresolved + 0 disputed
- instance rows:
        - R4-F1: 22 children (the review's 20 plus R4-F1.21 docs and R4-F1.22 host consumers); 16 fixed (R4-F1.1–R4-F1.15, R4-F1.21) and 6 clean (R4-F1.16–R4-F1.20, R4-F1.22); none open
        - R4-F2: 20 repair rows already fixed (1.000–2.843 s in the canonical package run), the 132 control rows clean (no test ≥5 s), shared-work row clean (not reproduced, so nothing to profile); none open
- verified repairs:
        - R4-F1: the review's independent object-union cases are repaired. A used default is judged from each declaring arm; sibling alternatives of a union that arm chooses are skipped, while each independent union is checked as a group. In the listed object-only fixtures, an invalid parent default is refused before child completion and a scalar default before conversion. The broader claim that this covers every independent union was false: a member with no definition for the key is accepted even when its type cannot describe the containing object. Review 5's public probes reproduce that remaining sibling as R5-F1. Both modes remain covered for every original structural route with `anyOf` and `oneOf`, plus direct `prepare_defaults`/`Prepared` probes
        - R4-F2: no code change. The review's readings came from a run at load 184 on 16 cores; the canonical package run after this cycle's change records no L1 test at or above 5 s
- open finding/child IDs: none
- no human-review item raised; the R4-F1 design decision (an unresolvable union pointer accepts) and the R4-F2 no-change decision are recorded in the work log only
- cross-OS: in-process JSON-schema logic only, no path, process, or `cfg` code; no `just cross-check` run required
- changed files (this iteration):
        - `simplified-schema/lib/src/defaults.rs` (R4-F1, R4-F1.21 rustdoc)
        - `simplified-schema/lib/tests/l1/pattern_defaults.rs` (R4-F1.1–R4-F1.20: module `independent_union_checks`, 18 tests)
        - `simplified-schema/docs/topics/validation.md`, `simplified-schema/docs/topics/language/unions.md`, `simplified-schema/docs/topics/diagnostics.md` (R4-F1.21)
        - `.claude/skills/simplified-schema/defaults.md` (R4-F1.21)
- final verification (macOS):

| Command | Result | Verifies |
| --- | --- | --- |
| `simplified-schema/`: `just test` | 2,325 + 1,966 (minimal) passed; none at or above 5 s | R4-F1 |
| `darkmatter/`: `just test` | 16,730 passed, 10 recipe skips; 1 load-sensitive test at 5.420 s (load 30–38, below the 10 s allowance), 2.179 s alone | R4-F1.22 host consumer |
| `claudine/`: `just test` | 12,170 passed, 15 recipe skips; none at or above 5 s; the 20 flagged rows took 1.000–2.843 s | R4-F2, R4-F1.22 |
| `just lint` in simplified-schema, darkmatter, claudine | exit 0 each | all |
| `just check-tier-coverage simplified-schema darkmatter claudine` | exit 0, nothing stranded | new tests |
| `just slow-tests fresh` | exit 0 | slow-test gate |
| `git diff --check` | clean | all |

### Review 5 record correction — 2026-11-15

The completion report's general statement that independent unions always
refuse an invalid default before child completion or conversion was too broad.
Its 22 listed instances are verified, but a non-object member without the key
is counted as accepting a default inside an object. Updated the report in
place. Verified using the same parent/required-child fixture with a string
alternative, through public `SimplifiedSchema`, `Prepared` and direct
`prepare_defaults` calls in default-feature and minimal builds. A plain
validator rejects the uncompleted default and accepts the completed object.
The same sweep verified the bypass for a closed object alternative whose
Boolean catch-all denial is dropped; explicit false property/pattern
definitions correctly refuse the default. This remaining sibling class is
review 5's R5-F1; the original instance dispositions
and recorded test runs remain unchanged.

## Implementation of Review Findings #5

> **started at:** 2026-11-15T10:05:00-08:00

- this implementation addresses the one finding in `review-5.md`
- this is iteration 5 of the review-to-implement cycle, run interactively at the author's direction after the automated cycle stopped at its five-cycle limit
- the review lists no `human_in_the_loop` items and no blockers. The reviewer noted that successive repairs were chasing ever narrower combinations, so the author added a boundary to the spec's used-default requirement in the same change (spec, "Default validity and reporting", ruling of 2026-11-15)

### Finding Ledger

| ID | Expected | Disposition | Evidence |
| --- | --- | --- | --- |
| R5-F1.1 | SimplifiedSchema string shorthand refuses the parent default | fixed | `members_that_cannot_hold_the_key::a_simplified_string_shorthand_refuses_the_default` |
| R5-F1.2–R5-F1.11 | Every raw structural route refuses beside a member that cannot hold the key | fixed | `every_route_refuses_a_default_beside_a_member_that_cannot_hold_the_key`: 10 routes × 2 keywords × 2 modes × 13 alternatives |
| R5-F1.12 | Static forms: type array without object, scalar `const`/`enum`, `false`, `$ref` to string and to null | fixed | same test's alternative list |
| R5-F1.13 | Scalar default refused before conversion | fixed | `a_scalar_default_beside_a_string_member_is_refused_before_conversion` |
| R5-F1.14–R5-F1.16 | Public `prepare_defaults`, `Prepared` strict/soft refuse | fixed | `public_preparation_and_prepared_verdicts_refuse_the_default` |
| R5-F1.17–R5-F1.21 | Property-local, conjunction, object-only union, one-arm and direct-null controls | clean | the existing `complete_default_checks`/`independent_union_checks` tests still pass |
| R5-F1.22 | Authored values keep current behavior | clean | `authored_values_beside_a_string_member_are_still_completed` |
| R5-F1.23 | Object-admitting members (open object, type array with object, `$ref` to object, closed object whose pattern admits the key) still accept | clean | `object_admitting_members_still_let_the_default_apply` |
| R5-F1.24 | Docs, rustdoc and skill state the admission rule | fixed | `defaults.rs` module docs and `admits_key`; `docs/topics/validation.md` (rule plus a new "What the default check reads" boundary bullet); `docs/topics/language/unions.md`; `.claude/skills/simplified-schema/defaults.md`. `diagnostics.md` had no statement of the rule, so no change there |
| R5-F1.25 | Hosts inherit the repair | clean | Darkmatter and Claudine delegate to `prepare_defaults`; their full `just test` runs pass |
| R5-F1.26 | Closed `additionalProperties: false` member, direct and through `$ref`, on every route | fixed | the alternative list for `every_route_…` includes both |
| R5-F1.27 | Explicit false property/pattern controls | clean | `explicit_false_definitions_refuse_the_default` |

### Work Log

- repair in `Fill::accepts`: a union member with no definition for the key now accepts only if `admits_key` (new, `defaults.rs`) says it can hold an object with that key. It reads the member statically: `type`/`const`/`enum` must allow an object, `false` never admits, and `additionalProperties: false` denies a key that no `properties` entry or own pattern names. It follows local `$ref` and `allOf`, needs one admitting member for a nested `anyOf`/`oneOf`, and uses a path guard against reference loops. Anything it cannot read (an unresolvable reference) admits. This replaces the earlier `admits_only_null` exclusion at that point, and covers it
- not changed: arm recording in `Compiler::links` still treats only `null`-only members as no alternative. A member `admits_key` excludes is now skipped where the empty selection is admitted, so the verdict is the same. Changing arm recording would also have changed compiled plans for the union-heavy lifecycle baseline, with no case where the outcome differs. Ledger only
- reproduction check: with the old exclusion temporarily restored, 4 of the 7 new tests failed (the string shorthand, every-route, scalar and public-API tests) and the 3 controls passed. The repair was then restored
- rejected approach: validating the unfinished parent against each member to pick one. The review rules it out (it would make defaults part of matching), and the spec boundary now says so

### Completion Report

- **outcome: complete**
- parent findings: 1 total, 1 fixed (R5-F1), 0 blocked, 0 unresolved, 0 disputed
- instance rows: 27. Fixed: .1–.16, .24, .26. Clean: .17–.23, .25, .27. None open
- changed files:
        - `simplified-schema/lib/src/defaults.rs` (repair, rustdoc)
        - `simplified-schema/lib/tests/l1/pattern_defaults.rs` (module `independent_union_checks::members_that_cannot_hold_the_key`, 7 tests)
        - `simplified-schema/docs/topics/validation.md`, `simplified-schema/docs/topics/language/unions.md`, `.claude/skills/simplified-schema/defaults.md`
        - `spec.md` (boundary ruling)
- cross-OS: in-process JSON Schema logic only. No path, process or `cfg` code changed, so `just cross-check` was not required
- final verification (macOS):

| Command | Result | Verifies |
| --- | --- | --- |
| `simplified-schema/`: `just test` | both builds pass (minimal build: 1,973) | R5-F1 |
| `simplified-schema/`: `just lint`, `just doctest` | exit 0; 40 doctests passed | all |
| `darkmatter/`: `just test` | 16,730 passed, 10 recipe skips; one test took 5.676 s at load ~60 on 16 cores (`darkmatter-lsp` `diagnostics_then_point_requests_reuse_one_snapshot`) and 2.315 s alone | R5-F1.25 |
| `claudine/`: `just test` | 12,170 passed, 15 recipe skips; one test took 5.191 s at load 67 (`claudine_schemas::shared_payload_properties_have_identical_types`) and 2.383 s alone | R5-F1.25 |
| root `just shipped-schemas` | every shipped artifact is current | no artifact change |
| `just check-tier-coverage simplified-schema` | 0 stranded | new tests |
| `git diff --check` | clean | all |
