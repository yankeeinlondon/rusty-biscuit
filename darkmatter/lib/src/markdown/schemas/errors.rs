//! Error types for the schemas subsystem.
//!
//! Phase 1 introduced [`SchemaError::Grammar`] for parser/lexer failures.
//! Phase 2 added [`SchemaError::Convert`] for failures that surface while
//! lowering a `SimplifiedSchema` to JSON Schema. Phase 3 adds the runtime
//! variants used by the resolution and validation pipeline:
//!
//! - [`SchemaError::Unresolved`] — a `$schema` file reference could not be
//!   resolved (invalid syntax or a resolver error).
//! - [`SchemaError::NoMatch`] — a `$schema` file reference matched no file.
//! - [`SchemaError::AmbiguousReferenced`] — a referenced file is neither a
//!   SimplifiedSchema (missing root `$schema:` key) nor a valid JSON Schema.
//! - [`SchemaError::RemoteUnsupported`] — a `$schema` value is an `http(s)://`
//!   URL; remote schemas are not supported in v1.
//! - [`SchemaError::Baseline`] — the baseline schema could not be loaded or
//!   violates the "simple object schema" restriction.
//! - [`SchemaError::BuildValidator`] — `jsonschema` rejected the lowered JSON
//!   Schema at validator-construction time.
//! - [`SchemaError::Io`] — filesystem I/O failed while reading a referenced
//!   file.
//! - [`SchemaError::FrontmatterShape`] — `$schema` is present in frontmatter
//!   but is not a YAML mapping/sequence/string.

use std::{ops::Range, path::PathBuf};

use thiserror::Error;

use super::simplified::{
    SchemaConstraintConflictKind, SchemaConstraintOrigin, SchemaReferenceSite, SchemaSourceId,
    SchemaUnionRejection,
};


/// Errors produced by the schemas subsystem.
#[derive(Debug, Error)]
pub enum SchemaError {
    /// Multiple independent property definitions failed conversion.
    ///
    /// Children retain their typed property and source information. Root
    /// structural failures remain standalone variants because traversal
    /// cannot continue meaningfully after them.
    #[error("multiple SimplifiedSchema properties are invalid ({})", errors.len())]
    Aggregate { errors: Vec<SchemaError> },

    /// A typed schema-definition failure originating in a referenced file.
    ///
    /// The wrapper preserves both the source path and the original error tree
    /// so editor clients can retain per-property diagnostics while pointing
    /// related information at the file that authored those properties.
    #[error("referenced schema `{}` is invalid: {source}", path.display())]
    ReferencedSchema {
        path: PathBuf,
        #[source]
        source: Box<SchemaError>,
    },

    /// A SimplifiedSchema failed to parse.
    ///
    /// `property` identifies the offending property (or a synthetic name like
    /// `"<root>"` / `"<arm[N]>"` when the failure is structural).
    /// `message` is a short description; `span` is the byte range within the
    /// originating type-and-constraint string.
    #[error("invalid SimplifiedSchema for property `{property}`: {message}")]
    Grammar {
        property: String,
        message: String,
        span: Range<usize>,
    },

    /// A parsed SimplifiedSchema failed to convert to JSON Schema.
    ///
    /// Raised for constraint/type mismatches (e.g. `integer` on a `string`
    /// atom), conflicting hoisted defaults across union arms, and root-union
    /// arms that are still `FileRef` at conversion time (resolution is a
    /// Phase 3 concern).
    #[error("cannot convert SimplifiedSchema for `{property}` to JSON Schema: {message}")]
    Convert { property: String, message: String },

    /// A `$schema` file reference could not be resolved: the syntax is
    /// invalid, or biscuit-file rejected it. A reference that resolves but
    /// matches no file is [`SchemaError::NoMatch`].
    #[error("could not resolve $schema reference `{reference}`")]
    Unresolved {
        reference: String,
        #[source]
        source: biscuit_file::FileReferenceError,
    },

    /// A `$schema` file reference matched no file
    /// ([`ResolutionFailure::NoMatch`](biscuit_file::ResolutionFailure::NoMatch)).
    /// `bare_name` is set for a bare schema name that no schema root holds.
    /// The message carries the literal-glob hint
    /// ([`ResolutionFailure::glob_hint`](biscuit_file::ResolutionFailure::glob_hint)).
    #[error("{}", crate::markdown::errors::with_glob_hint(format!("could not resolve $schema reference `{reference}`"), biscuit_file::ResolutionFailure::NoMatch, reference))]
    NoMatch { reference: String, bare_name: bool },

    /// A referenced schema file is neither a recognized standalone
    /// SimplifiedSchema envelope nor a recognizable JSON Schema.
    #[error(
        "referenced schema file `{path}` is neither a valid standalone SimplifiedSchema nor a \
         valid JSON Schema"
    )]
    AmbiguousReferenced { path: PathBuf },

    /// A recognized standalone SimplifiedSchema envelope is malformed.
    #[error("invalid standalone schema document `{path}`: {message}")]
    SchemaDocument { path: PathBuf, message: String },

    /// A `$schema` value is a remote URL; v1 only supports local references.
    #[error("remote $schema references are not supported in this version: `{reference}`")]
    RemoteUnsupported { reference: String },

    /// The baseline schema could not be loaded, parsed, or violates the
    /// "simple object schema" restriction documented in the spec.
    ///
    /// When the failure originated in another `SchemaError` (for example a
    /// parse or conversion error while loading the baseline from disk), the
    /// underlying error is preserved on `source` so programmatic consumers
    /// (e.g. `--format json` rendering) can inspect the chain. Structural
    /// violations of the "simple object schema" restriction have no
    /// upstream cause and leave `source` as `None`.
    #[error("baseline schema is invalid: {message}")]
    Baseline {
        message: String,
        #[source]
        source: Option<Box<SchemaError>>,
    },

    /// `jsonschema` rejected a converted schema at validator-construction
    /// time. The detailed message is preserved on the variant because
    /// `jsonschema::ValidationError` is not `'static` and is awkward to
    /// retain as a `#[source]`.
    #[error("could not build JSON Schema validator: {message}")]
    BuildValidator { message: String },

    /// I/O failed while reading a referenced or baseline schema file, or
    /// while inspecting or enumerating a schema root folder.
    #[error("io error reading `{path}`")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// `$schema` is present but its YAML/JSON shape is unsupported (not a
    /// mapping, sequence, or string).
    #[error("$schema frontmatter value has an unsupported shape: {message}")]
    FrontmatterShape { message: String },

    /// A cross-file named-type import (`Name@file`) forms a direct or
    /// transitive cycle. Named types must form a DAG in v1 — a type that
    /// (transitively) references itself is a structural recursion error,
    /// mirroring the inline-object depth cap. `chain` describes the offending
    /// import path.
    #[error("named-type import cycle detected: {chain}")]
    ImportCycle { chain: String },

    /// A named-type reference (bare `Name` or explicit `Name@file`) names a
    /// type its namespace does not define.
    ///
    /// `namespace` is the schema whose named types were searched: the
    /// defining file for a reference written inside a named-type file, else
    /// the inline `$schema`. `site` addresses the authored reference so an
    /// editor can range the diagnostic on the name.
    #[error("{}: unknown named type `{name}` in {namespace}", site.path)]
    UnknownNamedType {
        name: String,
        namespace: SchemaSourceId,
        site: SchemaReferenceSite,
    },

    /// Constraints merged onto one reused named type contradict each other.
    ///
    /// `site` is the consuming reference; `constraints` lists every authored
    /// constraint taking part, in merge order (inherited first), each with
    /// its own source and span so the conflict can point at all of them,
    /// including declarations reached through multi-hop references.
    #[error(
        "{}: contradictory {kind} on named type `{name}`: {}",
        site.path,
        constraints
            .iter()
            .map(|origin| format!("`{}` in {}", origin.keyword, origin.source))
            .collect::<Vec<_>>()
            .join(" and ")
    )]
    ConstraintConflict {
        name: String,
        kind: SchemaConstraintConflictKind,
        site: SchemaReferenceSite,
        constraints: Vec<SchemaConstraintOrigin>,
    },

    /// A use-site postfix asks for an operation a union-typed named type
    /// cannot take: a value constraint without `[]`, a constraint that is not
    /// valid on an array, `[]` over an arm that is already an array, or `[]`
    /// over an arm whose default is not an array.
    ///
    /// `site` is the consuming reference (its path names the property);
    /// `definition` is the union's declaration in the namespace that defines
    /// it. The two boxed fields keep [`SchemaError`] small.
    #[error("{}: {rejection} (named type `{name}`)", site.path)]
    UnionReference {
        name: String,
        rejection: Box<SchemaUnionRejection>,
        site: SchemaReferenceSite,
        definition: Box<SchemaReferenceSite>,
    },

    /// A `$schema` file-reference chain revisits a file it is already
    /// resolving. A schema file whose whole payload is a reference
    /// (`$schema: ./other.yaml`) — and each root-union file arm — re-enters
    /// reference resolution, so a self- or mutually-referencing pair would
    /// otherwise recurse until the process stack overflows. `chain` lists the
    /// canonical paths in visit order.
    #[error("$schema file-reference cycle detected: {chain}")]
    ReferenceCycle { chain: String },

    /// A `$schema` file-reference chain exceeded the delegation depth cap
    /// while every file in it was distinct. Deliberately distinct from
    /// [`SchemaError::ReferenceCycle`]: an acyclic chain has no loop to break,
    /// so reporting it as a cycle would send the author looking for a
    /// self-reference that does not exist. `limit` is the cap that was
    /// reached; `chain` lists the canonical paths in visit order, ending at
    /// the hop that would have exceeded it.
    #[error("$schema file-reference chain exceeded the {limit}-file depth limit: {chain}")]
    ReferenceDepthExceeded { limit: usize, chain: String },

    /// An `example(...)` constraint (Feature A) referenced a file that is
    /// missing, malformed, or does not validate against the example-artifact
    /// envelope (or its inherited `parameters` shape). Example files are
    /// documentation the schema promises are trustworthy, so a broken example
    /// is a fail-loud schema-load error, never a warning. `reference` is the
    /// authored `example(...)` reference; `message` is the underlying reason.
    #[error("invalid example artifact `{reference}`: {message}")]
    InvalidExample { reference: String, message: String },

    /// A schema-trigger envelope or match expression is malformed (a field of
    /// the wrong shape, an unknown key, or an otherwise unparseable `match:`
    /// payload). `message` starts with the field it is about (for example
    /// `` `match[1].group[0]`: ``) and describes the failure; the trigger
    /// grammar is documented in the "Repository Trigger Schemas" section of
    /// `darkmatter/docs/topics/schemas/definition.md`. A `$path` pattern that
    /// is not an allowed glob reference is reported here too.
    #[error("invalid schema-trigger: {message}")]
    TriggerMatch { message: String },

    /// A schema-trigger file uses a spelling that was renamed without an
    /// alias: `kind: trigger-schema` (now `schema-trigger`) or the `any:`
    /// combinator (now `group:`). `field` is where the spelling appears,
    /// `found` the rejected spelling, and `replacement` the one to use.
    #[error("`{field}`: `{found}` was renamed; use `{replacement}`")]
    TriggerRenamed {
        field: String,
        found: String,
        replacement: String,
    },

    /// A file claims `kind: schema-trigger` but is not valid YAML, so it never
    /// reached the match grammar. `message` is the YAML parser's reason without
    /// its location; `line` and `column` are one-based (the column counts
    /// characters, not bytes) and present when the parser reported them.
    #[error("schema-trigger envelope is not valid YAML{}: {message}", yaml_location(*.line, *.column))]
    TriggerEnvelopeYaml {
        message: String,
        line: Option<usize>,
        column: Option<usize>,
    },

    /// A schema-trigger match condition used a constraint outside the
    /// match-safe subset (constraints that resolve files, import types, load
    /// examples, provide defaults, mark generated values, or eager-existence-
    /// check `file`). `property` names the offending property; `constraint`
    /// names the forbidden constraint keyword.
    #[error(
        "schema-trigger match condition for `{property}` uses a forbidden constraint: {constraint}"
    )]
    TriggerForbiddenConstraint { property: String, constraint: String },

    /// A schema-trigger match mapping mixes structural keys (combinators
    /// `all`/`group`/`none`/`min-match` or the `$path` predicate) with
    /// property-condition keys. Mixing is a load error — never a guess about
    /// the author's intent. `structural` and `properties` list the offending
    /// keys.
    #[error(
        "schema-trigger match mapping mixes structural keys ({structural:?}) with property keys \
         ({properties:?}); use `all:` to combine them"
    )]
    TriggerMixedMapping {
        structural: Vec<String>,
        properties: Vec<String>,
    },

    /// A schema-trigger's whole match expression is vacuous: some way of
    /// satisfying it includes no presence-requiring condition (a `required`
    /// gate or `$path` predicate inside `all`/`group`/`min-match`, not under
    /// `none`), so the trigger can match a document that has none of its
    /// keys — almost always an authoring mistake.
    #[error("schema-trigger match is vacuous (no presence-requiring condition)")]
    TriggerVacuousMatch,

    /// Two trigger filenames in the same `schemas/` root collide after
    /// case-folding (e.g. `Foo.yaml` and `foo.yaml`), which would activate
    /// differently on case-sensitive vs case-insensitive filesystems. `root`
    /// is the offending schema root directory; `files` lists the colliding
    /// filenames.
    #[error(
        "trigger schema files in `{}` collide after case-folding: {files:?}",
        root.display()
    )]
    TriggerCaseFoldCollision {
        root: PathBuf,
        files: Vec<String>,
    },

    /// A schema-trigger envelope file claimed `kind: schema-trigger` but could
    /// not be parsed. The underlying YAML, grammar, or envelope error is on
    /// `source`; `path` is the offending file.
    #[error("schema-trigger file `{}` {}", path.display(), load_failure(source))]
    TriggerLoad {
        path: PathBuf,
        #[source]
        source: Box<SchemaError>,
    },

    /// A trigger in a host's [`EmbeddedSchemaBundle`](super::EmbeddedSchemaBundle)
    /// claimed `kind: schema-trigger` but could not be loaded: a malformed
    /// envelope, or a payload that failed to resolve inside its bundle.
    /// `bundle` and `name` identify the embedded document; it has no host
    /// path, so this is never a [`SchemaError::TriggerLoad`].
    #[error("embedded schema-trigger `{bundle}/{name}` {}", load_failure(source))]
    EmbeddedTriggerLoad {
        bundle: String,
        name: String,
        #[source]
        source: Box<SchemaError>,
    },

    /// A host's [`EmbeddedSchemaBundle`](super::EmbeddedSchemaBundle) has an
    /// unusable name or file list (see
    /// [`EmbeddedSchemaBundle::new`](super::EmbeddedSchemaBundle::new)).
    #[error("embedded schema bundle `{bundle}` is invalid: {message}")]
    EmbeddedBundle { bundle: String, message: String },

    /// A bare-name schema reference (no path component) was not found in any
    /// schema root, but a file of that name exists in the document's own
    /// directory. The author likely intended a relative reference; the error
    /// suggests `./‹name›` so the migration is pointed rather than opaque.
    /// `reference` is the authored bare name; `suggestion` is the `./`-prefixed
    /// form that would resolve to the sibling file.
    #[error(
        "bare-name schema reference `{reference}` was not found in any schema root; \
         a document-sibling file exists — use `{suggestion}` to reference it"
    )]
    BareNameSiblingExists { reference: String, suggestion: String },

    /// A schema-trigger payload did not resolve to a merge-compatible object
    /// schema. Trigger payloads must be SimplifiedSchema single objects or
    /// raw JSON Schemas satisfying the simple-object baseline contract; root
    /// unions and non-object schemas are rejected because the property-keyed
    /// merge has no sound "later wins" meaning for them. `path` is the
    /// trigger envelope file; `payload` is the authored `$schema:` reference
    /// (or `<inline>` for an inline payload); `reason` explains the rejection.
    #[error(
        "schema-trigger payload in `{}` is not merge-compatible: {reason}",
        path.display()
    )]
    TriggerPayloadNotMergeable {
        path: PathBuf,
        payload: String,
        reason: String,
    },

    /// A schema-trigger payload resolved to a trigger envelope file (directly
    /// or through an import cycle), rather than to an ordinary schema file.
    /// `trigger` is the originating envelope; `payload_path` is the file the
    /// payload resolved to; `chain` describes the resolution path for the
    /// indirect case.
    #[error(
        "schema-trigger payload in `{}` resolved to a schema-trigger envelope `{}`",
        trigger.display(),
        payload_path.display()
    )]
    TriggerPayloadCycle {
        trigger: PathBuf,
        payload_path: PathBuf,
        chain: String,
    },

    /// A document `$schema` directly referenced a `*.trigger.yaml` file.
    /// Trigger schemas activate by placement and match, never by reference;
    /// the error names the payload (`suggestion`) as the file to reference
    /// instead.
    #[error(
        "document `$schema` referenced a schema-trigger file `{reference}`; trigger schemas \
         activate by placement and match — reference the payload `{suggestion}` instead"
    )]
    TriggerSchemaReferenced {
        reference: String,
        suggestion: String,
    },
}

/// The ` at line L, column C` suffix of a YAML failure, empty without a
/// complete location.
fn yaml_location(line: Option<usize>, column: Option<usize>) -> String {
    match (line, column) {
        (Some(line), Some(column)) => format!(" at line {line}, column {column}"),
        (Some(line), None) => format!(" at line {line}"),
        _ => String::new(),
    }
}

/// The predicate of a trigger load failure: a YAML failure says so directly
/// rather than calling the file "malformed" with a nested envelope message.
fn load_failure(source: &SchemaError) -> String {
    match source {
        SchemaError::TriggerEnvelopeYaml { message, line, column } => {
            format!("is not valid YAML{}: {message}", yaml_location(*line, *column))
        }
        other => format!("is malformed: {other}"),
    }
}

/// Hint for a claimed trigger envelope that is not valid YAML.
const TRIGGER_YAML_HINT: &str = "Fix the YAML syntax at the reported position. A plain value \
     that contains <cyan>: </cyan> or <cyan> #</cyan>, or starts with an indicator such as \
     <cyan>@</cyan> or <cyan>*</cyan>, must be quoted, for example \
     <cyan>title: \"Note: quoted\"</cyan>.";

impl SchemaError {
    /// The hint a wrapping error (a compose-time schema preparation failure)
    /// shows in place of its generic one: today only a claimed trigger
    /// envelope that is not valid YAML, directly or inside a trigger load.
    pub(crate) fn preparation_hint(&self) -> Option<&'static str> {
        match self {
            SchemaError::TriggerEnvelopeYaml { .. } => Some(TRIGGER_YAML_HINT),
            SchemaError::TriggerLoad { source, .. } | SchemaError::EmbeddedTriggerLoad { source, .. } => {
                source.preparation_hint()
            }
            _ => None,
        }
    }
}

impl biscuit_terminal::errors::BlockError for SchemaError {
    fn status_block(
        &self,
        _term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::prose::Prose;
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            SchemaError::Aggregate { errors } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "multiple schema properties are invalid",
                ))
                .body(
                    errors
                        .iter()
                        .map(|error| Prose::new(Prose::escape_text_outside_code_spans(&error.to_string())))
                        .collect::<Vec<_>>(),
                )
                .hint("Fix each listed property definition, then validate the schema again."),
            SchemaError::ReferencedSchema { path, source } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "SchemaError",
                        "referenced schema is invalid",
                    ))
                    .body(vec![
                        Prose::new(format!(
                            "The schema at <cyan>{}</cyan> contains an invalid definition.",
                            Prose::escape_text(&path.display().to_string())
                        )),
                        Prose::new(format!(
                            "<dim>Cause:</dim> {}",
                            Prose::escape_text_outside_code_spans(&source.to_string())
                        )),
                    ])
                    .hint("Fix the referenced schema definition, then validate again.")
            }
            SchemaError::Grammar {
                property,
                message,
                span,
            } => {
                let body = vec![
                    Prose::new(format!(
                        "SimplifiedSchema parsing failed for <cyan>{}</cyan>:",
                        Prose::escape_text(property)
                    )),
                    Prose::new(format!(
                        "<dim>Span:</dim> bytes {}..{}",
                        span.start, span.end
                    )),
                    Prose::new(format!(
                        "<dim>Message:</dim> {}",
                        Prose::escape_text_outside_code_spans(message)
                    )),
                ];
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("SchemaError", "grammar parse failed"))
                    .body(body)
                    .hint(
                        "Check the type-and-constraint string. Shapes: <cyan>{type}</cyan>, \
                         <cyan>{type}(constraint;constraint)</cyan>, \
                         <cyan>{type} -> description</cyan>",
                    )
            }

            SchemaError::Convert { property, message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "JSON Schema conversion failed",
                ))
                .body(vec![
                    Prose::new(format!(
                        "Could not convert SimplifiedSchema for <cyan>{}</cyan>:",
                        Prose::escape_text(property)
                    )),
                    Prose::new(format!(
                        "<dim>Reason:</dim> {}",
                        Prose::escape_text_outside_code_spans(message)
                    )),
                ])
                .hint(
                    "Verify constraints are compatible with the declared type (e.g. <cyan>min</cyan> \
                     on <cyan>number</cyan>, <cyan>pattern</cyan> on <cyan>string</cyan>).",
                ),

            SchemaError::Unresolved { reference, source } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "$schema unresolved"))
                .body(vec![
                    Prose::new(format!(
                        "Could not resolve <cyan>$schema: {}</cyan>",
                        Prose::escape_text(reference)
                    )),
                    Prose::new(format!(
                        "<dim>Cause:</dim> {}",
                        Prose::escape_text_outside_code_spans(&source.to_string())
                    )),
                ])
                .hint(
                    "Confirm the file exists, uses a supported reference prefix (e.g. <cyan>./</cyan>, \
                     <cyan>~/</cyan>, <cyan>@/</cyan>), and is readable.",
                ),

            SchemaError::NoMatch { reference, bare_name } => {
                let cause = if *bare_name {
                    format!("no schema root holds `{reference}`")
                } else {
                    format!("no file matched `{reference}`")
                };
                let failure = biscuit_file::ResolutionFailure::NoMatch;
                let mut body = vec![
                    Prose::new(format!(
                        "Could not resolve <cyan>$schema: {}</cyan>",
                        Prose::escape_text(reference)
                    )),
                    Prose::new(format!("<dim>Cause:</dim> {}", Prose::escape_text_outside_code_spans(&cause))),
                ];
                if let Some(hint) = failure.glob_hint(reference) {
                    body.push(Prose::new(format!("<dim>hint:</dim> {hint}")));
                }
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("SchemaError", "$schema unresolved"))
                    .body(body)
                    .hint(
                        "Confirm the file exists, uses a supported reference prefix (e.g. <cyan>./</cyan>, \
                         <cyan>~/</cyan>, <cyan>@/</cyan>), and is readable.",
                    )
            }

            SchemaError::AmbiguousReferenced { path } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "ambiguous schema reference",
                ))
                .body(Prose::new(format!(
                    "<cyan>{}</cyan> is neither a recognized standalone SimplifiedSchema nor a \
                     valid JSON Schema.",
                    Prose::escape_text(&path.display().to_string())
                )))
                .hint(
                    "Either prefix a SimplifiedSchema mapping with <cyan>$schema:</cyan>, or \
                     supply a Draft 2020-12 JSON Schema.",
                ),

            SchemaError::SchemaDocument { path, message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "standalone schema document invalid",
                ))
                .body(vec![
                    Prose::new(format!(
                        "Could not parse <cyan>{}</cyan> as a standalone SimplifiedSchema.",
                        Prose::escape_text(&path.display().to_string())
                    )),
                    Prose::new(format!(
                        "<dim>Reason:</dim> {}",
                        Prose::escape_text_outside_code_spans(message)
                    )),
                ])
                .hint(
                    "Use either a sole top-level <cyan>$schema:</cyan> mapping/sequence or exactly \
                     <cyan>kind: schema</cyan> with a <cyan>types:</cyan> mapping.",
                ),

            SchemaError::RemoteUnsupported { reference } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "remote $schema not supported",
                ))
                .body(Prose::new(format!(
                    "Remote schemas are not supported in v1: <cyan>{}</cyan>",
                    Prose::escape_text(reference)
                )))
                .hint("Download the schema locally and reference it with a relative or absolute path."),

            SchemaError::Baseline { message, source } => {
                let mut body = vec![Prose::new(format!(
                    "<dim>Reason:</dim> {}",
                    Prose::escape_text_outside_code_spans(message)
                ))];
                if let Some(cause) = source {
                    body.push(Prose::new(format!(
                        "<dim>Cause:</dim> {}",
                        Prose::escape_text_outside_code_spans(&cause.to_string())
                    )));
                }
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("SchemaError", "baseline schema invalid"))
                    .body(body)
                    .hint(
                        "A baseline schema must be a simple object schema (root <cyan>type: object</cyan> \
                         with only <cyan>properties</cyan> / <cyan>required</cyan>).",
                    )
            }

            SchemaError::BuildValidator { message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "validator construction failed",
                ))
                .body(Prose::new(format!(
                    "<dim>jsonschema:</dim> {}",
                    Prose::escape_text_outside_code_spans(message)
                )))
                .hint(
                    "The lowered JSON Schema was rejected at compile time. Re-run with \
                     <cyan>RUST_LOG=darkmatter=debug</cyan> for more detail.",
                ),

            SchemaError::Io { path, source } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "I/O failure"))
                .body(vec![
                    Prose::new(format!(
                        "<dim>Path:</dim> <cyan>{}</cyan>",
                        Prose::escape_text(&path.display().to_string())
                    )),
                    Prose::new(format!(
                        "<dim>Cause:</dim> {} <dim>({:?})</dim>",
                        Prose::escape_text_outside_code_spans(&source.to_string()),
                        source.kind()
                    )),
                ])
                .hint("Check the path exists and the process has permission to read it."),

            SchemaError::FrontmatterShape { message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "$schema frontmatter shape unsupported",
                ))
                .body(Prose::new(format!(
                    "<dim>Reason:</dim> {}",
                    Prose::escape_text_outside_code_spans(message)
                )))
                .hint(
                    "<cyan>$schema</cyan> must be a YAML mapping (inline schema), a string (file \
                     reference), or a sequence (root union).",
                ),

            SchemaError::ImportCycle { chain } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "named-type import cycle",
                ))
                .body(Prose::new(format!(
                    "<dim>Cycle:</dim> {}",
                    Prose::escape_text(chain)
                )))
                .hint(
                    "Named-type imports (<cyan>Name@file</cyan>) must form a DAG. Break the \
                     self-reference; recursive types are not supported in v1.",
                ),

            SchemaError::UnknownNamedType { name, namespace, site } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("SchemaError", "unknown named type"))
                    .body(Prose::new(format!(
                        "<cyan>{}</cyan> references <cyan>{}</cyan>, which {} does not define.",
                        Prose::escape_text(&site.path.to_string()),
                        Prose::escape_text(name),
                        Prose::escape_text_outside_code_spans(&namespace.to_string())
                    )))
                    .hint(
                        "A bare name resolves only in the schema that writes it; use \
                         <cyan>Name@./file.yaml</cyan> for a type from another file.",
                    )
            }

            SchemaError::ConstraintConflict { name, kind, site, constraints } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("SchemaError", "contradictory constraints"))
                    .body(
                        std::iter::once(Prose::new(format!(
                            "<cyan>{}</cyan> reuses <cyan>{}</cyan> with contradictory {kind}:",
                            Prose::escape_text(&site.path.to_string()),
                            Prose::escape_text(name)
                        )))
                        .chain(constraints.iter().map(|origin| {
                            Prose::new(format!(
                                "<cyan>{}</cyan> at {} in {}",
                                Prose::escape_text(&origin.keyword),
                                Prose::escape_text(&origin.path.to_string()),
                                Prose::escape_text_outside_code_spans(&origin.source.to_string())
                            ))
                        }))
                        .collect::<Vec<_>>(),
                    )
                    .hint("Change the use-site constraint so it agrees with the named type.")
            }

            SchemaError::UnionReference { name, rejection, site, definition } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "SchemaError",
                        "unsupported operation on a union type",
                    ))
                    .body(vec![
                        Prose::new(format!(
                            "<cyan>{}</cyan> references the union <cyan>{}</cyan>: {}.",
                            Prose::escape_text(&site.path.to_string()),
                            Prose::escape_text(name),
                            Prose::escape_text_outside_code_spans(&rejection.to_string())
                        )),
                        Prose::new(format!(
                            "<dim>Defined at:</dim> {} in {}",
                            Prose::escape_text(&definition.path.to_string()),
                            Prose::escape_text_outside_code_spans(&definition.source.to_string())
                        )),
                    ])
                    .hint(
                        "A union reference accepts <cyan>required</cyan>, <cyan>generated</cyan>, \
                         <cyan>eager</cyan>, a description, and <cyan>[]</cyan> with array \
                         constraints. Put value constraints on the union's arms.",
                    )
            }

            SchemaError::ReferenceCycle { chain } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "$schema reference cycle"))
                .body(Prose::new(format!(
                    "<dim>Chain:</dim> {}",
                    Prose::escape_text(chain)
                )))
                .hint(
                    "A schema file that only declares <cyan>$schema: ./other.yaml</cyan> delegates \
                     to that file. Break the loop so the chain terminates at a file that declares \
                     an actual schema.",
                ),

            SchemaError::ReferenceDepthExceeded { limit, chain } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "SchemaError",
                        "$schema reference chain too deep",
                    ))
                    .body(vec![
                        Prose::new(format!("<dim>Limit:</dim> {limit} files")),
                        Prose::new(format!(
                            "<dim>Chain:</dim> {}",
                            Prose::escape_text(chain)
                        )),
                    ])
                    .hint(
                        "Every file in this chain is distinct, so there is no loop to break. \
                         Flatten the delegation: point <cyan>$schema</cyan> at the file that \
                         declares the actual schema instead of chaining through intermediate \
                         redirects.",
                    )
            }

            SchemaError::InvalidExample { reference, message } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("SchemaError", "invalid example artifact"))
                    .body(vec![
                        Prose::new(format!(
                            "<cyan>example({})</cyan> does not validate:",
                            Prose::escape_text(reference)
                        )),
                        Prose::new(format!(
                            "<dim>Reason:</dim> {}",
                            Prose::escape_text_outside_code_spans(message)
                        )),
                    ])
                    .hint(
                        "Each <cyan>example(...)</cyan> file must exist and validate against the \
                         example-artifact envelope (<cyan>kind</cyan>, <cyan>invocation</cyan>, \
                         <cyan>returns</cyan>, <cyan>description</cyan>).",
                    )
            }

            SchemaError::TriggerMatch { message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "schema-trigger invalid",
                ))
                .body(Prose::new(format!(
                    "<dim>Reason:</dim> {}",
                    Prose::escape_text_outside_code_spans(message)
                )))
                .hint(
                    "See the trigger grammar: combinators (<cyan>all</cyan>/<cyan>group</cyan>/\
                     <cyan>none</cyan>/<cyan>min-match</cyan>), property conditions, and the \
                     <cyan>$path</cyan> predicate.",
                ),

            SchemaError::TriggerRenamed { field, found, replacement } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "schema-trigger spelling renamed"))
                .body(Prose::new(format!(
                    "<cyan>{}</cyan> uses <cyan>{}</cyan>, which was renamed with no alias.",
                    Prose::escape_text(field),
                    Prose::escape_text(found)
                )))
                .hint(format!("Write <cyan>{}</cyan> instead.", Prose::escape_text(replacement))),

            SchemaError::TriggerEnvelopeYaml { message, line, column } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "SchemaError",
                        "schema-trigger envelope is not valid YAML",
                    ))
                    .body(Prose::new(format!(
                        "<dim>Reason:</dim> {}{}",
                        Prose::escape_text_outside_code_spans(message),
                        Prose::escape_text(&yaml_location(*line, *column))
                    )))
                    .hint(TRIGGER_YAML_HINT)
            }

            SchemaError::TriggerForbiddenConstraint { property, constraint } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "SchemaError",
                        "schema-trigger forbidden constraint",
                    ))
                    .body(vec![
                        Prose::new(format!(
                            "Property <cyan>{}</cyan> uses a forbidden constraint in a match \
                             condition: <cyan>{}</cyan>",
                            Prose::escape_text(property),
                            Prose::escape_text(constraint)
                        )),
                    ])
                    .hint(
                        "Match conditions allow only structural types and pure constraints: \
                         <cyan>required</cyan>, <cyan>enum</cyan>, <cyan>pattern</cyan>, \
                         length/range, item-count, key-count.",
                    )
            }

            SchemaError::TriggerMixedMapping {
                structural,
                properties,
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "schema-trigger mixed mapping",
                ))
                .body(Prose::new(format!(
                    "A match mapping cannot mix structural keys ({:?}) with property keys \
                     ({:?}).",
                    structural, properties
                )))
                .hint(
                    "Wrap the conditions in an <cyan>all:</cyan> combinator to combine a \
                     structural key with property conditions.",
                ),

            SchemaError::TriggerVacuousMatch => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "schema-trigger vacuous match"))
                .body(Prose::new(
                    "A way of satisfying the match has no presence-requiring condition (a <cyan>required</cyan> \
                     gate or <cyan>$path</cyan> predicate), so it matches every document.",
                ))
                .hint(
                    "Add a <cyan>required</cyan> constraint or a <cyan>$path</cyan> predicate so \
                     the trigger only fires on matching documents.",
                ),

            SchemaError::TriggerCaseFoldCollision { root, files } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "SchemaError",
                        "schema-trigger case-fold collision",
                    ))
                    .body(vec![
                        Prose::new(format!(
                            "Two files in <cyan>{}</cyan> collide after case-folding:",
                            Prose::escape_text(&root.display().to_string())
                        )),
                        Prose::new(format!(
                            "<dim>Files:</dim> {}",
                            files
                                .iter()
                                .map(|f| format!("{f:?}"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )),
                    ])
                    .hint(
                        "Rename one of the colliding files so they differ by more than case, \
                         preventing divergent behavior on case-sensitive vs case-insensitive \
                         filesystems.",
                    )
            }

            SchemaError::TriggerLoad { path, source } => {
                let yaml = matches!(**source, SchemaError::TriggerEnvelopeYaml { .. });
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "SchemaError",
                        if yaml { "schema-trigger file is not valid YAML" } else { "schema-trigger file malformed" },
                    ))
                    .body(vec![
                        Prose::new(format!(
                            "Trigger file <cyan>{}</cyan> could not be parsed:",
                            Prose::escape_text(&path.display().to_string())
                        )),
                        Prose::new(format!(
                            "<dim>Reason:</dim> {}",
                            Prose::escape_text_outside_code_spans(&source.to_string())
                        )),
                    ])
                    .hint(if yaml {
                        TRIGGER_YAML_HINT
                    } else {
                        "Fix the match grammar or envelope shape. See the trigger grammar: \
                         combinators (<cyan>all</cyan>/<cyan>any</cyan>/<cyan>none</cyan>/\
                         <cyan>min-match</cyan>), property conditions, and the \
                         <cyan>$path</cyan> predicate."
                    })
            }

            SchemaError::EmbeddedTriggerLoad { bundle, name, source } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("SchemaError", "embedded schema-trigger malformed"))
                    .body(vec![
                        Prose::new(format!(
                            "Embedded trigger <cyan>{}</cyan> could not be loaded:",
                            Prose::escape_text(&format!("{bundle}/{name}"))
                        )),
                        Prose::new(format!(
                            "<dim>Reason:</dim> {}",
                            Prose::escape_text_outside_code_spans(&source.to_string())
                        )),
                    ])
                    .hint(
                        "The trigger ships inside the host binary; a schema file of the same \
                         name in a schema root replaces it.",
                    )
            }

            SchemaError::EmbeddedBundle { bundle, message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "embedded schema bundle invalid"))
                .body(vec![Prose::new(format!(
                    "Embedded schema bundle <cyan>{}</cyan>: {}",
                    Prose::escape_text(bundle),
                    Prose::escape_text_outside_code_spans(message)
                ))]),

            SchemaError::BareNameSiblingExists {
                reference,
                suggestion,
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "bare-name reference not in schema roots",
                ))
                .body(vec![
                    Prose::new(format!(
                        "Bare-name reference <cyan>{}</cyan> was not found in any schema root.",
                        Prose::escape_text(reference)
                    )),
                    Prose::new(format!(
                        "A document-sibling file of that name exists. Use \
                         <cyan>{}</cyan> to reference it.",
                        Prose::escape_text(suggestion)
                    )),
                ])
                .hint(
                    "Prefix bare names with <cyan>./</cyan> to reference a file relative to the \
                     document's directory, or place the schema in a discovered <cyan>schemas/</cyan> \
                     root to use bare-name resolution.",
                ),

            SchemaError::TriggerPayloadNotMergeable {
                path,
                payload,
                reason,
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "schema-trigger payload not merge-compatible",
                ))
                .body(vec![
                    Prose::new(format!(
                        "Trigger file <cyan>{}</cyan> payload <cyan>{}</cyan> is not a \
                         merge-compatible object schema:",
                        Prose::escape_text(&path.display().to_string()),
                        Prose::escape_text(payload)
                    )),
                    Prose::new(format!(
                        "<dim>Reason:</dim> {}",
                        Prose::escape_text_outside_code_spans(reason)
                    )),
                ])
                .hint(
                    "A trigger payload must be a SimplifiedSchema single object or a raw JSON \
                     Schema satisfying the simple-object baseline contract. Root unions and \
                     non-object schemas are rejected as trigger payloads.",
                ),

            SchemaError::TriggerPayloadCycle {
                trigger,
                payload_path,
                chain,
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "schema-trigger payload cycle",
                ))
                .body(vec![
                    Prose::new(format!(
                        "Trigger file <cyan>{}</cyan> payload resolved to a schema-trigger \
                         envelope <cyan>{}</cyan>:",
                        Prose::escape_text(&trigger.display().to_string()),
                        Prose::escape_text(&payload_path.display().to_string())
                    )),
                    Prose::new(format!("<dim>Chain:</dim> {}", Prose::escape_text(chain))),
                ])
                .hint(
                    "A trigger payload must not resolve to a schema-trigger envelope, directly or \
                     through an import cycle. Reference an ordinary schema file instead.",
                ),

            SchemaError::TriggerSchemaReferenced {
                reference,
                suggestion,
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "schema-trigger file referenced as document schema",
                ))
                .body(vec![
                    Prose::new(format!(
                        "Document <cyan>$schema</cyan> referenced a schema-trigger file \
                         <cyan>{}</cyan>. Trigger schemas activate by placement and match, never \
                         by reference.",
                        Prose::escape_text(reference)
                    )),
                    Prose::new(format!(
                        "Reference the payload <cyan>{}</cyan> instead.",
                        Prose::escape_text(suggestion)
                    )),
                ])
                .hint(
                    "Use the payload file (without the <cyan>.trigger</cyan> infix) as the \
                     document <cyan>$schema</cyan>. Trigger envelopes live in <cyan>schemas/</cyan> \
                     and activate automatically when their match conditions hold.",
                ),
        }
    }
}

#[cfg(test)]
mod tests {
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::errors::BlockError;
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    use super::*;

    fn render(err: &SchemaError) -> String {
        let term = biscuit_terminal::terminal::Terminal::default();
        strip_escape_codes(err.status_block(&term).render_optimistic(Some(80)))
    }

    #[test]
    fn grammar_display_includes_property_and_message() {
        let err = SchemaError::Grammar {
            property: "title".into(),
            message: "unexpected `(`".into(),
            span: 6..7,
        };
        let rendered = err.to_string();
        assert!(rendered.contains("title"));
        assert!(rendered.contains("unexpected `(`"));
    }

    #[test]
    fn remote_unsupported_display_includes_reference() {
        let err = SchemaError::RemoteUnsupported {
            reference: "https://example.com/schema.json".into(),
        };
        assert!(err.to_string().contains("https://example.com/schema.json"));
    }

    #[test]
    fn ambiguous_referenced_display_includes_path() {
        let err = SchemaError::AmbiguousReferenced {
            path: PathBuf::from("./schemas/post.yaml"),
        };
        assert!(err.to_string().contains("post.yaml"));
    }

    #[test]
    fn grammar_block_renders_property_and_span() {
        let err = SchemaError::Grammar {
            property: "title".into(),
            message: "unexpected `(`".into(),
            span: 6..7,
        };
        let out = render(&err);
        assert!(out.contains("SchemaError"), "missing header type: {out}");
        assert!(
            out.contains("grammar parse failed"),
            "missing summary: {out}",
        );
        assert!(out.contains("title"), "missing property name: {out}");
        assert!(out.contains("unexpected"), "missing detail message: {out}");
    }

    #[test]
    fn convert_block_renders_property_and_reason() {
        let err = SchemaError::Convert {
            property: "age".into(),
            message: "min constraint not valid for string".into(),
        };
        let out = render(&err);
        assert!(out.contains("SchemaError"), "missing header type: {out}");
        assert!(
            out.contains("JSON Schema conversion failed"),
            "missing summary: {out}",
        );
        assert!(out.contains("age"), "missing property: {out}");
        assert!(out.contains("min constraint"), "missing reason: {out}");
    }

    fn site(property: &str) -> SchemaReferenceSite {
        SchemaReferenceSite {
            source: SchemaSourceId::InlineDocument,
            path: crate::markdown::schemas::SchemaSourcePath::root().property(property),
        }
    }

    #[test]
    fn unknown_named_type_names_the_type_the_reference_and_the_namespace() {
        let err = SchemaError::UnknownNamedType {
            name: "postcode".into(),
            namespace: SchemaSourceId::File(PathBuf::from("contacts.yaml")),
            site: site("address"),
        };
        assert_eq!(
            err.to_string(),
            "address: unknown named type `postcode` in `contacts.yaml`"
        );
        let out = render(&err);
        assert!(out.contains("unknown named type"), "missing summary: {out}");
        assert!(out.contains("postcode") && out.contains("contacts.yaml"), "{out}");
    }

    #[test]
    fn constraint_conflict_lists_every_origin_in_merge_order() {
        let origin = |source: SchemaSourceId, path: &str, keyword: &str| SchemaConstraintOrigin {
            source,
            path: crate::markdown::schemas::SchemaSourcePath::root().property(path),
            keyword: keyword.into(),
            occurrence: 0,
            span: None,
        };
        let err = SchemaError::ConstraintConflict {
            name: "title_base".into(),
            kind: SchemaConstraintConflictKind::Bounds,
            site: site("title"),
            constraints: vec![
                origin(SchemaSourceId::File(PathBuf::from("types.yaml")), "label", "min"),
                origin(SchemaSourceId::InlineDocument, "title", "max"),
            ],
        };
        assert_eq!(
            err.to_string(),
            "title: contradictory bounds on named type `title_base`: `min` in `types.yaml` and \
             `max` in the inline `$schema`"
        );
        let out = render(&err);
        assert!(out.contains("contradictory constraints"), "missing summary: {out}");
        assert!(out.contains("label") && out.contains("types.yaml"), "{out}");
    }

    #[test]
    fn union_reference_names_the_rejection_the_reference_and_the_definition() {
        let err = SchemaError::UnionReference {
            name: "policy".into(),
            rejection: Box::new(SchemaUnionRejection::Constraint {
                keyword: "max".into(),
                array_level: false,
            }),
            site: site("content_policy"),
            definition: Box::new(SchemaReferenceSite {
                source: SchemaSourceId::File(PathBuf::from("policy.yaml")),
                path: crate::markdown::schemas::SchemaSourcePath::root().property("policy"),
            }),
        };
        assert_eq!(
            err.to_string(),
            "content_policy: constraint `max` cannot apply to a union-typed type (named type \
             `policy`)"
        );
        let out = render(&err);
        assert!(out.contains("unsupported operation on a union type"), "missing summary: {out}");
        assert!(out.contains("content_policy") && out.contains("policy.yaml"), "{out}");
    }

    #[test]
    fn remote_unsupported_block_renders_url() {
        let err = SchemaError::RemoteUnsupported {
            reference: "https://example.com/schema.json".into(),
        };
        let out = render(&err);
        assert!(out.contains("SchemaError"), "missing header type: {out}");
        assert!(
            out.contains("remote $schema not supported"),
            "missing summary: {out}",
        );
        assert!(
            out.contains("https://example.com/schema.json"),
            "missing url: {out}",
        );
    }

    #[test]
    fn ambiguous_referenced_block_renders_path() {
        let err = SchemaError::AmbiguousReferenced {
            path: PathBuf::from("./schemas/post.yaml"),
        };
        let out = render(&err);
        assert!(out.contains("SchemaError"), "missing header type: {out}");
        assert!(
            out.contains("ambiguous schema reference"),
            "missing summary: {out}",
        );
        assert!(out.contains("post.yaml"), "missing path: {out}");
    }

    #[test]
    fn baseline_block_renders_message() {
        let err = SchemaError::Baseline {
            message: "must be a simple object schema".into(),
            source: None,
        };
        let out = render(&err);
        assert!(out.contains("SchemaError"), "missing header type: {out}");
        assert!(
            out.contains("baseline schema invalid"),
            "missing summary: {out}",
        );
        assert!(
            out.contains("simple object schema"),
            "missing message: {out}",
        );
    }

    #[test]
    fn build_validator_block_renders_message() {
        let err = SchemaError::BuildValidator {
            message: "unknown keyword `xfoo`".into(),
        };
        let out = render(&err);
        assert!(out.contains("SchemaError"), "missing header type: {out}");
        assert!(
            out.contains("validator construction failed"),
            "missing summary: {out}",
        );
        assert!(out.contains("xfoo"), "missing detail: {out}");
    }

    #[test]
    fn io_block_renders_path_and_kind() {
        let err = SchemaError::Io {
            path: PathBuf::from("/missing.yaml"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"),
        };
        let out = render(&err);
        assert!(out.contains("SchemaError"), "missing header type: {out}");
        assert!(out.contains("I/O failure"), "missing summary: {out}");
        assert!(out.contains("/missing.yaml"), "missing path: {out}");
        assert!(out.contains("NotFound"), "missing io kind: {out}");
    }

    #[test]
    fn frontmatter_shape_block_renders_message() {
        let err = SchemaError::FrontmatterShape {
            message: "expected mapping, got integer".into(),
        };
        let out = render(&err);
        assert!(out.contains("SchemaError"), "missing header type: {out}");
        assert!(
            out.contains("frontmatter shape unsupported"),
            "missing summary: {out}",
        );
        assert!(out.contains("expected mapping"), "missing detail: {out}",);
    }
}

