//! Compile-time + runtime guard for the deprecated compatibility re-exports
//! restored in Phase 1 of the review-3 closure plan.
//!
//! Spec §"Migration Plan" and design §6.1 require that the previously public
//! `events::Provider`, `events::PROVIDERS_DISPLAY_ORDER`,
//! `events::EventSupportLevel`, and `agents::AgentId` surfaces survive one
//! release cycle as `#[deprecated]` re-exports. If any of those re-exports
//! is removed prematurely, the import lines in this test file fail to
//! compile and the build breaks immediately.

#![allow(deprecated)]

use claudine::agents::AgentId as DeprecatedAgentId;
use claudine::events::{
    EventSupportLevel as DeprecatedEventSupportLevel,
    PROVIDERS_DISPLAY_ORDER as DEPRECATED_DISPLAY_ORDER, Provider as DeprecatedEventsProvider,
};
use claudine::provider::{
    EventSupportLevel, PROVIDERS_DISPLAY_ORDER as PROVIDER_DISPLAY_ORDER, Provider,
};

#[test]
fn events_provider_re_export_matches_canonical() {
    let canonical: Provider = Provider::Claude;
    let deprecated: DeprecatedEventsProvider = canonical;
    assert_eq!(canonical, deprecated);
}

#[test]
fn events_providers_display_order_re_export_matches() {
    assert_eq!(DEPRECATED_DISPLAY_ORDER.len(), PROVIDER_DISPLAY_ORDER.len());
    for (deprecated, canonical) in DEPRECATED_DISPLAY_ORDER
        .iter()
        .zip(PROVIDER_DISPLAY_ORDER.iter())
    {
        assert_eq!(deprecated, canonical);
    }
}

#[test]
fn events_event_support_level_re_export_compiles() {
    // Exercise the type alias by constructing a value through the canonical
    // path and storing it through the deprecated alias. This forces the
    // compiler to verify the two paths name the same type.
    let value: DeprecatedEventSupportLevel = EventSupportLevel::NotSupported;
    assert!(!value.is_supported());
}

#[test]
fn agents_agent_id_alias_is_provider() {
    let id: DeprecatedAgentId = Provider::Codex;
    assert_eq!(id, Provider::Codex);
}

#[test]
fn agents_agent_id_round_trips_through_canonical_provider() {
    // Drive every catalog provider through the deprecated alias to ensure
    // the alias remains a complete substitute, not a partial one.
    for provider in PROVIDER_DISPLAY_ORDER {
        let aliased: DeprecatedAgentId = provider;
        let recovered: Provider = aliased;
        assert_eq!(provider, recovered);
    }
}
