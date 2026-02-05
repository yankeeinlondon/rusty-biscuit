use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported agentic CLI providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    /// Claude Code (Anthropic).
    Claude,
    /// Codex CLI (OpenAI).
    Codex,
    /// Gemini CLI (Google).
    Gemini,
    /// Goose (Block).
    Goose,
    /// Kimi Code CLI (Moonshot AI).
    KimiCode,
    /// OpenCode.
    OpenCode,
}

impl Provider {
    /// Returns a snake_case identifier suitable for file paths and JSON keys.
    ///
    /// Use this for file system paths, config keys, and anywhere that needs a
    /// stable, machine-readable identifier. Use `Display` for user-facing output.
    pub fn as_slug(&self) -> &'static str {
        match self {
            Provider::Claude => "claude",
            Provider::Codex => "codex",
            Provider::Gemini => "gemini",
            Provider::Goose => "goose",
            Provider::KimiCode => "kimi_code",
            Provider::OpenCode => "open_code",
        }
    }

    /// Returns whether this provider supports skill discovery.
    ///
    /// Skills are directory bundles with a `SKILL.md` entry point that the
    /// agentic CLI automatically discovers and activates based on context.
    ///
    /// | Provider   | Skills | Entry Path(s)                                           |
    /// |------------|:------:|---------------------------------------------------------|
    /// | Claude     | ✓      | `~/.claude/skills/`, `.claude/skills/`                  |
    /// | Codex      | ✓      | `~/.codex/skills/`, `.codex/skills/`                    |
    /// | Gemini     | ✓      | `~/.gemini/skills/`, `.gemini/skills/`                  |
    /// | Goose      | ✗      | N/A                                                     |
    /// | KimiCode   | ✗      | N/A                                                     |
    /// | OpenCode   | ✓      | `~/.config/opencode/skills/`, `.opencode/skills/`       |
    pub fn supports_skills(&self) -> bool {
        matches!(
            self,
            Provider::Claude | Provider::Codex | Provider::Gemini | Provider::OpenCode
        )
    }

    /// Returns the documentation URL for this provider.
    pub fn docs_url(&self) -> &'static str {
        match self {
            Provider::Claude => "https://docs.anthropic.com/en/docs/claude-code",
            Provider::Codex => "https://github.com/openai/codex",
            Provider::Gemini => "https://github.com/google-gemini/gemini-cli",
            Provider::Goose => "https://block.github.io/goose/",
            Provider::KimiCode => "https://github.com/aspect-build/aspect-cli",
            Provider::OpenCode => "https://github.com/opencode-ai/opencode",
        }
    }

    /// Returns whether this provider supports the given event natively.
    ///
    /// Events that are not supported cannot be registered with the provider's
    /// hook system and will be skipped during sync.
    pub fn supports_event(&self, event: &super::AgenticEvent) -> bool {
        use super::AgenticEvent::*;
        match self {
            Provider::Claude => !matches!(event, BeforeModel | AfterModel | TurnError),
            Provider::Codex => matches!(event, TurnComplete),
            Provider::Gemini => !matches!(
                event,
                ToolError | PermissionRequest | TurnError | SubagentStart | SubagentStop
            ),
            Provider::Goose => false, // No hook support yet
            Provider::KimiCode => false, // No hook support yet
            Provider::OpenCode => {
                !matches!(event, ToolError | SubagentStart | SubagentStop | AfterModel)
            }
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Provider::Claude => "Claude",
            Provider::Codex => "Codex",
            Provider::Gemini => "Gemini",
            Provider::Goose => "Goose",
            Provider::KimiCode => "Kimi Code",
            Provider::OpenCode => "OpenCode",
        };
        f.write_str(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_json() {
        let provider = Provider::Claude;
        let json = serde_json::to_value(&provider).unwrap();
        assert_eq!(json, serde_json::json!("claude"));
        let back: Provider = serde_json::from_value(json).unwrap();
        assert_eq!(back, Provider::Claude);
    }

    #[test]
    fn all_variants_serialize_snake_case() {
        let cases = vec![
            (Provider::Claude, "claude"),
            (Provider::Codex, "codex"),
            (Provider::Gemini, "gemini"),
            (Provider::Goose, "goose"),
            (Provider::KimiCode, "kimi_code"),
            (Provider::OpenCode, "open_code"),
        ];
        for (variant, expected) in cases {
            let json = serde_json::to_value(&variant).unwrap();
            assert_eq!(json.as_str().unwrap(), expected, "Failed for {variant:?}");
        }
    }

    #[test]
    fn display_uses_friendly_names() {
        assert_eq!(Provider::Claude.to_string(), "Claude");
        assert_eq!(Provider::Codex.to_string(), "Codex");
        assert_eq!(Provider::Gemini.to_string(), "Gemini");
        assert_eq!(Provider::Goose.to_string(), "Goose");
        assert_eq!(Provider::KimiCode.to_string(), "Kimi Code");
        assert_eq!(Provider::OpenCode.to_string(), "OpenCode");
    }

    #[test]
    fn can_use_as_hashmap_key() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(Provider::Claude, "test");
        assert_eq!(map.get(&Provider::Claude), Some(&"test"));
    }

    #[test]
    fn supports_skills() {
        // Providers with native skill discovery
        assert!(Provider::Claude.supports_skills());
        assert!(Provider::Codex.supports_skills());
        assert!(Provider::Gemini.supports_skills());
        assert!(Provider::OpenCode.supports_skills());
        // Providers without skill discovery
        assert!(!Provider::Goose.supports_skills());
        assert!(!Provider::KimiCode.supports_skills());
    }

    #[test]
    fn as_slug_returns_snake_case() {
        assert_eq!(Provider::Claude.as_slug(), "claude");
        assert_eq!(Provider::Codex.as_slug(), "codex");
        assert_eq!(Provider::Gemini.as_slug(), "gemini");
        assert_eq!(Provider::Goose.as_slug(), "goose");
        assert_eq!(Provider::KimiCode.as_slug(), "kimi_code");
        assert_eq!(Provider::OpenCode.as_slug(), "open_code");
    }

    #[test]
    fn docs_url_returns_valid_urls() {
        // All providers should return HTTPS URLs
        for provider in [
            Provider::Claude,
            Provider::Codex,
            Provider::Gemini,
            Provider::Goose,
            Provider::KimiCode,
            Provider::OpenCode,
        ] {
            let url = provider.docs_url();
            assert!(url.starts_with("https://"), "Provider {provider:?} URL should start with https://");
        }
    }

    #[test]
    fn supports_event_claude() {
        use crate::events::AgenticEvent::*;
        // Claude supports most events except BeforeModel, AfterModel, TurnError
        assert!(Provider::Claude.supports_event(&SessionStart));
        assert!(Provider::Claude.supports_event(&TurnComplete));
        assert!(Provider::Claude.supports_event(&ToolError));
        assert!(!Provider::Claude.supports_event(&BeforeModel));
        assert!(!Provider::Claude.supports_event(&AfterModel));
        assert!(!Provider::Claude.supports_event(&TurnError));
    }

    #[test]
    fn supports_event_codex() {
        use crate::events::AgenticEvent::*;
        // Codex only supports TurnComplete
        assert!(Provider::Codex.supports_event(&TurnComplete));
        assert!(!Provider::Codex.supports_event(&SessionStart));
        assert!(!Provider::Codex.supports_event(&BeforeTool));
    }

    #[test]
    fn supports_event_gemini() {
        use crate::events::AgenticEvent::*;
        // Gemini doesn't support ToolError, PermissionRequest, TurnError, SubagentStart, SubagentStop
        assert!(Provider::Gemini.supports_event(&SessionStart));
        assert!(Provider::Gemini.supports_event(&BeforeModel));
        assert!(!Provider::Gemini.supports_event(&ToolError));
        assert!(!Provider::Gemini.supports_event(&PermissionRequest));
        assert!(!Provider::Gemini.supports_event(&SubagentStart));
    }

    #[test]
    fn supports_event_goose_kimicode_no_hooks() {
        use crate::events::AgenticEvent::*;
        // Goose and KimiCode have no hook support
        assert!(!Provider::Goose.supports_event(&SessionStart));
        assert!(!Provider::Goose.supports_event(&TurnComplete));
        assert!(!Provider::KimiCode.supports_event(&SessionStart));
        assert!(!Provider::KimiCode.supports_event(&TurnComplete));
    }
}
