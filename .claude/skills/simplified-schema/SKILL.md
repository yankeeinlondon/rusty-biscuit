---
name: simplified-schema
description: Expert knowledge for the simplified-schema Rust library, the home of the SimplifiedSchema language (compact YAML type strings such as `name: string(required)`, `age: number(min(0))`) extracted from Darkmatter. Use when working in the `simplified-schema/` package area; using `SimplifiedSchema`, `SchemaSource`, `SchemaRoot`, `SchemaShape`, `PropertyDef`, `Prepared`, `validate_strict`, `validate_soft`, `ValidationSuccess`, `ValidationFailure`, `AppliedDefault`, `DefaultPlan`, `prepare_defaults`, `ConflictingDefaults`, or `SchemaError`; required properties with a `default(...)`; writing, parsing, or lowering SimplifiedSchema type strings to JSON Schema; schema layering (`layer_under`, `layer_over`, `merge_baseline`); conversion and coercion (`coercion`, `coerce`); named-type imports (`Name@file`, `Name@this`, recursive and shared types); `PropertyPath`, `set_at`, `remove_at`, `RemovalPlan`, or `SetterValue`; `required` presence versus null admission, `not-empty` (and the removed `non-null`), closed roots, `schema(mapping)` / `property(...)` / `forms(...)` / `declared-required` / `container` declaration options, `denotes_container`, `located_failures`, or `claimed_values`; standalone schema documents; schema triggers (`kind: schema-trigger`, `simplified_schema::triggers`, `TriggerRegistry`, `scan`, `scan_with_source`, `EmbeddedSchemaBundle`, `prepare_trigger_bundle`), schema roots (`SchemaRoots`, `SCHEMAS_DIR`), or whole YAML documents selected by `kind` (`yaml_documents`, `KindRegistry`, `KindHost`, `LoadedYamlDocument`, kind checks); deciding whether schema code belongs in the library or in Darkmatter; fixing a failing schema ownership, dependency-direction, or context-construction guard; or adding the simplified-schema dependency.
last_updated: 2026-11-15T00:00:00Z
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
| JSON Schema lowering, subtyping, queries, serialization | Darkmatter's shipped artifact (`shipped.rs`), including the prepared kind definitions it records (`kind_definitions`) |
| Conversion engine (`coercion/`, `coerce.rs`), numeric model (`numeric.rs`) | Pending classification (`coerce.rs::nested_templates`) and eager-file rewrite (`rewrite.rs`) |
| Validators, formats, `x-darkmatter-*` keywords (`validate.rs`, `format.rs`, `file_match.rs`) | `ProblemPositions` (`positions.rs`), completion over an `EffectiveSchema` |
| `SchemaError`, `ValidationProblem`, `SchemaPhase`, `SchemaOrigin`, `ValidationTimings` | `md schema` CLI, compose integration, DMLS projection |
| Dotted paths and their pure edits (`property_path.rs`), the command-line value reader (`setter_value.rs`) | Which document and schema a path or value applies to; the nested text editor (`markdown/hash/path_edit.rs`); setter plans (`compose/setters.rs`) |
| Passive expression syntax (`expression/`), literal-token codec (`pending.rs`) | Expression evaluation and the function registry |
| The `SimplifiedSchema` facade object (`schema/`) | `schemas::resolve` wrappers that pass `SHIPPED_SOURCES` so `Name@darkmatter:<stem>` resolves |
| Schema roots (`roots.rs`), triggers (`triggers/`: grammar, envelope, lint, matcher, discovery, assembly, prepared bundles, findings, and the identifying-trigger proof `identify.rs::{admit_identifying_trigger, identifies}` host registration uses), `EmbeddedSchemaBundle` (`embedded.rs`), the trigger definition (`schemas/schema-trigger.yaml`) | Layering matched trigger payloads into an `EffectiveSchema`; the `triggers.rs` wrappers (`scan`, `scan_with_embedded`, `scan_with_source`, `prepare_trigger_bundle`) that add `SHIPPED_SOURCES` |
| Whole YAML documents (`yaml_documents/`): the loader, `KindRegistry`, selection, kind checks | The adapter (`yaml_documents/mod.rs`): `kind_host()`, `shipped_kind_registry()`, `YamlContract`, `YamlValidation`, `Route` |

Rule of thumb: if it answers "what does this schema text mean, and does this
value satisfy it?", or "which schema-kind documents exist for this context,
and which one does this document select?", it belongs in the library. If it answers "which schemas
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
- **A missing key and `null` are one value; `required` is presence only.**
  Defaults (required or not) fill missing or `null` values in a step of
  their own (`defaults::{DefaultPlan, prepare_defaults}`) before any
  validator judges the candidate; there is no "already valid" shortcut.
  Strict success stays `Cow::Borrowed` and lists `defaults`; soft applies
  them. Conflicting defaults fail at lowering/`Prepared::new`, never by arm
  at run time; the one exception is a key no schema names whose matching
  patterns/catch-alls disagree, a per-key `ConflictingDefaults` raised only
  when that key's default is needed (see [defaults.md](defaults.md)). Exported JSON Schema keeps `required`; `default` is only an
  annotation there. Never write a second defaulting walk in a host. Read
  [defaults.md](defaults.md) before touching presence, defaults, or
  `not-empty`.
- **Defaults keep their provenance.** Each `AppliedDefault` carries `pointer`
  (the `default` keyword's JSON Pointer in the lowered schema) and an `origin`
  a host fills. A problem the completed value still has at or below an
  inserted path (an eager file default with no target) is the default's:
  `defaults::attribute_problems` sets `ValidationProblem::default`, names it in
  the message, and clears line/column; a host judging its own completed
  candidate calls it too. Expansion inlines `Name@file` types without their
  file, so resolution records each imported default's declaring file in
  `ResolvedSchema::default_files` (`ImportEngine::default_files`, walked over
  the authored schema after expansion); Darkmatter's `EffectiveSchema` prefers
  it over the layer origin.
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
- **Host inputs are explicit.** Every entry point that resolves a trigger
  payload or prepares a kind definition takes the host's `ShippedSources`
  (`triggers::scan(context, sources)`, `scan_with_source(…, sources)`,
  `prepare_trigger_bundle(bundle, mode, sources)`, `TriggerRegistry::new`,
  `assemble::resolve_trigger_payload`); a kind registry takes a `KindHost { sources,
  shipped }` (`KindRegistry::build(context, source, scope, host)`,
  `from_filesystem(context, scope, host)`, `shipped_from(definitions)`).
  Never read a host's namespace from a global.

## Quick Reference

```rust
use serde_json::json;
use simplified_schema::SimplifiedSchema;

let schema = SimplifiedSchema::try_from("$schema:\n  name: string(required)\n  count: number\n")?;

let input = json!({ "name": "Ada", "count": "4" });
assert!(schema.validate_strict(&input).is_err());          // "4" is not a number as written
let accepted = schema.validate_soft(&input).unwrap();       // converted copy
assert!(accepted.is_modified());                          // converted or defaulted copy
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
grammar's typed error unchanged, `span` a byte range in the exact supplied
text. Referenced `$schema` file classification (`classify_yaml_schema_text`,
`classify_json_schema_text`), envelope admission (`envelope.rs`), and the
ownership line with Darkmatter's `KindRegistry` are in
[schema-documents.md](schema-documents.md).

## Shipped artifacts and identities

`schemas/*.yaml` are served from the checked-in `lib/generated/shipped.json`.
After editing shipped YAML or its preparation code, run the root
`just shipped-schemas`; a code-only change bumps `PREPARATION_REVISION` of
every owner whose payload it changes. Validator and phase-projection
identities (`SchemaIdentity`, `MERGE_ALGORITHM_REVISION`,
`PROJECTION_REVISION`) are pinned in `tests/l1/schema_identities.rs`. Details:
[shipped-and-identities.md](shipped-and-identities.md).

## Paths, features, and guards

`PropertyPath`/`SetterValue` rules, the `file`/`terminal`/`work-counters`
features, and the boundary guard table (context construction, dependency
direction, path lookup, language ownership, glob implementation) are in
[build-and-guards.md](build-and-guards.md); read it before fixing a failing guard.

## Testing

Run from `simplified-schema/`: `just test` (L1 with `work-counters,terminal`,
then `--no-default-features`), `just lint` (all three builds), and
`just doctest` (doctests are not part of `just test`). Every test lives in
the single `l1` binary (`tests/l1/main.rs`, `autotests = false`): add a new
file as a module there. After a change visible to Darkmatter, also run
`cargo nextest run -p darkmatter --test l1 -E 'test(schema_language_ownership_guard) | test(effective_schema_layering)'`
or `just test` in `darkmatter/`. Load the `rust-testing` skill for tiers and
CI test-input rules.

## Documentation

Topic pages (`simplified-schema/docs/topics/`): [README](../../../simplified-schema/README.md) · [`SimplifiedSchema` object](../../../simplified-schema/docs/topics/simplified-schema-struct.md) · [Strict and soft validation](../../../simplified-schema/docs/topics/validation.md) (defaults, the required/`null` table, export limits) · [Conversion](../../../simplified-schema/docs/topics/conversion.md) · [Diagnostics](../../../simplified-schema/docs/topics/diagnostics.md) · [Property paths](../../../simplified-schema/docs/topics/property-paths.md) · [Setter values](../../../simplified-schema/docs/topics/setter-values.md) · [Language core](../../../simplified-schema/docs/topics/language-core.md) · [Features](../../../simplified-schema/docs/topics/features.md) · [Foundations](../../../simplified-schema/docs/topics/foundations.md) · [Expression syntax](../../../simplified-schema/docs/topics/expression-syntax.md) · [Selecting a schema file](../../../simplified-schema/docs/topics/selecting-schema-files.md) · [Custom vocabulary](../../../simplified-schema/docs/topics/custom-vocabulary.md) · [Host capabilities](../../../simplified-schema/docs/topics/host-capabilities.md).

Language reference (`docs/topics/language/`): [Index](../../../simplified-schema/docs/topics/language/index.md) (start here) · [Types and constraints](../../../simplified-schema/docs/topics/language/types.md) (null admission, `not-empty`, phases) · [Objects and named types](../../../simplified-schema/docs/topics/language/objects-and-named-types.md) · [Unions and function signatures](../../../simplified-schema/docs/topics/language/unions.md) (defaults in unions) · [Schema documents](../../../simplified-schema/docs/topics/language/schema-documents.md) · [Slots](../../../simplified-schema/docs/topics/language/slots.md) · [Subtyping](../../../simplified-schema/docs/topics/language/subtyping.md).

Code maps in this skill: [defaults.md](defaults.md) (presence, `null`, default application) · [declaration-narrowing.md](declaration-narrowing.md) (`not-empty`, `closed`, meta-type options, `claimed_values`) · [schema-documents.md](schema-documents.md) · [templates.md](templates.md) · [build-and-guards.md](build-and-guards.md) · [shipped-and-identities.md](shipped-and-identities.md). Darkmatter integration: [definition.md](../../../darkmatter/docs/topics/schemas/definition.md).

## Related Skills

- **darkmatter** — the host: baseline, trigger layering, the YAML adapter,
  `md schema`, DMLS. Its [schema.md](../darkmatter/schema.md) holds implementation notes for
  both sides.
- **biscuit-file** — `FileReference`, `FileResolutionContext`,
  `GlobReference`, the `file-reference-syntax` / `file-reference` split, and
  the frontmatter envelope helper this crate uses.
- **rust-testing** — test tiers, `just` recipes, CI test-input rules.
