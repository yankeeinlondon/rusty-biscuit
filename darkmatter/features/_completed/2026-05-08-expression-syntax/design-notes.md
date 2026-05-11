---
phase: 1
status: decisions-locked
created: 2026-05-09
---

# Design Notes — Darkmatter Expression Syntax

These notes record the semantic decisions taken in Phase 1 of the
[execution plan](./plan.md) before any grammar or evaluator changes land in
Phase 2+. They are intended as the authoritative summary for the
implementation PR description.

## Package Targets

Source-of-truth list from `cargo metadata --no-deps --format-version 1`:

- `darkmatter` (lib, in `darkmatter/lib/`)
- `darkmatter-cli` (CLI, in `darkmatter/cli/`)

`just test` and `just lint` cover both. Phase 6 also exercises
`cargo test -p darkmatter`.

## Current Expression Shape (Pre-Phase 2)

- `compose::expression::ast::Expr` variants: `Variable`, `StringLiteral`,
  `NumberLiteral`, `BoolLiteral`, `UnaryNot`, `Paren`, `Fallback`, `Ternary`,
  `Comparison`, `FunctionCall`.
- `compose::expression::lexer::Token` variants include `Pipe`, `Question`,
  `Colon`, `LParen`, `RParen`, `Comma`, `StringLiteral`, `NumberLiteral`,
  `BoolLiteral`, `CompOp`, `Bang`, `AndAnd`, `OrOr`, `Eof`.
- `ComparisonOp` covers `Equal`, `NotEqual`, `GreaterThan`,
  `GreaterThanOrEqual`, `LessThan` — `LessThanOrEqual` is missing and is
  the first new lexer/parser addition in Phase 2.
- Parser modes: `ParseMode::Interpolation` (default; `||` becomes
  `Fallback`) and `ParseMode::Condition` (`&&` and `||` lower into
  `FunctionCall { name: "And"/"Or", … }` with short-circuit semantics).
- Public entry points: `expression::parse`, `expression::parse_condition`,
  `expression::evaluate`, `expression::is_truthy`, `expression::scalar_string`,
  plus the `EvaluationLookup` trait.
- Call sites that depend on these entry points:
  `compose::interpolation::evaluator`, `compose::conditions`,
  `compose::transclusion::conditions`, `compose::page_blocks::engine`,
  `compose::transclusion`, `compose::shell_expansion::discovery`, and
  `markdown::reference::graph`. Every consumer goes through `parse_condition` /
  `evaluate` / `is_truthy`, so changes to the evaluator are observed
  uniformly across the pipeline.

## Decision 1 — `||` Representation

**Keep separate `Fallback` and condition-mode `Or(...)` representations.**

- Interpolation `||` returns the first truthy operand's value
  (`{{ color || "unknown" }}` → `"red"` or `"unknown"`).
- Condition `||` returns a boolean and short-circuits.
- Lowering condition-mode `||`/`&&` into `FunctionCall { name: "Or" / "And" }`
  centralizes short-circuit handling in `evaluate_function` and means existing
  `And(…)` / `Or(…)` legacy function-call syntax keeps working unchanged.
- A unified AST node would force every consumer to branch on parse mode,
  duplicating the dispatch logic the existing split already encodes.

**Implication for Phase 2:** No change to `Fallback` / `Or` topology; new
operators slot into the existing precedence ladder.

## Decision 2 — Internal Value Contract

**`evaluate` continues to return `serde_json::Value`. Arrays and objects are
preserved end-to-end; conversion to strings happens only at interpolation
boundaries (`scalar_string`).**

- Required by spec: type predicates (`IsArray`, `IsObject`, `IsEmpty`,
  …) and bracket access (`config["key"]`, `items[-1]`) need the
  un-stringified value at every step of evaluation.
- `scalar_string` already handles array/object → JSON-string conversion at the
  interpolation boundary, so renderable output is unchanged.
- `EvalValue` (in `interpolation::evaluator`) stays a string-flavored
  surface for `eval()` callers; it is constructed from the JSON value via
  `EvalValue::from_json` and continues to coerce arrays/objects to their
  JSON-string form for backward compatibility.

**Implication for Phase 3+:** Bracket access produces real `Value::Array`
elements. New helpers (`first`, `last`, `min`, `max`, `abs`, type predicates,
date validators, string mutations) operate on the JSON value directly and
follow the spec's null-propagation / type-mismatch contract.

## Decision 3 — Timezone Helper Shape

**Reuse the existing `chrono` crate (no new feature flags) plus the
`sniff::os::detect_timezone()` already wired into context capture.**

- Local "today" reference: `chrono::Local::now().date_naive()`.
- UTC "today" reference: `chrono::Utc::now().date_naive()`.
- IANA / abbreviation strings continue to come from
  `sniff::os::detect_timezone()` (used by
  `compose::context::capture::capture_datetime_group`); date-validator helpers
  do not need them, only the relative reference dates.
- No `chrono-tz` dependency is introduced — relative validators only need a
  date for "today" / "this month" / "this year" comparisons; absolute IANA
  timezone math is unnecessary for the spec's behavior.

### Test Injection Strategy

- Implement date helpers as pure functions taking
  `(input: &str, today_local: NaiveDate, today_utc: NaiveDate)` so the
  validator logic is testable with a frozen reference date.
- Production wrappers thin-wrap the helpers and supply
  `Local::now().date_naive()` / `Utc::now().date_naive()`.
- `ComposeContext::fixed_for_testing()` already pins `today` /
  `today_utc` style fields for context-driven tests; new tests that exercise
  the date-helper functions directly use the same fixed reference dates
  (`2024-06-15` local, `2024-06-15` UTC) for parity with existing fixtures.
- Datetime-without-offset behavior is captured by the local vs UTC validator
  pair: the local form treats naïve datetimes as `NaiveDateTime` interpreted
  in `Local`, the UTC form interprets the same naïve datetime as UTC.

## Operator Precedence (Final)

From highest to lowest, matching the spec:

1. Primary / member access — literals, variables, function calls,
   `foo.bar`, `foo[0]`, `(expr)`.
2. Unary `!`.
3. Multiplicative — `*`, `/`, `%`.
4. Additive — `+`, `-`.
5. Comparison — `==`, `!=`, `>`, `>=`, `<`, `<=`.
6. Logical AND — `&&` (condition mode).
7. Logical OR / Fallback — `||` (mode-dependent).
8. Ternary `? :` — right-associative.

## Associativity

- All binary operators are left-associative
  (`a - b - c == (a - b) - c`, `a / b / c == (a / b) / c`,
  `a || b || c == (a || b) || c`).
- Ternary is right-associative
  (`a ? b : c ? d : e == a ? b : (c ? d : e)`).

## Truthiness

Falsy: `null`, `false`, `0`, `0.0`, `""`, `[]`, `{}`. Everything else is truthy.
The existing `is_truthy(Value)` already implements this contract; no change
needed.

## Null Propagation Summary

- **Dot access on null / missing path** → `null` (no error).
- **Numeric dot access** (e.g. `foo.0`) — rejected at parse time per spec.
- **Bracket access** — every invalid form (out-of-bounds, null base,
  string key on non-collection, non-string key on object) → `null`.
- **Negative indexing** — `items[-1]` returns last element, or `null`
  on empty arrays.
- **Function null propagation** — any null argument to `min`, `max`, `abs`,
  `first`, `last`, `StartsWith`, `EndsWith`, or any string mutation returns
  `null`. Type mismatches return an evaluation error.

## Arithmetic Errors (Hard Failures)

- Division by zero (`x / 0`) and remainder by zero (`x % 0`) raise an
  evaluator error with a clear message.
- Non-numeric operands for `-`, `*`, `/`, `%` (and `+` when neither side
  is a string) raise an evaluator error.
- `+` performs string concatenation when either operand is a string;
  otherwise it requires two numeric operands.
- Remainder follows C semantics: the sign of `a % b` follows the sign of
  `a` (`-5 % 3 == -2`).

## Date Validator Contract

- Strict format validators (`IsDate`, `IsDateTime`, and their UTC variants)
  accept strings only; non-string and unparseable inputs return `false`.
- Relative validators (`IsToday`, `IsYesterday`, `IsTomorrow`,
  `IsThisMonth`, `IsThisYear`, plus UTC pairs) accept ISO date or datetime
  strings; everything else returns `false`.
- Datetime strings without an offset are interpreted as **local** time
  for the non-UTC variants and as **UTC** for the `*Utc` variants.

## Documentation Rename

The user-facing topic moves from
`darkmatter/docs/topics/boolean-conditional-logic.md` to
`darkmatter/docs/topics/darkmatter-expressions.md` in Phase 5. References from
page-blocks docs, transclusion docs, and any README must be retargeted.
