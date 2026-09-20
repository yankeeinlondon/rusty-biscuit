//! L1 tier of the archive-portability fixture.
//!
//! Every assertion here is about a payload class that has to survive being
//! compiled in one checkout and executed from an extracted archive in another,
//! with no compiler, linker, or producer target directory in reach.

use std::process::Command;

#[test]
fn the_dynamic_library_is_reachable_from_the_extracted_archive() {
    assert_eq!(
        archive_portability::dylib_marker(),
        "archive-portability-dylib"
    );
}

#[test]
fn build_script_generated_source_is_baked_into_the_binary() {
    assert_eq!(
        archive_portability::GENERATED_MARKER,
        "build-script-generated"
    );
}

#[test]
fn the_build_script_runtime_asset_travels_with_the_archive() {
    let found = archive_portability::build_script_asset()
        .expect("the build script's OUT_DIR asset must be on the consumer's search path");
    let body = std::fs::read_to_string(&found).expect("reading the build-script asset");
    assert_eq!(body.trim(), "build-script-asset");
}

#[test]
fn the_repository_fixture_resolves_through_the_remapped_workspace() {
    let path = archive_portability::repository_fixture();
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    assert_eq!(body.trim(), "repository-fixture");
}

#[test]
fn the_non_test_executable_is_archived_and_spawnable() {
    let exe = archive_portability::tool_exe(env!("CARGO_BIN_EXE_archive-portability-tool"));
    let output = Command::new(&exe)
        .output()
        .unwrap_or_else(|error| panic!("spawning {}: {error}", exe.display()));
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "archive-portability-tool archive-portability-dylib"
    );
}

#[test]
fn the_build_sidecar_is_reachable_on_the_consumer_path() {
    // Spawned by bare name: the consumer puts the verified sidecar directory on
    // PATH, which is how the monorepo's real `md` fixture is reached.
    let output = Command::new("archive-portability-sidecar")
        .output()
        .expect("the build sidecar must be on PATH");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "archive-portability-sidecar"
    );
}
