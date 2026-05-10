---
phases: 6
created: 2026-05-09
start_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/tests/expression_regression.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/tests/expression_regression.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - darkmatter/lib/tests/expression_regression.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/tests/expression_regression.rs
docs_updated_during_phase_6:
  - darkmatter/docs/topics/darkmatter-expressions.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages:
  - darkmatter
  - darkmatter-cli
---

# Review Plan 1 - Darkmatter Expression Syntax

This plan closes the remaining production-readiness gaps for the Darkmatter expression syntax feature against `spec.md`, with emphasis on observable parser/evaluator behavior, user-facing compose behavior, and regression coverage.

## Phase 1: Rebaseline The Current Implementation

- [x] Confirm the active package targets with `cargo metadata --no-deps --format-version 1` and record the exact test commands for `darkmatter` and `darkmatter-cli` if CLI coverage is needed.
- [x] Read `darkmatter/lib/src/markdown/compose/expression/mod.rs` and identify every caller of the numeric conversion helper used by comparison, arithmetic, bracket indexing, and math helpers.
- [x] Read `darkmatter/lib/src/markdown/compose/expression/functions.rs` and identify the dispatch names and argument contracts for math, collection, string, type predicate, date, and datetime helpers.
- [x] Read `darkmatter/lib/src/markdown/compose/expression/{ast.rs,lexer.rs,parser.rs}` and confirm `<=`, arithmetic operators, dot access, bracket access, precedence, and associativity are already represented as expected.
- [x] Read `darkmatter/lib/tests/expression_regression.rs` and any expression unit tests to map existing coverage for numeric type errors, bracket invalid cases, date helper dispatch, and compose error reporting.
- [x] Validation checkpoint: produce a short implementation note or PR comment listing the exact source files and test gaps to change before editing.

### Phase 1 Validation Checkpoint

**Packages:** `darkmatter`, `darkmatter-cli`

**Numeric conversion helper callers in `mod.rs`:**
- `require_number` — used by `evaluate_binary` (arithmetic: `+`, `-`, `*`, `/`, `%`) and `UnaryMinus`
- `to_number` — used by `evaluate_index` (array bracket indexing), `evaluate_function` (`number()` and `round()` helpers), and comparison operators (via `to_number_coerce`)
- `to_number_coerce` — used by all six comparison operators (`==`, `!=`, `>`, `>=`, `<`, `<=`)

**Function dispatch names and contracts in `functions.rs`:**
- Type predicates: `IsString`, `IsNumber`, `IsArray`, `IsNull`, `IsObject`, `IsEmpty` — 1 arg, no null propagation (return bool directly)
- Math helpers: `min(a,b)`, `max(a,b)`, `abs(x)` — null propagates, type-mismatch errors for non-numbers
- Collection helpers: `first(x)`, `last(x)` — null propagates, type-mismatch for non-arrays
- String predicates: `StartsWith(x,find)`, `EndsWith(x,find)` — null propagates, type-mismatch for non-strings
- String mutations: `Lower`, `Upper`, `Capitalize`, `KebabCase`, `CamelCase`, `PascalCase`, `SnakeCase`, `TitleCase` — null propagates, type-mismatch for non-strings
- Strict date validators: `IsDate`, `IsDateUtc`, `IsDateTime`, `IsDateTimeUtc` — strings only, return false for non-strings
- Relative date validators: `IsToday`, `IsYesterday`, `IsTomorrow`, `IsThisMonth`, `IsThisYear` and UTC variants — accept date/datetime strings, return false for invalid/null

**AST/Lexer/Parser confirmation:**
- `<=` tokenized as `CompOp(LessThanOrEqual)` and parsed in comparisons ✓
- Arithmetic operators `+`, `-`, `*`, `/`, `%` tokenized and parsed with correct precedence ✓
- Dot access (`foo.bar`) folded into `Variable` token; postfix dot after `]`/``/`(` emits `Dot` token and parses as `MemberAccess` ✓
- Bracket access (`foo[0]`, `foo[-1]`, `foo["key"]`) tokenized as `LBracket`/`RBracket` and parsed as `Index` ✓
- Precedence order matches spec: primary > unary > multiplicative > additive > comparison > logical AND > logical OR/fallback > ternary ✓
- All binary operators left-associative; ternary right-associative ✓

**Existing test coverage gaps identified for later phases:**
- No tests for boolean operands in arithmetic (`true + 1`, etc.)
- No tests for non-numeric indexes in bracket access (`items[true]`, `items["0"]`, etc.)
- No tests for non-string keys in object bracket access (`obj[0]`, `obj[true]`, etc.)
- No expression-level tests for date helper dispatch through public parser/evaluator
- No compose error reporting tests for division by zero with `with_fail_fast(true/false)`
- No tests for arithmetic type mismatch in compose pipeline

## Phase 2: Harden Numeric Domain Semantics

- [x] Split numeric conversion into context-specific helpers so arithmetic, array indexes, and math helpers can reject booleans while any legacy comparison or `number()` behavior can remain intentionally scoped.
- [x] Update arithmetic evaluation for `+`, `-`, `*`, `/`, and `%` so `null`, booleans, arrays, and objects produce clear type-mismatch errors unless `+` is doing string concatenation.
- [x] Update `min(a, b)`, `max(a, b)`, and `abs(x)` so booleans and other non-number values are type-mismatch errors, while `null` still follows the spec's null-propagation contract.
- [x] Update array bracket indexing so boolean, string, object, array, and null index expressions return `null` instead of being coerced into numeric indexes.
- [x] Add evaluator unit tests for each arithmetic operator with boolean operands, including `true + 1`, `false - 1`, `true * 2`, `true / 2`, and `true % 2`.
- [x] Add function unit tests proving `min(true, 5)`, `max(false, 5)`, and `abs(true)` error, while `min(null, 5)`, `max(null, 5)`, and `abs(null)` return `null`.
- [x] Add bracket access tests proving `items[true]`, `items[false]`, `items["0"]`, `items[null]`, and `items[{}]` return `null`.
- [x] Validation checkpoint: run focused expression tests covering arithmetic, function dispatch, and bracket access, then confirm no existing comparison tests regressed.

## Phase 3: Harden Object Bracket Access

- [x] Change object bracket access so only `Value::String` index values are treated as object keys.
- [x] Ensure object bracket access returns `null` for numeric, boolean, array, object, and null index values without raising an evaluation error.
- [x] Preserve existing positive behavior for `config["key"]`, missing string keys, nested object access, and chained access like `config["key"][0]`.
- [x] Add evaluator tests for `obj[0]`, `obj[1.5]`, `obj[true]`, `obj[false]`, `obj[null]`, `obj[[]]`, and `obj[{}]` returning `null`.
- [x] Add interpolation regression tests proving invalid object bracket access renders as an empty/null interpolation result or the existing documented null rendering behavior.
- [x] Parallelizable: object bracket tests can be written alongside Phase 2 tests once the current access evaluator shape is known, but the code change should land after numeric helper separation to avoid overlapping edits in the same evaluator block.
- [x] Validation checkpoint: run focused bracket access tests and a targeted compose interpolation test for object and array access.

## Phase 4: Verify User-Facing Date Helper Dispatch

- [x] List every required date and datetime helper name from the spec: `IsDate`, `IsDateUtc`, `IsDateTime`, `IsDateTimeUtc`, `IsToday`, `IsYesterday`, `IsTomorrow`, `IsThisMonth`, `IsThisYear`, and each `*Utc` relative variant.
- [x] Add expression-level tests that parse and evaluate every date helper name through the public expression parser and evaluator, not only through private helper functions.
- [x] Add interpolation compose tests that call representative strict validators and all relative validator names with stable inputs.
- [x] Add condition-mode tests through `evaluate_condition_against` for representative local and UTC date helpers, including at least one true case and one false case.
- [x] Add a page-block or transclusion `when=` regression test using a date helper so the directive path is covered through real compose behavior.
- [x] Keep deterministic date math isolated behind existing injectable helper logic or fixed-date helper tests so tests do not depend on the machine's current date.
- [x] Parallelizable: date dispatch coverage can be added while Phase 2 and Phase 3 code fixes are in progress, provided the tests avoid touching the same helper implementation blocks.
- [x] Validation checkpoint: run date-specific tests and confirm every required helper name is covered by at least one public expression or compose-path test.

## Phase 5: Strengthen Compose Error Reporting Coverage

- [x] Add compose regression tests for default non-fail-fast behavior when interpolation hits division by zero and remainder by zero.
- [x] Add compose regression tests for `ComposeOptions::with_fail_fast(true)` when interpolation hits division by zero and remainder by zero.
- [x] Add compose regression tests for arithmetic type mismatches involving booleans, arrays, and objects in both default warning behavior and fail-fast behavior where the current API supports it.
- [x] Assert the user-visible error or warning text contains the relevant expression and a clear reason such as division by zero, remainder by zero, or non-numeric operand.
- [x] Confirm failed interpolation behavior is intentional and stable: either original expression text is preserved, output is omitted, or composition fails according to the current documented behavior.
- [x] Update error snapshots only if the expected message text changes as a result of making diagnostics clearer.
- [x] Parallelizable: compose error tests can be developed after Phase 2 defines final arithmetic error messages; snapshot review can happen independently after tests are green.
- [x] Validation checkpoint: run the expression regression suite and any error snapshot tests that cover interpolation and compose diagnostics.

## Phase 6: Documentation And Final Verification

- [ ] Review `darkmatter/docs/topics/darkmatter-expressions.md` and ensure numeric domain rules explicitly say booleans are invalid for arithmetic, math helpers, and array indexes.
- [ ] Ensure object bracket documentation says only string keys are supported and all other object index expression types return `null`.
- [ ] Confirm the docs describe strict date validators versus relative validators, local versus UTC behavior, and datetime-without-offset semantics consistently with implemented tests.
- [ ] Search for stale terminology that incorrectly calls all expression syntax "boolean expressions" where the docs now mean general Darkmatter expressions.
- [ ] Run formatting for touched Rust code with the repo's established formatter command.
- [ ] Run focused tests for expression lexer/parser/evaluator, function helpers, interpolation regression, conditions, page blocks, and transclusion behavior.
- [ ] Run `cargo test -p darkmatter`.
- [ ] Run the relevant darkmatter area or root lint command, preferring the existing `just lint` recipe if available.
- [ ] Run the relevant darkmatter area or root build command, preferring the existing `just build` recipe if available.
- [ ] Validation checkpoint: final acceptance requires green focused tests, `cargo test -p darkmatter`, lint/build results, reviewed docs, and a short implementation summary listing any intentionally skipped commands.
