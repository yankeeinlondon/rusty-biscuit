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

    /// A referenced schema file is neither a SimplifiedSchema (root `$schema:`
    /// is missing or is not a mapping) nor a recognisable JSON Schema.
    #[error(
        "referenced schema file `{path}` is neither a valid SimplifiedSchema (missing root \
         `$schema:` mapping) nor a valid JSON Schema"
    )]
    AmbiguousReferenced { path: PathBuf },

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
                    "<cyan>{}</cyan> is neither a SimplifiedSchema (missing root <cyan>$schema:</cyan> \
                     mapping) nor a valid JSON Schema.",
                    Prose::escape_text(&path.display().to_string())
                )))
                .hint(
                    "Either prefix a SimplifiedSchema mapping with <cyan>$schema:</cyan>, or \
                     supply a Draft 2020-12 JSON Schema.",
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
        }
    }
}

#[cfg(test)]
mod tests {
    use biscuit_terminal::components::renderable::Renderable;
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
        assert!(
            out.contains("expected mapping"),
            "missing detail: {out}",
        );
    }
}
