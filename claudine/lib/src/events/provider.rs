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
}
