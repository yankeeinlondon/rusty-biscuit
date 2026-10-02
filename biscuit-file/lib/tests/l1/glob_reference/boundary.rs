//! The relative boundary for bare, `./`, and `../` globs, the roots that are
//! not bound by it, and symlinks that lead out of the tree.

use biscuit_file::{FileReference, FileReferenceError, GlobReferenceError, SkippedEntry};

use super::{Fixture, glob, link_dir, link_file, listed, snapshot};

/// Criterion 14: a bare or `./` glob whose root climbs above the tree is a
/// typed error, not an empty result, unless the reader opted in.
#[test]
fn relative_globs_cannot_leave_the_tree() {
    let fx = Fixture::new();
    let outside = fx.file("outside.md");
    let ctx = fx.ctx(&fx.repo);

    for pattern in ["../*.md", "./../*.md", "a/../../*.md"] {
        match glob(&[pattern]).list_files(&ctx) {
            Err(GlobReferenceError::RelativeTreeEscape {
                pattern: named,
                base_dir,
                ..
            }) => {
                assert_eq!(named, pattern);
                assert_eq!(base_dir, fx.repo);
            }
            other => panic!("`{pattern}` must escape the tree, got {other:?}"),
        }
        assert_eq!(
            glob(&[pattern]).take_first(&ctx).unwrap_err().resolution_failure(),
            biscuit_file::ResolutionFailure::InvalidReference
        );
    }

    let opted_in = ctx.clone().allow_external_relative();
    assert_eq!(listed(&["../*.md"], &opted_in), [outside]);
}

/// Criterion 14: `~`, `@`, absolute, vault, and `{{VAR}}` roots are not
/// relative and may lie outside the repository.
#[test]
fn unbound_roots_may_lie_outside_the_repository() {
    let fx = Fixture::new();
    let note = fx.file("home/notes/n.md");
    let outside = fx.file("outside/o.md");
    let vault = fx.file("vault/v.md");
    let outside_dir = fx.root.join("outside");
    let outside_text = outside_dir.to_str().unwrap();
    let ctx = fx
        .ctx_with_env(&fx.pkg, &[("OUT", outside_text)])
        .add_vault(fx.root.join("vault"));

    assert_eq!(listed(&["~/notes/*.md"], &ctx), std::slice::from_ref(&note));
    assert_eq!(listed(&["@notes/*.md"], &ctx), [note]);
    let absolute = format!("{}/*.md", biscuit_file::to_portable_string(&outside_dir));
    assert_eq!(listed(&[absolute.as_str()], &ctx), std::slice::from_ref(&outside));
    assert_eq!(listed(&["vault:*.md"], &ctx), [vault]);
    assert_eq!(listed(&["{{OUT}}/*.md"], &ctx), [outside]);
}

/// Criterion 14: `&` and `^` stay repository-bound.
#[test]
fn repository_sigils_outside_a_repository_are_typed_errors() {
    let fx = Fixture::new();
    let ctx = snapshot(&fx.root.join("outside"), Some(&fx.home), &[]);

    for (pattern, expected) in [("&**/*.md", '&'), ("^**/*.md", '^')] {
        match glob(&[pattern]).list_files(&ctx) {
            Err(GlobReferenceError::OutsideRepository { pattern: named, sigil, .. }) => {
                assert_eq!((named.as_str(), sigil), (pattern, expected));
            }
            other => panic!("`{pattern}` outside a repository, got {other:?}"),
        }
        assert!(glob(&[pattern]).roots(&ctx).is_empty());
        assert!(!glob(&[pattern]).matches(&fx.root.join("outside/x.md"), &ctx));
    }
}

/// Criterion 14: a walk never follows a directory symlink, so nothing behind
/// one that leads out of the tree is listed.
#[test]
fn a_walk_does_not_follow_a_directory_link() {
    let fx = Fixture::new();
    let kept = fx.file("repo/docs/kept.md");
    fx.file("outside/shared/secret.md");
    link_dir(&fx.root.join("outside/shared"), &fx.repo.join("docs/shared"));
    let ctx = fx.ctx(&fx.repo);

    assert_eq!(listed(&["docs/**/*.md"], &ctx), std::slice::from_ref(&kept));
    assert_eq!(listed(&["&docs/**/*.md"], &ctx), [kept]);
}

/// Criterion 25 (biscuit-file part): a bound glob skips a file symlink whose
/// target leaves the tree and reports it; an in-tree link is listed; an
/// unbound glob skips nothing; a single-file reference to the link still
/// fails.
#[test]
fn a_file_link_out_of_the_tree_is_skipped_by_a_bound_glob() {
    let fx = Fixture::new();
    let secret = fx.file("outside/secret.md");
    let real = fx.file("repo/docs/real.md");
    let leak = fx.repo.join("docs/leak.md");
    let inside = fx.repo.join("docs/inside.md");
    link_file(&secret, &leak);
    link_file(&real, &inside);
    let ctx = fx.ctx(&fx.repo);

    for pattern in ["docs/*.md", "./docs/*.md"] {
        let listing = glob(&[pattern]).list_files(&ctx).unwrap();
        assert_eq!(listing.matches, [inside.clone(), real.clone()], "{pattern}");
        assert_eq!(
            listing.skipped,
            [SkippedEntry {
                link: leak.clone(),
                target: secret.clone(),
            }],
            "{pattern}"
        );
    }

    let unbound = glob(&["&docs/*.md"]).list_files(&ctx).unwrap();
    assert_eq!(unbound.matches, [inside.clone(), leak.clone(), real.clone()]);
    assert!(unbound.skipped.is_empty());

    // The reader opt-in removes the boundary, so nothing is skipped.
    let opted_in = ctx.clone().allow_external_relative();
    assert!(glob(&["docs/*.md"]).list_files(&opted_in).unwrap().skipped.is_empty());

    assert!(matches!(
        FileReference::new("docs/leak.md").unwrap().resolve_in_context(&ctx),
        Err(FileReferenceError::RelativeTreeEscape { .. })
    ));
}
