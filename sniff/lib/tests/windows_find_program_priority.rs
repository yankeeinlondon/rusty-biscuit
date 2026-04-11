//! Integration test: PATH must win over App Paths for the same program.
//!
//! Runs on Windows only; on other platforms the file compiles to an empty
//! module so the test binary still links.

#![cfg(target_os = "windows")]

use sniff::programs::{ExecutableIndex, ExecutableSource};

/// Any exe on the default Windows install that lives in PATH. `cmd.exe`
/// ships under `%SystemRoot%\System32`, which is always in PATH, and is
/// also not typically registered under App Paths — which means the test
/// only meaningfully tells us "PATH branch returned Path source".
#[test]
fn path_wins_over_fallbacks_for_cmd() {
    let index = ExecutableIndex::build();
    let (_, source) = index
        .find_with_source("cmd")
        .or_else(|| index.find_with_source("cmd.exe"))
        .expect("cmd.exe should always be on a Windows host");
    assert_eq!(source, ExecutableSource::Path);
}
