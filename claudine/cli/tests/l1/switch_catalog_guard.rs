//! Provider switch types come from research, through generated code only.
//!
//! `claudine-gen` projects the `agent-cli` research into each generated
//! `lib/src/provider/<slug>/data.rs`, and every reader goes through
//! `claudine::provider::lookup_switch`. A switch record, value type, or
//! catalog written by hand anywhere else would be a second table that drifts
//! from the research and that the generator's validation never sees.
//!
//! Those types decide who owns each argument after a composition file once
//! per invocation; retries, resumes, proxy targets, and `sequence` steps
//! recheck the tail against their provider but never classify again. A
//! second classification call would let a launch path reassign a setter.
//!
//! These guards scan the non-test source of both crates, comments and string
//! literals blanked by [`source_scan::sanitize`].

use crate::common::source_scan::{is_ident, line_at, sanitize};

use std::fs;
use std::path::{Path, PathBuf};

/// Constructions of a switch table: a record literal, an array of records,
/// or a researched catalog over a literal slice. Matching on a catalog or a
/// `SwitchValue` reads the research and is allowed; ownership has to.
const CONSTRUCTORS: &[&str] = &["CliSwitch {", "[CliSwitch", "CliSwitchCatalog::Researched(&"];

fn area_root() -> PathBuf {
    biscuit_test_harness::manifest_dir!()
        .parent()
        .expect("claudine-cli lives inside the claudine area")
        .to_path_buf()
}

fn is_test_path(relative: &str) -> bool {
    relative.split('/').any(|segment| segment == "tests" || segment == "tests.rs")
}

/// Generated provider data: `lib/src/provider/<slug>/data.rs`.
fn is_generated(relative: &str) -> bool {
    relative.starts_with("lib/src/provider/") && relative.ends_with("/data.rs")
}

fn rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

fn sources() -> Vec<(String, String)> {
    let root = area_root();
    let mut files = Vec::new();
    rust_files(&root.join("lib/src"), &mut files);
    rust_files(&root.join("cli/src"), &mut files);
    files
        .into_iter()
        .filter_map(|path| {
            let relative = biscuit_file::to_portable_string(path.strip_prefix(&root).unwrap());
            (!is_test_path(&relative)).then(|| (relative, fs::read_to_string(&path).unwrap()))
        })
        .collect()
}

/// Files (in order, one entry per occurrence) where `needle` appears as an
/// identifier, not as the tail of a longer one.
fn identifier_sites(files: &[(String, String)], needle: &str) -> Vec<String> {
    files
        .iter()
        .flat_map(|(file, source)| {
            let sanitized = sanitize(source);
            sanitized
                .windows(needle.len())
                .enumerate()
                .filter(|(offset, window)| {
                    *window == needle.as_bytes() && (*offset == 0 || !is_ident(sanitized[offset - 1]))
                })
                .map(|_| file.clone())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn sites(source: &str) -> Vec<(usize, &'static str)> {
    let sanitized = sanitize(source);
    CONSTRUCTORS
        .iter()
        .flat_map(|needle| {
            sanitized
                .windows(needle.len())
                .enumerate()
                .filter(|(_, window)| *window == needle.as_bytes())
                .map(|(offset, _)| (line_at(source, offset), *needle))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn switch_metadata_is_never_written_by_hand() {
    let files = sources();
    let strays: Vec<_> = files
        .iter()
        .filter(|(file, _)| !is_generated(file))
        .flat_map(|(file, source)| {
            sites(source)
                .into_iter()
                .map(move |(line, needle)| format!("{file}:{line} {needle}"))
        })
        .collect();
    assert!(
        strays.is_empty(),
        "provider switch types come from the agent-cli research through claudine-gen; \
         read them with claudine::provider::lookup_switch instead of writing them here:\n{}",
        strays.join("\n")
    );
    // The allowed site must still be what the guard thinks it is.
    assert!(
        files
            .iter()
            .any(|(file, source)| is_generated(file) && !sites(source).is_empty()),
        "no generated data.rs carries a researched switch; did the generated catalog move?"
    );
}

/// Ownership is decided once per invocation: the pure classifier has one
/// production caller, and that caller is reached only from the two command
/// entry points (`compose`/`inline-compose` share `prep.rs`), never from a
/// retry, resume, proxy, or step launch path.
#[test]
fn composition_arguments_are_classified_once_per_invocation() {
    let files = sources();
    assert_eq!(
        identifier_sites(&files, "own_arguments("),
        ["lib/src/composition/ownership.rs", "cli/src/commands/compose/ownership.rs"],
        "only `own_caller_arguments` classifies (plus the definition)"
    );
    assert_eq!(
        identifier_sites(&files, "own_caller_arguments("),
        [
            "cli/src/commands/compose/ownership.rs",
            "cli/src/commands/compose/prep.rs",
            "cli/src/commands/sequence.rs",
        ],
        "each command entry point classifies once (plus the definition)"
    );
}
