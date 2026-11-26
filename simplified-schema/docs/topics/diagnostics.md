# Diagnostics

Every failure in this crate is a typed value that says what went wrong and
where it came from, so a CLI can print it, an editor can underline it, and a
cache can tell which file to watch. You never parse a message to find the
property, the file, or the byte range: each is a field.

A failure belongs to one of three stages, and each stage has its own type:

| Stage | Type | Example |
| --- | --- | --- |
| Building, layering, selecting, or exporting a `SimplifiedSchema` | `SimplifiedSchemaError` | the file does not exist; the YAML holds no schema |
| The language itself: parsing, resolving, lowering, compiling | `SchemaError` | `string(min(nope))`; an unknown named type |
| Judging a value | `ValidationFailure`, holding `ValidationProblem`s | `age` is `-1` but the schema says `min(0)` |

Value failures are covered in [Strict and Soft Validation](./validation.md#failures);
this page covers the first two and the location data all three carry.

## Typed Errors and Provenance

### `SimplifiedSchemaError` wraps `SchemaError`

The `SimplifiedSchema` factories return `SimplifiedSchemaError`. Its
variants describe the *input*: `MissingFile`, `Read`, `Reference` (a
malformed or unresolvable reference), `Context` and `Discovery` (capturing a
raw path's request context failed; see
[Host Capabilities](./host-capabilities.md)), `MissingSchema`, `Frontmatter`,
`Unprepared`, `Unresolved` (a [template](./language/slots.md) whose slots
are not resolved), `Selection`, and `ExportUnsupported`. A malformed *definition*
is `SimplifiedSchemaError::Schema`, which holds the language's own
`SchemaError` by value, so the source chain is never flattened into text:

```rust
use simplified_schema::{SchemaError, SimplifiedSchema, SimplifiedSchemaError};

// Markdown content: the frontmatter `$schema` is the schema.
let text = "---\r\ntitle: test\r\n$schema:\r\n  age: 'number(min('\r\n---\r\n";
let error = SimplifiedSchema::try_from(text).unwrap_err();
match error {
    SimplifiedSchemaError::Schema(SchemaError::SchemaDocument { path, span: Some(span), source: Some(cause), .. }) => {
        assert_eq!(path, std::path::Path::new("content")); // the file, when one was read
        assert_eq!(&text[span], "'number(min('"); // a range in the whole document
        assert!(matches!(*cause, SchemaError::Grammar { ref property, .. } if property == "age"));
    }
    other => panic!("unexpected: {other}"),
}
```

`SimplifiedSchemaError` is `#[non_exhaustive]`; match the variants you act on
and keep a fallback arm. The lower-level functions (`parse_yaml_schema`,
`to_json_schema`, the `resolve` module) return `SchemaError` directly; its
variants are listed in the [Error Model](#error-model).

### A rejected definition in a schema document

When a definition inside a schema *document* fails to parse, the error is
`SchemaError::SchemaDocument`, located in the text that was supplied:

| Field | Holds |
| --- | --- |
| `path` | the file that was read, or `content` for text handed to `SimplifiedSchema::try_from` |
| `source` | the parser's own typed error, unchanged (usually `Grammar`, whose `span` stays relative to the type string) |
| `span` | the rejected text's byte range in the exact document supplied: the rejected token when it can be projected, otherwise the whole rejected value |
| `message` | the cause's text, prefixed with where it was found (`invalid SimplifiedSchema payload`, `invalid named types`, ``invalid frontmatter `$schema` ``) |

This holds for every way a document is read:

- a standalone YAML document's `$schema` payload and its `types` table, from
  `parse_standalone_schema_document`, the content factory, `try_from_yaml`,
  or `resolve`;
- a Markdown document's frontmatter `$schema`, from the content factory,
  `try_from_markdown`, or `resolve`, where `span` counts from the first byte
  of the Markdown file, fence lines included;
- a named-type file a schema imports (`Person@./types.yaml`), where `path`
  is the imported file and `span` is a range in that file.

Byte offsets count line endings as written, so a CRLF document's offsets
are larger than its LF twin's. A frontmatter that is not YAML at all is a
`SchemaDocument` whose `span` is the empty range at the YAML parser's
position in the whole document (its message's line and column still count
from the frontmatter's first line). An envelope its definition rejects is
located too ([below](#an-envelope-its-definition-rejects)). Other document
failures, such as a malformed `categories` entry or a suggestion list that
cannot be linted, have no `span`; a lint failure keeps its typed cause in
`source`.
`locate_standalone_definition_failure` returns the same failure with its
structural path and secondary locations (see
[below](#source-aware-parsing-and-located-failures)).

### An envelope its definition rejects

Every recognized standalone envelope is
[admitted](./language/schema-documents.md#admission-checking-an-envelope-against-its-definition)
against its definition (`schema.yaml` for `kind: schema` and the informal
envelope, `yaml-schema.yaml` for `kind: yaml-schema`) before its payload is
read. A rejected envelope is also `SchemaError::SchemaDocument`:

| Field | Holds |
| --- | --- |
| `path` | the file that was read, or `content` |
| `span` | the byte range of the offending token in the supplied text: the key itself for an unknown key, and only the first line of a block collection |
| `message` | ``the {envelope} envelope does not satisfy its definition `{definition}`: `{location}`: {problem}``, where `{envelope}` is `` `kind: schema` ``, `` `kind: yaml-schema` ``, or `informal schema`, and `{location}` (with its colon) is omitted when the problem has no path |
| `source` | `None` |

```yaml
kind: schema
types:
    - a
    - b
```

```text
the `kind: schema` envelope does not satisfy its definition `simplified-schema/schemas/schema.yaml`: `/types`: the declaration must be a mapping, not a list
```

One defect is reported per document, in this order:

1. a problem with the envelope itself (an unknown key, metadata of the wrong
   shape, a missing payload), preferring one that is not a missing required
   key, since a missing `types` is usually a misspelled `typed:`;
2. otherwise a definition inside `$schema` or `types` that the parser
   rejects, reported as [a rejected definition](#a-rejected-definition-in-a-schema-document)
   with the parser's typed `source` and token `span`;
3. otherwise the carrier's own shape (`types: null`, `types: [a, b]`).

To see every problem rather than the reported one, call
`simplified_schema::envelope::admit_envelope_text`, whose `EnvelopeAdmission`
lists each located `ValidationProblem`.

### When a file is not a schema target

A document's `$schema` may name only a
[schema definition](./language/schema-documents.md#what-a-schema-reference-may-target):
a YAML document with a top-level `$schema` that is not a `kind: yaml-schema`,
or a `.json` object declaring Draft 2020-12. Any other file fails with an error that names it, and where
the fix is mechanical the message says what to write instead. A
`NotASchemaDocument` puts it on a closing `hint:` line:

```text
`schemas/person.yaml` is not a valid schema definition: it declares only `types`, with no `$schema` property
hint: use a named-type import (`Name@file.yaml`) to reuse its types
```

| The file | Error | What the message suggests |
| --- | --- | --- |
| `kind: schema` with only `types` | `NotASchemaDocument` (`TypesOnly`) | use a named-type import (`Name@file.yaml`) to reuse its types |
| `kind: yaml-schema`, well formed or not, named as a `$schema` target, a root-union file arm, a `Name@file` import source, or an embedded `$schema` target | `NotASchemaDocument` (`SelectedByKind`): "it is a `yaml-schema`; it is selected by `kind`, not referenced" | move shared types into a `kind: schema` file and import them from there |
| `kind: namespace-schema`, named as a `$schema` target or a `Name@file` import source | `NotASchemaDocument` (`NamespaceCatalog`) | move shared types into a `kind: schema` file and import them from there |
| `kind: expression-schema` or `kind: mutation-schema` (a callable catalog), named as a `$schema` target or a `Name@file` import source | `NotASchemaDocument` (`CallableCatalog`) | move shared types into a `kind: schema` file and import them from there |
| `kind: schema-trigger`, well formed or not, named as a `$schema` target or a `Name@file` import source | `NotASchemaDocument` (`TriggerEnvelope`): "it is a `schema-trigger`; trigger schemas activate by placement and match, never by reference" | reference the trigger's payload file instead |
| a root-union schema, or one whose `$schema` is a whole-file reference, named as a `Name@file` import source | `NotASchemaDocument` (`RootUnionImport`, `ReferenceImport`): "`x.yaml` cannot supply named imports: …" | import the type from the file that declares it |
| a YAML mapping with no `$schema` | `NotASchemaDocument` (`NoSchemaProperty`) | none |
| YAML whose root is not a mapping | `NotASchemaDocument` (`NotAMapping`) | none |
| `.json` whose root is not an object | `NotASchemaDocument` (`NotAMapping`) | declare `"$schema": "https://json-schema.org/draft/2020-12/schema"` |
| a `.json` object without a string `$schema` | `NotASchemaDocument` (`JsonMissingDialect`) | the same declaration |
| a `.json` object declaring another dialect (Draft 7, `""`, a trailing `#`) | `UnsupportedJsonDialect`, naming the `declared` URI | only the 2020-12 URI is supported |
| a `.json` object declaring 2020-12 that fails its meta-schema | `InvalidJsonSchema`, with the failing location | none |
| YAML with `$schema`, no `kind: schema` or `kind: yaml-schema`, and another top-level key | `SchemaDocument`, "unsupported top-level key(s): ..." | when `$schema` is a `json-schema.org` URI (raw JSON Schema written in YAML), to write it as a `.json` file declaring the 2020-12 URI |

`NotSchemaReason::hint(path)` returns the hint for a reason, so a host can
show it without parsing the message; the `.json` hints depend on the path's
extension. With the `terminal` feature each of these renders as a status
block whose hint line carries the same text (a reason with no hint of its
own gets a general one). A named-type import (`Name@file.yaml`) is not a
`$schema` target and reads a `types`-only file without error; a
`kind: yaml-schema` file is refused there too, with `SelectedByKind`.

### Where a definition failure points

A `SchemaError` that is about one place in a schema names that place
structurally:

- **`Grammar`** carries the `property` (or a synthetic `"<root>"` /
  `"<arm[N]>"` for a structural failure), a byte `span` inside that
  property's type string, and `related` earlier declarations a duplicate
  conflicts with.
- **`UnknownNamedType`, `UnionReference`, and `ConstraintConflict`** carry a
  `SchemaReferenceSite`: the `SchemaSourceId` that wrote the reference and the
  `SchemaSourcePath` of the property or named type holding it.
  `ConstraintConflict` also lists every participant with its
  `SchemaConstraintOrigin` (source, path, keyword, occurrence, and span), so
  a conflict reached through several imports points at each declaration.
- **A reference that names a file and cannot be followed** (`Unresolved`,
  `NoMatch`, `RemoteUnsupported`, `BundleBoundary`, `BareNameSiblingExists`,
  `InvalidExample`, `TriggerSchemaReferenced`) carries an optional `site`,
  read with `SchemaError::reference_site()`: the declaration that wrote the
  reference (an import, an `example(...)` argument, a `$schema` file),
  recorded where the resolver first read it. An imported library's
  declaration keeps the library's file and path even after expansion copied
  it into the importer, so a host reports the failure in the file that wrote
  it. The site is `None` when no single declaration is known: two
  declarations of one resolution wrote the same `example(...)` spelling, the
  reference was not written in a schema (a selection called directly), or
  the error is raised after resolution (`TriggerSchemaReferenced` from
  `check_document_schema_not_trigger`, which sees only the paths read). A
  host then reports the failure at the file, never by searching its text for
  the reference.
- **A file that was found but refused for the role a declaration named it
  in** carries the same `site`, the declaration that named it:
  `NotASchemaDocument` for a role reason (`NotSchemaReason::is_role()`:
  `TypesOnly`, `SelectedByKind`, `NamespaceCatalog`, `CallableCatalog`,
  `TriggerEnvelope`, `RootUnionImport`, `ReferenceImport`), `UnsupportedJsonDialect`, a
  `ReferenceCycle` or `ReferenceDepthExceeded` at the `$schema` that closed
  the loop or made the last hop, and `TriggerPayloadCycle` when a trigger
  payload's refusal of a trigger file is reported as a cycle
  (`triggers::assemble::refused_trigger_file` returns the site with the path,
  and `trigger_schema_referenced(path, site)` keeps it for a document
  `$schema`). Behind a delegating file (`$schema: ./alias.yaml` naming the
  refused file), the site is the delegating file's `$schema`. A file refused
  for its own content (`NoSchemaProperty`, `NotAMapping`,
  `JsonMissingDialect`, `InvalidJsonSchema`, or a `SchemaDocument` parse
  failure) has no site: the problem is in that file, and a host reports it
  there.

  ```rust
  // A document `$schema: ./shared.yaml` naming a types-only library:
  let SchemaError::NotASchemaDocument { reason: NotSchemaReason::TypesOnly, .. } = &error else { unreachable!() };
  assert_eq!(error.reference_site().map(|site| &site.path), Some(&SchemaSourcePath::root()));
  ```

  ```rust
  // `lib.yaml` declares `widget: { size: string(example(./absent.yaml)) }`
  // and a document imports `widget@./lib.yaml`: the failure names lib.yaml.
  if let Some(site) = error.reference_site() {
      assert_eq!(site.source, SchemaSourceId::File(lib_yaml));
      assert_eq!(site.path, SchemaSourcePath::root().property("widget").property("size"));
  }
  ```
- **`NonDescendingCycle`** lists every `CycleReference` of each rejected
  cycle, with its site, its target, and the source that declares the target.
  A loop of `...name` / `$extends` and alias hops is reported this way too,
  each hop at the operand or alias that wrote it.
- **Composition errors** point at the operand, the `...name` or `$extends`
  item that failed (see
  [Object Composition](language/objects-and-named-types.md#object-composition-name-and-extends)):
  - `InvalidCompositionTarget { name, site, definition, reason }`: the
    operand resolved to something that is not one declared object. `name` is
    the operand as written, `site` the operand, `definition` the declaration
    that decided it (through aliases, the type they reach), and `reason` a
    `CompositionTargetKind`: `Primitive`, `OpaqueObject`, `Union`, `Array`
    (a list or tuple), `Callable`, or `Template`.
  - `CompositionReference { operand, site, source }`: the operand could not
    be looked up because its file failed to resolve or load, or the chain of
    composition and alias hops passed the 32-level limit. `source` keeps the
    original cause (`NoMatch`, `SchemaDocument`, `ImportCycle`, …).
  - An operand that names no type is the ordinary `UnknownNamedType`, at
    the operand.
  - `ComposedMember { site, source }`: lowering failed inside a member that
    was copied in; `site` is the member's declaration, and `source` the
    failure, whose property path names the receiving object.

  ```text
  $schema.order[...0]: `label` cannot be composed into an object: it is a primitive type, not an object
  ```
- **`ReferencedSchema`** wraps an error raised inside another schema file
  with that file's `path`, so per-property detail survives while the
  diagnostic points at the file that authored it. A reference written in a
  file the schema imports a named type from (`Name@./lib.yaml`), and that
  cannot be followed (a missing or refused file, an `example(...)` that
  fails), is wrapped the same way, naming the imported file, so a host
  reports it where it is written rather than in the importing document.

  ```text
  referenced schema `/repo/schemas/lib.yaml` is invalid: could not resolve $schema reference `./options.yaml`
  ```

A `SchemaSourceId` is one of three sources:

| Variant | Meaning | Displays as |
| --- | --- | --- |
| `InlineDocument` | the `$schema` value being prepared (inline frontmatter, or a value the caller supplied) | the inline `` `$schema` `` |
| `File(path)` | a schema file, by the canonical path the resolver loaded | `` `path` `` |
| `Embedded(path)` | a document compiled into a host binary, by its virtual path; never read from disk | embedded `` `path` `` |

Spans are present only when the resolver held that source's text. A schema
file's spans are filled in; a reference in the inline `$schema` has none,
because the resolver received a YAML value, not text. The caller that holds
the document projects its own range from the
[source-aware parsers](#source-aware-parsing-and-located-failures).

### Composition in the editor

DMLS reports a composition failure as `dm.schema.invalid_type_definition`,
like other named-type failures, ranged on the **whole failing operand**
(`label@./types.yaml`, not just `label`), whether it is written after `...`
or as a `$extends` value or list item. When the target's declaration is
known, it is attached as related information; for a failure further down a
chain, the diagnostic stays on the receiver's operand and relates the
declaration that failed. Editing an imported target, including in an
unsaved buffer, refreshes the receiving schema's diagnostics and property
completions. Completion, hover, and go-to-definition on operands are
described in the `use-dmls` skill and the
[DMLS schema support](../../../darkmatter/docs/topics/schemas/dmls-schema-support.md) page.

### Where a schema came from

Two records answer "which file made this schema what it is":

- **`SchemaProvenance`**, from `SimplifiedSchema::provenance()` (and
  `layer_provenance()` for every layer, highest precedence first): the file
  the schema was read from (`path()`, `None` for content), the files its
  `Name@file` imports, `example(...)` constraints, and whole-file references
  read (`imports()`, `examples()`, `referenced_files()`), every location
  probed while resolving, found or not (`searched()`), and its
  `advisories()`. These are the dependency edges a cache keys on: a change
  to any of those files, or a file appearing at a searched location, can
  change the schema.
- **`SchemaOrigin`** records which layer a resolved schema, or one of its
  properties, came from: `SchemaOriginKind::Document`, `ReferencedFile`,
  `Baseline`, or `Trigger`, with the declaring `SchemaSourceId` for the last
  two. `resolve::ResolvedSchema::origin` holds it for a resolved `$schema`,
  and a host that layers schemas keeps a `SchemaOriginMap` (one origin per
  top-level property, in declaration order) so a diagnostic can point at the
  schema that declared the failing property. `SchemaOrigin::file()` is the
  host file, when there is one.

```rust
let schema = SimplifiedSchema::try_from_yaml(Path::new("/work/schemas/person.yaml"))?;
assert_eq!(schema.provenance().path(), Some(Path::new("/work/schemas/person.yaml")));
for import in schema.provenance().imports() {
    watch(import); // re-resolve when an imported file changes
}
```

### Where a value problem points

A `ValidationProblem` locates itself in the value and in the schema:
`path` (and its decoded form `instance_path`) is the JSON Pointer of the
failing value, `schema_path` the pointer of the failing keyword in the
lowered schema, `arm_index` the root-union arm it was raised under, and
`code` a `ValidationProblemCode` whose `diagnostic_code()` is a stable string
such as `dm.schema.constraint`. A file-value failure also carries a typed
`FileReferenceDiagnostic` (malformed syntax, unresolvable, or no such file).

`ValidationProblemCode::KindCheck` (`dm.schema.kind_check`,
`SCHEMA_KIND_CHECK_CODE`) is never produced by the library's own validators.
It is the code a host gives a problem found by a check of one of its own
document kinds that the kind's definition cannot state: Darkmatter reports an
invalid trigger `$path` glob or an undeclared category this way, located in
the document like any other problem, after the document satisfies its
definition.

`hint` is what to do instead, when the schema proves it, kept apart from
`message` so a surface can render it on its own line:

| Code | `hint` |
|---|---|
| `dm.schema.removed_key` (`RemovedKey`) | the key's `removed(<replacement>)` advice; `path` is the owning object and `offending_property` the key |
| `dm.schema.unknown_key` (`UnknownKey`) | `` did you mean `name`? `` when one declared key of the object is near the offending key |
| `dm.schema.near_miss_key` (`NearMissKey`) | `` did you mean `prompt`? ``; advice on an opted-in open object, never a failure (see [`$advise-unknown-keys`](language/objects-and-named-types.md#advice-on-near-miss-keys-advise-unknown-keys)) |

Every suggestion comes from one ranker, `validation::suggest`: lengths under
three never match, case and `-`/`_` variants always do, otherwise one edit
(two for keys of five characters or more), where an adjacent transposition
counts as one edit. A closed object's tie goes to the first declared key; an
open object's advice requires a unique closest key. The candidates are the
declared keys of the object schemas the location reaches, through references
and the union arms the value can select; a removed key is never one.

`validate_strict` and `validate_soft` judge a `serde_json::Value`, which has
no source text, so their problems carry no `line` or `column`. A caller that
holds the YAML the value came from fills them in afterwards; positions are
one-based lines and one-based UTF-8 byte columns:

```rust
use simplified_schema::positions::build_position_map;

let yaml = "name: Ada\nage: -1\n";
let value: serde_json::Value = serde_yaml_ng::from_str(yaml)?;
let mut problems = person.validate_strict(&value).unwrap_err().into_problems();
build_position_map(yaml).locate_all(&mut problems);
assert_eq!(problems[0].line, Some(2));
```

`PositionMap::byte_range(&problem)` returns the authored byte range a problem
is reported at (the node at its instance path, or the offending key of an
unknown-key problem), for a caller that underlines text rather than printing
a line and column. It is `None` when the map holds no authored tree.

A caller that may never need a position can hold the text instead of a built
map. `PositionMap::deferred(Arc<str>)` keeps the text exactly as given (CRLF
lines and a byte-order mark included) and builds the same map
`build_position_map` would on the first request for a coordinate: `is_empty`,
`byte_range`, or `locate_all` with at least one problem. `locate_all` over an
empty list builds nothing. Clones share the text and that one build, from any
thread:

```rust
use std::sync::Arc;
use simplified_schema::PositionMap;

let positions = PositionMap::deferred(Arc::from(yaml));
positions.locate_all(&mut []);           // builds nothing
positions.locate_all(&mut problems);     // builds once, here
assert_eq!(problems[0].line, Some(2));
```

Envelope admission returns a deferred map, so admitting a valid envelope
builds none.

A YAML alias (`*name`) has no text for the value it expands to, so a problem
at an alias, or anywhere below one, is reported at the `*name` token. Every
other node in the document keeps its own position. In

```yaml
base: &b {port: x}
one: *b
```

against a schema requiring a numeric `port`, `/base/port` is line 1,
column 17 (the `x`) and `/one/port` is line 2, column 6, with `byte_range`
covering only `*b`. When the authored YAML cannot be matched to the parsed
value mapping for mapping (for example a key with an anchor, `&k a: 1`, or an
unquoted `{{ }}`), the map falls back to top-level keys: a problem is reported at
the line of the top-level key it falls under, column 1.

## Source-Aware Parsing and Located Failures

An editor or linter that underlines a schema error needs to know *where* the
rejected text was written, not just what was wrong. Four passive functions
return that location along with the semantic error. None of them reads a
file, resolves an import, or evaluates anything.

| Function | Input | Success | Failure |
|---|---|---|---|
| `parse_property_definition_with_source(property, &value, yaml_source, source_offset)` | one property definition | `SourceAware<PropertyDef>` | `LocatedDefinitionFailure` |
| `parse_schema_declaration_with_source(&value, yaml_source, source_offset)` | one whole declaration, such as a frontmatter `$schema` value | `SourceAware<SchemaDeclaration>` | `LocatedDefinitionFailure` |
| `parse_schema_table_with_source(&value, yaml_source, source_offset)` | a table of named declarations, such as a value typed `schema(mapping)`, read as a `types` table is | `SourceAware<SchemaDeclaration>` | `LocatedDefinitionFailure` |
| `locate_standalone_definition_failure(source)` | a whole standalone schema file | — | `Option<LocatedDefinitionFailure>` |

A successful `SourceAware<T>` holds the same `value` the source-free parser
returns and a `source_map` of authored byte ranges. The source-free parsers
(`parse_property_definition`, `parse_schema_declaration`, `parse_schema_table`, `parse_yaml_schema`)
still return `Result<_, SchemaError>` with the same variants and messages.

A failure carries five fields:

```rust
pub struct LocatedDefinitionFailure {
    pub path: SchemaSourcePath,              // the rejected node
    pub error: Box<SchemaError>,             // what the source-free parser reports
    pub authored_span: Option<Range<usize>>, // the rejected token
    pub value_span: Option<Range<usize>>,    // the whole rejected value
    pub related: Vec<LocatedRelated>,        // secondary locations, as labeled byte ranges
}
```

For example:

```rust
let source = "tags:\n  - string\n  - string(min(nope))\n";
let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(source)?;
let failure = parse_schema_declaration_with_source(&value, source, 0).unwrap_err();
assert_eq!(failure.path, SchemaSourcePath::root().property("tags").union_arm(1));
assert_eq!(&source[failure.authored_span.unwrap()], "nope");
```

- **`path`** is structural, never rebuilt from a message. Mapping keys are
  literal (`"a.b"` is one key), and sequence members, including root-union
  arms, are zero-based `union_arm` indexes. A `...name` operand is a
  `Spread(i)` segment, the `i`-th spread of its inline object, displayed
  `[...i]` (`order[...0]`), and a `$extends` list item is a `union_arm`
  under `$extends` (`$extends[1]`). A property call starts at that
  definition, a declaration call at the declaration, and the standalone
  helper at the envelope, so its paths begin with `types` or `$schema`.
- **The node that introduced the violation is the one reported.** In
  `color: [string(suggest(red)), string(suggest(blue))]` each arm is valid
  alone, but a property allows one `suggest`; the failure is
  `color[1]` with its token at `blue`.
- **Ranges are UTF-8 byte offsets into the caller's document**, with
  `source_offset` added once. A DMLS client converts them to its negotiated
  position encoding.
- **`authored_span` is `None`** when the failure has no token (an empty union
  `tags: []`, a wrong-type arm `tags: [string, 7]`), when
  the presentation is outside the
  [source-aware boundary](language/types.md#source-aware-presentation-boundary) (such as a
  decorated mapping key on a separate line), or when the
  authored text does not decode to the parsed value. `value_span` is then the
  rejected value when that can still be located, and `None` otherwise. The
  `error` is always returned; a missing location never hides it. A failure in
  an aliased value points to the alias token, including when the value itself
  is a sequence or mapping.
- **A successful semantic parse whose source cannot be projected** returns
  the projection error at the root path with no ranges.

`locate_standalone_definition_failure` recognizes a file exactly as
`parse_standalone_schema_document` does (an informal `$schema` envelope, a
`kind: schema` document with `$schema`, `types`, or both, or a
`kind: yaml-schema` document). It returns the
first definition failure in parser order, `$schema` before `types`,
including one inside a root-union mapping arm (`$schema[1].b`). It
returns `None` for a valid file, ordinary YAML, invalid YAML, and every
envelope or declaration-shape error, including an envelope its definition
rejects, which `parse_standalone_schema_document` reports inside
`SchemaError::SchemaDocument`.

For partial input, where no document parses yet, these helpers read a
scalar the way YAML folds it:

- `decode_scalar_node_prefix(source, start, parent_indent, cursor)` decodes
  the part of the scalar node at `start` written before `cursor`, in every
  presentation: plain, single- or double-quoted, literal (`|`), and folded
  (`>`). A quoted scalar is read only up to the cursor, so one whose closing
  quote has not been typed yet still decodes; for `event: 'base@` followed by
  the line `  th`, the prefix at the end is `base@ th`. It returns `None`
  for an alias, a collection, an empty value (whatever follows on later
  lines is another node), and a cursor outside the scalar (past its closing
  quote or its last line).
- `decode_plain_scalar_node(source, start, parent_indent)` decodes the plain
  scalar whose node starts at `start`, after any same-line anchor or tag,
  and returns its decoded map and end offset. It returns `None` for a quoted
  or block scalar, a collection, an alias, a comment, an empty value, and an
  anchor whose scalar starts on a later line.
- `locate_type_definition_cursor_in_scalar(source, &scalar, cursor)`,
  `locate_schema_declaration_cursor_in_scalar(source, &scalar, cursor)`, and
  `locate_composition_operand_cursor_in_scalar(source, &scalar, cursor)` are
  the cursor parsers over a scalar either helper decoded. An anchor or tag never enters
  the reported token. The cursor must sit inside the scalar, at its end, or
  after trailing spaces or tabs on its last line.

## Error Model

Every failure of the language itself is a variant of `SchemaError` (the
`SimplifiedSchema` object's input failures are `SimplifiedSchemaError`, which
wraps it; see [above](#simplifiedschemaerror-wraps-schemaerror)):

| Variant                 | Meaning                                                                                                   |
|-------------------------|-----------------------------------------------------------------------------------------------------------|
| `Aggregate`             | Several independent property definitions failed; each child keeps its own property and source information. |
| `ReferencedSchema`      | A definition failure inside a referenced schema file; carries that file's `path` and the original error as `source`. |
| `Grammar`               | A type-and-constraint string could not be parsed. Carries property name, message, and byte span.          |
| `Unresolved`            | `$schema` reference is malformed or `FileReference` rejected it; carries the typed resolver error.        |
| `NoMatch`               | `$schema` reference matched no file (a path, or a bare name no schema root holds); a text with `*`, `?`, or `[` adds the literal-glob hint. |
| `NotASchemaDocument`    | A file a `$schema` names is not a schema definition ([hints](#when-a-file-is-not-a-schema-target)). Carries the `path` and a `NotSchemaReason`: `TypesOnly` (a `kind: schema` file with `types` and no `$schema`; hint: use a named-type import `Name@file.yaml`), `NoSchemaProperty` (a YAML mapping without `$schema`), `NotAMapping` (the root is not a mapping or, for `.json`, an object), `JsonMissingDialect` (a JSON object without a string `$schema`), `SelectedByKind` (a `kind: yaml-schema` file, which is applied by `kind` and is never a `$schema` target or `Name@file` import source; hint: move shared types into a `kind: schema` file and import them from there), `NamespaceCatalog` (a `kind: namespace-schema` file), `CallableCatalog` (a `kind: expression-schema` or `kind: mutation-schema` file, well formed or not), `TriggerEnvelope` (a `kind: schema-trigger` file), or, for a `Name@file` import source only, `RootUnionImport` / `ReferenceImport` (a schema with no named types of its own). JSON reasons carry the hint to declare `"$schema": "https://json-schema.org/draft/2020-12/schema"`. A role reason ([`is_role`](#where-a-definition-failure-points)) also carries the `site` of the declaration that named the file. |
| `UnsupportedJsonDialect` | A referenced `.json` file declares a `$schema` dialect other than exactly `https://json-schema.org/draft/2020-12/schema` (an empty string or a trailing `#` included). Carries the `path`, the `declared` text, and the `site` of the declaration that named the file. |
| `InvalidJsonSchema`     | A referenced `.json` file declares the Draft 2020-12 dialect but fails its meta-schema (checked offline). Distinct from a format rejection. Carries the `path` and the validator's message. |
| `RemoteUnsupported`     | `http://` / `https://` `$schema` references are rejected in v1.                                           |
| `BundleBoundary`        | A bundle scope refused a reference that is not a listed member reached by `./` or `../` (see [Selecting a schema file](./selecting-schema-files.md#resolving-inside-a-bundle)). |
| `CallableCatalog`       | A callable catalog (`kind: expression-schema` or `kind: mutation-schema`) fails a host's check of its callables (a missing or disagreeing category, a callable mixed with an ordinary type, a duplicate or colliding name, an invalid overload). Carries the catalog `path`, the byte `span` of the failing declaration or field (`None` when only the file is known), and `related`: each other location a conflict involves (`CatalogRelated { message, file, span }`), such as the earlier declaration a duplicate name repeats. The library defines the representation; a host's catalog reader produces it. |
| `SchemaDocument`        | A recognized standalone schema document (informal, `kind: schema`, `kind: yaml-schema`, or a callable catalog envelope) fails its envelope definition ([details](#an-envelope-its-definition-rejects)) or has a payload the parser rejects, or a schema document's text is not a schema (not valid YAML or strict JSON, or an untagged `$schema` mapping with an unsupported top-level key, whose message adds the `.json` migration when `$schema` is a `json-schema.org` URI); carries the document `path`, and, for a rejected definition, the typed cause as `source` and its byte `span` in the supplied text ([details](#a-rejected-definition-in-a-schema-document)); an envelope rejection carries a `span` and no `source`. |
| `FrontmatterShape`      | `$schema` is present but is not a mapping, sequence, or string.                                           |
| `Baseline`              | Baseline could not be loaded or is not a simple object schema.                                            |
| `UnresolvedSlots`       | The schema is a [template](./language/slots.md): it holds `{{ … }}` slots and cannot be lowered, validated, or layered yet. Lists every slot (expression, spans, defining file); `pending` is the handle that resolves it when the call that met it can (`None` from lowering alone). |
| `SlotResult`            | A resolved slot's value cannot fill its position. Carries the `slot` (its `span`, in `slot.origin`'s file when set) and a `SlotResultKind`: `Empty`, `WrongShape { expected, found }`, `Inadmissible(reason)`, or `PatternRejected(reason)` (a slot pattern the linear engine must not take). See [What a Slot's Value Must Be](./language/slots.md#what-a-slots-value-must-be). |
| `Convert`               | SimplifiedSchema could not be lowered to JSON Schema (e.g. conflicting `default(...)` on a union, contradictory bounds or schemes on one atom, a `$constraints` key the block's object cannot use, an unresolved `@`-import reaching conversion, or a `<pattern::RE>` that cannot be wrapped for literal-key precedence). |
| `InvalidDefault`        | A default that a value needs breaks its own property's definition: a definition on its own arm, or every alternative of a union it is declared outside of. Carries the property `path`, the JSON Pointer of the `default` keyword (`pointer`), and the validator's `message`. Reported inside `ValidationFailure::Unprepared`; a default no value needs is never judged. See [Applying defaults](./validation.md#applying-defaults). |
| `ConflictingDefaults`   | One object location declares different defaults for one property: on two object arms of a union, or in raw JSON Schema. Carries the location's `pointer`, the `property`, and each `default` keyword's pointer (`declarations`). Raised when the schema is lowered or prepared, before any value, except for a key that no schema names and several patterns or catch-alls select with different defaults: that conflict depends on the key, so it is reported by validation, inside `ValidationFailure::Unprepared`, only when the key needs its default. See [Applying defaults](./validation.md#applying-defaults). |
| `RecursiveDefault`      | Applying defaults would never end: a `$ref` chain returns to itself without describing a child value (`path` is `None`), or a default keeps creating another defaulted parent (`path` names where it repeats). `reference` is the reference, or the `default` keyword's pointer. |
| `ImportCycle`           | A chain of named-type references (bare or `Name@file`) that never repeats a type is deeper than 32 levels. A type that refers back to itself is not this error. |
| `NonDescendingCycle`    | Named types form a cycle that never passes through a child value (an alias cycle, a self-union, a cycle through a `function(...)` signature, or a loop of `...name` / `$extends` and alias hops). Carries every reference of each rejected cycle as a `CycleReference` with its site, target, and, in a schema file, its span. See [Recursive Named Types](language/objects-and-named-types.md#recursive-named-types). |
| `UnresolvedReference`   | A local `$ref` names no definition in the schema it was written in, or a chain of references never reaches a schema. Carries the schema's label, the reference, and its JSON Pointer location. |
| `UnsupportedMergeReference` | A schema joined at a merge (a baseline or a root-union arm; a trigger payload reports it inside `TriggerPayloadNotMergeable`) uses a reference form whose meaning would change at the merged root (`$id`, an anchor, a dynamic or external reference, or a local reference outside `#/$defs/…`). See [JSON Schema Baseline Restrictions](../../../darkmatter/docs/topics/schemas/definition.md#json-schema-baseline-restrictions). |
| `UnknownNamedType`      | A named-type reference names a type its schema does not define. Carries the name, the schema searched (`SchemaSourceId`), and the reference site (`SchemaReferenceSite`). |
| `InvalidCompositionTarget` | A `...name` or `$extends` operand resolved to a type that is not one object with a declared shape. Carries the operand as written, its site, the deciding `definition` site, and a `CompositionTargetKind`. See [Eligible targets and errors](language/objects-and-named-types.md#eligible-targets-and-errors). |
| `CompositionReference`  | A composition operand could not be looked up: its file failed to resolve or load, or the composition chain passed 32 levels. Carries the operand, its site, and the original cause as `source`. |
| `ComposedMember`        | Lowering failed inside a member copied in by composition. Carries the member's declaration site and the failure as `source`. |
| `ConstraintConflict`    | Constraints merged onto a reused named type contradict each other (`Bounds`, `Scheme`, `Default`, or `Options` for a different `alphanumeric` option set). Carries every conflicting declaration as a `SchemaConstraintParticipant`: its `origin` (a `SchemaConstraintOrigin`) and the `constraint` value that took part. See [Merging use-site constraints](language/objects-and-named-types.md#merging-use-site-constraints). |
| `UnionReference`        | A reference to a union-typed named type adds something it cannot take. Carries the name, the `SchemaUnionRejection` (`Constraint`, `ArrayArm`, or `ScalarDefault`), the reference site, and the union's definition site. See [Named Union References](language/unions.md#named-union-references). |
| `ReferenceCycle`        | A `$schema` file-reference chain revisits a file it is already resolving; `chain` lists the files in visit order, and `site` is the `$schema` declaration that closed the loop. |
| `ReferenceDepthExceeded` | A chain of distinct `$schema` file references is deeper than `limit` (32) files. Not a cycle: no file repeats. `site` is the `$schema` declaration that made the last hop. |
| `BareNameSiblingExists` | A bare schema name matched no schema root, but a file of that name sits beside the document; `suggestion` is the `./`-prefixed reference to use. See [Selecting a Schema File](./selecting-schema-files.md) for how a reference picks its file. |
| `InvalidExample`        | An `example(...)` file is malformed or fails validation against the example envelope or inherited target shape. |
| `BuildValidator`        | `jsonschema` could not build a validator from the produced schema.                                        |
| `Io`                    | Filesystem read failure with the offending path.                                                          |

The remaining variants (`TriggerMatch`, `TriggerLoad`, `EmbeddedBundle`, and
the other `Trigger*` and `EmbeddedTriggerLoad` variants) describe a host's
schema-trigger files and embedded schema bundles. Darkmatter raises them; they
live here so that every schema failure is one type. `TriggerMatch` carries an
optional byte `span` of the trigger-file node its message names, filled in by
the host that holds the file's text.

`KindSelection { kind, cause }` is raised by a host that selects a whole YAML
document's schema by its `kind` (Darkmatter's `kind: yaml-schema` registry):
the document's `kind` has no single usable definition. `cause` is a
`KindSelectionCause`:

| Cause | Meaning |
|---|---|
| `Duplicate { files }` | Two or more definitions claim the kind; every file is named, sorted by path (registered definitions, which are consulted only for a kind no shipped or local definition claims, sorted by host, then definition path). |
| `Reserved { file, defect }` | The one definition claiming the kind failed; `defect` is its failure. |
| `Unavailable { file, defect }` | No healthy definition claims the kind, and `file` failed before revealing what it claims. |
| `RootUnreadable { root, defect }` | A folder searched for definitions could not be read. |

```text
cannot select a schema for `kind: roster`: it is claimed by more than one definition: `a.yaml`, `b.yaml`
```

With the opt-in `terminal` feature (see [Cargo features](./features.md)),
`SchemaError` implements `biscuit_terminal::errors::BlockError`, so a host
that renders errors in a terminal, as Darkmatter does, shows each variant as a
rich status block. Without the feature the errors are ordinary
`std::error::Error` values with their `Display` messages.
