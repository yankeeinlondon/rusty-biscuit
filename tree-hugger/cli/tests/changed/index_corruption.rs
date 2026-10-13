//! Every declaration format rejects malformed indexes before reporting.

use crate::{Repo, add_worktree, files_and_names};
use std::path::Path;

fn assert_reports_reject(repo: &Repo, root: &Path, index: &Path, length: usize) {
    let original = std::fs::read(index).unwrap();
    assert_eq!(original.len(), 104, "known tool-written index");
    for command in ["functions", "types", "symbols", "classes"] {
        let expected: Vec<&str> = match command {
            "functions" => vec!["edited"], "symbols" => vec!["edited", "Edited"], _ => vec!["Edited"],
        };
        for base in ["--changed", "--changed=HEAD"] {
            let report = repo.json_in(root, &[command, base, "--json"]);
            let files = files_and_names(&report);
            assert_eq!(files.len(), 1);
            assert_eq!(files[0].1, expected, "independent positive control");
            {
                std::fs::write(index, &original).unwrap();
                std::fs::write(index, &original[..length]).unwrap();
                let mut modes: Vec<Vec<&str>> = vec![
                    vec![], vec!["--plain"], vec!["--format", "plain"], vec!["--format", "table"],
                    vec!["--json"], vec!["--jsonl"], vec!["--json-full"], vec!["--count"],
                    vec!["--stats"], vec!["--stats", "--json"],
                ];
                if command != "classes" { modes.push(vec!["--tree"]); }
                for mode in modes {
                    let mut args = vec!["--verbose", command, base];
                    args.extend(mode);
                    let output = repo.hug_in(root, &args);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    assert_eq!(output.status.code(), Some(1), "{length}: {args:?}: {stderr}");
                    assert!(output.stdout.is_empty(), "no partial report for {args:?}");
                    assert!(stderr.contains("index"), "index context: {stderr}");
                    assert!(stderr.contains(&root.display().to_string()), "repository context: {stderr}");
                    assert!(!stderr.contains("cache: "), "failure precedes source/cache analysis: {stderr}");
                }
            }
            std::fs::write(index, &original).unwrap();
        }
    }
}

fn main_index(length: usize) {
    let repo = Repo::committed(&[("leaf.rs", "fn original() {}\nstruct Original;\n")]);
    repo.write("leaf.rs", "fn edited() {}\nstruct Edited;\n");
    assert_reports_reject(&repo, repo.dir.path(), &repo.dir.path().join(".git/index"), length);
}

fn linked_index(length: usize) {
    let repo = Repo::committed(&[("leaf.rs", "fn original() {}\nstruct Original;\n")]);
    let parent = tempfile::TempDir::new().unwrap();
    let linked = parent.path().join("linked");
    add_worktree(repo.dir.path(), "linked", &linked);
    std::fs::write(linked.join("leaf.rs"), "fn edited() {}\nstruct Edited;\n").unwrap();
    let gitdir = git2::Repository::open(&linked).unwrap().path().to_path_buf();
    assert_reports_reject(&repo, &linked, &gitdir.join("index"), length);
    std::fs::remove_file(gitdir.join("index")).unwrap();
    assert_eq!(files_and_names(&repo.json_in(&linked, &["symbols", "--changed", "--json"]))[0].1, vec!["edited", "Edited"]);
}

macro_rules! index_prefix_tests {
    ($(($main:ident, $linked:ident, $length:expr)),+ $(,)?) => {
        $(
            #[test]
            fn $main() { main_index($length); }
            #[test]
            fn $linked() { linked_index($length); }
        )+
    };
}

index_prefix_tests! {
    (main_0, linked_0, 0), (main_1, linked_1, 1), (main_4, linked_4, 4),
    (main_8, linked_8, 8), (main_11, linked_11, 11), (main_12, linked_12, 12),
    (main_13, linked_13, 13), (main_16, linked_16, 16), (main_20, linked_20, 20),
    (main_31, linked_31, 31), (main_84, linked_84, 84), (main_103, linked_103, 103),
}

#[test]
fn absent_and_tool_written_empty_indexes_preserve_current_declarations() {
    let repo = Repo::committed(&[("leaf.rs", "fn original() {}\nstruct Original;\n")]);
    repo.write("leaf.rs", "fn edited() {}\nstruct Edited;\n");
    std::fs::remove_file(repo.dir.path().join(".git/index")).unwrap();
    assert_eq!(files_and_names(&repo.json(&["symbols", "--changed", "--json"]))[0].1, vec!["edited", "Edited"]);
    let unborn = Repo::unborn();
    unborn.write("first.rs", "fn first() {}\nstruct First;\n");
    for empty_index in [false, true] {
        if empty_index { git2::Repository::open(unborn.dir.path()).unwrap().index().unwrap().write().unwrap(); }
        for (command, names) in [("functions", vec!["first"]), ("types", vec!["First"]), ("symbols", vec!["first", "First"]), ("classes", vec!["First"])] {
            let files = files_and_names(&unborn.json(&[command, "--changed", "--json"]));
            assert_eq!(files.len(), 1);
            assert_eq!(files[0].1, names);
        }
    }
}
