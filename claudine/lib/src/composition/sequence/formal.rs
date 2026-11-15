//! The one normalization pipeline for *formal* sequence documents.
//!
//! A formal sequence document is a root carrying the `sequence:` list together
//! with the optional `template:` and `$state:` declarations
//! ([`super::declarations`]). Its root `$schema` is the document schema and is
//! not read here. Two entry paths reach that
//! root — `claudine sequence steps.yaml` invokes it directly, while
//! `sequence: steps.yaml` reaches it from another document through
//! [`super::source`] — and both funnel through [`normalize_formal_plan`]. That
//! single funnel is what makes the spec's "both entry modes accept the identical
//! document shape" true rather than aspirational.
//!
//! Order inside the pipeline is load-bearing: scalar shorthand expands first so
//! `- blue` can carry template defaults, the template fills in next so a
//! templated key is ordinary authored state, generated fields (`id`, `index`,
//! `is_first`, …) are inserted by [`normalize_plan`] after that, and `$state`
//! validates the finished state last. `$state` is prepared before any row is
//! read, once, so a malformed declaration fails an empty list too.

use std::path::Path;

use serde_json::{Map, Value};

use super::super::error::CompositionError;
use super::super::resolve::is_yaml_source;
use super::super::types::ResolvedCompositionSource;
use super::declarations::{self, PreparedSequenceSchema};
use super::expr::{DocumentFrontmatter, SourceExpressionLookup, render_interpolated};
use super::model::{SequencePlan, SequenceSource};
use super::normalize::normalize_plan;

/// The declarations a formal sequence document carries beside its
/// `sequence:` list, as authored.
#[derive(Default, Clone, Copy)]
pub struct FormalKeys<'a> {
    /// The document's `template:` map: defaults merged into every step.
    pub template: Option<&'a Map<String, Value>>,
    /// The document's `$state`, applied to each step's normalized *state*.
    pub state_schema: Option<&'a Value>,
}

/// Whether `source` is a formal sequence document invoked directly.
///
/// A YAML source's root mapping *is* its frontmatter, so an inline `sequence:`
/// list there means the document Claudine is composing and the document
/// defining the steps are one and the same — and its `template:`/`$schema:`
/// belong to [`normalize_formal_plan`], not to the composition frontmatter.
/// Its `$schema` describes the document in either format.
pub fn is_direct_formal_document(source: &ResolvedCompositionSource) -> bool {
    is_yaml_source(&source.resolved_path)
        && source
            .markdown
            .frontmatter()
            .as_map()
            .get("sequence")
            .is_some_and(Value::is_array)
}

/// Read the formal declarations off the root of the document at
/// `source_path`, judging their placement and shape
/// ([`declarations::root_declarations`]).
///
/// ## Errors
///
/// The `SequenceDeclaration*` placement and shape errors, and
/// [`CompositionError::SequenceReservedTemplateKey`].
pub fn formal_keys<'a>(root: &'a Map<String, Value>, source_path: &Path) -> Result<FormalKeys<'a>, CompositionError> {
    if let Some(value) = root.get("fail_fast") {
        return Err(super::grammar::removed_task_key("fail_fast", value, source_path));
    }
    let declarations = declarations::root_declarations(root, source_path)?;
    Ok(FormalKeys {
        template: declarations.template,
        state_schema: declarations.state,
    })
}

/// Normalize a formal document's item list into a [`SequencePlan`].
///
/// `invocation_path` is the document that *ran* the sequence — it seeds the
/// `sequence_id` payload and anchors template expressions. `document_path` is
/// the document that *declared* the steps; the two differ only when the formal
/// document was referenced, and it is `document_path` whose context resolves
/// `$state` and that names the file in a declaration or state failure. `globals` are the invoking document's frontmatter,
/// shadowed by each item's own fields during template rendering.
///
/// ## Errors
///
/// Returns the typed [`CompositionError`] variants raised by `$state`
/// preparation, template evaluation, [`normalize_plan`], and step-state
/// validation.
#[allow(clippy::too_many_arguments)]
pub fn normalize_formal_plan(
    items: Vec<Value>,
    keys: FormalKeys<'_>,
    source: SequenceSource,
    invocation_path: &Path,
    document_path: &Path,
    globals: &dyn DocumentFrontmatter,
    document_fail_fast: bool,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> Result<SequencePlan, CompositionError> {
    // The contract is resolved before any row is read: a malformed or
    // unloadable `$state` fails an empty list too, and it resolves once
    // however many rows it judges.
    let state_schema = keys
        .state_schema
        .map(|declaration| {
            PreparedSequenceSchema::prepare(declaration, document_path, declarations::STATE_KEY, file_resolution_context)
        })
        .transpose()?;

    // Shorthand expansion is scoped to the templated case: without a template
    // there is nothing to merge into, and `normalize_plan` reads a bare scalar
    // and a `{name: …}` object as the same step anyway.
    let items = match keys.template {
        Some(template) => {
            let items = expand_scalar_shorthand(items);
            apply_template(template, items, globals, invocation_path, file_resolution_context)?
        }
        None => items,
    };

    let plan = normalize_plan(&items, source, invocation_path, document_fail_fast)?;

    if let Some(schema) = &state_schema {
        schema.validate_states(&plan, "sequence")?;
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
/// The template's keys were judged with its shape
/// ([`declarations::root_declarations`]). A template key never overwrites a value the item already defines. A string
/// value is rendered through the Darkmatter expression engine with the item's
/// fields shadowing the invoking document's frontmatter; any other value is a
/// literal default and is copied verbatim — `template: { rank: 42 }` is the
/// `'<string>': any` shape the spec asks for, not a type error.
fn apply_template(
    template: &Map<String, Value>,
    items: Vec<Value>,
    globals: &dyn DocumentFrontmatter,
    invocation_path: &Path,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> Result<Vec<Value>, CompositionError> {
    items
        .into_iter()
        .map(|item| {
            let Some(step_map) = item.as_object() else {
                return Ok(item);
            };
            let lookup = SourceExpressionLookup::new(globals, file_resolution_context, invocation_path)
                .with_item(step_map);

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
