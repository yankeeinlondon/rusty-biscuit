# Language Core

The crate parses SimplifiedSchema text into an AST, lowers it to Draft
2020-12 JSON Schema, resolves the named types it imports, and judges values
with the result. It also owns the document kinds built on the language:
schema triggers, the schema roots they are found in, and the selection and
checks of whole YAML documents by `kind`. Nothing here reads a Markdown
document, runs an expression, or decides which values are still waiting on a
template: a host (Darkmatter) owns those decisions and calls in here.

```rust
use std::sync::Arc;
use biscuit_file::FileResolutionContext;
use serde_json::json;
use simplified_schema::{Prepared, parse_yaml_schema, to_json_schema};

let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str("name: string(required)\nage: number(min(0))").unwrap();
let root = parse_yaml_schema(&yaml).unwrap();          // a SchemaRoot
let schema = to_json_schema(&root).unwrap();            // Draft 2020-12
let prepared = Prepared::new(Arc::new(schema), None, FileResolutionContext::new("/work")).unwrap();
assert!(prepared.validate_strict(&json!({ "name": "Ada", "age": 36 })).is_ok());
```

## Modules

```mermaid
flowchart TD
    foundations["numeric, span, expression, pending"]
    simplified["simplified: AST, grammar, lowering, source maps, queries"]
    resolve["resolve: imports, cycles, merges"]
    claims["claims: claimed values"]
    coercion["coercion, coerce: conversion"]
    validate["validate, format, file_match: validators"]
    prepared["prepared, outcome: strict and soft validation"]
    object["schema: the SimplifiedSchema object"]
    foundations --> simplified
    simplified --> resolve
    simplified --> coercion
    resolve --> validate
    resolve --> claims
    coercion --> validate
    validate --> prepared
    prepared --> object
    resolve --> object
```

| Module | What it holds |
| --- | --- |
| `simplified` | `SchemaRoot` (one shape, or a root union of arms), `SchemaShape`, `PropertyDef`, `TypeExpr`; the type grammar; `to_json_schema`; source maps (`SchemaSourceMap`), editor cursors, suggestion queries, serializers, subtyping |
| `resolve` | `resolve_schema` and its YAML and schema-root variants, named-type expansion (`expand_named_types`), cycle and use-site checks, `merge_baseline` (the layer merge: a higher layer replaces a whole property) |
| `claims` | `claimed_values`, `ClaimProblem`, `ClaimCause`: the strings a resolved schema claims for one property; see [Claimed Values](#claimed-values) |
| `coercion` | The conversion engine over shapes (`bind`, `bind_with`, `check_exact`), and `ConversionError::located_failures`, one record per failing location; see [Located failures](conversion.md#located-failures) |
| `coerce` | Frontmatter conversion against a lowered JSON Schema (`coerce_frontmatter`) |
| `validate`, `format`, `file_match` | Compiled validators and their cache, the `darkmatter-*` formats, the `x-darkmatter-*` keywords, and `file(match(...))` globs |
| `schema` | `SimplifiedSchema` (build from content or a file, validate, layer, query, export), `SchemaSource`, `TypeDefinition`, `FunctionView`, `SimplifiedSchemaError`; see [The `SimplifiedSchema` Object](simplified-schema-struct.md) |
| `prepared`, `outcome` | `Prepared` (a schema compiled once, with `validate_strict` and `validate_soft`), `ValidationSuccess`, `ValidationFailure`; see [Strict and Soft Validation](./validation.md) |
| `envelope` | The compiled shipped definitions (`ENVELOPE_DEFINITIONS`: the `kind: schema` and `kind: yaml-schema` envelopes and the `kind: schema-trigger` kind definition) and `admit_envelope_text`, which judges a standalone document against its envelope definition; see [Admission](./language/schema-documents.md#admission-checking-an-envelope-against-its-definition) |
| `roots` | `SchemaRoots::for_document`, the five schema roots of a document's context (`SCHEMAS_DIR`, package, package area, tree root, `~/schemas`) that bare file names and triggers are looked up in; `SchemaSearchRoot`, `SchemaRootKind` |
| `triggers` | Schema triggers: the `match` grammar, the `kind: schema-trigger` envelope parser and lint, the matcher, discovery in the schema roots (`scan`, `scan_with_embedded`, `scan_with_source`, `TriggerRegistry`), payload assembly, prepared bundles (`prepare_trigger_bundle`), and the `dm.schema.invalid_trigger` finding (`SCHEMA_INVALID_TRIGGER_CODE`) |
| `embedded` | Documents compiled into a binary: `EmbeddedDocument`, a host's reserved namespace (`ShippedSources`), and `EmbeddedSchemaBundle`, a host's trigger bundle |
| `yaml_documents` | Whole YAML documents: the one loader (`load_yaml_document`, `LoadedYamlDocument`), the kinds a context can select (`KindRegistry`, `KindHost`, `ShippedDefinition`), selection by `kind` or root `$schema` (`select`, `Selection`), and the checks of the language's own kinds (`CHECKED_KINDS`) |
| `validation`, `positions` | `ValidationProblem` and its codes, JSON Pointers, and where a problem sits in its source text (`PositionMap::byte_range`) |
| `completion`, `frontmatter_shape` | Which definition governs a frontmatter path, and union-aware completion choices |
| `slots` | Finding the `{{ … }}` slots of a template without evaluating them (`collect_slots`, `has_dynamic`, `reachable_slots`), and the `SLOT_NOT_ALLOWED_IN_KIND_CODE` finding for a slot in a kinded document; see [Slots](./language/slots.md) |
| `errors` | `SchemaError`, `UnresolvedSlots` |

`SchemaRoot` is the parsed `$schema` itself: a root union stays a union, never
flattened to one property map.

## Claimed Values

`claimed_values(&resolved, property)` answers "which values of this property
does the schema accept?" when the answer must be a finite list of names, such
as the document kinds a schema claims through its `kind`. It reads the
resolved lowering, so inline arms, local aliases, `Name@file` imports, and
file arms are judged alike, and it is the check that completes the parse-only
[`property(...)` declaration option](./language/schema-types.md#narrowing-a-declaration).

```rust
use std::path::Path;
use biscuit_file::FileResolutionContext;
use simplified_schema::{claimed_values, embedded::ShippedSources, resolve::resolve_yaml_schema};

let dir = Path::new("/work");
let declaration: serde_yaml_ng::Value =
    serde_yaml_ng::from_str("- kind: enum(a, b; required)\n- kind: literal(c; required)\n- kind: literal(a; required)\n").unwrap();
let resolved = resolve_yaml_schema(&declaration, dir, &FileResolutionContext::new(dir), ShippedSources::NONE).unwrap();
assert_eq!(claimed_values(&resolved, "kind").unwrap(), ["a", "b", "c"]);
```

Every root arm (or the single root) must be an object that lists the
property in `required` and declares it, free of `null`, as a `const`, an
`enum`, or a union of those, each choice a non-empty string. The result is
the union of every arm's choices in first-seen order, repeats removed. A file
arm that is itself a root union contributes each of its arms. The first arm
that fails is returned as a `ClaimProblem { property, arm_index, cause, path }`:

| `ClaimCause` | The arm | Example declaration of `kind` |
| --- | --- | --- |
| `NotObject` | is not an object schema | a file arm whose schema is `{"type": "string"}` |
| `Missing` | does not declare the property | `name: string` (no `kind`) |
| `NotRequired` | declares it optional | `literal(a)` |
| `NullAdmitted` | requires it, but its type admits `null`, so a written `null` meets presence without naming a choice | `null \| literal(a; required)`, or a raw JSON Schema `anyOf` with a `{"type": "null"}` arm |
| `NotEnumerable` | declares a type that is not a finite set | `string(required)` |
| `NonStringChoice` | has a choice that is not a string | `literal(1; required)` |
| `EmptyChoice` | has an empty string choice, or an empty `enum` | `literal(""; required)` |

`arm_index` is `None` for a single root; an arm reached through a file arm
keeps the referencing arm's index. `path` addresses the property (or, for
`NotObject`, the arm) as if it were declared inline. Its message is the same
for every cause, `` root-union arm 1: `kind` must be declared as `literal(...)` or `enum(...)` ``,
so read `cause` to tell them apart.

A host that defines YAML document kinds with
[`kind: yaml-schema`](./language/schema-documents.md#kind-yaml-schema) files
prepares an embedded one with `resolve::prepare_embedded_yaml_schema` and
reads the kinds it claims with `claimed_values(&resolved, "kind")`.

## Supplied Sources

The resolver reads only what the caller hands it: the schema value, the
directory it was written in, schema roots for bare file names, a
`biscuit_file::FileResolutionContext`, and a `ShippedSources` table.

A host may reserve a namespace for documents compiled into its binary.
Darkmatter supplies `darkmatter:` over its shipped catalog, so
`stack@darkmatter:lifecycle` names `stack` in its `lifecycle` partial and no
file is read. With `ShippedSources::NONE` that spelling is an ordinary file
reference. A host whose shipped artifact holds those partials' parsed named
types attaches it with `ShippedSources::with_prepared`, so they are not parsed
again (see [Shipped artifacts](./shipped-artifacts.md)).

```rust
use simplified_schema::embedded::{EmbeddedDocument, ShippedSources};

const DOCUMENTS: &[EmbeddedDocument<'static>] = &[EmbeddedDocument {
    path: "host/partials/talk.yaml",
    text: "kind: schema\ntypes:\n  greeting: \"{ say: string }\"\n",
}];
const HOST: ShippedSources = ShippedSources::new("host:", "host/partials", DOCUMENTS);

// `x: greeting@host:talk` resolves among DOCUMENTS:
// resolve_yaml_schema(&value, base_dir, &context, HOST)
```

A stem normally names `<partials>/<stem>.yaml`. To serve a shipped document
from outside the partials folder, name it with an alias:
`ShippedSources::with_aliases(&[("config", "host/schemas/host-config.yaml")])`
makes `settings@host:config` name `settings` in that document, which must be
one of the shipped documents. Every other stem keeps its partial meaning.
Aliases take part in equality and in every memo keyed on the sources, and a
host that prepares an artifact over them records them in the artifact's
preparation configuration, so a changed alias makes the artifact stale.
Darkmatter serves its configuration schema this way
(`permissions@darkmatter:config`).

Every entry point that resolves a trigger payload or prepares a kind
definition takes the host's `ShippedSources` the same way, so a trigger or a
`kind: yaml-schema` file may import from the host's namespace:
`triggers::scan(context, sources)`, `scan_with_embedded`, `scan_with_source`,
`prepare_trigger_bundle`, `TriggerRegistry::new`, and
`assemble::resolve_trigger_payload`. A YAML kind registry takes a `KindHost`, the
host's `ShippedSources` plus the shipped definitions it prepared with them
(`prepare_shipped_definitions(sources)`, or the prepared form its own shipped
artifact recorded):

```rust
use simplified_schema::embedded::ShippedSources;
use simplified_schema::yaml_documents::{KindHost, KindRegistry, RegistryScope, prepare_shipped_definitions};

let shipped = prepare_shipped_definitions(ShippedSources::NONE); // once per process
let host = KindHost { sources: ShippedSources::NONE, shipped: &shipped };
let registry = KindRegistry::from_filesystem(&context, RegistryScope::for_context(&context), host);
```

Darkmatter wraps these with its own `darkmatter:` sources
(`darkmatter::markdown::schemas::{shipped_sources, kind_host}` and its
`triggers::scan*` wrappers), so a Darkmatter caller gets exactly the
resolution Darkmatter uses.

An `http://` or `https://` schema, import, or example reference is
`SchemaError::RemoteUnsupported`.

## What a Host Keeps

- **Baseline and layering policy.** Which schemas layer under a document, and
  in what order, is the host's; `SimplifiedSchema::layer_under` /
  `layer_over`, `resolve::merge_baseline`, and `layered_property_shape` do the
  merging it asks for.
- **Pending values.** The host decides which `{{ … }}` templates are still
  pending and hands their paths in (`coerce::NestedTemplates`); the library
  never asks.
- **Layering triggers.** Discovery and matching are the library's
  (`TriggerRegistry`, `triggers::matched_triggers`; `TriggerOrigin` maps a
  `SchemaOrigin` back to its `TriggerSource`); which matched payloads layer
  between the host's baseline and a document's own `$schema` is the host's.
- **Validating a YAML document.** The library loads it and selects its
  schema; the host prepares the selected schema, judges the document, and
  reports (Darkmatter's `validate_yaml_document`).
- **Terminal output.** `SchemaError` renders in a terminal only with the
  opt-in `terminal` feature.

Without the default `file` feature, nothing that needs a file on disk passes;
see [Cargo features](./features.md).
