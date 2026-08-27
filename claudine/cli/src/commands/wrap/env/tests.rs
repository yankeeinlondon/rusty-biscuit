//! Wrapper env tests: sanitization, redaction, monorepo package resolution,
//! launch workspace context resolution, and `build_child_env` wiring.

use super::*;
use crate::commands::wrap::profile::profile_for_provider;
use sniff::filesystem::repo::{Package, RepoInfo};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn sanitization_removes_sensitive_names_unless_included() {
    let include_set = HashSet::from(["OPENAI_API_KEY".to_string()]);
    let env = vec![
        ("OPENAI_API_KEY".to_string(), "keep".to_string()),
        ("SVC_TOKEN".to_string(), "remove".to_string()),
        ("DB_PASSWORD".to_string(), "remove".to_string()),
        ("APP_SECRET".to_string(), "remove".to_string()),
        ("NORMAL_VAR".to_string(), "ok".to_string()),
    ];

    let (kept, removed) = sanitize_env_for_test(&env, &include_set);
    let kept_names: HashSet<_> = kept.into_iter().map(|(name, _)| name).collect();

    assert!(kept_names.contains("OPENAI_API_KEY"));
    assert!(kept_names.contains("NORMAL_VAR"));
    assert_eq!(
        removed,
        vec![
            "APP_SECRET".to_string(),
            "DB_PASSWORD".to_string(),
            "SVC_TOKEN".to_string()
        ]
    );
}

#[test]
fn sanitization_catches_new_sensitive_patterns() {
    let include_set = HashSet::new();
    let env = vec![
        ("SSH_PRIVATE_KEY".to_string(), "secret".to_string()),
        ("AWS_ACCESS_KEY_ID".to_string(), "secret".to_string()),
        ("DB_CREDENTIAL".to_string(), "secret".to_string()),
        ("KEY_PASSPHRASE".to_string(), "secret".to_string()),
        ("NORMAL_VAR".to_string(), "ok".to_string()),
    ];

    let (kept, removed) = sanitize_env_for_test(&env, &include_set);
    let kept_names: HashSet<_> = kept.into_iter().map(|(name, _)| name).collect();

    assert!(!kept_names.contains("SSH_PRIVATE_KEY"));
    assert!(!kept_names.contains("AWS_ACCESS_KEY_ID"));
    assert!(!kept_names.contains("DB_CREDENTIAL"));
    assert!(!kept_names.contains("KEY_PASSPHRASE"));
    assert!(kept_names.contains("NORMAL_VAR"));
    assert_eq!(removed.len(), 4);
}

#[test]
fn include_names_must_be_valid_env_identifiers() {
    let includes = vec!["VALID_NAME".to_string(), "9INVALID".to_string()];
    let error = validate_include_names(&includes).unwrap_err();
    assert!(error.to_string().contains("invalid --include env name"));
}

#[test]
fn redact_sensitive_args_hides_secret_values() {
    let args = vec![
        "--api-key=sk-12345".to_string(),
        "--token".to_string(),
        "bearer-abc".to_string(),
        "--model".to_string(),
        "gpt-4o".to_string(),
        "--password=hunter2".to_string(),
    ];

    let redacted = redact_sensitive_args(&args);
    assert_eq!(
        redacted,
        vec![
            "--api-key=****",
            "--token",
            "****",
            "--model",
            "gpt-4o",
            "--password=****",
        ]
    );
}

#[test]
fn is_sensitive_key_catches_bare_key_suffix_and_other_secret_patterns() {
    assert!(is_sensitive_key("STRIPE_KEY"));
    assert!(is_sensitive_key("SENDGRID_KEY"));
    assert!(is_sensitive_key("NPM_AUTH"));
    assert!(is_sensitive_key("GITHUB_PAT"));
    assert!(is_sensitive_key("DB_PWD"));
    assert!(is_sensitive_key("SSL_PEM"));
}

#[test]
fn is_sensitive_key_preserves_public_key_and_ssh_auth_sock() {
    assert!(!is_sensitive_key("PUBLIC_KEY"));
    assert!(!is_sensitive_key("SSH_AUTH_SOCK"));
    assert!(!is_sensitive_key("OLDPWD"));
    assert!(!is_sensitive_key("CWD"));
}

#[test]
fn redact_sensitive_args_is_case_insensitive_and_alias_aware() {
    let args = vec![
        "--ApiKey=sk-secret".to_string(),
        "--API_KEY".to_string(),
        "sk-other".to_string(),
        "-k".to_string(),
        "ghp_12345".to_string(),
        "--bearer".to_string(),
        "xoxb-token".to_string(),
        "AKIAIOSFODNN7EXAMPLE".to_string(),
    ];

    let redacted = redact_sensitive_args(&args);
    assert_eq!(
        redacted,
        vec![
            "--ApiKey=****",
            "--API_KEY",
            "****",
            "-k",
            "****",
            "--bearer",
            "****",
            "****",
        ]
    );
}

#[test]
fn redact_sensitive_args_preserves_non_secret_args() {
    let args = vec![
        "--json".to_string(),
        "summarize".to_string(),
        "--model".to_string(),
        "gpt-4o".to_string(),
    ];

    let redacted = redact_sensitive_args(&args);
    assert_eq!(redacted, args);
}

#[test]
fn package_selection_prefers_longest_matching_prefix() {
    let cwd = Path::new("/repo/apps/browser/my-app/src");
    let packages = vec![
        Package {
            path: PathBuf::from("/repo/apps/browser"),
            relative: "apps/browser".to_string(),
            package_area: "apps".to_string(),
            name: "browser-root".to_string(),
            ecosystem: Default::default(),
            standard: Default::default(),
            provenance: Default::default(),
            test_runners: vec![],
            nested_packages: vec![],
            primary_language: None,
            secondary_languages: vec![],
            frameworks: vec![],
            file_associations: vec![],
            languages: vec![],
            configuration: vec![],
            documentation: vec![],
            editor_config: None,
            command_runner: vec![],
            package_managers: vec![],
            version: None,
            features: vec![],
            depends_on: vec![],
            used_by: vec![],
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            is_updatable: None,
            has_major_update: None,
            is_excluded: false,
        },
        Package {
            path: PathBuf::from("/repo/apps/browser/my-app"),
            relative: "apps/browser/my-app".to_string(),
            package_area: "apps/browser".to_string(),
            name: "my-app".to_string(),
            ecosystem: Default::default(),
            standard: Default::default(),
            provenance: Default::default(),
            test_runners: vec![],
            nested_packages: vec![],
            primary_language: None,
            secondary_languages: vec![],
            frameworks: vec![],
            file_associations: vec![],
            languages: vec![],
            configuration: vec![],
            documentation: vec![],
            editor_config: None,
            command_runner: vec![],
            package_managers: vec![],
            version: None,
            features: vec![],
            depends_on: vec![],
            used_by: vec![],
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            is_updatable: None,
            has_major_update: None,
            is_excluded: false,
        },
    ];

    let selected = select_package_for_cwd(cwd, &packages).unwrap();
    assert_eq!(selected.package, Some("my-app".to_string()));
    assert_eq!(selected.package_area, "apps/browser");
}

#[test]
fn package_area_selection_supports_area_root_without_package_match() {
    let cwd = Path::new("/repo/claudine");
    let repo_root = Path::new("/repo");
    let packages = vec![
        Package {
            path: PathBuf::from("/repo/claudine/lib"),
            relative: "claudine/lib".to_string(),
            package_area: "claudine".to_string(),
            name: "claudine".to_string(),
            ecosystem: Default::default(),
            standard: Default::default(),
            provenance: Default::default(),
            test_runners: vec![],
            nested_packages: vec![],
            primary_language: None,
            secondary_languages: vec![],
            frameworks: vec![],
            file_associations: vec![],
            languages: vec![],
            configuration: vec![],
            documentation: vec![],
            editor_config: None,
            command_runner: vec![],
            package_managers: vec![],
            version: None,
            features: vec![],
            depends_on: vec![],
            used_by: vec![],
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            is_updatable: None,
            has_major_update: None,
            is_excluded: false,
        },
        Package {
            path: PathBuf::from("/repo/claudine/cli"),
            relative: "claudine/cli".to_string(),
            package_area: "claudine".to_string(),
            name: "claudine-cli".to_string(),
            ecosystem: Default::default(),
            standard: Default::default(),
            provenance: Default::default(),
            test_runners: vec![],
            nested_packages: vec![],
            primary_language: None,
            secondary_languages: vec![],
            frameworks: vec![],
            file_associations: vec![],
            languages: vec![],
            configuration: vec![],
            documentation: vec![],
            editor_config: None,
            command_runner: vec![],
            package_managers: vec![],
            version: None,
            features: vec![],
            depends_on: vec![],
            used_by: vec![],
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            is_updatable: None,
            has_major_update: None,
            is_excluded: false,
        },
    ];

    assert!(select_package_for_cwd(cwd, &packages).is_none());
    let area = select_package_area_for_cwd(cwd, repo_root, &packages);
    assert_eq!(area, Some("claudine".to_string()));
    assert_eq!(
        package_candidates_for_area("claudine", &packages),
        vec!["claudine".to_string(), "claudine-cli".to_string()]
    );
}

fn fake_repo_info(root: &Path) -> RepoInfo {
    RepoInfo {
        is_monorepo: false,
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages: None,
        monorepo_standards: vec![],
        monorepo_layers: vec![],
    }
}

#[test]
fn launch_workspace_repo_root_follows_source_when_outside_launch_repo() {
    // Out-of-repo prompt case: launch CWD lives in repo A, prompt
    // markdown lives in repo B. `repo_root` (metadata) must follow
    // repo B so guardrails / MCP / harness key off the document repo,
    // but `child_cwd` must follow repo A so the spawned provider
    // process stays in the user's worktree.
    let launch_cwd = PathBuf::from("/repo-a/sub");
    let launch_git = PathBuf::from("/repo-a");
    let source_repo = PathBuf::from("/repo-b");
    let launch_repo_info = fake_repo_info(&launch_git);

    let ctx = launch_workspace_context_from_repo_info(
        &launch_cwd,
        Some(&launch_git),
        Some(&launch_repo_info),
        Some(&source_repo),
    );

    assert_eq!(ctx.repo_root.as_deref(), Some(source_repo.as_path()));
    assert_eq!(ctx.child_cwd, launch_git);
    assert_eq!(ctx.launch_cwd, launch_cwd);
}

#[test]
fn launch_workspace_falls_back_to_launch_repo_when_no_source_hint() {
    // Common case (source inside launch repo, or direct wrapper with no
    // source at all): `repo_root` and `child_cwd` should both follow
    // the launch git root.
    let launch_cwd = PathBuf::from("/repo-a/sub");
    let launch_git = PathBuf::from("/repo-a");
    let launch_repo_info = fake_repo_info(&launch_git);

    let ctx = launch_workspace_context_from_repo_info(
        &launch_cwd,
        Some(&launch_git),
        Some(&launch_repo_info),
        None,
    );

    assert_eq!(ctx.repo_root.as_deref(), Some(launch_git.as_path()));
    assert_eq!(ctx.child_cwd, launch_git);
}

#[test]
fn launch_workspace_source_hint_with_no_launch_repo_uses_source_for_meta_only() {
    // Edge case: launch CWD is not inside any git repo, but the
    // composed source lives in one. Metadata follows the source repo;
    // the child process still launches in the user's CWD because we
    // have no launch-repo root to anchor it.
    let launch_cwd = PathBuf::from("/tmp/scratch");
    let source_repo = PathBuf::from("/repo-b");

    let ctx =
        launch_workspace_context_from_repo_info(&launch_cwd, None, None, Some(&source_repo));

    assert_eq!(ctx.repo_root.as_deref(), Some(source_repo.as_path()));
    assert_eq!(ctx.child_cwd, launch_cwd);
}

/// Regression: the spawned child's `PWD` env var must equal
/// `child_cwd`, not whatever the parent shell set as `PWD` before
/// invoking claudine. OpenCode (and other shell-aware tooling)
/// reads `process.env.PWD` BEFORE falling back to `process.cwd()`
/// when resolving its project root; without this sync claudine
/// silently leaks the user's pre-invocation shell PWD (often a
/// package subdirectory of the worktree) into the child, causing
/// git snapshot pathspec mismatches and external_directory false
/// positives.
#[test]
fn build_child_env_overrides_pwd_to_match_child_cwd() {
    let profile = profile_for_provider(claudine::provider::Provider::OpenCode).unwrap();
    let cwd = tempfile::tempdir().unwrap();

    let plan = build_child_env(
        profile,
        claudine::provider::Provider::OpenCode,
        &[],
        false,
        false,
        &[],
        cwd.path(),
        &[],
        false,
        false,
        None,
    )
    .unwrap();

    let pwd = plan
        .env
        .get(std::ffi::OsStr::new("PWD"))
        .map(|v| v.to_string_lossy().into_owned())
        .expect("env plan must always set PWD for the spawned child");
    assert_eq!(
        std::path::PathBuf::from(&pwd),
        plan.child_cwd,
        "child PWD must equal child_cwd; got PWD={pwd:?} child_cwd={:?}",
        plan.child_cwd,
    );
}

#[test]
fn build_child_env_overwrites_stale_agent_cwd_with_process_launch_directory() {
    let expected = claudine::child_environment::initialize_process_launch_directory(
        claudine::child_environment::LaunchDirectoryMode::Ordinary,
    )
    .unwrap()
    .to_path_buf();
    let profile = profile_for_provider(claudine::provider::Provider::Claude).unwrap();
    let cwd = tempfile::tempdir().unwrap();

    let plan = build_child_env(
        profile,
        claudine::provider::Provider::Claude,
        &[],
        false,
        false,
        &[],
        cwd.path(),
        &[("AGENT_CWD".to_string(), "stale/value".to_string())],
        false,
        false,
        None,
    )
    .unwrap();

    assert_eq!(
        plan.env.get(std::ffi::OsStr::new("AGENT_CWD")),
        Some(&expected.clone().into_os_string())
    );
    assert_eq!(
        plan.added
            .iter()
            .find(|(key, _)| key == "AGENT_CWD")
            .map(|(_, value)| std::path::PathBuf::from(value)),
        Some(expected)
    );
}

/// The wrapper stamps two interactiveness signals with distinct
/// audiences: the child-facing `INTERACTIVE` ("true"/"false") and the
/// `CLAUDINE_`-namespaced `CLAUDINE_INTERACTIVE` ("1"/"0") gate that the
/// hook subprocess (`claudine handle`) reads to drive the Trigger 2 idle
/// producer. Both must reflect the launch mode.
#[test]
fn build_child_env_stamps_interactive_gates_for_child_and_hook() {
    let profile = profile_for_provider(claudine::provider::Provider::Claude).unwrap();
    let cwd = tempfile::tempdir().unwrap();

    let interactive_plan = build_child_env(
        profile,
        claudine::provider::Provider::Claude,
        &[],
        false,
        true,
        &[],
        cwd.path(),
        &[],
        false,
        false,
        None,
    )
    .unwrap();

    let added: std::collections::HashMap<_, _> = interactive_plan.added.into_iter().collect();
    assert_eq!(added.get("INTERACTIVE").map(String::as_str), Some("true"));
    assert_eq!(added.get("CLAUDINE_INTERACTIVE").map(String::as_str), Some("1"));

    let non_interactive_plan = build_child_env(
        profile,
        claudine::provider::Provider::Claude,
        &[],
        false,
        false,
        &[],
        cwd.path(),
        &[],
        false,
        false,
        None,
    )
    .unwrap();

    let added: std::collections::HashMap<_, _> = non_interactive_plan.added.into_iter().collect();
    assert_eq!(added.get("INTERACTIVE").map(String::as_str), Some("false"));
    assert_eq!(added.get("CLAUDINE_INTERACTIVE").map(String::as_str), Some("0"));
}

/// `CLAUDINE_PID` must be stamped onto every wrapper env plan so the
/// spawned provider can correlate back to the Claudine process. It
/// is provider-agnostic, so a single assertion covers every profile.
#[test]
fn build_child_env_includes_claudine_pid_for_interactive_wrapper() {
    let profile = profile_for_provider(claudine::provider::Provider::Claude).unwrap();
    let cwd = tempfile::tempdir().unwrap();

    let plan = build_child_env(
        profile,
        claudine::provider::Provider::Claude,
        &[],
        false,
        true,
        &[],
        cwd.path(),
        &[],
        false,
        false,
        None,
    )
    .unwrap();

    let added: std::collections::HashMap<_, _> = plan.added.into_iter().collect();
    let pid_str = added
        .get("CLAUDINE_PID")
        .expect("CLAUDINE_PID must be added to every wrapper env plan");
    let pid: u32 = pid_str
        .parse()
        .expect("CLAUDINE_PID must be a valid u32 (matches std::process::id())");
    assert_eq!(pid, std::process::id());
    assert_eq!(
        plan.env
            .get(std::ffi::OsStr::new("CLAUDINE_PID"))
            .map(|v| v.to_string_lossy().into_owned())
            .as_deref(),
        Some(pid_str.as_str()),
        "CLAUDINE_PID must also be present in the child env map"
    );
}

/// Non-interactive wrapper runs (compose, inline-compose, sequence,
/// harness attempts) must receive the same `CLAUDINE_PID` injection
/// as interactive ones — the spec requires it regardless of mode.
#[test]
fn build_child_env_includes_claudine_pid_for_non_interactive_wrapper() {
    let profile = profile_for_provider(claudine::provider::Provider::OpenCode).unwrap();
    let cwd = tempfile::tempdir().unwrap();

    let plan = build_child_env(
        profile,
        claudine::provider::Provider::OpenCode,
        &[],
        false,
        false,
        &[],
        cwd.path(),
        &[],
        false,
        false,
        None,
    )
    .unwrap();

    let added: std::collections::HashMap<_, _> = plan.added.into_iter().collect();
    let pid_str = added
        .get("CLAUDINE_PID")
        .expect("CLAUDINE_PID must be added even in non-interactive mode");
    let pid: u32 = pid_str.parse().expect("CLAUDINE_PID must parse as u32");
    assert_eq!(pid, std::process::id());
}

/// Regression for the source-repo vs launch-repo split through the real
/// env wiring. When a composed source document lives in one repo (its
/// enclosing git root becomes `repo_root`, the metadata anchor) but the
/// user launched from a different repo (`child_cwd`), Codex shadow-HOME
/// prompt materialization must follow `child_cwd`, NOT the source
/// metadata root. This exercises `build_child_env_with_launch` ->
/// `needs_shadow_home` -> `build_repo_home_env`, so a future change that
/// accidentally threads `repo_root`/source metadata into the shadow-HOME
/// call is caught here even if the low-level `repo_home` tests stay green.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn build_child_env_codex_shadow_home_uses_child_cwd_not_source_repo_root() {
    let tmp = tempfile::tempdir().unwrap();
    let fake_home = tmp.path().join("home");
    let launch_repo = tmp.path().join("launch-repo");
    let source_repo = tmp.path().join("source-repo");

    fs::create_dir_all(fake_home.join(".codex")).unwrap();
    fs::create_dir_all(launch_repo.join(".claude/commands")).unwrap();
    fs::create_dir_all(source_repo.join(".claude/commands")).unwrap();
    fs::write(launch_repo.join(".claude/commands/launch.md"), "launch").unwrap();
    fs::write(source_repo.join(".claude/commands/source.md"), "source").unwrap();

    let profile = profile_for_provider(claudine::provider::Provider::Codex).unwrap();
    // repo_root (metadata) follows the source document's repo; child_cwd
    // (where the spawned provider runs) follows the launch repo.
    let launch_ctx = LaunchWorkspaceContext {
        launch_cwd: launch_repo.clone(),
        repo_root: Some(source_repo.clone()),
        child_cwd: launch_repo.clone(),
        package_context: None,
        warnings: Vec::new(),
    };

    let old_home = std::env::var_os("HOME");
    unsafe { std::env::set_var("HOME", &fake_home) };

    let plan = build_child_env_with_launch(
        profile,
        claudine::provider::Provider::Codex,
        &[],
        false,
        false,
        &[],
        &[],
        false,
        false,
        launch_ctx,
        false,
    );

    match old_home {
        Some(v) => unsafe { std::env::set_var("HOME", v) },
        None => unsafe { std::env::remove_var("HOME") },
    }

    let plan = plan.unwrap();
    let shadow_path = plan
        .shadow_home_path
        .expect("Codex repo-local prompts must trigger a shadow home");
    let prompts_dir = shadow_path.join("prompts");

    assert!(
        fs::symlink_metadata(prompts_dir.join("launch.md")).is_ok(),
        "shadow prompts must come from child_cwd (launch repo)"
    );
    assert!(
        fs::symlink_metadata(prompts_dir.join("source.md")).is_err(),
        "shadow prompts must NOT come from the source metadata repo_root"
    );
}

fn sanitize_env_for_test(
    env: &[(String, String)],
    include_set: &HashSet<String>,
) -> (Vec<(String, String)>, Vec<String>) {
    let auto_include = HashSet::new();
    sanitize_env_for_test_with_auto(env, include_set, &auto_include)
}

fn sanitize_env_for_test_with_auto(
    env: &[(String, String)],
    include_set: &HashSet<String>,
    auto_include: &HashSet<String>,
) -> (Vec<(String, String)>, Vec<String>) {
    let mut kept = Vec::new();
    let mut removed = BTreeSet::new();

    for (key, value) in env {
        if is_sensitive_key(key) && !include_set.contains(key) && !auto_include.contains(key) {
            removed.insert(key.clone());
        } else {
            kept.push((key.clone(), value.clone()));
        }
    }

    (kept, removed.into_iter().collect())
}

fn init_git_repo(path: &Path) -> bool {
    Command::new("git")
        .arg("init")
        .current_dir(path)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[test]
fn promptless_interactive_startup_skips_git_status_capture() {
    let repo = tempfile::tempdir().unwrap();
    fs::write(
        repo.path().join("Cargo.toml"),
        r#"[package]
name = "startup-probe"
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();

    if !init_git_repo(repo.path()) {
        eprintln!("Skipping integration test: git init unavailable");
        return;
    }

    let interactive = detect_wrap_startup(repo.path(), false).unwrap();
    let prompted = detect_wrap_startup(repo.path(), true).unwrap();

    let interactive_work = interactive.invocation.work_snapshot();
    let prompted_work = prompted.invocation.work_snapshot();
    assert_eq!(interactive_work.git_root_discoveries, 1);
    assert_eq!(interactive_work.topology_probes, 0);
    assert_eq!(prompted_work.git_root_discoveries, 1);
    assert_eq!(prompted_work.topology_probes, 1);
    assert!(!interactive.env_context.git.unwrap().is_dirty);
    assert_eq!(
        interactive.launch_workspace.package_context.unwrap().package_area,
        "root"
    );
    assert!(prompted.env_context.git.unwrap().is_dirty);
}

#[test]
fn repo_root_hint_sets_metadata_but_not_child_cwd() {
    // The hint describes the composition source's enclosing repo
    // (used for guardrails, MCP, harness path resolution). The
    // child process must still spawn in the user's launch directory
    // — never in whatever repo the document happens to live in.
    let profile = profile_for_provider(claudine::provider::Provider::Claude).unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let hint_dir = tempfile::tempdir().unwrap();

    let plan = build_child_env(
        profile,
        claudine::provider::Provider::Claude,
        &[],
        false,
        false,
        &[],
        cwd.path(),
        &[],
        false,
        false,
        Some(hint_dir.path()),
    )
    .unwrap();

    assert_eq!(plan.repo_root.as_deref(), Some(hint_dir.path()));
    assert_eq!(plan.child_cwd.as_path(), cwd.path());
}

#[test]
fn repo_root_hint_none_falls_back_to_cwd_detection() {
    let profile = profile_for_provider(claudine::provider::Provider::Claude).unwrap();
    let cwd = tempfile::tempdir().unwrap();

    let plan = build_child_env(
        profile,
        claudine::provider::Provider::Claude,
        &[],
        false,
        false,
        &[],
        cwd.path(),
        &[],
        false,
        false,
        None,
    )
    .unwrap();

    // With None hint and a non-git tempdir, repo_root should be None
    // (CWD detection finds no git repo in a tempdir).
    assert_eq!(plan.repo_root, None);
    assert_eq!(plan.child_cwd.as_path(), cwd.path());
}

#[test]
fn launch_workspace_context_keeps_repo_root_when_package_resolution_fails() {
    let repo_root = tempfile::tempdir().unwrap();
    let docs_dir = repo_root.path().join("docs");
    let package_dir = repo_root.path().join("claudine/cli");
    let lib_dir = repo_root.path().join("claudine/lib");
    fs::create_dir_all(&docs_dir).unwrap();
    fs::create_dir_all(&package_dir).unwrap();
    fs::create_dir_all(&lib_dir).unwrap();

    fs::write(
        repo_root.path().join("Cargo.toml"),
        r#"[workspace]
members = ["claudine/cli", "claudine/lib"]
"#,
    )
    .unwrap();
    fs::write(
        package_dir.join("Cargo.toml"),
        r#"[package]
name = "claudine-cli"
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();
    fs::write(
        lib_dir.join("Cargo.toml"),
        r#"[package]
name = "claudine-lib"
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();

    if !init_git_repo(repo_root.path()) {
        eprintln!("Skipping integration test: git init unavailable");
        return;
    }

    let ctx = resolve_launch_workspace_context(&docs_dir, None);
    let canonical_repo_root = repo_root.path().canonicalize().unwrap();

    assert_eq!(ctx.launch_cwd, docs_dir);
    assert_eq!(
        ctx.repo_root.as_deref().map(canonical_or_self),
        Some(canonical_repo_root.clone())
    );
    assert_eq!(canonical_or_self(&ctx.child_cwd), canonical_repo_root);
    assert!(ctx.package_context.is_none());
    assert!(
        ctx.warnings
            .iter()
            .any(|warning| warning.contains("no package area matched cwd")),
        "expected package-context warning, got: {:?}",
        ctx.warnings
    );
}
