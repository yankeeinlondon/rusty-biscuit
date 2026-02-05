use color_eyre::eyre::Result;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::terminal::{TerminalOptions, for_terminal};

use crate::log;

const ABOUT_TEXT: &str = r#"# Claudine

Cross-agent hook/event system for agentic CLIs.

## Supported Providers

- **Claude Code** (Anthropic) --- Full hook support via settings.json
- **Codex CLI** (OpenAI) --- Notify-based hooks via config.toml
- **Gemini CLI** (Google) --- Full hook support via settings.json
- **OpenCode** --- Plugin + experimental hooks
- **Roo Code** --- Wrapper-only (deferred)

## Event Types

session_start, session_end, before_prompt, before_tool, after_tool,
tool_error, permission_request, turn_complete, turn_error, subagent_start,
subagent_stop, before_model, after_model, before_compact, notification

## Actions

- **Speak** --- Text-to-speech via biscuit-speaks
- **Log** --- JSONL to file or HTTP POST to server
- **Report** --- Formatted output to stdout
- **SoundEffect** --- Audio feedback via playa

## Quick Start

    claudine init --quick    # Set up with defaults
    claudine status          # Check registrations
    claudine link            # Link skills across providers

## More Information

    claudine --help          # All commands
    claudine <cmd> --help    # Command-specific help
"#;

/// Render detailed help and usage information.
pub fn run() -> Result<()> {
    let md: Markdown = ABOUT_TEXT.into();
    match for_terminal(&md, TerminalOptions::default()) {
        Ok(rendered) => {
            log::output(&rendered);
        }
        Err(_) => {
            log::output(ABOUT_TEXT);
        }
    }
    Ok(())
}
