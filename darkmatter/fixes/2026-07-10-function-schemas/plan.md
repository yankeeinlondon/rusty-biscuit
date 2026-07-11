---
agent: opencode
total_phases: 7
created: 2026-07-10
phase: 7
yolo: "true"
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - darkmatter/fixes/2026-07-10-function-schemas/plan.md
docs_created_during_phase_1:
  - darkmatter/fixes/2026-07-10-function-schemas/phase-1-baseline.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/ast.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/parser.rs
docs_updated_during_phase_2:
  - darkmatter/fixes/2026-07-10-function-schemas/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/expression/catalog/parser.rs
docs_updated_during_phase_3:
  - darkmatter/fixes/2026-07-10-function-schemas/plan.md
docs_created_during_phase_3:
  - darkmatter/docs/schemas/expression-functions.yaml
  - darkmatter/fixes/2026-07-10-function-schemas/drift-log.md
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
docs_updated_during_phase_4:
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/fixes/2026-07-10-function-schemas/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_5:
  - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/predicates.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/collections.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/strings.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/terminal.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/dates.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/paths.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/skills.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/markdown_docs.rs
docs_updated_during_phase_5:
  - darkmatter/fixes/2026-07-10-function-schemas/plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_6:
  - darkmatter/cli/src/commands/schema/about.rs
docs_updated_during_phase_6:
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/fixes/2026-07-10-function-schemas/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_files_during_phase_7:
  - darkmatter/dmls/src/overlay/expressions.rs
docs_updated_during_phase_7:
  - claudine/docs/topics/context/drift.md
  - claudine/docs/topics/context/expression-engine.md
  - darkmatter/fixes/2026-07-10-function-schemas/plan.md
docs_created_during_phase_7:
  - darkmatter/fixes/2026-07-10-function-schemas/phase-7-closeout.md
skills_files_updated_during_phase_7: []
source_code:
  - darkmatter/cli/src/commands/schema/about.rs
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/ast.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/parser.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/collections.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/dates.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/markdown_docs.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/paths.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/predicates.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/skills.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/strings.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/terminal.rs
documentation:
  - claudine/docs/topics/context/drift.md
  - claudine/docs/topics/context/expression-engine.md
  - darkmatter/docs/schemas/expression-functions.yaml
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/fixes/2026-07-10-function-schemas/drift-log.md
  - darkmatter/fixes/2026-07-10-function-schemas/phase-1-baseline.md
  - darkmatter/fixes/2026-07-10-function-schemas/phase-7-closeout.md
  - darkmatter/fixes/2026-07-10-function-schemas/plan.md
packages:
  - darkmatter
  - dmls
  - claudine
---

# Execution Plan: Authored Expression-Function Schemas

## References

- **Specification**: `darkmatter/fixes/2026-07-10-function-schemas/spec.md`
- **Prerequisite**: Godless Beauty Improvement 4 — **complete** (phase-7-closeout
  confirmed; `FunctionRegistration` / domain-module split / `LazyLock` accessor
  are the starting architecture)
- **Baseline architecture files**:
  - `darkmatter/lib/src/markdown/compose/expression/catalog.rs` — `DataType`,
    `ParamType`, `ReturnType`, `ExpressionFunctionDescriptor`, shared `P_*`/`R_*`
    constants, `expression_function_descriptors()`, `generate_expression_function_table()`
  - `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs` —
    `FunctionRegistration`, `FunctionHandler`, `REGISTRATION_GROUPS`,
    `registrations()`, `dispatch`/`dispatch_fs`, `dispatchable_signatures`
  - `darkmatter/lib/src/markdown/compose/expression/functions/{predicates,
    collections,strings,terminal,dates,paths,skills,markdown_docs}.rs` — domain
    `REGISTRATIONS` slices
  - `darkmatter/lib/src/catalog/mod.rs` — `Example`, `ExampleVerification`,
    `Described` trait
- **Consumer files**:
  - `darkmatter/cli/src/commands/schema/about.rs` — `md schema about --verbose`
  - `darkmatter/lib/examples/expression_doc_generator.rs` — doc table generator
  - `darkmatter/dmls/src/overlay/expressions.rs` — LSP completion/hover accessors
  - `darkmatter/dmls/src/providers/dsl.rs` — DSL provider completion/hover wiring
  - `claudine/lib/src/composition/lifecycle_actions.rs` — verb validation
  - `claudine/cli/src/commands/context.rs` — `claudine context --expressions`
- **Schema pattern reference**: `darkmatter/lib/src/markdown/schemas/mod.rs`
  (`darkmatter_base_schema()` — `include_str!` + `OnceLock` pattern)
- **Type keyword reference**: `darkmatter/lib/src/markdown/schemas/simplified/types.rs`
  (`SimplifiedType::from_keyword`)

## Catalog Baseline (post-Godless-Beauty)

| Metric | Count |
|--------|-------|
| Domain modules | 8 + `LAZY_REGISTRATIONS` |
| Registrations (canonical names) | ~82 |
| Descriptors (overloads) | ~85 |
| Multi-overload functions | 3 (`link`, `frontmatter`, `validate_schema`) |
| `catalog_order` range | 0–84 |
| Categories | `Type Predicates`, `Collection`, `Type Conversion`, `Logical`, `List Formatting`, `String`, `Rendering`, `Date`, `Filesystem` |
| Consumers | CLI `schema about`, doc generator, DMLS (2 files), Claudine (2 files) |

---

## Phase 1 — Inventory and Baseline

**Goal**: Capture the exact post-Godless-Beauty catalog state so every later
phase can prove parity against it.

**Validation checkpoint**: A baseline inventory file exists on disk and a
baseline test run passes.

- [x] Run `just test` and `just lint` in the darkmatter area; record the test
  count and confirm zero failures before any changes.
- [x] Run `just test` in `claudine/` and `claudine/cli/` to record the Claudine
  baseline (expression-function consumer tests).
- [x] Produce a machine-readable inventory of every current
  `FunctionRegistration`: canonical name, aliases, `catalog_order`, category,
  per-overload `order`, signature string, parameter types (`DataType` + flags),
  return type (`DataType` + flags), example (`invocation`/`result`/verification),
  and handler kind (`Pure`/`Context`/`Lazy`).
  - Source: iterate `registrations()` in a one-off test or example binary that
    prints the inventory as structured data (YAML or JSON).
- [x] Save the inventory as
  `darkmatter/fixes/2026-07-10-function-schemas/phase-1-baseline.md`.
- [x] Record the exact `generate_expression_function_table()` output and the
  checked-in `darkmatter-expressions.md` function-table bytes for later
  byte-for-byte comparison.
- [x] Record the exact `expression_function_signatures_markdown()` output from
  `md schema about --verbose` for later comparison.

---

## Phase 2 — Catalog AST Types and Parser

**Goal**: Define the function-signature type domain (distinct from
`SimplifiedSchema`) and implement a fallible parser that accepts `&str` YAML and
returns a validated `ExpressionFunctionCatalog` AST or structured errors.

**Depends on**: Phase 1 (baseline inventory for parser test fixtures).

**Validation checkpoint**: Parser unit tests pass for every supported shape and
every rejection case listed in the spec's acceptance criteria.

### 2a — Catalog AST types

- [x] Create `darkmatter/lib/src/markdown/compose/expression/catalog/ast.rs`
  (or a `catalog/` submodule) with the function-specific AST types:
  - `CatalogFunction` — `name`, `category`, `order` (globally unique), `description`, `overloads: Vec<CatalogOverload>`
  - `CatalogOverload` — `parameters: Vec<CatalogParam>`, `returns: CatalogReturn`, `example: CatalogExample`
  - `CatalogParam` — `name: String`, `ty: DataType`, `array: bool`, `optional: bool`, `variadic: bool`
  - `CatalogReturn` — `ty: DataType`, `array: bool`, `fallible: bool`
  - `CatalogExample` — `expression: String`, `result: String`, `verification: CatalogVerification`, `reason: Option<String>`
  - `CatalogVerification` — `Executable` or `DisplayOnly(reason: String)`
  - `ExpressionFunctionCatalog` — `functions: Vec<CatalogFunction>` (declaration-order preserved)
- [x] Reuse `DataType` from the existing `catalog.rs` (it already mirrors the
  `SimplifiedType` keyword set plus `Any` and deliberately excludes `error` and
  function types). Add a `DataType::from_keyword(&str) -> Option<DataType>`
  method that delegates to `SimplifiedType::from_keyword` where possible,
  adding `Any` (which `SimplifiedType` already has) and `Integer` (which maps
  from the `number(integer)` keyword).
- [x] Keep `DataType`, `ParamType`, and `ReturnType` as public
  descriptor-facing projection types. They MUST no longer be independently
  authored authorities after Phase 4 but their public shape stays.

### 2b — Fallible parser

- [x] Create `darkmatter/lib/src/markdown/compose/expression/catalog/parser.rs`
  with:
  - `pub(crate) fn parse_expression_function_catalog(yaml: &str) -> Result<ExpressionFunctionCatalog, CatalogParseError>`
  - `CatalogParseError` — structured error identifying the function, overload,
    and field where possible (e.g. `CatalogParseError { function: Option<String>,
    overload: Option<usize>, field: Option<String>, kind: CatalogErrorKind }`)
- [x] The parser MUST:
  - Parse the YAML using `serde_yaml_ng` into an intermediate owned structure.
  - Validate `kind: expression-function-catalog` is present and exact.
  - Reject unknown fields at every level (catalog, function, overload,
    parameter, return, example).
  - Require at least one function.
  - Validate function names match `[a-z][a-z0-9_]*`.
  - Validate parameter names match `[a-z][a-z0-9_]*` and are unique within an
    overload.
  - Validate `order` is a positive integer and globally unique across the
    complete catalog.
  - Validate categories and descriptions are non-empty.
  - Validate type keywords via `DataType::from_keyword`; reject `error` and
    `function` in parameter/return type positions.
  - Enforce structural invariants:
    - `error` is legal only as a return union member (the `fallible` flag),
      never a parameter type.
    - An optional parameter cannot precede a later required parameter.
    - A variadic parameter is last and cannot also be optional.
    - Returns contain exactly one success type and at most one `error` member.
    - Every overload carries exactly one example.
    - `verification: executable` MUST NOT carry a `reason`.
    - `verification: display-only` MUST carry a non-empty `reason`.
  - Reject duplicate function names and duplicate rendered signatures.
- [x] The parser MUST NOT perform any filesystem probes, shell commands,
  network requests, or expression evaluation.
- [x] The parser MUST NOT leak (no `Box::leak`) — it returns owned data.
  Leaking happens only in the Phase 4 infallible accessor after validation.

### 2c — SimplifiedSchema declaration for the catalog document

- [x] Author the `$schema:` SimplifiedSchema declaration for
  `expression-functions.yaml` as specified in the spec (inline object arrays for
  functions, overloads, parameters, returns, examples). Use the grammar's
  multiline inline-object form if it improves readability.
- [x] The SimplifiedSchema declaration handles structural validation (field
  presence, types, `not-empty`, `required`, `min(1)`). The dedicated parser
  handles closed-field and cross-field invariants that SimplifiedSchema cannot
  express.
- [x] Add a test proving the checked-in catalog document validates against its
  own SimplifiedSchema declaration (structural pass), separate from the
  semantic parser tests.

### 2d — Parser unit tests (L1)

- [x] Parse the complete checked-in catalog (once it exists in Phase 3) and
  assert function/signature counts against the Phase 1 baseline. Until then,
  use focused fixtures.
- [x] Parse focused fixtures for every supported parameter shape: scalar,
    array, optional, variadic.
- [x] Parse focused fixtures for every supported return shape: infallible
    scalar, fallible scalar, array return.
- [x] Assert declaration/display ordering and overload grouping.
- [x] Assert precise failures for every illegal placement:
    `error` parameters, `error`-only returns, multiple success return members,
    required-after-optional, non-final variadics, duplicate parameter names,
    duplicate signatures, unknown type keywords, unknown fields, invalid
    identifiers, empty descriptions/categories, duplicate global order values,
    display-only without reason, executable with reason.
- [x] Assert `SimplifiedType::from_keyword("error")` and
    `SimplifiedType::from_keyword("function")` remain `None`.
- [x] Assert malformed fixture parsing performs no process-lifetime leaks
    (verify via a test that parses several invalid fixtures and checks that
    only the successfully validated embedded catalog is promoted to static
    descriptors — this test can be deferred to Phase 4 when the accessor
    exists, but the parser tests should confirm no panics on malformed input).

---

## Phase 3 — Authored YAML Catalog

**Goal**: Transcribe every existing descriptor into
`docs/schemas/expression-functions.yaml` without semantic edits.

**Depends on**: Phase 2a (AST types define the YAML shape) and Phase 2c
(SimplifiedSchema declaration).

**Can partially overlap with Phase 2d**: the YAML file is needed for the
"parse the complete checked-in catalog" test.

**Validation checkpoint**: The YAML file parses successfully through the Phase 2
parser with zero errors and the parsed catalog matches the Phase 1 baseline
inventory exactly.

- [x] Create `darkmatter/docs/schemas/expression-functions.yaml` with:
  - `kind: expression-function-catalog`
  - `$schema:` carrying the SimplifiedSchema declaration from Phase 2c
  - `functions:` list with every function from the Phase 1 inventory
- [x] For each function, transcribe faithfully:
  - `name` ← canonical name
  - `category` ← existing category (exact string)
  - `order` ← existing `catalog_order` (globally unique; replaces both
    `catalog_order` and per-category `order`)
  - `description` ← existing description (exact string)
  - `overloads:` — one entry per existing descriptor
    - `parameters:` — each with `name` (from the signature string's parameter
      names), `type` (from `DataType::as_keyword`), and `array`/`optional`/
      `variadic` flags (omitted when `false`)
    - `returns:` — `type`, `array`, `fallible` (omitted when `false`)
    - `example:` — `expression` (← `invocation`), `result`, `verification`
      (`executable` or `display-only`), `reason` (only when `display-only`;
      carries the existing `DisplayOnly` reason string)
- [x] Resolve registrations with no example while retaining the parser's
  requirement that every authored overload carry exactly one example.
  - **Note**: The spec says "Every overload MUST carry exactly one example."
    Verify during transcription whether `has_command` (currently `example: None`)
    needs an example added. If so, add one as a clearly separated commit per
    the spec's drift-correction policy. If the spec's "MUST" is a hard
    requirement, add a display-only example with a reason like
    `"result is host-dependent"`.
- [x] Verify the YAML file parses through the Phase 2 fallible parser with
  zero errors.
- [x] Verify the parsed catalog matches the Phase 1 baseline inventory:
  same function count, same signature count, same categories, same `order`
  values (mapped from `catalog_order`), same parameter types, same return
  types, same examples.
- [x] Do NOT make intentional catalog corrections in this transcription. If
  drift is discovered (stale descriptions, wrong examples, etc.), record it
  in `darkmatter/fixes/2026-07-10-function-schemas/drift-log.md` for a
  separate reviewable change.

---

## Phase 4 — Descriptor Projection and Accessor

**Goal**: Project `ExpressionFunctionDescriptor` values from the parsed YAML
catalog and back the public `expression_function_descriptors()` accessor with a
`LazyLock` that embeds, parses, validates, and intentionally leaks the
catalog.

**Depends on**: Phase 2 (parser) and Phase 3 (YAML data).

**Validation checkpoint**: `expression_function_descriptors()` returns
descriptors byte-identical to the Phase 1 baseline (same signatures, same
order, same typed signatures, same examples) and all existing catalog tests
pass unchanged.

### 4a — Projection from catalog AST to descriptor types

- [x] Implement a projection function that converts
  `ExpressionFunctionCatalog` (owned AST) into
  `Vec<ExpressionFunctionDescriptor>` with `&'static` fields:
  - Construct `signature` strings from `name + "(" + param_names.join(", ") + ")"`
    (with `[...]` for optional and `...` for variadic, matching the current
    signature format).
  - Construct `&'static [ParamType]` from the catalog's parameter vectors.
  - Construct `ReturnType` from the catalog's return shape.
  - Construct `Option<Example>` from the catalog's example data.
  - Use `Box::leak` to promote validated owned `String`s and `Vec`s to
    `&'static` — ONLY after the entire catalog has validated successfully.
- [x] The projection MUST preserve declaration order within overloads (same
  canonical name retains authored order).
- [x] The projection MUST sort the global descriptor list by the globally
  unique `order` field, matching the current `catalog_order + offset` sort.

### 4b — Infallible embedded accessor

- [x] Implement the `LazyLock`-backed accessor:
  ```rust
  pub fn expression_function_descriptors() -> &'static [ExpressionFunctionDescriptor] {
      static CATALOG: LazyLock<Vec<ExpressionFunctionDescriptor>> = LazyLock::new(|| {
          let yaml = include_str!("../../../../docs/schemas/expression-functions.yaml");
          let catalog = parse_expression_function_catalog(yaml)
              .expect("embedded expression-function catalog must parse");
          project_descriptors(&catalog)
      });
      &CATALOG
  }
  ```
  - This replaces the current `functions::expression_function_descriptors()`
    implementation that flattens `registrations()`.
  - The `include_str!` path is relative to the source file location.
  - Invalid checked-in catalog data panics with a precise message, matching
    `darkmatter_base_schema()`.
- [x] Keep the public signature unchanged:
  `pub fn expression_function_descriptors() -> &'static [ExpressionFunctionDescriptor]`
- [x] Keep `generate_expression_function_table()` unchanged — it already
  iterates `expression_function_descriptors()`, so it automatically consumes
  the new source.

### 4c — Crate-visible fallible accessor

- [x] Expose a crate-visible fallible entry point for fixture-based tests:
  ```rust
  pub(crate) fn try_parse_catalog(yaml: &str)
      -> Result<Vec<ExpressionFunctionDescriptor>, CatalogParseError>
  ```
  This wraps the parser + projection without leaking, so malformed fixtures
  produce structured errors without panicking.

### 4d — Parity validation (L1)

- [x] Add a parity test asserting the projected descriptors match the Phase 1
  baseline exactly: same count, same signatures in the same order, same
  categories, same `order` values, same `typed_signature()` output, same
  examples.
- [x] Run all existing catalog tests (`catalog.rs` tests module) unchanged.
  They MUST pass without modification because the public descriptor shape is
  identical.
- [x] Run `descriptor_signature_set_equals_dispatchable_signature_set` — this
  test compares descriptor signatures against `dispatchable_signatures()`.
  At this stage, `dispatchable_signatures()` still reads from
  `FunctionRegistration.descriptors`, so the set should still match. This
  test will become more significant in Phase 5.
- [x] Run `narrative_doc_function_table_matches_catalog` — verifies the
  generated table matches the checked-in doc. The table bytes should be
  unchanged because the descriptors are unchanged.
- [x] Run `every_example_evaluates_to_its_declared_result` — verifies
  executable examples still evaluate correctly.

---

## Phase 5 — Runtime Binding Migration

**Goal**: Reduce `FunctionRegistration` to a runtime-only `FunctionBinding`
(canonical, aliases, evaluation mode, handler) and join it to the parsed
catalog by canonical name with exact bidirectional parity.

**Depends on**: Phase 4 (catalog accessor is the descriptor authority).

**Validation checkpoint**: Bidirectional parity test passes; all dispatch
tests pass at every overload's declared arity; `expression_function_descriptors()`
output is unchanged.

### 5a — Define `FunctionBinding` and `EvaluationMode`

- [x] Introduce `EvaluationMode` enum (`Pure`, `Context`, `Lazy`) — making
  the evaluation mode an explicit field rather than implicit from the
  `FunctionHandler` variant.
- [x] Restructure `FunctionHandler` to hold only the function pointer:
  ```rust
  enum FunctionHandler {
      Pure(PureFn),
      Context(ContextFn),
      // Lazy has no function pointer — the evaluator handles it directly
  }
  ```
- [x] Define `FunctionBinding`:
  ```rust
  struct FunctionBinding {
      canonical: &'static str,
      aliases: &'static [&'static str],
      evaluation: EvaluationMode,
      handler: Option<FunctionHandler>, // None for Lazy
  }
  ```
  Or an equivalent shape that separates evaluation mode from the handler
  pointer. The spec says "conceptually equivalent to" — exact field layout
  is an implementation detail as long as descriptors are NOT carried.

### 5b — Migrate domain registrations to bindings

- [x] In each domain module (`functions/{predicates,collections,strings,
  terminal,dates,paths,skills,markdown_docs}.rs` and `LAZY_REGISTRATIONS`),
  replace `pub(super) const REGISTRATIONS: &[FunctionRegistration]` with
  `pub(super) const BINDINGS: &[FunctionBinding]`.
- [x] Each binding carries only: `canonical`, `aliases`, `evaluation`, and
  `handler`. No `catalog_order`, no `descriptors`.
- [x] The `REGISTRATION_GROUPS` aggregator becomes a `BINDING_GROUPS`
  aggregator. The `registrations()` function becomes `bindings()`.

### 5c — Registry initialization with bidirectional parity

- [x] Implement a registry initialization that:
  - Loads the parsed catalog (via the Phase 4 accessor or a crate-private
    catalog getter).
  - Iterates `bindings()`.
  - Asserts every catalog function has exactly one binding (by `canonical`).
  - Asserts every binding has exactly one catalog function.
  - Rejects alias/canonical collisions.
  - Caches the join in a `LazyLock`.
- [x] The initialization MUST be cached and not change per-call dispatch cost.
- [x] A catalog entry without a binding and a binding without a catalog entry
  are both library defects (panic with a precise message in the infallible
  path, structured error in the fallible path).

### 5d — Dispatch from catalog parameter shapes

- [x] Update `dispatch` and `dispatch_fs` to use `bindings()` instead of
  `registrations()`. The handler matching logic stays the same (match
  canonical/alias, then match `EvaluationMode`/`FunctionHandler` variant).
- [x] Update `dispatchable_signatures()` to return signatures derived from
    the parsed catalog (not from `registration.descriptors`). The catalog
    is the sole authority for signatures.
- [x] Update `dispatchable_canonical_names()` to return canonical names from
    `bindings()`.
- [x] Update `is_fs_function()` to check `EvaluationMode::Context` on
    bindings.
- [x] The handler retains its defensive argument validation and Rust-owned
    evaluation mode. The registry decides whether an authored overload is
    eligible before invoking it (dispatch arity selection is derived from
    catalog parameter shapes, but the handler still validates argument
    domains).

### 5e — Runtime parity tests (L1)

- [x] Update `descriptor_signature_set_equals_dispatchable_signature_set`:
  now the descriptor side comes from the catalog and the runtime side comes
  from `dispatchable_signatures()` (which also reads from the catalog). The
  test still proves bidirectional parity but both sides now originate from
  the same parsed source, with the binding join adding the runtime
  dimension.
- [x] Add a bidirectional canonical-name parity test:
  - Every catalog function name appears in `bindings()`.
  - Every binding canonical appears in the catalog.
  - Fail with a precise message naming the orphan on either side.
- [x] Add an alias/canonical collision rejection test.
- [x] Retain `every_descriptor_overload_is_dispatchable_at_its_declared_arity`
  — proves each overload's handler accepts its declared arity.
- [x] Retain `handler_kinds_dispatch_through_their_intended_paths`.
- [x] Retain `registration_names_aliases_and_signatures_are_unique` (update
  to use bindings + catalog).
- [x] Retain `lazy_operators_are_dispatchable`.
- [x] Retain behavioral tests for overloads, aliases, lazy evaluation,
  remote/local path rules, and injected date behavior.

---

## Phase 6 — Consumer Migration

**Goal**: Migrate every consumer of expression function metadata to use the
parsed catalog projection. No consumer should parse
`expression-functions.yaml` independently.

**Depends on**: Phase 4 (descriptor accessor is the new source) and Phase 5
(bindings work).

**Parallelizable**: The DMLS, Claudine, and doc-generation sub-tasks can
proceed in parallel once Phase 4 is complete. The CLI `md schema about` task
can also proceed in parallel.

**Validation checkpoint**: All consumer tests pass; regenerated documentation
is reviewed.

### 6a — Generated documentation

- [x] Update `darkmatter/docs/topics/darkmatter-expressions.md`:
  - Change the source-attribution prose to name
    `docs/schemas/expression-functions.yaml` as the authored source.
  - Add a "do not edit generated tables directly" notice if not already
    present.
  - Regenerate the function table via `just regen-expr-doc`.
  - Review the diff: existing bytes should remain stable except where the
    spec deliberately improves source-attribution prose or fixes catalog
    drift discovered during migration.
- [x] Run `narrative_doc_function_table_matches_catalog` to verify the
  checked-in table matches the generated output.
- [x] Update any architecture or dependency documentation that still
  describes Rust descriptors as the metadata authority.

### 6b — CLI `md schema about`

- [x] Verify `darkmatter/cli/src/commands/schema/about.rs`
  `expression_function_signatures_markdown()` still works unchanged — it
  already reads `expression_function_descriptors()`, so it automatically
  consumes the new source.
- [x] Run the existing test
  `expression_function_signatures_render_typed_list_formatters` to verify
  `as_csv(list: any[]) -> string | error` still renders.
- [x] Add or update tests asserting `md schema about` includes representative
  authored typed signatures from the catalog.

### 6c — DMLS expression function consumers

- [x] Verify `darkmatter/dmls/src/overlay/expressions.rs`
  `function_descriptors()` and `function_descriptor(name)` still work
  unchanged — they delegate to `expression_function_descriptors()`.
- [x] Verify `darkmatter/dmls/src/providers/dsl.rs` completion and hover
  still work unchanged — they read `typed_signature()` and `description`
  from descriptors.
- [x] Run DMLS L1 tests: `just test` in the darkmatter area (includes DMLS).
- [x] Run DMLS L2 LSP session tests: `just test-l2` in the darkmatter area.
- [x] Verify `function_completion_shape` integration test passes (asserts
  `detail` = `typed_signature()` with `| error` suffix).
- [x] Verify `function_call_hover_known_and_unknown` integration test passes.

### 6d — Claudine consumers

- [x] Verify `claudine/lib/src/composition/lifecycle_actions.rs` still works
  unchanged — `is_known_expression_function_verb()`,
  `expression_function_signature()`, and `all_lifecycle_verbs()` all read
  `expression_function_descriptors()`.
- [x] Verify `claudine/cli/src/commands/context.rs` still works unchanged —
  `claudine context --expressions` reads `expression_function_descriptors()`.
- [x] Run Claudine tests: `just test` in the claudine area.
- [x] Run `claudine/cli/tests/context_command.rs` to verify the context
  command reports expression functions.

### 6e — Catalog example verification

- [x] Verify `every_example_evaluates_to_its_declared_result` still passes —
  executable examples from the catalog are verified through the expression
  evaluator.
- [x] Verify the catalog parser only describes examples; it never executes
  an expression while loading the catalog (assert via the no-side-effects
  test or a new focused test).

---

## Phase 7 — Cleanup and Final Validation

**Goal**: Remove all dead Rust-authored descriptor tables and shared parameter
constants. Prove exact parity. Run the full validation suite.

**Depends on**: Phase 6 (all consumers migrated).

**Validation checkpoint**: No dead code; full test suite passes; lint clean;
documentation updated.

### 7a — Remove dead code

- [x] Remove the shared `P_*` parameter constants and `R_*` return constants
  from `catalog.rs` (`P_ANY`, `P_ANY2`, `P_STRING`, `P_STRING2`, `P_STRING3`,
  `P_NUM`, `P_NUM2`, `P_LIST`, `P_VARIADIC`, `P_OBJ_STRING`, `P_NUM_CONV`,
  `P_ROUND`, `P_FILE`, `P_FILE_STRING`, `P_FILE_OBJ`, `R_BOOL`, `R_BOOL_ERR`,
  `R_NUM`, `R_NUM_ERR`, `R_STRING_ERR`, `R_FILE_ERR`, `R_OBJ_ERR`,
  `R_ANY_ERR`).
- [x] Remove the old `FunctionRegistration` struct and any remaining
  descriptor-bearing registration code paths.
- [x] Remove `catalog_order` from any surviving types (it is replaced by the
  YAML `order` field).
- [x] Remove the old `expression_function_descriptors()` implementation in
  `functions/mod.rs` that flattened `registrations()` — the Phase 4
  `LazyLock` accessor in `catalog.rs` is now the sole implementation.
- [x] Verify `cargo check -p darkmatter -p darkmatter-cli -p dmls` passes
  with no dead-code warnings for removed items.

### 7b — Update skill and documentation

- [x] Update the local Darkmatter skill (`.claude/skills/darkmatter/SKILL.md`)
  to identify:
  - The YAML catalog at `docs/schemas/expression-functions.yaml`.
  - The catalog parser and accessor.
  - The runtime-binding boundary (bindings join to catalog by canonical name).
- [x] Update `docs/topics/darkmatter-expressions.md` authoring guide to
  reference the YAML catalog as the source for function metadata.
- [x] Update any Godless Beauty prose made stale by replacing
  descriptor-bearing Rust registrations with catalog-backed bindings.
- [x] Update `AGENTS.md` if any workspace-layout or convention references
  changed.

### 7c — Full validation suite

- [x] Run `just test` in the darkmatter area (L1 tests for darkmatter,
  darkmatter-cli, dmls).
- [x] Run `just test-l2` in the darkmatter area (L2 real-terminal tests).
- [x] Run `just lint` in the darkmatter area.
- [x] Run `just doctest` in the darkmatter area.
- [x] Run `just check` in the darkmatter area (cargo check + zed check).
- [x] Run `just test` in the claudine area (claudine lib + cli).
- [x] Run `cargo check -p darkmatter -p darkmatter-cli -p dmls -p claudine
  -p claudine-cli` from the repo root.
- [x] Verify all spec acceptance criteria:
  - Removing or renaming a catalog function without changing its Rust
    binding fails a bidirectional parity test.
  - Adding a Rust binding without a catalog entry fails the same invariant.
  - Changing an authored parameter or return type changes
    `typed_signature()`, DMLS detail/hover, and generated documentation from
    the same parsed value.
  - Representative scalar, array, optional, variadic, overloaded, and
    fallible functions round-trip from YAML into the expected descriptors.
  - `as_csv` renders as `as_csv(list: any[]) -> string | error` from the
    authored catalog.
  - Fixtures reject all illegal placements and invariant violations listed
    in the spec.
  - `SimplifiedType::from_keyword("error")` and
    `SimplifiedType::from_keyword("function")` remain `None`.
  - Catalog loading executes no expressions, filesystem probes, shell
    commands, or network requests.
  - Existing expression evaluator tests remain behaviorally unchanged.
  - Generated expression documentation is regenerated and its diff is
    reviewed.
  - DMLS function completion and hover tests pass against the parsed
    catalog.
- [x] Record final validation results in
  `darkmatter/fixes/2026-07-10-function-schemas/phase-7-closeout.md`.

---

## Dependency Graph

```
Phase 1 (Inventory)
   │
   ▼
Phase 2 (AST + Parser) ◄──── Phase 2c (Schema declaration)
   │         │                       │
   │         ▼                       │
   │    Phase 2d (Parser tests)      │
   │                                 │
   ▼                                 ▼
Phase 3 (YAML transcription) ──► Phase 2d (catalog parse test)
   │
   ▼
Phase 4 (Projection + Accessor)
   │
   ├──────────────────────┬──────────────────────┐
   ▼                      ▼                      ▼
Phase 5 (Bindings)   Phase 6a (Docs)        Phase 6b (CLI)
   │                      │                      │
   ▼                      ▼                      ▼
Phase 6c (DMLS)      Phase 6d (Claudine)    Phase 6e (Examples)
   │                      │                      │
   └──────────┬───────────┴──────────────────────┘
              ▼
Phase 7 (Cleanup + Final Validation)
```

## Parallelization Notes

- **Phase 2c** (SimplifiedSchema declaration) can proceed in parallel with
  **Phase 2a** (AST types) and **Phase 2b** (parser) once the YAML shape is
  agreed upon.
- **Phase 3** (YAML transcription) can start as soon as Phase 2a and 2c define
  the shape, even before the parser is fully implemented.
- **Phase 6a** (docs), **Phase 6b** (CLI), **Phase 6c** (DMLS), and **Phase 6d**
  (Claudine) are independent and can proceed in parallel once Phase 4 is
  complete.
- **Phase 7** MUST wait for all of Phase 6 to complete.
