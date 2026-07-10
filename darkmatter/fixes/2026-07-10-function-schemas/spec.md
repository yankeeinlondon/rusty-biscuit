---
status: draft
created: 2026-07-10
area: darkmatter
packages:
  - darkmatter
  - dmls
depends_on:
  - ../2026-07-09-godless-beauty/spec.md#improvement-4--split-expressionfunctionsrs-by-domain-and-single-source-function-registrations
---

# Authored Expression-Function Schemas

## Problem

Darkmatter's base frontmatter schema is authored declaratively in
`docs/schemas/darkmatter.yaml`, embedded with `include_str!`, parsed through the
SimplifiedSchema grammar, and projected into validation, context metadata, DMLS,
and documentation. This gives the schema one readable source of truth that can be
reviewed and reused without editing Rust.

Expression functions do not follow that model. Their public descriptors carry
typed parameters and returns, but the metadata is authored in Rust. The completed
single-sourcing-schema work introduced a separate `DataType` / `ParamType` /
`ReturnType` model because function signatures need concepts that are not legal
frontmatter property types, especially variadic parameters and fallible
`<success> | error` returns. That separation of type domains is correct. It does
not require the catalog itself to be handwritten Rust.

Godless Beauty Improvement 4 is a prerequisite for this fix. It consolidates
each runtime handler and its descriptors into one domain-owned Rust registration,
removes the old parallel public constants, and makes
`expression_function_descriptors()` the public catalog accessor. This
specification starts from that completed architecture. It does not restore the
pre-Godless-Beauty registries or undo the domain split.

After Godless Beauty, adding or changing a function still requires editing Rust
for information that is fundamentally authored data:

- canonical signature and overloads;
- parameter names, types, optionality, and variadic shape;
- return type and fallibility;
- category and display order;
- description; and
- documentation examples and their verification policy.

This makes the metadata harder to review and reuse than the authored Darkmatter
and Claudine schemas. It also leaves documentation coupled to Rust source even
though the runtime handler is the only inherently executable part.

## Goal

Create one authored expression-function catalog under `docs/schemas/` and make
it the sole authority for every handler-free property of an expression function.
Darkmatter MUST parse that catalog into a function-specific AST and project the
existing public descriptors, generated documentation, and DMLS metadata from it.

Runtime handlers, aliases, evaluation mode, and dispatch remain Rust concerns.
The Rust registration layer binds handlers to catalog entries by canonical name;
it does not repeat signatures or descriptive metadata.

Success means that changing a function's signature, description, grouping,
ordering, or example requires changing the authored catalog exactly once. Adding
a new executable function requires one catalog entry and one Rust handler binding.

## Non-goals

This work does not:

- add function types or `error` to frontmatter property types;
- represent a function signature directly with the `SimplifiedSchema` AST;
- move executable handlers, aliases, function pointers, effect classification,
  or lazy-evaluation behavior into YAML;
- dynamically load user-provided function catalogs or plugins;
- change expression syntax, coercion, arity behavior, or evaluation semantics;
- alter the public shape or ordering contract of
  `expression_function_descriptors()` unless separately reviewed; or
- undo the module boundaries introduced by Godless Beauty.

## Design

### Authored catalog

Add:

```text
docs/schemas/expression-functions.yaml
```

The file is checked into the crate, embedded with `include_str!`, parsed once,
and cached. Reading the catalog performs no filesystem or network I/O.

The authored document MUST use ordinary YAML shapes and MUST carry a
SimplifiedSchema declaration that validates the catalog data. The exact
constraint spelling should be finalized during planning against the grammar
available after Godless Beauty, but the semantic shape is:

```yaml
kind: expression-function-catalog

$schema:
  kind: string(required)
  functions:
    - object(required)

functions:
  - name: as_csv
    category: Collections
    order: 40
    description: Formats a list as comma-separated values.
    overloads:
      - parameters:
          - name: list
            type: any[]
        returns: string | error
        example:
          expression: as_csv(["a", "b"])
          result: "a, b"
          verification: executable
```

The illustrative `$schema` above is intentionally abbreviated; the implemented
schema MUST validate every catalog field and nested shape rather than accepting
an unconstrained object. If the current SimplifiedSchema grammar cannot express
a necessary catalog invariant, keep the nearest structural validation in the
authored schema and enforce the remaining semantic invariant in the
function-catalog parser. Do not extend frontmatter's type vocabulary merely to
make this catalog more convenient.

Canonical function names belong at function level. Overloads carry parameters,
returns, and any overload-specific example. Category and order are authored
once per function unless a demonstrated consumer requires overload-specific
ordering.

### Function-signature type domain

Introduce or retain a function-specific catalog AST distinct from
`SimplifiedSchema`. Its concepts include:

- data types aligned with the canonical SimplifiedSchema primitive vocabulary;
- scalar and array shapes;
- required, optional, and variadic parameters;
- return unions containing one success type and an optional `error` member; and
- complete function signatures containing named parameters and a return.

The parser SHOULD reuse `SimplifiedType::from_keyword` or a shared primitive
keyword conversion where that avoids duplicating the canonical vocabulary.
Function-only structural concepts remain outside `SimplifiedType`.

The following exclusions MUST remain structural:

- `error` is legal only as a return union member;
- function types are never legal frontmatter property types;
- parameters cannot use `error`;
- returns must contain exactly one success type and at most one `error` member;
- an optional parameter cannot precede a later required parameter;
- a variadic parameter is last and cannot also be optional; and
- parameter names within one overload are unique.

`DataType`, `ParamType`, and `ReturnType` may remain as public descriptor-facing
projection types if preserving them avoids an unnecessary API break. They MUST
no longer be independently authored authorities.

### Parsing and caching

Expose a library accessor appropriate to the existing schema API, for example:

```rust
pub fn expression_function_catalog() -> &'static ExpressionFunctionCatalog
```

The accessor embeds the authored YAML and initializes a `OnceLock` or
`LazyLock`. Invalid checked-in catalog data is a library-build/repository defect,
so the infallible embedded accessor MAY panic with a precise message, matching
`darkmatter_base_schema()`. The underlying parser SHOULD be separately exposed
or test-visible as a fallible function so malformed fixtures can produce
structured errors without panicking.

Errors MUST identify the function, overload, and field where possible. Parsing
must reject duplicate function names, duplicate rendered signatures, unknown
type keywords, illegal `error` placement, invalid parameter ordering, and
unknown verification modes.

Declaration order in YAML is preserved. Explicit `order` remains the stable
display contract if existing consumers depend on it; ties and duplicate order
values MUST have a documented deterministic rule or be rejected.

### Runtime binding after Godless Beauty

Godless Beauty's domain modules continue to own handlers and their runtime-only
metadata. Replace descriptor-bearing registrations with bindings conceptually
equivalent to:

```rust
struct FunctionBinding {
    canonical: &'static str,
    aliases: &'static [&'static str],
    handler: FunctionHandler,
}
```

The aggregated runtime registry joins each binding to exactly one authored
catalog function by `canonical`. Descriptors are projected from the authored
catalog rather than stored beside the handler.

This leaves two intentionally different sources:

1. authored YAML says what a function is to callers; and
2. Rust says how that function executes.

Their join is unavoidable because YAML cannot contain a function pointer. Exact
bidirectional parity MUST be checked during tests and SHOULD be checked during
registry initialization if that can be done without changing normal dispatch
costs. A catalog entry without a binding and a binding without a catalog entry
are both errors.

Aliases remain Rust-authored because they are dispatch compatibility behavior,
not part of the canonical documented signature. If aliases are later shown in
public documentation, that change requires a separate decision about moving
them into the authored catalog.

### Descriptor and consumer compatibility

`expression_function_descriptors()` remains the public handler-free catalog
surface established by Godless Beauty. It MUST return descriptors in the same
stable display order and retain typed-signature rendering.

Migrate all consumers to the parsed projection:

- generated `docs/topics/darkmatter-expressions.md` content;
- `md schema about` expression-function output;
- DMLS function hover, completion detail, and documentation;
- catalog example verification; and
- any Claudine consumer that remains after Godless Beauty's workspace migration.

No consumer should parse `expression-functions.yaml` independently. The
Darkmatter library parser and catalog accessor are the single semantic
authority.

### Documentation generation

The generated expression-function documentation MUST identify
`docs/schemas/expression-functions.yaml` as its source and MUST NOT invite users
to edit generated tables directly. Existing generated bytes should remain stable
except where this specification deliberately improves source-attribution prose
or fixes catalog drift discovered during migration.

Examples remain executable through the existing expression evaluator. The
catalog parser only describes them; it never executes an expression while
loading the catalog.

## Migration

1. Complete and merge Godless Beauty Improvement 4.
2. Inventory the post-Godless-Beauty catalog accessor, domain bindings,
   descriptor fields, consumers, generated docs, and invariants.
3. Define the authored catalog's complete SimplifiedSchema and
   function-signature parser types.
4. Transcribe every existing descriptor into
   `docs/schemas/expression-functions.yaml` without semantic edits.
5. Project `ExpressionFunctionDescriptor` values from the parsed catalog.
6. Reduce the Godless Beauty registrations to runtime bindings and join them to
   the parsed catalog.
7. Migrate documentation, DMLS, tests, and any remaining Claudine consumers.
8. Remove Rust-authored descriptor tables and shared parameter constants only
   after exact parity is proven.

The transcription SHOULD be isolated from intentional catalog corrections. If
the migration exposes stale descriptions, signatures, examples, or comments,
assume the runtime code is correct, record the drift, and fix the authored
catalog in a separate reviewable change or clearly separated commit.

## Requirements

1. `docs/schemas/expression-functions.yaml` MUST be the sole authored authority
   for canonical function metadata and overload signatures.
2. The file MUST be embedded in the library and parsed without runtime file or
   network access.
3. The catalog document MUST be structurally validated using SimplifiedSchema;
   function-domain semantic rules MUST be enforced by a dedicated parser.
4. Function-only concepts MUST NOT expand the legal frontmatter property type
   domain.
5. Runtime handlers MUST remain Rust functions grouped by the domain modules
   created by Godless Beauty.
6. Every catalog function MUST have exactly one runtime binding, and every
   runtime binding MUST have exactly one catalog function.
7. Every authored overload MUST be accepted by its bound runtime handler at the
   declared arity.
8. Public descriptor ordering, typed-signature rendering, DMLS presentation,
   and evaluator behavior MUST remain stable through the migration.
9. Generated documentation and verified examples MUST consume the parsed
   catalog, not a second transcription.
10. Malformed catalog fixtures MUST produce actionable structured errors; the
    embedded checked-in accessor may treat invalid data as a library defect.

## Acceptance Criteria

- Removing or renaming a catalog function without changing its Rust binding
  fails a bidirectional parity test.
- Adding a Rust binding without a catalog entry fails the same invariant.
- Changing an authored parameter or return type changes
  `typed_signature()`, DMLS detail/hover, and generated documentation from the
  same parsed value.
- Representative scalar, array, optional, variadic, overloaded, and fallible
  functions round-trip from YAML into the expected descriptors.
- `as_csv` renders as `as_csv(list: any[]) -> string | error` from the authored
  catalog.
- Fixtures reject `error` parameters, `error`-only returns, multiple success
  return members, required parameters after optional parameters, non-final
  variadics, duplicate parameter names, duplicate signatures, and unknown
  primitive keywords.
- `SimplifiedType::from_keyword("error")` and
  `SimplifiedType::from_keyword("function")` remain `None`.
- Catalog loading executes no expressions, filesystem probes, shell commands,
  or network requests.
- Existing expression evaluator tests remain behaviorally unchanged.
- Generated expression documentation is regenerated and its diff is reviewed.
- DMLS function completion and hover tests pass against the parsed catalog.

## Test Plan

### Level 1 — parser and catalog

- Parse the complete checked-in catalog and assert function/signature counts
  against the post-Godless-Beauty baseline.
- Parse focused fixtures for every supported parameter and return shape.
- Assert declaration/display ordering and overload grouping.
- Assert precise failures for every illegal type-domain placement and catalog
  invariant listed above.
- Assert that the authored document validates against its SimplifiedSchema.
- Assert descriptor projection preserves descriptions, categories, examples,
  verification policies, and typed signatures.

### Level 1 — runtime parity

- Prove exact canonical-name equality between catalog functions and runtime
  bindings.
- Prove each overload's name matches its binding and its declared arity reaches
  the intended pure, context-aware, or lazy dispatch path.
- Retain Godless Beauty's uniqueness and alias-collision tests.
- Retain behavioral tests for overloads, aliases, lazy evaluation, remote/local
  path rules, and injected date behavior.

### Consumer tests

- Regenerate the expression documentation and compare it with the checked-in
  output.
- Assert `md schema about` includes representative authored typed signatures.
- Assert DMLS completion detail and hover documentation use the catalog-derived
  signature and description.
- Run any remaining Claudine catalog-consumer tests identified after Godless
  Beauty completes.

### Package validation

- Run the canonical Darkmatter Level 1 tests and lint recipes from the local
  rust-testing skill.
- Run focused DMLS tests for expression hover/completion.
- Run broader package-area validation in proportion to the final workspace
  consumer set.

## Documentation and maintenance

Implementation MUST update:

- `docs/topics/darkmatter-expressions.md` to name the authored catalog;
- architecture or dependency documentation that still describes Rust
  descriptors as the metadata authority;
- the local Darkmatter skill to identify the YAML catalog, parser, accessor,
  and runtime-binding boundary; and
- any Godless Beauty prose made stale by replacing descriptor-bearing Rust
  registrations with catalog-backed bindings.

The authored YAML is source, not generated output. Generated tables remain
derived artifacts.

## Open Questions

1. Should `expression_function_catalog()` be public, or should the existing
   descriptor accessor remain the only public surface while the richer AST stays
   crate-private?
2. Should explicit `order` be required, or is YAML declaration order sufficient
   as the long-term display contract? Preserving the current explicit order is
   the safer migration default.
3. Should examples remain attached to overloads, or should functions with one
   overload allow a shorthand function-level example? A single canonical shape
   is preferable unless the shorthand materially improves authoring.
4. Can the current SimplifiedSchema grammar fully validate the nested catalog
   shape, or does it need a narrowly scoped, generally useful enhancement? Any
   enhancement must remain in the data-schema domain and must not add function
   or `error` property types.
