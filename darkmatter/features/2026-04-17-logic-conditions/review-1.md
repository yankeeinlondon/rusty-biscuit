---
ready: true
reviewer: Claude (Opus 4.7)
reviewed_on: 2026-04-18
feature: infix-logic-conditions
---

# Review 1 — Infix Logic Conditions

## Summary

The feature is implemented end-to-end and cleanly matches the tech design. All
four plan phases landed: parse-mode plumbing + condition entrypoint, lexer
tokens, condition precedence ladder, evaluator short-circuit alignment,
integration wiring, regression coverage across parser/evaluator/compose/graph,
and documentation refresh. 1939 library tests pass; `cargo fmt -p darkmatter`
is clean; no clippy regressions were introduced by this work (existing clippy
errors live in unrelated `terminal/tests.rs`).

**Recommendation: ready for production.** The gaps below are minor polish and
additive test coverage — none of them are correctness blockers.

## What was verified

- Lexer mode switch (`lexer.rs::Lexer::with_mode`) produces distinct token
  streams: `||` → `Token::Pipe` in interpolation vs `Token::OrOr` in condition
  mode; `&&` → `Token::AndAnd` in condition mode and a lexer error otherwise
  (`lexer.rs:419`, `lexer.rs:433`).
- Parser exposes `parse_condition()` (`parser.rs:411`) that routes through the
  condition-mode ternary branch → `parse_logical_or` → `parse_logical_and` →
  `parse_fallback` ladder, lowering infix operators into the existing
  `Expr::FunctionCall { name: "And"|"Or", .. }` nodes. No new public `Expr`
  variant was added (non-goal respected).
- `compose::conditions::evaluate_condition` now calls `parse_condition`
  (`conditions.rs:39`). Consumers — page blocks (`page_blocks/engine.rs`),
  transclusion (`transclusion/conditions.rs`), and reference graph
  (`reference/graph.rs`) — all inherit the new syntax through that shared
  path, confirming the design's single-call-site assumption.
- `eval_function` short-circuits both `and` and `or` (`conditions.rs:119-136`),
  matching infix semantics.
- Documentation in `darkmatter/docs/topics/boolean-conditional-logic.md`
  updated with precedence table, mode split, mixed-precedence guidance,
  fallback-vs-OR distinction, and `And/Or` short-circuit wording.

## Test coverage matrix (against tech design §Tests)

| Tech design item | Covered? | Notes |
|---|---|---|
| Lexer: `||` in condition mode → `OrOr` | Yes | `lexer.rs::condition_mode_double_pipe_is_or_or` |
| Lexer: `&&` in condition mode → `AndAnd` | Yes | `lexer.rs::condition_mode_double_amp_is_and_and` |
| Lexer: single `|` in condition mode still fallback | Yes | `lexer.rs::condition_mode_single_pipe_stays_fallback` |
| Lexer: single `&` still errors (both modes) | Partial | `lexer.rs::condition_mode_single_amp_still_errors` covers condition mode; interpolation-mode single `&` rejection is implicit but not pinned by an explicit test. |
| Lexer: interpolation-mode `||` → `Pipe` | Yes | `lexer.rs::interpolation_mode_double_pipe_collapses_to_fallback` |
| Lexer: interpolation-mode `&&` errors | Yes | `lexer.rs::interpolation_mode_double_amp_errors` |
| Parser: `a && b` | Yes | `parser.rs::condition_parses_infix_and` |
| Parser: `a || b` | Yes | `parser.rs::condition_parses_infix_or` |
| Parser: `a && b || c` | Yes | `parser.rs::condition_and_binds_tighter_than_or_left` |
| Parser: `a || b && c` | Yes | `parser.rs::condition_and_binds_tighter_than_or_right` |
| Parser: `(a || b) && c` | Yes | `parser.rs::condition_parenthesized_or_then_and` |
| Parser: `a || (b | c)` | Yes | `parser.rs::condition_fallback_inside_or` |
| Parser: `a | b && c` | Yes | `parser.rs::condition_fallback_binds_tighter_than_and` |
| Parser: `plan \|\| "plan.md"` still fallback in interpolation | Yes | `parser.rs::interpolation_double_pipe_still_fallback` |
| Parser: `a && b` still errors in interpolation | Yes | `parser.rs::interpolation_rejects_infix_and` |
| Parser: chained `a && b && c` left-assoc | Yes | `parser.rs::condition_chained_and_left_associative` |
| Parser: chained `a || b || c` left-assoc | Yes | `parser.rs::condition_chained_or_left_associative` |
| Evaluator: infix AND/OR | Yes | Six `infix_*` tests in `conditions.rs` |
| Evaluator: mixed precedence + grouping | Yes | `infix_and_binds_tighter_than_or`, `infix_parenthesized_or_then_and` |
| Evaluator: `And(...)`/`Or(...)` still work | Yes | `legacy_and_or_function_still_works` + `evaluates_and_or` |
| Evaluator: short-circuit infix + function form | Yes | Four `*_short_circuits_*` tests |
| Evaluator: non-short-circuit surfaces errors | Yes | `infix_and_without_short_circuit_propagates_eval_error` |
| Compose: page blocks with `&&` | Yes | `compose/mod.rs::page_block_with_infix_and_true/false` |
| Compose: page blocks with `||` | Yes | `page_block_with_infix_or_one_true/both_false` |
| Compose: transclusion with mixed infix logic | Yes | `transclusion_directive_with_mixed_infix_logic`, `transclusion_skipped_when_infix_condition_false` |
| Compose: fallback + infix in one condition | Yes | `page_block_fallback_mixed_with_infix` |
| Reference graph: infix respected | Yes | `graph.rs::when_infix_and_true_follows_transclusion`, `when_infix_and_false_skips_transclusion`, `when_infix_or_follows_on_either_true` |

## Findings

### Gaps in test coverage (non-blocking)

1. **Parse-error regressions missing.** The tech design called out specific
   new invalid inputs that should be rejected:

   - `a &&` (trailing operator)
   - `&& a` (leading operator)
   - `a ||` (trailing operator)
   - `a | | b` (spaced pipes)

   None of these are directly tested in `parser.rs`'s condition-mode tests.
   The existing interpolation tests cover trailing/leading `|`, but the
   condition-mode equivalents should be pinned so a future parser refactor
   cannot silently start accepting them. Suggested location:
   `parser.rs::condition_mode_logic` with tests like
   `condition_rejects_trailing_and`, `condition_rejects_leading_or`, etc.

2. **Implicit mixed fallback/OR missing.** Only the parenthesized form
   `a || (b | c)` is tested. Add an explicit test that `a || b | c` parses as
   `Or(a, Fallback(b, c))` and `a | b || c` parses as `Or(Fallback(a, b), c)`
   so the published precedence table stays honest.

3. **Ternary with infix in condition slot is untested.** Grammar allows
   `a && b ? x : y` to parse as `Ternary { condition: And(a, b), .. }`. Add
   one parser test to lock this in — the `parse_ternary_branch` routing is
   subtle enough to warrant explicit coverage.

4. **Page-block engine has no direct `&&`/`||` unit test.** All page-block
   coverage for the new operators lives at the compose-integration layer
   (`compose/mod.rs::infix_logic_conditions`). Add one smoke test inside
   `page_blocks/engine.rs::tests` (e.g. `render_block_with_infix_and`) so the
   engine module's own tests exercise the operator surface.

5. **`infix_fallback_inside_or` only exercises the short-circuit path.** It
   short-circuits on `a`, so the fallback rhs never runs. Add a variant where
   `a` is falsy and the parenthesized fallback determines the final value, so
   both the OR path and the fallback path are exercised together.

### Documentation polish (non-blocking)

6. **`interpolation/mod.rs` module docstring is out of date.** Lines 31-37
   still advertise the old 4-step precedence (function calls, comparison,
   fallback, ternary) and don't mention unary `!` or the new condition-mode
   operators. Even though condition mode is scoped behind `parse_condition`,
   the shared module doc should at least point readers at the condition-mode
   precedence (or cross-link to `boolean-conditional-logic.md`).

7. **`parser.rs` top-level docstring mirrors the old precedence.** Lines
   5-26 describe the interpolation grammar and precedence ladder but don't
   mention `parse_condition`, the additional logical-OR/logical-AND levels,
   or the lowering-to-`And`/`Or` strategy. Either extend the existing block
   or add a "## Condition Mode" subsection.

8. **`parse_condition` docstring could embed the precedence ladder.**
   `parser.rs:386-413` explains that infix operators are lowered but stops
   short of showing the full precedence. A one-line summary (fallback > AND >
   OR > ternary) would prevent callers from guessing.

### Ergonomic observations (optional)

9. **`Parser.mode` duplicates `Lexer.mode`.** Both structs carry a
   `ParseMode`, and only `parse_ternary_branch` reads the parser's copy.
   Since `OrOr`/`AndAnd` can never appear in interpolation-mode token
   streams, `parse_ternary_branch` could unconditionally call
   `parse_logical_or` — that path degrades to `parse_fallback` in
   interpolation mode because the OrOr/AndAnd branches are never taken.
   This would remove the parallel `mode` field without changing behavior,
   at the cost of a slightly less explicit mode boundary. Keep as-is if the
   self-documenting split is preferred; noting for future cleanup.

10. **Lexer `mode()` accessor is unused outside tests.** If it's only useful
    for debugging, consider gating behind `#[cfg(test)]` or removing it.
    Small API hygiene point.

11. **Error messaging for `&&` in interpolation mode is generic.** Tech
    design flagged this explicitly as acceptable for v1. Leaving the
    tailored "`&&` is only supported in `when` conditions" message as
    follow-up work is consistent with the design's call.

### Correctness / no-go items

None. Every goal from the tech design is honored, no non-goal has been
violated, `{{ plan || "plan.md" }}` continues to behave as fallback sugar,
and legacy `And(...)`/`Or(...)` continue to work. No broken or incomplete
features were found.

## Suggested follow-up (ordered by value)

1. Add the missing condition-mode parse-error tests (finding 1).
2. Update the `interpolation/mod.rs` + `parser.rs` top docstrings to mention
   condition mode (findings 6, 7).
3. Add the three precedence-pinning parser tests (findings 2, 3).
4. Strengthen `infix_fallback_inside_or` with a non-short-circuit variant
   (finding 5).
5. Add one engine-level page-block test (finding 4).
6. Optional ergonomic cleanups (findings 9, 10, 11).

None of these should block merging.
