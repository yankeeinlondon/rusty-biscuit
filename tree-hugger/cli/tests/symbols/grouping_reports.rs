use crate::{Scratch, json_of};

fn workspace() -> Scratch {
    Scratch::new(&[
        ("Cargo.toml", "[workspace]\nmembers = [\"a\", \"z\"]\n"),
        ("a/Cargo.toml", "[package]\nname = \"utils\"\nversion = \"0.1.0\"\n"),
        ("z/Cargo.toml", "[package]\nname = \"utils\"\nversion = \"0.1.0\"\n"),
        ("a/src/lib.rs", "/// Documented.\npub fn first() {}\npub fn second() {}\n"),
        ("z/src/lib.rs", "pub fn third() {}\n"),
        ("loose.rs", "pub fn loose() {}\n"),
    ])
}

#[test]
fn json_groups_preserve_overall_counts_and_order() {
    let scratch = workspace();
    let raw = scratch.stdout(&["--json", "functions", "--stats"]);
    let report = json_of(&raw);
    assert_eq!(report["report_schema_version"], 1);
    assert_eq!(report["package_root_dir"], biscuit_file::to_portable_string(&biscuit_file::canonicalize_simplified(scratch.repo.path()).unwrap()));
    for (before, after) in [("root_dir", "package_root_dir"), ("package_root_dir", "stats"), ("stats", "groups"), ("groups", "errors")] {
        assert!(raw.find(&format!("\"{before}\"")).unwrap() < raw.find(&format!("\"{after}\"")).unwrap());
    }
    assert!(raw.ends_with(",\"errors\":[]}\n"), "{raw}");
    let groups = report["groups"].as_array().expect("group array");
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0]["package"], serde_json::json!({"name": "utils", "path": "a"}));
    assert_eq!(groups[1]["package"], serde_json::json!({"name": "utils", "path": "z"}));
    assert!(groups[2]["package"].is_null());
    for group in groups {
        assert_eq!(group["stats"].as_object().unwrap().keys().collect::<Vec<_>>(), report["stats"].as_object().unwrap().keys().collect::<Vec<_>>());
    }
    for field in ["by_kind", "by_visibility", "by_language"] {
        for (key, value) in report["stats"][field].as_object().unwrap() {
            let sum: u64 = groups.iter().map(|g| g["stats"][field][key].as_u64().unwrap_or(0)).sum();
            assert_eq!(sum, value.as_u64().unwrap(), "{field}.{key}");
        }
    }
    for field in ["documented", "total", "unavailable"] {
        let sum: u64 = groups.iter().map(|g| g["stats"]["doc_coverage"][field].as_u64().unwrap()).sum();
        assert_eq!(sum, report["stats"]["doc_coverage"][field]);
    }
    assert_eq!(groups.iter().map(|g| g["stats"]["synthetic_exports"].as_u64().unwrap()).sum::<u64>(), report["stats"]["synthetic_exports"]);
    assert_eq!(report["stats"]["doc_coverage"]["ratio"], 0.25);
}

#[test]
fn root_unassigned_zero_and_empty_reports() {
    for (manifest, identity) in [
        (Some("[package]\nname = \"Unassigned\"\nversion = \"0.1.0\"\n"), serde_json::json!({"name": "Unassigned", "path": "."})),
        (None, serde_json::Value::Null),
    ] {
        let scratch = Scratch::new(&[("lib.rs", "pub fn selected() {}\n")]);
        if let Some(manifest) = manifest { std::fs::write(scratch.repo.path().join("Cargo.toml"), manifest).unwrap(); }
        for extra in [vec![], vec!["--name", "no_matching_declarations"]] {
            let mut args = vec!["--json", "functions", "--stats", "lib.rs"];
            args.extend(extra.iter().copied());
            let report = json_of(&scratch.stdout(&args));
            assert!(report["package_root_dir"].is_string());
            assert_eq!(report["groups"].as_array().unwrap().len(), 1);
            assert_eq!(report["groups"][0]["package"], identity);
            assert_eq!(report["groups"][0]["stats"], report["stats"]);
            if !extra.is_empty() { assert!(report["groups"][0]["stats"]["doc_coverage"]["ratio"].is_null()); }
        }
        std::fs::remove_file(scratch.repo.path().join("lib.rs")).unwrap();
        let report = json_of(&scratch.stdout(&["--json", "functions", "--stats", "--changed"]));
        assert!(report["package_root_dir"].is_string());
        assert_eq!(report["groups"], serde_json::json!([]));
        assert_eq!(report["stats"]["by_kind"], serde_json::json!({}));
    }
}

#[test]
fn terminal_sections_are_ordered_and_literal() {
    let scratch = workspace();
    let name = "ユーティル <b>bold</b> [link](https://example.invalid)\u{1b}]8;;evil\u{7}\n";
    std::fs::write(scratch.repo.path().join("a/Cargo.toml"), format!("[package]\nname = {}\nversion = \"0.1.0\"\n", serde_json::to_string(name).unwrap())).unwrap();
    let json = json_of(&scratch.stdout(&["--json", "functions", "--stats"]));
    assert_eq!(json["groups"][0]["package"]["name"], name);
    let expected = "ユーティル <b>bold</b> [link](https://example.invalid)\\u{1b}]8;;evil\\u{7}\\u{a} (a)";
    for format in [vec![], vec!["--format", "plain"], vec!["--format", "table"]] {
        let mut args = vec!["functions", "--stats", "--name", "nothing_matches"];
        args.extend(format);
        let text = scratch.stdout(&args);
        let headings = ["All selected declarations", expected, "utils (z)", "Unassigned"];
        let positions: Vec<_> = headings.iter().map(|h| text.find(h).unwrap_or_else(|| panic!("missing {h}: {text}"))).collect();
        assert!(positions.windows(2).all(|p| p[0] < p[1]), "{text}");
        assert_eq!(text.matches("Doc coverage: 0 of 0 documented (n/a); 0 unavailable").count(), 4);
        assert!(!text.contains('\u{1b}') && !text.contains('\u{7}'), "{text:?}");
    }
}

#[test]
fn stats_option_conflicts_remain_argument_errors() {
    let scratch = workspace();
    for command in ["functions", "types", "symbols", "classes"] {
        for flags in [vec!["--count"], vec!["--jsonl"], vec!["--json-full"], vec!["--sort", "file"], vec!["--signatures", "full"], vec!["--signatures", "short"], vec!["--signatures", "none"], vec!["--tree"]] {
            let mut args = vec![command, "--stats"];
            args.extend(flags);
            let output = scratch.hug(&args);
            assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
            assert!(output.stdout.is_empty(), "{args:?}");
            assert!(!output.stderr.is_empty(), "{args:?}");
        }
        for flags in [vec![], vec!["--json"], vec!["--format", "plain"], vec!["--format", "table"]] {
            let mut args = vec![command, "--stats"];
            args.extend(flags);
            assert!(scratch.hug(&args).status.success(), "{args:?}");
        }
    }
}

#[test]
fn subpackage_scan_keeps_separate_root_frames() {
    let scratch = workspace();
    let args = ["--json", "functions", "--stats"];
    let output = scratch.process(&args).current_dir(scratch.repo.path().join("a")).output().unwrap();
    let report = json_of(&crate::stdout_of(output, &args));
    let root = biscuit_file::canonicalize_simplified(scratch.repo.path()).unwrap();
    assert_eq!(report["root_dir"], biscuit_file::to_portable_string(&root.join("a")));
    assert_eq!(report["package_root_dir"], biscuit_file::to_portable_string(&root));
    assert_eq!(report["groups"].as_array().unwrap().len(), 1);
    assert_eq!(report["groups"][0]["package"]["path"], "a");
    assert_eq!(report["groups"][0]["stats"], report["stats"]);
}
