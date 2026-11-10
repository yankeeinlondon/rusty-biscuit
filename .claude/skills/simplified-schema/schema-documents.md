# Schema documents and envelopes: code map

Loaded from the `simplified-schema` skill. How a referenced `$schema` file
is classified, how envelopes are admitted, and where the line between this
crate and Darkmatter runs.

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
