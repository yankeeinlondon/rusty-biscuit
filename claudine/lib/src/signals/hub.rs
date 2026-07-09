//! Multi-producer fan-in for one wrapped run's signal pipeline.
//!
//! The stdout reader thread, the stderr log bridge, and the CLI's
//! termination synthesis all observe the SAME run, so they must share one
//! engine (for version narrowing) and one sink (for cross-source dedup).
//! [`SignalHub`] is that shared point: engine + sink behind a single lock,
//! cloned across threads as `Arc<SignalHub>`.

use std::sync::Mutex;

use claudine_catalog_types::{ProviderSignalTable, SignalEvent, SignalSource};
use serde_json::Value;

use super::bespoke::BespokeChain;
use super::engine::SignalEngine;
use super::harvest::{HarvestBatch, HarvestBuffer};
use super::sink::{ObservedSignal, SignalSink};
use crate::provider_id::{PROVIDERS_DISPLAY_ORDER, Provider};

/// Thread-safe fan-in of declarative and bespoke signal producers for one
/// wrapped run.
///
/// Lock discipline: the single `Mutex` is held for observe+emit only —
/// engine matching, version narrowing, and sink dedup are one atomic step
/// per payload so producers on different threads cannot interleave between
/// a `ProviderVersion` observation and the narrowing it implies. No I/O or
/// rendering ever happens under the lock.
pub struct SignalHub {
    inner: Mutex<HubInner>,
}

struct HubInner {
    /// `None` on table-less runs (see [`SignalHub::without_table`]):
    /// [`SignalHub::observe_json`] is inert, bespoke emission still works.
    engine: Option<SignalEngine>,
    /// Provider-keyed cross-payload detectors (empty on table-less runs);
    /// run after the declarative pass under the same lock.
    chain: BespokeChain,
    sink: SignalSink,
    /// Provider slug for harvest attribution; `None` on table-less hubs.
    slug: Option<&'static str>,
    /// `Some` only after [`SignalHub::enable_harvest`] — the `None` path
    /// keeps unmatched-payload handling zero-cost (no predicate
    /// evaluation, no buffering).
    harvest: Option<HarvestBuffer>,
}

impl SignalHub {
    /// Hub with the declarative engine compiled from `table`.
    ///
    /// Every wired provider has a compiled table
    /// ([`super::for_provider`] is total), so this is the constructor for
    /// all provider-attributed runs.
    pub fn new(table: &'static ProviderSignalTable) -> Self {
        Self {
            inner: Mutex::new(HubInner {
                engine: Some(SignalEngine::new(table)),
                chain: BespokeChain::for_slug(table.slug),
                sink: SignalSink::new(),
                slug: Some(table.slug),
                harvest: None,
            }),
        }
    }

    /// Bespoke-only hub for spawn paths with no provider attribution
    /// (e.g. the capture fallback path, tests). [`Self::observe_json`] is a
    /// no-op; [`Self::emit_bespoke`] and [`Self::drain`] work normally.
    pub fn without_table() -> Self {
        Self {
            inner: Mutex::new(HubInner {
                engine: None,
                chain: BespokeChain::empty(),
                sink: SignalSink::new(),
                slug: None,
                harvest: None,
            }),
        }
    }

    /// Run one JSON payload from `source` through the declarative engine
    /// and then the bespoke detector chain, emitting every fired event
    /// into the shared sink.
    ///
    /// An emitted [`SignalEvent::ProviderVersion`] narrows the engine's
    /// record selection for the remainder of the run (design
    /// §Version-range selection) — regardless of which source observed it,
    /// since the provider version is a run-level fact.
    pub fn observe_json(&self, source: SignalSource, payload: &Value) {
        let mut inner = self.lock();
        let inner = &mut *inner;
        let mut fired = false;
        if let Some(engine) = inner.engine.as_mut() {
            let observations = engine.observe_with_context(source, payload);
            for (event, context) in observations {
                fired = true;
                if let SignalEvent::ProviderVersion { version } = &event {
                    engine.observe_provider_version(version);
                }
                inner.sink.emit_with_context(event, context, source);
            }
        }
        // Table-less hubs have an empty chain, so this preserves the
        // "observe_json is inert without a table" contract.
        for event in inner.chain.observe(source, payload) {
            fired = true;
            inner.sink.emit(event, source);
        }
        // Harvest seam (design §Harvest): only payloads NO record claimed
        // are candidates — a matched payload is evidence we already have.
        if !fired && let Some(harvest) = inner.harvest.as_mut() {
            harvest.consider(source, payload);
        }
    }

    /// Opt this run in to unmatched-event harvesting (design §Harvest).
    ///
    /// No-op on table-less hubs: without provider attribution there is no
    /// `~/.claudine/harvest/<provider>/` destination to file candidates
    /// under (and their `observe_json` is inert regardless).
    pub fn enable_harvest(&self) {
        let mut inner = self.lock();
        if let Some(slug) = inner.slug
            && inner.harvest.is_none()
        {
            inner.harvest = Some(HarvestBuffer::new(slug));
        }
    }

    /// Take the buffered harvest candidates, leaving harvesting enabled
    /// with an empty buffer. `None` when harvesting was never enabled.
    pub(crate) fn take_harvest(&self) -> Option<HarvestBatch> {
        self.lock().harvest.as_mut().map(HarvestBuffer::take_batch)
    }

    /// Direct sink emission for bespoke producers (temporal guards,
    /// cross-event rules). Shares the dedup window with declarative
    /// emissions, so a guard that fires on two paths folds to one signal.
    pub fn emit_bespoke(&self, event: SignalEvent, source: SignalSource) {
        self.lock().sink.emit(event, source);
    }

    /// The `resolved` ids of every [`SignalEvent::ModelResolved`]
    /// currently in the sink, without draining.
    ///
    /// Exists so the wrapper can run catalog-drift checks against
    /// stream-observed resolution before the end-of-run [`Self::drain`].
    pub fn resolved_models(&self) -> Vec<String> {
        self.lock()
            .sink
            .signals()
            .iter()
            .filter_map(|signal| match &signal.event {
                SignalEvent::ModelResolved { resolved, .. } => Some(resolved.clone()),
                _ => None,
            })
            .collect()
    }

    /// The run's provider attribution, recovered from the compiled
    /// table's slug.
    ///
    /// `None` on table-less hubs and on any compiled slug that has no
    /// [`Provider`] variant (a researched-but-unwired provider). Exists
    /// so end-of-run wrapper checks (catalog drift) can key baseline
    /// lookups without threading a `Provider` through the spawn paths.
    pub fn provider(&self) -> Option<Provider> {
        let slug = self.lock().slug?;
        PROVIDERS_DISPLAY_ORDER
            .into_iter()
            .find(|provider| provider.as_slug() == slug)
    }

    /// Take every signal observed so far, in first-seen order, leaving the
    /// sink empty. The end-of-run handoff into `ProcessResult.signals` —
    /// `&self` (not consuming) because producer threads may still hold
    /// `Arc` clones when the main thread collects.
    pub fn drain(&self) -> Vec<ObservedSignal> {
        std::mem::take(&mut self.lock().sink).into_signals()
    }

    /// A producer thread that panics while holding the lock must not take
    /// the whole signal pipeline down with it — signals are diagnostics,
    /// so recover the data rather than propagating the poison.
    fn lock(&self) -> std::sync::MutexGuard<'_, HubInner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use claudine_catalog_types::SignalKind;

    use super::super::detection_table;
    use super::*;

    fn rate_limited(message: &str) -> SignalEvent {
        SignalEvent::RateLimited {
            status_code: None,
            reset_at: None,
            retry_after: None,
            message: Some(message.to_string()),
        }
    }

    #[test]
    fn concurrent_producers_share_dedup_and_drain_returns_all() {
        let hub = Arc::new(SignalHub::without_table());

        // Two threads: both emit RateLimited (same kind, inside the
        // correlation window → folds to one), plus one distinct kind each.
        let hub_a = Arc::clone(&hub);
        let a = std::thread::spawn(move || {
            hub_a.emit_bespoke(rate_limited("from stream"), SignalSource::Stream);
            hub_a.emit_bespoke(
                SignalEvent::Timeout {
                    message: Some("wall clock".into()),
                },
                SignalSource::Stream,
            );
        });
        let hub_b = Arc::clone(&hub);
        let b = std::thread::spawn(move || {
            hub_b.emit_bespoke(rate_limited("from log"), SignalSource::StderrPromoted);
            hub_b.emit_bespoke(
                SignalEvent::NoFunds {
                    message: Some("no credits".into()),
                    top_up_url: None,
                },
                SignalSource::StderrPromoted,
            );
        });
        a.join().unwrap();
        b.join().unwrap();

        let signals = hub.drain();
        let mut kinds: Vec<SignalKind> = signals.iter().map(|s| s.event.kind()).collect();
        kinds.sort_by_key(|kind| <&'static str>::from(*kind));
        assert_eq!(
            kinds,
            vec![
                SignalKind::NoFunds,
                SignalKind::RateLimited,
                SignalKind::Timeout
            ],
            "same-kind emissions fold across threads; distinct kinds survive"
        );
        let folded = signals
            .iter()
            .find(|s| s.event.kind() == SignalKind::RateLimited)
            .expect("rate_limited observation");
        assert_eq!(folded.occurrences, 2, "cross-source fold bumps the counter");

        assert!(hub.drain().is_empty(), "drain empties the sink");
    }

    #[test]
    fn observe_json_runs_engine_and_feeds_sink() {
        let hub = SignalHub::new(detection_table("claude").expect("claude table"));
        let payload: Value = serde_json::from_str(
            r#"{"type":"rate_limit_event","is_throttled":true,"retry_after_ms":5000,"message":"slow"}"#,
        )
        .unwrap();
        hub.observe_json(SignalSource::Stream, &payload);

        let signals = hub.drain();
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].event.kind(), SignalKind::RateLimited);
        assert_eq!(signals[0].source, SignalSource::Stream);
    }

    #[test]
    fn resolved_models_reads_without_draining() {
        let hub = SignalHub::without_table();
        hub.emit_bespoke(
            SignalEvent::ModelResolved {
                requested: Some("opus".into()),
                resolved: "claude-opus-4-8".into(),
            },
            SignalSource::Stream,
        );
        hub.emit_bespoke(rate_limited("throttle"), SignalSource::Stream);

        assert_eq!(hub.resolved_models(), vec!["claude-opus-4-8".to_string()]);
        // Non-destructive: the drain that follows still sees both signals.
        assert_eq!(hub.drain().len(), 2);
        assert!(hub.resolved_models().is_empty());
    }

    #[test]
    fn provider_recovers_wired_attribution_only() {
        let wired = SignalHub::new(detection_table("claude").expect("claude table"));
        assert_eq!(wired.provider(), Some(crate::provider_id::Provider::Claude));

        assert_eq!(SignalHub::without_table().provider(), None);

        // A compiled table whose slug has no `Provider` variant recovers no
        // attribution. Every researched slug in `SIGNAL_TABLES` is now wired
        // (pi graduated at M-Pi), so this path is exercised with a synthetic
        // dormant slug rather than a roster-only one — keeping the test robust
        // to future graduations instead of chasing the last unwired slug.
        static SYNTHETIC_DORMANT: claudine_catalog_types::ProviderSignalTable =
            claudine_catalog_types::ProviderSignalTable {
                slug: "nonesuch",
                records: &[],
            };
        let dormant = SignalHub::new(&SYNTHETIC_DORMANT);
        assert_eq!(dormant.provider(), None);
    }

    #[test]
    fn observe_json_without_table_is_inert() {
        let hub = SignalHub::without_table();
        let payload: Value =
            serde_json::from_str(r#"{"type":"rate_limit_event","is_throttled":true}"#).unwrap();
        hub.observe_json(SignalSource::Stream, &payload);
        assert!(hub.drain().is_empty());
    }

    // -----------------------------------------------------------------
    // Harvest seam (E6)
    // -----------------------------------------------------------------

    /// The committed claude billing fixture line: error-class AND fires the
    /// no_funds record, so it must never be harvested.
    const BILLING_FIXTURE_LINE: &str =
        include_str!("../../../docs/research/signals/fixtures/claude/error-billing.jsonl");

    fn unmatched_error_payload(n: usize) -> Value {
        serde_json::json!({ "level": "error", "message": "unrecognized", "n": n })
    }

    #[test]
    fn matched_error_payload_is_never_harvested() {
        let hub = SignalHub::new(detection_table("claude").expect("claude table"));
        hub.enable_harvest();
        let payload: Value = serde_json::from_str(BILLING_FIXTURE_LINE.trim()).unwrap();
        hub.observe_json(SignalSource::Stream, &payload);

        assert!(!hub.drain().is_empty(), "fixture line fires a record");
        let batch = hub.take_harvest().expect("harvest enabled");
        assert!(batch.entries.is_empty(), "matched payloads are not candidates");
    }

    #[test]
    fn unmatched_error_payload_is_harvested_and_benign_is_not() {
        let hub = SignalHub::new(detection_table("claude").expect("claude table"));
        hub.enable_harvest();
        hub.observe_json(SignalSource::Stream, &unmatched_error_payload(0));
        hub.observe_json(
            SignalSource::Stream,
            &serde_json::json!({ "type": "assistant", "message": "hello" }),
        );

        let batch = hub.take_harvest().expect("harvest enabled");
        assert_eq!(batch.entries.len(), 1);
        assert_eq!(batch.entries[0].provider, "claude");
        assert_eq!(batch.entries[0].source, "stream");
        assert_eq!(batch.dropped, 0);

        let again = hub.take_harvest().expect("take leaves harvesting enabled");
        assert!(again.entries.is_empty(), "take empties the buffer");
    }

    #[test]
    fn disabled_hub_buffers_nothing() {
        let hub = SignalHub::new(detection_table("claude").expect("claude table"));
        hub.observe_json(SignalSource::Stream, &unmatched_error_payload(0));
        assert!(hub.take_harvest().is_none(), "never enabled");
    }

    #[test]
    fn enable_harvest_is_a_noop_without_a_table() {
        let hub = SignalHub::without_table();
        hub.enable_harvest();
        assert!(hub.take_harvest().is_none());
    }

    #[test]
    fn harvest_buffer_caps_and_counts_drops() {
        use super::super::harvest::MAX_HARVEST_ENTRIES_PER_RUN;

        let hub = SignalHub::new(detection_table("claude").expect("claude table"));
        hub.enable_harvest();
        for n in 0..MAX_HARVEST_ENTRIES_PER_RUN + 5 {
            hub.observe_json(SignalSource::Stream, &unmatched_error_payload(n));
        }
        let batch = hub.take_harvest().expect("harvest enabled");
        assert_eq!(batch.entries.len(), MAX_HARVEST_ENTRIES_PER_RUN);
        assert_eq!(batch.dropped, 5);
    }
}
