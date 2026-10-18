//! Re-checking preflight-resolved shell text under a later document's
//! environment, and the one approval-scope resolver sequence preflight uses.
//!
//! Sequence preflight resolves each task `shell:`, `setup:`, and `teardown:`
//! command selected for resolved-byte execution once, under the launch
//! environment. Each task and group leg later captures its own environment, so
//! a task re-resolves each such command under its own map before anything
//! runs: identical text runs as resolved, and changed text is refused
//! ([`CompositionError::SequenceShellApprovalChanged`]). Resolved text never
//! authorizes new text. Template-mode commands have no record here, including
//! later-task commands whose values already exist at approval time.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::compose::{ComposeContext, EffectiveState, EffectiveStateBuilder};
use serde_json::Value;

use super::super::super::error::CompositionError;
use super::super::super::lifecycle::bindings::{LifecycleScope, LifecycleValues, runtime_bindings};
use super::super::task::TaskStage;
use crate::invocation_context::DocumentEnvironment;

/// One preflight-approved shell command that interpolates, with everything
/// preflight resolved it from.
#[derive(Clone)]
pub struct PreflightApproval {
    stage: TaskStage,
    approved: String,
    basis: Arc<ApprovalBasis>,
}

/// The inputs one approved command was resolved from. Re-resolution replaces
/// only the environment, so a refusal can come from nothing else.
struct ApprovalBasis {
    authored: String,
    origin: PathBuf,
    label: String,
    /// The step frontmatter the early-binding state was built from.
    frontmatter: Arc<HashMap<String, Value>>,
    /// The context the early-binding state carried.
    state_context: ComposeContext,
    /// The context the expression resolution read.
    runtime_context: ComposeContext,
    file_resolution_context: biscuit_file::FileResolutionContext,
}

impl std::fmt::Debug for PreflightApproval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreflightApproval")
            .field("stage", &self.stage)
            .field("authored", &self.basis.authored)
            .field("approved", &self.approved)
            .finish_non_exhaustive()
    }
}

impl PreflightApproval {
    /// Which of the task's stages runs the command.
    pub fn stage(&self) -> TaskStage {
        self.stage
    }

    /// The command as authored.
    pub fn authored(&self) -> &str {
        &self.basis.authored
    }

    /// The text sequence preflight approved.
    pub fn approved(&self) -> &str {
        &self.approved
    }

    /// Refuse the command when `environment` and the invocation's current
    /// `runtime_writes` resolve it to text other than the approved text.
    ///
    /// The writes are layered over the values preflight resolved from, so a
    /// command an earlier loop pass changed through a runtime `set` or a loop
    /// action is refused rather than run with its first pass's bytes.
    ///
    /// ## Errors
    ///
    /// [`CompositionError::SequenceShellApprovalChanged`] when the text
    /// differs, or the resolution failure when the command no longer
    /// resolves under `environment`.
    pub fn verify(
        &self,
        environment: &DocumentEnvironment,
        runtime_writes: &darkmatter::markdown::FrontmatterMap,
    ) -> Result<(), CompositionError> {
        let basis = &self.basis;
        let mut state_context = basis.state_context.clone();
        *state_context.env_mut() = environment.vars().clone();
        let mut runtime_context = basis.runtime_context.clone();
        *runtime_context.env_mut() = environment.vars().clone();
        let file_resolution_context = basis
            .file_resolution_context
            .clone()
            .with_env(environment.vars().clone());
        let frontmatter = if runtime_writes.is_empty() {
            Arc::clone(&basis.frontmatter)
        } else {
            let mut current = (*basis.frontmatter).clone();
            current.extend(runtime_writes.iter().map(|(key, value)| (key.clone(), value.clone())));
            Arc::new(current)
        };
        let state = early_binding_state(&frontmatter, state_context)?;
        let resolved = compose_shell_text(
            &basis.authored,
            &basis.origin,
            &basis.label,
            &state,
            &runtime_context,
            &file_resolution_context,
        )?;
        if resolved == self.approved {
            return Ok(());
        }
        Err(CompositionError::SequenceShellApprovalChanged {
            command: basis.authored.clone(),
            task: basis.label.clone(),
            approved: self.approved.clone(),
            resolved,
            changed: changed_variables(
                &basis.authored,
                basis.runtime_context.env(),
                environment.vars(),
                runtime_writes.iter()
                    .filter(|(key, value)| basis.frontmatter.get(*key) != Some(*value))
                    .map(|(key, _)| key.as_str()),
            ),
        })
    }
}

/// Everything one resolved command was resolved from, before its stage is
/// known.
pub(super) struct ApprovalRecordOwned {
    pub(super) authored: String,
    pub(super) approved: String,
    pub(super) origin: PathBuf,
    pub(super) label: String,
    pub(super) frontmatter: Arc<HashMap<String, Value>>,
    pub(super) state_context: ComposeContext,
    pub(super) runtime_context: ComposeContext,
    pub(super) file_resolution_context: biscuit_file::FileResolutionContext,
}

impl ApprovalRecordOwned {
    pub(super) fn into_approval(self, stage: TaskStage) -> PreflightApproval {
        PreflightApproval {
            stage,
            approved: self.approved,
            basis: Arc::new(ApprovalBasis {
                authored: self.authored,
                origin: self.origin,
                label: self.label,
                frontmatter: self.frontmatter,
                state_context: self.state_context,
                runtime_context: self.runtime_context,
                file_resolution_context: self.file_resolution_context,
            }),
        }
    }
}

/// The early-binding state a step's shell commands resolve against.
pub(super) fn early_binding_state(
    frontmatter: &HashMap<String, Value>,
    context: ComposeContext,
) -> Result<EffectiveState, CompositionError> {
    EffectiveStateBuilder::new()
        .with_frontmatter(frontmatter.clone())
        .with_context(context)
        .with_allow_ctx_override(true)
        .build()
        .map_err(|e| CompositionError::PreFlightStateBuildFailed { source: e })
}

/// Resolve one shell string's `{{ … }}` spans under the sequence approval
/// scope. Preflight and the per-task re-check share it, so the two can differ
/// only by their inputs.
pub(super) fn compose_shell_text(
    raw: &str,
    origin: &Path,
    label: &str,
    state: &EffectiveState,
    runtime_context: &ComposeContext,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> Result<String, CompositionError> {
    let composed = compose_approval_value(
        &Value::String(raw.to_string()),
        origin,
        state,
        runtime_context,
        file_resolution_context,
    )?
    .map_err(|e| shell_resolution_failure(raw, label, e))?;
    Ok(match composed {
        Value::String(s) => s,
        other => other.to_string(),
    })
}

/// The typed preflight failure for a shell string that did not resolve.
pub(super) fn shell_resolution_failure(
    raw: &str,
    label: &str,
    error: darkmatter::markdown::MarkdownError,
) -> CompositionError {
    CompositionError::SequenceShellResolution {
        command: raw.to_string(),
        task: label.to_string(),
        message: error.to_string(),
        source: Some(Box::new(error)),
    }
}

/// Resolve every `{{ … }}` span in `value` under the sequence approval scope,
/// against early-binding values only: a whole-value span keeps its type.
///
/// Every approval-time read sequence preflight makes goes through here — shell
/// text, each placeholder of a template, and a prompt task's `params` — so they
/// all see the same scope and values.
///
/// ## Errors
///
/// The outer error is a resolution context that could not be built; the inner
/// one is the composition failure, for the caller to name.
pub(super) fn compose_approval_value(
    value: &Value,
    origin: &Path,
    state: &EffectiveState,
    runtime_context: &ComposeContext,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> Result<Result<Value, darkmatter::markdown::MarkdownError>, CompositionError> {
    let resolution_ctx = super::super::super::document_expression_resolution_context(
        origin,
        Some(runtime_context),
        file_resolution_context,
        None,
    )?;
    let (view, globals) =
        runtime_bindings(LifecycleScope::SequenceShellApproval, LifecycleValues::default());
    Ok(darkmatter::markdown::compose::subtree::SubtreeCompose::new(value, state)
        .with_resolution_context(resolution_ctx)
        .with_globals(globals)
        .with_binding_view(view)
        .compose())
}

/// Name the environment variables and runtime values the command mentions
/// whose values differ from what preflight resolved it with.
fn changed_variables<'a>(
    authored: &str,
    approved: &'a HashMap<String, String>,
    document: &'a HashMap<String, String>,
    runtime: impl Iterator<Item = &'a str>,
) -> String {
    let mut changed: Vec<&str> = approved
        .keys()
        .chain(document.keys())
        .filter(|name| approved.get(*name) != document.get(*name))
        .map(String::as_str)
        .chain(runtime)
        .filter(|name| authored.contains(name))
        .collect();
    changed.sort_unstable();
    changed.dedup();
    if changed.is_empty() {
        "its environment changed".to_string()
    } else {
        format!("changed: {}", changed.join(", "))
    }
}
