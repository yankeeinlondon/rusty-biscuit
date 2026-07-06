---
status: draft
created: 2026-07-04
schema: ../../docs/schemas/darkmatter.yaml
review_iterations: 5
---

# Darkmatter Base Frontmatter Schema

## Context

Darkmatter already has a `SimplifiedSchema` grammar, schema validation stage,
baseline schema merge support, and a draft baseline schema at
`darkmatter/docs/schemas/darkmatter.yaml`. Today, however, Darkmatter's own
frontmatter contract is spread across compose, render, hash, style, and docs.

The base schema should make Darkmatter-owned frontmatter properties visible in
one place, usable by documentation, runtime validation, generated source, and
future editor tooling.

## Goals

1. Make `darkmatter/docs/schemas/darkmatter.yaml` the authored source of truth
   for Darkmatter-owned baseline frontmatter properties.
2. Keep `SimplifiedSchema` as the schema language. Do not introduce a second
   schema metadata format for this feature.
3. Use existing `SimplifiedSchema` metadata syntax, especially `-> description`
   and `default(...)`, to document properties directly in the schema.
4. Extend the `SimplifiedSchema` parser to accept nested YAML mappings as object
   schema definitions, while preserving the existing quoted object-literal
   syntax.
5. Add a `generated` constraint so schemas can describe values supplied by
   Darkmatter runtime context rather than authored in static frontmatter.
6. Add documentation that transcludes the schema file so the documentation and
   validation source cannot drift.
7. Expose the base schema from the Darkmatter library for compose callers,
   schema validation callers, and downstream packages such as Claudine.
8. Preserve author extensibility outside Darkmatter-owned namespaces: the base
   schema validates known Darkmatter properties but must not close the
   top-level frontmatter namespace to user properties.

## Non-Goals

1. Do not make the base schema reject unknown frontmatter keys.
2. Do not fully model every nested runtime DSL in v1. Runtime parsers remain
   authoritative for rich nested surfaces such as `style`.
3. Do not add a long-form schema-entry grammar in this feature.
4. Do not require build scripts to rewrite source files during ordinary builds.
5. Do not change `$schema` merge semantics. Document-level schema declarations
   continue to win over baseline properties on conflict.
6. Do not preserve deprecated root-level `hr` as part of the public schema
   contract.

## Source Of Truth

The source of truth is:

```text
darkmatter/docs/schemas/darkmatter.yaml
```

The file remains an ordinary referenced `SimplifiedSchema` YAML file:

```yaml
$schema:
  title: "string -> Human-readable document title."
```

Property descriptions should be authored in the schema with `-> description`
where the property is public or user-facing. Defaults may use the existing
`default(...)` constraint when the default is part of the semantic contract.
As with current `SimplifiedSchema`, default values are metadata; applying a
default remains the responsibility of the consumer.

Object literals remain valid for stable nested shapes:

```yaml
$schema:
  obj: "{ foo: string, bar: number, baz: boolean }"
```

The schema may also use the proposed nested mapping syntax:

```yaml
$schema:
  obj:
    foo: string
    bar: number
    baz: boolean
```

Broad `object` or `any` remains appropriate when another Darkmatter parser is
the more precise authority, when the shape is intentionally open, or when the
shape is still moving.

## Nested Object Syntax Requirement

The `SimplifiedSchema` parser must add a nested object form before these base
schemas can be consumed at runtime. The current inline object literal remains
valid:

```yaml
$schema:
  obj: "{ foo: string, bar: number, baz: boolean }"
```

The new form is equivalent:

```yaml
$schema:
  obj:
    foo: string
    bar: number
    baz: boolean
```

Parser rules:

1. A mapping value inside a `SimplifiedSchema` property definition means
   "object with these properties."
2. Mapping leaf values use the existing type-expression grammar, including
   constraints and `-> description`.
3. Mapping values may nest recursively.
4. Sequence values continue to mean property unions. A sequence arm may be a
   type-expression string or a nested mapping object shape.
5. The old quoted object-literal grammar remains valid and semantically
   equivalent.
6. This is not the future long-form metadata grammar. A nested mapping without
   an explicit `type:` key is an object shape, not a property descriptor object.

For example:

```yaml
$schema:
  hash:
    - "string -> shorthand hash"
    - kind: "enum(simple, structured, detailed)"
      value: any
      ignored: string[]
```

This lowers to a union of a string shorthand and an object with `kind`, `value`,
and `ignored` properties.

## Generated Constraint Requirement

`SimplifiedSchema` must add a universal `generated` constraint. It applies to
every schema type the same way `required` is syntactically available across
types, but it describes ownership/supply semantics rather than nullability.

Generated properties are available in the effective runtime schema because
Darkmatter, Claudine, or another host provides them. They are not required from
the static authored frontmatter document.

Initial example:

```yaml
$schema:
  ctx:
    today: "date(generated; required) -> today's date, provided by Darkmatter context capture"
    repo_root: "string(generated; required) -> repository root, provided by Darkmatter context capture"
```

Semantics:

1. Static authored-document validation must not fail because a `generated`
   property is absent.
2. LSP/editor tooling should expose generated properties for completion,
   hover, and diagnostics in expression surfaces where the generated value is
   available.
3. Runtime/effective validation may validate a generated property after the
   host has supplied it.
4. `generated` and `required` are orthogonal. `required` controls the type:
   `foo: string` means the effective value is `string | null`, while
   `foo: string(required)` means the effective value is `string`.
5. `generated` means the host supplies the value and static page authors are
   not expected to define it. A generated field can still be required when the
   runtime guarantees the value is available and non-null in the context where
   it is exposed.
6. The base Darkmatter `ctx` schema should mark runtime-guaranteed context
   values as both `generated` and `required`.

Darkmatter's `ctx` object is the motivating case: `ctx.*` values are always
available to interpolation once the relevant context group is captured, but a
static Markdown page is not expected to declare those properties in its
frontmatter.

## Baseline Property Scope

The base schema should include frontmatter properties Darkmatter defines,
interprets, mutates, or treats specially across the public pipeline. Initial
scope includes:

- `$schema`
- `title`, `description`, `tags`, `draft`, `metadata`
- `last_updated`, `hash`
- `style`
- `change`
- `replace`, `ctx`
- `prologue`, `epilogue`
- `ignore_invalid`
- `interpolate_code_blocks`

The schema review must compare this list against the compose, render, style,
hash, delta/change, and docs surfaces before implementation closes.

## Deprecated Root `hr` Removal

The base schema must not include the deprecated root-level `hr` frontmatter
property. Horizontal-rule styling belongs under `style.hr`.

As part of implementing this feature, Darkmatter should remove the remaining
runtime compatibility paths that read root `hr`:

- `darkmatter::style::parse::from_frontmatter` currently merges top-level
  `hr` into `style.hr` as a deprecated alias.
- The render-tree horizontal-rule defaults path currently reads root `hr`
  directly for bare-rule defaults.

The implementation should delete those compatibility paths, update tests and
docs that still advertise root `hr`, and make `style.hr` the only supported
horizontal-rule frontmatter surface. If a temporary migration diagnostic is
needed, it should be outside the base schema contract and clearly point authors
to `style.hr`.

## Documentation Contract

Add a documentation file next to the schema:

```text
darkmatter/docs/schemas/darkmatter-schema.md
```

It should explain that Darkmatter adds a base frontmatter schema and then
transclude the schema:

```md
# Darkmatter Schema Defaults

The darkmatter library, by default, adds a base schema to the frontmatter of
your documents. This schema is defined using the same `SimplifiedSchema`
language that Markdown authors can use with the `$schema` frontmatter property.

::code ./darkmatter.yaml
```

The documentation file may add prose for merge behavior, document `$schema`
precedence, and the fact that unknown frontmatter keys remain allowed.

## Library Integration

Darkmatter should expose the base schema as a library surface. The exact symbol
names can be decided during implementation, but the API should support:

- loading the authored base schema as `SimplifiedSchema`
- producing the compiled JSON Schema used by validators
- injecting the base schema into `ComposeOptions`
- validating the base schema during tests

Candidate API shape:

```rust
use darkmatter::markdown::schemas::SimplifiedSchema;

pub fn darkmatter_base_schema() -> SimplifiedSchema;
```

The implementation may use `include_str!` against the authored YAML or a
checked-in generated Rust artifact. If code generation is added, generated
files should be checked in so schema changes are reviewable and ordinary builds
remain deterministic on macOS, Windows, and Linux.

## CLI Integration

The schema validation CLI already accepts an explicit baseline schema file and
the `BASELINE_SCHEMA` environment variable. This feature should not change that
contract.

Open implementation question: whether `md compose` should automatically inject
the Darkmatter base schema by default. If it does, it must preserve current
document `$schema` precedence and continue allowing unknown user keys.

## Code Generation

V1 does not need a broad code generation framework. If implementation needs
generated artifacts, keep the generator small and deterministic:

```text
darkmatter/docs/schemas/darkmatter.yaml
  -> parse referenced SimplifiedSchema
  -> validate baseline compatibility
  -> emit Rust constant or checked-in generated module
  -> optionally emit documentation tables later
```

The generator should be callable from a `just` recipe rather than silently
rewriting files in `build.rs`.

## Future: Long Form Schema Entries

If metadata needs outgrow the current short-form grammar, `SimplifiedSchema`
may add a long-form property entry later:

```yaml
$schema:
  title:
    type: string
    description: Human-readable document title.
    default: Untitled
    examples:
      - Project Plan
      - Release Notes
```

That form would be equivalent to:

```yaml
$schema:
  title: "string(default(Untitled)) -> Human-readable document title."
```

This is explicitly future work. V1 should use the existing grammar.

## Testing

Tests should verify:

1. `darkmatter/docs/schemas/darkmatter.yaml` parses as referenced
   `SimplifiedSchema`.
2. The parsed schema converts to a baseline-compatible JSON Schema.
3. Known valid examples pass validation.
4. Invalid known-property values fail validation.
5. Unknown user-defined frontmatter keys remain accepted.
6. Document `$schema` definitions override baseline properties on conflict.
7. The documentation transclusion path points at the same schema file used by
   source integration.
8. Nested mapping object syntax lowers to the same JSON Schema as the existing
   quoted object-literal syntax.
9. Sequence union arms accept nested mapping object shapes.
10. `generated` properties are omitted from static-document required checks for
    authored frontmatter, but retain their `required` type/nullability semantics
    in runtime/effective schemas and LSP/completion metadata.

## Open Questions

1. Should `md compose` inject the base schema by default, or should the default
   be limited to library callers until after a compatibility pass?
2. Should `style` remain `object` in the baseline, or should stable top-level
   style buckets be modeled with inline-object modeling?
3. What is the correct v1 validation shape for `change`?
4. Should the schema docs include generated property tables in addition to the
   transcluded YAML?

### Resolutions (Phase 3)

1. **`md compose` baseline injection** — `md compose` injects the Darkmatter
   base schema by default so the CLI matches the library convenience default.
   Use `--no-baseline-schema` or `DARKMATTER_NO_BASELINE_SCHEMA=1` for raw
   compose behavior, or `--baseline-schema PATH` to replace the default with a
   custom SimplifiedSchema YAML baseline. The `md schema validate` baseline
   contract remains unchanged.
2. **`style` shape** — `style` remains a broad `object` in the v1 baseline. The
   `darkmatter::style` runtime parser and `style::descriptor::SCHEMA` are the
   authoritative validators; the shape is still moving and is explicitly out of
   scope for full modeling per Non-Goal 2.
3. **`change` shape** — `change` stays `any` in v1. The delta/change surface does
   not yet expose a stable contract worth narrowing in the baseline, matching
   Non-Goal 2.
4. **Generated property tables in docs** — Transclusion only for v1. The docs
   page will include the schema YAML verbatim via `::code ./darkmatter.yaml` and
   explanatory prose. Generated tables are future work once the schema language
   supports stable generated-artifact generation.

### Audit Findings (Phase 3)

The baseline property list was compared against the compose pipeline, render
 tree, `style::parse`, the hash subsystem, delta/change, and documentation
 surfaces. No missing Darkmatter-owned frontmatter properties were found. The
 following corrections were applied to `darkmatter/docs/schemas/darkmatter.yaml`
 to align the schema with runtime behavior:

- `ctx` is modeled as a closed generated object because it is a
  Darkmatter-owned runtime namespace. Runtime context merge behavior remains a
  compatibility path for documents that already define `ctx`, but authored
  custom `ctx.*` keys are discouraged and are not part of the base-schema
  extensibility contract.
- The deprecated root-level `hr` property is absent from the baseline, and
  `style.hr.*` remains the only horizontal-rule surface.

Generated `ctx.*` annotations are part of the v1 base schema. Static authored
documents may omit `ctx` entirely; effective runtime validation can still type
check host-supplied context values when they are present.
