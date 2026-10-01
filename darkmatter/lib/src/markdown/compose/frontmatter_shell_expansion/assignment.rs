//! Whole-value `$( … )` values that run outside a document's own frontmatter
//! pass, such as a Claudine lifecycle `set` assignment.
//!
//! Such a value has two lifetimes. Its command bytes are resolved once, when
//! the caller approves them ([`ResolvedShellValue::resolve`]), and are never
//! interpolated again. Its result is produced each time the value is executed
//! ([`execute_resolved_shell_values`]), in a fresh command cache, so a value
//! executed at two events runs its command twice. A ternary's condition and a
//! value branch are expression content, not command bytes: they are evaluated
//! when the value executes, against the state the caller supplies then.

use std::collections::HashMap;
use std::path::PathBuf;

use biscuit_terminal::errors::SourceContext;
use rayon::prelude::*;
use serde_json::Value;

use super::{
    Branch, BranchPosition, FrontmatterShellAst, FrontmatterShellDirective, Pending,
    ShellResultKind, evaluate_ternary_condition, evaluate_value_branch, expand_pending,
    frontmatter_parse_error, interpolate_branch_text, mask_interpolations,
    parse_shell_value, parse_static_pipeline_shape, prepare_branch_pipeline,
    remap_branch_parse_error, validate_pipeline_shape_preserved,
};
use crate::markdown::compose::expression::ResolutionContext;
use crate::markdown::compose::frontmatter_interpolation::FrontmatterSeedState;
use crate::markdown::compose::preflight::collect::resolve_executable;
use crate::markdown::compose::shell_expansion::policy::normalize_command;
use crate::markdown::compose::shell_expansion::store::resolve_policy_paths;
use crate::markdown::compose::shell_expansion::tokenize::{parse_pipeline, tokenize};
use crate::markdown::compose::shell_expansion::types::{
    ShellExpansionError, ShellExpansionRuntime, ShellPipeline,
};
use crate::markdown::compose::ComposeContext;

/// Checks an authored whole-value `$( … )` without resolving or running it.
///
/// Applies the same grammar as a top-level frontmatter value: the closing
/// parenthesis, the suffixes, at least one real command in an executed
/// position, no interpolated executable, and no chaining or piping the parser
/// rejects. `{{ … }}` spans are checked as opaque arguments.
///
/// ## Errors
///
/// A key-tagged [`ShellExpansionError::ParseDirective`] naming the problem, or
/// one saying the value is not a whole-value shell expression at all.
pub fn check_frontmatter_shell_value(key: &str, authored: &str) -> Result<(), ShellExpansionError> {
    let ctx = value_context(key, authored);
    let masked = mask_interpolations(authored);
    match parse_shell_value(&masked, key, Some(authored), &ctx)? {
        Some(_) => Ok(()),
        None => Err(not_a_shell_value(key, &ctx)),
    }
}

/// A whole-value `$( … )` whose command bytes were resolved once and are now
/// fixed.
///
/// Every pipeline it can run — one, or both branches of a ternary — was
/// interpolated at [`Self::resolve`] and passed the same shape checks as a
/// top-level frontmatter value. [`Self::commands`] lists exactly what
/// [`execute_resolved_shell_values`] may run, so approving that list approves
/// the value. The suffix is never part of those bytes.
#[derive(Debug, Clone)]
pub struct ResolvedShellValue {
    key: String,
    authored: String,
    resolved: String,
    directive: FrontmatterShellDirective,
    then_pipeline: Option<ShellPipeline>,
    else_pipeline: Option<ShellPipeline>,
}

impl PartialEq for ResolvedShellValue {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.authored == other.authored
            && self.resolved == other.resolved
            && self.then_pipeline == other.then_pipeline
            && self.else_pipeline == other.else_pipeline
    }
}

impl ResolvedShellValue {
    /// Fixes an authored value's command bytes.
    ///
    /// `resolved` is `authored` with its `{{ … }}` spans already interpolated
    /// by the caller; it supplies a plain command's bytes. A ternary's command
    /// branches are interpolated here from their authored text against
    /// `state`, so interpolation cannot move a branch boundary. Structure is
    /// always taken from `authored`, so interpolation also cannot change an
    /// executable or add a chained command.
    ///
    /// ## Errors
    ///
    /// Any parse, shape, or interpolation error the value raises.
    pub fn resolve(
        key: &str,
        authored: &str,
        resolved: &str,
        state: HashMap<String, Value>,
        context: &ComposeContext,
        resolution: ResolutionContext,
    ) -> Result<Self, ShellExpansionError> {
        let ctx = value_context(key, authored);
        let directive = parse_shell_value(resolved, key, Some(authored), &ctx)?
            .ok_or_else(|| not_a_shell_value(key, &ctx))?;
        let (mut then_pipeline, mut else_pipeline) = (None, None);
        if let FrontmatterShellAst::Ternary {
            then_branch,
            else_branch,
            ..
        } = &directive.ast
        {
            let seed = FrontmatterSeedState::new(state, context.clone())
                .with_current_authority(resolution.current.clone())
                .with_resolution_context(Some(resolution));
            then_pipeline = branch_pipeline(then_branch, BranchPosition::Then, key, &seed, &ctx)?;
            else_pipeline = branch_pipeline(else_branch, BranchPosition::Else, key, &seed, &ctx)?;
        }
        Ok(Self {
            key: key.to_string(),
            authored: authored.trim().to_string(),
            resolved: resolved.trim().to_string(),
            directive,
            then_pipeline,
            else_pipeline,
        })
    }

    /// The destination key the value was resolved for.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The value as authored.
    pub fn authored(&self) -> &str {
        &self.authored
    }

    /// The result suffix, when the value is the command's outcome rather than
    /// its stdout.
    pub fn result_kind(&self) -> Option<ShellResultKind> {
        self.directive.result
    }

    /// Every command the value can run, normalized the way shell policy and
    /// pre-approval compare them: one entry per chained command of every
    /// reachable pipeline, both ternary branches included.
    pub fn commands(&self) -> Vec<String> {
        let mut commands = Vec::new();
        for pipeline in self.pipelines() {
            for action in &pipeline.actions {
                let (executable, args) =
                    resolve_executable(&action.command.executable, &action.command.args);
                let normalized = normalize_command(&executable, &args);
                if !commands.contains(&normalized) {
                    commands.push(normalized);
                }
            }
        }
        commands
    }

    fn pipelines(&self) -> Vec<&ShellPipeline> {
        match &self.directive.ast {
            FrontmatterShellAst::Pipeline(pipeline) => vec![pipeline],
            FrontmatterShellAst::Ternary { .. } => self
                .then_pipeline
                .iter()
                .chain(self.else_pipeline.iter())
                .collect(),
        }
    }
}

/// Executes resolved values against `state` and returns each value's result,
/// in order.
///
/// All values read the same `state`; none sees another's result. The values
/// share one command cache created for this call, so a command two of them
/// name runs once, and nothing is reused from an earlier call. Commands run
/// under the request's shell settings and must be in its pre-approved set when it
/// has one; without one, shell policy and the approval handler decide as they
/// do for any frontmatter value.
///
/// ## Errors
///
/// The first value's error in order: a condition or value branch that fails to
/// evaluate, a command that is missing, blacklisted, denied, or not
/// pre-approved, a timeout that is not allowed, a command ended by a signal,
/// or, for a value without a result suffix, a non-zero exit.
pub fn execute_resolved_shell_values(
    values: &[&ResolvedShellValue],
    state: HashMap<String, Value>,
    request: &crate::markdown::compose::ComposeRequest,
) -> Result<Vec<(String, Value)>, ShellExpansionError> {
    let options = &request.root_options();
    let shell_opts = options.shell_options();
    let policy_paths = resolve_policy_paths(&shell_opts, &options.source)?;
    let mut shell = ShellExpansionRuntime::new();
    shell.ensure_loaded(&policy_paths)?;
    let resolution = options.frontmatter_resolution_context();
    let seed = FrontmatterSeedState::new(state, options.context().clone())
        .with_current_authority(resolution.current.clone())
        .with_resolution_context(Some(resolution));

    let mut pending = Vec::with_capacity(values.len());
    for value in values {
        let ctx = value_context(&value.key, &value.authored);
        let directive = &value.directive;
        let item = match &directive.ast {
            FrontmatterShellAst::Pipeline(pipeline) => Pending::Execute(Box::new(
                prepare_branch_pipeline(
                    pipeline.clone(),
                    directive.raw_command.clone(),
                    directive,
                    options,
                    &policy_paths,
                    &mut shell,
                    &ctx,
                )?,
            )),
            FrontmatterShellAst::Ternary {
                condition_source,
                then_branch,
                else_branch,
            } => {
                let pick_then = evaluate_ternary_condition(
                    condition_source,
                    &seed,
                    &value.key,
                    &ctx,
                    &mut Vec::new(),
                )?;
                let (branch, pipeline, position) = if pick_then {
                    (then_branch, &value.then_pipeline, BranchPosition::Then)
                } else {
                    (else_branch, &value.else_pipeline, BranchPosition::Else)
                };
                match (branch, pipeline) {
                    (Branch::Empty, _) => Pending::Value(String::new()),
                    (Branch::Value { source }, _) => Pending::Value(evaluate_value_branch(
                        source,
                        &seed,
                        position,
                        &value.key,
                        &ctx,
                        &mut Vec::new(),
                    )?),
                    (Branch::Pipeline { .. }, Some(pipeline)) => {
                        Pending::Execute(Box::new(prepare_branch_pipeline(
                            pipeline.clone(),
                            pipeline.display_string(),
                            directive,
                            options,
                            &policy_paths,
                            &mut shell,
                            &ctx,
                        )?))
                    }
                    (Branch::Pipeline { .. }, None) => {
                        unreachable!("resolve fixes every command branch's pipeline")
                    }
                }
            }
        };
        pending.push((value.key.clone(), directive.result, item));
    }

    let shell = &shell;
    pending
        .into_par_iter()
        .map(|(key, result, item)| {
            expand_pending(item, result, options, shell).map(|(value, _warnings)| (key, value))
        })
        .collect::<Vec<_>>()
        .into_iter()
        .collect()
}

/// Interpolates one command branch of a ternary from its authored text and
/// checks that interpolation kept its shape.
fn branch_pipeline(
    branch: &Branch,
    position: BranchPosition,
    key: &str,
    seed: &FrontmatterSeedState,
    ctx: &SourceContext,
) -> Result<Option<ShellPipeline>, ShellExpansionError> {
    let Branch::Pipeline { original_text } = branch else {
        return Ok(None);
    };
    let original = parse_static_pipeline_shape(original_text, ctx)
        .map_err(|error| remap_branch_parse_error(error, position, key, ctx))?;
    let resolved = interpolate_branch_text(original_text, seed, position, key, ctx, &mut Vec::new())?;
    let tokens =
        tokenize(&resolved, ctx).map_err(|error| remap_branch_parse_error(error, position, key, ctx))?;
    let pipeline = parse_pipeline(&tokens, ctx)
        .map_err(|error| remap_branch_parse_error(error, position, key, ctx))?;
    validate_pipeline_shape_preserved(&original, &pipeline, Some(position), key, ctx)?;
    Ok(Some(pipeline))
}

fn value_context(key: &str, authored: &str) -> SourceContext {
    SourceContext::new(
        PathBuf::from(key),
        PathBuf::from(key),
        authored.to_string(),
    )
}

fn not_a_shell_value(key: &str, ctx: &SourceContext) -> ShellExpansionError {
    frontmatter_parse_error(
        key,
        ctx,
        "Value is not a whole-value `$( … )` shell expression",
    )
}
