//! The typed failures of building, layering, and exporting a
//! [`super::SimplifiedSchema`].

use std::path::PathBuf;
use std::sync::Arc;

use biscuit_file::FileReferenceError;
use biscuit_file::frontmatter::FrontmatterEnvelopeError;

use crate::SchemaError;

/// Why a [`super::SimplifiedSchema`] could not be built, layered, or
/// exported.
///
/// A schema error keeps its source chain: a malformed definition is the
/// language's own [`SchemaError`], held by value.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SimplifiedSchemaError {
    /// The schema file does not exist. `path` is the location the input
    /// resolved to, or the reference as written when it matched no file.
    #[error("schema file `{}` does not exist", path.display())]
    MissingFile {
        /// The missing location.
        path: PathBuf,
    },

    /// The schema file exists but could not be read.
    #[error("could not read schema file `{}`", path.display())]
    Read {
        /// The file that failed to read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The input's file reference is malformed or could not be resolved in
    /// its context.
    #[error("could not resolve schema file reference `{reference}`")]
    Reference {
        /// The reference as written.
        reference: String,
        /// The resolver's typed failure.
        #[source]
        source: FileReferenceError,
    },

    /// The request context captured for a raw path is not usable (for
    /// example the current directory cannot be read).
    #[error("could not capture the file-resolution context for a schema file")]
    Context {
        /// The context failure.
        #[source]
        source: FileReferenceError,
    },

    /// Repository discovery failed while capturing a raw path's request
    /// context. No repository at all is not this error.
    #[error("could not discover the repository containing `{}`: {detail}", dir.display())]
    Discovery {
        /// The directory discovery started from.
        dir: PathBuf,
        /// The discovery failure, as reported.
        detail: String,
    },

    /// The input holds no schema-bearing structure: ordinary YAML, a YAML
    /// scalar, or Markdown whose frontmatter has no `$schema`.
    #[error("{source_name} holds no schema: {reason}")]
    MissingSchema {
        /// The file, or `content` for in-memory text.
        source_name: String,
        /// What was found instead.
        reason: String,
    },

    /// A Markdown input opens with a malformed frontmatter fence.
    #[error(transparent)]
    Frontmatter(#[from] FrontmatterEnvelopeError),

    /// The schema is malformed, or resolving or merging it failed.
    #[error(transparent)]
    Schema(#[from] SchemaError),

    /// A schema taking part in the operation cannot be lowered to JSON
    /// Schema (a function-typed property, or a reference an in-memory schema
    /// cannot read). The error is the one the schema holds, shared.
    #[error("the schema cannot be lowered to JSON Schema: {0}")]
    Unprepared(Arc<SchemaError>),

    /// [`super::SimplifiedSchema::select`] found no object type `name`.
    #[error("cannot select type `{name}`: {reason}")]
    Selection {
        /// The requested type.
        name: String,
        /// Why it cannot be selected.
        reason: String,
    },

    /// The schema has no SimplifiedSchema form to export: it is raw JSON
    /// Schema, or a layered definition whose layers do not combine into one
    /// document.
    #[error("cannot export this schema as SimplifiedSchema YAML: {reason}")]
    ExportUnsupported {
        /// Why the export is unsupported.
        reason: String,
    },
}
