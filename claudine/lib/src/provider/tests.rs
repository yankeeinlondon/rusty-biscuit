//! Exhaustiveness invariants for the central provider registry.
//!
//! These tests pin the four invariants that prevent provider drift across
//! the centralized catalog scaffolding:
//!
//! 1. Every variant in [`PROVIDERS_DISPLAY_ORDER`] resolves to a
//!    [`ProviderInfo`] whose `provider` field matches the lookup key.
//! 2. Every [`ProviderInfo`] field is non-empty / non-default for cross-
//!    cutting fields that must always be populated.
//! 3. The [`AiCli`](sniff::programs::AiCli) sniff binding round-trips
//!    through the existing `Provider::sniff_ai_cli()` implementation.
//! 4. The behavior trait objects are non-null (i.e. populated to a real
//!    static value, not accidentally aliased).

use sniff::programs::AiCli;

use super::identity::{Provider, PROVIDERS_DISPLAY_ORDER};
use super::registry::{all_providers, provider_info};

#[test]
fn registry_round_trip() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        assert_eq!(
            info.provider, provider,
            "provider_info({provider:?}) returned mismatched provider {:?}",
            info.provider
        );
    }
}

#[test]
fn registry_field_touch_coverage() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        assert!(
            !info.display_name.is_empty(),
            "{provider:?}: display_name is empty"
        );
        assert!(!info.slug.is_empty(), "{provider:?}: slug is empty");
        assert!(!info.binary.is_empty(), "{provider:?}: binary is empty");
        assert!(
            !info.agent_offset.is_empty(),
            "{provider:?}: agent_offset is empty"
        );
        assert!(
            info.agent_offset.starts_with('.'),
            "{provider:?}: agent_offset must begin with '.', got {:?}",
            info.agent_offset
        );
        assert!(
            !info.cli_aliases.is_empty(),
            "{provider:?}: cli_aliases is empty"
        );
        assert!(
            !info.docs_url.is_empty(),
            "{provider:?}: docs_url is empty"
        );
        // Behavior trait objects should be populated (they are required
        // fields with no `Option` wrapping).
        let _ = info.behavior.detect_from_payload(&serde_json::Value::Null);
        let _ = info.mcp.supported();
        let _ = info.adapter.detect(&serde_json::Value::Null);
        let _ = info.configurator.hooks_supported();
    }
}

#[test]
fn registry_sniff_binding_round_trip() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        let from_legacy: AiCli = provider.sniff_ai_cli();
        assert_eq!(
            info.sniff_binding, from_legacy,
            "{provider:?}: registry sniff_binding {:?} does not match Provider::sniff_ai_cli() {:?}",
            info.sniff_binding, from_legacy
        );
    }
}

#[test]
fn registry_static_facts_match_legacy() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let info = provider_info(provider);
        assert_eq!(
            info.display_name,
            provider.to_string(),
            "{provider:?}: display_name mismatch"
        );
        assert_eq!(
            info.slug,
            provider.as_slug(),
            "{provider:?}: slug mismatch"
        );
        assert_eq!(
            info.agent_offset,
            provider.agent_offset(),
            "{provider:?}: agent_offset mismatch"
        );
        assert_eq!(
            info.cli_aliases,
            provider.cli_aliases(),
            "{provider:?}: cli_aliases mismatch"
        );
        assert_eq!(
            info.docs_url,
            provider.docs_url(),
            "{provider:?}: docs_url mismatch"
        );
        assert_eq!(
            info.usage_dashboard_url,
            provider.usage_dashboard_url(),
            "{provider:?}: usage_dashboard_url mismatch"
        );
        assert_eq!(
            info.supports_skills,
            provider.supports_skills(),
            "{provider:?}: supports_skills mismatch"
        );
    }
}

#[test]
fn all_providers_iterates_in_display_order() {
    let collected: Vec<Provider> = all_providers().map(|info| info.provider).collect();
    assert_eq!(collected, PROVIDERS_DISPLAY_ORDER.to_vec());
}

#[test]
fn provider_info_serializes_round_trip() {
    let info = provider_info(Provider::Claude);
    let json = serde_json::to_value(info).expect("provider_info serializes");
    assert_eq!(json["provider"], serde_json::json!("claude"));
    assert_eq!(json["slug"], serde_json::json!("claude"));
    assert_eq!(json["display_name"], serde_json::json!("Claude"));
}
