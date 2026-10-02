//! Provider switch types come from research, through generated code only.
//!
//! `claudine-gen` projects the `agent-cli` research into each generated
//! `lib/src/provider/<slug>/data.rs`, and every reader goes through
//! `claudine::provider::lookup_switch`. A switch record, value type, or
//! catalog written by hand anywhere else would be a second table that drifts
//! from the research and that the generator's validation never sees.
//!
//! This guard scans the non-test source of both crates, comments and string
//! literals blanked by [`source_scan::sanitize`].

use crate::common::source_scan::{line_at, sanitize};

use std::fs;
use std::path::{Path, PathBuf};

/// Constructors of switch metadata. `SwitchValue::Unknown` is absent: it is
/// what a reader answers for an unestablished switch, not a type claim.
const CONSTRUCTORS: &[&str] = &[
    "CliSwitch {",
    "CliSwitchCatalog::Researched(",
    "SwitchValue::None",
    "SwitchValue::String",
    "SwitchValue::Number",
    "SwitchValue::Variadic",
    "VariadicMin::AtLeast(",
];

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
