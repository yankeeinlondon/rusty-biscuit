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

#[test]
fn explicit_context_can_clear_the_ambient_home_snapshot() {
    let base = TempDir::new().unwrap();
    let ctx = FileResolutionContext::new(base.path()).without_home_dir();

    let error = FileReference::new("~/cfg.toml")
        .unwrap()
        .resolve_in_context(&ctx)
        .unwrap_err();

    assert!(matches!(error, FileReferenceError::MissingHomeContext));
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

#[test]
#[serial_test::serial]
fn derived_source_preserves_request_snapshot_after_ambient_mutation() {
    let repo = TempDir::new().unwrap();
    let first = repo.path().join("docs/first");
    let nested = repo.path().join("docs/nested");
    let home = repo.path().join("home");
    let magic = repo.path().join("magic");
    let vault = repo.path().join("vault");
    let package = repo.path().join("area");
    for dir in [&first, &nested, &home, &magic, &vault, &package] {
        fs::create_dir_all(dir).unwrap();
    }
    fs::write(nested.join("local.md"), b"nested").unwrap();
    fs::write(home.join("home.md"), b"home").unwrap();
    fs::write(magic.join("magic.md"), b"magic").unwrap();
    fs::write(vault.join("vault.md"), b"vault").unwrap();
    fs::write(package.join("package.md"), b"package").unwrap();
    fs::write(repo.path().join("captured.md"), b"env").unwrap();

    let mut env = std::collections::HashMap::new();
    env.insert(
        "SNAPSHOT_ROOT".to_string(),
        repo.path().display().to_string(),
    );
    let request = FileResolutionContext::new(&first)
        .with_source_path(first.join("root.md"))
        .with_repository_root(repo.path())
        .with_package_area(&package)
        .with_home_dir(&home)
        .with_env(env)
        .add_magic_path(&magic, PathPosition::Start)
        .add_vault(&vault);

    let ambient = TempDir::new().unwrap();
    let prior_cwd = std::env::current_dir().unwrap();
    let prior_home = std::env::var_os("HOME");
    let prior_snapshot_root = std::env::var_os("SNAPSHOT_ROOT");
    // SAFETY: the test is serialized while mutating process-global state.
    unsafe {
        std::env::set_var("HOME", ambient.path());
        std::env::set_var("SNAPSHOT_ROOT", ambient.path());
    }
    std::env::set_current_dir(ambient.path()).unwrap();

    let child = request.for_source(nested.join("child.md"));
    let results = [
        ("./local.md", nested.join("local.md")),
        ("~/home.md", home.join("home.md")),
        ("@magic.md", magic.join("magic.md")),
        ("vault:vault.md", vault.join("vault.md")),
        ("!package.md", package.join("package.md")),
        ("{{SNAPSHOT_ROOT}}/captured.md", repo.path().join("captured.md")),
    ]
    .map(|(raw, expected)| {
        let actual = FileReference::new(raw)
            .unwrap()
            .resolve_in_context(&child)
            .unwrap();
        (actual, expected)
    });

    std::env::set_current_dir(prior_cwd).unwrap();
    match prior_home {
        Some(value) => unsafe { std::env::set_var("HOME", value) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    match prior_snapshot_root {
        Some(value) => unsafe { std::env::set_var("SNAPSHOT_ROOT", value) },
        None => unsafe { std::env::remove_var("SNAPSHOT_ROOT") },
    }

    assert_eq!(child.base_dir(), nested.as_path());
    assert_eq!(child.source_path(), Some(nested.join("child.md").as_path()));
    assert_eq!(child.repository_root(), request.repository_root());
    assert_eq!(child.package_area(), request.package_area());
    assert_eq!(child.home_dir(), request.home_dir());
    assert_eq!(child.env(), request.env());
    for (actual, expected) in results {
        assert_eq!(actual.as_deref(), Some(expected.as_path()));
    }
}

#[test]
fn external_source_requires_explicit_trust() {
    let repo = TempDir::new().unwrap();
    let launch = repo.path().join("docs");
    let external = TempDir::new().unwrap();
    fs::create_dir_all(&launch).unwrap();
    fs::write(repo.path().join("shared.md"), b"shared").unwrap();
    fs::write(external.path().join("local.md"), b"local").unwrap();

    let request = FileResolutionContext::new(&launch).with_repository_root(repo.path());
    request.validate().unwrap();

    let child = request.for_source(external.path().join("prompt.md"));
    assert!(matches!(
        child.validate().unwrap_err(),
        FileReferenceError::RepositoryRootNotContainingSource { .. }
    ));

    let child = request.for_trusted_external_source(external.path().join("prompt.md"));
    child.validate().unwrap();
    assert_eq!(
        FileReference::new("shared.md")
            .unwrap()
            .resolve_in_context(&child)
            .unwrap()
            .as_deref(),
        Some(repo.path().join("shared.md").as_path()),
    );
    assert_eq!(
        FileReference::new("./local.md")
            .unwrap()
            .resolve_in_context(&child)
            .unwrap()
            .as_deref(),
        Some(external.path().join("local.md").as_path()),
    );
}

#[test]
fn invalid_request_root_cannot_be_laundered_via_for_source() {
    let repo = TempDir::new().unwrap();
    let unrelated_base = TempDir::new().unwrap();
    fs::write(repo.path().join("file.md"), b"x").unwrap();

    let request =
        FileResolutionContext::new(unrelated_base.path()).with_repository_root(repo.path());
    assert!(matches!(
        request.validate().unwrap_err(),
        FileReferenceError::RepositoryRootNotContainingSource { .. }
    ));

    let derived = request.for_source(repo.path().join("file.md"));
    assert!(matches!(
        derived.validate().unwrap_err(),
        FileReferenceError::RepositoryRootNotContainingSource { .. }
    ));
    let resolve_err = FileReference::new("file.md")
        .unwrap()
        .resolve_in_context(&derived)
        .unwrap_err();
    assert!(matches!(
        resolve_err,
        FileReferenceError::RepositoryRootNotContainingSource { .. }
    ));
}

#[test]
fn invalid_request_root_cannot_be_laundered_via_for_base_or_trusted_external_derivation() {
    let repo = TempDir::new().unwrap();
    let unrelated_base = TempDir::new().unwrap();
    let external = TempDir::new().unwrap();
    let request =
        FileResolutionContext::new(unrelated_base.path()).with_repository_root(repo.path());

    for derived in [
        request.for_base(repo.path()),
        request.for_trusted_external_source(external.path().join("child.md")),
        request.for_trusted_external_base(external.path()),
    ] {
        assert!(matches!(
            derived.validate().unwrap_err(),
            FileReferenceError::RepositoryRootNotContainingSource { .. }
        ));
    }
}

#[test]
fn containment_is_component_aware_and_lexically_normalized() {
    let root = TempDir::new().unwrap();
    let repo = root.path().join("repo");
    let sibling = root.path().join("repository-copy");
    fs::create_dir_all(repo.join("docs")).unwrap();
    fs::create_dir_all(&sibling).unwrap();

    let contained = FileResolutionContext::new(repo.join("docs/../docs"))
        .with_repository_root(&repo);
    assert!(contained.validate().is_ok());

    let prefix_collision = FileResolutionContext::new(&sibling).with_repository_root(&repo);
    assert!(matches!(
        prefix_collision.validate().unwrap_err(),
        FileReferenceError::RepositoryRootNotContainingSource { .. }
    ));
}

#[test]
fn valid_request_supports_in_repo_and_explicit_trusted_external_derivations() {
    let repo = TempDir::new().unwrap();
    let launch = repo.path().join("docs");
    let external_dir = TempDir::new().unwrap();
    fs::create_dir_all(&launch).unwrap();
    fs::write(repo.path().join("child.md"), b"child").unwrap();
    fs::write(repo.path().join("shared.md"), b"shared").unwrap();
    fs::write(external_dir.path().join("prompt.md"), b"prompt").unwrap();

    let request = FileResolutionContext::new(&launch).with_repository_root(repo.path());
    request.validate().unwrap();

    let in_repo = request.for_source(repo.path().join("child.md"));
    in_repo.validate().unwrap();

    let external =
        request.for_trusted_external_source(external_dir.path().join("prompt.md"));
    external.validate().unwrap();
    request
        .for_trusted_external_base(external_dir.path())
        .validate()
        .unwrap();

    let shared = FileReference::new("shared.md")
        .unwrap()
        .resolve_in_context(&external)
        .unwrap();
    assert_eq!(shared.as_deref(), Some(repo.path().join("shared.md").as_path()));
}

#[test]
fn normal_derivation_reenables_containment_after_trusted_external_derivation() {
    let repo = TempDir::new().unwrap();
    let launch = repo.path().join("docs");
    let external = TempDir::new().unwrap();
    fs::create_dir_all(&launch).unwrap();
    let request = FileResolutionContext::new(&launch).with_repository_root(repo.path());

    let trusted = request.for_trusted_external_source(external.path().join("a.md"));
    trusted.validate().unwrap();

    let nested = trusted.for_source(external.path().join("b.md"));
    assert!(matches!(
        nested.validate().unwrap_err(),
        FileReferenceError::RepositoryRootNotContainingSource { .. }
    ));
}
