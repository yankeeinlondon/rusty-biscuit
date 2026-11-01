//! The [`SimplifiedSchema`] object: one schema, built from content or a
//! file, that layers, validates, answers type queries, and exports.
//!
//! - **Build.** [`SimplifiedSchema::try_from`] reads document *content* (a
//!   standalone schema YAML document, Markdown with a frontmatter `$schema`,
//!   or a raw JSON Schema) and never touches the filesystem. With the `file`
//!   feature, [`SimplifiedSchema::try_from_yaml`],
//!   [`SimplifiedSchema::try_from_markdown`],
//!   [`SimplifiedSchema::try_from_json_schema`], and
//!   [`SimplifiedSchema::resolve`] read one file through a [`SchemaSource`].
//! - **Validate.** [`SimplifiedSchema::validate_strict`] and
//!   [`SimplifiedSchema::validate_soft`] compile the schema on first use and
//!   reuse that preparation until a layering call changes the definition.
//! - **Layer.** [`SimplifiedSchema::layer_under`] and
//!   [`SimplifiedSchema::layer_over`] merge other schemas into this one with
//!   whole-property replacement; each call commits completely or not at all.
//! - **Inspect and export.** [`SimplifiedSchema::query`] and the
//!   [`TypeDefinition`] / [`FunctionView`] views borrow the typed
//!   definitions; [`SimplifiedSchema::to_json_schema`] and
//!   [`SimplifiedSchema::to_simplified_yaml`] export them.
//!
//! See the package's `docs/topics/schema-object.md`.

mod error;
mod export;
mod layer;
pub(crate) use layer::content_context;
mod layering;
mod view;

#[cfg(feature = "file")]
mod capture;
#[cfg(feature = "file")]
mod source;

#[cfg(test)]
mod tests;

use std::sync::{Arc, Mutex, MutexGuard};

use serde_json::Value;

use crate::outcome::{ValidationFailure, ValidationSuccess};
use crate::prepared::Prepared;
use crate::{SchemaAdvisory, SchemaError};

pub use error::SimplifiedSchemaError;
pub use layer::SchemaProvenance;
pub use view::{FunctionView, PathSyntaxError, TypeDefinition};

#[cfg(feature = "file")]
pub use capture::repository_scope_catalog;
#[cfg(feature = "file")]
pub use source::SchemaSource;

use layer::Layer;

/// A SimplifiedSchema (or raw JSON Schema) definition with its validation,
/// layering, inspection, and export.
///
/// The object keeps the forms the schema was authored in, never flattened:
/// the typed declaration (a [`crate::SchemaRoot`], root unions included) and
/// its named types for inspection and export, and the lowered JSON Schema
/// for validation. Validation prepares itself on first use; there is no
/// compile step to call.
///
/// Cloning is cheap and shares the definition and any prepared state.
pub struct SimplifiedSchema {
    definition: Arc<Definition>,
    /// Bumped by every layering call that changes the definition.
    revision: u64,
    /// The validators prepared for `revision`, built on first validation.
    /// `None` until then, and again after a definition change.
    prepared: Mutex<Option<Arc<PreparedState>>>,
}

/// The committed definition: replaced as a whole, never edited in place.
#[derive(Debug)]
struct Definition {
    /// The typed layers, highest precedence first. A schema from a factory
    /// has one; each layering call adds the incoming schemas' layers.
    layers: Vec<Arc<Layer>>,
    /// The index in `layers` of the layer the schema was built from. Its file
    /// context judges file values and its provenance is the schema's, however
    /// many layers were merged over or under it.
    receiver: usize,
    /// The merged lowered JSON Schema of a layered definition. `None` for a
    /// single layer, whose own lowering is the schema.
    merged: Option<Arc<Value>>,
}

/// A preparation outcome, kept so a schema that cannot be prepared reports
/// the same error on every validation without retrying it.
type PreparedState = Result<Prepared, Arc<SchemaError>>;

impl std::fmt::Debug for SimplifiedSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimplifiedSchema")
            .field("definition", &self.definition)
            .field("revision", &self.revision)
            .finish_non_exhaustive()
    }
}

impl Clone for SimplifiedSchema {
    fn clone(&self) -> Self {
        Self {
            definition: Arc::clone(&self.definition),
            revision: self.revision,
            prepared: Mutex::new(self.prepared_slot().clone()),
        }
    }
}

impl TryFrom<&str> for SimplifiedSchema {
    type Error = SimplifiedSchemaError;

    /// Reads schema-bearing document content: a standalone schema document
    /// (root `$schema`, or `kind: schema` with `types`), Markdown whose
    /// frontmatter has `$schema`, or a raw JSON Schema (a mapping whose
    /// `$schema` is a JSON Schema dialect URI).
    ///
    /// The text is content, never a path: no file is opened, and no request
    /// context is captured. References the schema makes to other files are
    /// kept as parsed references; such a schema can be inspected and
    /// exported, and validation reports [`ValidationFailure::Unprepared`].
    ///
    /// ## Errors
    ///
    /// [`SimplifiedSchemaError::MissingSchema`] when the content holds no
    /// schema-bearing structure (ordinary YAML, a YAML scalar, Markdown
    /// without `$schema`), [`SimplifiedSchemaError::Frontmatter`] for a
    /// malformed frontmatter fence, and [`SimplifiedSchemaError::Schema`] for
    /// a malformed definition.
    fn try_from(content: &str) -> Result<Self, Self::Error> {
        Layer::from_content(content).map(Self::from_layer)
    }
}

impl SimplifiedSchema {
    fn from_layer(layer: Layer) -> Self {
        Self {
            definition: Arc::new(Definition {
                layers: vec![Arc::new(layer)],
                receiver: 0,
                merged: None,
            }),
            revision: 0,
            prepared: Mutex::new(None),
        }
    }

    /// The definition revision: `0` when built, bumped by each layering call
    /// that changes the definition. Prepared validation state belongs to one
    /// revision.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Where this schema came from and what resolving it read: its file,
    /// imports, examples, referenced schema files, probed locations, and
    /// advisories. Empty for a schema built from content. Layering keeps the
    /// receiver's; [`Self::layer_provenance`] lists every layer's.
    pub fn provenance(&self) -> &SchemaProvenance {
        &self.receiver().provenance
    }

    /// The provenance of every layer, highest precedence first.
    pub fn layer_provenance(&self) -> impl Iterator<Item = &SchemaProvenance> {
        self.definition.layers.iter().map(|layer| &layer.provenance)
    }

    /// The object named type `name` as a schema of its own, sharing this
    /// schema's file context: a `types` entry, or a top-level entry of a
    /// `kind: schema` document's `types` payload.
    ///
    /// ```
    /// use simplified_schema::SimplifiedSchema;
    ///
    /// let catalog = SimplifiedSchema::try_from("kind: schema\ntypes:\n  Person:\n    name: string(required)\n")?;
    /// let person = catalog.select("Person")?;
    /// assert!(person.validate_strict(&serde_json::json!({ "name": "Ada" })).is_ok());
    /// # Ok::<(), simplified_schema::SimplifiedSchemaError>(())
    /// ```
    ///
    /// ## Errors
    ///
    /// [`SimplifiedSchemaError::Selection`] when there is no such type or it
    /// is not an object type.
    pub fn select(&self, name: impl AsRef<str>) -> Result<Self, SimplifiedSchemaError> {
        self.receiver().select(name.as_ref()).map(Self::from_layer)
    }

    /// Judges `value` as written: nothing is converted. On success the
    /// accepted value is `value` itself, borrowed.
    ///
    /// The first validation prepares the schema; later ones reuse that
    /// preparation until the definition changes.
    ///
    /// ## Errors
    ///
    /// [`ValidationFailure`] with every problem found, or
    /// [`ValidationFailure::Unprepared`] when the schema cannot be prepared.
    pub fn validate_strict<'input>(&self, value: &'input Value) -> Result<ValidationSuccess<'input>, ValidationFailure> {
        self.with_prepared(|prepared| prepared.validate_strict(value))
    }

    /// Judges `value` after the conversion engine's permitted conversions
    /// (the string `"4"` for a `number`). On success the accepted value is
    /// `value` itself, borrowed, when it was accepted as written, and the
    /// converted value, owned, otherwise. `value` is never changed.
    ///
    /// ## Errors
    ///
    /// As [`Self::validate_strict`]; a failure never carries a partially
    /// converted value.
    pub fn validate_soft<'input>(&self, value: &'input Value) -> Result<ValidationSuccess<'input>, ValidationFailure> {
        self.with_prepared(|prepared| prepared.validate_soft(value))
    }

    fn with_prepared<'input>(
        &self,
        judge: impl FnOnce(&Prepared) -> Result<ValidationSuccess<'input>, ValidationFailure>,
    ) -> Result<ValidationSuccess<'input>, ValidationFailure> {
        match self.prepared_state().as_ref() {
            Ok(prepared) => judge(prepared),
            Err(error) => Err(ValidationFailure::Unprepared { error: Arc::clone(error) }),
        }
    }

    /// The prepared state of the current revision, built on first use.
    fn prepared_state(&self) -> Arc<PreparedState> {
        let mut slot = self.prepared_slot();
        if let Some(state) = slot.as_ref() {
            return Arc::clone(state);
        }
        let state = Arc::new(self.prepare());
        *slot = Some(Arc::clone(&state));
        state
    }

    fn prepared_slot(&self) -> MutexGuard<'_, Option<Arc<PreparedState>>> {
        // A panic while preparing leaves the slot as it was; nothing in it
        // can be half-written.
        self.prepared.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn prepare(&self) -> PreparedState {
        let json_schema = self.lowered()?;
        let advisories: Vec<SchemaAdvisory> = self
            .definition
            .layers
            .iter()
            .flat_map(|layer| layer.provenance.advisories.iter().cloned())
            .collect();
        let prepared = match &self.receiver().binding {
            Some(binding) => Prepared::new(json_schema, Some(binding.base_dir.clone()), binding.context.clone()),
            // Content has no file context: file checks are unavailable, and
            // the placeholder context reads no process state.
            None => Prepared::new(json_schema, None, layer::content_context()).map(Prepared::without_file_checks),
        };
        prepared.map(|prepared| prepared.with_advisories(advisories)).map_err(Arc::new)
    }

    /// The lowered JSON Schema values are judged against.
    fn lowered(&self) -> Result<Arc<Value>, Arc<SchemaError>> {
        match &self.definition.merged {
            Some(merged) => Ok(Arc::clone(merged)),
            None => self.top().lowered.clone(),
        }
    }

    /// The highest-precedence layer.
    fn top(&self) -> &Layer {
        &self.definition.layers[0]
    }

    /// The layer the schema was built from.
    fn receiver(&self) -> &Layer {
        &self.definition.layers[self.definition.receiver]
    }

    /// The current prepared state's identity, for tests of cache reuse.
    #[cfg(test)]
    fn prepared_identity(&self) -> Option<*const PreparedState> {
        self.prepared_slot().as_ref().map(Arc::as_ptr)
    }
}
