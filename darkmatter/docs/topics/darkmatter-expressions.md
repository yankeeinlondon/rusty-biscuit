# Darkmatter Expressions

Darkmatter exposes a single expression language used in two surfaces:

- **interpolation** — `{{ ... }}` expansions in document content and frontmatter
- **conditions** — `when="..."` attributes on [page blocks](../inline/page-blocks.md), [transclusion directives](../transclusion/block-transclusion.md), and reference-graph conditional extraction

Both surfaces share the same lexer, parser, and evaluator, so the operator
set, truthiness rules, helper functions, and access semantics described here
are identical everywhere.

> Earlier docs called this "Boolean Conditional Logic". The language is now
> general-purpose and supports arithmetic, member/index access, type
> predicates, and date helpers, so this topic is the authoritative reference
> under its new name.

## Core Expression Engine

The parser, AST, and evaluator live in the [`expression`](../../lib/src/markdown/compose/expression/mod.rs) module and are public API:

- **`expression::Expr`** — the AST type
- **`expression::Lexer`** — tokenizes expression strings
- **`expression::Parser`** — builds an `Expr` from tokens
- **`expression::evaluate`** — evaluates an `Expr` against any [`EvaluationLookup`](#evaluationlookup-trait)
- **`expression::parse_condition`** — parses a string in condition mode (`||` is logical OR, `&&` is logical AND)

## Where Expressions Read Values From

Expressions evaluate against Darkmatter's effective state, which can include:

- frontmatter values from the current document
- inherited state passed from parent documents during recursive composition
- runtime context values under `ctx.*`
- environment variables under `env.*`

When an unprefixed key is not found in frontmatter or inherited state,
Darkmatter falls back to `ctx.<key>`. So `repo` resolves to `ctx.repo`.

## Operator Precedence

From highest to lowest:

1. **Primary / member access** — literals, variables, function calls, `foo.bar`, `foo[0]`, `(expr)`
2. **Unary** — `!`, `-`
3. **Multiplicative** — `*`, `/`, `%`
4. **Additive** — `+`, `-`
5. **Comparison** — `==`, `!=`, `>`, `>=`, `<`, `<=`
6. **Logical AND** — `&&` (condition mode)
7. **Logical OR / Fallback** — `||` (mode-dependent)
8. **Ternary** — `? :`

## Operator Associativity

- All binary operators are **left-associative**
    - `a - b - c` is `(a - b) - c`
    - `a / b / c` is `(a / b) / c`
    - `a || b || c` is `(a || b) || c`
- The ternary operator `? :` is **right-associative**
    - `a ? b : c ? d : e` is `a ? b : (c ? d : e)`

Use parentheses whenever the desired grouping would otherwise be ambiguous.

## Truthiness

The following values are **falsy**:

| Value | Falsy |
|-------|-------|
| `null` / missing | yes |
| `false` | yes |
| `0` | yes |
| `0.0` | yes |
| `""` (empty string) | yes |
| `[]` (empty array) | yes |
| `{}` (empty object) | yes |
| any other value | no |

This makes `||` a true fallback operator: `a || b` evaluates to `a` when `a`
is truthy, and `b` otherwise.

Note that the string `"false"` is still truthy — it is a non-empty string.
Prefer explicit comparisons (`flag == 'true'`) when working with stringly-typed
flags.

## Literals and Grouping

- string literals use double or single quotes: `"claude"`, `'claude'`
- numeric literals: `0`, `1`, `3.14`, `-2`
- boolean literals: `true`, `false`
- parentheses for grouping: `(env.AGENT == "claude" || env.AGENT == "opencode")`

## Variable Access

Supported variable forms:

- simple keys: `draft`
- nested keys: `user.role`
- context variables: `ctx.today`, `ctx.repo`, `ctx.current_package` — see [context variables](./context-variables.md)
- environment keys: `env.AGENT`, `env.HOME`

### Dot Access

Dot access reads named properties of a dictionary: `foo.bar.baz`.

- a non-existent path resolves to `null` (no error)
- dot access on a `null` base (`null.foo`) returns `null`
- numeric dot access (`foo.0`) is **not supported** — use bracket syntax for
  array indexing

### Bracket Access

Bracket access reads array indexes and object keys.

- **arrays**: `foo[0]`, `foo[-1]` (negative indexes count from the end). The
  index expression must evaluate to a number; booleans, strings, objects,
  arrays, and `null` indexes return `null`.
- **objects**: `foo["key"]` (string keys only). The index expression must
  evaluate to a string; numbers, booleans, objects, arrays, and `null` keys
  return `null`.
- **chained**: `items[-1].name`, `config["key"][0]`

Bracket access follows a **null-propagation philosophy**: any invalid bracket
access returns `null` and never errors.

| Form | Result |
|------|--------|
| `items[-1]` on empty array | `null` |
| `items[0]` where `items` is `null` | `null` |
| `items[true]` where `items` is an array | `null` |
| `config["key"]` where `config` is a string | `null` |
| `obj["missing"]` | `null` |
| `obj[0]` where `obj` is an object | `null` |

## Interpolation vs. Condition Mode

The parser supports two modes. The operator set differs:

| Surface | `||` meaning |
| --- | --- |
| `{{ ... }}` (interpolation) | fallback, first truthy value wins |
| `when="..."` (condition) | logical OR, returns a boolean |

Consequences:

- `when="a || b"` is logical OR and evaluates to a boolean
- `{{ a || "default" }}` is fallback sugar and expands to the first truthy value
- `{{ a && b }}` is rejected at parse time
- `when="a && b"` is logical AND

The function-call forms `And(...)` and `Or(...)` are valid in both modes.

## Comparison Operators

All six comparisons are supported: `==`, `!=`, `>`, `>=`, `<`, `<=`.

### Equality and Inequality

`==` and `!=` compare scalar string representations.

```md
::block when="stage == 'draft'"
Draft-only content
::end-block

::file ./public.md when="audience != 'internal'"
```

Edge cases:

- if both sides are missing, `a == b` is false
- if both sides are missing, `a != b` is also false
- if one side is defined and the other is missing, `==` is false and `!=` is true

### Numeric Comparisons

For `>`, `>=`, `<`, and `<=`, both sides are coerced to numbers.

- numbers stay numbers
- numeric strings are parsed
- `true` becomes `1`
- `false` becomes `0`
- `null`, arrays, objects, and non-numeric strings become `0`

```md
::file ./large.md when="Length(items) > 3"
::file ./small.md when="Length(items) <= 3"
```

## Arithmetic Operators

All arithmetic operators (`+`, `-`, `*`, `/`, `%`) require numeric operands,
with one exception: `+` performs **string concatenation** when either operand
is a string.

```md
{{ 5 + 3 }}            ⇒ 8
{{ "count: " + 5 }}    ⇒ "count: 5"
{{ 10 * (1 + 2) }}     ⇒ 30
{{ 7 % 3 }}            ⇒ 1
```

### Errors

Arithmetic errors fail composition:

- **Division by zero** — `x / 0` and `x % 0` raise an error
- **Non-numeric operands** — using `-`, `*`, `/`, or `%` on `null`, booleans,
  arrays, or objects raises an error; same for `+` when neither side is a string

### Remainder Semantics

`%` follows C-style semantics: the sign of `a % b` follows the sign of `a`
(the dividend).

```md
{{ -5 % 3 }}    ⇒ -2
{{  5 % -3 }}   ⇒ 2
```

## Unary Operators

- `!x` — boolean negation (`!truthy ⇒ false`, `!falsy ⇒ true`)
- `-x` — numeric negation; `-null` is `null`; `-"hi"` is an error

## Functions

Function names are **case-insensitive**: `HasKey`, `has_key`, and `haskey` all
resolve to the same function.

### Logical Helpers

- `And(a, b, c, ...)` — all arguments truthy; left-to-right short-circuit
- `Or(a, b, c, ...)` — any argument truthy; left-to-right short-circuit
- `HasKey(object, key)` — `true` when the first argument is an object containing `key`
- `Contains(collection, value)` — substring/array/object/scalar containment

### Length and Numbers

- `Length(value)` — string char count, array length, object key count, number's character count, `0` for `null`/booleans
- `number(value, default?)` — parses as number; falls back to `default` (or `0`)
- `round(value, default?)` — rounds the parsed number to an integer

### Math

- `min(a, b)` — minimum of two numbers
- `max(a, b)` — maximum of two numbers
- `abs(x)` — absolute value

Math helpers require numeric arguments. Booleans, strings, arrays, objects,
and `null` all produce a type-mismatch error (`null` propagates to `null` when
null-safety applies — see [Function Contracts](#function-contracts)).

### Type Predicates

- `IsString(x)`, `IsNumber(x)`, `IsArray(x)`, `IsNull(x)`, `IsObject(x)`
- `IsEmpty(x)` — `true` for `null`, `""`, `[]`, `{}`; `false` for numbers (including `0`), booleans, and non-empty containers

### Collection Helpers

- `first(x)` — first element of array `x`, or `null` if empty
- `last(x)` — last element of array `x`, or `null` if empty

### String Predicates

- `StartsWith(x, find)` — case-sensitive prefix test
- `EndsWith(x, find)` — case-sensitive suffix test

### String Mutations

- `Lower(x)`, `Upper(x)`, `Capitalize(x)`
- `KebabCase(x)`, `SnakeCase(x)`, `CamelCase(x)`, `PascalCase(x)`, `TitleCase(x)`

### Date Validators

Strict format validators (strings only, exact format required):

- `IsDate(x)` — `YYYY-MM-DD`
- `IsDateUtc(x)` — same format
- `IsDateTime(x)` — ISO 8601 datetime
- `IsDateTimeUtc(x)` — same format

Relative validators (accept date *and* datetime strings):

- Local: `IsToday(x)`, `IsYesterday(x)`, `IsTomorrow(x)`, `IsThisMonth(x)`, `IsThisYear(x)`
- UTC:   `IsTodayUtc(x)`, `IsYesterdayUtc(x)`, `IsTomorrowUtc(x)`, `IsThisMonthUtc(x)`, `IsThisYearUtc(x)`

All return `false` for non-string inputs and unparseable strings.

### Function Contracts

All functions added in the expression-syntax expansion follow a consistent
null-safety + type-mismatch contract:

- **Null argument propagation** — if any argument is `null`, the function returns `null`
- **Type-mismatch error** — if any argument has the wrong type for the function's domain, the function returns an error

This applies to:

- math: `min`, `max`, `abs`
- collections: `first`, `last`
- string predicates: `StartsWith`, `EndsWith`
- string mutations: `Lower`, `Upper`, `Capitalize`, `KebabCase`, `CamelCase`, `PascalCase`, `SnakeCase`, `TitleCase`

`IsString`/`IsNumber`/`IsArray`/`IsNull`/`IsObject`/`IsEmpty` are inspecting
predicates and never error or null-propagate; they always return a boolean.

## Null Propagation Summary

| Operation | Behavior |
|-----------|----------|
| Dot access on `null` / missing path | `null` |
| Numeric dot access (`foo.0`) | parse error / unsupported |
| Bracket out-of-bounds | `null` |
| Bracket on `null` base | `null` |
| Bracket key on non-collection | `null` |
| Negative index from the end | element or `null` if outside range |

## Timezone & Date Behavior

- The local system timezone is detected using the `sniff` library (already a
  Darkmatter dependency).
- Default behavior for date/datetime operators uses the **local timezone**.
- UTC variants (`*Utc`) are provided for every date/datetime operator.
- Datetime values with **no offset**:
    - treated as **local time** when using the base (non-UTC) variant
    - treated as **UTC** when using the UTC variant

### Date Validator Input Contracts

**Strict format validators** (`IsDate`, `IsDateTime`, and UTC variants):

- accept **strings only**
- return `false` for non-string inputs, including `null`
- return `false` for strings that do not match the expected exact format

**Relative validators** (`IsToday`, `IsYesterday`, `IsTomorrow`, `IsThisMonth`, `IsThisYear` plus UTC variants):

- accept both **date** and **datetime** strings
- when given a datetime string, extract the date portion for comparison
- return `false` on `null` or any invalid input
- use the operator's timezone semantics (local or UTC) for the reference date

## Programmatic Evaluation

Darkmatter exposes the condition evaluator as a Rust API so external tools and
tests can evaluate expressions without running the full compose pipeline.

### `evaluate_condition_against`

Use [`evaluate_condition_against`](../../lib/src/markdown/compose/conditions.rs)
when you have plain JSON data and want a simple boolean result:

```rust
use darkmatter::markdown::compose::conditions::evaluate_condition_against;
use serde_json::json;
use std::path::Path;

let data = json!({ "draft": true, "audience": "internal" });
let result = evaluate_condition_against(
    "draft && audience == 'internal'",
    &data,
    Path::new("."),
).unwrap();
assert!(result);
```

This shortcut resolves variables in the same order as the compose pipeline:

1. Top-level and nested paths against the provided `data`.
2. `env.*` against the system environment.
3. `ctx.*` via lazy runtime context capture.
4. Unprefixed missing keys fall back to `ctx.*` (same as `EffectiveState`).

### `evaluate_condition`

Use [`evaluate_condition`](../../lib/src/markdown/compose/conditions.rs) when
you are already inside the compose pipeline and have an `EffectiveState`.

### `EvaluationLookup` Trait

For custom evaluation backends, implement
[`EvaluationLookup`](../../lib/src/markdown/compose/expression/mod.rs) and call
`expression::evaluate` directly:

```rust
use darkmatter::markdown::compose::expression::{EvaluationLookup, evaluate, Expr};
use serde_json::{Value, json};
use std::collections::HashMap;

struct SimpleLookup {
    data: HashMap<String, Value>,
}

impl EvaluationLookup for SimpleLookup {
    fn get(&self, path: &str) -> Option<Value> {
        self.data.get(path).cloned()
    }
}

let lookup = SimpleLookup {
    data: [("name".to_string(), json!("Alice"))].into(),
};
let expr = Expr::Variable("name".to_string());
assert_eq!(evaluate(&expr, &lookup).unwrap(), json!("Alice"));
```

### Lazy `ctx.*` Resolution

Context capture is **lazy**: only the context groups actually referenced by
the expression are captured, and only when evaluation reaches a `ctx.*`
lookup. This matters because some context groups perform I/O (e.g. reading
git state or querying hardware).

Examples of lazy behavior:

- `false_flag && ctx.repo == "x"` — repo context is not captured (`&&` short-circuits)
- `true_flag || ctx.gpu` — hardware context is not captured (`||` short-circuits)
- `draft == true` — no `ctx.*` referenced, no context captured

### Error Handling

Both `evaluate_condition` and `evaluate_condition_against` return
[`ConditionError`](../../lib/src/markdown/compose/conditions.rs), which
implements
[`biscuit_terminal::errors::BlockError`](../../lib/src/markdown/compose/conditions.rs).
Parse and evaluation failures render as rich status blocks in terminal
output.

## Common Patterns

### Mutually Exclusive Includes

```md
::file ./claude.md when="env.AGENT == 'claude'"
::file ./opencode.md when="env.AGENT == 'opencode'"
::file ./default.md when="!env.AGENT"
```

### Gate Content on a Nested Property

```md
::block when="user.role == 'admin'"
Admin-only details
::end-block
```

### Indexed Access in Conditions

```md
::file ./first.md when="items[0] == 'release'"
::file ./latest.md when="releases[-1].published"
```

### Type Predicates

```md
::block when="IsArray(tags) && Length(tags) > 0"
Has tags
::end-block

::block when="IsEmpty(tags)"
No tags
::end-block
```

### String Mutations Inside Interpolation

```md
{{ KebabCase(title) }}      → "my-document"
{{ Upper(env.AGENT) }}      → "CLAUDE"
{{ StartsWith(slug, "x-") ? "experimental" : "stable" }}
```

### Date Gates

```md
::block when="IsToday(published)"
Published today!
::end-block

::block when="IsThisMonth(published)"
Recent
::end-block
```

### Grouped Boolean Expressions

Use parentheses whenever `&&` and `||` appear in the same expression and the
precedence is not obvious.

```md
::block when="(draft || preview) && user.is_admin"
Early access content for admins.
::end-block
```

## Errors and Unsupported Syntax

Invalid expressions fail composition with a parse or evaluation error that
includes the source line number.

Unsupported or easy-to-misread forms:

- `a && b` inside `{{ ... }}` interpolation — only `when="..."` accepts it
- a single `&` (always a lexer error)
- numeric dot access like `foo.0` — use `foo[0]` instead

## See Also

- [Page Blocks](../inline/page-blocks.md)
- [Block Transclusion](../transclusion/block-transclusion.md)
- [Context Variables](./context-variables.md)
