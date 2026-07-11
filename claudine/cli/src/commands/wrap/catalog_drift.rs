//! Wrapper-side model-catalog drift emission
//! (design/model-catalog-boundary.md §"Runtime model_catalog migration"
//! step 2: the dynamic listing demotes to a drift channel; set
//! differences emit a `ModelCatalogDrift` signal).
//!
//! Both entry points are cheap and infallible on the spawn path: pure
//! comparisons over the compiled baseline, one on-disk listing-cache
//! read, and a best-effort config load — no subprocess, no network.

use claudine::model_catalog::{ModelCatalogService, listing_drift, resolved_model_drift};
use claudine::provider::{ModelCatalogSource, Provider, provider_info};
use claudine::signals::{SignalHub, SignalSource};

/// Overrides-aware catalog service for drift comparison.
///
/// Config load is best-effort: a missing or broken config yields a
/// service without overrides (the same fallback
/// `composition::load_selection_config` applies on the target path).
fn drift_catalog_service() -> ModelCatalogService {
    match claudine::dispatch::loader::load_claudine_config(None, None) {
        Ok(config) => ModelCatalogService::from_config(&config),
        Err(_) => ModelCatalogService::new(),
    }
}

/// Once-per-run listing drift check for dynamically-listed providers
/// (`ModelCatalogSource::ShellCommand`; only OpenCode today).
///
/// Reads the drift-channel listing cache only — `None` (no cache written
/// yet) skips the check for this run; the background refresh will have
/// populated it for the next.
pub(crate) fn emit_listing_drift(provider: Provider, hub: &SignalHub) {
    if !matches!(
        provider_info(provider).model_catalog_source,
        ModelCatalogSource::ShellCommand { .. }
    ) {
        return;
    }
    let service = drift_catalog_service();
    let Some(listing) = service.cached_listing(provider) else {
        return;
    };
    if let Some(event) = listing_drift(provider, &listing, &service) {
        hub.emit_bespoke(event, SignalSource::Wrapper);
    }
}

/// End-of-run resolved-model drift: one check per stream-observed
/// `ModelResolved` id, run before the hub drains so any drift event
/// rides this run's `ProcessResult.signals`.
///
/// No-ops when nothing resolved or on hub-less fallback runs (no
/// provider attribution means no baseline to compare against).
pub(crate) fn emit_resolved_model_drift(hub: &SignalHub) {
    let resolved = hub.resolved_models();
    if resolved.is_empty() {
        return;
    }
    let Some(provider) = hub.provider() else {
        return;
    };
    let service = drift_catalog_service();
    for id in resolved {
        if let Some(event) = resolved_model_drift(provider, &id, &service) {
            hub.emit_bespoke(event, SignalSource::Wrapper);
        }
    }
}
