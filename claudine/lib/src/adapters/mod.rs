pub(crate) mod antigravity;
pub(crate) mod claude;
pub(crate) mod codex;
pub(crate) mod gemini;
pub(crate) mod goose;
pub(crate) mod kimicode;
pub(crate) mod opencode;
pub(crate) mod pi;
pub(crate) mod qwen;

use serde_json::Value;

use crate::actions::HookResponse;
use crate::events::{AgenticEvent, EventMeta};
use crate::protect::decision::ProtectDecision;
use crate::provider::Provider;

/// Adapter-level parse/format errors.
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    /// Required field was missing from payload.
    #[error("missing required field: {0}")]
    MissingField(&'static str),

    /// Payload field had unexpected type.
    #[error("invalid field type for `{field}`; expected {expected}")]
    InvalidFieldType {
        /// JSON field name.
        field: &'static str,
        /// Expected JSON type.
        expected: &'static str,
    },

    /// Event name is not recognized for this provider.
    #[error("unknown event `{0}`")]
    UnknownEvent(String),

    /// Invalid response for this provider/event.
    #[error("response not supported for provider={provider:?}, event={event:?}")]
    ResponseNotSupported {
        /// Provider.
        provider: Provider,
        /// Event.
        event: AgenticEvent,
    },

    /// Generic adapter parse/format failure.
    #[error("adapter error: {0}")]
    Message(String),
}

/// Trait for provider-specific event adapters.
pub trait ProviderAdapter: Send + Sync {
    /// Which provider this adapter handles.
    fn provider(&self) -> Provider;

    /// Parse raw provider JSON into normalized event + metadata.
    fn parse_event(
        &self,
        raw: &Value,
    ) -> std::result::Result<(AgenticEvent, EventMeta), AdapterError>;

    /// Whether this provider/event pair supports blocking response semantics.
    fn can_block(&self, event: &AgenticEvent) -> bool;

    /// Acknowledgment payload for non-blocking events.
    ///
    /// Providers that read hook stdout (Claude, Gemini, Kimi, OpenCode, Qwen)
    /// need a JSON acknowledgment even for non-blocking events like SessionEnd,
    /// otherwise they interpret silent stdout as "hook cancelled."
    ///
    /// Fire-and-forget providers (Codex, Goose) return `None`.
    ///
    /// Default: `Some(json!({}))` — a safe empty JSON object.
    fn non_blocking_ack(&self) -> Option<Value> {
        Some(Value::Object(Default::default()))
    }

    /// Convert unified hook response into provider-native response payload.
    fn format_response(
        &self,
        event: &AgenticEvent,
        response: &HookResponse,
    ) -> std::result::Result<Value, AdapterError>;

    /// Exit code to use for shell-driven providers.
    fn exit_code(&self, event: &AgenticEvent, response: &HookResponse) -> Option<i32>;

    /// Map a protect decision into a generic hook response for this provider.
    fn map_protect_outcome(
        &self,
        _event: &AgenticEvent,
        decision: &ProtectDecision,
    ) -> std::result::Result<HookResponse, AdapterError> {
        if decision.is_blocked() {
            let reason = decision
                .blocked
                .as_ref()
                .map(|m| format!("{}: {}", m.group, m.rule_id));
            Ok(HookResponse {
                decision: Some(crate::actions::HookDecision::Deny),
                reason,
                ..HookResponse::default()
            })
        } else {
            Ok(HookResponse::default())
        }
    }
}

pub(crate) static ANTIGRAVITY_ADAPTER: antigravity::AntigravityAdapter =
    antigravity::AntigravityAdapter;
pub(crate) static CLAUDE_ADAPTER: claude::ClaudeAdapter = claude::ClaudeAdapter;
pub(crate) static CODEX_ADAPTER: codex::CodexAdapter = codex::CodexAdapter;
pub(crate) static GEMINI_ADAPTER: gemini::GeminiAdapter = gemini::GeminiAdapter;
pub(crate) static GOOSE_ADAPTER: goose::GooseAdapter = goose::GooseAdapter;
pub(crate) static KIMI_ADAPTER: kimicode::KimiCodeAdapter = kimicode::KimiCodeAdapter;
pub(crate) static OPENCODE_ADAPTER: opencode::OpenCodeAdapter = opencode::OpenCodeAdapter;
pub(crate) static PI_ADAPTER: pi::PiAdapter = pi::PiAdapter;
pub(crate) static QWEN_ADAPTER: qwen::QwenAdapter = qwen::QwenAdapter;

/// Returns the adapter singleton for a provider.
pub fn adapter_for(provider: Provider) -> &'static dyn ProviderAdapter {
    crate::provider::provider_info(provider)
        .adapter
        .provider_adapter()
}

pub(crate) fn str_field(raw: &Value, key: &str) -> Option<String> {
    raw.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn raw_for_native(provider: Provider, native_name: &str) -> Value {
        match provider {
            Provider::Claude => json!({ "hook_event_name": native_name }),
            Provider::Gemini => json!({ "hook_event_name": native_name }),
            Provider::OpenCode => json!({ "event_type": native_name }),
            _ => json!({ "event": native_name }),
        }
    }

    #[test]
    fn adapter_factory_returns_matching_provider() {
        let providers = [
            Provider::Claude,
            Provider::Codex,
            Provider::Gemini,
            Provider::Goose,
            Provider::KimiCode,
            Provider::OpenCode,
            Provider::QwenCode,
        ];

        for provider in providers {
            let adapter = adapter_for(provider);
            assert_eq!(adapter.provider(), provider);
        }
    }

    #[test]
    fn blockable_events_are_supported_by_provider() {
        for provider in [
            Provider::Claude,
            Provider::Codex,
            Provider::Gemini,
            Provider::Goose,
            Provider::KimiCode,
            Provider::OpenCode,
            Provider::QwenCode,
        ] {
            let adapter = adapter_for(provider);
            for event in crate::events::AgenticEvent::ALL {
                if adapter.can_block(&event) {
                    assert!(
                        provider.supports_event(&event),
                        "adapter for {provider} marks {event} as blockable but provider does not support it"
                    );
                }
            }
        }
    }

    #[test]
    fn protect_outcome_mapping_allow_and_block() {
        use crate::protect::catalog::{RuleGroup, ScanSurface};
        use crate::protect::decision::ProtectMatch;

        let fixture_events = [
            AgenticEvent::BeforeTool,
            AgenticEvent::AfterTool,
            AgenticEvent::SubagentStop,
            AgenticEvent::TurnComplete,
        ];

        let fixture_decisions = [
            ProtectDecision::allow(),
            ProtectDecision::blocked(ProtectMatch {
                group: RuleGroup::FilesystemDestruction,
                rule_id: "rm_recursive_force".to_string(),
                pattern: r"rm\s+-rf".to_string(),
                matched_text: "rm -rf /".to_string(),
                surface: ScanSurface::BashCommand,
                target_path: None,
                config_key: "protect.rules.filesystem_destruction".to_string(),
            }),
        ];

        for provider in [
            Provider::Claude,
            Provider::Codex,
            Provider::Gemini,
            Provider::Goose,
            Provider::KimiCode,
            Provider::OpenCode,
            Provider::QwenCode,
        ] {
            let adapter = adapter_for(provider);
            for event in fixture_events {
                for decision in &fixture_decisions {
                    let mapped = adapter
                        .map_protect_outcome(&event, decision)
                        .expect("protect mapping should succeed");
                    let _ = adapter
                        .format_response(&event, &mapped)
                        .expect("mapped protect response should be format-compatible");
                }
            }
        }
    }

    #[test]
    fn shared_native_mappings_are_hook_supported() {
        use crate::provider::provider_info;

        for provider in [Provider::Claude, Provider::Gemini, Provider::OpenCode] {
            for mapping in provider_info(provider).event_mapping.registration_targets() {
                assert!(
                    provider.supports_event_via_hook(&mapping.event),
                    "shared mapping marks {provider}/{:?} but provider is not hook-supported",
                    mapping.event
                );
                assert_eq!(
                    provider_info(provider)
                        .event_mapping
                        .registration_native_name(mapping.event),
                    mapping.support_level.native_name(),
                    "shared registration mapping mismatch for {provider}/{:?}",
                    mapping.event
                );
            }
        }
    }

    #[test]
    fn adapters_conform_to_shared_native_mappings() {
        use crate::provider::provider_info;

        for provider in [Provider::Claude, Provider::Gemini, Provider::OpenCode] {
            let adapter = adapter_for(provider);

            for mapping in provider_info(provider).event_mapping.registration_targets() {
                for alias in mapping.parse_aliases {
                    let raw = raw_for_native(provider, alias);
                    let (event, _) = adapter.parse_event(&raw).unwrap_or_else(|err| {
                        panic!(
                            "adapter parse failed for provider={provider}, alias={alias}, expected={:?}: {err}",
                            mapping.event
                        )
                    });

                    assert_eq!(
                        event, mapping.event,
                        "adapter mapping mismatch for provider={provider}, alias={alias}"
                    );
                }
            }
        }
    }
}
