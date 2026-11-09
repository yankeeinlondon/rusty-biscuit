---
name: simplified-schema
description: Expert knowledge for the simplified-schema Rust library, the home of the SimplifiedSchema language (compact YAML type strings such as `name: string(required)`, `age: number(min(0))`) extracted from Darkmatter. Use when working in the `simplified-schema/` package area; using `SimplifiedSchema`, `SchemaSource`, `SchemaRoot`, `SchemaShape`, `PropertyDef`, `Prepared`, `validate_strict`, `validate_soft`, `ValidationSuccess`, `ValidationFailure`, or `SchemaError`; writing, parsing, or lowering SimplifiedSchema type strings to JSON Schema; schema layering (`layer_under`, `layer_over`, `merge_baseline`); conversion and coercion (`coercion`, `coerce`); named-type imports (`Name@file`, `Name@this`, recursive and shared types); `PropertyPath`, `set_at`, `remove_at`, `RemovalPlan`, or `SetterValue`; `non-null`, closed roots, `schema(mapping)` / `property(...)` / `forms(...)` / `declared-required` / `container` declaration options, `denotes_container`, `located_failures`, or `claimed_values`; standalone schema documents; deciding whether schema code belongs in the library or in Darkmatter; fixing a failing schema ownership, dependency-direction, or context-construction guard; or adding the simplified-schema dependency.
last_updated: 2026-11-06T00:00:00Z
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
- **Templates are passive and fail closed.** A `{{ … }}` element of
  `enum`/`suggest`, or the whole argument of `literal`/`default`/`min`/`max`/
  `pattern`, parses to `Constraint::Dynamic` / `Constraint::DynamicValue`
  (only when a slot is present; static lists stay `Members`/`Suggest`). The
  grammar reads a slot with `scan::scan_expression_at`; `source.rs`'s
  argument scanner skips it whole and `project_atom_suggestions` projects its
  spans. Every lowering entry returns `SchemaError::UnresolvedSlots` first
  (`slots::reject_unresolved`), so a reader that does not know slots never
  sees one; queries/completion set `pending`. A new `Constraint` reader that
  could see a template must decide its slot case (see
  [slots.md](../../../simplified-schema/docs/topics/language/slots.md)).
  Tests: `src/simplified/grammar/slot_tests.rs`,
  `tests/l1/{slot_spans,slot_templates}.rs`, `schema_object.rs::templates`.
- **Resolution replays, never patches.** A host (`slots::SlotHost`: passive
  `admit`, effectful `resolve`, `is_canceled`) supplies values; the
  library reruns the same resolution inside `slots::with_values`, and every
  lowering site calls `slots::apply_active` right after import expansion
  (`substitute`: result shapes, dedupe, stricter-bound/equal-default merge,
  pattern cap). Public entry points (`resolving_entry` in `resolve.rs`)
  attach `UnresolvedSlots::pending` (`PendingSchema`, replaying the recorded
  `read_sources()` texts); `SimplifiedSchema::resolve_slots` reruns the
  layer's own `Rebuild`. Expansion stamps `Slot::origin` with the file a
  slot was written in (`DefNamespace::file`); a new lowering site must call
  `apply_active` too (`apply_active_to_arm` for a root-union arm, never
  pruned by `omitting`; the union then marks the arm's template
  `is_union_arm` with the names every arm declares, `resolve.rs::pending_union_arm`,
  read by `declared_properties()`). Tests: `tests/l1/dynamic_slots.rs`. Whole-type slots (a
  slot supplying a property's whole definition) are **planned**, not built: do
  not widen the grammar to admit one ad hoc. The library never evaluates a
  slot and never reads its targets; approval, caching, and evaluation are the
  host's (Darkmatter's `schemas/slots/`).
- **Layering is atomic.** Each `layer_under` / `layer_over` commits completely
  or not at all, with whole-property replacement.
- **One implementation.** Darkmatter calls the library; it never redefines a
  library-owned item (see the guards below).
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
| Resolve a template's `{{ … }}` slots | `schema.resolve_slots(&host)`, or `UnresolvedSlots::pending` → `PendingSchema::resolve(&host, &SlotRunRequest { context })` |
| Resolve only the top-level properties a higher layer leaves | `pending.without_properties(&names)` (drops them and the definitions only they reach; `omitted_properties()` belongs in a cache key), then `resolve` or, with no slot left, `resolve_without_slots()` |
| The top-level names a template declares in every alternative (a root-union arm's: every arm's), unevaluated | `pending.declared_properties()` |
| Read a slot's data file honoring editor buffers | `resolve::buffers::read(path)` |
| Resolve an already-parsed inline `$schema` value with its slots spanned in the file it came from | `resolve::resolve_authored_yaml_schema(value, owner, Some(&AuthoredSchemaValue { file, text, mapping }), …)` (`resolve_authored_schema_with_roots` for JSON); the value is located and verified first, else no position is claimed |
| The file and text a slot's spans index, without reading anything | `pending.slot_text(&slot)`, `SlotResolutionFailure::of_pending(error, &pending).slot_text(&slot)` |
| Judge each schema source before it is read (a host's slot approvals) | `resolve::buffers::guarded(guard, \|\| …)`; a refusal fails the read as `PermissionDenied`; `unguarded` suspends the guards |
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
text (whole Markdown file, CRLF counted). Never stringify a `SchemaError` into
`message` alone; see [Diagnostics](../../../simplified-schema/docs/topics/diagnostics.md#a-rejected-definition-in-a-schema-document).

A referenced `$schema` file is classified by `classify_yaml_schema_text` /
`classify_json_schema_text` (`resolve.rs`): YAML must be an envelope with a
top-level `$schema` (`types`-only is `NotASchemaDocument { TypesOnly }`, a map
without `$schema` is `NoSchemaProperty`, a `kind: yaml-schema` is
`SelectedByKind` even when malformed, and so is one named by `Name@file`;
hosts reading a referenced file use `resolve::parse_referenced_schema_document`).
JSON must be an object whose
`$schema` is exactly `DRAFT_2020_12_DIALECT` (else `JsonMissingDialect` or
`UnsupportedJsonDialect`) and pass the offline meta-schema (else
`InvalidJsonSchema`). There is no raw-YAML JSON Schema fallback; see
[What a schema reference may target](../../../simplified-schema/docs/topics/language/schema-documents.md#what-a-schema-reference-may-target).

Envelopes (`kind: schema`, `kind: yaml-schema`, informal) are **admitted**
before their payload is read: `envelope.rs` soft-validates each against the
compiled `simplified-schema/schemas/schema.yaml` / `yaml-schema.yaml`
(`envelope::ENVELOPE_DEFINITIONS`, `admit_envelope_text`). Those files are the
only authority for envelope structure (closed keys, `$schema` or `types`,
`types` a mapping, non-null metadata); never add a structural check to
`standalone.rs` (`shipped_corpus.rs` guards it). Rust keeps only category
grammar, duplicate/undeclared categories, and duplicate overloads. Metadata
is read from the converted copy (`description: 123` → `"123"`); `$schema` /
`types` carriers are never converted. Report order: envelope problem, then a
parser-located definition failure inside a carrier, then the carrier shape
(`standalone.rs::rejected_envelope`; `locate_standalone_definition_failure`
returns `None` when the envelope itself is malformed). The definitions
bootstrap unadmitted (`parse_standalone_unadmitted`) through a private
validator cache, never `SHARED_VALIDATOR_CACHE`. A host prepares a
`yaml-schema`'s export with `resolve::prepare_embedded_yaml_schema` and reads
its kinds with `claimed_values`. A host that has already judged a whole file
against a definition it selected (Darkmatter's kind checks, where the
definition may be an edited checkout copy) reads it with
`parse_selected_standalone_schema_document(source, path, accepted)` (no
admission; metadata from `accepted`) and prepares a `yaml-schema` with
`resolve::prepare_standalone_yaml_schema`, so the compiled envelope never
judges its shape twice. Tests: `envelope_admission.rs`,
`yaml_schema_envelope.rs`.

Ownership boundary: this crate owns the `schema` and `yaml-schema`
definitions and the envelope admission above, and nothing about which YAML
document gets which schema. Darkmatter owns `schema-trigger.yaml`, the
`KindRegistry` (discovery in schema roots, duplicate and broken claims),
selection (`kind` before root `$schema`), and the kind checks a definition
cannot state; see the `darkmatter` skill's
[schema.md](../darkmatter/schema.md#whole-yaml-documents-schemasyaml_documents).
An envelope that fails admission is a located `SchemaDocument` error; the
library never falls back to treating it as an untagged payload.

## Property paths and setter values

`PropertyPath` is the only parser of the dotted grammar. It keeps decoded text
segments, because `0` is an index or a key depending on the value it meets, so
`PathStep` and `JsonPointer` come only from `resolve`'s `Resolved`, never from text.
`set_at` and `RemovalPlan` decide before changing anything and work on a frontmatter
`IndexMap`, not on source text.

`SetterValue::parse` sees only text; `bind` takes the `Shape` the host looked up.
`Shape::Any` is a declared shape, so text stays text under it; only `None` guesses. A
quoted string binds through the declared shape like text, and stays a string under `None`.
Syntax rejection is text, but a parsed container that fails admission is an error,
never text: `biscuit_file::json5_scan` finds duplicates and number spellings that
`serde_json::Value` has already lost, `numeric::admit_json5_text_numbers` judges
them, and `json5_is_well_formed` keeps an integer beyond `u64` from reaching the YAML
step as a key. A YAML list under `$schema` is a union of object arms, not an array,
which is a trap when writing fixtures; an array of objects is a named type with
`item[]`.

## Features

| Feature | Default | Adds |
|---|---|---|
| `file` | yes | File factories, `SchemaSource`, `repository_scope_catalog`; pulls `biscuit-file/file-reference` and `sniff` |
| `terminal` | no | `biscuit-terminal` `BlockError` rendering of `SchemaError` (Darkmatter enables it) |
| `work-counters` | no | Test-only `work_counters` module for complexity regressions |

Without `file`, the crate depends on `biscuit-file` with
`file-reference-syntax` only (no `gix`/`walkdir`/`sniff`); file-backed checks
fail with `FileReferenceError::ResolutionUnavailable` or
`ValidationFailure::UnavailableCheck`, never pass.

```toml
[dependencies]
simplified-schema = { path = "../../simplified-schema/lib" }                            # default
simplified-schema = { path = "../../simplified-schema/lib", default-features = false } # text only
```

## Boundary Guards

| Guard | Fails when | Satisfy it by |
|---|---|---|
| `simplified-schema/lib/tests/l1/context_construction_guard.rs` | Production code outside `schema/capture.rs` calls `std::env::current_dir`, `biscuit_file::home_dir`, `capture_env`, or `FileResolutionContext::from_snapshot` (besides the one empty `content_context` in `schema/layer.rs`), or adds a second optional context besides `SchemaSource`'s | Take a `FileResolutionContext` parameter; only extend `ALLOWLIST` (exact file, identifier, count, reason) for a genuinely new capture |
| `simplified-schema/lib/tests/l1/dependency_direction_guard.rs` | Any manifest table (dev included, followed through path deps) reaches `darkmatter`, `dmls`, `content-policy`, `claudine*`, `rendezvous-*`, `zed-dmls`, a `*-cli`, or a crate under those dirs; or source names one of their crates | Move the host-specific code to the host; invert the dependency |
| `simplified-schema/lib/tests/l1/path_lookup_guard.rs` | Raw `canonicalize` or home lookup | `biscuit_file::canonicalize_simplified`, `biscuit_file::home_dir` |
| `darkmatter/lib/tests/l1/schema_language_ownership_guard.rs` | Darkmatter lib, CLI, or DMLS production source *defines* (`fn`/`struct`/`enum`/`trait`/`type`/`union`) an owned name (`to_json_schema`, `is_subtype`, `parse_yaml_schema`, `SchemaRoot`, `Lexer`, `format_number`, `ConversionError`, ...); or the listed library file no longer defines a name | Call the library instead of re-implementing; when renaming or moving an owned item in the library, update the guard's owner table and Darkmatter's `[package.metadata.ci.tests] source-inputs` |
| `darkmatter/lib/tests/l1/glob_implementation_guard.rs` | `simplified-schema/lib/src` reaches a glob library directly or its manifest names one | Match through `biscuit_file::GlobReference` only |

The guards reuse Darkmatter's shared engines by `#[path]`
(`darkmatter/cli/tests/common/{context_guard,path_lookup_guard,source_scan}.rs`,
`darkmatter/lib/tests/common/dependency_graph.rs`); they are declared in this
crate's `source-inputs`, so editing them schedules this crate's tests.

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
