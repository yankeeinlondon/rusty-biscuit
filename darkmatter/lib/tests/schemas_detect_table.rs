//! Table-driven detection tests for the schemas subsystem.
//!
//! Each subdirectory of `tests/fixtures/detect/` is a self-contained case
//! with at least:
//!
//! - `inputs/*.md` — one or more Markdown documents (read in sorted order)
//! - `expected.yaml` — the SimplifiedSchema yaml output `schema_to_yaml`
//!   should produce
//!
//! Optional inputs:
//! - `options.json` — `{ "merge": true|false }` (default `false`)

use std::{fs, path::Path};

use darkmatter::markdown::{
    Markdown,
    schemas::{DetectOptions, detect_schema, schema_to_yaml},
};

fn fixtures_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/detect")
}

fn list_cases() -> Vec<std::path::PathBuf> {
    let mut cases: Vec<_> = fs::read_dir(fixtures_root())
        .expect("read fixtures directory")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.path())
        .collect();
    cases.sort();
    cases
}

fn load_options(dir: &Path) -> DetectOptions {
    let path = dir.join("options.json");
    if !path.exists() {
        return DetectOptions::default();
    }
    let raw = fs::read_to_string(&path).expect("read options.json");
    let json: serde_json::Value = serde_json::from_str(&raw).expect("parse options.json");
    DetectOptions {
        merge: json.get("merge").and_then(|v| v.as_bool()).unwrap_or(false),
    }
}

fn load_inputs(dir: &Path) -> Vec<Markdown> {
    let inputs_dir = dir.join("inputs");
    let mut entries: Vec<_> = fs::read_dir(&inputs_dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", inputs_dir.display()))
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        })
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    entries
        .into_iter()
        .map(|p| {
            let content = fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
            content.as_str().into()
        })
        .collect()
}

#[test]
fn detect_table() {
    let cases = list_cases();
    assert!(!cases.is_empty(), "no detect fixtures found");

    for dir in cases {
        let case_name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let opts = load_options(&dir);
        let inputs = load_inputs(&dir);
        let inputs_ref: Vec<&Markdown> = inputs.iter().collect();
        let schema = detect_schema(&inputs_ref, opts);
        let actual = schema_to_yaml(&schema);

        let expected_path = dir.join("expected.yaml");
        let expected = fs::read_to_string(&expected_path)
            .unwrap_or_else(|err| panic!("[{case_name}] read {}: {err}", expected_path.display()));

        assert_eq!(
            actual.trim(),
            expected.trim(),
            "[{case_name}] detected schema mismatch\n--- actual ---\n{actual}\n--- expected ---\n{expected}"
        );
    }
}
