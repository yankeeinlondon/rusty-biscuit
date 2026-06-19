# Expression Engine

The expression engine is Darkmatter's small, **read-only** language for
computing values during composition. It powers two surfaces:

- **Interpolation** — `{{ … }}` blocks whose result is substituted into the body.
- **Conditions** — `when="…"` attributes that gate a block on a boolean.

```markdown
{{ upper(ctx.current_package) }}
{{ ctx.gpu || "no GPU detected" }}

<!-- when="ctx.is_monorepo && length(ctx.packages) > 1" -->
```

It is read-only by construction: an expression can inspect context, call pure
functions, and read files, but it can never mutate state. Mutation is the
separate concern of the [side-effect engine](side-effects.md).

## Runtime-accessible descriptions

Every expression descriptor — functions *and* language semantics — implements
the shared `Described` trait from `darkmatter::catalog`. This powers exact lookup,
fuzzy suggestion, and plain-text error enrichment inside the evaluator:

- `describe(EXPRESSION_FUNCTION_DESCRIPTORS, "upper(x)")` returns the matching
  function descriptor.
- `suggest(EXPRESSION_FUNCTION_DESCRIPTORS, "uper", 1)` returns `upper(x)`.
- `describe_for_error(descriptor)` emits a plain-text line with signature,
  description, and verified example.

When evaluation fails with `Unknown function: uper`, the error message appends
the nearest match's signature, description, and example. Arity errors likewise
append the correct signature and example for the matched function. The error
text itself remains plain text; Claudine owns any terminal styling.

## How evaluation works

The engine lives in `darkmatter/lib/src/markdown/compose/expression/` and runs a
three-stage pipeline:

1. **Lexer** (`lexer.rs`) — tokenizes the source in one of two
   [parse modes](#interpolation-vs-condition-mode).
2. **Parser** (`parser.rs`) — builds an AST (`ast::Expr`). Entry points:
   `parse()` for interpolation, `parse_condition()` for conditions.
3. **Evaluator** (`mod.rs::evaluate`) — folds the AST against an
   `EvaluationLookup`, which resolves `ctx.*` / `env.*` variables and (for
   filesystem functions) supplies a `ResolutionContext`.

`Expr` covers literals, variables, member/index access, unary and binary
operators, comparisons, ternaries, fallback, and function calls.

## The language

The `--expressions` report documents the language in full. The headline rules:

### Operator precedence (high → low)

1. Primary / member access — literals, variables, calls, `foo.bar`, `foo[0]`, `(expr)`
2. Unary — `!`, `-`
3. Multiplicative — `*`, `/`, `%`
4. Additive — `+`, `-`
5. Comparison — `==`, `!=`, `>`, `>=`, `<`, `<=`
6. Logical AND — `&&` (condition mode)
7. Logical OR / fallback — `||` (mode-dependent)
8. Ternary — `? :`

### Truthiness

Falsy: `null` / missing, `false`, `0`, `0.0`, `""`, `[]`, `{}`. Everything else
is truthy.

### Variable access

- Simple and nested keys: `draft`, `user.role`.
- Context and environment: `ctx.today`, `env.HOME`.
- Bracket access: `foo[0]`, `foo[-1]` (negative from the end), `foo["key"]`,
  chained `items[-1].name`.
- **Null propagation** — a non-existent path resolves to `null` rather than
  erroring; dot access on a `null` base returns `null`; out-of-bounds and
  bracket-on-null return `null`.

### Interpolation vs. condition mode

The single most important distinction, because `||` and `&&` change meaning:

| Surface | `||` means | `&&` |
|---------|------------|------|
| `{{ … }}` (interpolation) | **fallback** — first truthy value wins | rejected at parse time |
| `when="…"` (condition) | **logical OR** — returns a boolean | logical AND |

The function forms `and(...)` / `or(...)` are valid in *both* modes.

## Functions

Functions are the extensible part of the language. They are registered in
**authoritative typed tables** in
`darkmatter/lib/src/markdown/compose/expression/functions.rs`:

- `PURE_FUNCTIONS` — pure functions resolved by `dispatch()` (type predicates,
  math, collections, string predicates/mutations, date formatting/validators,
  type conversion).
- `FS_FUNCTIONS` — context-aware functions resolved by `dispatch_fs()` that need
  a `ResolutionContext` (`absolute`, `relative`, `file_exists`, `frontmatter`,
  `markdown_body_empty`, `markdown_title`, `validate_schema`).
- **Lazy logical operators** — `and(...)` / `or(...)`, which short-circuit and
  therefore cannot go through the eager dispatchers; named in
  `LAZY_OPERATOR_NAMES`.

Each registration carries the full set of **signatures** it answers to,
including overloads and optional/variadic arity:

```rust
PureFunction { canonical: "number", aliases: &[], signatures: &["number(x, [default])"], handler: number_fn }
FsFunction   { canonical: "frontmatter", aliases: &[], signatures: &["frontmatter(file)", "frontmatter(file, prop)"], handler: … }
```

These tables are the single source of truth for *what the evaluator recognizes*.

## How the `--expressions` report is built

`render_expressions_report` (`claudine/cli/src/commands/context.rs`) emits two
kinds of content:

1. **The function catalog** — rendered directly from
   `expression_function_descriptors()` (`EXPRESSION_FUNCTION_DESCRIPTORS` in
   `expression/catalog.rs`). The CLI groups descriptors by `category` (each
   category emitted once, even though the catalog is physically laid out by
   implementation grouping) and orders within a category by the descriptor's
   `order`. **This part cannot drift from the catalog** — it *is* the catalog.

2. **The language-semantics sections** — operator precedence, truthiness, unary,
   comparison, arithmetic, variable access, mode table, and null propagation.
   These are rendered from typed descriptor catalogs in
   `expression/semantics.rs` that are anchored to the parser and evaluator:
   `operator_precedence_matches_parser` asserts the precedence catalog matches
   the parser's own table, and per-catalog `*_examples_evaluate_correctly` tests
   run every example through the real `evaluate` pipeline. **No hand-written
   literal arrays remain in the CLI.**

## Narrative documentation parity

The function table in `darkmatter/docs/topics/darkmatter-expressions.md` is
regenerated from `EXPRESSION_FUNCTION_DESCRIPTORS` by
`just darkmatter regen-expr-doc`. The generated region is guarded by
`narrative_doc_function_table_matches_catalog`, which fails the build if the
committed doc diverges from the catalog output.

## How to add an expression function

1. **Implement and register it.** Add a handler and a `PureFunction` (or
   `FsFunction`) entry in `functions.rs`, listing every signature/overload.
2. **Describe it.** Add an `ExpressionFunctionDescriptor` to
   `EXPRESSION_FUNCTION_DESCRIPTORS` in `catalog.rs` with a verified `example`.
3. The `--expressions` function table needs **no change** — it reads the catalog.

## Drift control for the function catalog

The catalog and the runtime registry are two parallel lists, kept in exact,
overload-aware lockstep by tests in `expression/catalog.rs`:

- **`descriptor_signature_set_equals_dispatchable_signature_set`** — bidirectional
  set equality between `EXPRESSION_FUNCTION_DESCRIPTORS` and
  `dispatchable_signatures()` (the runtime surface enumerated from
  `PURE_FUNCTIONS` + `FS_FUNCTIONS` + lazy operators). Comparing *signatures*
  (with arity), not just names, means a stray or missing overload fails too.
- **`every_descriptor_overload_is_dispatchable_at_its_declared_arity`** — an
  end-to-end proof: each descriptor is parsed and run through the real
  `evaluate` pipeline at its declared arity. A descriptor whose handler was
  removed yields `Unknown function`; a bogus overload yields an arity error.
- **`lazy_operators_are_dispatchable`** and **`unknown_function_is_rejected`**
  anchor the two ends (real operators dispatch; a fake name is rejected).
- **`every_example_evaluates_to_its_declared_result`** runs each descriptor's
  `Example` through the evaluator and asserts the rendered output equals the
  declared `result`.

So: add a function to only the catalog *or* only the registry, and the build
fails. The language-semantics prose is catalog-driven and parity-checked too.
