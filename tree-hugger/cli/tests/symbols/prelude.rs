//! `--prelude` on records: re-export aliases listed at their declarations as
//! synthetic records, unresolved re-exports, `PRELUDE` names, targets outside
//! the scan, and one parse per file across the prelude and scan phases.

use serde_json::Value;
use tree_hugger::file_identity;

use crate::{Scratch, json_of, stdout_of};

const ALPHA: &str = include_str!("../fixtures/prelude_pkg/src/alpha.rs");
const PRELUDE: &str = "use crate::alpha::VerboseEnum;\n\
                       pub use crate::alpha::{VerboseEnum as ElevatedEnum, VerboseType};\n\
                       pub use crate::missing::Ghost;\n";

fn package() -> Scratch {
    Scratch::new(&[
        ("Cargo.toml", "[package]\nname = \"prelude-scratch\"\nversion = \"0.0.0\"\n"),
        ("src/lib.rs", "pub mod alpha;\npub mod extra;\npub mod prelude;\n"),
        ("src/alpha.rs", ALPHA),
        ("src/prelude.rs", PRELUDE),
        ("src/extra.rs", "pub fn env_named() {}\npub fn not_named() {}\n"),
    ])
}

fn entry<'a>(json: &'a Value, file: &str) -> Option<&'a Value> {
    json["files"].as_array().expect("files").iter().find(|entry| entry["file"] == file)
}

fn record<'a>(records: &'a Value, name: &str) -> &'a Value {
    records
        .as_array()
        .expect("symbols")
        .iter()
        .find(|record| record["identity"]["name"] == name)
        .unwrap_or_else(|| panic!("no `{name}` record in {records}"))
}

/// A public re-export is listed under its exported name at its target's
/// declaration, marked synthetic and re-exported, with an ID of its own; a
/// private `use` is not an export, and an unresolved one stays listed at the
/// `use`.
#[test]
fn prelude_exports_are_synthetic_records_at_their_declarations() {
    let scratch = package();
    let args = ["--json", "symbols", "--prelude"];
    let json = json_of(&stdout_of(scratch.hug_with_env(&args, &[("PRELUDE", "env_named")]), &args));
    let alpha = json_of(&scratch.stdout(&["--json", "symbols", "src/alpha.rs"]));
    let alpha_records = &entry(&alpha, "src/alpha.rs").expect("alpha.rs entry")["symbols"];
    let target = record(alpha_records, "VerboseEnum");

    let exports = &entry(&json, "src/prelude.rs").expect("the prelude lists its exports")["symbols"];
    let mut names: Vec<&str> = exports
        .as_array()
        .expect("symbols")
        .iter()
        .map(|record| record["identity"]["name"].as_str().expect("name"))
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["ElevatedEnum", "Ghost", "VerboseType"], "VerboseEnum's private `use` is not an export");

    let alias = record(exports, "ElevatedEnum");
    assert_eq!(alias["kind"], "Enum");
    assert_eq!(alias["identity"]["keyword"], "enum");
    assert_eq!(alias["source"]["file_path"], "src/alpha.rs", "listed at the declaration");
    assert_eq!(alias["source"]["declaration_span"], target["source"]["declaration_span"]);
    assert_ne!(alias["id"], target["id"], "a synthetic record never borrows the target's ID");
    assert_eq!(alias["visibility"]["is_synthetic"], true);
    assert_eq!(alias["visibility"]["is_reexported"], true);
    assert!(alias["docs"]["raw_doc"].as_str().is_some_and(|doc| doc.contains("Enum exported through")));
    assert!(alias["kind_data"]["data"].is_null(), "the alias carries none of the target's own data");

    // Located at the target, keyed by the prelude: both paths root-relative.
    for name in ["ElevatedEnum", "VerboseType"] {
        let alias = record(exports, name);
        assert_eq!(alias["source"]["file_path"], "src/alpha.rs", "{name}");
        let key = alias["identity"]["stable_key"].as_str().expect("stable_key");
        let kind = alias["kind"].as_str().expect("kind");
        assert!(
            key.starts_with(&format!("rust::src/prelude.rs::{kind}::prelude-export::{name}::")),
            "{name}: {key}"
        );
    }

    let ghost = record(exports, "Ghost");
    assert_eq!(ghost["kind"], "Unknown", "an unresolved re-export keeps an unknown kind");
    assert_eq!(ghost["source"]["file_path"], "src/prelude.rs", "listed at the `use`");
    assert_eq!(ghost["visibility"]["is_synthetic"], true);

    // Other files keep only the names `PRELUDE` adds.
    let extra = &entry(&json, "src/extra.rs").expect("extra.rs entry")["symbols"];
    assert_eq!(extra.as_array().expect("symbols").len(), 1);
    assert_eq!(record(extra, "env_named")["kind"], "Function");
    assert!(entry(&json, "src/alpha.rs").is_none(), "alpha.rs declares no prelude-only names");

    let plain = scratch.stdout(&["--plain", "symbols", "--prelude"]);
    assert!(plain.contains("- enum ElevatedEnum src/alpha.rs:"), "{plain}");
    assert!(plain.contains("- unknown Ghost src/prelude.rs:3:"), "{plain}");
}

/// `PRELUDE` names select declarations for every command.
#[test]
fn prelude_environment_names_select_declarations() {
    let scratch = package();
    let args = ["--plain", "functions", "--prelude"];
    let stdout = stdout_of(scratch.hug_with_env(&args, &[("PRELUDE", "env_named")]), &args);
    assert!(stdout.contains("env_named()"), "{stdout}");
    assert!(!stdout.contains("not_named"), "{stdout}");
}

/// A narrowed scan still resolves exports through a target module it does
/// not list: the target is analyzed but has no file entry of its own.
#[test]
fn a_target_outside_the_scan_is_resolved_but_not_listed() {
    let scratch = package();
    let json = json_of(&scratch.stdout(&["--json", "symbols", "--prelude", "src/lib.rs"]));
    let files: Vec<&str> = json["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|entry| entry["file"].as_str().expect("file"))
        .collect();
    assert_eq!(files, ["src/prelude.rs"], "the prelude is added; alpha.rs is not listed");
    let exports = &entry(&json, "src/prelude.rs").expect("prelude entry")["symbols"];
    assert_eq!(record(exports, "VerboseType")["source"]["file_path"], "src/alpha.rs");
}

/// The prelude phase's analyses are reused by the scan: every file is parsed
/// once, with or without the persistent cache.
#[test]
fn prelude_and_scan_parse_each_file_once() {
    let scratch = package();
    let stats = |args: &[&str]| -> String {
        let output = scratch.hug(args);
        assert!(output.status.success(), "hug {args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        stderr.lines().find(|line| line.starts_with("cache: ")).unwrap_or_else(|| panic!("stats line in {stderr}")).to_string()
    };

    // Four files; the prelude phase analyzes prelude.rs and its target
    // alpha.rs, which the scan narrows instead of parsing again.
    let uncached = stats(&["--verbose", "--json", "symbols", "--prelude", "--no-cache"]);
    assert!(uncached.contains("2 reused in this run"), "{uncached}");
    assert!(uncached.contains("parsed 4 files, 4 definition captures"), "{uncached}");

    let cold = stats(&["--verbose", "--json", "functions", "--prelude"]);
    assert!(cold.contains("parsed 4 files"), "{cold}");
    let warm = stats(&["--verbose", "--json", "functions", "--prelude"]);
    assert!(warm.contains("parsed 0 files"), "a warm prelude run parses nothing: {warm}");
}

/// A forced-language explicit file is reused from the prelude phase under
/// every spelling of it, for all four commands: the run parses the prelude
/// and `alpha.rs` once each, and the report (IDs included) does not depend
/// on the spelling. The canonical absolute spelling is the control.
#[test]
fn prelude_reuse_holds_for_every_spelling_of_an_explicit_file() {
    let scratch = package();
    let canonical = file_identity(&scratch.repo.path().join("src/alpha.rs"));
    let as_given = scratch.repo.path().join("src").join("alpha.rs");
    let spellings = [
        "src/alpha.rs".to_string(),
        "src/../src/alpha.rs".to_string(),
        as_given.to_str().expect("utf-8 temp path").to_string(),
        canonical.to_str().expect("utf-8 temp path").to_string(),
    ];
    for command in ["functions", "types", "symbols", "classes"] {
        let mut reports = Vec::new();
        for spelling in &spellings {
            let args = ["--json", command, "--prelude", "--language", "rust", spelling.as_str(), "--no-cache", "--verbose"];
            let output = scratch.hug(&args);
            assert!(output.status.success(), "hug {args:?}: {}", String::from_utf8_lossy(&output.stderr));
            let stderr = String::from_utf8_lossy(&output.stderr);
            let line = stderr.lines().find(|line| line.starts_with("cache: ")).unwrap_or_else(|| panic!("stats line in {stderr}"));
            // `symbols` also lists the prelude file, so it reuses two.
            let reused = if command == "symbols" { 2 } else { 1 };
            assert!(line.contains(&format!(", {reused} reused in this run; parsed 2 files")), "hug {args:?}: {line}");
            reports.push(stdout_of(output, &args));
        }
        let control = reports.last().expect("the canonical control ran");
        for (spelling, report) in spellings.iter().zip(&reports) {
            assert_eq!(report, control, "hug {command} {spelling}: same report as the canonical spelling");
        }
    }
}

/// The record selectors apply to the prelude's synthetic export records as
/// to any other record.
#[test]
fn selectors_apply_to_prelude_exports() {
    let scratch = package();
    let names = |args: &[&str]| -> Vec<String> {
        let json = json_of(&stdout_of(scratch.hug_with_env(args, &[("PRELUDE", "env_named")]), args));
        let mut names: Vec<String> = entry(&json, "src/prelude.rs")
            .map(|entry| {
                entry["symbols"]
                    .as_array()
                    .expect("symbols")
                    .iter()
                    .map(|record| record["identity"]["name"].as_str().expect("name").to_string())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    };
    assert_eq!(names(&["--json", "symbols", "--prelude"]), ["ElevatedEnum", "Ghost", "VerboseType"]);
    assert_eq!(names(&["--json", "symbols", "--prelude", "--name", "elevated"]), ["ElevatedEnum"]);
    assert_eq!(names(&["--json", "symbols", "--prelude", "--exclude-symbols", "Ghost"]), ["ElevatedEnum", "VerboseType"]);
    assert_eq!(names(&["--json", "symbols", "--prelude", "--kind", "function"]), Vec::<String>::new());
}

/// A one-line prelude re-exporting a type declared on line 2 of another
/// file: every export locates the alias at its declaration, never at the
/// prelude it is analyzed and keyed with.
fn cross_file_package() -> Scratch {
    Scratch::new(&[
        ("Cargo.toml", "[package]\nname = \"cross-file\"\nversion = \"0.0.0\"\nedition = \"2021\"\n"),
        ("src/lib.rs", "pub mod alpha;\npub mod prelude;\n"),
        ("src/alpha.rs", "\npub struct Original;\n"),
        ("src/prelude.rs", "pub use crate::alpha::Original as Alias;\n"),
    ])
}

/// Stdout of `hug` run from `dir` (relative to the repository), so the
/// working directory differs from the package-root scan root.
fn stdout_from(scratch: &Scratch, dir: &str, args: &[&str]) -> String {
    let output = scratch.process(args).current_dir(scratch.repo.path().join(dir)).output().expect("run hug");
    stdout_of(output, args)
}

#[test]
fn prelude_exports_locate_cross_file_aliases_at_their_declaration_in_every_format() {
    let scratch = cross_file_package();
    let declared = file_identity(&scratch.repo.path().join("src/alpha.rs"));
    let url = url::Url::from_file_path(&declared).expect("native file URL");

    let tsv = scratch.stdout(&["symbols", "--prelude", "--format", "tsv"]);
    let rows: Vec<Vec<&str>> = tsv.lines().skip(1).map(|line| line.split('\t').collect()).collect();
    assert_eq!(rows.len(), 1, "{tsv}");
    assert_eq!(&rows[0][..5], ["src/alpha.rs", "2", "1", "type", "Alias"], "{tsv}");
    assert!(rows[0][5].starts_with("src/prelude.rs#"), "the qualified-name prefix stays the prelude's: {tsv}");

    let prose = scratch.stdout(&["symbols", "--prelude", "--format", "prose"]);
    assert!(
        prose.contains(&format!("<a href=\"{url}#L2\">src/alpha.rs:2:1</a>")),
        "label and link both name the declaration ({url}#L2):\n{prose}"
    );
    assert!(!prose.contains("prelude.rs#L"), "{prose}");

    let markdown = scratch.stdout(&["symbols", "--prelude", "--format", "markdown"]);
    assert_eq!(markdown.trim_end(), "- struct **Alias** src/alpha.rs:2:1", "{markdown}");

    let tags = scratch.stdout(&["symbols", "--prelude", "--format", "ctags"]);
    assert!(tags.lines().all(|line| line.starts_with("!_TAG_")), "a synthetic alias has no tag:\n{tags}");

    // Records are projected once, against the scan root, wherever `hug` runs.
    let nested = stdout_from(&scratch, "src", &["symbols", "--prelude", "--format", "tsv"]);
    assert_eq!(nested, tsv, "running from src/ changes no TSV path");
    let tags = stdout_from(&scratch, "src", &["symbols", "--format", "ctags"]);
    let tag = tags.lines().find(|line| line.starts_with("Original\t")).unwrap_or_else(|| panic!("{tags}"));
    assert_eq!(tag, "Original\tsrc/alpha.rs\t2;\"\tkind:type", "{tags}");
}
