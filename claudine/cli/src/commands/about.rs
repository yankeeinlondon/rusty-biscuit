use color_eyre::eyre::Result;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::terminal::{TerminalOptions, for_terminal};

use crate::log;

const ABOUT_TEXT: &str = r#"# Claudine

Cross-agent hook/event system for agentic CLIs. Configure once, use everywhere.

## Supported Providers

| Provider      | Hook Support | Notes                            |
|---------------|--------------|----------------------------------|
| Claude Code   | ✅ Full      | Config-based hooks via hooks.md  |
| Codex CLI     | ✅ Partial   | turn_complete + permission hooks |
| Gemini CLI    | ✅ Full      | Config-based via settings.json   |
| OpenCode      | ✅ Full      | Plugin + experimental hooks      |
| Goose         | ⛔️ Non-hook  | Requires wrapper (not yet impl)  |
| Kimi Code     | ⛔️ Non-hook  | Requires wire-mode proxy         |
| Qwen Code     | ❌ None      | No hook support currently        |

Run `claudine hooks --support` for the full event/provider matrix.

## Events

| Event             | Description                                      |
|-------------------|--------------------------------------------------|
| session_start     | Agent session started, resumed, or cleared       |
| session_end       | Agent session ended or terminated                |
| before_prompt     | User prompt submitted, before processing         |
| before_tool       | Tool call created, before execution              |
| after_tool        | Tool call completed successfully                 |
| tool_error        | Tool call failed with an error                   |
| permission_request| Agent requesting user permission                 |
| turn_complete     | Agent turn (request/response) completed          |
| turn_error        | Agent turn failed with an error                  |
| subagent_start    | Sub-agent spawned                                |
| subagent_stop     | Sub-agent finished                               |
| before_model      | Before sending prompt to the model               |
| after_model       | After receiving response from the model          |
| before_compact    | Before context compaction/summarization          |
| notification      | Provider-specific notification or status         |

## Actions

Actions execute when events fire. Multiple actions per event supported.

- **SoundEffect** --- Play audio via playa (53 built-in effects)
- **Speak** --- Text-to-speech via system TTS
- **Log** --- Write JSONL to file or POST to HTTP endpoint
- **Report** --- Formatted output to stdout
- **Run** --- Execute shell command (blocking or async)

## Skill Linking

Share skills and commands across all your AI agents:

    claudine link              # Symlink skills to all providers
    claudine link --dry-run    # Preview without changes

Links skills from `~/.claudine/skills/` to each provider's config directory,
so Claude, Codex, Gemini, and OpenCode all share the same knowledge base.

## Configuration

Config file: `~/.hooker` (JSON)

    claudine init              # Interactive setup wizard
    claudine sync              # Re-sync hooks with providers
    claudine hooks             # View registered hooks
    claudine providers         # Skill/slash/agent/hook support table

## Testing

    claudine dry-run <event>   # Test with mock payload
    claudine dry-run tool_error --provider codex

## More Information

    claudine --help            # All commands
    claudine <cmd> --help      # Command-specific help
    claudine hooks --describe  # Event schemas and details
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
