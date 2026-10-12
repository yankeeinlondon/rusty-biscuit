//! Checkout ownership survives aliases before source/cache work.
use std::path::Path;
use crate::{Repo, MAIN_BRANCH, commit_index, files_and_names, json_of};

macro_rules! command_cases {
    ($name:ident => $check:expr) => {
        mod $name {
            use super::*;
            #[test] fn functions() { ($check)("functions"); }
            #[test] fn types() { ($check)("types"); }
            #[test] fn symbols() { ($check)("symbols"); }
            #[test] fn classes() { ($check)("classes"); }
        }
    };
}


/// Symlinks `alias` to `target`. On Windows a target spelled with `/` does
/// not resolve, so callers join relative targets name by name.
fn link(target: &Path, alias: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, alias).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(target, alias).expect("file symlink requires Developer Mode or symlink privilege");
}

fn assert_rejected(repo: &Repo, changed: &str, selector: &str, command: &str) {
    {
        for (mode, extra) in [
            ("terminal", vec![]), ("plain", vec!["--plain"]),
            ("table", vec!["--format", "table"]), ("json", vec!["--json"]),
            ("json-lines", vec!["--jsonl"]), ("full-json", vec!["--json-full"]),
            ("count", vec!["--count"]), ("stats", vec!["--stats"]),
            ("stats-json", vec!["--stats", "--json"]),
        ].into_iter().chain((command != "classes").then_some(("tree", vec!["--tree"]))) {
            let mut args = vec!["--verbose", command, changed, "--file", selector];
            args.extend(extra);
            let output = repo.hug(&args);
            assert!(crate::cache_stats(&output).starts_with("cache: 0 hits, 0 partial hits, 0 misses; parsed 0 files"), "{args:?}: no source/cache work");
            let stdout = crate::stdout_of(output, &args);
            match mode {
                "count" => assert_eq!(stdout, "0\n"),
                "json-lines" => assert_eq!(stdout, ""),
                "json" | "full-json" => {
                    let report = json_of(&stdout);
                    assert_eq!(report[if mode == "json" { "files" } else { "indexes" }], serde_json::json!([]));
                    assert_eq!(report["errors"], serde_json::json!([]));
                }
                "stats-json" => {
                    let report = json_of(&stdout);
                    assert_eq!(report["stats"]["doc_coverage"], serde_json::json!({"documented":0,"total":0,"unavailable":0,"ratio":null}));
                    for field in ["by_kind", "by_visibility", "by_language"] { assert_eq!(report["stats"][field], serde_json::json!({})); }
                    assert_eq!(report["errors"], serde_json::json!([]));
                }
                "stats" => assert!(stdout.contains("0 of 0 documented (n/a)"), "{stdout}"),
                _ if command == "classes" && matches!(mode, "terminal" | "plain") => assert_eq!(stdout, ""),
                _ => assert!(stdout.contains("(no symbols)"), "{args:?}: {stdout:?}"),
            }
        }
    }
    let report = repo.json(&["symbols", changed, "--file", selector, "--json", "--no-cache"]);
    assert!(files_and_names(&report).is_empty());
    assert_eq!(report["errors"], serde_json::json!([]));
}

fn foreign_aliases_are_rejected(boundary: &str, phase: &str, command: &str) {
    let repo = Repo::committed(&[("clean.rs", "fn clean() {}\nstruct Clean;\n")]);
    let foreign = Repo::committed(&[("leaf.rs", "fn foreign() {}\nstruct Foreign;\n")]);
    let target = if boundary == "external" { foreign.dir.path().join("leaf.rs") } else {
        let inner = repo.dir.path().join("inner");
        if boundary == "nested-directory" {
            std::fs::create_dir(&inner).unwrap();
            crate::git_init(&inner);
            crate::commit_files(&inner, "nested", MAIN_BRANCH, &[("leaf.rs", "fn foreign() {}\nstruct Foreign;\n")]);
        } else { crate::add_worktree(foreign.dir.path(), "nested", &inner); }
        inner.join("leaf.rs")
    };
    git2::Repository::open(repo.dir.path()).unwrap().config().unwrap().set_bool("core.symlinks", true).unwrap();
    link(&target, &repo.dir.path().join("alias.rs"));
    let base = git2::Repository::open(repo.dir.path()).unwrap().head().unwrap().target().unwrap();
    if phase != "untracked" { repo.stage("alias.rs"); }
    if phase == "committed" { commit_index(repo.dir.path(), "alias", MAIN_BRANCH); }
    let changed = if phase == "committed" { format!("--changed={base}") } else { "--changed".into() };
    for selector in [repo.dir.path().join("alias.rs"), target.clone()] { assert_rejected(&repo, &changed, selector.to_str().unwrap(), command); }
}

command_cases!(external_untracked_aliases_are_rejected_in_every_selector_and_report => check_external_untracked_aliases_are_rejected_in_every_selector_and_report);

fn check_external_untracked_aliases_are_rejected_in_every_selector_and_report(command: &str) {
    foreign_aliases_are_rejected("external", "untracked", command);
}

command_cases!(external_staged_aliases_are_rejected_in_every_selector_and_report => check_external_staged_aliases_are_rejected_in_every_selector_and_report);

fn check_external_staged_aliases_are_rejected_in_every_selector_and_report(command: &str) {
    foreign_aliases_are_rejected("external", "staged", command);
}

command_cases!(external_committed_aliases_are_rejected_in_every_selector_and_report => check_external_committed_aliases_are_rejected_in_every_selector_and_report);

fn check_external_committed_aliases_are_rejected_in_every_selector_and_report(command: &str) {
    foreign_aliases_are_rejected("external", "committed", command);
}

command_cases!(nested_directory_untracked_aliases_are_rejected_in_every_selector_and_report => check_nested_directory_untracked_aliases_are_rejected_in_every_selector_and_report);

fn check_nested_directory_untracked_aliases_are_rejected_in_every_selector_and_report(command: &str) {
    foreign_aliases_are_rejected("nested-directory", "untracked", command);
}

command_cases!(nested_directory_staged_aliases_are_rejected_in_every_selector_and_report => check_nested_directory_staged_aliases_are_rejected_in_every_selector_and_report);

fn check_nested_directory_staged_aliases_are_rejected_in_every_selector_and_report(command: &str) {
    foreign_aliases_are_rejected("nested-directory", "staged", command);
}

command_cases!(nested_directory_committed_aliases_are_rejected_in_every_selector_and_report => check_nested_directory_committed_aliases_are_rejected_in_every_selector_and_report);

fn check_nested_directory_committed_aliases_are_rejected_in_every_selector_and_report(command: &str) {
    foreign_aliases_are_rejected("nested-directory", "committed", command);
}

command_cases!(nested_file_untracked_aliases_are_rejected_in_every_selector_and_report => check_nested_file_untracked_aliases_are_rejected_in_every_selector_and_report);

fn check_nested_file_untracked_aliases_are_rejected_in_every_selector_and_report(command: &str) {
    foreign_aliases_are_rejected("nested-file", "untracked", command);
}

command_cases!(nested_file_staged_aliases_are_rejected_in_every_selector_and_report => check_nested_file_staged_aliases_are_rejected_in_every_selector_and_report);

fn check_nested_file_staged_aliases_are_rejected_in_every_selector_and_report(command: &str) {
    foreign_aliases_are_rejected("nested-file", "staged", command);
}

command_cases!(nested_file_committed_aliases_are_rejected_in_every_selector_and_report => check_nested_file_committed_aliases_are_rejected_in_every_selector_and_report);

fn check_nested_file_committed_aliases_are_rejected_in_every_selector_and_report(command: &str) {
    foreign_aliases_are_rejected("nested-file", "committed", command);
}

command_cases!(same_checkout_aliases_preserve_expected_declarations => check_same_checkout_aliases_preserve_expected_declarations);

fn check_same_checkout_aliases_preserve_expected_declarations(command: &str) {
    let repo = Repo::committed(&[("clean.rs", "fn clean() {}\nstruct Clean;\n")]);
    link(Path::new("clean.rs"), &repo.dir.path().join("alias.rs"));
    for selector in ["alias.rs", "clean.rs"] {
        let names = match command { "functions" => vec!["clean"], "symbols" => vec!["clean", "Clean"], _ => vec!["Clean"] };
            {
            for cached in [true, false] {
                let extra = if cached { vec![] } else { vec!["--no-cache"] };
                let mut args = vec![command, "--changed", "--file", selector, "--json"];
                args.extend(extra);
                let report = repo.json(&args);
                let files = files_and_names(&report);
                assert_eq!(files.len(), 1);
                assert_eq!(files[0].1, names);
                assert_eq!(report["errors"], serde_json::json!([]));
            }
            let warm = repo.hug(&["--verbose", command, "--changed", "--file", selector, "--json"]);
            assert!(crate::cache_stats(&warm).starts_with("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"));
            let report = json_of(&crate::stdout_of(warm, &[command]));
            assert_eq!(files_and_names(&report)[0].1, names);
            assert_eq!(repo.stdout(&[command, "--changed", "--file", selector, "--count"]), format!("{}\n", names.len()));
        }
    }
}

command_cases!(nested_candidate_alias_cannot_return_changed_identity_to_outer_checkout => check_nested_candidate_alias_cannot_return_changed_identity_to_outer_checkout);

fn check_nested_candidate_alias_cannot_return_changed_identity_to_outer_checkout(command: &str) {
    let repo = Repo::committed(&[("clean.rs", "fn clean() {}\nstruct Clean;\n")]);
    let inner = repo.dir.path().join("inner");
    std::fs::create_dir(&inner).unwrap();
    link(&Path::new("..").join("clean.rs"), &inner.join("alias.rs"));
    repo.stage("inner/alias.rs");
    commit_index(repo.dir.path(), "tracked alias", MAIN_BRANCH);
    crate::git_init(&inner);
    std::fs::remove_file(inner.join("alias.rs")).unwrap();
    link(&repo.dir.path().join("clean.rs"), &inner.join("alias.rs"));
    let changed = sniff::filesystem::git::GitRepo::discover(repo.dir.path()).unwrap().unwrap()
        .changed_paths(sniff::filesystem::git::ChangedBase::Omitted).unwrap();
    assert!(changed.contains(&Path::new("inner").join("alias.rs")), "tracked alias must supply changed evidence");
    assert_rejected(&repo, "--changed", "clean.rs", command);
    std::fs::create_dir(inner.join("subdir")).unwrap();
    link(&Path::new("..").join("..").join("clean.rs"), &inner.join("subdir").join("back.rs"));
    #[cfg(unix)]
    std::os::unix::fs::symlink(Path::new("inner/subdir"), repo.dir.path().join("bridge")).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(Path::new("inner").join("subdir"), repo.dir.path().join("bridge")).unwrap();
    link(&Path::new("bridge").join("back.rs"), &repo.dir.path().join("alias.rs"));
    assert_rejected(&repo, "--changed", "alias.rs", command);
    assert_rejected(&repo, "--changed", "clean.rs", command);
}

command_cases!(external_directory_alias_cannot_return_changed_identity_to_active_checkout => check_external_directory_alias_cannot_return_changed_identity_to_active_checkout);

fn check_external_directory_alias_cannot_return_changed_identity_to_active_checkout(command: &str) {
    let repo = Repo::committed(&[("clean.rs", "fn clean() {}\nstruct Clean;\n")]);
    let foreign = Repo::committed(&[("foreign.rs", "fn foreign() {}\n")]);
    let subdir = foreign.dir.path().join("subdir");
    std::fs::create_dir(&subdir).unwrap();
    link(&repo.dir.path().join("clean.rs"), &subdir.join("back.rs"));
    #[cfg(unix)]
    std::os::unix::fs::symlink(&subdir, repo.dir.path().join("bridge")).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&subdir, repo.dir.path().join("bridge")).unwrap();
    link(&Path::new("bridge").join("back.rs"), &repo.dir.path().join("alias.rs"));
    // The final identity is local, but changed evidence crosses the foreign
    // directory before returning; neither selector may inherit eligibility.
    assert_eq!(tree_hugger::file_identity(&repo.dir.path().join("alias.rs")), tree_hugger::file_identity(&repo.dir.path().join("clean.rs")));
    let changed = sniff::filesystem::git::GitRepo::discover(repo.dir.path()).unwrap().unwrap()
        .changed_paths(sniff::filesystem::git::ChangedBase::Omitted).unwrap();
    assert!(changed.contains(&Path::new("alias.rs").to_path_buf()));
    assert_rejected(&repo, "--changed", "alias.rs", command);
    assert_rejected(&repo, "--changed", "clean.rs", command);
}

fn parent_traversal(shape: &str, phase: &str, command: &str) {
    let repo = Repo::committed(&[("clean.rs", "fn clean() {}\nstruct Clean;\n"), (".gitignore", "inner/\nbridge\n")]);
    let foreign = Repo::committed(&[("foreign.rs", "fn foreign() {}\n")]);
    let inner = repo.dir.path().join("inner");
    if shape == "nested-file" {
        crate::add_worktree(foreign.dir.path(), "nested", &inner);
    } else {
        std::fs::create_dir(&inner).unwrap();
        if shape != "ordinary" && shape != "ordinary-directory-alias" {
            crate::git_init(&inner);
        }
    }
    let target = match shape {
        "external" => {
            std::fs::create_dir(foreign.dir.path().join("subdir")).unwrap();
            foreign.dir.path().join("subdir").join("..").join("..").join(repo.dir.path().file_name().unwrap()).join("clean.rs")
        }
        "directory-alias" | "ordinary-directory-alias" => {
            std::fs::create_dir(inner.join("subdir")).unwrap();
            #[cfg(unix)]
            std::os::unix::fs::symlink(Path::new("inner/subdir"), repo.dir.path().join("bridge")).unwrap();
            #[cfg(windows)]
            std::os::windows::fs::symlink_dir(Path::new("inner/subdir"), repo.dir.path().join("bridge")).unwrap();
            Path::new("bridge").join("..").join("..").join("clean.rs")
        }
        "recursive-file-alias" => {
            link(Path::new("inner/../clean.rs"), &repo.dir.path().join("middle.rs"));
            repo.stage("middle.rs");
            commit_index(repo.dir.path(), "middle alias", MAIN_BRANCH);
            Path::new("middle.rs").to_path_buf()
        }
        "dot-parent" => Path::new("inner/./../clean.rs").to_path_buf(),
        _ => Path::new("inner/../clean.rs").to_path_buf(),
    };
    git2::Repository::open(repo.dir.path()).unwrap().config().unwrap().set_bool("core.symlinks", true).unwrap();
    link(&target, &repo.dir.path().join("alias.rs"));
    let base = git2::Repository::open(repo.dir.path()).unwrap().head().unwrap().target().unwrap();
    if phase != "untracked" { repo.stage("alias.rs"); }
    if phase == "committed" { commit_index(repo.dir.path(), "alias", MAIN_BRANCH); }
    let request = if phase == "committed" { sniff::filesystem::git::ChangedBase::Reference(base.to_string()) } else { sniff::filesystem::git::ChangedBase::Omitted };
    let paths = sniff::filesystem::git::GitRepo::discover(repo.dir.path()).unwrap().unwrap().changed_paths(request).unwrap();
    assert_eq!(paths, vec![Path::new("alias.rs").to_path_buf()], "alias alone supplies eligibility");
    assert_eq!(tree_hugger::file_identity(&repo.dir.path().join("alias.rs")), tree_hugger::file_identity(&repo.dir.path().join("clean.rs")), "fixture returns to the unchanged outer identity");
    let changed = if phase == "committed" { format!("--changed={base}") } else { "--changed".into() };
    for selector in ["alias.rs", "clean.rs"] {
        if shape.starts_with("ordinary") {
            let names = match command { "functions" => vec!["clean"], "symbols" => vec!["clean", "Clean"], _ => vec!["Clean"] };
            {
                let report = repo.json(&[command, &changed, "--file", selector, "--json", "--no-cache"]);
                let files = files_and_names(&report);
                assert_eq!(files.len(), 1);
                assert_eq!(files[0].1, names);
                assert_eq!(report["errors"], serde_json::json!([]));
                assert_eq!(repo.stdout(&[command, &changed, "--file", selector, "--count"]), format!("{}\n", names.len()));
                let cached = repo.json(&[command, &changed, "--file", selector, "--json"]);
                assert_eq!(files_and_names(&cached)[0].1, names);
                let warm = repo.hug(&["--verbose", command, &changed, "--file", selector, "--json"]);
                assert!(crate::cache_stats(&warm).starts_with("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"));
                assert_eq!(files_and_names(&json_of(&crate::stdout_of(warm, &[command])))[0].1, names);
            }
        } else {
            assert_rejected(&repo, &changed, selector, command);
        }
    }
}

macro_rules! parent_cases {
    ($shape:literal, $untracked:ident, $staged:ident, $committed:ident) => {
        command_cases!($untracked => |command| parent_traversal($shape, "untracked", command));
        command_cases!($staged => |command| parent_traversal($shape, "staged", command));
        command_cases!($committed => |command| parent_traversal($shape, "committed", command));
    };
}

parent_cases!("nested-directory", nested_directory_parent_untracked, nested_directory_parent_staged, nested_directory_parent_committed);
parent_cases!("nested-file", nested_file_parent_untracked, nested_file_parent_staged, nested_file_parent_committed);
parent_cases!("external", external_parent_untracked, external_parent_staged, external_parent_committed);
parent_cases!("directory-alias", directory_alias_parent_untracked, directory_alias_parent_staged, directory_alias_parent_committed);
parent_cases!("recursive-file-alias", recursive_file_alias_parent_untracked, recursive_file_alias_parent_staged, recursive_file_alias_parent_committed);
parent_cases!("dot-parent", dot_parent_untracked, dot_parent_staged, dot_parent_committed);
parent_cases!("ordinary", ordinary_parent_untracked, ordinary_parent_staged, ordinary_parent_committed);
parent_cases!("ordinary-directory-alias", ordinary_directory_alias_parent_untracked, ordinary_directory_alias_parent_staged, ordinary_directory_alias_parent_committed);
