//! The persistent cache across `hug` processes: concurrent runs over one file
//! leave one readable entry, cache use and cache failures print nothing
//! unless `--verbose` asks, and the work each command does (parses,
//! definition captures, diagnostics runs) as `--verbose` reports it.

use std::sync::{Arc, Barrier};

use serde_json::Value;

use crate::cache_upgrade::read_json;
use crate::golden::MULTI_FIXTURES;
use crate::{PYTHON_FIXTURE, Scratch, cache_entries, json_of, stdout_of};

const FILE: &str = "python.py";

/// Every file under `root` whose name ends in `.tmp`.
fn temporary_files(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    fn walk(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, found);
            } else if path.extension().is_some_and(|ext| ext == "tmp") {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(root, &mut found);
    found
}

/// Processes writing the same entry at once (with different diagnostics
/// settings, so their entries differ): the last rename wins, the survivor is
/// a complete entry whose coverage matches its contents, and the next run is
/// served from it with the uncached result.
#[test]
fn concurrent_processes_leave_one_readable_entry() {
    let scratch = Scratch::new(&[(FILE, PYTHON_FIXTURE)]);
    let runs: [&[&str]; 6] = [
        &["--json", "symbols", FILE],
        &["--json", "functions", FILE],
        &["--json", "lint", "--experimental-semantics", FILE],
        &["--json", "types", FILE],
        &["--json", "lint", FILE],
        &["--json", "classes", FILE],
    ];
    let barrier = Arc::new(Barrier::new(runs.len()));
    std::thread::scope(|scope| {
        for args in runs {
            let (scratch, barrier) = (&scratch, Arc::clone(&barrier));
            scope.spawn(move || {
                barrier.wait();
                // `lint` exits non-zero when it finds errors; only a crash or
                // unreadable output matters here.
                let output = scratch.hug(args);
                assert!(output.status.code().is_some(), "hug {args:?} was killed");
                let _: Value = serde_json::from_slice(&output.stdout)
                    .unwrap_or_else(|error| panic!("hug {args:?}: invalid JSON ({error})"));
            });
        }
    });

    assert!(temporary_files(scratch.temp.path()).is_empty(), "a temporary file was left");
    let entries = cache_entries(scratch.temp.path());
    assert_eq!(entries.len(), 1, "one entry per file: {entries:#?}");
    // Each command writes the coverage it computed, so the survivor may be a
    // narrow entry; whichever it is, its records are of the kinds it covers.
    let entry = read_json(&entries[0]);
    let covered = &entry["covered_kinds"];
    for record in entry["records"].as_array().expect("records") {
        assert!(
            covered == "all" || covered.as_array().expect("kind list").contains(&record["kind"]),
            "record {} is of a kind the entry does not cover ({covered})",
            record["identity"]["name"]
        );
    }
    let has_diagnostics = entry["completed_capabilities"]
        .as_array()
        .expect("capabilities")
        .iter()
        .any(|capability| capability == "diagnostics");
    assert_eq!(has_diagnostics, entry["diagnostics_fingerprint"].is_string());

    let args = ["--json", "symbols", FILE];
    let uncached = json_of(&scratch.stdout(&["--json", "symbols", "--no-cache", FILE]));
    assert_eq!(json_of(&scratch.stdout(&args)), uncached);
}

/// A warm run and a run whose cache cannot be written print nothing to
/// stderr, and both match an uncached run.
#[test]
fn cache_hits_and_cache_failures_print_nothing() {
    let scratch = Scratch::new(&[(FILE, PYTHON_FIXTURE)]);
    let args = ["--json", "symbols", FILE];
    let uncached = json_of(&scratch.stdout(&["--json", "symbols", "--no-cache", FILE]));
    assert!(cache_entries(scratch.temp.path()).is_empty(), "--no-cache wrote an entry");

    for run in ["cold", "warm"] {
        let output = scratch.hug(&args);
        assert_eq!(String::from_utf8_lossy(&output.stderr), "", "{run} run stderr");
        assert_eq!(json_of(&stdout_of(output, &args)), uncached, "{run} run");
    }
    assert_eq!(cache_entries(scratch.temp.path()).len(), 1);

    // A file where the cache directory belongs: nothing can be read or written.
    let blocked = tempfile::TempDir::new().expect("blocked temp");
    std::fs::write(blocked.path().join("tree-hugger"), "not a directory").expect("block cache");
    let output = scratch.hug_with_temp(blocked.path(), &args);
    assert_eq!(String::from_utf8_lossy(&output.stderr), "", "failed-write run stderr");
    assert_eq!(json_of(&stdout_of(output, &args)), uncached, "failed-write run");
}

/// Runs `hug` and returns its stderr, failing when it exits non-zero.
fn stderr_of_success(scratch: &Scratch, temp: &std::path::Path, args: &[&str]) -> String {
    let output = scratch.hug_with_temp(temp, args);
    assert!(output.status.success(), "hug {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// With `--verbose`, the same runs print the stats line (hits, partial hits,
/// misses and the work done) and each recoverable cache failure.
#[test]
fn verbose_reports_cache_use_and_cache_failures() {
    let scratch = Scratch::new(&[(FILE, PYTHON_FIXTURE)]);
    let temp = scratch.temp.path();
    let args = ["--verbose", "--json", "symbols", FILE];

    let cold = stderr_of_success(&scratch, temp, &args);
    assert_eq!(cold.lines().count(), 1, "cold: only the stats line:\n{cold}");
    assert!(cold.starts_with("cache: 0 hits, 0 partial hits, 1 misses; parsed 1 files"), "{cold}");
    let warm = stderr_of_success(&scratch, temp, &args);
    assert!(warm.starts_with("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"), "{warm}");

    // A corrupt entry is a miss, named with its reason.
    std::fs::write(&cache_entries(temp)[0], "{ not json").expect("corrupt the entry");
    let corrupt = stderr_of_success(&scratch, temp, &args);
    assert!(corrupt.contains(&format!("cache: {FILE}: malformed entry")), "{corrupt}");
    assert!(corrupt.contains("1 misses"), "{corrupt}");

    // A cache that cannot be written reports the failed write.
    let blocked = tempfile::TempDir::new().expect("blocked temp");
    std::fs::write(blocked.path().join("tree-hugger"), "not a directory").expect("block cache");
    let failed = stderr_of_success(&scratch, blocked.path(), &args);
    assert!(failed.contains(&format!("cache: {FILE}: write failed")), "{failed}");

    // `--no-cache` reports the cache as disabled.
    let disabled = stderr_of_success(&scratch, temp, &["--verbose", "--json", "symbols", "--no-cache", FILE]);
    assert!(disabled.starts_with("cache: disabled; parsed 1 files"), "{disabled}");
}

/// The `--verbose` stats line of a `hug` run in `scratch`.
fn multi_stats(scratch: &Scratch, args: &[&str]) -> String {
    let mut verbose = vec!["--verbose"];
    verbose.extend_from_slice(args);
    let output = scratch.hug(&verbose);
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    stderr
        .lines()
        .find(|line| line.starts_with("cache: "))
        .unwrap_or_else(|| panic!("hug {args:?}: no stats line in {stderr}"))
        .to_string()
}

/// Each declaration command captures definitions without running diagnostics.
mod symbol_commands_run_no_diagnostics {
    use super::*;

    fn check(command: &str) {
        let scratch = Scratch::new(MULTI_FIXTURES);
        let uncached = multi_stats(&scratch, &["--json", command, "--no-cache"]);
        assert!(
            uncached.ends_with("parsed 16 files, 16 definition captures, 0 diagnostics runs"),
            "{command}: {uncached}"
        );
    }

    #[test]
    fn functions() {
        check("functions");
    }
    #[test]
    fn types() {
        check("types");
    }
    #[test]
    fn symbols() {
        check("symbols");
    }
    #[test]
    fn classes() {
        check("classes");
    }
}

/// Commands that show diagnostics must still run them.
mod lint_and_json_full_run_diagnostics {
    use super::*;

    #[test]
    fn lint() {
        let scratch = Scratch::new(MULTI_FIXTURES);
        let lint = multi_stats(&scratch, &["--json", "lint", "--no-cache"]);
        assert!(!lint.ends_with(" 0 diagnostics runs"), "lint runs diagnostics: {lint}");
    }

    #[test]
    fn json_full() {
        let scratch = Scratch::new(MULTI_FIXTURES);
        let full = multi_stats(&scratch, &["--json-full", "functions", "--no-cache"]);
        assert!(full.ends_with("16 diagnostics runs"), "--json-full runs diagnostics: {full}");
    }
}

/// A complete warm hit parses nothing, captures nothing and runs no
/// diagnostics, for the command that wrote the entries and for narrower
/// ones it covers.
#[test]
fn complete_warm_hits_parse_nothing() {
    let scratch = Scratch::new(MULTI_FIXTURES);
    let cold = multi_stats(&scratch, &["--plain", "symbols"]);
    assert!(cold.contains("16 misses") && cold.ends_with("0 diagnostics runs"), "{cold}");
    for command in ["symbols", "functions", "types"] {
        let warm = multi_stats(&scratch, &["--plain", command]);
        assert!(
            warm.starts_with(
                "cache: 16 hits, 0 partial hits, 0 misses; parsed 0 files, 0 definition captures, 0 diagnostics runs"
            ),
            "{command}: {warm}"
        );
    }
}

/// The entry and the IDs do not depend on the directory `hug` runs from: a
/// run from a subdirectory hits the entry a run from the repository root
/// wrote, with the same IDs and in-file qualified names; only the
/// scan-root-relative `file_path` and the qualified-name prefix built from it
/// differ.
#[test]
fn a_run_from_another_scan_root_hits_the_same_entry_with_the_same_ids() {
    let scratch = Scratch::new(&[("sub/python.py", PYTHON_FIXTURE)]);
    let run = |dir: &std::path::Path, file: &str| {
        let args = ["--verbose", "--json", "symbols", file];
        let output = scratch.process(&args).current_dir(dir).output().expect("run hug");
        assert!(output.status.success(), "hug {args:?}: {}", String::from_utf8_lossy(&output.stderr));
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        (stderr, json_of(&String::from_utf8(output.stdout).expect("utf-8 stdout")))
    };
    let facts = |json: &Value| -> Vec<(Value, Value, Value)> {
        json["files"]
            .as_array()
            .expect("files")
            .iter()
            .flat_map(|file| file["symbols"].as_array().expect("symbols").clone())
            .map(|record| (record["id"].clone(), record["identity"]["qualified_name"].clone(), record["source"]["file_path"].clone()))
            .collect()
    };

    let (cold_stats, from_root) = run(scratch.repo.path(), "sub/python.py");
    assert!(cold_stats.starts_with("cache: 0 hits, 0 partial hits, 1 misses"), "{cold_stats}");
    let (warm_stats, from_sub) = run(&scratch.repo.path().join("sub"), "python.py");
    assert!(warm_stats.starts_with("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"), "{warm_stats}");
    assert_eq!(cache_entries(scratch.temp.path()).len(), 1, "one entry for one file");

    let (root_facts, sub_facts) = (facts(&from_root), facts(&from_sub));
    assert!(!root_facts.is_empty());
    assert_eq!(root_facts.len(), sub_facts.len());
    for ((root_id, root_name, root_path), (sub_id, sub_name, sub_path)) in root_facts.iter().zip(&sub_facts) {
        assert_eq!(root_id, sub_id, "ID of {root_name}");
        let in_file = root_name.as_str().and_then(|name| name.strip_prefix("sub/python.py#")).expect("root prefix");
        assert_eq!(sub_name.as_str().and_then(|name| name.strip_prefix("python.py#")), Some(in_file));
        assert_eq!(root_path, "sub/python.py");
        assert_eq!(sub_path, "python.py");
    }
}

/// `--json` asks for the flags a terminal listing did not compute: the entry
/// is widened in place (a partial hit that parses the file once, never a
/// miss), and both commands are then complete hits.
#[test]
fn a_json_run_after_a_listing_widens_the_entry_by_the_flags() {
    let scratch = Scratch::new(&[(FILE, PYTHON_FIXTURE)]);
    let temp = scratch.temp.path();
    let listing = ["--verbose", "--plain", "symbols", FILE];
    let json = ["--verbose", "--json", "symbols", FILE];

    let cold = stderr_of_success(&scratch, temp, &listing);
    assert!(cold.starts_with("cache: 0 hits, 0 partial hits, 1 misses; parsed 1 files"), "{cold}");
    let widened = stderr_of_success(&scratch, temp, &json);
    assert!(widened.starts_with("cache: 0 hits, 1 partial hits, 0 misses; parsed 1 files"), "{widened}");
    let entry = read_json(&cache_entries(temp)[0]);
    assert_eq!(entry["completed_capabilities"], serde_json::json!(["declarations", "flags"]));
    for args in [&listing, &json] {
        let warm = stderr_of_success(&scratch, temp, args);
        assert!(warm.starts_with("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"), "{args:?}: {warm}");
    }
}

/// A forwarding file reports the same re-exports and import markers fresh,
/// from a cache hit, and from another scan root; the hit parses nothing.
#[test]
fn json_full_reexports_agree_fresh_hit_and_from_another_scan_root() {
    let source = "import { a } from './a';\nexport { a };\nexport * from './b';\nexport { c as d } from './c';\n";
    let scratch = Scratch::new(&[("sub/index.ts", source)]);
    let run = |dir: &std::path::Path, file: &str, extra: &[&str]| {
        let mut args = vec!["--verbose", "--json-full", "symbols", file];
        args.extend_from_slice(extra);
        let output = scratch.process(&args).current_dir(dir).output().expect("run hug");
        assert!(output.status.success(), "hug {args:?}: {}", String::from_utf8_lossy(&output.stderr));
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let json = json_of(&String::from_utf8(output.stdout).expect("utf-8 stdout"));
        let index = json["indexes"][0].clone();
        let markers: Vec<Value> = index["imports"].as_array().expect("imports").iter().map(|import| import["reexported"].clone()).collect();
        (stderr, index["reexports"].clone(), markers)
    };

    let (_, uncached, uncached_markers) = run(scratch.repo.path(), "sub/index.ts", &["--no-cache"]);
    assert_eq!(uncached["coverage"]["state"], "complete");
    assert_eq!(uncached["entries"].as_array().expect("entries").len(), 3, "{uncached}");
    assert_eq!(uncached_markers, [Value::Bool(true)]);

    let (cold_stats, cold, cold_markers) = run(scratch.repo.path(), "sub/index.ts", &[]);
    assert!(cold_stats.starts_with("cache: 0 hits, 0 partial hits, 1 misses"), "{cold_stats}");
    let (warm_stats, warm, warm_markers) = run(scratch.repo.path(), "sub/index.ts", &[]);
    assert!(warm_stats.starts_with("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"), "{warm_stats}");
    let (sub_stats, from_sub, sub_markers) = run(&scratch.repo.path().join("sub"), "index.ts", &[]);
    assert!(sub_stats.starts_with("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"), "{sub_stats}");

    for (label, report, markers) in [("cold", &cold, &cold_markers), ("warm", &warm, &warm_markers), ("other root", &from_sub, &sub_markers)] {
        assert_eq!(report, &uncached, "{label}");
        assert_eq!(markers, &uncached_markers, "{label}");
    }
}
