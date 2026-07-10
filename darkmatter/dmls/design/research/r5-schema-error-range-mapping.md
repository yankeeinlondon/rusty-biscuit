---
prompt: |-
  DMLS (the Darkmatter Language Server) must map SimplifiedSchema validation
  results onto precise frontmatter source ranges so editors can underline the
  exact key or value at fault. Context:
  @darkmatter/features/2026-07-04-dmls/spec.md (Layer 2) and
  @darkmatter/features/2026-07-04-dmls/design.md (FrontmatterAst).

  Part 1 — repository research (read, do not modify):

  1. Study `darkmatter::markdown::schemas` (darkmatter/lib/src/markdown/
     schemas/**): what error/problem types does validation produce today?
     Do they carry JSON-Pointer-like paths, dotted paths, key names only, or
     free text? How do baseline merging, coercion, `file(eager)`
     normalization, and pending-`$(...)` deferral surface in the error
     shapes?
  2. Study how `md schema validate` renders those problems in the CLI — that
     rendering is the closest existing consumer to what DMLS needs.

  Part 2 — design:

  3. Propose a diagnostic taxonomy with stable codes for: YAML parse
     failure, invalid `$schema` shape, schema preparation error, type
     mismatch, constraint violation, missing required key, unknown key
     (strict mode), deprecated key, invalid file reference, and
     pending-shell-value notices. For each, define the ranging rule: which
     FrontmatterAst node gets the squiggle (key, value, whole entry, or the
     frontmatter block for missing keys) and what relatedInformation points
     at (e.g. the schema source line).
  4. Identify the gaps: which error types need library API changes to carry
     enough path data, and propose those changes at signature level.
  5. Survey how established schema-driven language servers solve this
     (Red Hat yaml-language-server, taplo for TOML): how do they map
     instance paths to ranges, and what conventions do their diagnostic
     codes follow?

  Deliverables: the current-state error-shape inventory, the diagnostic
  taxonomy table with ranging rules, and the prioritized library-change
  list.
last_updated: 2026-07-06
hash: 5721dd84352adf64-fef3981fa13f7294
---
# DMLS Frontmatter Diagnostic Mapping

## Current State Inventory

`darkmatter::markdown::schemas` currently produces two distinct shapes:

| Source              | Public shape        | Path fidelity                               | Notes                                                                                                                                                           |
|---------------------|---------------------|---------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Schema preparation  | `SchemaError`       | Mostly no instance path                     | Covers SimplifiedSchema grammar/convert, `$schema` reference resolution, baseline merge, validator build, I/O, remote unsupported, and invalid `$schema` shape. |
| Instance validation | `ValidationProblem` | JSON Pointer string + top-level line/column | Produced from `jsonschema::ValidationError`; carries `path`, `kind`, `property`, `line`, `column`, `arm_index`, `description`.                                  |

`ValidationProblem.path` is a JSON Pointer-like instance path such as `/title`, `/tags/2`, or `/config/name`. For `required`, the path points at the parent object and `property` carries the missing key. `ValidationProblemKind` is currently only `Missing`, `Type`, or `Invalid`, so constraints, unknown keys, enum mismatches, file-reference failures, etc. are all collapsed into `Invalid`.

The current source map is too coarse for DMLS. `PositionMap` is only top-level key -> 1-based line/column. `build_position_map` scans raw YAML lines and records column 1 for top-level keys only. Nested object keys, array items, scalar value spans, quoted key spans, and value ranges are not represented.

Baseline merging happens before validator construction. `resolve::merge_baseline` merges baseline JSON Schema properties under the document schema; document properties win on conflicts, and required arrays are unioned. The resulting `ValidationProblem` does not say whether the failing rule came from the baseline or the document schema.

Coercion happens before validation. `EffectiveSchema::validate_with_positions` validates a coerced copy via `coerce::coerce_frontmatter`, so boolish/numberlike/scalar conversions can make authored YAML pass even when the raw scalar type differs. The problem shape does not say whether a value was coerced, skipped, or failed after attempted coercion.

`file(eager)` validation and normalization are separate. `file(eager)` lowers to `format: darkmatter-file`; validation failures surface as `ValidationProblemKind::Invalid` with a substituted message from `darkmatter_file_format_message`. Successful eager file values are normalized later through `EffectiveSchema::normalize_frontmatter`, but normalization changes are not represented in diagnostics.

Pending `$(...)` and unresolved `{{ ... }}` values are handled only in the compose-time path. `compose::schema_validation` builds a `composition_pending` top-level key set, uses `coerce_frontmatter_with_pending`, defers problems attributable to pending keys when a later shell-expansion pass will revalidate, and skips eager-file normalization for pending keys. The public standalone validation API does not return pending notices; DMLS must add its own static notice layer unless the library exposes this path.

`md schema validate` is the closest existing consumer. Pretty output renders each `ValidationProblem` with `arm[N]`, path or missing property, message, optional `line N of frontmatter`, and optional schema description. JSON output emits `path`, `property`, `message`, `kind`, `line`, `column`, `arm_index`, and `description`. Parse failures and schema-preparation failures are not emitted as `ValidationProblem`; they are separate outcome variants.

Style warnings already have a path-shaped API: `StyleWarning { path, kind, source_span }`. `kind` distinguishes `UnknownKey`, `Deprecated`, and `KnownButInactive`, but `source_span` is always `None` today.

## Diagnostic Taxonomy

All codes are stable string codes under source `darkmatter.frontmatter` or `darkmatter.schema`.

| Code                               | Severity                        | Source input                                                                                                        | Ranging rule                                                                                                                            | Related information                                                                            |
|------------------------------------|--------------------------------:|---------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------|
| `dm.frontmatter.yaml_parse`        | Error                           | YAML parser diagnostic                                                                                              | Parser-provided span; if unavailable, opening frontmatter delimiter or whole block                                                      | None, unless parser exposes expected token context                                             |
| `dm.schema.invalid_schema_shape`   | Error                           | `SchemaError::FrontmatterShape`                                                                                     | `$schema` value node; if absent from partial AST, `$schema` key; fallback whole frontmatter block                                       | Schema shape help text                                                                         |
| `dm.schema.prepare`                | Error                           | `SchemaError::{Grammar, Convert, Baseline, BuildValidator, AmbiguousReferenced, RemoteUnsupported, Io, Unresolved}` | `$schema` value for document schema errors; configured baseline virtual source for baseline errors; fallback whole frontmatter block    | Referenced schema file/range when known; underlying cause chain                                |
| `dm.schema.type_mismatch`          | Error                           | `ValidationProblemKind::Type`                                                                                       | Value node at `path`; for scalar values underline value only, for object/array type mismatch underline whole value node                 | Schema property/rule range and description                                                     |
| `dm.schema.constraint`             | Error                           | `ValidationProblemKind::Invalid` except dedicated file/unknown cases                                                | Value node at `path`; for array item constraints use item node; for object-level constraints use object value node                      | Schema property/rule range and description                                                     |
| `dm.schema.missing_required`       | Error                           | `ValidationProblemKind::Missing`                                                                                    | Parent mapping node at `path`; if root missing, whole frontmatter block; prefer insertion point after nearest parent key when available | Required property declaration in schema                                                        |
| `dm.schema.unknown_key`            | Warning or Error in strict mode | JSON Schema `additionalProperties` / style `UnknownKey`                                                             | Offending key node, not whole entry                                                                                                     | Parent schema object or strict-mode rule                                                       |
| `dm.schema.deprecated_key`         | Warning                         | `StyleWarningKind::Deprecated` and future schema deprecation metadata                                               | Deprecated key node; for deprecated enum spelling, value node                                                                           | Replacement key/value and schema declaration                                                   |
| `dm.schema.invalid_file_reference` | Error                           | `format: darkmatter-file` / `darkmatter-file-reference` failures                                                    | Value scalar node at `path`                                                                                                             | Resolved anchor info: document directory, fallback directory, and target path if derived       |
| `dm.schema.pending_shell_value`    | Information                     | Static DMLS scan or future library pending result                                                                   | Whole value node containing `$(...)` or unresolved `{{ ... }}`                                                                          | Note that DMLS never executes shell values; link to post-compose validation stage if available |

For missing keys, the diagnostic range is intentionally a real visible range, not zero-width, because many clients render zero-width diagnostics poorly. Code actions can still use a precise insertion edit at the parent mapping.

For unknown-key diagnostics, DMLS needs the actual offending key. JSON Schema often reports `additionalProperties` at the parent object path and puts the unexpected key only in message text. That is not acceptable as the long-term source of truth.

## FrontmatterAst Mapping Rules

`FrontmatterAst` should provide these lookups:

| Input                                              | Lookup                                         | Preferred result                    | Fallback                                      |
|----------------------------------------------------|------------------------------------------------|-------------------------------------|-----------------------------------------------|
| JSON Pointer `/a/b/0`                              | `node_at_pointer`                              | Exact value node                    | Nearest existing ancestor                     |
| JSON Pointer `/a/b/0`                              | `entry_at_pointer`                             | Key/value entry containing the node | Nearest existing ancestor entry               |
| Missing property `{ parent: "/a", property: "b" }` | `mapping_at_pointer("/a")`                     | Parent mapping/block range          | Whole frontmatter block                       |
| Dotted path `style.page.left_margin`               | `entry_at_dotted_path` preserving raw segments | Exact raw key segment range         | Canonicalized path match, then nearest parent |
| `$schema` shape/prep                               | `schema_entry`                                 | `$schema` value range               | `$schema` key, then block                     |

## Required Library API Changes

Priority 1: make validation errors path-complete.

Add a richer problem shape without removing the existing fields:

```rust
pub enum ValidationProblemCode {
    MissingRequired,
    TypeMismatch,
    ConstraintViolation,
    UnknownKey,
    InvalidFileReference,
}

pub struct ValidationProblem {
    pub path: String,
    pub instance_path: JsonPointer,
    pub schema_path: Option<JsonPointer>,
    pub code: ValidationProblemCode,
    pub message: String,
    pub property: Option<String>,
    pub offending_property: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub arm_index: Option<usize>,
    pub description: Option<String>,
}
```

Priority 2: expose schema-origin metadata after baseline merge.

DMLS needs to point `relatedInformation` at the schema source. Preserve origin during `resolve_schema` and `merge_baseline`:

```rust
pub struct SchemaOrigin {
    pub uri: Option<PathBuf>,
    pub simplified_path: Option<JsonPointer>,
    pub source_range: Option<SourceRange>,
    pub origin_kind: SchemaOriginKind,
}

pub struct EffectiveSchema {
    pub json_schema: Value,
    pub origins: SchemaOriginMap,
    // existing fields...
}
```

Priority 3: expose compose-style pending validation as data.

DMLS must not execute shell, but it should mirror compose’s deferral rules:

```rust
pub struct PendingValue {
    pub key: String,
    pub path: JsonPointer,
    pub reason: PendingValueReason, // ShellExpression | UnresolvedTemplate
}

pub struct ValidationOptions {
    pub pending_policy: PendingPolicy,
    pub excluded_keys: HashSet<String>,
}

pub struct ValidationReport {
    pub valid: bool,
    pub problems: Vec<ValidationProblem>,
    pub pending: Vec<PendingValue>,
}
```

Priority 4: classify file-reference failures.

Today eager/lazy file failures are `Invalid` plus a human message. DMLS needs structured causes:

```rust
pub enum FileReferenceDiagnostic {
    InvalidSyntax { raw: String },
    ResolutionFailed { raw: String },
    NoMatch { raw: String, resolved_from: Option<PathBuf> },
}
```

Priority 5: fill style source spans or move style warnings onto the same path/range model.

`StyleWarning::source_span` is already reserved but always empty. Either populate it from `FrontmatterAst`, or change style parsing to return dotted/JSON-pointer paths plus warning codes and let DMLS range them.

## Ecosystem Survey

Red Hat `yaml-language-server` keeps byte offsets on YAML/JSON AST nodes and maps validation problem locations directly to LSP ranges. Its validator creates diagnostics from `p.location.offset` and `p.location.length`, then converts offsets with `textDocument.positionAt`. Its schema-load diagnostics range the `$schema` value when the first property is `$schema`, otherwise the root. Source: [baseValidator.ts](https://github.com/redhat-developer/yaml-language-server/blob/main/src/languageservice/parser/schemaValidation/baseValidator.ts), [yamlValidation.ts](https://github.com/redhat-developer/yaml-language-server/blob/main/src/languageservice/services/yamlValidation.ts).

Taplo follows the same broad pattern for TOML. Syntax, DOM, and schema errors carry text ranges; the LSP layer maps `error.text_ranges().next()` through the document mapper into an LSP range. It also uses `related_information` for paired structural errors such as conflicting keys. Source: [taplo-lsp diagnostics.rs](https://github.com/tamasfe/taplo/blob/master/crates/taplo-lsp/src/diagnostics.rs). Taplo’s public docs describe the language server as schema-aware TOML validation over JSON schemas: [Taplo language server](https://taplo.tamasfe.dev/cli/usage/language-server.html).

The convention to copy is: validator output should identify the semantic failing node, but final ranges should come from the concrete syntax tree, not from rendered YAML line maps or diagnostic message parsing. Diagnostic codes should be stable symbolic strings or enum variants; related information should carry schema/source context rather than overloading the main message.
