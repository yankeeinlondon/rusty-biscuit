mod claude;
mod codex;
mod gemini;
mod goose;
mod kimicode;
mod opencode;
mod qwen;
mod roo;

use serde_json::Value;

use crate::actions::HookResponse;
use crate::events::{AgenticEvent, EventMeta, Provider};
use crate::services::{
    ProtectDecision, ProtectObservation, ProtectOutcome, ProviderProtectCapabilities,
    ProviderProtectProfiles,
};
use crate::services::protect::observe::default_observe_protect;

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
    /// Fire-and-forget providers (Codex, Goose, Roo) return `None`.
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

    /// Extract a protect observation from an event.
    ///
    /// The default implementation mirrors the legacy `ProtectInput::from_event_meta()`
    /// logic but produces `ProtectObservation` with `ProtectIntent` variants.
    /// Provider-specific adapters can override for more precise intent extraction.
    fn observe_protect(
        &self,
        event: &AgenticEvent,
        meta: &EventMeta,
    ) -> Option<ProtectObservation> {
        default_observe_protect(event, meta)
    }

    /// Provider protect capability handshake.
    fn protect_capabilities(&self) -> ProviderProtectCapabilities {
        ProviderProtectProfiles::defaults().capabilities(self.provider())
    }

    /// Map a protect decision into a generic hook response for this provider.
    fn map_protect_outcome(
        &self,
        _event: &AgenticEvent,
        decision: &ProtectDecision,
    ) -> std::result::Result<HookResponse, AdapterError> {
        let reason = match &decision.outcome {
            ProtectOutcome::Allow => None,
            ProtectOutcome::AskThenAllowOrStop { reason }
            | ProtectOutcome::StopCurrent { reason }
            | ProtectOutcome::StopSession { reason }
            | ProtectOutcome::AllowWithRedaction { reason }
            | ProtectOutcome::AdvisoryOnly { reason } => Some(reason.clone()),
        };

        let response = match decision.outcome {
            ProtectOutcome::Allow | ProtectOutcome::AllowWithRedaction { .. } => HookResponse {
                decision: Some(crate::actions::HookDecision::Allow),
                reason,
                ..HookResponse::default()
            },
            ProtectOutcome::AskThenAllowOrStop { .. } => HookResponse {
                decision: Some(crate::actions::HookDecision::Ask),
                reason,
                ..HookResponse::default()
            },
            ProtectOutcome::StopCurrent { .. } | ProtectOutcome::StopSession { .. } => {
                HookResponse {
                    decision: Some(crate::actions::HookDecision::Deny),
                    reason,
                    ..HookResponse::default()
                }
            }
            ProtectOutcome::AdvisoryOnly { .. } => HookResponse {
                decision: Some(crate::actions::HookDecision::Continue),
                reason,
                ..HookResponse::default()
            },
        };

        Ok(response)
    }
}

static CLAUDE_ADAPTER: claude::ClaudeAdapter = claude::ClaudeAdapter;
static CODEX_ADAPTER: codex::CodexAdapter = codex::CodexAdapter;
static GEMINI_ADAPTER: gemini::GeminiAdapter = gemini::GeminiAdapter;
static GOOSE_ADAPTER: goose::GooseAdapter = goose::GooseAdapter;
static KIMI_ADAPTER: kimicode::KimiCodeAdapter = kimicode::KimiCodeAdapter;
static OPENCODE_ADAPTER: opencode::OpenCodeAdapter = opencode::OpenCodeAdapter;
static QWEN_ADAPTER: qwen::QwenAdapter = qwen::QwenAdapter;
static ROO_ADAPTER: roo::RooAdapter = roo::RooAdapter;

/// Returns the adapter singleton for a provider.
pub fn adapter_for(provider: Provider) -> &'static dyn ProviderAdapter {
    match provider {
        Provider::Claude => &CLAUDE_ADAPTER,
        Provider::Codex => &CODEX_ADAPTER,
        Provider::Gemini => &GEMINI_ADAPTER,
        Provider::Goose => &GOOSE_ADAPTER,
        Provider::KimiCode => &KIMI_ADAPTER,
        Provider::OpenCode => &OPENCODE_ADAPTER,
        Provider::QwenCode => &QWEN_ADAPTER,
        Provider::RooCode => &ROO_ADAPTER,
    }
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
            Provider::RooCode,
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
            Provider::RooCode,
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
    fn protect_capabilities_align_with_default_profiles() {
        let profiles = ProviderProtectProfiles::defaults();
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
            let adapter = adapter_for(provider);
            assert_eq!(
                adapter.protect_capabilities(),
                profiles.capabilities(provider),
                "protect capability mismatch for provider={provider}"
            );
        }
    }

    #[test]
    fn protect_outcome_mapping_fixtures_cover_subagent_and_mcp_paths() {
        let fixture_events = [
            AgenticEvent::BeforeTool,
            AgenticEvent::AfterTool,    // MCP-adjacent post-tool path
            AgenticEvent::SubagentStop, // subagent completion path
            AgenticEvent::TurnComplete,
        ];

        let fixture_decisions = [
            ProtectDecision {
                outcome: ProtectOutcome::Allow,
                desired_outcome: ProtectOutcome::Allow,
                degraded: false,
                reason: "fixture.allow".to_string(),
                capability: None,
            },
            ProtectDecision {
                outcome: ProtectOutcome::AskThenAllowOrStop {
                    reason: "fixture.ask".to_string(),
                },
                desired_outcome: ProtectOutcome::AskThenAllowOrStop {
                    reason: "fixture.ask".to_string(),
                },
                degraded: false,
                reason: "fixture.ask".to_string(),
                capability: None,
            },
            ProtectDecision {
                outcome: ProtectOutcome::StopCurrent {
                    reason: "fixture.stop".to_string(),
                },
                desired_outcome: ProtectOutcome::StopCurrent {
                    reason: "fixture.stop".to_string(),
                },
                degraded: false,
                reason: "fixture.stop".to_string(),
                capability: None,
            },
            ProtectDecision {
                outcome: ProtectOutcome::AdvisoryOnly {
                    reason: "fixture.advisory".to_string(),
                },
                desired_outcome: ProtectOutcome::StopSession {
                    reason: "fixture.original".to_string(),
                },
                degraded: true,
                reason: "fixture.advisory".to_string(),
                capability: None,
            },
        ];

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
        for provider in [Provider::Claude, Provider::Gemini, Provider::OpenCode] {
            for mapping in provider.shared_native_mappings() {
                assert!(
                    provider.supports_event_via_hook(&mapping.event),
                    "shared mapping marks {provider}/{:?} but provider is not hook-supported",
                    mapping.event
                );
                assert_eq!(
                    provider.registration_native_event_name(&mapping.event),
                    Some(mapping.native_name),
                    "shared registration mapping mismatch for {provider}/{:?}",
                    mapping.event
                );
            }
        }
    }

    #[test]
    fn adapters_conform_to_shared_native_mappings() {
        for provider in [Provider::Claude, Provider::Gemini, Provider::OpenCode] {
            let adapter = adapter_for(provider);

            for mapping in provider.shared_native_mappings() {
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
