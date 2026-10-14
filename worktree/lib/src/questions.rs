//! Repository reads `wt list` answers in-process when `gix` can, each named by
//! the Git command that answers it otherwise.
//!
//! In test builds (and with the `count-git` feature) every such read is
//! recorded here whichever way it is answered, and a
//! [`fail_matching`](crate::git::recorder::fail_matching) injection fails it,
//! so a test counts and fails reads, not Git processes.
//! [`crate::git::recorder`] still sees only the Git processes that run.

/// Records `args` and fails it when a
/// [`fail_matching`](crate::git::recorder::fail_matching) injection matches.
#[cfg(any(test, feature = "count-git"))]
pub(crate) fn ask(args: &[&str]) -> Result<(), ()> {
    if let Some(log) = LOG.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_mut() {
        log.push(args.iter().map(ToString::to_string).collect());
    }
    if crate::git::recorder::injected_failure(args) { Err(()) } else { Ok(()) }
}

#[cfg(not(any(test, feature = "count-git")))]
#[inline]
pub(crate) fn ask(_args: &[&str]) -> Result<(), ()> {
    Ok(())
}

#[cfg(any(test, feature = "count-git"))]
static LOG: std::sync::Mutex<Option<Vec<Vec<String>>>> = std::sync::Mutex::new(None);

/// Starts recording reads, from every thread, discarding any earlier
/// recording.
#[cfg(any(test, feature = "count-git"))]
pub fn start_recording() {
    *LOG.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Vec::new());
}

/// The reads asked since [`start_recording`], in order.
#[cfg(any(test, feature = "count-git"))]
pub fn finish_recording() -> Vec<Vec<String>> {
    LOG.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take().unwrap_or_default()
}
