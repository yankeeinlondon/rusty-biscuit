//! The prepared request an integration test composes under.
//!
//! `request` keeps the request directory the pipeline chose before requests
//! were prepared explicitly (a file source's directory, else the context's
//! anchor, else a per-process temp directory), takes home from the process, and the
//! environment from the options' own context: empty unless the test put
//! values there, so no test sees the process environment by accident. A test
//! whose subject is the request directory builds its own [`RequestSnapshot`].

use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::{
    ComposeOptions, ComposeRequest, ComposeSource, RequestSnapshot, build_resolution_context,
};

pub fn request(options: ComposeOptions) -> ComposeRequest {
    let dir = request_dir(&options);
    request_at(&dir, options)
}

/// Like [`request`], but a source-less request is anchored at the process
/// directory, for tests whose subject is the ambient capture of a fixture they
/// entered with `set_current_dir`.
pub fn process_dir_request(options: ComposeOptions) -> ComposeRequest {
    let dir = match options.source() {
        ComposeSource::File(_) => request_dir(&options),
        _ if options.context().anchor().is_absolute() => options.context().anchor().to_path_buf(),
        _ => std::env::current_dir().expect("process directory"),
    };
    request_at(&dir, options)
}

/// A request anchored at `dir`, for options with no source of their own
/// (a reference graph builds one per document it visits).
pub fn request_at(dir: &Path, options: ComposeOptions) -> ComposeRequest {
    let snapshot = RequestSnapshot::new(dir)
        .with_home(biscuit_file::home_dir())
        .with_env(options.context().env().clone());
    ComposeRequest::prepare(options, &snapshot).expect("test request")
}

/// The context a test resolves through when its subject is not the request
/// directory: built at `dir` with the process's home and environment, so a
/// repository containing `dir` supplies the `&`/`^` roots.
pub fn context_at(dir: &Path) -> FileResolutionContext {
    let snapshot = RequestSnapshot::new(dir)
        .with_home(biscuit_file::home_dir())
        .with_env(biscuit_file::capture_env());
    build_resolution_context(&snapshot).expect("test context")
}

/// A source-less, anchor-less request is anchored in a per-process temp
/// directory, not the process directory: that is inside this monorepo, and
/// preparing a request there runs repository discovery over all of it.
fn request_dir(options: &ComposeOptions) -> PathBuf {
    static SCRATCH: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    let scratch = &SCRATCH;
    let process = || std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match options.source() {
        ComposeSource::File(path) => {
            let absolute = if path.is_absolute() { path.clone() } else { process().join(path) };
            absolute.parent().map(Path::to_path_buf).unwrap_or_else(process)
        }
        _ if options.context().anchor().is_absolute() => options.context().anchor().to_path_buf(),
        _ => scratch.get_or_init(|| tempfile::tempdir().expect("scratch request dir")).path().to_path_buf(),
    }
}

/// The context a context-free API used before one was required: anchored at
/// the process directory with the process's home and environment, and no
/// repository discovery. For tests whose documents carry no path and whose
/// assertions do not depend on where they resolve.
pub fn cwd_context() -> FileResolutionContext {
    FileResolutionContext::new(std::env::current_dir().expect("process directory"))
}
