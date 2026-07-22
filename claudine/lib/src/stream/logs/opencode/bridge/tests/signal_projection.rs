//! signal projection bridge tests.

use super::*;
use claudine_catalog_types::SignalKind;

const USAGE_CAP_1178: &str = include_str!(
    "../../../../../../../docs/research/signals/fixtures/opencode/stream-error-1178-usage-cap.txt"
);
const VERSION_ANNOUNCEMENT: &str = include_str!(
    "../../../../../../../docs/research/signals/fixtures/opencode/version-announcement.txt"
);

#[test]
fn exit_expression_variant_clones_and_compares() {
    let original = EarlyTermination::ExitExpression {
        pattern: "STOP.".to_string(),
        scope: Some("opencode/kimi-for-coding/k2p7".to_string()),
    };
    let cloned = original.clone();
    assert_eq!(original, cloned);
    // Different pattern must not compare equal — guards against a
    // future derive mistake that ignores fields.
    let other = EarlyTermination::ExitExpression {
        pattern: "HALT.".to_string(),
        scope: Some("opencode/kimi-for-coding/k2p7".to_string()),
    };
    assert_ne!(original, other);
}

#[test]
fn runaway_repetition_variant_clones_and_compares() {
    let original = EarlyTermination::RunawayRepetition {
        cycle_len: 6,
        repeats: 30,
    };
    let cloned = original.clone();
    assert_eq!(original, cloned);
    let other = EarlyTermination::RunawayRepetition {
        cycle_len: 6,
        repeats: 31,
    };
    assert_ne!(original, other);
}

#[test]
fn runaway_volume_variant_clones_and_compares() {
    let original = EarlyTermination::RunawayVolume {
        lines: 50_001,
        bytes: 32 * 1024 * 1024,
    };
    let cloned = original.clone();
    assert_eq!(original, cloned);
    let other = EarlyTermination::RunawayVolume {
        lines: 50_001,
        bytes: 32 * 1024 * 1024 + 1,
    };
    assert_ne!(original, other);
}

#[test]
fn new_variants_are_distinct_from_legacy_terminations() {
    // Exhaustive-distinctness smoke test: none of the three new
    // variants may compare equal to a legacy variant or to each other.
    let exit = EarlyTermination::ExitExpression {
        pattern: "x".to_string(),
        scope: None,
    };
    let rep = EarlyTermination::RunawayRepetition {
        cycle_len: 1,
        repeats: 30,
    };
    let vol = EarlyTermination::RunawayVolume { lines: 1, bytes: 1 };
    let rate = EarlyTermination::RateLimit {
        message: "m".to_string(),
        reset_at: None,
    };
    assert_ne!(exit, rep);
    assert_ne!(exit, vol);
    assert_ne!(rep, vol);
    assert_ne!(exit, rate);
    assert_ne!(rep, rate);
    assert_ne!(vol, rate);
}

// --- Stalled-generation detector (Phase 2 sanity; full matrix in Phase 5) ---
//
// These exercise the two private detector helpers directly with an
// injected `now: Instant` so no real time passes. The `on_llm_call`
// handler reads `Instant::now()` itself; the count/time logic it delegates
// to is what these lock down.

/// Builds a bridge over the compiled OpenCode signal table.
fn shim_bridge() -> (OpenCodeLogBridge<RecordingSink>, Arc<SignalHub>) {
    let hub = Arc::new(SignalHub::new(
        crate::signals::detection_table("opencode").expect("opencode table"),
    ));
    let bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None, None)
        .with_signal_hub(Arc::clone(&hub));
    (bridge, hub)
}

#[test]
fn shim_promotes_usage_cap_stderr_line_to_usage_capped_signal() {
    let (mut bridge, hub) = shim_bridge();
    for line in USAGE_CAP_1178.lines().filter(|l| !l.trim().is_empty()) {
        bridge.ingest(line);
    }

    let signals = hub.drain();
    let capped = signals
        .iter()
        .find(|s| s.event.kind() == SignalKind::UsageCapped)
        .expect("usage_capped signal from promoted stderr");
    assert_eq!(capped.source, SignalSource::StderrPromoted);
    let TaxonomySignalEvent::UsageCapped {
        message, lifts_at, ..
    } = &capped.event
    else {
        panic!("kind checked above");
    };
    assert!(
        message
            .as_deref()
            .is_some_and(|m| m.contains("Usage limit reached for 5 hour")),
        "message must carry the provider error: {message:?}"
    );
    assert!(
        lifts_at.is_some(),
        "lifts_at must be extracted from the classifier's reset_at"
    );
}

#[test]
fn shim_boot_banner_emits_provider_version_and_narrows_selection() {
    let (mut bridge, hub) = shim_bridge();
    let banner = VERSION_ANNOUNCEMENT
        .lines()
        .next()
        .expect("fixture has a banner line");
    bridge.ingest(banner);
    // A usage-cap line after the banner still fires: 1.14.48 admits the
    // un-bounded/legacy records, proving version narrowing did not
    // wipe the candidate set.
    for line in USAGE_CAP_1178.lines().filter(|l| !l.trim().is_empty()) {
        bridge.ingest(line);
    }

    let signals = hub.drain();
    let version = signals
        .iter()
        .find(|s| s.event.kind() == SignalKind::ProviderVersion)
        .expect("provider_version signal from the boot banner");
    assert_eq!(
        version.event,
        TaxonomySignalEvent::ProviderVersion {
            version: "1.14.48".to_string()
        }
    );
    assert!(
        signals
            .iter()
            .any(|s| s.event.kind() == SignalKind::UsageCapped),
        "usage cap must still fire under the narrowed (1.14.x) selection"
    );
}

#[test]
fn fire_early_termination_mirrors_the_bespoke_signal_once() {
    let (tx, _rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
    let hub = Arc::new(SignalHub::without_table());
    let mut bridge =
        OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), Some(tx), None)
            .with_signal_hub(Arc::clone(&hub));

    bridge.fire_early_termination(EarlyTermination::RepeatedStreamError { count: 5 });
    // Second fire is idempotent — no second signal.
    bridge.fire_early_termination(EarlyTermination::RepeatedStreamError { count: 6 });

    let signals = hub.drain();
    assert_eq!(signals.len(), 1);
    assert_eq!(
        signals[0].event,
        TaxonomySignalEvent::RepeatedStreamError { count: 5 }
    );
    assert_eq!(signals[0].source, SignalSource::StderrPromoted);
}

/// EarlyTermination → SignalKind, one row per variant. The mapping fn's
/// match is exhaustive, so a new variant fails compilation there; this
/// test pins the KIND each existing variant maps to.
#[test]
fn early_termination_to_signal_event_covers_every_variant() {
    let cases: Vec<(EarlyTermination, SignalKind)> = vec![
        (
            EarlyTermination::RateLimit {
                message: "cap".into(),
                reset_at: None,
            },
            // Terminal-cap semantics (see `to_signal_event`), not a
            // transient rate_limited.
            SignalKind::UsageCapped,
        ),
        (
            EarlyTermination::Timeout {
                message: "wall".into(),
            },
            SignalKind::Timeout,
        ),
        (
            EarlyTermination::StepTimeout {
                message: "silent".into(),
                outstanding: Vec::new(),
            },
            SignalKind::StepTimeout,
        ),
        (
            EarlyTermination::ExitExpression {
                pattern: "FATAL".into(),
                scope: Some("opencode/kimi".into()),
            },
            SignalKind::ExitExpression,
        ),
        (
            EarlyTermination::RunawayRepetition {
                cycle_len: 3,
                repeats: 12,
            },
            SignalKind::RunawayRepetition,
        ),
        (
            EarlyTermination::RunawayVolume {
                lines: 50_000,
                bytes: 33_554_432,
            },
            SignalKind::RunawayVolume,
        ),
        (
            EarlyTermination::RepeatedStreamError { count: 5 },
            SignalKind::RepeatedStreamError,
        ),
        (
            EarlyTermination::StalledGeneration {
                generation_count: 4,
                stall_duration: Duration::from_secs(600),
                context: StalledGenerationContext::default(),
            },
            SignalKind::StalledGeneration,
        ),
    ];
    for (termination, expected) in cases {
        assert_eq!(
            termination.to_signal_event().kind(),
            expected,
            "mapping drifted for {termination:?}"
        );
    }
}

#[test]
fn exit_expression_mapping_carries_pattern_and_scope() {
    let event = EarlyTermination::ExitExpression {
        pattern: "unrecoverable".into(),
        scope: Some("opencode/kimi-for-coding/k2p7".into()),
    }
    .to_signal_event();
    assert_eq!(
        event,
        TaxonomySignalEvent::ExitExpression {
            pattern: "unrecoverable".into(),
            scope: Some("opencode/kimi-for-coding/k2p7".into()),
        }
    );
}
