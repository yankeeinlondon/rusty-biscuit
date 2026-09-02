//! Schema authoring, detection, and validation for Markdown frontmatter.
//!
//! This subsystem implements the SimplifiedSchema authoring grammar, JSON
//! Schema conversion, validator caching, schema resolution, and (in later
//! phases) a `md schema` CLI surface.
//!
//! Phase 3 lands the runtime: validator caching, `$schema` resolution
//! (inline, file references, root unions), baseline merging, and the public
//! [`DarkmatterSchemas`] entry point.
//!
//! ## Module layout
//!
//! - [`simplified::types`] — [`SimplifiedSchema`], [`PropertyDef`],
//!   [`SimplifiedType`], [`Constraint`].
//! - [`simplified::grammar`] — type-and-constraint string parser.
//! - [`simplified::convert`] — [`to_json_schema`] lowers a parsed schema to
//!   Draft 2020-12 JSON Schema.
//! - [`simplified`] — YAML-shape layer over `serde_yaml_ng::Value`.
//! - [`format`] — custom format validators (`darkmatter-file` eager,
//!   `darkmatter-file-reference` lazy) plus URL-scheme and passive semantic
//!   meta-type keyword validators. (`match(...)` is suggestion metadata only —
//!   never a validation keyword.)
//! - [`validate`] — `Validator` construction + LRU [`ValidatorCache`].
//! - [`rewrite`] — eager-`file` value normalization: rewrites a present
//!   `file(eager)`-typed value to its repo-relative resolved path after
//!   validation accepts it.
//! - [`resolve`] — `$schema` resolution and baseline merge.
//! - [`about`] — typed descriptor catalog that backs `md schema about`.
//! - [`errors`] — [`SchemaError`].
//!
//! ## Example
//!
//! ```ignore
//! use darkmatter::markdown::schemas::DarkmatterSchemas;
//! use darkmatter::markdown::Markdown;
//!
//! let md = Markdown::try_from(std::path::Path::new("./post.md"))?;
//! let api = DarkmatterSchemas::new();
//! let report = api.validate(&md)?;
//! assert!(report.valid);
//! ```
//!
//! See `darkmatter/features/2026-05-11-schemas/spec.md` for the full
//! specification.
//!
//! [`SimplifiedSchema`]: crate::markdown::schemas::simplified::SimplifiedSchema
//! [`PropertyDef`]: crate::markdown::schemas::simplified::PropertyDef
//! [`SimplifiedType`]: crate::markdown::schemas::simplified::SimplifiedType
//! [`Constraint`]: crate::markdown::schemas::simplified::Constraint
//! [`SchemaError`]: crate::markdown::schemas::errors::SchemaError

pub mod about;
pub mod clean;
pub mod coerce;
pub mod completion;
pub mod detect;
pub mod discriminant;
pub mod errors;
pub mod example;
pub mod format;
mod reference;
pub mod resolve;
pub mod rewrite;
pub mod simplified;
pub mod triggers;
pub mod validate;

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use jsonschema::Validator;
use serde_json::Value;

use crate::markdown::{Markdown, compose::ComposeSource};

pub use about::{
    CoercionRuleDescriptor, InlineObjectRuleDescriptor, MatchSafeConstraintDescriptor,
    SchemaConstraintDescriptor, SchemaShapeDescriptor, SchemaTypeDescriptor,
    TriggerGrammarDescriptor, ValidationBehaviorDescriptor, coercion_rule_descriptors,
    inline_object_rule_descriptors, schema_constraint_descriptors, schema_shape_descriptors,
    schema_type_descriptors, validation_behavior_descriptors, match_safe_constraint_descriptors,
    trigger_grammar_descriptors,
};
pub use clean::{
    CleanBaselineSchema, CleanSchemaConfig, CleanSchemaContext, SchemaCleanAnalysis,
    analyze_frontmatter, schema_result_set_identical,
};
pub use completion::{CompletionKind, CompletionSuggestion};
pub use detect::{
    DetectOptions, detect_from_document, detect_from_document_with_context, detect_schema,
    detect_schema_with_contexts, schema_to_yaml,
};
pub use discriminant::select_literal_discriminant_arm;
pub use errors::SchemaError;
pub use reference::{SchemaReference, SchemaReferenceKind, classify_schema_reference};
pub use rewrite::NormalizationOutcome;
pub use simplified::{
    Constraint, DRAFT_2020_12, DecodedScalar, PatternKey, PatternKeyDef, PropertyAtom, PropertyDef,
    SchemaArm,
    SchemaCursor, SchemaCursorRole, SchemaDeclaration, SchemaShape, SchemaSourceMap,
    SchemaSourcePath, SchemaSourcePathSegment,
    SchemaSpanKind, SchemaValueEntry, SchemaValueKind, SchemaValueNode, SimplifiedSchema,
    SimplifiedType, SourceAware, SuggestionItem,
    SuggestionLintProblem, SuggestionLintReason, SuggestionQuery, TypeExpr, decode_scalar,
    decode_partial_scalar_at, decode_scalar_at, is_union_arm_path, lint_suggestions,
    locate_schema_declaration_cursor, locate_schema_value, locate_type_definition_cursor,
    parse_property_definition,
    parse_property_definition_with_source, parse_schema_declaration,
    parse_schema_declaration_with_source, suggestions_for_def, suggestions_for_path,
    StandaloneSchemaDocument, StandaloneSchemaEnvelope, parse_standalone_schema_document,
    parse_yaml_schema, to_json_schema,
};
pub use triggers::{
    LoadedTrigger, MatchArms, MatchExpr, PathGlobs, ShadowedFile, TriggerEnvelope,
    TriggerArmTrace, TriggerEvaluation, TriggerRegistry, TriggerTrace, TriggerTraceEntry,
    evaluate_registry, matched_triggers, normalize_path,
    normalize_relative_path, parse_trigger_envelope, parse_trigger_envelope_from_str,
    trace_registry,
    schema_roots, scan,
};
pub use validate::{CACHE_SIZE_ENV, DEFAULT_CACHE_SIZE, PositionMap, ValidatorCache};

static BASE_SCHEMA: std::sync::OnceLock<SimplifiedSchema> = std::sync::OnceLock::new();
static BASE_JSON_SCHEMA: std::sync::OnceLock<Arc<Value>> = std::sync::OnceLock::new();

fn darkmatter_base_schema_ref() -> &'static SimplifiedSchema {
    BASE_SCHEMA.get_or_init(|| {
        let raw = include_str!("../../../../docs/schemas/darkmatter.yaml");
        let frontmatter: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(raw).expect("baseline yaml must parse");
        let schema_value = frontmatter
            .get("$schema")
            .expect("baseline file must have a `$schema` key");
        parse_yaml_schema(schema_value).expect("baseline schema must parse")
    })
}

fn darkmatter_base_json_schema_arc() -> &'static Arc<Value> {
    BASE_JSON_SCHEMA.get_or_init(|| {
        Arc::new(
            to_json_schema(darkmatter_base_schema_ref()).expect("baseline schema must convert"),
        )
    })
}

/// Returns the Darkmatter baseline frontmatter schema as a [`SimplifiedSchema`].
///
/// Loads the authored schema from `darkmatter/docs/schemas/darkmatter.yaml` via
/// `include_str!` and parses it at first call. The result is cached so repeated
/// calls are cheap.
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
/// Panics if the checked-in `darkmatter.yaml` cannot be parsed. This is a
/// library bug or repository corruption, not an author error.
pub fn darkmatter_base_schema() -> SimplifiedSchema {
    darkmatter_base_schema_ref().clone()
}

/// Returns the Darkmatter baseline frontmatter schema as a compiled Draft
/// 2020-12 JSON Schema value.
///
/// The result is derived from [`darkmatter_base_schema`] and cached so repeated
/// calls do not re-pay the conversion cost. Each call returns an independent
/// owned value; callers that only need to read the schema can use
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
/// Panics if the checked-in `darkmatter.yaml` cannot be converted to JSON
/// Schema. This is a library bug or repository corruption, not an author error.
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
/// Panics if the checked-in `darkmatter.yaml` cannot be converted to JSON
/// Schema. This is a library bug or repository corruption, not an author error.
pub fn darkmatter_base_json_schema_ref() -> &'static Value {
    darkmatter_base_json_schema_arc().as_ref()
}

/// Top-level entry point for the schemas subsystem.
///
/// Holds optional baseline schema configuration, optional trigger-schema
/// configuration, and the process-wide validator cache so repeated validations
/// against the same effective schema reuse the compiled `jsonschema::Validator`.
///
/// [`DarkmatterSchemas::new`] is deterministic and **never** scans disk.
/// Trigger-schema discovery is opt-in via [`Self::with_trigger_discovery`] or
/// [`Self::with_trigger_registry`]. Implicit CWD-based discovery is forbidden —
/// validation behavior must never depend silently on the process working
/// directory.
#[derive(Clone)]
pub struct DarkmatterSchemas {
    baseline: Option<BaselineSchema>,
    cache: ValidatorCache,
    triggers: Option<triggers::TriggerRegistry>,
    file_resolution_context: Option<biscuit_file::FileResolutionContext>,
}

/// The process-wide validator cache every [`DarkmatterSchemas`] shares (F8).
///
/// Compiling a `jsonschema::Validator` is milliseconds of work, and a single
/// `md compose` runs schema validation up to twice (a second pass after shell
/// expansion when trigger schemas are enabled) while a long-running host
/// (claudine) composes many documents against the same handful of schemas.
/// Sharing one cache — cloned into each instance — lets those passes reuse
/// compiled validators instead of recompiling per instance. The cache key folds
/// in both file-reference anchors (see [`ValidatorCache`]), so clones carrying
/// different launch-area fallbacks share the map without cross-contamination.
static SHARED_VALIDATOR_CACHE: std::sync::LazyLock<ValidatorCache> =
    std::sync::LazyLock::new(ValidatorCache::new);

impl Default for DarkmatterSchemas {
    fn default() -> Self {
        Self {
            baseline: None,
            cache: SHARED_VALIDATOR_CACHE.clone(),
            triggers: None,
            file_resolution_context: None,
        }
    }
}

#[derive(Clone)]
struct BaselineSchema {
    /// Held as `Arc<Value>` so `effective_for` shares it by reference (a
    /// refcount bump) instead of deep-cloning the whole baseline schema on the
    /// common baseline-only / baseline+document paths (F29).
    json_schema: Arc<Value>,
}

/// A resolved trigger payload layer, ready to merge into the effective schema.
#[derive(Clone)]
struct TriggerLayer {
    /// The trigger envelope file path (for origin attribution + dependency).
    source: PathBuf,
    /// The resolved payload JSON Schema (guaranteed simple-object).
    json_schema: Value,
    /// Payload dependency edges (referenced files + imports + examples).
    dependencies: Vec<PathBuf>,
}

impl DarkmatterSchemas {
    /// Creates a new [`DarkmatterSchemas`] with no baseline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the launch-area anchor (typically the captured launch area).
    ///
    /// Per D2 the launch area is **not** a resolution input for a
    /// document-authored `format: darkmatter-file` value: those resolve
    /// repository-first then against the document directory, never the launch
    /// area or the ambient CWD. This anchor is retained for structural parity
    /// with the validator-cache identity (it still participates in
    /// `ComposeOptions`/cache identity) but does not change the resolved path.
    #[must_use]
    pub fn with_file_ref_fallback_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cache = self.cache.with_file_ref_fallback_dir(dir);
        self
    }

    /// Supplies the immutable host request snapshot used by schema file
    /// values. Nested document bases derive from this snapshot.
    #[must_use]
    pub fn with_file_resolution_context(
        mut self,
        context: biscuit_file::FileResolutionContext,
    ) -> Self {
        self.cache = self.cache.with_file_resolution_context(context.clone());
        self.file_resolution_context = Some(context);
        self
    }

    /// Attaches a parsed SimplifiedSchema as the baseline.
    ///
    /// ## Errors
    ///
    /// Returns [`SchemaError::Convert`] if the schema cannot be converted to
    /// JSON Schema, or [`SchemaError::Baseline`] if the resulting JSON Schema
    /// is not a simple object schema (rooted at `"type": "object"` with only
    /// `properties` / `required`).
    pub fn with_baseline(mut self, schema: SimplifiedSchema) -> Result<Self, SchemaError> {
        let json = to_json_schema(&schema).map_err(|err| SchemaError::Baseline {
            message: "baseline could not be converted to JSON Schema".into(),
            source: Some(Box::new(err)),
        })?;
        self.set_baseline_json(json)?;
        Ok(self)
    }

    /// Loads a baseline schema from a YAML or JSON file. Uses the same
    /// disambiguation rule as document `$schema` references.
    ///
    /// ## Errors
    ///
    /// Propagates [`SchemaError::Io`], [`SchemaError::AmbiguousReferenced`],
    /// or [`SchemaError::Baseline`] depending on the failure shape.
    pub fn with_baseline_from_file(mut self, path: impl AsRef<Path>) -> Result<Self, SchemaError> {
        let path = path.as_ref();
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        // Drive resolution through the file-reference path so YAML/JSON
        // disambiguation matches `$schema` references.
        let yaml_value = serde_yaml_ng::Value::String(path.to_string_lossy().into_owned());
        let resolved =
            resolve::resolve_yaml_schema(&yaml_value, base).map_err(|err| match err {
                SchemaError::Io { .. } => err,
                SchemaError::AmbiguousReferenced { .. } => err,
                other => SchemaError::Baseline {
                    message: format!("could not load baseline from `{}`", path.display()),
                    source: Some(Box::new(other)),
                },
            })?;
        self.set_baseline_json(resolved.json_schema)?;
        Ok(self)
    }

    /// Attaches a raw JSON Schema as the baseline.
    ///
    /// ## Errors
    ///
    /// Returns [`SchemaError::Baseline`] when the value is not a simple
    /// object schema.
    pub fn with_baseline_json_schema(mut self, value: Value) -> Result<Self, SchemaError> {
        self.set_baseline_json(value)?;
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
        resolve::validate_baseline_schema(&json_schema)?;
        self.baseline = Some(BaselineSchema { json_schema });
        Ok(self)
    }

    fn set_baseline_json(&mut self, value: Value) -> Result<(), SchemaError> {
        resolve::validate_baseline_schema(&value)?;
        self.baseline = Some(BaselineSchema {
            json_schema: Arc::new(value),
        });
        Ok(())
    }

    /// Enables trigger-schema discovery by scanning from `document_path`'s
    /// directory up through `boundary` (inclusive).
    ///
    /// The scan is performed eagerly at configuration time. Every `schemas/`
    /// directory on the ancestor walk is a schema root, nearest first; trigger
    /// envelopes are loaded transactionally (one malformed trigger aborts the
    /// whole scan). The resulting [`triggers::TriggerRegistry`] is stored and
    /// consumed by [`Self::effective_for`].
    ///
    /// [`DarkmatterSchemas::new`] never scans disk — discovery is opt-in via
    /// this method or [`Self::with_trigger_registry`]. Implicit CWD-based
    /// discovery is forbidden.
    ///
    /// ## Errors
    ///
    /// Propagates [`SchemaError::TriggerLoad`] when a trigger file in a
    /// discovered root claims the `kind: trigger-schema` envelope but is
    /// malformed (bad envelope, bad match grammar, vacuous arm).
    pub fn with_trigger_discovery(
        self,
        document_path: impl AsRef<Path>,
        boundary: impl AsRef<Path>,
    ) -> Result<Self, SchemaError> {
        let registry = triggers::scan(document_path.as_ref(), boundary.as_ref())?;
        Ok(self.with_trigger_registry(registry))
    }

    /// Attaches a prebuilt trigger registry (e.g. one cached per boundary by
    /// DMLS, or constructed programmatically by a test).
    ///
    /// [`DarkmatterSchemas::new`] never scans disk — discovery is opt-in.
    /// Implicit CWD-based discovery is forbidden; a caller must explicitly
    /// supply a document path + boundary (via [`Self::with_trigger_discovery`])
    /// or a prebuilt registry.
    #[must_use]
    pub fn with_trigger_registry(mut self, registry: triggers::TriggerRegistry) -> Self {
        self.triggers = Some(registry);
        self
    }

    /// Returns the configured trigger registry, when present.
    pub fn trigger_registry(&self) -> Option<&triggers::TriggerRegistry> {
        self.triggers.as_ref()
    }

    /// Builds the effective schema for a Markdown document.
    ///
    /// Reads the document's `$schema` frontmatter (if any), resolves it, and
    /// merges the layers in precedence order:
    ///
    /// 1. The caller-configured baseline (if any).
    /// 2. Matching trigger-schema payloads — nearest root first,
    ///    filename-lexicographic within a root (the registry's built-in order).
    ///    Shadowing is applied before matching (a shadowed file is never in the
    ///    registry).
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
        let base_dir = base_dir_for(source);
        let frontmatter = source.frontmatter().as_map();
        let schema_value = schema_override.or_else(|| frontmatter.get("$schema"));

        // Schema roots from the trigger registry feed bare-name $schema
        // resolution (Phase 3 context).
        let trigger_roots: &[PathBuf] = self
            .triggers
            .as_ref()
            .map(|reg| reg.roots.as_slice())
            .unwrap_or(&[]);

        // Resolve the document $schema with schema-root context.
        let resolved = match schema_value {
            Some(value) => {
                let r = resolve::resolve_schema_with_roots_in_context(
                    value,
                    &base_dir,
                    trigger_roots,
                    self.file_resolution_context.as_ref(),
                )?;
                // The document must not directly reference a trigger-schema
                // file — triggers activate by placement and match, never by
                // reference.
                triggers::assemble::check_document_schema_not_trigger(&r.referenced_files)?;
                Some(r)
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

        let merged_json: Arc<Value> = match &self.baseline {
            Some(base) => {
                if trigger_layers.is_empty() && doc_json.is_none() {
                    // Baseline only — share the Arc, no deep clone.
                    Arc::clone(&base.json_schema)
                } else {
                    // Fold the higher-precedence layers into the borrowed baseline.
                    let mut higher: Vec<Value> =
                        Vec::with_capacity(trigger_layers.len() + 1);
                    for layer in &trigger_layers {
                        higher.push(layer.json_schema.clone());
                    }
                    if let Some(doc) = doc_json {
                        higher.push(doc);
                    }
                    let mut higher = higher.into_iter();
                    let mut acc =
                        resolve::merge_baseline(&base.json_schema, higher.next().expect("non-empty"))?;
                    for layer in higher {
                        acc = resolve::merge_baseline(&acc, layer)?;
                    }
                    Arc::new(acc)
                }
            }
            None => match (trigger_layers.is_empty(), doc_json) {
                (true, None) => return Ok(None),
                (true, Some(doc)) => Arc::new(doc),
                (false, doc_opt) => {
                    // No baseline: the first trigger is the (owned) merge base.
                    let mut layers: Vec<Value> = trigger_layers
                        .iter()
                        .map(|l| l.json_schema.clone())
                        .collect();
                    if let Some(doc) = doc_opt {
                        layers.push(doc);
                    }
                    let mut layers = layers.into_iter();
                    let mut acc = layers.next().expect("non-empty");
                    for layer in layers {
                        acc = resolve::merge_baseline(&acc, layer)?;
                    }
                    Arc::new(acc)
                }
            },
        };

        // Keep eager-file validation on the same request-scoped candidate plan
        // as expression-side `file_exists` and `frontmatter` resolution.
        let validator = self.cache.validator_for(&merged_json, Some(&base_dir))?;
        let arm_validators = build_arm_validators(&merged_json, &self.cache, &base_dir)?;
        let origins = build_origin_map_with_triggers(
            resolved.as_ref(),
            &trigger_layers,
            self.baseline.is_some(),
            &merged_json,
        );
        let dependencies = build_dependencies(resolved.as_ref(), &trigger_layers);
        Ok(Some(EffectiveSchema {
            simplified: resolved.and_then(|r| r.simplified),
            json_schema: merged_json,
            origins,
            validator,
            arm_validators,
            base_dir: Some(base_dir),
            file_ref_fallback_dir: self.cache.file_ref_fallback_dir().map(Path::to_path_buf),
            file_resolution_context: self.file_resolution_context.clone(),
            dependencies,
        }))
    }

    /// Matches the configured trigger registry against the document and
    /// resolves each matching payload. Returns the ordered trigger layers
    /// (nearest root first). Empty when no registry is configured or no
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

        // Normalized boundary-relative path for `$path` matching.
        let normalized_path = match source.source() {
            Some(ComposeSource::File(p)) => {
                triggers::normalize_path(p, &registry.boundary).unwrap_or_default()
            }
            _ => String::new(),
        };

        // Frontmatter snapshot for matching (strip the `$schema` control key).
        let fm_json = frontmatter_as_json(source);

        // Registries built by `scan` carry pre-resolved payloads; programmatically
        // constructed registries do not and fall back to on-demand resolution.
        let pre_resolved = !registry.payloads.is_empty();

        let evaluations =
            triggers::assemble::evaluate_registry(registry, &fm_json, &normalized_path);
        let mut layers = Vec::with_capacity(
            evaluations.iter().filter(|eval| eval.matched).count(),
        );
        for (idx, eval) in evaluations.into_iter().enumerate() {
            if !eval.matched {
                continue;
            }
            let (json_schema, dependencies) = if pre_resolved {
                let payload = &registry.payloads[idx];
                (payload.json_schema.clone(), payload.dependencies.clone())
            } else {
                let payload = triggers::assemble::resolve_trigger_payload(
                    eval.trigger,
                    &registry.roots,
                )?;
                (payload.json_schema, payload.dependencies)
            };
            layers.push(TriggerLayer {
                source: eval.trigger.source.clone(),
                json_schema,
                dependencies,
            });
        }
        Ok(layers)
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
        let frontmatter_value = frontmatter_as_json(source);
        let positions = positions_for(source);
        match self.effective_for(source)? {
            Some(effective) => {
                Ok(effective.validate_with_positions(&frontmatter_value, &positions))
            }
            None => Ok(ValidationReport {
                valid: true,
                problems: Vec::new(),
                pending: Vec::new(),
            }),
        }
    }

    /// Returns a shared reference to the underlying validator cache. Useful
    /// for sharing the cache between subsystems (e.g. a long-running CLI).
    pub fn cache(&self) -> &ValidatorCache {
        &self.cache
    }

    /// Detects a [`SimplifiedSchema`] from one or more documents. See
    /// [`detect::detect_schema`] for the algorithm.
    pub fn detect(&self, sources: &[&Markdown], opts: DetectOptions) -> SimplifiedSchema {
        detect::detect_schema(sources, opts)
    }
}

/// The fully-resolved schema for a document.
#[derive(Clone)]
pub struct EffectiveSchema {
    /// The SimplifiedSchema projection, when the document schema came from
    /// SimplifiedSchema input. `None` when the schema was a raw JSON Schema
    /// or when a root union mixed SimplifiedSchema and JSON Schema arms.
    pub simplified: Option<SimplifiedSchema>,
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
    validator: Arc<Validator>,
    /// Per-arm validators when `json_schema` is a root `anyOf` union.
    /// `None` for ordinary schemas.
    arm_validators: Option<Vec<Arc<Validator>>>,
    /// Prompt document directory used to reproduce the validator's
    /// repository-first, then source-relative candidate plan in diagnostics.
    base_dir: Option<PathBuf>,
    /// Captured launch-area metadata retained for file-reference diagnostics.
    file_ref_fallback_dir: Option<PathBuf>,
    /// Immutable host request snapshot used by eager-file normalization.
    file_resolution_context: Option<biscuit_file::FileResolutionContext>,
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
}

impl EffectiveSchema {
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

    /// Validates a frontmatter JSON value against this schema. Equivalent to
    /// [`Self::validate_with_positions`] with an empty position map (problems
    /// will carry no line/column information).
    ///
    /// Read-only: the caller's `frontmatter` is never mutated. To rewrite
    /// eager-`file` values to their resolved repo-relative paths, call
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
    /// The instance is type-coerced against the schema (see
    /// [`coerce::coerce_frontmatter`]) on a working copy before validation, so
    /// the report reflects post-coercion validity. No input is mutated —
    /// eager-`file` values are left in their raw caller-supplied form. To
    /// rewrite them to their resolved repo-relative paths, call
    /// [`Self::normalize_frontmatter`] explicitly on an already-valid
    /// instance.
    pub fn validate_with_positions(
        &self,
        frontmatter: &Value,
        positions: &PositionMap,
    ) -> ValidationReport {
        let coerced = coerce::coerce_frontmatter(&self.json_schema, frontmatter);
        self.validate_instance(&coerced.value, positions)
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
        self.validate_instance(frontmatter, positions)
    }

    /// Shared validation body over an already-prepared instance. Callers
    /// choose the preparation: [`Self::validate_with_positions`] passes a
    /// coerced working copy, the `validate_raw` pair passes the instance as
    /// parsed.
    fn validate_instance(&self, instance: &Value, positions: &PositionMap) -> ValidationReport {
        let anchors = validate::FileRefAnchors {
            base_dir: self.base_dir.as_deref(),
            fallback: self.file_ref_fallback_dir.as_deref(),
        };
        let mut problems = match &self.arm_validators {
            // Root `anyOf` union. When a shared literal discriminant selects a
            // single arm, report only that arm's problems; otherwise fall back
            // to the closest-matching-arm report (byte-identical to before).
            Some(arms) => {
                match self
                    .json_schema
                    .get("anyOf")
                    .and_then(Value::as_array)
                    .and_then(|root_arms| {
                        discriminant::select_literal_discriminant_arm(root_arms, instance)
                    }) {
                    Some(idx) => validate::collect_arm_problems_with_anchors(
                        &arms[idx], instance, positions, anchors, idx,
                    ),
                    None => validate::collect_root_union_problems_with_anchors(
                        arms, instance, positions, anchors,
                    ),
                }
            }
            // Ordinary object schema. Narrow any discriminated property-level
            // union to its selected arm; unrelated properties are untouched.
            None => {
                let problems = validate::collect_problems_with_anchors(
                    &self.validator,
                    instance,
                    positions,
                    anchors,
                );
                validate::narrow_property_union_problems(
                    &self.json_schema,
                    instance,
                    positions,
                    anchors,
                    problems,
                )
            }
        };
        // Enrich each problem with its declared property description (Decision
        // #2). Whitespace-only descriptions (#8) and descriptions identical to
        // the rendered message (#9) are suppressed so the output stays additive.
        for problem in &mut problems {
            problem.description = validate::resolve_problem_description(&self.json_schema, problem)
                .filter(|desc| !desc.trim().is_empty())
                .filter(|desc| *desc != problem.message);
        }
        ValidationReport {
            valid: problems.is_empty(),
            problems,
            pending: Vec::new(),
        }
    }

    /// Validates like [`Self::validate_with_positions`], then applies the
    /// compose-parity deferral rules from `options` as data (R-5 Priority 3).
    ///
    /// Top-level values still holding a `$(...)` shell expression or an
    /// unresolved `{{ ... }}` template are collected into
    /// [`ValidationReport::pending`]. With [`PendingPolicy::Defer`] (the
    /// default), problems attributable to a pending key are dropped from the
    /// report — mirroring compose, which lets a later shell-expansion pass
    /// re-validate the resolved value. Keys in `options.excluded_keys` have
    /// their problems dropped unconditionally (caller-owned keys).
    ///
    /// Nothing here executes a shell command, reads the environment, or
    /// touches the network: pending values are recognised lexically only.
    pub fn validate_with_options(
        &self,
        frontmatter: &Value,
        positions: &PositionMap,
        options: &ValidationOptions,
    ) -> ValidationReport {
        let mut report = self.validate_with_positions(frontmatter, positions);
        let pending = scan_pending_values(frontmatter);
        let pending_keys: HashSet<&str> = pending.iter().map(|p| p.key.as_str()).collect();
        let defer = matches!(options.pending_policy, PendingPolicy::Defer);

        report.problems.retain(|problem| {
            let Some(key) = attributable_top_level_key(problem) else {
                return true;
            };
            if options.excluded_keys.contains(&key) {
                return false;
            }
            !(defer && pending_keys.contains(key.as_str()))
        });
        report.valid = report.problems.is_empty();
        report.pending = pending;
        report
    }

    /// Returns the compiled validator. Mainly for advanced callers; the
    /// `validate` method is the normal entry point.
    pub fn validator(&self) -> &Validator {
        &self.validator
    }

    /// Normalizes eager-`file`-typed frontmatter values to their resolved
    /// repo-relative paths.
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
    /// `composition_pending` set [`coerce::coerce_frontmatter_with_pending`]
    /// consumes.
    ///
    /// ## Examples
    ///
    /// A raw caller-supplied reference under an eager `file(eager)` property is
    /// rewritten to its repo-relative resolved path — the same projection
    /// `relative(value)` / `dirname(value)` already produce:
    ///
    /// ```no_run
    /// use std::collections::HashSet;
    ///
    /// use darkmatter::markdown::Markdown;
    /// use darkmatter::markdown::schemas::DarkmatterSchemas;
    ///
    /// // `area/prompt.md` declares `$schema: { spec: file(eager; required) }`
    /// // and the repo root sits above `area/`. The schema resolves document-
    /// // first against the prompt's directory.
    /// let path = std::path::Path::new("area/prompt.md");
    /// let md = Markdown::try_from(path).unwrap();
    /// let effective = DarkmatterSchemas::new()
    ///     .effective_for(&md)
    ///     .unwrap()
    ///     .unwrap();
    ///
    /// let input = serde_json::json!({ "spec": "./spec.md" });
    /// let outcome = effective.normalize_frontmatter(&input, &HashSet::new());
    /// // Raw `./spec.md` -> repo-relative `area/spec.md`.
    /// assert_eq!(outcome.value["spec"], serde_json::json!("area/spec.md"));
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
        rewrite::rewrite_eager_file_values_in_context(
            &self.json_schema,
            frontmatter,
            base_dir,
            self.file_ref_fallback_dir.as_deref(),
            composition_pending,
            self.file_resolution_context.as_ref(),
        )
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
    /// Top-level values that could not be validated yet because they still
    /// hold a `$(...)` shell expression or an unresolved `{{ ... }}` template
    /// (R-5 Priority 3). Always empty for [`EffectiveSchema::validate`] /
    /// [`EffectiveSchema::validate_with_positions`]; populated only by
    /// [`EffectiveSchema::validate_with_options`], which mirrors the compose
    /// deferral rules without executing anything.
    pub pending: Vec<PendingValue>,
}

/// Coarse classification of a validation problem.
///
/// Renderers use this to pick the right category label (e.g. `missing`,
/// `type`, `invalid`) without having to infer from `property` presence or
/// substring-matching `message`. Mapped from `jsonschema::ValidationErrorKind`
/// at problem-construction time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationProblemKind {
    /// A required property is missing from the instance.
    Missing,
    /// The instance value did not match the expected schema type.
    Type,
    /// Any other constraint failure (format, range, length, pattern, etc.).
    Invalid,
}

/// A single validation problem.
#[derive(Debug, Clone)]
pub struct ValidationProblem {
    /// JSON pointer to the failing instance value (e.g. `/tags/2`).
    ///
    /// For `Required` failures this points at the *parent* object (empty for
    /// the document root), since the missing property has no instance value.
    /// Use [`Self::property`] to recover the missing property's name.
    pub path: String,
    /// Human-readable message; format mirrors `jsonschema`'s default.
    pub message: String,
    /// Coarse failure category, derived from the underlying validator error.
    pub kind: ValidationProblemKind,
    /// Name of the missing property, set only for `Required` failures.
    pub property: Option<String>,
    /// Source line of the problem in the frontmatter, when available.
    pub line: Option<u32>,
    /// Source column of the problem in the frontmatter, when available.
    pub column: Option<u32>,
    /// Index of the root-union arm under which this problem was raised, when
    /// the schema is a root union.
    pub arm_index: Option<usize>,
    /// Declared description of the failing property, resolved from the
    /// compiled JSON Schema (the SimplifiedSchema `-> {description}` text, or a
    /// `description` keyword authored in a referenced JSON Schema file).
    /// `None` when the property declares no description.
    pub description: Option<String>,
    /// Fine-grained failure classification (R-5 Priority 1). Unlike the coarse
    /// [`ValidationProblemKind`], this separates constraint violations, unknown
    /// keys, and file-reference failures so span-aware consumers (DMLS) can
    /// pick a ranging rule per category.
    pub code: ValidationProblemCode,
    /// The [`path`](Self::path) parsed into decoded JSON-Pointer segments, so
    /// consumers walk a frontmatter AST without re-parsing `~0`/`~1` escapes.
    pub instance_path: JsonPointer,
    /// Location of the failing keyword within the compiled JSON Schema, parsed
    /// into JSON-Pointer segments. `None` when the validator reported no schema
    /// path.
    pub schema_path: Option<JsonPointer>,
    /// The specific undeclared key for an `additionalProperties` /
    /// unknown-key failure. `None` for every other category. The failure's
    /// [`path`](Self::path) points at the *parent* object in this case, so this
    /// is the only way to recover the offending key.
    pub offending_property: Option<String>,
    /// Structured cause of a `format: darkmatter-file` /
    /// `darkmatter-file-reference` failure (R-5 Priority 4). `Some` only when
    /// [`code`](Self::code) is [`ValidationProblemCode::InvalidFileReference`];
    /// [`message`](Self::message) still carries the same rendered text.
    pub file_reference: Option<FileReferenceDiagnostic>,
    /// Caller-owned resolution context retained when projection, rather than
    /// ordinary document validation, produced the file-reference failure.
    pub caller_file: Option<CallerFileReferenceProvenance>,
}

/// Resolution evidence attached to a caller-owned file validation problem.
#[derive(Debug, Clone)]
pub struct CallerFileReferenceProvenance {
    /// The immutable context captured where the caller authored the value.
    pub origin: biscuit_file::FileResolutionContext,
    /// The selected or first attempted candidate, when the reference parsed.
    pub candidate: Option<biscuit_file::ResolutionCandidate>,
}

/// Fine-grained classification of a [`ValidationProblem`] (R-5 Priority 1).
///
/// Finer than [`ValidationProblemKind`]: constraint violations, unknown keys,
/// and file-reference failures each get their own variant so a diagnostic
/// surface can choose the correct ranging rule (value node, key node, whole
/// entry) without substring-matching the message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationProblemCode {
    /// A required property is absent.
    MissingRequired,
    /// The instance value's type did not match the declared type.
    TypeMismatch,
    /// A non-type constraint failed (range, length, pattern, enum, format
    /// other than a file reference, …).
    ConstraintViolation,
    /// A key not declared by the schema was present under
    /// `additionalProperties: false`.
    UnknownKey,
    /// A `format: darkmatter-file` / `darkmatter-file-reference` value failed
    /// to parse, resolve, or match an existing file.
    InvalidFileReference,
}

/// A parsed RFC 6901 JSON Pointer.
///
/// [`ValidationProblem::path`] carries the raw pointer string (`/tags/2`); this
/// is the decoded segment view (`["tags", "2"]`) span-aware consumers walk
/// without re-parsing the `~0`/`~1` escapes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JsonPointer {
    segments: Vec<String>,
}

impl JsonPointer {
    /// Parses a JSON-Pointer string into its decoded segments. An empty string
    /// (the document root) yields an empty pointer.
    pub fn parse(pointer: &str) -> Self {
        if pointer.is_empty() {
            return Self::default();
        }
        let body = pointer.strip_prefix('/').unwrap_or(pointer);
        let segments = body.split('/').map(decode_pointer_segment).collect();
        Self { segments }
    }

    /// The decoded segments, outermost first.
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// `true` for the document-root pointer (no segments).
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// The first (top-level) segment, if any.
    pub fn first(&self) -> Option<&str> {
        self.segments.first().map(String::as_str)
    }

    /// Re-encodes the pointer to its canonical RFC 6901 string form.
    pub fn as_pointer_string(&self) -> String {
        if self.segments.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        for segment in &self.segments {
            out.push('/');
            out.push_str(&segment.replace('~', "~0").replace('/', "~1"));
        }
        out
    }
}

fn decode_pointer_segment(segment: &str) -> String {
    segment.replace("~1", "/").replace("~0", "~")
}

/// Structured cause of a file-reference validation failure (R-5 Priority 4).
///
/// Replaces the previous opaque substituted message string with a typed cause
/// so consumers can offer the right remediation (fix the syntax, fix the
/// resolution context, or point at the missing file) and range the diagnostic
/// against the resolved-from directory when known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileReferenceDiagnostic {
    /// The value is not a parseable file reference.
    InvalidSyntax {
        /// The offending raw value.
        raw: String,
    },
    /// The reference parsed but could not be resolved against the filesystem
    /// or environment.
    ResolutionFailed {
        /// The offending raw value.
        raw: String,
    },
    /// The reference parsed and resolved, but no file exists at the resolved
    /// path.
    NoMatch {
        /// The offending raw value.
        raw: String,
        /// The directory resolution was anchored at, when known.
        resolved_from: Option<PathBuf>,
    },
}

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
    /// The value holds a `$(...)` shell expression that has not run.
    ShellExpression,
    /// The value holds an unresolved `{{ ... }}` template.
    UnresolvedTemplate,
}

/// How [`EffectiveSchema::validate_with_options`] treats pending values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingPolicy {
    /// Drop problems attributable to a pending top-level key (mirrors compose
    /// when shell expansion will re-validate the value downstream).
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
    /// A matching trigger-schema payload layered between the baseline and the
    /// document `$schema`.
    Trigger,
}

/// The origin of one effective-schema property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaOrigin {
    /// Which layer the property's schema came from.
    pub kind: SchemaOriginKind,
    /// The referenced schema file, when [`kind`](Self::kind) is
    /// [`SchemaOriginKind::ReferencedFile`].
    pub uri: Option<PathBuf>,
}

impl SchemaOrigin {
    /// An inline-document origin.
    pub fn document() -> Self {
        Self {
            kind: SchemaOriginKind::Document,
            uri: None,
        }
    }

    /// A baseline-schema origin.
    pub fn baseline() -> Self {
        Self {
            kind: SchemaOriginKind::Baseline,
            uri: None,
        }
    }

    /// A referenced-file origin carrying the resolved path.
    pub fn referenced_file(path: impl Into<PathBuf>) -> Self {
        Self {
            kind: SchemaOriginKind::ReferencedFile,
            uri: Some(path.into()),
        }
    }

    /// A trigger-schema-payload origin carrying the envelope file path.
    pub fn trigger(path: impl Into<PathBuf>) -> Self {
        Self {
            kind: SchemaOriginKind::Trigger,
            uri: Some(path.into()),
        }
    }
}

/// Per-top-level-property schema origins for an [`EffectiveSchema`], keyed by
/// property name in schema declaration order.
pub type SchemaOriginMap = indexmap::IndexMap<String, SchemaOrigin>;

fn build_arm_validators(
    schema: &Value,
    cache: &ValidatorCache,
    base_dir: &Path,
) -> Result<Option<Vec<Arc<Validator>>>, SchemaError> {
    let Some(arms) = schema.get("anyOf").and_then(Value::as_array) else {
        return Ok(None);
    };
    let mut out = Vec::with_capacity(arms.len());
    for arm in arms {
        let arm_schema = validate::wrap_arm_as_root_schema(arm);
        out.push(cache.validator_for(&arm_schema, Some(base_dir))?);
    }
    Ok(Some(out))
}

/// The top-level frontmatter key a problem is attributable to, mirroring the
/// compose-time deferral rule: a missing-required failure is keyed by the
/// missing property (its path points at the parent), everything else by the
/// first pointer segment.
fn attributable_top_level_key(problem: &ValidationProblem) -> Option<String> {
    if problem.code == ValidationProblemCode::MissingRequired {
        return problem.property.clone();
    }
    problem.instance_path.first().map(str::to_string)
}

/// Scans a frontmatter instance for top-level values that hold a deferred
/// composition construct (`$(...)` shell expression or `{{ ... }}` template),
/// mirroring compose's `value_pending_composition` — lexically only, never
/// executing anything. `$schema` is skipped (a control key, not data).
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

/// Classifies a value's deferral reason. A `$(...)` shell expression takes
/// precedence over a `{{ ... }}` template when both appear (the shell value is
/// what a later pass resolves first), matching compose's ordering.
fn pending_reason(value: &Value) -> Option<PendingValueReason> {
    if value_contains_marker(value, "$(") {
        Some(PendingValueReason::ShellExpression)
    } else if value_contains_marker(value, "{{") {
        Some(PendingValueReason::UnresolvedTemplate)
    } else {
        None
    }
}

fn value_contains_marker(value: &Value, marker: &str) -> bool {
    match value {
        Value::String(s) => s.contains(marker),
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
    resolved: Option<&resolve::ResolvedSchema>,
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
                    .then(|| SchemaOrigin::trigger(layer.source.clone()))
            });
            from_trigger.unwrap_or_else(SchemaOrigin::baseline)
        };
        out.insert(key.clone(), origin);
    }
    out
}

/// Collects all dependency edges for an effective schema: the document
/// `$schema`'s imports + examples + referenced files, plus each trigger's
/// envelope source + payload dependencies. Sorted and deduplicated.
fn build_dependencies(
    resolved: Option<&resolve::ResolvedSchema>,
    trigger_layers: &[TriggerLayer],
) -> Vec<PathBuf> {
    let mut deps: Vec<PathBuf> = Vec::new();
    if let Some(r) = resolved {
        deps.extend(r.imports.iter().cloned());
        deps.extend(r.examples.iter().cloned());
        deps.extend(r.referenced_files.iter().cloned());
    }
    for layer in trigger_layers {
        deps.push(layer.source.clone());
        deps.extend(layer.dependencies.iter().cloned());
    }
    deps.sort();
    deps.dedup();
    deps
}

pub(crate) fn positions_for(source: &Markdown) -> PositionMap {
    // Prefer the original frontmatter text so reported line/column numbers
    // match the source the user is editing. The raw text is the body
    // between the leading `---` markers — its line 1 is the file's line 2
    // (the opening `---` is line 1), so we offset the captured positions
    // by `LEADING_DELIMITER_LINES`.
    const LEADING_DELIMITER_LINES: u32 = 1;
    if let Some(raw) = source.frontmatter().raw_source() {
        let mut map = validate::build_position_map(raw);
        for (_, (line, _)) in map.iter_mut() {
            *line = line.saturating_add(LEADING_DELIMITER_LINES);
        }
        return map;
    }

    // Fall back to re-serialising the parsed map. Used only for
    // programmatically constructed `Markdown` documents that never went
    // through the parser; line numbers refer to the canonicalised view.
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
        Ok(rendered) => validate::build_position_map(&rendered),
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

fn base_dir_for(source: &Markdown) -> PathBuf {
    match source.source() {
        Some(ComposeSource::File(path)) => path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".")),
        _ => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    }
}

pub(crate) fn frontmatter_as_json(source: &Markdown) -> Value {
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
    use crate::markdown::Markdown;

    fn md_with_schema(yaml_body: &str) -> Markdown {
        let content = format!("---\n{yaml_body}---\nbody\n");
        let md: Markdown = content.as_str().into();
        md
    }

    #[test]
    fn validates_inline_schema_success() {
        let md = md_with_schema("$schema:\n  title: 'string(required)'\ntitle: Hello\n");
        let api = DarkmatterSchemas::new();
        let report = api.validate(&md).unwrap();
        assert!(report.valid, "expected valid: {:?}", report.problems);
    }

    #[test]
    fn validates_inline_schema_missing_required() {
        let md = md_with_schema("$schema:\n  title: 'string(required)'\nother: stuff\n");
        let api = DarkmatterSchemas::new();
        let report = api.validate(&md).unwrap();
        assert!(!report.valid);
        assert!(!report.problems.is_empty());
    }

    #[test]
    fn no_schema_no_baseline_is_vacuously_valid() {
        let md = md_with_schema("name: alice\n");
        let api = DarkmatterSchemas::new();
        let report = api.validate(&md).unwrap();
        assert!(report.valid);
        assert!(report.problems.is_empty());
    }

    #[test]
    fn baseline_applies_when_document_has_no_schema() {
        let md = md_with_schema("title: hi\n");
        let baseline = SimplifiedSchema::Single(SchemaShape {
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
        let api = DarkmatterSchemas::new().with_baseline(baseline).unwrap();
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
        let baseline = SimplifiedSchema::Single(SchemaShape {
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
        let api = DarkmatterSchemas::new().with_baseline(baseline).unwrap();
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
        let api = DarkmatterSchemas::new();
        let md1 = md_with_schema("$schema:\n  x: number\nx: 1\n");
        let md2 = md_with_schema("$schema:\n  x: number\nx: 2\n");
        api.validate(&md1).unwrap();
        api.validate(&md2).unwrap();
        assert_eq!(api.cache().len(), 1);
    }

    #[test]
    fn coerces_boolish_string_against_inline_schema() {
        let md = md_with_schema("$schema:\n  flag: boolean\nflag: \"true\"\n");
        let api = DarkmatterSchemas::new();
        let report = api.validate(&md).unwrap();
        assert!(report.valid, "expected valid: {:?}", report.problems);
    }

    #[test]
    fn coerces_numeric_string_against_inline_schema() {
        let md = md_with_schema("$schema:\n  n: number\nn: \"42\"\n");
        let api = DarkmatterSchemas::new();
        let report = api.validate(&md).unwrap();
        assert!(report.valid, "expected valid: {:?}", report.problems);
    }

    #[test]
    fn ambiguous_string_still_reports_type_problem() {
        let md = md_with_schema("$schema:\n  flag: boolean(required)\nflag: \"yes\"\n");
        let api = DarkmatterSchemas::new();
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
        let baseline = SimplifiedSchema::Single(SchemaShape {
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
        let api = DarkmatterSchemas::new().with_baseline(baseline).unwrap();
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
        let api = DarkmatterSchemas::new()
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
        let api = DarkmatterSchemas::new();
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
        let api = DarkmatterSchemas::new()
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
        let api = DarkmatterSchemas::new()
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

    /// RAII guard that restores the process CWD on drop, even on panic.
    /// Tests that mutate CWD are annotated with
    /// `#[serial_test::serial("darkmatter-file-cwd")]` to prevent races with
    /// the ambient-CWD tests in `format::tests` and `validate::tests`.
    struct CwdGuard {
        prior: std::path::PathBuf,
    }

    impl CwdGuard {
        fn enter(dir: &std::path::Path) -> Self {
            let prior = std::env::current_dir().expect("read CWD");
            std::env::set_current_dir(dir).expect("set CWD");
            Self { prior }
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.prior);
        }
    }

    /// A `file`-typed schema property value is **not** resolved via the
    /// captured launch-area fallback (D2): the launch area is not a resolution
    /// input for a reference authored inside the document. A value present only
    /// under the fallback (and not under the document base or ambient CWD) fails
    /// validation.
    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn file_format_does_not_resolve_via_launch_area_fallback() {
        let launch_dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(launch_dir.path().join("spec.md"), "# Spec\n").expect("write spec");
        let unrelated_dir = tempfile::tempdir().expect("tempdir");

        let md = md_with_schema("$schema:\n  spec: 'file(eager; required)'\nspec: spec.md\n");
        let api = DarkmatterSchemas::new()
            .with_file_ref_fallback_dir(launch_dir.path().to_path_buf());

        let _cwd = CwdGuard::enter(unrelated_dir.path());
        let report = api.validate(&md).expect("validate");

        assert!(
            !report.valid,
            "the launch-area fallback must not resolve a document-authored file reference: {:?}",
            report.problems,
        );
    }

    /// A `file`-typed schema property value that exists under the ambient CWD
    /// but NOT under the prompt directory fails validation — proving the
    /// document base directory (not the ambient CWD, and not the inert
    /// launch-area anchor) drives resolution. The prompt has a real file source
    /// in a third directory so its `base_dir` is distinct from the CWD.
    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn file_format_rejects_when_not_under_document_base() {
        let prompt_dir = tempfile::tempdir().expect("tempdir");
        let fallback_dir = tempfile::tempdir().expect("tempdir");
        let cwd_dir = tempfile::tempdir().expect("tempdir");
        // File exists under CWD but NOT under the prompt dir or the fallback dir.
        std::fs::write(cwd_dir.path().join("ambient.md"), "# Ambient\n").expect("write");

        let md = prompt_with_source(prompt_dir.path(), "$schema:\n  spec: 'file(eager; required)'\nspec: ambient.md\n");
        let api = DarkmatterSchemas::new()
            .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());

        let _cwd = CwdGuard::enter(cwd_dir.path());
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
            "title: string(required)\n",
        )
        .expect("write schema");
        let doc_path = doc_dir.path().join("doc.md");
        std::fs::write(
            &doc_path,
            "---\n$schema: ./schema.yaml\ntitle: Hello\n---\nbody\n",
        )
        .expect("write doc");

        let md = Markdown::try_from(doc_path.as_path()).expect("read doc");
        let api = DarkmatterSchemas::new()
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
            "kind: string(required)\n",
        )
        .expect("write arm-a");
        let doc_path = doc_dir.path().join("doc.md");
        std::fs::write(
            &doc_path,
            "---\n$schema:\n  - ./arm-a.yaml\n  - fallback: string\nkind: feature\n---\nbody\n",
        )
        .expect("write doc");

        let md = Markdown::try_from(doc_path.as_path()).expect("read doc");
        let api = DarkmatterSchemas::new()
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
    #[serial_test::serial(darkmatter_file_cwd)]
    fn file_property_ignores_launch_copy_when_source_exists() {
        let prompt_dir = tempfile::tempdir().expect("tempdir");
        let fallback_dir = tempfile::tempdir().expect("tempdir");
        let unrelated = tempfile::tempdir().expect("tempdir");
        std::fs::write(prompt_dir.path().join("spec.md"), "# prompt copy\n").expect("write prompt spec");
        std::fs::write(fallback_dir.path().join("spec.md"), "# fallback copy\n").expect("write fallback spec");

        let md = prompt_with_source(prompt_dir.path(), "$schema:\n  spec: 'file(eager; required)'\nspec: spec.md\n");
        let api = DarkmatterSchemas::new()
            .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());

        let _cwd = CwdGuard::enter(unrelated.path());
        let report = api.validate(&md).expect("validate");
        assert!(
            report.valid,
            "the source-local file value must validate independently of the launch copy: {:?}",
            report.problems,
        );
    }

    /// A source-local `file` value validates when no repository candidate
    /// exists, independently of captured launch metadata and ambient CWD.
    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn file_property_present_only_in_prompt_dir_validates() {
        let prompt_dir = tempfile::tempdir().expect("tempdir");
        let fallback_dir = tempfile::tempdir().expect("tempdir");
        let unrelated = tempfile::tempdir().expect("tempdir");
        std::fs::write(prompt_dir.path().join("local.md"), "# local\n").expect("write local");

        let md = prompt_with_source(prompt_dir.path(), "$schema:\n  spec: 'file(eager; required)'\nspec: ./local.md\n");
        let api = DarkmatterSchemas::new()
            .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());

        let _cwd = CwdGuard::enter(unrelated.path());
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
    /// ambient CWD.
    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn file_property_present_only_in_fallback_does_not_validate() {
        let prompt_dir = tempfile::tempdir().expect("tempdir");
        let fallback_dir = tempfile::tempdir().expect("tempdir");
        let unrelated = tempfile::tempdir().expect("tempdir");
        std::fs::write(fallback_dir.path().join("caller.md"), "# caller\n").expect("write caller");

        let md = prompt_with_source(prompt_dir.path(), "$schema:\n  spec: 'file(eager; required)'\nspec: caller.md\n");
        let api = DarkmatterSchemas::new()
            .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());

        let _cwd = CwdGuard::enter(unrelated.path());
        let report = api.validate(&md).expect("validate");
        assert!(
            !report.valid,
            "a file value present only under the launch-area fallback must not validate: {:?}",
            report.problems,
        );
    }

    /// Guard: with both a prompt-dir anchor and a fallback configured, a value
    /// that exists ONLY under the process CWD (neither the prompt dir nor the
    /// fallback) must NOT validate — there is no ambient-CWD rung on the
    /// production path.
    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn file_property_present_only_in_cwd_does_not_validate() {
        let prompt_dir = tempfile::tempdir().expect("tempdir");
        let fallback_dir = tempfile::tempdir().expect("tempdir");
        let cwd_dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(cwd_dir.path().join("ambient.md"), "# ambient\n").expect("write ambient");

        let md = prompt_with_source(prompt_dir.path(), "$schema:\n  spec: 'file(eager; required)'\nspec: ambient.md\n");
        let api = DarkmatterSchemas::new()
            .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());

        let _cwd = CwdGuard::enter(cwd_dir.path());
        let report = api.validate(&md).expect("validate");
        assert!(
            !report.valid,
            "a file value found only under the ambient CWD must NOT validate: {:?}",
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
        DarkmatterSchemas::new()
            .effective_for(&md)
            .expect("effective_for")
            .expect("schema present")
    }

    /// `normalize_frontmatter` rewrites a present eager-`file` value to its
    /// repo-relative resolved path, and the caller's input `Value` is left
    /// byte-identical (Decision #3: pure).
    #[test]
    fn normalize_frontmatter_rewrites_eager_file_and_leaves_input_untouched() {
        let repo = tempfile::tempdir().expect("tempdir");
        // A `.git` marker makes the projection git-root-relative.
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
        assert_eq!(outcome.value["spec"], serde_json::json!("area/spec.md"));
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
        assert_eq!(outcome.value["design"], serde_json::json!("area/design.md"));
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
        let api = DarkmatterSchemas::new();
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

    /// Builds a baseline `SimplifiedSchema` from a YAML body.
    fn baseline_from_yaml(yaml_body: &str) -> SimplifiedSchema {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml_body).expect("yaml parse");
        super::simplified::parse_yaml_schema(&value).expect("baseline schema parse")
    }

    #[test]
    fn baseline_generated_ctx_validates_when_authored_doc_omits_ctx() {
        let baseline = baseline_from_yaml(
            "ctx:\n  today: \"date(generated; required) -> today's date, host-supplied\"",
        );
        let api = DarkmatterSchemas::new()
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
        let api = DarkmatterSchemas::new()
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
        let api = DarkmatterSchemas::new()
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
            super::SimplifiedSchema::Single(s) => s,
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
        let api = DarkmatterSchemas::new()
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
        DarkmatterSchemas::new()
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
        let api = DarkmatterSchemas::new().with_baseline(baseline).unwrap();
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
        let effective = DarkmatterSchemas::new()
            .effective_for(&md)
            .unwrap()
            .unwrap();
        let origin = effective.origins.get("title").expect("title origin");
        assert_eq!(origin.kind, SchemaOriginKind::ReferencedFile);
        assert!(
            origin.uri.as_ref().map(|p| p.ends_with("schema.yaml")).unwrap_or(false),
            "expected the referenced file path, got {:?}",
            origin.uri,
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
        let effective = DarkmatterSchemas::new()
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
        let effective = DarkmatterSchemas::new()
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
        let effective = DarkmatterSchemas::new()
            .with_file_resolution_context(snapshot)
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
        let effective = DarkmatterSchemas::new()
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
    const CLAUDINE_TRIGGER: &str = "kind: trigger-schema\n\
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

        let baseline = SimplifiedSchema::Single(SchemaShape {
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
        let api = DarkmatterSchemas::new()
            .with_baseline(baseline)
            .unwrap()
            .with_trigger_discovery(&doc_path, root)
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

        let api = DarkmatterSchemas::new()
            .with_trigger_discovery(&doc_path, root)
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

        let api = DarkmatterSchemas::new()
            .with_trigger_discovery(&doc_path, root)
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

        let api = DarkmatterSchemas::new()
            .with_trigger_discovery(&doc_path, root)
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
            "kind: trigger-schema\nmatch:\n  title: string(required)\n$schema: other.yaml\n",
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

        let api = DarkmatterSchemas::new()
            .with_trigger_discovery(&doc_path, root)
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

    #[test]
    fn non_mergeable_root_union_payload_rejected() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(
            &root.join("schemas/bad.trigger.yaml"),
            "kind: trigger-schema\n\
             match:\n  prompt: string(required)\n\
             $schema:\n  - a: string(required)\n  - b: string(required)\n",
        );

        let doc_path = root.join("doc.md");
        write(&doc_path, "---\nprompt: hello\n---\nbody\n");

        let err = DarkmatterSchemas::new()
            .with_trigger_discovery(&doc_path, root)
            .err()
            .expect("non-mergeable payload must fail at scan time");
        assert!(
            matches!(err, SchemaError::TriggerLoad { ref source, .. }
                if matches!(**source, SchemaError::TriggerPayloadNotMergeable { .. })),
            "root union payload must be rejected at scan time: {err:?}"
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

        let api = DarkmatterSchemas::new()
            .with_trigger_discovery(&doc_path, root)
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
            "kind: trigger-schema\n\
             match:\n  prompt: string(required)\n\
             $schema: self.trigger.yaml\n",
        );

        let doc_path = root.join("doc.md");
        write(&doc_path, "---\nprompt: hello\n---\nbody\n");

        let err = DarkmatterSchemas::new()
            .with_trigger_discovery(&doc_path, root)
            .err()
            .expect("self-referencing payload must fail at scan time");
        assert!(
            matches!(err, SchemaError::TriggerLoad { ref source, .. }
                if matches!(**source, SchemaError::TriggerPayloadCycle { .. })),
            "self-referencing payload must fail at scan time: {err:?}"
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
            "kind: trigger-schema\n\
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

        let err = DarkmatterSchemas::new()
            .with_trigger_discovery(&doc_path, root)
            .err()
            .expect("payload referencing a trigger must fail at scan time");
        assert!(
            matches!(err, SchemaError::TriggerLoad { ref source, .. }
                if matches!(**source, SchemaError::TriggerPayloadCycle { .. })),
            "payload referencing another trigger must fail at scan time: {err:?}"
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

        let api = DarkmatterSchemas::new()
            .with_trigger_discovery(&doc_path, root)
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

        let api = DarkmatterSchemas::new()
            .with_trigger_discovery(&doc_path, root)
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
        let trigger_uri = model_origin.and_then(|o| o.uri.as_ref());
        assert!(
            trigger_uri.is_some_and(|u| u.to_string_lossy().contains("claudine.trigger.yaml")),
            "trigger origin must point at the envelope file: {trigger_uri:?}"
        );
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

        let api = DarkmatterSchemas::new()
            .with_trigger_discovery(&doc_path, root)
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
        let registry = triggers::scan(&doc_path, root).unwrap();
        let api = DarkmatterSchemas::new().with_trigger_registry(registry);

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

        let api = DarkmatterSchemas::new();
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

        let api = DarkmatterSchemas::new()
            .with_trigger_discovery(&doc_path, root)
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
        let baseline = SimplifiedSchema::Single(SchemaShape {
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
        let api = DarkmatterSchemas::new()
            .with_baseline(baseline)
            .unwrap()
            .with_trigger_discovery(&doc_path, root)
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
}
