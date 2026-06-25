mod artifact_tests;
mod cache_tests;
#[cfg(feature = "image")]
mod graph_tests;
#[cfg(feature = "image")]
mod mermaid_tests;
#[cfg(feature = "image")]
mod raster_tests;

#[cfg(feature = "image")]
use tempfile::TempDir;

#[cfg(feature = "image")]
use crate::cache::file_cache::CACHE_DIR_ENV;

/// Redirects the process-global render cache into a fresh temp directory.
///
/// `GraphDiagram::render` / `MermaidDiagram::render` build a [`FileCache`] via
/// `FileCache::new`, which reads [`CACHE_DIR_ENV`]. Pointing that env var at a
/// per-test directory isolates each render test from the shared
/// `$TMPDIR/biscuit-visualized/v1/` location. Callers must be `#[serial]`:
/// the env var is process-global, so concurrent threads would clobber it.
///
/// The returned [`TempDir`] must outlive the render calls; dropping it deletes
/// the directory.
///
/// [`FileCache`]: crate::cache::FileCache
#[cfg(feature = "image")]
fn isolated_cache_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    std::env::set_var(CACHE_DIR_ENV, dir.path());
    dir
}
