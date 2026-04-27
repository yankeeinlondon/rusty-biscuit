# Boolean Conditional Logic

Darkmatter relies on boolean logic expressions in various parts of Darkmatter:

- the `when="..."` clause of [page blocks](../inline/page-blocks.md), [transclusion](../transclusion/block-transclusion.md), and in all other directives which provide a _conditional_ feature.
- beyond directives with a `when` parameter, we can also use 

uses a shared condition evaluator for `when="..."` expressions. That evaluator powers:

- page blocks via `::block when="..."`
- block transclusion via `::file`, `::code`, and `::url` directives with `when="..."`
- reference-graph conditional extraction

This means page blocks, transclusion directives, and reference-graph traversal all support the same condition syntax, the same functions, and the same truthiness rules.

## Where Conditions Read Values From

Conditions evaluate against Darkmatter's effective state. That state can include:

- frontmatter values from the current document
- inherited state passed from parent documents during recursive composition
- runtime context values under `ctx.*`
- environment variables under `env.*`

Examples:

```md
::block when="draft"
This renders when `draft` is truthy.
::end-block

::file ./internal.md when="ctx.repo == 'darkmatter'"

::file ./agent-notes.md when="env.AGENT == 'claude'"
```

## Variable Resolution

Supported variable forms:

- simple keys: `draft`
- nested keys: `user.role`
- context variables: `ctx.today`, `ctx.repo`, `ctx.current_package`
    - see [context variables](./context-variables.md) for more details
- environment keys: `env.AGENT`, `env.HOME`

One important detail: 

- when an unprefixed key is not found in frontmatter or inherited state, Darkmatter falls back to `ctx.<key>`. That means `repo` can resolve to `ctx.repo`.

## Truthiness Rules

The final result of a condition is converted to a boolean using these rules:

| Value | Truthy? |
|-------|---------|
| `null` / missing | false |
| `false` | false |
| numeric `0` | false |
| empty string `""` | false |
| empty array `[]` | false |
| empty object `{}` | false |
| any other value | true |

This has a few consequences:

- `env.AGENT` is true when the environment variable exists and is non-empty
- `"false"` is still truthy because it is a non-empty string
- `Length(tags)` can be used directly as a condition because non-zero numbers are truthy

## Literals and Grouping

Conditions support:

- string literals with either double or single quotes: `"claude"`, `'claude'`
- numeric literals: `0`, `1`, `3.14`
- parentheses for grouping: `(env.AGENT == "claude" || env.AGENT == "opencode")`

## Operator Precedence

From highest precedence to lowest:

1. function calls
2. unary `!`
3. comparisons: `==`, `!=`, `>`, `>=`, `<`
4. logical AND: `&&`
5. logical OR: `||`
6. ternary: `? :`

`&&` binds tighter than `||`. Use parentheses whenever the desired grouping is not obvious.

There is no `<=` operator today. Express that as `!(x > y)` or by flipping the comparison.

## Interpolation vs. Condition Mode

Darkmatter parses `{{ ... }}` interpolation expressions and `when="..."` condition expressions with the same parser but in two different modes. The operator set differs between modes:

| Surface | `||` meaning |
| --- | --- |
| `{{ ... }}` | fallback, first truthy value wins |
| `when="..."` | logical OR, returns a boolean |

That means:

- `when="a || b"` is logical OR and evaluates to a boolean
- `{{ a || "default" }}` is fallback sugar and expands to the first truthy value
- `{{ a && b }}` is still rejected at parse time
- `when="a && b"` is logical AND

The function-call forms `And(...)` and `Or(...)` are supported in both modes and remain valid aliases for the infix operators in condition mode.

## Comparison Operators

Supported comparisons:

- `==`
- `!=`
- `>`
- `>=`
- `<`

### Equality and Inequality

Equality-style comparisons compare scalar string representations.

Examples:

```md
::block when="stage == 'draft'"
Draft-only content
::end-block

::file ./public.md when="audience != 'internal'"
```

Important edge cases:

- if both sides are missing, `a == b` is false
- if both sides are missing, `a != b` is also false
- if one side is defined and the other is missing, `==` is false and `!=` is true

### Numeric Comparisons

For `>`, `>=`, and `<`, both sides are coerced to numbers.

- numbers stay numbers
- numeric strings are parsed
- `true` becomes `1`
- `false` becomes `0`
- `null`, arrays, objects, and non-numeric strings become `0`

Example:

```md
::file ./large.md when="Length(items) > 3"
```

Be careful with non-numeric strings:

```md
name >= 0
```

This evaluates as `0 >= 0` when `name` is a non-numeric string, so it is `true`.

## Unary Logic

### Truthy Check

Using a value by itself checks whether it is truthy:

```md
::block when="draft"
Visible when `draft` is truthy
::end-block
```

### Negation

Prefix `!` negates truthiness:

```md
::file ./default.md when="!env.AGENT"
```

This is a common pattern for "render a default when no environment variable is set".

## Infix Boolean Operators

Condition expressions support infix `&&` and `||`. Both operators are short-circuited:

- `a && b` evaluates `b` only when `a` is truthy; otherwise it returns `false`
- `a || b` evaluates `b` only when `a` is falsy; otherwise it returns `true`

Short-circuit evaluation makes it safe to guard a function call with a cheap predicate:

```md
::file ./contributors.md when="HasKey(release, 'contributors') && Length(release.contributors) > 0"

::block when="env.CI || env.FORCE"
Only renders in CI or when explicitly forced.
::end-block
```

### Mixed Precedence

`&&` binds tighter than `||`, so:

```md
::file ./alert.md when="priority == 'high' && env.PROD || env.FORCE"
```

is parsed as:

```md
(priority == 'high' && env.PROD) || env.FORCE
```

Use parentheses whenever the desired grouping would otherwise be ambiguous:

```md
::file ./alert.md when="priority == 'high' && (env.PROD || env.FORCE)"
```

## Functions

Function names are case-insensitive. `HasKey`, `has_key`, and `haskey` all resolve to the same function.

### `And(a, b, c, ...)`

Returns true only if every argument is truthy. Arguments are evaluated left-to-right with short-circuit behavior matching the infix `&&` operator, so arguments after the first falsy one are not evaluated.

```md
::file ./private.md when="And(user.is_admin, env.INTERNAL_DOCS)"
```

`And(a, b)` is equivalent to `a && b`.

### `Or(a, b, c, ...)`

Returns true if any argument is truthy. Arguments are evaluated left-to-right with short-circuit behavior matching the infix `||` operator, so arguments after the first truthy one are not evaluated.

```md
::file ./llm-notes.md when="Or(env.OPENAI_API_KEY, env.ANTHROPIC_API_KEY)"
```

`Or(a, b)` is equivalent to `a || b`.

### `HasKey(object, key)`

Returns true when the first argument is an object and it contains the named key.

```md
::block when="HasKey(user, 'email')"
Email is present.
::end-block
```

### `Contains(collection, value)`

Checks for containment using behavior based on the first argument's type:

- arrays: compares each element
- objects: compares each value
- strings: substring match
- other scalar values: string containment after coercion

Examples:

```md
::block when="Contains(tags, 'release')"
Release notes
::end-block

::file ./linux.md when="Contains(ctx.os, 'Linux')"
```

### `Length(value)`

Returns:

- string character count
- array length
- object key count
- number character count after string conversion
- `0` for booleans and `null`

Because the result is numeric, you can use it directly or compare it:

```md
::block when="Length(tags)"
Renders when `tags` is non-empty.
::end-block

::block when="Length(tags) > 2"
Renders when at least three tags are present.
::end-block
```

### `number(value, default)`

Parses a value as a number. When parsing fails, it returns the optional default or `0`.

```md
::block when="number(priority, 0) >= 2"
High priority content
::end-block
```

### `round(value, default)`

Parses a value as a number, falls back to the optional default or `0`, then rounds it.

```md
::block when="round(score, 0) > 9"
Top-tier result
::end-block
```

## Fallback and Ternary Expressions

Because the condition evaluator reuses Darkmatter's expression parser, `when=` also supports fallback and ternary expressions.

### Ternary with `? :`

`condition ? then_value : else_value` returns one branch, and the final branch value is then tested for truthiness.

```md
::file ./notes.md when="env.AGENT ? env.AGENT == 'claude' : draft"
```

This is valid, but in practice explicit boolean expressions are usually easier to read.

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

### Require Multiple Conditions

Either infix or function-call form works. Pick whichever reads best for the use case.

```md
::file ./release.md when="release.enabled && env.CI"

::file ./release.md when="And(release.enabled, env.CI)"
```

### Match Any of Several Conditions

```md
::file ./llm-notes.md when="env.OPENAI_API_KEY || env.ANTHROPIC_API_KEY"
```

### Grouped Boolean Expressions

Use parentheses whenever `&&` and `||` appear in the same expression and the precedence is not obvious.

```md
::block when="(draft || preview) && user.is_admin"
Early access content for admins.
::end-block
```

### Prefer Explicit Comparison for String Flags

If a value may contain string data like `"false"` or `"0"`, prefer an explicit comparison over a bare truthy check:

```md
::block when="feature_flag == 'true'"
```

## Errors and Unsupported Syntax

Invalid expressions fail composition with a parse or evaluation error that includes the source line number.

Unsupported or easy-to-misread forms include:

- `a <= b`
- `a && b` inside `{{ ... }}` interpolation (still an error; only `when="..."` accepts it)
- a single `&` (always a lexer error)

Use these instead:

- `!(a > b)` or `b >= a`
- keep `&&` / `||` in `when="..."` conditions
- write `&&` for logical AND in conditions

## Programmatic Evaluation

Darkmatter exposes the condition evaluator as a Rust API so external tools and tests can evaluate boolean expressions without running the full compose pipeline.

### `evaluate_condition_against`

Use [`evaluate_condition_against`](../../lib/src/markdown/compose/conditions.rs) when you have plain JSON data and want a simple boolean result:

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

1. **Top-level and nested paths** against the provided `data`.
2. **`env.*` paths** against the system environment.
3. **`ctx.*` paths** via lazy runtime context capture.
4. **Unprefixed missing keys** fall back to `ctx.*` (same behavior as `EffectiveState`).

Because it takes `&serde_json::Value` and `&Path`, you can use it in tests, build scripts, or other Rust code without constructing a `ComposeContext` or `EffectiveState`.

### Lazy `ctx.*` Resolution

Context capture is **lazy**: only the context groups actually referenced by the expression are captured, and only when evaluation reaches a `ctx.*` lookup. This matters because some context groups perform I/O (e.g. reading git state or querying hardware).

Examples of lazy behavior:

- `false_flag && ctx.repo == "x"` — does **not** capture repo context because `&&` short-circuits on `false`.
- `true_flag || ctx.gpu` — does **not** capture hardware context because `||` short-circuits on `true`.
- `draft == true` — does **not** capture any context because no `ctx.*` key is referenced.

### Error Handling

Both `evaluate_condition` and `evaluate_condition_against` return [`ConditionError`](../../lib/src/markdown/compose/conditions.rs), which implements [`biscuit_terminal::errors::BlockError`](../../lib/src/markdown/compose/conditions.rs). This means parse and evaluation failures can be rendered as rich status blocks in terminal output.

```rust
use darkmatter::markdown::compose::conditions::ConditionError;

let err = ConditionError::Parse {
    expr: "a &&& b".to_string(),
    line: 1,
    message: "Unexpected token".to_string(),
    span: 2..3,
};

// Can be rendered via biscuit_terminal::errors::BlockError
```

## See Also

- [Page Blocks](../inline/page-blocks.md)
- [Block Transclusion](../transclusion/block-transclusion.md)
- [Context Variables](./context-variables.md)
