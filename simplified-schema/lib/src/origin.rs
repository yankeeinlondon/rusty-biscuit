//! Where each property of a resolved schema came from.

use std::path::{Path, PathBuf};

use crate::simplified::SchemaSourceId;

/// Where an effective-schema property came from (R-5 Priority 2), so a
/// diagnostic can point `relatedInformation` at the schema source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaOriginKind {
    /// An inline `$schema` mapping/sequence in the document frontmatter.
    Document,
    /// A `$schema` file reference.
    ReferencedFile,
    /// The configured baseline schema.
    Baseline,
    /// A matching schema-trigger payload layered between the baseline and the
    /// document `$schema`.
    Trigger,
}

/// The origin of one effective-schema property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaOrigin {
    /// Which layer the property's schema came from.
    pub kind: SchemaOriginKind,
    /// The source that declared the property: the referenced schema file
    /// ([`SchemaOriginKind::ReferencedFile`]) or the trigger envelope
    /// ([`SchemaOriginKind::Trigger`]): a [`SchemaSourceId::File`] for a
    /// schema-root file, a [`SchemaSourceId::Embedded`] (`{bundle}/{name}`)
    /// for a bundle trigger. `None` for the document and baseline layers.
    pub source: Option<SchemaSourceId>,
}

impl SchemaOrigin {
    /// An inline-document origin.
    pub fn document() -> Self {
        Self {
            kind: SchemaOriginKind::Document,
            source: None,
        }
    }

    /// A baseline-schema origin.
    pub fn baseline() -> Self {
        Self {
            kind: SchemaOriginKind::Baseline,
            source: None,
        }
    }

    /// A referenced-file origin carrying the resolved path.
    pub fn referenced_file(path: impl Into<PathBuf>) -> Self {
        Self {
            kind: SchemaOriginKind::ReferencedFile,
            source: Some(SchemaSourceId::File(path.into())),
        }
    }

    /// A schema-trigger-payload origin for the trigger envelope `source`
    /// identifies.
    pub fn trigger_layer(source: SchemaSourceId) -> Self {
        Self {
            kind: SchemaOriginKind::Trigger,
            source: Some(source),
        }
    }

    /// The host file the declaration was read from. `None` for the document
    /// and baseline layers and for an embedded trigger, which names no file.
    #[must_use]
    pub fn file(&self) -> Option<&Path> {
        match &self.source {
            Some(SchemaSourceId::File(path)) => Some(path),
            _ => None,
        }
    }
}

/// Per-top-level-property schema origins for an effective schema, keyed by
/// property name in schema declaration order.
pub type SchemaOriginMap = indexmap::IndexMap<String, SchemaOrigin>;
