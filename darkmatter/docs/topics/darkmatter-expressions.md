# Darkmatter Expressions

Darkmatter exposes a single expression language used in two kinds of surface:

- **interpolation** — `{{ ... }}` expansions in document content and frontmatter
- **conditions** — `when="..."` attributes on [page blocks](../inline/page-blocks.md), [transclusion directives](../transclusion/block-transclusion.md), and reference-graph conditional extraction

Both kinds share the same lexer, parser, and evaluator, so the operator
set, truthiness rules, helper functions, and access semantics described here
are identical everywhere.

### Availability across every surface

The same grammar — including the [read-side functions](#read-side-functions)
and the [`doc.*` namespace](#namespaces) — evaluates identically on every
surface that runs it:

| Surface | Form |
| --- | --- |
| Frontmatter interpolation (pass 1, pre-shell) | `key: "{{ ... }}"` |
| Frontmatter interpolation (pass 2, post-shell) | `key: "{{ ... }}"` |
| `$()` frontmatter shell ternary condition | `key: "$( cond ? a : b )"` |
| `$()` frontmatter shell ternary branch | `key: "$( cond ? a : b )"` |
| Body interpolation | `{{ ... }}` |
| `::block` / `::file` / `::code` conditions | `when="..."` |
| Reference-graph conditional extraction | `when="..."` |
| Public condition API | `evaluate_condition_against(...)` |
| Claudine loop conditions | `until=` / `while=` / `action=` |
| Claudine hook conditions | `when=` |

This is the **availability invariant**: a read-side function or `doc.*`
reference resolves on every surface above. (The historical asymmetry — where
read-side functions worked only in body interpolation — is gone.) The single
documented exception is the `markdown::transform` pipeline, which uses a bare
state and is not in scope.

> Earlier docs called this "Boolean Conditional Logic". The language is now
> general-purpose and supports arithmetic, member/index access, type
> predicates, and date helpers, so this topic is the authoritative reference
> under its new name.

All functions provided here DO NOT mutate state, they only _report_ on state. If you are looking for ways to mutate state then you should go to the [side effects](./side-effects.md) documentation. Most functions report only on values already in scope, but the read-side functions (such as `frontmatter(...)` and `file_exists(...)`) also _read_ other files — and, for some, remote URLs — so they report on more than just the local document.

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
- the whole frontmatter object and its properties under `doc` / `doc.*` (see [Namespaces](#namespaces))
- inherited state passed from parent documents during recursive composition
- runtime context values under `ctx.*`
- environment variables under `env.*`

When an unprefixed key is not found in frontmatter or inherited state,
Darkmatter falls back to `ctx.<key>`. So `repo` resolves to `ctx.repo`. The
reserved `doc` namespace is intercepted **before** this fallback, so bare `doc`
always means the frontmatter object and never falls back to `ctx.doc`.

## Operator Precedence

From highest to lowest:

1. **Primary / member access** — literals, variables, function calls, `foo.bar`, `foo[0]`, `(expr)`
2. **Unary** — `!`, `-`
3. **Multiplicative** — `*`, `/`, `%`
4. **Additive** — `+`, `-`
5. **Comparison** — `==`, `!=`, `>`, `>=`, `<`, `<=`
6. **Logical AND** — `&&`
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

## Namespaces

Three reserved prefixes select a distinct value source. They are intercepted
before ordinary key lookup, so a frontmatter property that happens to share a
namespace name never shadows the namespace.

| Namespace | Resolves to |
| --- | --- |
| `doc` / `doc.*` | the **current** document's frontmatter (this document) |
| `ctx.*` | runtime context (date/time, repo, OS, hardware, …) — see [context variables](./context-variables.md) |
| `env.*` | process environment variables |

### The `doc` namespace

- **Bare `doc`** is the whole root frontmatter object.
- **`doc.<path>`** is a root frontmatter property, with dotted traversal for
  nested values: `doc.build`, `doc.config.retries`.
- A property literally named `doc` is reached as **`doc.doc`** (its nested child
  as `doc.doc.child`).

`doc.*` is available in every expression surface (frontmatter and body
interpolation, `when=` conditions, the `$()` ternary condition/branches, and
claudine loop/hook conditions). It is the explicit, unambiguous form of a bare
property reference — useful when a property name collides with an executable
during [`$()` token resolution](#token-resolution-in--shell-expressions), where
`doc.build` bypasses the executable-first ladder and always reads the property.

`doc.*` is distinct from the [`frontmatter()`](#read-side-functions) function:
`doc.build` reads *this* document's frontmatter, whereas
`frontmatter('other.md')` reads *another* file's frontmatter.

During frontmatter interpolation, `doc.<root>` is dependency-ordered exactly
like the bare `<root>` reference: `b: "{{ doc.a }}"` waits for the templated key
`a`, and `doc.doc` waits for a literal key named `doc`. Bare `doc` is a snapshot
of the currently-resolved frontmatter and contributes no dependency — it does
not wait for every templated key (which would create all-key dependencies or a
self-cycle). To read the complete final frontmatter object, reference `doc` from
body interpolation, `when=`, or another post-frontmatter surface.

> **Breaking change.** Bare `doc` previously resolved to a frontmatter
> *property* named `doc`; it now means the whole object. Existing bare `{{doc}}`
> references that mean the property must migrate to `{{doc.doc}}`.

## Interpolation vs. Condition Mode

The parser supports two modes. Only `||` differs between them; `&&` is logical
AND in both:

| Surface | `||` meaning |
| --- | --- |
| `{{ ... }}` (interpolation) | fallback, first truthy value wins |
| `when="..."` (condition) | logical OR, returns a boolean |

Consequences:

- `when="a || b"` is logical OR and evaluates to a boolean
- `{{ a || "default" }}` is fallback sugar and expands to the first truthy value
- `{{ a && b }}` is logical AND (lowered to `and(a, b)`)
- `when="a && b"` is logical AND

The function-call forms `and(...)` and `or(...)` are valid in both modes.

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
::file ./large.md when="length(items) > 3"
::file ./small.md when="length(items) <= 3"
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

Function names are **case-insensitive**: `has_key`, `HasKey`, and `haskey` all
resolve to the same function.

### Logical Helpers

- `and(a, b, c, ...)` — all arguments truthy; left-to-right short-circuit
- `or(a, b, c, ...)` — any argument truthy; left-to-right short-circuit
- `has_key(object, key)` — `true` when the first argument is an object containing `key`
- `contains(collection, value)` — substring/array/object/scalar containment

### Length and Numbers

- `length(value)` — string char count, array length, object key count, number's character count, `0` for `null`/booleans
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

- `is_string(x)`, `is_number(x)`, `is_array(x)`, `is_null(x)`, `is_object(x)`
- `is_empty(x)` — `true` for `null`, `""`, `[]`, `{}`; `false` for numbers (including `0`), booleans, and non-empty containers
- `is_integer(val)` — `true` only for JSON numbers with no fractional component; never errors and does not null-propagate

### Numeric Predicates

- `is_positive(val)` — `true` when the value coerces to a number greater than zero
- `is_negative(val)` — `true` when the value coerces to a number less than zero

Both predicates use the same coercion as `number()`. `0` is neither positive nor
negative. Non-numeric values (including `null`) return an error rather than
propagating.

### Collection Helpers

- `first(x)` — first element of array `x`, or `null` if empty
- `last(x)` — last element of array `x`, or `null` if empty

### String Predicates

- `starts_with(x, find)` — case-sensitive prefix test
- `ends_with(x, find)` — case-sensitive suffix test

### String Mutations

- `lower(x)`, `upper(x)`, `capitalize(x)`
- `kebab_case(x)`, `snake_case(x)`, `camel_case(x)`, `pascal_case(x)`, `title_case(x)`
- `without_date(string)` — removes valid `YYYY-MM-DD` substrings; invalid dates such as `2026-02-30` are left untouched
- `ensure_leading(var, prefix)` — ensures the string form of `var` starts with `prefix`
- `ensure_trailing(var, postfix)` — ensures the string form of `var` ends with `postfix`

`ensure_leading` and `ensure_trailing` accept strings or numbers. If `var`
already has the prefix/suffix, the original value is returned unchanged
(including its JSON type). When `var` is a number and the result is
representable as a number, a JSON number is returned; otherwise a string is
returned.

```md
{{ ensure_leading("foobar", "foo") }}    ⇒ "foobar"
{{ ensure_leading("bar", "foo") }}       ⇒ "foobar"
{{ ensure_leading(123, 4) }}             ⇒ 4123
{{ ensure_leading("123", 4) }}           ⇒ "4123"
```

### Date Validators

Strict format validators (strings only, exact format required):

- `is_date(x)` — `YYYY-MM-DD`
- `is_date_utc(x)` — same format
- `is_date_time(x)` — ISO 8601 datetime (also accepted as `is_datetime(x)`)
- `is_date_time_utc(x)` — same format (also accepted as `is_datetime_utc(x)`)

Relative validators (accept date *and* datetime strings):

- Local: `is_today(x)`, `is_yesterday(x)`, `is_tomorrow(x)`, `is_this_month(x)`, `is_this_year(x)`
- UTC:   `is_today_utc(x)`, `is_yesterday_utc(x)`, `is_tomorrow_utc(x)`, `is_this_month_utc(x)`, `is_this_year_utc(x)`

All return `false` for non-string inputs and unparseable strings.

### Date Formatting

`date(iso, format)` reformats an ISO date or datetime string into a named
human format. The date portion is extracted from datetime inputs.

Supported format tokens (canonical name plus aliases):

| Format | Alias | Example output |
|--------|-------|----------------|
| `MMMM Do` | `short` | `July 12th` |
| `MMMM Do [YYYY]` | `short-optional` | `July 12th` (current year) / `July 12th 1999` |
| `MMMM Do YYYY` | | `July 12th 2026` |
| `D MMMM [YYYY]` | | `12 July` (current year) / `12 July 1999` |
| `D MMMM YYYY` | | `12 July 2021` |
| `ddd, MMMM Do, YYYY` | `long` | `Mon, July 12th, 2021` |

The `[YYYY]` token includes the year only when it differs from the current
year. Invalid ISO input or an unknown format token returns an error; a `null`
argument propagates as `null`.

### Rendering

- `terminal(string)` — renders the input as **Prose markup** and returns the
  resulting terminal string, including ANSI SGR sequences. The input is markup,
  not literal text, so angle brackets that should appear literally must be
  escaped before calling. Rendering uses deterministic, non-interactive
  terminal settings and does not probe the live terminal. `null` propagates.

### Read-Side Functions

Read-side functions report on the filesystem — and, for some, remote URLs. They
are pure in the sense that they mutate no state, but they require a
**resolution context** (a document-relative base directory) to resolve their
path arguments. The context is supplied automatically on every
[surface](#availability-across-every-surface).

All path arguments are resolved through the shared rules below. With the
exception of `file_exists`, read-side functions do not check whether a local
path exists; they operate on the resolved path shape.

#### Shared Path Rules

- Paths are resolved through `FileReference` plus the document's magic paths,
  package paths, and git-root fallbacks.
- Output paths use `/` as the separator, regardless of platform.
- Missing local files are generally **not** an error for path helpers; existence
  is checked only when the operation genuinely needs it.
- HTTP(S) URL strings are rejected by the path helpers. URL support for the
  document-reading functions (`file_exists`, `frontmatter`, `markdown_title`,
  `markdown_body_empty`, `validate_schema`) is available only in **body
  interpolation**, where a remote runtime exists.
- `absolute` and `relative` are local-only path transforms; they never perform
  remote egress.

#### Filesystem Helpers

| Function | Reads | Remote URL arg? |
| --- | --- | --- |
| `file_exists(path)` | whether a file exists | yes (body only) |
| `frontmatter(path)` | another file's frontmatter object | yes (body only) |
| `frontmatter(path, prop)` | a single property from another file's frontmatter | yes (body only) |
| `markdown_title(path)` | another file's first H1 title | yes (body only) |
| `markdown_body_empty(path)` | whether another file's body is empty | yes (body only) |
| `validate_schema(path)` | a file against its declared `$schema` | yes (body only) |
| `validate_schema(path, obj)` | accepted for forward compatibility | yes (body only) |
| `absolute(path)` | the absolute form of a path | **no** |
| `relative(path)` | a path relative to the base dir | **no** |

#### Indexed and Path Helpers

A filename matches the indexed grammar when its stem ends with `-` followed by
one or more digits, where the hyphen is not preceded by another hyphen:
`review-1.md`, `review-100.md`, and `review-001.md` match; `review1.md`,
`review_1.md`, `review-.md`, and `review--1.md` do not.

| Function | Description |
| --- | --- |
| `is_indexed_file(file)` | `true` when the filename stem matches `base-NNN` |
| `file_index(file)` | the parsed index suffix, or `-1` when non-indexed |
| `increment_file_index(file)` | bumps the index; non-indexed files start at `2`; preserves zero-padding width |
| `decrement_file_index(file)` | decrements the index, clamped at `0`; non-indexed files start at `0` |
| `basename(file)` | final component including extension |
| `basename_without_index(file)` | basename with any indexed suffix removed from the stem |
| `dir(file)` | directory portion of the display path |
| `ext(file)` | final extension without the leading dot; `""` when none |
| `parent_dir(file)` | directory segment immediately above the basename |
| `file_trailing(file)` | last directory segment plus basename |
| `dir_leading(file)` | directory path above the last directory segment, dropping the basename and its parent (complement of `file_trailing`) |
| `join(left, right)` | joins two path strings, normalizing separators |

#### Link Helpers

- `link(file)` — emits `[relative](absolute)` for a local file. The one-argument
  form rejects HTTP(S) URLs because a description is required.
- `link(target, desc)` — emits `[desc](destination)`. `target` may be a local
  file reference or an HTTP(S) URL; `desc` must be a string. Link text escapes
  `[` and `]`; destinations that would break CommonMark are wrapped in angle
  brackets or percent-encoded.

#### Skill Helpers

- `has_skill(name)` — `true` when a direct child directory named `name` exists
  in any user-scoped or local-scoped skill root for the executing agent.
- `has_local_skill(name)` — `true` when a direct child directory named `name`
  exists in any local-scoped skill root for the executing agent.

The agent is derived from `ctx.agent` when available, otherwise from the `AGENT`
environment variable. Recognized agent aliases are normalized to `claude`,
`opencode`, or `codex`. Names containing path separators or `..` are rejected.
Missing skill roots return `false`, not an error.

### Function Contracts

All functions added in the expression-syntax expansion follow a consistent
null-safety + type-mismatch contract:

- **Null argument propagation** — if any argument is `null`, the function returns `null`
- **Type-mismatch error** — if any argument has the wrong type for the function's domain, the function returns an error

This applies to:

- math: `min`, `max`, `abs`
- collections: `first`, `last`
- string predicates: `starts_with`, `ends_with`
- string mutations: `lower`, `upper`, `capitalize`, `kebab_case`, `camel_case`, `pascal_case`, `snake_case`, `title_case`, `without_date`, `ensure_leading`, `ensure_trailing`
- rendering: `terminal`
- date formatting: `date`

`is_string`/`is_number`/`is_array`/`is_null`/`is_object`/`is_empty`/`is_integer`
are inspecting predicates and never error or null-propagate; they always return
a boolean. `is_positive` and `is_negative` are coercing predicates: they error
when their argument cannot be coerced to a number (including `null`).

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

**Strict format validators** (`is_date`, `is_date_time`, and UTC variants;
`is_datetime` and `is_datetime_utc` are accepted aliases):

- accept **strings only**
- return `false` for non-string inputs, including `null`
- return `false` for strings that do not match the expected exact format

**Relative validators** (`is_today`, `is_yesterday`, `is_tomorrow`, `is_this_month`, `is_this_year` plus UTC variants):

- accept both **date** and **datetime** strings
- when given a datetime string, extract the date portion for comparison
- return `false` on `null` or any invalid input
- use the operator's timezone semantics (local or UTC) for the reference date

## Token Resolution in `$()` Shell Expressions

A frontmatter `$( … )` value is a **shell expansion**, but the engine and the
shell coexist inside it. A token in **executed position** (a non-ternary
directive body, or a ternary branch) resolves by this precedence ladder:

1. **Quoted** (single/double) → string literal.
2. **Numeric** → number literal.
3. **`true` / `false`** → boolean literal. *Never* a command or a property.
4. **`name(...)`** (trailing parentheses) → an expression function. These are
   **safe functions** — they spawn no process and require **no
   preflight/approval**. No shell executable contains `(` or `)`, so this is an
   unambiguous syntactic distinction.
5. **Bare name / path:**
   - **Path-bearing** (`/usr/bin/doit`, `./doit`) → an **executable**: it exists
     and is executable, or it does not. Never a frontmatter property.
   - **Bare relative** (`doit`):
     - found on `PATH` → an **executable** (a shell command, subject to
       preflight/approval),
     - not found on `PATH` → a **frontmatter property**,
     - property absent → **`null`**.

Because a bare name can resolve to an executable *or* a property depending on
what is installed, use [`doc.<name>`](#namespaces) to force the property
reading: `doc.build` always reads the `build` frontmatter property even when a
`build` executable is on `PATH`.

### Validity rule and the no-command diagnostic

A `$()` is valid only if at least one **executed-position** token is a real
shell command (for a ternary, at least one branch; for a non-ternary, the
directive itself). The **condition** of a ternary is always expression content
and never counts as the command.

A `$()` that contains no shell command — e.g.
`"$( file_exists('x') ? 'a' : 'b' )"`, which is entirely expression-engine
content — is a user error. It is rejected with a targeted diagnostic suggesting
`{{ … }}` interpolation instead.

Intermixing is fully supported when a real command is present:

```yaml
build: "$( file_exists('Cargo.toml') ? cargo build : make )"
```

Here the condition uses the engine (and resolves read-side functions against the
resolution context at the real run), while the chosen branch is a shell
pipeline (`cargo build` or `make`).

### Preflight behavior

Shell-approval preflight does **not** evaluate the expression engine and needs
**no resolution context**: it enumerates **both** ternary branches so the
approved set is a superset of what can run, and nothing executes unapproved. It
performs a read-only `PATH` probe to classify bare names (executable → needs
approval; otherwise property/null → ignored). Safe `name(...)` functions are
excluded by construction. The resolution context is needed only by the **real
run**, so the chosen branch's condition resolves.

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

1. `doc` / `doc.*` against the provided `data` (intercepted first; never falls back to `ctx.doc`).
2. Top-level and nested paths against the provided `data`.
3. `env.*` against the system environment.
4. `ctx.*` via lazy runtime context capture.
5. Unprefixed missing keys fall back to `ctx.*` (same as `EffectiveState`).

The `work_dir` argument supplies the resolution context, so the
[read-side functions](#read-side-functions) (`file_exists`, `absolute`,
`relative`, …) resolve against it — a public-API capability for external
callers.

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
::block when="is_array(tags) && length(tags) > 0"
Has tags
::end-block

::block when="is_empty(tags)"
No tags
::end-block
```

### String Mutations Inside Interpolation

```md
{{ kebab_case(title) }}      → "my-document"
{{ upper(env.AGENT) }}      → "CLAUDE"
{{ starts_with(slug, "x-") ? "experimental" : "stable" }}
```

### Date Gates

```md
::block when="is_today(published)"
Published today!
::end-block

::block when="is_this_month(published)"
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

- a single `&` (always a lexer error)
- numeric dot access like `foo.0` — use `foo[0]` instead

## Authoring a New Expression Function

Expression functions live in
[`expression/functions.rs`](../../lib/src/markdown/compose/expression/functions.rs)
and split into two registries:

- **Pure functions** (`PURE_FUNCTIONS`) — depend only on their arguments. Most
  helpers (`length`, `min`, `kebab_case`, `is_today`, …) are pure. Dispatched by
  `dispatch`, which needs no context.
- **Context-aware / read-side functions** (`FS_FUNCTIONS`) — need a
  [`ResolutionContext`](../../lib/src/markdown/compose/expression/resolve_ctx.rs)
  to resolve path arguments. The [read-side functions](#read-side-functions)
  live here. Dispatched by `dispatch_fs`, which receives the context; `is_fs_function`
  reports membership so the evaluator can emit the "requires a document
  resolution context" error when no context is available.

To add a function:

1. Implement it and register it in the correct slice (`PURE_FUNCTIONS` or
   `FS_FUNCTIONS`).
2. Add a matching descriptor to `EXPRESSION_FUNCTION_DESCRIPTORS` in
   [`catalog.rs`](../../lib/src/markdown/compose/expression/catalog.rs). This is
   **mandatory**: parity tests enforce exact bidirectional set equality between
   the registered functions and the descriptor catalog, so a missing or extra
   descriptor fails the build.

For a read-side function, obtain paths through the `ResolutionContext`
(`base_dir`, magic search paths, optional remote runtime) rather than the
process CWD. Honor remote URL arguments only when the context carries a remote
runtime; in a local-only context (any frontmatter surface) a remote URL
must **fail loudly**, not silently default. Because every surface now supplies a
context, a read-side function either resolves or fails loudly on every surface —
it never leaks an unresolved `{{ … }}` literal.

## See Also

- [Side Effects](./side-effects.md)
- [Page Blocks](../inline/page-blocks.md)
- [Block Transclusion](../transclusion/block-transclusion.md)
- [Context Variables](./context-variables.md)
