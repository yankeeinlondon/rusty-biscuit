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
        // The same composed text the invocation pipeline lexes for its own
        // recorded launch facets, so a seeded first attempt rebuilds to the
        // facet set the invocation recorded.
        mcp_body_tags: MaterializedHarnessPrompt::lex_body_mcp_tags(&prepared.prompt),
        prompt: prepared.prompt.clone(),
        env_overrides: Vec::new(),
        selection_hints: prepared.selection_hints.clone(),
        inline_closure_plan,
        file_resolution_context: prepared.input_layers.file_resolution_context.clone(),
        compose_context: Some(prepared.compose_context.clone()),
        lifecycle: Some(prepared.lifecycle.clone()),
        live_frontmatter,
        runtime_state,
    }
}

pub(crate) fn materialize_passthrough_harness_seed(
    source_path: &Path,
    prompt: String,
    shell_cwd: Option<&Path>,
    runtime_state: std::sync::Arc<claudine::composition::RuntimeState>,
    invocation: &claudine::invocation_context::InvocationContext,
    source_context: &claudine::invocation_context::SourceContext,
    target_env_overrides: &[(String, String)],
) -> Result<MaterializedHarnessPrompt> {
    super::super::overlay::materialize_passthrough_harness_seed(
        source_path,
        prompt,
        shell_cwd,
        runtime_state,
        invocation,
        source_context,
        target_env_overrides,
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

/// Read the document at `state.source_path` and merge its overlay into that
/// document's authored frontmatter, yielding the source the canonical service
/// prepares.
///
/// The authored-layer half of overlay assembly. The bootstrap read before
/// target `initialize` and the stabilized reread after it see the same
/// immutable mapping. [`harness_prepare_options`] also carries non-null overlay
/// values through the caller-supplied override layer so file references retain
/// their launch-area resolution provenance.
///
/// Precedence, low to high: authored frontmatter < `proxy.with` < the caller's
/// `key=value`/`--set`. Null removal lands here because an override object
/// cannot express an absent key. The caller's `set_overrides` ride on the
/// compose options and are applied by Darkmatter on top, so a router cannot
/// silently replace a value the caller pinned explicitly.
///
/// Every re-entry (`retry`/`resume`/`proxy`) reads fresh from disk rather than
/// reusing the first attempt's prepared prompt, so an `initialize`-time or
/// loop-time mutation of the document is visible.
fn load_overlaid_source(
    state: &HarnessPromptState,
) -> Result<
    claudine::composition::ResolvedCompositionSource,
    claudine::composition::CompositionError,
> {
    let source_text = fs::read_to_string(&state.source_path).map_err(|source| {
        claudine::composition::CompositionError::MarkdownLoad {
            path: state.source_path.clone(),
            source: claudine::composition::MarkdownLoadCause::Read(source),
        }
    })?;
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
    state: &mut HarnessPromptState,
    source: &claudine::composition::ResolvedCompositionSource,
    child_cwd: &Path,
) -> claudine::composition::PrepareOptions {
    let mut input_layers = state.input_layers.clone();
    let mut caller_values = serde_json::Map::new();
    for (key, value) in &state.overlay {
        if !value.is_null() {
            caller_values.insert(key.clone(), value.clone());
        }
    }
    if let Some(serde_json::Value::Object(explicit)) = input_layers.set_overrides.as_ref() {
        caller_values.extend(explicit.clone());
    }
    let caller_values = serde_json::Value::Object(caller_values);
    input_layers.set_overrides = Some(claudine::composition::layered_set_overrides(
        Some(&caller_values),
        Some(&state.runtime_state.snapshot()),
        None,
    ));
    let compatibility_source_context = if state.source_context.is_none() {
        state.invocation_context.as_ref().map(|invocation| {
            invocation
                .derive_source(&source.resolved_path)
                .expect("resolved harness document always has a parent directory")
        })
    } else {
        None
    };
    let source_context = state
        .source_context
        .as_ref()
        .or(compatibility_source_context.as_ref());
    let requirements =
        darkmatter::markdown::compose::ContextRequirements::for_document(&source.markdown);
    let propagated = state.invocation_context.as_ref().map(|invocation| {
        match (
            state.epoch_context.as_mut(),
            state.epoch_context_requirements.as_mut(),
        ) {
            (Some(context), Some(captured)) => {
                invocation.extend_launch_context(context, captured, &requirements);
            }
            _ => {
                let mut context = invocation.capture_launch_context(&requirements);
                for (key, value) in &input_layers.env_overrides {
                    context.env_mut().insert(key.clone(), value.clone());
                }
                state.epoch_context = Some(context);
                state.epoch_context_requirements = Some(requirements.clone());
            }
        }
        state
            .epoch_context
            .as_ref()
            .expect("invocation-backed epoch context was initialized")
            .clone()
    });
    let mut options = input_layers.apply_to(claudine::composition::PrepareOptions {
        invocation_context: state.invocation_context.clone(),
        shell_working_directory: Some(child_cwd.to_path_buf()),
        ..claudine::composition::PrepareOptions::default()
    });
    if let Some(context) = propagated {
        options.prepared_context = Some(context);
    }
    if let Some(source_context) = source_context {
        options.file_resolution_context = Some(source_context.file_resolution_context().clone());
    }
    options
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
    let source = load_overlaid_source(state).map_err(Box::new)?;
    let options = harness_prepare_options(state, &source, child_cwd);
    let approved =
        claudine::composition::preflight_document_shell(&source, &options, approval_options)
            .map_err(Box::new)?;
    state.input_layers.add_approved_commands(approved);
    Ok(())
}

pub(crate) fn materialize_harness_prompt(
    state: &mut HarnessPromptState,
    _repo_root: Option<&Path>,
    child_cwd: &Path,
    // The resume follow-up recorded on the active document's provider-attempt
    // slice, when this attempt is a resume. It overrides the composed prompt for
    // exactly this attempt; a retry or a first attempt passes `None` and the
    // document's own `prompt_tail` is appended instead.
    resume_followup: Option<&str>,
    // Whether this read owns the document's schema verdict. The pre-`initialize`
    // bootstrap read defers it to the stabilized reread taken after
    // `initialize` has had its chance to add or repair the property (R4).
    schema: claudine::composition::SchemaStage,
) -> Result<MaterializedHarnessPrompt> {
    let source = load_overlaid_source(state)?;
    let options = harness_prepare_options(state, &source, child_cwd);

    let prompt_source = match state.mode {
        // The prompt came from argv or stdin; the document is a provider memory
        // file whose body is context, not the request.
        HarnessPromptMode::Passthrough => claudine::composition::PromptSource::Supplied(
            state.base_prompt.clone().ok_or_else(|| {
                eyre!(
                    "missing passthrough prompt seed for '{}'",
                    biscuit_file::to_portable_string(&state.source_path)
                )
            })?,
        ),
        HarnessPromptMode::Compose | HarnessPromptMode::Inline => {
            claudine::composition::PromptSource::ComposedBody
        }
    };
    let mode = match state.mode {
        HarnessPromptMode::Inline => {
            claudine::composition::validate_file_permissions(&state.source_path)?;
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
        schema,
        options,
    })?;

    let inline_closure_plan = match prepared.closure {
        claudine::composition::CompositionClosurePlan::Inline(plan) => Some(plan),
        claudine::composition::CompositionClosurePlan::Direct => None,
    };
    let file_resolution_context = prepared.input_layers.file_resolution_context.clone();
    let compose_context = prepared.compose_context;
    let mut prompt = prepared.prompt;
    let frontmatter = prepared.effective_frontmatter;
    let lifecycle = prepared.lifecycle;
    let selection_hints = prepared.selection_hints;
    let env_overrides: Vec<(String, String)> = Vec::new();

    for tail in &state.prompt_tail {
        prompt.push_str("\n\n");
        prompt.push_str(tail);
    }

    // R3/R5/R8 — the MCP tag set belongs to the *composed* document, so it is
    // taken here, while `prompt` still holds the composed body, and before the
    // resume substitution below replaces the provider input with the follow-up
    // message. Re-lexing later from either the substituted prompt or the raw
    // source on disk loses every tag that composition produced.
    //
    // Passthrough is the exception: it wraps a provider memory file whose body is
    // context rather than the request, and its invocation records an empty facet
    // set for exactly that reason (`wrapper_stages::passthrough_launch_intent`).
    let mcp_body_tags = match state.mode {
        HarnessPromptMode::Passthrough => Vec::new(),
        HarnessPromptMode::Compose | HarnessPromptMode::Inline => {
            MaterializedHarnessPrompt::lex_body_mcp_tags(&prompt)
        }
    };

    if let Some(override_prompt) = resume_followup {
        prompt = override_prompt.to_string();
    }

    let live_frontmatter = MaterializedHarnessPrompt::live_cell_from(&frontmatter);
    Ok(MaterializedHarnessPrompt {
        frontmatter,
        prompt,
        env_overrides,
        selection_hints,
        inline_closure_plan,
        file_resolution_context,
        compose_context: Some(compose_context),
        lifecycle: Some(lifecycle),
        live_frontmatter,
        runtime_state: std::sync::Arc::clone(&state.runtime_state),
        mcp_body_tags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use claudine::composition::{CallerInputLayers, DocumentEntryReason, SchemaStage};
    use std::collections::BTreeMap;

    fn compose_state(source_path: &Path, input_layers: CallerInputLayers) -> HarnessPromptState {
        HarnessPromptState {
            mode: HarnessPromptMode::Compose,
            source_path: source_path.to_path_buf(),
            original_ref: source_path.display().to_string(),
            base_prompt: None,
            overlay: indexmap::IndexMap::new(),
            prompt_tail: Vec::new(),
            input_layers,
            runtime_state: std::sync::Arc::new(claudine::composition::RuntimeState::new()),
            suppress_output_commit: false,
            last_final_output: None,
            entry: DocumentEntryReason::ProxyTarget,
            invocation_context: None,
            source_context: None,
            epoch_context: None,
            epoch_context_requirements: None,
        }
    }

    fn init_repo(root: &Path) {
        std::fs::create_dir_all(root).unwrap();
        let output = std::process::Command::new("git")
            .args(["init", "--quiet", "--initial-branch=main"])
            .current_dir(root)
            .output()
            .unwrap();
        assert!(output.status.success());
    }

    #[test]
    fn stabilized_reread_extends_one_launch_epoch_without_reanchoring_identity() {
        let fixture = tempfile::tempdir().unwrap();
        let launch_root = fixture.path().join("launch");
        let launch_area = launch_root.join("alpha/lib");
        init_repo(&launch_root);
        let beta_area = launch_root.join("beta/lib");
        std::fs::create_dir_all(launch_area.join("src")).unwrap();
        std::fs::create_dir_all(beta_area.join("src")).unwrap();
        std::fs::write(
            launch_root.join("Cargo.toml"),
            "[workspace]\nresolver = \"2\"\nmembers = [\"alpha/lib\", \"beta/lib\"]\n",
        )
        .unwrap();
        std::fs::write(
            launch_area.join("Cargo.toml"),
            "[package]\nname = \"alpha-lib\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::write(launch_area.join("src/lib.rs"), "").unwrap();
        std::fs::write(
            beta_area.join("Cargo.toml"),
            "[package]\nname = \"beta-lib\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::write(beta_area.join("src/lib.rs"), "").unwrap();

        let source_root = fixture.path().join("source");
        init_repo(&source_root);
        let target = source_root.join("target.md");
        std::fs::write(&target, "---\n---\n{{ ctx.area }}/{{ ctx.agent }}\n").unwrap();

        let invocation =
            claudine::invocation_context::InvocationContext::capture_at(&launch_area);
        let source_context = invocation.derive_source(&target).unwrap();
        let source = load_overlaid_source(&compose_state(
            &target,
            CallerInputLayers::default(),
        ))
        .unwrap();
        let requirements =
            darkmatter::markdown::compose::ContextRequirements::for_document(&source.markdown);
        let mut context = invocation.capture_launch_context(&requirements);
        context
            .env_mut()
            .insert("AGENT".to_string(), "codex".to_string());
        context
            .env_mut()
            .insert("MODEL".to_string(), "gpt-test".to_string());

        let mut env = BTreeMap::new();
        env.insert("AGENT".to_string(), "codex".to_string());
        env.insert("MODEL".to_string(), "gpt-test".to_string());
        let mut state = compose_state(
            &target,
            CallerInputLayers {
                env_overrides: env,
                ..CallerInputLayers::default()
            },
        );
        state.invocation_context = Some(invocation.clone());
        state.source_context = Some(source_context);
        state.epoch_context = Some(context);
        state.epoch_context_requirements = Some(requirements);

        let first = harness_prepare_options(&mut state, &source, &launch_root);
        let first_context = first.prepared_context.expect("prepared epoch context");
        let first_effective = first_context.as_object();
        assert_eq!(first_context.get("area").and_then(|v| v.as_str()), Some("alpha-lib"));
        assert_eq!(first_effective.get("agent").and_then(|v| v.as_str()), Some("codex"));
        assert_eq!(first_effective.get("model").and_then(|v| v.as_str()), Some("gpt-test"));
        assert_eq!(invocation.work_snapshot().launch_context_constructions, 1);
        assert_eq!(invocation.work_snapshot().launch_context_extensions, 0);

        std::fs::write(
            &target,
            "---\n---\n{{ ctx.area }}/{{ ctx.agent }}/{{ ctx.os }}\n",
        )
        .unwrap();
        let reread = load_overlaid_source(&state).unwrap();
        let second = harness_prepare_options(&mut state, &reread, &launch_root);
        let second_context = second.prepared_context.expect("extended epoch context");
        let effective = second_context.as_object();
        assert_eq!(second_context.get("area").and_then(|v| v.as_str()), Some("alpha-lib"));
        assert_eq!(effective.get("agent").and_then(|v| v.as_str()), Some("codex"));
        assert_eq!(effective.get("model").and_then(|v| v.as_str()), Some("gpt-test"));
        assert!(second_context.get("os").is_some());

        let work = invocation.work_snapshot();
        assert_eq!(work.launch_context_constructions, 1);
        assert_eq!(work.launch_context_extensions, 1);
        assert_eq!(work.ambient_fallbacks, 0);
    }

    #[test]
    fn proxy_retry_and_resume_start_fresh_target_adjusted_launch_epochs() {
        let fixture = tempfile::tempdir().unwrap();
        let launch_root = fixture.path().join("launch");
        init_repo(&launch_root);
        let source_root = fixture.path().join("source");
        init_repo(&source_root);
        let target = source_root.join("target.md");
        std::fs::write(
            &target,
            "---\n---\n{{ ctx.agent }}/{{ ctx.model }}/{{ env.AGENT }}/{{ env.MODEL }}/{{ ctx.repo_root }}\n",
        )
        .unwrap();

        let invocation =
            claudine::invocation_context::InvocationContext::capture_at(&launch_root);
        for entry in [
            DocumentEntryReason::ProxyTarget,
            DocumentEntryReason::Retry,
            DocumentEntryReason::Resume,
        ] {
            let mut state = compose_state(
                &target,
                CallerInputLayers {
                    env_overrides: BTreeMap::from([
                        ("AGENT".to_string(), "codex".to_string()),
                        ("MODEL".to_string(), "gpt-reentry".to_string()),
                    ]),
                    ..CallerInputLayers::default()
                },
            );
            state.entry = entry;
            state.invocation_context = Some(invocation.clone());
            state.source_context = Some(invocation.derive_source(&target).unwrap());
            let expected_source_root = state
                .source_context
                .as_ref()
                .and_then(claudine::invocation_context::SourceContext::repository_root)
                .unwrap()
                .to_path_buf();

            let materialized = materialize_harness_prompt(
                &mut state,
                None,
                &launch_root,
                None,
                SchemaStage::Validate,
            )
            .unwrap();
            assert_eq!(
                materialized.prompt.trim(),
                format!(
                    "codex/gpt-reentry/codex/gpt-reentry/{}",
                    launch_root.display()
                ),
                "wrong target-adjusted launch snapshot for {entry:?}"
            );
            assert_eq!(
                materialized
                    .file_resolution_context
                    .as_ref()
                    .and_then(biscuit_file::FileResolutionContext::repository_root),
                Some(expected_source_root.as_path()),
                "file resolution must remain source-relative for {entry:?}"
            );
        }

        let work = invocation.work_snapshot();
        assert_eq!(work.launch_context_constructions, 3);
        assert_eq!(work.launch_context_extensions, 0);
        assert_eq!(work.ambient_fallbacks, 0);
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
        let mut state = compose_state(
            &target,
            CallerInputLayers {
                env_overrides: env,
                ..CallerInputLayers::default()
            },
        );

        let materialized = materialize_harness_prompt(&mut state, None, dir.path(), None, SchemaStage::Validate).unwrap();
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
        let materialized = materialize_harness_prompt(&mut state, None, dir.path(), None, SchemaStage::Validate).unwrap();
        assert_eq!(materialized.prompt.trim(), "reviewing spec.md");
    }
}
