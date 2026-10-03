//! The registry is process-wide. nextest runs each test in its own process,
//! which is what keeps these tests from seeing each other's deliveries.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::*;

/// Sets its flag when dropped, which is how a test sees an aborted task go.
struct DropSentinel(Arc<AtomicBool>);

impl Drop for DropSentinel {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

fn route(name: &str) -> DeliveryLabel {
    DeliveryLabel::Route(name.to_string())
}

/// Track a delivery that never finishes; the returned flag turns true once
/// its task is dropped.
fn track_stalled(label: DeliveryLabel) -> Arc<AtomicBool> {
    let dropped = Arc::new(AtomicBool::new(false));
    let sentinel = DropSentinel(dropped.clone());
    track(label, async move {
        let _sentinel = sentinel;
        std::future::pending::<()>().await;
    });
    dropped
}

/// Let the runtime process aborted tasks so their futures are dropped.
async fn settle_runtime() {
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
}

#[tokio::test(start_paused = true)]
async fn a_delivery_that_finishes_is_not_reported() {
    let delivered = Arc::new(AtomicBool::new(false));
    let flag = delivered.clone();
    track(route("alerts"), async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        flag.store(true, Ordering::SeqCst);
    });

    let start = Instant::now();
    let outcome = drain_deliveries(start + DELIVERY_DRAIN_BUDGET).await;

    assert!(delivered.load(Ordering::SeqCst));
    assert!(outcome.pending.is_empty());
    assert!(outcome.panicked.is_empty());
    assert_eq!(outcome.warning_prose(), None);
    assert_eq!(start.elapsed(), Duration::from_millis(200));
    assert_eq!(tracked_count(), 0);
}

#[tokio::test(start_paused = true)]
async fn a_delivery_registered_while_draining_is_awaited() {
    let inner_delivered = Arc::new(AtomicBool::new(false));
    let flag = inner_delivered.clone();
    track(route("outer"), async move {
        tokio::time::sleep(Duration::from_secs(1)).await;
        track(route("inner"), async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            flag.store(true, Ordering::SeqCst);
        });
    });

    let start = Instant::now();
    let outcome = drain_deliveries(start + DELIVERY_DRAIN_BUDGET).await;

    assert!(inner_delivered.load(Ordering::SeqCst));
    assert!(outcome.pending.is_empty());
    assert_eq!(start.elapsed(), Duration::from_secs(3));
}

#[tokio::test(start_paused = true)]
async fn stalled_deliveries_share_one_deadline_and_are_aborted() {
    let dropped: Vec<_> = ["first", "second", "third"]
        .into_iter()
        .map(|name| track_stalled(route(name)))
        .collect();
    let notification_dropped = track_stalled(DeliveryLabel::DesktopNotification);

    let start = Instant::now();
    let outcome = drain_deliveries(start + DELIVERY_DRAIN_BUDGET).await;

    // One budget for all four, not one per delivery.
    assert_eq!(start.elapsed(), DELIVERY_DRAIN_BUDGET);
    assert_eq!(
        outcome.pending,
        vec![
            route("first"),
            route("second"),
            route("third"),
            DeliveryLabel::DesktopNotification,
        ]
    );
    assert!(outcome.panicked.is_empty());
    assert_eq!(tracked_count(), 0);

    settle_runtime().await;
    for flag in dropped.iter().chain([&notification_dropped]) {
        assert!(flag.load(Ordering::SeqCst), "a stalled delivery was not aborted");
    }
}

#[tokio::test(start_paused = true)]
async fn a_deadline_already_passed_returns_without_waiting() {
    let finished = Arc::new(AtomicBool::new(false));
    let flag = finished.clone();
    track(route("quick"), async move { flag.store(true, Ordering::SeqCst) });
    settle_runtime().await;
    assert!(finished.load(Ordering::SeqCst));
    let stalled_dropped = track_stalled(route("stalled"));

    let start = Instant::now();
    let outcome = drain_deliveries(start).await;

    assert_eq!(start.elapsed(), Duration::ZERO);
    assert_eq!(outcome.pending, vec![route("stalled")]);
    settle_runtime().await;
    assert!(stalled_dropped.load(Ordering::SeqCst));
}

#[tokio::test(start_paused = true)]
async fn nothing_tracked_returns_immediately() {
    let start = Instant::now();
    let outcome = drain_deliveries(start + DELIVERY_DRAIN_BUDGET).await;

    assert_eq!(start.elapsed(), Duration::ZERO);
    assert!(outcome.pending.is_empty());
    assert!(outcome.panicked.is_empty());
}

#[tokio::test(start_paused = true)]
async fn pending_deliveries_are_seen_until_they_finish() {
    assert!(!has_pending_deliveries());

    let stalled_dropped = track_stalled(route("stalled"));
    track(route("quick"), async {});
    assert!(has_pending_deliveries());

    let outcome = drain_deliveries(Instant::now() + DELIVERY_DRAIN_BUDGET).await;
    assert_eq!(outcome.pending, vec![route("stalled")]);
    assert!(!has_pending_deliveries());
    settle_runtime().await;
    assert!(stalled_dropped.load(Ordering::SeqCst));

    // A delivery that has already finished but was never drained does not
    // count as pending.
    track(route("done"), async {});
    settle_runtime().await;
    assert_eq!(tracked_count(), 1);
    assert!(!has_pending_deliveries());
}

#[tokio::test(start_paused = true)]
async fn a_panicking_delivery_is_reported_and_not_re_raised() {
    track(route("boom"), async { panic!("delivery exploded") });
    track(DeliveryLabel::DesktopNotification, async {
        panic!("notification exploded")
    });
    let survivor = Arc::new(AtomicBool::new(false));
    let flag = survivor.clone();
    track(route("after"), async move { flag.store(true, Ordering::SeqCst) });

    let outcome = drain_deliveries(Instant::now() + DELIVERY_DRAIN_BUDGET).await;

    assert_eq!(
        outcome.panicked,
        vec![route("boom"), DeliveryLabel::DesktopNotification]
    );
    assert!(outcome.pending.is_empty());
    assert!(survivor.load(Ordering::SeqCst), "a panic stopped the drain");
}

#[tokio::test(start_paused = true)]
async fn registering_prunes_finished_deliveries() {
    for index in 0..50 {
        let label = if index % 2 == 0 {
            route("alerts")
        } else {
            DeliveryLabel::DesktopNotification
        };
        track(label, async {});
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    // A finished panicking task is pruned too, not retained.
    track(route("boom"), async { panic!("delivery exploded") });
    tokio::time::sleep(Duration::from_millis(1)).await;
    track(route("last"), async {});

    assert!(
        tracked_count() <= 2,
        "finished deliveries were retained: {} entries",
        tracked_count()
    );
}

#[test]
fn tracking_without_a_runtime_neither_panics_nor_registers() {
    track(route("alerts"), async {});
    track(DeliveryLabel::DesktopNotification, async {});

    assert_eq!(tracked_count(), 0);
}

#[test]
fn labels_render_only_the_route_name_or_the_notification_label() {
    assert_eq!(route("alerts").to_string(), "route alerts");
    assert_eq!(
        DeliveryLabel::DesktopNotification.to_string(),
        "desktop notification"
    );
}

#[test]
fn the_pending_warning_names_each_delivery_and_says_delivery_is_unknown() {
    let one = DrainOutcome {
        pending: vec![route("alerts")],
        panicked: Vec::new(),
    };
    assert_eq!(
        one.warning_prose().as_deref(),
        Some("Route <blue-500>alerts</blue-500> was still sending at exit; delivery is unknown")
    );

    let several = DrainOutcome {
        pending: vec![
            DeliveryLabel::DesktopNotification,
            route("alerts"),
            route("<ops>"),
        ],
        panicked: Vec::new(),
    };
    assert_eq!(
        several.warning_prose().as_deref(),
        Some(
            "Desktop notification, route <blue-500>alerts</blue-500>, \
             route <blue-500>\\<ops\\></blue-500> were still sending at exit; \
             delivery is unknown"
        )
    );
}

#[test]
fn panics_alone_produce_no_pending_warning() {
    let outcome = DrainOutcome {
        pending: Vec::new(),
        panicked: vec![route("boom")],
    };
    assert_eq!(outcome.warning_prose(), None);
}
