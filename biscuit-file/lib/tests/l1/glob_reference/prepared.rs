//! `PreparedGlobReference`: a caller that walks the roots itself may skip
//! every directory `may_list_below` rules out and still list exactly what
//! `list_files` lists.

use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, GlobReference, PathIdentity, PreparedGlobReference};

use super::{Fixture, glob, snapshot};

fn prepared(patterns: &[&str], ctx: &FileResolutionContext) -> PreparedGlobReference {
    glob(patterns).prepare(ctx).expect("a valid context")
}

fn below(prepared: &PreparedGlobReference, dir: &Path) -> bool {
    prepared.may_list_below(&PathIdentity::new(dir))
}

/// A walk of `prepared`'s roots that skips each directory `may_list_below`
/// rules out, keeping the files `lists_file` accepts, as a set.
fn pruned_walk(prepared: &PreparedGlobReference) -> Vec<PathBuf> {
    fn visit(prepared: &PreparedGlobReference, dir: &Path, out: &mut Vec<PathBuf>) {
        if !below(prepared, dir) {
            return;
        }
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            let kind = entry.file_type().unwrap();
            if kind.is_dir() {
                visit(prepared, &path, out);
            } else if kind.is_file() && prepared.lists_file(&path) && !out.contains(&path) {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    for root in prepared.roots() {
        visit(prepared, &root, &mut out);
    }
    out.sort();
    out
}

/// The roots of an unrelated tree beside `DOCS/` are skipped: a bare
/// `DOCS/*.md` launched from a folder holding far more than `DOCS` need not
/// walk the rest of it.
#[test]
fn a_literal_directory_prefix_rules_out_its_siblings() {
    let fx = Fixture::new();
    let launch = fx.root.join("outside");
    let ctx = snapshot(&launch, Some(&fx.home), &[]);
    let docs = prepared(&["DOCS/*.md"], &ctx);

    assert!(below(&docs, &launch), "the root leads to DOCS/");
    assert!(below(&docs, &launch.join("DOCS")));
    assert!(!below(&docs, &launch.join("bulk")));
    assert!(!below(&docs, &launch.join("bulk/DOCS")), "DOCS/ is literal from the root");
    assert!(!below(&docs, &launch.join("DOCS/deeper")), "`*` does not cross `/`");

    let anywhere = prepared(&["**/*.md"], &ctx);
    assert!(below(&anywhere, &launch.join("bulk/a/b/c")));

    let names = GlobReference::new(["*.md"]).unwrap().with_file_name_view();
    let names = names.prepare(&ctx).unwrap();
    assert!(below(&names, &launch.join("bulk/a/b/c")), "a name pattern matches at any depth");

    let excluded = prepared(&["DOCS/*.md", "!bulk/**"], &ctx);
    assert!(!below(&excluded, &launch.join("bulk")), "an exclusion admits nothing");
}

/// A directory above a nested root still leads to that root's matches, so
/// `^docs/*.md` from the package keeps the package area open while ruling
/// out the repository's unrelated folders.
#[test]
fn a_directory_leading_to_a_nested_root_stays_open() {
    let fx = Fixture::new();
    let ctx = fx.ctx(&fx.pkg);
    let docs = prepared(&["^docs/*.md"], &ctx);

    assert!(below(&docs, &fx.repo));
    assert!(below(&docs, &fx.area));
    assert!(below(&docs, &fx.repo.join("docs")));
    assert!(!below(&docs, &fx.repo.join("other")));
    assert!(!below(&docs, &fx.area.join("other")));
}

/// Pruning never changes what is listed: for each pattern set, the pruned
/// walk finds exactly the files `list_files` lists.
#[test]
fn a_pruned_walk_lists_what_list_files_lists() {
    let fx = Fixture::new();
    for file in [
        "repo/docs/a.md",
        "repo/docs/deep/b.md",
        "repo/area/docs/c.md",
        "repo/area/pkg/docs/d.md",
        "repo/area/pkg/notes/e.md",
        "repo/other/docs/f.md",
        "repo/other/g.txt",
        "repo/area/pkg/docs/_h.md",
    ] {
        fx.file(file);
    }
    let ctx = fx.ctx(&fx.pkg);

    for patterns in [
        &["docs/*.md"][..],
        &["^docs/*.md"],
        &["&docs/**/*.md"],
        &["^**/*.md", "!&other/**"],
        &["notes/*.md", "&docs/*.md"],
        &["../../other/*/*.md"],
    ] {
        let mut expected = glob(patterns).list_files(&ctx).unwrap().matches;
        expected.sort();
        assert!(!expected.is_empty(), "{patterns:?} lists something");
        assert_eq!(pruned_walk(&prepared(patterns, &ctx)), expected, "{patterns:?}");
    }
}
