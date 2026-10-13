//! File-type replacements remain selected and report current disk declarations.

use crate::{Repo, MAIN_BRANCH, commit_index, files_and_names, json_of};

fn symlink_file(target: &std::path::Path, link: &std::path::Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).expect("create file symlink");
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(target, link).expect("file symlink requires Developer Mode or symlink privilege");
}

fn check_file_type_replacement(initially_symlink: bool, command: &str) {
    {
        let repo = Repo::committed(&[
            ("leaf.rs", "fn original() {}\nstruct Original;\n"),
            ("target.rs", "fn target() {}\nstruct Target;\n"),
        ]);
        git2::Repository::open(repo.dir.path()).unwrap().config().unwrap().set_bool("core.symlinks", true).unwrap();
        if initially_symlink {
            std::fs::remove_file(repo.dir.path().join("leaf.rs")).unwrap();
            symlink_file(std::path::Path::new("target.rs"), &repo.dir.path().join("leaf.rs"));
            repo.stage("leaf.rs");
            commit_index(repo.dir.path(), "symlink base", MAIN_BRANCH);
        }
        let base = git2::Repository::open(repo.dir.path()).unwrap().head().unwrap().target().unwrap();
        std::fs::remove_file(repo.dir.path().join("leaf.rs")).unwrap();
        let (function, typ) = if initially_symlink {
            repo.write("leaf.rs", "fn restored() {}\nstruct Restored;\n");
            ("restored", "Restored")
        } else {
            symlink_file(std::path::Path::new("target.rs"), &repo.dir.path().join("leaf.rs"));
            ("target", "Target")
        };
        {
            let names: Vec<String> = match command {
                "functions" => vec![function.into()],
                "symbols" => vec![function.into(), typ.into()],
                _ => vec![typ.into()],
            };
            for (mode, extra) in [
                ("terminal", vec![]), ("plain", vec!["--plain"]),
                ("table", vec!["--format", "table"]), ("json", vec!["--json"]),
                ("json-lines", vec!["--jsonl"]), ("full-json", vec!["--json-full"]),
                ("count", vec!["--count"]), ("stats", vec!["--stats"]),
                ("stats-json", vec!["--json", "--stats"]),
            ] {
                let mut args = vec![command, "--changed", "--file", "leaf.rs"];
                args.extend(extra);
                let stdout = repo.stdout(&args);
                match mode {
                    "count" => assert_eq!(stdout, format!("{}\n", names.len())),
                    "json" => {
                        let report = json_of(&stdout);
                        let files = files_and_names(&report);
                        assert_eq!(files.len(), 1);
                        assert_eq!(files[0].1, names, "{command} / {mode}");
                        assert_eq!(report["errors"], serde_json::json!([]));
                    }
                    "json-lines" => {
                        let actual: Vec<String> = stdout.lines().map(|line| json_of(line)["record"]["identity"]["name"].as_str().unwrap().to_string()).collect();
                        assert_eq!(actual, names, "{command} / {mode}");
                    }
                    "full-json" => {
                        let report = json_of(&stdout);
                        let indexes = report["indexes"].as_array().unwrap();
                        assert_eq!(indexes.len(), 1);
                        let actual: Vec<String> = indexes[0]["symbols"].as_array().unwrap().iter().map(|record| record["identity"]["name"].as_str().unwrap().to_string()).collect();
                        assert_eq!(actual, vec![function, typ], "complete index preserves both declarations");
                        assert_eq!(report["errors"], serde_json::json!([]));
                    }
                    "stats-json" => {
                        let report = json_of(&stdout);
                        assert_eq!(report["stats"]["doc_coverage"]["total"], names.len());
                        assert_eq!(report["stats"]["by_language"], serde_json::json!({"Rust": names.len()}));
                        assert_eq!(report["errors"], serde_json::json!([]));
                    }
                    "stats" => assert!(stdout.contains(&format!("0 of {} documented", names.len())), "{stdout}"),
                    _ => {
                        for name in &names { assert!(stdout.contains(name), "{command} / {mode}: {stdout}"); }
                        assert!(!stdout.contains("original") && !stdout.contains("Original"), "old declarations must not be reported");
                    }
                }
            }
            let uncached = repo.json(&[command, "--changed", "--file", "leaf.rs", "--json", "--no-cache"]);
            assert_eq!(files_and_names(&uncached)[0].1, names);
            let warm = repo.hug(&["--verbose", command, "--changed", "--file", "leaf.rs", "--json"]);
            assert!(crate::cache_stats(&warm).starts_with("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"));
            let warm = json_of(&crate::stdout_of(warm, &[command]));
            assert_eq!(files_and_names(&warm).len(), 1);
            assert_eq!(files_and_names(&warm)[0].1, names);
            if command != "classes" {
                let stdout = repo.stdout(&[command, "--changed", "--file", "leaf.rs", "--tree"]);
                for name in &names { assert!(stdout.contains(name), "{command} / tree: {stdout}"); }
            }
        }
        repo.stage("leaf.rs");
        let count = if command == "symbols" { 2 } else { 1 };
        {
            assert_eq!(repo.stdout(&[command, "--changed", "--file", "leaf.rs", "--count"]), format!("{count}\n"));
        }
        commit_index(repo.dir.path(), "type replacement", MAIN_BRANCH);
        let reference = format!("--changed={base}");
        {
            assert_eq!(repo.stdout(&[command, &reference, "--file", "leaf.rs", "--count"]), format!("{count}\n"));
            assert_eq!(repo.stdout(&[command, "--changed", "--file", "leaf.rs", "--count"]), "0\n");
        }
    }
}

macro_rules! replacement_cases {
    ($($direction:ident => $symlink:literal),+ $(,)?) => {
        mod file_type_replacements_report_current_declarations_in_every_mode {
            $(mod $direction {
                #[test] fn functions() { super::super::check_file_type_replacement($symlink, "functions"); }
                #[test] fn types() { super::super::check_file_type_replacement($symlink, "types"); }
                #[test] fn symbols() { super::super::check_file_type_replacement($symlink, "symbols"); }
                #[test] fn classes() { super::super::check_file_type_replacement($symlink, "classes"); }
            })+
        }
    };
}
replacement_cases! { file_to_symlink => false, symlink_to_file => true }
