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
            error: Box::new(shell_err),
        },
        other => CompositionError::ComposeFailed(other),
    }
}

/// Options for composition preparation.
#[derive(Debug, Default)]
pub struct PrepareOptions {
    /// Frontmatter `--set` overrides (JSON object).
    pub set_overrides: Option<serde_json::Value>,
    /// Commands pre-approved during pre-flight shell discovery.
    pub pre_approved_commands: Option<std::collections::HashSet<String>>,
    /// Extra environment variables to inject into the composition context.
    pub env_overrides: BTreeMap<String, String>,
    /// Enable Darkmatter composition performance collection.
    pub perf_enabled: bool,
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
use super::lifecycle::parse_lifecycle_config;
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
    let mut ctx = ComposeContext::capture();
    for (key, value) in &options.env_overrides {
        ctx.env_mut().insert(key.clone(), value.clone());
    }
    let mut compose_opts = ComposeOptions::new_with_context(ctx)
        .with_source_file(&source.resolved_path)
        .with_perf(options.perf_enabled);
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

    let effective_frontmatter = frontmatter_to_value(composed.frontmatter());
    let agent_hint = composed
        .frontmatter()
        .as_map()
        .get("agent")
        .map_or(Ok(None), parse_agent_hint)?;
    let model_hint = composed
        .frontmatter()
        .as_map()
        .get("model")
        .map_or(Ok(None), parse_model_hint)?;
    let selection_hints = EffectiveSelectionHints {
        agent: agent_hint,
        model: model_hint,
    };
    let lifecycle = parse_lifecycle_config(&effective_frontmatter)?;

    let source_repo_root = find_git_root_from_path(&source.resolved_path);

    Ok(PreparedComposition {
        mode: CompositionMode::ChainedDocument,
        resolved_path: source.resolved_path.clone(),
        source_repo_root,
        prompt: composed.content().to_string(),
        effective_frontmatter,
        selection_hints,
        closure: CompositionClosurePlan::Direct,
        lifecycle,
        compose_perf: report.perf,
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
    let fm = source.markdown.frontmatter();

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
    let mut compose_opts = ComposeOptions::new_with_context(ctx)
        .with_source_file(&source.resolved_path)
        .with_perf(options.perf_enabled);
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
    let agent_hint = composed
        .frontmatter()
        .as_map()
        .get("agent")
        .map_or(Ok(None), parse_agent_hint)?;
    let model_hint = composed
        .frontmatter()
        .as_map()
        .get("model")
        .map_or(Ok(None), parse_model_hint)?;
    let selection_hints = EffectiveSelectionHints {
        agent: agent_hint,
        model: model_hint,
    };
    let lifecycle = parse_lifecycle_config(&effective_frontmatter)?;

    let mut prompt = composed.content().to_string();

    let source_repo_root = find_git_root_from_path(&source.resolved_path);

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
/// The CLI uses this for *eager* target resolution: it lets the wrapper
/// know which provider will run before composition templates are rendered,
/// so `{{env.AGENT}}` and similar references resolve correctly during
/// body and inline-prompt rendering.
///
/// Untemplated, literal `agent`/`model` values are recognized; values that
/// require composition to materialize (e.g. `agent: "{{env.SOMETHING}}"`)
/// are not resolved here and fall through to post-compose resolution.
///
/// ## Errors
///
/// Returns a [`CompositionError`] if either field is present but holds an
/// unsupported type, or if `agent` references an unknown provider.
pub fn parse_selection_hints_from_frontmatter(
    fm: &darkmatter::markdown::Frontmatter,
) -> Result<EffectiveSelectionHints, CompositionError> {
    let map = fm.as_map();
    let agent = map.get("agent").map_or(Ok(None), parse_agent_hint)?;
    let model = map.get("model").map_or(Ok(None), parse_model_hint)?;
    Ok(EffectiveSelectionHints { agent, model })
}

/// Parse the `agent` frontmatter value into a typed `AgentHint`.
///
/// Accepts a single string or an array of strings. Each string is
/// fuzzy-matched against known providers. Unknown provider names fail
/// early so composition preparation surfaces the error instead of
/// deferring it to launch-time resolver failures.
fn parse_agent_hint(value: &serde_json::Value) -> Result<Option<AgentHint>, CompositionError> {
    match value {
        serde_json::Value::String(s) => {
            let provider = Provider::fuzzy_match_cli_name(s)
                .ok_or_else(|| CompositionError::AgentHintInvalid(s.clone()))?;
            Ok(Some(AgentHint::Single(provider)))
        }
        serde_json::Value::Array(arr) => {
            let mut providers = Vec::with_capacity(arr.len());
            for item in arr {
                match item {
                    serde_json::Value::String(s) => {
                        let provider = Provider::fuzzy_match_cli_name(s)
                            .ok_or_else(|| CompositionError::AgentHintInvalid(s.clone()))?;
                        providers.push(provider);
                    }
                    other => {
                        return Err(CompositionError::AgentHintWrongType(
                            json_type_name(other).to_string(),
                        ));
                    }
                }
            }
            if providers.is_empty() {
                Ok(None)
            } else {
                Ok(Some(AgentHint::List(providers)))
            }
        }
        serde_json::Value::Null => Ok(None),
        other => Err(CompositionError::AgentHintWrongType(
            json_type_name(other).to_string(),
        )),
    }
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
    fn direct_composition_agent_unknown_provider_errors() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("agent", json!("unknown-provider"))], "Content");

        let err = prepare_direct(&source, PrepareOptions::default()).unwrap_err();
        assert!(matches!(err, CompositionError::AgentHintInvalid(_)));
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
}
