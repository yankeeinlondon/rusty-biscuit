//! Darkmatter's schema host: document-, trigger-, and composition-aware
//! schema work for Markdown frontmatter.
//!
//! The SimplifiedSchema language itself (grammar, named-type resolution,
//! conversion, JSON Schema lowering, validator caching) is the
//! [`simplified_schema`] crate's, and callers import its types from there.
//! This module adds Darkmatter's policy around it: the shipped baseline,
//! schema roots and trigger discovery, the effective schema of a document
//! ([`DarkmatterSchemas`], [`EffectiveSchema`]), pending and deferred values,
//! eager-file normalization, and the `md schema` surfaces.
//!
//! ## Module layout
//!
//! - [`resolve`] — `$schema` resolution with Darkmatter's shipped schema
//!   documents (`Name@darkmatter:<stem>`).
//! - [`catalog`] and [`roots`] — the global catalog and schema search roots.
//! - [`triggers`] — schema-trigger discovery, matching, and payloads.
//! - [`rewrite`] — eager-`file` value normalization: rewrites a present
//!   `file(eager)`-typed value to its document-relative resolved path after
//!   validation accepts it.
//! - [`about`] — typed descriptor catalog that backs `md schema about`.
//! - [`detect`], [`clean`], [`completion`] — `md schema detect`, frontmatter
//!   cleanup, and editor completion.
//!
//! ## Example
//!
//! ```ignore
//! use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
//! use darkmatter::markdown::schemas::DarkmatterSchemas;
//! use darkmatter::markdown::Markdown;
//!
//! let context = build_resolution_context(&RequestSnapshot::new("/work/blog"))?;
//! let md = Markdown::try_from(std::path::Path::new("/work/blog/post.md"))?;
//! let api = DarkmatterSchemas::new(context);
//! let report = api.validate(&md)?;
//! assert!(report.valid);
//! ```
//!
//! See `darkmatter/features/2026-05-11-schemas/spec.md` for the full
//! specification.
//!

pub mod about;
pub mod catalog;
mod bound_operands;
pub mod clean;
pub(crate) mod coerce;
pub mod completion;
pub mod detect;
mod frontmatter_shape;
pub mod resolve;
pub mod rewrite;
pub mod roots;
pub mod slots;
pub mod triggers;
pub mod yaml_documents;
mod positions;
pub(crate) mod embedded;

use simplified_schema::{
    phase, scope, simplified, timings, validate,
};

use std::sync::OnceLock;
use std::{
    collections::{BTreeSet, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use jsonschema::Validator;
use serde_json::Value;

use crate::markdown::{Markdown, compose::ComposeSource};

pub use about::{
    ALPHANUMERIC_OPTION_DESCRIPTORS, AlphanumericOptionDescriptor, alphanumeric_option_descriptors,
    BlockConstraintDescriptor, block_constraint_descriptors,
    schema_type_descriptor, CoercionRuleDescriptor, InlineObjectRuleDescriptor, MatchSafeConstraintDescriptor,
    SchemaConstraintDescriptor, SchemaShapeDescriptor, SchemaTypeDescriptor,
    TriggerGrammarDescriptor, ValidationBehaviorDescriptor, coercion_rule_descriptors,
    inline_object_rule_descriptors, schema_constraint_descriptors, schema_shape_descriptors,
    schema_type_descriptors, validation_behavior_descriptors, match_safe_constraint_descriptors,
    trigger_grammar_descriptors,
};
use simplified_schema::{
    JsonPointer, SchemaAdvisory, SchemaError, SchemaOrigin, SchemaOriginMap, SchemaPhase, ValidationProblem, ValidationProblemCode, ValidationProblemKind, authored_node_at,
};
pub use completion::{CompletionKind, CompletionSuggestion};
pub use clean::{
    CleanBaselineSchema, CleanSchemaConfig, CleanSchemaContext, SchemaCleanAnalysis,
    analyze_frontmatter, schema_result_set_identical,
};
pub use detect::{
    DetectOptions, detect_from_document, detect_schema,
    detect_schema_with_contexts, schema_to_yaml,
};
pub use frontmatter_shape::effective_property_shape;
pub use triggers::TriggerOrigin;
pub use rewrite::NormalizationOutcome;
use simplified::{
    DefKey, Definition, DefinitionOuter, DefinitionTable, PropertyAtom, PropertyDef, SchemaShape, SchemaRoot, SimplifiedType, TypeExpr, to_json_schema_with_definitions,
};
pub use roots::{
    InvalidSchemasDir, SCHEMAS_DIR_VARIABLE, SchemaSearchRoot, SchemaRootKind, SchemaRootState,
    SchemaRoots, SearchedRoot,
};
pub use embedded::EmbeddedSchemaBundle;
pub use yaml_documents::{KindRegistry, LoadedYamlDocument, RegistryScope, Route, YamlContract, YamlOutcome, YamlValidation};
pub use triggers::{
    InvalidTriggerFinding, LoadedTrigger, MatchConditions, MatchExpr, PathGlobs, PathSubject, ShadowedFile, TriggerEnvelope,
    TriggerConditionTrace, TriggerEvaluation, TriggerRegistry, TriggerSource, TriggerTrace,
    TriggerTraceEntry, claims_trigger_envelope, evaluate_registry, matched_triggers, parse_trigger_envelope,
    parse_trigger_envelope_from_str, trace_registry, scan, scan_with_embedded,
};
use simplified_schema::{
    ValidationStage, ValidationTimings,
};
use simplified_schema::{
    PositionMap, ValidatorCache,
};
use positions::ProblemPositions;

/// The runtime baseline, prepared from the embedded table: its typed form with
/// every import expanded, the shared definitions its references point into,
/// and the pair lowered to JSON Schema.
struct ShippedBaseline {
    schema: SchemaRoot,
    definitions: Arc<DefinitionTable>,
    json_schema: Arc<Value>,
}

/// Prepared and lowered once per process, on first use.
static BASELINE: std::sync::OnceLock<ShippedBaseline> = std::sync::OnceLock::new();

/// [`darkmatter_base_schema`]'s definition-free form, resolved on first use.
static INLINE_BASELINE: std::sync::OnceLock<SchemaRoot> = std::sync::OnceLock::new();

fn shipped_baseline() -> &'static ShippedBaseline {
    BASELINE.get_or_init(|| {
        let (schema, definitions) = catalog::GlobalCatalog::darkmatter()
            .baseline()
            .expect("Darkmatter's catalog declares `fm`");
        let json_schema = to_json_schema_with_definitions(&schema, &definitions)
            .unwrap_or_else(|error| panic!("the embedded baseline schema must lower: {error}"));
        ShippedBaseline { schema, definitions, json_schema: Arc::new(json_schema) }
    })
}

/// The runtime baseline with the shared definitions its references point
/// into: the form validation lowers ([`darkmatter_base_json_schema`]).
pub(crate) fn darkmatter_shared_baseline() -> (&'static SchemaRoot, &'static Arc<DefinitionTable>) {
    let baseline = shipped_baseline();
    (&baseline.schema, &baseline.definitions)
}

fn darkmatter_base_schema_ref() -> &'static SchemaRoot {
    INLINE_BASELINE.get_or_init(catalog::darkmatter_inline_baseline)
}

fn darkmatter_base_json_schema_arc() -> &'static Arc<Value> {
    &shipped_baseline().json_schema
}

/// Returns the Darkmatter baseline frontmatter schema as a [`SchemaRoot`].
///
/// The baseline is the resolved `fm` definition of Darkmatter's global
/// catalog, `darkmatter/schemas/darkmatter.yaml`, compiled into the library
/// with its partials ([`catalog::GlobalCatalog::darkmatter`]). Its imports,
/// such as `content_policy`'s Content Policy types, resolve against the other
/// compiled-in schema documents, never the disk.
///
/// The result carries no definition table: every acyclic named type is
/// expanded at each of its uses. The catalog declaration and the lowered
/// baseline ([`darkmatter_base_json_schema`]) store a structured type the
/// baseline uses at two or more sites once instead; both forms validate
/// alike. It is prepared at first call and cached.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::schemas::darkmatter_base_schema;
///
/// let schema = darkmatter_base_schema();
/// ```
///
/// ## Panics
///
/// Panics if the compiled-in baseline or a schema it imports cannot be
/// prepared. This is a library bug, not an author error.
pub fn darkmatter_base_schema() -> SchemaRoot {
    darkmatter_base_schema_ref().clone()
}

/// Returns the Darkmatter baseline frontmatter schema as a compiled Draft
/// 2020-12 JSON Schema value.
///
/// The result is the baseline lowered once and cached, with each structured
/// type it uses at two or more sites emitted once under its root `$defs`;
/// [`darkmatter_base_schema`] is the same baseline with those types expanded,
/// and it validates alike. Each call returns an independent owned value;
/// callers that only need to read the schema can use
/// [`darkmatter_base_json_schema_ref`] instead.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::schemas::darkmatter_base_json_schema;
///
/// let json = darkmatter_base_json_schema();
/// assert_eq!(json.get("type").and_then(|v| v.as_str()), Some("object"));
/// ```
///
/// ## Panics
///
/// Panics if the compiled-in baseline or a schema it imports cannot be
/// prepared and lowered. This is a library bug, not an author error.
pub fn darkmatter_base_json_schema() -> Value {
    darkmatter_base_json_schema_ref().clone()
}

/// Borrows the process-cached Darkmatter baseline JSON Schema.
///
/// This accessor avoids cloning the schema for read-only use. Call
/// [`darkmatter_base_json_schema`] when an independently owned, mutable value
/// is required.
///
/// ## Panics
///
/// Panics if the compiled-in baseline or a schema it imports cannot be
/// prepared and lowered. This is a library bug, not an author error.
pub fn darkmatter_base_json_schema_ref() -> &'static Value {
    darkmatter_base_json_schema_arc().as_ref()
}

/// Top-level entry point for the schemas subsystem.
///
/// Holds the request's file-resolution context, optional baseline schema
/// configuration, optional schema-trigger configuration, and the process-wide
/// validator cache so repeated validations against the same effective schema
/// reuse the compiled `jsonschema::Validator`.
///
/// [`DarkmatterSchemas::new`] is deterministic and **never** scans disk. Its
/// context resolves every file value and `$schema` reference; a document with
/// no path of its own is anchored at the context's `cwd`.
/// Trigger-schema discovery is opt-in via [`Self::with_trigger_discovery`] or
/// [`Self::with_trigger_registry`]. Implicit CWD-based discovery is forbidden —
/// validation behavior must never depend silently on the process working
/// directory.
#[derive(Clone)]
pub struct DarkmatterSchemas {
    baseline: Option<BaselineSchema>,
    cache: ValidatorCache,
    triggers: Option<triggers::TriggerRegistry>,
    /// Bundles [`Self::with_trigger_discovery`] appends after the roots.
    embedded_bundles: Vec<EmbeddedSchemaBundle>,
    file_resolution_context: biscuit_file::FileResolutionContext,
    /// The YAML document kinds [`Self::validate_yaml_document`] selects from;
    /// built per call when `None`.
    kind_registry: Option<Arc<KindRegistry>>,
    /// The session a template layer resolves through; without one a template
    /// is [`SchemaError::UnresolvedSlots`].
    slot_session: Option<Arc<slots::SlotSession>>,
    /// Use only a snapshot the session already pinned (a pass that must not
    /// read anything a slot names).
    slot_pins_only: bool,
}

use simplified_schema::prepared::{CompiledValidators, wrapped_root_arms};
use simplified_schema::validate::SHARED_VALIDATOR_CACHE;

/// Benchmarking seam: empties the process-wide schema caches so the next
/// validation does its schema work cold.
///
/// Cleared: the shared, conversion, completion-alternative, and
/// example-target validator caches; the conversion-plan, format-validator,
/// example-artifact, emitted-pattern, and named-type parse caches; the
/// trigger payload and embedded-trigger memos; the trigger merge, phase
/// derivation, and root-arm memos; the schema-slot resolution cache; and the calling thread's trigger-matcher regex cache. Compiled-in
/// constants (the shipped baseline, the global catalog) are kept.
///
/// Never call it from production code: it only discards work. Verdicts are
/// unchanged; a reset makes the next lookup of each cache a miss.
#[doc(hidden)]
pub fn reset_process_caches_for_bench() {
    simplified_schema::reset_process_caches();
    triggers::assemble::reset_process_caches();
    triggers::discovery::reset_process_caches();
    triggers::matcher::reset_thread_caches();
    DERIVATIONS.lock().expect("phase schema memo poisoned").clear();
    MERGES.lock().expect("trigger merge memo poisoned").clear();
    slots::cache::clear();
}

/// One template an effective schema would resolve
/// ([`DarkmatterSchemas::pending_templates_for`]).
#[derive(Debug, Clone)]
pub struct PendingTemplate {
    /// The template, as its layer's resolution met it.
    pub pending: simplified_schema::slots::PendingSchema,
    /// The slots of it the effective schema depends on: all of them, except
    /// a trigger slot under a property the document `$schema` redeclares.
    pub slots: Vec<simplified_schema::simplified::Slot>,
}

/// The top-level property names `root` declares (every arm's, for a union).
fn top_level_names(root: &SchemaRoot) -> Vec<String> {
    match root {
        SchemaRoot::Single(shape) => shape.properties.keys().cloned().collect(),
        SchemaRoot::Union(arms) => arms.iter().filter_map(|arm| match arm {
            simplified_schema::simplified::SchemaArm::Inline(shape) => Some(shape.properties.keys().cloned().collect::<Vec<_>>()),
            _ => None,
        }).flatten().collect(),
    }
}

/// What identifies a document's `$schema` template beyond the schema files
/// it read: the value as written (an inline template's whole text is here)
/// and the folder it resolves from.
fn document_template_identity(value: &Value, base_dir: &Path) -> String {
    format!(
        "document\0{}\0{}",
        biscuit_file::to_portable_string(base_dir),
        biscuit_hash::canonical_json(value)
    )
}

#[derive(Clone)]
struct BaselineSchema {
    /// Held as `Arc<Value>` so `effective_for` shares it by reference (a
    /// refcount bump) instead of deep-cloning the whole baseline schema on the
    /// common baseline-only / baseline+document paths (F29).
    json_schema: Arc<Value>,
    advisories: Vec<SchemaAdvisory>,
    declarations: BaselineDeclarations,
}

/// What a baseline declares, for reading it the way the document and trigger
/// layers are read rather than through its compiled JSON Schema alone.
#[derive(Clone)]
enum BaselineDeclarations {
    /// Darkmatter's own baseline, whose shape is compiled in and definition-free.
    Darkmatter,
    /// A caller's baseline. A raw JSON Schema one holds its conservative
    /// projection ([`RawDeclarations`](crate::markdown::compose::expression::RawDeclarations)).
    Custom {
        shape: Arc<SchemaShape>,
        definitions: Arc<DefinitionTable>,
    },
}

impl BaselineDeclarations {
    fn custom(simplified: Option<&SchemaRoot>, definitions: Arc<DefinitionTable>, json_schema: &Value) -> Self {
        let shape = match simplified {
            Some(SchemaRoot::Single(shape)) => shape.clone(),
            _ => declarations_of_json_schema(json_schema),
        };
        Self::Custom { shape: Arc::new(shape), definitions }
    }

    fn shape(&self) -> SchemaShape {
        match self {
            Self::Darkmatter => match darkmatter_base_schema_ref() {
                SchemaRoot::Single(shape) => shape.clone(),
                SchemaRoot::Union(_) => SchemaShape::default(),
            },
            Self::Custom { shape, .. } => shape.as_ref().clone(),
        }
    }

    fn definitions(&self) -> Option<&Arc<DefinitionTable>> {
        match self {
            Self::Darkmatter => None,
            Self::Custom { definitions, .. } => Some(definitions),
        }
    }
}

/// The declarations a layer with no SimplifiedSchema projection can prove.
fn declarations_of_json_schema(json_schema: &Value) -> SchemaShape {
    crate::markdown::compose::expression::RawDeclarations::from_json_schema(json_schema).to_schema_shape()
}

/// A resolved trigger payload layer, ready to merge into the effective schema.
#[derive(Clone)]
struct TriggerLayer {
    /// The trigger envelope (for origin attribution; a file is also a
    /// dependency).
    source: triggers::TriggerSource,
    /// The resolved payload JSON Schema (guaranteed simple-object), shared
    /// with the registry that resolved it.
    json_schema: Arc<Value>,
    /// The payload's SimplifiedSchema object shape, when it has one.
    shape: Option<Arc<SchemaShape>>,
    /// The cyclic named types `shape` reaches.
    definitions: Arc<DefinitionTable>,
    /// Payload dependency edges (referenced files + imports + examples).
    dependencies: Vec<PathBuf>,
    advisories: Vec<SchemaAdvisory>,
}

impl DarkmatterSchemas {
    /// Creates a new [`DarkmatterSchemas`] with no baseline, resolving file
    /// values through the request's `context`.
    pub fn new(context: biscuit_file::FileResolutionContext) -> Self {
        Self {
            baseline: None,
            cache: SHARED_VALIDATOR_CACHE.clone(),
            triggers: None,
            embedded_bundles: Vec::new(),
            file_resolution_context: context,
            kind_registry: None,
            slot_session: None,
            slot_pins_only: false,
        }
    }

    /// Resolves a template layer (a `$schema` or matched trigger payload
    /// holding `{{ … }}` slots) through `session` before it is layered.
    /// Without a session such a layer makes [`Self::effective_for`] fail with
    /// [`SchemaError::UnresolvedSlots`].
    #[must_use]
    pub fn with_slot_session(mut self, session: Arc<slots::SlotSession>) -> Self {
        self.slot_session = Some(session);
        self
    }

    /// Uses only the snapshots the session already pinned: a template no
    /// earlier pass resolved stays [`SchemaError::UnresolvedSlots`], so this
    /// pass reads nothing a slot names.
    #[must_use]
    pub(crate) fn with_slot_pins_only(mut self, pins_only: bool) -> Self {
        self.slot_pins_only = pins_only;
        self
    }

    /// `pending`, a template met while preparing a layer, resolved through
    /// the session; `unresolved` (the error that met it) without one.
    fn resolve_template(
        &self,
        unresolved: SchemaError,
        template: &str,
    ) -> Result<simplified_schema::resolve::ResolvedSchema, SchemaError> {
        let (Some(session), SchemaError::UnresolvedSlots(slots)) = (&self.slot_session, &unresolved) else {
            return Err(unresolved);
        };
        let Some(pending) = slots.pending.as_ref() else {
            return Err(unresolved);
        };
        session
            .resolve(pending, template, &self.file_resolution_context, self.slot_pins_only)
            .map(|resolved| simplified_schema::resolve::ResolvedSchema::clone(&resolved))
    }

    /// Records the launch-area anchor (typically the captured launch area).
    ///
    /// Per D2 the launch area is **not** a resolution input for a
    /// document-authored `format: darkmatter-file` value: those resolve
    /// document-first then against the repository root, never the launch
    /// area or the ambient CWD. This anchor is retained for structural parity
    /// with the validator-cache identity (it still participates in
    /// `ComposeOptions`/cache identity) but does not change the resolved path.
    #[must_use]
    pub fn with_file_ref_fallback_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cache = self.cache.with_file_ref_fallback_dir(dir);
        self
    }

    /// Records the contexts caller-supplied properties were authored in, so
    /// their `match()` globs are judged from there rather than from the
    /// document's folder.
    #[must_use]
    pub(crate) fn with_caller_origins(mut self, origins: validate::CallerOrigins) -> Self {
        self.cache = self.cache.with_caller_origins(origins);
        self
    }

    pub(crate) fn caller_origins(&self) -> &validate::CallerOrigins {
        self.cache.caller_origins()
    }

    /// Marks the top-level properties in `records` as caller-supplied: a
    /// bare or `./` `match()` pattern on one of them is judged from the
    /// record's origin (typically the launch directory) rather than from the
    /// document's folder, as composition judges it.
    #[must_use]
    pub fn with_caller_input_records(self, records: &crate::markdown::compose::CallerInputRecords) -> Self {
        self.with_caller_origins(validate::CallerOrigins::new(
            records
                .iter()
                .map(|(property, record)| (property.clone(), record.origin().clone())),
        ))
    }

    /// This instance resolving through `context` instead, keeping its
    /// baseline and trigger registry: one configuration validating documents
    /// that each carry their own context.
    #[must_use]
    pub fn with_file_resolution_context(
        mut self,
        context: biscuit_file::FileResolutionContext,
    ) -> Self {
        self.file_resolution_context = context;
        self
    }

    /// Whether this instance's baseline is Darkmatter's own compiled one.
    #[must_use]
    pub fn uses_darkmatter_baseline(&self) -> bool {
        matches!(self.baseline, Some(BaselineSchema { declarations: BaselineDeclarations::Darkmatter, .. }))
    }

    /// The context this instance resolves through.
    #[must_use]
    pub fn file_resolution_context(&self) -> &biscuit_file::FileResolutionContext {
        &self.file_resolution_context
    }

    /// Attaches a parsed SimplifiedSchema as the baseline.
    ///
    /// ## Errors
    ///
    /// Returns [`SchemaError::Convert`] if the schema cannot be converted to
    /// JSON Schema, or [`SchemaError::Baseline`] if the resulting JSON Schema
    /// is not a simple object schema (rooted at `"type": "object"` with only
    /// `properties` / `required`).
    pub fn with_baseline(self, schema: SchemaRoot) -> Result<Self, SchemaError> {
        self.with_baseline_and_definitions(&schema, &DefinitionTable::default())
    }

    /// [`Self::with_baseline`] for a schema whose stored named types are
    /// references into `definitions`.
    pub(crate) fn with_baseline_and_definitions(
        mut self,
        schema: &SchemaRoot,
        definitions: &DefinitionTable,
    ) -> Result<Self, SchemaError> {
        let json = to_json_schema_with_definitions(schema, definitions).map_err(|err| match err {
            SchemaError::UnresolvedSlots(_) => slots::codes::refuse_slots_in_baseline("the baseline", err),
            err => SchemaError::Baseline {
                message: "baseline could not be converted to JSON Schema".into(),
                source: Some(Box::new(err)),
            },
        })?;
        let declarations = BaselineDeclarations::custom(Some(schema), Arc::new(definitions.clone()), &json);
        self.set_baseline_json(json, Vec::new(), declarations)?;
        Ok(self)
    }

    /// Loads a baseline schema from a YAML or JSON file. Uses the same
    /// disambiguation rule as document `$schema` references, except that a
    /// global catalog ([`catalog::is_global_catalog`]) contributes its resolved
    /// `fm` definition, as Darkmatter's built-in baseline does.
    ///
    /// ## Errors
    ///
    /// Propagates [`SchemaError::Io`], the referenced-file format errors
    /// ([`SchemaError::NotASchemaDocument`], [`SchemaError::UnsupportedJsonDialect`],
    /// [`SchemaError::InvalidJsonSchema`]), or [`SchemaError::Baseline`]
    /// depending on the failure shape; a catalog
    /// that does not load is a [`SchemaError::Baseline`] whose message is the
    /// field-located [`catalog::CatalogError`].
    ///
    /// ## Notes
    ///
    /// One call reads each schema file once: the bytes read to recognize a
    /// catalog are the ones resolved, and a file several catalog globals
    /// import is read for the first of them only.
    pub fn with_baseline_from_file(self, path: impl AsRef<Path>) -> Result<Self, SchemaError> {
        simplified_schema::resolve::buffers::with_reads_retained(|| self.load_baseline_file(path.as_ref()))
    }

    fn load_baseline_file(mut self, path: &Path) -> Result<Self, SchemaError> {
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        // Read through the retaining scope, so resolving the file below reuses these bytes.
        let text = simplified_schema::resolve::buffers::read_to_string(path).ok();
        if let Some(text) = text.as_deref().filter(|text| catalog::is_global_catalog(text)) {
            let loaded = catalog::GlobalCatalog::from_text(
                path,
                text,
                catalog::DARKMATTER_GLOBALS,
                &self.file_resolution_context,
            )
            .map_err(|error| match error.reason {
                catalog::CatalogErrorReason::SlotsNotAllowed(ref unresolved) => SchemaError::Baseline {
                    message: error.to_string(),
                    source: Some(Box::new(SchemaError::UnresolvedSlots(unresolved.clone()))),
                },
                _ => SchemaError::Baseline { message: error.to_string(), source: None },
            })?;
            let (schema, definitions) = loaded.baseline().expect("Darkmatter's requirements include `fm`");
            let json = to_json_schema_with_definitions(&schema, &definitions)
                .map_err(|error| slots::codes::refuse_slots_in_baseline("the baseline", error))?;
            let declarations = BaselineDeclarations::custom(Some(&schema), definitions, &json);
            self.set_baseline_json(json, Vec::new(), declarations)?;
            return Ok(self);
        }
        // Drive resolution through the file-reference path so YAML/JSON
        // disambiguation matches `$schema` references.
        let yaml_value = serde_yaml_ng::Value::String(path.to_string_lossy().into_owned());
        let resolved = resolve::resolve_yaml_schema(&yaml_value, base, &self.file_resolution_context)
            .map_err(|err| match err {
                SchemaError::Io { .. } => err,
                SchemaError::NotASchemaDocument { .. }
                | SchemaError::UnsupportedJsonDialect { .. }
                | SchemaError::InvalidJsonSchema { .. } => err,
                SchemaError::UnresolvedSlots(_) => slots::codes::refuse_slots_in_baseline("the baseline", err),
                other => SchemaError::Baseline {
                    message: format!("could not load baseline from `{}`", path.display()),
                    source: Some(Box::new(other)),
                },
            })?;
        let declarations =
            BaselineDeclarations::custom(resolved.simplified.as_ref(), resolved.definitions, &resolved.json_schema);
        self.set_baseline_json(resolved.json_schema, resolved.advisories, declarations)?;
        Ok(self)
    }

    /// Attaches a raw JSON Schema as the baseline.
    ///
    /// ## Errors
    ///
    /// Returns [`SchemaError::Baseline`] when the value is not a simple
    /// object schema.
    pub fn with_baseline_json_schema(mut self, value: Value) -> Result<Self, SchemaError> {
        let declarations = BaselineDeclarations::custom(None, Arc::default(), &value);
        self.set_baseline_json(value, Vec::new(), declarations)?;
        Ok(self)
    }

    /// Attaches Darkmatter's built-in frontmatter schema as the baseline.
    ///
    /// ## Errors
    ///
    /// Returns [`SchemaError::Baseline`] if the built-in schema is not a
    /// simple object schema. This indicates a library defect.
    pub fn with_darkmatter_baseline_json_schema(mut self) -> Result<Self, SchemaError> {
        let json_schema = Arc::clone(darkmatter_base_json_schema_arc());
        simplified_schema::resolve::validate_baseline_schema(&json_schema)?;
        self.baseline = Some(BaselineSchema {
            json_schema,
            advisories: Vec::new(),
            declarations: BaselineDeclarations::Darkmatter,
        });
        Ok(self)
    }

    fn set_baseline_json(
        &mut self,
        value: Value,
        advisories: Vec<SchemaAdvisory>,
        declarations: BaselineDeclarations,
    ) -> Result<(), SchemaError> {
        simplified_schema::resolve::validate_baseline_schema(&value)?;
        self.baseline = Some(BaselineSchema {
            json_schema: Arc::new(value),
            advisories,
            declarations,
        });
        Ok(())
    }

    /// Enables schema-trigger discovery in the five [schema roots](roots) of
    /// this instance's context, which must be the checked document's own
    /// context.
    ///
    /// The scan is performed eagerly at configuration time and recovers per
    /// trigger: an invalid trigger is skipped and reported by
    /// [`Self::discovery_findings`] while every valid one is installed. The
    /// resulting [`triggers::TriggerRegistry`] is stored and consumed by
    /// [`Self::effective_for`], whose bare-name `$schema` lookup also
    /// searches its roots.
    ///
    /// [`DarkmatterSchemas::new`] never scans disk — discovery is opt-in via
    /// this method or [`Self::with_trigger_registry`].
    ///
    /// ## Errors
    ///
    /// Only discovery's infrastructure failures (see [`triggers::scan`]): an
    /// unreadable schema root or enumerated file, a case-fold collision, or
    /// an invalid embedded bundle structure.
    pub fn with_trigger_discovery(self) -> Result<Self, SchemaError> {
        let registry =
            triggers::scan_with_embedded(&self.file_resolution_context, &self.embedded_bundles)?;
        Ok(self.with_trigger_registry(registry))
    }

    /// Registers a host's embedded trigger bundle as a last-resort trigger
    /// location after the five schema roots.
    ///
    /// A bundle trigger applies only when no schema root holds a file of the
    /// same name; its `$schema` and imports resolve inside the bundle. When a
    /// registry is already installed (by [`Self::with_trigger_discovery`] or
    /// [`Self::with_trigger_registry`]) the bundle is appended to it now;
    /// either way a later [`Self::with_trigger_discovery`] appends it after
    /// its scan. [`Self::with_trigger_registry`] installs its registry as
    /// given, without registered bundles.
    ///
    /// ## Errors
    ///
    /// [`SchemaError::EmbeddedBundle`] (see
    /// [`triggers::TriggerRegistry::append_embedded_bundle`]); the instance is
    /// consumed. An invalid trigger in the bundle is a discovery finding, not
    /// an error.
    pub fn with_embedded_trigger_bundle(
        mut self,
        bundle: EmbeddedSchemaBundle,
    ) -> Result<Self, SchemaError> {
        bundle.check()?;
        if let Some(registry) = self.triggers.as_mut() {
            registry.append_embedded_bundle(bundle)?;
        }
        self.embedded_bundles.push(bundle);
        Ok(self)
    }

    /// Attaches a prebuilt trigger registry (e.g. one cached per boundary by
    /// DMLS, or constructed programmatically by a test).
    ///
    /// [`DarkmatterSchemas::new`] never scans disk — discovery is opt-in.
    /// A caller must explicitly ask for discovery (via
    /// [`Self::with_trigger_discovery`]) or supply a prebuilt registry.
    #[must_use]
    pub fn with_trigger_registry(mut self, registry: triggers::TriggerRegistry) -> Self {
        self.triggers = Some(registry);
        self
    }

    /// Returns the configured trigger registry, when present.
    pub fn trigger_registry(&self) -> Option<&triggers::TriggerRegistry> {
        self.triggers.as_ref()
    }

    /// The schema triggers discovery skipped as invalid, in discovery order;
    /// empty without a registry. They belong to the discovery result, so they
    /// are reported whether or not any effective schema applies.
    #[must_use]
    pub fn discovery_findings(&self) -> &[triggers::InvalidTriggerFinding] {
        self.triggers.as_ref().map_or(&[], |registry| registry.findings())
    }

    /// Builds the effective schema for a Markdown document.
    ///
    /// Reads the document's `$schema` frontmatter (if any), resolves it, and
    /// merges the layers in precedence order:
    ///
    /// 1. The caller-configured baseline (if any).
    /// 2. Matching schema-trigger payloads — first schema root first,
    ///    filename-lexicographic within a root, then embedded bundle triggers
    ///    (the registry's built-in order). Shadowing is applied before matching
    ///    (a shadowed file is never in the registry).
    /// 3. The document `$schema` (always wins on conflict).
    ///
    /// When `$schema` is absent, no baseline is configured, and no triggers
    /// match, returns `Ok(None)`.
    ///
    /// ## Errors
    ///
    /// Propagates [`SchemaError`] from parsing, resolution, conversion,
    /// validator construction, trigger-payload resolution, or cycle detection.
    pub fn effective_for(&self, source: &Markdown) -> Result<Option<EffectiveSchema>, SchemaError> {
        self.effective_for_with_override(source, None)
    }

    /// Builds the effective schema like [`Self::effective_for`], but
    /// `schema_override` — when `Some` — **replaces** the document's own
    /// `$schema` as the document schema layer (an operator override in the
    /// posture of `md schema validate --schema`). The override takes the same
    /// shapes a `$schema` frontmatter value takes: a string file reference or
    /// bare name, an inline mapping, or a root-union sequence, resolved
    /// against the document's own base directory and the configured trigger
    /// schema roots. Baseline and trigger layering, matching, and merge
    /// precedence are unchanged.
    ///
    /// ## Errors
    ///
    /// Propagates [`SchemaError`] from parsing, resolution, conversion,
    /// validator construction, trigger-payload resolution, or cycle detection.
    pub fn effective_for_with_override(
        &self,
        source: &Markdown,
        schema_override: Option<&Value>,
    ) -> Result<Option<EffectiveSchema>, SchemaError> {
        let _resolve = timings::stage(ValidationStage::Resolve);
        crate::note_work!(EffectivePreparation);
        let base_dir = base_dir_for(source, &self.file_resolution_context);
        let frontmatter = source.frontmatter().as_map();
        let schema_value = schema_override.or_else(|| frontmatter.get("$schema"));

        // Schema roots from the trigger registry feed bare-name $schema
        // resolution.
        let trigger_roots: &[PathBuf] = self
            .triggers
            .as_ref()
            .map(|reg| reg.roots.search_paths())
            .unwrap_or(&[]);

        // Resolve the document $schema with schema-root context.
        let resolved = match schema_value {
            Some(value) => {
                // The document must not directly reference a schema-trigger
                // file — triggers activate by placement and match, never by
                // reference. The loader refuses a trigger file as a schema
                // document, so its refusal is reported as that reference.
                // That check reads the bytes the resolution just read.
                Some(simplified_schema::resolve::buffers::with_reads_retained(|| {
                    let r = resolve::resolve_schema_with_roots(
                        value,
                        &base_dir,
                        trigger_roots,
                        &self.file_resolution_context,
                    )
                    .or_else(|error| self.resolve_template(error, &document_template_identity(value, &base_dir)))
                    .map_err(|error| match triggers::assemble::refused_trigger_file(&error) {
                        Some(path) => triggers::assemble::trigger_schema_referenced(&path),
                        None => error,
                    })?;
                    triggers::assemble::check_document_schema_not_trigger(&r.referenced_files)?;
                    Ok::<_, SchemaError>(r)
                })?)
            }
            None => None,
        };

        // Trigger matching + payload resolution.
        let trigger_layers = self.resolve_trigger_layers(source)?;

        // Build the merged schema in precedence order: baseline → triggers →
        // document (highest precedence wins). The lowest-precedence layer is the
        // borrowed merge base and is never deep-cloned; only the
        // higher-precedence layers are consumed (owned) by `merge_baseline`. When
        // there is a single layer, it is shared directly (an `Arc` refcount bump)
        // rather than materialized (F29).
        let doc_json: Option<Value> = resolved.as_ref().map(|r| r.json_schema.clone());
        let layers = SchemaLayers {
            document: doc_json.is_some(),
            triggers: trigger_layers.iter().map(|layer| layer.source.clone()).collect(),
            baseline: self.baseline.is_some(),
        };

        let merged_json: Arc<Value> = match (&self.baseline, trigger_layers.is_empty(), doc_json) {
            // Baseline only — share the Arc, no deep clone.
            (Some(base), true, None) => Arc::clone(&base.json_schema),
            (None, true, None) => return Ok(None),
            // A lone trigger payload is shared as resolved.
            (None, false, None) if trigger_layers.len() == 1 => Arc::clone(&trigger_layers[0].json_schema),
            (baseline, _, doc) => merged_trigger_schema(
                baseline.as_ref().map(|base| &base.json_schema),
                &trigger_layers,
                doc,
            )?,
        };

        // Keep eager-file validation on the same request-scoped candidate plan
        // as expression-side `file_exists` and `frontmatter` resolution.
        let validators =
            CompiledValidators::build(&merged_json, &self.cache, Some(&base_dir), &self.file_resolution_context)?;
        let origins = build_origin_map_with_triggers(
            resolved.as_ref(),
            &trigger_layers,
            self.baseline.is_some(),
            &merged_json,
        );
        let dependencies = build_dependencies(resolved.as_ref(), &trigger_layers);
        let advisories = build_advisories(
            self.baseline.as_ref(),
            &trigger_layers,
            resolved.as_ref(),
        );
        let definitions = resolved.as_ref().map(|r| Arc::clone(&r.definitions)).unwrap_or_default();
        let raw_document = resolved
            .as_ref()
            .filter(|document| document.simplified.is_none())
            .map(|document| SchemaRoot::Single(declarations_of_json_schema(&document.json_schema)));
        Ok(Some(EffectiveSchema {
            simplified: resolved.and_then(|r| r.simplified),
            raw_document,
            baseline_declarations: self.baseline.as_ref().map(|baseline| baseline.declarations.clone()),
            declared_trigger_shapes: trigger_layers
                .iter()
                .map(|layer| {
                    layer
                        .shape
                        .clone()
                        .unwrap_or_else(|| Arc::new(declarations_of_json_schema(&layer.json_schema)))
                })
                .collect(),
            definitions,
            trigger_shapes: trigger_layers.iter().filter_map(|layer| layer.shape.clone()).collect(),
            trigger_definitions: trigger_layers
                .iter()
                .filter(|layer| !layer.definitions.is_empty())
                .map(|layer| Arc::clone(&layer.definitions))
                .collect(),
            layers,
            json_schema: merged_json,
            origins,
            validators,
            base_dir: Some(base_dir),
            file_ref_fallback_dir: self.cache.file_ref_fallback_dir().map(Path::to_path_buf),
            file_resolution_context: self.file_resolution_context.clone(),
            caller_origins: self.cache.caller_origins().clone(),
            dependencies,
            advisories,
            conversion: OnceLock::new(),
            layered_definitions: OnceLock::new(),
            raw_declarations: OnceLock::new(),
        }))
    }

    /// The templates (schemas holding `{{ … }}` slots) the effective schema
    /// of `source` would have to resolve, for a pre-flight to collect the
    /// reads their slots will make: the document `$schema` layer, then each
    /// matched trigger payload that is a template, in layer order.
    ///
    /// Only the effective schema's slots count: a trigger slot under a
    /// top-level property the document `$schema` redeclares is left out of
    /// [`PendingTemplate::slots`], because the document's declaration
    /// replaces it. An unmatched trigger contributes nothing.
    ///
    /// Passive with respect to slots: schema files are read as validation
    /// reads them, but no slot is admitted or evaluated and nothing a slot
    /// names is read. The baseline and kind definitions refuse slots. A
    /// `$schema` that fails for any other reason yields nothing here;
    /// validation reports it.
    pub fn pending_templates_for(&self, source: &Markdown) -> Vec<PendingTemplate> {
        let base_dir = base_dir_for(source, &self.file_resolution_context);
        let trigger_roots: &[PathBuf] =
            self.triggers.as_ref().map(|registry| registry.roots.search_paths()).unwrap_or(&[]);
        let mut templates = Vec::new();
        // The top-level properties the document layer declares, which
        // replace a trigger's declaration of the same name.
        let mut redeclared: Vec<String> = Vec::new();
        if let Some(value) = source.frontmatter().as_map().get("$schema").cloned() {
            match resolve::resolve_schema_with_roots(&value, &base_dir, trigger_roots, &self.file_resolution_context) {
                Err(SchemaError::UnresolvedSlots(unresolved)) => {
                    if let Some(pending) = unresolved.pending {
                        if let Some((root, _)) = pending.template() {
                            redeclared.extend(top_level_names(root));
                        }
                        templates.push(PendingTemplate { slots: pending.slots().to_vec(), pending });
                    }
                }
                Ok(resolved) => {
                    if let Some(root) = &resolved.simplified {
                        redeclared.extend(top_level_names(root));
                    }
                }
                Err(_) => {}
            }
        }
        let Some(registry) = self.triggers.as_ref().filter(|registry| registry.payloads.len() == registry.triggers.len()) else {
            return templates;
        };
        let document = match source.source() {
            Some(ComposeSource::File(path)) => Some(path.as_path()),
            _ => None,
        };
        let fm_json = frontmatter_as_json(source);
        for (idx, eval) in triggers::assemble::evaluate_registry(registry, &fm_json, document).into_iter().enumerate() {
            let Some(pending) = registry.payloads[idx].pending.clone().filter(|_| eval.matched) else {
                continue;
            };
            let shadowed: Vec<simplified_schema::simplified::Slot> = match pending.template() {
                Some((SchemaRoot::Single(shape), definitions)) => shape
                    .properties
                    .iter()
                    .filter(|(name, _)| redeclared.iter().any(|declared| declared == *name))
                    .flat_map(|(_, def)| simplified_schema::slots::reachable_slots(def, definitions))
                    .collect(),
                _ => Vec::new(),
            };
            let slots = pending.slots().iter().filter(|slot| !shadowed.contains(slot)).cloned().collect();
            templates.push(PendingTemplate { pending, slots });
        }
        templates
    }

    /// Matches the configured trigger registry against the document and
    /// resolves each matching payload. Returns the ordered trigger layers
    /// (first schema root first). Empty when no registry is configured or no
    /// triggers match.
    fn resolve_trigger_layers(
        &self,
        source: &Markdown,
    ) -> Result<Vec<TriggerLayer>, SchemaError> {
        let Some(registry) = &self.triggers else {
            return Ok(Vec::new());
        };
        if registry.is_empty() {
            return Ok(Vec::new());
        }

        // `$path` judges the document's file; an in-memory source has none.
        let document = match source.source() {
            Some(ComposeSource::File(p)) => Some(p.as_path()),
            _ => None,
        };

        // Frontmatter snapshot for matching (strip the `$schema` control key).
        let fm_json = frontmatter_as_json(source);

        // Registries built by `scan` carry pre-resolved payloads; programmatically
        // constructed registries do not and fall back to on-demand resolution.
        let pre_resolved = registry.payloads.len() == registry.triggers.len();

        let evaluations = ValidationStage::Match.time(|| {
            let evaluations = triggers::assemble::evaluate_registry(registry, &fm_json, document);
            for eval in evaluations.iter().filter(|eval| eval.matched) {
                timings::matched_trigger(|| {
                    eval.trigger
                        .source
                        .file_name()
                        .map_or_else(|| eval.trigger.source.to_string(), str::to_owned)
                });
            }
            evaluations
        });
        let mut layers = Vec::with_capacity(
            evaluations.iter().filter(|eval| eval.matched).count(),
        );
        for (idx, eval) in evaluations.into_iter().enumerate() {
            // A matcher-only trigger (no `$schema`) layers nothing.
            if !eval.matched || eval.trigger.envelope.payload.is_none() {
                continue;
            }
            let resolved_payload;
            let mut payload = if pre_resolved {
                &registry.payloads[idx]
            } else {
                resolved_payload = triggers::assemble::resolve_trigger_payload(
                    eval.trigger,
                    registry.roots.search_paths(),
                    &self.file_resolution_context,
                )?;
                &resolved_payload
            };
            // A matched template payload resolves now, in this document's
            // slot session, and then passes the ordinary payload rules.
            let concrete;
            if let Some(pending) = &payload.pending {
                concrete = self.resolve_pending_payload(eval.trigger, pending)?;
                payload = &concrete;
            }
            let (json_schema, shape, definitions, dependencies, advisories) = (
                payload.json_schema.clone(),
                payload.shape.clone(),
                Arc::clone(&payload.definitions),
                payload.dependencies.clone(),
                payload.advisories.clone(),
            );
            layers.push(TriggerLayer {
                source: eval.trigger.source.clone(),
                json_schema,
                shape,
                definitions,
                dependencies,
                advisories,
            });
        }
        Ok(layers)
    }

    /// The matched trigger `trigger`'s template payload `pending`, resolved
    /// through the slot session and checked as a payload.
    ///
    /// ## Errors
    ///
    /// [`SchemaError::UnresolvedSlots`] without a session; the resolution's
    /// failure, positioned at the slot in the trigger's files; or a payload
    /// rule the resolved schema breaks.
    fn resolve_pending_payload(
        &self,
        trigger: &triggers::LoadedTrigger,
        pending: &simplified_schema::slots::PendingSchema,
    ) -> Result<triggers::assemble::ResolvedPayload, SchemaError> {
        let (TriggerSource::File(source), Some(payload)) = (&trigger.source, trigger.envelope.payload.as_ref()) else {
            return Err(SchemaError::UnresolvedSlots(Box::new(simplified_schema::UnresolvedSlots::new(
                pending.slots().to_vec(),
            ))));
        };
        let identity = format!(
            "trigger\0{}\0{}",
            biscuit_file::to_portable_string(source),
            serde_json::to_string(payload).unwrap_or_default()
        );
        let mut unresolved = simplified_schema::UnresolvedSlots::new(pending.slots().to_vec());
        unresolved.pending = Some(pending.clone());
        let resolved = self.resolve_template(SchemaError::UnresolvedSlots(Box::new(unresolved)), &identity)?;
        triggers::assemble::finish_file_payload(source, payload, resolved)
    }

    /// Validates a document's frontmatter, returning a [`ValidationReport`].
    ///
    /// When the document has no `$schema` and no baseline is configured,
    /// the report is vacuously valid with `schema = None`.
    ///
    /// ## Errors
    ///
    /// Returns [`SchemaError`] when schema resolution or validator
    /// construction fails. Validation problems (the schema compiled but the
    /// frontmatter didn't satisfy it) are reported via
    /// [`ValidationReport::problems`] rather than this error path.
    pub fn validate(&self, source: &Markdown) -> Result<ValidationReport, SchemaError> {
        self.validate_prepared(source, None).map(|(report, _)| report)
    }

    /// [`Self::validate`], also returning what preparing the effective schema
    /// found, from the same preparation.
    ///
    /// A caller that words its outcome by which schema layers applied (`md
    /// schema validate`'s success line) reads [`PreparationSummary::layers`]
    /// instead of preparing the schema a second time with
    /// [`Self::effective_for`].
    ///
    /// ## Errors
    ///
    /// See [`Self::validate`].
    pub fn validate_summarized(
        &self,
        source: &Markdown,
    ) -> Result<(ValidationReport, PreparationSummary), SchemaError> {
        self.validate_prepared(source, None)
    }

    /// [`Self::validate`], also returning the request's stage timings.
    ///
    /// The result is exactly what [`Self::validate`] returns; timings are
    /// returned on failure too, covering the work done before it stopped.
    /// Trigger discovery done by [`Self::with_trigger_discovery`] is not part
    /// of this call (see [`ValidationTimings`]).
    pub fn validate_timed(&self, source: &Markdown) -> (Result<ValidationReport, SchemaError>, ValidationTimings) {
        ValidationTimings::record(|| self.validate(source))
    }

    /// Validates a document's frontmatter at a runtime [`SchemaPhase`].
    ///
    /// The phase-aware twin of [`Self::validate`]: the same effective schema
    /// resolution, judged through
    /// [`EffectiveSchema::validate_for_phase_with_positions`].
    ///
    /// ## Errors
    ///
    /// See [`Self::validate`].
    pub fn validate_for_phase(
        &self,
        source: &Markdown,
        phase: SchemaPhase,
    ) -> Result<ValidationReport, SchemaError> {
        self.validate_prepared(source, Some(phase)).map(|(report, _)| report)
    }

    /// The one place a document's [`ValidationReport`] is produced: prepares
    /// the effective schema once and judges the frontmatter against it,
    /// unphased or at `phase`. Every document-level validation entry point
    /// delegates here.
    ///
    /// Source positions are built only when the report holds a problem, once,
    /// and shared by all of its problems ([`positions::ProblemPositions`]).
    fn validate_prepared(
        &self,
        source: &Markdown,
        phase: Option<SchemaPhase>,
    ) -> Result<(ValidationReport, PreparationSummary), SchemaError> {
        let frontmatter_value = frontmatter_as_json(source);
        let positions = ProblemPositions::deferred(source);
        let effective = self.effective_for(source)?;
        let summary = PreparationSummary {
            layers: effective.as_ref().map(|effective| effective.layers.clone()).unwrap_or_default(),
            dependencies: effective.as_ref().map(|effective| effective.dependencies.clone()).unwrap_or_default(),
        };
        let mut report = match (effective, phase) {
            (Some(effective), None) => {
                effective.validate_converted(&frontmatter_value, &frontmatter_value, &positions)
            }
            (Some(effective), Some(phase)) => {
                effective.validate_for_phase_located(&frontmatter_value, &positions, phase)?
            }
            (None, _) => ValidationReport::vacuous(),
        };
        report.trigger_findings = self.discovery_findings().to_vec();
        Ok((report, summary))
    }

    /// Returns a shared reference to the underlying validator cache. Useful
    /// for sharing the cache between subsystems (e.g. a long-running CLI).
    pub fn cache(&self) -> &ValidatorCache {
        &self.cache
    }

    /// Detects a [`SchemaRoot`] from one or more documents through this
    /// instance's context, which must admit every source (see
    /// [`detect::detect_from_document`]); use
    /// [`detect::detect_schema_with_contexts`] for documents that each carry
    /// their own context. See [`detect::detect_schema`] for the algorithm.
    pub fn detect(&self, sources: &[&Markdown], opts: DetectOptions) -> SchemaRoot {
        detect::detect_schema(sources, opts, &self.file_resolution_context)
    }
}

fn schema_declares_property(root: &Value, schema: &Value, name: &str, depth: usize) -> bool {
    // Bounds a `$ref` cycle; real schemas nest a handful of levels.
    const MAX_DEPTH: usize = 32;
    let Some(object) = schema.as_object() else {
        return false;
    };
    if depth > MAX_DEPTH {
        return false;
    }
    if object
        .get("properties")
        .and_then(Value::as_object)
        .is_some_and(|properties| properties.contains_key(name))
    {
        return true;
    }
    if object
        .get("patternProperties")
        .and_then(Value::as_object)
        .is_some_and(|patterns| {
            patterns
                .keys()
                .any(|pattern| regex::Regex::new(pattern).is_ok_and(|re| re.is_match(name)))
        })
    {
        return true;
    }
    let arms = ["allOf", "anyOf", "oneOf"]
        .into_iter()
        .filter_map(|key| object.get(key).and_then(Value::as_array))
        .flatten()
        .chain(["if", "then", "else"].into_iter().filter_map(|key| object.get(key)));
    // The compiled schema already resolved every reference, so a chain that
    // fails here declares nothing rather than failing the query.
    let referenced = scope::SchemaScope::new(root)
        .chain(schema, "")
        .map(|hops| hops.into_iter().skip(1).collect::<Vec<_>>())
        .unwrap_or_default();
    arms.chain(referenced)
        .any(|arm| schema_declares_property(root, arm, name, depth + 1))
}

/// The fully-resolved schema for a document.
#[derive(Clone)]
pub struct EffectiveSchema {
    /// The SimplifiedSchema projection of the **authored** document schema
    /// (`$schema`, or the operator override) alone. `None` when there is no
    /// document schema, when it was a raw JSON Schema, or when a root union
    /// mixed SimplifiedSchema and JSON Schema arms.
    ///
    /// Baseline and trigger layers are not in it: consumers that mean "the
    /// author's own parameters" (Claudine's prompt parameters, expression
    /// property names, optional-binding materialization) read this field. The
    /// editor's full property shape, with matching trigger payloads layered
    /// in, is [`Self::property_shape`].
    pub simplified: Option<SchemaRoot>,
    /// The projection of an authored document schema [`Self::simplified`] cannot
    /// represent; only [`Self::declared_property_shape`] reads it.
    raw_document: Option<SchemaRoot>,
    /// The baseline this schema was actually assembled with; `None` when it has
    /// none.
    baseline_declarations: Option<BaselineDeclarations>,
    /// Every matching trigger payload in layering order, a raw JSON Schema one
    /// as its projection. [`Self::trigger_shapes`] leaves those out.
    declared_trigger_shapes: Vec<Arc<SchemaShape>>,
    /// The cyclic named types [`Self::simplified`] reaches through
    /// [`TypeExpr::Ref`]; empty when nothing recurses.
    definitions: Arc<DefinitionTable>,
    /// The object shapes of the matching trigger payloads that have a
    /// SimplifiedSchema projection, in layering order (a later shape wins a
    /// shared top-level property, as it does in [`Self::json_schema`]).
    trigger_shapes: Vec<Arc<SchemaShape>>,
    /// The non-empty definition tables of the matching trigger payloads,
    /// searched by [`Self::definition`] after [`Self::definitions`].
    trigger_definitions: Vec<Arc<DefinitionTable>>,
    /// The layers merged into [`Self::json_schema`].
    layers: SchemaLayers,
    /// The final Draft 2020-12 JSON Schema used by the validator.
    ///
    /// Held as `Arc<Value>` so the common baseline-only effective schema is
    /// shared from the configured baseline by reference rather than deep-cloned
    /// per resolution (F29). Derefs to `&Value` transparently at read sites.
    pub json_schema: Arc<Value>,
    /// Per-top-level-property origins (document vs baseline vs referenced
    /// file), so diagnostics can point `relatedInformation` at the schema
    /// source (R-5 Priority 2). Empty for root-union schemas, whose per-arm
    /// property provenance is not modelled in v1.
    pub origins: SchemaOriginMap,
    validators: CompiledValidators,
    /// Prompt document directory used to reproduce the validator's
    /// document-first, then repository-relative candidate plan in diagnostics.
    base_dir: Option<PathBuf>,
    /// Captured launch-area metadata retained for file-reference diagnostics.
    file_ref_fallback_dir: Option<PathBuf>,
    /// The request's context, used by eager-file normalization.
    file_resolution_context: biscuit_file::FileResolutionContext,
    /// Caller-supplied properties and their origins, for the `match()`
    /// judgment of a phase's rebuilt validators.
    caller_origins: validate::CallerOrigins,
    /// Resolved paths of the files this schema depends on: the sorted,
    /// deduplicated union of the document `$schema`'s `Name@file`/`@this` imports
    /// (Feature B), its `example(...)` artifacts (Feature A), and the referenced
    /// schema files themselves (`$schema: ./schema.yaml` and each root-union
    /// string arm). Empty when the document has an inline `$schema` mapping with
    /// no imports/examples, or no `$schema` at all.
    ///
    /// Contract: these are dependency edges of the effective schema — a change to
    /// any listed file's *content* (with the document text and schema config
    /// otherwise unchanged) must invalidate a cached [`EffectiveSchema`]. The DMLS
    /// overlay cache content-hashes each entry to honor this.
    dependencies: Vec<PathBuf>,
    /// Non-fatal findings produced while resolving the effective schema's
    /// referenced files, sorted and deduplicated by semantic kind and path.
    advisories: Vec<SchemaAdvisory>,
    /// The conversion plan for [`Self::json_schema`], found on first use.
    conversion: OnceLock<Arc<simplified_schema::coerce::ConversionPlan>>,
    /// [`Self::layered_definitions`], merged on first use.
    layered_definitions: OnceLock<Arc<DefinitionTable>>,
    /// [`Self::raw_declarations`], projected on first use.
    raw_declarations: OnceLock<Arc<crate::markdown::compose::expression::RawDeclarations>>,
}

impl EffectiveSchema {
    /// Builds the conversion plan validation uses now, so a prepared schema
    /// judges its first document without building it.
    pub(crate) fn prepare_conversion(&self) {
        self.conversion();
    }

    /// The conversion plan for [`Self::json_schema`].
    fn conversion(&self) -> &simplified_schema::coerce::ConversionPlan {
        self.conversion.get_or_init(|| {
            simplified_schema::coerce::ConversionPlan::for_hashed(SHARED_VALIDATOR_CACHE.shared_schema_hash(&self.json_schema))
        })
    }

    /// Converts `instance` against [`Self::json_schema`] as
    /// [`simplified_schema::coerce::coerce_frontmatter_with_pending`] does, reusing this
    /// schema's conversion plan.
    pub fn coerce_frontmatter_with_pending(
        &self,
        instance: &Value,
        composition_pending: &HashSet<String>,
    ) -> simplified_schema::coerce::CoercionOutcome {
        if self.accepts_as_is(instance) {
            return simplified_schema::coerce::CoercionOutcome {
                value: instance.clone(),
                changed: false,
            };
        }
        simplified_schema::coerce::coerce_with_plan(self.conversion(), &self.json_schema, instance, composition_pending)
    }

    /// Whether this schema's own compiled validators accept `instance`
    /// unchanged: such a value is kept, so conversion needs nothing else.
    fn accepts_as_is(&self, instance: &Value) -> bool {
        self.validators.accepts(instance)
    }

    /// Whether this schema declares the top-level frontmatter property `name`:
    /// a `properties` key, a matching `patternProperties` pattern, or either
    /// of those in any root-union, `allOf`, or `if`/`then`/`else` arm
    /// (baseline and trigger layers are already merged in). Declaring is not
    /// requiring; an optional property is declared.
    pub fn declares_top_level_property(&self, name: &str) -> bool {
        schema_declares_property(&self.json_schema, &self.json_schema, name, 0)
    }

    /// The cyclic named types [`Self::simplified`] reaches through
    /// [`TypeExpr::Ref`], each lowered once under the schema's root `$defs`.
    /// Empty when nothing recurses.
    pub fn definitions(&self) -> &Arc<DefinitionTable> {
        &self.definitions
    }

    /// The definition a [`TypeExpr::Ref`] in [`Self::simplified`], one of
    /// [`Self::trigger_shapes`], or a custom baseline names. A [`DefKey`] identifies one
    /// definition wherever it is reached, so one lookup serves every layer.
    ///
    /// Look a reference up here, not by its `$defs` key: each table's keys
    /// are those of its own lowering, and a trigger payload's keys may have
    /// been re-keyed when its JSON merged under the document's.
    pub fn definition(&self, key: &DefKey) -> Option<&Definition> {
        std::iter::once(&self.definitions)
            .chain(&self.trigger_definitions)
            .chain(self.baseline_definitions())
            .find_map(|table| table.get(key))
    }

    /// One table holding every definition [`Self::definition`] finds: the
    /// document's, then each trigger layer's and a custom baseline's it lacks. A reference from any
    /// layer resolves in it to the body [`Self::definition`] returns, so the
    /// expression type layer looks references up here. Merged on first use;
    /// the document's own table when no other layer adds one.
    pub fn layered_definitions(&self) -> &Arc<DefinitionTable> {
        self.layered_definitions.get_or_init(|| {
            let baseline = self.baseline_definitions().filter(|table| !table.is_empty());
            if self.trigger_definitions.is_empty() && baseline.is_none() {
                return Arc::clone(&self.definitions);
            }
            let mut merged = self.definitions.as_ref().clone();
            for table in self.trigger_definitions.iter().chain(baseline) {
                merged.absorb(table);
            }
            Arc::new(merged)
        })
    }

    fn baseline_definitions(&self) -> Option<&Arc<DefinitionTable>> {
        self.baseline_declarations.as_ref().and_then(BaselineDeclarations::definitions)
    }

    /// The conservative projection of [`Self::json_schema`], the merged
    /// schema of every layer: the declared layer's source for the baseline
    /// and for layers no SimplifiedSchema represents. Projected on first use.
    pub fn raw_declarations(&self) -> &crate::markdown::compose::expression::RawDeclarations {
        self.raw_declarations.get_or_init(|| {
            Arc::new(crate::markdown::compose::expression::RawDeclarations::from_json_schema(&self.json_schema))
        })
    }

    /// The files this schema depends on (imports + examples + the referenced
    /// schema files themselves), as resolved absolute/canonical paths, sorted and
    /// deduplicated. Empty when the document has an inline `$schema` mapping with
    /// no imports/examples, or no `$schema` at all.
    ///
    /// A change to any of these files' content invalidates this effective schema;
    /// consumers that cache an [`EffectiveSchema`] keyed on document text must
    /// also track these paths' content to stay correct.
    pub fn dependencies(&self) -> &[PathBuf] {
        &self.dependencies
    }

    /// The layers merged into this schema: the document's own schema, the
    /// matching triggers, and the baseline.
    #[must_use]
    pub fn layers(&self) -> &SchemaLayers {
        &self.layers
    }

    /// Non-fatal findings discovered while resolving this effective schema.
    pub fn advisories(&self) -> &[SchemaAdvisory] {
        &self.advisories
    }

    /// The object shapes of the matching trigger payloads, in layering order.
    /// A raw JSON Schema payload has no shape and is absent here, though it
    /// still validates through [`Self::json_schema`].
    pub fn trigger_shapes(&self) -> &[Arc<SchemaShape>] {
        &self.trigger_shapes
    }

    /// The top-level property shape an editor projects for this document: the
    /// Darkmatter base properties, then each `extension_shapes` entry, then
    /// the matching trigger payloads, then the authored document schema.
    ///
    /// A later layer replaces an earlier layer's whole definition of a shared
    /// top-level property — the per-property precedence validation applies
    /// when it merges the same layers into [`Self::json_schema`], so an
    /// authored redeclaration wins over a trigger's. Root unions and
    /// `frontmatter` behave as in [`effective_property_shape`].
    ///
    /// The base is always the compiled baseline, whatever this schema was
    /// assembled with: this is the editor's projection. A request that selects its
    /// baseline reads [`Self::declared_property_shape`].
    pub fn property_shape(&self, extension_shapes: &[SchemaShape], frontmatter: &Value) -> SchemaShape {
        frontmatter_shape::layered_property_shape(
            extension_shapes,
            &self.trigger_shapes,
            self.simplified.as_ref(),
            frontmatter,
        )
    }

    /// The top-level properties this schema itself declares, in the layers
    /// [`Self::json_schema`] merges: the baseline it was assembled with (custom,
    /// compiled, or none), the matching triggers, then the document.
    ///
    /// A layer with no SimplifiedSchema projection contributes what its JSON Schema
    /// proves, so it still replaces a lower layer's declaration of a shared key.
    /// Root unions and `frontmatter` behave as in [`effective_property_shape`].
    pub fn declared_property_shape(&self, frontmatter: &Value) -> SchemaShape {
        let base = self.baseline_declarations.as_ref().map_or_else(SchemaShape::default, BaselineDeclarations::shape);
        simplified_schema::layered_property_shape(
            base,
            &[],
            &self.declared_trigger_shapes,
            self.simplified.as_ref().or(self.raw_document.as_ref()),
            frontmatter,
        )
    }

    /// Validates a frontmatter JSON value against this schema. Equivalent to
    /// [`Self::validate_with_positions`] with an empty position map (problems
    /// will carry no line/column information).
    ///
    /// Read-only: the caller's `frontmatter` is never mutated. To rewrite
    /// eager-`file` values to their resolved document-relative paths, call
    /// [`Self::normalize_frontmatter`] explicitly.
    pub fn validate(&self, frontmatter: &Value) -> ValidationReport {
        self.validate_with_positions(frontmatter, &PositionMap::new())
    }

    /// Validates a frontmatter JSON value against this schema, annotating
    /// problems with line/column information drawn from `positions` (see
    /// [`validate::build_position_map`]). For root unions, problems are
    /// attributed to the closest-matching arm and carry the corresponding
    /// `arm_index`.
    ///
    /// The instance is converted against the schema by the conversion core
    /// (see [`simplified_schema::coerce::coerce_frontmatter`]) on a working copy before
    /// validation, so the report reflects post-conversion validity. When some
    /// value cannot convert, every other value that converts on its own is
    /// still converted in the working copy, so the report names only the
    /// values that fail, each with the core's reason. No input is mutated —
    /// eager-`file` values are left in their raw caller-supplied form. To
    /// rewrite them to their resolved document-relative paths, call
    /// [`Self::normalize_frontmatter`] explicitly on an already-valid
    /// instance.
    ///
    /// ## Nested templates
    ///
    /// A `{{ … }}` template below a top-level key, at a place the schema
    /// marks `deferred` (`success: [{pr_list: "{{ fm.count }}"}]` against
    /// `pr_list: number(deferred)`), is supplied only when it is rendered, so
    /// it is accepted without judging its value; nothing is evaluated. Its
    /// literal siblings and the shape of the collection holding it are still
    /// judged: an unknown key, an extra or missing tuple element, or an
    /// invalid literal beside it is reported. A template at an unmarked place,
    /// and malformed template syntax anywhere, is judged as written. A whole
    /// top-level template value is not deferred here (see
    /// [`Self::validate_with_options`]).
    pub fn validate_with_positions(
        &self,
        frontmatter: &Value,
        positions: &PositionMap,
    ) -> ValidationReport {
        self.validate_converted(frontmatter, frontmatter, &ProblemPositions::Built(positions))
    }

    /// [`Self::validate_with_positions`] of `frontmatter`, whose templates
    /// are found in `stored`, its form before literal tokens were decoded,
    /// so decoded literal text is never taken for a template.
    fn validate_converted(&self, frontmatter: &Value, stored: &Value, positions: &ProblemPositions<'_>) -> ValidationReport {
        let _validate = timings::stage(ValidationStage::Validate);
        if self.accepts_as_is(frontmatter) {
            return self.validate_instance(frontmatter, frontmatter, stored, positions, &simplified_schema::coerce::NestedTemplates::default());
        }
        let templates = self.nested_templates(stored);
        let working_copy = simplified_schema::coerce::validation_copy_with_plan(self.conversion(), &self.json_schema, frontmatter, &templates);
        self.validate_instance(&working_copy, frontmatter, stored, positions, &templates)
    }

    /// The effective schema of a whole YAML document: `resolved` alone, with
    /// no baseline and no trigger layer, judging file values from `base_dir`
    /// in `schemas`' context and validator cache.
    pub(crate) fn for_yaml_document(
        resolved: simplified_schema::resolve::ResolvedSchema,
        base_dir: PathBuf,
        schemas: &DarkmatterSchemas,
    ) -> Result<Self, SchemaError> {
        let json_schema = Arc::new(resolved.json_schema.clone());
        let validators =
            CompiledValidators::build(&json_schema, &schemas.cache, Some(&base_dir), &schemas.file_resolution_context)?;
        let origins = build_origin_map_with_triggers(Some(&resolved), &[], false, &json_schema);
        let dependencies = build_dependencies(Some(&resolved), &[]);
        let advisories = resolved.advisories.clone();
        let raw_document =
            resolved.simplified.is_none().then(|| SchemaRoot::Single(declarations_of_json_schema(&resolved.json_schema)));
        Ok(Self {
            definitions: Arc::clone(&resolved.definitions),
            simplified: resolved.simplified,
            raw_document,
            baseline_declarations: None,
            declared_trigger_shapes: Vec::new(),
            trigger_shapes: Vec::new(),
            trigger_definitions: Vec::new(),
            layers: SchemaLayers { document: true, ..SchemaLayers::default() },
            json_schema,
            origins,
            validators,
            base_dir: Some(base_dir),
            file_ref_fallback_dir: schemas.cache.file_ref_fallback_dir().map(Path::to_path_buf),
            file_resolution_context: schemas.file_resolution_context.clone(),
            caller_origins: schemas.cache.caller_origins().clone(),
            dependencies,
            advisories,
            conversion: OnceLock::new(),
            layered_definitions: OnceLock::new(),
            raw_declarations: OnceLock::new(),
        })
    }

    /// Validates `instance` as data: softly, on a converted copy, with no
    /// template deferred, so a whole-value or nested `{{ … }}` / `$(…)`
    /// string is judged as the text it is. `positions` locates problems.
    pub(crate) fn validate_data(&self, instance: &Value, positions: &PositionMap) -> ValidationReport {
        let _validate = timings::stage(ValidationStage::Validate);
        let templates = simplified_schema::coerce::NestedTemplates::default();
        let positions = ProblemPositions::Built(positions);
        let working_copy = self.data_copy(instance);
        self.validate_instance(&working_copy, instance, instance, &positions, &templates)
    }

    /// The copy of `instance` [`Self::validate_data`] judges: `instance`
    /// itself when it is accepted as written, the softly converted copy
    /// otherwise. A host that reads a document's meaning after this schema
    /// accepted it (an editor reading a `kind: schema` document with
    /// [`simplified_schema::parse_selected_standalone_schema_document`])
    /// reads it from this copy, as the kind checks do.
    pub fn data_copy<'a>(&self, instance: &'a Value) -> std::borrow::Cow<'a, Value> {
        if self.accepts_as_is(instance) {
            return std::borrow::Cow::Borrowed(instance);
        }
        let templates = simplified_schema::coerce::NestedTemplates::default();
        std::borrow::Cow::Owned(simplified_schema::coerce::validation_copy_with_plan(
            self.conversion(),
            &self.json_schema,
            instance,
            &templates,
        ))
    }

    /// Validates a working frontmatter instance at a runtime schema phase.
    ///
    /// SimplifiedSchema phase rules are projected from the already-resolved
    /// schema: `eager` controls when a present value is validated, while
    /// `required` controls whether it must be present. Raw JSON Schema retains
    /// its authored behavior in both phases. The caller's instance and the
    /// effective schema are not mutated.
    ///
    /// ## Errors
    ///
    /// Returns [`SchemaError`] when the phase projection cannot be converted,
    /// merged, or compiled into a validator.
    pub fn validate_for_phase(
        &self,
        frontmatter: &Value,
        phase: SchemaPhase,
    ) -> Result<ValidationReport, SchemaError> {
        self.validate_for_phase_with_positions(frontmatter, &PositionMap::new(), phase)
    }

    /// Phase-aware validation with source positions attached to problems.
    pub fn validate_for_phase_with_positions(
        &self,
        frontmatter: &Value,
        positions: &PositionMap,
        phase: SchemaPhase,
    ) -> Result<ValidationReport, SchemaError> {
        self.validate_for_phase_located(frontmatter, &ProblemPositions::Built(positions), phase)
    }

    /// [`Self::validate_for_phase_with_positions`] locating its problems with
    /// `positions`.
    fn validate_for_phase_located(
        &self,
        frontmatter: &Value,
        positions: &ProblemPositions<'_>,
        phase: SchemaPhase,
    ) -> Result<ValidationReport, SchemaError> {
        let projected = self.phase_projection(phase)?;
        Ok(match phase {
            SchemaPhase::Launch => projected.validate_converted(frontmatter, frontmatter, positions),
            // Raw, as `validate_raw` is, but nested templates at `deferred`
            // places are deferred as in every phase.
            SchemaPhase::Completion => {
                let _validate = timings::stage(ValidationStage::Validate);
                projected.validate_instance(frontmatter, frontmatter, frontmatter, positions, &projected.nested_templates(frontmatter))
            }
        })
    }

    /// Validates `stored`, a frontmatter value as loaded with its literal
    /// tokens still encoded, unphased like [`Self::validate_with_positions`]
    /// or at `phase` like [`Self::validate_for_phase_with_positions`].
    ///
    /// Each token is judged by the text it holds, and that text is data: it
    /// is never deferred as a `{{ … }}` template, whether nested at a
    /// `deferred` place or in a generated lifecycle verb's operand, so a
    /// token holding `{{ fm.count }}` fails a number parameter as `three`
    /// does. A malformed token leaves the value raw; composition reports it.
    /// [`Self::validate_with_options`] judges its instance this way too.
    ///
    /// ## Errors
    ///
    /// See [`Self::validate_for_phase_with_positions`]; never fails without
    /// a phase.
    pub fn validate_stored(
        &self,
        stored: &Value,
        positions: &PositionMap,
        phase: Option<SchemaPhase>,
    ) -> Result<ValidationReport, SchemaError> {
        let projected;
        let schema = match phase {
            Some(phase) => {
                projected = self.phase_projection(phase)?;
                &projected
            }
            None => self,
        };
        Ok(schema.judge_stored(stored, &ProblemPositions::Built(positions), phase == Some(SchemaPhase::Completion)))
    }

    /// [`Self::validate_stored`] against this schema as it is: converted
    /// first, or raw (`completion`) as the completion phase judges.
    fn judge_stored(&self, stored: &Value, positions: &ProblemPositions<'_>, completion: bool) -> ValidationReport {
        let decoded = crate::markdown::literal_token::decode_literal_tokens(stored).ok();
        let decoded = decoded.as_ref().unwrap_or(stored);
        if completion {
            let _validate = timings::stage(ValidationStage::Validate);
            self.validate_instance(decoded, decoded, stored, positions, &self.nested_templates(stored))
        } else {
            self.validate_converted(decoded, stored, positions)
        }
    }

    /// This schema projected to `phase`'s presence rules.
    fn phase_projection(&self, phase: SchemaPhase) -> Result<Self, SchemaError> {
        let _resolve = timings::stage(ValidationStage::Resolve);
        let (projected, definitions, projected_document) = match &self.simplified {
            Some(simplified) => {
                let projected = phase::project(simplified, phase);
                let definitions = phase::project_definitions(&self.definitions, phase);
                let projected_document = simplified::to_json_schema_with_definitions(&projected, &definitions)?;
                (Some(projected), Arc::new(definitions), Some(projected_document))
            }
            None => (None, Arc::clone(&self.definitions), None),
        };
        let json_schema = phase_json_schema(&self.json_schema, projected_document, phase)?;
        let mut cache = SHARED_VALIDATOR_CACHE.clone().with_caller_origins(self.caller_origins.clone());
        if let Some(fallback) = self.file_ref_fallback_dir.as_deref() {
            cache = cache.with_file_ref_fallback_dir(fallback);
        }
        let validators =
            CompiledValidators::build(&json_schema, &cache, self.base_dir.as_deref(), &self.file_resolution_context)?;
        let projected = Self {
            simplified: projected,
            raw_document: self.raw_document.clone(),
            baseline_declarations: self.baseline_declarations.clone(),
            declared_trigger_shapes: self.declared_trigger_shapes.clone(),
            definitions,
            trigger_shapes: self.trigger_shapes.clone(),
            trigger_definitions: self.trigger_definitions.clone(),
            layers: self.layers.clone(),
            json_schema,
            origins: self.origins.clone(),
            validators,
            base_dir: self.base_dir.clone(),
            file_ref_fallback_dir: self.file_ref_fallback_dir.clone(),
            file_resolution_context: self.file_resolution_context.clone(),
            caller_origins: self.caller_origins.clone(),
            dependencies: self.dependencies.clone(),
            advisories: self.advisories.clone(),
            conversion: OnceLock::new(),
            layered_definitions: OnceLock::new(),
            raw_declarations: OnceLock::new(),
        };
        Ok(projected)
    }

    /// Validates a frontmatter JSON value against this schema **without any
    /// type coercion** — the raw repair-analysis companion to
    /// [`Self::validate_with_positions`]. Equivalent to
    /// [`Self::validate_raw_with_positions`] with an empty position map.
    ///
    /// Coercion hides exactly the mismatches format-preserving repair exists
    /// for (e.g. the number `1.2` coerces to the string `"1.2"`, silently
    /// losing the authored `1.20`), so repair analysis must validate the
    /// instance as parsed. Read-only: the caller's value is never mutated.
    pub fn validate_raw(&self, frontmatter: &Value) -> ValidationReport {
        self.validate_raw_with_positions(frontmatter, &PositionMap::new())
    }

    /// Validates like [`Self::validate_raw`], annotating problems with
    /// line/column information drawn from `positions`.
    pub fn validate_raw_with_positions(
        &self,
        frontmatter: &Value,
        positions: &PositionMap,
    ) -> ValidationReport {
        let _validate = timings::stage(ValidationStage::Validate);
        self.validate_instance(
            frontmatter,
            frontmatter,
            frontmatter,
            &ProblemPositions::Built(positions),
            &simplified_schema::coerce::NestedTemplates::default(),
        )
    }

    /// The nested templates of `authored` that this schema marks `deferred`
    /// ([`coerce::nested_templates`]).
    fn nested_templates(&self, authored: &Value) -> simplified_schema::coerce::NestedTemplates {
        coerce::nested_templates(self.conversion(), &self.json_schema, authored)
    }

    /// Shared validation body over an already-prepared instance. Callers
    /// choose the preparation: [`Self::validate_with_positions`] passes a
    /// coerced working copy, the `validate_raw` pair passes the instance as
    /// parsed. Problems owed to one of `templates` are left out as they are
    /// collected.
    ///
    /// Every value the schema types as a generated lifecycle verb's operands
    /// is bound through that verb's runtime registration, whose verdict
    /// decides its literal operands ([`bound_operands`]): the operands of an
    /// accepted call are set aside like templates, and a rejected call is a
    /// problem even where the types alone accept it. Calls are bound from
    /// `authored`, the instance before conversion, since a registration
    /// judges the value as written (`length` refuses `true`, which the
    /// working copy holds as `"true"`). A malformed template in an operand
    /// is a syntax problem whatever its type accepts; operands are scanned
    /// for one in `stored`, `authored` as loaded with its literal tokens
    /// still encoded.
    ///
    /// Problems are located with `positions` once the report is settled, so a
    /// valid instance never asks for a position map.
    fn validate_instance(
        &self,
        instance: &Value,
        authored: &Value,
        stored: &Value,
        positions: &ProblemPositions<'_>,
        templates: &simplified_schema::coerce::NestedTemplates,
    ) -> ValidationReport {
        let bound = self
            .conversion()
            .marks_operands(&self.json_schema)
            .then(|| bound_operands::BoundOperands::bind(&self.json_schema, authored, stored));
        let widened;
        let templates = match &bound {
            Some(bound) if !bound.settled.is_empty() => {
                widened = templates.with_settled(&bound.settled);
                &widened
            }
            _ => templates,
        };
        let failures = std::cell::OnceCell::new();
        let excused = |problem: &ValidationProblem| {
            templates.excuses_problem(problem, self.conversion(), &self.json_schema, instance, &failures)
        };
        let anchors = validate::FileRefAnchors::Resolved {
            base_dir: self.base_dir.as_deref(),
            fallback: self.file_ref_fallback_dir.as_deref(),
            context: &self.file_resolution_context,
            excused: (!templates.is_empty()).then_some(&excused as &validate::Excused<'_>),
        };
        let mut problems = self.validators.collect_problems(&self.json_schema, instance, anchors);
        simplified_schema::coerce::annotate_problems(self.conversion(), &self.json_schema, instance, templates, &mut problems);
        simplified_schema::prepared::describe_problems(&self.json_schema, instance, &mut problems);
        // After the enrichment: a binder's problem names its call, and the
        // description found at its place would be an arbitrary arm's.
        if let Some(bound) = &bound {
            bound.report_malformed(&mut problems);
            bound.report_rejections(&mut problems);
        }
        positions.locate_all(&mut problems);
        ValidationReport {
            valid: problems.is_empty(),
            problems,
            pending: Vec::new(),
            deferred: templates.pointers(),
            advisories: self.advisories.clone(),
            trigger_findings: Vec::new(),
        }
    }

    /// Validates like [`Self::validate_with_positions`], then applies the
    /// compose-parity deferral rules from `options` as data (R-5 Priority 3).
    ///
    /// Top-level values that are a whole-value `$(...)` shell expression or
    /// still hold an unresolved `{{ ... }}` template are collected into
    /// [`ValidationReport::pending`]. With [`PendingPolicy::Defer`] (the
    /// default), a problem is dropped when its key is a whole-value `$(...)`
    /// or when the authored value at the problem's own path holds a
    /// `{{ ... }}` template — mirroring compose's pre-shell gate, which lets
    /// a later pass re-validate the rendered value. A problem on a literal
    /// sibling of a template stays, and so does one on a malformed template
    /// (`{{ 1 + }}`), which no rendering can settle. Independently of the policy, templates at
    /// places the schema marks `deferred` are never judged. Keys in
    /// `options.excluded_keys` have their problems dropped unconditionally
    /// (caller-owned keys).
    ///
    /// Nothing here executes a shell command, reads the environment, or
    /// touches the network: pending values are recognised lexically only.
    pub fn validate_with_options(
        &self,
        frontmatter: &Value,
        positions: &PositionMap,
        options: &ValidationOptions,
    ) -> ValidationReport {
        let mut report = self.judge_stored(frontmatter, &ProblemPositions::Built(positions), false);
        let pending = scan_pending_values(frontmatter);
        let shell_pending_keys: HashSet<&str> = pending
            .iter()
            .filter(|p| p.reason == PendingValueReason::ShellExpression)
            .map(|p| p.key.as_str())
            .collect();
        let defer = matches!(options.pending_policy, PendingPolicy::Defer);

        // Path-precise, as compose's pre-shell gate is: rendering a template
        // changes only the value at its own path, so it can settle a problem
        // there or on an ancestor whose subtree holds it, never one on an
        // unrelated sibling. A whole-value `$( … )` replaces its whole key.
        let deferred = &report.deferred;
        report.problems.retain(|problem| {
            let Some(key) = attributable_top_level_key(problem) else {
                return true;
            };
            if options.excluded_keys.contains(&key) {
                return false;
            }
            if !defer {
                return true;
            }
            if shell_pending_keys.contains(key.as_str()) {
                return false;
            }
            // A malformed `{{ … }}` span can never render into a valid value,
            // so its problem stands, as plain validation and Claudine's launch
            // verdict report it.
            let holds = |text: &str| {
                text.contains("{{")
                    && crate::markdown::literal_token::holds_pending_syntax(text)
                    && !crate::markdown::compose::expression::is_malformed_template(text)
            };
            !authored_node_at(frontmatter, &problem.path)
                .is_some_and(|(pointer, node)| awaits_rendering(problem, node, &pointer, deferred, &holds))
        });
        report.valid = report.problems.is_empty();
        report.pending = pending;
        report
    }

    /// Returns the compiled validator of an object schema; `None` for a root
    /// union, which is judged arm by arm. Mainly for advanced callers; the
    /// `validate` method is the normal entry point.
    pub fn validator(&self) -> Option<&Validator> {
        match &self.validators {
            CompiledValidators::Object(validator) => Some(validator),
            CompiledValidators::RootUnion(_) => None,
        }
    }

    /// Normalizes eager-`file`-typed frontmatter values to their resolved
    /// document-relative paths.
    ///
    /// Walks the compiled schema for every present, non-null value under an
    /// eager `format: darkmatter-file` marker and rewrites it to the same
    /// projection `relative(value)` / `dirname(value)` already produce, using
    /// the request-scoped resolution context this schema was built with. Pure:
    /// the caller's `frontmatter` is never mutated.
    ///
    /// ## Precondition
    ///
    /// The caller MUST have validated `frontmatter` against this schema
    /// already (compose does so on its accepted effective schema). This method
    /// does not validate; it assumes every present eager-`file` value already
    /// resolves to an existing local file. Rewriting an unvalidated instance
    /// is harmless (the rewrite leaves unresolvable values verbatim) but the
    /// caller is responsible for gating it behind a successful validation.
    ///
    /// ## Composition-pending keys
    ///
    /// Top-level keys still holding a `$(...)` shell expression or unresolved
    /// `{{ ... }}` template (compose pre-shell stage) are skipped verbatim —
    /// they are re-resolved at post-shell re-validation. Pass the same
    /// `composition_pending` set [`simplified_schema::coerce::coerce_frontmatter_with_pending`]
    /// consumes.
    ///
    /// ## Examples
    ///
    /// A raw caller-supplied reference under an eager `file(eager)` property is
    /// rewritten to its document-relative resolved path — the same projection
    /// `relative(value)` / `dirname(value)` already produce:
    ///
    /// ```no_run
    /// use std::collections::HashSet;
    ///
    /// use darkmatter::markdown::Markdown;
    /// use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
    /// use darkmatter::markdown::schemas::DarkmatterSchemas;
    ///
    /// // `area/prompt.md` declares `$schema: { spec: file(eager; required) }`
    /// // and the repo root sits above `area/`. The schema resolves document-
    /// // first against the prompt's directory.
    /// let path = std::path::Path::new("area/prompt.md");
    /// let md = Markdown::try_from(path).unwrap();
    /// let snapshot = RequestSnapshot::new(std::env::current_dir().unwrap());
    /// let context = build_resolution_context(&snapshot).unwrap();
    /// let effective = DarkmatterSchemas::new(context)
    ///     .effective_for(&md)
    ///     .unwrap()
    ///     .unwrap();
    ///
    /// let input = serde_json::json!({ "spec": "./spec.md" });
    /// let outcome = effective.normalize_frontmatter(&input, &HashSet::new());
    /// // Raw `./spec.md` -> document-relative `spec.md`.
    /// assert_eq!(outcome.value["spec"], serde_json::json!("spec.md"));
    /// // The caller's input is never mutated.
    /// assert_eq!(input["spec"], serde_json::json!("./spec.md"));
    /// ```
    ///
    /// ## Returns
    ///
    /// A [`NormalizationOutcome`] carrying the rewritten instance plus a flag
    /// telling the caller whether any eager-`file` value actually changed.
    /// The caller's `frontmatter` argument is never mutated.
    pub fn normalize_frontmatter(
        &self,
        frontmatter: &Value,
        composition_pending: &HashSet<String>,
    ) -> NormalizationOutcome {
        let Some(base_dir) = self.base_dir.as_deref() else {
            // No document anchor was threaded (only possible outside the
            // production `effective_for` path, which always sets `base_dir`).
            // Without an anchor the projection is undefined; leave the instance
            // verbatim rather than introducing an ambient-CWD anchor (spec:
            // "Implementation constraints").
            return NormalizationOutcome {
                value: frontmatter.clone(),
                changed: false,
            };
        };
        rewrite::rewrite_for_effective(self, frontmatter, base_dir, composition_pending)
    }
}

/// Outcome of validating a document's frontmatter against its effective
/// schema.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// `true` when no problems were found.
    pub valid: bool,
    /// Individual problems, in `iter_errors` order.
    pub problems: Vec<ValidationProblem>,
    /// Top-level values that could not be validated yet, wholly or in part,
    /// because they still hold a `$(...)` shell expression or an unresolved
    /// `{{ ... }}` template (R-5 Priority 3). A list or mapping holding a
    /// template is listed even though at most its templates at places the
    /// schema marks `deferred` are deferred.
    /// Always empty for [`EffectiveSchema::validate`] /
    /// [`EffectiveSchema::validate_with_positions`]; populated only by
    /// [`EffectiveSchema::validate_with_options`], which mirrors the compose
    /// deferral rules without executing anything.
    pub pending: Vec<PendingValue>,
    /// JSON Pointers of the `{{ ... }}` templates this validation accepted
    /// without judging because the schema marks their place `deferred`. A
    /// caller that sets problems aside for later rendering consults it (see
    /// [`awaits_rendering`]): a problem left beside an accepted template is
    /// genuine, since that template's value was already allowed for.
    pub deferred: Vec<String>,
    /// Non-fatal findings discovered while resolving the effective schema.
    pub advisories: Vec<SchemaAdvisory>,
    /// The schema triggers automatic discovery skipped as invalid, in
    /// discovery order. Filled by [`DarkmatterSchemas::validate`] and
    /// [`DarkmatterSchemas::validate_for_phase`] whether or not an effective
    /// schema exists; empty from an [`EffectiveSchema`]'s own validation.
    pub trigger_findings: Vec<triggers::InvalidTriggerFinding>,
}

/// What preparing a document's effective schema found, returned beside the
/// [`ValidationReport`] the same preparation produced (see
/// [`DarkmatterSchemas::validate_summarized`]).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PreparationSummary {
    /// The layers that applied; empty when no schema did.
    pub layers: SchemaLayers,
    /// The files the schema was prepared from, as
    /// [`EffectiveSchema::dependencies`] lists them; empty when no schema
    /// applied.
    pub dependencies: Vec<PathBuf>,
}

impl PreparationSummary {
    /// Whether any schema layer applied. When `false` the report is vacuously
    /// valid.
    #[must_use]
    pub fn has_schema(&self) -> bool {
        !self.layers.is_empty()
    }
}

/// The layers merged into an [`EffectiveSchema`], so a caller can say which
/// schema judged a document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SchemaLayers {
    /// The document's own schema applied: its `$schema` (an inline mapping,
    /// a file reference or bare name, or a root union), or a YAML document's
    /// claimed definition.
    pub document: bool,
    /// The matching triggers that layered a payload, in layering order. A
    /// matcher-only trigger layers nothing and is absent.
    pub triggers: Vec<TriggerSource>,
    /// The configured baseline applied.
    pub baseline: bool,
}

impl SchemaLayers {
    /// Whether no layer applied.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.document && self.triggers.is_empty() && !self.baseline
    }
}

impl ValidationReport {
    /// The valid report of a document no schema applies to.
    fn vacuous() -> Self {
        Self {
            valid: true,
            problems: Vec::new(),
            pending: Vec::new(),
            deferred: Vec::new(),
            advisories: Vec::new(),
            trigger_findings: Vec::new(),
        }
    }
}

/// Code of the warning for a schema trigger automatic discovery skipped
/// because its envelope or payload failed to load.
pub const SCHEMA_INVALID_TRIGGER_CODE: &str = "dm.schema.invalid_trigger";
/// A top-level frontmatter value that cannot be validated yet because it holds
/// a deferred composition construct (R-5 Priority 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingValue {
    /// The top-level frontmatter key.
    pub key: String,
    /// JSON Pointer to the pending value (`/{key}`).
    pub path: JsonPointer,
    /// Why the value is pending.
    pub reason: PendingValueReason,
}

/// Why a [`PendingValue`] is deferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingValueReason {
    /// The value is a whole-value `$(...)` shell expression that has not run.
    ShellExpression,
    /// The value holds an unresolved `{{ ... }}` template.
    UnresolvedTemplate,
}

/// How [`EffectiveSchema::validate_with_options`] treats pending values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingPolicy {
    /// Drop a problem whose key is a whole-value `$(...)`, or whose own
    /// path holds an unresolved `{{ ... }}` template (mirrors compose when a
    /// later pass will re-validate the rendered value).
    Defer,
    /// Keep every problem; pending values are still listed on
    /// [`ValidationReport::pending`] but do not suppress diagnostics.
    Report,
}

/// Options controlling [`EffectiveSchema::validate_with_options`] (R-5
/// Priority 3). Mirrors the compose deferral rules without executing anything.
#[derive(Debug, Clone)]
pub struct ValidationOptions {
    /// How to treat top-level values still holding `$(...)` / `{{ ... }}`.
    pub pending_policy: PendingPolicy,
    /// Top-level keys whose problems are dropped entirely (caller-owned keys,
    /// mirroring compose's `exclude_keys`).
    pub excluded_keys: HashSet<String>,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            pending_policy: PendingPolicy::Defer,
            excluded_keys: HashSet::new(),
        }
    }
}


struct Derivation {
    effective: Arc<Value>,
    projected_document: Option<Value>,
    phase: SchemaPhase,
    derived: Arc<Value>,
}
const RECENT_DERIVATIONS: usize = 8;
/// [`phase_json_schema`]'s memo; module-level so [`reset_process_caches_for_bench`] can clear it.
static DERIVATIONS: std::sync::Mutex<Vec<Derivation>> = std::sync::Mutex::new(Vec::new());

/// `effective` judged at `phase`: the phase projection of its document schema
/// (`projected_document`, when it has one) merged over it, made passive.
///
/// Memoized for the process over a few recent derivations, matched by `Arc`
/// identity for `effective` (which an entry holds alive) and by value for the
/// small projected document. A derivation copies and walks the whole
/// effective schema, and a run judges the same schema at a phase several
/// times; the shared result also lets the validator cache reuse one compiled
/// validator for it.
fn phase_json_schema(
    effective: &Arc<Value>,
    projected_document: Option<Value>,
    phase: SchemaPhase,
) -> Result<Arc<Value>, SchemaError> {
    let same = |derivation: &Derivation| {
        Arc::ptr_eq(&derivation.effective, effective)
            && derivation.phase == phase
            && derivation.projected_document == projected_document
    };
    {
        let mut derivations = DERIVATIONS.lock().expect("phase schema memo poisoned");
        if let Some(index) = derivations.iter().position(same) {
            let hit = derivations.remove(index);
            let derived = Arc::clone(&hit.derived);
            derivations.push(hit);
            return Ok(derived);
        }
    }

    let mut json_schema = match projected_document.clone() {
        Some(document) => phase::merge_with_effective(effective, document)?,
        None => effective.as_ref().clone(),
    };
    phase::make_passive(&mut json_schema);
    // Nothing to project and no eager file to make passive: the phase judges
    // the effective schema itself, and shares its compiled validator.
    let derived = if json_schema == **effective {
        Arc::clone(effective)
    } else {
        Arc::new(json_schema)
    };

    let mut derivations = DERIVATIONS.lock().expect("phase schema memo poisoned");
    if derivations.len() == RECENT_DERIVATIONS {
        derivations.remove(0);
    }
    derivations.push(Derivation {
        effective: Arc::clone(effective),
        projected_document,
        phase,
        derived: Arc::clone(&derived),
    });
    Ok(derived)
}

struct Merge {
    baseline: Option<Arc<Value>>,
    layers: Vec<Arc<Value>>,
    doc: Option<Value>,
    merged: Arc<Value>,
}
const RECENT_MERGES: usize = 8;
/// [`merged_trigger_schema`]'s memo; module-level so [`reset_process_caches_for_bench`] can clear it.
static MERGES: std::sync::Mutex<Vec<Merge>> = std::sync::Mutex::new(Vec::new());

/// The merge of `baseline`, the trigger `layers` (in order), and the document
/// schema `doc`, highest precedence last.
///
/// This is the library's generic layering,
/// `doc.layer_under([baseline, ...layers])` on
/// [`simplified_schema::SimplifiedSchema`], done here over the already
/// lowered layers with the same merge ([`merge_baseline`](simplified_schema::resolve::merge_baseline)) so a hit can be
/// matched by `Arc` identity and the baseline and payloads are never rebuilt
/// as schema objects per page.
///
/// Memoized for the process over a few recent merges. Trigger payloads are
/// large (a host bundle can resolve to megabytes) and the same document is
/// judged several times per run, so re-merging them would dominate every
/// verdict. An entry holds its inputs alive and is matched by `Arc` identity
/// for the baseline and payloads and by value for the (small) document
/// schema, so a hit is always the merge of equal inputs. The shared result
/// also keeps the validator cache's pointer-keyed content hash warm, which is
/// why a document schema with no trigger layer (alone or over the baseline)
/// is memoized here too: each request resolves it to a new value.
fn merged_trigger_schema(
    baseline: Option<&Arc<Value>>,
    layers: &[TriggerLayer],
    doc: Option<Value>,
) -> Result<Arc<Value>, SchemaError> {
    let same = |merge: &Merge| {
        merge.baseline.as_ref().map(Arc::as_ptr) == baseline.map(Arc::as_ptr)
            && merge.layers.len() == layers.len()
            && merge.layers.iter().zip(layers).all(|(held, layer)| Arc::ptr_eq(held, &layer.json_schema))
            && merge.doc == doc
    };
    {
        let mut merges = MERGES.lock().expect("trigger merge memo poisoned");
        if let Some(index) = merges.iter().position(same) {
            let hit = merges.remove(index);
            let merged = Arc::clone(&hit.merged);
            merges.push(hit);
            return Ok(merged);
        }
    }

    // The lowest layer is borrowed as the first merge base; each higher one
    // is cloned because a merge consumes it.
    let (lowest, rest): (Option<&Value>, &[TriggerLayer]) = match (baseline, layers.split_first()) {
        (Some(base), _) => (Some(base), layers),
        (None, Some((first, rest))) => (Some(&first.json_schema), rest),
        (None, None) => (None, layers),
    };
    let mut higher = rest.iter().map(|layer| layer.json_schema.as_ref().clone()).chain(doc.clone());
    let merged = match (lowest, higher.next()) {
        (Some(lowest), Some(next)) => {
            let mut acc = simplified_schema::resolve::merge_baseline(lowest, next)?;
            for layer in higher {
                acc = simplified_schema::resolve::merge_baseline(&acc, layer)?;
            }
            Arc::new(acc)
        }
        (None, Some(doc)) => Arc::new(doc),
        (Some(lowest), None) => Arc::new(lowest.clone()),
        (None, None) => unreachable!("the caller merges at least one schema"),
    };

    let mut merges = MERGES.lock().expect("trigger merge memo poisoned");
    if merges.len() == RECENT_MERGES {
        merges.remove(0);
    }
    merges.push(Merge {
        baseline: baseline.cloned(),
        layers: layers.iter().map(|layer| Arc::clone(&layer.json_schema)).collect(),
        doc,
        merged: Arc::clone(&merged),
    });
    Ok(merged)
}


/// The top-level frontmatter key a problem is attributable to, mirroring the
/// compose-time deferral rule: the first pointer segment, or the missing
/// property for a missing-required failure on the root itself (its path
/// points at the parent, so a nested one belongs to the key that holds it).
fn attributable_top_level_key(problem: &ValidationProblem) -> Option<String> {
    match problem.instance_path.first() {
        Some(key) => Some(key.to_string()),
        None if problem.code == ValidationProblemCode::MissingRequired => problem.property.clone(),
        None => None,
    }
}

/// Scans a frontmatter instance for top-level values a later compose stage
/// resolves — a whole-value `$(...)` shell expression or a `{{ ... }}`
/// template anywhere in the value — mirroring compose's pre-shell deferral,
/// lexically only, never executing anything. `$(...)` text elsewhere (inside
/// a list, a mapping, or after other text) is never expanded, so it is not
/// pending. `$schema` is skipped (a control key, not data).
fn scan_pending_values(frontmatter: &Value) -> Vec<PendingValue> {
    let Value::Object(map) = frontmatter else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, value) in map {
        if key == "$schema" {
            continue;
        }
        if let Some(reason) = pending_reason(value) {
            out.push(PendingValue {
                key: key.clone(),
                path: JsonPointer::parse(&format!(
                    "/{}",
                    key.replace('~', "~0").replace('/', "~1")
                )),
                reason,
            });
        }
    }
    out
}

/// Classifies a value's deferral reason. A whole-value `$(...)` shell
/// expression takes precedence over a `{{ ... }}` template it contains (the
/// shell value is what a later pass resolves first), matching compose's
/// ordering.
fn pending_reason(value: &Value) -> Option<PendingValueReason> {
    let shell_candidate = value.as_str().is_some_and(|text| {
        crate::markdown::compose::value_origin::authored_shell_candidate(text)
            && crate::markdown::literal_token::holds_pending_syntax(text)
    });
    if shell_candidate {
        Some(PendingValueReason::ShellExpression)
    } else if value_contains_marker(value, "{{") {
        Some(PendingValueReason::UnresolvedTemplate)
    } else {
        None
    }
}

/// Whether rendering a template could settle `problem`, so a caller that
/// renders later may set it aside: `node` (at `node_pointer`, see
/// [`authored_node_at`]) is a template itself, or a collection holding one,
/// and that template is not one the schema already accepted (`deferred`, from
/// [`ValidationReport::deferred`]). `holds` is the caller's template test.
///
/// Rendering changes values, never keys: an unknown or missing key on a
/// collection is never set aside, and a problem on a literal sibling of a
/// template never reaches a node that holds it.
pub fn awaits_rendering(
    problem: &ValidationProblem,
    node: &Value,
    node_pointer: &str,
    deferred: &[String],
    holds: &dyn Fn(&str) -> bool,
) -> bool {
    fn unaccepted(value: &Value, pointer: &mut String, deferred: &[String], holds: &dyn Fn(&str) -> bool) -> bool {
        match value {
            Value::String(text) => holds(text) && !deferred.iter().any(|accepted| accepted == pointer),
            Value::Array(items) => items.iter().enumerate().any(|(index, item)| {
                let len = pointer.len();
                pointer.push_str(&format!("/{index}"));
                let found = unaccepted(item, pointer, deferred, holds);
                pointer.truncate(len);
                found
            }),
            Value::Object(map) => map.iter().any(|(key, item)| {
                let len = pointer.len();
                pointer.push('/');
                pointer.push_str(&key.replace('~', "~0").replace('/', "~1"));
                let found = unaccepted(item, pointer, deferred, holds);
                pointer.truncate(len);
                found
            }),
            _ => false,
        }
    }
    if !node.is_string()
        && matches!(problem.code, ValidationProblemCode::UnknownKey | ValidationProblemCode::MissingRequired)
    {
        return false;
    }
    unaccepted(node, &mut node_pointer.to_string(), deferred, holds)
}

fn value_contains_marker(value: &Value, marker: &str) -> bool {
    match value {
        Value::String(s) => {
            s.contains(marker) && crate::markdown::literal_token::holds_pending_syntax(s)
        }
        Value::Array(items) => items.iter().any(|v| value_contains_marker(v, marker)),
        Value::Object(map) => map.values().any(|v| value_contains_marker(v, marker)),
        _ => false,
    }
}

/// Builds the per-top-level-property origin map for an effective schema.
///
/// Attribution precedence (mirrors the merge order — later layers win):
/// 1. A property present in the resolved **document** schema → document /
///    referenced-file origin.
/// 2. A property present in a **trigger payload** → trigger origin (last
///    trigger in the ordered list wins, since later layers override).
/// 3. Everything else → baseline origin.
///
/// Root-union schemas (no top-level `properties`) yield an empty map — per-arm
/// provenance is not modelled in v1.
fn build_origin_map_with_triggers(
    resolved: Option<&simplified_schema::resolve::ResolvedSchema>,
    trigger_layers: &[TriggerLayer],
    _has_baseline: bool,
    merged: &Value,
) -> SchemaOriginMap {
    let mut out = SchemaOriginMap::new();
    let Some(props) = merged.get("properties").and_then(Value::as_object) else {
        return out;
    };
    let doc_props = resolved
        .map(|r| &r.json_schema)
        .and_then(|s| s.get("properties"))
        .and_then(Value::as_object);
    for key in props.keys() {
        let origin = if doc_props.is_some_and(|doc| doc.contains_key(key)) {
            // Document schema contributed this property.
            resolved
                .map(|r| r.origin.clone())
                .unwrap_or_else(SchemaOrigin::document)
        } else {
            // Check trigger payloads in reverse order — the last trigger that
            // declares the property won the merge slot.
            let from_trigger = trigger_layers.iter().rev().find_map(|layer| {
                layer
                    .json_schema
                    .get("properties")
                    .and_then(Value::as_object)
                    .is_some_and(|p| p.contains_key(key))
                    .then(|| SchemaOrigin::for_trigger(&layer.source))
            });
            from_trigger.unwrap_or_else(SchemaOrigin::baseline)
        };
        out.insert(key.clone(), origin);
    }
    out
}

/// Collects all dependency edges for an effective schema: the document
/// `$schema`'s imports + examples + referenced files, plus each file
/// trigger's envelope + payload dependencies. An embedded trigger adds none:
/// it cannot change while the host runs. Sorted and deduplicated.
fn build_dependencies(
    resolved: Option<&simplified_schema::resolve::ResolvedSchema>,
    trigger_layers: &[TriggerLayer],
) -> Vec<PathBuf> {
    let mut deps: Vec<PathBuf> = Vec::new();
    if let Some(r) = resolved {
        deps.extend(r.imports.iter().cloned());
        deps.extend(r.examples.iter().cloned());
        deps.extend(r.referenced_files.iter().cloned());
    }
    for layer in trigger_layers {
        deps.extend(layer.source.path().map(Path::to_path_buf));
        deps.extend(layer.dependencies.iter().cloned());
    }
    deps.sort();
    deps.dedup();
    deps
}

fn build_advisories(
    baseline: Option<&BaselineSchema>,
    trigger_layers: &[TriggerLayer],
    resolved: Option<&simplified_schema::resolve::ResolvedSchema>,
) -> Vec<SchemaAdvisory> {
    let mut advisories = BTreeSet::new();
    if let Some(baseline) = baseline {
        advisories.extend(baseline.advisories.iter().cloned());
    }
    for layer in trigger_layers {
        advisories.extend(layer.advisories.iter().cloned());
    }
    if let Some(resolved) = resolved {
        advisories.extend(resolved.advisories.iter().cloned());
    }
    advisories.into_iter().collect()
}

/// The [`PositionMap`] for one validation source snapshot.
///
/// Positions come from the bytes the document was loaded from, so lines count
/// the complete file, opening `---` included, and CRLF offsets stay exact.
/// They name where a path was authored even after an assignment or coercion
/// replaced its value. Without loaded text, the retained frontmatter YAML is
/// used, offset by the opening delimiter line; a programmatically constructed
/// document is located in its serialized view.
pub(crate) fn positions_for(source: &Markdown) -> PositionMap {
    // The raw YAML's first line is the file's second: the opening `---` is
    // line 1.
    const RAW_SOURCE_BASE_LINE: u32 = 2;
    crate::note_work!(PositionMapBuild);
    let frontmatter = source.frontmatter();
    // Loaded text is kept even when frontmatter parsing failed, so only a
    // parsed frontmatter's own source may be projected.
    if let Some(raw) = frontmatter.raw_source() {
        let orders = frontmatter.mapping_orders();
        let projected = match source.loaded_source() {
            Some(loaded) => crate::markdown::extract_frontmatter_block(&loaded.text)
                .ok()
                .flatten()
                .and_then(|block| {
                    PositionMap::project(&loaded.text, block.yaml_span.clone(), 1, orders)
                }),
            None => PositionMap::project(raw, 0..raw.len(), RAW_SOURCE_BASE_LINE, orders),
        };
        return projected.unwrap_or_else(|| PositionMap::top_level(raw, RAW_SOURCE_BASE_LINE));
    }

    // Programmatically constructed documents never went through the parser;
    // line numbers refer to the canonicalised view.
    let map = source.frontmatter().as_map();
    if map.is_empty() {
        return PositionMap::new();
    }
    let mut yaml_map = serde_yaml_ng::Mapping::new();
    for (k, v) in map {
        yaml_map.insert(
            serde_yaml_ng::Value::String(k.clone()),
            json_to_yaml_value(v),
        );
    }
    let yaml_value = serde_yaml_ng::Value::Mapping(yaml_map);
    match serde_yaml_ng::to_string(&yaml_value) {
        Ok(rendered) => PositionMap::project(
            &rendered,
            0..rendered.len(),
            1,
            &crate::markdown::MappingOrders::collect(&yaml_value),
        )
        .unwrap_or_else(|| PositionMap::top_level(&rendered, 1)),
        Err(_) => PositionMap::new(),
    }
}

fn json_to_yaml_value(value: &Value) -> serde_yaml_ng::Value {
    match value {
        Value::Null => serde_yaml_ng::Value::Null,
        Value::Bool(b) => serde_yaml_ng::Value::Bool(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(i))
            } else if let Some(u) = n.as_u64() {
                serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(u))
            } else if let Some(f) = n.as_f64() {
                serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(f))
            } else {
                serde_yaml_ng::Value::Null
            }
        }
        Value::String(s) => serde_yaml_ng::Value::String(s.clone()),
        Value::Array(items) => {
            serde_yaml_ng::Value::Sequence(items.iter().map(json_to_yaml_value).collect())
        }
        Value::Object(map) => {
            let mut out = serde_yaml_ng::Mapping::new();
            for (k, v) in map {
                out.insert(
                    serde_yaml_ng::Value::String(k.clone()),
                    json_to_yaml_value(v),
                );
            }
            serde_yaml_ng::Value::Mapping(out)
        }
    }
}

/// The directory a document's file values resolve from: its own, else the
/// request context's `cwd`.
fn base_dir_for(source: &Markdown, context: &biscuit_file::FileResolutionContext) -> PathBuf {
    match source.source() {
        Some(ComposeSource::File(path)) => path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map_or_else(|| context.cwd().to_path_buf(), Path::to_path_buf),
        _ => context.cwd().to_path_buf(),
    }
}

/// `source`'s frontmatter as the value a schema judges: everything but the
/// `$schema` control key.
pub fn frontmatter_as_json(source: &Markdown) -> Value {
    let map = source.frontmatter().as_map();
    let mut object = serde_json::Map::with_capacity(map.len());
    for (k, v) in map {
        // The Darkmatter `$schema` control key is not document data — strip
        // it before validation so raw JSON Schema baselines with
        // `additionalProperties: false` don't reject every document.
        if k == "$schema" {
            continue;
        }
        object.insert(k.clone(), v.clone());
    }
    Value::Object(object)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod claudine_compat_tests {
    use super::*;
    use simplified_schema::{Constraint, SchemaOriginKind};
    use crate::markdown::Markdown;

    fn md_with_schema(yaml_body: &str) -> Markdown {
        let content = format!("---\n{yaml_body}---\nbody\n");
        let md: Markdown = content.as_str().into();
        md
    }

    #[test]
    fn validates_inline_schema_success() {
        let md = md_with_schema("$schema:\n  title: 'string(required)'\ntitle: Hello\n");
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()));
        let report = api.validate(&md).unwrap();
        assert!(report.valid, "expected valid: {:?}", report.problems);
    }

    #[test]
    fn validates_inline_schema_missing_required() {
        let md = md_with_schema("$schema:\n  title: 'string(required)'\nother: stuff\n");
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()));
        let report = api.validate(&md).unwrap();
        assert!(!report.valid);
        assert!(!report.problems.is_empty());
    }

    #[test]
    fn no_schema_no_baseline_is_vacuously_valid() {
        let md = md_with_schema("name: alice\n");
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()));
        let report = api.validate(&md).unwrap();
        assert!(report.valid);
        assert!(report.problems.is_empty());
    }

    #[test]
    fn baseline_applies_when_document_has_no_schema() {
        let md = md_with_schema("title: hi\n");
        let baseline = SchemaRoot::Single(SchemaShape {
            properties: {
                let mut m = indexmap::IndexMap::new();
                m.insert(
                    "owner".into(),
                    PropertyDef::Single(PropertyAtom {
                        ty: TypeExpr::Primitive(SimplifiedType::String),
                        is_array: false,
                        constraints: vec![Constraint::Required],
                        array_constraints: vec![],
                        description: None,
                    }),
                );
                m
            },
            ..Default::default()
        });
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir())).with_baseline(baseline).unwrap();
        let report = api.validate(&md).unwrap();
        assert!(!report.valid);
        assert!(
            report
                .problems
                .iter()
                .any(|p| p.message.to_ascii_lowercase().contains("owner"))
        );
    }

    #[test]
    fn baseline_merges_with_document_schema() {
        let md = md_with_schema("$schema:\n  title: 'string(required)'\ntitle: hi\n");
        let baseline = SchemaRoot::Single(SchemaShape {
            properties: {
                let mut m = indexmap::IndexMap::new();
                m.insert(
                    "owner".into(),
                    PropertyDef::Single(PropertyAtom {
                        ty: TypeExpr::Primitive(SimplifiedType::String),
                        is_array: false,
                        constraints: vec![Constraint::Required],
                        array_constraints: vec![],
                        description: None,
                    }),
                );
                m
            },
            ..Default::default()
        });
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir())).with_baseline(baseline).unwrap();
        let report = api.validate(&md).unwrap();
        assert!(!report.valid);
        assert!(
            report
                .problems
                .iter()
                .any(|p| p.message.to_ascii_lowercase().contains("owner"))
        );
    }

    #[test]
    fn validator_cache_reuses_across_documents() {
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()));
        let md1 = md_with_schema("$schema:\n  x: number\nx: 1\n");
        let md2 = md_with_schema("$schema:\n  x: number\nx: 2\n");
        api.validate(&md1).unwrap();
        api.validate(&md2).unwrap();
        assert_eq!(api.cache().len(), 1);
    }

    #[test]
    fn coerces_boolish_string_against_inline_schema() {
        let md = md_with_schema("$schema:\n  flag: boolean\nflag: \"true\"\n");
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()));
        let report = api.validate(&md).unwrap();
        assert!(report.valid, "expected valid: {:?}", report.problems);
    }

    #[test]
    fn coerces_numeric_string_against_inline_schema() {
        let md = md_with_schema("$schema:\n  n: number\nn: \"42\"\n");
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()));
        let report = api.validate(&md).unwrap();
        assert!(report.valid, "expected valid: {:?}", report.problems);
    }

    #[test]
    fn non_boolean_word_still_reports_type_problem() {
        let md = md_with_schema("$schema:\n  flag: boolean(required)\nflag: \"maybe\"\n");
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()));
        let report = api.validate(&md).unwrap();
        assert!(!report.valid);
        assert!(
            report
                .problems
                .iter()
                .any(|p| p.kind == ValidationProblemKind::Type && p.path == "/flag"),
            "expected Type problem on /flag: {:?}",
            report.problems
        );
    }

    #[test]
    fn coerces_baseline_merged_field_without_document_schema() {
        // Document has no `$schema`; the boolean field comes solely from the
        // baseline. Coercion reads the post-merge json_schema, so it fires even
        // though the document's `simplified` AST never declares `enabled`.
        let md = md_with_schema("enabled: \"false\"\n");
        let baseline = SchemaRoot::Single(SchemaShape {
            properties: {
                let mut m = indexmap::IndexMap::new();
                m.insert(
                    "enabled".into(),
                    PropertyDef::Single(PropertyAtom {
                        ty: TypeExpr::Primitive(SimplifiedType::Boolean),
                        is_array: false,
                        constraints: vec![],
                        array_constraints: vec![],
                        description: None,
                    }),
                );
                m
            },
            ..Default::default()
        });
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir())).with_baseline(baseline).unwrap();
        let report = api.validate(&md).unwrap();
        assert!(report.valid, "expected valid: {:?}", report.problems);
    }

    #[test]
    fn coerces_raw_json_schema_baseline_with_no_simplified_ast() {
        // Raw JSON Schema baseline → `simplified` is None on the effective
        // schema. Coercion never consults the AST, so the boolish string still
        // coerces to a real boolean and validates.
        let md = md_with_schema("flag: \"true\"\n");
        let raw = serde_json::json!({
            "type": "object",
            "properties": { "flag": {"type": "boolean"} }
        });
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()))
            .with_baseline_json_schema(raw)
            .unwrap();
        let effective = api.effective_for(&md).unwrap().unwrap();
        assert!(
            effective.simplified.is_none(),
            "raw JSON Schema baseline should have no simplified AST"
        );
        let report = api.validate(&md).unwrap();
        assert!(report.valid, "expected valid: {:?}", report.problems);
    }

    #[test]
    fn enriches_problem_with_property_description() {
        // End-to-end: the SimplifiedSchema `->` description surfaces on the
        // populated `ValidationProblem.description`.
        let md = md_with_schema(
            "$schema:\n  title: 'string(required) -> The headline shown in listings'\nother: x\n",
        );
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()));
        let report = api.validate(&md).unwrap();
        assert!(!report.valid);
        assert!(
            report
                .problems
                .iter()
                .any(|p| p.description.as_deref() == Some("The headline shown in listings")),
            "expected a populated description: {:?}",
            report.problems
        );
    }

    #[test]
    fn description_equal_to_message_is_suppressed() {
        // Decision #9: a description byte-for-byte equal to the rendered message
        // is dropped so the same sentence is not printed twice. The missing
        // `title` message is `"title" is a required property`.
        let md = md_with_schema("other: x\n");
        let raw = serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "\"title\" is a required property" }
            },
            "required": ["title"]
        });
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()))
            .with_baseline_json_schema(raw)
            .unwrap();
        let report = api.validate(&md).unwrap();
        let problem = report
            .problems
            .iter()
            .find(|p| p.property.as_deref() == Some("title"))
            .expect("missing-title problem");
        assert_eq!(problem.message, "\"title\" is a required property");
        assert_eq!(
            problem.description, None,
            "description equal to the message must be suppressed"
        );
    }

    #[test]
    fn whitespace_only_description_is_suppressed() {
        // Decision #8: a whitespace-only description renders nothing.
        let md = md_with_schema("title:\n  - 1\n  - 2\n");
        let raw = serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "   " }
            }
        });
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()))
            .with_baseline_json_schema(raw)
            .unwrap();
        let report = api.validate(&md).unwrap();
        let problem = report
            .problems
            .iter()
            .find(|p| p.path == "/title")
            .expect("title type problem");
        assert_eq!(
            problem.description, None,
            "whitespace-only description must be suppressed"
        );
    }

    // ── file_ref_fallback_dir threading (Phase 2 Track B) ───────────────

    /// A `file`-typed schema property value is **not** resolved via the
    /// captured launch-area fallback (D2): the launch area is not a resolution
    /// input for a reference authored inside the document. A value present only
    /// under the fallback (and not under the context's `cwd`, which anchors this
    /// sourceless document) fails validation.
    #[test]
    fn file_format_does_not_resolve_via_launch_area_fallback() {
        let launch_dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(launch_dir.path().join("spec.md"), "# Spec\n").expect("write spec");
        let unrelated_dir = tempfile::tempdir().expect("tempdir");

        let md = md_with_schema("$schema:\n  spec: 'file(eager; required)'\nspec: spec.md\n");
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(unrelated_dir.path()))
            .with_file_ref_fallback_dir(launch_dir.path().to_path_buf());
        let report = api.validate(&md).expect("validate");

        assert!(
            !report.valid,
            "the launch-area fallback must not resolve a document-authored file reference: {:?}",
            report.problems,
        );
    }

    /// A `file`-typed schema property value that exists under the context's
    /// `cwd` but NOT under the prompt directory fails validation — proving the
    /// document base directory (not the context's `cwd`, and not the inert
    /// launch-area anchor) drives resolution. The prompt has a real file source
    /// in a third directory so its `base_dir` is distinct from the `cwd`.
    #[test]
    fn file_format_rejects_when_not_under_document_base() {
        let prompt_dir = tempfile::tempdir().expect("tempdir");
        let fallback_dir = tempfile::tempdir().expect("tempdir");
        let cwd_dir = tempfile::tempdir().expect("tempdir");
        // File exists under the context cwd but NOT under the prompt dir or the fallback dir.
        std::fs::write(cwd_dir.path().join("ambient.md"), "# Ambient\n").expect("write");

        let md = prompt_with_source(prompt_dir.path(), "$schema:\n  spec: 'file(eager; required)'\nspec: ambient.md\n");
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(cwd_dir.path()))
            .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());
        let report = api.validate(&md).expect("validate");

        assert!(
            !report.valid,
            "expected validation to fail because ambient.md is under neither the prompt dir nor the fallback dir: {:?}",
            report.problems,
        );
    }

    /// `$schema: ./schema.yaml` is explicitly source-relative. Captured launch
    /// metadata is not a candidate for a schema reference.
    #[test]
    fn explicit_schema_reference_ignores_launch_anchor() {
        let doc_dir = tempfile::tempdir().expect("tempdir");
        let fallback_dir = tempfile::tempdir().expect("tempdir");
        // schema.yaml lives ONLY in the document dir.
        std::fs::write(
            doc_dir.path().join("schema.yaml"),
            "$schema:\n  title: string(required)\n",
        )
        .expect("write schema");
        let doc_path = doc_dir.path().join("doc.md");
        std::fs::write(
            &doc_path,
            "---\n$schema: ./schema.yaml\ntitle: Hello\n---\nbody\n",
        )
        .expect("write doc");

        let md = Markdown::try_from(doc_path.as_path()).expect("read doc");
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(doc_dir.path()))
            .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());
        let report = api.validate(&md).expect("validate");
        assert!(
            report.valid,
            "explicit $schema reference must resolve from the document directory: {:?}",
            report.problems,
        );
    }

    /// A root-union `$schema` string arm with `./` remains explicitly
    /// source-relative and does not search captured launch metadata.
    #[test]
    fn explicit_root_union_schema_arm_ignores_launch_anchor() {
        let doc_dir = tempfile::tempdir().expect("tempdir");
        let fallback_dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            doc_dir.path().join("arm-a.yaml"),
            "$schema:\n  kind: string(required)\n",
        )
        .expect("write arm-a");
        let doc_path = doc_dir.path().join("doc.md");
        std::fs::write(
            &doc_path,
            "---\n$schema:\n  - ./arm-a.yaml\n  - fallback: string\nkind: feature\n---\nbody\n",
        )
        .expect("write doc");

        let md = Markdown::try_from(doc_path.as_path()).expect("read doc");
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(doc_dir.path()))
            .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());
        let report = api.validate(&md).expect("validate");
        assert!(
            report.valid,
            "root-union string arm must resolve from the document dir: {:?}",
            report.problems,
        );
    }

    /// Returns file-backed Markdown with `dir` as its source directory.
    fn prompt_with_source(dir: &std::path::Path, frontmatter: &str) -> Markdown {
        let path = dir.join("prompt.md");
        std::fs::write(&path, format!("---\n{frontmatter}---\nbody\n")).expect("write prompt");
        Markdown::try_from(path.as_path()).expect("read prompt")
    }

    /// Captured launch metadata does not displace a valid source-local
    /// candidate when no repository candidate exists.
    #[test]
    fn file_property_ignores_launch_copy_when_source_exists() {
        let prompt_dir = tempfile::tempdir().expect("tempdir");
        let fallback_dir = tempfile::tempdir().expect("tempdir");
        let unrelated = tempfile::tempdir().expect("tempdir");
        std::fs::write(prompt_dir.path().join("spec.md"), "# prompt copy\n").expect("write prompt spec");
        std::fs::write(fallback_dir.path().join("spec.md"), "# fallback copy\n").expect("write fallback spec");

        let md = prompt_with_source(prompt_dir.path(), "$schema:\n  spec: 'file(eager; required)'\nspec: spec.md\n");
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(unrelated.path()))
            .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());
        let report = api.validate(&md).expect("validate");
        assert!(
            report.valid,
            "the source-local file value must validate independently of the launch copy: {:?}",
            report.problems,
        );
    }

    /// A source-local `file` value validates when no repository candidate
    /// exists, independently of captured launch metadata and the context's `cwd`.
    #[test]
    fn file_property_present_only_in_prompt_dir_validates() {
        let prompt_dir = tempfile::tempdir().expect("tempdir");
        let fallback_dir = tempfile::tempdir().expect("tempdir");
        let unrelated = tempfile::tempdir().expect("tempdir");
        std::fs::write(prompt_dir.path().join("local.md"), "# local\n").expect("write local");

        let md = prompt_with_source(prompt_dir.path(), "$schema:\n  spec: 'file(eager; required)'\nspec: ./local.md\n");
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(unrelated.path()))
            .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());
        let report = api.validate(&md).expect("validate");
        assert!(
            report.valid,
            "a file value beside the prompt must validate even with a fallback set: {:?}",
            report.problems,
        );
    }

    /// A `file` value that exists ONLY under the launch-area fallback (not the
    /// prompt directory) does **not** validate: per D2 the launch area is not a
    /// resolution input for a document-authored reference. Independent of the
    /// context's `cwd`.
    #[test]
    fn file_property_present_only_in_fallback_does_not_validate() {
        let prompt_dir = tempfile::tempdir().expect("tempdir");
        let fallback_dir = tempfile::tempdir().expect("tempdir");
        let unrelated = tempfile::tempdir().expect("tempdir");
        std::fs::write(fallback_dir.path().join("caller.md"), "# caller\n").expect("write caller");

        let md = prompt_with_source(prompt_dir.path(), "$schema:\n  spec: 'file(eager; required)'\nspec: caller.md\n");
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(unrelated.path()))
            .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());
        let report = api.validate(&md).expect("validate");
        assert!(
            !report.valid,
            "a file value present only under the launch-area fallback must not validate: {:?}",
            report.problems,
        );
    }

    /// Guard: with both a prompt-dir anchor and a fallback configured, a value
    /// that exists ONLY under the context's `cwd` (neither the prompt dir nor
    /// the fallback) must NOT validate — a sourced document never falls back to
    /// the `cwd`.
    #[test]
    fn file_property_present_only_in_cwd_does_not_validate() {
        let prompt_dir = tempfile::tempdir().expect("tempdir");
        let fallback_dir = tempfile::tempdir().expect("tempdir");
        let cwd_dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(cwd_dir.path().join("ambient.md"), "# ambient\n").expect("write ambient");

        let md = prompt_with_source(prompt_dir.path(), "$schema:\n  spec: 'file(eager; required)'\nspec: ambient.md\n");
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(cwd_dir.path()))
            .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());
        let report = api.validate(&md).expect("validate");
        assert!(
            !report.valid,
            "a file value found only under the context cwd must NOT validate: {:?}",
            report.problems,
        );
    }

    // ── Phase 3: normalize_frontmatter + read-only validation contract ────

    /// Writes a `---\n{frontmatter}---\nbody\n` document to `dir/prompt.md`
    /// and returns the `EffectiveSchema` for it, with the prompt directory as
    /// `base_dir`.
    fn effective_for_prompt(
        dir: &std::path::Path,
        frontmatter: &str,
    ) -> EffectiveSchema {
        let path = dir.join("prompt.md");
        std::fs::write(&path, format!("---\n{frontmatter}---\nbody\n")).expect("write prompt");
        let md = Markdown::try_from(path.as_path()).expect("read prompt");
        DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(dir))
            .effective_for(&md)
            .expect("effective_for")
            .expect("schema present")
    }

    /// `normalize_frontmatter` rewrites a present eager-`file` value to its
    /// document-relative resolved path, and the caller's input `Value` is left
    /// byte-identical (Decision #3: pure).
    #[test]
    fn normalize_frontmatter_rewrites_eager_file_and_leaves_input_untouched() {
        let repo = tempfile::tempdir().expect("tempdir");
        // The repository marker provides the secondary implicit candidate.
        std::fs::create_dir_all(repo.path().join(".git")).expect("git marker");
        std::fs::create_dir_all(repo.path().join("area")).expect("area dir");
        std::fs::write(repo.path().join("area/spec.md"), "# Spec\n").expect("write spec");

        let effective = effective_for_prompt(
            &repo.path().join("area"),
            "$schema:\n  spec: 'file(eager; required)'\nspec: ./spec.md\n",
        );

        let input = serde_json::json!({ "spec": "./spec.md" });
        let snapshot = input.clone();
        let pending = HashSet::new();
        let outcome = effective.normalize_frontmatter(&input, &pending);
        assert!(outcome.changed, "expected the eager-file value to be rewritten");
        assert_eq!(outcome.value["spec"], serde_json::json!("spec.md"));
        // Decision #3: the caller's input is never mutated.
        assert_eq!(input, snapshot, "normalize_frontmatter must not mutate its input");
    }

    /// A `composition_pending` set containing the eager-file key leaves that
    /// key verbatim while a concrete sibling eager-file value is still
    /// rewritten (Decision #4).
    #[test]
    fn normalize_frontmatter_skips_pending_key_and_rewrites_sibling() {
        let repo = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(repo.path().join(".git")).expect("git marker");
        std::fs::create_dir_all(repo.path().join("area")).expect("area dir");
        std::fs::write(repo.path().join("area/spec.md"), "# Spec\n").expect("write spec");
        std::fs::write(repo.path().join("area/design.md"), "# Design\n").expect("write design");

        let effective = effective_for_prompt(
            &repo.path().join("area"),
            "$schema:\n  spec: 'file(eager; required)'\n  design: 'file(eager)'\nspec: ./spec.md\ndesign: ./design.md\n",
        );

        let input = serde_json::json!({
            "spec": "$(echo spec.md)",
            "design": "./design.md"
        });
        let pending: HashSet<String> = ["spec".to_string()].into_iter().collect();
        let outcome = effective.normalize_frontmatter(&input, &pending);
        assert!(outcome.changed, "concrete sibling must still be rewritten");
        // Pending key is left verbatim.
        assert_eq!(outcome.value["spec"], serde_json::json!("$(echo spec.md)"));
        // Concrete sibling is rewritten.
        assert_eq!(outcome.value["design"], serde_json::json!("design.md"));
    }

    /// Decision #3 regression: `validate_with_positions` keeps the documented
    /// read-only contract even when the schema has eager `file` properties.
    /// The caller's `serde_json::Value` is byte-identical afterward — the
    /// rewrite never runs inside validation.
    #[test]
    fn validate_with_positions_does_not_mutate_eager_file_input() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("spec.md"), "# Spec\n").expect("write spec");

        let effective = effective_for_prompt(
            dir.path(),
            "$schema:\n  spec: 'file(eager; required)'\nspec: ./spec.md\n",
        );

        let input = serde_json::json!({ "spec": "./spec.md" });
        let snapshot = input.clone();
        let report = effective.validate_with_positions(&input, &PositionMap::new());
        assert!(report.valid, "expected valid: {:?}", report.problems);
        assert_eq!(
            input, snapshot,
            "validate_with_positions must not mutate its input, even with eager-file properties",
        );
    }

    // ── Phase 4: wider read-only validation sweep ───────────────────────
    //
    // Complements the unit-level regression above with a wider call-site
    // sweep (spec Integration bullet 4): every validation-only entry point
    // keeps the documented read-only contract. The eager-`file` rewrite is
    // opt-in via `normalize_frontmatter`; it must never fire from a
    // validation-only call.

    /// `EffectiveSchema::validate` (the thin wrapper) does not mutate the
    /// caller's `serde_json::Value`, even when an eager-`file` property's
    /// raw value is eligible for rewriting.
    #[test]
    fn validate_does_not_mutate_eager_file_input() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("spec.md"), "# Spec\n").expect("write spec");

        let effective = effective_for_prompt(
            dir.path(),
            "$schema:\n  spec: 'file(eager; required)'\nspec: ./spec.md\n",
        );

        let input = serde_json::json!({ "spec": "./spec.md" });
        let snapshot = input.clone();
        let report = effective.validate(&input);
        assert!(report.valid, "expected valid: {:?}", report.problems);
        assert_eq!(
            input, snapshot,
            "validate must not mutate its input, even with eager-file properties",
        );
    }

    /// `DarkmatterSchemas::validate` (the top-level API) does not mutate the
    /// document's stored frontmatter, even when an eager-`file` property's
    /// raw value is eligible for rewriting. Compose owns the write-back; the
    /// validation-only library surface stays read-only.
    #[test]
    fn darkmatter_schemas_validate_does_not_mutate_eager_file_frontmatter() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("spec.md"), "# Spec\n").expect("write spec");
        let prompt_path = dir.path().join("prompt.md");
        std::fs::write(
            &prompt_path,
            "---\n$schema:\n  spec: 'file(eager; required)'\nspec: ./spec.md\n---\nbody\n",
        )
        .expect("write prompt");

        let md = Markdown::try_from(prompt_path.as_path()).expect("read prompt");
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(dir.path()));
        let report = api.validate(&md).expect("validate");
        assert!(report.valid, "expected valid: {:?}", report.problems);
        // The stored frontmatter still carries the raw reference — the
        // rewrite only runs through `normalize_frontmatter` / compose.
        assert_eq!(
            md.frontmatter().as_map().get("spec"),
            Some(&serde_json::json!("./spec.md")),
            "DarkmatterSchemas::validate must not rewrite the stored eager-file value",
        );
    }

    #[test]
    fn darkmatter_schemas_validate_does_not_materialize_lazy_file_frontmatter() {
        let dir = tempfile::tempdir().expect("tempdir");
        let prompt_path = dir.path().join("prompt.md");
        std::fs::write(
            &prompt_path,
            "---\n$schema:\n  spec: file\nspec: missing.md\n---\nbody\n",
        )
        .expect("write prompt");

        let md = Markdown::try_from(prompt_path.as_path()).expect("read prompt");
        let report = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(dir.path())).validate(&md).expect("validate");
        assert!(report.valid, "expected valid: {:?}", report.problems);
        assert_eq!(
            md.frontmatter().as_map().get("spec"),
            Some(&serde_json::json!("missing.md")),
            "validation-only APIs must not materialize a lazy caller-style value",
        );
    }

    /// A root-union schema whose committed arm carries an eager-`file`
    /// property also keeps the read-only contract: validation reports
    /// validity without rewriting the caller's value.
    #[test]
    fn validate_root_union_eager_file_arm_does_not_mutate_input() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("spec.md"), "# Spec\n").expect("write spec");

        let effective = effective_for_prompt(
            dir.path(),
            "$schema:\n  - spec: 'file(eager; required)'\n    kind: 'string(required)'\nspec: ./spec.md\nkind: feature\n",
        );

        let input = serde_json::json!({ "spec": "./spec.md", "kind": "feature" });
        let snapshot = input.clone();
        let report = effective.validate(&input);
        assert!(report.valid, "expected valid: {:?}", report.problems);
        assert_eq!(
            input, snapshot,
            "validate must not mutate root-union eager-file input",
        );
    }

    // ── `generated` baseline integration (Phase 2) ─────────────────────────
    //
    // End-to-end at the public `DarkmatterSchemas` API: an authored document
    // omitting a baseline `ctx.today` `generated; required` property validates
    // cleanly (spec semantics point 1), while a wrongly-typed `ctx.today`
    // value fails type validation (point 3).

    /// Builds a baseline `SchemaRoot` from a YAML body.
    fn baseline_from_yaml(yaml_body: &str) -> SchemaRoot {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml_body).expect("yaml parse");
        super::simplified::parse_yaml_schema(&value).expect("baseline schema parse")
    }

    #[test]
    fn baseline_generated_ctx_validates_when_authored_doc_omits_ctx() {
        let baseline = baseline_from_yaml(
            "ctx:\n  today: \"date(generated; required) -> today's date, host-supplied\"",
        );
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()))
            .with_baseline(baseline)
            .expect("baseline converts");

        // Authored document omits `ctx` entirely — validates (spec point 1).
        let md = md_with_schema("title: Hello\n");
        let report = api.validate(&md).expect("validate");
        assert!(
            report.valid,
            "authored doc omitting generated ctx must validate: {:?}",
            report.problems,
        );
    }

    #[test]
    fn baseline_generated_ctx_type_checks_wrongly_typed_value() {
        let baseline = baseline_from_yaml(
            "ctx:\n  today: \"date(generated; required) -> today's date, host-supplied\"",
        );
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()))
            .with_baseline(baseline)
            .expect("baseline converts");

        // Host supplies a wrongly-typed `ctx.today` — fails validation
        // (spec point 3: the non-nullable type semantics of `required` are
        // preserved even though static presence is suppressed). A numeric
        // value against `date` (`{ type: "string", format: "date" }`) is
        // rejected by the format/type check.
        let md = md_with_schema("ctx:\n  today: 42\n");
        let report = api.validate(&md).expect("validate");
        assert!(
            !report.valid,
            "wrongly-typed ctx.today must fail validation: {:?}",
            report.problems,
        );
        assert!(
            report
                .problems
                .iter()
                .any(|p| p.path == "/ctx/today"),
            "expected a problem on /ctx/today: {:?}",
            report.problems,
        );
    }

    #[test]
    fn baseline_generated_ctx_accepts_correctly_typed_value() {
        let baseline = baseline_from_yaml(
            "ctx:\n  today: \"date(generated; required) -> today's date, host-supplied\"",
        );
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()))
            .with_baseline(baseline)
            .expect("baseline converts");

        // Host supplies a correctly-typed `ctx.today` — validates.
        let md = md_with_schema("ctx:\n  today: 2026-07-04\n");
        let report = api.validate(&md).expect("validate");
        assert!(
            report.valid,
            "correctly-typed ctx.today must validate: {:?}",
            report.problems,
        );
    }

    // ── Phase 4: library base-schema accessors ───────────────────────────

    /// `darkmatter_base_schema()` returns a non-empty baseline schema with the
    /// expected top-level properties present (spec testing requirement 1).
    #[test]
    fn darkmatter_base_schema_returns_expected_properties() {
        let schema = super::darkmatter_base_schema();
        let shape = match schema {
            super::SchemaRoot::Single(s) => s,
            other => panic!("expected Single, got {other:?}"),
        };
        assert!(
            !shape.properties.is_empty(),
            "baseline schema must declare properties"
        );
        for key in ["$schema", "title", "ctx"] {
            assert!(
                shape.properties.contains_key(key),
                "baseline schema missing property `{key}`"
            );
        }
    }

    /// `darkmatter_base_json_schema()` returns a compiled JSON Schema that
    /// validates known-good frontmatter and rejects a wrongly-typed `title`
    /// (spec testing requirements 3 and 4).
    #[test]
    fn darkmatter_base_json_schema_validates_known_samples() {
        let json = super::darkmatter_base_json_schema();
        let validator = jsonschema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .build(&json)
            .expect("compiled baseline must build a validator");

        let valid = serde_json::json!({
            "title": "Hello",
            "draft": false,
            "tags": ["a", "b"],
        });
        assert!(
            validator.is_valid(&valid),
            "known-good frontmatter must validate"
        );

        let invalid_title = serde_json::json!({ "title": 42 });
        assert!(
            !validator.is_valid(&invalid_title),
            "wrongly-typed title must be rejected"
        );
    }

    /// `darkmatter_base_json_schema()` caches the converted value so repeated
    /// calls return an equivalent schema without re-parsing the YAML.
    #[test]
    fn darkmatter_base_json_schema_is_cached() {
        let a = super::darkmatter_base_json_schema();
        let b = super::darkmatter_base_json_schema();
        assert_eq!(a, b, "cached JSON schemas must be equal");
    }

    /// The compiled baseline allows unknown user-defined frontmatter keys
    /// (Non-Goal 1; spec testing requirement 5).
    #[test]
    fn darkmatter_base_json_schema_allows_unknown_keys() {
        let json = super::darkmatter_base_json_schema();
        let validator = jsonschema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .build(&json)
            .expect("compiled baseline must build a validator");

        let with_unknown = serde_json::json!({
            "custom_key": 42,
            "my_custom_namespace": { "nested": true },
        });
        assert!(
            validator.is_valid(&with_unknown),
            "unknown user keys must remain accepted"
        );
    }

    /// Document-level `$schema` definitions override baseline properties on
    /// conflict (Non-Goal 5; spec testing requirement 6).
    #[test]
    fn document_schema_overrides_baseline_title() {
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()))
            .with_baseline(super::darkmatter_base_schema())
            .expect("baseline converts");

        // Baseline says `title` is a string. The document redeclares it as a
        // number and supplies a number; validation must follow the document
        // schema, so this is valid.
        let md = md_with_schema("$schema:\n  title: number\ntitle: 42\n");
        let report = api.validate(&md).expect("validate");
        assert!(
            report.valid,
            "document schema should override baseline title type: {:?}",
            report.problems
        );
    }

    // ── R-5 Priority 3: validate_with_options pending + deferral ─────────

    fn effective_number_field() -> EffectiveSchema {
        let md = md_with_schema("$schema:\n  n: number\n");
        DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()))
            .effective_for(&md)
            .expect("effective_for")
            .expect("schema present")
    }

    #[test]
    fn validate_with_options_defers_shell_pending_value() {
        let effective = effective_number_field();
        let instance = serde_json::json!({ "n": "$(echo 1)" });
        let report = effective.validate_with_options(
            &instance,
            &PositionMap::new(),
            &ValidationOptions::default(),
        );
        assert!(report.valid, "shell-pending value deferred: {:?}", report.problems);
        assert_eq!(report.pending.len(), 1);
        assert_eq!(report.pending[0].key, "n");
        assert_eq!(report.pending[0].reason, PendingValueReason::ShellExpression);
        assert_eq!(report.pending[0].path.segments(), ["n"]);
    }

    #[test]
    fn validate_with_options_classifies_template_pending_value() {
        let effective = effective_number_field();
        let instance = serde_json::json!({ "n": "{{ x }}" });
        let report = effective.validate_with_options(
            &instance,
            &PositionMap::new(),
            &ValidationOptions::default(),
        );
        assert!(report.valid, "template-pending value deferred: {:?}", report.problems);
        assert_eq!(report.pending.len(), 1);
        assert_eq!(report.pending[0].reason, PendingValueReason::UnresolvedTemplate);
    }

    #[test]
    fn validate_with_options_judges_a_malformed_template_as_written() {
        let effective = effective_number_field();
        let instance = serde_json::json!({ "n": "{{ 1 + }}" });
        let report = effective.validate_with_options(
            &instance,
            &PositionMap::new(),
            &ValidationOptions::default(),
        );
        assert!(!report.valid, "a malformed template is never deferred");
        assert!(report.problems.iter().any(|p| p.path == "/n"), "{:?}", report.problems);
    }

    #[test]
    fn validate_with_options_judges_shell_text_compose_never_expands() {
        let effective = effective_number_field();
        for value in [serde_json::json!(["$(echo 1)"]), serde_json::json!("1 $(echo 1)")] {
            let instance = serde_json::json!({ "n": value });
            let report = effective.validate_with_options(
                &instance,
                &PositionMap::new(),
                &ValidationOptions::default(),
            );
            assert!(!report.valid, "{instance}: never-expanded shell text is judged");
            assert!(report.pending.is_empty(), "{instance}: {:?}", report.pending);
        }
    }

    #[test]
    fn validate_with_options_report_policy_keeps_pending_problem() {
        let effective = effective_number_field();
        let instance = serde_json::json!({ "n": "$(echo 1)" });
        let options = ValidationOptions {
            pending_policy: PendingPolicy::Report,
            excluded_keys: HashSet::new(),
        };
        let report =
            effective.validate_with_options(&instance, &PositionMap::new(), &options);
        assert!(!report.valid, "Report policy keeps the type problem");
        assert!(report.problems.iter().any(|p| p.path == "/n"));
        // The pending value is still surfaced as data.
        assert_eq!(report.pending.len(), 1);
    }

    #[test]
    fn validate_with_options_excludes_caller_owned_key() {
        let effective = effective_number_field();
        let instance = serde_json::json!({ "n": "not-a-number" });
        let options = ValidationOptions {
            pending_policy: PendingPolicy::Defer,
            excluded_keys: ["n".to_string()].into_iter().collect(),
        };
        let report =
            effective.validate_with_options(&instance, &PositionMap::new(), &options);
        assert!(report.valid, "excluded key's problem is dropped: {:?}", report.problems);
        // Excluded, not pending — the raw value holds no `$(...)`/`{{ }}`.
        assert!(report.pending.is_empty());
    }

    #[test]
    fn validate_with_options_coerced_value_passes_with_no_pending() {
        let effective = effective_number_field();
        // A coercible string is not pending and validates after coercion.
        let instance = serde_json::json!({ "n": "42" });
        let report = effective.validate_with_options(
            &instance,
            &PositionMap::new(),
            &ValidationOptions::default(),
        );
        assert!(report.valid, "{:?}", report.problems);
        assert!(report.pending.is_empty());
    }

    #[test]
    fn plain_validate_report_has_empty_pending() {
        // Compose-parity: the non-options entry points never populate `pending`.
        let effective = effective_number_field();
        let report = effective.validate(&serde_json::json!({ "n": 1 }));
        assert!(report.pending.is_empty());
    }

    // ── R-5 Priority 2: schema origins ──────────────────────────────────

    #[test]
    fn origins_attribute_document_and_baseline_properties() {
        let md = md_with_schema("$schema:\n  title: 'string(required)'\ntitle: hi\n");
        let baseline = baseline_from_yaml("owner: 'string(required)'");
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir())).with_baseline(baseline).unwrap();
        let effective = api.effective_for(&md).unwrap().unwrap();
        assert_eq!(
            effective.origins.get("title").map(|o| o.kind),
            Some(SchemaOriginKind::Document),
        );
        assert_eq!(
            effective.origins.get("owner").map(|o| o.kind),
            Some(SchemaOriginKind::Baseline),
        );
    }

    #[test]
    fn origins_referenced_file_carries_uri() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("schema.yaml"),
            "$schema:\n  title: 'string(required)'\n",
        )
        .unwrap();
        let doc_path = dir.path().join("doc.md");
        std::fs::write(
            &doc_path,
            "---\n$schema: ./schema.yaml\ntitle: Hello\n---\nbody\n",
        )
        .unwrap();
        let md = Markdown::try_from(doc_path.as_path()).unwrap();
        let effective = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(dir.path()))
            .effective_for(&md)
            .unwrap()
            .unwrap();
        let origin = effective.origins.get("title").expect("title origin");
        assert_eq!(origin.kind, SchemaOriginKind::ReferencedFile);
        assert!(
            origin.file().map(|p| p.ends_with("schema.yaml")).unwrap_or(false),
            "expected the referenced file path, got {:?}",
            origin.source,
        );
    }

    #[test]
    fn dependencies_surface_import_and_example_edges() {
        // A `$schema` that pulls in a `Name@file` import and an `example(...)`
        // artifact surfaces both resolved paths on `EffectiveSchema::dependencies`
        // so a warm cache can invalidate the schema when either file changes.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("types.yaml"), "$schema:\n  type: 'enum(a, b)'\n")
            .expect("write types");
        std::fs::write(
            dir.path().join("today-example.yaml"),
            "kind: example\ninvocation: \"{{ ctx.today }}\"\nreturns: \"2024-12-25\"\ndescription: d\n",
        )
        .expect("write example");
        let md = prompt_with_source(
            dir.path(),
            "$schema:\n  value: type@./types.yaml\n  today: \"date(example(./today-example.yaml))\"\nvalue: a\n",
        );
        let effective = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(dir.path()))
            .effective_for(&md)
            .unwrap()
            .unwrap();
        let deps = effective.dependencies();
        assert!(
            deps.iter().any(|p| p.ends_with("types.yaml")),
            "import edge missing: {deps:?}",
        );
        assert!(
            deps.iter().any(|p| p.ends_with("today-example.yaml")),
            "example edge missing: {deps:?}",
        );
        // The union is sorted and deduplicated.
        let mut expected = deps.to_vec();
        expected.sort();
        expected.dedup();
        assert_eq!(deps, expected.as_slice());
    }

    #[test]
    fn dependencies_surface_referenced_schema_file() {
        // A `$schema: ./schema.yaml` document depends on that file's content —
        // editing the referenced schema's own type must invalidate a warm cache,
        // so the resolved file surfaces on `EffectiveSchema::dependencies`.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("schema.yaml"),
            "$schema:\n  title: 'string(required)'\n",
        )
        .expect("write schema");
        let md = prompt_with_source(dir.path(), "$schema: ./schema.yaml\ntitle: hi\n");
        let effective = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(dir.path()))
            .effective_for(&md)
            .unwrap()
            .unwrap();
        let deps = effective.dependencies();
        assert!(
            deps.iter().any(|p| p.ends_with("schema.yaml")),
            "referenced schema file missing: {deps:?}",
        );
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn schema_union_reuses_snapshot_environment_for_nested_source() {
        let request = tempfile::tempdir().expect("request tempdir");
        let nested = request.path().join("nested");
        std::fs::create_dir_all(&nested).expect("create nested source directory");
        let schema = request.path().join("captured-schema.yaml");
        std::fs::write(&schema, "$schema:\n  title: 'string(required)'\n")
            .expect("write schema");
        let ambient = tempfile::tempdir().expect("ambient tempdir");
        let prior = std::env::var_os("DARKMATTER_SCHEMA_ROOT");
        let mut env = std::collections::HashMap::new();
        env.insert(
            "DARKMATTER_SCHEMA_ROOT".to_string(),
            request.path().display().to_string(),
        );
        let snapshot = biscuit_file::FileResolutionContext::new(request.path()).with_env(env);
        let content = "---\n$schema:\n  - '{{DARKMATTER_SCHEMA_ROOT}}/captured-schema.yaml'\ntitle: captured\n---\nbody\n";
        let md: Markdown = content.into();
        let md = md.with_source(crate::markdown::compose::ComposeSource::File(
            nested.join("child.md"),
        ));

        // SAFETY: this test is serialized while mutating process-global state.
        unsafe { std::env::set_var("DARKMATTER_SCHEMA_ROOT", ambient.path()) };
        let effective = DarkmatterSchemas::new(snapshot)
            .effective_for(&md);
        match prior {
            Some(value) => unsafe { std::env::set_var("DARKMATTER_SCHEMA_ROOT", value) },
            None => unsafe { std::env::remove_var("DARKMATTER_SCHEMA_ROOT") },
        }

        let effective = effective.unwrap().expect("effective schema");
        assert!(
            effective
                .dependencies()
                .iter()
                .any(|path| path.ends_with("captured-schema.yaml")),
            "schema dependency missing: {:?}",
            effective.dependencies(),
        );
    }

    #[test]
    fn dependencies_empty_without_imports_or_examples() {
        // The no-dependency fast path: a plain inline `$schema` records no edges.
        let md = md_with_schema("$schema:\n  title: 'string(required)'\ntitle: hi\n");
        let effective = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()))
            .effective_for(&md)
            .unwrap()
            .unwrap();
        assert!(effective.dependencies().is_empty());
    }
}

/// Phase 4 — effective-schema assembly integration tests.
///
/// These tests exercise the full [`DarkmatterSchemas::effective_for`] assembly
/// with trigger schemas: precedence (baseline → triggers → document),
/// shadowing-before-matching, non-mergeable payload rejection, envelope/payload
/// cycle detection, origin attribution, and complete dependency collection.
#[cfg(test)]
mod phase4_trigger_assembly {
    use super::*;
    use simplified_schema::{Constraint, SchemaOriginKind};
    use crate::markdown::Markdown;
    use std::fs;
    use tempfile::TempDir;

    /// Extracts the error from a `Result<Option<EffectiveSchema>, _>` without
    /// requiring `EffectiveSchema: Debug`.
    fn unwrap_effective_err(
        result: Result<Option<EffectiveSchema>, SchemaError>,
    ) -> SchemaError {
        match result {
            Err(e) => e,
            Ok(_) => panic!("expected an error but got Ok"),
        }
    }

    /// The hermetic context of a document in `doc_dir` of repository `root`:
    /// no home directory and no environment, so no user-level schema root.
    /// A document under `{root}/pkg` belongs to the package `{root}/pkg`.
    fn doc_context(root: &Path, doc_path: &Path) -> biscuit_file::FileResolutionContext {
        let doc_dir = doc_path.parent().unwrap();
        let context = biscuit_file::FileResolutionContext::from_snapshot(
            doc_dir,
            None,
            std::collections::HashMap::new(),
        )
        .with_repository_root(root);
        if doc_path.starts_with(root.join("pkg")) {
            context.with_package_root(root.join("pkg"))
        } else {
            context
        }
    }

    fn repo_fixture() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        dir
    }

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    /// Loads a Markdown document from a file path so it carries a real
    /// `ComposeSource::File` (needed for trigger path normalization).
    fn md_from_file(path: &Path) -> Markdown {
        Markdown::try_from(path).unwrap()
    }

    /// A Claudine-shaped trigger: activates on `prompt: string(required)`,
    /// layers a payload declaring `model: string(required)`.
    const CLAUDINE_TRIGGER: &str = "kind: schema-trigger\n\
        match:\n  prompt: string(required)\n\
        $schema: claudine.yaml\n";
    const CLAUDINE_PAYLOAD: &str = "$schema:\n  model: 'string(required)'\n";

    // ── Precedence: baseline → triggers → document ──────────────────────

    #[test]
    fn trigger_layers_between_baseline_and_document() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(&root.join("schemas/claudine.trigger.yaml"), CLAUDINE_TRIGGER);
        write(&root.join("schemas/claudine.yaml"), CLAUDINE_PAYLOAD);

        // Document matches the trigger (`prompt` present) and has its own
        // `$schema` declaring `title`.
        let doc_path = root.join("doc.md");
        write(
            &doc_path,
            "---\n\
             $schema:\n  title: 'string(required)'\n\
             prompt: hello\n\
             ---\nbody\n",
        );

        let baseline = SchemaRoot::Single(SchemaShape {
            properties: {
                let mut m = indexmap::IndexMap::new();
                m.insert(
                    "owner".into(),
                    PropertyDef::Single(PropertyAtom {
                        ty: TypeExpr::Primitive(SimplifiedType::String),
                        is_array: false,
                        constraints: vec![],
                        array_constraints: vec![],
                        description: None,
                    }),
                );
                m
            },
            ..Default::default()
        });
        let api = DarkmatterSchemas::new(doc_context(root, &doc_path))
            .with_baseline(baseline)
            .unwrap()
            .with_trigger_discovery()
            .unwrap();

        let effective = api.effective_for(&md_from_file(&doc_path))
            .unwrap()
            .unwrap();
        let props = effective.json_schema["properties"].as_object().unwrap();
        // Baseline property present.
        assert!(props.contains_key("owner"), "baseline `owner` must survive: {props:?}");
        // Trigger payload property present.
        assert!(props.contains_key("model"), "trigger `model` must be layered: {props:?}");
        // Document schema property present.
        assert!(props.contains_key("title"), "document `title` must be present: {props:?}");
    }

    #[test]
    fn document_schema_wins_over_trigger_on_conflict() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(&root.join("schemas/claudine.trigger.yaml"), CLAUDINE_TRIGGER);
        // Payload declares `title` as a number.
        write(
            &root.join("schemas/claudine.yaml"),
            "$schema:\n  title: 'number(required)'\n",
        );

        let doc_path = root.join("doc.md");
        // Document also declares `title` (as string) — document wins.
        write(
            &doc_path,
            "---\n\
             $schema:\n  title: 'string(required)'\n\
             prompt: hello\n\
             title: hello\n\
             ---\nbody\n",
        );

        let api = DarkmatterSchemas::new(doc_context(root, &doc_path))
            .with_trigger_discovery()
            .unwrap();

        let effective = api.effective_for(&md_from_file(&doc_path))
            .unwrap()
            .unwrap();
        // Document's `title` (string) wins over trigger's `title` (number).
        let title = &effective.json_schema["properties"]["title"];
        // The document made `title` a required string; the trigger's number
        // definition is fully replaced.
        assert!(
            title.get("anyOf").is_some() || title.get("type").is_some(),
            "document `title` must override trigger: {title}"
        );
        // Validating with a string title should pass.
        let report = api.validate(&md_from_file(&doc_path)).unwrap();
        assert!(report.valid, "string title should validate against document-wins schema: {:?}", report.problems);
    }

    #[test]
    fn trigger_activates_and_merges_without_document_schema() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(&root.join("schemas/claudine.trigger.yaml"), CLAUDINE_TRIGGER);
        write(&root.join("schemas/claudine.yaml"), CLAUDINE_PAYLOAD);

        // Document matches the trigger but has no `$schema` of its own.
        let doc_path = root.join("doc.md");
        write(
            &doc_path,
            "---\n\
             prompt: hello\n\
             model: gpt-4\n\
             ---\nbody\n",
        );

        let api = DarkmatterSchemas::new(doc_context(root, &doc_path))
            .with_trigger_discovery()
            .unwrap();

        let effective = api.effective_for(&md_from_file(&doc_path))
            .unwrap()
            .unwrap();
        let props = effective.json_schema["properties"].as_object().unwrap();
        assert!(
            props.contains_key("model"),
            "trigger payload `model` must be present even without document $schema"
        );
    }

    #[test]
    fn trigger_does_not_activate_on_non_matching_document() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(&root.join("schemas/claudine.trigger.yaml"), CLAUDINE_TRIGGER);
        write(&root.join("schemas/claudine.yaml"), CLAUDINE_PAYLOAD);

        // Document does NOT have `prompt` — trigger does not activate.
        let doc_path = root.join("doc.md");
        write(
            &doc_path,
            "---\n\
             title: hello\n\
             ---\nbody\n",
        );

        let api = DarkmatterSchemas::new(doc_context(root, &doc_path))
            .with_trigger_discovery()
            .unwrap();

        // No $schema, no matching trigger → None.
        let effective = api.effective_for(&md_from_file(&doc_path)).unwrap();
        assert!(effective.is_none(), "non-matching document should yield no effective schema");
    }

    // ── Shadowing before matching ───────────────────────────────────────

    #[test]
    fn shadowing_applied_before_matching() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("pkg/schemas")).unwrap();
        fs::create_dir_all(root.join("schemas")).unwrap();

        // Near root: matches on `prompt`.
        write(
            &root.join("pkg/schemas/claudine.trigger.yaml"),
            CLAUDINE_TRIGGER,
        );
        write(&root.join("pkg/schemas/claudine.yaml"), CLAUDINE_PAYLOAD);

        // Far root (shadowed): matches on `title` — should NEVER activate.
        write(
            &root.join("schemas/claudine.trigger.yaml"),
            "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema: other.yaml\n",
        );
        write(
            &root.join("schemas/other.yaml"),
            "$schema:\n  shadowed_prop: 'string(required)'\n",
        );

        let doc_path = root.join("pkg/sub/doc.md");
        write(
            &doc_path,
            "---\n\
             prompt: hello\n\
             model: gpt-4\n\
             ---\nbody\n",
        );

        let api = DarkmatterSchemas::new(doc_context(root, &doc_path))
            .with_trigger_discovery()
            .unwrap();

        let effective = api.effective_for(&md_from_file(&doc_path))
            .unwrap()
            .unwrap();
        let props = effective.json_schema["properties"].as_object().unwrap();
        assert!(
            props.contains_key("model"),
            "near trigger payload must activate: {props:?}"
        );
        assert!(
            !props.contains_key("shadowed_prop"),
            "shadowed far trigger must NOT activate: {props:?}"
        );
    }

    // ── Non-mergeable payload rejection ─────────────────────────────────

    /// The one finding of discovery for `doc_path`, whose trigger file is
    /// `name`; discovery itself succeeds.
    fn only_finding(root: &Path, doc_path: &Path, name: &str) -> (DarkmatterSchemas, triggers::InvalidTriggerFinding) {
        let api = DarkmatterSchemas::new(doc_context(root, doc_path))
            .with_trigger_discovery()
            .expect("an invalid trigger never fails discovery");
        let findings: Vec<_> = api.discovery_findings().iter().filter(|finding| {
            finding.source().file_name() == Some(name)
        }).cloned().collect();
        let [finding] = findings.as_slice() else {
            panic!("one finding for {name} expected: {:?}", api.discovery_findings());
        };
        assert!(
            api.trigger_registry().unwrap().triggers.iter().all(|trigger| trigger.source.file_name() != Some(name)),
            "{name} must not be installed"
        );
        let finding = finding.clone();
        (api, finding)
    }

    #[test]
    fn non_mergeable_root_union_payload_rejected() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(
            &root.join("schemas/bad.trigger.yaml"),
            "kind: schema-trigger\n\
             match:\n  prompt: string(required)\n\
             $schema:\n  - a: string(required)\n  - b: string(required)\n",
        );

        let doc_path = root.join("doc.md");
        write(&doc_path, "---\nprompt: hello\n---\nbody\n");

        let (_, finding) = only_finding(root, &doc_path, "bad.trigger.yaml");
        assert!(
            matches!(finding.cause(), SchemaError::TriggerPayloadNotMergeable { .. }),
            "root union payload must be rejected at scan time: {finding:?}"
        );
    }

    // ── Envelope + payload resolve without self-reference ───────────────

    #[test]
    fn claudine_shaped_fixture_resolves_cleanly() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(&root.join("schemas/claudine.trigger.yaml"), CLAUDINE_TRIGGER);
        write(&root.join("schemas/claudine.yaml"), CLAUDINE_PAYLOAD);

        let doc_path = root.join("doc.md");
        write(
            &doc_path,
            "---\n\
             prompt: hello\n\
             model: gpt-4\n\
             ---\nbody\n",
        );

        let api = DarkmatterSchemas::new(doc_context(root, &doc_path))
            .with_trigger_discovery()
            .unwrap();

        let report = api.validate(&md_from_file(&doc_path)).unwrap();
        assert!(
            report.valid,
            "claudine-shaped fixture must validate: {:?}",
            report.problems
        );
    }

    // ── Direct and indirect cycles ──────────────────────────────────────

    #[test]
    fn direct_payload_self_reference_cycle_fails() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(
            &root.join("schemas/self.trigger.yaml"),
            "kind: schema-trigger\n\
             match:\n  prompt: string(required)\n\
             $schema: self.trigger.yaml\n",
        );

        let doc_path = root.join("doc.md");
        write(&doc_path, "---\nprompt: hello\n---\nbody\n");

        let (_, finding) = only_finding(root, &doc_path, "self.trigger.yaml");
        assert!(
            matches!(finding.cause(), SchemaError::TriggerPayloadCycle { .. }),
            "self-referencing payload must be skipped at scan time: {finding:?}"
        );
    }

    #[test]
    fn payload_referencing_another_trigger_fails() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        // a.trigger.yaml's payload references b.trigger.yaml (a trigger file).
        write(
            &root.join("schemas/a.trigger.yaml"),
            "kind: schema-trigger\n\
             match:\n  prompt: string(required)\n\
             $schema: b.trigger.yaml\n",
        );
        write(
            &root.join("schemas/b.trigger.yaml"),
            CLAUDINE_TRIGGER,
        );
        write(&root.join("schemas/claudine.yaml"), CLAUDINE_PAYLOAD);

        let doc_path = root.join("doc.md");
        write(&doc_path, "---\nprompt: hello\n---\nbody\n");

        let (api, finding) = only_finding(root, &doc_path, "a.trigger.yaml");
        assert!(
            matches!(finding.cause(), SchemaError::TriggerPayloadCycle { .. }),
            "payload referencing another trigger must be skipped at scan time: {finding:?}"
        );
        // The referenced trigger is valid and still applies.
        let effective = api.effective_for(&md_from_file(&doc_path)).unwrap().unwrap();
        assert!(effective.json_schema["properties"].as_object().unwrap().contains_key("model"));
    }

    /// Discovery skipping a trigger does not make it a schema: a document
    /// `$schema` naming the same ignored envelope still fails.
    #[test]
    fn document_schema_referencing_an_ignored_trigger_file_still_fails() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(
            &root.join("schemas/memory.trigger.yaml"),
            "kind: schema-trigger\nname: Memory Trigger\ndescription: TODO\nmatch:\n",
        );
        let doc_path = root.join("doc.md");
        write(&doc_path, "---\n$schema: ./schemas/memory.trigger.yaml\n---\nbody\n");

        let (api, _) = only_finding(root, &doc_path, "memory.trigger.yaml");
        let error = api.validate(&md_from_file(&doc_path)).expect_err("an authored trigger reference fails");
        assert!(
            matches!(error, SchemaError::TriggerSchemaReferenced { .. }),
            "{error:?}"
        );
    }

    // ── Document $schema referencing a trigger file ─────────────────────

    #[test]
    fn document_schema_referencing_trigger_file_fails() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(&root.join("schemas/claudine.trigger.yaml"), CLAUDINE_TRIGGER);
        write(&root.join("schemas/claudine.yaml"), CLAUDINE_PAYLOAD);

        // Document directly references the trigger envelope as its $schema.
        let doc_path = root.join("doc.md");
        write(
            &doc_path,
            "---\n\
             $schema: ./schemas/claudine.trigger.yaml\n\
             ---\nbody\n",
        );

        let api = DarkmatterSchemas::new(doc_context(root, &doc_path))
            .with_trigger_discovery()
            .unwrap();

        let err = unwrap_effective_err(api.effective_for(&md_from_file(&doc_path)));
        match err {
            SchemaError::TriggerSchemaReferenced { suggestion, .. } => {
                assert!(
                    suggestion.contains("claudine.yaml"),
                    "error should suggest the payload: {suggestion}"
                );
            }
            other => panic!("expected TriggerSchemaReferenced, got {other:?}"),
        }
    }

    // ── Origin attribution ──────────────────────────────────────────────

    #[test]
    fn origins_attribute_trigger_properties() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(&root.join("schemas/claudine.trigger.yaml"), CLAUDINE_TRIGGER);
        write(&root.join("schemas/claudine.yaml"), CLAUDINE_PAYLOAD);

        let doc_path = root.join("doc.md");
        write(
            &doc_path,
            "---\n\
             $schema:\n  title: 'string(required)'\n\
             prompt: hello\n\
             title: hi\n\
             ---\nbody\n",
        );

        let api = DarkmatterSchemas::new(doc_context(root, &doc_path))
            .with_trigger_discovery()
            .unwrap();

        let effective = api.effective_for(&md_from_file(&doc_path))
            .unwrap()
            .unwrap();
        // `title` came from the document $schema.
        assert_eq!(
            effective.origins.get("title").map(|o| o.kind),
            Some(SchemaOriginKind::Document),
            "title origin must be Document"
        );
        // `model` came from the trigger payload.
        let model_origin = effective.origins.get("model");
        assert!(
            model_origin.is_some(),
            "model must have an origin entry"
        );
        assert_eq!(
            model_origin.map(|o| o.kind),
            Some(SchemaOriginKind::Trigger),
            "model origin must be Trigger"
        );
        let trigger_uri = model_origin.and_then(SchemaOrigin::file);
        assert!(
            trigger_uri.is_some_and(|u| u.to_string_lossy().contains("claudine.trigger.yaml")),
            "trigger origin must point at the envelope file: {trigger_uri:?}"
        );
    }

    /// An authored `$schema` that redeclares a trigger property replaces it
    /// whole and takes over its origin, so a host that keys a policy on the
    /// winning declaration sees the document, not the trigger.
    #[test]
    fn authored_redeclaration_takes_over_a_trigger_property_origin() {
        let repo = repo_fixture();
        let root = repo.path();
        write(&root.join("schemas/claudine.trigger.yaml"), CLAUDINE_TRIGGER);
        write(&root.join("schemas/claudine.yaml"), CLAUDINE_PAYLOAD);

        let doc_path = root.join("doc.md");
        write(
            &doc_path,
            "---\n$schema:\n  model: number\nprompt: hello\nmodel: 42\n---\nbody\n",
        );
        let api = DarkmatterSchemas::new(doc_context(root, &doc_path))
            .with_trigger_discovery()
            .unwrap();
        let doc = md_from_file(&doc_path);

        let effective = api.effective_for(&doc).unwrap().unwrap();
        assert_eq!(
            effective.origins.get("model").map(|o| o.kind),
            Some(SchemaOriginKind::Document),
        );
        let report = api.validate(&doc).unwrap();
        assert!(report.valid, "the authored number type wins: {:?}", report.problems);
    }

    /// A root-union authored `$schema` receives every matching trigger layer in
    /// each arm, and the merged schema records no per-property origins, because
    /// there are no top-level properties to attribute.
    #[test]
    fn root_union_document_schema_merges_trigger_per_arm_without_origins() {
        let repo = repo_fixture();
        let root = repo.path();
        write(&root.join("schemas/claudine.trigger.yaml"), CLAUDINE_TRIGGER);
        write(&root.join("schemas/claudine.yaml"), "$schema:\n  model: string\n");
        let union = "$schema:\n  - title: string(required)\n  - name: string(required)\n";

        let doc_path = root.join("doc.md");
        write(&doc_path, &format!("---\n{union}prompt: hello\ntitle: hi\nmodel: [1]\n---\nbody\n"));
        let api = DarkmatterSchemas::new(doc_context(root, &doc_path))
            .with_trigger_discovery()
            .unwrap();
        let doc = md_from_file(&doc_path);

        let effective = api.effective_for(&doc).unwrap().unwrap();
        assert!(effective.origins.is_empty(), "root union has no attributed properties");
        assert!(effective.json_schema.get("anyOf").is_some(), "the root stays a union");
        let report = api.validate(&doc).unwrap();
        assert!(!report.valid, "the trigger's `model: string` applies in every arm");

        write(&doc_path, &format!("---\n{union}prompt: hello\nname: n\nmodel: m\n---\nbody\n"));
        let report = api.validate(&md_from_file(&doc_path)).unwrap();
        assert!(report.valid, "a conforming value passes either arm: {:?}", report.problems);
    }

    // ── Complete dependency collection ──────────────────────────────────

    #[test]
    fn dependencies_include_trigger_envelope_and_payload() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(&root.join("schemas/claudine.trigger.yaml"), CLAUDINE_TRIGGER);
        write(&root.join("schemas/claudine.yaml"), CLAUDINE_PAYLOAD);

        let doc_path = root.join("doc.md");
        write(
            &doc_path,
            "---\n\
             $schema: ./local.yaml\n\
             prompt: hello\n\
             ---\nbody\n",
        );
        write(
            &root.join("local.yaml"),
            "$schema:\n  doc_prop: 'string(required)'\n",
        );

        let api = DarkmatterSchemas::new(doc_context(root, &doc_path))
            .with_trigger_discovery()
            .unwrap();

        let effective = api.effective_for(&md_from_file(&doc_path))
            .unwrap()
            .unwrap();
        let deps = effective.dependencies();

        // The trigger envelope file.
        let has_envelope = deps.iter().any(|d| d
            .to_string_lossy()
            .contains("claudine.trigger.yaml"));
        assert!(has_envelope, "dependencies must include the trigger envelope: {deps:?}");

        // The payload file.
        let has_payload = deps
            .iter()
            .any(|d| d.to_string_lossy().contains("claudine.yaml") && !d.to_string_lossy().contains(".trigger"));
        assert!(has_payload, "dependencies must include the payload file: {deps:?}");

        // The document's own $schema file.
        let has_doc_schema = deps
            .iter()
            .any(|d| d.to_string_lossy().contains("local.yaml"));
        assert!(has_doc_schema, "dependencies must include the document $schema: {deps:?}");
    }

    // ── with_trigger_registry (prebuilt) ────────────────────────────────

    #[test]
    fn prebuilt_registry_works() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(&root.join("schemas/claudine.trigger.yaml"), CLAUDINE_TRIGGER);
        write(&root.join("schemas/claudine.yaml"), CLAUDINE_PAYLOAD);

        let doc_path = root.join("doc.md");
        write(
            &doc_path,
            "---\n\
             prompt: hello\n\
             model: gpt-4\n\
             ---\nbody\n",
        );

        // Build the registry separately (simulating DMLS per-boundary caching).
        let registry = triggers::scan(&doc_context(root, &doc_path)).unwrap();
        let api = DarkmatterSchemas::new(doc_context(root, &doc_path)).with_trigger_registry(registry);

        let effective = api.effective_for(&md_from_file(&doc_path))
            .unwrap()
            .unwrap();
        let props = effective.json_schema["properties"].as_object().unwrap();
        assert!(
            props.contains_key("model"),
            "prebuilt registry must activate trigger: {props:?}"
        );
    }

    // ── new() never scans disk ──────────────────────────────────────────

    #[test]
    fn new_never_scans_disk() {
        // Even with trigger files on disk, new() + effective_for does not
        // discover them.
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(&root.join("schemas/claudine.trigger.yaml"), CLAUDINE_TRIGGER);
        write(&root.join("schemas/claudine.yaml"), CLAUDINE_PAYLOAD);

        let doc_path = root.join("doc.md");
        write(
            &doc_path,
            "---\n\
             prompt: hello\n\
             ---\nbody\n",
        );

        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(root));
        // No trigger discovery → no effective schema (no $schema, no baseline).
        let effective = api.effective_for(&md_from_file(&doc_path)).unwrap();
        assert!(
            effective.is_none(),
            "new() must not discover triggers implicitly"
        );
    }

    // ── Bare-name resolution through trigger roots ──────────────────────

    #[test]
    fn document_schema_bare_name_resolves_via_trigger_roots() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(&root.join("schemas/claudine.trigger.yaml"), CLAUDINE_TRIGGER);
        write(&root.join("schemas/claudine.yaml"), CLAUDINE_PAYLOAD);

        // Document uses bare-name $schema (no path) — resolves via schema roots.
        let doc_path = root.join("doc.md");
        write(
            &doc_path,
            "---\n\
             $schema: claudine.yaml\n\
             model: gpt-4\n\
             ---\nbody\n",
        );

        let api = DarkmatterSchemas::new(doc_context(root, &doc_path))
            .with_trigger_discovery()
            .unwrap();

        let effective = api.effective_for(&md_from_file(&doc_path))
            .unwrap()
            .unwrap();
        let props = effective.json_schema["properties"].as_object().unwrap();
        assert!(
            props.contains_key("model"),
            "bare-name $schema must resolve via trigger schema roots: {props:?}"
        );
    }

    // ── Programmatic baseline not double-applied ───────────────────────

    #[test]
    fn programmatic_baseline_not_re_applied_by_identical_trigger() {
        // The Checkpoint 4 contract: a programmatically supplied baseline
        // must not be re-applied by a trigger that resolves to the same schema.
        // The merge is idempotent on property keys, so the trigger layer wins
        // and the baseline's identical definition is replaced (not duplicated).
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(&root.join("schemas/claudine.trigger.yaml"), CLAUDINE_TRIGGER);
        write(&root.join("schemas/claudine.yaml"), CLAUDINE_PAYLOAD);

        let doc_path = root.join("doc.md");
        write(
            &doc_path,
            "---\n\
             prompt: hello\n\
             model: gpt-4\n\
             ---\nbody\n",
        );

        // Baseline declares the same `model` property.
        let baseline = SchemaRoot::Single(SchemaShape {
            properties: {
                let mut m = indexmap::IndexMap::new();
                m.insert(
                    "model".into(),
                    PropertyDef::Single(PropertyAtom {
                        ty: TypeExpr::Primitive(SimplifiedType::String),
                        is_array: false,
                        constraints: vec![Constraint::Required],
                        array_constraints: vec![],
                        description: None,
                    }),
                );
                m
            },
            ..Default::default()
        });
        let api = DarkmatterSchemas::new(doc_context(root, &doc_path))
            .with_baseline(baseline)
            .unwrap()
            .with_trigger_discovery()
            .unwrap();

        let effective = api.effective_for(&md_from_file(&doc_path))
            .unwrap()
            .unwrap();
        let props = effective.json_schema["properties"].as_object().unwrap();
        // `model` appears once (not duplicated).
        assert!(props.contains_key("model"));
        assert_eq!(
            props.keys().filter(|k| *k == "model").count(),
            1,
            "property must not be duplicated"
        );
        // Validates — model is supplied.
        let report = api.validate(&md_from_file(&doc_path)).unwrap();
        assert!(report.valid, "expected valid: {:?}", report.problems);
    }

    // ── Editor property shape: trigger layers ───────────────────────────

    /// A repository whose `schemas/` root holds three triggers: `a` and `b`
    /// match `prompt` (in that registry order) and both declare `shared`;
    /// `c` matches only `sequence`.
    fn three_trigger_repo() -> TempDir {
        let repo = repo_fixture();
        let root = repo.path();
        let trigger = |payload: &str, key: &str| {
            format!("kind: schema-trigger\nmatch:\n  {key}: string(required)\n$schema: {payload}\n")
        };
        write(&root.join("schemas/a.trigger.yaml"), &trigger("a.yaml", "prompt"));
        write(&root.join("schemas/b.trigger.yaml"), &trigger("b.yaml", "prompt"));
        write(&root.join("schemas/c.trigger.yaml"), &trigger("c.yaml", "sequence"));
        write(
            &root.join("schemas/a.yaml"),
            "$schema:\n  alpha: 'string -> from a'\n  shared: 'number -> from a'\n",
        );
        write(
            &root.join("schemas/b.yaml"),
            "$schema:\n  beta: 'boolean -> from b'\n  shared: 'string -> from b'\n",
        );
        write(&root.join("schemas/c.yaml"), "$schema:\n  gamma: 'string -> from c'\n");
        repo
    }

    fn effective_of(root: &Path, frontmatter: &str) -> (EffectiveSchema, Value) {
        let doc_path = root.join("doc.md");
        write(&doc_path, &format!("---\n{frontmatter}---\nbody\n"));
        let md = md_from_file(&doc_path);
        let effective = DarkmatterSchemas::new(doc_context(root, &doc_path))
            .with_trigger_discovery()
            .unwrap()
            .effective_for(&md)
            .unwrap()
            .expect("an effective schema");
        (effective, frontmatter_as_json(&md))
    }

    /// `(type keyword, description)` of a single-atom primitive property.
    fn declared(shape: &SchemaShape, name: &str) -> Option<(&'static str, String)> {
        match shape.properties.get(name)? {
            PropertyDef::Single(PropertyAtom { ty: TypeExpr::Primitive(ty), description, .. }) => {
                Some((ty.as_keyword(), description.clone().unwrap_or_default()))
            }
            other => panic!("`{name}` is not a single primitive: {other:?}"),
        }
    }

    #[test]
    fn property_shape_layers_matching_triggers_in_registry_order() {
        let repo = three_trigger_repo();
        let (effective, frontmatter) = effective_of(repo.path(), "prompt: hi\n");

        let shape = effective.property_shape(&[], &frontmatter);

        assert!(effective.simplified.is_none(), "no authored `$schema`");
        assert_eq!(effective.trigger_shapes().len(), 2, "a and b match; c does not");
        assert_eq!(declared(&shape, "alpha"), Some(("string", "from a".into())));
        assert_eq!(declared(&shape, "beta"), Some(("boolean", "from b".into())));
        assert_eq!(declared(&shape, "shared"), Some(("string", "from b".into())), "the later trigger wins");
        assert!(!shape.properties.contains_key("gamma"), "a non-matching trigger contributes nothing");
        let validates = |shared: Value| effective.validate_raw(&serde_json::json!({ "prompt": "hi", "shared": shared })).valid;
        assert!(validates("x".into()) && !validates(3.into()), "validation applies the same precedence");
    }

    #[test]
    fn property_shape_lets_an_authored_redeclaration_win_per_property() {
        let repo = three_trigger_repo();
        let (effective, frontmatter) =
            effective_of(repo.path(), "$schema:\n  shared: 'boolean -> authored'\nprompt: hi\n");

        let shape = effective.property_shape(&[], &frontmatter);

        assert_eq!(declared(&shape, "shared"), Some(("boolean", "authored".into())));
        assert_eq!(declared(&shape, "alpha"), Some(("string", "from a".into())));
        assert_eq!(declared(&shape, "beta"), Some(("boolean", "from b".into())));
        let Some(SchemaRoot::Single(authored)) = &effective.simplified else {
            panic!("the authored `$schema` is a single object");
        };
        let authored_keys: Vec<&str> = authored.properties.keys().map(String::as_str).collect();
        assert_eq!(authored_keys, ["shared"], "`simplified` stays the authored declaration alone");
        let validates = |shared: Value| effective.validate_raw(&serde_json::json!({ "prompt": "hi", "shared": shared })).valid;
        assert!(validates(true.into()) && !validates("x".into()), "validation applies the same precedence");
    }

    #[test]
    fn property_shape_puts_triggers_over_the_base_and_extension_layers() {
        let repo = three_trigger_repo();
        let (effective, frontmatter) = effective_of(repo.path(), "prompt: hi\n");
        let SchemaRoot::Single(extension) =
            simplified::parse_yaml_schema(
                &serde_yaml_ng::from_str("alpha: 'number -> extension'\ndelta: 'string -> extension'\n").unwrap(),
            )
            .unwrap()
        else {
            panic!("a single-object extension");
        };
        let SchemaRoot::Single(base) = darkmatter_base_schema() else {
            panic!("the base is a single object");
        };

        let shape = effective.property_shape(&[extension], &frontmatter);

        assert_eq!(declared(&shape, "alpha"), Some(("string", "from a".into())), "trigger over extension");
        assert_eq!(declared(&shape, "delta"), Some(("string", "extension".into())));
        for name in base.properties.keys() {
            assert!(shape.properties.contains_key(name), "base property `{name}` kept");
        }
        assert!(!shape.properties.contains_key("gamma"));
    }
}
