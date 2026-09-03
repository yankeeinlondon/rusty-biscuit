use std::path::{Path, PathBuf};

/// Stable semantic identity for a schema-resolution advisory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SchemaAdvisoryKind {
    /// A referenced YAML map resembles SimplifiedSchema authoring but lacks a
    /// supported standalone schema envelope.
    MissingSimplifiedEnvelope,
}

/// A non-fatal schema-resolution finding tied to a referenced file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaAdvisory {
    kind: SchemaAdvisoryKind,
    path: PathBuf,
}

impl SchemaAdvisory {
    /// Diagnostic source shared by every consumer projection.
    pub const SOURCE: &'static str = "darkmatter.schema";

    /// Creates the advisory for a simplified-looking YAML map without an
    /// authoring envelope.
    pub fn missing_simplified_envelope(path: impl Into<PathBuf>) -> Self {
        Self {
            kind: SchemaAdvisoryKind::MissingSimplifiedEnvelope,
            path: path.into(),
        }
    }

    /// Returns the stable advisory kind.
    pub fn kind(&self) -> SchemaAdvisoryKind {
        self.kind
    }

    /// Returns the stable diagnostic source.
    pub fn source(&self) -> &'static str {
        Self::SOURCE
    }

    /// Returns the stable machine-readable diagnostic code.
    pub fn code(&self) -> &'static str {
        match self.kind {
            SchemaAdvisoryKind::MissingSimplifiedEnvelope => {
                "dm.schema.missing_simplified_envelope"
            }
        }
    }

    /// Returns the referenced file associated with this advisory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Builds the human-readable diagnostic message.
    pub fn message(&self) -> String {
        match self.kind {
            SchemaAdvisoryKind::MissingSimplifiedEnvelope => format!(
                "{} looks like a SimplifiedSchema but has no envelope, so it was read as raw JSON Schema and constrains nothing. Wrap the properties under a root `$schema:` key (or `kind: schema` + `types:`).",
                self.path.display()
            ),
        }
    }
}
