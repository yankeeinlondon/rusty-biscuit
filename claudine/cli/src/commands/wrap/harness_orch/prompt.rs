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
        lifecycle: Some(prepared.lifecycle.clone()),
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

/// Read the document at `state.source_path` and merge the caller overlay into
/// its authored frontmatter, yielding the source the canonical service prepares.
///
/// Every re-entry (`retry`/`resume`/`proxy`) reads fresh from disk rather than
/// reusing the first attempt's prepared prompt, so an `initialize`-time or
/// loop-time mutation of the document is visible.
fn load_overlaid_source(
    state: &HarnessPromptState,
) -> Result<claudine::composition::ResolvedCompositionSource> {
    let source_text = fs::read_to_string(&state.source_path)
        .map_err(|e| eyre!("failed to read '{}': {e}", state.source_path.display()))?;
    let mut markdown: darkmatter::markdown::Markdown = source_text.clone().into();
    super::super::overlay::merge_frontmatter_overlay(
        markdown.frontmatter_mut().as_map_mut(),
        &state.overlay,
    );
    Ok(claudine::composition::ResolvedCompositionSource {
        original_ref: state.original_ref.clone(),
        resolved_path: state.source_path.clone(),
        original_text: source_text,
        markdown,
    })
}

/// Assemble this document's canonical [`PrepareOptions`] from the invocation's
/// input layers plus the target-specific workspace.
///
/// This is the single input-layer assembly point the harness re-entry uses; the
/// composition commands' first preparation assembles the same shape in
/// `compose/prep.rs`.
fn harness_prepare_options(
    state: &HarnessPromptState,
    child_cwd: &Path,
) -> claudine::composition::PrepareOptions {
    state
        .input_layers
        .apply_to(claudine::composition::PrepareOptions {
            shell_working_directory: Some(child_cwd.to_path_buf()),
            ..claudine::composition::PrepareOptions::default()
        })
}

/// Run the proxied target document's own pre-flight shell audit and fold any
/// newly-approved commands into the invocation's approved set.
///
/// Only [`HarnessPromptMode::Compose`] re-runs a body compose that expands
/// shell; `Inline` runs its own `prepare_inline` audit and `Passthrough` has no
/// compose-time shell, so both are left untouched.
///
/// ## Errors
///
/// Propagates a shell-audit denial as a [`CompositionError`] so the caller
/// routes it through the standard `blocked`/`finalize` path.
///
/// [`CompositionError`]: claudine::composition::CompositionError
pub(crate) fn preflight_proxy_target(
    state: &mut HarnessPromptState,
    approval_options: &claudine::harness::ShellApprovalOptions,
    child_cwd: &Path,
) -> Result<(), Box<claudine::composition::CompositionError>> {
    if state.mode != HarnessPromptMode::Compose {
        return Ok(());
    }
    let source = load_overlaid_source(state).map_err(|e| {
        Box::new(claudine::composition::CompositionError::PreFlightFailed(format!(
            "proxy target pre-flight: {e}"
        )))
    })?;
    let options = harness_prepare_options(state, child_cwd);
    let approved =
        claudine::composition::preflight_document_shell(&source, &options, approval_options)
            .map_err(Box::new)?;
    state.input_layers.add_approved_commands(approved);
    Ok(())
}

pub(crate) fn materialize_harness_prompt(
    state: &HarnessPromptState,
    _repo_root: Option<&Path>,
    child_cwd: &Path,
) -> Result<MaterializedHarnessPrompt> {
    let source = load_overlaid_source(state)?;
    let options = harness_prepare_options(state, child_cwd);

    let prompt_source = match state.mode {
        // The prompt came from argv or stdin; the document is a provider memory
        // file whose body is context, not the request.
        HarnessPromptMode::Passthrough => claudine::composition::PromptSource::Supplied(
            state.base_prompt.clone().ok_or_else(|| {
                eyre!(
                    "missing passthrough prompt seed for '{}'",
                    state.source_path.display()
                )
            })?,
        ),
        HarnessPromptMode::Compose | HarnessPromptMode::Inline => {
            claudine::composition::PromptSource::ComposedBody
        }
    };
    let mode = match state.mode {
        HarnessPromptMode::Inline => {
            claudine::composition::validate_file_permissions(&state.source_path)
                .map_err(|e| eyre!("frontmatter-prompt: {e}"))?;
            claudine::composition::CompositionMode::InlineFrontmatterPrompt
        }
        HarnessPromptMode::Compose | HarnessPromptMode::Passthrough => {
            claudine::composition::CompositionMode::ChainedDocument
        }
    };

    let prepared = claudine::composition::prepare_document(claudine::composition::DocumentPreparation {
        entry: state.entry,
        mode,
        source: &source,
        prompt_source,
        options,
    })?;

    let inline_closure_plan = match prepared.closure {
        claudine::composition::CompositionClosurePlan::Inline(plan) => Some(plan),
        claudine::composition::CompositionClosurePlan::Direct => None,
    };
    let mut prompt = prepared.prompt;
    let frontmatter = prepared.effective_frontmatter;
    let lifecycle = prepared.lifecycle;
    let env_overrides: Vec<(String, String)> = Vec::new();

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
        lifecycle: Some(lifecycle),
        live_frontmatter,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use claudine::composition::{CallerInputLayers, DocumentEntryReason};
    use std::collections::BTreeMap;

    fn compose_state(source_path: &Path, input_layers: CallerInputLayers) -> HarnessPromptState {
        HarnessPromptState {
            mode: HarnessPromptMode::Compose,
            source_path: source_path.to_path_buf(),
            original_ref: source_path.display().to_string(),
            base_prompt: None,
            overlay: indexmap::IndexMap::new(),
            prompt_tail: Vec::new(),
            next_prompt_override: None,
            next_resume_session_id: None,
            input_layers,
            entry: DocumentEntryReason::ProxyTarget,
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
            CallerInputLayers {
                env_overrides: env,
                ..CallerInputLayers::default()
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
            CallerInputLayers {
                set_overrides: Some(serde_json::json!({ "spec": "features/x/spec.md" })),
                ..CallerInputLayers::default()
            },
        );

        let approval_options = claudine::harness::ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: None,
            ..Default::default()
        };

        // Before hand-off pre-flight, the carried set has no approval for the
        // target's own frontmatter shell command.
        assert!(state.input_layers.pre_approved_commands.is_none());

        preflight_proxy_target(&mut state, &approval_options, dir.path())
            .expect("whitelisted proxy-target command must pre-flight cleanly");

        let approved = state
            .input_layers
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
