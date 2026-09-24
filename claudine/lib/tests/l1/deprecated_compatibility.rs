//! Compile-time + runtime guard for the deprecated compatibility re-exports
//! restored in Phase 1 of the review-3 closure plan.
//!
//! Spec §"Migration Plan" and design §6.1 require that the previously public
//! `events::Provider`, `events::PROVIDERS_DISPLAY_ORDER`, and
//! `events::EventSupportLevel` surfaces survive one
//! release cycle as `#[deprecated]` re-exports. If any of those re-exports
//! is removed prematurely, the import lines in this test file fail to
//! compile and the build breaks immediately.
//!
//! The `agents::AgentId` alias covered here previously was deleted with the
//! `agents` module at AgentCapabilities retirement (design/module-split.md).

#![allow(deprecated)]

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
