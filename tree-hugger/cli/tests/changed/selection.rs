//! The acceptance-table selection rows: which files `--changed` selects,
//! what content it reports, and how the changed set intersects the
//! ordinary file selection.

use crate::{
    Repo, checkout_branch, commit_files, commit_on_branch, files_and_names, git_init,
    stage_deletion,
};

#[test]
fn staged_unstaged_both_and_untracked_report_current_declarations_once() {
    let repo = Repo::committed(&[
        ("base.rs", "fn base() {}\n"),
        ("clean.rs", "fn clean() {}\n"),
    ]);
    // Staged: a new file in the index.
    repo.write("staged.rs", "fn staged() {}\n");
    repo.stage("staged.rs");
    // Unstaged: a tracked file edited on disk only.
    repo.write("base.rs", "fn base() {}\nfn unstaged_edit() {}\n");
    // Both: staged, then edited again on disk.
    repo.write("both.rs", "fn both_staged() {}\n");
    repo.stage("both.rs");
    repo.write("both.rs", "fn both_staged() {}\nfn both_worktree() {}\n");
    // Untracked: never added.
    repo.write("untracked.rs", "fn untracked() {}\n");

    let report = repo.json(&["--json", "symbols", "--changed"]);
    assert_eq!(
        files_and_names(&report),
        vec![
            ("base.rs".to_string(), vec!["base".to_string(), "unstaged_edit".to_string()]),
            ("both.rs".to_string(), vec!["both_staged".to_string(), "both_worktree".to_string()]),
            ("staged.rs".to_string(), vec!["staged".to_string()]),
            ("untracked.rs".to_string(), vec!["untracked".to_string()]),
        ],
        "every changed path is reported once with its current on-disk declarations; clean.rs is not"
    );
    assert_eq!(report["errors"].as_array().map(Vec::len), Some(0));
}

#[test]
fn a_staged_edit_undone_on_disk_selects_the_restored_content() {
    let repo = Repo::committed(&[("doc.rs", "fn original() {}\n")]);
    repo.write("doc.rs", "fn original() {}\nfn staged_edit() {}\n");
    repo.stage("doc.rs");
    // Undo the edit on disk: the path stays selected (it is staged), and
    // the restored content is what gets reported.
    repo.write("doc.rs", "fn original() {}\n");

    let report = repo.json(&["--json", "symbols", "--changed"]);
    assert_eq!(
        files_and_names(&report),
        vec![("doc.rs".to_string(), vec!["original".to_string()])],
    );
}

#[test]
fn a_committed_change_restored_on_disk_is_selected_with_a_base() {
    let dir = Repo::committed(&[("file.rs", "fn base_content() {}\n")]);
    commit_on_branch(
        dir.dir.path(),
        "feature",
        "feature side",
        &[("file.rs", "fn base_content() {}\nfn feature_side() {}\n")],
    );
    // The committed feature edit is undone locally.
    dir.write("file.rs", "fn base_content() {}\n");

    let report = dir.json(&["--json", "symbols", "--changed=main"]);
    assert_eq!(
        files_and_names(&report),
        vec![("file.rs".to_string(), vec!["base_content".to_string()])],
        "the tip difference selects the path; the restored on-disk content is reported"
    );
}

#[test]
fn ignored_untracked_files_are_excluded_even_when_explicitly_named() {
    let repo = Repo::committed(&[
        (".gitignore", "gen/\nignored.rs\n"),
        ("clean.rs", "fn clean() {}\n"),
    ]);
    repo.write("gen/x.rs", "fn generated() {}\n");
    repo.write("ignored.rs", "fn ignored_one() {}\n");

    let report = repo.json(&["--json", "symbols", "--changed"]);
    assert_eq!(report["files"].as_array().map(Vec::len), Some(0));
    assert_eq!(report["errors"].as_array().map(Vec::len), Some(0));

    // An explicit selector normally re-includes an ignored file…
    let ordinary = repo.json(&["--json", "symbols", "--file", "gen/x.rs"]);
    assert_eq!(
        files_and_names(&ordinary),
        vec![("gen/x.rs".to_string(), vec!["generated".to_string()])],
    );
    // …but Git never adds ignored untracked paths to the changed set, so
    // the same explicit selector under `--changed` selects nothing.
    for selector in ["gen/x.rs", "ignored.rs"] {
        let excluded = repo.json(&["--json", "symbols", "--changed", "--file", selector]);
        assert_eq!(excluded["files"].as_array().map(Vec::len), Some(0), "{selector}");
        assert_eq!(excluded["errors"].as_array().map(Vec::len), Some(0), "{selector}");
    }
}

#[test]
fn fixture_directories_stay_excluded_and_a_targeted_override_wins() {
    let repo = Repo::committed(&[("clean.rs", "fn clean() {}\n")]);
    repo.write("fixtures/data.rs", "fn fixture_data() {}\n");

    // The default fixture exclusion keeps the walk from selecting it.
    let report = repo.json(&["--json", "symbols", "--changed"]);
    assert_eq!(report["files"].as_array().map(Vec::len), Some(0));
    assert_eq!(report["errors"].as_array().map(Vec::len), Some(0));

    // The normal user overrides re-include the targeted fixture file, and
    // it is changed, so it is reported.
    let positional = repo.json(&["--json", "symbols", "--changed", "fixtures/data.rs"]);
    assert_eq!(
        files_and_names(&positional),
        vec![("fixtures/data.rs".to_string(), vec!["fixture_data".to_string()])],
    );
    let explicit = repo.json(&["--json", "symbols", "--changed", "--file", "fixtures/data.rs"]);
    assert_eq!(files_and_names(&explicit), files_and_names(&positional));
}

#[test]
fn renames_and_deletions_keep_present_destinations_and_drop_absent_paths() {
    let repo = Repo::committed(&[
        ("old.rs", "fn old_fn() {}\n"),
        ("gone.rs", "fn gone_fn() {}\n"),
        ("sibling.rs", "fn sibling() {}\n"),
    ]);
    // A staged rename is a delete+add pair: the destination survives, the
    // absent source is skipped.
    std::fs::remove_file(repo.dir.path().join("old.rs")).unwrap();
    stage_deletion(repo.dir.path(), "old.rs");
    repo.write("new.rs", "fn new_fn() {}\n");
    repo.stage("new.rs");
    // A staged deletion with no file on disk selects nothing for that path.
    std::fs::remove_file(repo.dir.path().join("gone.rs")).unwrap();
    stage_deletion(repo.dir.path(), "gone.rs");
    // An ordinary unstaged sibling edit keeps being reported.
    repo.write("sibling.rs", "fn sibling() {}\nfn sibling_edit() {}\n");

    let report = repo.json(&["--json", "symbols", "--changed"]);
    assert_eq!(
        files_and_names(&report),
        vec![
            ("new.rs".to_string(), vec!["new_fn".to_string()]),
            ("sibling.rs".to_string(), vec!["sibling".to_string(), "sibling_edit".to_string()]),
        ],
        "old.rs and gone.rs are absent on disk and dropped without an error"
    );
    assert_eq!(report["errors"].as_array().map(Vec::len), Some(0));
}

#[test]
fn a_staged_deletion_recreated_on_disk_reports_the_current_content() {
    let repo = Repo::committed(&[("rec.rs", "fn rec_v1() {}\n")]);
    stage_deletion(repo.dir.path(), "rec.rs");
    repo.write("rec.rs", "fn rec_v2() {}\n");

    let report = repo.json(&["--json", "symbols", "--changed"]);
    assert_eq!(
        files_and_names(&report),
        vec![("rec.rs".to_string(), vec!["rec_v2".to_string()])],
        "absence is decided by the current filesystem, not Git's deletion status"
    );
}

#[test]
fn a_copied_file_selects_the_copy_and_an_edited_original_selects_both() {
    let repo = Repo::committed(&[("orig.rs", "fn orig_fn() {}\n")]);
    std::fs::copy(repo.dir.path().join("orig.rs"), repo.dir.path().join("copy.rs")).unwrap();
    repo.write("orig.rs", "fn orig_fn() {}\nfn orig_edit() {}\n");

    let report = repo.json(&["--json", "symbols", "--changed"]);
    assert_eq!(
        files_and_names(&report),
        vec![
            ("copy.rs".to_string(), vec!["orig_fn".to_string()]),
            ("orig.rs".to_string(), vec!["orig_fn".to_string(), "orig_edit".to_string()]),
        ],
    );
}

#[test]
fn a_supplied_branch_tip_compares_tips_not_the_merge_base() {
    let repo = Repo::committed(&[
        ("feature.rs", "fn feature_base() {}\n"),
        ("mainside.rs", "fn main_base() {}\n"),
    ]);
    commit_on_branch(repo.dir.path(), "feature", "feature side", &[("feature.rs", "fn feature_side() {}\n")]);
    checkout_branch(repo.dir.path(), "main");
    commit_files(repo.dir.path(), "main side", crate::MAIN_BRANCH, &[("mainside.rs", "fn main_side() {}\n")]);
    checkout_branch(repo.dir.path(), "feature");
    // Restore feature's exact tree so status contributes no paths. Only a
    // direct tip comparison can select main's independently edited file.
    repo.write("mainside.rs", "fn main_base() {}\n");
    let own = repo.json(&["--json", "symbols", "--changed=feature"]);
    assert_eq!(own["files"], serde_json::json!([]));
    let report = repo.json(&["--json", "symbols", "--changed=main"]);
    assert_eq!(files_and_names(&report), vec![
        ("feature.rs".into(), vec!["feature_side".into()]),
        ("mainside.rs".into(), vec!["main_base".into()]),
    ]);
    assert_eq!(report["errors"], serde_json::json!([]));
}

#[test]
fn selectors_intersect_the_changed_set() {
    let repo = Repo::committed(&[
        ("keep.rs", "fn keep() {}\n"),
        ("drop.rs", "fn drop_fn() {}\n"),
        ("notes.txt", "x = 1\n"),
        ("script", "print(\"hi\")\n"),
    ]);
    repo.write("keep.rs", "fn keep() {}\nfn keep_edit() {}\n");
    repo.write("drop.rs", "fn drop_fn() {}\nfn drop_edit() {}\n");
    repo.write("notes.txt", "x = 2\n");
    repo.write("script", "print(\"hi\")\n\n\ndef edited():\n    pass\n");

    // A positional file-like token intersects…
    let positional = repo.json(&["--json", "symbols", "--changed", "keep.rs"]);
    assert_eq!(
        files_and_names(&positional).iter().map(|(file, _)| file.as_str()).collect::<Vec<_>>(),
        vec!["keep.rs"],
    );
    // …and so does `--file`.
    let filed = repo.json(&["--json", "symbols", "--changed", "--file", "keep.rs"]);
    assert_eq!(files_and_names(&filed), files_and_names(&positional));
    // `--exclude-files` wins over inclusion: drop.rs is changed but
    // excluded from the ordinary selection.
    let excluded = repo.json(&["--json", "symbols", "--changed", "--exclude-files", "drop.rs"]);
    assert_eq!(
        files_and_names(&excluded).iter().map(|(file, _)| file.as_str()).collect::<Vec<_>>(),
        vec!["keep.rs"],
    );
    // A forced language narrows the intersection to that language's files.
    let rust_only = repo.json(&["--json", "symbols", "--changed", "--language", "rust"]);
    assert_eq!(
        files_and_names(&rust_only).iter().map(|(file, _)| file.as_str()).collect::<Vec<_>>(),
        vec!["drop.rs", "keep.rs"],
    );
    // A forced language also keeps an explicitly named extensionless file
    // eligible.
    let forced = repo.json(&["--json", "symbols", "--changed", "--language", "python", "script"]);
    assert_eq!(
        files_and_names(&forced),
        vec![("script".to_string(), vec!["edited".to_string()])],
    );
    // A wrong forced language drops it again.
    let wrong = repo.json(&["--json", "symbols", "--changed", "--language", "go", "script"]);
    assert_eq!(wrong["files"].as_array().map(Vec::len), Some(0));
}

#[test]
fn a_package_invocation_keeps_the_scan_scope() {
    let repo = Repo::committed(&[
        ("Cargo.toml", "[workspace]\n"),
        ("pkg/Cargo.toml", "[package]\nname = \"p\"\nversion = \"0.1.0\"\n"),
        ("pkg/lib.rs", "fn in_package() {}\n"),
        ("root.rs", "fn at_root() {}\n"),
        ("other_root.rs", "fn other_root() {}\n"),
    ]);
    repo.write("pkg/lib.rs", "fn in_package() {}\nfn pkg_edit() {}\n");
    repo.write("root.rs", "fn at_root() {}\nfn root_edit() {}\n");
    let pkg = repo.dir.path().join("pkg");

    // From the package, the scan root stays the package: the changed file
    // at the repository root does not expand the default scan.
    let from_pkg = repo.json_in(&pkg, &["--json", "symbols", "--changed"]);
    assert_eq!(
        files_and_names(&from_pkg),
        vec![("lib.rs".to_string(), vec!["in_package".to_string(), "pkg_edit".to_string()])],
    );

    // From the repository root, both changed files are eligible.
    let from_root = repo.json(&["--json", "symbols", "--changed"]);
    assert_eq!(
        files_and_names(&from_root)
            .iter()
            .map(|(file, _)| file.as_str())
            .collect::<Vec<_>>(),
        vec!["pkg/lib.rs", "root.rs"],
    );
}

#[test]
fn an_explicit_file_outside_the_package_root_stays_eligible() {
    let repo = Repo::committed(&[
        ("Cargo.toml", "[workspace]\n"),
        ("pkg/Cargo.toml", "[package]\nname = \"p\"\nversion = \"0.1.0\"\n"),
        ("pkg/lib.rs", "fn in_package() {}\n"),
        ("root.rs", "fn at_root() {}\n"),
        ("unchanged_root.rs", "fn unchanged() {}\n"),
    ]);
    repo.write("root.rs", "fn at_root() {}\nfn root_edit() {}\n");
    let pkg = repo.dir.path().join("pkg");

    // Outside the package scan root but inside the discovered repository
    // and changed: eligible.
    let changed = repo.json_in(&pkg, &["--json", "symbols", "--changed", "--file", "../root.rs"]);
    assert_eq!(
        files_and_names(&changed)
            .iter()
            .map(|(file, names)| (file.as_str(), names.len()))
            .collect::<Vec<_>>(),
        vec![("../root.rs", 2)],
    );

    // The same explicit selector on an unchanged file selects nothing.
    let unchanged =
        repo.json_in(&pkg, &["--json", "symbols", "--changed", "--file", "../unchanged_root.rs"]);
    assert_eq!(unchanged["files"].as_array().map(Vec::len), Some(0));
    assert_eq!(unchanged["errors"].as_array().map(Vec::len), Some(0));
}

#[test]
fn a_linked_worktree_selects_its_own_checkout() {
    let repo = Repo::committed(&[("lib.rs", "fn one() {}\n")]);
    let worktrees = tempfile::TempDir::new().unwrap();
    let wt = worktrees.path().join("wt");
    crate::add_worktree(repo.dir.path(), "wt", &wt);

    // Each checkout edits the same tracked file differently.
    crate::write_file(&wt, "lib.rs", "fn one() {}\nfn worktree_edit() {}\n");
    repo.write("lib.rs", "fn one() {}\nfn main_edit() {}\n");

    // The linked worktree's own root and index decide: its edit is
    // reported, the main checkout's is not, and files that exist only in
    // the main worktree (say, an untracked one) never appear.
    repo.write("main_only.rs", "fn main_only() {}\n");
    let from_wt = repo.json_in(&wt, &["--json", "symbols", "--changed"]);
    assert_eq!(
        files_and_names(&from_wt),
        vec![("lib.rs".to_string(), vec!["one".to_string(), "worktree_edit".to_string()])],
    );

    let from_main = repo.json(&["--json", "symbols", "--changed", "--file", "lib.rs"]);
    assert_eq!(
        files_and_names(&from_main),
        vec![("lib.rs".to_string(), vec!["one".to_string(), "main_edit".to_string()])],
    );
}

#[test]
fn a_nested_repository_is_not_descended_into() {
    let repo = Repo::committed(&[("outer.rs", "fn outer_fn() {}\n")]);
    let inner = repo.dir.path().join("inner");
    std::fs::create_dir_all(&inner).unwrap();
    git_init(&inner);
    crate::write_file(&inner, "leaf.rs", "fn inner_fn() {}\n");
    crate::stage(&inner, "leaf.rs");
    repo.write("outer.rs", "fn outer_fn() {}\nfn outer_edit() {}\n");

    // The ordinary walk sees the nested repository's file…
    let ordinary = repo.json(&["--json", "symbols"]);
    assert_eq!(
        files_and_names(&ordinary)
            .iter()
            .map(|(file, _)| file.as_str())
            .collect::<Vec<_>>(),
        vec!["inner/leaf.rs", "outer.rs"],
    );
    // …but no recursive changed-file discovery selects inside it.
    let changed = repo.json(&["--json", "symbols", "--changed"]);
    assert_eq!(
        files_and_names(&changed),
        vec![("outer.rs".to_string(), vec!["outer_fn".to_string(), "outer_edit".to_string()])],
    );
}

#[test]
fn on_an_unborn_head_only_the_omitted_request_succeeds() {
    let repo = Repo::unborn();
    repo.write("new.rs", "fn first() {}\n");

    // The bare flag uses an empty base: staged and nonignored untracked
    // paths still select.
    let report = repo.json(&["--json", "symbols", "--changed"]);
    assert_eq!(
        files_and_names(&report),
        vec![("new.rs".to_string(), vec!["first".to_string()])],
    );

    // An explicit reference — `HEAD` included — must resolve to a commit,
    // and an unborn HEAD resolves to none.
    let output = repo.hug(&["symbols", "--changed=HEAD"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--changed=HEAD"), "{stderr}");
    assert!(stderr.contains("'HEAD' does not resolve to any object"), "{stderr}");
    assert!(output.stdout.is_empty());
}

#[test]
fn a_detached_head_uses_its_own_commit() {
    let repo = Repo::unborn();
    let first = commit_files(repo.dir.path(), "first", crate::MAIN_BRANCH, &[("a.rs", "fn first() {}\n")]);
    commit_files(repo.dir.path(), "second", crate::MAIN_BRANCH, &[("a.rs", "fn second() {}\n")]);
    crate::detach_head(repo.dir.path(), first);
    crate::reset_index_to_head(repo.dir.path());
    repo.write("a.rs", "fn first() {}\n");
    let ordinary_changed = repo.json(&["--json", "symbols", "--changed"]);
    assert_eq!(ordinary_changed["files"], serde_json::json!([]));
    let report = repo.json(&["--json", "symbols", "--changed=main"]);
    assert_eq!(files_and_names(&report), vec![("a.rs".into(), vec!["first".into()])]);
    assert_eq!(report["errors"], serde_json::json!([]));
}

#[test]
fn commit_ids_and_tags_select_the_same_current_records() {
    let repo = Repo::committed(&[("lib.rs", "fn before() {}\n")]);
    let handle = git2::Repository::open(repo.dir.path()).unwrap();
    let base = handle.head().unwrap().peel_to_commit().unwrap();
    let id = base.id().to_string();
    handle.tag_lightweight("light", base.as_object(), false).unwrap();
    handle.tag("annotated", base.as_object(), &crate::signature(), "fixture tag", false).unwrap();
    drop(base);
    drop(handle);
    commit_files(repo.dir.path(), "second", crate::MAIN_BRANCH, &[("lib.rs", "fn committed() {}\n")]);
    let clean = repo.json(&["--json", "symbols", "--changed"]);
    assert_eq!(clean["files"], serde_json::json!([]));
    for reference in [id.as_str(), "light", "annotated"] {
        let flag = format!("--changed={reference}");
        let report = repo.json(&["--json", "symbols", &flag]);
        assert_eq!(files_and_names(&report), vec![("lib.rs".into(), vec!["committed".into()])]);
        assert_eq!(report["errors"], serde_json::json!([]));
    }
    repo.write("lib.rs", "fn current() {}\n");
    for reference in [id.as_str(), "light", "annotated"] {
        let flag = format!("--changed={reference}");
        let report = repo.json(&["--json", "symbols", &flag]);
        assert_eq!(files_and_names(&report), vec![("lib.rs".into(), vec!["current".into()])]);
        assert_eq!(report["errors"], serde_json::json!([]));
    }
}

#[test]
fn an_explicit_base_on_unborn_head_compares_with_the_empty_tree() {
    let repo = Repo::committed(&[("base.rs", "fn base() {}\n")]);
    let handle = git2::Repository::open(repo.dir.path()).unwrap();
    handle.set_head("refs/heads/unborn").unwrap();
    let mut index = handle.index().unwrap();
    index.clear().unwrap();
    index.write().unwrap();
    drop(index);
    drop(handle);
    repo.write("staged.rs", "fn staged() {}\n");
    repo.stage("staged.rs");
    // Exclude the old base from untracked status: only the committed
    // comparison can bring it back through the explicit file override.
    repo.write(".gitignore", "base.rs\n");
    let omitted = repo.json(&["--json", "symbols", "--changed", "--file", "base.rs"]);
    assert_eq!(omitted["files"], serde_json::json!([]));
    let base = repo.json(&["--json", "symbols", "--changed=main", "--file", "base.rs"]);
    assert_eq!(files_and_names(&base), vec![("base.rs".into(), vec!["base".into()])]);
    // The default walk still honors .gitignore, even for a base-tip path.
    let report = repo.json(&["--json", "symbols", "--changed=main"]);
    assert_eq!(files_and_names(&report), vec![("staged.rs".into(), vec!["staged".into()])]);
    assert_eq!(report["errors"], serde_json::json!([]));
}

#[test]
fn submodule_contents_are_never_selected() {
    let repo = Repo::committed(&[("outer.rs", "fn outer() {}\n")]);
    let inner = repo.dir.path().join("module");
    std::fs::create_dir_all(&inner).unwrap();
    git_init(&inner);
    let commit = commit_files(&inner, "submodule", crate::MAIN_BRANCH, &[("inner.rs", "fn inner() {}\n")]);
    let handle = git2::Repository::open(repo.dir.path()).unwrap();
    let mut index = handle.index().unwrap();
    index.add(&git2::IndexEntry {
        ctime: git2::IndexTime::new(0, 0), mtime: git2::IndexTime::new(0, 0),
        dev: 0, ino: 0, mode: 0o160000, uid: 0, gid: 0, file_size: 0,
        id: commit, flags: 0, flags_extended: 0, path: b"module".to_vec(),
    }).unwrap();
    index.write().unwrap();
    drop(index);
    drop(handle);
    crate::write_file(&inner, "inner.rs", "fn changed_inner() {}\n");
    repo.write("outer.rs", "fn changed_outer() {}\n");
    for extra in [vec![], vec!["--file", "module/inner.rs"]] {
        let mut args = vec!["--json", "symbols", "--changed"];
        args.extend(extra.iter().copied());
        let report = repo.json(&args);
        let expected = if extra.is_empty() { vec![("outer.rs".into(), vec!["changed_outer".into()])] } else { vec![] };
        assert_eq!(files_and_names(&report), expected);
        assert_eq!(report["errors"], serde_json::json!([]));
    }
}

#[test]
fn an_explicit_file_in_another_repository_is_not_selected() {
    let repo = Repo::committed(&[("lib.rs", "fn local() {}\n")]);
    let foreign = Repo::committed(&[("lib.rs", "fn foreign() {}\n")]);
    foreign.write("lib.rs", "fn foreign_edit() {}\n");
    let path = foreign.dir.path().join("lib.rs");
    let selector = path.to_str().unwrap();
    let ordinary = repo.json(&["--json", "symbols", "--file", selector]);
    assert_eq!(files_and_names(&ordinary)[0].1, vec!["foreign_edit"]);
    let changed = repo.json(&["--json", "symbols", "--changed", "--file", selector]);
    assert_eq!(changed["files"], serde_json::json!([]));
    assert_eq!(changed["errors"], serde_json::json!([]));
}

#[test]
fn tracked_ignored_files_and_combined_selectors_keep_existing_precedence() {
    let repo = Repo::committed(&[("ignored.rs", "fn old() {}\n"), ("clean.rs", "fn clean() {}\n")]);
    repo.write(".gitignore", "ignored.rs\n");
    repo.write("ignored.rs", "fn ignored_edit() {}\n");
    repo.write("other.rs", "fn other_edit() {}\nfn unrelated() {}\n");
    let args = ["--json", "symbols", "--changed", "ignored.rs", "--file", "other.rs", "--name", "edit"];
    let report = repo.json(&args);
    assert_eq!(files_and_names(&report), vec![
        ("ignored.rs".into(), vec!["ignored_edit".into()]),
        ("other.rs".into(), vec!["other_edit".into()]),
    ]);
    let mut excluded = args.to_vec();
    excluded.extend(["--exclude-files", "ignored.rs"]);
    let report = repo.json(&excluded);
    assert_eq!(files_and_names(&report), vec![("other.rs".into(), vec!["other_edit".into()])]);
    assert_eq!(report["errors"], serde_json::json!([]));
}
