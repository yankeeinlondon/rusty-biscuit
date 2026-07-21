// `CompositionError` is intentionally large (it carries frontmatter excerpts);
// the whole composition/wrap execution path returns it and opts out of the
// `result_large_err` lint the same way (see `wrap/composition/target.rs`,
// `commands/compose/`, `commands/sequence.rs`).
#![allow(clippy::result_large_err)]

use claudine::provider::Provider;
use color_eyre::eyre::{Result, eyre};
use std::fs;
use std::path::{Path, PathBuf};

use super::{HarnessPromptMode, HarnessPromptState, MaterializedHarnessPrompt};

pub(crate) fn materialized_harness_prompt_from_prepared(
    prepared: &claudine::composition::PreparedComposition,
    runtime_state: std::sync::Arc<claudine::composition::RuntimeState>,
) -> MaterializedHarnessPrompt {
    let inline_closure_plan = match &prepared.closure {
        claudine::composition::CompositionClosurePlan::Inline(plan) => Some(plan.clone()),
        claudine::composition::CompositionClosurePlan::Direct => None,
    };

    let live_frontmatter =
        MaterializedHarnessPrompt::live_cell_from(&prepared.effective_frontmatter);
    MaterializedHarnessPrompt {
        frontmatter: prepared.effective_frontmatter.clone(),
        prompt: prepared.prompt.clone(),
        env_overrides: Vec::new(),
        inline_closure_plan,
        live_frontmatter,
        runtime_state,
    }
}

pub(crate) fn materialize_passthrough_harness_seed(
    source_path: &Path,
    prompt: String,
    shell_cwd: Option<&Path>,
    runtime_state: std::sync::Arc<claudine::composition::RuntimeState>,
) -> Result<MaterializedHarnessPrompt> {
    super::super::overlay::materialize_passthrough_harness_seed(
        source_path,
        prompt,
        shell_cwd,
        runtime_state,
    )
}

pub(crate) fn find_wrapper_harness_source(
    provider: Provider,
    repo_root: Option<&Path>,
    cwd: &Path,
) -> Option<PathBuf> {
    let info = claudine::provider::provider_info(provider);
    let search_root = repo_root.unwrap_or(cwd);

    info.memory_files
        .iter()
        .map(|template| template.raw())
        .filter(|path| !path.starts_with('~'))
        .map(PathBuf::from)
        .find_map(|relative| {
            let candidate = search_root.join(relative);
            candidate.is_file().then_some(candidate)
        })
}

/// Re-apply the caller's compose inputs onto a re-materialization's
/// [`ComposeOptions`], mirroring [`prepare_direct`]/[`prepare_inline`].
///
/// Without this a `retry`/`resume`/`proxy` re-composition drops the caller's
/// `--set` params, launch-area file-ref anchor, and pre-approved shell
/// commands, so a `$schema`-bearing target validates against inputs it was
/// never handed.
///
/// [`prepare_direct`]: claudine::composition::prepare_direct
/// [`prepare_inline`]: claudine::composition::prepare_inline
fn apply_rematerialize_inputs(
    mut options: darkmatter::markdown::compose::ComposeOptions,
    inputs: &claudine::composition::RematerializeInputs,
) -> darkmatter::markdown::compose::ComposeOptions {
    if let Some(overrides) = inputs.set_overrides.clone() {
        options = options.with_set_overrides(overrides);
    }
    if let Some(approved) = inputs.pre_approved_commands.clone() {
        options = options.with_pre_approved_commands(approved);
    }
    if let Some(fallback) = inputs.file_ref_fallback_dir.clone() {
        options = options.with_file_ref_fallback_dir(fallback);
    }
    if let Some(context) = inputs.file_resolution_context.clone() {
        options = options.with_file_resolution_context(context);
    }
    options
}

/// Build the base [`ComposeOptions`] for a re-materialization, seeding its
/// [`ComposeContext`][darkmatter::markdown::compose::ComposeContext] with the
/// caller's env overrides (`AGENT`/`MODEL`/`YOLO`).
///
/// The env is what backs `ctx.agent`/`ctx.model`; without it a
/// `retry`/`resume`/`proxy` re-composition captures a fresh env-less context and
/// those values collapse to `unknown`/`default`.
fn rematerialize_compose_options(
    inputs: &claudine::composition::RematerializeInputs,
) -> darkmatter::markdown::compose::ComposeOptions {
    let mut ctx = darkmatter::markdown::compose::ComposeContext::capture();
    for (key, value) in &inputs.env_overrides {
        ctx.env_mut().insert(key.clone(), value.clone());
    }
    darkmatter::markdown::compose::ComposeOptions::new_with_context(ctx)
}

/// Run the proxied target document's own pre-flight shell audit and fold any
/// newly-approved commands into `rematerialize.pre_approved_commands`.
///
/// The lifecycle spec treats a `proxy` hand-off as "a fresh prompt run,
/// including pre-flight": the target's frontmatter `$(...)` / template `::shell`
/// commands are the target's own, and the proxying document's pre-flight never
/// saw them. Without re-auditing the target here, the subsequent re-materialize
/// compose expands those commands against the (proxying document's) pre-approved
/// set and fails with `NotPreApproved` — even for whitelisted commands the audit
/// would have auto-approved.
///
/// Only [`HarnessPromptMode::Compose`] re-runs a body compose that expands
/// shell; `Inline` runs its own `prepare_inline` audit and `Passthrough` has no
/// compose-time shell, so both are left untouched.
///
/// ## Errors
///
/// Propagates a shell-audit denial (blacklisted command, denied approval, or —
/// after the approval handler is frozen — an un-whitelisted, un-cached command)
/// as a [`CompositionError`] so the caller routes it through the standard
/// `blocked`/`finalize` path.
pub(crate) fn preflight_proxy_target(
    state: &mut HarnessPromptState,
    approval_options: &claudine::harness::ShellApprovalOptions,
    child_cwd: &Path,
) -> Result<(), Box<claudine::composition::CompositionError>> {
    if state.mode != HarnessPromptMode::Compose {
        return Ok(());
    }
    let source_text = fs::read_to_string(&state.source_path).map_err(|e| {
        Box::new(claudine::composition::CompositionError::MarkdownLoad {
            path: state.source_path.clone(),
            source: claudine::composition::MarkdownLoadCause::Read(e),
        })
    })?;
    let markdown: darkmatter::markdown::Markdown = source_text.into();

    // Mirror the primary compose pre-flight (`compose/prep.rs`): defer the
    // lifecycle event keys, anchor `file`-typed reads on the launch area, and
    // apply the caller's `--set` / env so the discovered commands match exactly
    // what the re-materialize compose will expand.
    let mut options = claudine::composition::bind_agent_workspace(
        rematerialize_compose_options(&state.rematerialize),
        &state.source_path,
        Some(child_cwd),
    )
    .with_exclude_keys(
        claudine::composition::LIFECYCLE_EVENT_KEYS
            .iter()
            .copied(),
    );
    if let Some(fallback) = state.rematerialize.file_ref_fallback_dir.clone() {
        options = options.with_file_ref_fallback_dir(fallback);
    }
    if let Some(context) = state.rematerialize.file_resolution_context.clone() {
        options = options.with_file_resolution_context(context);
    }
    if let Some(overrides) = state.rematerialize.set_overrides.clone() {
        options = options.with_set_overrides(overrides);
    }

    let result = match claudine::composition::resolve_shell_approvals(
        Some(&markdown),
        Some(&options),
        approval_options,
        None,
        None,
    ) {
        Ok(result) => result,
        // A discovery-walk failure (bad `::file` transclusion, unparsable graph)
        // is not a shell rejection — there are no commands to approve, and the
        // re-materialize compose below will surface the authoritative typed error
        // (e.g. a styled `TransclusionError` block). Defer to it rather than
        // preempting with a coarser preflight error.
        Err(claudine::composition::CompositionError::PreFlightDiscoveryFailed(_)) => {
            return Ok(());
        }
        Err(e) => return Err(Box::new(e)),
    };

    if !result.approved_commands.is_empty() {
        state
            .rematerialize
            .pre_approved_commands
            .get_or_insert_with(std::collections::HashSet::new)
            .extend(result.approved_commands);
    }
    Ok(())
}

pub(crate) fn materialize_harness_prompt(
    state: &HarnessPromptState,
    _repo_root: Option<&Path>,
    child_cwd: &Path,
) -> Result<MaterializedHarnessPrompt> {
    let source_text = fs::read_to_string(&state.source_path).map_err(|e| {
        claudine::composition::CompositionError::MarkdownLoad {
            path: state.source_path.clone(),
            source: claudine::composition::MarkdownLoadCause::Read(e),
        }
    })?;
    let mut effective_markdown: darkmatter::markdown::Markdown = source_text.clone().into();
    super::super::overlay::merge_frontmatter_overlay(
        effective_markdown.frontmatter_mut().as_map_mut(),
        &state.overlay,
    );

    // Re-materialization re-reads the document from disk, so the accumulated
    // runtime layer has to be re-applied on top of the caller's setters or a
    // loop iteration would silently roll back every `set` and lose `outputs`.
    let rematerialize = claudine::composition::RematerializeInputs {
        set_overrides: Some(claudine::composition::layered_set_overrides(
            state.rematerialize.set_overrides.as_ref(),
            Some(&state.runtime_state.snapshot()),
            None,
        )),
        ..state.rematerialize.clone()
    };

    let (mut prompt, frontmatter, env_overrides, inline_closure_plan) = match state.mode {
        HarnessPromptMode::Passthrough => {
            let options = apply_rematerialize_inputs(
                claudine::composition::bind_agent_workspace(
                    rematerialize_compose_options(&rematerialize),
                    &state.source_path,
                    Some(child_cwd),
                )
                .with_exclude_keys(
                    claudine::composition::LIFECYCLE_EVENT_KEYS
                        .iter()
                        .copied(),
                ),
                &rematerialize,
            );
            let (composed, _report) = effective_markdown.compose_with(options)?;
            let prompt = state.base_prompt.clone().ok_or_else(|| {
                eyre!(
                    "missing passthrough prompt seed for '{}'",
                    state.source_path.display()
                )
            })?;
            (
                prompt,
                super::super::overlay::frontmatter_map_to_value(composed.frontmatter()),
                Vec::new(),
                None,
            )
        }
        HarnessPromptMode::Compose => {
            // Defer the seven lifecycle event subtrees so their authored `{{ }}`
            // spans survive raw in the materialized frontmatter for event-time
            // interpolation — mirroring the prep-time seed (`prepare.rs`). This
            // is the re-materialization path (retries and proxy hand-offs);
            // without the deferral a proxy target's lifecycle `{{ err.* }}`
            // spans resolve here, before the run, and bake to empty.
            let options = apply_rematerialize_inputs(
                claudine::composition::bind_agent_workspace(
                    rematerialize_compose_options(&rematerialize),
                    &state.source_path,
                    Some(child_cwd),
                )
                .with_exclude_keys(
                    claudine::composition::LIFECYCLE_EVENT_KEYS
                        .iter()
                        .copied(),
                )
                // Preserve authored line breaks in the delivered body, mirroring
                // `prepare_direct`. Without this the re-materialized body (a proxy
                // hand-off / retry) is stripped of incidental single newlines, so
                // an author's line-structured prompt — e.g. a block-quoted list —
                // collapses into one paragraph and the agent prompt mis-renders.
                .with_incidental_newline_mode(
                    darkmatter::markdown::cleanup::IncidentalNewlineMode::Preserve,
                ),
                &rematerialize,
            );
            let (composed, _report) = effective_markdown.compose_with(options)?;
            let body = composed.content().to_string();

            let env_overrides = Vec::new();

            (
                body,
                super::super::overlay::frontmatter_map_to_value(composed.frontmatter()),
                env_overrides,
                None,
            )
        }
        HarnessPromptMode::Inline => {
            claudine::composition::validate_file_permissions(&state.source_path)?;
            let source = claudine::composition::ResolvedCompositionSource {
                original_ref: state.source_path.display().to_string(),
                resolved_path: state.source_path.clone(),
                original_text: source_text.clone(),
                markdown: effective_markdown.clone(),
            };
            let prepared = claudine::composition::prepare_inline(
                &source,
                claudine::composition::PrepareOptions {
                    set_overrides: rematerialize.set_overrides.clone(),
                    pre_approved_commands: rematerialize.pre_approved_commands.clone(),
                    file_ref_fallback_dir: rematerialize.file_ref_fallback_dir.clone(),
                    file_resolution_context: rematerialize.file_resolution_context.clone(),
                    // Carry the resolved `AGENT`/`MODEL` env so `ctx.agent`/
                    // `ctx.model` in the re-materialized body resolve to the run's
                    // provider instead of the `unknown`/`default` fallbacks.
                    env_overrides: rematerialize.env_overrides.clone(),
                    ..claudine::composition::PrepareOptions::default()
                },
            )?;
            (
                prepared.prompt,
                prepared.effective_frontmatter,
                Vec::new(),
                match prepared.closure {
                    claudine::composition::CompositionClosurePlan::Inline(plan) => Some(plan),
                    claudine::composition::CompositionClosurePlan::Direct => None,
                },
            )
        }
    };

    if let Some(ref override_prompt) = state.next_prompt_override {
        prompt = override_prompt.clone();
    } else {
        for tail in &state.prompt_tail {
            prompt.push_str("\n\n");
            prompt.push_str(tail);
        }
    }

    let live_frontmatter = MaterializedHarnessPrompt::live_cell_from(&frontmatter);
    Ok(MaterializedHarnessPrompt {
        frontmatter,
        prompt,
        env_overrides,
        inline_closure_plan,
        live_frontmatter,
        runtime_state: std::sync::Arc::clone(&state.runtime_state),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use claudine::composition::RematerializeInputs;
    use std::collections::BTreeMap;

    fn compose_state(source_path: &Path, rematerialize: RematerializeInputs) -> HarnessPromptState {
        HarnessPromptState {
            mode: HarnessPromptMode::Compose,
            source_path: source_path.to_path_buf(),
            original_ref: source_path.display().to_string(),
            base_prompt: None,
            overlay: indexmap::IndexMap::new(),
            prompt_tail: Vec::new(),
            next_prompt_override: None,
            next_resume_session_id: None,
            rematerialize,
            runtime_state: std::sync::Arc::new(claudine::composition::RuntimeState::new()),
            suppress_output_commit: false,
            last_final_output: None,
        }
    }

    /// Issue #2 regression: a proxy/retry re-materialization must resolve
    /// `ctx.agent`/`ctx.model` from the carried env overrides. Before the fix
    /// the re-composition captured a fresh env-less context and both collapsed
    /// to the `unknown`/`default` fallbacks.
    #[test]
    fn compose_rematerialize_resolves_ctx_agent_from_env() {
        let dir = tempfile::TempDir::new().unwrap();
        let target = dir.path().join("target.md");
        std::fs::write(&target, "---\ndescription: t\n---\n{{ ctx.agent }}/{{ ctx.model }}\n")
            .unwrap();

        let mut env = BTreeMap::new();
        env.insert("AGENT".to_string(), "codex".to_string());
        env.insert("MODEL".to_string(), "gpt-5".to_string());
        let state = compose_state(
            &target,
            RematerializeInputs {
                env_overrides: env,
                ..RematerializeInputs::default()
            },
        );

        let materialized = materialize_harness_prompt(&state, None, dir.path()).unwrap();
        assert_eq!(
            materialized.prompt.trim(),
            "codex/gpt-5",
            "ctx.agent/ctx.model must resolve from the carried env, not the fallbacks",
        );
    }

    /// Issue #1 regression: a proxy target's own frontmatter `$(...)` shell
    /// command must be discovered and approved at hand-off and folded into the
    /// carried pre-approved set, so the subsequent re-materialize compose does
    /// not reject a whitelisted command with `NotPreApproved`.
    #[test]
    fn proxy_target_preflight_approves_frontmatter_shell_and_rematerializes() {
        let dir = tempfile::TempDir::new().unwrap();
        // Whitelist `basename` so the audit auto-approves without a handler,
        // mirroring the real review prompt's reliance on the repo whitelist.
        std::fs::write(dir.path().join(".darkmatter-shell-whitelist"), "prefix basename\n")
            .unwrap();
        let target = dir.path().join("target.md");
        std::fs::write(
            &target,
            "---\nbase: \"$(basename '{{ spec }}')\"\n---\nreviewing {{ base }}\n",
        )
        .unwrap();

        let mut state = compose_state(
            &target,
            RematerializeInputs {
                set_overrides: Some(serde_json::json!({ "spec": "features/x/spec.md" })),
                ..RematerializeInputs::default()
            },
        );

        let approval_options = claudine::harness::ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: None,
            ..Default::default()
        };

        // Before hand-off pre-flight, the carried set has no approval for the
        // target's own frontmatter shell command.
        assert!(state.rematerialize.pre_approved_commands.is_none());

        preflight_proxy_target(&mut state, &approval_options, dir.path())
            .expect("whitelisted proxy-target command must pre-flight cleanly");

        let approved = state
            .rematerialize
            .pre_approved_commands
            .as_ref()
            .expect("pre-approved set must be populated after target pre-flight");
        assert!(
            approved.contains("basename features/x/spec.md"),
            "expected the resolved frontmatter shell command; got: {approved:?}",
        );

        // The re-materialize compose now expands the frontmatter command against
        // the augmented pre-approved set instead of failing NotPreApproved.
        let materialized = materialize_harness_prompt(&state, None, dir.path()).unwrap();
        assert_eq!(materialized.prompt.trim(), "reviewing spec.md");
    }
}
