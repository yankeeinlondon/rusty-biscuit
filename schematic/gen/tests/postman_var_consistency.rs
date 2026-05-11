//! Variable-declaration consistency check for committed Postman artifacts.
//!
//! For every committed `schematic/postman/*.postman_collection.json`, this
//! test parses the JSON and asserts that every `{{name}}` reference inside
//! an auth, url, header, query, or body field is declared in the
//! collection's top-level `variable` list, or is on a small allowlist of
//! intentionally-external variable names.
//!
//! Counter-tests live alongside the unit tests in
//! `schematic/gen/src/postman_output.rs`. This integration test guards
//! against committed-artifact drift between auth blocks and the variable
//! list — i.e. exactly the regression class flagged by review-2 Finding 2.
//!
//! ## Notes
//!
//! - The set of allowed external variables starts empty and should be
//!   widened only with explicit justification (an external Postman runtime
//!   variable that we deliberately do not declare).
//! - `baseUrl` is always declared, but for grouped collections we may also
//!   declare `baseUrl2`, `baseUrl3`, etc. The check tolerates any number
//!   of trailing-digit base-URL aliases automatically because they show up
//!   directly in `collection.variable[*].key`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Walks `value` recursively, accumulating every `{{name}}` reference
/// found in any string into `out`. Comments inside this function name the
/// JSON pointers we care about (auth/url/header/query/body) but we walk
/// the whole tree to avoid missing any future reference site.
fn collect_variable_references(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::String(s) => {
            extract_var_names(s, out);
        }
        Value::Array(items) => {
            for item in items {
                collect_variable_references(item, out);
            }
        }
        Value::Object(map) => {
            for v in map.values() {
                collect_variable_references(v, out);
            }
        }
        _ => {}
    }
}

/// Extracts `name` from every `{{name}}` substring of `s` and inserts the
/// names into `out`. Names are restricted to identifier-ish characters
/// (`[A-Za-z0-9_-]+`) to avoid false positives from arbitrary user text
/// that happens to contain literal `{{` sequences.
fn extract_var_names(s: &str, out: &mut BTreeSet<String>) {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 3 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            // Find the closing }}
            let start = i + 2;
            let mut end = start;
            while end + 1 < bytes.len() && !(bytes[end] == b'}' && bytes[end + 1] == b'}') {
                end += 1;
            }
            if end + 1 < bytes.len() && bytes[end] == b'}' && bytes[end + 1] == b'}' {
                let name = &s[start..end];
                if !name.is_empty() && is_identifier_like(name) {
                    out.insert(name.to_string());
                }
                i = end + 2;
                continue;
            }
        }
        i += 1;
    }
}

fn is_identifier_like(s: &str) -> bool {
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Returns the absolute path to the `schematic/postman/` directory that
/// holds committed artifacts. The integration test runs from
/// `schematic/gen/`, so `../postman/` is the canonical location.
fn postman_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has a parent")
        .join("postman")
}

/// Allowlist of variable names that may appear inside a collection
/// without being declared at the collection level. Empty by design — any
/// addition must come with a written justification in the comment block
/// here.
const EXTERNAL_VAR_ALLOWLIST: &[&str] = &[];

#[test]
fn every_referenced_variable_is_declared_for_all_apis() {
    let dir = postman_dir();
    assert!(
        dir.is_dir(),
        "expected postman artifact directory at {}",
        dir.display(),
    );

    let mut checked_files = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(&dir).expect("read postman dir") {
        let entry = entry.expect("read postman dir entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if !name.ends_with(".postman_collection.json") {
            continue;
        }

        checked_files += 1;
        let bytes =
            std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
        let json: Value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));

        // Collect declared variable keys.
        let declared: BTreeSet<String> = json
            .get("variable")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.get("key").and_then(Value::as_str).map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // Collect referenced variable names from the entire JSON document.
        let mut referenced: BTreeSet<String> = BTreeSet::new();
        collect_variable_references(&json, &mut referenced);

        for var in &referenced {
            if declared.contains(var) {
                continue;
            }
            if EXTERNAL_VAR_ALLOWLIST.contains(&var.as_str()) {
                continue;
            }
            failures.push(format!(
                "{}: references {{{{{}}}}} but it is not declared in collection.variable",
                name, var,
            ));
        }
    }

    assert!(
        checked_files > 0,
        "no Postman collection JSON files found under {}",
        dir.display(),
    );

    assert!(
        failures.is_empty(),
        "undeclared variable references found ({} files checked):\n  - {}",
        checked_files,
        failures.join("\n  - "),
    );
}

/// Sanity check the reference-extraction helper itself so a regression in
/// the matcher cannot silently make the artifact check pass for the wrong
/// reason.
#[test]
fn extract_var_names_picks_up_typical_postman_references() {
    let mut out = BTreeSet::new();
    extract_var_names(
        "Bearer {{bearerToken}} for {{baseUrl}}/repos/{owner} ({{apiKey}})",
        &mut out,
    );
    assert!(out.contains("bearerToken"));
    assert!(out.contains("baseUrl"));
    assert!(out.contains("apiKey"));
    // `{owner}` is a Postman path variable, not a `{{...}}` reference.
    assert!(!out.contains("owner"));
}

#[test]
fn extract_var_names_ignores_unmatched_braces() {
    let mut out = BTreeSet::new();
    extract_var_names("no vars here {{ unterminated", &mut out);
    assert!(out.is_empty());
}
