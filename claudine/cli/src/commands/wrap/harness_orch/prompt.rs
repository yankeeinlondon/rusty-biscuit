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
) -> Result<MaterializedHarnessPrompt> {
    super::super::overlay::materialize_passthrough_harness_seed(
        source_path,
        prompt,
        shell_cwd,
        runtime_state,
        invocation,
        source_context,
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
    markdown = markdown.with_source(darkmatter::markdown::compose::ComposeSource::File(
        state.source_path.clone(),
    ));
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
///
/// The prepared context follows the document-epoch contract: a retry or resume
/// constructs one fresh launch capture (a new epoch), while the stabilized
/// reread and any same-document refresh extend the retained epoch snapshot in
/// place — the capture anchor, environment capture, and target overrides never
/// change within an epoch. Every launch-facing field comes from the invocation
/// owner either way.
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
    // The launch-anchored epoch snapshot. Constructed through the invocation
    // owner — never from this document's source context — so a harness
    // re-materialization (bootstrap read, stabilized reread, retry, resume,
    // loop refresh) cannot re-anchor prepared `ctx.*` on the prompt directory.
    let propagated = state
        .invocation_context
        .as_ref()
        .zip(source_context)
        .map(|(invocation, source_context)| {
            let requirements = darkmatter::markdown::compose::ContextRequirements::for_document(
                &source.markdown,
            );
            let fresh_epoch = matches!(
                state.entry,
                claudine::composition::DocumentEntryReason::Retry
                    | claudine::composition::DocumentEntryReason::Resume
            );
            let context = if fresh_epoch || state.epoch_context.is_none() {
                invocation.capture_launch_context(&requirements)
            } else {
                let mut retained = state
                    .epoch_context
                    .clone()
                    .expect("epoch snapshot checked above");
                invocation.extend_launch_context(&mut retained, &requirements);
                retained
            };
            // A fresh epoch replaces the retained snapshot; a same-epoch
            // reread keeps the (now possibly extended) one for later stages.
            state.epoch_context = Some(context.clone());
            (context, source_context.file_resolution_context().clone())
        });
    let mut options = input_layers.apply_to(claudine::composition::PrepareOptions {
        invocation_context: state.invocation_context.clone(),
        shell_working_directory: Some(child_cwd.to_path_buf()),
        ..claudine::composition::PrepareOptions::default()
    });
    if let Some((context, file_resolution)) = propagated {
        options.prepared_context = Some(context);
        options.file_resolution_context = Some(file_resolution);
    } else if let Some(source_context) = source_context {
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
            epoch_context: None,
            source_context: None,
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

    #[test]
    fn harness_reentry_epochs_keep_launch_values_and_source_ownership() {
        let temp = tempfile::tempdir().unwrap();
        let launch_repo = temp.path().join("launch");
        let launch_dir = launch_repo.join("alpha/lib");
        let source_repo = temp.path().join("source");
        let source_dir = source_repo.join("nested");
        std::fs::create_dir_all(&launch_dir).unwrap();
        std::fs::create_dir_all(&source_dir).unwrap();
        for repo in [&launch_repo, &source_repo] {
            assert!(std::process::Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(repo)
                .status()
                .unwrap()
                .success());
        }
        std::fs::write(
            launch_repo.join("Cargo.toml"),
            "[workspace]\nmembers = [\"alpha/lib\", \"sibling\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        std::fs::write(
            launch_dir.join("Cargo.toml"),
            "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(launch_repo.join("sibling")).unwrap();
        std::fs::write(
            launch_repo.join("sibling/Cargo.toml"),
            "[package]\nname = \"sibling\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::write(
            launch_dir.join("schema.yaml"),
            "launch_only: string(required)\n",
        )
        .unwrap();
        std::fs::write(launch_dir.join("fragment.md"), "LAUNCH-FRAGMENT\n").unwrap();
        std::fs::write(
            source_dir.join("schema.yaml"),
            "source_marker: string(required)\nspec: 'file(eager; required)'\nprepared_area: \
             string(required)\nprepared_agent: string(required)\nprepared_model: string(required)\n",
        )
        .unwrap();
        std::fs::write(source_dir.join("spec.md"), "SOURCE-SPEC\n").unwrap();
        std::fs::write(source_dir.join("fragment.md"), "SOURCE-FRAGMENT\n").unwrap();
        let target = source_dir.join("target.md");
        std::fs::write(
            &target,
            concat!(
                "---\n",
                "$schema: ./schema.yaml\n",
                "source_marker: source-owned\n",
                "spec: spec.md\n",
                "prepared_area: '{{ ctx.area }}'\n",
                "prepared_agent: '{{ ctx.agent }}'\n",
                "prepared_model: '{{ ctx.model }}'\n",
                "---\n",
                "AREA={{ ctx.area }} AGENT={{ ctx.agent }} MODEL={{ ctx.model }} ",
                "ENV={{ env.AGENT }}/{{ env.MODEL }} FILE={{ file_exists(spec) }}\n",
                "SOURCE-BODY\n",
            ),
        )
        .unwrap();

        let env = BTreeMap::from([
            ("AGENT".to_string(), "codex".to_string()),
            ("MODEL".to_string(), "gpt-5".to_string()),
        ]);
        let mut state = compose_state(
            &target,
            CallerInputLayers {
                env_overrides: env,
                file_ref_fallback_dir: Some(launch_dir.clone()),
                ..CallerInputLayers::default()
            },
        );
        let invocation = claudine::invocation_context::InvocationContext::capture_at(&launch_dir);
        state.source_context = Some(invocation.derive_source(&target).unwrap());
        state.invocation_context = Some(invocation.clone());
        let approval_options = claudine::harness::ShellApprovalOptions::default();

        let assert_materialized = |entry: DocumentEntryReason,
                                   materialized: &MaterializedHarnessPrompt| {
            assert!(
                materialized
                    .prompt
                    .contains("AREA=alpha AGENT=codex MODEL=gpt-5 ENV=codex/gpt-5 FILE=true"),
                "entry {entry:?} lost launch or target identity: {}",
                materialized.prompt
            );
            assert!(
                materialized.prompt.contains("SOURCE-BODY")
                    && !materialized.prompt.contains("LAUNCH-FRAGMENT"),
                "entry {entry:?} did not keep the document body source-owned: {}",
                materialized.prompt
            );
            assert_eq!(materialized.frontmatter["prepared_area"], serde_json::json!("alpha"));
            assert_eq!(materialized.frontmatter["prepared_agent"], serde_json::json!("codex"));
            assert_eq!(materialized.frontmatter["prepared_model"], serde_json::json!("gpt-5"));
            assert_eq!(materialized.frontmatter["source_marker"], serde_json::json!("source-owned"));
        };

        let before_proxy = invocation.work_snapshot();
        preflight_proxy_target(&mut state, &approval_options, &launch_dir).unwrap();
        let proxy = materialize_harness_prompt(
            &mut state,
            Some(&source_repo),
            &launch_dir,
            None,
            SchemaStage::Validate,
        )
        .unwrap();
        assert_materialized(DocumentEntryReason::ProxyTarget, &proxy);
        assert_eq!(
            invocation
                .work_snapshot()
                .document_epoch_since(&before_proxy),
            claudine::invocation_context::DocumentEpochWork {
                launch_context_constructions: 1,
                launch_context_extensions: 0,
                ambient_fallbacks: 0,
                prepared_context_consumers: BTreeMap::from([
                    ("body".to_string(), 1),
                    ("effective-frontmatter".to_string(), 1),
                    ("preflight".to_string(), 1),
                ]),
            },
            "the proxy target's first canonical read must be one complete epoch"
        );

        // Model an initialize-time rewrite that adds a group the bootstrap
        // document did not require. The next materialization is the
        // stabilized reread in the same epoch and must extend, not recapture.
        let original = std::fs::read_to_string(&target).unwrap();
        std::fs::write(
            &target,
            original.replace(
                "SOURCE-BODY\n",
                "SOURCE-BODY OS={{ ctx.os }} REPO={{ ctx.repo_root }}\n",
            ),
        )
        .unwrap();

        let before_stabilized = invocation.work_snapshot();
        preflight_proxy_target(&mut state, &approval_options, &launch_dir).unwrap();
        let stabilized = materialize_harness_prompt(
            &mut state,
            Some(&source_repo),
            &launch_dir,
            None,
            SchemaStage::Validate,
        )
        .unwrap();
        assert_materialized(DocumentEntryReason::ProxyTarget, &stabilized);
        assert!(
            stabilized.prompt.contains(" OS=") && !stabilized.prompt.contains("OS= REPO="),
            "the stabilized reread must populate the newly required OS group: {}",
            stabilized.prompt
        );
        assert_eq!(
            invocation
                .work_snapshot()
                .document_epoch_since(&before_stabilized),
            claudine::invocation_context::DocumentEpochWork {
                launch_context_constructions: 0,
                launch_context_extensions: 1,
                ambient_fallbacks: 0,
                prepared_context_consumers: BTreeMap::from([
                    ("body".to_string(), 1),
                    ("effective-frontmatter".to_string(), 1),
                    ("preflight".to_string(), 1),
                ]),
            },
            "the stabilized reread stays inside the proxy epoch and only extends it"
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
