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

- `describe(expression_function_descriptors(), "upper(x)")` returns the matching
  function descriptor.
- `suggest(expression_function_descriptors(), "uper", 1)` returns `upper(x)`.
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

The single most important distinction, because `||` changes meaning between the
two surfaces (`&&` does not — it is logical AND in both):

| Surface | `||` means | `&&` |
|---------|------------|------|
| `{{ … }}` (interpolation) | **fallback** — first truthy value wins | logical AND |
| `when="…"` (condition) | **logical OR** — returns a boolean | logical AND |

The function forms `and(...)` / `or(...)` are valid in *both* modes.

## Functions

Functions are the extensible part of the language. Handler-free metadata is
authored in `darkmatter/docs/schemas/expression-functions.yaml`; runtime
behavior is bound by canonical name in domain slices under
`darkmatter/lib/src/markdown/compose/expression/functions/`:

- `FunctionHandler::Pure` — functions resolved by `dispatch()` (type predicates,
  math, collections, string predicates/mutations, date formatting/validators,
  type conversion).
- `FunctionHandler::Context` — functions resolved by `dispatch_fs()` that need
  a `ResolutionContext` (`absolute`, `relative`, `file_exists`, `frontmatter`,
  `markdown_body_empty`, `markdown_title`, `validate_schema`).
- lazy bindings — `and(...)` / `or(...)`, which short-circuit and therefore do
  not carry an eager handler.

Each runtime binding carries only its canonical name, aliases, evaluation mode,
and handler. The cached registry joins it to catalog descriptors, including
overloads and optional/variadic arity:

```rust
FunctionBinding { canonical: "number", aliases: &[], evaluation: EvaluationMode::Pure, handler: Some(FunctionHandler::Pure(number_fn)) }
FunctionBinding { canonical: "frontmatter", aliases: &[], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(frontmatter_fn)) }
```

The catalog is the source of truth for metadata; bindings are the source of
truth for executable behavior and aliases. Registry initialization requires
bidirectional canonical-name parity between them.

## How the `--expressions` report is built

`render_expressions_report` (`claudine/cli/src/commands/context.rs`) emits two
kinds of content:

1. **The function catalog** — rendered directly from
   `expression_function_descriptors()`. The CLI groups descriptors by `category` (each
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
regenerated from `expression_function_descriptors()` by
`just darkmatter regen-expr-doc`. The generated region is guarded by
`narrative_doc_function_table_matches_catalog`, which fails the build if the
committed doc diverges from the catalog output.

## How to add an expression function

1. Add signatures, descriptions, ordering, and examples to
   `darkmatter/docs/schemas/expression-functions.yaml`.
2. Add the handler and its runtime binding to the owning domain module.
3. The `--expressions` function table needs **no change** — it reads the
   catalog projection through `expression_function_descriptors()`.

## Drift control for the function catalog

The authored catalog and runtime bindings are joined by canonical name and
guarded by registry invariants and behavior tests:

- Registry invariants reject duplicate canonical names, alias collisions, and
  missing entries on either side of the catalog/binding boundary.
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
