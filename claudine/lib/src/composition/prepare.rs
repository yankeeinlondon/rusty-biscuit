//! Prompt preparation for composition workflows.

use std::collections::BTreeSet;
use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;

use super::error::CompositionError;
use super::guardrails::load_or_create_guardrails;
use super::types::{
    CompositionClosurePlan, CompositionMode, InlineClosurePlan, PreparedComposition,
    ResolvedCompositionSource,
};

/// Prepare a direct (chained) composition with effective frontmatter.
///
/// Composes the entire document through Darkmatter and extracts the
/// effective frontmatter from the composed state. The closure is
/// [`CompositionClosurePlan::Direct`] — no file mutation occurs.
pub fn prepare_direct(
    source: &ResolvedCompositionSource,
) -> Result<PreparedComposition, CompositionError> {
    let options = ComposeOptions::new().with_source_file(&source.resolved_path);
    let (composed, _report) = source
        .markdown
        .compose_with(options)
        .map_err(|e| CompositionError::ComposeFailed(e.to_string()))?;

    let effective_frontmatter = frontmatter_to_value(composed.frontmatter());
    let effective_agent_hint = composed.frontmatter().as_map().get("agent").cloned();

    Ok(PreparedComposition {
        mode: CompositionMode::ChainedDocument,
        resolved_path: source.resolved_path.clone(),
        prompt: composed.content().to_string(),
        effective_frontmatter,
        effective_agent_hint,
        closure: CompositionClosurePlan::Direct,
    })
}

/// Prepare an inline composition with effective frontmatter.
///
/// Extracts the `prompt` frontmatter property, builds a temporary
/// document, composes through Darkmatter, and captures closure state
/// (hashes, managed fields) for deterministic post-execution rewrite.
pub fn prepare_inline(
    source: &ResolvedCompositionSource,
    repo_root: Option<&Path>,
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
    let options = ComposeOptions::new().with_source_file(&source.resolved_path);
    let (composed, _report) = temp_md
        .compose_with(options)
        .map_err(|e| CompositionError::ComposeFailed(e.to_string()))?;

    let effective_frontmatter = frontmatter_to_value(composed.frontmatter());
    let effective_agent_hint = composed.frontmatter().as_map().get("agent").cloned();

    let mut prompt = composed.content().to_string();

    // Append guardrails with the new inline contract
    let guardrails = load_or_create_guardrails(repo_root);
    prompt.push_str("\n\n");
    prompt.push_str(&guardrails);

    // Capture pre-execution hashes for closure
    let original_frontmatter_hash = source.markdown.hash_frontmatter(false);
    let original_body_hash = source.markdown.hash_body(false);

    Ok(PreparedComposition {
        mode: CompositionMode::InlineFrontmatterPrompt,
        resolved_path: source.resolved_path.clone(),
        prompt,
        effective_frontmatter,
        effective_agent_hint,
        closure: CompositionClosurePlan::Inline(InlineClosurePlan {
            original_document_text: source.original_text.clone(),
            original_frontmatter_hash,
            original_body_hash,
            managed_fields: BTreeSet::from(["last_updated".into()]),
        }),
    })
}

/// Convert a `Frontmatter` to a `serde_json::Value::Object`.
fn frontmatter_to_value(fm: &darkmatter::markdown::Frontmatter) -> serde_json::Value {
    serde_json::Value::Object(fm.as_map().iter().map(|(k, v)| (k.clone(), v.clone())).collect())
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

        // Re-parse from disk to match real workflow
        let markdown = Markdown::try_from(file.as_path()).unwrap();
        let original_text = fs::read_to_string(&file).unwrap();
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

        let prepared = prepare_direct(&source).unwrap();
        assert_eq!(prepared.mode, CompositionMode::ChainedDocument);
        assert!(prepared.prompt.contains("Research"));
        // Effective frontmatter should be a JSON object with the keys
        assert!(prepared.effective_frontmatter.is_object());
        let fm_obj = prepared.effective_frontmatter.as_object().unwrap();
        assert_eq!(fm_obj.get("title"), Some(&json!("Research")));
        assert_eq!(prepared.effective_agent_hint, Some(json!("codex")));
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

        let prepared = prepare_inline(&source, None).unwrap();
        assert_eq!(prepared.mode, CompositionMode::InlineFrontmatterPrompt);
        assert!(prepared.prompt.contains("List three colors"));
        assert!(prepared
            .prompt
            .contains("Return the replacement Markdown body content only"));

        // Effective frontmatter should contain composed keys
        assert!(prepared.effective_frontmatter.is_object());
        let fm_obj = prepared.effective_frontmatter.as_object().unwrap();
        assert!(fm_obj.contains_key("prompt"));
        assert_eq!(prepared.effective_agent_hint, Some(json!("claude")));

        // Closure should be Inline with captured hashes
        match &prepared.closure {
            CompositionClosurePlan::Inline(plan) => {
                assert!(!plan.original_document_text.is_empty());
                assert!(plan.managed_fields.contains("last_updated"));
                // Hashes should be non-zero for non-empty content
                assert_ne!(plan.original_frontmatter_hash, 0);
            }
            CompositionClosurePlan::Direct => panic!("expected Inline closure plan"),
        }
    }

    #[test]
    fn inline_composition_missing_prompt() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("title", json!("Test"))], "Content");

        let err = prepare_inline(&source, None).unwrap_err();
        assert!(matches!(err, CompositionError::PromptPropertyMissing));
    }

    #[test]
    fn inline_composition_wrong_type() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("prompt", json!(42))], "Content");

        let err = prepare_inline(&source, None).unwrap_err();
        assert!(matches!(err, CompositionError::PromptPropertyWrongType(_)));
    }
}
