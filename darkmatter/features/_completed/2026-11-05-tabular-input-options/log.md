---
implementation_1: "2026-11-08T20:09:52-08:00"
---

# Tabular Input Options — Review/Fix Log

## Implementation of Review Findings #1

> **started at:** 2026-11-08T20:09:52-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-tabular-input-options/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review `implemented` confirmed `false` before repairs began
- the review has no `human_in_the_loop` items and no blockers; its 19 `human_review_items` are informational and need no action

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Passive option checks ignore shared coercion results | fixed | every child row fixed/clean; orchestrator reviewed `OptionsView::of` binding through `bind_callback_value`; full `just test` green |
| R1-F1.1 | Passive headerless decision: CSV object contract, `{header: "false"}` → `dm.data.row_shape_conflict` at `as` | fixed | `load_data_options_passive::coerced_values::string_header_false_with_csv_object_contract_is_rejected_at_as`; `load_data_options::coerced_literal_options_are_rejected_before_anything_runs` (live + dead branch, no reads) |
| R1-F1.2 | Passive delimiter validator: CSV tuple contract, `{delimiter: 10}` → `dm.data.invalid_options` | fixed | `coerced_values::numeric_delimiter_on_csv_binds_to_text_and_is_invalid_options`; same composition test |
| R1-F1.3 | Passive non-tabular applicability: JSON `string[]`, `{header: "true"}` → `dm.data.invalid_options` | fixed | `coerced_values::string_header_on_json_is_invalid_options`; same composition test |
| R1-F1.4 | Passive TSV applicability: tuple contract, `{delimiter: 5}` → `dm.data.invalid_options` | fixed | `coerced_values::numeric_delimiter_on_tsv_is_invalid_options`; same composition test |
| R1-F1.5 | Partial object: CSV object contract, `{header: "false", delimiter: sep}` → row conflict rejected | fixed | `coerced_values::string_header_false_beside_a_computed_member_is_rejected_at_as`; composition test with frontmatter `sep: ","` (live + dead) |
| R1-F1.6 | DMLS projection of the four literal cases → same errors and ranges | fixed | `dmls option_arguments::coerced_literal_options_are_ranged_like_composition` (codes + argument ranges) |
| R1-F1.7 | Runtime binding of all five failing shapes → retain correct errors | clean | `load_data_options::coerced_computed_options_fail_at_execution_with_the_same_codes` |
| R1-F1.8 | Native-value controls `{header: false}` CSV objects, `{header: true}` JSON → still rejected | clean | `coerced_values::native_controls_are_still_rejected` |
| R1-F1.9 | Valid coerced control: CSV tuple contract, `{header: "true"}` → accepted and executes | clean | `coerced_values::string_header_true_on_a_csv_tuple_contract_is_accepted`; `load_data_options::coerced_header_true_on_a_tuple_contract_reads_rows` |
| R1-F1.10 | Format field aliases/defaults → clean | clean | format is a string enum the binder accepts unchanged or refuses; now read from bound map; `coerced_values::literal_format_is_read_from_the_bound_options` |
| R1-F1.11 | (sweep) `docs/topics/loading-data.md` claimed `{header: "false"}`/`{delimiter: 5}` are refused as wrong types; binder converts them | fixed | doc paragraph and two `dmls/docs/diagnostics.md` rows rewritten to describe judging after conversion |
| R1-F2   | Computed contracts suppress provable option applicability errors | fixed | every child row fixed/clean; orchestrator reviewed `load_data_problem`/`literal_carriers`/`binder_refuses`; full `just test` green |
| R1-F2.1 | Extension JSON, computed contract, `{header: true}` → invalid options | fixed | `load_data_options_passive::computed_contract::header_on_an_extension_json_file_is_invalid_options`; `load_data_options::computed_contract_does_not_hide_inapplicable_literal_options` (direct + dead branch, options column, no reads); CLI repro now exits 1 with `dm.data.invalid_options` at 4:45 |
| R1-F2.2 | Extension JSON, computed contract, `{delimiter: ";"}` → invalid options | fixed | `computed_contract::delimiter_on_an_extension_json_file_is_invalid_options`; same composition test |
| R1-F2.3 | Extension TSV, computed contract, `{delimiter: ";"}` → invalid options | fixed | `computed_contract::non_tab_delimiter_on_an_extension_tsv_file_is_invalid_options`; same composition test |
| R1-F2.4 | Explicit JSON override of `rows.csv`, computed contract, `{header: true}` → invalid options | fixed | `computed_contract::explicit_json_format_on_a_csv_file_with_header_is_invalid_options`; same composition test |
| R1-F2.5 | Every explicit non-tabular format and alias, computed contract, `{header: true}` → invalid options | fixed | `macro_rules!` tests `header_with_explicit_{yaml,yml,json,jsonl,ndjson,toml,json5}_is_invalid_options` |
| R1-F2.6 | DMLS projection of F2.1–F2.3 → same error and range | fixed | dmls `option_arguments::computed_contract_keeps_option_applicability_diagnostics` (exact options range; controls add no error) |
| R1-F2.7 | Runtime applicability → retain behavior | clean | `load_data_options::computed_contract_and_options_fail_applicability_at_execution` |
| R1-F2.8 | Format-independent delimiter check, computed contract, `{delimiter: ";;"}` → still rejected | clean | `computed_contract::unusable_delimiter_is_still_invalid_options` |
| R1-F2.9 | Computed contract with null fields; explicit CSV/TSV `header: true` → accepted | clean | `computed_contract::applicable_or_null_options_are_accepted`; `load_data_options::computed_contract_with_applicable_options_reads_rows` |
| R1-F2.10 | Whole computed options or computed format → defer | clean | `computed_contract::computed_options_or_format_still_defer` |
| R1-F2.11 | (sweep) binder refusal of `file`/`as` (e.g. `'{{bad'`) must outrank option checks, matching execution | fixed | `contract_precedence::malformed_contract_keeps_the_binder_error`; `provably_invalid_literal_contract_outranks_applicability` |
| R1-F2.12 | (sweep) partly-literal or nullable contract unions (`x ? 'string[]' : c`) also skipped applicability | fixed | `computed_contract::partly_literal_contracts_still_judge_applicability` |
| R1-F2.13 | (control) computed contract + CSV + `{header: false}` is not rejected passively (row shape unknowable) | clean | `computed_contract::headerless_csv_is_not_judged`; DMLS test; execution still raises `row_shape_conflict` |

### Progress

- starting the work on 'R1-F1: Passive option checks ignore shared coercion results' at 20:10:25
        - defect class: a passive semantic judgment reads an argument's authored value instead of the value the shared binder would produce from it, so any decision downstream of coercion diverges from execution
        - repair: `OptionsView::of` binds the non-null literal members (whole literal object, or the literal members of a partial object) through `FunctionRegistration::bind_callback_value(2, …)`; all four decisions (delimiter validity, non-tabular applicability, TSV tab rule, headerless rows) read the bound map; members that do not bind leave the view `Unknown` so the binder's error keeps precedence
        - sibling sweep: `OptionsView`/`"header"`/`"delimiter"`/`"format"` across `compose/expression/` and `dmls/src`; only `call_check.rs` reads authored option values (DMLS diagnostics reuse `check_call`; `option_argument.rs` only completes)
        - decisions: partial members that do not bind → judge nothing (alternative: judge the members that bind, rejected because it could report `invalid_options` where a run reports a binder error first); bind literal members together (equivalent since every field is optional)
        - tests added: `load_data_options_passive::coerced_values` (8), `load_data_options::coerced_*` (3), `dmls option_arguments::coerced_literal_options_are_ranged_like_composition` (1); with binding disabled the 8 fix-dependent tests failed
        - subagent checks: `just test coerced` 24/24, `just test load_data` 464/464, `just test option_arguments` 13/13, `just test call_check` 110/110, `just lint` exit 0
- work completed for 'R1-F1: Passive option checks ignore shared coercion results' at 20:18:58
- starting the work on 'R1-F2: Computed contracts suppress provable option applicability errors' at 20:18:58
        - defect class: an unknown value for one argument ends passive validation early, so independently decidable constraints on other arguments are never judged
        - repair: `load_data_problem` keeps an unreadable contract as `None` verdicts (new `literal_carriers` helper) instead of returning early; step 1 (delimiter validity) and step 3 (applicability on a provable format) always run, step 4 (row rules) needs a known contract; a provably invalid literal contract still reports first
        - sweep found and fixed R1-F2.11 (binder refusal of `file`/`as` now outranks option checks via `binder_refuses` + `leading_bind`) and R1-F2.12 (partly-literal/nullable contract unions)
        - decision: a bad *computed* contract beside inapplicable literal options now reports `invalid_options` passively, whereas execution would meet the contract error first; alternatives were deferring applicability for any computed contract (reintroduces R1-F2) or a new "reported problem may differ" marker; chosen because the spec requires checking literal options beside computed arguments and the call is rejected either way
        - docs: `docs/topics/loading-data.md`, `docs/cli/data-contracts.md`, `dmls/docs/diagnostics.md`, `.claude/skills/darkmatter/expressions.md`
        - tests added: `load_data_options_passive::{computed_contract, contract_precedence}`, `load_data_options` "Computed contracts" (3), dmls `computed_contract_keeps_option_applicability_diagnostics`; reverting the fix made 11 new tests fail
        - subagent checks: `just test load_data` 485/485, `just test option_arguments` 14/14, `just test call_check` 110/110, `just test docs_style` 14/14, `just test catalog` 265/265, `just lint` clean
- work completed for 'R1-F2: Computed contracts suppress provable option applicability errors' at 20:35:21

### Completion Report

- **outcome:** complete; elapsed about 25 minutes
- parent findings: total 2 = fixed 2 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- instance rows: R1-F1.1–F1.11 and R1-F2.1–F2.13 are all fixed or clean; none are open
- verified repairs:
        - R1-F1: the passive `load_data` check binds the literal options (whole object, or the literal members of a partial object) through the shared binder, then judges the bound values for delimiter validity, non-tabular applicability, the TSV tab rule, and headerless rows
        - R1-F2: an unknown `as` no longer ends the passive check; applicability is judged whenever the format is provable, and the binder's refusal of `file`/`as` keeps precedence
- human review: one decision was added to the review's `human_review_items` (precedence between a bad computed contract and inapplicable literal options)
- changed files:
        - `darkmatter/lib/src/markdown/compose/expression/call_check.rs` (R1-F1, R1-F2)
        - `darkmatter/lib/src/markdown/compose/expression/call_binding/bind.rs` (doc only, R1-F1)
        - `darkmatter/lib/tests/l1/load_data_options_passive.rs`, `darkmatter/lib/tests/l1/load_data_options.rs`, `darkmatter/dmls/tests/l1/option_arguments.rs` (tests, both findings)
        - `darkmatter/docs/topics/loading-data.md`, `darkmatter/docs/cli/data-contracts.md`, `darkmatter/dmls/docs/diagnostics.md`, `.claude/skills/darkmatter/expressions.md` (docs, both findings)
- final verification by orchestrator (in `darkmatter/`, after all changes, with no subagent running):
        - `just test`: 15898 passed, 10 skipped, exit 0; 9 tests unrelated to this feature ran at or just over 5 s under full-suite load; rerun alone, all passed, and the two closest (`compose_shell_template::every_suggested_rule_approves_the_command` 3.7 s, `content_policy_baseline::every_shared_declaration_publishes_the_baseline_verdict_at_its_anchor` 2.9 s) were already slow on earlier commits (37 prior records); recorded as load-sensitive
        - `just lint`: exit 0
        - cross-OS check skipped: the change is OS-neutral passive-check logic with no path, process, or OS-specific code
