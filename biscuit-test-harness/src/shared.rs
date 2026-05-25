//! Process-shared harness wrapper with at-exit cleanup.
//!
//! Real-terminal harnesses (WezTerm, Kitty, tmux, Apple Terminal) cost
//! 2–3 seconds per spawn. When a test binary holds many `#[serial]`
//! tests against the same harness, those tests reuse a single harness
//! stored in a `static`. Rust never runs `Drop` on `static` values, so
//! without an explicit hook the harness leaks its pane / window / session
//! at process exit.
//!
//! [`SharedHarness`] encapsulates that pattern: a `Mutex<Option<T>>`
//! protecting the singleton, plus a `libc::atexit` handler registered on
//! first spawn so the harness's `Drop` impl runs (which in turn shells
//! out to e.g. `wezterm cli kill-pane`).
//!
//! ## Usage
//!
//! ```ignore
//! use biscuit_test_harness::shared::SharedHarness;
//! use biscuit_test_harness::wezterm::WezTermHarness;
//!
//! static SHARED: SharedHarness<WezTermHarness> = SharedHarness::new();
//!
//! fn run_in_pane() {
//!     let mut guard = SHARED.get_or_init(|| {
//!         let mut h = WezTermHarness::new();
//!         h.spawn_shell().expect("spawn_shell");
//!         h
//!     });
//!     let harness = guard.as_mut().expect("harness present");
//!     // ... drive the harness ...
//! }
//! ```

use std::sync::{Mutex, MutexGuard, Once};

/// Process-shared harness slot with at-exit cleanup.
///
/// `SharedHarness<T>` is a `const`-constructible singleton wrapper. The
/// first call to [`get_or_init`](Self::get_or_init) constructs the
/// underlying harness and registers a `libc::atexit` handler that drops
/// it at process exit — so the harness's `Drop` impl runs even though
/// Rust normally skips `Drop` for `static` values.
///
/// Lock poisoning is recovered transparently: if a panicking test poisons
/// the inner mutex, subsequent callers still see the stored harness so
/// the next test can reset it (e.g. by sending a `clear` command).
///
/// ## Cleanup hook
///
/// The at-exit handler is registered exactly once per `SharedHarness`
/// instance, the first time `get_or_init` constructs the harness. The
/// hook is idempotent — if the slot has already been emptied by the
/// time the process exits, the handler is a no-op.
pub struct SharedHarness<T: Send + 'static> {
    slot: Mutex<Option<T>>,
    register: Once,
    cleanup_registered: std::sync::atomic::AtomicBool,
}

impl<T: Send + 'static> SharedHarness<T> {
    /// Construct an empty `SharedHarness`. Use as the initializer for a
    /// `static SHARED: SharedHarness<MyHarness> = SharedHarness::new();`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slot: Mutex::new(None),
            register: Once::new(),
            cleanup_registered: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Acquire the shared slot, constructing the harness via `init` if
    /// it is empty.
    ///
    /// The returned guard dereferences to `Option<T>` and is held until
    /// the caller drops it. After this method returns, the slot is
    /// guaranteed to be `Some` (because `init` either produced a `T` or
    /// panicked).
    ///
    /// On the first successful construction, an at-exit cleanup handler
    /// is registered so the harness's `Drop` impl runs at process exit.
    ///
    /// Lock poisoning is recovered transparently.
    pub fn get_or_init<F>(&'static self, init: F) -> SharedHarnessGuard<'static, T>
    where
        F: FnOnce() -> T,
    {
        let mut guard = self
            .slot
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if guard.is_none() {
            *guard = Some(init());
            self.ensure_atexit_cleanup();
        }
        SharedHarnessGuard { inner: guard }
    }

    /// Take the harness out of the slot, if present.
    ///
    /// Returns `Some(T)` if the slot was non-empty (the caller is
    /// responsible for dropping it), or `None` if it was already empty.
    /// Used by the at-exit hook; tests rarely need this directly.
    pub fn take(&self) -> Option<T> {
        let mut guard = self
            .slot
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        guard.take()
    }

    fn ensure_atexit_cleanup(&'static self) {
        self.register.call_once(|| {
            // Track the SharedHarness so the C handler can find it. We
            // use a per-instance trampoline so each SharedHarness gets
            // its own atexit slot.
            let ptr: *const SharedHarness<T> = self;
            let boxed = Box::new(AtexitCleanup {
                run: cleanup_trampoline::<T>,
                target: ptr as *const (),
            });
            // SAFETY: AtexitCleanup is leaked deliberately. The pointer
            // lives forever via a leaked Box; the trampoline reads
            // through it during process exit only.
            let leaked = Box::leak(boxed);
            let leaked_ptr = leaked as *mut AtexitCleanup as usize;
            // Stash a thread-local-free static slot. Because each
            // SharedHarness has its own Once + atomic, we need a
            // per-instance "current cleanup ptr" too. We use a per-instance
            // AtomicUsize stored alongside.
            CURRENT_CLEANUP.with_value(leaked_ptr, || {
                // SAFETY: handler matches `atexit`'s required signature.
                unsafe {
                    libc::atexit(atexit_dispatch);
                }
            });
            self.cleanup_registered
                .store(true, std::sync::atomic::Ordering::SeqCst);
        });
    }
}

impl<T: Send + 'static> Default for SharedHarness<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Guard returned by [`SharedHarness::get_or_init`].
///
/// Dereferences to `Option<T>`. The slot is guaranteed to be `Some`
/// immediately after `get_or_init` returns, but a caller can `take()`
/// the value out if needed.
pub struct SharedHarnessGuard<'a, T: Send + 'static> {
    inner: MutexGuard<'a, Option<T>>,
}

impl<T: Send + 'static> std::ops::Deref for SharedHarnessGuard<'_, T> {
    type Target = Option<T>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Send + 'static> std::ops::DerefMut for SharedHarnessGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

// ----------------------------------------------------------------------
// at-exit dispatch
//
// `libc::atexit` requires an `extern "C" fn()` with no arguments and no
// captures. To support multiple `SharedHarness` instances per process,
// we register one trampoline per instance via a small thread-local that
// `ensure_atexit_cleanup` populates just before the `atexit()` call.
// The dispatcher pops the pending cleanup at registration time and
// stores it in a global vec so the at-exit hook can find the right
// target.
// ----------------------------------------------------------------------

struct AtexitCleanup {
    run: unsafe extern "C" fn(*const ()),
    target: *const (),
}

// SAFETY: AtexitCleanup is only ever leaked and read via raw pointer
// during process exit. The target pointer points at a `'static`
// SharedHarness<T>, which is `Send + Sync` via its inner Mutex/AtomicBool.
unsafe impl Send for AtexitCleanup {}
unsafe impl Sync for AtexitCleanup {}

unsafe extern "C" fn cleanup_trampoline<T: Send + 'static>(target: *const ()) {
    // SAFETY: target was originally produced by `&SharedHarness<T> as
    // *const _`, so casting it back to that pointer type and dereferencing
    // is sound. The SharedHarness lives for `'static`.
    let shared = unsafe { &*(target as *const SharedHarness<T>) };
    // Cleanup must not unwind out of atexit. `Drop` for the harness
    // shells out to a process-killer command; treat any panic as
    // best-effort cleanup.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        drop(shared.take());
    }));
}

extern "C" fn atexit_dispatch() {
    // Drain every cleanup that was registered before any of them ran.
    // libc::atexit guarantees handlers run in reverse registration order,
    // so each instance's dispatcher runs once and finds exactly its own
    // leaked AtexitCleanup at the top of the queue.
    let ptr_usize = PENDING_CLEANUP_QUEUE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .pop();
    if let Some(ptr) = ptr_usize {
        // SAFETY: We control the pointer's lifetime — it was produced
        // from `Box::leak(Box::new(AtexitCleanup { ... }))` inside
        // `ensure_atexit_cleanup`.
        let cleanup = unsafe { &*(ptr as *const AtexitCleanup) };
        // SAFETY: `cleanup.run` was set to a properly-typed trampoline
        // for the original `SharedHarness<T>`.
        unsafe {
            (cleanup.run)(cleanup.target);
        }
    }
}

static PENDING_CLEANUP_QUEUE: Mutex<Vec<usize>> = Mutex::new(Vec::new());

/// Helper so `ensure_atexit_cleanup` can hand the per-instance pointer
/// to the global dispatcher without resorting to thread-locals (which
/// would not survive an atexit handler running on a different thread
/// than registration).
struct CurrentCleanup;

impl CurrentCleanup {
    fn with_value<R>(&self, ptr: usize, f: impl FnOnce() -> R) -> R {
        PENDING_CLEANUP_QUEUE
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(ptr);
        f()
    }
}

#[allow(non_upper_case_globals)]
static CURRENT_CLEANUP: CurrentCleanup = CurrentCleanup;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

    struct CountingHarness {
        _tag: &'static str,
    }

    impl Drop for CountingHarness {
        fn drop(&mut self) {
            DROP_COUNT.fetch_add(1, Ordering::SeqCst);
        }
    }

    static H1: SharedHarness<CountingHarness> = SharedHarness::new();

    #[test]
    fn shared_harness_initializes_on_first_get() {
        let init_count = std::sync::atomic::AtomicUsize::new(0);
        let guard = H1.get_or_init(|| {
            init_count.fetch_add(1, Ordering::SeqCst);
            CountingHarness { _tag: "first" }
        });
        assert!(guard.is_some());
        assert_eq!(init_count.load(Ordering::SeqCst), 1);
        drop(guard);

        // Second call must not re-run init.
        let guard = H1.get_or_init(|| {
            init_count.fetch_add(1, Ordering::SeqCst);
            CountingHarness { _tag: "second" }
        });
        assert!(guard.is_some());
        assert_eq!(init_count.load(Ordering::SeqCst), 1);
    }

    static H2: SharedHarness<CountingHarness> = SharedHarness::new();

    #[test]
    fn shared_harness_take_empties_slot() {
        let guard = H2.get_or_init(|| CountingHarness { _tag: "take-me" });
        assert!(guard.is_some());
        drop(guard);

        let taken = H2.take();
        assert!(taken.is_some());
        // After take, the slot is empty.
        assert!(H2.take().is_none());
    }
}
