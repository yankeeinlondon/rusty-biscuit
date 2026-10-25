//! Typed records of the schema triggers automatic discovery skipped.
//!
//! Discovery recovers per trigger: a trigger whose envelope or payload fails
//! to load is left out of the registry and reported as an
//! [`InvalidTriggerFinding`] instead of failing the scan. The library never
//! prints one; each host delivers them through its own warning surface under
//! [`SCHEMA_INVALID_TRIGGER_CODE`](crate::markdown::schemas::SCHEMA_INVALID_TRIGGER_CODE).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::markdown::schemas::SchemaAdvisory;
use crate::markdown::schemas::errors::SchemaError;

use super::discovery::TriggerSource;

/// One schema trigger automatic discovery skipped, with the first failure
/// encountered while loading it.
///
/// A registry holds its findings in discovery order. Two findings report the
/// same trigger exactly when their [`identity`](Self::identity) is equal;
/// hosts deduplicate display by it, never by file name.
#[derive(Debug, Clone)]
pub struct InvalidTriggerFinding {
    source: TriggerSource,
    identity: TriggerSource,
    cause: Arc<SchemaError>,
    cause_path: Option<PathBuf>,
    position: Option<(usize, usize)>,
    /// Files a failed payload resolution read before it stopped; the
    /// registry's dependencies, so editing one refreshes the result.
    pub(crate) read: Vec<PathBuf>,
    /// Locations a failed payload resolution probed, present or absent; the
    /// registry's searched locations, so creating one refreshes the result.
    pub(crate) searched: Vec<PathBuf>,
}

impl InvalidTriggerFinding {
    /// Shared diagnostic code of every skipped-trigger warning.
    pub const CODE: &'static str = crate::markdown::schemas::SCHEMA_INVALID_TRIGGER_CODE;
    /// Diagnostic source family, shared with the schema advisories.
    pub const SOURCE: &'static str = SchemaAdvisory::SOURCE;

    /// The finding for `source`, skipped because of `cause`.
    ///
    /// `cause` is the underlying error, never a `TriggerLoad` wrapper. A
    /// [`SchemaError::TriggerEnvelopeYaml`] cause with a complete location
    /// gives the finding its [`position`](Self::position); a payload or import
    /// path the cause names gives its [`cause_path`](Self::cause_path).
    pub(crate) fn new(source: TriggerSource, cause: SchemaError) -> Self {
        let identity = match &source {
            TriggerSource::File(path) => {
                TriggerSource::File(biscuit_file::canonicalize_simplified(path).unwrap_or_else(|_| path.clone()))
            }
            embedded @ TriggerSource::Embedded { .. } => embedded.clone(),
        };
        let position = match &cause {
            SchemaError::TriggerEnvelopeYaml {
                line: Some(line),
                column: Some(column),
                ..
            } => Some((*line, *column)),
            _ => None,
        };
        let cause_path = match &source {
            TriggerSource::File(trigger) => related_path(&cause, trigger),
            TriggerSource::Embedded { .. } => None,
        };
        Self {
            source,
            identity,
            cause: Arc::new(cause),
            cause_path,
            position,
            read: Vec::new(),
            searched: Vec::new(),
        }
    }

    /// This finding with the observations of the failed payload resolution.
    #[must_use]
    pub(crate) fn with_observations(mut self, read: Vec<PathBuf>, searched: Vec<PathBuf>) -> Self {
        self.read = read;
        self.searched = searched;
        self
    }

    /// The skipped trigger: a schema-root file or an embedded bundle document.
    #[must_use]
    pub fn source(&self) -> &TriggerSource {
        &self.source
    }

    /// The trigger's full identity: its normalized path (see
    /// [`biscuit_file::canonicalize_simplified`]) for a file, the
    /// `(bundle, document)` pair for an embedded trigger.
    #[must_use]
    pub fn identity(&self) -> &TriggerSource {
        &self.identity
    }

    /// The first failure encountered while loading the trigger.
    #[must_use]
    pub fn cause(&self) -> &SchemaError {
        &self.cause
    }

    /// Consumes the finding into its cause.
    #[must_use]
    pub fn into_cause(self) -> Arc<SchemaError> {
        self.cause
    }

    /// The payload or import file the cause points into, when it is not the
    /// trigger file itself. Always `None` for an embedded trigger, whose
    /// documents are never host files.
    #[must_use]
    pub fn cause_path(&self) -> Option<&Path> {
        self.cause_path.as_deref()
    }

    /// The 1-based `(line, column)` in the trigger file of a malformed
    /// envelope's YAML error; `None` for every other cause.
    #[must_use]
    pub fn position(&self) -> Option<(usize, usize)> {
        self.position
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn code(&self) -> &'static str {
        Self::CODE
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn source_family(&self) -> &'static str {
        Self::SOURCE
    }

    /// The one-line explanation: the trigger, that it was ignored, and why.
    #[must_use]
    pub fn message(&self) -> String {
        let reason = reason(&self.cause);
        match &self.cause_path {
            Some(path) if !reason.contains(&path.display().to_string()) => format!(
                "ignoring invalid schema trigger {}: {reason} (in {})",
                self.source,
                path.display()
            ),
            _ => format!("ignoring invalid schema trigger {}: {reason}", self.source),
        }
    }
}

impl PartialEq for InvalidTriggerFinding {
    /// Equal findings name the same trigger, cause text, and locations.
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
            && self.identity == other.identity
            && self.cause_path == other.cause_path
            && self.position == other.position
            && self.cause.to_string() == other.cause.to_string()
    }
}

/// The cause's own text, with a YAML error's location spelled out.
fn reason(cause: &SchemaError) -> String {
    match cause {
        SchemaError::TriggerEnvelopeYaml { message, line, column } => match (line, column) {
            (Some(line), Some(column)) => format!("not valid YAML at line {line}, column {column}: {message}"),
            _ => format!("not valid YAML: {message}"),
        },
        other => other.to_string(),
    }
}

/// The file other than `trigger` that `cause` names as the failure's
/// location: the payload or import a resolution failure stopped in.
fn related_path(cause: &SchemaError, trigger: &Path) -> Option<PathBuf> {
    let path = match cause {
        SchemaError::Io { path, .. }
        | SchemaError::ReferencedSchema { path, .. }
        | SchemaError::SchemaDocument { path, .. } => path,
        SchemaError::TriggerPayloadCycle { payload_path, .. } => payload_path,
        SchemaError::TriggerPayloadCompile { payload_path: Some(path), .. } => path,
        _ => return None,
    };
    (path != trigger).then(|| path.clone())
}
