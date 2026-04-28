use serde_json::Value;
use sniff::programs::AiCli;
use std::fmt;

use super::AgenticEvent;

// Canonical Provider enum and PROVIDERS_DISPLAY_ORDER moved to
// `crate::provider` in Phase 1 of the centralized providers refactor.
// This module retains an un-deprecated re-export so existing internal
// callers compile without churn; Phase 8 will mark the re-export as
// `#[deprecated]` after consumer migration is complete.
pub use crate::provider::{Provider, PROVIDERS_DISPLAY_ORDER};

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

/// Shared provider-native event mappings used by both configurators and adapters.
///
/// This is the canonical source for event-name mappings that appear in both:
/// - `config/*` registration logic (`AgenticEvent -> native_name`)
/// - `adapters/*` parse logic (`native_name -> AgenticEvent`)
#[derive(Debug, Clone, Copy)]
pub(crate) struct SharedNativeEventMapping {
    pub(crate) event: AgenticEvent,
    pub(crate) native_name: &'static str,
    pub(crate) parse_aliases: &'static [&'static str],
}

const CLAUDE_SHARED_NATIVE_MAPPINGS: &[SharedNativeEventMapping] = &[
    SharedNativeEventMapping {
        event: AgenticEvent::SessionStart,
        native_name: "SessionStart",
        parse_aliases: &["SessionStart"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::SessionEnd,
        native_name: "SessionEnd",
        parse_aliases: &["SessionEnd"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::BeforePrompt,
        native_name: "UserPromptSubmit",
        parse_aliases: &["UserPromptSubmit"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::BeforeTool,
        native_name: "PreToolUse",
        parse_aliases: &["PreToolUse"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::AfterTool,
        native_name: "PostToolUse",
        parse_aliases: &["PostToolUse"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::ToolError,
        native_name: "PostToolUseFailure",
        parse_aliases: &["PostToolUseFailure"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::PermissionRequest,
        native_name: "PermissionRequest",
        parse_aliases: &["PermissionRequest"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::TurnComplete,
        native_name: "Stop",
        parse_aliases: &["Stop", "TeammateIdle", "TaskCompleted"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::SubagentStart,
        native_name: "SubagentStart",
        parse_aliases: &["SubagentStart"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::SubagentStop,
        native_name: "SubagentStop",
        parse_aliases: &["SubagentStop"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::BeforeCompact,
        native_name: "PreCompact",
        parse_aliases: &["PreCompact"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::Notification,
        native_name: "Notification",
        parse_aliases: &["Notification"],
    },
];

const GEMINI_SHARED_NATIVE_MAPPINGS: &[SharedNativeEventMapping] = &[
    SharedNativeEventMapping {
        event: AgenticEvent::SessionStart,
        native_name: "SessionStart",
        parse_aliases: &["SessionStart"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::SessionEnd,
        native_name: "SessionEnd",
        parse_aliases: &["SessionEnd"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::BeforePrompt,
        native_name: "BeforeAgent",
        parse_aliases: &["BeforeAgent"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::TurnComplete,
        native_name: "AfterAgent",
        parse_aliases: &["AfterAgent"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::BeforeModel,
        native_name: "BeforeModel",
        parse_aliases: &["BeforeModel", "BeforeToolSelection"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::AfterModel,
        native_name: "AfterModel",
        parse_aliases: &["AfterModel"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::BeforeTool,
        native_name: "BeforeTool",
        parse_aliases: &["BeforeTool"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::AfterTool,
        native_name: "AfterTool",
        parse_aliases: &["AfterTool"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::BeforeCompact,
        native_name: "PreCompress",
        parse_aliases: &["PreCompress"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::Notification,
        native_name: "Notification",
        parse_aliases: &["Notification"],
    },
];

const OPENCODE_SHARED_NATIVE_MAPPINGS: &[SharedNativeEventMapping] = &[
    SharedNativeEventMapping {
        event: AgenticEvent::SessionStart,
        native_name: "session.created",
        parse_aliases: &["session.created"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::SessionEnd,
        native_name: "session.deleted",
        parse_aliases: &["session.deleted"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::BeforePrompt,
        native_name: "chat.message",
        parse_aliases: &["chat.message"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::BeforeTool,
        native_name: "tool.execute.before",
        parse_aliases: &["tool.execute.before"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::AfterTool,
        native_name: "tool.execute.after",
        parse_aliases: &["tool.execute.after"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::BeforeModel,
        native_name: "chat.params",
        parse_aliases: &[
            "chat.params",
            "chat.headers",
            "experimental.chat.system.transform",
            "experimental.chat.messages.transform",
        ],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::AfterModel,
        native_name: "message.updated",
        parse_aliases: &[
            "message.updated",
            "message.part.updated",
            "experimental.text.complete",
        ],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::TurnComplete,
        native_name: "session.idle",
        parse_aliases: &["session.idle"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::TurnError,
        native_name: "session.error",
        parse_aliases: &["session.error"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::BeforeCompact,
        native_name: "session.compacted",
        parse_aliases: &[
            "session.compacted",
            "session.compacting",
            "experimental.session.compacting",
        ],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::PermissionRequest,
        native_name: "permission.ask",
        parse_aliases: &["permission.ask"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::HumanInTheLoop,
        native_name: "permission.asked",
        parse_aliases: &["permission.asked"],
    },
    SharedNativeEventMapping {
        event: AgenticEvent::Notification,
        native_name: "tui.toast.show",
        parse_aliases: &["tui.toast.show", "event"],
    },
];

impl Provider {
    /// Returns common CLI aliases accepted for this provider.
    pub fn cli_aliases(&self) -> &'static [&'static str] {
        match self {
            Provider::Claude => &["claude"],
            Provider::Codex => &["codex"],
            Provider::Gemini => &["gemini"],
            Provider::Goose => &["goose"],
            Provider::KimiCode => &["kimi", "kimicode", "kimi_code", "kimi-code"],
            Provider::OpenCode => &["opencode", "open_code", "open-code"],
            Provider::QwenCode => &["qwen", "qwencode", "qwen_code", "qwen-code"],
            Provider::RooCode => &["roo", "roocode", "roo_code", "roo-code"],
        }
    }

    /// Parse a provider from a CLI-facing name or alias.
    pub fn parse_cli_name(input: &str) -> Option<Self> {
        let normalized = normalize_provider_input(input);
        if normalized.is_empty() {
            return None;
        }

        PROVIDERS_DISPLAY_ORDER.into_iter().find(|provider| {
            provider.as_slug() == normalized
                || normalize_provider_input(&provider.to_string()) == normalized
                || provider.cli_aliases().contains(&normalized.as_str())
        })
    }

    /// Fuzzy match a provider from user input (exact/prefix/contains).
    pub fn fuzzy_match_cli_name(input: &str) -> Option<Self> {
        let normalized = normalize_provider_input(input);
        if normalized.is_empty() {
            return None;
        }

        if let Some(provider) = Self::parse_cli_name(&normalized) {
            return Some(provider);
        }

        // Prefix match
        for provider in PROVIDERS_DISPLAY_ORDER {
            let display = normalize_provider_input(&provider.to_string());
            if display.starts_with(&normalized) || provider.as_slug().starts_with(&normalized) {
                return Some(provider);
            }
            if provider
                .cli_aliases()
                .iter()
                .any(|alias| alias.starts_with(&normalized))
            {
                return Some(provider);
            }
        }

        // Contains match
        for provider in PROVIDERS_DISPLAY_ORDER {
            let display = normalize_provider_input(&provider.to_string());
            if display.contains(&normalized) || provider.as_slug().contains(&normalized) {
                return Some(provider);
            }
            if provider
                .cli_aliases()
                .iter()
                .any(|alias| alias.contains(&normalized))
            {
                return Some(provider);
            }
        }

        None
    }

    /// Fuzzy match returning all matching providers (exact/prefix/contains).
    ///
    /// Unlike `fuzzy_match_cli_name` which returns the first match, this
    /// collects all providers that match the input at the best matching tier.
    /// Exact matches are returned alone; otherwise all prefix matches; otherwise
    /// all contains matches.
    pub fn fuzzy_match_all(input: &str) -> Vec<Self> {
        let normalized = normalize_provider_input(input);
        if normalized.is_empty() {
            return Vec::new();
        }

        // Exact match
        if let Some(provider) = Self::parse_cli_name(&normalized) {
            return vec![provider];
        }

        // Prefix matches
        let mut prefix_matches = Vec::new();
        for provider in PROVIDERS_DISPLAY_ORDER {
            let display = normalize_provider_input(&provider.to_string());
            if display.starts_with(&normalized)
                || provider.as_slug().starts_with(&normalized)
                || provider
                    .cli_aliases()
                    .iter()
                    .any(|alias| alias.starts_with(&normalized))
            {
                prefix_matches.push(provider);
            }
        }
        if !prefix_matches.is_empty() {
            return prefix_matches;
        }

        // Contains matches
        let mut contains_matches = Vec::new();
        for provider in PROVIDERS_DISPLAY_ORDER {
            let display = normalize_provider_input(&provider.to_string());
            if display.contains(&normalized)
                || provider.as_slug().contains(&normalized)
                || provider
                    .cli_aliases()
                    .iter()
                    .any(|alias| alias.contains(&normalized))
            {
                contains_matches.push(provider);
            }
        }
        contains_matches
    }

    /// Returns the corresponding sniff `AiCli` variant for install detection.
    pub fn sniff_ai_cli(&self) -> AiCli {
        match self {
            Provider::Claude => AiCli::Claude,
            Provider::Codex => AiCli::Codex,
            Provider::Gemini => AiCli::GeminiCli,
            Provider::Goose => AiCli::Goose,
            Provider::KimiCode => AiCli::KimiCli,
            Provider::OpenCode => AiCli::Opencode,
            Provider::QwenCode => AiCli::QwenCli,
            Provider::RooCode => AiCli::Roo,
        }
    }

    /// Detect a provider from raw payload shape.
    pub fn detect_from_payload(raw: &Value) -> Option<Self> {
        if let Some(name) = raw.get("hook_event_name").and_then(Value::as_str) {
            // Both Claude and Gemini use `hook_event_name`, but with different
            // native event names. If the value resolves to a Gemini event but
            // NOT a Claude event, it's Gemini.
            let is_gemini = Provider::Gemini
                .event_from_shared_native_name(name)
                .is_some()
                && Provider::Claude
                    .event_from_shared_native_name(name)
                    .is_none();
            return Some(if is_gemini {
                Provider::Gemini
            } else {
                Provider::Claude
            });
        }
        if looks_like_codex_payload(raw) {
            return Some(Provider::Codex);
        }
        if raw.get("event_type").is_some() || raw.get("eventType").is_some() {
            return Some(Provider::OpenCode);
        }
        if raw.get("event_name").is_some() {
            return Some(Provider::Gemini);
        }
        if raw.get("method").is_some() {
            return Some(Provider::KimiCode);
        }
        None
    }

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
            Provider::RooCode => "roo_code",
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
                | Provider::RooCode
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
            Provider::RooCode => "https://github.com/RooVetGit/Roo-Code",
        }
    }

    /// Returns the usage/billing dashboard URL for this provider, if one exists.
    pub fn usage_dashboard_url(&self) -> Option<&'static str> {
        match self {
            Provider::Claude => Some("https://console.anthropic.com/settings/billing"),
            Provider::Codex => Some("https://platform.openai.com/usage"),
            Provider::Gemini => Some("https://aistudio.google.com/billing"),
            Provider::Goose => None,
            Provider::KimiCode => Some("https://platform.moonshot.cn/console/account"),
            Provider::OpenCode => None, // depends on upstream provider
            Provider::QwenCode => Some("https://bailian.console.aliyun.com/"),
            Provider::RooCode => None,
        }
    }

    /// Shared native mappings for providers that currently deduplicate
    /// configurator registration and adapter parsing through one source.
    pub(crate) fn shared_native_mappings(&self) -> &'static [SharedNativeEventMapping] {
        match self {
            Provider::Claude => CLAUDE_SHARED_NATIVE_MAPPINGS,
            Provider::Gemini => GEMINI_SHARED_NATIVE_MAPPINGS,
            Provider::OpenCode => OPENCODE_SHARED_NATIVE_MAPPINGS,
            _ => &[],
        }
    }

    /// Returns the native name used by configurators for hook registration.
    pub(crate) fn registration_native_event_name(
        &self,
        event: &AgenticEvent,
    ) -> Option<&'static str> {
        self.shared_native_mappings()
            .iter()
            .find(|entry| entry.event == *event)
            .map(|entry| entry.native_name)
    }

    /// Maps a provider-native event name back to a canonical `AgenticEvent`.
    pub(crate) fn event_from_shared_native_name(&self, native_name: &str) -> Option<AgenticEvent> {
        self.shared_native_mappings().iter().find_map(|entry| {
            if entry
                .parse_aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(native_name))
            {
                Some(entry.event)
            } else {
                None
            }
        })
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
            // HumanInTheLoop captured via PreToolUse hook with AskUserQuestion tool matcher
            Provider::Claude => match event {
                BeforeModel | AfterModel | TurnError => NotSupported,
                _ => Hook,
            },

            // Gemini: All supported events use native hooks
            // No native support for HumanInTheLoop
            Provider::Gemini => match event {
                ToolError | PermissionRequest | TurnError | SubagentStart | SubagentStop
                | HumanInTheLoop => NotSupported,
                _ => Hook,
            },

            // OpenCode: All supported events use plugin hooks
            // HumanInTheLoop via permission.asked hook
            Provider::OpenCode => match event {
                ToolError | SubagentStart | SubagentStop => NotSupported,
                _ => Hook,
            },

            // Codex: Only turn_complete via notify config, rest via JSONL stream
            // HumanInTheLoop via tool/requestUserInput in JSONL stream
            Provider::Codex => match event {
                TurnComplete => Hook,
                SessionEnd | PermissionRequest | SubagentStart | SubagentStop | BeforeModel
                | BeforeCompact => NotSupported,
                _ => NonHook, // SessionStart, BeforePrompt, BeforeTool, AfterTool, ToolError, TurnError, AfterModel, Notification, HumanInTheLoop
            },

            // Goose: All events via stream-json/env var (no config hooks)
            // HumanInTheLoop via request_permission in ACP stream
            Provider::Goose => match event {
                SessionStart | SessionEnd | BeforePrompt | BeforeTool | AfterTool | ToolError
                | PermissionRequest | BeforeModel | BeforeCompact => NotSupported,
                _ => NonHook, // TurnComplete, TurnError, AfterModel, Notification, SubagentStart, SubagentStop, HumanInTheLoop
            },

            // Kimi Code: All events via wire mode JSON-RPC (requires proxy)
            // HumanInTheLoop via ApprovalRequest in wire mode
            Provider::KimiCode => match event {
                SessionStart | SessionEnd | BeforeModel => NotSupported,
                _ => NonHook, // All other events via wire mode, including HumanInTheLoop
            },

            // Qwen Code: Limited events via stream-json output
            // No native support for HumanInTheLoop
            Provider::QwenCode => match event {
                TurnComplete | TurnError | AfterModel | Notification | PermissionRequest => NonHook,
                _ => NotSupported, // Including HumanInTheLoop
            },

            // Roo Code: rich event emitter + stream-json, all observational.
            Provider::RooCode => match event {
                SessionStart | SessionEnd | BeforeTool | AfterTool | ToolError | HumanInTheLoop
                | TurnComplete | TurnError | SubagentStart | SubagentStop | BeforeModel
                | AfterModel | Notification => NonHook,
                BeforePrompt | PermissionRequest | BeforeCompact => NotSupported,
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

        if let Some(name) = self.registration_native_event_name(event) {
            return Some(name);
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
                HumanInTheLoop => "PreToolUse", // AskUserQuestion tool
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
                HumanInTheLoop => "tool/requestUserInput",
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
                HumanInTheLoop => "",
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
                HumanInTheLoop => "request_permission",
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
                HumanInTheLoop => "ApprovalRequest",
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
                HumanInTheLoop => "permission.asked",
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
                PermissionRequest => "CanUseTool",
                HumanInTheLoop => "",
                TurnComplete => "result",
                TurnError => "result",
                SubagentStart => "",
                SubagentStop => "",
                BeforeModel => "",
                AfterModel => "assistant",
                BeforeCompact => "",
                Notification => "system",
            },
            Provider::RooCode => match event {
                SessionStart => "TaskCreated",
                SessionEnd => "TaskAborted",
                BeforePrompt => "",
                BeforeTool => "ToolUseOutput",
                AfterTool => "ToolResultOutput",
                ToolError => "TaskToolFailed",
                PermissionRequest => "",
                HumanInTheLoop => "WaitingForInput",
                TurnComplete => "TaskCompleted",
                TurnError => "Error",
                SubagentStart => "TaskSpawned",
                SubagentStop => "TaskDelegationCompleted",
                BeforeModel => "StreamingStarted",
                AfterModel => "StreamingEnded",
                BeforeCompact => "",
                Notification => "ModeChanged",
            },
        })
    }

    /// Returns the agent offset directory name for this provider.
    ///
    /// This is used to create shadow HOME directories for repo-scoped mode,
    /// where the agent's config is isolated to repo-specific resources.
    ///
    /// | Provider   | Agent Offset |
    /// |-----------|--------------|
    /// | Claude    | `.claude`   |
    /// | Codex     | `.codex`    |
    /// | Gemini    | `.gemini`    |
    /// | Goose     | `.goose`    |
    /// | KimiCode  | `.kimi`     |
    /// | OpenCode  | `.opencode` |
    /// | QwenCode  | `.qwen`     |
    /// | RooCode   | `.roo`      |
    pub fn agent_offset(&self) -> &'static str {
        match self {
            Provider::Claude => ".claude",
            Provider::Codex => ".codex",
            Provider::Gemini => ".gemini",
            Provider::Goose => ".goose",
            Provider::KimiCode => ".kimi",
            Provider::OpenCode => ".opencode",
            Provider::QwenCode => ".qwen",
            Provider::RooCode => ".roo",
        }
    }
}

fn looks_like_codex_payload(raw: &Value) -> bool {
    if raw.get("thread_id").is_some() || raw.get("thread-id").is_some() {
        return true;
    }

    if raw
        .get("hook_event")
        .and_then(|value| value.get("event_type"))
        .and_then(Value::as_str)
        .is_some_and(|kind| matches!(kind, "after_tool_use"))
    {
        return true;
    }

    raw.get("event_type")
        .and_then(Value::as_str)
        .is_some_and(|kind| matches!(kind, "after_tool_use"))
        || raw.get("type").and_then(Value::as_str).is_some_and(|kind| {
            matches!(
                kind,
                "agent-turn-complete"
                    | "thread.started"
                    | "turn.started"
                    | "turn.completed"
                    | "turn.failed"
                    | "item.started"
                    | "item.updated"
                    | "item.completed"
                    | "error"
            )
        })
}

fn normalize_provider_input(input: &str) -> String {
    input.trim().to_ascii_lowercase().replace([' ', '-'], "_")
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
            Provider::RooCode => "Roo Code",
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
        let json = serde_json::to_value(provider).unwrap();
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
            (Provider::RooCode, "roo_code"),
        ];
        for (variant, expected) in cases {
            let json = serde_json::to_value(variant).unwrap();
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
        assert_eq!(Provider::RooCode.to_string(), "Roo Code");
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
        assert!(Provider::RooCode.supports_skills());
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
        assert_eq!(Provider::RooCode.as_slug(), "roo_code");
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
            Provider::RooCode,
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
        // Supported: TurnComplete, TurnError, AfterModel, Notification, PermissionRequest
        assert!(Provider::QwenCode.supports_event(&TurnComplete));
        assert!(Provider::QwenCode.supports_event(&TurnError));
        assert!(Provider::QwenCode.supports_event(&AfterModel));
        assert!(Provider::QwenCode.supports_event(&Notification));
        assert!(Provider::QwenCode.supports_event(&PermissionRequest));
        // Not supported: most events (no native hook system yet)
        assert!(!Provider::QwenCode.supports_event(&SessionStart));
        assert!(!Provider::QwenCode.supports_event(&SessionEnd));
        assert!(!Provider::QwenCode.supports_event(&BeforePrompt));
        assert!(!Provider::QwenCode.supports_event(&BeforeTool));
        assert!(!Provider::QwenCode.supports_event(&AfterTool));
        assert!(!Provider::QwenCode.supports_event(&ToolError));
        assert!(!Provider::QwenCode.supports_event(&SubagentStart));
        assert!(!Provider::QwenCode.supports_event(&SubagentStop));
        assert!(!Provider::QwenCode.supports_event(&BeforeModel));
        assert!(!Provider::QwenCode.supports_event(&BeforeCompact));
    }

    #[test]
    fn supports_event_roocode() {
        use crate::events::AgenticEvent::*;

        assert!(Provider::RooCode.supports_event(&SessionStart));
        assert!(Provider::RooCode.supports_event(&BeforeTool));
        assert!(Provider::RooCode.supports_event(&AfterModel));
        assert!(Provider::RooCode.supports_event(&HumanInTheLoop));
        assert!(!Provider::RooCode.supports_event(&BeforePrompt));
        assert!(!Provider::RooCode.supports_event(&PermissionRequest));
        assert!(!Provider::RooCode.supports_event(&BeforeCompact));
    }

    #[test]
    fn event_support_level_claude_all_hook() {
        use super::EventSupportLevel::*;
        use crate::events::AgenticEvent::*;
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
        use super::EventSupportLevel::*;
        use crate::events::AgenticEvent::*;
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
        use super::EventSupportLevel::*;
        use crate::events::AgenticEvent::*;
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
        use super::EventSupportLevel::*;
        use crate::events::AgenticEvent::*;
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
        use super::EventSupportLevel::*;
        use crate::events::AgenticEvent::*;
        // Qwen Code: limited events via stream-json
        assert_eq!(
            Provider::QwenCode.event_support_level(&TurnComplete),
            NonHook
        );
        assert_eq!(Provider::QwenCode.event_support_level(&AfterModel), NonHook);
        assert_eq!(
            Provider::QwenCode.event_support_level(&PermissionRequest),
            NonHook
        );
        assert_eq!(
            Provider::QwenCode.event_support_level(&BeforeTool),
            NotSupported
        );
    }

    #[test]
    fn event_support_level_roocode_all_non_hook() {
        use super::EventSupportLevel::*;
        use crate::events::AgenticEvent::*;

        assert_eq!(
            Provider::RooCode.event_support_level(&TurnComplete),
            NonHook
        );
        assert_eq!(Provider::RooCode.event_support_level(&BeforeTool), NonHook);
        assert_eq!(
            Provider::RooCode.event_support_level(&BeforePrompt),
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
        assert!(!Provider::RooCode.supports_event_via_hook(&TurnComplete));
    }

    #[test]
    fn opencode_shared_native_mappings_cover_core_hook_events() {
        use crate::events::AgenticEvent::*;

        assert_eq!(
            Provider::OpenCode.registration_native_event_name(&BeforePrompt),
            Some("chat.message")
        );
        assert_eq!(
            Provider::OpenCode.registration_native_event_name(&BeforeTool),
            Some("tool.execute.before")
        );
        assert_eq!(
            Provider::OpenCode.registration_native_event_name(&AfterTool),
            Some("tool.execute.after")
        );
        assert_eq!(
            Provider::OpenCode.registration_native_event_name(&BeforeModel),
            Some("chat.params")
        );
        assert_eq!(
            Provider::OpenCode.registration_native_event_name(&AfterModel),
            Some("message.updated")
        );

        assert_eq!(
            Provider::OpenCode
                .event_from_shared_native_name("experimental.chat.messages.transform"),
            Some(BeforeModel)
        );
        assert_eq!(
            Provider::OpenCode.event_from_shared_native_name("experimental.text.complete"),
            Some(AfterModel)
        );
    }

    #[test]
    fn parse_cli_name_accepts_aliases() {
        assert_eq!(Provider::parse_cli_name("claude"), Some(Provider::Claude));
        assert_eq!(Provider::parse_cli_name("kimi"), Some(Provider::KimiCode));
        assert_eq!(
            Provider::parse_cli_name("open-code"),
            Some(Provider::OpenCode)
        );
        assert_eq!(
            Provider::parse_cli_name("QWEN_CODE"),
            Some(Provider::QwenCode)
        );
        assert_eq!(
            Provider::parse_cli_name("Roo Code"),
            Some(Provider::RooCode)
        );
        assert_eq!(Provider::parse_cli_name(""), None);
    }

    #[test]
    fn fuzzy_match_cli_name_supports_prefix_and_contains() {
        assert_eq!(
            Provider::fuzzy_match_cli_name("gemi"),
            Some(Provider::Gemini)
        );
        assert_eq!(
            Provider::fuzzy_match_cli_name("kimi"),
            Some(Provider::KimiCode)
        );
        assert_eq!(
            Provider::fuzzy_match_cli_name("open"),
            Some(Provider::OpenCode)
        );
        assert_eq!(Provider::fuzzy_match_cli_name("unknown"), None);
    }

    #[test]
    fn sniff_ai_cli_maps_all_providers() {
        assert_eq!(Provider::Claude.sniff_ai_cli(), AiCli::Claude);
        assert_eq!(Provider::Codex.sniff_ai_cli(), AiCli::Codex);
        assert_eq!(Provider::Gemini.sniff_ai_cli(), AiCli::GeminiCli);
        assert_eq!(Provider::Goose.sniff_ai_cli(), AiCli::Goose);
        assert_eq!(Provider::KimiCode.sniff_ai_cli(), AiCli::KimiCli);
        assert_eq!(Provider::OpenCode.sniff_ai_cli(), AiCli::Opencode);
        assert_eq!(Provider::QwenCode.sniff_ai_cli(), AiCli::QwenCli);
        assert_eq!(Provider::RooCode.sniff_ai_cli(), AiCli::Roo);
    }

    #[test]
    fn agent_offset_returns_correct_directories() {
        assert_eq!(Provider::Claude.agent_offset(), ".claude");
        assert_eq!(Provider::Codex.agent_offset(), ".codex");
        assert_eq!(Provider::Gemini.agent_offset(), ".gemini");
        assert_eq!(Provider::Goose.agent_offset(), ".goose");
        assert_eq!(Provider::KimiCode.agent_offset(), ".kimi");
        assert_eq!(Provider::OpenCode.agent_offset(), ".opencode");
        assert_eq!(Provider::QwenCode.agent_offset(), ".qwen");
        assert_eq!(Provider::RooCode.agent_offset(), ".roo");
    }

    #[test]
    fn detect_from_payload_recognizes_known_shapes() {
        // Claude-only native names
        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"Stop"})),
            Some(Provider::Claude)
        );
        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"PreToolUse"})),
            Some(Provider::Claude)
        );
        assert_eq!(
            Provider::detect_from_payload(
                &serde_json::json!({"hook_event_name":"UserPromptSubmit"})
            ),
            Some(Provider::Claude)
        );

        // Gemini-only native names via hook_event_name
        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"BeforeAgent"})),
            Some(Provider::Gemini)
        );
        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"AfterAgent"})),
            Some(Provider::Gemini)
        );
        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"BeforeModel"})),
            Some(Provider::Gemini)
        );
        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"BeforeTool"})),
            Some(Provider::Gemini)
        );
        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"AfterTool"})),
            Some(Provider::Gemini)
        );

        // Ambiguous names (both Claude and Gemini) default to Claude
        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"SessionStart"})),
            Some(Provider::Claude)
        );
        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"Notification"})),
            Some(Provider::Claude)
        );

        // Gemini via event_name field
        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"event_name":"BeforeAgent"})),
            Some(Provider::Gemini)
        );

        // Codex
        assert_eq!(
            Provider::detect_from_payload(
                &serde_json::json!({"type":"turn.completed","thread_id":"t-1"})
            ),
            Some(Provider::Codex)
        );
        assert_eq!(
            Provider::detect_from_payload(
                &serde_json::json!({"type":"agent-turn-complete","thread-id":"t-1"})
            ),
            Some(Provider::Codex)
        );
        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({
                "session_id":"ses_123",
                "hook_event":{"event_type":"after_tool_use"}
            })),
            Some(Provider::Codex)
        );

        // OpenCode
        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"event_type":"session.idle"})),
            Some(Provider::OpenCode)
        );

        // KimiCode
        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"method":"notification"})),
            Some(Provider::KimiCode)
        );

        // Unknown
        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"unknown":true})),
            None
        );
    }
}
