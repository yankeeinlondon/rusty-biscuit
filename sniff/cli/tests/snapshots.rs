use serde_json::{Value, json};

fn run_stdout(args: &[&str]) -> String {
    let output = assert_cmd::Command::cargo_bin("sniff")
        .unwrap()
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    String::from_utf8(output).expect("stdout should be valid UTF-8")
}

fn strip_ansi(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if matches!(chars.peek(), Some('[')) {
                let _ = chars.next();
                for next in chars.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
            continue;
        }
        output.push(ch);
    }

    output
}

fn normalize_text(output: &str) -> String {
    strip_ansi(output).trim().to_string()
}

fn normalize_help_output(output: &str) -> String {
    // Clap derives the usage binary from argv[0], which carries `.exe` on Windows.
    normalize_text(output).replace("Usage: sniff.exe ", "Usage: sniff ")
}

fn normalize_topics_table(output: &str) -> String {
    let cleaned = normalize_text(output);

    // Already in markdown table form.
    if cleaned
        .lines()
        .next()
        .is_some_and(|line| line.starts_with('|'))
    {
        return cleaned;
    }

    // Convert box-drawn table output back to stable markdown.
    let rows: Vec<Vec<String>> = cleaned
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.starts_with('│') {
                return None;
            }

            let cells: Vec<String> = trimmed
                .split('│')
                .skip(1)
                .take(3)
                .map(|cell| cell.trim().to_string())
                .collect();

            if cells.is_empty() { None } else { Some(cells) }
        })
        .collect();

    if rows.is_empty() {
        return cleaned;
    }

    let mut lines = Vec::new();
    lines.push(format!("| {} |", rows[0].join(" | ")));
    lines.push(format!(
        "| {} |",
        rows[0]
            .iter()
            .map(|_| "---")
            .collect::<Vec<_>>()
            .join(" | ")
    ));

    for row in rows.iter().skip(1) {
        lines.push(format!("| {} |", row.join(" | ")));
    }

    lines.join("\n")
}

fn normalized_os_summary(output: &str) -> Value {
    let json: Value = serde_json::from_str(&normalize_text(output)).expect("valid JSON");
    // The manager count is host-dependent (how many package managers happen to
    // be installed), so the golden asserts presence of the array, not the count.
    assert!(
        json.get("system_package_managers")
            .and_then(|v| v.get("managers"))
            .is_some_and(Value::is_array),
        "system_package_managers.managers should be an array"
    );

    for field in ["kernel", "long_version", "version"] {
        assert!(
            json.get(field)
                .and_then(Value::as_str)
                .is_some_and(|value| !value.is_empty()),
            "{field} should be a non-empty string"
        );
    }

    json!({
        "distribution": json
            .get("distribution")
            .and_then(Value::as_str)
            .map(|_| "<normalized>"),
        "kernel": json
            .get("kernel")
            .and_then(Value::as_str)
            .map(|_| "<normalized>"),
        "long_version": json
            .get("long_version")
            .and_then(Value::as_str)
            .map(|_| "<normalized>"),
        "name": json
            .get("name")
            .and_then(Value::as_str)
            .map(|_| "<normalized>"),
        "os_type": json
            .get("os_type")
            .and_then(Value::as_str)
            .map(|_| "<normalized>"),
        "version": json
            .get("version")
            .and_then(Value::as_str)
            .map(|_| "<normalized>"),
        "manager_count": "<normalized>"
    })
}

#[test]
fn help_output_snapshot() {
    insta::assert_snapshot!(
        "help_output",
        normalize_help_output(&run_stdout(&["--help"]))
    );
}

#[test]
fn completions_help_output_snapshot() {
    insta::assert_snapshot!(
        "completions_help_output",
        normalize_text(&run_stdout(&["--completions", "--help"]))
    );
}

#[test]
fn topics_table_snapshot() {
    insta::assert_snapshot!(
        "topics_table",
        normalize_topics_table(&run_stdout(&["topics"]))
    );
}

#[test]
fn os_json_snapshot() {
    let stdout = run_stdout(&["os", "--json"]);
    insta::assert_json_snapshot!("os_json_summary", normalized_os_summary(&stdout));
}

// ============================================================================
// Phase 8 — monorepo topology CLI snapshots
// ============================================================================

fn init_git_repo(path: &std::path::Path) -> git2::Repository {
    let mut options = git2::RepositoryInitOptions::new();
    options.initial_head("main");
    let repo = git2::Repository::init_opts(path, &options).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test").unwrap();
    repo
}

fn commit_all(repo: &git2::Repository, message: &str) {
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let sig = repo.signature().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let parents: Vec<git2::Commit<'_>> = match repo.head() {
        Ok(head) => vec![head.peel_to_commit().unwrap()],
        Err(_) => vec![],
    };
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
        .unwrap();
}

fn create_cargo_monorepo_fixture() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let repo = init_git_repo(dir.path());

    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[workspace]
members = ["pkg-a", "pkg-b"]
"#,
    )
    .unwrap();

    for (name, pkg_name) in [("pkg-a", "pkg-a"), ("pkg-b", "pkg-b")] {
        let pkg = dir.path().join(name);
        std::fs::create_dir_all(pkg.join("src")).unwrap();
        std::fs::write(
            pkg.join("Cargo.toml"),
            format!(
                r#"[package]
name = "{}"
version = "0.1.0"
edition = "2024"
"#,
                pkg_name
            ),
        )
        .unwrap();
        std::fs::write(pkg.join("src/lib.rs"), "pub fn x() {}").unwrap();
    }

    commit_all(&repo, "initial cargo monorepo");
    let path = dir.path().to_path_buf();
    (dir, path)
}

fn create_cargo_pnpm_monorepo_fixture() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let repo = init_git_repo(dir.path());

    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[workspace]
members = ["rust-a", "rust-b"]
"#,
    )
    .unwrap();

    for (name, pkg_name) in [("rust-a", "rust-a"), ("rust-b", "rust-b")] {
        let pkg = dir.path().join(name);
        std::fs::create_dir_all(pkg.join("src")).unwrap();
        std::fs::write(
            pkg.join("Cargo.toml"),
            format!(
                r#"[package]
name = "{}"
version = "0.1.0"
edition = "2024"
"#,
                pkg_name
            ),
        )
        .unwrap();
        std::fs::write(pkg.join("src/lib.rs"), "pub fn a() {}").unwrap();
    }

    std::fs::write(
        dir.path().join("pnpm-workspace.yaml"),
        r#"packages:
  - "js-a"
  - "js-b"
"#,
    )
    .unwrap();

    for (name, pkg_name) in [("js-a", "js-a"), ("js-b", "js-b")] {
        let pkg = dir.path().join(name);
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(
            pkg.join("package.json"),
            format!(
                r#"{{
  "name": "{}",
  "version": "0.1.0"
}}
"#,
                pkg_name
            ),
        )
        .unwrap();
    }

    commit_all(&repo, "initial cargo + pnpm monorepo");
    let path = dir.path().to_path_buf();
    (dir, path)
}

fn create_pnpm_nx_monorepo_fixture() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let repo = init_git_repo(dir.path());

    std::fs::write(
        dir.path().join("pnpm-workspace.yaml"),
        r#"packages:
  - "apps/*"
"#,
    )
    .unwrap();

    std::fs::write(
        dir.path().join("nx.json"),
        r#"{
  "extends": "nx/presets/npm.json"
}
"#,
    )
    .unwrap();

    for name in ["web", "api"] {
        let app = dir.path().join(format!("apps/{}", name));
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(
            app.join("package.json"),
            format!(
                r#"{{
  "name": "{}",
  "version": "0.1.0"
}}
"#,
                name
            ),
        )
        .unwrap();
    }

    commit_all(&repo, "initial pnpm + nx monorepo");
    let path = dir.path().to_path_buf();
    (dir, path)
}

fn create_degenerate_cargo_fixture() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let repo = init_git_repo(dir.path());

    std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();

    commit_all(&repo, "initial degenerate workspace");
    let path = dir.path().to_path_buf();
    (dir, path)
}

fn run_repo_structure(base: &std::path::Path) -> String {
    let output = assert_cmd::Command::cargo_bin("sniff")
        .unwrap()
        .args([
            "--base",
            base.to_str().unwrap(),
            "repo",
            "structure",
            "--plain",
        ])
        .env("NO_COLOR", "1")
        .output()
        .expect("run sniff repo structure --plain");

    assert!(
        output.status.success(),
        "repo structure must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("stdout is utf8")
}

fn run_repo_structure_json(base: &std::path::Path) -> serde_json::Value {
    let output = assert_cmd::Command::cargo_bin("sniff")
        .unwrap()
        .args([
            "--base",
            base.to_str().unwrap(),
            "repo",
            "structure",
            "--json",
        ])
        .env("NO_COLOR", "1")
        .output()
        .expect("run sniff repo structure --json");

    assert!(
        output.status.success(),
        "repo structure --json must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_str(std::str::from_utf8(&output.stdout).unwrap()).expect("valid json")
}

fn stable_topology_json(json: &serde_json::Value) -> serde_json::Value {
    let standards = json
        .get("monorepo_standards")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|s| {
                    json!({
                        "standard": s.get("standard").cloned().unwrap_or(Value::Null),
                        "confidence": s.get("confidence").cloned().unwrap_or(Value::Null),
                    })
                })
                .collect::<Vec<_>>()
        })
        .map(Value::Array);

    let layers = json
        .get("monorepo_layers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|layer| {
                    let packages = layer
                        .get("packages")
                        .and_then(|p| p.as_array())
                        .map(|pkgs| {
                            pkgs.iter()
                                .map(|p| {
                                    p.as_str()
                                        .map(|s| Value::String(s.to_string()))
                                        .unwrap_or_else(|| p.clone())
                                })
                                .collect::<Vec<_>>()
                        })
                        .map(Value::Array)
                        .unwrap_or_else(|| json!([]));
                    json!({
                        "authority": layer.get("authority").cloned().unwrap_or(Value::Null),
                        "orchestrators": layer.get("orchestrators").cloned().unwrap_or_else(|| json!([])),
                        "provenance": layer.get("provenance").cloned().unwrap_or(Value::Null),
                        "packages": packages,
                    })
                })
                .collect::<Vec<_>>()
        })
        .map(Value::Array);

    json!({
        "is_monorepo": json.get("is_monorepo").cloned().unwrap_or(Value::Null),
        "monorepo_standards": standards.unwrap_or_else(|| json!([])),
        "monorepo_layers": layers.unwrap_or_else(|| json!([])),
        "package_count": json["packages"].as_array().map(|a| a.len()),
    })
}

fn normalize_snapshot_paths(input: &str, base: &std::path::Path) -> String {
    input.replace(base.to_str().unwrap(), "[BASE]")
}

#[test]
fn cargo_monorepo_structure_text_snapshot() {
    let (_dir, path) = create_cargo_monorepo_fixture();
    let stdout = run_repo_structure(&path);
    insta::assert_snapshot!(
        "cargo_monorepo_structure_text",
        normalize_snapshot_paths(&stdout, &path)
    );
}

#[test]
fn cargo_monorepo_structure_json_snapshot() {
    let (_dir, path) = create_cargo_monorepo_fixture();
    let json = run_repo_structure_json(&path);
    insta::assert_json_snapshot!("cargo_monorepo_structure_json", stable_topology_json(&json));
}

#[test]
fn cargo_pnpm_monorepo_structure_text_snapshot() {
    let (_dir, path) = create_cargo_pnpm_monorepo_fixture();
    let stdout = run_repo_structure(&path);
    insta::assert_snapshot!(
        "cargo_pnpm_monorepo_structure_text",
        normalize_snapshot_paths(&stdout, &path)
    );
}

#[test]
fn cargo_pnpm_monorepo_structure_json_snapshot() {
    let (_dir, path) = create_cargo_pnpm_monorepo_fixture();
    let json = run_repo_structure_json(&path);
    insta::assert_json_snapshot!(
        "cargo_pnpm_monorepo_structure_json",
        stable_topology_json(&json)
    );
}

#[test]
fn pnpm_nx_monorepo_structure_text_snapshot() {
    let (_dir, path) = create_pnpm_nx_monorepo_fixture();
    let stdout = run_repo_structure(&path);
    insta::assert_snapshot!(
        "pnpm_nx_monorepo_structure_text",
        normalize_snapshot_paths(&stdout, &path)
    );
}

#[test]
fn pnpm_nx_monorepo_structure_json_snapshot() {
    let (_dir, path) = create_pnpm_nx_monorepo_fixture();
    let json = run_repo_structure_json(&path);
    insta::assert_json_snapshot!(
        "pnpm_nx_monorepo_structure_json",
        stable_topology_json(&json)
    );
}

#[test]
fn degenerate_cargo_structure_text_is_empty() {
    let (_dir, path) = create_degenerate_cargo_fixture();
    let stdout = run_repo_structure(&path);
    insta::assert_snapshot!("degenerate_cargo_structure_text", stdout);
}

#[test]
fn degenerate_cargo_structure_json_has_no_topology_keys() {
    let (_dir, path) = create_degenerate_cargo_fixture();
    let json = run_repo_structure_json(&path);
    assert!(json.as_object().is_none_or(|o| o.is_empty()));
    assert!(json.get("monorepo_standards").is_none());
    assert!(json.get("monorepo_layers").is_none());
    assert!(json.get("monorepo_tool").is_none());
    assert!(json.get("workspace_tools").is_none());
}

// ============================================================================
// Bare `sniff repo --json` aggregate golden (perf feature, Phase 2 / Lane A)
// ============================================================================

/// A repository with one file in each working-tree scope, so the golden covers
/// every `ScopeBucket` the aggregate projects from `GitInfo.file_changes`.
fn create_aggregate_fixture() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let repo = init_git_repo(dir.path());

    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[workspace]
resolver = "2"
members = ["area-a/alpha", "area-b/beta"]

[workspace.package]
version = "0.3.1"
"#,
    )
    .unwrap();

    for (area, name) in [("area-a", "alpha"), ("area-b", "beta")] {
        let pkg = dir.path().join(area).join(name);
        std::fs::create_dir_all(pkg.join("src")).unwrap();
        std::fs::write(
            pkg.join("Cargo.toml"),
            format!(
                r#"[package]
name = "{name}"
version.workspace = true
edition = "2021"

[dependencies]
serde = "1"
"#
            ),
        )
        .unwrap();
        std::fs::write(pkg.join("src/lib.rs"), "pub fn x() {}\n").unwrap();
    }
    std::fs::create_dir_all(dir.path().join("docs")).unwrap();
    std::fs::write(dir.path().join("docs/guide.md"), "# Guide\n").unwrap();
    commit_all(&repo, "initial aggregate fixture");

    // A remote pins `name` without depending on the temp directory's basename.
    repo.remote("origin", "https://github.com/example/agg-fixture.git")
        .unwrap();

    // unstaged, staged, and untracked, respectively.
    std::fs::write(
        dir.path().join("area-a/alpha/src/lib.rs"),
        "pub fn x() {}\npub fn x2() {}\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("area-b/beta/src/lib.rs"),
        "pub fn x() {}\npub fn y2() {}\n",
    )
    .unwrap();
    let mut index = repo.index().unwrap();
    index
        .add_path(std::path::Path::new("area-b/beta/src/lib.rs"))
        .unwrap();
    index.write().unwrap();
    std::fs::write(dir.path().join("docs/new.md"), "# New\n").unwrap();

    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Reduce the aggregate to the parts that are stable across hosts and runs.
///
/// Dropped, and why — each would make the golden assert the host rather than
/// the projection:
///
/// - `git_status.config` reads the host's global gitconfig.
/// - `branches[].sha`, `recent_commits`, and the two commit-change families
///   carry commit ids and timestamps.
/// - `worktrees[].path`, `root`, and `structure.root` are temp paths.
/// - `git_status.file_changes` order is *not* deterministic: the gix status
///   walk is parallel, which reproduces on clean HEAD. Sorted here rather than
///   dropped, since the entries themselves are contract.
fn stable_aggregate_json(json: &Value) -> Value {
    let mut file_changes = json["git_status"]["file_changes"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    file_changes.sort_by_key(|c| c["path"].as_str().unwrap_or_default().to_string());

    let worktrees = json["worktrees"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .map(|e| {
                    json!({
                        "branch": e["branch"],
                        "current": e["current"],
                        "detached": e["detached"],
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let branches = json["branches"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .map(|b| {
                    json!({
                        "name": b["name"],
                        "current": b["current"],
                        "upstream": b["upstream"],
                        "ahead": b["ahead"],
                        "behind": b["behind"],
                        "remote_represented": b["remote_represented"],
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // The detected standard binary resolves through the host's PATH, so its
    // path asserts the host, not the projection.
    let monorepo_standards = match json["structure"]["monorepo_standards"].as_array() {
        Some(entries) => entries
            .iter()
            .map(|s| {
                let mut entry = s.clone();
                if let Some(binary) = entry.get_mut("binary") {
                    if binary.get("path").is_some_and(Value::is_string) {
                        binary["path"] = json!("<normalized>");
                    }
                }
                entry
            })
            .collect::<Vec<_>>()
            .into(),
        None => json["structure"]["monorepo_standards"].clone(),
    };

    json!({
        "name": json["name"],
        "version": json["version"],
        "language": json["language"],
        "is_monorepo": json["is_monorepo"],
        "package_count": json["package_count"],
        "packages": json["packages"],
        "package_areas": json["package_areas"],
        "package_manager": json["package_manager"],
        "test_runner": json["test_runner"],
        "package_dependencies": json["package_dependencies"],
        "dependencies": json["dependencies"],
        "structure": {
            "is_monorepo": json["structure"]["is_monorepo"],
            "monorepo_standards": monorepo_standards,
            "monorepo_layers": json["structure"]["monorepo_layers"],
        },
        "git_status": {
            "current_branch": json["git_status"]["current_branch"],
            "file_changes": file_changes,
            "is_dirty": json["git_status"]["is_dirty"],
            "staged_count": json["git_status"]["staged_count"],
            "unstaged_count": json["git_status"]["unstaged_count"],
            "untracked_count": json["git_status"]["untracked_count"],
        },
        "branches": branches,
        "worktrees": worktrees,
        "context": {
            "package": json["context"]["package"],
            "package_area": json["context"]["package_area"],
            "area": json["context"]["area"],
            "worktree": json["context"]["worktree"],
            "is_current_package_area_dirty": json["context"]["is_current_package_area_dirty"],
            "package_area_has_source_code_changes":
                json["context"]["package_area_has_source_code_changes"],
        },
        "dirty": json["dirty"],
        "staged": json["staged"],
        "unstaged": json["unstaged"],
        "untracked": json["untracked"],
        "has_merge_conflict": json["has_merge_conflict"],
        "commit_family_keys": {
            "recent_commits": json["recent_commits"]["period"]["label"],
            "source_code_changes": json["source_code_changes"]["filter"],
            "documentation_changes": json["documentation_changes"]["filter"],
        },
    })
}

/// Replace every occurrence of the fixture root with `[BASE]`.
///
/// Both the given and the canonicalized form are replaced: on macOS a temp dir
/// handed out as `/var/...` is reported back through its `/private/var/...`
/// realpath, so replacing only one leaves absolute paths in the snapshot. Path
/// separators after the replacement are normalized for cross-platform output.
fn redact_base_paths(value: &Value, base: &std::path::Path) -> Value {
    let mut redacted = value.clone();
    let mut roots = vec![base.to_path_buf()];
    if let Ok(canonical) = std::fs::canonicalize(base) {
        roots.push(canonical);
    }
    // Longest first: a prefix would otherwise mask the form that contains it.
    roots.sort_by_key(|p| std::cmp::Reverse(p.as_os_str().len()));

    fn redact_strings(value: &mut Value, roots: &[std::path::PathBuf]) {
        match value {
            Value::String(text) => {
                for root in roots {
                    *text = text.replace(root.to_str().expect("temp path is utf8"), "[BASE]");
                }
                if text.contains("[BASE]") {
                    *text = text.replace('\\', "/");
                }
            }
            Value::Array(values) => {
                for value in values {
                    redact_strings(value, roots);
                }
            }
            Value::Object(values) => {
                for value in values.values_mut() {
                    redact_strings(value, roots);
                }
            }
            _ => {}
        }
    }

    redact_strings(&mut redacted, &roots);
    redacted
}

fn run_repo_aggregate_json(base: &std::path::Path) -> Value {
    let output = assert_cmd::Command::cargo_bin("sniff")
        .unwrap()
        .args(["--base", base.to_str().unwrap(), "repo", "--json"])
        .env("NO_COLOR", "1")
        .output()
        .expect("run sniff repo --json");

    assert!(
        output.status.success(),
        "repo --json must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "repo --json must not emit stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("repo --json stdout must be valid JSON")
}

/// The bare `sniff repo --json` contract, captured from the pre-Phase-2 builder
/// and unchanged by rewriting it into a pure projection.
///
/// This is the regression gate for umbrella spec R2.5: Phase 2 removed nine
/// observations from this path (one identity detection, one repository-root
/// lookup pair, worktree/branch/conflict/current-worktree queries, and eight
/// `collect_changed_paths` status walks) and every key, value, and ordering
/// below must survive that.
#[test]
fn repo_aggregate_json_snapshot() {
    let (_dir, path) = create_aggregate_fixture();
    let json = run_repo_aggregate_json(&path);
    let stable = redact_base_paths(&stable_aggregate_json(&json), &path);
    insta::assert_json_snapshot!("repo_aggregate_json", stable);
}
