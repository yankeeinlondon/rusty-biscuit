//! Methods on the canonical [`Provider`] enum.
//!
//! This module hosts the `impl Provider` blocks and the [`Display`] impl.
//! Phase 2 of the centralized providers refactor (Review 1) eliminated all
//! lib-side `match Provider { ... }` dispatch from this module: every
//! identity helper now reads its data straight off
//! [`provider_info`](super::registry::provider_info), so the only authoritative
//! `match Provider` site in the lib crate is the central registry.

use serde_json::Value;
use sniff::programs::AiCli;
use std::fmt;

use super::event_mapping::EventSupportLevel;
use super::identity::{PROVIDERS_DISPLAY_ORDER, Provider};
use super::registry::provider_info;
use crate::events::AgenticEvent;

impl Provider {
    /// Returns common CLI aliases accepted for this provider.
    pub fn cli_aliases(&self) -> &'static [&'static str] {
        provider_info(*self).cli_aliases
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

        if let Some(provider) = Self::parse_cli_name(&normalized) {
            return vec![provider];
        }

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
        provider_info(*self).sniff_binding
    }

    /// Detect a provider from raw payload shape.
    ///
    /// Walks providers in canonical display order and asks each provider's
    /// adapter behavior to recognize the payload. The first hit wins; this
    /// keeps detection table-driven rather than re-implementing payload
    /// shape rules in a per-variant `match` here.
    pub fn detect_from_payload(raw: &Value) -> Option<Self> {
        // Provider adapters get first refusal — they own the most specific
        // shape detection (e.g. Codex's payload-shape heuristics).
        for provider in PROVIDERS_DISPLAY_ORDER {
            if provider_info(provider).adapter.detect(raw) {
                return Some(provider);
            }
        }

        // Cross-cutting fallbacks for ambiguous shapes that match multiple
        // providers (e.g. Claude vs. Gemini both use `hook_event_name`).
        // These are JSON-shape checks, not provider-typed dispatch.
        if let Some(name) = raw.get("hook_event_name").and_then(Value::as_str) {
            let gemini_table = provider_info(Provider::Gemini).event_mapping;
            let claude_table = provider_info(Provider::Claude).event_mapping;
            let is_gemini = gemini_table.event_from_native_name(name).is_some()
                && claude_table.event_from_native_name(name).is_none();
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
        provider_info(*self).slug
    }

    /// Returns whether this provider supports skill discovery.
    pub fn supports_skills(&self) -> bool {
        provider_info(*self).supports_skills
    }

    /// Returns the documentation URL for this provider.
    pub fn docs_url(&self) -> &'static str {
        provider_info(*self).docs_url
    }

    /// Returns the usage/billing dashboard URL for this provider, if one exists.
    pub fn usage_dashboard_url(&self) -> Option<&'static str> {
        provider_info(*self).usage_dashboard_url
    }

    /// Returns the native name used by configurators for hook registration.
    pub(crate) fn registration_native_event_name(
        &self,
        event: &AgenticEvent,
    ) -> Option<&'static str> {
        provider_info(*self)
            .event_mapping
            .registration_native_name(*event)
    }

    /// Maps a provider-native event name back to a canonical `AgenticEvent`.
    pub(crate) fn event_from_shared_native_name(&self, native_name: &str) -> Option<AgenticEvent> {
        provider_info(*self)
            .event_mapping
            .event_from_native_name(native_name)
    }

    /// Returns the level of support for the given event.
    pub fn event_support_level(&self, event: &AgenticEvent) -> EventSupportLevel {
        provider_info(*self).event_mapping.support_level(*event)
    }

    /// Returns whether this provider supports the given event (via any method).
    pub fn supports_event(&self, event: &AgenticEvent) -> bool {
        self.event_support_level(event).is_supported()
    }

    /// Returns whether this provider supports the given event via native hooks.
    pub fn supports_event_via_hook(&self, event: &AgenticEvent) -> bool {
        self.event_support_level(event).is_hook()
    }

    /// Returns the native event name used by this provider for the given event.
    pub fn native_event_name(&self, event: &AgenticEvent) -> Option<&'static str> {
        provider_info(*self).event_mapping.native_name(*event)
    }

    /// Returns the agent offset directory name for this provider.
    pub fn agent_offset(&self) -> &'static str {
        provider_info(*self).agent_offset
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
        f.write_str(provider_info(*self).display_name)
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
        assert!(Provider::Claude.supports_skills());
        assert!(Provider::Codex.supports_skills());
        assert!(Provider::Gemini.supports_skills());
        assert!(Provider::OpenCode.supports_skills());
        assert!(Provider::QwenCode.supports_skills());
        assert!(Provider::RooCode.supports_skills());
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
        assert!(Provider::Codex.supports_event(&SessionStart));
        assert!(Provider::Codex.supports_event(&TurnComplete));
        assert!(Provider::Codex.supports_event(&TurnError));
        assert!(Provider::Codex.supports_event(&BeforeTool));
        assert!(Provider::Codex.supports_event(&AfterTool));
        assert!(Provider::Codex.supports_event(&AfterModel));
        assert!(Provider::Codex.supports_event(&Notification));
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
        assert!(Provider::Gemini.supports_event(&SessionStart));
        assert!(Provider::Gemini.supports_event(&BeforeModel));
        assert!(!Provider::Gemini.supports_event(&ToolError));
        assert!(!Provider::Gemini.supports_event(&PermissionRequest));
        assert!(!Provider::Gemini.supports_event(&SubagentStart));
    }

    #[test]
    fn supports_event_goose() {
        use crate::events::AgenticEvent::*;
        assert!(Provider::Goose.supports_event(&TurnComplete));
        assert!(Provider::Goose.supports_event(&TurnError));
        assert!(Provider::Goose.supports_event(&AfterModel));
        assert!(Provider::Goose.supports_event(&Notification));
        assert!(Provider::Goose.supports_event(&SubagentStart));
        assert!(Provider::Goose.supports_event(&SubagentStop));
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
        assert!(!Provider::KimiCode.supports_event(&SessionStart));
        assert!(!Provider::KimiCode.supports_event(&SessionEnd));
        assert!(!Provider::KimiCode.supports_event(&BeforeModel));
    }

    #[test]
    fn supports_event_opencode() {
        use crate::events::AgenticEvent::*;
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
        assert!(!Provider::OpenCode.supports_event(&ToolError));
        assert!(!Provider::OpenCode.supports_event(&SubagentStart));
        assert!(!Provider::OpenCode.supports_event(&SubagentStop));
    }

    #[test]
    fn supports_event_qwencode() {
        use crate::events::AgenticEvent::*;
        assert!(Provider::QwenCode.supports_event(&TurnComplete));
        assert!(Provider::QwenCode.supports_event(&TurnError));
        assert!(Provider::QwenCode.supports_event(&AfterModel));
        assert!(Provider::QwenCode.supports_event(&Notification));
        assert!(Provider::QwenCode.supports_event(&PermissionRequest));
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
        use crate::events::AgenticEvent::*;
        assert!(
            Provider::Claude
                .event_support_level(&TurnComplete)
                .is_hook()
        );
        assert!(Provider::Claude.event_support_level(&BeforeTool).is_hook());
        assert!(
            !Provider::Claude
                .event_support_level(&BeforeModel)
                .is_supported()
        );
    }

    #[test]
    fn event_support_level_codex_mixed() {
        use crate::events::AgenticEvent::*;
        assert!(Provider::Codex.event_support_level(&TurnComplete).is_hook());
        assert!(
            Provider::Codex
                .event_support_level(&BeforeTool)
                .is_supported()
                && !Provider::Codex.event_support_level(&BeforeTool).is_hook()
        );
        assert!(
            Provider::Codex
                .event_support_level(&AfterTool)
                .is_supported()
                && !Provider::Codex.event_support_level(&AfterTool).is_hook()
        );
        assert!(
            Provider::Codex
                .event_support_level(&SessionStart)
                .is_supported()
                && !Provider::Codex.event_support_level(&SessionStart).is_hook()
        );
        assert!(
            !Provider::Codex
                .event_support_level(&PermissionRequest)
                .is_supported()
        );
    }

    #[test]
    fn event_support_level_goose_all_non_hook() {
        use crate::events::AgenticEvent::*;
        assert!(
            Provider::Goose
                .event_support_level(&TurnComplete)
                .is_supported()
                && !Provider::Goose.event_support_level(&TurnComplete).is_hook()
        );
        assert!(
            Provider::Goose
                .event_support_level(&Notification)
                .is_supported()
                && !Provider::Goose.event_support_level(&Notification).is_hook()
        );
        assert!(
            !Provider::Goose
                .event_support_level(&SessionStart)
                .is_supported()
        );
        assert!(
            !Provider::Goose
                .event_support_level(&BeforeTool)
                .is_supported()
        );
    }

    #[test]
    fn event_support_level_kimicode_all_non_hook() {
        use crate::events::AgenticEvent::*;
        assert!(
            Provider::KimiCode
                .event_support_level(&TurnComplete)
                .is_supported()
                && !Provider::KimiCode
                    .event_support_level(&TurnComplete)
                    .is_hook()
        );
        assert!(
            Provider::KimiCode
                .event_support_level(&BeforeTool)
                .is_supported()
                && !Provider::KimiCode
                    .event_support_level(&BeforeTool)
                    .is_hook()
        );
        assert!(
            Provider::KimiCode
                .event_support_level(&PermissionRequest)
                .is_acp()
        );
        assert!(
            !Provider::KimiCode
                .event_support_level(&SessionStart)
                .is_supported()
        );
    }

    #[test]
    fn event_support_level_qwencode_all_non_hook() {
        use crate::events::AgenticEvent::*;
        assert!(
            Provider::QwenCode
                .event_support_level(&TurnComplete)
                .is_supported()
                && !Provider::QwenCode
                    .event_support_level(&TurnComplete)
                    .is_hook()
        );
        assert!(
            Provider::QwenCode
                .event_support_level(&AfterModel)
                .is_supported()
                && !Provider::QwenCode
                    .event_support_level(&AfterModel)
                    .is_hook()
        );
        assert!(
            Provider::QwenCode
                .event_support_level(&PermissionRequest)
                .is_supported()
                && !Provider::QwenCode
                    .event_support_level(&PermissionRequest)
                    .is_hook()
        );
        assert!(
            !Provider::QwenCode
                .event_support_level(&BeforeTool)
                .is_supported()
        );
    }

    #[test]
    fn event_support_level_roocode_all_non_hook() {
        use crate::events::AgenticEvent::*;

        assert!(
            Provider::RooCode
                .event_support_level(&TurnComplete)
                .is_supported()
                && !Provider::RooCode
                    .event_support_level(&TurnComplete)
                    .is_hook()
        );
        assert!(
            Provider::RooCode
                .event_support_level(&BeforeTool)
                .is_supported()
                && !Provider::RooCode.event_support_level(&BeforeTool).is_hook()
        );
        assert!(
            !Provider::RooCode
                .event_support_level(&BeforePrompt)
                .is_supported()
        );
    }

    #[test]
    fn supports_event_via_hook() {
        use crate::events::AgenticEvent::*;
        assert!(Provider::Claude.supports_event_via_hook(&TurnComplete));
        assert!(Provider::Gemini.supports_event_via_hook(&TurnComplete));
        assert!(Provider::OpenCode.supports_event_via_hook(&TurnComplete));
        assert!(Provider::Codex.supports_event_via_hook(&TurnComplete));

        assert!(!Provider::Codex.supports_event_via_hook(&BeforeTool));

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

        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"SessionStart"})),
            Some(Provider::Claude)
        );
        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"hook_event_name":"Notification"})),
            Some(Provider::Claude)
        );

        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"event_name":"BeforeAgent"})),
            Some(Provider::Gemini)
        );

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

        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"event_type":"session.idle"})),
            Some(Provider::OpenCode)
        );

        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"method":"notification"})),
            Some(Provider::KimiCode)
        );

        assert_eq!(
            Provider::detect_from_payload(&serde_json::json!({"unknown":true})),
            None
        );
    }
}
