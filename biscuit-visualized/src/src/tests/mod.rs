mod artifact_tests;
#[cfg(feature = "image")]
mod bundled_font_tests;
#[cfg(feature = "image")]
mod cache_identity_tests;
mod cache_tests;
#[cfg(feature = "image")]
mod contrast_corpus_tests;
#[cfg(feature = "image")]
mod corpus_measure_tests;
#[cfg(feature = "image")]
mod embed_comment_tests;
#[cfg(feature = "image")]
mod embed_embedding_tests;
#[cfg(feature = "image")]
mod embed_faces_tests;
#[cfg(feature = "image")]
mod embed_render_tests;
#[cfg(feature = "image")]
mod embed_glyphs_tests;
#[cfg(feature = "image")]
mod embed_permission_tests;
#[cfg(feature = "image")]
mod embed_subset_tests;
#[cfg(feature = "image")]
mod font_chain_tests;
#[cfg(feature = "image")]
mod gitgraph_layout_tests;
#[cfg(feature = "image")]
mod gitgraph_tests;
#[cfg(feature = "image")]
mod golden_tests;
#[cfg(feature = "image")]
mod graph_tests;
#[cfg(feature = "image")]
mod html_static_tests;
#[cfg(feature = "image")]
mod measure_tests;
#[cfg(feature = "image")]
mod mermaid_cache_key_tests;
#[cfg(feature = "image")]
mod mermaid_family_tests;
#[cfg(feature = "image")]
mod mermaid_tests;
#[cfg(feature = "image")]
mod newline_tests;
#[cfg(feature = "image")]
mod palette_tests;
#[cfg(feature = "image")]
mod postprocess_tests;
#[cfg(feature = "image")]
mod raster_tests;
#[cfg(feature = "image")]
mod theme_tests;

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
pub(crate) fn isolated_cache_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    // SAFETY: callers are `#[serial]` (see above), so no other thread reads
    // or writes the environment meanwhile.
    unsafe { std::env::set_var(CACHE_DIR_ENV, dir.path()) };
    dir
}

/// Runs `run` with a subscriber that records every `WARN` event's message,
/// for tests that assert a warning is logged.
#[cfg(feature = "image")]
pub(crate) fn capture_warnings<T>(run: impl FnOnce() -> T) -> (T, Vec<String>) {
    capture_logs(tracing::Level::WARN, run)
}

/// Runs `run` with a subscriber that records the message of every event at
/// `level` (no `tracing-subscriber` dependency). The subscriber is the
/// thread's default only while `run` runs.
#[cfg(feature = "image")]
pub(crate) fn capture_logs<T>(level: tracing::Level, run: impl FnOnce() -> T) -> (T, Vec<String>) {
    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing::span::{Attributes, Id, Record};
    use tracing::{Event, Level, Metadata};

    struct Message(String);

    impl Visit for Message {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.0 = format!("{value:?}");
            }
        }
    }

    struct Recorder(Level, Arc<Mutex<Vec<String>>>);

    impl tracing::Subscriber for Recorder {
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &Attributes<'_>) -> Id {
            Id::from_u64(1)
        }
        fn record(&self, _: &Id, _: &Record<'_>) {}
        fn record_follows_from(&self, _: &Id, _: &Id) {}
        fn event(&self, event: &Event<'_>) {
            if *event.metadata().level() == self.0 {
                let mut message = Message(String::new());
                event.record(&mut message);
                self.1.lock().expect("log lock").push(message.0);
            }
        }
        fn enter(&self, _: &Id) {}
        fn exit(&self, _: &Id) {}
    }

    let messages = Arc::new(Mutex::new(Vec::new()));
    let result = tracing::subscriber::with_default(Recorder(level, Arc::clone(&messages)), run);
    let messages = messages.lock().expect("log lock").clone();
    (result, messages)
}
