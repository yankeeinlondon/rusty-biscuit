//! Prompt preparation for composition workflows.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::{Markdown, MarkdownError};

/// Convert a `MarkdownError` into a `CompositionError`, preserving the
/// structured `ShellExpansion` variant so the CLI can render rich errors.
///
/// Shell errors route to the `ShellExpansionFailed` variant so the CLI's
/// renderer has the composing file's path available alongside the structured
/// error. All other `MarkdownError` variants ride through `ComposeFailed`
/// while retaining their typed source in the error chain (via `#[source]`)
/// so the top-level `as_block_error` walker can still produce a rich
/// `BlockError` report.
fn map_compose_error(source_path: &std::path::Path, err: MarkdownError) -> CompositionError {
    match err {
        MarkdownError::ShellExpansion(shell_err) => CompositionError::ShellExpansionFailed {
            source_path: source_path.to_path_buf(),
            error: shell_err,
        },
        other => CompositionError::ComposeFailed(other),
    }
}

/// Bind the agent's workspace onto compose options: the source file (for
/// `::file` transclusion and diagnostic spans) and, when known, the directory
/// the dispatched agent will run in.
///
/// Without `shell_cwd`, Darkmatter defaults `::shell` execution to the source
/// file's parent directory — wrong for `@`-resolved prompts that physically
/// live under `~/.claudine/prompts/` but reason about the repo the agent runs
/// in. Pinning the working directory keeps compose-time shell and the agent on
/// the same root.
pub fn bind_agent_workspace(
    opts: ComposeOptions,
    source_path: &Path,
    shell_cwd: Option<&Path>,
) -> ComposeOptions {
    let opts = opts.with_source_file(source_path);
    match shell_cwd {
        Some(cwd) => opts.with_shell_working_directory(cwd),
        None => opts,
    }
}

/// Options for composition preparation.
#[derive(Debug, Default, Clone)]
pub struct PrepareOptions {
    /// Frontmatter `--set` overrides (JSON object).
    pub set_overrides: Option<serde_json::Value>,
    /// Commands pre-approved during pre-flight shell discovery.
    pub pre_approved_commands: Option<std::collections::HashSet<String>>,
    /// Extra environment variables to inject into the composition context.
    pub env_overrides: BTreeMap<String, String>,
    /// Enable Darkmatter composition performance collection.
    pub perf_enabled: bool,
    /// Pre-computed source repo root.
    ///
    /// When `Some`, [`prepare_direct`] and [`prepare_inline`] use this value
    /// instead of walking up from the source path. CLI callers populate this
    /// from a shared `CompositionPrepContext` so the same repo-root value is
    /// reused across eager target resolution, shell preflight, and
    /// composition preparation. When `None`, the fallback walk
    /// (`find_git_root_from_path`) preserves the original behavior for
    /// library-only callers and tests.
    pub source_repo_root: Option<PathBuf>,
    /// Directory the dispatched agent will run in.
    ///
    /// When `Some`, `::shell` directives execute here instead of the prompt
    /// file's parent, keeping compose-time shell expansion and the agent on
    /// one working directory. CLI callers populate this from
    /// `CompositionPrepContext::launch_workspace.child_cwd`. `None` (the
    /// default) preserves Darkmatter's source-relative fallback for
    /// library-only callers and tests.
    pub shell_working_directory: Option<PathBuf>,
}

/// Walk up from a file path to find the nearest `.git` directory.
fn find_git_root_from_path(path: &Path) -> Option<PathBuf> {
    let start = if path.is_file() { path.parent()? } else { path };
    let mut dir = start;
    loop {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
}

use super::error::CompositionError;
use super::guardrails::load_or_create_guardrails;
use super::lifecycle::{
    parse_lifecycle_config, validate_no_err_in_no_error_events, validate_no_interpolation_leaks,
    validate_no_undefined_lifecycle_variables,
};
use super::types::{
    AgentHint, CompositionClosurePlan, CompositionMode, EffectiveSelectionHints, InlineClosurePlan,
    ModelHint, PreparedComposition, ResolvedCompositionSource,
};
use crate::provider::Provider;
/// Prepare a direct (chained) composition with effective frontmatter.
///
/// Composes the entire document through Darkmatter and extracts the
/// effective frontmatter from the composed state. The closure is
/// [`CompositionClosurePlan::Direct`] — no file mutation occurs.
pub fn prepare_direct(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
) -> Result<PreparedComposition, CompositionError> {
    let override_keys = top_level_override_keys(options.set_overrides.as_ref());
    if let Some((key, replacement)) =
        super::lifecycle::scan_removed_validation_keys(&frontmatter_to_value(source.markdown.frontmatter()))
    {
        return Err(CompositionError::RemovedValidationKey {
            source_path: source.resolved_path.clone(),
            key,
            replacement: replacement.to_string(),
        });
    }
    let mut ctx = ComposeContext::capture();
    for (key, value) in &options.env_overrides {
        ctx.env_mut().insert(key.clone(), value.clone());
    }
    let mut compose_opts = bind_agent_workspace(
        ComposeOptions::new_with_context(ctx),
        &source.resolved_path,
        options.shell_working_directory.as_deref(),
    )
    .with_perf(options.perf_enabled)
    // The composed body is delivered verbatim to the agent and reported as the
    // user prompt. Darkmatter's default strips incidental single newlines, which
    // would collapse an author's line-structured prompt into one paragraph —
    // altering the delivered text and defeating line-count-based report
    // truncation. Preserve the source line breaks for prompt delivery.
    .with_incidental_newline_mode(
        darkmatter::markdown::cleanup::IncidentalNewlineMode::Preserve,
    );
    if let Some(overrides) = options.set_overrides {
        compose_opts = compose_opts.with_set_overrides(overrides);
    }
    if let Some(approved) = options.pre_approved_commands {
        compose_opts = compose_opts.with_pre_approved_commands(approved);
    }
    let (composed, report) = source
        .markdown
        .compose_with(compose_opts)
        .map_err(|e| map_compose_error(&source.resolved_path, e))?;

    let prompt = composed.content().to_string();
    if prompt.trim().is_empty() {
        return Err(CompositionError::ComposedBodyEmpty {
            source_path: source.resolved_path.clone(),
            mode: CompositionMode::ChainedDocument,
            provided_overrides: override_keys,
        });
    }

    let effective_frontmatter = frontmatter_to_value(composed.frontmatter());
    if let Some((key, replacement)) =
        super::lifecycle::scan_removed_validation_keys(&effective_frontmatter)
    {
        return Err(CompositionError::RemovedValidationKey {
            source_path: source.resolved_path.clone(),
            key,
            replacement: replacement.to_string(),
        });
    }
    let agent_full = composed
        .frontmatter()
        .as_map()
        .get("agent")
        .map_or(Ok(ParsedAgentHint::default()), parse_agent_hint_full)?;
    let agent_hint = agent_full.to_agent_hint();
    let model_hint = composed
        .frontmatter()
        .as_map()
        .get("model")
        .map_or(Ok(None), parse_model_hint)?;
    let interactive_hint = composed
        .frontmatter()
        .as_map()
        .get("interactive")
        .map_or(Ok(None), parse_interactive_hint)?;
    let selection_hints = EffectiveSelectionHints {
        agent: agent_hint,
        model: model_hint,
        interactive: interactive_hint,
        agent_invalid: agent_full.invalid,
        agent_was_list: agent_full.is_list,
    };
    let lifecycle = parse_lifecycle_config(&effective_frontmatter, &source.resolved_path)?;
    validate_no_interpolation_leaks(&lifecycle, &source.resolved_path, &report.warnings)?;
    validate_no_undefined_lifecycle_variables(
        source.markdown.frontmatter(),
        &effective_frontmatter,
        &lifecycle,
        &source.resolved_path,
    )?;
    validate_no_err_in_no_error_events(&lifecycle, &source.resolved_path)?;

    let source_repo_root = options
        .source_repo_root
        .or_else(|| find_git_root_from_path(&source.resolved_path));

    Ok(PreparedComposition {
        mode: CompositionMode::ChainedDocument,
        resolved_path: source.resolved_path.clone(),
        source_repo_root,
        prompt,
        effective_frontmatter,
        selection_hints,
        closure: CompositionClosurePlan::Direct,
        lifecycle,
        compose_perf: report.perf,
        dropped_optionals: Vec::new(),
        warnings: report.warnings.clone(),
    })
}

/// Prepare an inline composition with effective frontmatter.
///
/// Extracts the `prompt` frontmatter property, builds a temporary
/// document, composes through Darkmatter, and captures closure state
/// for deterministic post-execution rewrite.
pub fn prepare_inline(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
) -> Result<PreparedComposition, CompositionError> {
    let override_keys = top_level_override_keys(options.set_overrides.as_ref());
    let fm = source.markdown.frontmatter();
    if let Some((key, replacement)) =
        super::lifecycle::scan_removed_validation_keys(&frontmatter_to_value(fm))
    {
        return Err(CompositionError::RemovedValidationKey {
            source_path: source.resolved_path.clone(),
            key,
            replacement: replacement.to_string(),
        });
    }

    let prompt_value = fm
        .as_map()
        .get("prompt")
        .ok_or(CompositionError::PromptPropertyMissing)?;

    let prompt_text = match prompt_value {
        serde_json::Value::String(s) => s.clone(),
        other => {
            return Err(CompositionError::PromptPropertyWrongType(
                json_type_name(other).to_string(),
            ));
        }
    };

    // Build temporary markdown (frontmatter + prompt as body) and compose
    let temp_md = Markdown::with_frontmatter(fm.clone(), &prompt_text);
    let mut ctx = ComposeContext::capture();
    for (key, value) in &options.env_overrides {
        ctx.env_mut().insert(key.clone(), value.clone());
    }
    let mut compose_opts = bind_agent_workspace(
        ComposeOptions::new_with_context(ctx),
        &source.resolved_path,
        options.shell_working_directory.as_deref(),
    )
    .with_perf(options.perf_enabled)
    // The composed body is delivered verbatim to the agent and reported as the
    // user prompt. Darkmatter's default strips incidental single newlines, which
    // would collapse an author's line-structured prompt into one paragraph —
    // altering the delivered text and defeating line-count-based report
    // truncation. Preserve the source line breaks for prompt delivery.
    .with_incidental_newline_mode(
        darkmatter::markdown::cleanup::IncidentalNewlineMode::Preserve,
    );
    if let Some(overrides) = options.set_overrides {
        compose_opts = compose_opts.with_set_overrides(overrides);
    }
    if let Some(approved) = options.pre_approved_commands {
        compose_opts = compose_opts.with_pre_approved_commands(approved);
    }
    let (composed, report) = temp_md
        .compose_with(compose_opts)
        .map_err(|e| map_compose_error(&source.resolved_path, e))?;

    let effective_frontmatter = frontmatter_to_value(composed.frontmatter());
    if let Some((key, replacement)) =
        super::lifecycle::scan_removed_validation_keys(&effective_frontmatter)
    {
        return Err(CompositionError::RemovedValidationKey {
            source_path: source.resolved_path.clone(),
            key,
            replacement: replacement.to_string(),
        });
    }
    let agent_full = composed
        .frontmatter()
        .as_map()
        .get("agent")
        .map_or(Ok(ParsedAgentHint::default()), parse_agent_hint_full)?;
    let agent_hint = agent_full.to_agent_hint();
    let model_hint = composed
        .frontmatter()
        .as_map()
        .get("model")
        .map_or(Ok(None), parse_model_hint)?;
    let interactive_hint = composed
        .frontmatter()
        .as_map()
        .get("interactive")
        .map_or(Ok(None), parse_interactive_hint)?;
    let selection_hints = EffectiveSelectionHints {
        agent: agent_hint,
        model: model_hint,
        interactive: interactive_hint,
        agent_invalid: agent_full.invalid,
        agent_was_list: agent_full.is_list,
    };
    let lifecycle = parse_lifecycle_config(&effective_frontmatter, &source.resolved_path)?;
    validate_no_interpolation_leaks(&lifecycle, &source.resolved_path, &report.warnings)?;
    validate_no_undefined_lifecycle_variables(
        source.markdown.frontmatter(),
        &effective_frontmatter,
        &lifecycle,
        &source.resolved_path,
    )?;
    validate_no_err_in_no_error_events(&lifecycle, &source.resolved_path)?;

    let mut prompt = composed.content().to_string();
    if prompt.trim().is_empty() {
        return Err(CompositionError::ComposedBodyEmpty {
            source_path: source.resolved_path.clone(),
            mode: CompositionMode::InlineFrontmatterPrompt,
            provided_overrides: override_keys,
        });
    }

    let source_repo_root = options
        .source_repo_root
        .or_else(|| find_git_root_from_path(&source.resolved_path));

    // Append guardrails with the new inline contract
    let guardrails = load_or_create_guardrails(source_repo_root.as_deref());
    prompt.push_str("\n\n");
    prompt.push_str(&guardrails);

    // Capture pre-execution hash for closure
    let original_body_hash = source.markdown.hash_body(false);

    Ok(PreparedComposition {
        mode: CompositionMode::InlineFrontmatterPrompt,
        resolved_path: source.resolved_path.clone(),
        source_repo_root,
        prompt,
        effective_frontmatter,
        selection_hints,
        closure: CompositionClosurePlan::Inline(InlineClosurePlan {
            original_document_text: source.original_text.clone(),
            original_body_hash,
        }),
        lifecycle,
        compose_perf: report.perf,
        dropped_optionals: Vec::new(),
        warnings: report.warnings.clone(),
    })
}

/// Convert a `Frontmatter` to a `serde_json::Value::Object`.
fn frontmatter_to_value(fm: &darkmatter::markdown::Frontmatter) -> serde_json::Value {
    serde_json::Value::Object(
        fm.as_map()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    )
}

/// Parse selection hints (`agent`, `model`) from a raw Markdown frontmatter
/// without composing the document.
///
/// The CLI calls this for *eager* target resolution — knowing which
/// provider will run before composition templates render, so `{{env.AGENT}}`
/// and similar references resolve correctly during body/inline rendering.
/// Only untemplated literal values are recognized here; templated values
/// fall through to post-compose resolution.
///
/// ## Errors
///
/// Returns a [`CompositionError`] if either field is present but holds an
/// unsupported type, or if `agent` references an unknown provider.
pub fn parse_selection_hints_from_frontmatter(
    fm: &darkmatter::markdown::Frontmatter,
) -> Result<EffectiveSelectionHints, CompositionError> {
    let map = fm.as_map();
    let agent_full = map
        .get("agent")
        .map_or(Ok(ParsedAgentHint::default()), parse_agent_hint_full)?;
    let agent = agent_full.to_agent_hint();
    let model = map.get("model").map_or(Ok(None), parse_model_hint)?;
    let interactive = map
        .get("interactive")
        .map_or(Ok(None), parse_interactive_hint)?;
    Ok(EffectiveSelectionHints {
        agent,
        model,
        interactive,
        agent_invalid: agent_full.invalid,
        agent_was_list: agent_full.is_list,
    })
}

/// Raw parse result for an `agent` frontmatter value.
///
/// Preserves fuzzy-matched providers alongside unknown strings so invalid
/// entries can be surfaced as non-fatal state rather than aborting
/// composition. The `is_list` flag distinguishes a single value from an
/// array so a one-element list is still rendered as a list suggestion.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ParsedAgentHint {
    valid: Vec<Provider>,
    invalid: Vec<String>,
    is_list: bool,
}

impl ParsedAgentHint {
    fn to_agent_hint(&self) -> Option<AgentHint> {
        if self.valid.is_empty() {
            return None;
        }
        if self.is_list {
            Some(AgentHint::List(self.valid.clone()))
        } else {
            Some(AgentHint::Single(self.valid[0]))
        }
    }
}

/// Parse the `agent` frontmatter value while preserving invalid entries.
///
/// Strings that do not match a known provider are collected in
/// [`ParsedAgentHint::invalid`] rather than raising an error. Type errors
/// (non-string array entries, booleans, numbers, objects) are still fatal
/// because they cannot be meaningfully classified.
fn parse_agent_hint_full(value: &serde_json::Value) -> Result<ParsedAgentHint, CompositionError> {
    let mut result = ParsedAgentHint::default();
    match value {
        serde_json::Value::String(s) => {
            if let Some(provider) = Provider::fuzzy_match_cli_name(s) {
                result.valid.push(provider);
            } else {
                result.invalid.push(s.clone());
            }
        }
        serde_json::Value::Array(arr) => {
            result.is_list = true;
            for item in arr {
                match item {
                    serde_json::Value::String(s) => {
                        if let Some(provider) = Provider::fuzzy_match_cli_name(s) {
                            result.valid.push(provider);
                        } else {
                            result.invalid.push(s.clone());
                        }
                    }
                    other => {
                        return Err(CompositionError::AgentHintWrongType(
                            json_type_name(other).to_string(),
                        ));
                    }
                }
            }
        }
        serde_json::Value::Null => {}
        other => {
            return Err(CompositionError::AgentHintWrongType(
                json_type_name(other).to_string(),
            ));
        }
    }
    Ok(result)
}

/// Parse the `model` frontmatter value into a typed `ModelHint`.
///
/// Accepts a single string or an array of strings. Model identifiers
/// are stored verbatim; validation against provider catalogs happens
/// later in the resolution pipeline.
fn parse_model_hint(value: &serde_json::Value) -> Result<Option<ModelHint>, CompositionError> {
    match value {
        serde_json::Value::String(s) => Ok(Some(ModelHint::Single(s.clone()))),
        serde_json::Value::Array(arr) => {
            let mut models = Vec::with_capacity(arr.len());
            for item in arr {
                match item {
                    serde_json::Value::String(s) => models.push(s.clone()),
                    other => {
                        return Err(CompositionError::ModelHintWrongType(
                            json_type_name(other).to_string(),
                        ));
                    }
                }
            }
            if models.is_empty() {
                Ok(None)
            } else {
                Ok(Some(ModelHint::List(models)))
            }
        }
        serde_json::Value::Null => Ok(None),
        other => Err(CompositionError::ModelHintWrongType(
            json_type_name(other).to_string(),
        )),
    }
}

/// Parse the `interactive` frontmatter value into an optional boolean.
///
/// Accepts `true`, `false`, or `null` (treated as absent). Anything else
/// is a typed error naming the offending JSON type.
pub fn parse_interactive_hint(
    value: &serde_json::Value,
) -> Result<Option<bool>, CompositionError> {
    match value {
        serde_json::Value::Bool(b) => Ok(Some(*b)),
        serde_json::Value::Null => Ok(None),
        other => Err(CompositionError::InteractiveHintWrongType(
            json_type_name(other).to_string(),
        )),
    }
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Extract the top-level keys from a `set_overrides` JSON object, in the
/// underlying `serde_json::Map` iteration order (alphabetical without the
/// `preserve_order` feature). Returns an empty `Vec` when overrides are
/// absent or the value is not an object. Used to surface "what the user
/// provided" in the `ComposedBodyEmpty` error so the diagnostic can name
/// the variables that were visible to `::block when=…` conditions.
fn top_level_override_keys(overrides: Option<&serde_json::Value>) -> Vec<String> {
    match overrides {
        Some(serde_json::Value::Object(map)) => map.keys().cloned().collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Provider;
    use darkmatter::markdown::Frontmatter;
    use serde_json::json;
    use std::fs;
    use tempfile::TempDir;

    fn make_source(
        dir: &TempDir,
        frontmatter: &[(&str, serde_json::Value)],
        content: &str,
    ) -> ResolvedCompositionSource {
        let file = dir.path().join("test.md");
        let mut fm = Frontmatter::new();
        for (key, value) in frontmatter {
            fm.insert(key, value.clone()).unwrap();
        }
        let md = Markdown::with_frontmatter(fm, content);
        fs::write(&file, md.as_string()).unwrap();

        // Read once and construct Markdown from the string to avoid double I/O
        let original_text = fs::read_to_string(&file).unwrap();
        let markdown: Markdown = original_text.clone().into();
        ResolvedCompositionSource {
            original_ref: file.to_str().unwrap().to_string(),
            resolved_path: file,
            original_text,
            markdown,
        }
    }

    #[test]
    fn direct_composition_uses_effective_frontmatter() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[("title", json!("Research")), ("agent", json!("codex"))],
            "# Research\n\nDo the research.",
        );

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        assert_eq!(prepared.mode, CompositionMode::ChainedDocument);
        assert!(prepared.prompt.contains("Research"));
        // Effective frontmatter should be a JSON object with the keys
        assert!(prepared.effective_frontmatter.is_object());
        let fm_obj = prepared.effective_frontmatter.as_object().unwrap();
        assert_eq!(fm_obj.get("title"), Some(&json!("Research")));
        assert_eq!(
            prepared.selection_hints.agent,
            Some(AgentHint::Single(Provider::Codex))
        );
        assert!(matches!(prepared.closure, CompositionClosurePlan::Direct));
    }

    #[test]
    fn inline_composition_uses_effective_frontmatter() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[
                ("prompt", json!("List three colors")),
                ("agent", json!("claude")),
            ],
            "Old content",
        );

        let prepared = prepare_inline(&source, PrepareOptions::default()).unwrap();
        assert_eq!(prepared.mode, CompositionMode::InlineFrontmatterPrompt);
        assert!(prepared.prompt.contains("List three colors"));
        assert!(
            prepared
                .prompt
                .contains("Return the replacement Markdown body content only")
        );

        // Effective frontmatter should contain composed keys
        assert!(prepared.effective_frontmatter.is_object());
        let fm_obj = prepared.effective_frontmatter.as_object().unwrap();
        assert!(fm_obj.contains_key("prompt"));
        assert_eq!(
            prepared.selection_hints.agent,
            Some(AgentHint::Single(Provider::Claude))
        );

        // Closure should be Inline with captured hash
        match &prepared.closure {
            CompositionClosurePlan::Inline(plan) => {
                assert!(!plan.original_document_text.is_empty());
                // Body hash should be non-zero for non-empty content
                assert_ne!(plan.original_body_hash, 0);
            }
            CompositionClosurePlan::Direct => panic!("expected Inline closure plan"),
        }
    }

    #[test]
    fn inline_composition_missing_prompt() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("title", json!("Test"))], "Content");

        let err = prepare_inline(&source, PrepareOptions::default()).unwrap_err();
        assert!(matches!(err, CompositionError::PromptPropertyMissing));
    }

    #[test]
    fn inline_composition_wrong_type() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("prompt", json!(42))], "Content");

        let err = prepare_inline(&source, PrepareOptions::default()).unwrap_err();
        assert!(matches!(err, CompositionError::PromptPropertyWrongType(_)));
    }

    #[test]
    fn direct_composition_parses_lifecycle_config() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[
                ("title", json!("Test")),
                ("start", json!({"stderr": "Starting", "effect": "doorbell"})),
                ("success", json!({"say": "All done"})),
            ],
            "Do the work.",
        );

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        assert!(prepared.lifecycle.start.is_some());
        assert!(prepared.lifecycle.success.is_some());
        assert!(prepared.lifecycle.blocked.is_none());
        assert!(prepared.lifecycle.failure.is_none());
    }

    /// Regression: Darkmatter's `normalize_list_spacing` used to insert a
    /// blank line between a tight parent list item and the first child of a
    /// nested unordered list. That blank line was mis-rendered as an indented
    /// code block by downstream renderers, corrupting prompts like the
    /// `## Closure` section of `prompts/review-feature.md`.
    #[test]
    fn direct_composition_preserves_tight_nested_list() {
        let dir = TempDir::new().unwrap();
        let body = r#"## Closure

- Save your review suggestions to a file
- Save the following frontmatter properties on "review.md":
    - based on your review suggestions indicate whether you think this feature is **ready for production**
    - set the `agent` frontmatter property to claude
    - set the `model` frontmatter property to some-model
    - set the `created` frontmatter property to today

**bold:**
"#;
        let source = make_source(&dir, &[("title", json!("Test"))], body);

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        assert!(
            prepared.prompt.contains("properties on \"review.md\":\n    - based on"),
            "parent item must be immediately followed by its first child; got:\n{}",
            prepared.prompt
        );
        assert!(
            !prepared.prompt.contains("properties on \"review.md\":\n\n    - "),
            "tight nested list must not gain a blank line between parent and child; got:\n{}",
            prepared.prompt
        );
    }

    #[test]
    fn inline_composition_parses_lifecycle_config() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[
                ("prompt", json!("Write something")),
                ("failure", json!({"stderr": "Failed"})),
            ],
            "Old content",
        );

        let prepared = prepare_inline(&source, PrepareOptions::default()).unwrap();
        assert!(prepared.lifecycle.failure.is_some());
        assert!(prepared.lifecycle.start.is_none());
    }

    #[test]
    fn invalid_lifecycle_config_fails_preparation() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[
                ("title", json!("Test")),
                ("start", json!({"say": "Hello", "say_first": "Also hello"})),
            ],
            "Content",
        );

        let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
        assert!(matches!(err, CompositionError::LifecycleSayConflict(_)));
    }

    #[test]
    fn malformed_lifecycle_interpolation_fails_preparation() {
        // A *mixed* lifecycle string (literal text plus a malformed `{{ … }}`)
        // is not whole-value executable state, so Darkmatter leaves it lenient
        // and the claudine leak guard is the layer that rejects it. (A
        // whole-value malformed span is caught earlier by Darkmatter's strict
        // frontmatter interpolation — see
        // `implement_suggestions_prompt_rejects_malformed_spec_path`.)
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[
                ("title", json!("Test")),
                ("start", json!({"message": "leak {{ parent_dir(review)) }}"})),
            ],
            "Content",
        );

        let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
        match err {
            CompositionError::LifecycleInterpolationLeak {
                property,
                expression,
                ..
            } => {
                assert_eq!(property, "start.message");
                assert!(expression.contains("parent_dir(review))"));
            }
            other => panic!("expected LifecycleInterpolationLeak, got: {other:?}"),
        }
    }

    #[test]
    fn undefined_lifecycle_variable_fails_preparation() {
        // Regression: a bare `{{ missing_lifecycle_var }}` resolves to an empty
        // string during composition with no Darkmatter warning, so the
        // post-compose leak guard never sees it. Preparation must still fail
        // before the message becomes eligible for lifecycle dispatch.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[
                ("title", json!("Test")),
                (
                    "start",
                    json!({"message": "before {{ missing_lifecycle_var }} after"}),
                ),
            ],
            "Content",
        );

        let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
        match err {
            CompositionError::LifecycleUndefinedVariable {
                property, variable, ..
            } => {
                assert_eq!(property, "start.message");
                assert_eq!(variable, "missing_lifecycle_var");
            }
            other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
        }
    }

    #[test]
    fn lifecycle_variable_defined_in_frontmatter_passes_preparation() {
        // A bare variable that names a real frontmatter key is defined, so it
        // must not trip the undefined-variable guard.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[
                ("area", json!("claudine")),
                ("start", json!({"message": "working on {{ area }}"})),
            ],
            "Content",
        );

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        let message = prepared
            .lifecycle
            .start
            .as_ref()
            .unwrap()
            .message
            .as_ref()
            .unwrap();
        assert_eq!(message, "working on claudine");
    }

    #[test]
    fn lifecycle_fallback_for_undefined_variable_passes_preparation() {
        // Fallback (`{{ x || 'y' }}`) intentionally tolerates an undefined
        // operand, so the guard must leave it alone.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[(
                "start",
                json!({"message": "{{ missing_lifecycle_var || 'default' }}"}),
            )],
            "Content",
        );

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        let message = prepared
            .lifecycle
            .start
            .as_ref()
            .unwrap()
            .message
            .as_ref()
            .unwrap();
        assert_eq!(message, "default");
    }

    #[test]
    fn clean_lifecycle_interpolation_passes_preparation() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[
                ("title", json!("Test")),
                ("start", json!({"message": "{{ ctx.today }}"})),
            ],
            "Content",
        );

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        let start = prepared.lifecycle.start.as_ref().unwrap();
        let message = start.message.as_ref().unwrap();
        assert!(!message.contains("{{"));
        assert!(!message.contains("}}"));
    }

    #[test]
    fn lifecycle_leak_reported_for_first_field_in_deterministic_order() {
        let dir = TempDir::new().unwrap();
        // Mixed lifecycle strings stay lenient in Darkmatter and surface
        // through the claudine leak guard, which reports the first leaking
        // field in deterministic order. Whole-value malformed spans would
        // instead be rejected earlier by Darkmatter's strict interpolation.
        let source = make_source(
            &dir,
            &[
                ("title", json!("Test")),
                ("start", json!({"message": "leak {{ parent_dir(review)) }}"})),
                ("failure", json!({"say": "leak {{ broken( }}"})),
            ],
            "Content",
        );

        let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
        match err {
            CompositionError::LifecycleInterpolationLeak { property, .. } => {
                assert_eq!(property, "start.message");
            }
            other => panic!("expected LifecycleInterpolationLeak, got: {other:?}"),
        }
    }

    #[test]
    fn malformed_whole_value_spec_path_is_rejected() {
        // Regression for the original `implement-suggestions.md` reproduction:
        // a `spec_path` frontmatter value that is a malformed whole-value
        // interpolation — `{{ dirname(review) + '/spec.md') }}` carries an
        // unbalanced paren. A whole-value `{{ … }}` is executable state, so
        // composition must abort with a frontmatter interpolation parse error
        // that names `spec_path`, instead of leaking the raw template
        // downstream as a successful effective-frontmatter value. The fixture
        // is self-contained — it must not read the shipped prompt, whose shape
        // is free to change without breaking this guard.
        let dir = TempDir::new().unwrap();
        let review_file = dir.path().join("review.md");
        fs::write(&review_file, "# Review\n").unwrap();

        let source = make_source(
            &dir,
            &[
                ("title", json!("Implement Suggestions")),
                ("spec_path", json!("{{ dirname(review) + '/spec.md') }}")),
            ],
            "Implement the suggestions from {{ spec_path }}.",
        );

        let options = PrepareOptions {
            set_overrides: Some(json!({ "review": review_file.to_str().unwrap() })),
            ..Default::default()
        };

        let err = prepare_direct(&source, options).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("spec_path"),
            "error must name the offending key, got: {msg}"
        );
        assert!(
            msg.contains("Interpolation parse failed"),
            "error must report the interpolation parse failure, got: {msg}"
        );
    }

    #[test]
    fn direct_composition_with_env_overrides() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[("title", json!("Test"))],
            "FAIL_FAST is {{env.FAIL_FAST}}",
        );

        let options = PrepareOptions {
            env_overrides: std::collections::BTreeMap::from([(
                "FAIL_FAST".to_string(),
                "false".to_string(),
            )]),
            ..Default::default()
        };

        let prepared = prepare_direct(&source, options).unwrap();
        assert!(prepared.prompt.contains("FAIL_FAST is false"));
    }

    #[test]
    fn direct_lifecycle_ctx_agent_uses_env_overrides() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[(
                "start",
                json!({
                    "message": "{{ctx.agent}}/{{ctx.model}}"
                }),
            )],
            "Prompt",
        );

        let options = PrepareOptions {
            env_overrides: std::collections::BTreeMap::from([
                ("AGENT".to_string(), "codex".to_string()),
                ("MODEL".to_string(), "gpt-5".to_string()),
            ]),
            ..Default::default()
        };

        let prepared = prepare_direct(&source, options).unwrap();
        let start = prepared.lifecycle.start.as_ref().unwrap();
        assert_eq!(start.message.as_deref(), Some("codex/gpt-5"));
    }

    #[test]
    fn direct_composition_perf_disabled_yields_none() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("title", json!("Test"))], "Simple content.");

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        assert!(prepared.compose_perf.is_none());
    }

    #[test]
    fn direct_composition_perf_enabled_yields_some() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("title", json!("Test"))], "Simple content.");

        let options = PrepareOptions {
            perf_enabled: true,
            ..Default::default()
        };
        let prepared = prepare_direct(&source, options).unwrap();
        assert!(prepared.compose_perf.is_some());
    }

    #[test]
    fn inline_composition_preserves_closure_with_perf_enabled() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[
                ("prompt", json!("List three colors")),
                ("agent", json!("claude")),
            ],
            "Old content",
        );

        let options = PrepareOptions {
            perf_enabled: true,
            ..Default::default()
        };
        let prepared = prepare_inline(&source, options).unwrap();
        assert_eq!(prepared.mode, CompositionMode::InlineFrontmatterPrompt);
        assert!(prepared.compose_perf.is_some());

        // Closure should still be Inline with captured hash
        match &prepared.closure {
            CompositionClosurePlan::Inline(plan) => {
                assert!(!plan.original_document_text.is_empty());
                assert_ne!(plan.original_body_hash, 0);
            }
            CompositionClosurePlan::Direct => panic!("expected Inline closure plan"),
        }
    }

    #[test]
    fn direct_composition_parses_agent_list() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("agent", json!(["gemini", "codex"]))], "Content");

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        assert_eq!(
            prepared.selection_hints.agent,
            Some(AgentHint::List(vec![Provider::Gemini, Provider::Codex]))
        );
    }

    #[test]
    fn direct_composition_parses_model_single() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("model", json!("gpt-4o"))], "Content");

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        assert_eq!(
            prepared.selection_hints.model,
            Some(ModelHint::Single("gpt-4o".to_string()))
        );
    }

    #[test]
    fn direct_composition_parses_model_list() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("model", json!(["gpt-4o", "o3-mini"]))], "Content");

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        assert_eq!(
            prepared.selection_hints.model,
            Some(ModelHint::List(vec![
                "gpt-4o".to_string(),
                "o3-mini".to_string()
            ]))
        );
    }

    #[test]
    fn direct_composition_agent_unknown_provider_is_non_fatal() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("agent", json!("unknown-provider"))], "Content");

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        assert_eq!(prepared.selection_hints.agent, None);
        assert_eq!(
            prepared.selection_hints.agent_invalid,
            vec!["unknown-provider".to_string()]
        );
    }

    #[test]
    fn direct_composition_agent_list_skips_invalid_entries() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[("agent", json!(["claude", "not-real", "codex"]))],
            "Content",
        );

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        assert_eq!(
            prepared.selection_hints.agent,
            Some(AgentHint::List(vec![Provider::Claude, Provider::Codex]))
        );
        assert_eq!(
            prepared.selection_hints.agent_invalid,
            vec!["not-real".to_string()]
        );
    }

    #[test]
    fn direct_composition_agent_list_all_invalid_is_empty_hint() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("agent", json!(["bad", "worse"]))], "Content");

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        assert_eq!(prepared.selection_hints.agent, None);
        assert_eq!(
            prepared.selection_hints.agent_invalid,
            vec!["bad".to_string(), "worse".to_string()]
        );
        // Even with no valid providers, the list-ness must survive so a
        // zero-valid list is not mistaken for a scalar value downstream.
        assert!(prepared.selection_hints.agent_was_list);
    }

    #[test]
    fn direct_composition_single_entry_all_invalid_list_preserves_list_flag() {
        // A one-element invalid list (`agent: ["not-real"]`) must record
        // `agent_was_list = true` so classification routes it to the
        // zero-installed-list state, not the single-invalid scalar state.
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("agent", json!(["not-real"]))], "Content");

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        assert_eq!(prepared.selection_hints.agent, None);
        assert_eq!(
            prepared.selection_hints.agent_invalid,
            vec!["not-real".to_string()]
        );
        assert!(prepared.selection_hints.agent_was_list);
    }

    #[test]
    fn direct_composition_single_scalar_invalid_is_not_list() {
        // A scalar invalid value (`agent: not-real`) must record
        // `agent_was_list = false`.
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("agent", json!("not-real"))], "Content");

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        assert_eq!(prepared.selection_hints.agent, None);
        assert!(!prepared.selection_hints.agent_was_list);
    }

    #[test]
    fn direct_composition_agent_wrong_type_errors() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("agent", json!(42))], "Content");

        let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
        assert!(matches!(err, CompositionError::AgentHintWrongType(_)));
    }

    #[test]
    fn direct_composition_model_wrong_type_errors() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("model", json!(42))], "Content");

        let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
        assert!(matches!(err, CompositionError::ModelHintWrongType(_)));
    }

    #[test]
    fn direct_composition_agent_list_with_non_string_errors() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("agent", json!(["claude", 42]))], "Content");

        let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
        assert!(matches!(err, CompositionError::AgentHintWrongType(_)));
    }

    #[test]
    fn direct_composition_model_list_with_non_string_errors() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("model", json!(["gpt-4o", 42]))], "Content");

        let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
        assert!(matches!(err, CompositionError::ModelHintWrongType(_)));
    }

    #[test]
    fn direct_composition_no_agent_or_model_hints() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("title", json!("Test"))], "Content");

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        assert_eq!(prepared.selection_hints.agent, None);
        assert_eq!(prepared.selection_hints.model, None);
    }

    #[test]
    fn direct_composition_empty_body_returns_composed_body_empty() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("title", json!("Test"))], "   \n\n  \t\n");

        let options = PrepareOptions {
            set_overrides: Some(json!({"spec": "plan.md", "phase": 1})),
            ..Default::default()
        };

        let err = prepare_direct(&source, options).unwrap_err();
        match err {
            CompositionError::ComposedBodyEmpty {
                source_path,
                mode,
                provided_overrides,
            } => {
                assert_eq!(source_path, source.resolved_path);
                assert_eq!(mode, CompositionMode::ChainedDocument);
                let keys: std::collections::HashSet<String> =
                    provided_overrides.into_iter().collect();
                assert_eq!(
                    keys,
                    ["spec", "phase"].iter().map(|s| s.to_string()).collect()
                );
            }
            other => panic!("expected ComposedBodyEmpty, got: {other:?}"),
        }
    }

    #[test]
    fn direct_composition_block_strips_everything_returns_composed_body_empty() {
        let dir = TempDir::new().unwrap();
        // Body consists entirely of a `::block when="review"` whose guard is
        // not satisfied (no `review` override provided). After composition
        // the body should be empty and detection must fire — this mirrors
        // the real-world prompt that triggered the user-facing bug report.
        let body = "::block when=\"review\"\n## Context\n\nThis is review-only content.\n::end-block\n";
        let source = make_source(&dir, &[("title", json!("Test"))], body);

        let options = PrepareOptions {
            set_overrides: Some(json!({"spec": "plan.md"})),
            ..Default::default()
        };

        let err = prepare_direct(&source, options).unwrap_err();
        let CompositionError::ComposedBodyEmpty {
            mode,
            provided_overrides,
            ..
        } = err
        else {
            panic!("expected ComposedBodyEmpty, got a different error");
        };
        assert_eq!(mode, CompositionMode::ChainedDocument);
        assert_eq!(provided_overrides, vec!["spec".to_string()]);
    }

    /// Regression: a `@`-resolved prompt physically lives outside the repo
    /// it reasons about (e.g. `~/.claudine/prompts/commit.md`). With
    /// `shell_working_directory` set, `::shell` directives must run there,
    /// not next to the template file — otherwise `sniff repo packages` and
    /// friends execute in the wrong repo.
    #[test]
    fn direct_composition_runs_shell_in_configured_working_directory() {
        let source_dir = TempDir::new().unwrap();
        let work_dir = TempDir::new().unwrap();
        let source = make_source(&source_dir, &[("title", json!("T"))], "::shell pwd\n");

        let mut approved = std::collections::HashSet::new();
        approved.insert("pwd".to_string());
        let options = PrepareOptions {
            pre_approved_commands: Some(approved),
            shell_working_directory: Some(work_dir.path().to_path_buf()),
            ..Default::default()
        };

        let prepared = prepare_direct(&source, options).unwrap();
        // `pwd` reports the physical path, so compare against canonicalized
        // temp dirs (macOS routes `/var` through a `/private` symlink).
        let work_canon = std::fs::canonicalize(work_dir.path()).unwrap();
        let source_canon = std::fs::canonicalize(source_dir.path()).unwrap();
        assert!(
            prepared.prompt.contains(work_canon.to_str().unwrap()),
            "expected shell to run in work_dir; got: {}",
            prepared.prompt
        );
        assert!(
            !prepared.prompt.contains(source_canon.to_str().unwrap()),
            "shell must not run in the prompt's parent dir; got: {}",
            prepared.prompt
        );
    }

    /// Documents the library default: with no `shell_working_directory`,
    /// Darkmatter's source-relative fallback runs `::shell` in the prompt
    /// file's parent directory.
    #[test]
    fn parse_interactive_hint_accepts_true_false_and_null() {
        assert_eq!(parse_interactive_hint(&json!(true)).unwrap(), Some(true));
        assert_eq!(parse_interactive_hint(&json!(false)).unwrap(), Some(false));
        assert_eq!(parse_interactive_hint(&json!(null)).unwrap(), None);
    }

    #[test]
    fn parse_interactive_hint_rejects_non_booleans() {
        for value in [
            json!("true"),
            json!(42),
            json!([true]),
            json!({"interactive": true}),
        ] {
            let err = parse_interactive_hint(&value).unwrap_err();
            match err {
                CompositionError::InteractiveHintWrongType(found) => {
                    assert_eq!(found, json_type_name(&value));
                }
                other => panic!("expected InteractiveHintWrongType for {value}, got {other:?}"),
            }
        }
    }

    #[test]
    fn direct_composition_parses_interactive_hint() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("interactive", json!(true))], "Content");

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        assert_eq!(prepared.selection_hints.interactive, Some(true));
    }

    #[test]
    fn direct_composition_interactive_null_is_absent() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("interactive", json!(null))], "Content");

        let prepared = prepare_direct(&source, PrepareOptions::default()).unwrap();
        assert_eq!(prepared.selection_hints.interactive, None);
    }

    #[test]
    fn direct_composition_interactive_wrong_type_errors() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("interactive", json!("yes"))], "Content");

        let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
        assert!(matches!(err, CompositionError::InteractiveHintWrongType(_)));
    }

    #[test]
    fn inline_composition_parses_interactive_hint() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[("prompt", json!("Write something")), ("interactive", json!(false))],
            "Old content",
        );

        let prepared = prepare_inline(&source, PrepareOptions::default()).unwrap();
        assert_eq!(prepared.selection_hints.interactive, Some(false));
    }

    #[test]
    fn parse_selection_hints_from_frontmatter_reads_interactive() {
        let mut fm = Frontmatter::new();
        fm.insert("interactive", json!(true)).unwrap();
        let md = Markdown::with_frontmatter(fm, "Content");

        let hints = parse_selection_hints_from_frontmatter(md.frontmatter()).unwrap();
        assert_eq!(hints.interactive, Some(true));
    }
}
