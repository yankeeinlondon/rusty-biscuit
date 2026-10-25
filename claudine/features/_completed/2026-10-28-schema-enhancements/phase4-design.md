# Phase 4 Design (frozen by the integrator before Wave 5)

This file freezes the shared shapes every Phase 4 task builds on. It is a
planning artifact for `2026-10-28-schema-enhancements`; the behavior it
describes is documented in `darkmatter/docs/` by the tasks that build it.

Authoritative inputs, in order: the spec (`spec.md`), `behavior-ledger.md`
(every function's target signature and null outcome, approved by the author),
`inventory/function-registry.md` (the 113 registered functions and aliases),
and the Phase 3 call binder (`darkmatter/lib/src/markdown/compose/expression/call_binding/`).

## 1. The catalog file

- **Home:** `darkmatter/schemas/partials/functions.yaml` (`kind: schema`,
  `categories:`, `types:`; nothing else). `mutations.yaml` moves beside it
  with its `name:` key dropped. Both drafts under
  `claudine/docs/schemas/partials/` are deleted once moved.
- **Entries:** one `types:` entry per canonical function (113 registered plus
  `trim`, `keys`, `values` = 116). An overloaded function is a YAML list of
  `function(...)` types. Signatures are the ledger's "Target signature"
  column, written in the grammar (`name?: T`, `...name: T`, `parameters(void)`,
  `| null` arms exactly as the ledger says). `fallible` only where a function
  can fail after its arguments bound.
- **Named helper types** (non-function `types:` entries) are allowed and are
  not functions: `agentic-cli` (an `enum(...)` of every accepted
  `has_agentic_cli` name, in `AGENTIC_CLI_NAMES` order; a test pins parity
  with the generated table), `ip-address` (a `string(pattern(...))` accepting
  IPv4, IPv6 and scoped IPv6 such as `fe80::1%en0`, never a host name), and
  `recent-commits` (the shared result type of `recent_commits()`, which
  `ctx.recent_commits` also uses when the embedded partial import works; see
  section 6).
- **Examples:** every one of the old catalog's 121 examples is carried.
  Executable ones become `example(CALL => RESULT)`; display-only ones add
  `; display-only: REASON` with the old reason. A result is a Darkmatter
  expression literal or `error`. The four "verified via example file" entries
  (`as_line_separated`, `as_tsv`, `as_unordered_list`, `as_ordered_list`)
  become **file-form** examples, `example(./function-examples/<name>.yaml)`,
  whose files live in `darkmatter/schemas/partials/function-examples/` and
  use the existing example envelope (`kind: example`, `invocation`,
  `returns`, `description`). New executable examples are added for the
  criterion 12 cases (`number`, `round`, `fallback`), `keys`, `values`,
  `trim`, and `length`.
- **Categories:** list order is documentation order; every entry has a real
  description; every category a function uses is listed (`Git`, `CI/CD`,
  `Date Arithmetic`, `Network`, …).

## 2. Catalog loader and descriptors (`expression/catalog/`)

The old parser (`catalog/parser.rs`, `catalog/ast.rs`) and the reduced type
vocabulary (`DataType`, `ParamType`, `ParamRefinement`, `ReturnType`,
`ReturnValueType`, `pair`) are deleted. The loader embeds `functions.yaml`
with `include_str!`, parses it once with `parse_standalone_schema_document`,
and projects descriptors once (`LazyLock`). It is strict: the file must be the
tagged envelope with `categories:`; every function entry is a `function` or a
union of `function`s; each overload set already passes the duplicate-contract
check of the standalone parser.

Public API (re-exported from `markdown::compose::expression`):

```rust
pub struct CatalogCategory { pub name: &'static str, pub description: Option<&'static str> }

pub struct FunctionParameterDescriptor {
    pub name: &'static str,
    pub type_text: &'static str,   // canonical grammar spelling, e.g. `number | null`
    pub optional: bool,
    pub rest: bool,
}

pub enum FunctionExample {
    /// `example(CALL => RESULT)`; `result` is the authored literal text or `error`.
    Call { call: &'static str, expected: ExampleOutcome, result: &'static str, display_only: Option<&'static str> },
    /// `example(./function-examples/x.yaml)`, read from the embedded file.
    File { path: &'static str, invocation: &'static str, returns: &'static str, description: Option<&'static str> },
}

pub struct ExpressionFunctionDescriptor {
    pub name: &'static str,                 // canonical name
    pub aliases: &'static [&'static str],   // registered aliases
    pub signature: &'static str,            // display key, unchanged format: `round(x, [fallback])`, `and(...)`
    pub description: &'static str,
    pub category: &'static str,
    pub order: usize,                       // 1-based position within its category
    pub overload: usize,                    // index within the function's overloads
    pub parameters: &'static [FunctionParameterDescriptor],
    pub returns: &'static str,              // canonical return type text
    pub fallible: bool,
    pub function_type: &'static FunctionType,
    pub examples: &'static [FunctionExample],
    pub example: Option<Example>,           // the first inline example, for `Described`
}

pub struct FunctionCatalog {
    pub categories: &'static [CatalogCategory],
    pub descriptors: &'static [ExpressionFunctionDescriptor], // documentation order
}

pub fn expression_function_catalog() -> &'static FunctionCatalog;
pub fn expression_function_descriptors() -> &'static [ExpressionFunctionDescriptor];
/// Every overload of the function `name` names, resolving aliases and case like dispatch.
pub fn function_descriptors(name: &str) -> &'static [ExpressionFunctionDescriptor];
```

- **Order:** categories in `categories:` list order; functions alphabetical
  by canonical name within a category; overloads in authored order. Each
  canonical appears once per overload and never under an alias.
- `typed_signature()` renders `min(a: number, b: number) -> number`,
  `round(x: numberlike, [fallback: number]) -> number`,
  `and(...values: unknown | null) -> boolean`; a fallible function appends
  ` | error` (illustrative, as the spec allows).
- `Described` stays implemented (`key` = `signature`, `example()` = first
  inline example; `Example.result` is the authored literal text).
- `Example`/`ExampleVerification` in `crate::catalog` are unchanged.
- `generate_expression_function_table()` groups by category list order,
  alphabetical within each, and shows every inline example (executable and
  display-only, the latter marked) plus file-form examples.

## 3. Registry, binding, and dispatch (`expression/functions/`)

```rust
pub(crate) type PureFn = fn(&BoundCall<'_>) -> Result<Value, String>;
pub(crate) type ContextFn = fn(&BoundCall<'_>, &ResolutionContext) -> Result<Value, ExpressionError>;

struct FunctionBinding {
    canonical: &'static str,
    aliases: &'static [&'static str],
    evaluation: EvaluationMode,            // Pure | Context | Lazy
    handler: Option<FunctionHandler>,      // None only for Lazy
    rejects: &'static [(&'static str, &'static [ValueKind])], // registered source exclusions
    recovery: Option<(&'static str, &'static str)>,           // (subject, fallback)
}
```

- The joined registry is built once: for each binding, its catalog overloads
  compile into a `FunctionRegistration` (`builder_from_types`, `reject_sources`,
  `numeric_recovery`, lazy evaluation), with a `TypeResolver` over the
  catalog's named helper types. Parity is checked both ways; any failure
  panics with the registry error (and a unit test asserts the registry builds).
- `length` registers `rejects: &[("val", &[ValueKind::Boolean])]`; `number`
  and `round` register `recovery: Some(("x", "fallback"))`.
- **One dispatch entry** for eager calls:
  `pub(crate) fn call_function(name, args: &[Value], ctx: Option<&ResolutionContext>) -> Option<Result<Value, ExpressionError>>`.
  `None` only for an unknown name. Order: resolve (lowercase, canonical or
  alias) → a context function without a context returns the existing
  "requires a document resolution context" error, nothing bound → `bind_call`
  (failure: `ExpressionError::Call`, handler never entered) → handler entry
  (counted by the test hooks) → `check_return` on success (failure:
  `ExpressionError::Call` with `ImplementationContract`). Domain errors from
  pure handlers stay `ExpressionError::Other { function, message }`.
- **Lazy calls:** the evaluator asks the registry for the evaluation mode
  (no literal `"and"`/`"or"` match), checks arity before evaluating, and binds
  each operand through `LazyCall::bind_next` as it evaluates it; a skipped
  operand is never evaluated or bound. Truthiness and short-circuiting stay
  the evaluator's domain logic.
- Handlers read arguments only through `BoundCall`/`BoundArgs` accessors
  (`args().text(i)`, `number(i)`, `list(i)`, `object(i)`, `get(i)`,
  `selected(i)`, `rest()`, `is_omitted(i)`, `overload()`). No handler checks
  arity, inspects a type to convert it, or calls a conversion helper, except
  where conversion is the function's documented purpose (`number`, `round`,
  `as_json`, …). `require_args`, `require_number`, `require_string`,
  `require_array`, `any_null`, `to_number_coerce` uses, `string_mutation`'s
  checks, and handler-level `ExpressionError::ArgType`/arity errors are
  removed.

## 4. Errors

- New variant `ExpressionError::Call(Box<CallBindingError>)`, displayed as the
  binder's message. `is_authoring_fatal` is `true` only for
  `ImplementationContract` (a Darkmatter defect, like `ContractViolation`);
  type, arity, and ambiguous-overload errors keep the old arity/arg-type
  severity (demoted in lenient body interpolation, fatal elsewhere).
- `ExpressionError::ArgType` and `ExpressionError::Arity` are removed once no
  handler builds them.

## 5. Test hooks (normal dispatch with counters)

Compiled under `cfg(test)` or the existing `effects-instrumentation` feature
(already enabled by `just test` and CI for the library and DMLS):

```rust
pub mod function_test_hooks {
    /// Registers a test-only function for the current thread; dispatch
    /// consults it before the built-in registry. Dropping the guard removes it.
    pub fn register(registration: FunctionRegistration, handler: TestHandler) -> TestFunctionGuard;
    pub type TestHandler = std::sync::Arc<dyn Fn(&BoundCall<'_>) -> Result<Value, String> + Send + Sync>;
    /// How many times the handler of `name` (canonical, built-in or test) was
    /// entered on the current thread since the last reset.
    pub fn handler_entries(name: &str) -> usize;
    pub fn reset_handler_entries();
}
```

Lazy test functions register with `EvaluationMode::Lazy` are not needed: the
effect counter is an eager test function placed in a skipped operand.

## 6. Integrator decisions recorded in the log

The implementation log's `## Phase 4` → `### Design Decisions` records each
of: handler signature, the `Call` error and its fatality, the test-hook
feature, the display-key format, file-form example embedding, named helper
types, the `recent-commits` sharing outcome, and the move of context bodies
to `functions/fs.rs` (move-only, before Wave 5).
