//! The process snapshot every `claudine` route resolves file references from.
//!
//! `dispatch` reads the process once ([`RequestSnapshot::from_process`]),
//! anchors the snapshot on the invocation's launch directory, and stores it
//! here before any command runs. Every invocation context, completion scope,
//! and hook dispatch builds its file-resolution context from this snapshot, so
//! no route reads the current directory, home directory, or environment again
//! to resolve a reference or seed `ctx.*`. See
//! `darkmatter/docs/topics/compose-requests.md`.

use std::path::Path;
use std::sync::OnceLock;

use darkmatter::markdown::compose::RequestSnapshot;

static LAUNCH_SNAPSHOT: OnceLock<RequestSnapshot> = OnceLock::new();

/// Captures the process snapshot, anchored at `launch_dir`.
///
/// `launch_dir` is the invocation's launch directory: the entry directory, or
/// for a provider hook the `AGENT_CWD` its wrapper supplied. Repeated calls
/// keep the first snapshot.
///
/// ## Errors
///
/// Returns the I/O error when the current directory cannot be read.
pub(crate) fn initialize(launch_dir: &Path) -> std::io::Result<&'static RequestSnapshot> {
    if let Some(snapshot) = LAUNCH_SNAPSHOT.get() {
        return Ok(snapshot);
    }
    let snapshot = RequestSnapshot::from_process()?.at_request_dir(launch_dir);
    Ok(LAUNCH_SNAPSHOT.get_or_init(|| snapshot))
}

/// The snapshot `dispatch` captured.
///
/// ## Panics
///
/// Panics when called before [`initialize`]: every route runs after
/// `dispatch` has captured the snapshot.
#[cfg(not(test))]
pub(crate) fn snapshot() -> &'static RequestSnapshot {
    LAUNCH_SNAPSHOT
        .get()
        .expect("the request snapshot is captured in `dispatch` before any command runs")
}

/// Unit tests reach routes without `dispatch`, so they capture the test
/// process on first use.
#[cfg(test)]
pub(crate) fn snapshot() -> &'static RequestSnapshot {
    LAUNCH_SNAPSHOT.get_or_init(|| RequestSnapshot::from_process().expect("read the test process"))
}

/// Claudine's launch context for a request launched at `dir`, from the test
/// process.
#[cfg(test)]
pub(crate) fn test_context_at(dir: impl AsRef<Path>) -> biscuit_file::FileResolutionContext {
    claudine::composition::capture_file_resolution_context(&snapshot().at_request_dir(dir.as_ref()))
        .expect("build the test context")
}

/// A launch context outside any repository (the system temporary directory),
/// whose fallback tree admits a test document wherever its fixture lives, as
/// a request launched beside that document would.
#[cfg(test)]
pub(crate) fn test_context() -> biscuit_file::FileResolutionContext {
    test_context_at(std::env::temp_dir())
}

/// [`test_context`] built once, for a test that borrows it.
#[cfg(test)]
pub(crate) fn test_context_ref() -> &'static biscuit_file::FileResolutionContext {
    static CONTEXT: OnceLock<biscuit_file::FileResolutionContext> = OnceLock::new();
    CONTEXT.get_or_init(test_context)
}
