//! Unit tests for the connector's dispatch, retry, and classification logic.
//!
//! The retry and classification tests drive [`open_with_busy_retry`] through
//! injected `open`/`sleep` seams, so they assert the same code the Windows
//! connector runs while staying deterministic and runnable on every host. Real
//! byte-stream coverage — a bound daemon, a live pipe, genuine contention —
//! belongs to the Phase 7 integration suites.

use std::cell::RefCell;
use std::error::Error;
use std::ffi::OsString;
use std::path::PathBuf;

use super::*;

fn unix_endpoint() -> LocalEndpoint {
    LocalEndpoint::UnixSocket(PathBuf::from("/tmp/rendezvous-test/daemon.sock"))
}

fn pipe_endpoint() -> LocalEndpoint {
    LocalEndpoint::WindowsNamedPipe(OsString::from(r"\\.\pipe\claudine-rendezvous-test"))
}

/// The endpoint whose transport this target speaks.
fn native_endpoint() -> LocalEndpoint {
    if cfg!(unix) {
        unix_endpoint()
    } else {
        pipe_endpoint()
    }
}

/// The endpoint whose transport this target cannot speak.
fn foreign_endpoint() -> LocalEndpoint {
    if cfg!(unix) {
        pipe_endpoint()
    } else {
        unix_endpoint()
    }
}

fn busy_error() -> io::Error {
    io::Error::from_raw_os_error(ERROR_PIPE_BUSY)
}

/// Records what the retry loop asked for, standing in for a real sleep so the
/// loop's timing decisions are observable without spending wall clock.
#[derive(Default)]
struct SleepLog(RefCell<Vec<Duration>>);

impl SleepLog {
    fn record(&self, waited: Duration) -> impl Future<Output = ()> + use<> {
        self.0.borrow_mut().push(waited);
        std::future::ready(())
    }

    fn calls(&self) -> Vec<Duration> {
        self.0.borrow().clone()
    }
}

// ---------------------------------------------------------------------------
// Endpoint dispatch
// ---------------------------------------------------------------------------

/// `connect` must reject the other target's transport up front, by variant —
/// never by inspecting the name as text, and never by attempting a connect
/// that would fail with a confusing OS error instead.
#[tokio::test]
async fn connect_rejects_a_target_incompatible_endpoint() {
    let err = connect(&foreign_endpoint())
        .await
        .expect_err("the other target's transport is not connectable here");

    let ConnectError::IncompatibleEndpoint(inner) = err else {
        panic!("expected IncompatibleEndpoint, got: {err:?}");
    };
    assert!(
        matches!(inner, LocalEndpointError::IncompatibleTransport { .. }),
        "got: {inner:?}"
    );
}

/// A native endpoint with no daemon behind it must reach the transport and
/// come back as `NotFound` — proving dispatch selected a real connector rather
/// than bailing out early.
#[tokio::test]
async fn connect_to_a_native_endpoint_with_no_daemon_reports_not_found() {
    let temp = tempfile::tempdir().expect("tempdir");
    let endpoint = if cfg!(unix) {
        LocalEndpoint::UnixSocket(temp.path().join("absent.sock"))
    } else {
        LocalEndpoint::WindowsNamedPipe(OsString::from(
            r"\\.\pipe\claudine-rendezvous-absent-test",
        ))
    };

    let err = connect(&endpoint).await.expect_err("nothing is listening");
    assert!(
        matches!(err, ConnectError::NotFound { .. }),
        "an absent endpoint must not collapse into an opaque transport error, got: {err:?}"
    );
    assert!(
        err.to_string().contains(&endpoint.to_string()),
        "the message must name the endpoint: {err}"
    );
}

// ---------------------------------------------------------------------------
// Busy retry
// ---------------------------------------------------------------------------

#[tokio::test]
async fn retry_returns_the_stream_once_the_endpoint_stops_being_busy() {
    let endpoint = native_endpoint();
    let retry = BusyRetry {
        budget: Duration::from_millis(500),
        backoff: Duration::from_millis(50),
    };
    let sleeps = SleepLog::default();
    let attempts = RefCell::new(0);

    let opened = open_with_busy_retry(
        &endpoint,
        retry,
        || {
            *attempts.borrow_mut() += 1;
            let busy = *attempts.borrow() <= 2;
            std::future::ready(if busy { Err(busy_error()) } else { Ok("stream") })
        },
        |waited| sleeps.record(waited),
    )
    .await
    .expect("the third attempt succeeds");

    assert_eq!(opened, "stream");
    assert_eq!(*attempts.borrow(), 3, "it must retry, not give up on the first busy");
    assert_eq!(
        sleeps.calls(),
        vec![Duration::from_millis(50); 2],
        "one fixed backoff between attempts, and none after success"
    );
}

#[tokio::test]
async fn retry_gives_up_at_the_deadline_with_the_time_it_waited() {
    let endpoint = native_endpoint();
    let retry = BusyRetry {
        budget: Duration::from_millis(200),
        backoff: Duration::from_millis(50),
    };
    let sleeps = SleepLog::default();
    let attempts = RefCell::new(0);

    let err = open_with_busy_retry(
        &endpoint,
        retry,
        || {
            *attempts.borrow_mut() += 1;
            std::future::ready(Err::<&str, _>(busy_error()))
        },
        |waited| sleeps.record(waited),
    )
    .await
    .expect_err("a permanently busy endpoint must not retry forever");

    let ConnectError::BusyTimeout {
        endpoint: reported,
        waited,
        source,
    } = err
    else {
        panic!("expected BusyTimeout, got: {err:?}");
    };
    assert_eq!(reported, endpoint.to_string());
    assert_eq!(
        waited,
        Duration::from_millis(200),
        "it must spend the whole budget and no more"
    );
    assert_eq!(
        source.raw_os_error(),
        Some(ERROR_PIPE_BUSY),
        "the last busy report must survive as the source"
    );
    assert_eq!(
        sleeps.calls(),
        vec![Duration::from_millis(50); 4],
        "budget/backoff waits, each bounded — never one unbounded sleep"
    );
    assert_eq!(*attempts.borrow(), 5, "one attempt per wait, plus the final one");
}

/// A zero budget still gets one attempt: the endpoint may not be busy at all.
#[tokio::test]
async fn a_zero_budget_still_tries_once_and_never_sleeps() {
    let endpoint = native_endpoint();
    let retry = BusyRetry {
        budget: Duration::ZERO,
        backoff: Duration::from_millis(50),
    };
    let sleeps = SleepLog::default();
    let attempts = RefCell::new(0);

    let err = open_with_busy_retry(
        &endpoint,
        retry,
        || {
            *attempts.borrow_mut() += 1;
            std::future::ready(Err::<&str, _>(busy_error()))
        },
        |waited| sleeps.record(waited),
    )
    .await
    .expect_err("busy with no budget to wait it out");

    assert!(matches!(err, ConnectError::BusyTimeout { .. }), "got: {err:?}");
    assert_eq!(*attempts.borrow(), 1);
    assert!(sleeps.calls().is_empty(), "there was no budget to sleep against");
}

/// Only busy is retryable. Waiting out a permission error would turn a clear,
/// immediate failure into a slow one.
#[tokio::test]
async fn a_non_busy_error_fails_immediately_without_retrying() {
    let endpoint = native_endpoint();
    let sleeps = SleepLog::default();
    let attempts = RefCell::new(0);

    let err = open_with_busy_retry(
        &endpoint,
        BusyRetry::default(),
        || {
            *attempts.borrow_mut() += 1;
            std::future::ready(Err::<&str, _>(io::Error::from(
                io::ErrorKind::PermissionDenied,
            )))
        },
        |waited| sleeps.record(waited),
    )
    .await
    .expect_err("permission denied is not a busy endpoint");

    assert!(matches!(err, ConnectError::PermissionDenied { .. }), "got: {err:?}");
    assert_eq!(*attempts.borrow(), 1);
    assert!(sleeps.calls().is_empty());
}

/// The default budget must be finite and divisible into whole backoff units,
/// or the shipped Windows path would either never time out or overshoot.
#[test]
fn the_default_retry_budget_is_bounded() {
    let retry = BusyRetry::default();
    assert!(retry.backoff > Duration::ZERO, "a zero backoff would spin");
    assert!(retry.budget > retry.backoff);
    assert!(retry.budget <= Duration::from_secs(30), "a caller must not hang");
}

// ---------------------------------------------------------------------------
// OS error classification
// ---------------------------------------------------------------------------

#[test]
fn classification_distinguishes_the_cases_callers_branch_on() {
    let endpoint = native_endpoint();

    let cases: [(io::ErrorKind, &str); 4] = [
        (io::ErrorKind::NotFound, "NotFound"),
        (io::ErrorKind::ConnectionRefused, "NotFound"),
        (io::ErrorKind::PermissionDenied, "PermissionDenied"),
        (io::ErrorKind::BrokenPipe, "Io"),
    ];

    for (kind, expected) in cases {
        let actual = match classify(&endpoint, io::Error::from(kind)) {
            ConnectError::NotFound { .. } => "NotFound",
            ConnectError::PermissionDenied { .. } => "PermissionDenied",
            ConnectError::Io { .. } => "Io",
            other => panic!("{kind:?} classified as {other:?}"),
        };
        assert_eq!(actual, expected, "{kind:?} must classify as {expected}");
    }
}

/// A stale socket file whose daemon is gone refuses the connect rather than
/// vanishing. To a caller that is the same actionable state as an absent
/// endpoint, so it must not surface as an unclassified `Io`.
#[test]
fn a_refused_connection_reads_as_no_daemon_listening() {
    let endpoint = native_endpoint();
    let err = classify(&endpoint, io::Error::from(io::ErrorKind::ConnectionRefused));
    assert!(matches!(err, ConnectError::NotFound { .. }), "got: {err:?}");
    assert!(err.to_string().contains("no rendezvous daemon is listening"), "got: {err}");
}

/// Classification must not flatten the OS error into text: the original — with
/// its `raw_os_error` intact — has to stay reachable through the source chain,
/// or a caller diagnosing an odd failure loses the only precise fact about it.
#[test]
fn classification_preserves_the_original_os_error_as_the_source() {
    let endpoint = native_endpoint();

    let err = classify(&endpoint, busy_error());
    let source = err.source().expect("the OS error must survive classification");
    let source = source
        .downcast_ref::<io::Error>()
        .expect("the source must still be an io::Error, not a rendered string");
    assert_eq!(
        source.raw_os_error(),
        Some(ERROR_PIPE_BUSY),
        "the exact OS code must reach a caller walking the chain"
    );
}

#[test]
fn every_classified_error_names_the_endpoint() {
    let endpoint = native_endpoint();
    for kind in [
        io::ErrorKind::NotFound,
        io::ErrorKind::PermissionDenied,
        io::ErrorKind::BrokenPipe,
    ] {
        let err = classify(&endpoint, io::Error::from(kind));
        assert!(
            err.to_string().contains(&endpoint.to_string()),
            "{kind:?} produced a message with no endpoint: {err}"
        );
    }
}

/// The busy predicate keys on the raw OS code, not on the message text, so a
/// re-worded or localized Win32 message cannot silently disable the retry.
#[test]
fn busy_is_recognized_by_os_code_and_nothing_else_is() {
    assert!(is_busy(&busy_error()));
    assert!(!is_busy(&io::Error::from(io::ErrorKind::NotFound)));
    assert!(
        !is_busy(&io::Error::other("all pipe instances are busy")),
        "message text must not stand in for the OS code"
    );
}

// ---------------------------------------------------------------------------
// Error carriage out of tonic's connector closure
// ---------------------------------------------------------------------------

/// Tonic discards the connector's error into its own opaque type. The slot is
/// what keeps the classification, so a failure must come back as the real
/// cause rather than a generic `Transport`.
#[test]
fn a_deflected_error_is_recovered_instead_of_tonics_opaque_one() {
    let endpoint = native_endpoint();
    let slot = ErrorSlot::default();

    let stand_in = slot
        .deflect::<()>(Err(classify(
            &endpoint,
            io::Error::from(io::ErrorKind::PermissionDenied),
        )))
        .expect_err("deflect passes the failure through");
    assert!(
        stand_in.to_string().contains("permission denied"),
        "tonic's stand-in should still read sensibly if it ever surfaces: {stand_in}"
    );

    let recovered = slot.take().expect("the classification must be recoverable");
    assert!(
        matches!(recovered, ConnectError::PermissionDenied { .. }),
        "got: {recovered:?}"
    );
    assert!(slot.take().is_none(), "the slot must not replay a stale error");
}

#[test]
fn deflect_leaves_the_slot_empty_on_success() {
    let slot = ErrorSlot::default();
    assert_eq!(slot.deflect(Ok::<_, ConnectError>(7)).expect("success"), 7);
    assert!(slot.take().is_none());
}

// ---------------------------------------------------------------------------
// Live named-pipe contention (Windows only)
// ---------------------------------------------------------------------------
//
// Everything above fabricates `ERROR_PIPE_BUSY` and injects the sleep, which is
// what makes the retry loop's decisions assertable on any host. What that
// cannot show is whether Win32 *actually* reports a saturated pipe the way the
// loop expects: the seam tests would pass unchanged if the real code were some
// other number, or if it arrived as an `ErrorKind` rather than a raw code. These
// close that gap by saturating a real pipe.

#[cfg(windows)]
mod live_pipe {
    use super::*;
    use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};

    /// A pipe name unique to this test, so a parallel test cannot collide with
    /// it on the machine-wide pipe namespace.
    fn name(test: &str) -> OsString {
        OsString::from(format!(
            r"\\.\pipe\claudine-rendezvous-clienttest-{test}-{}",
            std::process::id()
        ))
    }

    /// A pipe with exactly one instance, already connected to a client — the
    /// saturated state a daemon presents while it is between accepts.
    ///
    /// The returned client must be held: dropping it frees the instance.
    fn saturated(name: &OsString) -> (tokio::net::windows::named_pipe::NamedPipeServer, tokio::net::windows::named_pipe::NamedPipeClient) {
        let server = ServerOptions::new()
            .first_pipe_instance(true)
            .max_instances(1)
            .create(name)
            .expect("create the single instance");
        let client = ClientOptions::new().open(name).expect("occupy it");
        (server, client)
    }

    /// The constant the retry loop keys on, checked against the number Windows
    /// itself produces. `ERROR_PIPE_BUSY` being wrong would not fail any seam
    /// test — it would silently turn every busy open into a hard `Io` failure.
    #[tokio::test]
    async fn a_real_saturated_pipe_reports_the_code_the_retry_loop_recognizes() {
        let name = name("recognized");
        let (_server, _occupier) = saturated(&name);

        let error = ClientOptions::new()
            .open(&name)
            .expect_err("a pipe with its only instance taken must refuse");

        assert!(
            is_busy(&error),
            "the retry loop must recognize the real Win32 busy report; got {error:?} \
             (raw code {:?})",
            error.raw_os_error()
        );
    }

    /// The recovery path against genuine contention: the instance frees up
    /// mid-budget and the open succeeds rather than burning the whole deadline.
    #[tokio::test]
    async fn a_real_busy_pipe_is_connected_once_an_instance_frees_up() {
        let name = name("recovers");
        let (server, occupier) = saturated(&name);

        // Free the instance after a couple of backoffs, so the loop has to
        // actually retry rather than win on its first attempt.
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(120)).await;
            drop(occupier);
            drop(server);
        });

        let endpoint = LocalEndpoint::WindowsNamedPipe(name.clone());
        let retry = BusyRetry {
            budget: Duration::from_secs(5),
            backoff: Duration::from_millis(50),
        };
        let opened = open_with_busy_retry(
            &endpoint,
            retry,
            || async { ClientOptions::new().open(&name) },
            tokio::time::sleep,
        )
        .await;

        assert!(
            opened.is_ok(),
            "a pipe that frees up within the budget must connect; got {:?}",
            opened.err()
        );
    }

    /// Deadline exhaustion against a pipe that never frees: bounded means
    /// bounded, and the reported wait must be the real one.
    #[tokio::test]
    async fn a_real_pipe_that_stays_busy_gives_up_at_the_deadline() {
        let name = name("exhausts");
        let (_server, _occupier) = saturated(&name);

        let endpoint = LocalEndpoint::WindowsNamedPipe(name.clone());
        let retry = BusyRetry {
            budget: Duration::from_millis(150),
            backoff: Duration::from_millis(50),
        };
        let started = std::time::Instant::now();
        let error = open_with_busy_retry(
            &endpoint,
            retry,
            || async { ClientOptions::new().open(&name) },
            tokio::time::sleep,
        )
        .await
        .expect_err("a permanently busy pipe must not connect");

        let ConnectError::BusyTimeout { waited, .. } = &error else {
            panic!("expected BusyTimeout, got: {error:?}");
        };
        assert!(
            *waited <= retry.budget,
            "the loop reported waiting {waited:?}, beyond its own {:?} budget",
            retry.budget
        );
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "the budget must bound the wall clock, not merely the bookkeeping"
        );
    }
}
