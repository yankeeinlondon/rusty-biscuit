//! Every archive payload class the producer/consumer contract has to carry,
//! in one package small enough to compile in seconds.
//!
//! The fixture proves *archive plumbing*. It deliberately does not depend on
//! the monorepo's test harness, a terminal backend, or a browser: a failure
//! here means an archive did not relocate, never that a production backend was
//! unavailable.

use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

pub use archive_portability_dylib::dylib_marker;

/// The repository fixture this package reads at run time.
///
/// Rooted in the *runtime* `CARGO_MANIFEST_DIR` rather than the compile-time
/// `env!`, which is what `--workspace-remap` rewrites. A relocated run that
/// consulted the baked path would look for the producer's checkout.
pub fn repository_fixture() -> PathBuf {
    let root = env::var_os("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|| OsString::from(env!("CARGO_MANIFEST_DIR")));
    PathBuf::from(root).join("fixtures/data.txt")
}

/// Absolute path to the fixture's non-test executable.
///
/// `NEXTEST_BIN_EXE_<name>` is nextest's run-time republication of where it
/// extracted the binary; `compiled` is the caller's compile-time
/// `CARGO_BIN_EXE_…` constant, which names the producer's target directory and
/// is the fallback of last resort. Cargo publishes that constant only to
/// integration-test targets, which is why it is a parameter rather than an
/// `env!` here — the same shape as `biscuit_test_harness::bin_exe!`.
pub fn tool_exe(compiled: &str) -> PathBuf {
    for name in [
        "NEXTEST_BIN_EXE_archive_portability_tool",
        "CARGO_BIN_EXE_archive-portability-tool",
    ] {
        if let Some(value) = env::var_os(name) {
            if !value.is_empty() {
                return PathBuf::from(value);
            }
        }
    }
    PathBuf::from(compiled)
}

/// The build script's run-time asset, found through the linked-path directories
/// nextest republishes on the consumer's dynamic-library search path.
///
/// ## Returns
///
/// `None` when no search path contains the asset, which is the shape a missing
/// build-script output takes on a consumer.
pub fn build_script_asset() -> Option<PathBuf> {
    // One variable per platform loader; nextest writes the archived linked
    // paths into whichever one this host uses.
    let vars = [
        "LD_LIBRARY_PATH",
        "DYLD_FALLBACK_LIBRARY_PATH",
        "DYLD_LIBRARY_PATH",
        "PATH",
    ];
    for var in vars {
        let Some(value) = env::var_os(var) else {
            continue;
        };
        for entry in env::split_paths(&value) {
            let candidate = entry.join("build-script-asset.txt");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
