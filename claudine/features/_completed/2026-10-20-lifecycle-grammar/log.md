---
implementation_1: "2026-10-13T19:45:49-07:00"
---

# Lifecycle Grammar — Implementation Log

## Implementation of Review Findings #1

> **started at:** 2026-10-13T19:45:49-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'claudine/features/2026-10-20-lifecycle-grammar/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | High: mixed-text expressions bypass validation in unselected branches | fixed | all required children R1-F1.1–F1.14 fixed or clean; claudine `just test` 8885 passed / 9 skipped; `just lint` exit 0 |
| R1-F1.1 | Communication action reader (`say`/`speak`/`effect`/`message`/`notify`/`stderr`/`stdout`/`info`/`warn`/`success`, short and long forms): malformed span rejected before launch | fixed | `mixed_span::every_reader_rejects_a_malformed_span_in_either_unselected_branch` (10 verbs × short/long × then/else) |
| R1-F1.2 | Control parameter reader (`error.reason`, `proxy.target`, `retry.max_attempts`, `retry.delay`, `resume.message`, `resume.max_attempts`, `defer.delay`, `defer.reason`): rejected before launch | fixed | same test, 13 control rows |
| R1-F1.3 | Recursive runtime mutation reader (`set.note`, nested `set.meta.notes[0]`): rejected before launch | fixed | same test, `set.note` and `set.meta.notes[0]` |
| R1-F1.4 | Recursive overlay reader (`proxy.with`, `retry.with`, scalar and nested): rejected before launch | fixed | same test, `proxy.with`/`retry.with` scalar and nested; `proxy_with_evaluation::a_malformed_span_in_mixed_text_is_a_parse_error` |
| R1-F1.5 | Side-effect positional reader (all 12 side-effect verbs): rejected before launch | fixed | same test, all 12 side-effect verbs positional |
| R1-F1.6 | Side-effect named-parameter reader (`ensure_file.content`): rejected before launch | fixed | same test, `ensure_file.content` |
| R1-F1.7 | Shared expression-function reader (`length` positional and named): rejected before launch | fixed | same test, `length` positional and named |
| R1-F1.8 | Shell preflight reader (`shell.command`, `shell.on_error`): already rejects (clean control) | clean | preflight `resolve_lifecycle_shell_commands` resolves every shell span in both branches; parser now also rejects earlier |
| R1-F1.9 | Guard parser (nested `when: "fm."`): already rejects (clean control) | clean | guards parsed by `parse_condition`; covered by `binding_contract` tests |
| R1-F1.10 | Whole-value action parser (`info: "{{ fm. }}"`): already rejects (clean control) | clean | `mixed_span::the_whole_value_and_mixed_forms_share_one_typed_cause` |
| R1-F1.11 | Every stack home (`initialize`, `start`, `blocked`, `success`, `failure`, `finalize`, `loop.gate`, task `setup`/`teardown`): rejected before launch / task execution | fixed | `mixed_span::every_stack_home_rejects_a_malformed_span_in_an_unselected_branch` (six events, `loop.gate`, loop loose verb, task setup/teardown); CLI `lifecycle_mixed_span_validation` loop-gate and task rows |
| R1-F1.12 | Command boundaries `compose`, `inline-compose`, `sequence`: rejected before provider launch | fixed | CLI `lifecycle_mixed_span_validation`: compose, `inline_compose_rejects_…`, `sequence_rejects_…`; non-zero exit, zero provider launches |
| R1-F1.13 | Error names authored operand path (e.g. `start[0].then[0].info.text`) | fixed | library tests assert exact operand path (e.g. `failure[0].else[0].info.text`); CLI compose/loop rows assert it in output (task rows assert action path because the step summary truncates) |
| R1-F1.14 | Syntactically valid operand that fails only at evaluation stays unevaluated in unselected branch | fixed | `conditions_control::an_unselected_mixed_span_that_fails_only_at_evaluation_is_accepted_and_skipped`; CLI `compose_accepts_an_evaluation_only_failure_…` and `compose_accepts_valid_mixed_text_…` |
| R1-O1   | Observation: existing opt-in prompt failures remain separate work (nonblocking) | deferred | nonblocking observation; no repair required; pre-existing opt-in prompt failures stay with the author to schedule |

- starting the work on 'R1-F1 mixed-text span validation' at 19:46:37
        - defect class: mixed-text action operands keep their interpolation spans unparsed through static validation, so syntactically invalid spans are accepted until evaluated
        - root cause: `action_shape.rs::action_value_to_expr` parsed only a whole-value `{{ … }}` and returned any other string as `Expr::StringLiteral` unchecked; `validate.rs::validate_no_err_in_no_error_events` then `continue`d on the `prepare_value` failure
        - reference behavior: shell `command`/`on_error` mixed text is rejected because `preflight::resolve_lifecycle_shell_commands` resolves (evaluates) every shell span in both branches for approval; guards are parsed by `parse_condition`; whole values by `parse` inside `action_value_to_expr`
        - sweep: every authored-operand reader funnels through `action_value_to_expr` — short form (`classify_positional_value`: communication, control, side-effect, expression-function positional args), long form (`parse.rs` named params: `text`, `reason`, `target`, `max_attempts`, `delay`, `message`, `ensure_file.content`, `length` named), `set` and `proxy.with`/`retry.with` recursion (`actions.rs::type_with_value`); task `setup`/`teardown` (`parse_task_action_stack_with`), `loop.gate`, and loop loose verbs use the same `StackParser`
        - other `StringLiteral(`/`find_all_plain` sites checked and left alone: `executor.rs::evaluate_operand` (runtime), `mod.rs::emit_signal` (drop-net skips templated text), `parse.rs` `set`/`with` whole-mapping detection, `looping/config.rs` identifier collection (analysis only), `looping/actions.rs` (loop action renderer, fails closed on parse), `sequence/preflight/shape.rs::first_target_identity_root` (graph shell analysis), `sequence/task/mod.rs::apply_approved_commands` (compares authored text already parsed), `dispatch/template.rs` (not lifecycle)
        - fix: `action_value_to_expr` now parses every span of mixed text with the same Darkmatter `parse` the whole-value case uses, without evaluating, and still returns the literal; failure is the new `ActionExprError::SpanParse { expression, source }` inside `LifecycleActionInvalidLongForm`
        - operand path: short-form conversion errors now say `` `{action}.{field}` is not a valid value `` (field from the new shared `action_shape.rs::positional_operand_field`, also used by `source_map.rs`; `arg[k]` otherwise); long-form says `` `{action}.{key}` ``; `set`/`with` already named the nested path
        - `validate.rs`: doc comment and the `continue` comment corrected (syntax is the parser's job; the branch is unreachable for a parsed config)
        - tests updated (encoded the old event-time deferral): `binding_contract::malformed_expressions_and_unknown_functions_halt_before_side_effects`, `proxy_with_evaluation::malformed_expression_and_unknown_function_raise` (+ new `a_malformed_span_in_mixed_text_is_a_parse_error`), `prepare::tests::lifecycle_malformed_span_is_deferred_raw_through_prepare` → `lifecycle_malformed_mixed_span_is_rejected_at_prepare`, `multiple_lifecycle_spans_all_deferred_raw` now uses evaluation-only failures
        - tests added (library): `lifecycle/tests/mixed_span.rs` — `every_reader_rejects_a_malformed_span_in_either_unselected_branch` (54 reader rows × then/else), `every_stack_home_rejects_a_malformed_span_in_an_unselected_branch` (six events, `loop.gate`, loop loose verb, task setup/teardown), `the_whole_value_and_mixed_forms_share_one_typed_cause`, `valid_mixed_text_and_span_free_literals_are_accepted_unchanged`; `conditions_control::an_unselected_mixed_span_that_fails_only_at_evaluation_is_accepted_and_skipped`
        - tests added (CLI L1, Unix, recording `goose` stub): `cli/tests/l1/lifecycle_mixed_span_validation.rs` — compose `start` then / `failure` else, `loop.gate`, `inline-compose`, `sequence` own stack, referenced task `setup`/`teardown`, plus two accepting controls (valid mixed text; `no_such_fn(fm.missing.deep)`); each 0.2–0.3 s
        - note: a referenced task file's stacks are parsed when the task starts (before setup runs or its provider launches), and the one-line step failure truncates the message, so the CLI row names the action path; inline `setup:` in the sequence document was already refused by sequence frontmatter interpolation
        - docs: `docs/topics/flow-control/lifecycle.md` (both-branches bullet; `LifecycleActionInvalidLongForm` row); `.claude/skills/claudine/architecture.md` (DM2 is the backstop)
        - checks: `just test-library lifecycle` 626 passed; `just test-cli lifecycle_mixed_span_validation` 9 passed; `compose_removed_grammar` 29, `overlay_layering` 31, `lifecycle_set_shell_values` 9 passed; full `just test` 8885 passed, 9 skipped; full `just test-library` 4859 passed; `just lint` exit 0 (only the host linker `__eh_frame` warning); no Darkmatter change
- work completed for 'R1-F1 mixed-text span validation' at 20:06:17

- orchestrator reconciliation at 20:09:30
        - inspected the combined diff: the repair is one change in `action_shape.rs::action_value_to_expr` (every span in mixed text is parsed with Darkmatter's `parse`, never evaluated, and the value is still stored as a literal) plus the operand-path naming in `classify_positional_value`; no new grammar, escaping layer, or syntax; lift-ready imports unchanged
        - the CLI L1 file is `#[cfg(unix)]`, matching the existing recording-stub CLI tests in `cli/tests/l1/main.rs`; the library tests that carry the per-reader matrix are cross-platform
        - no OS-specific code was touched (pure expression parsing), so no cross-OS rig run was needed; CI's Linux/macOS cells and the nightly Windows/WSL2 cells cover it
        - `just check-tier-coverage claudine`: 0 stranded tests

### Completion Report

- **status:** complete
- **elapsed:** 2026-10-13T19:45:49-07:00 → 20:10 (about 24 minutes)
- **parent findings:** total 1 = fixed 1 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0 + disputed 0
        - instance rows: 14 (11 fixed, 3 clean controls)
        - observation R1-O1 is nonblocking; no repair required, and it stays with the author to schedule
- **verified repair (R1-F1):** a malformed `{{ … }}` span inside mixed text is now rejected at parse time in every reader (communication, control, `set`, `with`, side-effect, and expression-function operands) and every stack home (six events, `loop.gate`, loop loose verbs, task `setup`/`teardown`), in either unselected branch, before provider launch in `compose`, `inline-compose`, and `sequence`; the error is `LifecycleActionInvalidLongForm` with the new typed cause `ActionExprError::SpanParse` and names the authored operand path; a span that parses but would fail only when evaluated is accepted and stays unevaluated
- **open findings:** none
- **changed files:**
        - `claudine/lib/src/composition/lifecycle/action_shape.rs`, `parse.rs`, `source_map.rs`, `validate.rs`
        - `claudine/lib/src/composition/error/mod.rs`
        - `claudine/lib/src/composition/lifecycle/tests/mixed_span.rs` (new), `tests/mod.rs`
        - `claudine/lib/src/composition/lifecycle/executor/tests/conditions_control.rs`, `binding_contract.rs`, `proxy_with_evaluation.rs`
        - `claudine/lib/src/composition/prepare/tests.rs`
        - `claudine/cli/tests/l1/lifecycle_mixed_span_validation.rs` (new), `claudine/cli/tests/l1/main.rs`
        - `claudine/docs/topics/flow-control/lifecycle.md`, `.claude/skills/claudine/architecture.md`
- **final verification (all tied to R1-F1):**
        - `claudine/`: `just test` → 8885 passed, 9 skipped, 0 failed
        - `claudine/`: `just lint` → exit 0
        - repo root: `just check-tier-coverage claudine` → 0 stranded
        - focused (subagent): `just test-library lifecycle` 626 passed; `just test-cli lifecycle_mixed_span_validation` 9 passed; `compose_removed_grammar` 29, `overlay_layering` 31, `lifecycle_set_shell_values` 9 passed
        - Darkmatter was not changed, so its checks were not rerun
