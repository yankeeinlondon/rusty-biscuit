//! RAII ownership for the raw OS handles a platform wait loop creates.
//!
//! A Win32 `HANDLE` is a bare pointer newtype with no `Drop`, so a handle that
//! is merely held in a local is leaked on every return and every `?`. For a Job
//! Object that leak is not just a handle-table entry: `JOB_OBJECT_LIMIT_KILL_ON_
//! JOB_CLOSE` fires when the *last* handle closes, so a leaked handle silently
//! defers the kill of the child's surviving descendants to process exit.
//!
//! The closer is a trait parameter rather than a `cfg`-selected body so the
//! ownership bookkeeping — released exactly once, released on error paths,
//! released in declaration-reverse order relative to other guards — is
//! exercised by ordinary unit tests on every host, not only where the Win32
//! calls compile.

use std::marker::PhantomData;

/// Releases one raw OS handle.
pub(crate) trait HandleCloser {
    /// Called at most once per owned handle, and never with a null handle.
    fn close(raw: isize);
}

/// Owns a raw OS handle and releases it through `C` when it goes out of scope.
///
/// ## Notes
///
/// Declaration order is load-bearing wherever another guard publishes this
/// handle to a second thread: Rust drops locals in reverse declaration order,
/// so the publishing guard must be declared *after* the handle it publishes.
pub(crate) struct OwnedRawHandle<C: HandleCloser> {
    raw: isize,
    _closer: PhantomData<C>,
}

impl<C: HandleCloser> OwnedRawHandle<C> {
    pub(crate) fn new(raw: isize) -> Self {
        Self {
            raw,
            _closer: PhantomData,
        }
    }

    pub(crate) fn raw(&self) -> isize {
        self.raw
    }
}

impl<C: HandleCloser> Drop for OwnedRawHandle<C> {
    fn drop(&mut self) {
        if self.raw != 0 {
            C::close(self.raw);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    thread_local! {
        static CLOSED: RefCell<Vec<isize>> = const { RefCell::new(Vec::new()) };
    }

    struct RecordingCloser;

    impl HandleCloser for RecordingCloser {
        fn close(raw: isize) {
            CLOSED.with(|closed| closed.borrow_mut().push(raw));
        }
    }

    fn closed() -> Vec<isize> {
        CLOSED.with(|closed| closed.borrow().clone())
    }

    fn reset() {
        CLOSED.with(|closed| closed.borrow_mut().clear());
    }

    type Owned = OwnedRawHandle<RecordingCloser>;

    #[test]
    fn handle_is_closed_exactly_once_when_the_guard_drops() {
        reset();
        {
            let owned = Owned::new(0x1234);
            assert_eq!(owned.raw(), 0x1234);
            assert!(closed().is_empty(), "must not close while still in scope");
        }
        assert_eq!(closed(), vec![0x1234]);
    }

    /// The leak this guard exists to fix: the Job was created, then a `?` on the
    /// next Win32 call returned before any close could run.
    #[test]
    fn handle_is_closed_when_the_creating_function_returns_early() {
        reset();

        fn fallible(fail: bool) -> Result<isize, &'static str> {
            let owned = Owned::new(0xBEEF);
            if fail {
                return Err("setup failed");
            }
            Ok(owned.raw())
        }

        assert!(fallible(true).is_err());
        assert_eq!(closed(), vec![0xBEEF]);
    }

    #[test]
    fn a_null_handle_is_never_closed() {
        reset();
        drop(Owned::new(0));
        assert!(closed().is_empty());
    }

    /// The Windows wait loop publishes the Job handle to the console handler
    /// through a registration guard. The registration must deregister *before*
    /// the handle closes, or the handler can dereference a closed handle.
    /// Reverse-declaration drop order is what guarantees it, so the ordering is
    /// pinned here rather than left to a comment.
    #[test]
    fn a_later_declared_guard_releases_before_the_handle_closes() {
        reset();

        struct Registration;

        impl Drop for Registration {
            fn drop(&mut self) {
                CLOSED.with(|closed| closed.borrow_mut().push(-1));
            }
        }

        {
            let _job = Owned::new(0x77);
            let _registration = Registration;
        }

        assert_eq!(closed(), vec![-1, 0x77], "deregistration must come first");
    }
}
