use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::{Value, json};

fn run_stdout(args: &[&str]) -> String {
    let output = cargo_bin_cmd!("sniff")
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
    let manager_count = json
        .get("system_package_managers")
        .and_then(|v| v.get("managers"))
        .and_then(Value::as_array)
        .map_or(0, Vec::len);

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
        "manager_count": manager_count
    })
}

#[test]
fn help_output_snapshot() {
    insta::assert_snapshot!("help_output", normalize_text(&run_stdout(&["--help"])));
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
    let repo = git2::Repository::init(path).unwrap();
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
    let output = cargo_bin_cmd!("sniff")
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
    let output = cargo_bin_cmd!("sniff")
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
