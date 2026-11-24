# Schema documents and envelopes: code map

Loaded from the `simplified-schema` skill. How a referenced `$schema` file
is classified, how envelopes are admitted, and where the line between this
crate and Darkmatter runs.

A rejected definition's `SchemaError::SchemaDocument` keeps the parser's
typed error as `source`, and its `span` is a byte range in the exact supplied
text (whole Markdown file, CRLF counted). Never stringify a `SchemaError` into
`message` alone; see [Diagnostics](../../../simplified-schema/docs/topics/diagnostics.md#a-rejected-definition-in-a-schema-document).

A referenced `$schema` file is classified by `classify_yaml_schema_text` /
`classify_json_schema_text` (`resolve.rs`): YAML must be an envelope with a
top-level `$schema` (`types`-only is `NotASchemaDocument { TypesOnly }`, a map
without `$schema` is `NoSchemaProperty`, a `kind: yaml-schema` is
`SelectedByKind` even when malformed, and so is one named by `Name@file`;
a file whose `kind` names the trigger envelope is `TriggerEnvelope`; an
import source that is a root union or whole-file reference is
`RootUnionImport`/`ReferenceImport`; hosts reading a referenced file use
`resolve::parse_referenced_schema_document`). A role refusal
(`NotSchemaReason::is_role`) gets the naming declaration as its `site`
through `SchemaError::refused_at` at the load boundaries.
JSON must be an object whose
`$schema` is exactly `DRAFT_2020_12_DIALECT` (else `JsonMissingDialect` or
`UnsupportedJsonDialect`) and pass the offline meta-schema (else
`InvalidJsonSchema`). There is no raw-YAML JSON Schema fallback; see
[What a schema reference may target](../../../simplified-schema/docs/topics/language/schema-documents.md#what-a-schema-reference-may-target).

Envelopes (`kind: schema`, `kind: yaml-schema`, `kind: namespace-schema`,
`kind: expression-schema`, `kind: mutation-schema`, informal) are
**admitted** before their payload is read: `envelope.rs` validates each
against the compiled `simplified-schema/schemas/schema.yaml` /
`yaml-schema.yaml` / `namespace-schema.yaml` / `expression-schema.yaml` /
`mutation-schema.yaml` (`envelope::ENVELOPE_DEFINITIONS`,
`admit_envelope_text`), softly except for the two callable catalogs
(`StandaloneSchemaEnvelope::is_callable_catalog`), which are admitted
strictly (`validate_strict`: no metadata conversion). Those files are the
only authority for envelope structure (closed keys, `$schema` or `types`,
`types` a mapping, `not-empty` metadata); never add a structural check to
`standalone.rs` (`shipped_corpus.rs` guards it). Rust keeps only category
grammar, duplicate/undeclared categories, and duplicate overloads. Metadata
is read from the converted copy (`description: 123` → `"123"`); `$schema` /
`types` carriers are never converted, and a null carrier is absent
(`RecognizedStandalone::{payload_key, payload, local_types}`: a tagged
`$schema: null` beside `types` is a `types`-only document). Report order: envelope problem, then a
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

Host registration members have one *structural* admission,
`member_envelope.rs` (`admit_member`/`admit_member_text` →
`AdmittedMember {kind, name}` or `MemberRefusal {kind, field, absent, message}`;
`absent` marks an omitted required key, which a reader locates at the
document's mapping rather than at a value):
the four member kinds' closed key sets and value types, std +
`serde_yaml_ng` only because Claudine's `build.rs` includes it by `#[path]`
(the library reaches it as `simplified_schema::member_envelope`). Darkmatter's
`config::registration::member_kind` classifies every member with it before
full preparation. Its null rules mirror the full admission (a null
`yaml-schema`/library `description` is absent, a trigger's/namespace's is
refused), and `tests/l1/member_envelope.rs::the_closed_key_sets_are_the_definitions_keys`
pins `MemberKind::keys` to the definition files; edit both together.
A namespace file's whole check is `resolve::prepare_namespace_document`
(admission + `resolve_namespace_payload` + one-object), shared by
registration (`namespaces::prepare_one`) and the `namespace-schema` kind
check (`yaml_documents/semantic.rs`), which is why `md schema validate` and
DMLS skip resolving it as a schema target.

Null carriers are absent: a null `description`/`categories`/`types` is
omitted, a tagged `$schema: null` beside a usable `types` table is a
types-only document, an untagged document whose `$schema` is null is
ordinary YAML (`standalone.rs::recognize_standalone`), and a trigger's
`$schema: null` is matcher-only. An
*empty* carrier is not absent: `types: {}` and a trigger's `$schema: {}`/`[]`/
blank text are refused (`triggers/envelope.rs::parse_trigger_envelope`
mirrors `schema-trigger.yaml`'s `schema(not-empty)`, so discovery and
whole-YAML loading agree). `envelope_null_carriers.rs` runs every carrier cell
through the parser, admission (normal and `PreparationMode::Uncached`), both
factories, and whole-YAML loading, which must agree.

Ownership boundary: this crate owns all six shipped definitions
(`schemas/{schema,yaml-schema,schema-trigger,namespace-schema,expression-schema,mutation-schema}.yaml`, compiled
into `envelope::ENVELOPE_DEFINITIONS`; all but the trigger definition admit
envelopes, the trigger definition is a kind definition), envelope admission, trigger
discovery and matching (`triggers/`), the schema roots (`roots.rs`), and whole
YAML documents (`yaml_documents/`: `load.rs`, `select.rs`, `registry.rs`'s
`KindRegistry`, `semantic.rs`'s kind checks). A host supplies only its inputs:
its `ShippedSources` and its prepared shipped definitions (`KindHost`;
`prepare_shipped_definitions(sources)` when it records none). Darkmatter
passes `kind_host()`, records the prepared definitions in its own shipped
artifact (`kind_definitions`), and keeps the adaptation of a selected schema
into its `EffectiveSchema`/`ValidationReport` (`YamlContract`, `YamlValidation`,
`Route`). Its user docs: the "YAML documents" section of
[definition.md](../../../darkmatter/docs/topics/schemas/definition.md) and
[Schema Discovery](../../../darkmatter/docs/topics/schemas/schema-discovery.md).

## Namespaces, labels, and bundles

- **Callable catalogs** (`kind: expression-schema`, `kind: mutation-schema`)
  have closed, strictly admitted envelopes (`kind`, optional `description`,
  required non-empty `categories` and `types`); `check_written_labels`
  refuses explicit `null` metadata and a bare-null `types` entry. They are
  private like namespaces: `parse_referenced_schema_document_with_mode` (the
  one reader every disk, embedded, and bundle reference goes through),
  `check_standalone_named_types`, both factories (`schema/layer.rs`,
  `schema/export.rs`), and Darkmatter's `schema_files.rs::parse` refuse them
  as `NotSchemaReason::CallableCatalog`, recognized leniently by
  `claimed_envelope` so a malformed catalog is refused for its kind too.
  Their whole-YAML kind check (`semantic.rs::callable_catalog_checks`) is
  syntax-only (the selected parse, nothing resolved). Callable semantics
  (category inheritance, callable-vs-helper, names) belong to Darkmatter's
  catalog reader, which reports `SchemaError::CallableCatalog { path,
  message, span, related: Vec<CatalogRelated> }`. On a duplicate-key load
  error, `yaml_documents::duplicate_key(text)` returns both spellings
  (`marks::duplicate_key_marks`) for a related location.

- **Namespace files** (`kind: namespace-schema`) are catalog inputs only.
  `simplified/standalone.rs::parse_namespace_schema_document` admits the
  envelope and refuses explicit `null` for `description`/`types` (the
  definition cannot: an optional property admits `null`); whole-YAML loading
  reaches the same refusal through the `namespace-schema` kind check
  (`prepare_namespace_document`, see above).
  `resolve.rs::resolve_namespace_payload` resolves `$schema` beside the
  private `types` through the trigger-payload path (`InlineOwner::Trigger`),
  so the file owns its inline types and no other file reaches them. Naming a
  namespace file as a `$schema` target, root-union arm, or `Name@file` source,
  or loading it through the `SimplifiedSchema` factories, is
  `NotSchemaReason::NamespaceCatalog` (`resolve.rs`, `schema/layer.rs`,
  `schema/export.rs`), even when malformed. Reserved names (Darkmatter
  globals, expression roots, `null`) are the host's check
  (`darkmatter/lib/src/config/registration/namespaces.rs`), not this crate's.
  Tests: `kind_envelopes.rs` (`namespace_schema_*`,
  `files::a_namespace_file_is_refused_*`).
- **Labels.** A trigger's or `types`-only library's `name` is a kebab-case
  display label (`schema-trigger.yaml`/`schema.yaml` pattern; the envelope
  parsers refuse `null`). `triggers/discovery.rs` judges uniqueness per
  physical root and per embedded bundle, before shadowing
  (`duplicates`, `duplicate_bundle_names`, `duplicate_name_finding`): every
  participant becomes an `InvalidTriggerFinding` located at its `name`
  (reusing `SchemaError::TriggerMatch`, so a library participant reads as a
  trigger finding) and none is installed. A refused file's label is read
  leniently by `member_envelope::declared_name` (a valid `name` on a trigger
  or `$schema`-free `kind: schema`, whatever else is wrong; never an invalid
  one), the one rule discovery's skipped findings, Darkmatter's registration
  warnings, and Claudine's build errors share. Diagnostics show `name (path)`;
  machine output keeps the path and span. Identity, shadowing, and lookups
  stay path-based. Trigger `types` are private to the trigger payload. Tests:
  `kind_envelopes.rs` (`files::a_duplicate_name_*`, `…two_roots…`,
  `…shadowed…`, `…trigger_payload_names_its_own_types…`).
- **Bundles.** `resolve/bundle.rs` is a thread-local scope (like
  `resolve::buffers`) consulted by `select.rs` and `buffers.rs` before any
  probe or open: only explicit `./`/`../` references to listed members are
  served, from the verified bytes; unlisted, escaping, bare, rooted, home,
  magic, variable, absolute, and recursive references are refused
  (`BundleRefusal`, `SchemaError::BundleBoundary`); built-in `ShippedSources`
  names stay allowed. It is independent of `slots::TemplateReads` (a template's
  returned import must pass both). Hosts: Darkmatter's registration loader
  (`config/registration/{bundle,namespaces}.rs`) and Claudine's embedded
  inventory. User docs:
  [Path-based imports inside a bundle](../../../simplified-schema/docs/topics/language/schema-documents.md#path-based-imports-inside-a-bundle),
  [Resolving inside a bundle](../../../simplified-schema/docs/topics/selecting-schema-files.md#resolving-inside-a-bundle).
  Tests: `bundle_resolution.rs`.
