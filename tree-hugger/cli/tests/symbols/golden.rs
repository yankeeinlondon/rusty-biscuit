//! `hug symbols` output goldens over the 16-language `multi/` fixture tree,
//! and the byte identity of `--json` across thread counts and cache states.
//!
//! The plain golden was first captured from the binary before the
//! record-native renderer and regenerated once for the renderer's deliberate
//! differences (file/source order, labels from declaring keywords). The JSON
//! golden is the report with `root_dir` and record IDs (which hash the
//! absolute path) normalized, pretty-printed with sorted keys. Regenerate
//! either only for a deliberate output change (set `HUG_UPDATE_GOLDEN=1` and
//! rerun), and record each difference in the change that makes it.

use std::collections::HashMap;
use std::path::Path;

use biscuit_test_harness::manifest_dir;
use serde_json::Value;
use tree_hugger::corpus::{redact_snapshot_text, strip_ansi_codes};

use crate::{Scratch, json_of};

/// Every `multi/` file, keyed by its path inside the tree.
pub(crate) const MULTI_FIXTURES: &[(&str, &str)] = &[
    ("bash.sh", include_str!("../../../lib/tests/fixtures/symbols/multi/bash.sh")),
    ("c.c", include_str!("../../../lib/tests/fixtures/symbols/multi/c.c")),
    ("c_sharp.cs", include_str!("../../../lib/tests/fixtures/symbols/multi/c_sharp.cs")),
    ("cpp.cpp", include_str!("../../../lib/tests/fixtures/symbols/multi/cpp.cpp")),
    ("java.java", include_str!("../../../lib/tests/fixtures/symbols/multi/java.java")),
    ("javascript.js", include_str!("../../../lib/tests/fixtures/symbols/multi/javascript.js")),
    ("lua.lua", include_str!("../../../lib/tests/fixtures/symbols/multi/lua.lua")),
    ("nested/go.go", include_str!("../../../lib/tests/fixtures/symbols/multi/nested/go.go")),
    ("nested/python.py", include_str!("../../../lib/tests/fixtures/symbols/multi/nested/python.py")),
    ("perl.pl", include_str!("../../../lib/tests/fixtures/symbols/multi/perl.pl")),
    ("php.php", include_str!("../../../lib/tests/fixtures/symbols/multi/php.php")),
    ("rust.rs", include_str!("../../../lib/tests/fixtures/symbols/multi/rust.rs")),
    ("scala.scala", include_str!("../../../lib/tests/fixtures/symbols/multi/scala.scala")),
    ("swift.swift", include_str!("../../../lib/tests/fixtures/symbols/multi/swift.swift")),
    ("typescript.ts", include_str!("../../../lib/tests/fixtures/symbols/multi/typescript.ts")),
    ("zsh.zsh", include_str!("../../../lib/tests/fixtures/symbols/multi/zsh.zsh")),
];

const SYMBOLS_PLAIN_GOLDEN: &str = include_str!("../fixtures/golden/symbols.plain");
const SYMBOLS_JSON_GOLDEN: &str = include_str!("../fixtures/golden/symbols.json");

/// Strips ANSI escapes, redacts scratch and temp paths, and normalizes line
/// endings and in-line whitespace, so the comparison holds on every OS.
fn normalized(text: &str, root: &Path) -> String {
    redact_snapshot_text(&strip_ansi_codes(text), root)
}

#[test]
fn plain_symbols_over_multi_match_the_golden() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let actual = scratch.stdout(&["--plain", "symbols", "--no-cache"]);

    if std::env::var_os("HUG_UPDATE_GOLDEN").is_some() {
        std::fs::write(manifest_dir!().join("tests/fixtures/golden/symbols.plain"), &actual)
            .expect("write symbols.plain golden");
        return;
    }

    let root = scratch.repo.path();
    let actual = normalized(&actual, root);
    let expected = normalized(SYMBOLS_PLAIN_GOLDEN, root);
    if actual != expected {
        let first_difference = actual
            .lines()
            .zip(expected.lines())
            .position(|(actual_line, expected_line)| actual_line != expected_line)
            .unwrap_or_else(|| actual.lines().count().min(expected.lines().count()));
        panic!(
            "`hug --plain symbols` over multi/ differs from tests/fixtures/golden/symbols.plain \
             at line {}:\n  expected: {:?}\n  actual:   {:?}\n\nfull output:\n{actual}",
            first_difference + 1,
            expected.lines().nth(first_difference),
            actual.lines().nth(first_difference),
        );
    }
}

/// Records carry no wall-clock data, so two uncached runs over the same tree
/// print the same bytes.
#[test]
fn json_symbols_over_multi_are_identical_across_runs() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let args = ["--json", "symbols", "--no-cache"];
    let first = scratch.stdout(&args);
    let second = scratch.stdout(&args);
    assert!(first.contains("\"signature_text\""), "the report must carry records");
    assert!(first == second, "two `hug --json symbols --no-cache` runs over multi/ differ");
}

/// `value` as pretty JSON with every object's keys sorted, whatever map
/// order the `serde_json` build preserves, and every record ID (a 64-digit
/// hex string, which hashes the absolute path) replaced by `<id-N>`,
/// numbered by first appearance.
fn canonical(value: &Value, indent: usize, ids: &mut HashMap<String, usize>, out: &mut String) {
    let pad = "  ".repeat(indent + 1);
    match value {
        Value::Object(map) if !map.is_empty() => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push_str("{\n");
            for (position, key) in keys.iter().enumerate() {
                out.push_str(&pad);
                out.push_str(&Value::from(normalized_id(key, ids)).to_string());
                out.push_str(": ");
                canonical(&map[*key], indent + 1, ids, out);
                out.push_str(if position + 1 < keys.len() { ",\n" } else { "\n" });
            }
            out.push_str(&"  ".repeat(indent));
            out.push('}');
        }
        Value::Array(items) if !items.is_empty() => {
            out.push_str("[\n");
            for (position, item) in items.iter().enumerate() {
                out.push_str(&pad);
                canonical(item, indent + 1, ids, out);
                out.push_str(if position + 1 < items.len() { ",\n" } else { "\n" });
            }
            out.push_str(&"  ".repeat(indent));
            out.push(']');
        }
        Value::String(text) => out.push_str(&Value::from(normalized_id(text, ids)).to_string()),
        other => out.push_str(&other.to_string()),
    }
}

fn normalized_id(text: &str, ids: &mut HashMap<String, usize>) -> String {
    let is_id = text.len() == 64
        && text.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !is_id {
        return text.to_string();
    }
    let next = ids.len() + 1;
    format!("<id-{}>", ids.entry(text.to_string()).or_insert(next))
}

/// The report as the golden stores it.
fn normalized_json(stdout: &str) -> String {
    let mut report = json_of(stdout);
    report["root_dir"] = Value::from("<ROOT>");
    let mut text = String::new();
    canonical(&report, 0, &mut HashMap::new(), &mut text);
    text.push('\n');
    text
}

#[test]
fn json_symbols_over_multi_match_the_golden() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let actual = normalized_json(&scratch.stdout(&["--json", "symbols", "--no-cache"]));

    if std::env::var_os("HUG_UPDATE_GOLDEN").is_some() {
        std::fs::write(manifest_dir!().join("tests/fixtures/golden/symbols.json"), &actual)
            .expect("write symbols.json golden");
        return;
    }

    let expected = SYMBOLS_JSON_GOLDEN.replace("\r\n", "\n");
    if actual != expected {
        let first_difference = actual
            .lines()
            .zip(expected.lines())
            .position(|(actual_line, expected_line)| actual_line != expected_line)
            .unwrap_or_else(|| actual.lines().count().min(expected.lines().count()));
        panic!(
            "`hug --json symbols` over multi/ differs from tests/fixtures/golden/symbols.json \
             at line {}:\n  expected: {:?}\n  actual:   {:?}",
            first_difference + 1,
            expected.lines().nth(first_difference),
            actual.lines().nth(first_difference),
        );
    }
}

/// `hug args` prints the same bytes with eight rayon workers and with one.
fn assert_identical_with_one_thread(args: &[&str]) {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let parallel = crate::stdout_of(scratch.hug_with_env(args, &[("RAYON_NUM_THREADS", "8")]), args);
    let single = crate::stdout_of(scratch.hug_with_env(args, &[("RAYON_NUM_THREADS", "1")]), args);
    assert!(parallel == single, "hug {args:?}: one rayon thread printed different bytes");
}

/// Files are analyzed on rayon workers and delivered in file order, so one
/// worker and many print the same bytes.
#[test]
fn json_symbols_over_multi_are_identical_with_one_thread() {
    assert_identical_with_one_thread(&["--json", "symbols", "--no-cache"]);
    assert_identical_with_one_thread(&["--json", "classes", "--no-cache", "--sort", "name"]);
}

#[test]
fn json_full_over_multi_is_identical_with_one_thread() {
    assert_identical_with_one_thread(&["--json-full", "symbols", "--no-cache"]);
}

/// A cold run that writes the cache and the warm run served from it print
/// the same bytes, for a listing and for the complete index.
#[test]
fn json_symbols_over_multi_are_identical_cold_and_warm() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    for args in [&["--json", "symbols"][..], &["--json-full", "functions"]] {
        let cold = scratch.stdout(args);
        let warm = scratch.stdout(args);
        assert!(cold == warm, "hug {args:?}: cold and warm runs differ");
    }
}
