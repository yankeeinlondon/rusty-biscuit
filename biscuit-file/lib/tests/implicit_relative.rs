//! Integration tests for the `ImplicitRelative` reference kind.
//!
//! These tests build a real git repository in a temp directory so the
//! `find_git_root` logic in `biscuit-file` can discover it the same way it
//! would in production.

use std::fs;
use std::path::Path;

use biscuit_file::FileReference;
use tempfile::TempDir;

/// Initialise a fresh git repository at `path`.
fn git_init(path: &Path) {
    // Use git2 directly so we don't depend on a system git binary.
    git2::Repository::init(path).expect("git init failed");
}

#[test]
fn resolves_file_in_git_root_when_absent_from_cwd() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    // Create `file_in_root.md` at the repo root.
    fs::write(repo_root.join("file_in_root.md"), b"root").unwrap();

    // Create a nested subdir that will act as the "CWD".
    let subdir = repo_root.join("sub/dir");
    fs::create_dir_all(&subdir).unwrap();

    // Canonicalize to match what the resolver does internally.
    let subdir = subdir.canonicalize().unwrap();
    let repo_root_canon = repo_root.canonicalize().unwrap();

    let resolved = FileReference::new("file_in_root.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(repo_root_canon.join("file_in_root.md").as_path()),
        "implicit relative path should fall back to git root"
    );
}

#[test]
fn prefers_cwd_over_git_root_on_name_collision() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    // Same filename exists in both git root and a subdirectory.
    fs::write(repo_root.join("notes.md"), b"root").unwrap();
    let subdir = repo_root.join("pkg");
    fs::create_dir_all(&subdir).unwrap();
    fs::write(subdir.join("notes.md"), b"subdir").unwrap();

    let subdir = subdir.canonicalize().unwrap();

    let resolved = FileReference::new("notes.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(subdir.join("notes.md").as_path()),
        "CWD should take priority over git root for implicit relative refs"
    );
}

#[test]
fn explicit_relative_does_not_fall_back_to_git_root() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    fs::write(repo_root.join("file_in_root.md"), b"root").unwrap();
    let subdir = repo_root.join("sub");
    fs::create_dir_all(&subdir).unwrap();
    let subdir = subdir.canonicalize().unwrap();

    let resolved = FileReference::new("./file_in_root.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert!(
        resolved.is_none(),
        "./ prefix should pin lookup to CWD only; got {resolved:?}"
    );
}

#[test]
fn subdir_path_resolves_against_git_root() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    fs::create_dir_all(repo_root.join("foo/bar")).unwrap();
    fs::write(repo_root.join("foo/bar/doc.md"), b"nested").unwrap();

    // CWD is the repo root's sibling subdir with no `foo/bar/doc.md`.
    let subdir = repo_root.join("other");
    fs::create_dir_all(&subdir).unwrap();
    let subdir = subdir.canonicalize().unwrap();
    let repo_root_canon = repo_root.canonicalize().unwrap();

    let resolved = FileReference::new("foo/bar/doc.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(repo_root_canon.join("foo/bar/doc.md").as_path()),
    );
}

#[test]
fn returns_none_when_neither_cwd_nor_git_root_has_file() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    let subdir = repo_root.join("pkg");
    fs::create_dir_all(&subdir).unwrap();
    let subdir = subdir.canonicalize().unwrap();

    let resolved = FileReference::new("does_not_exist.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert!(resolved.is_none());
}

#[test]
fn resolves_outside_any_git_repo() {
    // No `git_init` — plain temp dir.
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    fs::write(dir.join("loose.md"), b"loose").unwrap();

    let dir_canon = dir.canonicalize().unwrap();

    let resolved = FileReference::new("loose.md")
        .unwrap()
        .resolve_from(&dir_canon)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(dir_canon.join("loose.md").as_path()),
        "implicit relative should resolve from CWD when no git root exists",
    );
}

#[test]
fn resolves_when_cwd_is_git_root() {
    // Dedup guard: CWD == git root should produce a single root, not two.
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    fs::write(repo_root.join("top.md"), b"top").unwrap();
    let repo_root_canon = repo_root.canonicalize().unwrap();

    let resolved = FileReference::new("top.md")
        .unwrap()
        .resolve_from(&repo_root_canon)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(repo_root_canon.join("top.md").as_path()),
        "file at git root should resolve when CWD is the git root"
    );
}

#[test]
fn recursive_implicit_relative_with_subdir_filter() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    // a/docs/spec.md under the git root; sibling CWD doesn't contain it.
    fs::create_dir_all(repo_root.join("a/docs")).unwrap();
    fs::write(repo_root.join("a/docs/spec.md"), b"spec").unwrap();

    // Also write a file with the same name but not under `docs/` to ensure
    // the subdir filter is enforced.
    fs::create_dir_all(repo_root.join("b/other")).unwrap();
    fs::write(repo_root.join("b/other/spec.md"), b"other").unwrap();

    let subdir = repo_root.join("other");
    fs::create_dir_all(&subdir).unwrap();
    let subdir = subdir.canonicalize().unwrap();
    let repo_root_canon = repo_root.canonicalize().unwrap();

    let resolved = FileReference::new("%docs/spec.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(repo_root_canon.join("a/docs/spec.md").as_path()),
        "subdir filter should require parent path to end with `docs`",
    );
}

mod ambient_cwd {
    //! Tests that mutate the process CWD must run serially.

    use super::*;
    use serial_test::serial;
    use std::env;

    /// Guard that restores the original CWD on drop.
    struct CwdGuard {
        original: std::path::PathBuf,
    }

    impl CwdGuard {
        fn set(new_cwd: &Path) -> Self {
            let original = env::current_dir().expect("current_dir");
            env::set_current_dir(new_cwd).expect("set_current_dir");
            Self { original }
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.original);
        }
    }

    #[test]
    #[serial]
    fn resolve_relative_with_implicit_relative() {
        let tmp = TempDir::new().unwrap();
        let repo_root = tmp.path();
        git_init(repo_root);

        fs::write(repo_root.join("file_in_root.md"), b"root").unwrap();
        let subdir = repo_root.join("sub/dir");
        fs::create_dir_all(&subdir).unwrap();
        let subdir = subdir.canonicalize().unwrap();

        let _guard = CwdGuard::set(&subdir);

        // ImplicitRelative resolves via CWD → git root. Then diff_paths from
        // `subdir` back up to the file at the git root.
        let relative = FileReference::new("file_in_root.md")
            .unwrap()
            .resolve_relative(Some(&subdir))
            .unwrap();

        assert_eq!(
            relative,
            Some(std::path::PathBuf::from("../../file_in_root.md")),
            "resolve_relative should return `../../` path from nested CWD to git-root file",
        );
    }

    #[test]
    #[serial]
    fn env_var_interpolation_resolves_via_implicit_relative() {
        let tmp = TempDir::new().unwrap();
        let repo_root = tmp.path();
        git_init(repo_root);

        fs::create_dir_all(repo_root.join("docs")).unwrap();
        fs::write(repo_root.join("docs/readme.md"), b"doc").unwrap();
        let repo_root_canon = repo_root.canonicalize().unwrap();

        // Use a name unlikely to collide with ambient env.
        let var_name = "BISCUIT_FILE_IMPLICIT_REL_TEST_DIR";
        // Safety: single-threaded (serial), scoped with cleanup.
        unsafe { env::set_var(var_name, "docs") };

        let result = FileReference::new(&format!("{{{{{var_name}}}}}/readme.md"))
            .unwrap()
            .resolve_from(&repo_root_canon);

        unsafe { env::remove_var(var_name) };

        let resolved = result.unwrap();
        assert_eq!(
            resolved.as_deref(),
            Some(repo_root_canon.join("docs/readme.md").as_path()),
            "interpolated ImplicitRelative should resolve against CWD/git root",
        );
    }
}

#[test]
fn recursive_implicit_relative_finds_file_under_git_root() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    // Deeply nested file.
    fs::create_dir_all(repo_root.join("a/b/c")).unwrap();
    fs::write(repo_root.join("a/b/c/deep.md"), b"deep").unwrap();

    // CWD is a sibling that won't see `deep.md` via its own walk.
    let subdir = repo_root.join("other");
    fs::create_dir_all(&subdir).unwrap();
    let subdir = subdir.canonicalize().unwrap();
    let repo_root_canon = repo_root.canonicalize().unwrap();

    let resolved = FileReference::new("%deep.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(repo_root_canon.join("a/b/c/deep.md").as_path()),
        "recursive search should include git root as a traversal start"
    );
}
