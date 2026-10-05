//! Authored directory names are case-sensitive in every pattern form,
//! including an absolute pattern whose literal directories become its root.
//!
//! The mismatched spelling reaches the real directory only on a
//! case-insensitive filesystem (the macOS and Windows default), which is where
//! these tests catch a regression; on a case-sensitive one (Linux, or a
//! case-sensitive macOS volume) the directory simply does not exist. The
//! Unicode fixtures (`ς` for a stored `Σ`, `ß` for a stored `SS`) are aliases
//! that lowercasing does not reveal. Whether an alias opens is probed on the
//! fixture's own filesystem, never inferred from the OS, and every exact-name
//! and mismatch check runs either way.

use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, to_portable_string};
use tempfile::TempDir;

use super::{Fixture, glob, link_dir, snapshot};

/// Assert every public membership view rejects `file` for `pattern`.
pub(super) fn assert_rejects(pattern: &str, file: &Path, ctx: &FileResolutionContext) {
    let globs = glob(&[pattern]);
    assert!(globs.list_files(ctx).unwrap().matches.is_empty(), "`{pattern}` lists nothing");
    assert_eq!(globs.take_first(ctx).unwrap(), None, "`{pattern}` has no first match");
    assert!(!globs.matches(file, ctx), "`{pattern}` does not match {}", file.display());
    assert!(!globs.lists_file(file, ctx), "`{pattern}` does not list {}", file.display());
}

/// Assert every public membership view admits `file` (and only it) for
/// `pattern`.
pub(super) fn assert_admits(pattern: &str, file: &Path, ctx: &FileResolutionContext) {
    let globs = glob(&[pattern]);
    assert_eq!(globs.list_files(ctx).unwrap().matches, [file], "`{pattern}` lists the file");
    assert_eq!(globs.take_first(ctx).unwrap().as_deref(), Some(file), "`{pattern}` first");
    assert!(globs.matches(file, ctx), "`{pattern}` matches {}", file.display());
    assert!(globs.lists_file(file, ctx), "`{pattern}` lists {}", file.display());
}

pub(super) fn absolute(dir: &Path, rest: &str) -> String {
    format!("{}/{rest}", to_portable_string(dir))
}

/// Criterion 26: an absolute pattern's literal directories are matched
/// case-sensitively, as a relative pattern's are.
#[test]
fn an_absolute_pattern_rejects_a_directory_spelled_in_another_case() {
    let fx = Fixture::new();
    let file = fx.file("repo/docs/a.md");
    let ctx = fx.ctx(&fx.repo);

    assert_rejects(&absolute(&fx.repo, "DOCS/*.md"), &file, &ctx);
    assert_rejects(&absolute(&fx.root, "REPO/docs/*.md"), &file, &ctx);
    assert!(!glob(&[&absolute(&fx.repo, "DOCS/*.md")]).matches_without_context(&file));

    assert_admits(&absolute(&fx.repo, "docs/*.md"), &file, &ctx);
    assert!(glob(&[&absolute(&fx.repo, "docs/*.md")]).matches_without_context(&file));
}

/// Criterion 26: a pattern made absolute by a `{{VAR}}` value keeps its
/// authored directory names case-sensitive.
#[test]
fn an_interpolated_absolute_pattern_rejects_a_directory_spelled_in_another_case() {
    let fx = Fixture::new();
    let file = fx.file("repo/docs/a.md");
    let repo_text = fx.repo.to_str().unwrap();
    let ctx = fx.ctx_with_env(&fx.repo, &[("ROOT", repo_text)]);

    assert_rejects("{{ROOT}}/DOCS/*.md", &file, &ctx);
    assert_admits("{{ROOT}}/docs/*.md", &file, &ctx);
}

/// Criterion 26: every relative and context-rooted prefix form rejects a
/// directory spelled in another case and admits the correct spelling.
#[test]
fn every_prefix_form_rejects_a_directory_spelled_in_another_case() {
    let fx = Fixture::new();
    let in_repo = fx.file("repo/docs/a.md");
    let in_home = fx.file("home/notes/h.md");
    let in_vault = fx.file("vault/docs/v.md");
    let ctx = fx.ctx(&fx.repo).add_vault(fx.root.join("vault"));

    for (form, dir, file) in [
        ("", "docs", &in_repo),
        ("./", "docs", &in_repo),
        ("&", "docs", &in_repo),
        ("^", "docs", &in_repo),
        ("@", "notes", &in_home),
        ("~/", "notes", &in_home),
        ("vault:", "docs", &in_vault),
    ] {
        let upper = dir.to_uppercase();
        assert_rejects(&format!("{form}{upper}/*.md"), file, &ctx);
        assert_admits(&format!("{form}{dir}/*.md"), file, &ctx);
    }
}

/// An absolute pattern through a directory symlink keeps working: the
/// authored link name is an exact directory entry even though the canonical
/// directory is named differently. A mismatched case below or at the link is
/// still rejected.
#[test]
fn an_absolute_pattern_through_a_directory_symlink_is_judged_by_its_authored_names() {
    let fx = Fixture::new();
    fx.file("repo/docs/a.md");
    link_dir(&fx.repo, &fx.root.join("alias"));
    let through_link = fx.root.join("alias/docs/a.md");
    let ctx = fx.ctx(&fx.repo);

    assert_admits(&absolute(&fx.root, "alias/docs/*.md"), &through_link, &ctx);
    assert_rejects(&absolute(&fx.root, "alias/DOCS/*.md"), &through_link, &ctx);
    assert_rejects(&absolute(&fx.root, "ALIAS/docs/*.md"), &through_link, &ctx);
}

/// An absolute pattern spelled with a temporary directory's own, possibly
/// symlinked, spelling (macOS `/var` is `/private/var`) still lists the file
/// under that spelling; a mismatched case below it is rejected.
#[test]
fn an_absolute_pattern_under_an_uncanonical_temporary_directory_keeps_its_spelling() {
    let tmp = TempDir::new().unwrap();
    let dir: PathBuf = tmp.path().to_path_buf();
    let file = dir.join("docs/a.md");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "a").unwrap();
    let ctx = snapshot(&dir, None, &[]);

    assert_admits(&absolute(&dir, "docs/*.md"), &file, &ctx);
    assert_rejects(&absolute(&dir, "DOCS/*.md"), &file, &ctx);
}

/// The Unicode alias pairs: (stored, authored).
const UNICODE_ALIASES: [(&str, &str); 2] = [("Σ", "ς"), ("SS", "ß")];

/// Whether `authored` opens the directory stored as `stored` in `dir`, a
/// property of the fixture's filesystem rather than of the OS. When it does,
/// the alias must reach that very directory, which is what makes the caller's
/// mismatch checks a regression test; the outcome is printed so a run's
/// output shows whether alias coverage ran. A caller with no alias-specific
/// check calls it for that record and those fixture assertions alone.
fn alias_opens(dir: &Path, stored: &str, authored: &str) -> bool {
    let names: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert!(!names.iter().any(|name| name == authored), "no entry is spelled `{authored}`");
    if dir.join(authored).is_dir() {
        let reached = std::fs::canonicalize(dir.join(authored)).unwrap();
        assert_eq!(reached, std::fs::canonicalize(dir.join(stored)).unwrap(), "`{authored}` reaches `{stored}`");
        eprintln!("alias coverage ran: `{authored}` opens `{stored}` on this filesystem");
        true
    } else {
        eprintln!("alias coverage not applicable: `{authored}` does not open `{stored}` on this filesystem");
        false
    }
}

/// Criterion 26: an absolute pattern spelling a directory with a Unicode
/// alias (`ς` for `Σ`, `ß` for `SS`) admits nothing in any view, exposes no
/// root where the filesystem folds it, and judges detached membership false;
/// the stored spelling is admitted everywhere.
#[test]
fn an_absolute_pattern_rejects_a_unicode_case_alias_in_every_view() {
    for (stored, authored) in UNICODE_ALIASES {
        let fx = Fixture::new();
        let file = fx.file(&format!("repo/{stored}/a.md"));
        let aliased = alias_opens(&fx.repo, stored, authored);
        let ctx = fx.ctx(&fx.repo);
        let wrong = absolute(&fx.repo, &format!("{authored}/*.md"));
        let right = absolute(&fx.repo, &format!("{stored}/*.md"));

        assert_rejects(&wrong, &file, &ctx);
        // Where the alias does not open, its directory is merely absent, and
        // an absent root is still a root.
        if aliased {
            assert!(glob(&[&wrong]).roots(&ctx).is_empty(), "`{wrong}` exposes no root");
        }
        assert!(!glob(&[&wrong]).matches_without_context(&file), "`{wrong}` detached");

        assert_admits(&right, &file, &ctx);
        assert_eq!(glob(&[&right]).roots(&ctx), [fx.repo.join(stored)], "`{right}` root");
        assert!(glob(&[&right]).matches_without_context(&file), "`{right}` detached");
    }
}

/// Criterion 26: a `{{VAR}}` value is supplied, but the directory names
/// authored after it must be spelled as stored, Unicode aliases included.
#[test]
fn an_interpolated_absolute_pattern_rejects_a_unicode_case_alias() {
    for (stored, authored) in UNICODE_ALIASES {
        let fx = Fixture::new();
        let file = fx.file(&format!("repo/{stored}/a.md"));
        alias_opens(&fx.repo, stored, authored);
        let repo_text = fx.repo.to_str().unwrap();
        let ctx = fx.ctx_with_env(&fx.repo, &[("ROOT", repo_text)]);

        assert_rejects(&format!("{{{{ROOT}}}}/{authored}/*.md"), &file, &ctx);
        assert_admits(&format!("{{{{ROOT}}}}/{stored}/*.md"), &file, &ctx);
    }
}

/// Criterion 26: an absolute exclusion spelled with an alias excludes
/// nothing; spelled as stored, it excludes the file.
#[test]
fn an_absolute_exclusion_spelled_with_an_alias_excludes_nothing() {
    for (stored, authored) in UNICODE_ALIASES.into_iter().chain([("docs", "DOCS")]) {
        let fx = Fixture::new();
        let file = fx.file(&format!("repo/{stored}/a.md"));
        alias_opens(&fx.repo, stored, authored);
        let ctx = fx.ctx(&fx.repo);
        let wrong = format!("!{}", absolute(&fx.repo, &format!("{authored}/*.md")));
        let right = format!("!{}", absolute(&fx.repo, &format!("{stored}/*.md")));

        let kept = glob(&["**/*.md", &wrong]);
        assert_eq!(kept.list_files(&ctx).unwrap().matches, std::slice::from_ref(&file), "`{wrong}`");
        assert!(kept.matches(&file, &ctx), "`{wrong}` keeps {}", file.display());
        assert!(kept.lists_file(&file, &ctx), "`{wrong}` keeps {}", file.display());

        let excluded = glob(&["**/*.md", &right]);
        assert!(excluded.list_files(&ctx).unwrap().matches.is_empty(), "`{right}`");
        assert!(!excluded.matches(&file, &ctx), "`{right}` excludes {}", file.display());
    }
}

/// Criterion 26: every relative and context-rooted prefix form keeps a
/// Unicode alias in its matcher and rejects it.
#[test]
fn every_prefix_form_rejects_a_unicode_case_alias() {
    for (stored, authored) in UNICODE_ALIASES {
        let fx = Fixture::new();
        let file = fx.file(&format!("repo/{stored}/a.md"));
        alias_opens(&fx.repo, stored, authored);
        let ctx = fx.ctx(&fx.repo);
        for form in ["", "./", "&", "^"] {
            assert_rejects(&format!("{form}{authored}/*.md"), &file, &ctx);
            assert_admits(&format!("{form}{stored}/*.md"), &file, &ctx);
        }
    }
}
