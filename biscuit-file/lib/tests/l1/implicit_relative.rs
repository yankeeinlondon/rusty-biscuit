//! Integration tests for the `ImplicitRelative` reference kind.
//!
//! These tests build a real git repository in a temp directory so the
//! `find_git_root` logic in `biscuit-file` can discover it the same way it
//! would in production.

use std::fs;
use std::path::Path;

use biscuit_file::FileReference;
use tempfile::TempDir;

/// Initialize a fresh git repository at `path`.
fn git_init(path: &Path) {
    // Use gix directly so we don't depend on a system git binary.
    gix::init(path).expect("git init failed");
}

/// Canonicalize into the spelling the resolver reports.
///
/// `std::fs::canonicalize` returns a `\\?\` verbatim path on Windows, but the
/// resolver reduces every anchor to the legacy form -- and so does `gix`
/// worktree discovery, which supplies the repository leg of these expectations.
fn canonical(path: &Path) -> std::path::PathBuf {
    dunce::canonicalize(path).expect("canonicalize")
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
    let subdir = canonical(&subdir);
    let repo_root_canon = canonical(repo_root);

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

    let subdir = canonical(&subdir);
    let resolved = FileReference::new("notes.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(subdir.join("notes.md").as_path()),
        "CWD should take priority over the git root for implicit relative refs"
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
    let subdir = canonical(&subdir);

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
    let subdir = canonical(&subdir);
    let repo_root_canon = canonical(repo_root);

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
    let subdir = canonical(&subdir);

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

    let dir_canon = canonical(dir);

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
    let repo_root_canon = canonical(repo_root);

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
    let subdir = canonical(&subdir);
    let repo_root_canon = canonical(repo_root);

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
        let subdir = canonical(&subdir);

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
        let repo_root_canon = canonical(repo_root);

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
    let subdir = canonical(&subdir);
    let repo_root_canon = canonical(repo_root);

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

mod partial_completion {
    //! Integration tests for `FileReference::complete_partial`.
    //!
    //! Tests that need a controlled `HOME` value run serially so they
    //! don't trample the ambient environment used by other tests.

    use super::*;
    use biscuit_file::{CompletionEntryForm, FileReference};
    use serial_test::serial;
    use std::env;

    /// Guard that restores the original HOME on drop.
    struct HomeGuard {
        previous: Option<std::ffi::OsString>,
    }

    impl HomeGuard {
        fn set(new_home: &Path) -> Self {
            let previous = env::var_os("HOME");
            // Safety: callers serialize these via `#[serial]`.
            unsafe { env::set_var("HOME", new_home) };
            Self { previous }
        }
    }

    impl Drop for HomeGuard {
        fn drop(&mut self) {
            // Safety: callers serialize these via `#[serial]`.
            match &self.previous {
                Some(v) => unsafe { env::set_var("HOME", v) },
                None => unsafe { env::remove_var("HOME") },
            }
        }
    }

    /// The home root the resolver reports while `guard_home` is installed as
    /// `$HOME`.
    ///
    /// POSIX home discovery honors `$HOME`, so the guard's directory is what
    /// appears. Native Windows resolves the profile directory through the OS
    /// known-folder API and ignores `HOME` outright (D11), so the guard cannot
    /// redirect it and the native profile is the only home leg there.
    #[cfg(not(windows))]
    fn expected_home(guard_home: &Path) -> std::path::PathBuf {
        guard_home.to_path_buf()
    }

    #[cfg(windows)]
    fn expected_home(_guard_home: &Path) -> std::path::PathBuf {
        biscuit_file::home_dir().expect("native Windows profile directory")
    }

    #[test]
    #[serial]
    fn magic_partial_includes_repo_and_home_roots() {
        let tmp_repo = TempDir::new().unwrap();
        let repo_root = tmp_repo.path();
        git_init(repo_root);
        let repo_root_canon = canonical(repo_root);

        let tmp_home = TempDir::new().unwrap();
        let home_canon = canonical(tmp_home.path());
        let _home_guard = HomeGuard::set(&home_canon);

        let completion = FileReference::complete_partial("@p", &repo_root_canon)
            .unwrap()
            .expect("magic form is supported");

        assert_eq!(completion.entry_form(), CompletionEntryForm::Magic);
        assert_eq!(completion.rendered_prefix(), "@");
        assert_eq!(completion.active_segment(), "p");
        assert_eq!(
            completion.roots(),
            &[repo_root_canon.clone(), expected_home(&home_canon)],
            "repo root must precede home root for @",
        );
    }

    #[test]
    #[serial]
    fn magic_with_scope_appends_scope_to_each_root() {
        let tmp_repo = TempDir::new().unwrap();
        let repo_root = tmp_repo.path();
        git_init(repo_root);
        let repo_root_canon = canonical(repo_root);

        let tmp_home = TempDir::new().unwrap();
        let home_canon = canonical(tmp_home.path());
        let _home_guard = HomeGuard::set(&home_canon);

        let completion = FileReference::complete_partial("@prompts/ab", &repo_root_canon)
            .unwrap()
            .expect("magic form is supported");

        assert_eq!(completion.entry_form(), CompletionEntryForm::Magic);
        assert_eq!(completion.rendered_prefix(), "@prompts/");
        assert_eq!(completion.active_segment(), "ab");
        assert_eq!(
            completion.roots(),
            &[
                repo_root_canon.join("prompts"),
                expected_home(&home_canon).join("prompts"),
            ],
        );
    }

    #[test]
    #[serial]
    fn magic_without_repo_only_returns_home_root() {
        // Use a temp dir that is not a git repo as the base.
        let base = TempDir::new().unwrap();
        let base_canon = canonical(base.path());

        let tmp_home = TempDir::new().unwrap();
        let home_canon = canonical(tmp_home.path());
        let _home_guard = HomeGuard::set(&home_canon);

        let completion = FileReference::complete_partial("@pr", &base_canon)
            .unwrap()
            .expect("magic form is supported");

        assert_eq!(completion.entry_form(), CompletionEntryForm::Magic);
        assert_eq!(completion.rendered_prefix(), "@");
        assert_eq!(completion.active_segment(), "pr");
        assert_eq!(
            completion.roots(),
            &[expected_home(&home_canon)],
            "only the home leg contributes when base is not inside a git repo",
        );
    }

    #[test]
    #[serial]
    fn magic_without_home_env_still_discovers_native_home() {
        let tmp_repo = TempDir::new().unwrap();
        let repo_root = tmp_repo.path();
        git_init(repo_root);
        let repo_root_canon = canonical(repo_root);

        // Unset HOME while the test runs. Unlike a bare `$HOME` read, the shared
        // cross-platform provider still supplies the native profile home (D11),
        // so the home leg persists in the magic search order.
        let previous_home = env::var_os("HOME");
        unsafe { env::remove_var("HOME") };

        let completion = FileReference::complete_partial("@p", &repo_root_canon)
            .unwrap()
            .expect("magic form is supported");
        // Capture the provider's home under the same HOME-unset condition the
        // completion saw, so the expectation tracks the same source.
        let native_home = biscuit_file::home_dir();

        // Restore HOME before asserting so a panic still cleans up.
        if let Some(v) = previous_home {
            unsafe { env::set_var("HOME", v) };
        }

        let mut expected = vec![repo_root_canon];
        expected.extend(native_home);
        assert_eq!(
            completion.roots(),
            expected.as_slice(),
            "the native home leg persists even when $HOME is unset",
        );
    }

    #[test]
    fn implicit_relative_with_scope_inside_repo() {
        let tmp = TempDir::new().unwrap();
        let repo_root = tmp.path();
        git_init(repo_root);
        let repo_root_canon = canonical(repo_root);
        let subdir = repo_root.join("pkg");
        fs::create_dir_all(&subdir).unwrap();
        let subdir = canonical(&subdir);

        let completion = FileReference::complete_partial("prompts/ab", &subdir)
            .unwrap()
            .expect("implicit-relative form is supported");

        assert_eq!(
            completion.entry_form(),
            CompletionEntryForm::ImplicitRelative
        );
        assert_eq!(completion.rendered_prefix(), "prompts/");
        assert_eq!(completion.active_segment(), "ab");
        assert_eq!(
            completion.roots(),
            &[subdir.join("prompts"), repo_root_canon.join("prompts")],
            "CWD-relative root precedes the git-root-relative fallback",
        );
    }

    #[test]
    fn implicit_relative_base_is_git_root_dedupes_to_one_root() {
        let tmp = TempDir::new().unwrap();
        let repo_root = tmp.path();
        git_init(repo_root);
        let repo_root_canon = canonical(repo_root);

        let completion = FileReference::complete_partial("", &repo_root_canon)
            .unwrap()
            .expect("implicit-relative form is supported");

        assert_eq!(completion.roots(), &[repo_root_canon]);
    }

    #[test]
    fn implicit_relative_without_repo_only_returns_base() {
        // Temp directory with no git init.
        let tmp = TempDir::new().unwrap();
        let base = canonical(tmp.path());

        let completion = FileReference::complete_partial("prompts/", &base)
            .unwrap()
            .expect("implicit-relative form is supported");

        assert_eq!(
            completion.roots(),
            &[base.join("prompts")],
            "no git root means no repo-root entry",
        );
        assert_eq!(completion.active_segment(), "");
        assert_eq!(completion.rendered_prefix(), "prompts/");
    }

    #[test]
    fn separator_reset_empties_active_segment() {
        let tmp = TempDir::new().unwrap();
        let base = canonical(tmp.path());

        let completion = FileReference::complete_partial("prompts/", &base)
            .unwrap()
            .expect("implicit-relative form is supported");
        assert_eq!(completion.active_segment(), "");

        let completion = FileReference::complete_partial("prompts/sub/", &base)
            .unwrap()
            .expect("implicit-relative form is supported");
        assert_eq!(completion.active_segment(), "");
        assert_eq!(completion.rendered_prefix(), "prompts/sub/");
    }

    #[test]
    fn unsupported_forms_return_none() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();

        assert!(matches!(
            FileReference::complete_partial("!foo", base),
            Err(biscuit_file::FileReferenceError::InvalidSyntax(_))
        ));

        for token in &[
            "/abs/path",
            "./rel.md",
            "../rel.md",
            ".",
            "..",
            "vault:notes",
            "%foo",
            "%@foo",
            "{{DIR}}/x.md",
        ] {
            let result = FileReference::complete_partial(token, base).unwrap();
            assert!(
                result.is_none(),
                "expected None for unsupported token `{token}`",
            );
        }
    }
}
