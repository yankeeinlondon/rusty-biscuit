//! R14: `dmls --bench-index` reports the index store's cold fill and its
//! warm restart: revalidation, the first references answer with and without
//! the file-list walk, rename, workspace symbols, and the resident memory
//! the warm half adds (acceptance 10).
//!
//! The figures are not asserted (timing and memory on shared runners are
//! noise); the report's shape and its deterministic counts are. The library
//! test runs both halves in this process; the binary test proves the launch
//! path, which runs the warm half in a fresh process over the cold half's
//! store.

use std::path::Path;
use std::process::{Command, Stdio};

use biscuit_test_harness::bin_exe;
use darkmatter::markdown::compose::RequestSnapshot;
use dmls::context::RepositoryContexts;
use serde_json::Value;

/// `README.md` (the default target) with a heading two files reference, and
/// one unrelated file.
fn fixture(root: &Path) {
    std::fs::write(root.join("README.md"), "# Overview\n\nThe project.\n").unwrap();
    std::fs::write(root.join("guide.md"), "# Guide\n\nRead [the overview](README.md#overview).\n").unwrap();
    std::fs::write(root.join("notes.md"), "# Notes\n\nSee [[README]].\n").unwrap();
    std::fs::write(root.join("other.md"), "# Other\n\nNothing.\n").unwrap();
}

#[test]
fn bench_index_reports_a_cold_fill_and_a_warm_restart() {
    let temp = tempfile::tempdir().unwrap();
    let root = biscuit_file::canonicalize_simplified(temp.path()).unwrap().join("ws");
    std::fs::create_dir(&root).unwrap();
    fixture(&root);
    let store_dir = temp.path().join("stores");
    let contexts = RepositoryContexts::new(RequestSnapshot::new(&root));

    let report = dmls::bench_index(&root, &contexts, &store_dir, None).unwrap();

    assert_eq!(report.files, 4);
    let (cold, warm) = (&report.cold, &report.warm);
    assert_eq!((cold.fill.read, cold.fill.reparsed, cold.fill.unchanged, cold.fill.failed), (4, 4, 0, 0));
    assert!(cold.store_bytes.is_some_and(|bytes| bytes > 0), "{cold:?}");
    // Warm: the cold half's rows match every file, so nothing is parsed.
    assert_eq!((warm.revalidate.read, warm.revalidate.reparsed, warm.revalidate.unchanged), (4, 0, 4));
    assert_eq!(warm.target, root.join("README.md"));
    assert_eq!(warm.heading.as_deref(), Some("Overview"));
    assert!(warm.candidates >= 2, "guide.md and notes.md: {warm:?}");
    assert_eq!(warm.references, 1, "the heading's one path-link backlink: {warm:?}");
    assert!(warm.rename_ms.is_some());
    assert!(warm.symbols >= 1, "{warm:?}");
    assert!(warm.first_references_ms >= warm.references_ms);
    assert!(warm.first_references_with_list_ms >= warm.first_references_ms);
    if cfg!(unix) {
        assert!(cold.peak_rss_bytes.is_some() && warm.added_rss_bytes.is_some(), "{report:?}");
    }

    let json: Value = serde_json::from_str(&report.to_json()).unwrap();
    for (section, keys) in [
        ("cold", &["list_ms", "open_ms", "fill_ms", "fill", "store_bytes", "peak_rss_bytes"][..]),
        ("warm", &[
            "list_ms", "open_ms", "revalidate_ms", "revalidate", "references_ms", "rename_ms", "symbols_ms",
            "first_references_ms", "first_references_with_list_ms", "added_rss_bytes",
        ][..]),
    ] {
        for key in keys {
            assert!(json[section].get(key).is_some(), "{section}.{key} missing: {json:#}");
        }
    }
    let summary = report.to_summary();
    for needle in ["cold:", "warm:", "first references", "added RSS"] {
        assert!(summary.contains(needle), "{needle:?} missing from:\n{summary}");
    }
}

#[test]
fn the_binary_runs_the_warm_half_over_the_cold_half_store_and_cleans_up() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("ws");
    let tmp = temp.path().join("tmp");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(&tmp).unwrap();
    fixture(&root);

    let output = Command::new(bin_exe!("dmls"))
        .arg("--bench-index")
        .arg(&root)
        .args(["--json", "--log-level", "off", "--bench-target", "guide.md"])
        .env("TMPDIR", &tmp)
        .env("TMP", &tmp)
        .env("TEMP", &tmp)
        .stdin(Stdio::null())
        .output()
        .expect("run the compiled dmls binary");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(json["files"], 4);
    assert_eq!(json["cold"]["fill"]["reparsed"], 4);
    // The warm process read the store the cold half filled.
    assert_eq!(json["warm"]["revalidate"]["reparsed"], 0);
    assert_eq!(json["warm"]["revalidate"]["unchanged"], 4);
    assert!(json["warm"]["target"].as_str().unwrap().ends_with("guide.md"), "{json:#}");
    assert_eq!(json["warm"]["heading"], "Guide");
    assert!(std::fs::read_dir(&tmp).unwrap().next().is_none(), "the temporary store directory is removed");
}

#[test]
fn a_bench_over_no_markdown_or_an_unreadable_target_fails_with_the_path() {
    let temp = tempfile::tempdir().unwrap();
    let root = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
    let contexts = RepositoryContexts::new(RequestSnapshot::new(&root));
    let error = dmls::bench_index(&root, &contexts, &root.join("stores"), None).unwrap_err();
    assert!(error.contains("lists no Markdown file") && error.contains(&root.display().to_string()), "{error}");

    std::fs::write(root.join("a.md"), "# A\n").unwrap();
    let error = dmls::bench_index(&root, &contexts, &root.join("stores"), Some(Path::new("missing.md"))).unwrap_err();
    assert!(error.contains("cannot read") && error.contains("missing.md"), "{error}");
}
