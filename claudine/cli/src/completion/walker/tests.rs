use super::*;
use crate::completion::scopes::ScopeKind;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn init_git(root: &Path) {
    // WalkBuilder uses the presence of a `.git` directory to root its
    // `.gitignore` discovery. An empty dir is enough.
    fs::create_dir_all(root.join(".git")).unwrap();
}

fn scope(path: PathBuf, follow_links: bool) -> Scope {
    Scope {
        kind: ScopeKind::RepoPrompts,
        path,
        follow_links,
    }
}

#[test]
fn nonexistent_scope_returns_empty() {
    let tmp = TempDir::new().unwrap();
    let missing = tmp.path().join("does-not-exist");
    let got = walk_scope(&scope(missing, true));
    assert!(got.is_empty());
}

#[test]
fn walks_flat_directory() {
    let tmp = TempDir::new().unwrap();
    init_git(tmp.path());
    let root = tmp.path().join("prompts");
    write_file(&root.join("a.md"), "# a");
    write_file(&root.join("b.md"), "# b");

    let got = walk_scope(&scope(root.clone(), true));
    let names: Vec<String> = got
        .iter()
        .filter_map(|p| p.file_name().and_then(|s| s.to_str()).map(String::from))
        .collect();
    assert!(
        names.contains(&"a.md".to_string()),
        "missing a.md: {names:?}"
    );
    assert!(
        names.contains(&"b.md".to_string()),
        "missing b.md: {names:?}"
    );
}

#[test]
fn underscore_prefixed_files_are_elided() {
    let tmp = TempDir::new().unwrap();
    init_git(tmp.path());
    let root = tmp.path().join("prompts");
    write_file(&root.join("_draft.md"), "# d");
    write_file(&root.join("public.md"), "# p");

    let got = walk_scope(&scope(root, true));
    let names: Vec<String> = got
        .iter()
        .filter_map(|p| p.file_name().and_then(|s| s.to_str()).map(String::from))
        .collect();
    assert!(
        !names.contains(&"_draft.md".to_string()),
        "underscore file leaked: {names:?}"
    );
    assert!(names.contains(&"public.md".to_string()));
}

#[test]
fn underscore_prefixed_directories_are_elided_recursively() {
    let tmp = TempDir::new().unwrap();
    init_git(tmp.path());
    let root = tmp.path().join("prompts");
    write_file(&root.join("_wip").join("notes.md"), "# n");
    write_file(&root.join("ok").join("kept.md"), "# k");

    let got = walk_scope(&scope(root, true));
    let rendered: Vec<String> = got.iter().map(|p| p.display().to_string()).collect();
    assert!(
        !rendered.iter().any(|p| p.contains("_wip")),
        "underscore dir descendant leaked: {rendered:?}"
    );
    assert!(
        rendered.iter().any(|p| p.ends_with("kept.md")),
        "ok/kept.md missing: {rendered:?}"
    );
}

#[test]
fn skip_list_honored_at_every_depth() {
    let tmp = TempDir::new().unwrap();
    init_git(tmp.path());
    let root = tmp.path().join("prompts");
    write_file(&root.join("target").join("debris.md"), "# d");
    write_file(
        &root.join("inner").join("node_modules").join("pkg.md"),
        "# p",
    );
    write_file(&root.join("real.md"), "# r");

    let got = walk_scope(&scope(root, true));
    let rendered: Vec<String> = got.iter().map(|p| p.display().to_string()).collect();
    assert!(
        !rendered.iter().any(|p| p.contains("target")),
        "target/ leaked: {rendered:?}"
    );
    assert!(
        !rendered.iter().any(|p| p.contains("node_modules")),
        "node_modules/ leaked: {rendered:?}"
    );
    assert!(
        rendered.iter().any(|p| p.ends_with("real.md")),
        "real.md missing: {rendered:?}"
    );
}

#[test]
fn gitignore_honored_at_every_depth() {
    let tmp = TempDir::new().unwrap();
    init_git(tmp.path());
    let root = tmp.path();
    // `.gitignore` rule applied at repo root must be honored inside
    // nested scopes — the walker inherits the git repo's ignore
    // context because `.git` lives at `tmp.path()`.
    write_file(&root.join(".gitignore"), "prompts/ignored/\nskip.md\n");
    write_file(
        &root.join("prompts").join("ignored").join("secret.md"),
        "# s",
    );
    write_file(&root.join("prompts").join("kept.md"), "# k");
    write_file(&root.join("prompts").join("skip.md"), "# s");

    let got = walk_scope(&scope(root.join("prompts"), true));
    let rendered: Vec<String> = got.iter().map(|p| p.display().to_string()).collect();
    assert!(
        rendered.iter().any(|p| p.ends_with("kept.md")),
        "kept.md missing: {rendered:?}"
    );
    assert!(
        !rendered.iter().any(|p| p.ends_with("secret.md")),
        "gitignored secret.md leaked: {rendered:?}"
    );
    assert!(
        !rendered.iter().any(|p| p.ends_with("skip.md")),
        "gitignored skip.md leaked: {rendered:?}"
    );
}

#[test]
fn candidate_budget_honored() {
    let tmp = TempDir::new().unwrap();
    init_git(tmp.path());
    let root = tmp.path().join("prompts");
    for i in 0..20 {
        write_file(&root.join(format!("f{i}.md")), "# x");
    }
    let got = walk_scope_limited(&scope(root, true), 5);
    assert_eq!(got.len(), 5, "budget not honored: {got:?}");
}

#[test]
fn zero_budget_returns_empty() {
    let tmp = TempDir::new().unwrap();
    init_git(tmp.path());
    let root = tmp.path().join("prompts");
    write_file(&root.join("a.md"), "# a");
    let got = walk_scope_limited(&scope(root, true), 0);
    assert!(got.is_empty());
}

#[cfg(unix)]
#[test]
fn symlinks_are_not_followed_when_follow_links_is_false() {
    use std::os::unix::fs::symlink;

    let tmp = TempDir::new().unwrap();
    init_git(tmp.path());
    let real_dir = tmp.path().join("real");
    write_file(&real_dir.join("hidden-behind-link.md"), "# h");

    let scope_root = tmp.path().join("skills-scope");
    fs::create_dir_all(&scope_root).unwrap();
    write_file(&scope_root.join("direct.md"), "# d");
    symlink(&real_dir, scope_root.join("link-to-real")).unwrap();

    let got = walk_scope(&scope(scope_root, false));
    let rendered: Vec<String> = got.iter().map(|p| p.display().to_string()).collect();
    assert!(
        rendered.iter().any(|p| p.ends_with("direct.md")),
        "direct file missing: {rendered:?}"
    );
    assert!(
        !rendered
            .iter()
            .any(|p| p.ends_with("hidden-behind-link.md")),
        "symlink followed despite follow_links=false: {rendered:?}"
    );
}

#[cfg(unix)]
#[test]
fn symlinks_are_followed_when_follow_links_is_true() {
    use std::os::unix::fs::symlink;

    let tmp = TempDir::new().unwrap();
    init_git(tmp.path());
    let real_dir = tmp.path().join("real");
    write_file(&real_dir.join("behind-link.md"), "# h");

    let scope_root = tmp.path().join("generic-scope");
    fs::create_dir_all(&scope_root).unwrap();
    symlink(&real_dir, scope_root.join("link-to-real")).unwrap();

    let got = walk_scope(&scope(scope_root, true));
    let rendered: Vec<String> = got.iter().map(|p| p.display().to_string()).collect();
    assert!(
        rendered.iter().any(|p| p.ends_with("behind-link.md")),
        "symlink not followed despite follow_links=true: {rendered:?}"
    );
}

#[test]
fn git_directory_is_never_descended() {
    let tmp = TempDir::new().unwrap();
    init_git(tmp.path());
    let root = tmp.path();
    write_file(&root.join(".git").join("hooks").join("README.md"), "# h");
    write_file(&root.join("real.md"), "# r");

    let got = walk_scope(&scope(root.to_path_buf(), true));
    let rendered: Vec<String> = got.iter().map(|p| p.display().to_string()).collect();
    assert!(
        !rendered.iter().any(|p| p.contains(".git")),
        ".git/ leaked: {rendered:?}"
    );
}

#[test]
fn filtered_walk_counts_matches_not_raw_discoveries() {
    let tmp = TempDir::new().unwrap();
    init_git(tmp.path());
    let root = tmp.path().join("prompts");
    write_file(&root.join("plan.md"), "# p");
    write_file(&root.join("notes.md"), "# n");
    write_file(&root.join("other.md"), "# o");

    let got = walk_scope_filtered(&scope(root, true), MAX_CANDIDATES, |p| {
        p.to_str()
            .map(|s| s.to_ascii_lowercase().contains("plan"))
            .unwrap_or(false)
    });
    let paths = got.unwrap_complete();
    assert_eq!(paths.len(), 1, "only plan.md matches: {paths:?}");
    assert!(
        paths[0].to_str().unwrap().contains("plan.md"),
        "expected plan.md: {paths:?}"
    );
}

#[test]
fn filtered_walk_over_capacity_reports_more_than_cap() {
    let tmp = TempDir::new().unwrap();
    init_git(tmp.path());
    let root = tmp.path().join("prompts");
    for i in 0..20 {
        write_file(&root.join(format!("plan{i}.md")), "# x");
    }
    write_file(&root.join("other.md"), "# o");

    let got = walk_scope_filtered(&scope(root, true), 5, |p| {
        p.to_str()
            .map(|s| s.to_ascii_lowercase().contains("plan"))
            .unwrap_or(false)
    });
    assert!(
        matches!(got, WalkOutcome::OverCapacity(n) if n > 5),
        "expected over-capacity with at least 6 matches, got: {got:?}"
    );
}

#[test]
fn filtered_walk_zero_budget_returns_empty() {
    let tmp = TempDir::new().unwrap();
    init_git(tmp.path());
    let root = tmp.path().join("prompts");
    write_file(&root.join("plan.md"), "# p");

    let got = walk_scope_filtered(&scope(root, true), 0, |p| {
        p.to_str()
            .map(|s| s.to_ascii_lowercase().contains("plan"))
            .unwrap_or(false)
    });
    assert_eq!(got.unwrap_complete().len(), 0);
}

#[test]
fn filtered_walk_nonexistent_scope_returns_empty() {
    let tmp = TempDir::new().unwrap();
    let missing = tmp.path().join("does-not-exist");
    let got = walk_scope_filtered(&scope(missing, true), MAX_CANDIDATES, |_| true);
    assert!(got.unwrap_complete().is_empty());
}
