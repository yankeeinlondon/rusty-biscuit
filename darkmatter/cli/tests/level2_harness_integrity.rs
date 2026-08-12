//! Level 2 harness integrity tests.
//!
//! These tests do not exercise `md` behavior; they verify the Level 2
//! harness itself so the real-terminal suite cannot silently pass against
//! a stale host-installed `md` (review-2 finding #1). The shim helpers
//! support symlink, hard-link, and copy fallbacks so the suite stays
//! robust on Windows where symlink creation requires Developer Mode or
//! elevated privileges (review-3 finding).

mod common;

use common::level2::{
    assert_shim_resolves_to_built, is_same_binary, link_or_copy, md_bin, md_shim,
};
use std::fs;
use std::path::{Path, PathBuf};

/// The shim returned by [`md_shim`] resolves to the same `md` built for this
/// test binary. If this ever fails, every Level 2 test in the binary is
/// potentially exercising the wrong binary.
#[test]
fn md_shim_resolves_to_cargo_built_binary() {
    let shim_path = md_shim();
    assert!(
        is_same_binary(Path::new(shim_path), md_bin()),
        "md_shim() pointed at {} but the Cargo-built binary is {};\
         tests would run against the wrong binary",
        shim_path,
        md_bin().display(),
    );
}

/// [`assert_shim_resolves_to_built`] (called inside `md_shim`) accepts a
/// valid shim to the built binary and does not panic. Uses
/// [`link_or_copy`] so the test stays robust on hosts where symlink
/// creation is unavailable (review-3 finding).
#[test]
fn assert_shim_resolves_to_built_accepts_valid_link() {
    let dir = tempfile::tempdir().expect("tempdir");
    let link = dir.path().join("md-link");
    link_or_copy(md_bin(), &link).expect("link_or_copy md_bin");
    // Should not panic.
    assert_shim_resolves_to_built(&link);
}

/// [`assert_shim_resolves_to_built`] rejects a shim that does not point
/// at the Cargo-built binary. This pins the integrity check so future
/// harness changes cannot accidentally relax it. Uses [`link_or_copy`]
/// so the test stays robust on hosts where symlink creation is
/// unavailable (review-3 finding).
#[test]
#[should_panic(expected = "tests would run against the wrong binary")]
fn assert_shim_resolves_to_built_rejects_foreign_link() {
    let dir = tempfile::tempdir().expect("tempdir");
    let foreign = dir.path().join("foreign-bin");
    fs::write(&foreign, "#!/bin/sh\nexit 0\n").expect("write foreign binary");

    let link = dir.path().join("md-foreign");
    link_or_copy(&foreign, &link).expect("link_or_copy foreign");
    assert_shim_resolves_to_built(&link);
}

/// `run_md` invokes the shim path, not a bare `md`. This is a structural
/// assertion that the Level 2 default helper does not regress to a
/// `PATH`-resolved `md`. The shim returned by `md_shim` must be an
/// absolute path under the system temp dir; if it ever regresses to the
/// bare binary name (`md` on Unix, `md.exe` on Windows), the host `PATH`
/// would silently take over.
#[test]
fn md_shim_path_is_absolute_temp_dir_link() {
    let shim = PathBuf::from(md_shim());
    assert!(
        shim.is_absolute(),
        "md_shim returned {shim:?}; expected an absolute path",
    );
    // The shim lives at `<temp_dir>/dm-md-shim-<pid>/md[.exe]`. A bare
    // `md` is a single path segment with no parent directory.
    assert!(
        shim.parent().is_some(),
        "md_shim returned {shim:?}; expected a path with a parent directory",
    );
    // Guard against a future change that returns the literal binary name
    // (the most likely regression mode). On Windows this is `md.exe`;
    // on Unix it is `md`.
    let bare = if cfg!(windows) { "md.exe" } else { "md" };
    assert_ne!(
        shim.to_str(),
        Some(bare),
        "md_shim returned the bare binary name; the host PATH would silently \
         take over",
    );
}
