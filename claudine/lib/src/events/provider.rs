use serde::{Deserialize, Serialize};
use std::fmt;

/// Level of event support for a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSupportLevel {
    /// Event is supported via native hooks (config-file based).
    ///
    /// These events can be registered by modifying the provider's config file.
    /// Examples: Claude hooks, Gemini hooks, OpenCode plugins.
    Hook,

    /// Event is supported via non-hook methods (wrapper/wire-mode/stream parsing).
    ///
    /// These events require alternative capture methods that are not yet implemented:
    /// - **Wrapper scripts**: Intercept CLI invocation (Goose GOOSE_STATUS_HOOK)
    /// - **Wire mode proxy**: JSON-RPC interception (Kimi Code --wire)
    /// - **Stream parsing**: Parse CLI output (Codex JSONL, Qwen stream-json)
    NonHook,

    /// Event is not supported by this provider.
    NotSupported,
}

impl EventSupportLevel {
    /// Returns whether this level indicates any form of support.
    pub fn is_supported(&self) -> bool {
        !matches!(self, EventSupportLevel::NotSupported)
    }

    /// Returns whether this level indicates hook-based support.
    pub fn is_hook(&self) -> bool {
        matches!(self, EventSupportLevel::Hook)
    }
}

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
    /// Qwen Code CLI (Alibaba).
    QwenCode,
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
            Provider::QwenCode => "qwen_code",
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
    /// | QwenCode   | ✓      | `~/.qwen/skills/`, `.qwen/skills/` (experimental)       |
    pub fn supports_skills(&self) -> bool {
        matches!(
            self,
            Provider::Claude
                | Provider::Codex
                | Provider::Gemini
                | Provider::OpenCode
                | Provider::QwenCode
        )
    }

    /// Returns the documentation URL for this provider.
    pub fn docs_url(&self) -> &'static str {
        match self {
            Provider::Claude => "https://docs.anthropic.com/en/docs/claude-code",
            Provider::Codex => "https://github.com/openai/codex",
            Provider::Gemini => "https://github.com/google-gemini/gemini-cli",
            Provider::Goose => "https://block.github.io/goose/",
            Provider::KimiCode => "https://moonshotai.github.io/kimi-cli/en/",
            Provider::OpenCode => "https://github.com/opencode-ai/opencode",
            Provider::QwenCode => "https://qwenlm.github.io/qwen-code-docs/",
        }
    }

    /// Returns the level of support for the given event.
    ///
    /// ## Support Levels
    ///
    /// - `Hook`: Event can be registered via config file modification
    /// - `NonHook`: Event requires wrapper/proxy (not yet implemented)
    /// - `NotSupported`: Event is not available from this provider
    ///
    /// ## Provider Capabilities
    ///
    /// | Provider   | Hook Method                | Non-Hook Method              |
    /// |------------|----------------------------|------------------------------|
    /// | Claude     | settings.json hooks        | -                            |
    /// | Gemini     | settings.json hooks        | -                            |
    /// | OpenCode   | opencode.json plugins      | -                            |
    /// | Codex      | config.toml notify         | JSONL stream parsing         |
    /// | Goose      | -                          | stream-json + env var        |
    /// | Kimi Code  | -                          | Wire mode JSON-RPC proxy     |
    /// | Qwen Code  | -                          | stream-json parsing          |
    pub fn event_support_level(&self, event: &super::AgenticEvent) -> EventSupportLevel {
        use super::AgenticEvent::*;
        use EventSupportLevel::*;

        match self {
            // Claude: All supported events use native hooks
            Provider::Claude => match event {
                BeforeModel | AfterModel | TurnError => NotSupported,
                _ => Hook,
            },

            // Gemini: All supported events use native hooks
            Provider::Gemini => match event {
                ToolError | PermissionRequest | TurnError | SubagentStart | SubagentStop => {
                    NotSupported
                }
                _ => Hook,
            },

            // OpenCode: All supported events use plugin hooks
            Provider::OpenCode => match event {
                ToolError | SubagentStart | SubagentStop => NotSupported,
                _ => Hook,
            },

            // Codex: Only turn_complete via notify config, rest via JSONL stream
            Provider::Codex => match event {
                TurnComplete => Hook,
                SessionEnd | PermissionRequest | SubagentStart | SubagentStop | BeforeModel
                | BeforeCompact => NotSupported,
                _ => NonHook, // SessionStart, BeforePrompt, BeforeTool, AfterTool, ToolError, TurnError, AfterModel, Notification
            },

            // Goose: All events via stream-json/env var (no config hooks)
            Provider::Goose => match event {
                SessionStart | SessionEnd | BeforePrompt | BeforeTool | AfterTool | ToolError
                | PermissionRequest | BeforeModel | BeforeCompact => NotSupported,
                _ => NonHook, // TurnComplete, TurnError, AfterModel, Notification, SubagentStart, SubagentStop
            },

            // Kimi Code: All events via wire mode JSON-RPC (requires proxy)
            Provider::KimiCode => match event {
                SessionStart | SessionEnd | BeforeModel => NotSupported,
                _ => NonHook, // All other events via wire mode
            },

            // Qwen Code: Limited events via stream-json output
            Provider::QwenCode => match event {
                TurnComplete | TurnError | AfterModel | Notification => NonHook,
                _ => NotSupported,
            },
        }
    }

    /// Returns whether this provider supports the given event (via any method).
    ///
    /// This includes both hook-based support and non-hook methods (wrapper/stream).
    /// Use [`event_support_level`](Self::event_support_level) to distinguish between them.
    ///
    /// Events that return `false` cannot be captured from this provider at all.
    pub fn supports_event(&self, event: &super::AgenticEvent) -> bool {
        self.event_support_level(event).is_supported()
    }

    /// Returns whether this provider supports the given event via native hooks.
    ///
    /// Only events with hook-based support can be registered via config file
    /// modification. Events that return `false` either require non-hook methods
    /// (wrapper/proxy) or are not supported at all.
    pub fn supports_event_via_hook(&self, event: &super::AgenticEvent) -> bool {
        self.event_support_level(event).is_hook()
    }

    /// Returns the native event name used by this provider for the given event.
    ///
    /// Returns `None` if the provider doesn't support the event.
    /// Returns `Some("")` (empty string) if supported but no specific native name.
    pub fn native_event_name(&self, event: &super::AgenticEvent) -> Option<&'static str> {
        use super::AgenticEvent::*;

        if !self.supports_event(event) {
            return None;
        }

        Some(match self {
            Provider::Claude => match event {
                SessionStart => "PreToolUse",
                SessionEnd => "",
                BeforePrompt => "PreToolUse",
                BeforeTool => "PreToolUse",
                AfterTool => "PostToolUse",
                ToolError => "PostToolUse",
                PermissionRequest => "PreToolUse",
                TurnComplete => "Stop",
                TurnError => "",
                SubagentStart => "PreToolUse",
                SubagentStop => "PostToolUse",
                BeforeModel => "",
                AfterModel => "",
                BeforeCompact => "",
                Notification => "Notification",
            },
            Provider::Codex => match event {
                SessionStart => "thread.started",
                SessionEnd => "",
                BeforePrompt => "turn.started",
                BeforeTool => "item.started",
                AfterTool => "item.completed",
                ToolError => "error",
                PermissionRequest => "",
                TurnComplete => "turn.completed",
                TurnError => "turn.failed",
                SubagentStart => "",
                SubagentStop => "",
                BeforeModel => "",
                AfterModel => "agent_message",
                BeforeCompact => "",
                Notification => "reasoning",
            },
            Provider::Gemini => match event {
                SessionStart => "SessionStart",
                SessionEnd => "SessionEnd",
                BeforePrompt => "BeforeAgent",
                BeforeTool => "BeforeTool",
                AfterTool => "AfterTool",
                ToolError => "",
                PermissionRequest => "",
                TurnComplete => "AfterAgent",
                TurnError => "",
                SubagentStart => "",
                SubagentStop => "",
                BeforeModel => "BeforeModel",
                AfterModel => "AfterModel",
                BeforeCompact => "PreCompress",
                Notification => "Notification",
            },
            Provider::Goose => match event {
                SessionStart => "",
                SessionEnd => "",
                BeforePrompt => "",
                BeforeTool => "",
                AfterTool => "",
                ToolError => "",
                PermissionRequest => "",
                TurnComplete => "complete",
                TurnError => "error",
                SubagentStart => "subagent_tool_request",
                SubagentStop => "tasks_complete",
                BeforeModel => "",
                AfterModel => "message",
                BeforeCompact => "",
                Notification => "notification",
            },
            Provider::KimiCode => match event {
                SessionStart => "",
                SessionEnd => "",
                BeforePrompt => "TurnBegin",
                BeforeTool => "ToolCall",
                AfterTool => "ToolResult",
                ToolError => "ToolResult",
                PermissionRequest => "ApprovalRequest",
                TurnComplete => "TurnEnd",
                TurnError => "prompt.status",
                SubagentStart => "SubagentEvent",
                SubagentStop => "SubagentEvent",
                BeforeModel => "",
                AfterModel => "ContentPart",
                BeforeCompact => "CompactionBegin",
                Notification => "StatusUpdate",
            },
            Provider::OpenCode => match event {
                SessionStart => "session.created",
                SessionEnd => "session.deleted",
                BeforePrompt => "chat.message",
                BeforeTool => "tool.execute.before",
                AfterTool => "tool.execute.after",
                ToolError => "",
                PermissionRequest => "permission.ask",
                TurnComplete => "session.idle",
                TurnError => "session.error",
                SubagentStart => "",
                SubagentStop => "",
                BeforeModel => "chat.params",
                AfterModel => "message.part.updated",
                BeforeCompact => "session.compacting",
                Notification => "event",
            },
            Provider::QwenCode => match event {
                // Stream-json events (headless mode)
                SessionStart => "",
                SessionEnd => "",
                BeforePrompt => "",
                BeforeTool => "",
                AfterTool => "",
                ToolError => "",
                PermissionRequest => "",
                TurnComplete => "result",
                TurnError => "result",
                SubagentStart => "",
                SubagentStop => "",
                BeforeModel => "",
                AfterModel => "assistant",
                BeforeCompact => "",
                Notification => "system",
            },
        })
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
            Provider::QwenCode => "Qwen Code",
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
            (Provider::QwenCode, "qwen_code"),
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
        assert_eq!(Provider::QwenCode.to_string(), "Qwen Code");
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
        assert!(Provider::QwenCode.supports_skills()); // Experimental but supported
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
        assert_eq!(Provider::QwenCode.as_slug(), "qwen_code");
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
            Provider::QwenCode,
        ] {
            let url = provider.docs_url();
            assert!(
                url.starts_with("https://"),
                "Provider {provider:?} URL should start with https://"
            );
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
        // Codex supports most events via JSONL stream + notify hook
        // Missing: SessionEnd, PermissionRequest, SubagentStart/Stop, BeforeModel, BeforeCompact
        assert!(Provider::Codex.supports_event(&SessionStart));
        assert!(Provider::Codex.supports_event(&TurnComplete));
        assert!(Provider::Codex.supports_event(&TurnError));
        assert!(Provider::Codex.supports_event(&BeforeTool));
        assert!(Provider::Codex.supports_event(&AfterTool));
        assert!(Provider::Codex.supports_event(&AfterModel));
        assert!(Provider::Codex.supports_event(&Notification));
        // Not supported
        assert!(!Provider::Codex.supports_event(&SessionEnd));
        assert!(!Provider::Codex.supports_event(&PermissionRequest));
        assert!(!Provider::Codex.supports_event(&SubagentStart));
        assert!(!Provider::Codex.supports_event(&SubagentStop));
        assert!(!Provider::Codex.supports_event(&BeforeModel));
        assert!(!Provider::Codex.supports_event(&BeforeCompact));
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
    fn supports_event_goose() {
        use crate::events::AgenticEvent::*;
        // Goose supports events via stream-json output and MCP notifications
        // Supported: TurnComplete, TurnError, AfterModel, Notification, SubagentStart/Stop
        assert!(Provider::Goose.supports_event(&TurnComplete));
        assert!(Provider::Goose.supports_event(&TurnError));
        assert!(Provider::Goose.supports_event(&AfterModel));
        assert!(Provider::Goose.supports_event(&Notification));
        assert!(Provider::Goose.supports_event(&SubagentStart));
        assert!(Provider::Goose.supports_event(&SubagentStop));
        // Not supported: session lifecycle, tool events, permission, BeforeModel, compaction
        assert!(!Provider::Goose.supports_event(&SessionStart));
        assert!(!Provider::Goose.supports_event(&SessionEnd));
        assert!(!Provider::Goose.supports_event(&BeforePrompt));
        assert!(!Provider::Goose.supports_event(&BeforeTool));
        assert!(!Provider::Goose.supports_event(&AfterTool));
        assert!(!Provider::Goose.supports_event(&ToolError));
        assert!(!Provider::Goose.supports_event(&PermissionRequest));
        assert!(!Provider::Goose.supports_event(&BeforeModel));
        assert!(!Provider::Goose.supports_event(&BeforeCompact));
    }

    #[test]
    fn supports_event_kimicode() {
        use crate::events::AgenticEvent::*;
        // Kimi Code Wire mode supports most events via JSON-RPC
        // Missing: SessionStart/End, BeforeModel
        assert!(Provider::KimiCode.supports_event(&TurnComplete));
        assert!(Provider::KimiCode.supports_event(&TurnError));
        assert!(Provider::KimiCode.supports_event(&BeforePrompt));
        assert!(Provider::KimiCode.supports_event(&BeforeTool));
        assert!(Provider::KimiCode.supports_event(&AfterTool));
        assert!(Provider::KimiCode.supports_event(&ToolError));
        assert!(Provider::KimiCode.supports_event(&PermissionRequest));
        assert!(Provider::KimiCode.supports_event(&SubagentStart));
        assert!(Provider::KimiCode.supports_event(&SubagentStop));
        assert!(Provider::KimiCode.supports_event(&AfterModel));
        assert!(Provider::KimiCode.supports_event(&BeforeCompact));
        // Not supported
        assert!(!Provider::KimiCode.supports_event(&SessionStart));
        assert!(!Provider::KimiCode.supports_event(&SessionEnd));
        assert!(!Provider::KimiCode.supports_event(&BeforeModel));
    }

    #[test]
    fn supports_event_opencode() {
        use crate::events::AgenticEvent::*;
        // OpenCode supports most events via plugin hooks and event system
        // Missing: ToolError (no tool.execute.error hook), SubagentStart/Stop (not intercepted)
        assert!(Provider::OpenCode.supports_event(&SessionStart));
        assert!(Provider::OpenCode.supports_event(&SessionEnd));
        assert!(Provider::OpenCode.supports_event(&BeforePrompt));
        assert!(Provider::OpenCode.supports_event(&BeforeTool));
        assert!(Provider::OpenCode.supports_event(&AfterTool));
        assert!(Provider::OpenCode.supports_event(&PermissionRequest));
        assert!(Provider::OpenCode.supports_event(&TurnComplete));
        assert!(Provider::OpenCode.supports_event(&TurnError));
        assert!(Provider::OpenCode.supports_event(&BeforeModel));
        assert!(Provider::OpenCode.supports_event(&AfterModel));
        assert!(Provider::OpenCode.supports_event(&BeforeCompact));
        assert!(Provider::OpenCode.supports_event(&Notification));
        // Not supported
        assert!(!Provider::OpenCode.supports_event(&ToolError));
        assert!(!Provider::OpenCode.supports_event(&SubagentStart));
        assert!(!Provider::OpenCode.supports_event(&SubagentStop));
    }

    #[test]
    fn supports_event_qwencode() {
        use crate::events::AgenticEvent::*;
        // Qwen Code has limited event support via stream-json output
        // Supported: TurnComplete, TurnError, AfterModel, Notification
        assert!(Provider::QwenCode.supports_event(&TurnComplete));
        assert!(Provider::QwenCode.supports_event(&TurnError));
        assert!(Provider::QwenCode.supports_event(&AfterModel));
        assert!(Provider::QwenCode.supports_event(&Notification));
        // Not supported: most events (no native hook system yet)
        assert!(!Provider::QwenCode.supports_event(&SessionStart));
        assert!(!Provider::QwenCode.supports_event(&SessionEnd));
        assert!(!Provider::QwenCode.supports_event(&BeforePrompt));
        assert!(!Provider::QwenCode.supports_event(&BeforeTool));
        assert!(!Provider::QwenCode.supports_event(&AfterTool));
        assert!(!Provider::QwenCode.supports_event(&ToolError));
        assert!(!Provider::QwenCode.supports_event(&PermissionRequest));
        assert!(!Provider::QwenCode.supports_event(&SubagentStart));
        assert!(!Provider::QwenCode.supports_event(&SubagentStop));
        assert!(!Provider::QwenCode.supports_event(&BeforeModel));
        assert!(!Provider::QwenCode.supports_event(&BeforeCompact));
    }

    #[test]
    fn event_support_level_claude_all_hook() {
        use crate::events::AgenticEvent::*;
        use super::EventSupportLevel::*;
        // Claude: all supported events are via hooks
        assert_eq!(Provider::Claude.event_support_level(&TurnComplete), Hook);
        assert_eq!(Provider::Claude.event_support_level(&BeforeTool), Hook);
        assert_eq!(
            Provider::Claude.event_support_level(&BeforeModel),
            NotSupported
        );
    }

    #[test]
    fn event_support_level_codex_mixed() {
        use crate::events::AgenticEvent::*;
        use super::EventSupportLevel::*;
        // Codex: turn_complete via hook, others via JSONL stream
        assert_eq!(Provider::Codex.event_support_level(&TurnComplete), Hook);
        assert_eq!(Provider::Codex.event_support_level(&BeforeTool), NonHook);
        assert_eq!(Provider::Codex.event_support_level(&AfterTool), NonHook);
        assert_eq!(Provider::Codex.event_support_level(&SessionStart), NonHook);
        assert_eq!(
            Provider::Codex.event_support_level(&PermissionRequest),
            NotSupported
        );
    }

    #[test]
    fn event_support_level_goose_all_non_hook() {
        use crate::events::AgenticEvent::*;
        use super::EventSupportLevel::*;
        // Goose: all supported events via stream-json/env var
        assert_eq!(Provider::Goose.event_support_level(&TurnComplete), NonHook);
        assert_eq!(Provider::Goose.event_support_level(&Notification), NonHook);
        assert_eq!(
            Provider::Goose.event_support_level(&SessionStart),
            NotSupported
        );
        assert_eq!(
            Provider::Goose.event_support_level(&BeforeTool),
            NotSupported
        );
    }

    #[test]
    fn event_support_level_kimicode_all_non_hook() {
        use crate::events::AgenticEvent::*;
        use super::EventSupportLevel::*;
        // Kimi Code: all supported events via wire mode
        assert_eq!(
            Provider::KimiCode.event_support_level(&TurnComplete),
            NonHook
        );
        assert_eq!(Provider::KimiCode.event_support_level(&BeforeTool), NonHook);
        assert_eq!(
            Provider::KimiCode.event_support_level(&PermissionRequest),
            NonHook
        );
        assert_eq!(
            Provider::KimiCode.event_support_level(&SessionStart),
            NotSupported
        );
    }

    #[test]
    fn event_support_level_qwencode_all_non_hook() {
        use crate::events::AgenticEvent::*;
        use super::EventSupportLevel::*;
        // Qwen Code: limited events via stream-json
        assert_eq!(
            Provider::QwenCode.event_support_level(&TurnComplete),
            NonHook
        );
        assert_eq!(Provider::QwenCode.event_support_level(&AfterModel), NonHook);
        assert_eq!(
            Provider::QwenCode.event_support_level(&BeforeTool),
            NotSupported
        );
    }

    #[test]
    fn supports_event_via_hook() {
        use crate::events::AgenticEvent::*;
        // Hook-based providers
        assert!(Provider::Claude.supports_event_via_hook(&TurnComplete));
        assert!(Provider::Gemini.supports_event_via_hook(&TurnComplete));
        assert!(Provider::OpenCode.supports_event_via_hook(&TurnComplete));
        assert!(Provider::Codex.supports_event_via_hook(&TurnComplete));

        // Codex: only turn_complete via hook
        assert!(!Provider::Codex.supports_event_via_hook(&BeforeTool));

        // Non-hook providers
        assert!(!Provider::Goose.supports_event_via_hook(&TurnComplete));
        assert!(!Provider::KimiCode.supports_event_via_hook(&TurnComplete));
        assert!(!Provider::QwenCode.supports_event_via_hook(&TurnComplete));
    }
}
