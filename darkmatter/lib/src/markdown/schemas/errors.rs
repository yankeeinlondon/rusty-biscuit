//! Error types for the schemas subsystem.
//!
//! Phase 1 introduced [`SchemaError::Grammar`] for parser/lexer failures.
//! Phase 2 added [`SchemaError::Convert`] for failures that surface while
//! lowering a `SimplifiedSchema` to JSON Schema. Phase 3 adds the runtime
//! variants used by the resolution and validation pipeline:
//!
//! - [`SchemaError::Unresolved`] — a `$schema` file reference could not be
//!   resolved on disk.
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

/// Errors produced by the schemas subsystem.
#[derive(Debug, Error)]
pub enum SchemaError {
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

    /// A `$schema` file reference could not be resolved (the path does not
    /// exist, the syntax is invalid, or biscuit-file rejected it).
    #[error("could not resolve $schema reference `{reference}`")]
    Unresolved {
        reference: String,
        #[source]
        source: biscuit_file::FileReferenceError,
    },

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

    /// I/O failed while reading a referenced or baseline schema file.
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

    /// A `$schema` file-reference chain revisits a file it is already
    /// resolving, or exceeds the delegation depth cap. A schema file whose
    /// whole payload is a reference (`$schema: ./other.yaml`) — and each
    /// root-union file arm — re-enters reference resolution, so a self- or
    /// mutually-referencing pair would otherwise recurse until the process
    /// stack overflows. `chain` lists the canonical paths in visit order.
    #[error("$schema file-reference cycle detected: {chain}")]
    ReferenceCycle { chain: String },

    /// An `example(...)` constraint (Feature A) referenced a file that is
    /// missing, malformed, or does not validate against the example-artifact
    /// envelope (or its inherited `parameters` shape). Example files are
    /// documentation the schema promises are trustworthy, so a broken example
    /// is a fail-loud schema-load error, never a warning. `reference` is the
    /// authored `example(...)` reference; `message` is the underlying reason.
    #[error("invalid example artifact `{reference}`: {message}")]
    InvalidExample { reference: String, message: String },

    /// A trigger-schema match expression is malformed (bad combinator shape,
    /// unknown key, or an otherwise unparseable `match:` payload). `message`
    /// describes the structural failure; the trigger grammar is defined in
    /// `darkmatter/features/2026-07-10-schema-triggers/spec.md`.
    #[error("invalid trigger-schema match expression: {message}")]
    TriggerMatch { message: String },

    /// A trigger-schema match condition used a constraint outside the
    /// match-safe subset (constraints that resolve files, import types, load
    /// examples, provide defaults, mark generated values, or eager-existence-
    /// check `file`). `property` names the offending property; `constraint`
    /// names the forbidden constraint keyword.
    #[error(
        "trigger-schema match condition for `{property}` uses a forbidden constraint: {constraint}"
    )]
    TriggerForbiddenConstraint { property: String, constraint: String },

    /// A trigger-schema match arm mapping mixes structural keys (combinators
    /// `all`/`any`/`none`/`min-match` or the `$path` predicate) with
    /// property-condition keys. Mixing is a load error — never a guess about
    /// the author's intent. `structural` and `properties` list the offending
    /// keys.
    #[error(
        "trigger-schema match arm mixes structural keys ({structural:?}) with property keys \
         ({properties:?}); use `all:` to combine them"
    )]
    TriggerMixedMapping {
        structural: Vec<String>,
        properties: Vec<String>,
    },

    /// A trigger-schema match arm is vacuous: no satisfiable path through the
    /// boolean tree contains a presence-requiring condition (a `required` gate
    /// or `$path` predicate inside `all`/`any`, not under `none`). Because
    /// arms OR together, one vacuous arm makes the whole trigger match every
    /// document — almost always an authoring mistake.
    #[error("trigger-schema match arm is vacuous (no presence-requiring condition)")]
    TriggerVacuousArm,

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

    /// A trigger-schema envelope file claimed `kind: trigger-schema` but could
    /// not be parsed. The underlying grammar/envelope error is on `source`;
    /// `path` is the offending file.
    #[error("trigger-schema file `{}` is malformed: {source}", path.display())]
    TriggerLoad {
        path: PathBuf,
        #[source]
        source: Box<SchemaError>,
    },

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

    /// A trigger-schema payload did not resolve to a merge-compatible object
    /// schema. Trigger payloads must be SimplifiedSchema single objects or
    /// raw JSON Schemas satisfying the simple-object baseline contract; root
    /// unions and non-object schemas are rejected because the property-keyed
    /// merge has no sound "later wins" meaning for them. `path` is the
    /// trigger envelope file; `payload` is the authored `$schema:` reference
    /// (or `<inline>` for an inline payload); `reason` explains the rejection.
    #[error(
        "trigger-schema payload in `{}` is not merge-compatible: {reason}",
        path.display()
    )]
    TriggerPayloadNotMergeable {
        path: PathBuf,
        payload: String,
        reason: String,
    },

    /// A trigger-schema payload resolved to a trigger envelope file (directly
    /// or through an import cycle), rather than to an ordinary schema file.
    /// `trigger` is the originating envelope; `payload_path` is the file the
    /// payload resolved to; `chain` describes the resolution path for the
    /// indirect case.
    #[error(
        "trigger-schema payload in `{}` resolved to a trigger-schema envelope `{}`",
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
        "document `$schema` referenced a trigger-schema file `{reference}`; trigger schemas \
         activate by placement and match — reference the payload `{suggestion}` instead"
    )]
    TriggerSchemaReferenced {
        reference: String,
        suggestion: String,
    },
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
                        Prose::escape_text(message)
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
                        Prose::escape_text(message)
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
                        Prose::escape_text(&source.to_string())
                    )),
                ])
                .hint(
                    "Confirm the file exists, uses a supported reference prefix (e.g. <cyan>./</cyan>, \
                     <cyan>~/</cyan>, <cyan>@/</cyan>), and is readable.",
                ),

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
                        Prose::escape_text(message)
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
                    Prose::escape_text(message)
                ))];
                if let Some(cause) = source {
                    body.push(Prose::new(format!(
                        "<dim>Cause:</dim> {}",
                        Prose::escape_text(&cause.to_string())
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
                    Prose::escape_text(message)
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
                        Prose::escape_text(&source.to_string()),
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
                    Prose::escape_text(message)
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
                            Prose::escape_text(message)
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
                    "trigger-schema match expression invalid",
                ))
                .body(Prose::new(format!(
                    "<dim>Reason:</dim> {}",
                    Prose::escape_text(message)
                )))
                .hint(
                    "See the trigger grammar: combinators (<cyan>all</cyan>/<cyan>any</cyan>/\
                     <cyan>none</cyan>/<cyan>min-match</cyan>), property conditions, and the \
                     <cyan>$path</cyan> predicate.",
                ),

            SchemaError::TriggerForbiddenConstraint { property, constraint } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "SchemaError",
                        "trigger-schema forbidden constraint",
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
                    "trigger-schema mixed mapping",
                ))
                .body(Prose::new(format!(
                    "A match arm mapping cannot mix structural keys ({:?}) with property keys \
                     ({:?}).",
                    structural, properties
                )))
                .hint(
                    "Wrap the conditions in an <cyan>all:</cyan> combinator to combine a \
                     structural key with property conditions.",
                ),

            SchemaError::TriggerVacuousArm => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("SchemaError", "trigger-schema vacuous arm"))
                .body(Prose::new(
                    "A match arm has no presence-requiring condition (a <cyan>required</cyan> \
                     gate or <cyan>$path</cyan> predicate), so it matches every document.",
                ))
                .hint(
                    "Add a <cyan>required</cyan> constraint or a <cyan>$path</cyan> predicate so \
                     the arm only fires on matching documents.",
                ),

            SchemaError::TriggerCaseFoldCollision { root, files } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "SchemaError",
                        "trigger-schema case-fold collision",
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
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("SchemaError", "trigger-schema file malformed"))
                    .body(vec![
                        Prose::new(format!(
                            "Trigger file <cyan>{}</cyan> could not be parsed:",
                            Prose::escape_text(&path.display().to_string())
                        )),
                        Prose::new(format!(
                            "<dim>Reason:</dim> {}",
                            Prose::escape_text(&source.to_string())
                        )),
                    ])
                    .hint(
                        "Fix the match grammar or envelope shape. See the trigger grammar: \
                         combinators (<cyan>all</cyan>/<cyan>any</cyan>/<cyan>none</cyan>/\
                         <cyan>min-match</cyan>), property conditions, and the \
                         <cyan>$path</cyan> predicate.",
                    )
            }

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
                    "trigger-schema payload not merge-compatible",
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
                        Prose::escape_text(reason)
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
                    "trigger-schema payload cycle",
                ))
                .body(vec![
                    Prose::new(format!(
                        "Trigger file <cyan>{}</cyan> payload resolved to a trigger-schema \
                         envelope <cyan>{}</cyan>:",
                        Prose::escape_text(&trigger.display().to_string()),
                        Prose::escape_text(&payload_path.display().to_string())
                    )),
                    Prose::new(format!("<dim>Chain:</dim> {}", Prose::escape_text(chain))),
                ])
                .hint(
                    "A trigger payload must not resolve to a trigger-schema envelope, directly or \
                     through an import cycle. Reference an ordinary schema file instead.",
                ),

            SchemaError::TriggerSchemaReferenced {
                reference,
                suggestion,
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "SchemaError",
                    "trigger-schema file referenced as document schema",
                ))
                .body(vec![
                    Prose::new(format!(
                        "Document <cyan>$schema</cyan> referenced a trigger-schema file \
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
