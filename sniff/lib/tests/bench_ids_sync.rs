//! Keep `benches/ci-bench-ids.txt` and `benches/support/bench_ids.rs`
//! in lockstep.
//!
//! The text file drives `sniff-performance.yml` via shell tooling; the
//! Rust constant drives future bench validators and developer-facing
//! helpers. Drift between the two is how the original CI regression
//! described in the 2026-04-11 performance-testing review slipped in,
//! so this test asserts that both sources describe the exact same set
//! of IDs in the exact same order.

#[path = "../benches/support/bench_ids.rs"]
mod bench_ids;

use std::fs;
use std::path::PathBuf;

use bench_ids::{CI_BENCH_IDS, ci_filter_regex};

fn text_file_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("benches/ci-bench-ids.txt")
}

fn load_text_file_ids() -> Vec<String> {
    let path = text_file_path();
    let raw =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect()
}

#[test]
fn text_file_matches_rust_constant() {
    let text_ids = load_text_file_ids();
    let rust_ids: Vec<String> = CI_BENCH_IDS.iter().map(|s| (*s).to_string()).collect();
    assert_eq!(
        text_ids, rust_ids,
        "sniff/lib/benches/ci-bench-ids.txt and benches/support/bench_ids.rs must list the same IDs in the same order"
    );
}

#[test]
fn every_ci_bench_id_is_well_formed() {
    for id in CI_BENCH_IDS {
        assert!(
            id.contains('/'),
            "bench ID {id} should be '<group>/<name>'"
        );
        assert!(!id.starts_with('/'), "bench ID {id} starts with '/'");
        assert!(!id.ends_with('/'), "bench ID {id} ends with '/'");
        assert!(!id.contains(' '), "bench ID {id} contains whitespace");
    }
}

#[test]
fn ci_filter_regex_compiles_and_matches_exactly() {
    let filter = ci_filter_regex();
    let re =
        regex::Regex::new(&filter).unwrap_or_else(|e| panic!("invalid filter regex: {filter}: {e}"));

    for id in CI_BENCH_IDS {
        assert!(
            re.is_match(id),
            "filter regex should match {id}: regex={filter}"
        );
    }

    // Prefix-only matches must not leak through — anchors ensure the
    // filter does not accidentally co-match longer bench names.
    assert!(!re.is_match("system/detect_summary_extra"));
    assert!(!re.is_match("prefix_system/detect_summary"));
}
