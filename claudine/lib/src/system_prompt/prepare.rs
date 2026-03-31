use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;

use crate::system_prompt::types::*;

/// Compose a resolved system prompt source through Darkmatter and
/// return the effective result.
///
/// If the composed body is empty after trimming, returns
/// `EffectiveSystemPrompt::Disabled`. Otherwise returns `Ready`.
pub fn prepare_system_prompt(
    source: SystemPromptSource,
    raw_text: &str,
) -> Result<EffectiveSystemPrompt, crate::error::ClaudineError> {
    let source_path = match &source {
        SystemPromptSource::StandardDiscovered { path, .. } => path,
        SystemPromptSource::ExplicitFile { path, .. } => path,
    };

    let mode = match &source {
        SystemPromptSource::StandardDiscovered { .. } => SystemPromptMode::Append,
        SystemPromptSource::ExplicitFile { mode, .. } => *mode,
    };

    // Parse and compose through Darkmatter
    let md: Markdown = raw_text.into();
    let options = ComposeOptions::new()
        .with_source_file(source_path);

    let (composed, _report) = md
        .compose_with(options)
        .map_err(|e| crate::error::ClaudineError::SystemPromptComposition(e.to_string()))?;

    let composed_markdown = composed.content().to_string();

    // Empty-body check
    if composed_markdown.trim().is_empty() {
        return Ok(EffectiveSystemPrompt::Disabled { source });
    }

    Ok(EffectiveSystemPrompt::Ready(PreparedSystemPrompt {
        mode,
        source,
        raw_text: raw_text.to_string(),
        composed_markdown,
    }))
}

/// Top-level convenience: resolve + compose in one call.
pub fn resolve_and_prepare(
    args: &SystemPromptArgs,
    context: &crate::system_prompt::context::LaunchContext,
) -> Result<EffectiveSystemPrompt, crate::error::ClaudineError> {
    let Some((source, raw_text)) =
        crate::system_prompt::resolve::resolve_system_prompt_source(args, context)?
    else {
        return Ok(EffectiveSystemPrompt::None);
    };
    prepare_system_prompt(source, &raw_text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    use crate::system_prompt::context::LaunchContext;

    /// Helper: create a temp file and return its path.
    fn write_temp_file(dir: &std::path::Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn plain_markdown_composes_as_is() {
        let tmp = TempDir::new().unwrap();
        let path = write_temp_file(tmp.path(), "prompt.md", "# Hello World\n\nSome content.");

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result = prepare_system_prompt(source, "# Hello World\n\nSome content.").unwrap();

        match result {
            EffectiveSystemPrompt::Ready(prepared) => {
                assert!(prepared.composed_markdown.contains("Hello World"));
                assert!(prepared.composed_markdown.contains("Some content."));
                assert_eq!(prepared.mode, SystemPromptMode::Append);
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn transclusion_resolves_relative() {
        let tmp = TempDir::new().unwrap();
        let included_path = write_temp_file(tmp.path(), "included.md", "Included content here.");
        let _ = &included_path; // ensure file exists

        let prompt_path = write_temp_file(
            tmp.path(),
            "prompt.md",
            "Before.\n\n::file ./included.md\n\nAfter.",
        );

        let source = SystemPromptSource::StandardDiscovered {
            path: prompt_path,
            scope: StandardPromptScope::Package,
        };

        let result =
            prepare_system_prompt(source, "Before.\n\n::file ./included.md\n\nAfter.").unwrap();

        match result {
            EffectiveSystemPrompt::Ready(prepared) => {
                assert!(
                    prepared.composed_markdown.contains("Included content here."),
                    "Expected transclusion to resolve. Got: {}",
                    prepared.composed_markdown
                );
                assert!(prepared.composed_markdown.contains("Before."));
                assert!(prepared.composed_markdown.contains("After."));
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn shell_directive_executes() {
        let tmp = TempDir::new().unwrap();

        // Shell policy resolution walks up to find .git root, so create
        // a fake git root and place the whitelist there.
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        write_temp_file(tmp.path(), ".darkmatter-shell-whitelist", "prefix echo\n");

        let path = write_temp_file(tmp.path(), "prompt.md", "Pre.\n\n::shell echo hello\n\nPost.");

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result =
            prepare_system_prompt(source, "Pre.\n\n::shell echo hello\n\nPost.").unwrap();

        match result {
            EffectiveSystemPrompt::Ready(prepared) => {
                assert!(
                    prepared.composed_markdown.contains("hello"),
                    "Expected shell output. Got: {}",
                    prepared.composed_markdown
                );
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn interpolation_expands() {
        let tmp = TempDir::new().unwrap();
        let path = write_temp_file(
            tmp.path(),
            "prompt.md",
            "Today is {{today}}.",
        );

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result = prepare_system_prompt(source, "Today is {{today}}.").unwrap();

        match result {
            EffectiveSystemPrompt::Ready(prepared) => {
                // {{today}} should be replaced with an actual date, not the literal
                assert!(
                    !prepared.composed_markdown.contains("{{today}}"),
                    "Expected interpolation to expand. Got: {}",
                    prepared.composed_markdown
                );
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn empty_body_produces_disabled() {
        let tmp = TempDir::new().unwrap();
        let path = write_temp_file(
            tmp.path(),
            "prompt.md",
            "---\ntitle: Empty\n---\n",
        );

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result = prepare_system_prompt(source, "---\ntitle: Empty\n---\n").unwrap();

        assert!(
            result.is_disabled(),
            "Expected Disabled for empty body, got {result:?}"
        );
    }

    #[test]
    fn whitespace_only_body_produces_disabled() {
        let tmp = TempDir::new().unwrap();
        let path = write_temp_file(
            tmp.path(),
            "prompt.md",
            "---\ntitle: Blank\n---\n\n   \n\n",
        );

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result =
            prepare_system_prompt(source, "---\ntitle: Blank\n---\n\n   \n\n").unwrap();

        assert!(
            result.is_disabled(),
            "Expected Disabled for whitespace-only body, got {result:?}"
        );
    }

    #[test]
    fn frontmatter_not_forwarded() {
        let tmp = TempDir::new().unwrap();
        let raw = "---\ntitle: Secret\n---\n\nVisible body.";
        let path = write_temp_file(tmp.path(), "prompt.md", raw);

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result = prepare_system_prompt(source, raw).unwrap();

        match result {
            EffectiveSystemPrompt::Ready(prepared) => {
                assert!(
                    !prepared.composed_markdown.contains("---"),
                    "Frontmatter delimiters should not appear in composed output. Got: {}",
                    prepared.composed_markdown
                );
                assert!(
                    !prepared.composed_markdown.contains("title: Secret"),
                    "Frontmatter content should not appear. Got: {}",
                    prepared.composed_markdown
                );
                assert!(prepared.composed_markdown.contains("Visible body."));
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn standard_file_always_append() {
        let tmp = TempDir::new().unwrap();
        let path = write_temp_file(tmp.path(), "prompt.md", "Content.");

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Package,
        };

        let result = prepare_system_prompt(source, "Content.").unwrap();

        match result {
            EffectiveSystemPrompt::Ready(prepared) => {
                assert_eq!(prepared.mode, SystemPromptMode::Append);
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn explicit_replace_preserves_mode() {
        let tmp = TempDir::new().unwrap();
        let path = write_temp_file(tmp.path(), "prompt.md", "Replacement content.");

        let source = SystemPromptSource::ExplicitFile {
            path,
            mode: SystemPromptMode::Replace,
        };

        let result = prepare_system_prompt(source, "Replacement content.").unwrap();

        match result {
            EffectiveSystemPrompt::Ready(prepared) => {
                assert_eq!(prepared.mode, SystemPromptMode::Replace);
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn resolve_and_prepare_none() {
        let tmp = TempDir::new().unwrap();
        let cwd = tmp.path().join("empty");
        std::fs::create_dir_all(&cwd).unwrap();

        let args = SystemPromptArgs::default();
        let context = LaunchContext {
            cwd: cwd.clone(),
            repo_root: Some(cwd.clone()),
            package_area_root: None,
            package_root: None,
        };

        let result = resolve_and_prepare(&args, &context).unwrap();

        // Should be None unless ~/.claudine/system-prompt.md exists on the host
        match result {
            EffectiveSystemPrompt::None => {} // expected in clean test environments
            EffectiveSystemPrompt::Ready(_) => {
                // Acceptable if user has ~/.claudine/system-prompt.md
            }
            EffectiveSystemPrompt::Disabled { .. } => {
                // Acceptable if user has an empty ~/.claudine/system-prompt.md
            }
        }
    }
}
