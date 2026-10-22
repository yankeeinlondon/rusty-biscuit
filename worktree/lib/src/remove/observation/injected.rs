//! An observer the test drives, so the deletion engine and the policy tests
//! never depend on host timing. Events go through the same bounded
//! [`EventQueue`] and [`Sink`] as a native adapter's.
//!
//! A *confirming* observer ([`Observation::injected_confirming`]) also waits
//! like an adapter: every drain, the move's labeling, and the admin release
//! await a marker through the adapters' own wait loop, on a virtual clock that
//! jumps to the next [`Scripted`] delivery instead of sleeping.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use sniff::filesystem::query::EvidenceKind;

use super::diagnostics::RoundTrips;
use super::wait::{Delivery, Outcome, Source, WaitPolicy, WaitState, await_marker};
use super::{Backend, CoverageLost, EventKind, EventQueue, Observation, ObservationBackend, ObserverOptions, RawEvent, Root, Sink, Targets, Waited};

/// The test's side of an injected observation.
#[derive(Debug, Clone)]
pub(crate) struct Injector {
    shared: Arc<Shared>,
}

#[derive(Debug)]
struct Shared {
    queue: EventQueue,
    delayed: Mutex<Vec<RawEvent>>,
    handles: Mutex<Vec<(EvidenceKind, u64)>>,
    admin_released: AtomicBool,
    moved: AtomicBool,
    ended: AtomicBool,
    drains: AtomicUsize,
    round_trips: Mutex<RoundTrips>,
    /// `Some` for a confirming observer.
    confirming: Option<Confirming>,
}

/// A delivery the virtual clock makes at its scheduled time.
#[derive(Debug, Clone)]
pub(crate) enum Scripted {
    /// The marker of the wait in progress arrives.
    Acknowledge,
    /// An event arrives; progress only when it is in the session's scope.
    Event(Root, PathBuf, EventKind),
    Overflow,
    /// The test's interrupt flag ([`Injector::interrupted`]) is set.
    Interrupt,
}

#[derive(Debug)]
struct Confirming {
    policy: WaitPolicy,
    started: Instant,
    /// Virtual time since `started`.
    clock: Mutex<Duration>,
    /// Pending deliveries by time, earliest first.
    script: Mutex<Vec<(Duration, Scripted)>>,
    delivery: Delivery<()>,
    /// The marker the wait in progress awaits.
    marker: AtomicU64,
    interrupt: AtomicBool,
}

struct InjectedBackend {
    shared: Arc<Shared>,
}

impl Observation {
    /// A session whose events come only from the returned [`Injector`].
    pub(crate) fn injected(targets: Targets, options: ObserverOptions) -> (Self, Injector) {
        Self::injected_with(targets, options, None)
    }

    /// [`Observation::injected`] whose waits run under `options.wait` on a
    /// virtual clock: each awaits a marker only a [`Scripted::Acknowledge`]
    /// delivers.
    pub(crate) fn injected_confirming(targets: Targets, options: ObserverOptions) -> (Self, Injector) {
        let confirming = Confirming {
            policy: options.wait,
            started: Instant::now(),
            clock: Mutex::new(Duration::ZERO),
            script: Mutex::new(Vec::new()),
            delivery: Delivery::new([()]),
            marker: AtomicU64::new(0),
            interrupt: AtomicBool::new(false),
        };
        Self::injected_with(targets, options, Some(confirming))
    }

    fn injected_with(targets: Targets, options: ObserverOptions, confirming: Option<Confirming>) -> (Self, Injector) {
        let shared = Arc::new(Shared {
            queue: EventQueue::new(targets.clone(), options.queue_capacity),
            delayed: Mutex::new(Vec::new()),
            handles: Mutex::new(Vec::new()),
            admin_released: AtomicBool::new(false),
            moved: AtomicBool::new(false),
            ended: AtomicBool::new(false),
            drains: AtomicUsize::new(0),
            round_trips: Mutex::new(RoundTrips::default()),
            confirming,
        });
        let backend = InjectedBackend { shared: Arc::clone(&shared) };
        (Self::with_backend(targets, options, Box::new(backend)), Injector { shared })
    }

    /// [`Observation::injected`] for a tree already at the quarantine.
    pub(crate) fn injected_at_quarantine(targets: Targets, options: ObserverOptions) -> (Self, Injector) {
        let (mut observation, injector) = Self::injected(targets, options);
        observation.moved = true;
        (observation, injector)
    }
}

impl Injector {
    /// Delivers an event by the next drain.
    /// Like a native adapter, it drops admin events once the admin directory
    /// is released.
    pub fn event(&self, root: Root, path: impl AsRef<Path>, kind: EventKind) {
        if !(root == Root::Admin && self.admin_released()) {
            self.shared.queue.push(raw(root, path.as_ref(), kind));
        }
    }

    /// Holds an event back until [`Injector::deliver_delayed`], as a slow
    /// platform would.
    pub fn delayed(&self, root: Root, path: impl AsRef<Path>, kind: EventKind) {
        self.shared.delayed.lock().unwrap().push(raw(root, path.as_ref(), kind));
    }

    pub fn deliver_delayed(&self) {
        for event in self.shared.delayed.lock().unwrap().drain(..) {
            self.shared.queue.push(event);
        }
    }

    pub fn overflow(&self) {
        self.shared.queue.lose(CoverageLost::Overflow);
    }

    pub fn lose_watch(&self, root: Root) {
        if !(root == Root::Admin && self.admin_released()) {
            self.shared.queue.lose(CoverageLost::WatchLost { root });
        }
    }

    /// A handle the session reports holding.
    pub fn hold_handle(&self, kind: EvidenceKind, descriptor: u64) {
        self.shared.handles.lock().unwrap().push((kind, descriptor));
    }

    pub fn admin_released(&self) -> bool {
        self.shared.admin_released.load(Ordering::SeqCst)
    }

    pub fn moved(&self) -> bool {
        self.shared.moved.load(Ordering::SeqCst)
    }

    /// How many times the session was drained.
    pub fn drains(&self) -> usize {
        self.shared.drains.load(Ordering::SeqCst)
    }

    /// Whether the session was dropped.
    pub fn ended(&self) -> bool {
        self.shared.ended.load(Ordering::SeqCst)
    }

    fn confirming(&self) -> &Confirming {
        self.shared.confirming.as_ref().expect("a confirming observer")
    }

    /// Schedules `delivery` at virtual time `at` since the session started.
    pub fn at(&self, at: Duration, delivery: Scripted) {
        let mut script = self.confirming().script.lock().unwrap();
        script.push((at, delivery));
        script.sort_by_key(|(at, _)| *at);
    }

    /// Virtual time since the session started.
    pub fn elapsed(&self) -> Duration {
        *self.confirming().clock.lock().unwrap()
    }

    /// Whether a [`Scripted::Interrupt`] has been delivered; the test's
    /// interrupt predicate.
    pub fn interrupted(&self) -> bool {
        self.confirming().interrupt.load(Ordering::SeqCst)
    }
}

/// The virtual clock: parking never blocks, it moves time to the next
/// scheduled delivery or to the end of the timeout, whichever is first.
struct Virtual<'a> {
    confirming: &'a Confirming,
    queue: &'a EventQueue,
}

impl Source<()> for Virtual<'_> {
    fn now(&self) -> Instant {
        self.confirming.started + *self.confirming.clock.lock().unwrap()
    }

    fn generation(&self) -> u64 {
        self.confirming.delivery.generation()
    }

    fn delivered(&self, key: ()) -> super::wait::Delivered {
        self.confirming.delivery.delivered(key)
    }

    fn park(&self, seen: u64, timeout: Duration) {
        if self.generation() != seen {
            return;
        }
        let mut clock = self.confirming.clock.lock().unwrap();
        let mut script = self.confirming.script.lock().unwrap();
        let next = script.first().map_or(Duration::MAX, |(at, _)| *at);
        *clock = (*clock).max((*clock + timeout).min(next));
        while script.first().is_some_and(|(at, _)| *at <= *clock) {
            match script.remove(0).1 {
                Scripted::Acknowledge => self.confirming.delivery.acknowledge((), self.confirming.marker.load(Ordering::SeqCst)),
                Scripted::Event(root, path, kind) => {
                    if self.queue.push(raw(root, &path, kind)) {
                        self.confirming.delivery.event(());
                    }
                }
                Scripted::Overflow => self.queue.lose(CoverageLost::Overflow),
                Scripted::Interrupt => self.confirming.interrupt.store(true, Ordering::SeqCst),
            }
        }
    }
}

impl Shared {
    /// A confirming observer's wait for its next marker; a stall or the
    /// ceiling is recorded as loss. Anything else waits for nothing.
    fn confirm(&self, interrupted: &dyn Fn() -> bool) -> Waited {
        let Some(confirming) = &self.confirming else {
            return Waited::Done;
        };
        let marker = confirming.marker.fetch_add(1, Ordering::SeqCst) + 1;
        let source = Virtual { confirming, queue: &self.queue };
        let mut wait = WaitState::new(confirming.policy, source.now(), [()]);
        match await_marker(&source, &mut wait, marker, &|| self.queue.has_loss(), interrupted) {
            Outcome::Confirmed { round_trip } => {
                self.round_trips.lock().unwrap().record(round_trip);
                Waited::Done
            }
            Outcome::Lost => Waited::Done,
            Outcome::Interrupted => Waited::Interrupted,
            Outcome::Stalled { waited, .. } => {
                self.queue.lose(CoverageLost::DeliveryStalled { waited, watch: None });
                Waited::Done
            }
            Outcome::Ceiling { waited, .. } => {
                self.queue.lose(CoverageLost::DrainCeiling { waited, watch: None });
                Waited::Done
            }
        }
    }
}

fn raw(root: Root, path: &Path, kind: EventKind) -> RawEvent {
    RawEvent { root, path: path.to_path_buf(), kind, directory: None, received: SystemTime::now() }
}

impl Backend for InjectedBackend {
    fn drain(&mut self, sink: &mut Sink<'_>, interrupted: &dyn Fn() -> bool) -> Waited {
        self.shared.drains.fetch_add(1, Ordering::SeqCst);
        let waited = if sink.lost.is_some() { Waited::Done } else { self.shared.confirm(interrupted) };
        self.shared.queue.take_into(sink);
        waited
    }

    fn moved_to_quarantine(&mut self, interrupted: &dyn Fn() -> bool) -> Waited {
        let waited = self.shared.confirm(interrupted);
        self.shared.moved.store(true, Ordering::SeqCst);
        waited
    }

    fn release_admin(&mut self, interrupted: &dyn Fn() -> bool) -> Waited {
        let waited = self.shared.confirm(interrupted);
        if waited == Waited::Done {
            self.shared.admin_released.store(true, Ordering::SeqCst);
        }
        waited
    }

    fn queued_loss(&self) -> Option<CoverageLost> {
        self.shared.queue.loss()
    }

    fn handles(&self) -> Vec<(EvidenceKind, u64)> {
        self.shared.handles.lock().unwrap().clone()
    }

    fn backend(&self) -> ObservationBackend {
        ObservationBackend::NATIVE
    }

    fn round_trips(&self) -> RoundTrips {
        *self.shared.round_trips.lock().unwrap()
    }
}

impl Drop for InjectedBackend {
    fn drop(&mut self) {
        self.shared.ended.store(true, Ordering::SeqCst);
    }
}
