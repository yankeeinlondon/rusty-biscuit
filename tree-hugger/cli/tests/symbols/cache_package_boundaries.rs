//! The Rust package-`tests/` rule reads the package layout around a file, not
//! its source, so a warm cache must follow manifest changes: every command
//! consuming `is_test` (`--tests-only`, `--exclude-tests`, `--json`) gives
//! the fresh-analysis answer on a cache hit, while source-derived test
//! markers keep being served from the entry.

use serde_json::Value;

use crate::{Scratch, json_of, stdout_of};

const SAMPLE: &str = "tests/sample.rs";

/// What one file's functions look like to the test-flag consumers.
#[derive(Debug, PartialEq, Eq)]
struct Classification {
    tests_only: String,
    exclude_tests: String,
    /// `(name, is_test)` per `--json` record.
    json: Vec<(String, bool)>,
}

/// Runs the three consumers over `file`, from the cache (each run asserted a
/// complete hit) or with `--no-cache`.
fn classify(scratch: &Scratch, file: &str, cached: bool) -> Classification {
    let run = |args: &[&str]| {
        let mut full = args.to_vec();
        full.extend(["--file", file]);
        if cached {
            full.push("--verbose");
        } else {
            full.push("--no-cache");
        }
        let output = scratch.hug(&full);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if cached {
            assert!(
                stderr.starts_with("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"),
                "hug {full:?} was not a complete cache hit: {stderr}"
            );
        }
        stdout_of(output, &full)
    };
    let json = json_of(&run(&["--json", "functions"]));
    let mut records: Vec<(String, bool)> = json["files"][0]["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .map(|record: &Value| {
            (
                record["identity"]["name"].as_str().expect("name").to_string(),
                record["visibility"]["is_test"].as_bool().expect("is_test"),
            )
        })
        .collect();
    records.sort();
    Classification {
        tests_only: run(&["functions", "--tests-only", "--count"]).trim().to_string(),
        exclude_tests: run(&["functions", "--exclude-tests", "--count"]).trim().to_string(),
        json: records,
    }
}

/// Asserts that the cached and fresh classifications of `file` agree and
/// returns the cached one.
fn agreed(scratch: &Scratch, file: &str, step: &str) -> Classification {
    let warm = classify(scratch, file, true);
    let fresh = classify(scratch, file, false);
    assert_eq!(warm, fresh, "{step}: a cache hit must give the fresh classification");
    warm
}

fn sample_is_test(is_test: bool) -> Classification {
    Classification {
        tests_only: usize::from(is_test).to_string(),
        exclude_tests: usize::from(!is_test).to_string(),
        json: vec![("helper".to_string(), is_test)],
    }
}

/// Adding, nesting and removing manifests around an unchanged
/// `tests/sample.rs` changes its classification on cache hits exactly as it
/// does under fresh analysis.
#[test]
fn cached_package_test_flags_follow_manifest_changes() {
    let scratch = Scratch::new(&[(SAMPLE, "pub fn helper() {}\n")]);
    let root = scratch.repo.path();
    let outer = root.join("Cargo.toml");
    let nearer = root.join("tests").join("Cargo.toml");
    let manifest = "[package]\nname = \"sample\"\nversion = \"0.1.0\"\n";

    // The cold run writes an entry with the flags capability.
    let cold = scratch.hug(&["--verbose", "--json", "functions", "--file", SAMPLE]);
    let cold_stderr = String::from_utf8_lossy(&cold.stderr).into_owned();
    assert!(cold_stderr.starts_with("cache: 0 hits, 0 partial hits, 1 misses"), "{cold_stderr}");

    assert_eq!(agreed(&scratch, SAMPLE, "no manifest"), sample_is_test(false));

    std::fs::write(&outer, manifest).expect("write outer manifest");
    assert_eq!(agreed(&scratch, SAMPLE, "outer manifest added"), sample_is_test(true));

    // `tests/` is now its own package root, and `sample.rs` sits at its top.
    std::fs::write(&nearer, manifest).expect("write nearer manifest");
    assert_eq!(agreed(&scratch, SAMPLE, "nearer manifest added"), sample_is_test(false));

    std::fs::remove_file(&nearer).expect("remove nearer manifest");
    assert_eq!(agreed(&scratch, SAMPLE, "nearer manifest removed"), sample_is_test(true));

    std::fs::remove_file(&outer).expect("remove outer manifest");
    assert_eq!(agreed(&scratch, SAMPLE, "outer manifest removed"), sample_is_test(false));
}

/// Test attributes and `cfg(test)` containment come from the source, are
/// stored in the entry and survive a cache hit outside any `tests/`
/// directory, with or without a package around the file.
#[test]
fn source_derived_test_flags_survive_a_cache_hit() {
    const MARKED: &str = "src/marked.rs";
    let source = "pub fn plain() {}\n\n#[test]\nfn attributed() {}\n\n#[cfg(test)]\nmod checks {\n    fn inside_cfg_test() {}\n}\n";
    let scratch = Scratch::new(&[(MARKED, source)]);
    let expected = Classification {
        tests_only: "2".to_string(),
        exclude_tests: "1".to_string(),
        json: vec![
            ("attributed".to_string(), true),
            ("inside_cfg_test".to_string(), true),
            ("plain".to_string(), false),
        ],
    };

    let cold = scratch.hug(&["--json", "functions", "--file", MARKED]);
    assert!(cold.status.success(), "{}", String::from_utf8_lossy(&cold.stderr));
    assert_eq!(agreed(&scratch, MARKED, "no manifest"), expected);

    std::fs::write(scratch.repo.path().join("Cargo.toml"), "[package]\nname = \"m\"\nversion = \"0.1.0\"\n")
        .expect("write manifest");
    assert_eq!(agreed(&scratch, MARKED, "manifest added"), expected);
}
