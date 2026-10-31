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
`Unprepared`, `Selection`, and `ExportUnsupported`. A malformed *definition*
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
from the frontmatter's first line). Other document failures, such as an
envelope of the wrong shape or a suggestion list that cannot be linted, have
no `span`; a lint failure keeps its typed cause in `source`.
`locate_standalone_definition_failure` returns the same failure with its
structural path and secondary locations (see
[below](#source-aware-parsing-and-located-failures)).

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
- **`NonDescendingCycle`** lists every `CycleReference` of each rejected
  cycle, with its site, its target, and the source that declares the target.
- **`ReferencedSchema`** wraps an error raised inside another schema file
  with that file's `path`, so per-property detail survives while the
  diagnostic points at the file that authored it.

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

## Source-Aware Parsing and Located Failures

An editor or linter that underlines a schema error needs to know *where* the
rejected text was written, not just what was wrong. Three passive functions
return that location along with the semantic error. None of them reads a
file, resolves an import, or evaluates anything.

| Function | Input | Success | Failure |
|---|---|---|---|
| `parse_property_definition_with_source(property, &value, yaml_source, source_offset)` | one property definition | `SourceAware<PropertyDef>` | `LocatedDefinitionFailure` |
| `parse_schema_declaration_with_source(&value, yaml_source, source_offset)` | one whole declaration, such as a frontmatter `$schema` value | `SourceAware<SchemaDeclaration>` | `LocatedDefinitionFailure` |
| `locate_standalone_definition_failure(source)` | a whole standalone schema file | — | `Option<LocatedDefinitionFailure>` |

A successful `SourceAware<T>` holds the same `value` the source-free parser
returns and a `source_map` of authored byte ranges. The source-free parsers
(`parse_property_definition`, `parse_schema_declaration`, `parse_yaml_schema`)
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
  arms, are zero-based `union_arm` indexes. A property call starts at that
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
  `tags: []`, a wrong-type arm `tags: [string, 7]`, an aliased value), when
  the presentation is outside the
  [source-aware boundary](language/types.md#source-aware-presentation-boundary) (such as a
  quoted scalar continued across lines anywhere in the document), or when the
  authored text does not decode to the parsed value. `value_span` is then the
  rejected value when that can still be located, and `None` otherwise. The
  `error` is always returned; a missing location never hides it.
- **A successful semantic parse whose source cannot be projected** returns
  the projection error at the root path with no ranges.

`locate_standalone_definition_failure` recognizes a file exactly as
`parse_standalone_schema_document` does (a pure `$schema` envelope or a
`kind: schema` document with `types`). It returns the first definition
failure in parser order, including one inside a pure root-union mapping arm
(`$schema[1].b`). It returns `None` for a valid file, ordinary YAML, invalid
YAML, a global catalog, and every envelope or declaration-shape error, which
`parse_standalone_schema_document` reports as before inside
`SchemaError::SchemaDocument`.

For partial input, where no document parses yet, two helpers read a plain
scalar the way YAML folds it:

- `decode_plain_scalar_node(source, start, parent_indent)` decodes the plain
  scalar whose node starts at `start`, after any same-line anchor or tag,
  and returns its decoded map and end offset. It returns `None` for a quoted
  or block scalar, a collection, an alias, a comment, an empty value, and an
  anchor whose scalar starts on a later line.
- `locate_type_definition_cursor_in_scalar(source, &scalar, cursor)` and
  `locate_schema_declaration_cursor_in_scalar(source, &scalar, cursor)` are
  the cursor parsers over that decoded scalar. An anchor or tag never enters
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
| `NotASchemaDocument`    | A file a `$schema` names is not a schema definition. Carries the `path` and a `NotSchemaReason`: `TypesOnly` (a `kind: schema` file with `types` and no `$schema`; hint: use a named-type import `Name@file.yaml`), `NoSchemaProperty` (a YAML mapping without `$schema`), `NotAMapping` (the root is not a mapping or, for `.json`, an object), or `JsonMissingDialect` (a JSON object without a string `$schema`). JSON reasons carry the hint to declare `"$schema": "https://json-schema.org/draft/2020-12/schema"`. |
| `UnsupportedJsonDialect` | A referenced `.json` file declares a `$schema` dialect other than exactly `https://json-schema.org/draft/2020-12/schema` (an empty string or a trailing `#` included). Carries the `path` and the `declared` text. |
| `InvalidJsonSchema`     | A referenced `.json` file declares the Draft 2020-12 dialect but fails its meta-schema (checked offline). Distinct from a format rejection. Carries the `path` and the validator's message. |
| `RemoteUnsupported`     | `http://` / `https://` `$schema` references are rejected in v1.                                           |
| `SchemaDocument`        | A recognized standalone schema document (pure or tagged envelope) has a missing or malformed payload, or a schema document's text is not a schema; carries the document `path`, and, for a rejected definition, the typed cause as `source` and its byte `span` in the supplied text ([details](#a-rejected-definition-in-a-schema-document)). |
| `FrontmatterShape`      | `$schema` is present but is not a mapping, sequence, or string.                                           |
| `Baseline`              | Baseline could not be loaded or is not a simple object schema.                                            |
| `Convert`               | SimplifiedSchema could not be lowered to JSON Schema (e.g. conflicting `default(...)` on a union, contradictory bounds or schemes on one atom, a `$constraints` key the block's object cannot use, an unresolved `@`-import reaching conversion, or a `<pattern::RE>` that cannot be wrapped for literal-key precedence). |
| `ImportCycle`           | A chain of named-type references (bare or `Name@file`) that never repeats a type is deeper than 32 levels. A type that refers back to itself is not this error. |
| `NonDescendingCycle`    | Named types form a cycle that never passes through a child value (an alias cycle, a self-union, or a cycle through a `function(...)` signature). Carries every reference of each rejected cycle as a `CycleReference` with its site, target, and, in a schema file, its span. See [Recursive Named Types](language/objects-and-named-types.md#recursive-named-types). |
| `UnresolvedReference`   | A local `$ref` names no definition in the schema it was written in, or a chain of references never reaches a schema. Carries the schema's label, the reference, and its JSON Pointer location. |
| `UnsupportedMergeReference` | A schema joined at a merge (a baseline or a root-union arm; a trigger payload reports it inside `TriggerPayloadNotMergeable`) uses a reference form whose meaning would change at the merged root (`$id`, an anchor, a dynamic or external reference, or a local reference outside `#/$defs/…`). See [JSON Schema Baseline Restrictions](../../../darkmatter/docs/topics/schemas/definition.md#json-schema-baseline-restrictions). |
| `UnknownNamedType`      | A named-type reference names a type its schema does not define. Carries the name, the schema searched (`SchemaSourceId`), and the reference site (`SchemaReferenceSite`). |
| `ConstraintConflict`    | Constraints merged onto a reused named type contradict each other (`Bounds`, `Scheme`, `Default`, or `Options` for a different `alphanumeric` option set). Carries every conflicting declaration as a `SchemaConstraintParticipant`: its `origin` (a `SchemaConstraintOrigin`) and the `constraint` value that took part. See [Merging use-site constraints](language/objects-and-named-types.md#merging-use-site-constraints). |
| `UnionReference`        | A reference to a union-typed named type adds something it cannot take. Carries the name, the `SchemaUnionRejection` (`Constraint`, `ArrayArm`, or `ScalarDefault`), the reference site, and the union's definition site. See [Named Union References](language/unions.md#named-union-references). |
| `ReferenceCycle`        | A `$schema` file-reference chain revisits a file it is already resolving; `chain` lists the files in visit order. |
| `ReferenceDepthExceeded` | A chain of distinct `$schema` file references is deeper than `limit` (32) files. Not a cycle: no file repeats. |
| `BareNameSiblingExists` | A bare schema name matched no schema root, but a file of that name sits beside the document; `suggestion` is the `./`-prefixed reference to use. |
| `InvalidExample`        | An `example(...)` file is malformed or fails validation against the example envelope or inherited target shape. |
| `BuildValidator`        | `jsonschema` could not build a validator from the produced schema.                                        |
| `Io`                    | Filesystem read failure with the offending path.                                                          |

The remaining variants (`TriggerMatch`, `TriggerLoad`, `EmbeddedBundle`, and
the other `Trigger*` and `EmbeddedTriggerLoad` variants) describe a host's
schema-trigger files and embedded schema bundles. Darkmatter raises them; they
live here so that every schema failure is one type.

With the opt-in `terminal` feature (see [Cargo features](./features.md)),
`SchemaError` implements `biscuit_terminal::errors::BlockError`, so a host
that renders errors in a terminal, as Darkmatter does, shows each variant as a
rich status block. Without the feature the errors are ordinary
`std::error::Error` values with their `Display` messages.
