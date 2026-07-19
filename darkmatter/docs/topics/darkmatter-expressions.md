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
reference resolves on every surface above. The historical asymmetry — where
read-side functions worked only in body interpolation — is gone; the compose
pipeline and public condition API use the same evaluation contract.

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

## Interpolation Literals

A triple-brace span `{{{ ... }}}` is an **interpolation literal**. It is recognized on every expression-scanning surface but is **inert**: the content is never lexed, parsed, or evaluated, and it produces no diagnostic.

- `{{{ name }}}` composes to the literal text `{{ name }}`.
- `{{{ {{ x }} }}}` composes to `{{ {{ x }} }}` with `x` unevaluated.
- The literal closes at the first subsequent `}}}`; an unclosed `{{{` falls back to legacy `{{` scanning.
- Literals are inert on every scanner consumer: interpolation, DMLS diagnostics, demand-driven context capture, and remote-reference discovery.

Use interpolation literals when documentation needs to display `{{ ... }}` syntax rather than evaluate it. The fenced-code-block alternative remains the way to show literal `{{{ ... }}}` syntax itself.

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

All arithmetic operators (`+`, `-`, `*`, `/`, `%`) accept numbers and numeric
strings. For `+`, a numeric string paired with a number is coerced before
addition. Other string combinations use **string concatenation**, including
two numeric strings.

```md
{{ 5 + 3 }}            ⇒ 8
{{ "2" + 1 }}          ⇒ 3
{{ "2" + "1" }}        ⇒ "21"
{{ "count: " + 5 }}    ⇒ "count: 5"
{{ 10 * (1 + 2) }}     ⇒ 30
{{ 7 % 3 }}            ⇒ 1
```

### Errors

Arithmetic errors fail composition:

- **Division by zero** — `x / 0` and `x % 0` raise an error
- **Non-numeric operands** — using `-`, `*`, `/`, or `%` on `null`, booleans,
  arrays, or objects raises an error

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

The authored source for this table is
[`docs/schemas/expression-functions.yaml`](../schemas/expression-functions.yaml),
projected through `expression_function_descriptors()`. Do not edit the generated
table directly; run `just darkmatter regen-expr-doc` from the repository root to
refresh it after changing the catalog.

<!-- BEGIN GENERATED FUNCTION TABLE -->

| Category | Function | Description | Example |
| --- | --- | --- | --- |
| Type Predicates | `is_string(x)` | Returns true when the value is a string. | `is_string("hello")` ⇒ `true` |
| Type Predicates | `is_number(x)` | Returns true when the value is a number. | `is_number(42)` ⇒ `true` |
| Type Predicates | `is_array(x)` | Returns true when the value is an array. | `is_array(items)` ⇒ `true` |
| Type Predicates | `is_null(x)` | Returns true when the value is null. | `is_null(null)` ⇒ `true` |
| Type Predicates | `is_object(x)` | Returns true when the value is an object. | `is_object(obj)` ⇒ `true` |
| Type Predicates | `is_empty(x)` | Returns true when the value is null, empty string, empty array, or empty object. | `is_empty("")` ⇒ `true` |
| Type Predicates | `is_positive(val)` | Returns true when the coerced value is greater than zero. | `is_positive(5)` ⇒ `true` |
| Type Predicates | `is_negative(val)` | Returns true when the coerced value is less than zero. | `is_negative(-3)` ⇒ `true` |
| Type Predicates | `is_integer(val)` | Returns true when the value is a JSON number with no fractional component. | `is_integer(7)` ⇒ `true` |
| Math | `min(a, b)` | Returns the smaller of two numbers. | `min(2, 5)` ⇒ `2` |
| Math | `max(a, b)` | Returns the larger of two numbers. | `max(2, 5)` ⇒ `5` |
| Math | `abs(x)` | Returns the absolute value of a number. | `abs(-3)` ⇒ `3` |
| Collection | `first(x)` | Returns the first element of an array, or null when empty. | `first(items)` ⇒ `1` |
| Collection | `last(x)` | Returns the last element of an array, or null when empty. | `last(items)` ⇒ `3` |
| String Predicates | `starts_with(x, find)` | Returns true when the string starts with the given prefix (case-sensitive). | `starts_with("hello", "he")` ⇒ `true` |
| String Predicates | `ends_with(x, find)` | Returns true when the string ends with the given suffix (case-sensitive). | `ends_with("hello", "lo")` ⇒ `true` |
| String Mutations | `lower(x)` | Converts a string to lowercase. | `lower("HELLO")` ⇒ `hello` |
| String Mutations | `upper(x)` | Converts a string to uppercase. | `upper("hello")` ⇒ `HELLO` |
| String Mutations | `capitalize(x)` | Capitalizes the first character of a string. | `capitalize("hello")` ⇒ `Hello` |
| String Mutations | `kebab_case(x)` | Converts a string to kebab-case. | `kebab_case("Hello World")` ⇒ `hello-world` |
| String Mutations | `snake_case(x)` | Converts a string to snake_case. | `snake_case("Hello World")` ⇒ `hello_world` |
| String Mutations | `camel_case(x)` | Converts a string to camelCase. | `camel_case("hello world")` ⇒ `helloWorld` |
| String Mutations | `pascal_case(x)` | Converts a string to PascalCase. | `pascal_case("hello world")` ⇒ `HelloWorld` |
| String Mutations | `title_case(x)` | Converts a string to Title Case. | `title_case("hello world")` ⇒ `Hello World` |
| String Mutations | `without_date(string)` | Removes substrings that are real YYYY-MM-DD calendar dates, leaving surrounding text untouched. | `without_date("Note 2024-06-15")` ⇒ `Note ` |
| String Mutations | `ensure_leading(var, prefix)` | Ensures the string form of a value starts with a prefix. | `ensure_leading("world", "hello ")` ⇒ `hello world` |
| String Mutations | `ensure_trailing(var, postfix)` | Ensures the string form of a value ends with a postfix. | `ensure_trailing("hello", " world")` ⇒ `hello world` |
| String Mutations | `replace(x, find, replacement)` | Replaces every literal occurrence of a substring; empty find is a no-op. | `replace("a.b.c", ".", "/")` ⇒ `a/b/c` |
| String Mutations | `replace_first(x, find, replacement)` | Replaces the first literal occurrence of a substring; empty find is a no-op. | `replace_first("a.b.c", ".", "/")` ⇒ `a/b.c` |
| String Mutations | `replace_last(x, find, replacement)` | Replaces the last literal occurrence of a substring; empty find is a no-op. | `replace_last("a.b.c", ".", "/")` ⇒ `a.b/c` |
| Rendering | `terminal(string)` | Renders Prose markup to a terminal string with ANSI SGR sequences. | `terminal("hello")` ⇒ `hello` |
| Date Formatting | `date(iso, fmt)` | Reformats an ISO date/datetime string into a named human format. | `date("2024-06-15", "long")` ⇒ `Sat, June 15th, 2024` |
| Date Validators | `is_date(x)` | Returns true when the string is a valid ISO date (YYYY-MM-DD). | `is_date("2024-06-15")` ⇒ `true` |
| Date Validators | `is_date_utc(x)` | Same as is_date (the format itself is timezone-agnostic). | `is_date_utc("2024-06-15")` ⇒ `true` |
| Date Validators | `is_date_time(x)` | Returns true when the string is a valid ISO datetime. | `is_date_time("2024-06-15T12:30:00")` ⇒ `true` |
| Date Validators | `is_date_time_utc(x)` | Same parse contract as is_date_time. | `is_date_time_utc("2024-06-15T12:30:00Z")` ⇒ `true` |
| Date Validators | `is_today(x)` | Returns true when the date/datetime is today (local). |  |
| Date Validators | `is_today_utc(x)` | Returns true when the date/datetime is today (UTC). |  |
| Date Validators | `is_yesterday(x)` | Returns true when the date/datetime is yesterday (local). |  |
| Date Validators | `is_yesterday_utc(x)` | Returns true when the date/datetime is yesterday (UTC). |  |
| Date Validators | `is_tomorrow(x)` | Returns true when the date/datetime is tomorrow (local). |  |
| Date Validators | `is_tomorrow_utc(x)` | Returns true when the date/datetime is tomorrow (UTC). |  |
| Date Validators | `is_this_month(x)` | Returns true when the date/datetime is in the current month (local). |  |
| Date Validators | `is_this_month_utc(x)` | Returns true when the date/datetime is in the current month (UTC). |  |
| Date Validators | `is_this_year(x)` | Returns true when the date/datetime is in the current year (local). |  |
| Date Validators | `is_this_year_utc(x)` | Returns true when the date/datetime is in the current year (UTC). |  |
| Date Arithmetic | `date_delta(date1, date2, diff)` | Returns true when the two dates are at least the given duration apart, ignoring order (duration like 14d, 2mo, 1 hour). | `date_delta("2024-06-01", "2024-06-20", "14d")` ⇒ `true` |
| Date Arithmetic | `older_than(date1, date2, diff)` | Returns true when date1 is at least the given duration older (earlier) than date2. | `older_than("2024-06-01", "2024-06-20", "14d")` ⇒ `true` |
| Date Arithmetic | `newer_than(date1, date2, diff)` | Returns true when date1 is at least the given duration newer (later) than date2. | `newer_than("2024-06-20", "2024-06-01", "14d")` ⇒ `true` |
| Logical | `and(...)` | Logical AND of all arguments. Short-circuits on first falsy value. | `and(true, true)` ⇒ `true` |
| Logical | `or(...)` | Logical OR of all arguments. Short-circuits on first truthy value. | `or(false, true)` ⇒ `true` |
| Collection | `has_key(obj, key)` | Returns true when the object contains the given key. | `has_key(obj, "a")` ⇒ `true` |
| Collection | `contains(haystack, needle)` | Returns true when haystack contains needle (array, object, or string). | `contains("hello", "ell")` ⇒ `true` |
| Collection | `length(x)` | Returns the length of a string, array, or object. | `length("hello")` ⇒ `5` |
| Type Conversion | `number(x, [default])` | Converts a value to a number, with an optional default. | `number("42")` ⇒ `42` |
| Math | `round(x, [default])` | Rounds a value to the nearest integer, with an optional default. | `round(3.7)` ⇒ `4` |
| Filesystem | `absolute(file)` | Resolves a file path to an absolute path. |  |
| Filesystem | `relative(file)` | Returns a best-effort relative path from the document base directory. | `relative("fixture.md")` ⇒ `fixture.md` |
| Filesystem | `file_exists(file)` | Returns true when the file exists (local or remote URL). | `file_exists("fixture.md")` ⇒ `true` |
| Filesystem | `frontmatter(file)` | Reads the frontmatter of a Markdown file as an object. | `frontmatter("fixture.md")` ⇒ `{"title":"Fixture Title"}` |
| Filesystem | `frontmatter(file, prop)` | Reads the frontmatter of a Markdown file as an object. | `frontmatter("fixture.md", "title")` ⇒ `Fixture Title` |
| Filesystem | `markdown_body_empty(file)` | Returns true when the Markdown body has only whitespace. | `markdown_body_empty("fixture.md")` ⇒ `false` |
| Filesystem | `markdown_title(file)` | Returns the title from frontmatter or the first H1 heading. | `markdown_title("fixture.md")` ⇒ `Fixture Title` |
| Filesystem | `validate_schema(file)` | Validates a Markdown document against its declared schema. | `validate_schema("fixture.md")` ⇒ `true` |
| Filesystem | `validate_schema(file, obj)` | Validates a Markdown document against its declared schema. | `validate_schema("fixture.md", {})` ⇒ `true` |
| Filesystem | `is_indexed_file(file)` | Returns true when the filename stem matches the indexed grammar (base-NNN). | `is_indexed_file("review-1.md")` ⇒ `true` |
| Filesystem | `file_index(file)` | Returns the parsed index suffix, or -1 when non-indexed. | `file_index("review-1.md")` ⇒ `1` |
| Filesystem | `increment_file_index(file)` | Increments the numeric index suffix, preserving zero-padding width. | `increment_file_index("review-1.md")` ⇒ `review-2.md` |
| Filesystem | `decrement_file_index(file)` | Decrements the numeric index suffix, clamped at 0. | `decrement_file_index("review-2.md")` ⇒ `review-1.md` |
| Filesystem | `basename(file)` | Returns the final path component including extension. | `basename("sub/note.md")` ⇒ `note.md` |
| Filesystem | `basename_without_index(file)` | Returns the basename with any indexed suffix removed from the stem. | `basename_without_index("review-1.md")` ⇒ `review.md` |
| Filesystem | `dirname(file)` | Returns the directory portion of the display path. | `dirname("sub/note.md")` ⇒ `sub` |
| Filesystem | `ext(file)` | Returns the final extension without the leading dot. | `ext("sub/note.md")` ⇒ `md` |
| Filesystem | `parent_dir(file)` | Returns the directory segment immediately above the basename. | `parent_dir("sub/note.md")` ⇒ `sub` |
| Filesystem | `file_trailing(file)` | Returns the last directory segment plus the basename. | `file_trailing("sub/note.md")` ⇒ `sub/note.md` |
| Filesystem | `dir_leading(file)` | Returns the directory path above the last directory segment, dropping the basename and its parent (the complement of file_trailing). | `dir_leading("sub/note.md")` ⇒ `` |
| Filesystem | `join(left, right)` | Joins two path strings with normalized separators. | `join("sub", "note.md")` ⇒ `sub/note.md` |
| Filesystem | `link(file)` | Creates a Markdown link to a local file, using its relative path as the link text. |  |
| Filesystem | `link(target, desc)` | Creates a Markdown link to a local file, using its relative path as the link text. |  |
| Filesystem | `has_command(cmd)` | Returns true when the command is found on PATH or is an existing executable absolute path. |  |
| Context | `has_skill(name)` | Returns true when a skill directory exists in a user-scoped or local-scoped skill root. |  |
| Context | `has_local_skill(name)` | Returns true when a skill directory exists in a local-scoped skill root. |  |
| List Formatting | `as_line_separated(list)` | Joins a list into a newline-separated string (the default bare-array rendering). |  |
| List Formatting | `as_csv(list)` | Joins a list into a comma-separated string. | `as_csv(items)` ⇒ `1, 2, 3` |
| List Formatting | `as_tsv(list)` | Joins a list into a tab-separated string. |  |
| List Formatting | `as_space_separated(list)` | Joins a list into a space-separated string. | `as_space_separated(items)` ⇒ `1 2 3` |
| List Formatting | `as_unordered_list(list)` | Renders a list as a Markdown unordered list, auto-nesting nested arrays and object-array shapes as indented sublists. |  |
| List Formatting | `as_ordered_list(list)` | Renders a list as a Markdown ordered list, auto-nesting nested arrays and object-array shapes as indented sublists. |  |
| Filesystem | `find_first_index(file)` | Returns the lowest-indexed existing sibling in the file's directory, with the unindexed base sorting first; returns the input when the family has no existing member. | `find_first_index("review-2.md")` ⇒ `review-1.md` |
| Filesystem | `find_last_index(file)` | Returns the highest-indexed existing sibling in the file's directory; returns the input when the family has no existing member. | `find_last_index("review-1.md")` ⇒ `review-2.md` |
| Git | `predict_conflicts(branch)` | Returns the repository-relative paths that would conflict if the named local branch were merged into the caller's current branch. |  |
| Git | `branch_exists_on_remote()` | Returns whether an exact branch exists in the selected remote's live ref advertisement or authoritative provider API. |  |
| Git | `branch_exists_on_remote(branch)` | Returns whether an exact branch exists in the selected remote's live ref advertisement or authoritative provider API. |  |
| Git | `branch_exists_on_remote(branch, remote)` | Returns whether an exact branch exists in the selected remote's live ref advertisement or authoritative provider API. |  |
| Git | `remote_vendor([remote])` | Returns the canonical provider token for the selected configured remote, probing an ambiguous self-hosted server only when allowlisted. |  |
| Pull Requests | `pr(id)` | Returns one provider pull or merge request in canonical Markdown form. |  |
| Pull Requests | `pr_list(query)` | Queries pull or merge requests with the canonical bounded filter vocabulary. See the [provider query vocabulary](darkmatter-expressions.md#provider-query-vocabulary) for keys, enum values, defaults, and bounds. |  |
| Pull Requests | `pr_list(count)` | Queries pull or merge requests with the canonical bounded filter vocabulary. See the [provider query vocabulary](darkmatter-expressions.md#provider-query-vocabulary) for keys, enum values, defaults, and bounds. |  |
| CI/CD | `cicd(id)` | Returns one provider-addressable CI/CD job in canonical Markdown form. |  |
| CI/CD | `cicd_list(query)` | Queries CI/CD jobs with bounded direct listing or parent-execution traversal. See the [provider query vocabulary](darkmatter-expressions.md#provider-query-vocabulary) for keys, enum values, defaults, and bounds. |  |
| CI/CD | `cicd_list(count)` | Queries CI/CD jobs with bounded direct listing or parent-execution traversal. See the [provider query vocabulary](darkmatter-expressions.md#provider-query-vocabulary) for keys, enum values, defaults, and bounds. |  |
<!-- END GENERATED FUNCTION TABLE -->

### `date()` format tokens

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

### Read-Side Functions

Read-side functions report on the filesystem — and, for some, remote URLs. They
are pure in the sense that they mutate no state, but they require a
**resolution context** (a document-relative base directory) to resolve their
path arguments. The context is supplied automatically on every
[surface](#availability-across-every-surface).

All path arguments are resolved through the shared rules below. With the
exception of `file_exists` and `has_command`, read-side functions do not check
whether a local path exists; they operate on the resolved path shape.

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
| `has_command(cmd)` | whether a command is runnable on the host | **no** |

`has_command(cmd)` is a `PATH`/executable existence probe: it reports whether
`cmd` can be run on the host and **never executes** it, so it needs no command
whitelisting. A bare name (`git`) triggers an OS-native `PATH` search; an
absolute path (`/usr/bin/git`) must both exist and be executable. On Windows the
search honors `PATHEXT`; on Unix the executable bit is required. Symlinked
executables are followed, and directories are rejected.

Unlike the document-reading helpers, `has_command` takes **no** remote URL
argument and does not use the shared path-resolution rules. Two path shapes are
intentionally **not** resolved and always return `false` by design:

- **Tilde** — `~` is not expanded, so `has_command("~/bin/mytool")` is `false`.
- **Relative paths** — `./mytool` and `bin/foo` are not resolved against `PATH`,
  a base directory, or the CWD.

Both follow the never-error contract — they return `false` rather than raising —
and can be addressed later without an API change.

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
| `dirname(file)` | directory portion of the display path |
| `ext(file)` | final extension without the leading dot; `""` when none |
| `parent_dir(file)` | directory segment immediately above the basename |
| `file_trailing(file)` | last directory segment plus basename |
| `dir_leading(file)` | directory path above the last directory segment, dropping the basename and its parent (complement of `file_trailing`) |
| `join(left, right)` | joins two path strings, normalizing separators |

#### Git Helpers

`predict_conflicts(branch)` predicts the unresolved paths produced by merging
the named local branch into the caller's current local branch. The named branch
is incoming (`theirs`); the current branch is the destination (`ours`). The
repository is resolved from the caller or launch-area anchor, not from the
Markdown document's directory.

Prediction uses only the two captured committed branch tips. Staged, unstaged,
untracked, and already-conflicted index state do not affect it, including staged
`.gitattributes`. It performs no fetch, network request, hook, merge driver,
filter, or subprocess, and it does not change HEAD, refs, the index, worktree,
or on-disk object database. A clean merge returns `[]`; missing prerequisites
such as a repository, attached current branch, exact incoming local branch, or
supported merge configuration return an error.

Render predicted paths as a Markdown list:

```md
{{ as_unordered_list(predict_conflicts("feature/api")) }}
```

Branch on the result in an ordinary interpolation:

```md
{{ is_truthy(predict_conflicts("feature/api")) ? "Conflicts need resolution" : "Merge is clean" }}
```

The same function is available in frontmatter interpolation and `$()` ternary
conditions or branches:

```yaml
merge_status: '$( is_truthy(predict_conflicts("feature/api")) ? "conflicted" : "clean" )'
```

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
- string mutations: `lower`, `upper`, `capitalize`, `kebab_case`, `camel_case`, `pascal_case`, `snake_case`, `title_case`, `without_date`, `ensure_leading`, `ensure_trailing`, `replace`, `replace_first`, `replace_last`
- rendering: `terminal`
- date formatting: `date`

`is_string`/`is_number`/`is_array`/`is_null`/`is_object`/`is_empty`/`is_integer`
are inspecting predicates and never error or null-propagate; they always return
a boolean. `is_positive` and `is_negative` are coercing predicates: they error
when their argument cannot be coerced to a number (including `null`).

## Provider Query Vocabulary

`pr_list()` and `cicd_list()` accept a structured query object drawn from a
closed, canonical vocabulary. This section is the authority for that
vocabulary; the catalog descriptions for both functions link here, so editor
hover and completion reach the same tables.

### Shared rules

- **Repository-only scope.** A query is scoped to the repository identified by
  the selected configured remote. Organization, group, workspace, account, and
  all-visible scopes are not available.
- **No provider-native escape hatch.** Only the canonical keys below are
  accepted. Adapter-native query syntax stays internal to Sniff. A canonical
  field the selected provider cannot honor exactly fails with an
  unsupported-filter error naming the field and the provider flavor — it is
  never ignored, approximated, or silently downgraded.
- **Limits.** An omitted `limit` means 20; the hard maximum is 100. A
  non-positive or over-maximum limit is an invalid-query error.
- **Ordering.** Both functions return newest-first by default.
- **Empty results.** A successful query with no matches returns `[]`.
- **Datetimes.** `*_after` / `*_before` values are RFC 3339 / ISO 8601 strings
  (for example `2026-07-13T00:00:00Z`). Both bounds are inclusive; an inverted
  range is an invalid-query error.
- **Validation is pre-network.** Unknown keys, wrong types, invalid enum
  values, inverted ranges, and invalid filter combinations are rejected before
  any provider request.
- **Integer overload.** `pr_list(count)` and `cicd_list(count)` are shorthand
  for the newest `count` items — open pull requests for `pr_list`, jobs of any
  status for `cicd_list`. `count` must be a positive integer and is capped by
  the same hard maximum of 100.

### `pr_list(query)` keys

| Key | Type | Meaning |
|-----|------|---------|
| `remote` | string | Exact configured remote name; preferred remote when absent |
| `state` | string or string[] | Any of `open`, `closed`, `merged`; defaults to `open` |
| `draft` | boolean | Independently select draft/non-draft state |
| `source_branch` | string | Exact source branch |
| `target_branch` | string | Exact destination branch |
| `author` | string | Provider login/username |
| `assignee` | string | Provider login/username |
| `reviewer` | string | Provider login/username |
| `labels` | string[] | Require all listed labels |
| `milestone` | string | Provider milestone title/identifier |
| `search` | string | Portable title/body search term |
| `commit` | string | Pull requests associated with a commit SHA |
| `created_after` / `created_before` | datetime | Inclusive creation window |
| `updated_after` / `updated_before` | datetime | Inclusive update window |
| `sort` | string | `created`, `updated`, or `provider-default` |
| `direction` | string | `ascending` or `descending` |
| `limit` | number(integer) | Maximum returned items; default 20, maximum 100 |

### `cicd_list(query)` keys

| Key | Type | Meaning |
|-----|------|---------|
| `remote` | string | Exact configured remote name; preferred remote when absent |
| `statuses` | string or string[] | Normalized lifecycle states; defaults to all statuses |
| `name` | string | Exact or provider-supported job-name match |
| `stage` | string | Pipeline stage when the provider exposes stages |
| `workflow` | string | Parent workflow/pipeline name, definition ID, or path |
| `parent` | number(integer) or string | Exact parent workflow-run/pipeline identity |
| `branch` | string | Exact branch/ref |
| `commit` | string | Exact commit SHA |
| `actor` | string | Triggering provider login/username |
| `trigger` | string | Push, PR/MR, schedule, manual, parent, or provider event |
| `created_after` / `created_before` | datetime | Inclusive creation window |
| `updated_after` / `updated_before` | datetime | Inclusive update window |
| `direction` | string | `ascending` or `descending` |
| `limit` | number(integer) | Maximum returned jobs; default 20, maximum 100 |

### Closed enum values

| Enum | Accepted values |
|------|-----------------|
| `state` (`pr_list`) | `open`, `closed`, `merged` |
| `statuses` (`cicd_list`) | `success`, `failed`, `cancelled`, `queued`, `running`, `manual`, `skipped` |
| `sort` (`pr_list`) | `created`, `updated`, `provider-default` |
| `direction` | `ascending`, `descending` |

The `statuses` values are Darkmatter's *normalized* vocabulary. Provider-native
spellings are mapped onto it (for example GitHub's `completed` and Bitbucket's
`successful` both normalize to `success`); the provider's raw state is retained
in the structured record alongside the normalized one.

```text
pr_list({ state: ["open", "merged"], target_branch: "main", limit: 10 })
cicd_list({ statuses: ["failed", "cancelled"], branch: "main", limit: 10 })
```

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

Expression functions live in domain modules under
[`expression/functions/`](../../lib/src/markdown/compose/expression/functions)
and share one registration model:

- **Pure functions** — depend only on their arguments. Most
  helpers (`length`, `min`, `kebab_case`, `is_today`, …) are pure. Dispatched by
  `dispatch`, which needs no context.
- **Context-aware / read-side functions** — need a
  [`ResolutionContext`](../../lib/src/markdown/compose/expression/resolve_ctx.rs)
  to resolve path arguments. The [read-side functions](#read-side-functions)
  live here. Dispatched by `dispatch_fs`, which receives the context; `is_fs_function`
  reports membership so the evaluator can emit the "requires a document
  resolution context" error when no context is available.

To add a function:

1. Add its signatures, description, ordering, and examples to the authored
   [`expression-functions.yaml`](../schemas/expression-functions.yaml) catalog.
2. In the owning domain module, implement the handler and add one runtime binding
   containing its canonical name, aliases, evaluation mode, and handler kind.
   Consumers read the projected catalog through
   `expression_function_descriptors()`; runtime bindings do not repeat authored
   metadata.

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
