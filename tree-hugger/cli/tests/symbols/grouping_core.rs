use crate::{Scratch, json_of};

#[test]
fn discovery_failure_is_stats_only_and_writes_no_envelope() {
    let scratch = Scratch::new(&[
        ("Cargo.toml", "[workspace]\nmembers = ["),
        ("lib.rs", "pub fn selected() {}\n"),
    ]);
    for command in ["functions", "types", "symbols", "classes"] {
        let output = scratch.hug(&["--json", command, "--stats", "lib.rs"]);
        assert_eq!(output.status.code(), Some(1), "{command}: {output:?}");
        assert!(output.stdout.is_empty(), "no success envelope: {output:?}");
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains("package catalog") && error.contains("unclosed array") && error.contains("check the repository and package manifests"), "{error}");
    }
    for args in [
        vec!["symbols", "--count", "lib.rs"],
        vec!["--json", "symbols", "lib.rs"],
        vec!["--json-full", "symbols", "lib.rs"],
        vec!["symbols", "--jsonl", "lib.rs"],
        vec!["symbols", "--format", "plain", "lib.rs"],
    ] {
        let output = scratch.hug(&args);
        assert!(output.status.success(), "{args:?}: {output:?}");
        assert!(!output.stdout.is_empty(), "{args:?}");
    }
}

#[test]
fn absent_catalog_still_reports_selected_totals() {
    let scratch = Scratch::new(&[("lib.rs", "/// Documented.\npub fn selected() {}\n")]);
    let report = json_of(&scratch.stdout(&["--json", "functions", "--stats", "lib.rs"]));
    assert_eq!(report["stats"]["by_kind"]["Function"], 1);
    assert_eq!(report["stats"]["doc_coverage"]["documented"], 1);
    assert_eq!(report["stats"]["doc_coverage"]["ratio"], 1.0);
    assert_eq!(report["errors"], serde_json::json!([]));
}

#[test]
fn shipped_manifest_through_normal_stats_invocation() {
    let scratch = Scratch::new(&[
        ("Cargo.toml", include_str!("../../Cargo.toml")),
        ("src/main.rs", "/// Documented.\nfn selected() {}\n"),
    ]);
    let report = json_of(&scratch.stdout(&["--json", "functions", "--stats", "src/main.rs"]));
    assert_eq!(report["groups"][0]["package"], serde_json::json!({"name": "tree-hugger-cli", "path": "."}));
    assert_eq!(report["groups"][0]["stats"], report["stats"]);
    assert_eq!(report["stats"]["by_kind"], serde_json::json!({"Function": 1}));
    assert_eq!(report["stats"]["doc_coverage"]["ratio"], 1.0);
    assert_eq!(report["errors"], serde_json::json!([]));
}
