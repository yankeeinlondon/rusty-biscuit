use std::path::{Path, PathBuf};

use predicates::prelude::*;
use serde_json::Value;

// ============================================================================
// Help and Version Tests
// ============================================================================

#[test]
fn test_help_flag() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Detect system"));
}

#[test]
fn test_version_flag() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("sniff"));
}

#[test]
fn test_help_mentions_subcommands() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("sniff os"))
        .stdout(predicate::str::contains("sniff cpu"))
        .stdout(predicate::str::contains("sniff hardware"))
        .stdout(predicate::str::contains("sniff software"));
}

// ============================================================================
// Shell Completions Tests
// ============================================================================

#[test]
fn test_completions_bash_shows_setup() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("source <(COMPLETE=bash sniff)"))
        .stdout(predicate::str::contains("~/.bashrc"));
}

#[test]
fn test_completions_zsh_shows_setup() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("source <(COMPLETE=zsh sniff)"))
        .stdout(predicate::str::contains("~/.zshrc"));
}

#[test]
fn test_completions_fish_shows_setup() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("COMPLETE=fish sniff | source"))
        .stdout(predicate::str::contains("config.fish"));
}

#[test]
fn test_completions_powershell_shows_setup() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--completions", "powershell"])
        .assert()
        .success()
        .stdout(predicate::str::contains("$env:COMPLETE"))
        .stdout(predicate::str::contains("$PROFILE"));
}

#[test]
fn test_dynamic_completions_bash() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .env("COMPLETE", "bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("_clap_complete_sniff"))
        .stdout(predicate::str::contains("COMPREPLY"));
}

#[test]
fn test_dynamic_completions_zsh() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .env("COMPLETE", "zsh")
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef sniff"))
        .stdout(predicate::str::contains("_clap_dynamic_completer_sniff"));
}

#[test]
fn test_dynamic_completions_fish() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .env("COMPLETE", "fish")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "complete --keep-order --exclusive --command sniff",
        ));
}

#[test]
fn test_help_mentions_completions() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--completions").not())
        .stdout(predicate::str::contains("Shell completions").not());
}

#[test]
fn test_completions_help_flag_shows_setup() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--completions", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Shell completions"))
        .stdout(predicate::str::contains("sniff --completions"))
        .stdout(predicate::str::contains("COMPLETE=bash sniff"));
}

// ============================================================================
// Output Mode Tests
// No subcommand = show help
// No subcommand + --json = JSON output (all data)
// With subcommand = text output by default, --json for JSON
// ============================================================================

#[test]
fn test_no_subcommand_shows_help() {
    // Without a subcommand, the output should be the help text
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .assert()
        .success()
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("sniff os"));
}

#[test]
fn test_no_subcommand_with_json_outputs_json() {
    // Without a subcommand but with --json, the output should be JSON
    let (_dir, path) = create_test_repo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .current_dir(&path)
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"hardware\""))
        .stdout(predicate::str::contains("\"os\""));
}

#[test]
fn test_subcommand_outputs_text_by_default() {
    // With a subcommand (os), the output should be text by default
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("os")
        .assert()
        .success()
        .stdout(predicate::str::contains("Operating System"));
}

#[test]
fn test_subcommand_with_json_flag_outputs_json() {
    // With a subcommand and --json, output should be JSON
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["os", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("\"kernel\""));
}

// ============================================================================
// Global Flag Position Tests
// Global flags should work before or after subcommand
// ============================================================================

#[test]
fn test_json_flag_before_subcommand() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--json", "cpu"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"brand\""))
        .stdout(predicate::str::contains("\"logical_cores\""));
}

#[test]
fn test_json_flag_after_subcommand() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["cpu", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"brand\""))
        .stdout(predicate::str::contains("\"logical_cores\""));
}

#[test]
fn test_verbose_flag_before_subcommand() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["-v", "cpu"])
        .assert()
        .success()
        .stdout(predicate::str::contains("=== CPU ==="));
}

#[test]
fn test_verbose_flag_after_subcommand() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["cpu", "-v"])
        .assert()
        .success()
        .stdout(predicate::str::contains("=== CPU ==="));
}

#[test]
fn test_double_verbose_flag() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["cpu", "-vv"])
        .assert()
        .success();
}

#[test]
fn with_network_global_flag_is_rejected() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--with-network", "repo", "name"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--with-network"));
}

#[test]
fn with_network_subcommand_flag_is_rejected() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "name", "--with-network"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--with-network"));
}

#[test]
fn repo_name_json_is_leaf_only() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "name", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).expect("utf8");
    let json: serde_json::Value = serde_json::from_str(json_str).expect("valid json");

    let obj = json.as_object().expect("json should be an object");

    // The only key allowed is `name`. No version, language, is_monorepo,
    // or package_count may appear at the leaf level.
    assert_eq!(
        obj.len(),
        1,
        "repo name --json must contain exactly one key; got: {json}"
    );
    assert!(
        obj.contains_key("name"),
        "repo name --json must contain `name`: {json}"
    );
    assert!(
        obj.get("name").and_then(|v| v.as_str()).is_some(),
        "`name` must be a string: {json}"
    );

    for forbidden in ["version", "language", "is_monorepo", "package_count"] {
        assert!(
            !obj.contains_key(forbidden),
            "repo name --json must NOT contain `{forbidden}`: {json}"
        );
    }
}

// ============================================================================
// `sniff repo --json` aggregate tests (scope-complete-json plan, Phase 2)
// ============================================================================

fn repo_aggregate_json_output() -> std::process::Output {
    let (_dir, path) = create_test_repo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .current_dir(path)
        .args(["repo", "--json"])
        .output()
        .expect("run sniff repo --json")
}

#[test]
fn repo_aggregate_json_is_valid_object() {
    let output = repo_aggregate_json_output();

    assert!(
        output.status.success(),
        "sniff repo --json must succeed: stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json_str = std::str::from_utf8(&output.stdout).expect("utf8");
    let json: serde_json::Value = serde_json::from_str(json_str).expect("valid json");
    assert!(
        json.is_object(),
        "repo --json aggregate must be an object: {json}"
    );
}

#[test]
fn repo_aggregate_perf_covers_complete_command() {
    let (_dir, path) = create_cli_monorepo();
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "--perf",
            "--plain",
            "repo",
            "--json",
        ])
        .output()
        .expect("run sniff repo --json --perf");

    assert!(
        output.status.success(),
        "aggregate command must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("valid aggregate JSON");
    let report = &value["performance"];
    let counters = report["counters"].as_object().expect("performance counters");
    let stages = report["stages"].as_object().expect("performance stages");

    for (counter, expected) in [
        ("git.repository_discoveries", 1),
        ("git.status_walks", 1),
        ("git.ref_walks", 1),
        ("git.worktree_opens", 0),
    ] {
        assert_eq!(
            counters.get(counter).and_then(Value::as_u64).unwrap_or(0),
            expected,
            "complete aggregate command counter `{counter}`: {counters:?}"
        );
    }

    let aggregate_stage = &stages["cli.repo.aggregate_projection"];
    assert_eq!(aggregate_stage["calls"], 1);
    let aggregate_ms = aggregate_stage["total_duration_ms"]
        .as_f64()
        .expect("aggregate stage duration");
    let detect_ms = stages["detect.total"]["total_duration_ms"]
        .as_f64()
        .expect("detection stage duration");
    let total_ms = report["total_duration_ms"]
        .as_f64()
        .expect("complete command duration");
    assert!(
        total_ms >= detect_ms + aggregate_ms,
        "complete elapsed time must cover detection plus aggregate projection: \
         total={total_ms}, detection={detect_ms}, aggregate={aggregate_ms}"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cli.repo.aggregate_projection"),
        "stderr report must include post-detection aggregate projection: {stderr}"
    );
    assert!(
        stderr.contains("git.repository_discoveries: 1")
            && stderr.contains("git.status_walks: 1"),
        "stderr report must include complete command-wide bounds: {stderr}"
    );
}

#[test]
fn repo_aggregate_json_excludes_network_and_parameterized_keys() {
    let output = repo_aggregate_json_output();

    let json_str = std::str::from_utf8(&output.stdout).expect("utf8");
    let json: serde_json::Value = serde_json::from_str(json_str).expect("valid json");
    let obj = json.as_object().expect("aggregate object");

    for forbidden in ["remote", "pr", "hash"] {
        assert!(
            !obj.contains_key(forbidden),
            "aggregate must not contain network/parameterized key `{forbidden}`: {json}"
        );
    }
}

#[test]
fn repo_aggregate_json_not_partial() {
    let output = repo_aggregate_json_output();

    let json_str = std::str::from_utf8(&output.stdout).expect("utf8");
    let json: serde_json::Value = serde_json::from_str(json_str).expect("valid json");
    let obj = json.as_object().expect("aggregate object");

    let expected = [
        "name",
        "version",
        "language",
        "is_monorepo",
        "package_count",
        "root",
        "structure",
        "packages",
        "package_areas",
        "package_manager",
        "test_runner",
        "package_dependencies",
        "dependencies",
        "git_status",
        "branches",
        "worktrees",
        "context",
        "dirty",
        "staged",
        "unstaged",
        "untracked",
        "has_merge_conflict",
        "recent_commits",
        "source_code_changes",
        "documentation_changes",
    ];

    for key in expected {
        assert!(
            obj.contains_key(key),
            "aggregate must contain participating key `{key}`: {json}"
        );
    }
}

#[test]
fn repo_aggregate_json_uses_snake_case_and_drops_old_kebab_keys() {
    let output = repo_aggregate_json_output();

    let json_str = std::str::from_utf8(&output.stdout).expect("utf8");
    let json: serde_json::Value = serde_json::from_str(json_str).expect("valid json");
    let obj = json.as_object().expect("aggregate object");

    for key in obj.keys() {
        assert!(
            !key.contains('-'),
            "aggregate top-level key must be snake_case, got `{key}`: {json}"
        );
    }

    for old_key in [
        "is-monorepo",
        "package-count",
        "package-areas",
        "package-dependencies",
        "git-status",
        "staged-files",
        "unstaged-files",
        "untracked-files",
        "dirty-source-code",
        "staged-source-code",
        "unstaged-source-code",
        "dirty-files",
        "package-area",
        "package-root",
        "package-area-root",
        "is-current-package-area-dirty",
        "package-area-has-source-code-changes",
        "has-merge-conflict",
        "recent-commits",
        "source-code-changes",
        "documentation-changes",
    ] {
        assert!(
            !obj.contains_key(old_key),
            "aggregate must not contain old kebab-case key `{old_key}`: {json}"
        );
    }
}

#[test]
fn repo_aggregate_json_context_groups_cwd_relative_facts() {
    let output = repo_aggregate_json_output();

    let json_str = std::str::from_utf8(&output.stdout).expect("utf8");
    let json: serde_json::Value = serde_json::from_str(json_str).expect("valid json");

    // Identity leaves are unwrapped values, not nested objects.
    assert!(json["name"].is_string());
    assert!(json["is_monorepo"].is_boolean());
    assert!(
        json["package_count"].is_number(),
        "package_count must be unwrapped number: {json}"
    );

    assert!(json["root"].is_string());

    let context = json["context"].as_object().expect("context object");
    assert!(context["package"].is_string());
    assert!(context["package_area"].is_string());
    assert!(context["area"].is_string());
    assert!(context["package_root"].is_string());
    assert!(context["package_area_root"].is_string());
    assert!(context["worktree"].is_string() || context["worktree"].is_null());
    assert!(context["is_current_package_area_dirty"].is_boolean());
    assert!(context["package_area_has_source_code_changes"].is_boolean());
    assert!(json["has_merge_conflict"].is_boolean());
}

#[test]
fn repo_json_output_is_valid_json_on_stdout_with_clean_stderr() {
    let output = repo_aggregate_json_output();

    assert!(
        output.status.success(),
        "sniff repo --json must succeed: stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = std::str::from_utf8(&output.stdout).expect("stdout should be UTF-8");
    let _: serde_json::Value =
        serde_json::from_str(stdout).expect("repo --json stdout must be valid JSON");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.trim().is_empty(),
        "repo --json must not emit diagnostics on stderr: {stderr}"
    );
}

/// `--json` stdout must be exactly one document, not a document followed by
/// anything else. A trailing render or perf block would still let a lenient
/// `from_str` succeed on some inputs, so this asserts the stream is fully
/// consumed by the single value.
#[test]
fn repo_json_stdout_is_exactly_one_json_document() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "--json"])
        .output()
        .expect("run sniff repo --json");

    assert!(output.status.success(), "sniff repo --json must succeed");

    let stdout = std::str::from_utf8(&output.stdout).expect("stdout should be UTF-8");
    let mut stream =
        serde_json::Deserializer::from_str(stdout).into_iter::<serde_json::Value>();

    let first = stream
        .next()
        .expect("stdout must carry a JSON document")
        .expect("that document must be valid JSON");
    assert!(first.is_object(), "the aggregate must be a JSON object");
    assert!(
        stream.next().is_none(),
        "stdout must carry exactly one JSON document, found trailing content"
    );
}

/// `--json` must survive a **shallow** repository, because that is the normal CI
/// checkout: `actions/checkout@v4` fetches depth 1 by default.
///
/// Owns the repository it asserts against, unlike
/// `repo_json_stdout_is_exactly_one_json_document` above, which runs in the
/// ambient checkout. That one passed on every developer machine and failed on
/// every runner — a shallow clone's HEAD names a parent the object database does
/// not contain, and resolving it aborted the aggregate with "An object with id …
/// could not be found". A test that can only fail on CI cannot drive a fix.
#[test]
fn repo_json_succeeds_in_a_shallow_clone() {
    fn git(args: &[&str], cwd: &std::path::Path) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap_or_else(|e| panic!("git {args:?}: {e}"));
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let root = tempfile::tempdir().unwrap();
    let origin = root.path().join("origin");
    std::fs::create_dir(&origin).unwrap();

    // `-b main` explicitly: an ambient `init.defaultBranch` would otherwise
    // decide the branch this fixture clones.
    git(&["init", "-b", "main"], &origin);
    git(&["config", "user.email", "test@example.com"], &origin);
    git(&["config", "user.name", "Test"], &origin);

    // Two commits, so a depth-1 clone genuinely has an unreachable parent. One
    // commit would make HEAD a root and never exercise the boundary.
    for n in 1..=2 {
        std::fs::write(origin.join(format!("file{n}.txt")), format!("contents {n}"))
            .expect("write fixture file");
        git(&["add", "."], &origin);
        git(&["commit", "-m", &format!("commit {n}")], &origin);
    }

    let shallow = root.path().join("shallow");
    // A `file://` URL is required: git silently ignores `--depth` for a plain
    // local path clone, which would make this test vacuous.
    git(
        &[
            "clone",
            "--depth",
            "1",
            "--single-branch",
            "--branch",
            "main",
            &format!("file://{}", origin.display()),
            shallow.to_str().expect("utf8 path"),
        ],
        root.path(),
    );
    assert!(
        shallow.join(".git/shallow").exists(),
        "fixture must actually be shallow, or this test proves nothing"
    );

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "--json"])
        .current_dir(&shallow)
        .output()
        .expect("run sniff repo --json in a shallow clone");

    assert!(
        output.status.success(),
        "sniff repo --json must succeed in a shallow clone: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = std::str::from_utf8(&output.stdout).expect("stdout should be UTF-8");
    let value: serde_json::Value =
        serde_json::from_str(stdout).expect("shallow-clone stdout must be valid JSON");
    assert!(value.is_object(), "the aggregate must be a JSON object");
}

/// Bare `sniff repo` renders text and `--plain` renders plain text; neither
/// goes through the `--json` aggregate. Pinned so the aggregate rewrite cannot
/// leak JSON into, or diagnostics out of, the human-facing paths.
#[test]
fn repo_default_and_plain_emit_text_with_clean_stderr() {
    for args in [vec!["repo"], vec!["repo", "--plain"]] {
        let label = args.join(" ");
        let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
            .args(&args)
            .output()
            .unwrap_or_else(|e| panic!("run sniff {label}: {e}"));

        assert!(output.status.success(), "sniff {label} must succeed");

        let stdout = std::str::from_utf8(&output.stdout).expect("stdout should be UTF-8");
        assert!(!stdout.trim().is_empty(), "sniff {label} must render output");
        assert!(
            serde_json::from_str::<serde_json::Value>(stdout).is_err(),
            "sniff {label} must render text, not JSON"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.trim().is_empty(),
            "sniff {label} must not emit diagnostics on stderr: {stderr}"
        );
    }
}

#[test]
fn repo_structure_json_output_is_valid_json_on_stdout_with_clean_stderr() {
    let (_dir, path) = create_test_repo();
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .current_dir(&path)
        .args(["repo", "structure", "--json"])
        .output()
        .expect("run sniff repo structure --json");

    assert!(
        output.status.success(),
        "sniff repo structure --json must succeed: stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = std::str::from_utf8(&output.stdout).expect("stdout should be UTF-8");
    let _: serde_json::Value =
        serde_json::from_str(stdout).expect("repo structure --json stdout must be valid JSON");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.trim().is_empty(),
        "repo structure --json must not emit diagnostics on stderr: {stderr}"
    );
}

#[test]
fn repo_aggregate_json_scope_buckets_have_stable_shape() {
    let output = repo_aggregate_json_output();

    let json_str = std::str::from_utf8(&output.stdout).expect("utf8");
    let json: serde_json::Value = serde_json::from_str(json_str).expect("valid json");

    for key in ["dirty", "staged", "unstaged", "untracked"] {
        let leaf = &json[key];
        assert!(
            leaf.is_object(),
            "{key} must be an object in aggregate: {json}"
        );
        for field in ["files", "source_code", "documentation", "packages", "package_areas"] {
            assert!(
                leaf[field].is_array(),
                "{key}.{field} must be an array: {leaf}"
            );
        }
    }
}

#[test]
fn repo_aggregate_json_does_not_duplicate_full_package_catalogs() {
    let output = repo_aggregate_json_output();

    let json_str = std::str::from_utf8(&output.stdout).expect("utf8");
    let json: serde_json::Value = serde_json::from_str(json_str).expect("valid json");

    assert!(json["packages"].is_array(), "top-level packages: {json}");
    assert!(
        json["structure"].get("packages").is_none(),
        "structure must not duplicate the full package catalog: {json}"
    );
    assert!(
        json["recent_commits"].get("packages").is_none(),
        "recent_commits must not duplicate the full package catalog: {json}"
    );
    assert!(
        json["package_dependencies"]["packages"].is_array(),
        "package_dependencies keeps only its narrow dependency projection: {json}"
    );
}

/// Generalized form of [`repo_aggregate_json_does_not_duplicate_full_package_catalogs`]:
/// the full package catalog may live only in its designated homes; no other
/// section anywhere in the tree may re-embed it. Walks the whole document
/// rather than two named keys, so a *new* section that starts serializing full
/// `Package` objects is caught. Replaces the former absolute-byte size check,
/// which tracked git-history growth rather than catalog duplication.
#[test]
fn repo_aggregate_json_never_re_embeds_the_full_package_catalog() {
    let output = repo_aggregate_json_output();
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid json");

    // Dotted paths permitted to carry full-catalog entries. Every other section
    // must reference packages by a slim summary or a narrow projection.
    const ALLOWED: &[&str] = &["packages", "package_dependencies.packages"];

    // The heavy structural field `package_area` is emitted only by full
    // `Package` serialization — the slim top-level summary (strings), the narrow
    // dependency projection, and the git-status change buckets all omit it.
    fn is_full_catalog_entry(v: &serde_json::Value) -> bool {
        v.get("package_area").is_some()
    }

    fn walk(v: &serde_json::Value, path: &str, offenders: &mut Vec<String>) {
        match v {
            serde_json::Value::Array(items) => {
                if items.iter().any(is_full_catalog_entry) && !ALLOWED.contains(&path) {
                    offenders.push(path.to_string());
                }
                for item in items {
                    walk(item, path, offenders);
                }
            }
            serde_json::Value::Object(map) => {
                for (k, child) in map {
                    let next = if path.is_empty() {
                        k.clone()
                    } else {
                        format!("{path}.{k}")
                    };
                    walk(child, &next, offenders);
                }
            }
            _ => {}
        }
    }

    let mut offenders = Vec::new();
    walk(&json, "", &mut offenders);
    assert!(
        offenders.is_empty(),
        "sections re-embed the full package catalog (entries carrying `package_area`); \
         the catalog belongs only at {ALLOWED:?}, found at: {offenders:?}"
    );
}

#[test]
fn repo_aggregate_json_is_offline() {
    // The aggregate must not trigger a network call. We verify this indirectly
    // by ensuring the output completes successfully and does not contain the
    // excluded network-primary keys; the spec excludes `remote`, `pr`, and
    // `hash` from the aggregate.
    let output = repo_aggregate_json_output();

    assert!(
        output.status.success(),
        "offline aggregate must succeed: stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json_str = std::str::from_utf8(&output.stdout).expect("utf8");
    let json: serde_json::Value = serde_json::from_str(json_str).expect("valid json");
    let obj = json.as_object().expect("aggregate object");
    assert!(!obj.contains_key("remote"));
    assert!(!obj.contains_key("pr"));
    assert!(!obj.contains_key("hash"));
}

#[test]
fn repo_name_json_is_still_leaf_only() {
    // Regression guard: `sniff repo name --json` must remain a single-key leaf
    // even after the bare `repo --json` aggregate landed.
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "name", "--json"])
        .output()
        .expect("run sniff repo name --json");

    assert!(output.status.success());
    let json_str = std::str::from_utf8(&output.stdout).expect("utf8");
    let json: serde_json::Value = serde_json::from_str(json_str).expect("valid json");
    let obj = json.as_object().expect("object");
    assert_eq!(
        obj.len(),
        1,
        "repo name --json must remain leaf-only: {json}"
    );
    assert!(obj.contains_key("name"));
}

// ============================================================================
// `repo is-monorepo` / `package-count` / `version` leaf end-to-end tests
// (scope-complete-json plan — single-key public leaves)
// ============================================================================

#[test]
fn repo_is_monorepo_json_emits_object() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "is-monorepo", "--json"])
        .output()
        .expect("run sniff repo is-monorepo --json");

    assert!(
        output.status.success(),
        "repo is-monorepo --json must exit 0: stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value =
        serde_json::from_str(std::str::from_utf8(&output.stdout).expect("utf8")).expect("json");
    let obj = json.as_object().expect("object");
    assert!(
        obj["is_monorepo"].as_bool().unwrap_or(false),
        "is-monorepo --json must report a monorepo: {json}"
    );
    assert!(
        obj["authority"].as_str().is_some(),
        "is-monorepo --json must include authority: {json}"
    );
    if obj.contains_key("orchestrators") {
        assert!(
            obj["orchestrators"].is_array(),
            "orchestrators must be an array when present: {json}"
        );
    }
}

#[test]
fn repo_is_monorepo_text_prints_label() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "is-monorepo"])
        .env("NO_COLOR", "1")
        .output()
        .expect("run sniff repo is-monorepo");

    assert!(output.status.success());
    let stdout = std::str::from_utf8(&output.stdout).expect("utf8").trim();
    assert!(
        stdout != "yes" && stdout != "no",
        "repo is-monorepo text must no longer be `yes`/`no`: {stdout:?}"
    );
    assert!(
        !stdout.is_empty(),
        "repo is-monorepo text must print the monorepo label: {stdout:?}"
    );
}

#[test]
fn repo_is_monorepo_no_error_exits_zero_when_false() {
    let dir = tempfile::tempdir().unwrap();
    let git_init = std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .expect("git init");
    assert!(git_init.status.success(), "git init failed");

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "is-monorepo", "--no-error"])
        .current_dir(dir.path())
        .env("NO_COLOR", "1")
        .output()
        .expect("run sniff repo is-monorepo --no-error in non-monorepo");

    assert!(
        output.status.success(),
        "repo is-monorepo --no-error in a non-monorepo must exit 0: stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = std::str::from_utf8(&output.stdout).expect("utf8").trim();
    assert_eq!(
        stdout, "false",
        "--no-error in a non-monorepo must print false"
    );
}

#[test]
fn repo_is_monorepo_text_in_monorepo_exits_zero_with_label() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "is-monorepo"])
        .env("NO_COLOR", "1")
        .output()
        .expect("run sniff repo is-monorepo");

    assert!(
        output.status.success(),
        "repo is-monorepo in a monorepo must exit 0: stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = std::str::from_utf8(&output.stdout).expect("utf8").trim();
    assert_eq!(
        stdout, "cargo",
        "repo is-monorepo text must print the unified label `cargo`: {stdout:?}"
    );
    assert!(
        output.stderr.is_empty(),
        "repo is-monorepo in a monorepo must not emit stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn repo_is_monorepo_text_in_non_monorepo_exits_nonzero_with_false() {
    let dir = tempfile::tempdir().unwrap();
    let git_init = std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .expect("git init");
    assert!(git_init.status.success(), "git init failed");

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "is-monorepo"])
        .current_dir(dir.path())
        .env("NO_COLOR", "1")
        .output()
        .expect("run sniff repo is-monorepo in non-monorepo");

    assert!(
        !output.status.success(),
        "repo is-monorepo in a non-monorepo must exit non-zero"
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "repo is-monorepo in a non-monorepo must exit 1"
    );
    let stdout = std::str::from_utf8(&output.stdout).expect("utf8").trim();
    assert_eq!(stdout, "false");
    assert!(
        output.stderr.is_empty(),
        "predicate failure must not emit stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn repo_is_monorepo_json_in_non_monorepo_exits_nonzero_with_valid_json() {
    let dir = tempfile::tempdir().unwrap();
    let git_init = std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .expect("git init");
    assert!(git_init.status.success(), "git init failed");

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "is-monorepo", "--json"])
        .current_dir(dir.path())
        .env("NO_COLOR", "1")
        .output()
        .expect("run sniff repo is-monorepo --json in non-monorepo");

    assert!(
        !output.status.success(),
        "repo is-monorepo --json in a non-monorepo must exit non-zero"
    );
    assert_eq!(output.status.code(), Some(1));

    let json: Value =
        serde_json::from_str(std::str::from_utf8(&output.stdout).expect("utf8")).expect("json");
    assert_eq!(json, serde_json::json!({ "is_monorepo": false }));
    assert!(
        output.stderr.is_empty(),
        "predicate failure must not emit stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn repo_is_monorepo_genuine_failure_exits_nonzero_with_stderr_even_with_no_error() {
    let dir = tempfile::tempdir().unwrap();
    // Not a git repository — genuine failure path.

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "is-monorepo", "--no-error"])
        .current_dir(dir.path())
        .env("NO_COLOR", "1")
        .output()
        .expect("run sniff repo is-monorepo --no-error outside a repo");

    assert!(
        !output.status.success(),
        "genuine failure must exit non-zero even with --no-error"
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "genuine failure must exit 1 even with --no-error"
    );
    assert!(
        output.stdout.is_empty(),
        "genuine failure must not emit stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Not a git repository"),
        "genuine failure must report to stderr: {stderr}"
    );
}

#[test]
fn repo_package_count_json_is_single_key_number() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "package-count", "--json"])
        .output()
        .expect("run sniff repo package-count --json");

    assert!(
        output.status.success(),
        "repo package-count --json must exit 0: stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value =
        serde_json::from_str(std::str::from_utf8(&output.stdout).expect("utf8")).expect("json");
    let obj = json.as_object().expect("object");
    assert_eq!(
        obj.len(),
        1,
        "package-count --json must be a single key: {json}"
    );
    assert!(
        obj["package-count"].is_u64() || obj["package-count"].is_i64(),
        "package-count value must be an integer (kebab-case key): {json}"
    );
}

#[test]
fn repo_package_count_text_is_integer() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "package-count"])
        .env("NO_COLOR", "1")
        .output()
        .expect("run sniff repo package-count");

    assert!(output.status.success());
    let stdout = std::str::from_utf8(&output.stdout).expect("utf8").trim();
    assert!(
        stdout.parse::<u64>().is_ok(),
        "repo package-count text must be an integer: {stdout:?}"
    );
}

#[test]
fn repo_version_json_returns_array_shape_under_real_repo() {
    // The repo under test is the rusty-biscuit monorepo (a real Cargo
    // workspace with packages). `sniff repo version --json` must report
    // the new `{ "versions": [...] }` contract — never the legacy
    // `{ "version": ... }` single-key shape.
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "version", "--json"])
        .output()
        .expect("run sniff repo version --json");

    assert!(
        output.status.success(),
        "version --json must exit 0 for the real repo: stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value =
        serde_json::from_str(std::str::from_utf8(&output.stdout).expect("utf8"))
            .expect("stdout is valid JSON");
    let obj = json.as_object().expect("JSON object at the top level");
    assert!(
        obj.contains_key("versions"),
        "version --json must surface the `versions` array, got {json}"
    );
    let versions = obj["versions"].as_array().expect("`versions` is an array");
    assert!(!versions.is_empty(), "real repo should report at least one version");
    for entry in versions {
        let entry_obj = entry.as_object().expect("entry is an object");
        assert!(entry_obj.contains_key("version"));
        assert!(entry_obj.contains_key("packages"));
        assert!(entry_obj.contains_key("sources"));
    }
}

#[test]
fn repo_version_json_no_error_exits_zero() {
    // The real repo always has at least one resolvable version, so
    // `--no-error` is exercised on the success path. The empty
    // `--no-error` behaviour is covered by integration tests in the
    // `repo_version_empty_with_no_error` family.
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "version", "--json", "--no-error"])
        .output()
        .expect("run sniff repo version --json --no-error");

    assert!(
        output.status.success(),
        "version --json --no-error must exit 0: stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value =
        serde_json::from_str(std::str::from_utf8(&output.stdout).expect("utf8"))
            .expect("stdout is valid JSON");
    assert!(json.as_object().expect("JSON object").contains_key("versions"));
}

#[test]
fn repo_version_text_absent_exits_one() {
    // Run from a clean temp dir (no recognizable repo) so the command has
    // nothing to report. This exercises the empty-result path under real
    // shell conditions.
    let tmp = tempfile::tempdir().expect("tempdir");
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "version"])
        .env("NO_COLOR", "1")
        .current_dir(tmp.path())
        .output()
        .expect("run sniff repo version in empty dir");

    assert_eq!(
        output.status.code(),
        Some(1),
        "repo version with no resolvable version must exit 1; stdout={:?}, stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

// ============================================================================
// `sniff repo` / `sniff repo name` terminal-subset tests (Phase 3)
// ============================================================================

#[test]
fn repo_name_verbose_is_name_only() {
    let name_output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "name"])
        .env("NO_COLOR", "1")
        .output()
        .expect("run sniff repo name");

    let name_verbose_output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "name", "-v"])
        .env("NO_COLOR", "1")
        .output()
        .expect("run sniff repo name -v");

    assert!(name_verbose_output.status.success());
    let stdout = std::str::from_utf8(&name_verbose_output.stdout).expect("utf8");
    let name_stdout = std::str::from_utf8(&name_output.stdout).expect("utf8");

    assert_eq!(
        stdout, name_stdout,
        "repo name -v must match repo name (no foreign fields): got {stdout:?}"
    );
    assert!(
        !stdout.contains(" v"),
        "repo name -v must not contain version suffix: {stdout:?}"
    );
    assert!(
        !stdout.contains("package monorepo"),
        "repo name -v must not contain monorepo suffix: {stdout:?}"
    );
}

#[test]
fn repo_default_is_bare_name() {
    let name_output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "name"])
        .env("NO_COLOR", "1")
        .output()
        .expect("run sniff repo name");

    let default_output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo"])
        .env("NO_COLOR", "1")
        .output()
        .expect("run sniff repo");

    assert!(default_output.status.success());
    assert_eq!(
        std::str::from_utf8(&default_output.stdout).expect("utf8"),
        std::str::from_utf8(&name_output.stdout).expect("utf8"),
        "sniff repo must print the bare name"
    );
}

#[test]
fn repo_default_verbose_is_rich_oneliner() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "-v"])
        .env("NO_COLOR", "1")
        .output()
        .expect("run sniff repo -v");

    assert!(output.status.success());
    let stdout = std::str::from_utf8(&output.stdout).expect("utf8");

    assert!(
        stdout.contains(" v")
            || stdout.contains("package monorepo")
            || (stdout.contains('[') && stdout.contains(']')),
        "sniff repo -v must print a rich one-liner with version, monorepo, or language suffix: {stdout:?}"
    );
}

#[test]
fn test_base_flag_before_subcommand() {
    let (_dir, path) = create_test_repo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .current_dir(&path)
        .args(["-b", ".", "filesystem"])
        .assert()
        .success();
}

#[test]
fn test_base_flag_after_subcommand_is_accepted() {
    let (_dir, path) = create_test_repo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .current_dir(&path)
        .args(["filesystem", "-b", "."])
        .assert()
        .success();
}

#[test]
fn test_filesystem_scoped_flags_parse_in_help() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["filesystem", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--refresh-remotes"))
        .stdout(predicate::str::contains("--latest-versions"));
}

#[test]
fn test_repo_scoped_flags_parse_in_help() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest-versions"))
        .stdout(predicate::str::contains("package-dependencies"))
        .stdout(predicate::str::contains("packages"))
        .stdout(predicate::str::contains("package-area"))
        .stdout(predicate::str::contains("dirty-packages"))
        .stdout(predicate::str::contains("dirty-package-areas"))
        .stdout(predicate::str::contains("--refresh-remotes").not());
}

#[test]
fn test_topics_subcommand_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("topics")
        .assert()
        .success()
        .stdout(predicate::str::contains("hardware"))
        .stdout(predicate::str::contains("filesystem"))
        .stdout(predicate::str::contains("software"))
        .stdout(predicate::str::contains("test-runners"));
}

// ============================================================================
// Top-Level Section Subcommand Tests
// os, hardware, network, filesystem
// ============================================================================

#[test]
fn test_os_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("os")
        .assert()
        .success()
        .stdout(predicate::str::contains("Operating System"))
        .stdout(predicate::str::contains("Name:"))
        .stdout(predicate::str::contains("Kernel:"));
}

#[test]
fn test_os_subcommand_json_output() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["os", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should have OS fields at top level (flattened)
    assert!(json.get("name").is_some(), "name should be at top level");
    assert!(
        json.get("kernel").is_some(),
        "kernel should be at top level"
    );
    assert!(
        json.get("hostname").is_some(),
        "hostname should be at top level"
    );

    // Should NOT have wrapper or other sections
    assert!(json.get("os").is_none(), "os wrapper should not exist");
    assert!(json.get("hardware").is_none());
    assert!(json.get("network").is_none());
    assert!(json.get("filesystem").is_none());
}

#[test]
fn test_runtime_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["runtime", "--plain"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("Runtime: "));
}

#[test]
fn test_runtime_subcommand_json_output() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["runtime", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let runtime: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(
        matches!(runtime.as_str(), Some("native" | "wsl1" | "wsl2")),
        "unexpected runtime: {runtime}"
    );
}

#[test]
fn test_hardware_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("hardware")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Hardware ==="))
        .stdout(predicate::str::contains("CPU:"))
        .stdout(predicate::str::contains("Memory:"));
}

#[test]
fn test_hardware_subcommand_json_output() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["hardware", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should have hardware fields at top level (flattened)
    assert!(json.get("cpu").is_some(), "cpu should be at top level");
    assert!(json.get("gpu").is_some(), "gpu should be at top level");
    assert!(
        json.get("memory").is_some(),
        "memory should be at top level"
    );
    assert!(
        json.get("storage").is_some(),
        "storage should be at top level"
    );

    // Should NOT have wrapper or other sections
    assert!(
        json.get("hardware").is_none(),
        "hardware wrapper should not exist"
    );
    assert!(json.get("os").is_none());
    assert!(json.get("network").is_none());
    assert!(json.get("filesystem").is_none());
}

#[test]
fn test_network_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("network")
        .assert()
        .success()
        .stdout(predicate::str::contains("Network"))
        .stdout(predicate::str::contains("Primary interface:"))
        .stdout(predicate::str::contains("##").not());
}

#[test]
fn test_network_subcommand_verbose_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["network", "-v"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Interfaces"))
        .stdout(predicate::str::contains("##").not());
}

#[test]
fn test_network_subcommand_json_output() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["network", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should have network fields at top level (flattened)
    assert!(
        json.get("interfaces").is_some(),
        "interfaces should be at top level"
    );
    assert!(
        json.get("permission_denied").is_some(),
        "permission_denied should be at top level"
    );
    assert!(
        json.get("wan_ip_address").is_some(),
        "wan_ip_address should be at top level"
    );

    // Should NOT have wrapper or other sections
    assert!(
        json.get("network").is_none(),
        "network wrapper should not exist"
    );
    assert!(json.get("os").is_none());
    assert!(json.get("hardware").is_none());
    assert!(json.get("filesystem").is_none());
}

#[test]
fn test_filesystem_subcommand_text_output() {
    let (_dir, path) = create_test_repo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .current_dir(&path)
        .arg("filesystem")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Filesystem ==="));
}

#[test]
fn test_filesystem_subcommand_json_output() {
    let (_dir, path) = create_test_repo();
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .current_dir(&path)
        .args(["filesystem", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should have filesystem fields at top level (flattened)
    assert!(json.get("git").is_some(), "git should be at top level");

    // Should NOT have wrapper or other sections
    assert!(
        json.get("filesystem").is_none(),
        "filesystem wrapper should not exist"
    );
    assert!(json.get("os").is_none());
    assert!(json.get("hardware").is_none());
    assert!(json.get("network").is_none());
}

// ============================================================================
// Hardware Detail Subcommand Tests
// cpu, gpu, memory, storage
// ============================================================================

#[test]
fn test_cpu_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("cpu")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== CPU ==="))
        .stdout(predicate::str::contains("Brand:"))
        .stdout(predicate::str::contains("Logical cores:"));
}

#[test]
fn test_cpu_subcommand_json_output() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["cpu", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should have CPU fields at top level (flattened)
    assert!(json.get("brand").is_some(), "brand should be at top level");
    assert!(
        json.get("logical_cores").is_some(),
        "logical_cores should be at top level"
    );
    assert!(json.get("simd").is_some(), "simd should be at top level");

    // Should NOT have wrappers
    assert!(json.get("cpu").is_none(), "cpu wrapper should not exist");
    assert!(json.get("hardware").is_none());
}

#[test]
fn test_gpu_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("gpu")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== GPU ==="));
}

#[test]
fn test_gpu_subcommand_json_output() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["gpu", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Top level should be an array (GPU list)
    assert!(
        json.is_array(),
        "GPU output should be an array at top level"
    );
}

#[test]
fn test_memory_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("memory")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Memory ==="))
        .stdout(predicate::str::contains("Total:"))
        .stdout(predicate::str::contains("Available:"));
}

#[test]
fn test_memory_subcommand_json_output() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["memory", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should have memory fields at top level (flattened)
    assert!(
        json.get("total_bytes").is_some(),
        "total_bytes should be at top level"
    );
    assert!(
        json.get("available_bytes").is_some(),
        "available_bytes should be at top level"
    );
    assert!(
        json.get("used_bytes").is_some(),
        "used_bytes should be at top level"
    );

    // Should NOT have wrappers
    assert!(
        json.get("memory").is_none(),
        "memory wrapper should not exist"
    );
    assert!(json.get("hardware").is_none());
}

#[test]
fn test_storage_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("storage")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Storage ==="));
}

#[test]
fn test_storage_subcommand_json_output() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["storage", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Top level should be an array (storage/disk list)
    assert!(
        json.is_array(),
        "Storage output should be an array at top level"
    );

    // Should have at least one disk
    let storage = json.as_array().unwrap();
    assert!(!storage.is_empty(), "storage should have at least one disk");
}

// ============================================================================
// Filesystem Detail Subcommand Tests
// git, repo, language
// ============================================================================

#[test]
fn test_git_status_subcommand_text_output() {
    let (_dir, path) = create_test_repo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .current_dir(&path)
        .args(["repo", "git-status"])
        .assert()
        .success()
        // Rich output format has Status and Meta sections
        .stdout(predicate::str::contains("Status"))
        .stdout(predicate::str::contains("Meta"));
}

#[test]
fn test_git_status_subcommand_with_history_flag() {
    // Test that the --history flag is accepted
    let (_dir, path) = create_test_repo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .current_dir(&path)
        .args(["repo", "git-status", "--history", "3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Status"));
}

#[test]
fn test_git_status_subcommand_compact_output() {
    let (_dir, path) = create_test_repo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .current_dir(&path)
        .args(["repo", "git-status", "--compact"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Status"))
        .stdout(predicate::str::contains("\x1b[1m\x1b[4mMeta").not());
}

#[test]
fn test_git_status_subcommand_json_output() {
    let (_dir, path) = create_test_repo();
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .current_dir(&path)
        .args(["repo", "git-status", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // `repo git-status --json` returns a focused `GitInfo` object — not the
    // full `RepoInfo` blob. Top-level keys mirror `GitInfo` fields.
    assert!(json.is_object(), "git-status JSON should be an object");
    assert!(
        json.get("repo_root").is_some(),
        "git-status JSON should have top-level `repo_root`: {json}"
    );
    assert!(
        json.get("status").is_some(),
        "git-status JSON should have top-level `status`: {json}"
    );
    assert!(
        json.get("recent").is_some(),
        "git-status JSON should have top-level `recent`: {json}"
    );
    assert!(
        json.get("branches").is_some(),
        "git-status JSON should have top-level `branches`: {json}"
    );

    // The identity-only `head_id` field must not appear in status-bearing
    // git-status JSON — the identity request work did not expand this shape.
    assert!(
        json.get("head_id").is_none(),
        "git-status JSON should NOT contain identity-only `head_id`: {json}"
    );

    // RepoInfo-only fields must not leak into git-status JSON.
    assert!(
        json.get("is_monorepo").is_none(),
        "git-status JSON should NOT contain RepoInfo `is_monorepo`: {json}"
    );
    assert!(
        json.get("packages").is_none(),
        "git-status JSON should NOT contain RepoInfo `packages`: {json}"
    );
}

#[test]
fn test_repo_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap().arg("repo").assert().success();
}

#[test]
fn test_repo_subcommand_json_output() {
    let (_dir, path) = create_test_repo();
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .current_dir(&path)
        .args(["repo", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should be object or null at top level
    assert!(
        json.is_object() || json.is_null(),
        "repo output should be object or null at top level"
    );

    // Should NOT have wrappers
    assert!(json.get("repo").is_none(), "repo wrapper should not exist");
    assert!(json.get("filesystem").is_none());
}

#[test]
fn test_language_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "language", "--breakdown"])
        .assert()
        .success();
}

#[test]
fn test_language_subcommand_json_output() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "language", "--breakdown", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap();
    let json: serde_json::Value = serde_json::from_str(json_str).unwrap();

    // Should be object or null at top level
    assert!(
        json.is_object() || json.is_null(),
        "language output should be object or null at top level"
    );

    // Should NOT have wrappers
    assert!(
        json.get("language").is_none(),
        "language wrapper should not exist"
    );
    assert!(json.get("filesystem").is_none());
}

// ============================================================================
// `sniff repo language` Subcommand Tests (review-plan-1, Phase 2)
// Pins:
//   - text output exact contract: `Rust\n` / empty + exit 1
//   - JSON output exact contract: `{"language":"Rust"}` / `{"language":null}` + exit 1
//   - `--base` works in all three placements (global pre, repo-nested, leaf)
// ============================================================================

#[test]
fn test_repo_language_text_returns_rust_for_rust_repo() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "language"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout, "Rust\n", "expected exact `Rust\\n` output");
}

#[test]
fn test_repo_language_json_returns_rust_for_rust_repo() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "language",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap().trim_end();
    let parsed: serde_json::Value =
        serde_json::from_str(json_str).expect("repo language --json must emit valid JSON");

    // Exact shape contract: object with single key "language" → "Rust".
    assert_eq!(parsed, serde_json::json!({ "language": "Rust" }));
}

#[test]
fn test_repo_language_base_flag_all_three_placements() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    let base = path.to_str().unwrap();

    // Placement A: `sniff --base <repo> repo language` (global, before subcommand)
    let a = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", base, "repo", "language"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(
        String::from_utf8(a).unwrap(),
        "Rust\n",
        "placement A failed"
    );

    // Placement B: `sniff repo --base <repo> language` (between repo and leaf)
    let b = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "--base", base, "language"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(
        String::from_utf8(b).unwrap(),
        "Rust\n",
        "placement B failed"
    );

    // Placement C: `sniff repo language --base <repo>` (after the leaf subcommand)
    let c = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "language", "--base", base])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(
        String::from_utf8(c).unwrap(),
        "Rust\n",
        "placement C failed"
    );
}

#[test]
fn test_repo_language_text_empty_repo_exits_one_with_no_stdout() {
    // create_test_repo creates a git repo with one empty initial commit
    // and no source files — primary language detection returns None.
    let (_dir, path) = create_test_repo();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "language"])
        .assert()
        .failure() // exit 1 by Phase 1 contract
        .code(1);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(
        stdout, "",
        "text mode must emit no stdout when no language detected"
    );
}

#[test]
fn test_repo_language_json_empty_repo_emits_null_and_exits_one() {
    let (_dir, path) = create_test_repo();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "language",
            "--json",
        ])
        .assert()
        .failure()
        .code(1);

    let stdout = assert.get_output().stdout.clone();
    let json_str = std::str::from_utf8(&stdout).unwrap().trim_end();
    let parsed: serde_json::Value = serde_json::from_str(json_str)
        .expect("repo language --json must emit valid JSON even when null");
    assert_eq!(parsed, serde_json::json!({ "language": null }));
}

#[test]
fn test_repo_help_lists_language_subcommand() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sniff repo language"));
}

// ============================================================================
// Software Subcommand Tests
// sniff software and all reparented categories
// ============================================================================

#[test]
fn test_software_subcommand_text_output() {
    // In a non-TTY context, terminal width defaults to 80 columns which may be
    // too narrow for the programs table. Accept either the rendered table
    // or the graceful width error message.
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_software_subcommand_json_output() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    let entries = json
        .as_array()
        .expect("software --json should return an array");
    assert!(!entries.is_empty(), "software JSON should not be empty");

    let first = entries[0]
        .as_object()
        .expect("software JSON entries should be objects");
    assert!(first.contains_key("name"));
    assert!(first.contains_key("binary_name"));
    assert!(first.contains_key("description"));
    assert!(first.contains_key("website"));
}

#[test]
fn test_software_subcommand_rejects_json_format_flag() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "--json-format", "full"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "unexpected argument '--json-format'",
        ));
}

#[test]
fn test_software_editors_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "editors"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_software_editors_subcommand_json_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "editors", "--json"])
        .assert()
        .success();
}

#[test]
fn test_software_utilities_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "utilities"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_software_utilities_subcommand_json_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "utilities", "--json"])
        .assert()
        .success();
}

#[test]
fn test_software_language_package_managers_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "language-package-managers"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_software_language_package_managers_subcommand_json_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "language-package-managers", "--json"])
        .assert()
        .success();
}

#[test]
fn test_software_os_package_managers_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "os-package-managers"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Name"))
        .stdout(predicate::str::contains("Installed"));
}

#[test]
fn test_software_os_package_managers_subcommand_json_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "os-package-managers", "--json"])
        .assert()
        .success();
}

#[test]
fn test_software_tts_clients_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "tts-clients"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_software_tts_clients_subcommand_json_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "tts-clients", "--json"])
        .assert()
        .success();
}

#[test]
fn test_software_terminal_apps_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "terminal-apps"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_software_terminal_apps_subcommand_json_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "terminal-apps", "--json"])
        .assert()
        .success();
}

#[test]
fn test_software_audio_players_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "audio-players"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_software_audio_players_subcommand_json_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "audio-players", "--json"])
        .assert()
        .success();
}

#[test]
fn test_software_agents_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "agents"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_software_agents_subcommand_json_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "agents", "--json"])
        .assert()
        .success();
}

#[test]
fn test_software_notification_helpers_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "notification-helpers"])
        .assert()
        .success();
}

#[test]
fn test_software_notification_helpers_subcommand_json_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "notification-helpers", "--json"])
        .assert()
        .success();
}

// ============================================================================
// Test runner subcommand
// ============================================================================

#[test]
fn test_software_test_runners_subcommand_text_output() {
    // In a non-TTY context, terminal width defaults to 80 columns which may be
    // too narrow for the test-runner table. Accept either the rendered table
    // or the graceful width error message.
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "test-runners"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_software_test_runners_subcommand_json_output_shape() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "test-runners", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("stdout is valid JSON");
    let entries = json
        .as_object()
        .expect("software test-runners --json should return a map keyed by serde_key");
    assert!(!entries.is_empty(), "test-runner map should not be empty");

    // Every entry must carry an `availability` discriminator with one of the
    // four documented values. The discriminator and per-variant fields
    // (`path`, `root`, `parent`) live at the same level as the entry metadata.
    let allowed = ["installed", "local", "via_parent", "not_found"];
    for (_, entry) in entries {
        let entry = entry.as_object().expect("test-runner entry is an object");
        assert!(entry.contains_key("name"), "entry has a name: {:?}", entry);
        assert!(entry.contains_key("binary_name"), "entry has a binary_name");
        assert!(
            entry.contains_key("ecosystem"),
            "entry carries its ecosystem"
        );
        let availability = entry
            .get("availability")
            .and_then(Value::as_str)
            .expect("entry has an availability discriminator");
        assert!(
            allowed.contains(&availability),
            "availability {availability:?} is one of {allowed:?}"
        );

        // Per-variant fields must be present when the discriminator claims them.
        match availability {
            "installed" => assert!(entry.contains_key("path")),
            "local" => {
                assert!(entry.contains_key("path"));
                assert!(entry.contains_key("root"));
            }
            "via_parent" => assert!(entry.contains_key("parent")),
            _ => {}
        }
    }

    // cargo_test is one of the catalog entries; it should be present.
    assert!(
        entries.contains_key("cargo_test"),
        "cargo_test entry is present: keys = {:?}",
        entries.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_software_test_runners_json_stdout_is_parseable_without_stderr() {
    // The search-context hint must go to stderr, not stdout, so stdout is
    // valid JSON even when the hint is shown.
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "test-runners", "--json"])
        .assert()
        .success()
        .get_output()
        .clone();
    let _: Value = serde_json::from_slice(&output.stdout).expect("stdout is valid JSON");
    assert!(
        output.stderr.is_empty(),
        "software test-runners --json must not emit hints or legends to stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_software_test_runners_plain_suppresses_hint_and_ansi() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "test-runners", "--plain"])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "plain software test-runners stdout must not contain ANSI escapes: {stdout:?}"
    );
    assert!(
        output.stderr.is_empty(),
        "plain software test-runners must not emit the styled search hint: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_cargo_workspace_repo() -> (tempfile::TempDir, PathBuf) {
    let (dir, path) = create_test_repo();
    std::fs::write(
        path.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/app\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(path.join("crates/app/src")).unwrap();
    std::fs::write(
        path.join("crates/app/Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(path.join("crates/app/src/lib.rs"), "pub fn app() {}\n").unwrap();
    (dir, path)
}

#[test]
fn test_repo_test_runner_json_reports_package_usage() {
    let (_dir, path) = create_cargo_workspace_repo();

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "test-runner",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("stdout is valid JSON");
    let runners = json["test_runners"]
        .as_array()
        .expect("test_runners is always an array");
    assert_eq!(runners.len(), 1, "single Rust crate => one runner");
    let entry = &runners[0];
    assert_eq!(entry["runner"], "CargoTest");
    assert_eq!(entry["source"]["kind"], "ecosystem_default");
    // Enriched metadata: the run command and documentation website.
    assert_eq!(entry["binary"], "cargo test");
    assert_eq!(entry["website"], "https://doc.rust-lang.org/cargo/commands/cargo-test.html");
}

#[test]
fn test_repo_test_runner_list_reports_library_values() {
    let (_dir, path) = create_cargo_workspace_repo();

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "test-runner",
            "--list",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("cargo test"));
}

#[test]
fn test_repo_test_runner_output_modes() {
    let (_dir, path) = create_cargo_workspace_repo();
    let base = path.to_str().unwrap();

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", base, "repo", "test-runner", "--csv"])
        .assert()
        .success()
        .stdout("cargo test\n");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", base, "repo", "test-runner", "--md"])
        .assert()
        .success()
        .stdout("- cargo test\n");

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", base, "repo", "test-runner", "--plain"])
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "plain repo test-runner stdout must not contain ANSI escapes: {stdout:?}"
    );
}

#[test]
fn test_repo_test_runner_verbose_machine_formats_keep_evidence() {
    // `--list`/`--md`/`--csv` with `-v` carry the same styled provenance the
    // default CSV shows, formatted per their delimiter (`--plain` strips ANSI;
    // the config link degrades to a Markdown link in plain mode).
    let (_dir, path) = create_cargo_workspace_repo();
    std::fs::create_dir_all(path.join(".config")).unwrap();
    std::fs::write(path.join(".config/nextest.toml"), "[profile.default]\n").unwrap();
    let base = path.to_str().unwrap();

    let list = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", base, "repo", "test-runner", "-v", "--list", "--plain"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let list = String::from_utf8_lossy(&list);
    assert!(
        list.starts_with("cargo-nextest (configuration located at:")
            && list.contains(".config/nextest.toml"),
        "--list -v should keep the evidence, got {list:?}"
    );

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", base, "repo", "test-runner", "-v", "--md", "--plain"])
        .assert()
        .success()
        .stdout(
            predicate::str::starts_with("- cargo-nextest (configuration located at:")
                .and(predicate::str::contains(".config/nextest.toml")),
        );

    // Without -v the machine formats stay names-only.
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", base, "repo", "test-runner", "--list"])
        .assert()
        .success()
        .stdout("cargo-nextest\n");
}

#[test]
fn test_repo_test_runner_detects_workspace_root_nextest() {
    // nextest is configured once at the workspace root; the member crate carries
    // no nextest marker of its own. The repo aggregate must still surface it.
    let (_dir, path) = create_cargo_workspace_repo();
    std::fs::create_dir_all(path.join(".config")).unwrap();
    std::fs::write(
        path.join(".config/nextest.toml"),
        "[profile.default]\n",
    )
    .unwrap();

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "test-runner",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).expect("stdout is valid JSON");
    let runners = json["test_runners"]
        .as_array()
        .expect("test_runners is always an array");
    // The configured runner supersedes the cargo test ecosystem default, so the
    // single member crate collapses to nextest alone.
    assert_eq!(runners.len(), 1, "configured nextest should be the lone answer, got {json}");
    let entry = &runners[0];
    assert_eq!(entry["runner"], "Nextest");
    assert_eq!(entry["source"]["kind"], "config");
    assert_eq!(entry["source"]["filename"], ".config/nextest.toml");
    assert_eq!(entry["binary"], "cargo nextest run");

    // Default text output is the single answer with no cargo test noise.
    let text = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "test-runner", "--plain"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8_lossy(&text);
    assert_eq!(text.trim(), "cargo-nextest");
    assert!(
        !text.contains("cargo test"),
        "cargo test must be superseded by configured nextest, got {text:?}"
    );
}

#[test]
fn test_repo_package_manager_json_uses_shared_collapse() {
    let (_dir, path) = create_cargo_workspace_repo();

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-manager",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    assert!(
        output.stderr.is_empty(),
        "repo package-manager --json must not emit hints or legends to stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout is valid JSON");
    assert_eq!(json["package_manager"], "cargo");
}

// ============================================================================
// `sniff repo version` integration tests — focused array contract mirroring
// the test-runner JSON shape.
// ============================================================================

/// Monorepo root reports its cross-package collapse with the new
/// `{ "versions": [...] }` shape, never the legacy `{ "version": ... }`.
#[test]
fn test_repo_version_json_reports_array_shape() {
    let (_dir, path) = create_cli_monorepo();

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "version", "--json"])
        .assert()
        .success()
        .get_output()
        .clone();

    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout is valid JSON");
    let versions = json["versions"]
        .as_array()
        .expect("`versions` is always an array");
    assert_eq!(versions.len(), 1, "two crates at 0.1.0 → one entry");
    let entry = &versions[0];
    assert_eq!(entry["version"], "0.1.0");
    assert!(
        entry["packages"].as_array().expect("packages array").len() >= 2,
        "in-scope packages should be reported, got {entry:?}"
    );
    let sources = entry["sources"]
        .as_array()
        .expect("sources is always an array");
    assert!(!sources.is_empty(), "at least one source, got {entry:?}");
    assert_eq!(sources[0]["manifest"], "Cargo.toml");
    assert_eq!(sources[0]["inherited"], false);
    assert!(sources[0]["href"]
        .as_str()
        .expect("href is a string")
        .starts_with("file://"));
}

#[test]
fn test_repo_version_text_output_default() {
    let (_dir, path) = create_cli_monorepo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "version", "--plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
fn test_repo_version_csv_output_is_names_only() {
    let (_dir, path) = create_cli_monorepo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "version", "--csv"])
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
fn test_repo_version_md_output_uses_dash_prefix() {
    let (_dir, path) = create_cli_monorepo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "version", "--md"])
        .assert()
        .success()
        .stdout(predicate::str::contains("- 0.1.0"));
}

#[test]
fn test_repo_version_list_output_is_names_only() {
    let (_dir, path) = create_cli_monorepo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "version", "--list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

/// Monorepo root with no flag reports the cross-package collapse (all crates
/// at `0.1.0` collapse to one entry).
#[test]
fn test_repo_version_monorepo_root_reports_collapsed_versions() {
    let (_dir, path) = create_cli_monorepo();

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "version", "--json"])
        .assert()
        .success()
        .get_output()
        .clone();

    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout is valid JSON");
    let versions = json["versions"].as_array().expect("versions array");
    assert_eq!(versions.len(), 1, "uniform collapse, got {json}");
    assert_eq!(versions[0]["version"], "0.1.0");
    let packages = versions[0]["packages"]
        .as_array()
        .expect("packages array");
    let names: Vec<&str> = packages.iter().map(|p| p.as_str().unwrap()).collect();
    assert!(names.contains(&"pkg-a"));
    assert!(names.contains(&"pkg-b"));
}

/// `--all` from inside a package directory must discover the *enclosing* repo
/// and span every package, not analyze the subdir as a standalone package.
/// Asserting the package list (not just the collapsed string) is what proves
/// the enclosing repo was discovered — uniform versions alone would mask a
/// missing `pkg-b`.
#[test]
fn test_repo_version_all_override_returns_repo_scope() {
    let (_dir, path) = create_cli_monorepo();

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.join("pkg-a/lib").to_str().unwrap(),
            "repo",
            "version",
            "--all",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout is valid JSON");
    let versions = json["versions"].as_array().expect("versions array");
    assert_eq!(versions.len(), 1, "uniform collapse, got {json}");
    assert_eq!(versions[0]["version"], "0.1.0");
    let names: Vec<&str> = versions[0]["packages"]
        .as_array()
        .expect("packages array")
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    assert!(
        names.contains(&"pkg-a") && names.contains(&"pkg-b"),
        "`--all` from inside pkg-a must span the whole repo, got {names:?}"
    );
}

/// `--package <name>` scopes the collapse to one package.
#[test]
fn test_repo_version_package_override_scopes_to_single_package() {
    let (_dir, path) = create_cli_monorepo();

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "version",
            "--package",
            "pkg-a",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout is valid JSON");
    let versions = json["versions"].as_array().expect("versions array");
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0]["version"], "0.1.0");
    let packages = versions[0]["packages"].as_array().expect("packages array");
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0], "pkg-a");
}

/// `--package-area <name>` scopes the collapse to one area.
#[test]
fn test_repo_version_package_area_override_scopes_to_single_area() {
    let (_dir, path) = create_cli_monorepo();

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "version",
            "--package-area",
            "pkg-a",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout is valid JSON");
    let versions = json["versions"].as_array().expect("versions array");
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0]["version"], "0.1.0");
    let packages = versions[0]["packages"].as_array().expect("packages array");
    let names: Vec<&str> = packages.iter().map(|p| p.as_str().unwrap()).collect();
    assert_eq!(names, vec!["pkg-a"]);
}

/// Unknown `--package` errors clearly (no JSON, exit non-zero).
#[test]
fn test_repo_version_unknown_package_errors() {
    let (_dir, path) = create_cli_monorepo();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "version",
            "--package",
            "ghost",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("ghost") || stderr.contains("package"),
        "expected unknown-package error, got {stderr}"
    );
}

/// Unknown `--package-area` errors clearly.
#[test]
fn test_repo_version_unknown_package_area_errors() {
    let (_dir, path) = create_cli_monorepo();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "version",
            "--package-area",
            "ghost",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("ghost") || stderr.contains("area"),
        "expected unknown-area error, got {stderr}"
    );
}

/// A synthesized single-package repo (`is_monorepo == false`) must still
/// validate `--package` against its catalog: an unknown name errors clearly
/// instead of silently printing the local version.
#[test]
fn test_repo_version_single_package_unknown_package_errors() {
    let (_dir, path) = create_single_package_repo();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "version",
            "--package",
            "ghost",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("ghost") || stderr.contains("package"),
        "expected unknown-package error on a single-package repo, got {stderr}"
    );
}

/// A synthesized single-package repo also validates `--package-area`: an
/// unknown area errors rather than falling back to the local version.
#[test]
fn test_repo_version_single_package_unknown_area_errors() {
    let (_dir, path) = create_single_package_repo();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "version",
            "--package-area",
            "ghost",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("ghost") || stderr.contains("area"),
        "expected unknown-area error on a single-package repo, got {stderr}"
    );
}

/// The known package on a single-package repo resolves and reports its
/// version, confirming override validation accepts valid catalog targets.
#[test]
fn test_repo_version_single_package_known_package_resolves() {
    let (_dir, path) = create_single_package_repo();

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "version",
            "--package",
            "solo",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout is valid JSON");
    let versions = json["versions"].as_array().expect("versions array");
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0]["version"], "0.4.2");
    let names: Vec<&str> = versions[0]["packages"]
        .as_array()
        .expect("packages array")
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["solo"]);
}

/// Variance across packages: two distinct versions render as a multi-entry
/// list, with each version collapsing its own packages.
#[test]
fn test_repo_version_variance_reports_each_version_separately() {
    let (_dir, path) = create_cli_monorepo();
    // Bump pkg-b to a different version to force variance.
    std::fs::write(
        path.join("pkg-b/lib/Cargo.toml"),
        r#"[package]
name = "pkg-b"
version = "2.0.0"
edition = "2024"
"#,
    )
    .unwrap();

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "version", "--json"])
        .assert()
        .success()
        .get_output()
        .clone();

    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout is valid JSON");
    let versions = json["versions"].as_array().expect("versions array");
    assert_eq!(versions.len(), 2, "variance should not collapse, got {json}");
    let mut values: Vec<&str> = versions
        .iter()
        .map(|v| v["version"].as_str().unwrap())
        .collect();
    values.sort();
    assert_eq!(values, vec!["0.1.0", "2.0.0"]);

    let list = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "version", "--list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let list = String::from_utf8_lossy(&list);
    assert!(list.contains("0.1.0") && list.contains("2.0.0"));
}

/// Empty result (no resolvable version) prints nothing and exits 1.
#[test]
fn test_repo_version_empty_exits_one_with_no_stdout() {
    let (_dir, path) = create_test_repo();
    // No manifest → nothing to read.
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "version"])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.trim().is_empty(),
        "no-version context must print nothing, got {stdout:?}"
    );
}

/// `--no-error` flips empty to exit 0 (and JSON still emits `{ "versions": [] }`).
#[test]
fn test_repo_version_empty_with_no_error_exits_zero() {
    let (_dir, path) = create_test_repo();

    let json = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "version",
            "--json",
            "--no-error",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&json).expect("stdout is valid JSON");
    assert_eq!(json["versions"], serde_json::json!([]));

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "version",
            "--no-error",
        ])
        .assert()
        .success();
}

/// Verbose mode shows the manifest source for each entry; the single-source
/// path is hyperlinked, and workspace inheritance is named explicitly.
#[test]
fn test_repo_version_verbose_named_workspace_inheritance() {
    let (_dir, path) = create_test_repo();
    // Multi-member workspace with `[workspace.package].version`; one member
    // uses `version.workspace = true` so the inheritance path is exercised.
    std::fs::write(
        path.join("Cargo.toml"),
        r#"[workspace]
members = ["pkg-a/lib", "pkg-b/lib"]
resolver = "2"

[workspace.package]
version = "3.1.4"
edition = "2024"
"#,
    )
    .unwrap();
    let pkg_a = path.join("pkg-a/lib");
    std::fs::create_dir_all(pkg_a.join("src")).unwrap();
    std::fs::write(
        pkg_a.join("Cargo.toml"),
        r#"[package]
name = "pkg-a"
version.workspace = true
edition.workspace = true
"#,
    )
    .unwrap();
    std::fs::write(pkg_a.join("src/lib.rs"), "pub fn a() {}").unwrap();
    let pkg_b = path.join("pkg-b/lib");
    std::fs::create_dir_all(pkg_b.join("src")).unwrap();
    std::fs::write(
        pkg_b.join("Cargo.toml"),
        r#"[package]
name = "pkg-b"
version = "0.5.0"
edition = "2024"
"#,
    )
    .unwrap();
    std::fs::write(pkg_b.join("src/lib.rs"), "pub fn b() {}").unwrap();

    let json = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "version",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&json).expect("stdout is valid JSON");
    let versions = json["versions"].as_array().expect("versions array");
    // Two distinct versions, one inherited.
    let mut seen: Vec<&str> = versions
        .iter()
        .map(|v| v["version"].as_str().unwrap())
        .collect();
    seen.sort();
    assert_eq!(seen, vec!["0.5.0", "3.1.4"]);
    let inherited_entry = versions
        .iter()
        .find(|v| v["version"] == "3.1.4")
        .expect("3.1.4 entry");
    let sources = inherited_entry["sources"]
        .as_array()
        .expect("sources array");
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0]["inherited"], true, "inherited flag must be set");
    assert_eq!(sources[0]["path"], "Cargo.toml");

    // Verbose text surfaces `[workspace.package]` rather than a misleading
    // member-crate path.
    let text = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "version",
            "--verbose",
            "--plain",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8_lossy(&text);
    assert!(
        text.contains("[workspace.package]"),
        "verbose text should name the inherited source, got {text:?}"
    );
}

/// Bare `sniff repo --json` invoked from inside a member package of a
/// Cargo workspace that inherits its version must still collapse the
/// top-level `version` to the workspace's `[workspace.package].version`.
///
/// Regression guard: the aggregate previously resolved inheritance relative
/// to the invocation directory (the member), where `Cargo.toml` carries no
/// `[workspace.package]`, so the top-level `version` came back `null`. It
/// must be rooted at the repo root instead.
#[test]
fn test_bare_repo_json_version_collapses_workspace_inheritance_from_member() {
    let (_dir, path) = create_test_repo();
    std::fs::write(
        path.join("Cargo.toml"),
        r#"[workspace]
members = ["member"]
resolver = "2"

[workspace.package]
version = "7.2.0"
edition = "2024"
"#,
    )
    .unwrap();
    let member = path.join("member");
    std::fs::create_dir_all(member.join("src")).unwrap();
    std::fs::write(
        member.join("Cargo.toml"),
        r#"[package]
name = "member"
version.workspace = true
edition.workspace = true
"#,
    )
    .unwrap();
    std::fs::write(member.join("src/lib.rs"), "pub fn m() {}").unwrap();

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", member.to_str().unwrap(), "repo", "--json"])
        .assert()
        .success()
        .get_output()
        .clone();

    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout is valid JSON");
    assert_eq!(
        json["version"], "7.2.0",
        "bare repo --json from a member must inherit the workspace version, got {json}"
    );
}

/// `--json` on an empty result still emits valid JSON; nothing leaks to
/// stderr in JSON mode (stdout stays machine-parseable).
#[test]
fn test_repo_version_empty_json_emits_array_shape() {
    let (_dir, path) = create_test_repo();
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "version", "--json"])
        .assert()
        .failure()
        .get_output()
        .clone();
    let json: Value = serde_json::from_slice(&output.stdout)
        .expect("stdout is valid JSON even on empty result");
    assert_eq!(json["versions"], serde_json::json!([]));
}

#[test]
fn test_repo_package_manager_variant_list_uses_unique_values() {
    let (_dir, path) = create_cargo_workspace_repo();
    std::fs::write(
        path.join("crates/app/package.json"),
        r#"{"name":"app-js","version":"0.1.0"}"#,
    )
    .unwrap();

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-manager",
            "--list",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("cargo"))
        .stdout(predicate::str::contains("npm"));
}

#[test]
fn test_repo_package_manager_output_modes() {
    let (_dir, path) = create_cargo_workspace_repo();
    let base = path.to_str().unwrap();

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", base, "repo", "package-manager", "--csv"])
        .assert()
        .success()
        .stdout("cargo\n");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", base, "repo", "package-manager", "--md"])
        .assert()
        .success()
        .stdout("- cargo\n");

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", base, "repo", "package-manager", "--plain"])
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "plain repo package-manager stdout must not contain ANSI escapes: {stdout:?}"
    );
}

// ============================================================================
// Negative tests — old top-level program paths are rejected
// ============================================================================

#[test]
fn test_old_programs_command_fails() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("programs")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_old_editors_command_fails() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("editors")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_old_utilities_command_fails() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("utilities")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_old_language_package_managers_command_fails() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("language-package-managers")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_old_os_package_managers_command_fails() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("os-package-managers")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_old_tts_clients_command_fails() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("tts-clients")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_old_terminal_apps_command_fails() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("terminal-apps")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_old_audio_players_command_fails() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("audio-players")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_old_agents_command_fails() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("agents")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_old_notification_helpers_command_fails() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("notification-helpers")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

// ============================================================================
// Services Subcommand Tests
// ============================================================================

#[test]
fn test_services_subcommand_text_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("services")
        .assert()
        .success()
        .stdout(predicate::str::contains("=== Services ==="))
        .stdout(predicate::str::contains("Init System:"));
}

#[test]
fn test_services_subcommand_json_output() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["services", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("init_system"))
        .stdout(predicate::str::contains("services"));
}

#[test]
fn test_services_state_all() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["services", "--state", "all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Services:"));
}

#[test]
fn test_services_state_running() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["services", "--state", "running"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Running Services:"));
}

#[test]
fn test_services_state_stopped() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["services", "--state", "stopped"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stopped Services:"));
}

// ============================================================================
// Scoped Enrichment Flag Tests
// ============================================================================

#[test]
fn test_enrichment_flags_in_help() {
    // Top-level help should mention --plain and repo, but not --deep
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--deep").not())
        .stdout(predicate::str::contains("--plain"))
        .stdout(predicate::str::contains("sniff repo"));
}

#[test]
fn test_filesystem_help_mentions_scoped_flags() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["filesystem", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--refresh-remotes"))
        .stdout(predicate::str::contains("--latest-versions"));
}

#[test]
fn test_git_status_help_mentions_refresh_remotes() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "git-status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--refresh-remotes"))
        .stdout(predicate::str::contains("--compact"));
}

#[test]
fn test_repo_help_mentions_latest_versions() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest-versions"))
        .stdout(predicate::str::contains("--refresh-remotes").not());
}

#[test]
fn test_git_status_json_is_git_info() {
    // Verify JSON output is a `GitInfo` object — not the full `RepoInfo`
    // blob. The top-level `repo_root` field is unique to `GitInfo`'s shape
    // (RepoInfo serializes its root field as `root`).
    let (_dir, path) = create_test_repo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .current_dir(&path)
        .args(["repo", "git-status", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("repo_root"))
        .stdout(predicate::str::contains("\"is_monorepo\"").not())
        .stdout(predicate::str::contains("\"packages\"").not());
}

#[test]
fn test_repo_package_dependencies_help_mentions_ui() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "package-dependencies", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--ui"));
}

#[test]
fn test_repo_deps_is_not_an_alias() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "deps"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_repo_branches_json_shape() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "branches", "--json"])
        .assert()
        .success()
        .get_output()
        .clone();
    assert!(
        output.stderr.is_empty(),
        "repo branches --json must not emit hints or legends to stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("branches JSON is valid");
    let branches = json.as_array().expect("branches JSON is an array");
    let current = branches
        .iter()
        .find(|branch| branch["current"].as_bool() == Some(true))
        .expect("one current branch");

    // Every contract key must be present, even when the value is null.
    let obj = current.as_object().expect("branch is an object");
    for key in [
        "name",
        "current",
        "sha",
        "remote_represented",
        "upstream",
        "ahead",
        "behind",
    ] {
        assert!(obj.contains_key(key), "branch is missing key `{key}`: {json}");
    }

    assert!(current["name"].is_string(), "branch has name: {json}");
    assert!(current["sha"].is_string(), "branch has sha: {json}");
    assert!(
        current["remote_represented"].is_boolean(),
        "branch has remote_represented: {json}"
    );

    // A freshly created repo has no configured upstream, so the tracking fields
    // serialize as null rather than collapsing to `0` / an omitted key.
    assert!(
        current["upstream"].is_null(),
        "no-upstream branch serializes upstream as null: {json}"
    );
    assert!(
        current["ahead"].is_null(),
        "no-upstream branch serializes ahead as null: {json}"
    );
    assert!(
        current["behind"].is_null(),
        "no-upstream branch serializes behind as null: {json}"
    );
}

/// Run `sniff <args>` against `path` and return parsed stdout JSON.
fn repo_json_at(path: &Path, args: &[&str]) -> Value {
    let mut full = vec!["--base", path.to_str().unwrap()];
    full.extend_from_slice(args);
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(&full)
        .assert()
        .success()
        .get_output()
        .clone();
    serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).expect("valid JSON on stdout")
}

#[test]
fn test_single_package_cargo_reports_root_facts() {
    let (_dir, path) = create_test_repo();
    std::fs::write(
        path.join("Cargo.toml"),
        "[package]\nname = \"solo\"\nversion = \"0.4.2\"\nedition = \"2021\"\n\n\
         [dependencies]\nserde = \"1\"\n",
    )
    .unwrap();

    // A plain Cargo package (no `[workspace]`) still reports its package manager.
    let pm = repo_json_at(&path, &["repo", "package-manager", "--json"]);
    assert_eq!(pm["package_manager"], "cargo");

    // ... and its declared external dependencies.
    let deps = repo_json_at(&path, &["repo", "dependencies", "--json"]);
    let names: Vec<&str> = deps["dependencies"]
        .as_array()
        .expect("dependencies array")
        .iter()
        .map(|d| d["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"serde"), "single-package deps: {deps}");

    // The bare aggregate carries the same root-package facts.
    let agg = repo_json_at(&path, &["repo", "--json"]);
    assert_eq!(agg["package_manager"], "cargo", "aggregate pm: {agg}");
    assert!(
        agg["dependencies"]["dependencies"]
            .as_array()
            .is_some_and(|d| !d.is_empty()),
        "aggregate external deps populated: {agg}"
    );
    assert!(
        agg["package_dependencies"]["packages"]
            .as_array()
            .is_some_and(|p| !p.is_empty()),
        "aggregate package_dependencies populated: {agg}"
    );

    // The standalone package counts as one in the catalog: `package_count` and
    // `packages` must describe the same single-package universe.
    assert_eq!(agg["package_count"], 1, "aggregate package_count: {agg}");
    assert_eq!(
        agg["packages"],
        serde_json::json!(["solo"]),
        "aggregate packages: {agg}"
    );

    // Focused package commands agree with the aggregate.
    let count = repo_json_at(&path, &["repo", "package-count", "--json"]);
    assert_eq!(count["package-count"], 1, "focused package-count: {count}");
    let packages = repo_json_at(&path, &["repo", "packages", "--json"]);
    assert_eq!(
        packages,
        serde_json::json!(["solo"]),
        "focused packages: {packages}"
    );
}

#[test]
fn test_single_package_node_reports_root_facts() {
    let (_dir, path) = create_test_repo();
    std::fs::write(
        path.join("package.json"),
        r#"{"name":"node-app","version":"1.0.0","dependencies":{"lodash":"^4"}}"#,
    )
    .unwrap();

    let pm = repo_json_at(&path, &["repo", "package-manager", "--json"]);
    assert_eq!(pm["package_manager"], "npm");

    let deps = repo_json_at(&path, &["repo", "dependencies", "--json"]);
    let names: Vec<&str> = deps["dependencies"]
        .as_array()
        .expect("dependencies array")
        .iter()
        .map(|d| d["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"lodash"), "single-package deps: {deps}");

    let agg = repo_json_at(&path, &["repo", "--json"]);
    assert_eq!(agg["package_manager"], "npm", "aggregate pm: {agg}");
    assert_eq!(agg["package_count"], 1, "aggregate package_count: {agg}");
    assert_eq!(
        agg["packages"],
        serde_json::json!(["node-app"]),
        "aggregate packages: {agg}"
    );
}

#[test]
fn test_single_package_python_reports_root_facts() {
    let (_dir, path) = create_test_repo();
    std::fs::write(
        path.join("pyproject.toml"),
        "[project]\nname = \"py-app\"\nversion = \"2.1.0\"\ndependencies = [\"requests>=2\"]\n",
    )
    .unwrap();

    let pm = repo_json_at(&path, &["repo", "package-manager", "--json"]);
    assert_eq!(pm["package_manager"], "pip");

    let deps = repo_json_at(&path, &["repo", "dependencies", "--json"]);
    let names: Vec<&str> = deps["dependencies"]
        .as_array()
        .expect("dependencies array")
        .iter()
        .map(|d| d["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"requests"), "single-package deps: {deps}");

    let agg = repo_json_at(&path, &["repo", "--json"]);
    assert_eq!(agg["package_manager"], "pip", "aggregate pm: {agg}");
    assert_eq!(agg["package_count"], 1, "aggregate package_count: {agg}");
    assert_eq!(
        agg["packages"],
        serde_json::json!(["py-app"]),
        "aggregate packages: {agg}"
    );
}

#[test]
fn test_repo_dependencies_filters_dev_dependencies() {
    let (_dir, path) = create_test_repo();
    std::fs::write(
        path.join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\"]\nresolver = \"3\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(path.join("app")).unwrap();
    std::fs::write(
        path.join("app/Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
         [dependencies]\nserde = \"1\"\n\n[dev-dependencies]\ninsta = \"1\"\n",
    )
    .unwrap();

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dependencies",
            "--dev-dependencies",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .clone();
    assert!(
        output.stderr.is_empty(),
        "repo dependencies --json must not emit hints or legends to stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("dependencies JSON is valid");
    let deps = json["dependencies"]
        .as_array()
        .expect("dependencies key is an array");

    assert_eq!(deps.len(), 1, "only dev dependencies should be emitted: {json}");
    assert_eq!(deps[0]["name"], "insta");
    assert_eq!(deps[0]["family"], "dev_dependencies");
    assert_eq!(deps[0]["package"], "app");
}

#[test]
fn test_repo_aggregate_dependencies_are_cwd_invariant() {
    // The bare `repo --json` aggregate classifies `dependencies` as a repo-wide
    // group-A fact: it must report the same set regardless of where in the tree
    // the command runs. Two packages declare distinct external dependencies; the
    // aggregate's `.dependencies.dependencies` must be byte-identical whether
    // invoked at the repo root or inside one package directory.
    let (_dir, path) = create_test_repo();
    std::fs::write(
        path.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/app-a\", \"crates/app-b\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(path.join("crates/app-a/src")).unwrap();
    std::fs::write(
        path.join("crates/app-a/Cargo.toml"),
        "[package]\nname = \"app-a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
         [dependencies]\nserde = \"1\"\n",
    )
    .unwrap();
    std::fs::write(path.join("crates/app-a/src/lib.rs"), "pub fn a() {}\n").unwrap();
    std::fs::create_dir_all(path.join("crates/app-b/src")).unwrap();
    std::fs::write(
        path.join("crates/app-b/Cargo.toml"),
        "[package]\nname = \"app-b\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
         [dependencies]\nrand = \"0.8\"\n",
    )
    .unwrap();
    std::fs::write(path.join("crates/app-b/src/lib.rs"), "pub fn b() {}\n").unwrap();

    let aggregate_deps = |base: &Path| -> Value {
        let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
            .args(["--base", base.to_str().unwrap(), "repo", "--json"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let json: Value = serde_json::from_slice(&output).expect("aggregate JSON is valid");
        json["dependencies"]["dependencies"].clone()
    };

    let from_root = aggregate_deps(&path);
    let from_package = aggregate_deps(&path.join("crates/app-a"));

    // Repo-wide set must include both packages' externals from either vantage.
    let names: Vec<&str> = from_root
        .as_array()
        .expect("dependencies is an array")
        .iter()
        .filter_map(|d| d["name"].as_str())
        .collect();
    assert!(
        names.contains(&"serde") && names.contains(&"rand"),
        "repo-root aggregate must list both packages' externals: {from_root}"
    );

    assert_eq!(
        from_root, from_package,
        "aggregate `.dependencies.dependencies` must be identical at the repo root and inside a package"
    );
}

#[test]
fn test_invalid_refresh_remotes_on_remote_subcommand_fails() {
    // --refresh-remotes is only valid on git-status, not on remote
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "remote", "origin", "--refresh-remotes"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--refresh-remotes"));
}

// ============================================================================
// Verbose Flag Tests with Subcommands
// ============================================================================

#[test]
fn test_verbose_with_software_adds_columns() {
    // In a non-TTY context, terminal width defaults to 80 columns which may be
    // too narrow for the verbose programs table. Accept either the rendered table
    // or the graceful width error message.
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "-v"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Binary")
                .or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_verbose_with_hardware_shows_details() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["hardware", "-v"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Total:"));
}

// ============================================================================
// Invalid Subcommand Tests
// ============================================================================

#[test]
fn test_invalid_subcommand_fails() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("invalid-subcommand")
        .assert()
        .failure();
}

#[test]
fn test_old_flag_syntax_fails() {
    // Old --hardware flag should not work (not a valid subcommand or flag)
    assert_cmd::Command::cargo_bin("sniff").unwrap().arg("--hardware").assert().failure();
}

// ============================================================================
// Remote Subcommand Tests
// ============================================================================

#[test]
fn test_repo_remote_help() {
    // Remote subcommand is documented via `sniff repo --help`
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("remote"));
}

#[test]
fn test_help_mentions_remote_via_repo() {
    // Remote inspection is now under `sniff repo --help`
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sniff repo remote origin"))
        .stdout(predicate::str::contains("Inspect the 'origin' remote"));
}

// ============================================================================
// Install Subcommand Tests
// ============================================================================

#[test]
fn test_software_editors_shows_table_without_install() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "editors"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Name").or(predicate::str::contains("could not be rendered")),
        );
}

#[test]
fn test_software_editors_install_invalid_name_fails() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "editors", "install", "nonexistent-editor-xyz"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown editor"))
        .stderr(predicate::str::contains("Valid names:"));
}

#[test]
fn test_software_utilities_install_invalid_name_fails() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "utilities", "install", "nonexistent-util-xyz"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown utility"))
        .stderr(predicate::str::contains("Valid names:"));
}

#[test]
fn test_software_install_invalid_name_fails() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "install", "nonexistent-program-xyz"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown program"));
}

#[test]
fn test_software_editors_install_help_works() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "editors", "install", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Install a program"));
}

#[test]
fn test_help_mentions_software_install() {
    // Top-level help mentions software editors with install support
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("sniff software"));
}

#[test]
fn test_software_editors_json_still_works_with_install_subcommand() {
    // --json flag should still work for listing (no install action)
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["software", "editors", "--json"])
        .assert()
        .success();
}

// ============================================================================
// --plain flag tests
// ============================================================================

#[test]
fn test_plain_flag_strips_escape_codes() {
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["os", "--plain"])
        .output()
        .expect("failed to run sniff os --plain");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // ANSI escape codes start with \x1b[
    assert!(
        !stdout.contains("\x1b["),
        "Plain output should not contain ANSI escape codes"
    );
}

#[test]
fn test_plain_with_json_ignores_plain() {
    // --plain --json should produce normal JSON (plain is irrelevant for JSON)
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["os", "--plain", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""));
}

// ============================================================================
// ============================================================================
// Repo subcommand tests (Phase 1 verification)
// ============================================================================

#[test]
fn test_repo_git_status_subcommand() {
    let (_dir, path) = create_test_repo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .current_dir(&path)
        .args(["repo", "git-status"])
        .assert()
        .success();
}

#[test]
fn test_repo_help_shows_examples() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sniff repo git-status"))
        .stdout(predicate::str::contains("sniff repo hash"))
        .stdout(predicate::str::contains("sniff repo staged-files"));
}

// ============================================================================
// Blast-radius CLI integration tests (temp-repo based)
// ============================================================================

/// Create a temp git repo with an initial commit.
fn create_test_repo() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();

    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test").unwrap();

    let sig = repo.signature().unwrap();
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
        .unwrap();

    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Commit a file to the test repo.
fn test_commit_file(repo_path: &Path, relative: &str, content: &str) {
    test_commit_file_with_message(repo_path, relative, content, "add file");
}

/// Commit a file to the test repo with a custom commit message.
fn test_commit_file_with_message(repo_path: &Path, relative: &str, content: &str, message: &str) {
    let full = repo_path.join(relative);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&full, content).unwrap();

    let repo = git2::Repository::open(repo_path).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(relative)).unwrap();
    index.write().unwrap();

    let sig = repo.signature().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head])
        .unwrap();
}

/// Overwrite the loose object file for `sha` with garbage so any decode fails.
/// Git creates objects read-only, so make the file writable first.
fn corrupt_loose_object(repo_path: &Path, sha: &str) {
    let obj_path = repo_path
        .join(".git")
        .join("objects")
        .join(&sha[..2])
        .join(&sha[2..]);
    let mut perms = std::fs::metadata(&obj_path).unwrap().permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o644);
    }
    #[cfg(not(unix))]
    perms.set_readonly(false);
    std::fs::set_permissions(&obj_path, perms).unwrap();
    std::fs::write(&obj_path, b"garbage").unwrap();
}

/// Flip the trailing checksum byte of the index so a read detects the mismatch.
fn corrupt_index(repo_path: &Path) {
    let index_path = repo_path.join(".git").join("index");
    let mut bytes = std::fs::read(&index_path).unwrap();
    let len = bytes.len();
    assert!(len >= 20, "index must have a trailing checksum to corrupt");
    bytes[len - 1] = bytes[len - 1].wrapping_add(1);
    std::fs::write(&index_path, bytes).unwrap();
}

/// A corrupt index must surface as a CLI failure through `repo has-merge-conflict`,
/// not be reported as a clean "no conflicts" result.
#[test]
fn test_repo_has_merge_conflict_surfaces_corrupt_index() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    corrupt_index(&path);

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "has-merge-conflict",
        ])
        .assert()
        .failure();
}

/// A corrupt commit object must surface as a CLI failure through `repo hash`,
/// not be reported as "commit not found".
#[test]
fn test_repo_hash_surfaces_corrupt_commit_object() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let repo = git2::Repository::open(&path).unwrap();
    let sha = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();
    corrupt_loose_object(&path, &sha);

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "hash", &sha])
        .assert()
        .failure();
}

/// Corrupt the HEAD commit object of a freshly-built test repo and return its
/// path, so corruption surfaces through any history-reading command.
fn repo_with_corrupt_head() -> (tempfile::TempDir, PathBuf) {
    let (dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let repo = git2::Repository::open(&path).unwrap();
    let sha = repo
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();
    corrupt_loose_object(&path, &sha);
    (dir, path)
}

/// A corrupt commit object must surface as a CLI failure through
/// `repo git-status`, not be reported as a clean, empty history.
#[test]
fn test_repo_git_status_surfaces_corrupt_history() {
    let (_dir, path) = repo_with_corrupt_head();

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "git-status"])
        .assert()
        .failure();
}

/// A corrupt commit object must surface as a CLI failure through
/// `repo recent-commits`, not produce a successful but empty list.
#[test]
fn test_repo_recent_commits_surfaces_corrupt_history() {
    let (_dir, path) = repo_with_corrupt_head();

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "recent-commits"])
        .assert()
        .failure();
}

/// A corrupt commit object must surface as a CLI failure through
/// `repo source-code-changes`, not produce a successful but empty report.
#[test]
fn test_repo_source_code_changes_surfaces_corrupt_history() {
    let (_dir, path) = repo_with_corrupt_head();

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "source-code-changes",
        ])
        .assert()
        .failure();
}

/// Pack the loose ref for the checked-out branch into `packed-refs` and delete
/// the loose file, mirroring `git pack-refs --all --prune`. Returns the branch.
fn pack_and_prune_head_branch(repo_path: &Path) -> String {
    let repo = git2::Repository::open(repo_path).unwrap();
    let branch = repo.head().unwrap().shorthand().unwrap().to_string();
    let git = repo_path.join(".git");
    let loose = git.join("refs").join("heads").join(&branch);
    let oid = std::fs::read_to_string(&loose).unwrap().trim().to_string();
    std::fs::write(
        git.join("packed-refs"),
        format!("# pack-refs with: peeled fully-peeled\n{oid} refs/heads/{branch}\n"),
    )
    .unwrap();
    std::fs::remove_file(&loose).unwrap();
    branch
}

/// A checked-out branch that lives only in `packed-refs` (after a pack + prune)
/// must still be reported by `repo git-status`, not collapse to a null branch.
#[test]
fn test_repo_git_status_reports_packed_checkout_branch() {
    let (_dir, path) = create_test_repo();
    let branch = pack_and_prune_head_branch(&path);

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "git-status",
            "--json",
        ])
        .assert()
        .success();

    let json: Value = serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(
        json.get("current_branch").and_then(|v| v.as_str()),
        Some(branch.as_str()),
        "packed checked-out branch must appear in git-status JSON: {json}"
    );
}

/// A malformed `refs/remotes/origin/main` must surface as a CLI failure through
/// `repo git-status --branch origin/main`, not be reported as an empty history.
#[test]
fn test_repo_git_status_branch_surfaces_malformed_remote_ref() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let remote_ref = path
        .join(".git")
        .join("refs")
        .join("remotes")
        .join("origin")
        .join("main");
    std::fs::create_dir_all(remote_ref.parent().unwrap()).unwrap();
    std::fs::write(&remote_ref, b"not a valid ref target\n").unwrap();

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "git-status",
            "--branch",
            "origin/main",
        ])
        .assert()
        .failure();
}

/// An absent branch whose name is hex but too short for an object-ID prefix
/// (`add`, 3 chars) is genuine absence — `repo git-status --branch add`
/// succeeds with empty history rather than failing as a malformed-SHA lookup.
#[test]
fn test_repo_git_status_branch_absent_short_hex_succeeds() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "git-status",
            "--branch",
            "add",
        ])
        .assert()
        .success();
}

/// A validly-shaped hex branch name that matches no object resolves to empty
/// history, not a CLI failure.
#[test]
fn test_repo_git_status_branch_absent_valid_length_hex_succeeds() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "git-status",
            "--branch",
            "abcdef12",
        ])
        .assert()
        .success();
}

/// An absent non-hex branch name resolves to empty history, not a CLI failure.
#[test]
fn test_repo_git_status_branch_absent_ordinary_name_succeeds() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "git-status",
            "--branch",
            "nonexistent",
        ])
        .assert()
        .success();
}

/// Stage a file in the test repo (no commit).
fn test_stage_file(repo_path: &Path, relative: &str, content: &str) {
    let full = repo_path.join(relative);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&full, content).unwrap();

    let repo = git2::Repository::open(repo_path).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(relative)).unwrap();
    index.write().unwrap();
}

#[test]
fn test_repo_dirty_source_code_returns_source_files() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    // Create a dirty source file
    std::fs::write(path.join("src/main.rs"), "fn main() { dirty }").unwrap();

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-source-code",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"));
}

#[test]
fn test_repo_staged_source_code_returns_staged_only() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/a.rs", "a");
    test_commit_file(&path, "src/b.rs", "b");

    // Stage a change to a.rs only
    test_stage_file(&path, "src/a.rs", "a modified");
    // Modify b.rs without staging
    std::fs::write(path.join("src/b.rs"), "b modified").unwrap();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "staged-source-code",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("a.rs"), "Should contain staged file a.rs");
    assert!(
        !stdout.contains("b.rs"),
        "Should not contain unstaged file b.rs"
    );
}

#[test]
fn test_repo_staged_files_uses_new_path() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    test_commit_file(&path, "docs/guide.md", "# Guide");

    // Stage changes to both
    test_stage_file(&path, "src/main.rs", "fn main() { updated }");
    test_stage_file(&path, "docs/guide.md", "# Updated Guide");

    // staged-files should now go through the new path (all files, not just source)
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "staged-files",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("main.rs"), "Should contain source file");
    assert!(stdout.contains("guide.md"), "Should contain markdown file");
}

#[test]
fn test_repo_staged_files_json_uses_new_shape() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    test_stage_file(&path, "src/main.rs", "fn main() { updated }");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "staged-files",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    assert_eq!(json["scope"], "staged", "scope should be lowercase");
    assert_eq!(json["kind"], "all_files", "kind should be snake_case");
    let paths = json["paths"].as_array().expect("paths should be an array");
    assert!(
        paths
            .iter()
            .any(|p| p.as_str().unwrap().contains("main.rs"))
    );
}

#[test]
fn test_repo_unstaged_files_json_uses_new_shape() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    // Modify without staging
    std::fs::write(path.join("src/main.rs"), "fn main() { updated }").unwrap();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "unstaged-files",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    assert_eq!(json["scope"], "unstaged", "scope should be lowercase");
    assert_eq!(json["kind"], "all_files", "kind should be snake_case");
    let paths = json["paths"].as_array().expect("paths should be an array");
    assert!(
        paths
            .iter()
            .any(|p| p.as_str().unwrap().contains("main.rs"))
    );
}

#[test]
fn test_repo_untracked_files_json_uses_new_shape() {
    let (_dir, path) = create_test_repo();
    // Create a new file without adding it to git
    std::fs::write(path.join("new_file.rs"), "// new").unwrap();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "untracked-files",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    assert_eq!(json["scope"], "untracked", "scope should be lowercase");
    assert_eq!(json["kind"], "all_files", "kind should be snake_case");
    let paths = json["paths"].as_array().expect("paths should be an array");
    assert!(
        paths
            .iter()
            .any(|p| p.as_str().unwrap().contains("new_file.rs"))
    );
}

#[test]
fn test_repo_dirty_files_returns_all_file_types() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    test_commit_file(&path, "config.json", "{}");

    // Dirty both files
    std::fs::write(path.join("src/main.rs"), "fn main() { dirty }").unwrap();
    std::fs::write(path.join("config.json"), "{\"key\": true}").unwrap();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-files",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("config.json"));
}

#[test]
fn test_repo_file_list_no_results_exits_1() {
    let (_dir, path) = create_test_repo();
    // Commit a file, no dirty files
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-source-code",
        ])
        .assert()
        .code(1);
}

#[test]
fn test_repo_file_list_no_error_exits_0() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-source-code",
            "--no-error",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_file_list_on_error_to_stderr() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-source-code",
            "--on-error",
            "No dirty source code found",
            "--plain",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("No dirty source code found"));
}

#[test]
fn test_repo_file_list_on_error_plus_no_error_to_stdout() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-source-code",
            "--no-error",
            "--on-error",
            "clean!",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("clean!"));
}

#[test]
fn test_blast_radius_dirty_matches_documents() {
    let (_dir, path) = create_test_repo();
    // Commit source and doc
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    let doc = "---\ntitle: Guide\nblast_radius:\n  - src/main.rs\n---\n# Guide\n";
    test_commit_file(&path, "docs/guide.md", doc);

    // Dirty the source file
    std::fs::write(path.join("src/main.rs"), "fn main() { changed }").unwrap();

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "blast-radius",
            "dirty",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("docs/guide.md"));
}

#[test]
fn test_blast_radius_staged_matches_documents() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    let doc = "---\ntitle: Guide\nblast_radius:\n  - src/main.rs\n---\n# Guide\n";
    test_commit_file(&path, "docs/guide.md", doc);

    // Stage a modification
    test_stage_file(&path, "src/main.rs", "fn main() { staged }");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "blast-radius",
            "staged",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("docs/guide.md"));
}

#[test]
fn test_blast_radius_last_commit_matches_documents() {
    let (_dir, path) = create_test_repo();
    let doc = "---\ntitle: Guide\nblast_radius:\n  - src/main.rs\n---\n# Guide\n";
    test_commit_file(&path, "docs/guide.md", doc);
    // Commit the source file last (it will be in HEAD)
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "blast-radius",
            "last-commit",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("docs/guide.md"));
}

#[test]
fn test_blast_radius_no_matches_exits_1() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    // No dirty files -> no blast radius matches

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "blast-radius", "dirty"])
        .assert()
        .code(1);
}

#[test]
fn test_blast_radius_no_error_exits_0() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "blast-radius",
            "dirty",
            "--no-error",
        ])
        .assert()
        .success();
}

#[test]
fn test_blast_radius_on_error_to_stderr() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "blast-radius",
            "dirty",
            "--on-error",
            "No docs affected",
            "--plain",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("No docs affected"));
}

#[test]
fn test_blast_radius_json_output() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    let doc = "---\ntitle: Guide\nblast_radius:\n  - src/main.rs\n---\n# Guide\n";
    test_commit_file(&path, "docs/guide.md", doc);
    std::fs::write(path.join("src/main.rs"), "fn main() { changed }").unwrap();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "blast-radius",
            "dirty",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    assert_eq!(json["scope"], "dirty", "scope should be lowercase");
    let docs = json["documents"]
        .as_array()
        .expect("documents should be an array");
    assert_eq!(docs.len(), 1);
    assert_eq!(
        docs[0].as_str().unwrap(),
        "docs/guide.md",
        "documents should be path strings"
    );
}

#[test]
fn test_blast_radius_list_format() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    let doc = "---\ntitle: Guide\nblast_radius:\n  - src/main.rs\n---\n# Guide\n";
    test_commit_file(&path, "docs/guide.md", doc);
    std::fs::write(path.join("src/main.rs"), "fn main() { changed }").unwrap();

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "blast-radius",
            "dirty",
            "--list",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("docs/guide.md"));
}

#[test]
fn test_docs_stdout_stderr_split() {
    let (_dir, path) = create_test_repo();
    test_commit_file(
        &path,
        "docs/readme.md",
        "---\ntitle: Readme\n---\n# Readme\n",
    );

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "docs", "--plain"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);

    // Header should be on stderr
    assert!(stderr.contains("Docs"), "Header should be on stderr");
    // Document list should be on stdout
    assert!(stdout.contains("readme.md"), "Doc list should be on stdout");
    // Footer should be on stderr
    assert!(stderr.contains("--verbose"), "Footer should be on stderr");
}

#[test]
fn test_docs_blast_radius_filter() {
    let (_dir, path) = create_test_repo();
    // Doc WITH blast_radius
    let doc_with = "---\ntitle: API Guide\nblast_radius:\n  - src/main.rs\n---\n# API Guide\n";
    test_commit_file(&path, "docs/api.md", doc_with);
    // Doc WITHOUT blast_radius
    let doc_without = "---\ntitle: Readme\n---\n# Readme\n";
    test_commit_file(&path, "docs/readme.md", doc_without);

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "docs",
            "--blast-radius",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("api.md"),
        "Should include doc with blast_radius"
    );
    assert!(
        !stdout.contains("readme.md"),
        "Should exclude doc without blast_radius"
    );
}

#[test]
fn test_repo_dirty_source_code_with_list_flag() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    std::fs::write(path.join("src/main.rs"), "fn main() { dirty }").unwrap();

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-source-code",
            "--list",
            "--plain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"));
}

#[test]
fn test_repo_unstaged_source_code_returns_modified_only() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/a.rs", "a");
    test_commit_file(&path, "src/b.rs", "b");

    // Stage a.rs
    test_stage_file(&path, "src/a.rs", "a staged");
    // Modify b.rs without staging
    std::fs::write(path.join("src/b.rs"), "b modified").unwrap();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "unstaged-source-code",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("b.rs"), "Should contain unstaged file b.rs");
    assert!(
        !stdout.contains("a.rs"),
        "Should not contain staged file a.rs"
    );
}

// ============================================================================
// Recent Commits CLI Integration Tests (Step 14)
// ============================================================================

#[test]
fn test_repo_recent_commits_default_period() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "recent-commits"])
        .assert()
        .success();
}

#[test]
fn test_repo_recent_commits_with_period() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "1d",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_recent_commits_with_count_period() {
    let (_dir, path) = create_test_repo();
    for i in 0..5 {
        test_commit_file(&path, &format!("src/file{i}.rs"), "fn main() {}");
    }

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "2",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let commits = json["commits"].as_array().expect("commits array");
    assert_eq!(commits.len(), 2, "expected exactly 2 commits");
    assert_eq!(
        json["period_label"].as_str().unwrap(),
        "last 2 commits",
        "period label should describe the count"
    );
}

#[test]
fn test_repo_source_code_changes_with_count_period() {
    let (_dir, path) = create_test_repo();
    for i in 0..3 {
        test_commit_file(&path, &format!("src/file{i}.rs"), "fn main() {}");
    }

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "source-code-changes",
            "2",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_recent_commits_with_json() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // JSON output should contain commit fields
    assert!(
        stdout.contains("\"commits\""),
        "JSON should have commits array"
    );
    assert!(
        stdout.contains("\"period_label\""),
        "JSON should have period_label"
    );
}

#[test]
fn test_repo_recent_commits_with_plain() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--plain",
        ])
        .output()
        .expect("failed to run sniff");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Plain output should not have ANSI escape codes
    assert!(
        !stdout.contains("\x1b["),
        "Plain output should not have ANSI escape codes"
    );
}

#[test]
fn test_repo_source_code_changes() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "source-code-changes",
            "1w",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_source_code_changes_with_json() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "source-code-changes",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("\"commits\""),
        "JSON should have commits array"
    );
}

#[test]
fn test_repo_documentation_changes() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "docs/guide.md", "# Guide\n");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "documentation-changes",
            "1w",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_documentation_changes_with_json() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "docs/guide.md", "# Guide\n");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "documentation-changes",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("\"commits\""),
        "JSON should have commits array"
    );
}

#[test]
fn test_source_code_changes_json_filters_commits_and_files() {
    // Two commits: one touches a source file, one touches only docs.
    // `source-code-changes --json` must keep only the source commit and
    // tag the payload with `"filter": "source_code"`.
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    test_commit_file(&path, "README.md", "# readme");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "source-code-changes",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");

    assert_eq!(
        value["filter"], "source_code",
        "source-code-changes --json must include `filter: source_code`: {value}"
    );

    let commits = value["commits"].as_array().expect("commits must be array");
    // Only the source-touching commit should remain after filtering.
    assert_eq!(
        commits.len(),
        1,
        "expected exactly one commit after source-code filtering: {value}"
    );

    // All files left in the kept commit must look like source code.
    for commit in commits {
        let files = commit["files"].as_array().expect("files must be array");
        assert!(!files.is_empty(), "filtered commit must keep its files");
        for file in files {
            let path_str = file["path"].as_str().expect("path is a string");
            assert!(
                !path_str.ends_with(".md"),
                "source-code filter must not keep markdown: {path_str}"
            );
        }
    }
}

#[test]
fn test_documentation_changes_json_filters_commits_and_files() {
    // Two commits: one touches a source file, one touches docs.
    // `documentation-changes --json` must keep only doc commits and tag
    // the payload with `"filter": "documentation"`.
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    test_commit_file(&path, "README.md", "# readme");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "documentation-changes",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");

    assert_eq!(
        value["filter"], "documentation",
        "documentation-changes --json must include `filter: documentation`: {value}"
    );

    let commits = value["commits"].as_array().expect("commits must be array");
    assert!(
        !commits.is_empty(),
        "expected at least one doc commit: {value}"
    );

    for commit in commits {
        let files = commit["files"].as_array().expect("files must be array");
        assert!(!files.is_empty(), "filtered commit must keep its files");
        for file in files {
            let path_str = file["path"].as_str().expect("path is a string");
            assert!(
                !path_str.ends_with(".rs"),
                "documentation filter must not keep .rs files: {path_str}"
            );
        }
    }
}

#[test]
fn test_filtered_commit_json_trims_packages() {
    // `source-code-changes` and `documentation-changes` should NOT include
    // the full `packages` metadata for brevity.
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    test_commit_file(&path, "README.md", "# readme");

    for (subcommand, label) in [
        ("source-code-changes", "source_code"),
        ("documentation-changes", "documentation"),
    ] {
        let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
            .args([
                "--base",
                path.to_str().unwrap(),
                "repo",
                subcommand,
                "--json",
            ])
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let value: Value = serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");

        assert_eq!(
            value["filter"], label,
            "{subcommand} --json must include `filter: {label}`"
        );
        assert!(
            value.get("packages").is_none(),
            "{subcommand} --json must NOT include full `packages` metadata: {value}"
        );
    }
}

#[test]
fn test_recent_commits_json_unchanged() {
    // Regression guard — `recent-commits --json` must NOT include the
    // `filter` field that the filtered variants add.
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    test_commit_file(&path, "README.md", "# readme");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");

    let obj = value.as_object().expect("payload must be a JSON object");
    assert!(
        !obj.contains_key("filter"),
        "recent-commits --json must NOT include `filter`: {value}"
    );
    assert!(
        obj.contains_key("commits"),
        "recent-commits --json must include `commits`"
    );
    assert!(
        obj.contains_key("period_label"),
        "recent-commits --json must include `period_label`"
    );
}

#[test]
fn test_repo_recent_commits_no_error_flag() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    // Use a future date - valid period that returns no commits
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "2099-01-01",
            "--no-error",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_recent_commits_invalid_period_error() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "invalid-period",
        ])
        .assert()
        .failure();
}

#[test]
fn test_repo_recent_commits_on_error_flag() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "2099-01-01",
            "--on-error",
            "No recent commits",
            "--plain",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("No recent commits"));
}

// ============================================================================
// Recent Commits CLI — Hash, Package, and Date routing tests
// ============================================================================

/// Create a monorepo-style test repo for CLI testing.
fn create_cli_monorepo() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();

    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test").unwrap();

    // Create workspace Cargo.toml
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[workspace]
members = ["pkg-a/lib", "pkg-b/lib"]
"#,
    )
    .unwrap();

    // Package A
    let pkg_a = dir.path().join("pkg-a/lib");
    std::fs::create_dir_all(pkg_a.join("src")).unwrap();
    std::fs::write(
        pkg_a.join("Cargo.toml"),
        r#"[package]
name = "pkg-a"
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();
    std::fs::write(pkg_a.join("src/lib.rs"), "pub fn a() {}").unwrap();

    // Package B
    let pkg_b = dir.path().join("pkg-b/lib");
    std::fs::create_dir_all(pkg_b.join("src")).unwrap();
    std::fs::write(
        pkg_b.join("Cargo.toml"),
        r#"[package]
name = "pkg-b"
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();
    std::fs::write(pkg_b.join("src/lib.rs"), "pub fn b() {}").unwrap();

    // Commit everything
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let sig = repo.signature().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial monorepo", &tree, &[])
        .unwrap();

    let path = dir.path().to_path_buf();
    (dir, path)
}

/// A standalone single-package Cargo project (no `[workspace]`). Detection
/// synthesizes a one-package catalog with `is_monorepo == false`, so this
/// exercises the non-monorepo override-validation path of `repo version`.
fn create_single_package_repo() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test").unwrap();

    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "solo"
version = "0.4.2"
edition = "2024"
"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/lib.rs"), "pub fn solo() {}").unwrap();

    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let sig = repo.signature().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial solo", &tree, &[])
        .unwrap();

    let path = dir.path().to_path_buf();
    (dir, path)
}

fn create_cli_monorepo_with_root_package() -> (tempfile::TempDir, PathBuf) {
    let (dir, path) = create_cli_monorepo();
    let repo = git2::Repository::open(&path).unwrap();

    std::fs::write(
        path.join("Cargo.toml"),
        r#"[package]
name = "root-tool"
version = "0.1.0"
edition = "2024"

[workspace]
members = ["pkg-a/lib", "pkg-b/lib", "."]
"#,
    )
    .unwrap();
    std::fs::create_dir_all(path.join("src")).unwrap();
    std::fs::write(path.join("src/lib.rs"), "pub fn root_tool() {}").unwrap();

    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let sig = repo.signature().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "add root workspace package",
        &tree,
        &[&parent],
    )
    .unwrap();

    (dir, path)
}

#[test]
fn test_repo_recent_commits_with_hash_period() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    test_commit_file(&path, "src/lib.rs", "pub fn lib() {}");

    let repo = git2::Repository::open(&path).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    // Get the parent commit hash to use as boundary
    let parent = head.parent(0).unwrap();
    let parent_hash = parent.id().to_string();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            &parent_hash,
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(!stdout.is_empty(), "Hash-based query should produce output");
}

#[test]
fn test_repo_recent_commits_with_today_period() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "today",
            "--plain",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_recent_commits_with_date_period() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "2020-01-01",
            "--plain",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_recent_commits_action_filter_single_action() {
    let (_dir, path) = create_test_repo();
    test_commit_file_with_message(
        &path,
        "src/feature.rs",
        "pub fn feature() {}",
        "feat(cli): add action filter",
    );
    test_commit_file_with_message(
        &path,
        "src/fix.rs",
        "pub fn fix() {}",
        "fix(cli): tighten recent commit filtering",
    );

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--action",
            "feat",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
    let commits = json["commits"]
        .as_array()
        .expect("Should have commits array");

    assert_eq!(commits.len(), 1, "Only feat commits should remain");
    assert_eq!(
        commits[0]["description"].as_str(),
        Some("feat(cli): add action filter")
    );
}

#[test]
fn test_repo_recent_commits_action_filter_or_semantics() {
    let (_dir, path) = create_test_repo();
    test_commit_file_with_message(
        &path,
        "src/feature.rs",
        "pub fn feature() {}",
        "feat(cli): add action filter",
    );
    test_commit_file_with_message(
        &path,
        "src/refactor.rs",
        "pub fn refactor() {}",
        "refactor(cli): simplify commit filtering",
    );
    test_commit_file_with_message(
        &path,
        "src/fix.rs",
        "pub fn fix() {}",
        "fix(cli): tighten recent commit filtering",
    );

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--action",
            "feat",
            "--action",
            "refactor",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
    let commits = json["commits"]
        .as_array()
        .expect("Should have commits array");
    let descriptions: Vec<&str> = commits
        .iter()
        .filter_map(|commit| commit["description"].as_str())
        .collect();

    assert_eq!(
        descriptions.len(),
        2,
        "feat and refactor commits should remain"
    );
    assert!(descriptions.contains(&"feat(cli): add action filter"));
    assert!(descriptions.contains(&"refactor(cli): simplify commit filtering"));
    assert!(!descriptions.contains(&"fix(cli): tighten recent commit filtering"));
}

#[test]
fn test_repo_recent_commits_package_filter() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/src/lib.rs", "pub fn a2() {}");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--package",
            "pkg-a",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        !stdout.is_empty(),
        "Package-filtered query should produce output"
    );
}

#[test]
fn test_repo_recent_commits_package_area_filter() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-b/lib/src/lib.rs", "pub fn b2() {}");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--package-area",
            "pkg-b",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        !stdout.is_empty(),
        "Package-area filtered query should produce output"
    );
}

#[test]
fn test_repo_recent_commits_package_json_scoped() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/src/lib.rs", "pub fn a2() {}");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--package",
            "pkg-a",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // The packages array should only contain the filtered package
    if let Some(packages) = json["packages"].as_array() {
        for pkg in packages {
            assert_eq!(
                pkg["name"], "pkg-a",
                "JSON packages should be scoped to the filter"
            );
        }
    }

    // No files from pkg-b should appear in any commit
    if let Some(commits) = json["commits"].as_array() {
        for commit in commits {
            if let Some(files) = commit["files"].as_array() {
                for file in files {
                    let f = file.as_str().unwrap_or("");
                    assert!(
                        !f.starts_with("pkg-b/"),
                        "Filtered JSON should not contain pkg-b files, got: {}",
                        f
                    );
                }
            }
        }
    }
}

#[test]
fn test_repo_recent_commits_unknown_package_error() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/src/lib.rs", "pub fn a2() {}");

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--package",
            "nonexistent",
        ])
        .assert()
        .failure();
}

// ============================================================================
// Recent Commits CLI — Empty commit and exact payload tests
// ============================================================================

#[test]
fn test_repo_recent_commits_json_includes_empty_commits() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    // Create an empty commit on top
    let repo = git2::Repository::open(&path).unwrap();
    let sig = repo.signature().unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = head.tree().unwrap();
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "chore: empty marker",
        &tree,
        &[&head],
    )
    .unwrap();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
    let commits = json["commits"]
        .as_array()
        .expect("Should have commits array");

    // Find the empty commit
    let empty = commits
        .iter()
        .find(|c| c["description"].as_str() == Some("chore: empty marker"));
    assert!(empty.is_some(), "Empty commit should appear in JSON output");
    let empty = empty.unwrap();
    let files = empty["files"].as_array().expect("Should have files array");
    assert!(files.is_empty(), "Empty commit should have files: []");
}

#[test]
fn test_repo_recent_commits_json_exact_commit_fields() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
    let commits = json["commits"]
        .as_array()
        .expect("Should have commits array");

    // Should have at least 2 commits (initial + add file)
    assert!(
        commits.len() >= 2,
        "Should have at least 2 commits, got {}",
        commits.len()
    );

    // Verify each commit has required fields
    for commit in commits {
        assert!(commit["hash"].is_string(), "Commit should have hash");
        assert!(
            commit["datetime"].is_string(),
            "Commit should have datetime"
        );
        assert!(commit["files"].is_array(), "Commit should have files array");
        assert!(
            commit["description"].is_string(),
            "Commit should have description"
        );
        assert!(
            commit["bullet_points"].is_array(),
            "Commit should have bullet_points"
        );
    }
}

#[test]
fn test_repo_source_code_changes_json_exact_fields() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "source-code-changes",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
    let commits = json["commits"]
        .as_array()
        .expect("Should have commits array");

    // At least one commit should have a .rs file
    let has_rs_file = commits.iter().any(|c| {
        c["files"].as_array().is_some_and(|files| {
            files
                .iter()
                .any(|f| f["path"].as_str().is_some_and(|s| s.ends_with(".rs")))
        })
    });
    assert!(has_rs_file, "Source code changes should include .rs files");
}

#[test]
fn test_repo_documentation_changes_json_exact_fields() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "docs/guide.md", "# Guide\n");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "documentation-changes",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("Output should be valid JSON");
    let commits = json["commits"]
        .as_array()
        .expect("Should have commits array");

    // At least one commit should have a .md file
    let has_md_file = commits.iter().any(|c| {
        c["files"].as_array().is_some_and(|files| {
            files
                .iter()
                .any(|f| f["path"].as_str().is_some_and(|s| s.ends_with(".md")))
        })
    });
    assert!(
        has_md_file,
        "Documentation changes should include .md files"
    );
}

#[test]
fn test_repo_recent_commits_plain_output_exact_structure() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--plain",
        ])
        .output()
        .expect("failed to run sniff");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Plain output should contain markdown structure
    assert!(
        stdout.contains("[") && stdout.contains("] at "),
        "Plain output should have `[hash] at TIME` commit markers, got:\n{stdout}"
    );
    assert!(
        stdout.contains("**Files Impacted:**"),
        "Plain output should have files section, got:\n{stdout}"
    );
    assert!(
        stdout.contains("add file"),
        "Plain output should include the commit description, got:\n{stdout}"
    );
    assert!(
        stdout.contains("src/main.rs"),
        "Plain output should list the committed file, got:\n{stdout}"
    );
}

// ============================================================================
// repo packages Subcommand Tests
// ============================================================================

#[test]
fn test_repo_packages_csv_default() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-a, pkg-b");
}

#[test]
fn test_repo_packages_md_format() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--md",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "- pkg-a\n- pkg-b");
}

#[test]
fn test_repo_packages_list_format() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--list",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-a\npkg-b");
}

#[test]
fn test_repo_packages_md_and_list_conflict() {
    let (_dir, path) = create_cli_monorepo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--md",
            "--list",
        ])
        .assert()
        .failure();
}

#[test]
fn test_repo_packages_package_area_filter() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--package-area",
            "pkg-b",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-b");
}

#[test]
fn test_repo_packages_verbose_shows_root_dir() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--list",
            "--verbose",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("pkg-a(./pkg-a/lib)"),
        "Verbose list output should include the package root, got:\n{stdout}"
    );
    assert!(
        stdout.contains("pkg-b(./pkg-b/lib)"),
        "Verbose list output should include the package root, got:\n{stdout}"
    );
}

#[test]
fn test_repo_packages_verbose_does_not_emit_tracing() {
    let (_dir, path) = create_cli_monorepo();
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--verbose",
        ])
        .output()
        .expect("failed to run sniff");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("performance stage complete"),
        "--verbose must not leak tracing output to stderr, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("INFO"),
        "--verbose must not emit INFO tracing, got:\n{stderr}"
    );
}

#[test]
fn test_repo_packages_json_output() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("output should be valid JSON");
    let names = json.as_array().expect("top-level JSON must be an array");
    assert_eq!(
        names
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["pkg-a", "pkg-b"]
    );
}

#[test]
fn test_repo_packages_no_error_empty_filter() {
    let (_dir, path) = create_cli_monorepo();
    // Filter that matches nothing — without --no-error should exit 1
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "nonexistent",
            "--plain",
        ])
        .assert()
        .failure();
}

#[test]
fn test_repo_packages_no_error_allows_empty_filter() {
    let (_dir, path) = create_cli_monorepo();
    // Filter that matches nothing — with --no-error should exit 0
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "nonexistent",
            "--no-error",
            "--plain",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_packages_on_error_message() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "nonexistent",
            "--on-error",
            "nothing here",
            "--plain",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("nothing here"),
        "stderr should contain custom error message, got: {stderr}"
    );
}

#[test]
fn test_repo_packages_no_error_json_empty_filter() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "nonexistent",
            "--json",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("output should be valid JSON");
    let names = json.as_array().expect("top-level JSON must be an array");
    assert!(names.is_empty());
}

#[test]
fn test_repo_packages_no_error_json_with_flag() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "nonexistent",
            "--no-error",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("output should be valid JSON");
    let names = json.as_array().expect("top-level JSON must be an array");
    assert!(names.is_empty());
}

// ============================================================================
// repo package-areas Subcommand Tests
// ============================================================================

#[test]
fn test_repo_package_areas_csv_default() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-a, pkg-b");
}

#[test]
fn test_repo_package_areas_md_format() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--md",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "- pkg-a\n- pkg-b");
}

#[test]
fn test_repo_package_areas_list_format() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--list",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-a\npkg-b");
}

#[test]
fn test_repo_package_areas_md_and_list_conflict() {
    let (_dir, path) = create_cli_monorepo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--md",
            "--list",
        ])
        .assert()
        .failure();
}

#[test]
fn test_repo_package_areas_package_area_filter() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--package-area",
            "pkg-b",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-b");
}

#[test]
fn test_repo_package_areas_positional_filter() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "pkg-a",
            "--list",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-a");
}

#[test]
fn test_repo_package_areas_positional_filter_negation() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "!pkg-a",
            "--list",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-b");
}

#[test]
fn test_repo_package_areas_verbose_shows_root_dir() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--list",
            "--verbose",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("pkg-a (./pkg-a)"),
        "Verbose list output should include the area root, got:\n{stdout}"
    );
    assert!(
        stdout.contains("pkg-b (./pkg-b)"),
        "Verbose list output should include the area root, got:\n{stdout}"
    );
}

#[test]
fn test_repo_package_areas_root_area_verbose_renders_dot_slash() {
    let (_dir, path) = create_cli_monorepo_with_root_package();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--list",
            "--verbose",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("root (./)"),
        "Root package area should render as the repo root, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("root (./root)"),
        "Root package area should not render a non-existent root directory, got:\n{stdout}"
    );
}

#[test]
fn test_repo_package_areas_verbose_does_not_emit_tracing() {
    let (_dir, path) = create_cli_monorepo();
    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--verbose",
        ])
        .output()
        .expect("failed to run sniff");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("performance stage complete"),
        "--verbose must not leak tracing output to stderr, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("INFO"),
        "--verbose must not emit INFO tracing, got:\n{stderr}"
    );
}

#[test]
fn test_repo_package_areas_json_output() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("output should be valid JSON");
    let names = json.as_array().expect("top-level JSON must be an array");
    assert_eq!(
        names
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["pkg-a", "pkg-b"]
    );
}

#[test]
fn test_repo_package_areas_json_perf_stdout_is_valid_json() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "--json",
            "--perf",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("output should be valid JSON");
    let names = json.as_array().expect("top-level JSON must be an array");
    assert_eq!(
        names
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["pkg-a", "pkg-b"]
    );

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        !stderr.trim().is_empty(),
        "perf output should be written to stderr"
    );
    assert!(
        stderr.contains("Performance") || stderr.contains("Total"),
        "stderr should contain performance timing text, got:\n{stderr}"
    );
}

#[test]
fn test_repo_package_areas_no_error_empty_filter() {
    let (_dir, path) = create_cli_monorepo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "nonexistent",
            "--plain",
        ])
        .assert()
        .failure();
}

#[test]
fn test_repo_package_areas_no_error_allows_empty_filter() {
    let (_dir, path) = create_cli_monorepo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "nonexistent",
            "--no-error",
            "--plain",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_package_areas_on_error_message() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "nonexistent",
            "--on-error",
            "no areas",
            "--plain",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("no areas"),
        "stderr should contain custom error message, got: {stderr}"
    );
}

#[test]
fn test_repo_package_areas_no_error_json_empty_filter() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "nonexistent",
            "--json",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("output should be valid JSON");
    let names = json.as_array().expect("top-level JSON must be an array");
    assert!(names.is_empty());
}

#[test]
fn test_repo_package_areas_no_error_json_with_flag() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-areas",
            "nonexistent",
            "--no-error",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let json: Value = serde_json::from_str(&stdout).expect("output should be valid JSON");
    let names = json.as_array().expect("top-level JSON must be an array");
    assert!(names.is_empty());
}

#[test]
fn test_repo_root_json_perf_stdout_is_valid_json() {
    // `repo root --json --perf` must produce parseable JSON on stdout.
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "root", "--json", "--perf"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let _: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert!(stdout.contains("root"), "should contain root key");
}

#[test]
fn test_repo_root_is_absolute_without_base_from_subdir() {
    // Regression: discovering with the default "." (no --base) from a
    // subdirectory must still print an absolute root, not a relative ".."/".".
    let (_dir, repo_path) = create_test_repo();
    let subdir = repo_path.join("nested/deep");
    std::fs::create_dir_all(&subdir).unwrap();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .current_dir(&subdir)
        .args(["repo", "root"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let printed = stdout.trim();
    let root = Path::new(printed);
    assert!(
        root.is_absolute(),
        "root must be absolute, got: {printed:?}"
    );
    assert!(
        !printed.ends_with('/'),
        "root must not have a trailing separator, got: {printed:?}"
    );
    assert_eq!(
        std::fs::canonicalize(root).unwrap(),
        std::fs::canonicalize(&repo_path).unwrap(),
        "root must resolve to the repository working directory"
    );
}

#[test]
fn test_repo_dirty_files_json_perf_stdout_is_valid_json() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    std::fs::write(path.join("src/main.rs"), "fn main() { dirty }").unwrap();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-files",
            "--json",
            "--perf",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let _: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
}

#[test]
fn test_repo_recent_commits_json_perf_stdout_is_valid_json() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "recent-commits",
            "--json",
            "--perf",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let _: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert!(stdout.contains("commits"), "should contain commits key");
}

#[test]
fn test_repo_has_merge_conflict_json_perf_stdout_is_valid_json() {
    let (_dir, path) = create_test_repo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "has-merge-conflict",
            "--json",
            "--perf",
        ])
        .assert();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let _: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
}

// ============================================================================
// Phase 2 — Stable JSON shape for `package` / `package-area` / `root`
// when the result resolves to empty.
// ============================================================================
//
// JSON consumers must always see a stable object even when the lookup
// resolves to nothing. Text mode emits prose via `handle_no_results`, but
// JSON mode emits `{ "name": "" }` (or `{ "root": "" }`) and exits 1.

#[test]
fn test_package_json_empty_name_stable_shape() {
    // A bare git repo with no packages — `repo package --json` must emit
    // `{ "name": "" }` instead of prose / no output.
    let (_dir, path) = create_test_repo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package",
            "--json",
        ])
        .assert();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["name"], Value::String(String::new()));
}

#[test]
fn test_package_area_json_empty_name_stable_shape() {
    let (_dir, path) = create_test_repo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-area",
            "--json",
        ])
        .assert();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["name"], Value::String(String::new()));
}

#[test]
fn test_root_json_outside_git_repo_stable_shape() {
    // Pointing `--base` at a non-git directory must still emit
    // `{ "root": "" }` so JSON consumers see a stable shape rather than
    // a Box<dyn Error> bubble.
    let dir = tempfile::tempdir().unwrap();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            dir.path().to_str().unwrap(),
            "repo",
            "root",
            "--json",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["root"], Value::String(String::new()));
}

// ============================================================================
// `repo {dirty,staged,unstaged}-{packages,package-areas} --json` Shape Tests
// ============================================================================
//
// Phase 3 of the `incorrect-json` feature: every package/area family
// subcommand returns `{ scope, kind, names }` instead of the full RepoInfo
// blob. Non-monorepo repos return an empty `names` array, NOT a prose
// "only intended to be used in a monorepo" error string.

fn assert_package_family_shape_when_non_monorepo(
    subcommand: &str,
    expected_scope: &str,
    expected_kind: &str,
) {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    // Modify so there's something to scan.
    std::fs::write(path.join("src/main.rs"), "fn main() { dirty }").unwrap();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            subcommand,
            "--json",
        ])
        .assert();

    // Accept either exit code: text mode exits 1 on empty rendered output,
    // but JSON mode is structurally well-formed regardless.
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    let json: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("expected JSON for `{subcommand}`, got: {stdout:?} ({e})"));
    assert_eq!(
        json["scope"], expected_scope,
        "scope mismatch for {subcommand}"
    );
    assert_eq!(
        json["kind"], expected_kind,
        "kind mismatch for {subcommand}"
    );
    let names = json["names"]
        .as_array()
        .unwrap_or_else(|| panic!("expected `names` array for {subcommand}, got: {stdout}"));
    // Non-monorepo: empty array, not prose error.
    assert!(
        names.is_empty(),
        "non-monorepo repo should produce empty names; got {names:?}"
    );
}

#[test]
fn test_dirty_packages_json_shape() {
    assert_package_family_shape_when_non_monorepo("dirty-packages", "dirty", "packages");
}

#[test]
fn test_dirty_package_areas_json_shape() {
    assert_package_family_shape_when_non_monorepo("dirty-package-areas", "dirty", "package_areas");
}

#[test]
fn test_staged_packages_json_shape() {
    assert_package_family_shape_when_non_monorepo("staged-packages", "staged", "packages");
}

#[test]
fn test_staged_package_areas_json_shape() {
    assert_package_family_shape_when_non_monorepo(
        "staged-package-areas",
        "staged",
        "package_areas",
    );
}

#[test]
fn test_unstaged_packages_json_shape() {
    assert_package_family_shape_when_non_monorepo("unstaged-packages", "unstaged", "packages");
}

#[test]
fn test_unstaged_package_areas_json_shape() {
    assert_package_family_shape_when_non_monorepo(
        "unstaged-package-areas",
        "unstaged",
        "package_areas",
    );
}

#[test]
fn test_dirty_packages_json_does_not_emit_prose_error_for_non_monorepo() {
    // Regression: the legacy text-mode renderer returns
    // "- the \"--dirty-packages\" switch is only intended to be used in a monorepo"
    // for non-monorepos. JSON consumers must NEVER see that prose string —
    // they must see an empty `names` array.
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-packages",
            "--json",
        ])
        .assert()
        .get_output()
        .clone();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("only intended to be used in a monorepo"),
        "JSON output leaked the prose error string: {stdout}"
    );
    let json: Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(json["names"], Value::Array(vec![]));
}

// ============================================================================
// Phase 5 — `package-dependencies --json` builder
// ============================================================================

/// `sniff repo package-dependencies --json` must return a `{ packages: [...] }` object,
/// not the full `RepoInfo` blob.
///
/// The created test repo is a non-monorepo so `packages` will be empty;
/// the assertion focuses on the top-level shape (object with `packages`
/// array) and the absence of `RepoInfo`-only fields like `is_monorepo`.
#[test]
fn test_repo_deps_json_shape() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "package-dependencies",
            "--json",
        ])
        .assert()
        .get_output()
        .clone();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("expected JSON, got: {stdout:?} ({e})"));

    assert!(
        json.is_object(),
        "package-dependencies --json must return an object, got: {json}"
    );
    assert!(
        json["packages"].is_array(),
        "package-dependencies --json must have `packages` array, got: {json}"
    );
    // Must NOT leak full RepoInfo blob fields.
    assert!(
        json.get("is_monorepo").is_none(),
        "package-dependencies --json must not include `is_monorepo`: {json}"
    );
}

// ============================================================================
// `repo pr` Subcommand Tests
// ============================================================================

#[test]
fn test_repo_pr_help_documents_bitbucket_draft_limitation() {
    // The --status flag's help text must call out the Bitbucket draft
    // limitation so users know `--status draft` returns nothing for
    // Bitbucket-hosted repositories.
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "pr", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--status"))
        .stdout(predicate::str::contains("Bitbucket"));
}

// ============================================================================
// Phase 4 — locator and boolean JSON shapes
// ============================================================================

/// `has-merge-conflict --json` on a clean repo must:
///   - exit 1 (no conflict)
///   - emit `{ "has_merge_conflict": false }` on stdout
#[test]
fn test_has_merge_conflict_json_false() {
    let (_dir, path) = create_test_repo();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "has-merge-conflict",
            "--json",
        ])
        .assert()
        .failure()
        .code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["has_merge_conflict"], Value::Bool(false));
}

/// `is-current-package-area-dirty --json` always emits a `{ "dirty": <bool> }`
/// object, even when invoked outside any package area (where text mode would
/// also exit 1).
///
/// Note: the underlying detection only consults `RepoStatus.dirty` /
/// `RepoStatus.untracked`, which are populated only when `--refresh-remotes`
/// (deep git mode) is in effect. This test pins the JSON contract for the
/// `false` case; the `true` case is exercised at the pure-helper layer in
/// `output::filesystem::tests::boolean_helpers`.
#[test]
fn test_is_current_package_area_dirty_json_outside_area_emits_false() {
    let (_dir, path) = create_test_repo();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "is-current-package-area-dirty",
            "--json",
        ])
        .assert()
        .failure()
        .code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["dirty"], Value::Bool(false));
}

/// `is-current-package-area-dirty --json` on a clean monorepo package area
/// must exit 1 and emit `{ "dirty": false }`.
#[test]
fn test_is_current_package_area_dirty_json_clean() {
    let (_dir, path) = create_cli_monorepo();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.join("pkg-a").to_str().unwrap(),
            "repo",
            "is-current-package-area-dirty",
            "--json",
        ])
        .assert()
        .failure()
        .code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["dirty"], Value::Bool(false));
}

/// `package-area-has-source-code-changes --json` on a clean monorepo
/// package emits `{ "has_source_code_changes": false }` and exits 1.
///
/// Note: like the dirty check, the underlying detection consults
/// `RepoStatus.dirty` / `RepoStatus.untracked`, which are populated only
/// in deep git mode (`--refresh-remotes`). The `true` case is exercised
/// at the pure-helper layer in
/// `output::filesystem::tests::boolean_helpers`.
#[test]
fn test_package_area_has_source_code_changes_json_clean() {
    let (_dir, path) = create_cli_monorepo();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.join("pkg-a").to_str().unwrap(),
            "repo",
            "package-area-has-source-code-changes",
            "--json",
        ])
        .assert()
        .failure()
        .code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["has_source_code_changes"], Value::Bool(false));
}

/// `package-root --json` inside a known package emits `{ "root": "<abs path>" }`.
#[test]
fn test_package_root_json_when_present() {
    let (_dir, path) = create_cli_monorepo();
    let pkg_a_lib = path.join("pkg-a/lib");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            pkg_a_lib.to_str().unwrap(),
            "repo",
            "package-root",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    let root = value["root"].as_str().expect("root must be a string");
    assert!(
        !root.is_empty(),
        "package-root must be non-empty inside a real package, got: {value}"
    );
    assert!(
        root.contains("pkg-a"),
        "package-root should resolve to the pkg-a directory, got: {root}"
    );
}

/// `package --json` inside a known package emits `{ "name": <pkg> }`.
#[test]
fn test_package_name_json() {
    let (_dir, path) = create_cli_monorepo();
    let pkg_a_lib = path.join("pkg-a/lib");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            pkg_a_lib.to_str().unwrap(),
            "repo",
            "package",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["name"], Value::String("pkg-a".to_string()));
}

// ============================================================================
// Phase 7 — End-to-end regression: every `repo` subcommand emits a
// distinguishable JSON shape, and `--perf` keeps working alongside the
// new shapes.
// ============================================================================

/// Build a monorepo fixture whose package name differs from its area name.
///
/// Used by the Phase 7 distinctness matrix: the legacy `create_cli_monorepo`
/// helper places each package in `<area>/lib`, so `package` and `package-area`
/// resolve to the same string and produce identical `{ "name": ... }`
/// payloads. The `incorrect-json` contract is about distinct *shapes per
/// subcommand* under realistic input — pick a fixture where the values
/// differ so the matrix exercises real-world distinctness without needing
/// shape-only comparisons.
fn create_cli_monorepo_distinct_area_and_package() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();

    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test").unwrap();

    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[workspace]
members = ["alpha/core", "alpha/cli", "beta/core"]
"#,
    )
    .unwrap();

    let members = [
        ("alpha/core", "alpha-core"),
        ("alpha/cli", "alpha-cli"),
        ("beta/core", "beta-core"),
    ];
    for (rel, name) in &members {
        let pkg = dir.path().join(rel);
        std::fs::create_dir_all(pkg.join("src")).unwrap();
        std::fs::write(
            pkg.join("Cargo.toml"),
            format!(
                r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"
"#
            ),
        )
        .unwrap();
        std::fs::write(pkg.join("src/lib.rs"), "pub fn entry() {}").unwrap();
    }

    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let sig = repo.signature().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial monorepo", &tree, &[])
        .unwrap();

    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Distinctness matrix for `sniff repo <subcommand> --json`.
///
/// The `incorrect-json` feature was triggered by every subcommand returning
/// the same `RepoInfo` blob. This test pins the contract: no two subcommands
/// in the matrix produce identical stdout payloads under a realistic
/// monorepo fixture.
///
/// The matrix deliberately covers every subcommand whose JSON shape changed
/// in Phases 2-6 (`git-status`, `package-dependencies`, the package/area family, the
/// locator family, the boolean family, and the commit-family filtered
/// variants). Bare `repo` and `repo structure` are intentionally excluded —
/// they're meant to be identical (`structure` is the canonical alias).
///
/// The fixture is a monorepo where package names differ from area names
/// (`alpha-core` in area `alpha`, etc.) so `package` vs `package-area` and
/// `package-root` vs `package-area-root` resolve to different strings.
///
/// Some subcommands exit `1` when they have nothing to report; we accept
/// either exit code and only compare stdout.
#[test]
fn test_repo_subcommand_json_shapes_are_distinct() {
    let (_dir, path) = create_cli_monorepo_distinct_area_and_package();
    test_commit_file(&path, "alpha/core/src/lib.rs", "pub fn changed() {}");
    test_commit_file(&path, "README.md", "# readme");

    // Match groups intentionally avoid `structure` / bare `repo`. Each entry
    // is a tuple of (label, args after `repo`).
    let cases: &[(&str, &[&str])] = &[
        ("git-status", &["git-status"]),
        ("package-dependencies", &["package-dependencies"]),
        ("dirty-packages", &["dirty-packages"]),
        ("dirty-package-areas", &["dirty-package-areas"]),
        ("staged-packages", &["staged-packages"]),
        ("staged-package-areas", &["staged-package-areas"]),
        ("unstaged-packages", &["unstaged-packages"]),
        ("unstaged-package-areas", &["unstaged-package-areas"]),
        ("package-root", &["package-root"]),
        ("package-area-root", &["package-area-root"]),
        ("package", &["package"]),
        ("package-area", &["package-area"]),
        (
            "is-current-package-area-dirty",
            &["is-current-package-area-dirty"],
        ),
        (
            "package-area-has-source-code-changes",
            &["package-area-has-source-code-changes"],
        ),
        ("has-merge-conflict", &["has-merge-conflict"]),
        ("source-code-changes", &["source-code-changes"]),
        ("documentation-changes", &["documentation-changes"]),
    ];

    // Run from inside `alpha/core` so locator/boolean subcommands resolve
    // to a real package and area.
    let cwd = path.join("alpha/core");
    let mut payloads: Vec<(String, String)> = Vec::with_capacity(cases.len());

    for (label, sub_args) in cases {
        let mut args: Vec<&str> = vec!["--base", cwd.to_str().unwrap(), "repo"];
        args.extend_from_slice(sub_args);
        args.push("--json");
        let output = assert_cmd::Command::cargo_bin("sniff").unwrap()
            .args(&args)
            .assert()
            .get_output()
            .clone();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        // Every JSON payload must parse — that's a baseline contract even
        // for empty boolean/locator outputs.
        let _: Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
            panic!(
                "stdout for `{label}` was not JSON: {e}\n--- stdout ---\n{stdout}\n--------------"
            )
        });
        payloads.push(((*label).to_string(), stdout));
    }

    // Cross-product distinctness check. We compare on raw stdout — the
    // shape, keys, and values combined are what consumers see.
    for i in 0..payloads.len() {
        for j in (i + 1)..payloads.len() {
            let (left_label, left) = &payloads[i];
            let (right_label, right) = &payloads[j];
            assert_ne!(
                left.trim(),
                right.trim(),
                "subcommands `{left_label}` and `{right_label}` returned identical JSON \
                 — every repo subcommand must emit a distinct shape:\n--- {left_label} ---\n{left}\n--- {right_label} ---\n{right}"
            );
        }
    }
}

/// `--perf --json` on a `git-status` invocation must inject a top-level
/// `performance` field into the existing object shape, leaving the rest of
/// the `GitInfo` payload intact.
///
/// Object-shaped payloads receive the perf data via `attach_performance`
/// inserting a sibling key — they are NOT wrapped in `{ data, performance }`.
#[test]
fn test_git_status_json_perf_attaches_performance_field() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "git-status",
            "--json",
            "--perf",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));

    // `GitInfo` fields stay at the top level.
    assert!(
        value.get("repo_root").is_some(),
        "git-status payload should still expose `repo_root`: {value}"
    );
    // `performance` should be a sibling key (not wrapped under `data`).
    assert!(
        value.get("performance").is_some(),
        "--perf must inject a `performance` field into object-shaped payloads: {value}"
    );
    assert!(
        value.get("data").is_none(),
        "object-shaped payloads must NOT be wrapped in `{{ data, ... }}`: {value}"
    );
}

/// `--perf --json` on a boolean subcommand still emits the boolean object
/// alongside the `performance` field, and still honours the boolean's
/// exit-code semantics (clean repo → exit 1 for `is-current-package-area-dirty`).
#[test]
fn test_is_current_package_area_dirty_json_perf_attaches_performance_field() {
    let (_dir, path) = create_cli_monorepo();
    let pkg_a = path.join("pkg-a/lib");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            pkg_a.to_str().unwrap(),
            "repo",
            "is-current-package-area-dirty",
            "--json",
            "--perf",
        ])
        .assert()
        .failure()
        .code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));

    assert_eq!(
        value["dirty"],
        Value::Bool(false),
        "boolean payload must remain intact alongside --perf: {value}"
    );
    assert!(
        value.get("performance").is_some(),
        "--perf must inject a `performance` field into boolean payloads: {value}"
    );
}

/// Phase 3 — `repo structure --json --filter` must scope the `packages`
/// array, matching text mode. Without `--filter` every workspace member is
/// listed; with `--filter pkg-a` only the matching package remains. The
/// non-`packages` `RepoInfo` fields (workspace tools, monorepo flag, root)
/// stay intact in both cases.
#[test]
fn test_repo_structure_filter_json_filters_packages() {
    let (_dir, path) = create_cli_monorepo();

    let assert_all = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "structure",
            "--json",
        ])
        .assert()
        .success();
    let stdout_all = String::from_utf8(assert_all.get_output().stdout.clone()).unwrap();
    let json_all: Value = serde_json::from_str(stdout_all.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout_all}\n---"));
    let all_packages = json_all["packages"]
        .as_array()
        .expect("packages must be array");
    assert_eq!(
        all_packages.len(),
        2,
        "unfiltered structure should list all 2 monorepo packages: {json_all}"
    );

    let assert_filtered = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "structure",
            "--json",
            "pkg-a",
        ])
        .assert()
        .success();
    let stdout_filtered = String::from_utf8(assert_filtered.get_output().stdout.clone()).unwrap();
    let json_filtered: Value = serde_json::from_str(stdout_filtered.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout_filtered}\n---"));
    let filtered_packages = json_filtered["packages"]
        .as_array()
        .expect("packages must be array");
    assert_eq!(
        filtered_packages.len(),
        1,
        "filter pkg-a should narrow to 1 package: {json_filtered}"
    );
    assert_eq!(filtered_packages[0]["name"], "pkg-a");

    // Non-packages fields must remain intact under the filter.
    assert!(
        json_filtered.get("root").is_some(),
        "filtered structure must preserve `root`: {json_filtered}"
    );
    assert_eq!(
        json_filtered["is_monorepo"],
        Value::Bool(true),
        "filtered structure must preserve `is_monorepo`: {json_filtered}"
    );
}

// ============================================================================
// Phase 4 — Targeted integration coverage for previously untested JSON paths
// ============================================================================
//
// These tests exercise the success branches of locator and boolean
// subcommands that were previously only covered by their empty/false
// branches, plus the `--package` scoping path on `git-status --json`.

/// `package-area --json` from inside a real package emits `{ "name": <area> }`
/// where the area name is distinct from the package name (the fixture
/// places `alpha-core` inside area `alpha`).
#[test]
fn test_package_area_json_resolves_to_real_area() {
    let (_dir, path) = create_cli_monorepo_distinct_area_and_package();
    let cwd = path.join("alpha/core");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            cwd.to_str().unwrap(),
            "repo",
            "package-area",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["name"], Value::String("alpha".to_string()));
}

/// `package-area-root --json` from inside a known package area emits
/// `{ "root": <abs path containing the area name> }`.
#[test]
fn test_package_area_root_json_when_present() {
    let (_dir, path) = create_cli_monorepo_distinct_area_and_package();
    let cwd = path.join("alpha/core");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            cwd.to_str().unwrap(),
            "repo",
            "package-area-root",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    let root = value["root"].as_str().expect("root must be a string");
    assert!(
        !root.is_empty(),
        "package-area-root must be non-empty inside a real area, got: {value}"
    );
    assert!(
        root.contains("alpha"),
        "package-area-root should contain the `alpha` area segment, got: {root}"
    );
}

/// `git-status --package <name> --json` must scope `file_changes` to the
/// named package's path prefix while preserving the `GitInfo` shape (top-level
/// `repo_root` key).
#[test]
fn test_git_status_json_with_package_scope() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/src/lib.rs", "pub fn a2() {}");

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "git-status",
            "--package",
            "pkg-a",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));

    assert!(
        value.get("repo_root").is_some(),
        "package-scoped git-status must keep GitInfo shape (repo_root): {value}"
    );

    if let Some(file_changes) = value["file_changes"].as_array() {
        for fc in file_changes {
            let p = fc["path"].as_str().unwrap_or("");
            assert!(
                !p.starts_with("pkg-b/"),
                "pkg-a-scoped git-status must not contain pkg-b files, got: {p}"
            );
        }
    }
}

/// `is-current-package-area-dirty --json` from inside a package area whose
/// files are dirty must emit `{ "dirty": true }` and exit 0.
#[test]
fn test_is_current_package_area_dirty_json_true_branch() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/src/lib.rs", "pub fn a() {}");
    std::fs::write(path.join("pkg-a/lib/src/lib.rs"), "pub fn a() { dirty }").unwrap();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.join("pkg-a/lib").to_str().unwrap(),
            "repo",
            "is-current-package-area-dirty",
            "--json",
        ])
        .assert()
        .success()
        .code(0);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(
        value["dirty"],
        Value::Bool(true),
        "dirty area should emit dirty: true, got: {value}"
    );
}

/// `package-area-has-source-code-changes --json` from inside a package area
/// whose source files are dirty must emit
/// `{ "has_source_code_changes": true }` and exit 0, even in the normal
/// (non-deep) git request path where `RepoStatus.dirty` is empty.
///
/// Regression test for review-4 High finding: the helper used to read only
/// `git.status.dirty` / `git.status.untracked` and missed dirty files
/// surfaced via `git.file_changes`.
#[test]
fn test_package_area_has_source_code_changes_json_true_branch() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/src/lib.rs", "pub fn a() {}");
    std::fs::write(path.join("pkg-a/lib/src/lib.rs"), "pub fn a() { dirty }").unwrap();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.join("pkg-a/lib").to_str().unwrap(),
            "repo",
            "package-area-has-source-code-changes",
            "--json",
        ])
        .assert()
        .success()
        .code(0);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(
        value["has_source_code_changes"],
        Value::Bool(true),
        "dirty source file in the area should emit has_source_code_changes: true, got: {value}"
    );
}

/// `package-area-has-source-code-changes --json` must remain `false` when
/// only documentation files are dirty in the current package area, even
/// though those paths are reported via `git.file_changes` in the normal
/// CLI path.
#[test]
fn test_package_area_has_source_code_changes_json_docs_only_is_false() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/README.md", "# pkg-a");
    std::fs::write(path.join("pkg-a/lib/README.md"), "# pkg-a (dirty)").unwrap();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.join("pkg-a/lib").to_str().unwrap(),
            "repo",
            "package-area-has-source-code-changes",
            "--json",
        ])
        .assert()
        .failure()
        .code(1);
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(
        value["has_source_code_changes"],
        Value::Bool(false),
        "docs-only dirty file must not flip has_source_code_changes, got: {value}"
    );
}

// ============================================================================
// Phase 4 — `repo worktree` CLI integration tests
// ============================================================================

/// Create a temp git repo with an initial commit and a linked worktree.
fn create_test_repo_with_worktree() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let (dir, repo_path) = create_test_repo();
    let repo = git2::Repository::open(&repo_path).unwrap();

    let worktree_path = repo_path.join("my-worktree");
    let _wt = repo.worktree("my-worktree", &worktree_path, None).unwrap();

    (dir, repo_path, worktree_path)
}

/// Add one commit on the branch checked out in `worktree_path`, advancing it
/// past the base branch so its ahead-count is non-zero.
fn commit_in_worktree(worktree_path: &Path, relative: &str, content: &str) {
    std::fs::write(worktree_path.join(relative), content).unwrap();
    let repo = git2::Repository::open(worktree_path).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(relative)).unwrap();
    index.write().unwrap();
    let sig = repo.signature().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "worktree commit", &tree, &[&head])
        .unwrap();
}

/// Repo with two linked worktrees: `even-wt` left at main's tip, and `ahead-wt`
/// advanced by one commit so it is one ahead of the base branch. Returns
/// `(tempdir, main_repo_path, even_wt_path, ahead_wt_path)`.
fn create_test_repo_with_two_worktrees() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
    let (dir, repo_path) = create_test_repo();
    let repo = git2::Repository::open(&repo_path).unwrap();

    let even_path = repo_path.join("even-wt");
    repo.worktree("even-wt", &even_path, None).unwrap();

    let ahead_path = repo_path.join("ahead-wt");
    repo.worktree("ahead-wt", &ahead_path, None).unwrap();
    commit_in_worktree(&ahead_path, "extra.txt", "extra\n");

    (dir, repo_path, even_path, ahead_path)
}

/// Case A: from inside a linked worktree, the report shows the main worktree
/// location, the current worktree's own details, and a count of the rest.
#[test]
fn test_git_status_from_linked_worktree_renders_case_a() {
    let (_dir, _repo, _even, ahead) = create_test_repo_with_two_worktrees();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            ahead.to_str().unwrap(),
            "repo",
            "git-status",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(
        stdout.contains("main:"),
        "Case A shows main location: {stdout}"
    );
    assert!(
        stdout.contains("Current Worktree:"),
        "Case A shows current worktree: {stdout}"
    );
    assert!(
        stdout.contains("ahead-wt"),
        "current worktree named by its directory: {stdout}"
    );
    // The current worktree gets full detail, so its real ahead-count shows.
    assert!(
        stdout.contains("1 ahead"),
        "current worktree ahead-count is computed: {stdout}"
    );
    assert!(
        stdout.contains("1 other active worktrees in this repo"),
        "exactly one other linked worktree (even-wt): {stdout}"
    );
    // The main worktree is the parent directory, so its visible label is
    // relative (`..`) rather than an absolute or home-abbreviated path.
    assert!(
        stdout.contains("[..](file://") || stdout.contains("located at .."),
        "main worktree path label is relative to the current worktree: {stdout}"
    );

    // Verify proper nested list layout: headings are top-level bullets and
    // details are indented child bullets, not literal "  - " prefixes.
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.iter().any(|l| l.starts_with("- Current Worktree:")),
        "Current Worktree heading is a top-level bullet: {stdout}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("  - you are in the")),
        "current worktree details are nested bullets: {stdout}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("  - this worktree is on the")),
        "ahead/behind detail is a nested bullet: {stdout}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("- Other Worktrees:")),
        "Other Worktrees heading is a top-level bullet: {stdout}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("  - there are")),
        "other worktree count is a nested bullet: {stdout}"
    );
    assert!(
        !lines.iter().any(|l| l.starts_with("-   -")),
        "no double-bulleted literal prefixes: {stdout}"
    );
}

/// Case B: from the main worktree, the report shows the current (main) worktree
/// and a count of all linked worktrees.
#[test]
fn test_git_status_from_main_worktree_renders_case_b() {
    let (_dir, repo, _even, _ahead) = create_test_repo_with_two_worktrees();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            repo.to_str().unwrap(),
            "repo",
            "git-status",
            "--plain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(
        stdout.contains("Current Worktree:"),
        "Case B shows current worktree: {stdout}"
    );
    assert!(
        stdout.contains("2 other active worktrees"),
        "both linked worktrees counted as other: {stdout}"
    );
    assert!(
        !stdout.contains("main:"),
        "Case B omits the separate main location line: {stdout}"
    );

    // Verify proper nested list layout for Case B.
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.iter().any(|l| l.starts_with("- Current Worktree:")),
        "Current Worktree heading is a top-level bullet: {stdout}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("  - you are in the")),
        "current worktree details are nested bullets: {stdout}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("- Other Worktrees:")),
        "Other Worktrees heading is a top-level bullet: {stdout}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("  - there are")),
        "other worktree count is a nested bullet: {stdout}"
    );
    assert!(
        !lines.iter().any(|l| l.starts_with("-   -")),
        "no double-bulleted literal prefixes: {stdout}"
    );
}

/// Default JSON computes ahead/behind only for the current worktree; a
/// divergent *non-current* worktree reports `ahead == 0` (not computed).
/// `--refresh-remotes` (full detail) restores its real ahead-count.
#[test]
fn test_git_status_json_worktree_ahead_is_lazy_by_default() {
    let (_dir, repo, _even, _ahead) = create_test_repo_with_two_worktrees();

    let read_ahead = |args: &[&str]| -> u64 {
        let assert = assert_cmd::Command::cargo_bin("sniff").unwrap().args(args).assert().success();
        let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
        let json: Value = serde_json::from_str(stdout.trim())
            .unwrap_or_else(|e| panic!("not JSON: {e}\n{stdout}"));
        json["worktrees"]["ahead-wt"]["ahead"]
            .as_u64()
            .unwrap_or_else(|| panic!("missing worktrees.ahead-wt.ahead: {json}"))
    };

    // Default: ahead-wt is not the current worktree, so its ahead-count is
    // skipped and reported as 0 even though it is genuinely one ahead.
    let lazy = read_ahead(&[
        "--base",
        repo.to_str().unwrap(),
        "repo",
        "git-status",
        "--json",
    ]);
    assert_eq!(
        lazy, 0,
        "non-current worktree ahead must be lazy (0) by default"
    );

    // Full detail (deep) computes it: ahead-wt is one ahead of main.
    let eager = read_ahead(&[
        "--base",
        repo.to_str().unwrap(),
        "repo",
        "git-status",
        "--json",
        "--refresh-remotes",
    ]);
    assert_eq!(eager, 1, "full detail restores the real ahead-count");
}

/// Text and JSON must agree on which worktree is current.
#[test]
fn test_git_status_text_and_json_agree_on_current_worktree() {
    let (_dir, _repo, _even, ahead) = create_test_repo_with_two_worktrees();
    let base = ahead.to_str().unwrap();

    let json_assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", base, "repo", "git-status", "--json"])
        .assert()
        .success();
    let json_out = String::from_utf8(json_assert.get_output().stdout.clone()).unwrap();
    let json: Value = serde_json::from_str(json_out.trim()).unwrap();

    let current: Vec<&str> = json["worktrees"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(_, v)| v["is_current"].as_bool() == Some(true))
        .map(|(k, _)| k.as_str())
        .collect();
    assert_eq!(
        current,
        vec!["ahead-wt"],
        "JSON marks exactly the running worktree as current: {json}"
    );

    let text_assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", base, "repo", "git-status", "--plain"])
        .assert()
        .success();
    let text = String::from_utf8(text_assert.get_output().stdout.clone()).unwrap();
    assert!(
        text.contains("ahead-wt"),
        "text names the same current worktree: {text}"
    );
}

#[test]
fn test_repo_worktree_inside_linked_worktree_returns_name() {
    let (_dir, _repo_path, worktree_path) = create_test_repo_with_worktree();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            worktree_path.to_str().unwrap(),
            "repo",
            "worktree",
        ])
        .assert()
        .success()
        .code(0);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.trim(), "my-worktree");
}

#[test]
fn test_repo_worktree_inside_main_worktree_exits_1() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", repo_path.to_str().unwrap(), "repo", "worktree"])
        .assert()
        .failure()
        .code(1);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout, "");
}

#[test]
fn test_repo_worktree_no_error_exits_0() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktree",
            "--no-error",
        ])
        .assert()
        .success()
        .code(0);
}

#[test]
fn test_repo_worktree_on_error_to_stderr() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktree",
            "--on-error",
            "Not in a worktree",
            "--plain",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("Not in a worktree"));
}

#[test]
fn test_repo_worktree_json_success() {
    let (_dir, _repo_path, worktree_path) = create_test_repo_with_worktree();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            worktree_path.to_str().unwrap(),
            "repo",
            "worktree",
            "--json",
        ])
        .assert()
        .success()
        .code(0);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["worktree"], Value::String("my-worktree".to_string()));
}

#[test]
fn test_repo_worktree_json_failure_no_error() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktree",
            "--json",
            "--no-error",
        ])
        .assert()
        .success()
        .code(0);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["worktree"], Value::Null);
}

#[test]
fn test_repo_worktree_verbose_includes_path() {
    let (_dir, _repo_path, worktree_path) = create_test_repo_with_worktree();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            worktree_path.to_str().unwrap(),
            "repo",
            "worktree",
            "-v",
        ])
        .assert()
        .success()
        .code(0);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let trimmed = stdout.trim();
    assert!(trimmed.starts_with("my-worktree ["));
    assert!(trimmed.ends_with("]"));
    assert!(trimmed.contains(worktree_path.to_str().unwrap()));
}

#[test]
fn test_repo_worktree_help_mentions_subcommand() {
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["repo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("worktree"));
}

// ============================================================================
// Phase 3 — `repo worktrees` CLI integration tests
// ============================================================================

#[test]
fn test_repo_worktrees_default_output() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", repo_path.to_str().unwrap(), "repo", "worktrees"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "expected main + 1 linked worktree: {stdout}"
    );
    assert!(
        lines.iter().any(|l| l.contains("my-worktree")),
        "should list linked worktree: {stdout}"
    );
    // No worktree line may begin with whitespace; non-current entries are
    // unprefixed, current entries use a "* " marker.
    for line in &lines {
        assert!(
            !line.starts_with(' '),
            "worktree line must not start with a space: {line:?}"
        );
    }

    // The default output must be byte-identical to `--list`.
    let list = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "--list",
        ])
        .assert()
        .success();
    let list_stdout = String::from_utf8(list.get_output().stdout.clone()).unwrap();
    assert_eq!(
        stdout, list_stdout,
        "default output must match `--list` output"
    );
}

#[test]
fn test_repo_worktrees_md_output() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "--md",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    for line in stdout.trim().lines() {
        assert!(
            line.starts_with("- "),
            "md output should start with '- ': {line}"
        );
    }
}

#[test]
fn test_repo_worktrees_list_output() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "--list",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.trim().lines().collect();
    for line in &lines {
        assert!(
            !line.starts_with("- "),
            "list output should not use markdown bullets: {line}"
        );
    }
    assert!(
        lines.iter().any(|l| l.contains("my-worktree")),
        "list should contain worktree name: {stdout}"
    );
}

#[test]
fn test_repo_worktrees_csv_output() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "--csv",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let trimmed = stdout.trim();
    assert!(
        trimmed.contains("my-worktree"),
        "csv should contain worktree name: {stdout}"
    );
    assert!(
        !trimmed.contains('\n'),
        "csv should be single line: {stdout}"
    );
}

#[test]
fn test_repo_worktrees_verbose_output() {
    let (_dir, repo_path, worktree_path) = create_test_repo_with_worktree();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "-v",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("my-worktree"),
        "verbose should list worktree name: {stdout}"
    );
    assert!(
        stdout.contains("located at"),
        "verbose should include path: {stdout}"
    );
    assert!(
        stdout.contains(worktree_path.to_str().unwrap()),
        "verbose should contain worktree path: {stdout}"
    );
}

#[test]
fn test_repo_worktrees_list_verbose_composes_and_has_no_leading_space() {
    // `-v` must compose with `--list`: metadata is appended, structure stays
    // one-per-line, and no line begins with whitespace. Bare `-v` must be
    // byte-identical to `--list -v`.
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let list_v = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "--list",
            "-v",
            "--plain",
        ])
        .assert()
        .success();
    let list_v_out = String::from_utf8(list_v.get_output().stdout.clone()).unwrap();

    for line in list_v_out.lines() {
        assert!(
            !line.starts_with(' '),
            "verbose list line must not start with a space: {line:?}"
        );
        assert!(
            line.contains("located at"),
            "verbose list line must include metadata: {line:?}"
        );
    }

    let bare_v = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "-v",
            "--plain",
        ])
        .assert()
        .success();
    let bare_v_out = String::from_utf8(bare_v.get_output().stdout.clone()).unwrap();
    assert_eq!(bare_v_out, list_v_out, "bare `-v` must match `--list -v`");
}

#[test]
fn test_repo_worktrees_md_verbose_keeps_bullet_and_metadata() {
    // `--md -v` must keep the markdown bullet AND append metadata.
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "--md",
            "-v",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    for line in stdout.lines() {
        assert!(
            line.starts_with("- "),
            "md verbose line must start with '- ': {line:?}"
        );
        assert!(
            line.contains("located at"),
            "md verbose line must include metadata: {line:?}"
        );
    }
}

#[test]
fn test_repo_worktrees_csv_verbose_single_line_with_metadata() {
    // `--csv -v` must stay a single comma-separated line and gain metadata.
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "--csv",
            "-v",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(
        stdout.trim().lines().count(),
        1,
        "csv verbose must be a single line: {stdout}"
    );
    assert!(
        stdout.contains("located at"),
        "csv verbose must include metadata: {stdout}"
    );
    assert!(
        stdout.contains("my-worktree"),
        "csv verbose must list worktree name: {stdout}"
    );
}

#[test]
fn test_repo_worktrees_json_output() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    let arr = value["worktrees"]
        .as_array()
        .expect("worktrees must be array");
    assert_eq!(arr.len(), 2, "expected main + 1 linked worktree");
    assert!(
        arr.iter().any(|w| w["name"] == "my-worktree"),
        "should include linked worktree: {value}"
    );
}

#[test]
fn test_repo_worktrees_plain_verbose_no_escape_codes() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "-v",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        !stdout.contains('\x1b'),
        "plain output must not contain escape codes: {stdout:?}"
    );
    assert!(
        stdout.contains("located at"),
        "plain verbose should still show words: {stdout}"
    );
}

#[test]
fn test_repo_worktrees_current_marker_from_main_worktree() {
    let (_dir, repo_path, _worktree_path) = create_test_repo_with_worktree();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .current_dir(&repo_path)
        .args(["repo", "worktrees"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.trim().lines().collect();
    let current_line = lines
        .iter()
        .find(|l| l.starts_with("* "))
        .expect("should have a current marker line");
    assert!(
        current_line.contains(repo_path.file_name().unwrap().to_str().unwrap()),
        "current marker should be on main worktree: {stdout}"
    );
}

#[test]
fn test_repo_worktrees_current_marker_from_linked_worktree() {
    let (_dir, _repo_path, worktree_path) = create_test_repo_with_worktree();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .current_dir(&worktree_path)
        .args(["repo", "worktrees"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.trim().lines().collect();
    let current_line = lines
        .iter()
        .find(|l| l.starts_with("* "))
        .expect("should have a current marker line");
    assert!(
        current_line.contains("my-worktree"),
        "current marker should be on linked worktree: {stdout}"
    );
}

#[test]
fn test_repo_worktrees_detached_head() {
    let (_dir, repo_path) = create_test_repo();
    let repo = git2::Repository::open(&repo_path).unwrap();

    let worktree_path = repo_path.join("detached-wt");
    let _wt = repo.worktree("detached-wt", &worktree_path, None).unwrap();

    // Detach HEAD in the linked worktree.
    let wt_repo = git2::Repository::open(&worktree_path).unwrap();
    let head_commit = wt_repo.head().unwrap().peel_to_commit().unwrap();
    wt_repo.set_head_detached(head_commit.id()).unwrap();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            repo_path.to_str().unwrap(),
            "repo",
            "worktrees",
            "-v",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("detached HEAD"),
        "verbose should show detached HEAD fallback: {stdout}"
    );
}

// ============================================================================
// Phase 5 — `--package` / `--package-area` flag matrix
//
// These tests pin the consistency contract for the new flag pair across the
// `repo` subcommand surface:
//
// 1. `--package` returns exactly one package.
// 2. `--package-area homelab` matches both `homelab` and `homelab/server`
//    (case-insensitive prefix semantics).
// 3. `--package` AND `--package-area` overlapping → success.
// 4. `--package` AND `--package-area` non-overlapping → hard error citing
//    the package's real area.
// 5. Unknown `--package` → error names valid package list.
// 6. Unknown `--package-area` → error names valid area list.
// 7. Positional `filter` plus `--package` → AND of both.
// 8. `-p` short flag works on `FileListArgs`-based commands.
// 9. `git-status -p <area-name>` no longer falls back to area matching —
//    must hard-error.
// ============================================================================

/// Build a monorepo with three areas where two share a common prefix
/// (`homelab` and `homelab/server`) and one is wholly distinct (`sniff`).
///
/// Layout:
///
/// - `homelab/lib`        → area `homelab`,        name `homelab-lib`
/// - `homelab/server/srv` → area `homelab/server`, name `homelab-srv`
/// - `sniff/cli`          → area `sniff`,          name `sniff-cli`
fn create_cli_monorepo_with_nested_areas() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();

    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test").unwrap();

    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[workspace]
members = ["homelab/lib", "homelab/server/srv", "sniff/cli"]
"#,
    )
    .unwrap();

    let members = [
        ("homelab/lib", "homelab-lib"),
        ("homelab/server/srv", "homelab-srv"),
        ("sniff/cli", "sniff-cli"),
    ];
    for (rel, name) in &members {
        let pkg = dir.path().join(rel);
        std::fs::create_dir_all(pkg.join("src")).unwrap();
        std::fs::write(
            pkg.join("Cargo.toml"),
            format!(
                r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"
"#
            ),
        )
        .unwrap();
        std::fs::write(pkg.join("src/lib.rs"), "pub fn entry() {}").unwrap();
    }

    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let sig = repo.signature().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "initial nested-area monorepo",
        &tree,
        &[],
    )
    .unwrap();

    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Spec §6.1 — `--package` returns exactly one package.
#[test]
fn test_repo_package_flag_returns_single_package() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--package",
            "sniff-cli",
            "--list",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "sniff-cli");
}

/// Spec §6.2 — `--package-area homelab` matches both `homelab` and
/// `homelab/server` packages via prefix semantics.
#[test]
fn test_repo_package_area_flag_uses_prefix_semantics() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--package-area",
            "homelab",
            "--list",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let mut names: Vec<&str> = stdout.trim().lines().collect();
    names.sort();
    assert_eq!(names, vec!["homelab-lib", "homelab-srv"]);
}

/// Spec §6.3 — `--package` AND `--package-area` overlap → intersection,
/// returns the package itself.
#[test]
fn test_repo_package_and_area_flags_overlap_succeeds() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--package",
            "homelab-srv",
            "--package-area",
            "homelab",
            "--list",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "homelab-srv");
}

/// Spec §6.4 — `--package` AND `--package-area` non-overlapping → hard error
/// naming the package's real area and the requested area.
#[test]
fn test_repo_package_and_area_flags_non_overlap_errors() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--package",
            "sniff-cli",
            "--package-area",
            "homelab",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("sniff-cli"),
        "intersection error must name the package, got:\n{stderr}"
    );
    assert!(
        stderr.contains("sniff"),
        "intersection error must name the package's real area, got:\n{stderr}"
    );
    assert!(
        stderr.contains("homelab"),
        "intersection error must name the requested area, got:\n{stderr}"
    );
}

/// Spec §6.5 — Unknown `--package` → error lists valid package names.
#[test]
fn test_repo_unknown_package_errors_with_valid_list() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--package",
            "no-such-pkg",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("no-such-pkg"),
        "error must name the unknown package, got:\n{stderr}"
    );
    assert!(
        stderr.contains("Valid package names"),
        "error must mention valid package names, got:\n{stderr}"
    );
    assert!(
        stderr.contains("sniff-cli") && stderr.contains("homelab-lib"),
        "error must list the actual valid package names, got:\n{stderr}"
    );
}

/// Spec §6.6 — Unknown `--package-area` → error lists valid package areas.
#[test]
fn test_repo_unknown_package_area_errors_with_valid_list() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "--package-area",
            "no-such-area",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("no-such-area"),
        "error must name the unknown area, got:\n{stderr}"
    );
    assert!(
        stderr.contains("Valid package areas"),
        "error must mention valid package areas, got:\n{stderr}"
    );
    assert!(
        stderr.contains("sniff") && stderr.contains("homelab"),
        "error must list the actual valid areas, got:\n{stderr}"
    );
}

/// Spec §6.7 — Positional `filter` plus `--package` are AND-combined.
///
/// `homelab-lib` and `homelab-srv` are both in areas starting with `homelab`,
/// so the positional `@homelab` filter selects both. The `--package` flag
/// then narrows to the single named package.
#[test]
fn test_repo_positional_filter_and_package_flag_combine() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "packages",
            "@homelab",
            "--package",
            "homelab-lib",
            "--list",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "homelab-lib");
}

/// Spec §6.8 — `-p` short flag works on a `FileListArgs`-based command.
///
/// Stage and modify a file under `sniff/cli` so `dirty-files` has something
/// to report, then verify `-p sniff-cli` scopes the result to that package.
#[test]
fn test_repo_dirty_files_short_p_flag_scopes_to_package() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    // Mark the sniff-cli source file dirty (untracked modification of an
    // already-tracked file).
    std::fs::write(
        path.join("sniff/cli/src/lib.rs"),
        "pub fn entry() { let _x = 1; }",
    )
    .unwrap();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "dirty-files",
            "-p",
            "sniff-cli",
            "--list",
            "--plain",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("sniff/cli"),
        "scoped dirty-files must include the sniff/cli path, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("homelab/"),
        "scoped dirty-files must not include unrelated areas, got:\n{stdout}"
    );
}

/// Spec §6.9 — Regression guard: `git-status -p <area-name>` (a real area
/// that is **not** a package name) must hard-error rather than fall back to
/// area matching.
#[test]
fn test_repo_git_status_package_with_area_name_errors() {
    let (_dir, path) = create_cli_monorepo_with_nested_areas();

    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "git-status",
            "-p",
            "homelab",
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("homelab"),
        "error must name the rejected input, got:\n{stderr}"
    );
    assert!(
        stderr.contains("Valid package names"),
        "error must list valid package names (not areas), got:\n{stderr}"
    );
}

// ============================================================================
// `repo area` Subcommand
// ============================================================================
//
// `sniff repo area` returns a single "area" name combining the notions of
// "package" and "package-area": package name when inside a package, else the
// surrounding area string (or "root").

#[test]
fn test_repo_area_inside_package_returns_package_name() {
    let (_dir, path) = create_cli_monorepo();
    let inside_pkg_a = path.join("pkg-a/lib/src");
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", inside_pkg_a.to_str().unwrap(), "repo", "area"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-a");
}

#[test]
fn test_repo_area_at_area_dir_returns_area_name() {
    let (_dir, path) = create_cli_monorepo();
    let area_dir = path.join("pkg-a");
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", area_dir.to_str().unwrap(), "repo", "area"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "pkg-a");
}

#[test]
fn test_repo_area_at_repo_root_returns_root() {
    let (_dir, path) = create_cli_monorepo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "area"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout.trim(), "root");
}

#[test]
fn test_repo_area_json_emits_name_outcome() {
    let (_dir, path) = create_cli_monorepo();
    let inside_pkg_b = path.join("pkg-b/lib");
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            inside_pkg_b.to_str().unwrap(),
            "--json",
            "repo",
            "area",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["name"], Value::String("pkg-b".to_string()));
}

#[test]
fn test_repo_area_non_monorepo_repo_silent_failure() {
    let (_dir, path) = create_test_repo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args(["--base", path.to_str().unwrap(), "repo", "area"])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stdout.is_empty(), "stdout must be empty, got: {stdout:?}");
    assert!(
        stderr.is_empty(),
        "stderr must be empty without --verbose, got: {stderr:?}"
    );
}

#[test]
fn test_repo_area_non_monorepo_verbose_message_on_stderr() {
    let (_dir, path) = create_test_repo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "--verbose",
            "--plain",
            "repo",
            "area",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stdout.is_empty(), "stdout must be empty, got: {stdout:?}");
    assert!(
        stderr.contains("you are in a repo but not a monorepo"),
        "verbose stderr must explain not-a-monorepo, got: {stderr:?}"
    );
}

#[test]
fn test_repo_area_not_in_repo_verbose_message_on_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            dir.path().to_str().unwrap(),
            "--verbose",
            "--plain",
            "repo",
            "area",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stdout.is_empty(), "stdout must be empty, got: {stdout:?}");
    assert!(
        stderr.contains("you are not in a repo"),
        "verbose stderr must explain not-in-repo, got: {stderr:?}"
    );
}

#[test]
fn test_repo_area_no_error_zero_exit_when_no_monorepo() {
    let (_dir, path) = create_test_repo();
    assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "repo",
            "area",
            "--no-error",
        ])
        .assert()
        .success();
}

#[test]
fn test_repo_area_on_error_prints_message_to_stdout() {
    let (_dir, path) = create_test_repo();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            path.to_str().unwrap(),
            "--plain",
            "repo",
            "area",
            "--no-error",
            "--on-error",
            "n/a",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("n/a"),
        "--on-error message must reach stdout, got: {stdout:?}"
    );
}

#[test]
fn test_repo_git_status_outside_git_repo_is_graceful() {
    let dir = tempfile::tempdir().unwrap();
    let assert = assert_cmd::Command::cargo_bin("sniff").unwrap()
        .args([
            "--base",
            dir.path().to_str().unwrap(),
            "--plain",
            "repo",
            "git-status",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stdout.is_empty(),
        "git-status outside a repo should produce no stdout, got: {stdout:?}"
    );
    assert!(
        stderr.is_empty(),
        "git-status outside a repo should produce no stderr, got: {stderr:?}"
    );
}
