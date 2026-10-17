//! Process-wide counts of the file reads, content hashes, and Markdown
//! document parses DMLS performs.
//!
//! A work counter, like [`crate::context::context_build_count`]: a test takes
//! [`work_counts`] before and after the code under test and asserts the delta
//! with [`WorkCounts::since`], for example that a cached lookup reads nothing.
//! The counts are process-wide, which nextest's process-per-test model keeps
//! private to one test.
//!
//! Every DMLS file read, content hash, and document parse goes through
//! [`read`], [`read_to_string`], [`hash_bytes`], and [`record_parse`]. Not
//! counted: reads the darkmatter library performs on DMLS's behalf, and hashes
//! that only build an in-memory cache key.

use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

static READS: AtomicUsize = AtomicUsize::new(0);
static HASHES: AtomicUsize = AtomicUsize::new(0);
static PARSES: AtomicUsize = AtomicUsize::new(0);

/// A snapshot of the process's work counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkCounts {
    /// File reads attempted, successful or not.
    pub reads: usize,
    /// Hashes of file or document content.
    pub hashes: usize,
    /// Markdown documents parsed into a document index.
    pub parses: usize,
}

impl WorkCounts {
    /// The work done between `earlier` and this snapshot.
    pub fn since(self, earlier: WorkCounts) -> WorkCounts {
        WorkCounts {
            reads: self.reads - earlier.reads,
            hashes: self.hashes - earlier.hashes,
            parses: self.parses - earlier.parses,
        }
    }
}

/// The counters' current values.
pub fn work_counts() -> WorkCounts {
    WorkCounts {
        reads: READS.load(Ordering::SeqCst),
        hashes: HASHES.load(Ordering::SeqCst),
        parses: PARSES.load(Ordering::SeqCst),
    }
}

/// [`std::fs::read`], counted.
pub(crate) fn read(path: &Path) -> io::Result<Vec<u8>> {
    READS.fetch_add(1, Ordering::SeqCst);
    std::fs::read(path)
}

/// [`std::fs::read_to_string`], counted.
pub(crate) fn read_to_string(path: &Path) -> io::Result<String> {
    READS.fetch_add(1, Ordering::SeqCst);
    std::fs::read_to_string(path)
}

/// [`biscuit_hash::xx_hash_bytes`], counted.
pub(crate) fn hash_bytes(bytes: &[u8]) -> u64 {
    HASHES.fetch_add(1, Ordering::SeqCst);
    biscuit_hash::xx_hash_bytes(bytes)
}

/// Counts one Markdown document parse.
pub(crate) fn record_parse() {
    PARSES.fetch_add(1, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_helper_counts_its_own_work() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.md");
        std::fs::write(&file, "# A\n").unwrap();
        let before = work_counts();
        let bytes = read(&file).unwrap();
        let _ = read_to_string(&dir.path().join("missing.md"));
        assert_eq!(hash_bytes(&bytes), biscuit_hash::xx_hash_bytes(b"# A\n"));
        record_parse();
        assert_eq!(work_counts().since(before), WorkCounts { reads: 2, hashes: 1, parses: 1 });
    }
}
