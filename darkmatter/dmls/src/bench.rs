//! `dmls --bench-index <dir>`: the R-6 measurement harness.
//!
//! Ships with the first indexer slice so performance budgets are enforced from
//! day one rather than retrofitted. It runs the exact discovery + parse +
//! graph-build path the server uses, times each stage, and emits the R-6 JSON
//! shape (per-stage timings, peak RSS, graph counts) for `--json` or a compact
//! human summary otherwise.
//!
//! Timing granularity is per-stage wall clock captured around the same units
//! of work the `tracing` spans (`discover`, `read`, `hash`, `parse_markdown`,
//! `frontmatter`, `directives`, `graph_build`, `reverse_index`, `diagnostics`,
//! `snapshot_swap`) name; stages not exercised by the substrate slice report
//! zero so the shape is stable as later phases fill them in.

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;

use crate::config::WorkspaceConfig;
use crate::graph::WorkspaceIndex;
use crate::workspace::discover_workspace;
use crate::workspace::startup::{SilentProgress, collect_indices};

/// Per-stage wall-clock timings in milliseconds (R-6 stage names).
#[derive(Debug, Default, Serialize)]
pub struct StageTimings {
    /// Workspace discovery (walk + glob filtering).
    pub discover_ms: f64,
    /// Reading file contents from disk.
    pub read_ms: f64,
    /// Content hashing (xxHash identity).
    pub hash_ms: f64,
    /// Markdown parsing (headings, links).
    pub parse_markdown_ms: f64,
    /// Frontmatter block extraction.
    pub frontmatter_ms: f64,
    /// Directive scanning (overlay; zero in the substrate slice).
    pub directives_ms: f64,
    /// Graph node/edge construction.
    pub graph_build_ms: f64,
    /// Reverse-index construction.
    pub reverse_index_ms: f64,
    /// Diagnostics (zero in the substrate slice).
    pub diagnostics_ms: f64,
    /// Immutable snapshot swap.
    pub snapshot_swap_ms: f64,
}

/// Graph size counters for the indexed workspace.
#[derive(Debug, Default, Serialize)]
pub struct GraphCounts {
    /// Documents indexed.
    pub documents: usize,
    /// Total nodes.
    pub nodes: usize,
    /// Total edges.
    pub edges: usize,
}

/// The full R-6 bench report.
#[derive(Debug, Serialize)]
pub struct BenchReport {
    /// The directory that was indexed.
    pub root: PathBuf,
    /// Files discovered (after filtering).
    pub files: usize,
    /// Total wall-clock time for the whole index.
    pub total_ms: f64,
    /// Per-stage timings.
    pub stages: StageTimings,
    /// Graph size counters.
    pub counts: GraphCounts,
    /// Peak resident set size in bytes, when the platform exposes it.
    pub peak_rss_bytes: Option<u64>,
}

impl BenchReport {
    /// Serializes the report as pretty JSON (the `--json` shape).
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("BenchReport serializes")
    }

    /// A compact one-block human summary.
    pub fn to_summary(&self) -> String {
        let rss = match self.peak_rss_bytes {
            Some(bytes) => format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0)),
            None => "n/a".to_string(),
        };
        format!(
            "indexed {files} files in {total:.1} ms\n  \
             discover {discover:.1} · read {read:.1} · hash {hash:.1} · \
             parse {parse:.1} · frontmatter {fm:.1} · build {build:.1} · \
             reverse {rev:.1} · swap {swap:.1}\n  \
             graph: {docs} docs / {nodes} nodes / {edges} edges · peak RSS {rss}",
            files = self.files,
            total = self.total_ms,
            discover = self.stages.discover_ms,
            read = self.stages.read_ms,
            hash = self.stages.hash_ms,
            parse = self.stages.parse_markdown_ms,
            fm = self.stages.frontmatter_ms,
            build = self.stages.graph_build_ms,
            rev = self.stages.reverse_index_ms,
            swap = self.stages.snapshot_swap_ms,
            docs = self.counts.documents,
            nodes = self.counts.nodes,
            edges = self.counts.edges,
            rss = rss,
        )
    }
}

/// Runs the bench index over `root`.
pub fn bench_index(root: &Path) -> BenchReport {
    let config = WorkspaceConfig::default();
    let total_start = Instant::now();
    let mut stages = StageTimings::default();

    let discover_start = Instant::now();
    let report = discover_workspace(&[root.to_path_buf()], &config);
    stages.discover_ms = elapsed_ms(discover_start);

    // Read + hash + frontmatter + markdown parsing run across the worker pool —
    // the same path the server takes at startup, so the total reflects real
    // cold-start latency rather than a sequential worst case. The finer
    // per-span split (read vs hash vs parse) lands with the `tracing`-span
    // backing in later work; here they are folded into `parse_markdown_ms`.
    let parse_start = Instant::now();
    let indices = collect_indices(&[root.to_path_buf()], &config, &SilentProgress);
    stages.parse_markdown_ms = elapsed_ms(parse_start);

    let build_start = Instant::now();
    let index = WorkspaceIndex::from_indices(indices);
    stages.graph_build_ms = elapsed_ms(build_start);

    let snapshot = index.snapshot();
    let counts = GraphCounts {
        documents: snapshot.document_count(),
        nodes: snapshot.node_count(),
        edges: snapshot.edge_count(),
    };

    BenchReport {
        root: root.to_path_buf(),
        files: report.files.len(),
        total_ms: elapsed_ms(total_start),
        stages,
        counts,
        peak_rss_bytes: peak_rss_bytes(),
    }
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

/// Peak resident set size in bytes.
///
/// Uses `getrusage` on Unix (`ru_maxrss` — KiB on Linux, bytes on macOS);
/// returns `None` where unavailable (Windows here — a later pass can add the
/// `GetProcessMemoryInfo` path).
fn peak_rss_bytes() -> Option<u64> {
    #[cfg(unix)]
    {
        use std::mem::MaybeUninit;
        // SAFETY: `getrusage` fills the provided `rusage` struct and returns 0
        // on success; we only read `ru_maxrss` on success.
        let mut usage = MaybeUninit::<libc_rusage::rusage>::zeroed();
        let result = unsafe { libc_rusage::getrusage(libc_rusage::RUSAGE_SELF, usage.as_mut_ptr()) };
        if result != 0 {
            return None;
        }
        let usage = unsafe { usage.assume_init() };
        let max_rss = usage.ru_maxrss as u64;
        // Linux reports KiB; macOS reports bytes.
        if cfg!(target_os = "macos") {
            Some(max_rss)
        } else {
            Some(max_rss * 1024)
        }
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Minimal `getrusage` FFI so the bench can read peak RSS without pulling in
/// the whole `libc` crate as a dependency. Unix-only.
#[cfg(unix)]
mod libc_rusage {
    #![allow(non_camel_case_types)]

    pub const RUSAGE_SELF: i32 = 0;

    // The leading fields we read (`ru_maxrss`) are stable across Unix ABIs;
    // the tail is padded generously so the struct is at least as large as the
    // platform's real `rusage`.
    #[repr(C)]
    pub struct rusage {
        pub ru_utime: timeval,
        pub ru_stime: timeval,
        pub ru_maxrss: core::ffi::c_long,
        _tail: [core::ffi::c_long; 16],
    }

    #[repr(C)]
    pub struct timeval {
        pub tv_sec: core::ffi::c_long,
        pub tv_usec: core::ffi::c_long,
    }

    unsafe extern "C" {
        pub fn getrusage(who: i32, usage: *mut rusage) -> i32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn test_bench_report_counts_and_json_shape() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, "a.md", "# A\n\n[to b](b.md)\n");
        write(root, "b.md", "# B\n\n## Sub\n");

        let report = bench_index(root);
        assert_eq!(report.files, 2);
        assert_eq!(report.counts.documents, 2);
        assert!(report.counts.nodes >= 4); // 2 docs + headings + links
        assert!(report.total_ms >= 0.0);

        let json: serde_json::Value = serde_json::from_str(&report.to_json()).unwrap();
        assert_eq!(json["files"], 2);
        assert!(json["stages"]["graph_build_ms"].is_number());
        assert!(json["counts"]["edges"].is_number());
    }

    #[test]
    fn test_summary_renders() {
        let temp = tempfile::tempdir().unwrap();
        write(temp.path(), "a.md", "# A\n");
        let report = bench_index(temp.path());
        let summary = report.to_summary();
        assert!(summary.contains("indexed 1 files"));
        assert!(summary.contains("graph:"));
    }

    #[test]
    fn test_peak_rss_is_positive_where_available() {
        if let Some(bytes) = peak_rss_bytes() {
            assert!(bytes > 0);
        }
    }
}
