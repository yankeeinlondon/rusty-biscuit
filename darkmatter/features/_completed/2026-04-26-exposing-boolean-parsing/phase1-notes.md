# Phase 1 Implementation Notes: Baseline and Compatibility Map

## Test Baseline

All tests pass before any changes:

- `cargo test -p darkmatter conditions` → **53 passed, 0 failed**
- `cargo test -p darkmatter interpolation` → **345 passed, 0 failed**  
- `cargo test -p darkmatter --doc` → **143 passed, 0 failed, 2 ignored**
- `cargo clippy -p darkmatter --all-targets -- -D warnings` → **clean**

## Public and Internal Entry Points

### Core Condition Evaluation
- `darkmatter/lib/src/markdown/compose/conditions.rs`
  - `pub fn evaluate_condition(expr, state, line) -> Result<bool, ConditionError>`
  - `pub enum ConditionError` (implements `BlockError`)
  - Private helpers: `eval_expr`, `eval_function`, `is_truthy`, `to_number`, `to_number_coerce`, `scalar_string`

### Interpolation Expression System
- `darkmatter/lib/src/markdown/compose/interpolation/ast.rs` — `Expr` AST nodes
- `darkmatter/lib/src/markdown/compose/interpolation/lexer.rs` — `Lexer`, `Token`, `ComparisonOp`, `ParseMode`
- `darkmatter/lib/src/markdown/compose/interpolation/parser.rs` — `Parser`, `ParseError`, `parse()`, `parse_condition()`
- `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs` — `Evaluator`, `InterpolationLookup`, `EvalResult`, `EvalValue`
- `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs` — `interpolate_text()`, `ScanMode`

### State Management
- `darkmatter/lib/src/markdown/compose/state.rs` — `EffectiveState`, `EffectiveStateBuilder`
  - `EffectiveState` implements `InterpolationLookup`

### Frontmatter Interpolation
- `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs`
  - `FrontmatterSeedState` implements `InterpolationLookup`

### Call Sites
1. **Transclusion** (`compose/mod.rs:1124`) — `transclusion::evaluate_condition(expr, state, line)` for `when=` on `::file` directives
2. **Page Blocks** (`page_blocks/engine.rs:32,75`) — `conditions::evaluate_condition(expr, state, line)` for `when=` on `::block` directives
3. **Shell Expansion Discovery** (`shell_expansion/discovery.rs:230`) — `transclusion::evaluate_condition(expr, &state, line)` for filtering transclusions during shell command discovery
4. **Reference Graph** (`reference/graph.rs:291`) — `conditions::evaluate_condition(when_expr, &effective_state, line)` for graph building

## Existing Exports

### From `compose/mod.rs`
```rust
pub mod conditions;
pub mod interpolation;
pub use state::{EffectiveState, EffectiveStateBuilder};
```

### From `compose/interpolation/mod.rs`
```rust
pub use ast::Expr;
pub use evaluator::{EvalResult, EvalValue, Evaluator, InterpolationLookup};
pub use lexer::{ComparisonOp, ExpressionFinder, ExpressionLocation, Lexer, LexerError, ParseMode, Token};
pub use parser::{ParseError, Parser, parse, parse_condition};
pub(crate) use rewrite::{ScanMode, interpolate_text};
```

## Current Behavior Captured from Tests

### Truthiness
| Type | Truthy | Falsy |
|------|--------|-------|
| Null | — | always |
| Bool | `true` | `false` |
| Number | non-zero | `0`, `0.0` |
| String | non-empty | `""` |
| Array | non-empty | empty |
| Object | non-empty | empty |

### Missing-Value Equality
- `missing_a == missing_b` → `false` (both null)
- `missing_a != missing_b` → `false` (both null)
- `defined == missing` → `false`
- `defined != missing` → `true`

### Numeric Coercion
- Non-numeric values coerce to `0.0` in comparisons (`name >= 0` where `name="Alice"` → `true`)
- String numbers parse correctly (`"10" > 5` → `true`)

### Interpolation Fallback (`||`)
- Primary truthy → returns primary
- Primary falsy (empty, null, false, 0) → evaluates and returns fallback
- Chained: `a || b || c` → first truthy wins

### Condition Short-Circuit (`&&` / `||`)
- Infix `&&` and `||` lowered to `And(...)` and `Or(...)` function calls in AST
- `And` short-circuits on first falsy argument
- `Or` short-circuits on first truthy argument
- Function-form `And(a, b)` and `Or(a, b)` also short-circuit
- Precedence: `&&` binds tighter than `||`

### Helper Functions
| Function | Behavior |
|----------|----------|
| `And(a, b, ...)` | true if all arguments truthy |
| `Or(a, b, ...)` | true if any argument truthy |
| `HasKey(obj, key)` | true if object contains key |
| `Contains(haystack, needle)` | true if array/obj contains value, or string contains substring |
| `Length(x)` | string chars, array len, object keys, number digits, null→0, bool→0 |
| `number(x, default=0)` | parse as f64, fallback to default |
| `round(x, default=0)` | round to nearest i64, fallback to default |

### Lookup Behavior
- `env.*` → system environment variables (via `ComposeContext.env()`)
- `ctx.*` → runtime context (today, year, etc.) via `ComposeContext`
- Unprefixed → frontmatter/state data first, then fallback to `ctx.*`
- Nested paths supported: `user.name`, `user.address.city`

## Duplicated Tests to Move

`transclusion/conditions.rs` (lines 45–174) contains **exact duplicates** of the core condition tests in `conditions.rs` (lines 315–549). These tests should be removed from `transclusion/conditions.rs` after extraction since they test the same underlying evaluator logic.

Duplicated test names:
- `evaluates_unary_not`
- `evaluates_has_key`
- `evaluates_and_or`
- `numeric_comparison_coerces_non_numeric_to_zero`
- `null_equal_null_is_false`
- `null_not_equal_null_is_false`
- `defined_equal_null_is_false`
- `defined_not_equal_null_is_true`
- `equality_with_string_literal`
- `equality_with_single_quoted_string`
- `env_equality_with_string_literal`
- `unset_env_equality_with_string_literal_is_false`
- `mutual_exclusion_pattern`
- `mutual_exclusion_pattern_unset`

## Parser Mode Split

- **Interpolation mode** (`ParseMode::Interpolation`): `||` → `Token::Pipe` (fallback operator)
- **Condition mode** (`ParseMode::Condition`): `||` → `Token::OrOr` (logical OR), `&&` → `Token::AndAnd` (logical AND)
- Infix operators in condition mode are lowered to `FunctionCall` AST nodes (`And(...)`, `Or(...)`) so the evaluator does not need separate AST variants
