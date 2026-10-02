//! Request inputs for unit tests.
//!
//! Production code receives its [`RequestSnapshot`] from the `claudine`
//! binary. A unit test stands in for that binary: it reads the test process
//! once, the way the binary would, so a test that never names its inputs
//! still runs against the process it was launched from.

use std::path::Path;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::RequestSnapshot;

/// The test process's directory, home, and environment.
pub(crate) fn snapshot() -> RequestSnapshot {
    RequestSnapshot::from_process().expect("read the test process")
}

/// [`snapshot`] anchored at `dir`.
pub(crate) fn snapshot_at(dir: impl AsRef<Path>) -> RequestSnapshot {
    snapshot().at_request_dir(dir.as_ref())
}

/// A launch context outside any repository (the system temporary directory).
///
/// Its fallback tree admits a test document wherever its fixture lives, as a
/// request launched beside that document would, and building it never walks
/// the checkout the tests run from.
pub(crate) fn context() -> FileResolutionContext {
    context_at(std::env::temp_dir())
}

/// Claudine's launch context for a request launched at `dir`.
pub(crate) fn context_at(dir: impl AsRef<Path>) -> FileResolutionContext {
    crate::composition::capture_file_resolution_context(&snapshot_at(dir))
        .expect("build the test context")
}

/// Claudine's launch context for a request launched in the directory of the
/// document at `path`, as a request with no caller-supplied context resolves
/// that document.
pub(crate) fn context_for(path: impl AsRef<Path>) -> FileResolutionContext {
    let path = path.as_ref();
    context_at(path.parent().unwrap_or(path))
}

/// [`snapshot`] read once, for a test that borrows it for the rest of the
/// test (a lookup that holds a reference).
pub(crate) fn process_snapshot() -> &'static RequestSnapshot {
    static SNAPSHOT: std::sync::OnceLock<RequestSnapshot> = std::sync::OnceLock::new();
    SNAPSHOT.get_or_init(snapshot)
}

/// [`context`] built once, for a test that borrows it.
pub(crate) fn process_context() -> &'static FileResolutionContext {
    static CONTEXT: std::sync::OnceLock<FileResolutionContext> = std::sync::OnceLock::new();
    CONTEXT.get_or_init(context)
}

/// The request context a document at `source_path` resolves its own
/// references in when its request names only `repo_root`: built at the
/// document's directory with that repository's topology. A root that does not
/// contain the document's directory is dropped, as a discovered one would be.
pub(crate) fn document_context(source_path: &Path, repo_root: Option<&Path>) -> FileResolutionContext {
    let base_dir = source_path.parent().unwrap_or(source_path);
    let repo_root = repo_root.filter(|root| base_dir.starts_with(root));
    let repo_info = repo_root
        .and_then(|root| sniff::filesystem::repo::detect_repo_structure(root).ok().flatten());
    crate::composition::build_prompt_resolution_context(&snapshot_at(base_dir), repo_root, repo_info.as_ref())
        .expect("build the document context")
}
