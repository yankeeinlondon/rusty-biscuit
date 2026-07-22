//! Shared benchmark identifier constants.
//!
//! These are the canonical `<group>/<benchmark>` IDs referenced by the
//! narrow CI subset. The same list is mirrored in `ci-bench-ids.txt` at
//! the benches root so shell tooling (`cargo bench --`, GitHub Actions)
//! can use it without parsing Rust. A unit test in
//! `sniff/lib/tests/bench_ids_sync.rs` asserts the two stay in sync.
//!
//! The module is compiled from two places: the bench harness (where it
//! is only read by tooling and unit tests inside `mod tests`) and the
//! integration test at `sniff/lib/tests/bench_ids_sync.rs` (where every
//! exported item is exercised). `allow(dead_code)` therefore only
//! silences the bench-harness warnings without masking real regressions.
#![allow(dead_code)]

/// Canonical list of `<group>/<benchmark>` identifiers exercised by the
/// narrow CI benchmark subset. Each entry is a Criterion filter that
/// matches exactly one registered benchmark.
pub const CI_BENCH_IDS: &[&str] = &[
    "system/detect_summary",
    "system_full/detect_full",
    "filesystem_git/git_summary_monorepo",
    "filesystem_repo/repo_structure_monorepo",
    "inventory/programs_detect",
    "network/interfaces_only",
    // Representative git_ops rows for the gitoxide migration. High-cardinality
    // rows (status `1000`, `worktree_fanout`, deep revwalks, commit-graph
    // variants) stay opt-in and out of the CI subset.
    "git_ops/discover",
    "git_ops/status_dirty_flag/100",
    "git_ops/status_file_changes/100",
    "git_ops/request_levels/identity",
    "git_ops/request_levels/summary",
    "git_ops/config_read",
    // Migration-critical git_ops IDs (spec §5.5).
    "git_ops/revwalk_recent_gated/nograph",
    "git_ops/worktree_fanout/4",
    "workloads_service_listing/500",
];

/// Build a Criterion-compatible regex that matches every ID in
/// [`CI_BENCH_IDS`] and nothing else.
///
/// Criterion's CLI accepts a single positional filter, which it treats
/// as a `regex::Regex`. We join the IDs with `|` and anchor with `^`/`$`
/// so benchmarks whose names are prefixes of other benchmarks (e.g.
/// `system/detect_summary` vs `system/detect_summary_long`) are not
/// accidentally co-matched.
pub fn ci_filter_regex() -> String {
    let escaped: Vec<String> = CI_BENCH_IDS.iter().map(|id| regex_escape(id)).collect();
    format!("^({})$", escaped.join("|"))
}

/// Minimal regex metacharacter escaper.
///
/// Only the metacharacters that can appear in a bench ID (`.`, `/`, `_`,
/// digits, letters) matter here. `/` and `_` are literal; `.` is the
/// only metacharacter that regularly slips into group names. Escape the
/// full conservative set so future IDs do not silently regress.
fn regex_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 4);
    for ch in input.chars() {
        match ch {
            '.' | '+' | '*' | '?' | '(' | ')' | '|' | '[' | ']' | '{' | '}' | '^' | '$' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

// Unit tests for this module live in `sniff/lib/tests/bench_ids_sync.rs`,
// where the whole module is re-included via `#[path]`. That test file is
// the authoritative consumer — keeping an extra `#[cfg(test)] mod tests`
// here would just duplicate coverage and triggers spurious
// `unused_imports` warnings when the bench harness is compiled as a
// test target.
