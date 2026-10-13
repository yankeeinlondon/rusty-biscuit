Part of [Symbol Records](../symbol-records.md).

# Schema Versions

Every symbol record, every file index and every `hug` JSON envelope carries a
`schema_version`:

```json
"schema_version": { "major": 5, "minor": 2 }
```

The current version is **5.2**. A consumer can check it before reading
anything else and refuse a major version it does not know.

## How the version moves

- **minor** moves when the record gains a field or a key a reader may now
  see.
- **major** moves on a breaking change: a renamed or removed field, a removed
  enum variant, or a field whose meaning changes.

A change to the extraction rules that changes no shape (a new language rule,
a field filled that was previously empty) does not move the schema version.
It moves the extractor revision instead, which appears in each record's
`provenance.extractor_version` (`0.1.0+rev.50`) and is part of the cache key,
so the symbol cache never serves records made under older rules.

The symbol cache applies a stricter rule of its own: it serves an entry only
when the entry and every record in it are exactly the current version, minor
included, and recomputes anything else. Cache entries are never migrated (see
[Cache Design](../cache-design.md)).

Neither the schema version nor the extractor revision is part of a record's
`id`, so IDs survive both.

## History

| Version | Change |
|---|---|
| 2.0 | the first emitted record shape |
| 3.0 | kinds decided at extraction (`Constructor`, `Property`, `TypeAlias` and `EnumVariant` no longer collapse to `Unknown`); visibility for every kind, with the C# compound levels; the `StaticStorage` modifier; `identity.container_kind`; `source.declaration_text`; Rust `relations.implements`; `is_exported` no longer derived from a `Public` level |
| 4.0 | `identity.module_path` renamed `container_name`; `identity.keyword` added; `source.signature_text` filled; `provenance.created_at_epoch_ms` and `updated_at_epoch_ms` removed |
| 4.1 | the required `metrics.lines` key inside a present `metrics` facet |
| 4.2 | `relations.unclassified_bases`, and the optional `source_span` on a relation's symbol reference |
| 4.3 | the required `metrics.cyclomatic` key inside a present `metrics` facet |
| 5.0 | `relations.references`, `dependencies`, `referenced_by` and `semantics.is_recursive` are derived from scope-aware reference occurrences instead of the first same-named declaration (a change of meaning, so a major bump); a file index may carry `occurrences` and `reference_coverage` |
| 5.1 | the optional `import_span` on an occurrence, naming the particular import binding a reference uses |
| 5.2 (current) | the required `imports[].reexported` marker and the optional file-index `reexports` report |

Only 5.2 is emitted today.

### Fields added without a bump

Several fields were added while records still said 4.0, before the
minor-version rule was applied: `scope`, `identity.qualified_name` (always
present), `identity.is_namespace_rooted`, the `Async` modifier,
`visibility.is_test`, values for `visibility.is_deprecated`, and the
`metrics` facet itself. A consumer of early 4.0 output only meets new keys,
but `scope`, `identity.qualified_name`, `identity.is_namespace_rooted` and
`visibility.is_test` are required when a record is read back, so a record
written before them does not deserialize.

### 3.0 to 4.0, by example

```json
// 3.0
"identity": { "name": "fmt", "module_path": "PublicStruct", "container_kind": "Type" },
"source": { "declaration_text": "fn fmt(&self, f: &mut Formatter) -> Result" },
"provenance": { "extractor": "tree-sitter", "extractor_version": "0.1.0+rev.4",
                "parse_pass": "docs", "created_at_epoch_ms": 1759500000000,
                "updated_at_epoch_ms": 1759500000004 }

// 4.0
"identity": { "name": "fmt", "container_name": "PublicStruct", "container_kind": "Type",
              "keyword": "fn" },
"source": { "declaration_text": "fn fmt(&self, f: &mut Formatter) -> Result",
            "signature_text": "fn fmt(&self, f: &mut Formatter) -> Result" },
"provenance": { "extractor": "tree-sitter", "extractor_version": "0.1.0+rev.6",
                "parse_pass": "docs" }
```

### 4.3 to 5.0, by example

5.0 keeps every 4.3 key and every declaration ID; what changed is what the
reference edges say. A 4.3 edge bound a name to the first declaration spelled
that way anywhere in the file; a 5.0 edge names only a target the language's
scope rules establish:

```rust
fn first() { let total = 1; }
fn second(total: u32) -> u32 { total }
// 4.3: second.references = [first::total]   (the first `total` in the file)
// 5.0: second.references = [second::total]  (the parameter in scope)
```

Languages without resolver rules have no reference edges at all in 5.0, and
`is_recursive` is `Unknown` wherever the evidence cannot prove `Yes` or `No`.
The upgrade is described from the references side in
[Scope-Aware References](../references.md).

## The older `SymbolInfo` model

The library still offers its first symbol model, `SymbolInfo`
(`TreeFile::symbols()`), alongside `SymbolRecord`, with conversions both ways:
`SymbolRecord::from(symbol_info)` and `SymbolInfo::try_from(record)` (which
fails only for a record with an empty name).

| `SymbolInfo` | `SymbolRecord` |
|---|---|
| `name` | `identity.name` |
| `kind` | `kind` |
| `scope` | `scope` |
| `qualified_name`, `is_namespace_rooted` | `identity.qualified_name`, `identity.is_namespace_rooted` (an empty `qualified_name` converts as the bare name, and the ID is keyed on it) |
| `range` | `source.declaration_span` |
| `container_name`, `container_kind` | `identity.container_name`, `identity.container_kind` |
| `doc_comment` | `docs.raw_doc` |
| `visibility` (`None`) | `visibility.level` (`Unknown`) |
| `is_static`, `is_static_storage`, `is_async` | the `Static`, `StaticStorage` and `Async` modifiers |
| `signature` | a `Function` payload |
| `type_metadata` | a `Type` or `Enum` payload |

The kind and visibility conversions cover every variant on both sides. The
conversion does not carry `keyword`, `declaration_text`, `signature_text`,
`relations` (`extends` and `implements` included), `semantics`, the
`is_test` and `is_deprecated` flags, `metrics`, `attributes`, any modifier
other than `Static`, `StaticStorage` and `Async`,
`type_info.generic_params` and `type_info.constraints`,
`TypeMetadata::type_parameters`, or a type alias's aliased type. A record
converted to `SymbolInfo` and back has none of these. A record converted
from a `SymbolInfo` is never exported, because export comes from syntax that
`SymbolInfo` does not hold.
