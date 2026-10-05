//! Criterion 13 (biscuit-file part): a glob reference applies no filters.

use super::{Fixture, listed};

#[test]
fn hidden_ignored_and_underscore_files_are_listed() {
    let fx = Fixture::new();
    fx.file("repo/.gitignore");
    std::fs::write(fx.repo.join(".gitignore"), "ignored/\n").unwrap();
    let files = [
        fx.file("repo/.x.md"),
        fx.file("repo/_x.md"),
        fx.file("repo/.hidden/h.md"),
        fx.file("repo/_completed/c.md"),
        fx.file("repo/ignored/i.md"),
    ];
    let ctx = fx.ctx(&fx.repo);

    let listed = listed(&["&**/*.md"], &ctx);
    for file in &files {
        assert!(listed.contains(file), "{} missing from {listed:?}", file.display());
    }
    assert_eq!(listed.len(), files.len());
}
