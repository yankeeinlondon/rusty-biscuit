//! `test-fixtures` seam: a recording preparer selected from outside the
//! process.
//!
//! The CLI's tests cannot reach the crate-private fakes, and a real
//! preparation would synthesize speech. With [`TEST_PREWARM_ENV`] set to
//! `record:<path>` every preparation appends one line to `<path>` and
//! succeeds: its text, a tab, and its playback configuration serialized as
//! JSON; with `block:<path>` it appends and then never
//! completes, so a test can prove launch does not wait on it. Any other value,
//! or none, leaves the production preparer in place. The gates still apply:
//! the recorder only replaces what a started worker calls.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use biscuit_speaks::TtsConfig;

use super::Preparer;
use super::preparer::{PrepareFuture, PrewarmError};

#[cfg(feature = "test-fixtures")]
const TEST_PREWARM_ENV: &str = "CLAUDINE_TEST_PREWARM";

/// The recorder [`TEST_PREWARM_ENV`] selects, if any.
pub(super) fn recording_preparer() -> Option<Arc<dyn Preparer>> {
    let value = std::env::var(TEST_PREWARM_ENV).ok()?;
    let (blocks, path) = match value.split_once(':')? {
        ("record", path) => (false, path),
        ("block", path) => (true, path),
        _ => return None,
    };
    Some(Arc::new(FileRecorder {
        path: PathBuf::from(path),
        blocks,
    }))
}

struct FileRecorder {
    path: PathBuf,
    blocks: bool,
}

impl Preparer for FileRecorder {
    fn prepare(&self, text: String, config: TtsConfig) -> PrepareFuture {
        let config = serde_json::to_string(&config).unwrap_or_default();
        let recorded = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .and_then(|mut file| file.write_all(format!("{text}\t{config}\n").as_bytes()));
        let blocks = self.blocks;
        Box::pin(async move {
            if blocks {
                std::future::pending::<()>().await;
            }
            recorded.map_err(|_| PrewarmError::Prepare)
        })
    }
}
