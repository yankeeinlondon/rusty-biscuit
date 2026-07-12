use claudine::provider::Provider;
use color_eyre::eyre::{Result, eyre};
use std::fs;
use std::path::{Path, PathBuf};

use super::{HarnessPromptMode, HarnessPromptState, MaterializedHarnessPrompt};

pub(crate) fn materialized_harness_prompt_from_prepared(
    prepared: &claudine::composition::PreparedComposition,
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
    }
}

pub(crate) fn materialize_passthrough_harness_seed(
    source_path: &Path,
    prompt: String,
    shell_cwd: Option<&Path>,
) -> Result<MaterializedHarnessPrompt> {
    super::super::overlay::materialize_passthrough_harness_seed(source_path, prompt, shell_cwd)
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
    options
}

pub(crate) fn materialize_harness_prompt(
    state: &HarnessPromptState,
    _repo_root: Option<&Path>,
    child_cwd: &Path,
) -> Result<MaterializedHarnessPrompt> {
    let source_text = fs::read_to_string(&state.source_path)
        .map_err(|e| eyre!("failed to read '{}': {e}", state.source_path.display()))?;
    let mut effective_markdown: darkmatter::markdown::Markdown = source_text.clone().into();
    super::super::overlay::merge_frontmatter_overlay(
        effective_markdown.frontmatter_mut().as_map_mut(),
        &state.overlay,
    );

    let (mut prompt, frontmatter, env_overrides, inline_closure_plan) = match state.mode {
        HarnessPromptMode::Passthrough => {
            let options = apply_rematerialize_inputs(
                claudine::composition::bind_agent_workspace(
                    darkmatter::markdown::compose::ComposeOptions::new(),
                    &state.source_path,
                    Some(child_cwd),
                )
                .with_exclude_keys(
                    claudine::composition::LIFECYCLE_EVENT_KEYS
                        .iter()
                        .copied(),
                ),
                &state.rematerialize,
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
                    darkmatter::markdown::compose::ComposeOptions::new(),
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
                &state.rematerialize,
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
            claudine::composition::validate_file_permissions(&state.source_path)
                .map_err(|e| eyre!("frontmatter-prompt: {e}"))?;
            let source = claudine::composition::ResolvedCompositionSource {
                original_ref: state.source_path.display().to_string(),
                resolved_path: state.source_path.clone(),
                original_text: source_text.clone(),
                markdown: effective_markdown.clone(),
            };
            let prepared = claudine::composition::prepare_inline(
                &source,
                claudine::composition::PrepareOptions {
                    set_overrides: state.rematerialize.set_overrides.clone(),
                    pre_approved_commands: state.rematerialize.pre_approved_commands.clone(),
                    file_ref_fallback_dir: state.rematerialize.file_ref_fallback_dir.clone(),
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
    })
}
