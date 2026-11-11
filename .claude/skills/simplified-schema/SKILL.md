---
name: simplified-schema
description: Expert knowledge for the simplified-schema Rust library, the home of the SimplifiedSchema language (compact YAML type strings such as `name: string(required)`, `age: number(min(0))`) extracted from Darkmatter. Use when working in the `simplified-schema/` package area; using `SimplifiedSchema`, `SchemaSource`, `SchemaRoot`, `SchemaShape`, `PropertyDef`, `Prepared`, `validate_strict`, `validate_soft`, `ValidationSuccess`, `ValidationFailure`, or `SchemaError`; writing, parsing, or lowering SimplifiedSchema type strings to JSON Schema; schema layering (`layer_under`, `layer_over`, `merge_baseline`); conversion and coercion (`coercion`, `coerce`); named-type imports (`Name@file`, `Name@this`, recursive and shared types); `PropertyPath`, `set_at`, `remove_at`, `RemovalPlan`, or `SetterValue`; `non-null`, closed roots, `schema(mapping)` / `property(...)` / `forms(...)` / `declared-required` / `container` declaration options, `denotes_container`, `located_failures`, or `claimed_values`; standalone schema documents; deciding whether schema code belongs in the library or in Darkmatter; fixing a failing schema ownership, dependency-direction, or context-construction guard; or adding the simplified-schema dependency.
last_updated: 2026-11-09T00:00:00Z
---

# simplified-schema

The SimplifiedSchema language as a standalone crate (`simplified-schema/lib`,
imported as `simplified_schema`). It parses, resolves, converts, validates, and
lowers schemas to Draft 2020-12 JSON Schema, and has no dependency on any
Markdown host. Darkmatter, its CLI, DMLS, Claudine, and Messenger (`research`
feature) depend on it directly.

## Library or Darkmatter?

| Lives in `simplified-schema/lib/src/` | Stays in `darkmatter/lib/src/markdown/schemas/` |
|---|---|
| AST and grammar (`simplified/`: `SchemaRoot`, `SchemaShape`, `PropertyDef`, `PropertyAtom`, `TypeExpr`, `DefinitionTable`) | `DarkmatterSchemas`, `EffectiveSchema`, `ValidationOptions`, `PendingPolicy` (`mod.rs`) |
| Named-type resolution, cycles, use-site merges, `merge_baseline` (`resolve.rs`, `resolve/`) | The shipped baseline and global catalog (`catalog.rs`, `embedded.rs::SHIPPED_SOURCES`) |
| JSON Schema lowering, subtyping, queries, serialization | Schema roots (`roots.rs`) and trigger discovery/matching (`triggers/`) |
| Conversion engine (`coercion/`, `coerce.rs`), numeric model (`numeric.rs`) | Pending classification (`coerce.rs::nested_templates`) and eager-file rewrite (`rewrite.rs`) |
| Validators, formats, `x-darkmatter-*` keywords (`validate.rs`, `format.rs`, `file_match.rs`) | `ProblemPositions` (`positions.rs`), completion over an `EffectiveSchema` |
| `SchemaError`, `ValidationProblem`, `SchemaPhase`, `SchemaOrigin`, `ValidationTimings` | `md schema` CLI, compose integration, DMLS projection |
| Dotted paths and their pure edits (`property_path.rs`), the command-line value reader (`setter_value.rs`) | Which document and schema a path or value applies to; the nested text editor (`markdown/hash/path_edit.rs`); setter plans (`compose/setters.rs`) |
| Passive expression syntax (`expression/`), literal-token codec (`pending.rs`) | Expression evaluation and the function registry |
| The `SimplifiedSchema` facade object (`schema/`) | `schemas::resolve` wrappers that pass `SHIPPED_SOURCES` so `Name@darkmatter:<stem>` resolves |

Rule of thumb: if it answers "what does this schema text mean, and does this
value satisfy it?", it belongs in the library. If it answers "which schemas
apply to this Markdown document, and what do we do about it?", it is
Darkmatter policy. Darkmatter-side details: the `darkmatter` skill's
[schema.md](../darkmatter/schema.md).

## Core Principles

- **Content is never a path.** `SimplifiedSchema::try_from(&str)` reads
  document text (standalone schema YAML, Markdown with a frontmatter `$schema`,
  or raw JSON Schema), opens no file, and captures no context.
- **One capture.** Only a *raw* `SchemaSource` (a bare path or reference string)
  reads the process: `schema/capture.rs::request_context` takes cwd, home,
  env, and repository topology (`sniff`) once at factory entry. A host that
  already has a request context passes `SchemaSource::from(path).bound(ctx)`
  and nothing is captured.
- **Validation is passive.** Parsing, resolution of supplied text, conversion,
  lookup, and export perform no I/O beyond reading referenced schema files
  through the bound context. No expression is ever evaluated.
- **Strict never converts; soft converts a copy.** The input `Value` is never
  mutated; `ValidationSuccess::value` is `Cow::Borrowed` when accepted as
  written.
  A failed soft validation reports on a diagnostic copy
  (`coerce.rs::partial_copy`): converted siblings, failing values as written,
  and unexpected keys/tuple elements masked from the retry then put back.
- **One failure flattening.** `ConversionError::located_failures()` /
  `coercion::locate_failures` (null arms dropped, an arm unreachable only on
  `NoConversion` at the union's path) serves annotation, `partial_copy`, and
  hosts; never re-implement it in Darkmatter.
- **One `{{ }}` closing rule.** `expression::scan` ends an expression at the
  quote- and brace-balanced `}}` only when that span parses and the legacy
  first-`}}` span does not (no parsing span ever changes). Editor cursor
  state comes from `scan::expression_region_at` (same lexical rules, no parse
  gate). Any other `{{`/`}}` end detector (`rfind("{{")`, a private depth
  scan) is a second rule: route it through these. Tests:
  `tests/l1/{expression_scanner,expression_regions,scanner_corpus}.rs`.
- **Templates are passive, resolution replays, and a template type is the
  pending type.** `{{ … }}` template constraints and template types parse
  without evaluating anything, every lowering entry fails with
  `SchemaError::UnresolvedSlots`, and only a host (`slots::SlotHost`) resolves
  them, with every read a returned definition causes judged by
  `SlotHost::template_reads`. Read [templates.md](templates.md) before
  touching `slots/`, `TypeExpr::Slot`, or any `Constraint` reader.
- **Layering is atomic.** Each `layer_under` / `layer_over` commits completely
  or not at all, with whole-property replacement.
- **One implementation.** Darkmatter calls the library; it never redefines a
  library-owned item (see [build-and-guards.md](build-and-guards.md)).
- **The library sits below its hosts.** It never depends on Darkmatter, DMLS,
  Claudine, Content Policy, or any CLI, in any manifest table or source.

## Quick Reference

```rust
use serde_json::json;
use simplified_schema::SimplifiedSchema;

let schema = SimplifiedSchema::try_from("$schema:\n  name: string(required)\n  count: number\n")?;

let input = json!({ "name": "Ada", "count": "4" });
assert!(schema.validate_strict(&input).is_err());          // "4" is not a number as written
let accepted = schema.validate_soft(&input).unwrap();       // converted copy
assert!(accepted.is_converted());
assert_eq!(accepted.value["count"], json!(4));

let name = schema.query("name").unwrap().unwrap();          // TypeDefinition view
assert!(name.is_required());
let json_schema = schema.to_json_schema()?;                 // Arc<serde_json::Value>
# Ok::<(), simplified_schema::SimplifiedSchemaError>(())
```

| Need | API |
|---|---|
| Build from text | `SimplifiedSchema::try_from(&str)` |
| Build from a file (`file` feature) | `try_from_yaml`, `try_from_markdown`, `try_from_json_schema`, `resolve` (by extension), each taking `impl Into<SchemaSource>` |
| Use a captured context / editor buffer | `SchemaSource::from(path).bound(ctx).with_text(buffer)` |
| Resolve over every unsaved buffer (targets, imports, examples, unsaved-only files) | `resolve::buffers::with(&SourceBuffers, \|\| …)` (thread-scoped; reads and selection only) |
| Several resolutions in one host operation reading each file once | `resolve::buffers::with_reads_retained(\|\| …)`; read a file you will resolve with `resolve::buffers::read_to_string` (test builds count opens in `observed::Observed::opened`) |
| One named type as its own schema | `schema.select("Person")` |
| Resolve a template, drop replaced properties, slot positions and texts | see [templates.md](templates.md#slot-and-template-apis) |
| Which file a `$schema`/`Name@file`/`example(file)` reference names, its probes, and a missing name's creation path | `resolve::select_schema_file(reference, base_dir, roots, &ctx)` → `SchemaFileSelection` (`into_result()` gives the resolver's error) |
| Validate | `validate_strict` / `validate_soft` → `Result<ValidationSuccess, ValidationFailure>` |
| Layer | `layer_under(&[&s])`, `layer_over(&[&s])`, `as_base`, `override_with` |
| Inspect | `query(path)`, `property`, `properties`, `named_type` → `TypeDefinition`; `FunctionView` |
| Export | `to_json_schema()`, `to_simplified_yaml()`; `provenance()` lists files read |
| Lower-level compile once | `Prepared::new(Arc<Value>, base_dir, ctx)` then `validate_strict`/`validate_soft` |
| Parse only | `parse_yaml_schema`, `parse_schema_declaration`, `parse_property_definition`, `parse_standalone_schema_document` |
| Lower only | `to_json_schema`, `to_json_schema_with_definitions` (needed once a `TypeExpr::Ref` exists) |
| Strings a schema claims for a property | `claimed_values(&resolved, "kind")` → `Vec<String>` / `ClaimProblem` |

`ValidationFailure` is `#[non_exhaustive]`: `InvalidData { problems }`,
`UnavailableCheck { problems }` (a check this build or an in-memory schema
cannot run, such as a file check without a file context), and
`Unprepared { error }` (the schema itself cannot be prepared).

A definition rejected inside a schema document (standalone payload, `types`
table, Markdown frontmatter `$schema`, or an imported named-type file) is
`SchemaError::SchemaDocument { path, message, source, span }`: `source` is the
Referenced `$schema` file classification (`classify_yaml_schema_text`,
`classify_json_schema_text`), envelope admission (`envelope.rs`), and the
ownership line with Darkmatter's `KindRegistry` are in
[schema-documents.md](schema-documents.md).

## Shipped artifacts

`schemas/*.yaml` are served from the checked-in `lib/generated/shipped.json`
(`shipped::registry`; a refused artifact silently falls back to the text).
`simplified_schema::shipped` is the mechanism Darkmatter and Claudine share
(`accept`, `generate`, `check_fresh`, `PreparationMode::Uncached`, the
`*_with_mode` entry points, `ShippedSources::with_prepared`). After editing
shipped YAML or its preparation code, run `just shipped-schemas` (bump
`PREPARATION_REVISION` for code-only changes); see
[shipped-artifacts.md](../../../simplified-schema/docs/topics/shipped-artifacts.md).

## Paths, features, and guards

`PropertyPath`/`SetterValue` rules, the `file`/`terminal`/`work-counters`
features, and the boundary guard table (context construction, dependency
direction, path lookup, language ownership, glob implementation) are in
[build-and-guards.md](build-and-guards.md); read it before fixing a failing guard.

## Testing

Run from `simplified-schema/`:

- `just test` — L1 (`--features work-counters,terminal`), then `test-minimal`
  (`--no-default-features`).
- `just lint` — clippy for the default build, `work-counters,terminal`, and
  `--no-default-features`.
- Doctests are **not** part of `just test`: run `just doctest` or
  `cargo test -p simplified-schema --doc`.
- All tests live in the single `l1` binary (`tests/l1/main.rs`,
  `autotests = false`); a new test file must be added as a module there.
  `tests/l1/shipped_corpus.rs` parses every shipped schema artifact.
- After a change visible to Darkmatter, also run the Darkmatter guard and
  parity tests (`cargo nextest run -p darkmatter --test l1 -E 'test(schema_language_ownership_guard) | test(effective_schema_layering)'`)
  or `just test` in `darkmatter/`. Load the `rust-testing` skill for tiers and
  CI test-input rules.

## Documentation

- [README](../../../simplified-schema/README.md) — overview and install
- [The `SimplifiedSchema` object](../../../simplified-schema/docs/topics/simplified-schema-struct.md) — build, validate, layer, inspect, export
- [Strict and soft validation](../../../simplified-schema/docs/topics/validation.md) — `Prepared`, success and failure shapes
- [Conversion](../../../simplified-schema/docs/topics/conversion.md) — coercion rules, unions, function binding, type errors
- [Diagnostics](../../../simplified-schema/docs/topics/diagnostics.md) — located failures and the error model
- [Property paths](../../../simplified-schema/docs/topics/property-paths.md) · [Setter values](../../../simplified-schema/docs/topics/setter-values.md) — the dotted grammar, its edits, and the command-line value reader
- [Language core](../../../simplified-schema/docs/topics/language-core.md) — modules, supplied sources, what a host keeps
- [Features](../../../simplified-schema/docs/topics/features.md) · [Foundations](../../../simplified-schema/docs/topics/foundations.md) · [Expression syntax](../../../simplified-schema/docs/topics/expression-syntax.md)

Language reference (`simplified-schema/docs/topics/language/`):

- [Index](../../../simplified-schema/docs/topics/language/index.md) — start here
- [Types and constraints](../../../simplified-schema/docs/topics/language/types.md) — grammar, built-in types, files, tuples, `null`/`unknown`
- [Objects and named types](../../../simplified-schema/docs/topics/language/objects-and-named-types.md) — imports, recursive and shared types, dictionaries
- [Unions and function signatures](../../../simplified-schema/docs/topics/language/unions.md) — `|`, named union references, root unions, narrowing
- [Schema documents](../../../simplified-schema/docs/topics/language/schema-documents.md) — resolution, standalone envelopes, raw JSON Schema
- [Selecting a schema file](../../../simplified-schema/docs/topics/selecting-schema-files.md) — `select_schema_file` outcomes, probes, creation paths; `inspect_schema_target`
- [Slots](../../../simplified-schema/docs/topics/language/slots.md) — `{{ … }}` slots, templates, what each operation reports before resolution
- [Subtyping](../../../simplified-schema/docs/topics/language/subtyping.md) · [Custom vocabulary](../../../simplified-schema/docs/topics/custom-vocabulary.md) · [Host capabilities](../../../simplified-schema/docs/topics/host-capabilities.md)
- [declaration-narrowing.md](declaration-narrowing.md) — code map for `non-null`, root `closed`, `schema`/`type-definition` options, `claimed_values`

Darkmatter integration: [definition.md](../../../darkmatter/docs/topics/schemas/definition.md).

## Related Skills

- **darkmatter** — the host: baseline, triggers, schema roots, `md schema`,
  DMLS. Its [schema.md](../darkmatter/schema.md) holds implementation notes for
  both sides.
- **biscuit-file** — `FileReference`, `FileResolutionContext`,
  `GlobReference`, the `file-reference-syntax` / `file-reference` split, and
  the frontmatter envelope helper this crate uses.
- **rust-testing** — test tiers, `just` recipes, CI test-input rules.
