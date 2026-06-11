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
//! - [`format`] — custom format validators (`darkmatter-file`) and keyword
//!   validators (`x-darkmatter-match`, `x-darkmatter-url-scheme`).
//! - [`validate`] — `Validator` construction + LRU [`ValidatorCache`].
//! - [`resolve`] — `$schema` resolution and baseline merge.
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
pub mod coerce;
pub mod completion;
pub mod detect;
pub mod errors;
pub mod format;
pub mod resolve;
pub mod simplified;
pub mod validate;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use jsonschema::Validator;
use serde_json::Value;

use crate::markdown::{Markdown, compose::ComposeSource};

pub use about::{
    CoercionRuleDescriptor, InlineObjectRuleDescriptor, SchemaConstraintDescriptor, SchemaShapeDescriptor,
    SchemaTypeDescriptor, ValidationBehaviorDescriptor, coercion_rule_descriptors,
    inline_object_rule_descriptors, schema_constraint_descriptors, schema_shape_descriptors,
    schema_type_descriptors, validation_behavior_descriptors,
};
pub use completion::{CompletionKind, CompletionSuggestion};
pub use detect::{DetectOptions, detect_from_document, detect_schema, schema_to_yaml};
pub use errors::SchemaError;
pub use simplified::{
    Constraint, DRAFT_2020_12, PropertyAtom, PropertyDef, SchemaArm, SchemaShape, SimplifiedSchema,
    SimplifiedType, TypeExpr, parse_yaml_schema, to_json_schema,
};
pub use validate::{CACHE_SIZE_ENV, DEFAULT_CACHE_SIZE, PositionMap, ValidatorCache};

/// Top-level entry point for the schemas subsystem.
///
/// Holds optional baseline schema configuration and the process-wide
/// validator cache so repeated validations against the same effective
/// schema reuse the compiled `jsonschema::Validator`.
#[derive(Default, Clone)]
pub struct DarkmatterSchemas {
    baseline: Option<BaselineSchema>,
    cache: ValidatorCache,
}

#[derive(Clone)]
struct BaselineSchema {
    json_schema: Value,
}

impl DarkmatterSchemas {
    /// Creates a new [`DarkmatterSchemas`] with no baseline.
    pub fn new() -> Self {
        Self::default()
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

    fn set_baseline_json(&mut self, value: Value) -> Result<(), SchemaError> {
        // Probe the baseline by running it through the merger on a trivial
        // document — this enforces the simple-object-schema restriction at
        // configuration time rather than at first use.
        let probe = serde_json::json!({"type":"object","properties":{}});
        resolve::merge_baseline(&value, probe)?;
        self.baseline = Some(BaselineSchema { json_schema: value });
        Ok(())
    }

    /// Builds the effective schema for a Markdown document. Reads its
    /// `$schema` frontmatter (if any), resolves it, and merges the baseline.
    ///
    /// When `$schema` is absent and a baseline is configured, the baseline
    /// is returned as-is. When both are absent, returns `Ok(None)`.
    ///
    /// ## Errors
    ///
    /// Propagates [`SchemaError`] from parsing, resolution, conversion, or
    /// validator construction.
    pub fn effective_for(&self, source: &Markdown) -> Result<Option<EffectiveSchema>, SchemaError> {
        let base_dir = base_dir_for(source);
        let frontmatter = source.frontmatter().as_map();
        let schema_value = frontmatter.get("$schema");

        let resolved = match schema_value {
            Some(value) => Some(resolve::resolve_schema(value, &base_dir)?),
            None => None,
        };

        let merged_json = match (&resolved, &self.baseline) {
            (Some(r), Some(b)) => resolve::merge_baseline(&b.json_schema, r.json_schema.clone())?,
            (Some(r), None) => r.json_schema.clone(),
            (None, Some(b)) => b.json_schema.clone(),
            (None, None) => return Ok(None),
        };

        let validator = self.cache.validator_for(&merged_json)?;
        let arm_validators = build_arm_validators(&merged_json, &self.cache)?;
        Ok(Some(EffectiveSchema {
            simplified: resolved.and_then(|r| r.simplified),
            json_schema: merged_json,
            validator,
            arm_validators,
        }))
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
    pub json_schema: Value,
    validator: Arc<Validator>,
    /// Per-arm validators when `json_schema` is a root `anyOf` union.
    /// `None` for ordinary schemas.
    arm_validators: Option<Vec<Arc<Validator>>>,
}

impl EffectiveSchema {
    /// Validates a frontmatter JSON value against this schema. Equivalent to
    /// [`Self::validate_with_positions`] with an empty position map (problems
    /// will carry no line/column information).
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
    /// the report reflects post-coercion validity. No input is mutated.
    pub fn validate_with_positions(
        &self,
        frontmatter: &Value,
        positions: &PositionMap,
    ) -> ValidationReport {
        let coerced = coerce::coerce_frontmatter(&self.json_schema, frontmatter);
        let problems = match &self.arm_validators {
            Some(arms) => validate::collect_root_union_problems(arms, &coerced.value, positions),
            None => validate::collect_problems(&self.validator, &coerced.value, positions),
        };
        ValidationReport {
            valid: problems.is_empty(),
            problems,
        }
    }

    /// Returns the compiled validator. Mainly for advanced callers; the
    /// `validate` method is the normal entry point.
    pub fn validator(&self) -> &Validator {
        &self.validator
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
}

fn build_arm_validators(
    schema: &Value,
    cache: &ValidatorCache,
) -> Result<Option<Vec<Arc<Validator>>>, SchemaError> {
    let Some(arms) = schema.get("anyOf").and_then(Value::as_array) else {
        return Ok(None);
    };
    let mut out = Vec::with_capacity(arms.len());
    for arm in arms {
        let arm_schema = validate::wrap_arm_as_root_schema(arm);
        out.push(cache.validator_for(&arm_schema)?);
    }
    Ok(Some(out))
}

fn positions_for(source: &Markdown) -> PositionMap {
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

fn frontmatter_as_json(source: &Markdown) -> Value {
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
mod tests {
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
        let md = md_with_schema("$schema:\n  flag: boolean\nflag: \"yes\"\n");
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
}
