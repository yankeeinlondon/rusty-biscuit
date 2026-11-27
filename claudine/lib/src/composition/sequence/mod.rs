//! Sequence Plus: detection, normalization, sources, and the reserved overlay.
//!
//! [`resolve_sequence_plan`] detects whether a resolved composition source
//! defines a `sequence`, and if so normalizes it into a typed [`SequencePlan`]
//! of generated [`StepState`]s. [`build_step_overlay`] constructs the reserved
//! per-step binding used to build the lexical `seq` root, carried
//! into each composition run.
//!
//! The module is split by responsibility:
//!
//! - [`model`] — the typed domain vocabulary (states, sources, overlay, the
//!   executable/task/output/runtime representations later phases consume);
//! - [`grammar`] — source classification and the
//!   `<file-ref> [-> offset] [::op(args)]` suffix parser;
//! - [`data`] — multi-format loading, offset traversal, operator application;
//! - [`expr`] — expression evaluation for dynamic sources and templates;
//! - [`formal`] — the one pipeline for authored lists (scalar shorthand →
//!   `template:` → generated fields → `$state`) every entry mode shares;
//! - [`document`] — the document that owns a list, and the prepared values
//!   its `template:` reads;
//! - [`normalize`] — strict/lenient state normalization, id suffixing, and the
//!   `sequence_id` invocation token;
//! - [`preflight`] — the recursive task graph: external task/group/catalog/
//!   prompt loading, cycle detection, and early-binding shell resolution;
//! - [`source`] — file-reference resolution and referenced-source loading;
//! - [`task`] — recursive task execution, container lifecycles, and
//!   the one [`task::TaskOutcome`] every task variant reports;
//! - [`reserved`] — the canonical reserved-key catalog shared by every layer.
//!
//! See `claudine/features/2026-07-11-sequence-plus/spec.md`.

pub mod data;
pub mod declarations;
pub mod document;
pub mod expr;
pub mod formal;
pub mod grammar;
pub mod model;
pub mod normalize;
pub mod preflight;
pub mod removed;
pub mod reserved;
pub mod source;
pub mod task;

use std::path::Path;
use std::sync::Arc;

use biscuit_file::classify_list;
use serde_json::{Map, Value};

use super::authored_order::AuthoredOrder;
use super::types::ResolvedCompositionSource;

pub use grammar::{SequenceReference, SequenceSourceSpec, SourceOperator};
pub use model::{
    ExecutableField, ExternalTaskRef, GroupRef, OutputEntry, RuntimeMutation, SequencePlan,
    SequenceSource, SequenceStep, SequenceStepOverlay, StepExecutable, StepState, Strictness,
};

use super::error::CompositionError;
use expr::SourceExpressionLookup;
use model::SequenceSource as Source;

/// Expands one approved `$( … )` sequence source to its stdout.
///
/// Injected rather than called directly so the library never owns the approval
/// prompt: the CLI supplies a runner backed by the shared approval cache and
/// the interactive handler. Without one, a `$( … )` source is a typed error
/// rather than a silent unapproved execution.
pub type ShellSourceRunner<'a> = &'a dyn Fn(&str) -> Result<String, CompositionError>;

/// Caller-supplied capabilities for sequence-source resolution.
///
/// Start from [`Self::new`] and add capabilities with the `with_*` builders.
#[derive(Clone, Copy)]
pub struct SequenceSourceOptions<'a> {
    /// Expander for `$( … )` sources. `None` rejects them.
    shell_runner: Option<ShellSourceRunner<'a>>,
    /// The request's file-resolution context, for external file-backed
    /// sources and source expressions.
    file_resolution_context: &'a biscuit_file::FileResolutionContext,
    /// The invocation's captured context. With it, an owner's `template:`
    /// reads the owner's prepared values
    /// ([`document::prepare_sequence_document_values`]); without it, its
    /// authored values under the caller's setters.
    compose_context: Option<&'a darkmatter::markdown::compose::ComposeContext>,
    /// The invocation's caller inputs: setters with their recorded origins.
    caller_layers: Option<&'a super::types::CallerInputLayers>,
    /// The invocation, which derives a referenced document's own
    /// file-resolution context for its owner values. Without it they resolve
    /// through [`Self::file_resolution_context`].
    invocation: Option<&'a crate::invocation_context::InvocationContext>,
}

impl<'a> SequenceSourceOptions<'a> {
    /// Options resolving through `file_resolution_context`, with no shell
    /// runner, no captured context, and no caller input.
    pub fn new(file_resolution_context: &'a biscuit_file::FileResolutionContext) -> Self {
        Self { shell_runner: None, file_resolution_context, compose_context: None, caller_layers: None, invocation: None }
    }

    /// Expand `$( … )` sources through `runner`.
    #[must_use]
    pub fn with_shell_runner(mut self, runner: ShellSourceRunner<'a>) -> Self {
        self.shell_runner = Some(runner);
        self
    }

    /// Prepare owner values against the invocation's captured `context`
    /// rather than capturing one.
    #[must_use]
    pub fn with_compose_context(mut self, context: &'a darkmatter::markdown::compose::ComposeContext) -> Self {
        self.compose_context = Some(context);
        self
    }

    /// Lay the invocation's caller inputs over every owner's own values.
    #[must_use]
    pub fn with_caller_layers(mut self, layers: &'a super::types::CallerInputLayers) -> Self {
        self.caller_layers = Some(layers);
        self
    }

    /// Resolve a referenced document's authored owner values from its own
    /// folder, through the context `invocation` derives for it, as the graph
    /// walk does. The caller's values keep their recorded origins.
    #[must_use]
    pub fn with_invocation(mut self, invocation: &'a crate::invocation_context::InvocationContext) -> Self {
        self.invocation = Some(invocation);
        self
    }

    /// The request's file-resolution context.
    pub fn file_resolution_context(&self) -> &'a biscuit_file::FileResolutionContext {
        self.file_resolution_context
    }
}

/// Detect and resolve a sequence plan from a resolved composition source.
///
/// Returns `Ok(None)` when the source has no `sequence` frontmatter key, and
/// `Ok(Some(plan))` for a valid inline or externally-referenced sequence.
///
/// ## Errors
///
/// Returns `Err` for invalid sequence definitions: wrong types, missing `name`
/// on object steps, more than one executable field, reserved-key
/// collisions, a root `when:`, an invalid root `loop:`, or external file load
/// failures. A valid root `loop:` or task `when:` is refused with its
/// not-yet-supported error. A YAML file declaring `kind: sequence` that
/// does not meet the formal sequence kind is
/// [`CompositionError::SchemaValidation`].
pub fn resolve_sequence_plan(
    source: &ResolvedCompositionSource,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> Result<Option<SequencePlan>, CompositionError> {
    resolve_sequence_plan_with(source, SequenceSourceOptions::new(file_resolution_context))
}

/// Judges a YAML file that declares `kind: sequence` and an inline
/// `sequence` list against Claudine's prepared formal sequence kind
/// ([`formal_sequence_kind`]), strictly and as written. A Markdown document,
/// or a YAML file claiming no kind or another one, is not judged here.
///
/// ## Panics
///
/// When the compiled-in definition does not compile: a build defect a test
/// catches.
///
/// [`formal_sequence_kind`]: crate::composition::schema::shipped::formal_sequence_kind
fn validate_formal_sequence_file(
    source: &ResolvedCompositionSource,
    context: &biscuit_file::FileResolutionContext,
) -> Result<(), CompositionError> {
    let fm = source.markdown.frontmatter();
    let claims_sequence = fm.as_map().get("kind").and_then(|kind| kind.as_str()) == Some("sequence");
    if !claims_sequence || !super::resolve::is_yaml_source(&source.resolved_path) {
        return Ok(());
    }
    let refused = |message: String, problems: Vec<String>| CompositionError::SchemaValidation {
        source_path: source.resolved_path.clone(),
        message,
        problems,
        failures: Vec::new(),
    };
    let kind = crate::composition::schema::shipped::formal_sequence_kind();
    let base_dir = source.resolved_path.parent().map(Path::to_path_buf);
    // The definition is compiled in and proven to compile by the shipped
    // artifact's tests; nothing the document holds can make it fail.
    let prepared = simplified_schema::Prepared::new(Arc::clone(&kind.json_schema), base_dir, context.clone())
        .unwrap_or_else(|error| panic!("Claudine's `kind: sequence` definition must compile: {error}"));
    let document: Value = fm
        .as_map()
        .iter()
        .map(|(key, value)| (key.clone(), crate::composition::closure::stored_text(value)))
        .collect::<Map<String, Value>>()
        .into();
    let failure = match prepared.validate_strict(&document) {
        Ok(_) => return Ok(()),
        Err(failure) => failure,
    };
    if let simplified_schema::ValidationFailure::Unprepared { error } = &failure {
        return Err(refused(format!("the `kind: sequence` definition cannot judge the file: {error}"), Vec::new()));
    }
    let problems = failure.problems();
    let message = problems
        .iter()
        .map(|problem| if problem.path.is_empty() { problem.message.clone() } else { format!("{}: {}", problem.path, problem.message) })
        .collect::<Vec<_>>()
        .join("; ");
    Err(refused(
        format!("not a valid `kind: sequence` file: {message}"),
        problems.iter().map(|problem| problem.path.clone()).collect(),
    ))
}

/// [`resolve_sequence_plan`] with caller-supplied capabilities.
///
/// ## Errors
///
/// As [`resolve_sequence_plan`], plus
/// [`CompositionError::SequenceShellFailed`] when a `$( … )` source is present
/// and `options` carries no runner.
pub fn resolve_sequence_plan_with(
    source: &ResolvedCompositionSource,
    options: SequenceSourceOptions<'_>,
) -> Result<Option<SequencePlan>, CompositionError> {
    // A failure the source layers raised without knowing the file is the
    // root's `sequence` value's.
    resolve_root_plan(source, options).map_err(|error| error.locate_sequence(&source.resolved_path, "sequence", None))
}

fn resolve_root_plan(
    source: &ResolvedCompositionSource,
    options: SequenceSourceOptions<'_>,
) -> Result<Option<SequencePlan>, CompositionError> {
    let fm = source.markdown.frontmatter();
    let sequence_value = match fm.as_map().get("sequence") {
        Some(v) => v.clone(),
        None => return Ok(None),
    };

    if let Some(value) = fm.as_map().get("fail_fast") {
        return Err(removed::removed_key(removed::RemovedOwner::Root, "fail_fast", value, &source.resolved_path));
    }
    let fail_fast = true;

    // The frontmatter's insertion-ordered map is projected into a plain JSON
    // object once here: expression lookups only ever read it by key, and the
    // conversion keeps every downstream signature on one map type. Stored
    // literal tokens are decoded, as composition would read them.
    let frontmatter: Map<String, Value> = fm
        .as_map()
        .iter()
        .map(|(key, value)| (key.clone(), crate::composition::closure::stored_text(value)))
        .collect();
    // The root's `when` and `loop` are judged before the source is expanded,
    // so a refused root runs no `$( … )` source either.
    grammar::validate_sequence_root(&frontmatter, &source.resolved_path)?;
    let mut plan = match grammar::classify_source(&sequence_value)? {
        // An authored list is the document's own, in Markdown frontmatter or a
        // YAML root alike, and takes the one formal pipeline.
        SequenceSourceSpec::Inline(items) => {
            let authored = Arc::new(frontmatter.clone());
            let keys = formal::formal_keys(&authored, &source.resolved_path)?;
            let values = Arc::new(document::owner_values(&authored, &keys, &options, None, || {
                Ok(std::borrow::Cow::Borrowed(source))
            })?);
            let mut plan = formal::normalize_formal_plan(
                items,
                formal::FormalOwner {
                    keys,
                    declaring_path: &source.resolved_path,
                    list_property: "sequence",
                    values: &*values,
                },
                Source::Inline,
                &source.resolved_path,
                fail_fast,
                options.file_resolution_context,
            )?;
            // The runtime grammar's own refusals are more precise (a task's
            // declaration, a `$state` that does not load), so the kind judges
            // only what that grammar accepted, before anything runs.
            validate_formal_sequence_file(source, options.file_resolution_context)?;
            plan.document_scope = Some(Arc::new(document::SequenceDocumentScope {
                declaring_path: source.resolved_path.clone(),
                property: "sequence".into(),
                authored: Arc::clone(&authored),
                values,
            }));
            plan
        }
        SequenceSourceSpec::Expression(expression) => resolve_expression_source(
            &expression,
            &frontmatter,
            &source.resolved_path,
            options.file_resolution_context,
            fail_fast,
        )?,
        SequenceSourceSpec::Shell(command) => {
            resolve_shell_source(&command, &options, &source.resolved_path, fail_fast)?
        }
        SequenceSourceSpec::Reference(reference) => {
            let path = source::resolve_sequence_reference(
                &reference.reference,
                model::ExecutableField::Sequence,
                &source.resolved_path,
                options.file_resolution_context,
            )?;
            source::load_referenced_sequence(
                &reference,
                &path,
                &source.resolved_path,
                &frontmatter,
                fail_fast,
                &options,
            )?
        }
    };

    // An inline `sequence:` list lives in this document's own frontmatter, so
    // its steps are indexed under `/sequence`. A referenced plan already
    // carries the *referenced* document's order root and must keep it.
    if sequence_value.is_array() {
        plan.authored = AuthoredOrder::new(Arc::new(fm.mapping_orders().clone()), "/sequence");
    }

    Ok(Some(plan))
}

/// Resolve a whole-value `{{ … }}` source.
///
/// A typed array result is already a list of entries and bypasses `ListFormat`
/// classification entirely; any other value is rendered to a string and
/// classified, which is what makes the CSV-by-default `ctx.*` list variables
/// work without asking the author to convert them.
fn resolve_expression_source(
    expression: &str,
    frontmatter: &Map<String, Value>,
    invocation_path: &Path,
    file_resolution_context: &biscuit_file::FileResolutionContext,
    fail_fast: bool,
) -> Result<SequencePlan, CompositionError> {
    let items = expression_source_items(expression, frontmatter, invocation_path, file_resolution_context)?;
    normalize::normalize_plan(&items, Source::Expression, invocation_path, fail_fast)
}

/// The items a whole-value `{{ … }}` source yields, before any is read as a
/// task: the list [`resolve_expression_source`] normalizes.
fn expression_source_items(
    expression: &str,
    frontmatter: &Map<String, Value>,
    invocation_path: &Path,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> Result<Vec<Value>, CompositionError> {
    let lookup = SourceExpressionLookup::new(frontmatter, file_resolution_context, invocation_path);
    Ok(match expr::evaluate_whole(expression, &lookup)? {
        Value::Array(items) => items,
        Value::Null => Vec::new(),
        Value::String(text) => classify_string_items(&text),
        other => classify_string_items(&other.to_string()),
    })
}

/// Resolve a `$( … )` source through the caller's approved-command runner.
fn resolve_shell_source(
    command: &str,
    options: &SequenceSourceOptions<'_>,
    invocation_path: &Path,
    fail_fast: bool,
) -> Result<SequencePlan, CompositionError> {
    let runner = options
        .shell_runner
        .ok_or_else(|| CompositionError::SequenceShellFailed {
            command: command.to_string(),
            source: super::error::SequenceShellCause::Unavailable,
            site: Box::new(super::error::SequenceSite::at(invocation_path, "sequence")),
        })?;

    let output = runner(command)?;
    let items = classify_string_items(&output);
    normalize::normalize_plan(&items, Source::Shell, invocation_path, fail_fast)
}

/// Classify a textual list and turn its entries into item values.
fn classify_string_items(text: &str) -> Vec<Value> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    classify_list(text)
        .1
        .into_iter()
        .map(Value::String)
        .collect()
}

/// Build the reserved overlay for `step_index` within `plan`.
///
/// Neighbor rows are absent at the boundaries. `SequenceBinding` projects
/// this position into `seq`; the runtime snapshot supplies its live outputs.
pub fn build_step_overlay(plan: &SequencePlan, step_index: usize) -> SequenceStepOverlay {
    let total = plan.steps.len();
    SequenceStepOverlay {
        state: plan.steps[step_index].state.clone(),
        previous: (step_index > 0).then(|| plan.steps[step_index - 1].state.clone()),
        next: (step_index + 1 < total).then(|| plan.steps[step_index + 1].state.clone()),
        sequence_id: plan.sequence_id.clone(),
    }
}

#[cfg(test)]
mod tests;
