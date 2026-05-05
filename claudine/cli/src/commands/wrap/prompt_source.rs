use color_eyre::eyre::{Result, eyre};
use std::io::IsTerminal;

use crate::log;

pub(crate) fn maybe_edit_prompt_source(
    prompt_source: super::profile::PromptSource,
    silent_requested: bool,
) -> Result<Option<super::profile::PromptSource>> {
    maybe_edit_prompt_source_with(
        prompt_source,
        silent_requested,
        std::io::stdin().is_terminal() && std::io::stdout().is_terminal(),
        darkmatter::editor::resolve_editor_command,
        darkmatter::editor::edit_text,
    )
}

pub(crate) fn maybe_edit_prompt_source_with<Resolve, Edit>(
    prompt_source: super::profile::PromptSource,
    silent_requested: bool,
    terminal_ready: bool,
    resolve_editor: Resolve,
    edit_text_fn: Edit,
) -> Result<Option<super::profile::PromptSource>>
where
    Resolve: FnOnce() -> std::result::Result<String, darkmatter::editor::EditorError>,
    Edit:
        FnOnce(&str, &str) -> std::result::Result<Option<String>, darkmatter::editor::EditorError>,
{
    if !terminal_ready || matches!(prompt_source, super::profile::PromptSource::InheritStdin) {
        return Err(eyre!("--edit requires an interactive terminal"));
    }

    let seed = match prompt_source {
        super::profile::PromptSource::None => String::new(),
        super::profile::PromptSource::Inline(text) => text,
        super::profile::PromptSource::InheritStdin => unreachable!("handled by terminal preflight"),
    };

    let editor_command = resolve_editor()?;
    let editor_name = editor_command
        .split_whitespace()
        .next()
        .unwrap_or(editor_command.as_str());

    if !silent_requested {
        log::info(&format!("opening {editor_name} for prompt..."));
    }

    match edit_text_fn(&seed, ".md")? {
        Some(edited_text) => Ok(Some(super::profile::PromptSource::Inline(edited_text))),
        None => {
            if !silent_requested {
                log::info("prompt empty; aborted");
            }
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maybe_edit_prompt_source_rejects_non_tty_sessions() {
        let err = maybe_edit_prompt_source_with(
            super::super::profile::PromptSource::Inline("seed".to_string()),
            false,
            false,
            || Ok("code".to_string()),
            |_, _| Ok(Some("edited".to_string())),
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "--edit requires an interactive terminal");
    }

    #[test]
    fn maybe_edit_prompt_source_rejects_inherited_stdin() {
        let err = maybe_edit_prompt_source_with(
            super::super::profile::PromptSource::InheritStdin,
            false,
            true,
            || Ok("code".to_string()),
            |_, _| Ok(Some("edited".to_string())),
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "--edit requires an interactive terminal");
    }

    #[test]
    fn maybe_edit_prompt_source_replaces_seed_with_edited_text() {
        let edited = maybe_edit_prompt_source_with(
            super::super::profile::PromptSource::Inline("seed prompt".to_string()),
            true,
            true,
            || Ok("code".to_string()),
            |seed, suffix| {
                assert_eq!(seed, "seed prompt");
                assert_eq!(suffix, ".md");
                Ok(Some("edited prompt".to_string()))
            },
        )
        .unwrap();

        assert_eq!(
            edited,
            Some(super::super::profile::PromptSource::Inline("edited prompt".to_string()))
        );
    }

    #[test]
    fn maybe_edit_prompt_source_uses_empty_seed_when_no_prompt_exists() {
        let edited = maybe_edit_prompt_source_with(
            super::super::profile::PromptSource::None,
            true,
            true,
            || Ok("code".to_string()),
            |seed, suffix| {
                assert_eq!(seed, "");
                assert_eq!(suffix, ".md");
                Ok(Some("typed from scratch".to_string()))
            },
        )
        .unwrap();

        assert_eq!(
            edited,
            Some(super::super::profile::PromptSource::Inline(
                "typed from scratch".to_string()
            ))
        );
    }

    #[test]
    fn maybe_edit_prompt_source_returns_clean_abort_for_empty_buffer() {
        let edited = maybe_edit_prompt_source_with(
            super::super::profile::PromptSource::Inline("seed".to_string()),
            true,
            true,
            || Ok("code".to_string()),
            |_, _| Ok(None),
        )
        .unwrap();

        assert_eq!(edited, None);
    }
}
