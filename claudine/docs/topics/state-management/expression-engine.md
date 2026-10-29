---
kind: canonical-documentation
---

# Expression Engine

Expressions let a Markdown document calculate values and choose which content
to include. For example, a prompt can insert today's date, convert a name to
uppercase, or include a section only when it applies to the current project.

**Darkmatter implements the expression language.** Claudine uses it when
composing documents. Expressions read values and return results; operations
that save changes belong to the separate [side-effect engine](side-effects.md).
Some expression functions read files, so read-only does not mean they perform
no I/O.

To browse the language rules and available functions, run:

```sh
claudine context --expressions
```

This report displays documentation; it does not evaluate its examples.

## Insert a value into text

Put an expression inside `{{ … }}` to replace it with its result. This is called
**interpolation**:

```markdown
Today is {{ ctx.today }}.
Greeting: {{ upper("hello") }}.
```

The first expression reads today's date from the runtime context. The second
calls `upper`, a function that converts text to uppercase, producing
`Greeting: HELLO.`

Expressions can also appear in frontmatter, the YAML metadata block at the top
of a Markdown document. For example, a document can define a title and use it
in its body:

```markdown
---
title: Release notes
---
# {{ upper(fm.title) }}
```

The resulting heading is `# RELEASE NOTES`.

## Include content conditionally

A `when="…"` expression decides whether a supported block or directive should
be included. For example, this page block appears only in a monorepo—a
repository containing multiple packages:

```markdown
::block when="ctx.is_monorepo"
Check which package this change belongs to before editing.
::end-block
```

If the condition is false, composition removes the block's content. See
[Page Blocks](../../../../darkmatter/docs/inline/page-blocks.md) for nesting and
other examples.

## Read variables and handle missing values

An expression can read document values, runtime context, and environment
variables:

| Form | Meaning |
|---|---|
| `fm.title` | The current document's `title` frontmatter property. |
| `draft` or `user.role` | A simple or nested value supplied to the expression. |
| `ctx.today` | A runtime fact from the [context variables](context-variables.md). |
| `env.HOME` | An environment variable, when available. |
| `items[0]` | The first array element; indexes start at zero. |
| `items[-1].name` | The `name` property of the last array element. |
| `fm["release-name"]` | A property accessed by its quoted key. |

A bare name and its `fm.` form read the same frontmatter value (`title` and
`fm.title`), except when the name is a reserved root. `{{ env }}` is always the
environment, so a frontmatter key named `env` is read as `{{ fm.env }}`; the
same applies to `ctx` and `current`, and in a lifecycle event to `err`,
`timing`, and `group`. `doc` is not reserved: a frontmatter key named `doc` is
read as `{{ doc }}`.

Missing paths, properties accessed on `null`, and out-of-range array indexes
resolve to `null`. This allows an expression to provide a fallback:

```markdown
Package: {{ ctx.current_package || "no package selected" }}
```

Here, `||` uses the package name if it is truthy, otherwise the text on the
right. “Truthy” is explained below; a fallback applies to empty or false values
as well as `null`.

### Truthiness

Conditions interpret values as true or false. The following values count as
false, or **falsy**:

- `null`, including missing values
- `false`
- zero (`0` or `0.0`)
- an empty string (`""`), array (`[]`), or object (`{}`)

All other values are **truthy**. For example, a nonempty string is truthy even
if its text is `"false"`.

### Interpolation vs. condition mode

The meaning of `||` depends on where the expression appears:

| Where | Meaning of `||` | Example |
|---|---|---|
| Interpolation: `{{ … }}` | Returns the first truthy value, or the fallback value. | `{{ "ready" || "waiting" }}` returns `ready`. |
| Condition: `when="…"` | Logical OR: returns a boolean. | `when="draft || review_needed"` is true if either value is truthy. |

`&&` means logical AND in both modes: both sides must be truthy. The function
forms `and(...)` and `or(...)` also work in both modes and stop evaluating once
the result is determined.

## Calculate and compare values

Expressions support arithmetic (`+`, `-`, `*`, `/`, `%`), comparisons (`==`,
`!=`, `>`, `>=`, `<`, `<=`), logical negation (`!`), and numeric negation (`-`).
Use a ternary expression to choose between two values:

```markdown
{{ 2 + 3 * 4 }}
{{ ctx.is_monorepo ? "Multiple packages" : "Single project" }}
```

The first expression produces `14`. The second chooses its text according to
the condition before `?`.

### Operator precedence (high → low)

Operators higher in this list are evaluated before those below them. Use
parentheses to make a different order explicit: `(2 + 3) * 4` produces `20`.

1. Values, function calls, member/index access, and parentheses: `foo.bar`, `foo[0]`, `(expr)`
2. Unary operators: `!`, `-`
3. Multiplication, division, remainder: `*`, `/`, `%`
4. Addition and subtraction: `+`, `-`
5. Comparisons: `==`, `!=`, `>`, `>=`, `<`, `<=`
6. Logical AND: `&&`
7. Logical OR or fallback: `||`, depending on mode
8. Conditional choice: `? :`

## Find and use functions

Functions cover text, numbers, collections, dates, type checks and conversions,
and reading information from files. For example:

| Expression | Result or purpose |
|---|---|
| `upper("hello")` | Returns `HELLO`. |
| `length("hello")` | Returns `5`. |
| `file_exists("notes.md")` | Checks whether a file exists. |

Use `claudine context --expressions` to find supported signatures, descriptions,
and examples. A **signature** shows the function name and its arguments. For
the full written reference, see
[Darkmatter Expressions](../../../../darkmatter/docs/topics/darkmatter-expressions.md).

An unknown function name produces an error with a suggested match when one is
available. For example, `uper` can suggest `upper` along with its signature,
description, and example.

### Arguments have types

Each function declares a type for each argument: `min(a: number, b: number)`
takes two numbers. When a value is not already that type, Darkmatter converts
it only when the value has an exact equivalent (the text `"4"` is the number
`4`; the number `2026` is the text `"2026"`), and otherwise fails the call
with a type error instead of guessing:

```markdown
{{ min("4", 9) }}         → 4: the text "4" converts to a number
{{ min("pear", 9) }}      → type error: "pear" is not a number
{{ number("pear", 0) }}   → 0: number() returns its fallback instead of failing
{{ upper(missing) }}      → empty: upper passes a missing value through
```

The same rules convert frontmatter values to their schema types. Darkmatter's
[Type Coercion](../../../../darkmatter/docs/topics/schemas/definition.md#type-coercion)
lists them, and [Numbers](../../../../darkmatter/docs/topics/numbers.md)
describes how numbers are held and displayed.

**A call that can never work stops the whole document.** Before any
expression or shell command runs, Darkmatter checks every call against the
types the document's `$schema` declares. A wrong argument count (`min(1)`), a
literal that cannot convert (`min("pear", 9)`), or a declared type that never
fits, such as a `boolean(required)` property passed to `min`, rejects the
document, even inside a branch that would never run. Claudine adds no rule of
its own: `claudine compose`, `inline-compose`, and every `sequence` task fail
the same way `md compose` does. See
[Call Check](../../../../darkmatter/docs/inline/call-check.md).

A call that might fail only for some values, such as a text property passed to
`min`, still composes; its value is checked when the call runs. The DMLS
editor integration shows both cases as you type: an error for a call
composition will reject, a warning for one that may fail. See
[DMLS call diagnostics](../../../../darkmatter/dmls/docs/diagnostics.md#call-diagnostics).

## Implementation reference

The remaining sections are for contributors extending the language or reports.

### How evaluation works

The engine lives in the
[Darkmatter expression module](../../../../darkmatter/lib/src/markdown/compose/expression/).
An expression passes through three stages:

1. The **lexer** splits its text into tokens, such as names and operators.
2. The **parser** builds an expression tree (`ast::Expr`). `parse()` selects
   interpolation mode; `parse_condition()` selects condition mode.
3. The **evaluator** computes the result. `EvaluationLookup` supplies variable
   values and the resolution context needed by file-reading functions.

### Function descriptions and executable behavior

Function metadata—typed signatures, categories, descriptions, and examples—is
authored as `function(...)` types in the catalog
[functions.yaml](../../../../darkmatter/schemas/partials/functions.yaml); its
`categories:` list fixes the order categories are shown in.
Executable implementations live in domain modules under
[functions/](../../../../darkmatter/lib/src/markdown/compose/expression/functions/).

A runtime binding associates a canonical name and any aliases with an evaluation
mode and handler. `FunctionHandler::Pure` handles value-based calculations;
`FunctionHandler::Context` handles operations that need a resolution context.
The lazy `and` and `or` bindings evaluate arguments only as needed.

The registry joins these bindings to the catalog's function entries by
canonical name.
Initialization rejects duplicate names, alias collisions, and names missing
from either side. This keeps the documented list tied to executable functions.

Descriptors implement `darkmatter::catalog::Described`. The shared `describe`,
`suggest`, and `describe_for_error` helpers provide exact lookup, similar-name
suggestions, and plain-text error details. Claudine supplies terminal styling.

### How the report and written reference stay current

Claudine's [context command](../../../cli/src/commands/context/mod.rs) renders
functions from `expression_function_catalog()`: categories in the catalog's
`categories:` order, functions alphabetically within each, and every overload
once under its canonical name, never an alias. Each row shows all of that
overload's examples, inline ones as `call ⇒ result` (a display-only example,
one that tests do not execute, followed by its reason) and file-form ones as
the example file's invocation and result. Language rules come from the descriptor catalogs in
[semantics.rs](../../../../darkmatter/lib/src/markdown/compose/expression/semantics.rs).
The CLI does not maintain a second list of functions or language rules.

The function table in the Darkmatter written reference is generated from the
same function descriptors. Regenerate it with:

```sh
just darkmatter regen-expr-doc
```

Tests check different parts of this connection:

| Check | What it verifies |
|---|---|
| `operator_precedence_matches_parser` | Documented precedence matches the parser's table. |
| Semantics `*_examples_evaluate_correctly` tests | Language-rule examples produce their stated results. |
| `catalog_and_runtime_bindings_have_bidirectional_canonical_parity` | Every catalog function has a runtime binding, and every binding a catalog entry. |
| `each_registration_compiles_its_catalog_overloads` | Each function's declared overloads compile for dispatch. |
| `every_executable_example_evaluates_to_its_declared_result` | Inline examples not marked display-only produce their stated results. |
| `every_embedded_example_file_evaluates_to_its_returns` | File-form examples produce the result their file states. |
| `unknown_names_are_not_dispatched` | Unknown function names fail. |
| `narrative_doc_function_table_matches_catalog` | The generated function table matches the catalog. |
| `context_expressions_function_order_snapshot` | `claudine context --expressions` keeps the category and alphabetical order. |

These checks verify the catalog connections and examples, rather than every
sentence of explanatory prose. See [Drift Control](drift.md) for broader limits.

### How to add an expression function

1. Add its `function(...)` type—parameters, return type, category,
   description, and examples—to `darkmatter/schemas/partials/functions.yaml`.
2. Add its handler and runtime binding to the appropriate domain module under
   `expression/functions/`.
3. Regenerate the written function table and run the relevant catalog and
   behavior tests.

The CLI function table updates from the catalog and needs no separate entry.
