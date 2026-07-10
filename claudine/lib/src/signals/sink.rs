//! Session-scoped signal collection with correlation-window dedup.

use chrono::{DateTime, Utc};
use claudine_catalog_types::{SignalEvent, SignalSource};
use std::collections::BTreeMap;
use std::time::Duration;
use tracing::debug;

/// Same-kind emissions inside this window fold into one observation. The
/// window exists to collapse stream+log double-observation of one underlying
/// provider event (the same throttle can arrive on stdout AND in a session
/// log within moments) into a single signal with a bumped counter.
pub const CORRELATION_WINDOW: Duration = Duration::from_secs(5);

/// Extraction fields a fired record resolved but the typed [`SignalEvent`]
/// variant has no slot for — held as observation provenance (more-struture
/// Cat 1B) instead of debug-dropped. Keyed by the corpus `field` name (e.g.
/// `provider`, `model`, `session_id`); values are display strings.
pub type SignalContext = BTreeMap<&'static str, String>;

/// One deduplicated signal observation.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ObservedSignal {
    pub event: SignalEvent,
    /// The source that FIRST observed the signal (first event wins).
    pub source: SignalSource,
    pub first_seen: DateTime<Utc>,
    /// Emissions folded into this observation, including the first.
    pub occurrences: u32,
    /// Supplementary extraction fields with no variant slot, from the FIRST
    /// event (first-event-wins, mirroring the other fields). Empty for
    /// bespoke emissions and records whose fields all landed in the variant.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub context: SignalContext,
}

/// Collects emitted [`SignalEvent`]s for one wrapped run.
///
/// Session scoping is implicit: one sink per run.
#[derive(Debug, Default)]
pub struct SignalSink {
    signals: Vec<ObservedSignal>,
}

impl SignalSink {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record `event`, folding it into the most recent same-kind signal
    /// when that signal's `first_seen` is within [`CORRELATION_WINDOW`] —
    /// the FIRST event wins and only its counter moves.
    pub fn emit(&mut self, event: SignalEvent, source: SignalSource) {
        self.emit_at(event, source, Utc::now());
    }

    /// Record `event` with its supplementary [`SignalContext`] (the declarative
    /// engine path). Bespoke emissions use [`emit`](Self::emit) (empty context).
    pub fn emit_with_context(
        &mut self,
        event: SignalEvent,
        context: SignalContext,
        source: SignalSource,
    ) {
        self.emit_at_with_context(event, context, source, Utc::now());
    }

    /// Clock-injected form of [`emit`](Self::emit) for deterministic tests.
    pub(crate) fn emit_at(&mut self, event: SignalEvent, source: SignalSource, at: DateTime<Utc>) {
        self.emit_at_with_context(event, SignalContext::new(), source, at);
    }

    /// Clock-injected form of [`emit_with_context`](Self::emit_with_context).
    pub(crate) fn emit_at_with_context(
        &mut self,
        event: SignalEvent,
        context: SignalContext,
        source: SignalSource,
        at: DateTime<Utc>,
    ) {
        let kind = event.kind();
        let window = chrono::Duration::from_std(CORRELATION_WINDOW)
            .expect("CORRELATION_WINDOW fits chrono::Duration");
        if let Some(existing) = self
            .signals
            .iter_mut()
            .rev()
            .find(|signal| signal.event.kind() == kind)
            && at.signed_duration_since(existing.first_seen) <= window
        {
            existing.occurrences = existing.occurrences.saturating_add(1);
            debug!(
                ?kind,
                occurrences = existing.occurrences,
                "signal folded into correlation window"
            );
            return;
        }
        debug!(?kind, ?source, "signal accepted");
        self.signals.push(ObservedSignal {
            event,
            source,
            first_seen: at,
            occurrences: 1,
            context,
        });
    }

    /// Signals observed so far, in first-seen order.
    pub fn signals(&self) -> &[ObservedSignal] {
        &self.signals
    }

    /// Consume the sink, yielding the collected signals.
    pub fn into_signals(self) -> Vec<ObservedSignal> {
        self.signals
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use claudine_catalog_types::SignalKind;

    fn at(seconds: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000 + seconds, 0).unwrap()
    }

    fn rate_limited(message: &str) -> SignalEvent {
        SignalEvent::RateLimited {
            status_code: None,
            reset_at: None,
            retry_after: None,
            message: Some(message.to_string()),
        }
    }

    #[test]
    fn same_kind_within_window_folds_and_first_event_wins() {
        let mut sink = SignalSink::new();
        sink.emit_at(rate_limited("first"), SignalSource::Stream, at(0));
        sink.emit_at(rate_limited("second"), SignalSource::SessionLog, at(3));

        let signals = sink.signals();
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].occurrences, 2);
        assert_eq!(signals[0].source, SignalSource::Stream);
        assert_eq!(signals[0].event, rate_limited("first"));
    }

    #[test]
    fn same_kind_outside_window_records_separately() {
        let mut sink = SignalSink::new();
        sink.emit_at(rate_limited("first"), SignalSource::Stream, at(0));
        sink.emit_at(rate_limited("later"), SignalSource::Stream, at(6));
        assert_eq!(sink.signals().len(), 2);
    }

    #[test]
    fn different_kinds_never_fold() {
        let mut sink = SignalSink::new();
        sink.emit_at(rate_limited("throttle"), SignalSource::Stream, at(0));
        sink.emit_at(
            SignalEvent::NoFunds {
                message: Some("no credits".to_string()),
                top_up_url: None,
            },
            SignalSource::Stream,
            at(0),
        );
        let signals = sink.into_signals();
        assert_eq!(signals.len(), 2);
        assert_eq!(signals[0].event.kind(), SignalKind::RateLimited);
        assert_eq!(signals[1].event.kind(), SignalKind::NoFunds);
    }
}
