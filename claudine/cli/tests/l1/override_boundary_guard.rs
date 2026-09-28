//! Claudine hands frontmatter overrides to Darkmatter in exactly one place.
//!
//! `LayeredOverrides::apply_to` (`lib/src/composition/runtime_state.rs`) splits
//! the run's overrides by origin: what a person typed stays a template, and
//! everything the run produced — agent and task output, lifecycle `set:`, loop
//! values, the step overlay, a `proxy.with:` overlay — is data Darkmatter never
//! scans. A second call to one of Darkmatter's override builders anywhere else
//! would be a second path whose origin nobody decided, and would quietly turn
//! run output back into a template. So would a runtime module that reached for
//! the on-disk literal token, which belongs to persistence only.
//!
//! This guard scans the non-test source of both crates, comments and string
//! literals blanked by [`source_scan::sanitize`].

use crate::common::source_scan::{line_at, sanitize};

use std::fs;
use std::path::{Path, PathBuf};

/// Darkmatter's override builders on `ComposeOptions`.
const BUILDERS: &[&str] = &[
    ".with_set_overrides(",
    ".with_data_overrides(",
    ".with_override_layers(",
];

/// The one file allowed to call a builder, relative to the `claudine/` area.
const BOUNDARY: &str = "lib/src/composition/runtime_state.rs";

/// Runtime-layer source that must never encode a literal token: values stay
/// raw and typed at runtime; the token exists only in a persisted file.
const RUNTIME_LAYERS: &[&str] = &[
    "lib/src/composition/runtime_state.rs",
    "lib/src/composition/looping",
    "lib/src/composition/sequence",
    "cli/src/commands/wrap/sequence",
    "cli/src/commands/wrap/overlay.rs",
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

/// Every non-test source file of both crates, as `(area-relative path, text)`.
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

fn hits(source: &str, needle: &str) -> Vec<usize> {
    let sanitized = sanitize(source);
    sanitized
        .windows(needle.len())
        .enumerate()
        .filter(|(_, window)| *window == needle.as_bytes())
        .map(|(offset, _)| line_at(source, offset))
        .collect()
}

fn builder_sites(files: &[(String, String)]) -> Vec<String> {
    files
        .iter()
        .flat_map(|(file, source)| {
            BUILDERS.iter().flat_map(move |builder| {
                hits(source, builder)
                    .into_iter()
                    .map(move |line| format!("{file}:{line} {builder}"))
            })
        })
        .collect()
}

#[test]
fn overrides_reach_darkmatter_only_through_the_layered_boundary() {
    let files = sources();
    let strays: Vec<_> = builder_sites(&files)
        .into_iter()
        .filter(|site| !site.starts_with(&format!("{BOUNDARY}:")))
        .collect();
    assert!(
        strays.is_empty(),
        "hand overrides to Darkmatter through `LayeredOverrides::apply_to`, never directly:\n{}",
        strays.join("\n")
    );
    assert!(
        !builder_sites(
            &files
                .iter()
                .filter(|(file, _)| file == BOUNDARY)
                .cloned()
                .collect::<Vec<_>>()
        )
        .is_empty(),
        "the boundary itself must still call a builder; did it move?"
    );
}

#[test]
fn runtime_layers_never_encode_a_literal_token() {
    let offenders: Vec<_> = sources()
        .into_iter()
        .filter(|(file, _)| RUNTIME_LAYERS.iter().any(|layer| file.starts_with(layer)))
        .flat_map(|(file, source)| {
            hits(&source, "literal_token")
                .into_iter()
                .map(move |line| format!("{file}:{line}"))
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "runtime values stay raw; only persistence encodes a token:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn the_scan_sees_a_builder_call_but_not_prose_about_it() {
    let source = "// .with_set_overrides(x)\nlet s = \".with_set_overrides(\";\nopts.with_set_overrides(v);\n";
    assert_eq!(hits(source, ".with_set_overrides("), vec![3]);
}
