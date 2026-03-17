//! Prompt preparation for composition workflows.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::transform::TransformOptions;

use super::error::CompositionError;
use super::types::{CompositionMode, PreparedPrompt, ResolvedCompositionSource};

/// Prepare an inline prompt from a source document's `prompt` frontmatter.
///
/// Extracts the `prompt` property, builds a temporary `Markdown` with the
/// original frontmatter context, and transforms it through Darkmatter.
pub fn prepare_inline_prompt(
    source: &ResolvedCompositionSource,
) -> Result<PreparedPrompt, CompositionError> {
    let fm = source.markdown.frontmatter();

    // Extract the prompt property
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

    // Build a temporary Markdown with the original frontmatter and prompt as content
    let temp_md = Markdown::with_frontmatter(fm.clone(), &prompt_text);

    let options = TransformOptions::new().with_source_file(&source.resolved_path);
    let (transformed, _report) = temp_md
        .transform_with(options)
        .map_err(|e| CompositionError::ComposeFailed(e.to_string()))?;

    let agent_hint = fm.as_map().get("agent").cloned();

    let mut prompt = transformed.content().to_string();

    // Append guardrails so the agent doesn't mangle the source file.
    prompt.push_str(INLINE_PROMPT_GUARDRAILS);

    Ok(PreparedPrompt {
        mode: CompositionMode::InlineFrontmatterPrompt,
        resolved_path: source.resolved_path.clone(),
        prompt,
        source_agent_hint: agent_hint,
    })
}

/// Guardrail instructions appended to every `--frontmatter-prompt` prompt.
///
/// Prevents the agent from rewriting the frontmatter, creating separate
/// documents, or otherwise defeating the inline-composition workflow.
const INLINE_PROMPT_GUARDRAILS: &str = "\n\n\
> **IMPORTANT:**\n\
>\n\
> - Never change the `prompt` frontmatter property, that property is to read and should not be reformatted or changed in any way\n\
> - Your task is to use the prompt from the `prompt` property to update the body of this document\n\
> - Do not create another document and have this document link to it unless the frontmatter `prompt` explicitly tells you to\n";

/// Prepare a chained prompt from a full source document.
///
/// Transforms the entire document through Darkmatter and uses the
/// composed content as the prompt.
pub fn prepare_chained_prompt(
    source: &ResolvedCompositionSource,
) -> Result<PreparedPrompt, CompositionError> {
    let options = TransformOptions::new().with_source_file(&source.resolved_path);
    let (transformed, _report) = source
        .markdown
        .transform_with(options)
        .map_err(|e| CompositionError::ComposeFailed(e.to_string()))?;

    let agent_hint = source.markdown.frontmatter().as_map().get("agent").cloned();

    Ok(PreparedPrompt {
        mode: CompositionMode::ChainedDocument,
        resolved_path: source.resolved_path.clone(),
        prompt: transformed.content().to_string(),
        source_agent_hint: agent_hint,
    })
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
        ResolvedCompositionSource {
            original_ref: file.to_str().unwrap().to_string(),
            resolved_path: file,
            markdown,
        }
    }

    #[test]
    fn inline_prompt_extracts_and_composes() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[("prompt", json!("List three colors"))],
            "Old content",
        );

        let prepared = prepare_inline_prompt(&source).unwrap();
        assert_eq!(prepared.mode, CompositionMode::InlineFrontmatterPrompt);
        assert!(
            prepared.prompt.contains("List three colors"),
            "expected prompt to contain 'List three colors', got: {:?}",
            prepared.prompt
        );
        // Verify guardrails are appended
        assert!(
            prepared
                .prompt
                .contains("Never change the `prompt` frontmatter property"),
            "expected inline prompt guardrails to be appended"
        );
        assert!(prepared.source_agent_hint.is_none());
    }

    #[test]
    fn inline_prompt_missing_property() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("title", json!("Test"))], "Content");

        let err = prepare_inline_prompt(&source).unwrap_err();
        assert!(matches!(err, CompositionError::PromptPropertyMissing));
    }

    #[test]
    fn inline_prompt_wrong_type() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("prompt", json!(42))], "Content");

        let err = prepare_inline_prompt(&source).unwrap_err();
        assert!(matches!(err, CompositionError::PromptPropertyWrongType(_)));
    }

    #[test]
    fn inline_prompt_preserves_agent_hint() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[
                ("prompt", json!("Do something")),
                ("agent", json!("claude")),
            ],
            "Body",
        );

        let prepared = prepare_inline_prompt(&source).unwrap();
        assert_eq!(prepared.source_agent_hint, Some(json!("claude")));
    }

    #[test]
    fn chained_prompt_uses_full_document() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            &[("title", json!("Research"))],
            "# Research\n\nDo the research.",
        );

        let prepared = prepare_chained_prompt(&source).unwrap();
        assert_eq!(prepared.mode, CompositionMode::ChainedDocument);
        assert!(prepared.prompt.contains("Research"));
        assert!(prepared.prompt.contains("Do the research."));
    }

    #[test]
    fn chained_prompt_preserves_agent_hint() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, &[("agent", json!(true))], "Content");

        let prepared = prepare_chained_prompt(&source).unwrap();
        assert_eq!(prepared.source_agent_hint, Some(json!(true)));
    }
}
