//! Corpus guard for every production `run_harness_loop` call site
//! (`features/2026-07-13-proxy-with`, review-5 Finding 1).
//!
//! `run_harness_loop` now returns a 4-element `HarnessLoopResult`
//! `(i32, Option<AgentExecutionPerf>, Option<IterationSummarySignals>,
//! Option<SurfacedHandoff>)` and takes a trailing
//! `handoff_ledger: Option<SharedRunLedger>` before `emit_prompt_timing`.
//! A signature change that misses a call site is how Finding 1 slipped in
//! (one site kept a 3-tuple destructure and omitted `handoff_ledger`, which
//! compiled far enough to reach `cargo check` but broke `claudine-cli`).
//!
//! The compiler already rejects an arity mismatch; this guard adds the piece
//! the compiler cannot: it **enumerates** the call sites. A new or removed
//! call site fails this test, forcing a conscious update that threads
//! `handoff_ledger` and consumes the surfaced handoff (whether by propagating
//! it or explicitly rejecting an impossible one). It also asserts each call
//! destructures the full 4-element tuple, so a site cannot silently drop the
//! surfaced handoff by binding a shorter tuple.
//!
//! ## Live-branch reachability (review-5 Finding 2)
//!
//! Threading the ledger *type* is not the same as reaching the live
//! `Some(ledger)` branch. The endpoint census above proves the types line up;
//! [`build_execution_request_populates_the_handoff_ledger`] proves the compose
//! path actually constructs a `CompositionExecutionRequest` with
//! `handoff_ledger: Some(...)`. Without it, `surface_or_adopt_terminal_proxy`
//! always takes the `None` (adopt-in-harness) arm and a terminal-event proxy is
//! never surfaced to the command coordinator. The guard fails if a refactor
//! reverts `build_execution_request` to an always-`None` field.

use std::fs;
use std::path::{Path, PathBuf};

/// Production call sites, relative to `claudine/cli/src`. Both thread
/// `handoff_ledger` and destructure the 4-tuple; update this set (and the
/// callers) whenever a `run_harness_loop` call site is added or removed.
const EXPECTED_CALL_SITES: &[&str] = &[
    "commands/wrap/composition/runner.rs",
    "commands/wrap/wrapper_stages.rs",
];

fn cli_src_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// Count the elements of a tuple pattern given the byte index of its opening
/// `(`. Returns `commas_at_depth_1 + 1`. Tuple *patterns* carry no string or
/// comment tokens, so a plain paren-depth walk is exact here.
fn tuple_elem_count(bytes: &[u8], open_paren: usize) -> usize {
    let mut depth = 0usize;
    let mut commas = 0usize;
    for &b in &bytes[open_paren..] {
        match b {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            b',' if depth == 1 => commas += 1,
            _ => {}
        }
    }
    commas + 1
}

/// True when the occurrence at `idx` is the `fn run_harness_loop(` definition
/// rather than a call.
fn is_definition(content: &str, idx: usize) -> bool {
    content[..idx].trim_end().ends_with("fn")
}

/// True when a `//` line comment opens before `idx` on the same line.
fn is_commented(content: &str, idx: usize) -> bool {
    let line_start = content[..idx].rfind('\n').map_or(0, |p| p + 1);
    content[line_start..idx].contains("//")
}

#[test]
fn every_run_harness_loop_call_site_is_accounted_for() {
    let src_root = cli_src_root();
    let mut files = Vec::new();
    collect_rs_files(&src_root, &mut files);
    assert!(
        !files.is_empty(),
        "no .rs files under {} — scan root is broken",
        src_root.display()
    );

    let needle = "run_harness_loop(";
    let mut found: Vec<String> = Vec::new();

    for file in &files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
        let bytes = content.as_bytes();
        let rel = file
            .strip_prefix(&src_root)
            .expect("scanned file lives under src root")
            .to_string_lossy()
            .replace('\\', "/");

        let mut search_from = 0usize;
        while let Some(rel_idx) = content[search_from..].find(needle) {
            let idx = search_from + rel_idx;
            search_from = idx + needle.len();
            if is_definition(&content, idx) || is_commented(&content, idx) {
                continue;
            }

            // The call must destructure the full 4-element `HarnessLoopResult`
            // (the 4th element is the surfaced proxy handoff): find the
            // preceding `let (` tuple pattern and count its elements.
            let head = &content[..idx];
            let let_at = head
                .rfind("let (")
                .unwrap_or_else(|| panic!("{rel}: run_harness_loop call is not a `let (...) =` destructure"));
            let tuple_open = let_at + "let ".len();
            let elems = tuple_elem_count(bytes, tuple_open);
            assert_eq!(
                elems, 4,
                "{rel}: run_harness_loop must destructure a 4-element tuple \
                 (i32, perf, signals, surfaced_handoff); found {elems}"
            );

            found.push(rel.clone());
        }
    }

    found.sort();
    found.dedup();

    let mut expected: Vec<String> = EXPECTED_CALL_SITES.iter().map(|s| s.to_string()).collect();
    expected.sort();

    assert_eq!(
        found, expected,
        "run_harness_loop call sites drifted from the guard.\n\
         found:    {found:?}\n\
         expected: {expected:?}\n\
         A new or removed call site must thread `handoff_ledger` + \
         `emit_prompt_timing` and consume the surfaced handoff; update \
         EXPECTED_CALL_SITES once the caller is correct."
    );
}

/// True when the occurrence at `idx` falls inside an inline `#[cfg(test)]`
/// module. Coarse but sufficient here: `build_execution_request` lives in
/// production module scope, and `prep.rs`'s test module is a separate
/// `mod tests;` file (excluded via the `tests.rs`/`/tests/` path filter below),
/// so any inline `#[cfg(test)]` that appears before the match is a different
/// file's fixture, not the production construction we assert on.
fn precedes_cfg_test(content: &str, idx: usize) -> bool {
    content[..idx].contains("#[cfg(test)]")
}

/// The live compose path must construct a request with a `Some` handoff ledger.
///
/// The endpoint census proves the ledger *type* is threaded; this proves the
/// live `Some(ledger)` branch is reachable — i.e. `build_execution_request`
/// hands the harness a real ledger so `surface_or_adopt_terminal_proxy` can
/// surface a terminal-event proxy to the command coordinator instead of always
/// adopting it in-harness. Fails if a refactor reverts the field to always
/// `None`.
#[test]
fn build_execution_request_populates_the_handoff_ledger() {
    let src_root = cli_src_root();
    let mut files = Vec::new();
    collect_rs_files(&src_root, &mut files);
    assert!(!files.is_empty(), "no .rs files under {}", src_root.display());

    let needle = "handoff_ledger: Some(";
    let mut hits: Vec<String> = Vec::new();

    for file in &files {
        let rel = file
            .strip_prefix(&src_root)
            .expect("scanned file lives under src root")
            .to_string_lossy()
            .replace('\\', "/");
        // Exclude separate `mod tests;` files and any `tests/` subtree: only
        // production constructions count.
        if rel.ends_with("tests.rs") || rel.contains("/tests/") {
            continue;
        }

        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("read {}: {e}", file.display()));

        let mut search_from = 0usize;
        while let Some(rel_idx) = content[search_from..].find(needle) {
            let idx = search_from + rel_idx;
            search_from = idx + needle.len();
            if is_commented(&content, idx) || precedes_cfg_test(&content, idx) {
                continue;
            }
            hits.push(rel.clone());
        }
    }

    assert!(
        !hits.is_empty(),
        "no production `handoff_ledger: Some(...)` construction found under {}.\n\
         The compose path must build a `CompositionExecutionRequest` with a real \
         invocation ledger (see `build_execution_request` in \
         `commands/compose/prep.rs`); an always-`None` field silently disables \
         terminal-event proxy surfacing (review-5 Finding 2).",
        src_root.display()
    );
}
