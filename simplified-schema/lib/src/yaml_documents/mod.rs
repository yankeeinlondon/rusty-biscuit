//! Whole YAML documents (`.yaml`, `.yml`): loading, kind selection, and the
//! checks of the language's own document kinds.
//!
//! A YAML document selects its schema by its top-level `kind` when a
//! `kind: yaml-schema` definition claims it, and otherwise by a root
//! `$schema`; a document with neither is not validated ([`select`]). The
//! claims of one document context are its [`KindRegistry`]; registered
//! hosts' definitions ([`RegisteredKinds`]) claim only kinds it leaves
//! unclaimed ([`select_with_registered`]). The language's
//! own kinds (`schema`, `schema-trigger`, `yaml-schema`) then get the checks
//! their definitions cannot state ([`semantic`]).
//!
//! This module is host-independent: a host (Darkmatter's
//! `DarkmatterSchemas::validate_yaml_document`) supplies its shipped sources
//! and prepared shipped definitions ([`KindHost`]), prepares the selected
//! schema into its own effective schema, and judges the document with it.
//!
//! The user-facing contract is the "YAML documents" section of
//! `darkmatter/docs/topics/schemas/definition.md`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::SchemaError;

pub mod load;
mod marks;
pub mod registry;
pub mod select;
pub mod semantic;

pub use load::{LoadedYaml, first_tag_start, load_yaml_document, locating_problem, pointer_string};
pub use registry::{
    Claim, DefinitionSource, Incomplete, KindDefinition, KindHost, KindRegistry, RegisteredDefinition, RegisteredKinds,
    RegisteredSource, RegistryScope, ShippedDefinition, prepare_shipped, prepare_shipped_definitions,
};
pub use select::{Selection, select, select_with_registered};
pub use semantic::{CHECKED_KINDS, FILE_READING_KINDS};

/// A whole YAML document admitted by the one YAML document loader, ready
/// for a host's validation.
///
/// Loading is separate from validation so a caller can tell a document that
/// is not admissible YAML from one whose schema failed: both are
/// [`SchemaError`]s, and a `$schema` may name a file whose own load error
/// looks exactly like the document's.
#[derive(Debug, Clone)]
pub struct LoadedYamlDocument {
    text: String,
    path: PathBuf,
    loaded: Arc<LoadedYaml>,
}

impl LoadedYamlDocument {
    /// Loads `text`, the whole YAML document at `path`.
    ///
    /// ## Errors
    ///
    /// [`SchemaError::SchemaDocument`] naming `path`, with a span, when the
    /// text is not one admissible YAML document (malformed YAML, a second
    /// document, a duplicate or non-string key, an inadmissible number).
    pub fn load(text: &str, path: &Path) -> Result<Self, SchemaError> {
        let loaded = Arc::new(load::load_yaml_document(text, path)?);
        Ok(Self { text: text.to_string(), path: path.to_path_buf(), loaded })
    }

    /// The text as given to [`Self::load`].
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The path given to [`Self::load`].
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The loaded document: its authored value, its JSON value, and what
    /// locates its problems. Shared, so a validation result can keep it.
    #[must_use]
    pub fn loaded(&self) -> &Arc<LoadedYaml> {
        &self.loaded
    }

    /// A hash of what a host's selection and contract preparation read from
    /// this document: its top-level `kind` and root `$schema`, as authored.
    /// Two texts of one document with the same key select and prepare the
    /// same contract in the same context and registry, so a prepared
    /// contract judges both.
    #[must_use]
    pub fn declaration_key(&self) -> u64 {
        let map = self.loaded.value.as_mapping();
        let declaration = (
            map.is_some(),
            map.and_then(|map| map.get("kind")),
            map.and_then(|map| map.get("$schema")),
        );
        // The loader admitted these values (string keys, admissible
        // numbers), so serializing them does not fail.
        biscuit_hash::xx_hash(&serde_yaml_ng::to_string(&declaration).unwrap_or_default())
    }

    /// Whether this document's `kind` names a kind whose checks read files
    /// (`schema`, `yaml-schema`): judging it then depends on its whole text,
    /// not only on its [`Self::declaration_key`].
    #[must_use]
    pub fn kind_checks_read_files(&self) -> bool {
        self.loaded
            .value
            .get("kind")
            .and_then(serde_yaml_ng::Value::as_str)
            .is_some_and(|kind| semantic::FILE_READING_KINDS.contains(&kind))
    }

    /// The whole document as JSON, exactly as loaded: no soft conversion,
    /// and a root `$schema` still present.
    #[must_use]
    pub fn json(&self) -> &serde_json::Value {
        &self.loaded.json
    }

    /// Runs `kind`'s checks ([`semantic`]) over this document; `accepted` is
    /// the copy of it the selected definition accepted, and `schema_roots`,
    /// `context`, and `sources` resolve what a schema or definition imports.
    /// Empty for a kind without checks. Passive unless `kind` is one of
    /// [`FILE_READING_KINDS`].
    #[must_use]
    pub fn kind_checks(
        &self,
        kind: &str,
        accepted: &serde_json::Value,
        schema_roots: &[PathBuf],
        context: &biscuit_file::FileResolutionContext,
        sources: crate::embedded::ShippedSources,
    ) -> Vec<crate::ValidationProblem> {
        semantic::kind_checks(kind, self, accepted, schema_roots, context, sources)
    }
}

/// The repository-relative paths of the shipped kind definitions every YAML
/// document kind registry starts from.
#[must_use]
pub fn shipped_kind_definition_paths() -> Vec<PathBuf> {
    crate::envelope::ENVELOPE_DEFINITIONS.iter().map(|document| PathBuf::from(document.path)).collect()
}
