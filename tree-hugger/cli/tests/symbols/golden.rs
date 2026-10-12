//! `hug symbols` output goldens over the 16-language `multi/` fixture tree,
//! one per output format and view (`--plain` and `--format plain`, `--json`,
//! `--jsonl`, `--format table`, `--tree`, `--stats`, `--stats --json`,
//! `--signatures full`), and the byte identity of `--json` across thread
//! counts and cache states.
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

const SYMBOLS_TABLE_GOLDEN: &str = include_str!("../fixtures/golden/symbols.table");
const SYMBOLS_TREE_GOLDEN: &str = include_str!("../fixtures/golden/symbols.tree");
const SYMBOLS_SIGNATURES_FULL_GOLDEN: &str = include_str!("../fixtures/golden/symbols-signatures-full.plain");
const SYMBOLS_STATS_GOLDEN: &str = include_str!("../fixtures/golden/symbols.stats");
const SYMBOLS_STATS_JSON_GOLDEN: &str = include_str!("../fixtures/golden/symbols-stats.json");
const SYMBOLS_JSONL_GOLDEN: &str = include_str!("../fixtures/golden/symbols.jsonl");
const SYMBOLS_TSV_GOLDEN: &str = include_str!("../fixtures/golden/symbols.tsv");
const SYMBOLS_MARKDOWN_GOLDEN: &str = include_str!("../fixtures/golden/symbols.markdown");
const SYMBOLS_PROSE_GOLDEN: &str = include_str!("../fixtures/golden/symbols.prose");
const SYMBOLS_CTAGS_GOLDEN: &str = include_str!("../fixtures/golden/symbols.ctags");

/// Compares `hug args` over multi/ with the text golden `file` (holding
/// `golden`), after [`normalized`]; `HUG_UPDATE_GOLDEN` rewrites the file.
fn assert_text_golden(args: &[&str], file: &str, golden: &str) {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let actual = scratch.stdout(args);

    if std::env::var_os("HUG_UPDATE_GOLDEN").is_some() {
        std::fs::write(manifest_dir!().join("tests/fixtures/golden").join(file), &actual)
            .unwrap_or_else(|error| panic!("write {file} golden: {error}"));
        return;
    }

    let root = scratch.repo.path();
    let actual = normalized(&actual, root);
    let expected = normalized(golden, root);
    assert_same(&actual, &expected, &format!("`hug {}`", args.join(" ")), file);
}

/// Export formats must be byte-safe before root-path normalization.
fn assert_export_golden(args: &[&str], file: &str, golden: &str) {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let actual = scratch.stdout(args);
    assert!(!actual.as_bytes().contains(&0x1b), "{} contains ESC", args.join(" "));
    assert!(!actual.contains(['\u{009b}', '\u{009d}']), "{} contains CSI/OSC control", args.join(" "));
    assert!(!actual.contains('\r'), "{} contains CR", args.join(" "));
    if std::env::var_os("HUG_UPDATE_GOLDEN").is_some() {
        std::fs::write(manifest_dir!().join("tests/fixtures/golden").join(file), &actual)
            .unwrap_or_else(|error| panic!("write {file} golden: {error}"));
        return;
    }
    assert_same(&normalize_file_urls(&actual), &normalize_file_urls(golden), &format!("`hug {}`", args.join(" ")), file);
}

fn normalize_file_urls(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(start) = remaining.find("file://") {
        result.push_str(&remaining[..start]);
        let url_start = start + "file://".len();
        let after_marker = &remaining[url_start..];
        let url_end = after_marker.find('"').unwrap_or(after_marker.len());
        let url = &after_marker[..url_end];
        let relative = MULTI_FIXTURES
            .iter()
            .map(|(path, _)| *path)
            .filter(|path| url.contains(path))
            .max_by_key(|path| path.len())
            .expect("file URL names a multi fixture");
        let path_start = url.rfind(relative).expect("fixture path in file URL");
        result.push_str("file://<ROOT>/");
        result.push_str(&url[path_start..]);
        remaining = &after_marker[url_end..];
    }
    result.push_str(remaining);
    result
}

#[test]
fn tsv_symbols_over_multi_match_the_golden() {
    assert_export_golden(&["symbols", "--format", "tsv", "--no-cache"], "symbols.tsv", SYMBOLS_TSV_GOLDEN);
}

#[test]
fn markdown_symbols_over_multi_match_the_golden() {
    assert_export_golden(&["symbols", "--format", "markdown", "--no-cache"], "symbols.markdown", SYMBOLS_MARKDOWN_GOLDEN);
}

#[test]
fn prose_symbols_over_multi_match_the_golden() {
    assert_export_golden(&["symbols", "--format", "prose", "--no-cache"], "symbols.prose", SYMBOLS_PROSE_GOLDEN);
}

#[test]
fn ctags_symbols_over_multi_match_the_golden() {
    assert_export_golden(&["symbols", "--format", "ctags", "--no-cache"], "symbols.ctags", SYMBOLS_CTAGS_GOLDEN);
}

fn assert_export_stability(format: &str) {
    let scratch = Scratch::new(MULTI_FIXTURES);
    {
        let args = ["symbols", "--format", format];
        let cold = scratch.hug_with_env(&args, &[("RAYON_NUM_THREADS", "1"), ("COLUMNS", "20")]);
        assert!(cold.status.success(), "{format}: {}", String::from_utf8_lossy(&cold.stderr));
        let warm_wide = scratch.hug_with_env(&args, &[("RAYON_NUM_THREADS", "4"), ("COLUMNS", "200")]);
        assert!(warm_wide.status.success(), "{format}: {}", String::from_utf8_lossy(&warm_wide.stderr));
        assert_eq!(cold.stdout, warm_wide.stdout, "{format} changed across worker count, cache state, or COLUMNS");
        let uncached = scratch.hug(&["symbols", "--format", format, "--no-cache"]);
        assert!(uncached.status.success(), "{format}: {}", String::from_utf8_lossy(&uncached.stderr));
        assert_eq!(cold.stdout, uncached.stdout, "{format} changed between cached and uncached runs");
    }
}

/// Fails with the first differing line of `actual` against the golden `file`.
fn assert_same(actual: &str, expected: &str, what: &str, file: &str) {
    if actual != expected {
        let first_difference = actual
            .lines()
            .zip(expected.lines())
            .position(|(actual_line, expected_line)| actual_line != expected_line)
            .unwrap_or_else(|| actual.lines().count().min(expected.lines().count()));
        panic!(
            "{what} over multi/ differs from tests/fixtures/golden/{file} at line {}:\n  \
             expected: {:?}\n  actual:   {:?}\n\nfull output:\n{actual}",
            first_difference + 1,
            expected.lines().nth(first_difference),
            actual.lines().nth(first_difference),
        );
    }
}

#[test]
fn plain_symbols_over_multi_match_the_golden() {
    assert_text_golden(&["--plain", "symbols", "--no-cache"], "symbols.plain", SYMBOLS_PLAIN_GOLDEN);
}

/// `--format plain` is the default layout without styles, links or
/// wrapping: the same text as `--plain`, so it shares that golden.
#[test]
fn format_plain_symbols_over_multi_match_the_plain_golden() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let root = scratch.repo.path();
    let actual = normalized(&scratch.stdout(&["symbols", "--format", "plain", "--no-cache"]), root);
    let expected = normalized(SYMBOLS_PLAIN_GOLDEN, root);
    assert_same(&actual, &expected, "`hug symbols --format plain`", "symbols.plain");
}

#[test]
fn table_symbols_over_multi_match_the_golden() {
    assert_text_golden(&["symbols", "--format", "table", "--no-cache"], "symbols.table", SYMBOLS_TABLE_GOLDEN);
}

#[test]
fn tree_symbols_over_multi_match_the_golden() {
    assert_text_golden(&["--plain", "symbols", "--tree", "--no-cache"], "symbols.tree", SYMBOLS_TREE_GOLDEN);
}

#[test]
fn full_signature_symbols_over_multi_match_the_golden() {
    assert_text_golden(
        &["--plain", "symbols", "--signatures", "full", "--no-cache"],
        "symbols-signatures-full.plain",
        SYMBOLS_SIGNATURES_FULL_GOLDEN,
    );
}

#[test]
fn stats_symbols_over_multi_match_the_golden() {
    assert_text_golden(&["symbols", "--stats", "--no-cache"], "symbols.stats", SYMBOLS_STATS_GOLDEN);
}

#[test]
fn stats_json_symbols_over_multi_match_the_golden() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let actual = normalized_json(&scratch.stdout(&["--json", "symbols", "--stats", "--no-cache"]));
    if std::env::var_os("HUG_UPDATE_GOLDEN").is_some() {
        std::fs::write(manifest_dir!().join("tests/fixtures/golden/symbols-stats.json"), &actual)
            .expect("write symbols-stats.json golden");
        return;
    }
    let expected = SYMBOLS_STATS_JSON_GOLDEN.replace("\r\n", "\n");
    assert_same(&actual, &expected, "`hug --json symbols --stats`", "symbols-stats.json");
}

/// The JSON Lines golden holds each line canonicalized like the JSON golden
/// (sorted keys, numbered IDs shared across lines), one blank line apart.
#[test]
fn jsonl_symbols_over_multi_match_the_golden() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let stdout = scratch.stdout(&["symbols", "--jsonl", "--no-cache"]);
    let mut ids = HashMap::new();
    let mut actual = String::new();
    for line in stdout.lines() {
        let value: Value = serde_json::from_str(line).unwrap_or_else(|error| panic!("invalid line ({error}): {line}"));
        canonical(&value, 0, &mut ids, &mut actual);
        actual.push_str("\n\n");
    }
    if std::env::var_os("HUG_UPDATE_GOLDEN").is_some() {
        std::fs::write(manifest_dir!().join("tests/fixtures/golden/symbols.jsonl"), &actual)
            .expect("write symbols.jsonl golden");
        return;
    }
    let expected = SYMBOLS_JSONL_GOLDEN.replace("\r\n", "\n");
    assert_same(&actual, &expected, "`hug symbols --jsonl`", "symbols.jsonl");
}

/// `--count` is one decimal integer line: the number of records `--json`
/// reports, and as many as `--jsonl` writes lines.
#[test]
fn count_symbols_over_multi_is_one_integer_line() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let count = scratch.stdout(&["symbols", "--count", "--no-cache"]);
    let report = json_of(&scratch.stdout(&["--json", "symbols", "--no-cache"]));
    let records: usize = report["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|file| file["symbols"].as_array().expect("symbols").len())
        .sum();
    assert!(records > 50, "multi/ declares dozens of symbols: {records}");
    assert_eq!(count, format!("{records}\n"), "one integer and a newline");
    let lines = scratch.stdout(&["symbols", "--jsonl", "--no-cache"]).lines().count();
    assert_eq!(lines, records);
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
    if let Some(root) = report.get_mut("package_root_dir") {
        *root = Value::from("<ROOT>");
    }
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
}

#[test]
fn json_classes_over_multi_are_identical_with_one_thread() {
    assert_identical_with_one_thread(&["--json", "classes", "--no-cache", "--sort", "name"]);
}

#[test]
fn jsonl_symbols_over_multi_are_identical_with_one_thread() {
    assert_identical_with_one_thread(&["symbols", "--jsonl", "--no-cache", "--sort", "kind"]);
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

mod export_formats_are_stable_across_workers_cache_and_terminal_width {
    use super::*;
    #[test]
    fn tsv() { assert_export_stability("tsv"); }
    #[test]
    fn markdown() { assert_export_stability("markdown"); }
    #[test]
    fn prose() { assert_export_stability("prose"); }
    #[test]
    fn ctags() { assert_export_stability("ctags"); }
}
