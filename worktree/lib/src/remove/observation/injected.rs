//! An observer the test drives, so the deletion engine and the policy tests
//! never depend on host timing. Events go through the same bounded
//! [`EventQueue`] and [`Sink`] as a native adapter's.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use sniff::filesystem::query::EvidenceKind;

use super::{Backend, CoverageLost, EventKind, EventQueue, Observation, ObserverOptions, RawEvent, Root, Sink, Targets};

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
}

struct InjectedBackend {
    shared: Arc<Shared>,
}

impl Observation {
    /// A session whose events come only from the returned [`Injector`].
    pub(crate) fn injected(targets: Targets, options: ObserverOptions) -> (Self, Injector) {
        let shared = Arc::new(Shared {
            queue: EventQueue::new(targets.clone(), options.queue_capacity),
            delayed: Mutex::new(Vec::new()),
            handles: Mutex::new(Vec::new()),
            admin_released: AtomicBool::new(false),
            moved: AtomicBool::new(false),
            ended: AtomicBool::new(false),
        });
        let backend = InjectedBackend { shared: Arc::clone(&shared) };
        (Self::with_backend(targets, options, Box::new(backend)), Injector { shared })
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

    /// Whether the session was dropped.
    pub fn ended(&self) -> bool {
        self.shared.ended.load(Ordering::SeqCst)
    }
}

fn raw(root: Root, path: &Path, kind: EventKind) -> RawEvent {
    RawEvent { root, path: path.to_path_buf(), kind, directory: None, received: SystemTime::now() }
}

impl Backend for InjectedBackend {
    fn drain(&mut self, sink: &mut Sink<'_>) {
        self.shared.queue.take_into(sink);
    }

    fn moved_to_quarantine(&mut self) {
        self.shared.moved.store(true, Ordering::SeqCst);
    }

    fn release_admin(&mut self) {
        self.shared.admin_released.store(true, Ordering::SeqCst);
    }

    fn handles(&self) -> Vec<(EvidenceKind, u64)> {
        self.shared.handles.lock().unwrap().clone()
    }
}

impl Drop for InjectedBackend {
    fn drop(&mut self) {
        self.shared.ended.store(true, Ordering::SeqCst);
    }
}
