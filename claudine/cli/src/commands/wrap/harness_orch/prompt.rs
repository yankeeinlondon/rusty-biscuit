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
        launch_schema: prepared.launch_schema.clone(),
        file_resolution_context: prepared.input_layers.file_resolution_context.clone(),
        compose_context: Some(prepared.compose_context.clone()),
        document_epoch: prepared.document_epoch.clone(),
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
    load_overlaid_document(&state.source_path, &state.original_ref, &state.overlay)
}

/// Read `path` fresh from disk and merge `overlay` into its authored
/// frontmatter. The one fresh-read assembly shared by the harness re-entry and
/// the command coordinator's post-`initialize` stabilized reread.
pub(crate) fn load_overlaid_document(
    path: &Path,
    original_ref: &str,
    overlay: &indexmap::IndexMap<String, serde_json::Value>,
) -> Result<
    claudine::composition::ResolvedCompositionSource,
    claudine::composition::CompositionError,
> {
    let source_text = fs::read_to_string(path).map_err(|source| {
        claudine::composition::CompositionError::MarkdownLoad {
            path: path.to_path_buf(),
            source: claudine::composition::MarkdownLoadCause::Read(source),
        }
    })?;
    let mut markdown: darkmatter::markdown::Markdown = source_text.clone().into();
    markdown = markdown.with_source(darkmatter::markdown::compose::ComposeSource::File(
        path.to_path_buf(),
    ));
    super::super::overlay::merge_frontmatter_overlay(
        markdown.frontmatter_mut().as_map_mut(),
        overlay,
    );
    Ok(claudine::composition::ResolvedCompositionSource {
        original_ref: original_ref.to_string(),
        resolved_path: path.to_path_buf(),
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
            let context = if state.epoch_context.is_none() {
                let epoch = invocation.begin_document_epoch();
                let context = epoch.capture_launch_context(&requirements);
                state.document_epoch = Some(epoch);
                context
            } else {
                let mut retained = state
                    .epoch_context
                    .clone()
                    .expect("epoch snapshot checked above");
                if let Some(epoch) = state.document_epoch.as_ref() {
                    epoch.extend_launch_context(&mut retained, &requirements);
                } else {
                    invocation.extend_launch_context(&mut retained, &requirements);
                }
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
        options.document_epoch = state.document_epoch.clone();
        options.file_resolution_context = Some(file_resolution);
    } else if let Some(source_context) = source_context {
        options.file_resolution_context = Some(source_context.file_resolution_context().clone());
    }
    options
}

/// Run the active document's pre-flight shell audit and fold any newly-approved
/// commands into the invocation's approved set.
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
pub(crate) fn preflight_harness_document(
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
    let launch_schema = prepared.launch_schema;
    let file_resolution_context = prepared.input_layers.file_resolution_context.clone();
    let compose_context = prepared.compose_context;
    let document_epoch = prepared.document_epoch;
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
        launch_schema,
        file_resolution_context,
        compose_context: Some(compose_context),
        document_epoch,
        lifecycle: Some(lifecycle),
        live_frontmatter,
        runtime_state: std::sync::Arc::clone(&state.runtime_state),
        mcp_body_tags,
    })
}

/// The bootstrap read of an adopted target that authors `initialize`, as the
/// materialized prompt the target's `initialize` runs against.
///
/// Returns `None` for a target without `initialize`: it keeps the eager full
/// read. Otherwise the read composes the frontmatter and lifecycle surface only,
/// after approving its frontmatter `$(...)`, so no body dependency is
/// dereferenced before `initialize` can create it. The result carries no prompt,
/// no closure plan, and no launch schema; the stabilized reread supplies them.
///
/// ## Errors
///
/// A load failure, a frontmatter shell denial, or a frontmatter-side
/// preparation failure.
pub(crate) fn bootstrap_harness_prompt(
    state: &mut HarnessPromptState,
    child_cwd: &Path,
    approval_options: &claudine::harness::ShellApprovalOptions,
) -> Result<Option<MaterializedHarnessPrompt>, claudine::composition::CompositionError> {
    let source = load_overlaid_source(state)?;
    if !crate::commands::wrap::composition::staged_boot::authors_initialize(&source) {
        return Ok(None);
    }
    let mut options = harness_prepare_options(state, &source, child_cwd);
    let mode = match state.mode {
        HarnessPromptMode::Inline => claudine::composition::CompositionMode::InlineFrontmatterPrompt,
        HarnessPromptMode::Compose | HarnessPromptMode::Passthrough => {
            claudine::composition::CompositionMode::ChainedDocument
        }
    };
    let approved =
        claudine::composition::preflight_bootstrap_shell(&source, mode, &options, approval_options)?;
    state.input_layers.add_approved_commands(approved.clone());
    match options.pre_approved_commands.as_mut() {
        Some(set) => set.extend(approved),
        None => options.pre_approved_commands = Some(approved),
    }
    let bootstrap = claudine::composition::prepare_bootstrap(claudine::composition::BootstrapRequest {
        entry: state.entry,
        mode,
        source: &source,
        options,
    })?;
    let live_frontmatter = MaterializedHarnessPrompt::live_cell_from(&bootstrap.effective_frontmatter);
    Ok(Some(MaterializedHarnessPrompt {
        frontmatter: bootstrap.effective_frontmatter,
        prompt: String::new(),
        env_overrides: Vec::new(),
        selection_hints: bootstrap.selection_hints,
        inline_closure_plan: None,
        launch_schema: None,
        file_resolution_context: bootstrap.input_layers.file_resolution_context,
        compose_context: Some(bootstrap.compose_context),
        document_epoch: bootstrap.document_epoch,
        lifecycle: Some(bootstrap.lifecycle),
        live_frontmatter,
        runtime_state: std::sync::Arc::clone(&state.runtime_state),
        mcp_body_tags: Vec::new(),
    }))
}

#[cfg(test)]
mod tests;
