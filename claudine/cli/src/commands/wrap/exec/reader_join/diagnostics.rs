//! Reader diagnostics written while the output worker is stuck on the
//! terminal, before any drain has found the stall: production tracing and the
//! real panic hook must not stop the settling thread or a panicking reader.

use super::*;
use crate::terminal_gate::tests::{RoutedToBlockedWorker, returns_while_std_streams_are_locked};

/// With production tracing enabled at warn level, a reader warning is traced
/// and queued without waiting on the terminal, so the settling caller still
/// reaches its bounded drain and returns.
#[test]
fn a_traced_reader_warning_reaches_the_bounded_drain_while_the_worker_holds_stderr() {
    let _rust_log = test_toolkit::EnvGuard::remove_safe("RUST_LOG");
    crate::telemetry::init_tracing(Some(crate::args::DebugLevel::Warn));
    assert!(tracing::enabled!(tracing::Level::WARN), "production tracing is on");
    let routed = RoutedToBlockedWorker::new();
    let output = Arc::clone(routed.output());

    let returned = returns_while_std_streams_are_locked(move || {
        queue_reader_warnings(&output, &["the reader timed out".to_string()]);
        assert_eq!(output.queued_frames(), 2, "the traced and the shown warning are queued");
        let drained = output.drain(Instant::now() + Duration::from_millis(50));
        assert_eq!(drained, super::super::super::output_worker::Drained::Disabled);
    });

    assert!(returned, "the settling caller waited on the terminal");
    assert!(routed.output().loss().stalled, "the drain found the stall");
}

/// A reader that panics while the worker holds stderr unwinds at once under
/// the CLI's real panic hook, so the join reports the panic and its payload,
/// never a processing timeout.
#[test]
fn a_reader_panic_under_the_real_hook_is_reported_as_a_panic_while_the_worker_holds_stderr() {
    let (panic_hook, _eyre_hook) = color_eyre::config::HookBuilder::default().into_hooks();
    std::panic::set_hook(crate::terminal_gate::queued_panic_hook(panic_hook));
    let routed = RoutedToBlockedWorker::new();
    let budget = ReaderBudget {
        pipe_cap: Duration::from_secs(5),
        pipe_settle: Duration::from_secs(5),
        drain_limit: Duration::from_secs(3),
    };

    let (outcome_tx, outcome_rx) = std::sync::mpsc::channel();
    let returned = returns_while_std_streams_are_locked(move || {
        let progress = ReaderProgress::default();
        let reader_progress = progress.clone();
        let reader = thread::spawn(move || {
            if let Some(line) = reader_progress.track(["defect"].into_iter()).next() {
                panic!("parser {line}");
            }
        });
        let _ = outcome_tx.send(join_reader(reader, &progress, budget, Instant::now()));
    });
    let _ = std::panic::take_hook();

    assert!(returned, "the panicking reader's report waited on the terminal");
    match outcome_rx.recv().expect("the join finished") {
        JoinOutcome::Panicked(message) => assert_eq!(message, "parser defect"),
        other => panic!("a reader panic must be reported as a panic, got {other:?}"),
    }
    let written = routed.release();
    let report = String::from_utf8_lossy(&written.last().expect("the report was queued").1)
        .into_owned();
    assert!(report.contains("parser defect"), "{report}");
}
