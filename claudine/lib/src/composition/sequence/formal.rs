//! The one normalization pipeline for authored sequence lists.
//!
//! An authored list's owner is the document whose root carries the
//! `sequence:` list together with the optional `template:` and `$state:`
//! declarations ([`super::declarations`]). Its root `$schema` is the document
//! schema and is not read here. The owner may be a Markdown or YAML document
//! invoked directly (`claudine sequence steps.yaml`), or a whole document
//! another one references (`sequence: steps.yaml`, through [`super::source`]);
//! every route funnels through [`normalize_formal_plan`], which is what makes
//! "every entry mode accepts the identical document shape" true rather than
//! aspirational.
//!
//! Order inside the pipeline is load-bearing: scalar shorthand expands first so
//! `- blue` can carry template defaults, the template fills in next so a
//! templated key is ordinary authored state, generated fields (`id`, `index`,
//! `is_first`, …) are inserted by [`normalize_plan_at`] after that, and `$state`
//! validates the finished state last. `$state` is prepared before any row is
//! read, once, so a malformed declaration fails an empty list too.

use std::path::Path;

use serde_json::{Map, Value};

use super::super::error::CompositionError;
use super::declarations::{self, PreparedSequenceSchema};
use super::expr::{DocumentFrontmatter, SourceExpressionLookup, render_interpolated};
use super::model::{SequencePlan, SequenceSource};
use super::normalize::normalize_plan_at;

/// The declarations a formal sequence document carries beside its
/// `sequence:` list, as authored.
#[derive(Default, Clone, Copy)]
pub struct FormalKeys<'a> {
    /// The document's `template:` map: defaults merged into every step.
    pub template: Option<&'a Map<String, Value>>,
    /// The document's `$state`, applied to each step's normalized *state*.
    pub state_schema: Option<&'a Value>,
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

/// The document that declares an authored list, as the formal pipeline
/// reads it.
pub struct FormalOwner<'a> {
    /// The owner's declarations ([`formal_keys`]).
    pub keys: FormalKeys<'a>,
    /// The file that declares the list. Its context resolves `$state` and
    /// anchors `template:` expressions, and it names the file in a
    /// declaration or state failure.
    pub declaring_path: &'a Path,
    /// The list's property path within that file (`sequence`), which locates
    /// task-grammar diagnostics.
    pub list_property: &'a str,
    /// The owner's values, which `template:` expressions read below each
    /// item's own fields.
    pub values: &'a dyn DocumentFrontmatter,
}

/// Normalize an owner's authored item list into a [`SequencePlan`].
///
/// `invocation_path` is the document that *ran* the sequence: it seeds the
/// `sequence_id` payload. Everything else about the list comes from `owner`,
/// which differs from the invoking document only when the list was read from
/// a referenced whole document.
///
/// ## Errors
///
/// Returns the typed [`CompositionError`] variants raised by `$state`
/// preparation, template evaluation, [`normalize_plan_at`], and step-state
/// validation.
pub fn normalize_formal_plan(
    items: Vec<Value>,
    owner: FormalOwner<'_>,
    source: SequenceSource,
    invocation_path: &Path,
    document_fail_fast: bool,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> Result<SequencePlan, CompositionError> {
    // The contract is resolved before any row is read: a malformed or
    // unloadable `$state` fails an empty list too, and it resolves once
    // however many rows it judges.
    let state_schema = owner
        .keys
        .state_schema
        .map(|declaration| {
            let property = super::preflight::property_child(
                list_property_owner(owner.list_property), declarations::STATE_KEY,
            );
            PreparedSequenceSchema::prepare(declaration, owner.declaring_path, &property, file_resolution_context)
        })
        .transpose()?;
    normalize_formal_plan_prepared(items, owner, state_schema.as_ref(), source, invocation_path, document_fail_fast, file_resolution_context)
}

/// The owner path of a list authored at `list_property`: empty for a root
/// `sequence`, `sequence[1]` for `sequence[1].sequence`.
fn list_property_owner(list_property: &str) -> &str {
    list_property.strip_suffix("sequence").map_or("", |owner| owner.strip_suffix('.').unwrap_or(owner))
}

/// [`normalize_formal_plan`] with the owner's `$state` already prepared
/// (`state_schema`), so a caller that keeps prepared declarations across
/// rereads of an unchanged owner does not resolve it again.
///
/// ## Errors
///
/// As [`normalize_formal_plan`], less `$state` preparation.
pub(crate) fn normalize_formal_plan_prepared(
    items: Vec<Value>,
    owner: FormalOwner<'_>,
    state_schema: Option<&PreparedSequenceSchema>,
    source: SequenceSource,
    invocation_path: &Path,
    document_fail_fast: bool,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> Result<SequencePlan, CompositionError> {
    let FormalOwner { keys, declaring_path, list_property, values } = owner;

    // Shorthand expansion is scoped to the templated case: without a template
    // there is nothing to merge into, and `normalize_plan_at` reads a bare
    // scalar and a `{name: …}` object as the same step anyway.
    let items = match keys.template {
        Some(template) => {
            let items = expand_scalar_shorthand(items);
            apply_template(template, items, values, declaring_path, file_resolution_context)?
        }
        None => items,
    };

    let plan = normalize_plan_at(&items, source, invocation_path, document_fail_fast, list_property)?;

    if let Some(schema) = state_schema {
        schema.validate_states(&plan, list_property)?;
    }
    Ok(plan)
}

/// Whether filling `keys`' template reads the owner's values at all: only a
/// string default is an expression; any other default is a literal.
#[must_use]
pub fn template_reads_values(keys: &FormalKeys<'_>) -> bool {
    keys.template.is_some_and(|template| template.values().any(Value::is_string))
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
/// fields shadowing the owner's values, and resolving file references from
/// `declaring_path`; any other value is a literal default and is copied
/// verbatim — `template: { rank: 42 }` is the `'<string>': any` shape the
/// spec asks for, not a type error.
fn apply_template(
    template: &Map<String, Value>,
    items: Vec<Value>,
    values: &dyn DocumentFrontmatter,
    declaring_path: &Path,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> Result<Vec<Value>, CompositionError> {
    items
        .into_iter()
        .map(|item| {
            let Some(step_map) = item.as_object() else {
                return Ok(item);
            };
            let mut new_map = step_map.clone();
            for (key, value) in template {
                if new_map.contains_key(key) {
                    continue;
                }
                let filled = fill_template_default(value, step_map, values, declaring_path, file_resolution_context)?;
                new_map.insert(key.clone(), filled);
            }
            Ok(Value::Object(new_map))
        })
        .collect()
}

/// One `template:` default filled for `item`: a string renders against the item's fields over `values`, resolving file
/// references from `declaring_path`; any other value is copied.
///
/// ## Errors
///
/// The template's evaluation error.
pub(crate) fn fill_template_default(
    default: &Value,
    item: &Map<String, Value>,
    values: &dyn DocumentFrontmatter,
    declaring_path: &Path,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> Result<Value, CompositionError> {
    match default {
        Value::String(raw) => {
            let lookup = SourceExpressionLookup::new(values, file_resolution_context, declaring_path).with_item(item);
            render_interpolated(raw, &lookup)
        }
        literal => Ok(literal.clone()),
    }
}
