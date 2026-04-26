# Boolean Conditional Logic

Darkmatter uses a shared condition evaluator for `when="..."` expressions. That evaluator powers:

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

One important detail: when an unprefixed key is not found in frontmatter or inherited state, Darkmatter falls back to `ctx.<key>`. That means `repo` can resolve to `ctx.repo`.

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
- parentheses for grouping: `(env.AGENT | "default") == "claude"`

## Operator Precedence

From highest precedence to lowest:

1. function calls
2. unary `!`
3. comparisons: `==`, `!=`, `>`, `>=`, `<`
4. fallback: `|`
5. logical AND: `&&`
6. logical OR: `||`
7. ternary: `? :`

Fallback binds tighter than `&&`, and `&&` binds tighter than `||`. Use parentheses whenever the desired grouping is not obvious.

There is no `<=` operator today. Express that as `!(x > y)` or by flipping the comparison.

## Interpolation vs. Condition Mode

Darkmatter parses `{{ ... }}` interpolation expressions and `when="..."` condition expressions with the same parser but in two different modes. The operator set differs between modes:

| Context | `\|` | `\|\|` | `&&` |
|---------|------|--------|------|
| Interpolation `{{ ... }}` | fallback | fallback (alias of `\|`) | syntax error |
| Condition `when="..."` | fallback | logical OR | logical AND |

That means:

- `when="a || b"` is logical OR and evaluates to a boolean
- `{{ a || "default" }}` remains fallback sugar and expands to the first truthy value
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

### Fallback Mixed with Boolean Logic

Fallback `|` still binds tighter than `&&`. You can pick a default for an operand without changing the surrounding boolean expression:

```md
::file ./notes.md when="(env.AGENT | env.DEFAULT_AGENT) == 'claude' && !draft"
```

Here `env.AGENT | env.DEFAULT_AGENT` resolves first, then the comparison runs, and finally the AND combines with `!draft`.

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

### Fallback with `|`

`a | b` returns `a` when `a` is truthy; otherwise it returns `b`.

```md
::block when="env.AGENT | env.DEFAULT_AGENT"
Some agent value is available.
::end-block
```

In condition mode, `|` is still fallback — only `||` is logical OR.

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

## See Also

- [Page Blocks](../inline/page-blocks.md)
- [Block Transclusion](../transclusion/block-transclusion.md)
- [Context Variables](./context-variables.md)
