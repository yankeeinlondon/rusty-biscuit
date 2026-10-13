//! Re-checking preflight-approved shell text under a later document's
//! environment.
//!
//! Sequence preflight resolves every task `shell:`, `setup:`, and `teardown:`
//! command once, under the launch environment, and approves that exact text.
//! Each task and group leg later captures its own environment. Until shell
//! template approval exists, an approval covers only the text it approved, so
//! a task re-resolves each approved command under its own map before anything
//! runs: identical text runs as approved, and changed text is refused
//! ([`CompositionError::SequenceShellApprovalChanged`]). A stale approval never
//! authorizes new text.

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

    /// Refuse the command when `environment` resolves it to text other than
    /// the approved text.
    ///
    /// ## Errors
    ///
    /// [`CompositionError::SequenceShellApprovalChanged`] when the text
    /// differs, or the resolution failure when the command no longer
    /// resolves under `environment`.
    pub fn verify(&self, environment: &DocumentEnvironment) -> Result<(), CompositionError> {
        let basis = &self.basis;
        let mut state_context = basis.state_context.clone();
        *state_context.env_mut() = environment.vars().clone();
        let mut runtime_context = basis.runtime_context.clone();
        *runtime_context.env_mut() = environment.vars().clone();
        let file_resolution_context = basis
            .file_resolution_context
            .clone()
            .with_env(environment.vars().clone());
        let state = early_binding_state(&basis.frontmatter, state_context)?;
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
            ),
        })
    }
}

/// Builds a [`PreflightApproval`] for one resolved command.
pub(super) struct ApprovalRecord<'a> {
    pub(super) stage: TaskStage,
    pub(super) authored: &'a str,
    pub(super) approved: &'a str,
    pub(super) origin: &'a Path,
    pub(super) label: &'a str,
    pub(super) frontmatter: &'a Arc<HashMap<String, Value>>,
    pub(super) state: &'a EffectiveState,
    pub(super) runtime_context: &'a ComposeContext,
    pub(super) file_resolution_context: &'a biscuit_file::FileResolutionContext,
}

impl ApprovalRecord<'_> {
    pub(super) fn into_approval(self) -> PreflightApproval {
        PreflightApproval {
            stage: self.stage,
            approved: self.approved.to_string(),
            basis: Arc::new(ApprovalBasis {
                authored: self.authored.to_string(),
                origin: self.origin.to_path_buf(),
                label: self.label.to_string(),
                frontmatter: Arc::clone(self.frontmatter),
                state_context: self.state.context().clone(),
                runtime_context: self.runtime_context.clone(),
                file_resolution_context: self.file_resolution_context.clone(),
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
    let resolution_ctx = super::super::super::document_expression_resolution_context(
        origin,
        Some(runtime_context),
        file_resolution_context,
        None,
    )?;
    let (view, globals) =
        runtime_bindings(LifecycleScope::SequenceShellApproval, LifecycleValues::default());
    let composed = darkmatter::markdown::compose::subtree::SubtreeCompose::new(
        &Value::String(raw.to_string()),
        state,
    )
    .with_resolution_context(resolution_ctx)
    .with_globals(globals)
    .with_binding_view(view)
    .compose()
    .map_err(|e| CompositionError::SequenceShellResolution {
        command: raw.to_string(),
        task: label.to_string(),
        message: e.to_string(),
        source: Some(Box::new(e)),
    })?;
    Ok(match composed {
        Value::String(s) => s,
        other => other.to_string(),
    })
}

/// Name the variables the command mentions whose values differ between the
/// approval's map and the document's.
fn changed_variables(
    authored: &str,
    approved: &HashMap<String, String>,
    document: &HashMap<String, String>,
) -> String {
    let mut changed: Vec<&str> = approved
        .keys()
        .chain(document.keys())
        .filter(|name| approved.get(*name) != document.get(*name))
        .map(String::as_str)
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
