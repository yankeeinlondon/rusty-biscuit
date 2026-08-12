//! Packaging contract: the release archive names `just dist` produces must
//! match the names the Zed extension downloads.
//!
//! `zed-dmls` is workspace-excluded (it targets wasm32) and neither crate
//! depends on the other, so the naming agreement cannot be enforced by the type
//! system. This test reads both source files and asserts the four per-platform
//! asset names line up on both sides. A drift here means the extension would
//! request an artifact `just dist` never publishes (a download 404), so the
//! server would work in every in-memory test yet fail to launch from Zed.
//!
//! Pure file reads — cross-platform, no shell, no build, non-flaky.

use std::fs;
use std::path::PathBuf;

/// The per-platform archive suffixes (after `dmls-<version>-`) the distribution
/// matrix defines. `just dist` writes these; the Zed extension's `asset_name`
/// downloads them.
const ASSET_SUFFIXES: [&str; 4] = [
    "macos-universal.tar.gz",
    "linux-x86_64.tar.gz",
    "linux-aarch64.tar.gz",
    "windows-x86_64.zip",
];

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn dist_recipe_and_zed_extension_agree_on_archive_names() {
    // `just dist` lives in the area justfile, one directory up from this crate.
    let justfile = fs::read_to_string(manifest_dir().join("../justfile"))
        .expect("read the darkmatter area justfile");
    // The Zed extension resolves the same names in its download logic.
    let zed_lib = fs::read_to_string(manifest_dir().join("zed-dmls/src/lib.rs"))
        .expect("read zed-dmls/src/lib.rs");

    for suffix in ASSET_SUFFIXES {
        assert!(
            justfile.contains(suffix),
            "`just dist` does not produce a `dmls-<version>-{suffix}` archive; \
             the Zed extension downloads it, so the extension would 404",
        );
        assert!(
            zed_lib.contains(suffix),
            "zed-dmls `asset_name` never requests a `dmls-<version>-{suffix}` \
             archive, but `just dist` publishes one — the extension cannot find it",
        );
    }

    // Both sides must use the `dmls-<version>-` stem so the version splice lines
    // up (justfile: `dmls-$version-…`; extension: `format!(\"dmls-{version}-…\")`).
    assert!(justfile.contains("dmls-$version-"), "dist recipe lost the versioned stem");
    assert!(zed_lib.contains("dmls-{version}-"), "extension lost the versioned stem");
}
