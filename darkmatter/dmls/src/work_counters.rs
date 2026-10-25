//! Process-wide counts of the file reads, content hashes, Markdown document
//! parses, schema-root enumerations, trigger-registry scans, and live-graph
//! path work DMLS performs.
//!
//! A work counter, like [`crate::context::context_build_count`]: a test takes
//! [`work_counts`] before and after the code under test and asserts the delta
//! with [`WorkCounts::since`], for example that a cached lookup reads nothing.
//! The counts are process-wide, which nextest's process-per-test model keeps
//! private to one test.
//!
//! Every DMLS file read, content hash, document parse, schema-root directory
//! enumeration, and trigger-registry scan goes through [`read`],
//! [`read_to_string`], [`hash_bytes`], [`record_parse`], [`record_enumeration`],
//! and [`record_registry_scan`]. The path-derived work of a graph build is
//! counted where it happens: each [`WikiUniverse`](crate::wiki::WikiUniverse)
//! constructed, each document's wiki path facts
//! ([`WikiDoc::for_path`](crate::wiki::WikiDoc::for_path)), each portability
//! key computed, and each graph document table constructed. Not counted: reads the darkmatter library
//! performs on DMLS's behalf (a registry scan's payload resolution among
//! them, which is why scans are counted), and hashes that only build an
//! in-memory cache key.
//!
//! [`registry_scan_threads`] also names every thread a registry scan ran on,
//! so a test can show that none ran on the server's dispatch thread, and
//! [`peak_concurrent_registry_scans`] is the most scans ever in progress at
//! once, so a test can show that no two overlapped.

use std::io;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::ThreadId;

static READS: AtomicUsize = AtomicUsize::new(0);
static HASHES: AtomicUsize = AtomicUsize::new(0);
static PARSES: AtomicUsize = AtomicUsize::new(0);
static ENUMERATIONS: AtomicUsize = AtomicUsize::new(0);
static REGISTRY_SCANS: AtomicUsize = AtomicUsize::new(0);
static WIKI_UNIVERSES: AtomicUsize = AtomicUsize::new(0);
static WIKI_PATH_FACTS: AtomicUsize = AtomicUsize::new(0);
static WIKI_PORTABILITY_KEYS: AtomicUsize = AtomicUsize::new(0);
static DOCUMENT_TABLES: AtomicUsize = AtomicUsize::new(0);
/// Distinct threads, so the set stays as small as the thread count.
static REGISTRY_SCAN_THREADS: Mutex<Vec<ThreadId>> = Mutex::new(Vec::new());
static REGISTRY_SCANS_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
static REGISTRY_SCANS_PEAK: AtomicUsize = AtomicUsize::new(0);

/// A snapshot of the process's work counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkCounts {
    /// File reads attempted, successful or not.
    pub reads: usize,
    /// Hashes of file or document content.
    pub hashes: usize,
    /// Markdown documents parsed into a document index.
    pub parses: usize,
    /// Schema-root directories listed.
    pub enumerations: usize,
    /// Trigger-registry scans (root classification and payload resolution).
    pub registry_scans: usize,
    /// Wiki universes constructed.
    pub wiki_universes: usize,
    /// Wiki path facts (canonical logical path and owning root) computed.
    pub wiki_path_facts: usize,
    /// Portability keys computed.
    pub wiki_portability_keys: usize,
    /// Graph document tables constructed.
    pub document_tables: usize,
}

impl WorkCounts {
    /// The work done between `earlier` and this snapshot.
    pub fn since(self, earlier: WorkCounts) -> WorkCounts {
        WorkCounts {
            reads: self.reads - earlier.reads,
            hashes: self.hashes - earlier.hashes,
            parses: self.parses - earlier.parses,
            enumerations: self.enumerations - earlier.enumerations,
            registry_scans: self.registry_scans - earlier.registry_scans,
            wiki_universes: self.wiki_universes - earlier.wiki_universes,
            wiki_path_facts: self.wiki_path_facts - earlier.wiki_path_facts,
            wiki_portability_keys: self.wiki_portability_keys - earlier.wiki_portability_keys,
            document_tables: self.document_tables - earlier.document_tables,
        }
    }
}

/// The counters' current values.
pub fn work_counts() -> WorkCounts {
    WorkCounts {
        reads: READS.load(Ordering::SeqCst),
        hashes: HASHES.load(Ordering::SeqCst),
        parses: PARSES.load(Ordering::SeqCst),
        enumerations: ENUMERATIONS.load(Ordering::SeqCst),
        registry_scans: REGISTRY_SCANS.load(Ordering::SeqCst),
        wiki_universes: WIKI_UNIVERSES.load(Ordering::SeqCst),
        wiki_path_facts: WIKI_PATH_FACTS.load(Ordering::SeqCst),
        wiki_portability_keys: WIKI_PORTABILITY_KEYS.load(Ordering::SeqCst),
        document_tables: DOCUMENT_TABLES.load(Ordering::SeqCst),
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

/// Counts one schema-root directory listing.
pub(crate) fn record_enumeration() {
    ENUMERATIONS.fetch_add(1, Ordering::SeqCst);
}

/// Counts one wiki universe constructed.
pub(crate) fn record_wiki_universe() {
    WIKI_UNIVERSES.fetch_add(1, Ordering::SeqCst);
}

/// Counts one document's wiki path facts computed.
pub(crate) fn record_wiki_path_facts() {
    WIKI_PATH_FACTS.fetch_add(1, Ordering::SeqCst);
}

/// Counts one portability key computed.
pub(crate) fn record_wiki_portability_key() {
    WIKI_PORTABILITY_KEYS.fetch_add(1, Ordering::SeqCst);
}

/// Counts one graph document table constructed.
pub(crate) fn record_document_table() {
    DOCUMENT_TABLES.fetch_add(1, Ordering::SeqCst);
}

/// A trigger-registry scan in progress, counted as in flight until dropped.
pub(crate) struct RegistryScan(());

impl Drop for RegistryScan {
    fn drop(&mut self) {
        REGISTRY_SCANS_IN_FLIGHT.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Counts one trigger-registry scan and the thread it runs on. The scan is
/// in flight until the returned guard is dropped.
pub(crate) fn record_registry_scan() -> RegistryScan {
    let thread = std::thread::current().id();
    {
        let mut threads = REGISTRY_SCAN_THREADS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if !threads.contains(&thread) {
            threads.push(thread);
        }
    }
    REGISTRY_SCANS.fetch_add(1, Ordering::SeqCst);
    let in_flight = REGISTRY_SCANS_IN_FLIGHT.fetch_add(1, Ordering::SeqCst) + 1;
    REGISTRY_SCANS_PEAK.fetch_max(in_flight, Ordering::SeqCst);
    RegistryScan(())
}

/// The most trigger-registry scans ever in progress at once.
pub fn peak_concurrent_registry_scans() -> usize {
    REGISTRY_SCANS_PEAK.load(Ordering::SeqCst)
}

/// Every thread a trigger-registry scan has run on, in first-scan order.
pub fn registry_scan_threads() -> Vec<ThreadId> {
    REGISTRY_SCAN_THREADS.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
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
        record_enumeration();
        drop(record_registry_scan());
        record_wiki_universe();
        record_wiki_path_facts();
        record_wiki_path_facts();
        record_wiki_portability_key();
        record_document_table();
        let middle = work_counts();
        assert_eq!(
            middle.since(before),
            WorkCounts {
                reads: 2,
                hashes: 1,
                parses: 1,
                enumerations: 1,
                registry_scans: 1,
                wiki_universes: 1,
                wiki_path_facts: 2,
                wiki_portability_keys: 1,
                document_tables: 1,
            }
        );
        // A later snapshot subtracts only what happened after `middle`.
        record_wiki_portability_key();
        record_document_table();
        assert_eq!(
            work_counts().since(middle),
            WorkCounts { wiki_portability_keys: 1, document_tables: 1, ..WorkCounts::default() }
        );
    }
}
