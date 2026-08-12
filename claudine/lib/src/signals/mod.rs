//! Signal-detection tables compiled by `claudine-gen` from the signals
//! research corpus (`docs/research/signals/`).
//!
//! Alongside the compiled tables this module hosts the generic detection
//! engine ([`SignalEngine`]), the typed-event builder, the dedup sink
//! ([`SignalSink`]), the per-run multi-producer fan-in ([`SignalHub`]),
//! consumer-side projections ([`project`]), and the opt-in unmatched-event
//! harvest ([`harvest`]). Roster-only providers (kilo, pi) compile "dormant":
//! their tables are reachable via [`detection_table`] by slug only — no
//! [`Provider`] variant exists for them — which is the path the
//! fixture-replay `signals check` uses.

mod bespoke;
mod engine;
mod event_builder;
mod generated;
pub mod harvest;
mod hub;
mod path;
pub mod project;
mod sink;
mod version;

pub use claudine_catalog_types::{
    CapScope, DetectionMode, DetectionRecord, DriftObservation, ExtractStrategy, ExtractionSpec,
    ProviderSignalTable, Quantity, SignalEvent, SignalKind, SignalSource,
};
pub use bespoke::{
    EXIT_STDERR_TAIL_LINES, EXIT_STDOUT_TAIL_LINES, bespoke_replayer, exit_source_payload,
};
pub use engine::{ReplayObservation, SignalEngine};
pub use hub::SignalHub;
pub use sink::{CORRELATION_WINDOW, ObservedSignal, SignalSink};

use crate::provider_id::Provider;

/// The compiled detection table for `slug`, when the signals corpus has
/// one (this is the only route to the dormant kilo/pi tables).
pub fn detection_table(slug: &str) -> Option<&'static ProviderSignalTable> {
    generated::SIGNAL_TABLES
        .iter()
        .copied()
        .find(|table| table.slug == slug)
}

/// Every compiled provider table, alphabetical by slug — dormant kilo/pi
/// included (the enumeration surface for `claudine signals`).
pub fn all_detection_tables() -> &'static [&'static ProviderSignalTable] {
    generated::SIGNAL_TABLES
}

/// The compiled detection table for a wired provider.
pub fn for_provider(provider: Provider) -> &'static ProviderSignalTable {
    detection_table(provider.as_slug()).unwrap_or_else(|| {
        // Every Provider slug has a signals doc; the gen drift test keeps
        // the committed tables in lockstep with the corpus, so this is
        // unreachable in a consistent build.
        panic!(
            "no compiled signal table for provider slug `{}`",
            provider.as_slug()
        )
    })
}

#[cfg(test)]
mod projection_equivalence_tests;
#[cfg(test)]
mod semantics_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_id::PROVIDERS_DISPLAY_ORDER;

    #[test]
    fn every_wired_provider_has_a_table() {
        for provider in PROVIDERS_DISPLAY_ORDER {
            assert_eq!(for_provider(provider).slug, provider.as_slug());
        }
    }

    #[test]
    fn detection_tables_are_reachable_by_slug() {
        // `detection_table()` is the slug-keyed replay seam used by
        // `signals check`. Every compiled table must be reachable by its slug
        // (including any researched-but-unwired ones, should they exist), and an
        // unknown slug returns None.
        for table in all_detection_tables() {
            let looked_up =
                detection_table(table.slug).expect("compiled table reachable by slug");
            assert_eq!(looked_up.slug, table.slug);
            assert!(!looked_up.records.is_empty());
        }
        assert!(detection_table("nonesuch").is_none());
    }
}
