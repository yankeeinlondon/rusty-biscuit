//! Equivalent spellings of one file are one analysis identity: they share a
//! persistent-cache entry, are listed once, and are reported under the same
//! root-relative path. A symlink shares its target's identity.

use std::path::Path;

use tree_hugger::file_identity;

use crate::{Scratch, cache_entries, stdout_of};

const ALPHA: &str = include_str!("../fixtures/prelude_pkg/src/alpha.rs");

fn package() -> Scratch {
    Scratch::new(&[
        ("Cargo.toml", "[package]\nname = \"spelling-scratch\"\nversion = \"0.0.0\"\n"),
        ("src/alpha.rs", ALPHA),
    ])
}

fn utf8(path: &Path) -> String {
    path.to_str().expect("utf-8 temp path").to_string()
}

/// The relative, canonical absolute and `..` spellings of `src/alpha.rs`,
/// plus the temporary directory's own absolute spelling (on macOS `/var`,
/// whose canonical form is `/private/var`).
fn spellings(scratch: &Scratch) -> Vec<String> {
    vec![
        "src/alpha.rs".to_string(),
        utf8(&file_identity(&scratch.repo.path().join("src/alpha.rs"))),
        "src/../src/alpha.rs".to_string(),
        utf8(&scratch.repo.path().join("src").join("alpha.rs")),
    ]
}

/// The `--verbose` stats line of a successful run.
fn stats_line(scratch: &Scratch, args: &[&str]) -> String {
    let output = scratch.hug(args);
    assert!(output.status.success(), "hug {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    stderr
        .lines()
        .find(|line| line.starts_with("cache: "))
        .unwrap_or_else(|| panic!("hug {args:?}: no stats line in {stderr}"))
        .to_string()
}

/// Under one isolated cache, the first spelling misses and writes the one
/// entry; every other spelling hits it without parsing.
#[test]
fn equivalent_spellings_share_one_cache_entry() {
    let scratch = package();
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut spellings = spellings(&scratch);
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("src/alpha.rs", scratch.repo.path().join("alpha_link.rs")).expect("symlink");
        spellings.push("alpha_link.rs".to_string());
    }

    for (run, spelling) in spellings.iter().enumerate() {
        let args = ["--verbose", "--json", "functions", "--language", "rust", spelling.as_str()];
        let line = stats_line(&scratch, &args);
        let expected = if run == 0 {
            "cache: 0 hits, 0 partial hits, 1 misses; parsed 1 files"
        } else {
            "cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"
        };
        assert!(line.starts_with(expected), "hug {args:?}: {line}");
        assert_eq!(cache_entries(scratch.temp.path()).len(), 1, "hug {args:?}: one entry per file");
    }
}

/// A file named twice under different spellings, or named and also matched
/// by a glob, is analyzed and listed once, and every report spells it the
/// same root-relative way.
#[test]
fn a_file_named_under_several_spellings_is_listed_once() {
    let scratch = package();
    let spellings = spellings(&scratch);
    let control = stdout_of(
        scratch.hug(&["--plain", "types", "--language", "rust", "--no-cache", "src/alpha.rs"]),
        &["control"],
    );
    assert_eq!(control.matches("src/alpha.rs:").count(), 2, "two types listed once each:\n{control}");

    let mut cases: Vec<Vec<String>> = spellings.iter().map(|spelling| vec![spelling.clone()]).collect();
    cases.push(spellings.clone());
    cases.push(vec!["src/../src/alpha.rs".to_string(), "src/*.rs".to_string()]);
    for inputs in cases {
        let mut args = vec!["--plain", "types", "--language", "rust", "--no-cache", "--verbose"];
        args.extend(inputs.iter().map(String::as_str));
        let output = scratch.hug(&args);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        assert!(stderr.contains("parsed 1 files"), "hug {args:?}: one analysis: {stderr}");
        assert_eq!(stdout_of(output, &args), control, "hug {args:?}: the same listing");

        let mut json_args = vec!["--json", "types", "--language", "rust", "--no-cache"];
        json_args.extend(inputs.iter().map(String::as_str));
        let json = crate::json_of(&scratch.stdout(&json_args));
        let files: Vec<&str> = json["files"]
            .as_array()
            .expect("files")
            .iter()
            .map(|entry| entry["file"].as_str().expect("file"))
            .collect();
        assert_eq!(files, ["src/alpha.rs"], "hug {json_args:?}: one root-relative entry");
    }
}

/// A literal positional path (which walks only toward the file) and a glob
/// matching the same file (the full walk) reach one identity: one cache
/// entry, hit by the second, and the same report. The temporary root is an
/// alias on macOS (`/var`), so the walk's own spelling is not the identity.
#[test]
fn literal_and_glob_selections_share_one_identity() {
    let scratch = package();
    let mut reports = Vec::new();
    for (run, input) in ["src/alpha.rs", "src/alpha.r[s]", "src/*.rs"].into_iter().enumerate() {
        let line = stats_line(&scratch, &["--verbose", "--json", "functions", input]);
        let expected = if run == 0 {
            "cache: 0 hits, 0 partial hits, 1 misses; parsed 1 files"
        } else {
            "cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"
        };
        assert!(line.starts_with(expected), "{input}: {line}");
        assert_eq!(cache_entries(scratch.temp.path()).len(), 1, "{input}: one entry");
        reports.push(scratch.stdout(&["--json", "functions", input]));
    }
    assert!(reports.windows(2).all(|pair| pair[0] == pair[1]), "one report: {reports:#?}");
}
