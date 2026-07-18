//! Public-API tests for `FileReference::resolve_in_context` and
//! `FileResolutionContext`.
//!
//! These exercise the explicit, caller-owned resolution context introduced by
//! the file-resolution work: a caller-suppliable repository root (no `sniff`
//! dependency, no live git needed), home-pinned `~` resolution, and lexical
//! containment validation. Phase 4 flipped the *precedence* to repository-root
//! first, then base, so these assert that order plus the new context plumbing.

use std::fs;

use biscuit_file::{FileReference, FileReferenceError, FileResolutionContext, PathPosition};
use tempfile::TempDir;

#[test]
fn implicit_is_repository_first_with_caller_supplied_repo_root() {
    // Collision: same filename at both the base dir and the supplied repo root.
    let repo = TempDir::new().unwrap();
    let base = repo.path().join("pkg");
    fs::create_dir_all(&base).unwrap();

    fs::write(repo.path().join("notes.md"), b"root").unwrap();
    fs::write(base.join("notes.md"), b"base").unwrap();

    let ctx = FileResolutionContext::new(&base).with_repository_root(repo.path());

    let resolved = FileReference::new("notes.md")
        .unwrap()
        .resolve_in_context(&ctx)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(repo.path().join("notes.md").as_path()),
        "Phase 4 resolves implicit repository-first even with a supplied repo root",
    );
}

#[test]
fn implicit_falls_back_to_caller_supplied_repo_root() {
    // No live git repository here: the base is a plain temp dir. The file
    // exists only at the *supplied* repository root, proving the caller root
    // participates as an implicit fallback candidate.
    let repo = TempDir::new().unwrap();
    let base = repo.path().join("pkg");
    fs::create_dir_all(&base).unwrap();
    fs::write(repo.path().join("root_only.md"), b"root").unwrap();

    let ctx = FileResolutionContext::new(&base).with_repository_root(repo.path());

    let resolved = FileReference::new("root_only.md")
        .unwrap()
        .resolve_in_context(&ctx)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(repo.path().join("root_only.md").as_path()),
        "implicit reference must fall back to the caller-supplied repo root",
    );
}

#[test]
fn explicit_relative_never_falls_back_to_repo_root() {
    let repo = TempDir::new().unwrap();
    let base = repo.path().join("pkg");
    fs::create_dir_all(&base).unwrap();
    fs::write(repo.path().join("root_only.md"), b"root").unwrap();

    let ctx = FileResolutionContext::new(&base).with_repository_root(repo.path());

    let resolved = FileReference::new("./root_only.md")
        .unwrap()
        .resolve_in_context(&ctx)
        .unwrap();

    assert!(
        resolved.is_none(),
        "`./` pins to base only; must not reach the repo root, got {resolved:?}",
    );
}

#[test]
fn home_reference_resolves_against_context_home() {
    let home = TempDir::new().unwrap();
    fs::write(home.path().join("cfg.toml"), b"cfg").unwrap();

    let base = TempDir::new().unwrap();
    let ctx = FileResolutionContext::new(base.path()).with_home_dir(home.path());

    let resolved = FileReference::new("~/cfg.toml")
        .unwrap()
        .resolve_in_context(&ctx)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(home.path().join("cfg.toml").as_path()),
        "`~/cfg.toml` must resolve under the context home directory",
    );
}

/// Native Windows must discover the user's home from the OS profile API, not
/// the frequently-unset `HOME` variable (D11 / Acceptance Criterion 11).
///
/// Unlike the injected-`with_home_dir` case above, this exercises default
/// discovery at the capture boundary: `HOME` is cleared, yet `~/...` still
/// resolves under the native profile directory.
#[cfg(target_os = "windows")]
#[test]
#[serial_test::serial]
fn home_reference_resolves_from_native_profile_without_home_env() {
    let native_home = biscuit_file::home_dir().expect("native Windows profile directory");

    // A uniquely named probe inside the real profile directory; dropped on
    // scope exit so the test leaves the home directory as it found it.
    let probe = tempfile::Builder::new()
        .prefix("biscuit-file-home-probe-")
        .suffix(".toml")
        .tempfile_in(&native_home)
        .expect("write a probe file into the native home directory");
    let file_name = probe
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();

    let prior_home = std::env::var_os("HOME");
    // SAFETY: env mutation is serialized against other tests via `#[serial]`.
    unsafe { std::env::remove_var("HOME") };

    let base = TempDir::new().unwrap();
    let ctx = FileResolutionContext::new(base.path());
    let resolved = FileReference::new(&format!("~/{file_name}"))
        .unwrap()
        .resolve_in_context(&ctx);

    // Restore `HOME` before asserting so a panic cannot leak process state.
    if let Some(prior) = prior_home {
        // SAFETY: see above.
        unsafe { std::env::set_var("HOME", prior) };
    }

    assert_eq!(
        resolved.unwrap().as_deref(),
        Some(probe.path()),
        "`~/...` must resolve under the native profile directory without `$HOME`",
    );
}

/// On POSIX, default home discovery (no injected `with_home_dir`) must honor
/// `$HOME`, the mirror of the native-Windows profile path above.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn home_reference_resolves_from_home_env_on_posix() {
    let home = TempDir::new().unwrap();
    fs::write(home.path().join("cfg.toml"), b"cfg").unwrap();

    let prior_home = std::env::var_os("HOME");
    // SAFETY: env mutation is serialized against other tests via `#[serial]`.
    unsafe { std::env::set_var("HOME", home.path()) };

    let base = TempDir::new().unwrap();
    let ctx = FileResolutionContext::new(base.path());
    let resolved = FileReference::new("~/cfg.toml")
        .unwrap()
        .resolve_in_context(&ctx);

    // Restore `HOME` before asserting so a panic cannot leak process state.
    match prior_home {
        // SAFETY: see above.
        Some(prior) => unsafe { std::env::set_var("HOME", prior) },
        None => unsafe { std::env::remove_var("HOME") },
    }

    assert_eq!(
        resolved.unwrap().as_deref(),
        Some(home.path().join("cfg.toml").as_path()),
        "`~/cfg.toml` must resolve under `$HOME` by default on POSIX",
    );
}

#[test]
fn magic_roots_come_from_the_context() {
    let magic = TempDir::new().unwrap();
    fs::write(magic.path().join("commit.md"), b"prompt").unwrap();

    let base = TempDir::new().unwrap();
    let ctx = FileResolutionContext::new(base.path())
        .add_magic_path(magic.path(), PathPosition::Start);

    let resolved = FileReference::new("@commit.md")
        .unwrap()
        .resolve_in_context(&ctx)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(magic.path().join("commit.md").as_path()),
        "context-configured magic roots must be honored",
    );
}

#[test]
fn repository_root_not_containing_base_is_a_typed_error() {
    let repo = TempDir::new().unwrap();
    let unrelated_base = TempDir::new().unwrap();

    let ctx = FileResolutionContext::new(unrelated_base.path()).with_repository_root(repo.path());

    // Both the direct validation and the resolve entry point must reject it.
    let validate_err = ctx.validate().unwrap_err();
    assert!(matches!(
        validate_err,
        FileReferenceError::RepositoryRootNotContainingSource { .. }
    ));

    let resolve_err = FileReference::new("notes.md")
        .unwrap()
        .resolve_in_context(&ctx)
        .unwrap_err();
    assert!(
        matches!(
            resolve_err,
            FileReferenceError::RepositoryRootNotContainingSource { .. }
        ),
        "resolve_in_context must validate containment; got {resolve_err}",
    );
}

#[test]
fn repository_root_containing_base_validates() {
    let repo = TempDir::new().unwrap();
    let base = repo.path().join("a/b");
    fs::create_dir_all(&base).unwrap();

    let ctx = FileResolutionContext::new(&base).with_repository_root(repo.path());
    assert!(ctx.validate().is_ok(), "contained base must validate");
}

#[test]
fn context_accessors_reflect_builders() {
    let repo = TempDir::new().unwrap();
    let base = repo.path().join("pkg");
    fs::create_dir_all(&base).unwrap();

    let ctx = FileResolutionContext::new(&base)
        .with_repository_root(repo.path())
        .with_source_path(base.join("router.md"))
        .with_package_area(repo.path().join("area"));

    assert_eq!(ctx.base_dir(), base.as_path());
    assert_eq!(ctx.repository_root(), Some(repo.path()));
    assert_eq!(ctx.source_path(), Some(base.join("router.md").as_path()));
    assert_eq!(ctx.package_area(), Some(repo.path().join("area").as_path()));
}
