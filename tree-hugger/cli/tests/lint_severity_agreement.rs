//! The library and `hug lint --experimental-semantics` give `undefined-symbol`
//! the same severity for one shared fixture, which the library's
//! `lint_severity_registry` tests also read.

use std::process::Command;

use biscuit_test_harness::{bin_exe, manifest_dir};
use serde_json::Value;
use tree_hugger::{DiagnosticSeverity, TreeFile};

#[test]
fn library_and_hug_lint_agree_on_undefined_symbol_severity() {
    let fixture = manifest_dir!().parent().unwrap().join("lib/tests/fixtures/lint/undefined_symbol.py");
    // `hug` skips `fixtures/` directories, so it scans a copy.
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("undefined_symbol.py");
    std::fs::copy(&fixture, &path).expect("copy the shared fixture");

    let mut library = TreeFile::new(&path).unwrap();
    library.experimental_semantics = true;
    let library_severities: Vec<String> = library
        .lint_diagnostics()
        .into_iter()
        .filter(|d| d.rule.as_deref() == Some("undefined-symbol"))
        .map(|d| {
            assert_eq!(d.severity, DiagnosticSeverity::Warning);
            format!("{:?}", d.severity)
        })
        .collect();
    assert_eq!(library_severities.len(), 2, "`compute_total` and `items`");

    let output = Command::new(bin_exe!("hug"))
        .current_dir(dir.path())
        .args(["--json", "lint", "--no-cache", "--experimental-semantics", "undefined_symbol.py"])
        .output()
        .expect("run hug");
    assert!(output.status.success(), "a warning does not fail `hug lint`: {output:?}");
    let json: Value = serde_json::from_slice(&output.stdout).expect("hug prints JSON");
    let hug_severities: Vec<String> = json["files"][0]["lint"]
        .as_array()
        .expect("lint diagnostics")
        .iter()
        .filter(|d| d["rule"] == "undefined-symbol")
        .map(|d| d["severity"].as_str().expect("severity").to_string())
        .collect();
    assert_eq!(hug_severities, library_severities);
}
