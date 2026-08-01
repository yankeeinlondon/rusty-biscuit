//! The one normalization pipeline for *formal* sequence documents.
//!
//! A formal sequence document is a root carrying the `sequence:` list together
//! with the optional `template:` and `$schema:` keys. Two entry paths reach that
//! root — `claudine sequence steps.yaml` invokes it directly, while
//! `sequence: steps.yaml` reaches it from another document through
//! [`super::source`] — and both funnel through [`normalize_formal_plan`]. That
//! single funnel is what makes the spec's "both entry modes accept the identical
//! document shape" true rather than aspirational.
//!
//! Order inside the pipeline is load-bearing: scalar shorthand expands first so
//! `- blue` can carry template defaults, the template fills in next so a
//! templated key is ordinary authored state, generated fields (`id`, `index`,
//! `is_first`, …) are inserted by [`normalize_plan`] after that, and `$schema`
//! validates the finished state last.

use std::path::Path;

use serde_json::{Map, Value};

use super::super::error::{CompositionError, SequenceLoadCause};
use super::super::json_util::json_type_name;
use super::super::resolve::is_yaml_source;
use super::super::types::ResolvedCompositionSource;
use super::expr::{SourceExpressionLookup, render_interpolated};
use super::model::{SequencePlan, SequenceSource};
use super::normalize::normalize_plan;
use super::reserved;

/// The keys a formal sequence document may carry beside its `sequence:` list.
#[derive(Default, Clone, Copy)]
pub struct FormalKeys<'a> {
    /// The document's `template:` map: defaults merged into every step.
    pub template: Option<&'a Map<String, Value>>,
    /// The document's `$schema`, applied to each step's normalized *state*.
    pub schema: Option<&'a Value>,
}

/// Whether `source` is a formal sequence document invoked directly.
///
/// A YAML source's root mapping *is* its frontmatter, so an inline `sequence:`
/// list there means the document Claudine is composing and the document
/// defining the steps are one and the same — and its `template:`/`$schema:`
/// belong to [`normalize_formal_plan`], not to the composition frontmatter.
/// A Markdown document's frontmatter is a composition frontmatter that happens
/// to declare a sequence, so its `$schema` keeps describing the document.
pub fn is_direct_formal_document(source: &ResolvedCompositionSource) -> bool {
    is_yaml_source(&source.resolved_path)
        && source
            .markdown
            .frontmatter()
            .as_map()
            .get("sequence")
            .is_some_and(Value::is_array)
}

/// Read the formal keys off a document root, validating `template`'s shape.
///
/// ## Errors
///
/// Returns [`CompositionError::SequenceExternalWrongType`] when `template` is
/// present but is not a mapping.
pub fn formal_keys(root: &Map<String, Value>) -> Result<FormalKeys<'_>, CompositionError> {
    let template = match root.get("template") {
        Some(Value::Object(map)) => Some(map),
        Some(other) => {
            return Err(CompositionError::SequenceExternalWrongType(format!(
                "`template` must be an object, got {}",
                json_type_name(other)
            )));
        }
        None => None,
    };
    Ok(FormalKeys {
        template,
        schema: root.get("$schema"),
    })
}

/// Normalize a formal document's item list into a [`SequencePlan`].
///
/// `invocation_path` is the document that *ran* the sequence — it seeds the
/// `sequence_id` payload and anchors template expressions. `document_path` is
/// the document that *declared* the steps; the two differ only when the formal
/// document was referenced, and it is `document_path` that names the file in a
/// schema-load failure. `globals` are the invoking document's frontmatter,
/// shadowed by each item's own fields during template rendering.
///
/// ## Errors
///
/// Returns the typed [`CompositionError`] variants raised by template
/// evaluation, by [`normalize_plan`], and by step-state schema validation.
pub fn normalize_formal_plan(
    items: Vec<Value>,
    keys: FormalKeys<'_>,
    source: SequenceSource,
    invocation_path: &Path,
    document_path: &Path,
    globals: &Map<String, Value>,
    document_fail_fast: bool,
) -> Result<SequencePlan, CompositionError> {
    // Shorthand expansion is scoped to the templated case: without a template
    // there is nothing to merge into, and `normalize_plan` reads a bare scalar
    // and a `{name: …}` object as the same step anyway.
    let items = match keys.template {
        Some(template) => {
            let items = expand_scalar_shorthand(items);
            apply_template(template, items, globals, invocation_path)?
        }
        None => items,
    };

    let plan = normalize_plan(&items, source, invocation_path, document_fail_fast)?;

    if let Some(schema) = keys.schema {
        validate_state_schema(&plan, schema, document_path)?;
    }
    Ok(plan)
}

/// Expand `- blue` shorthand into `{name: blue}` so a scalar step is an
/// ordinary state object by the time template defaults are merged.
///
/// Anything that is neither a string nor an object passes through untouched:
/// `normalize_plan` owns the vocabulary for rejecting it (or, for a foreign
/// source, for coercing it), and pre-empting that here would trade a precise
/// error for a vague one.
fn expand_scalar_shorthand(items: Vec<Value>) -> Vec<Value> {
    items
        .into_iter()
        .map(|item| match item {
            Value::String(name) => {
                let mut map = Map::new();
                map.insert("name".to_string(), Value::String(name));
                Value::Object(map)
            }
            other => other,
        })
        .collect()
}

/// Merge a formal document's `template:` defaults into every item.
///
/// A template key never overwrites a value the item already defines. A string
/// value is rendered through the Darkmatter expression engine with the item's
/// fields shadowing the invoking document's frontmatter; any other value is a
/// literal default and is copied verbatim — `template: { rank: 42 }` is the
/// `'<string>': any` shape the spec asks for, not a type error.
fn apply_template(
    template: &Map<String, Value>,
    items: Vec<Value>,
    globals: &Map<String, Value>,
    invocation_path: &Path,
) -> Result<Vec<Value>, CompositionError> {
    for key in template.keys() {
        if reserved::is_reserved_state_key(key) {
            return Err(CompositionError::SequenceReservedTemplateKey(key.clone()));
        }
    }

    let base_dir = invocation_path.parent().unwrap_or_else(|| Path::new("."));

    items
        .into_iter()
        .map(|item| {
            let Some(step_map) = item.as_object() else {
                return Ok(item);
            };
            let lookup = SourceExpressionLookup::new(globals, base_dir).with_item(step_map);

            let mut new_map = step_map.clone();
            for (key, value) in template {
                if new_map.contains_key(key) {
                    continue;
                }
                let filled = match value {
                    Value::String(raw) => render_interpolated(raw, &lookup)?,
                    literal => literal.clone(),
                };
                new_map.insert(key.clone(), filled);
            }
            Ok(Value::Object(new_map))
        })
        .collect()
}

/// Validate every step's normalized state against the document's `$schema`.
///
/// Only the state portion is validated: executable and task keys are not state
/// and were already partitioned out during normalization. The schema is applied
/// through the ordinary Darkmatter document validator by presenting each state
/// as a frontmatter-shaped document, so `$schema` behaves identically here and
/// on a normal composition source.
fn validate_state_schema(
    plan: &SequencePlan,
    schema: &Value,
    document_path: &Path,
) -> Result<(), CompositionError> {
    let schemas = darkmatter::markdown::schemas::DarkmatterSchemas::new();

    let load_failure = |source: SequenceLoadCause| CompositionError::SequenceExternalLoad {
        context: biscuit_file::to_portable_string(document_path),
        source,
    };

    for step in &plan.steps {
        let mut frontmatter = darkmatter::markdown::Frontmatter::new();
        frontmatter
            .insert("$schema", schema.clone())
            .map_err(|e| load_failure(SequenceLoadCause::Frontmatter(Box::new(e))))?;
        let Value::Object(state) = step.state.to_value() else {
            unreachable!("StepState::to_value always produces an object");
        };
        for (key, value) in state {
            frontmatter
                .insert(&key, value)
                .map_err(|e| load_failure(SequenceLoadCause::Frontmatter(Box::new(e))))?;
        }

        let markdown = darkmatter::markdown::Markdown::with_frontmatter(frontmatter, "");
        let report = schemas
            .validate(&markdown)
            .map_err(|e| load_failure(SequenceLoadCause::Schema(Box::new(e))))?;

        if let Some(problem) = report.problems.first() {
            // `property` is set only for missing-required failures; every other
            // kind locates itself with the instance pointer instead.
            let property = problem
                .property
                .clone()
                .unwrap_or_else(|| problem.path.clone());
            return Err(CompositionError::SequenceStateSchemaViolation {
                index: step.index,
                id: step.state.id.clone(),
                property,
                message: problem.message.clone(),
            });
        }
    }

    Ok(())
}
