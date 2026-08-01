#[cfg(unix)]
use std::process::Command;

use super::*;
use serial_test::serial;
use tempfile::TempDir;
use test_toolkit::EnvGuard;

#[cfg(unix)]
fn init_git_repo(path: &Path) -> bool {
    Command::new("git")
        .arg("init")
        .arg("-q")
        .current_dir(path)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[test]
fn purge_volatile_state_preserves_shadow_owned_databases() {
    let tmp = TempDir::new().unwrap();
    let shadow = tmp.path();

    fs::write(shadow.join("goals_1.sqlite"), b"db").unwrap();
    fs::write(shadow.join("goals_1.sqlite-wal"), b"wal").unwrap();
    fs::write(shadow.join("goals_1.sqlite-shm"), b"stale-shm").unwrap();
    fs::write(shadow.join("config.toml"), b"cfg").unwrap();

    purge_volatile_state(shadow).unwrap();

    assert!(shadow.join("goals_1.sqlite").exists());
    assert!(shadow.join("goals_1.sqlite-wal").exists());
    assert!(shadow.join("goals_1.sqlite-shm").exists());
    assert!(shadow.join("config.toml").exists());
}

#[cfg(unix)]
#[test]
fn purge_volatile_state_removes_legacy_link_and_sidecars() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("source.sqlite");
    let shadow = tmp.path().join("shadow");
    fs::write(&source, b"db").unwrap();
    fs::create_dir(&shadow).unwrap();
    std::os::unix::fs::symlink(&source, shadow.join("goals_1.sqlite")).unwrap();
    fs::write(shadow.join("goals_1.sqlite-wal"), b"wal").unwrap();
    fs::write(shadow.join("goals_1.sqlite-shm"), b"stale-shm").unwrap();
    fs::write(shadow.join("config.toml"), b"cfg").unwrap();

    purge_volatile_state(&shadow).unwrap();

    assert!(!shadow.join("goals_1.sqlite").exists());
    assert!(!shadow.join("goals_1.sqlite-wal").exists());
    assert!(!shadow.join("goals_1.sqlite-shm").exists());
    assert!(shadow.join("config.toml").exists());
    assert!(source.exists());
}

#[test]
fn purge_volatile_state_tolerates_missing_shadow_dir() {
    let tmp = TempDir::new().unwrap();
    let missing = tmp.path().join("does-not-exist");
    purge_volatile_state(&missing).unwrap();
}

#[test]
fn volatile_state_files_match_live_dbs_only() {
    // Live DBs + sidecars must be detected (never shared via symlink).
    assert!(is_volatile_state_file(OsStr::new("state_5.sqlite")));
    assert!(is_volatile_state_file(OsStr::new("logs_2.sqlite-wal")));
    assert!(is_volatile_state_file(OsStr::new("memories_1.sqlite-shm")));
    assert!(is_volatile_state_file(OsStr::new("goals_1.sqlite-journal")));

    // Shared config and codex's own repair backups must NOT match.
    assert!(!is_volatile_state_file(OsStr::new("config.toml")));
    assert!(!is_volatile_state_file(OsStr::new("auth.json")));
    assert!(!is_volatile_state_file(OsStr::new(
        "state_5.sqlite.codex-repair-1780436523.0.bak"
    )));
}

#[test]
#[serial]
fn codex_sqlite_home_defaults_to_pre_shadow_codex_home() {
    let _codex_home = EnvGuard::remove_safe("CODEX_HOME");
    let _sqlite_home = EnvGuard::remove_safe("CODEX_SQLITE_HOME");
    let native_home = dirs::home_dir().expect("native user home should resolve");

    assert_eq!(codex_sqlite_home().unwrap(), native_home.join(".codex"));
}

#[test]
#[serial]
fn codex_sqlite_home_respects_codex_home_and_explicit_sqlite_home() {
    let tmp = TempDir::new().unwrap();
    let codex_home = tmp.path().join("custom-codex");
    let sqlite_home = tmp.path().join("custom-sqlite");
    let _home = EnvGuard::set_safe("HOME", tmp.path());
    let _codex_home = EnvGuard::set_safe("CODEX_HOME", &codex_home);
    let _sqlite_home = EnvGuard::remove_safe("CODEX_SQLITE_HOME");

    assert_eq!(codex_sqlite_home().unwrap(), codex_home);

    let _sqlite_home = EnvGuard::set_safe("CODEX_SQLITE_HOME", &sqlite_home);
    assert_eq!(codex_sqlite_home().unwrap(), sqlite_home);
}

#[test]
#[serial]
fn codex_sqlite_home_rejects_relative_paths() {
    let _sqlite_home = EnvGuard::set_safe("CODEX_SQLITE_HOME", "relative/state");
    assert!(codex_sqlite_home().is_err());
}

#[cfg(unix)]
#[test]
#[serial]
fn codex_shadow_home_uses_real_sqlite_directory_and_preserves_legacy_state() {
    let tmp = TempDir::new().unwrap();
    let user_home = tmp.path().join("home");
    let repo = tmp.path().join("repo");
    let real_codex_home = user_home.join(".codex");
    let shadow_codex_home = user_home.join(".claudine/.codex");
    let legacy_state = shadow_codex_home.join("state_5.sqlite");
    fs::create_dir_all(&real_codex_home).unwrap();
    fs::create_dir_all(&shadow_codex_home).unwrap();
    fs::create_dir_all(&repo).unwrap();
    fs::write(&legacy_state, b"legacy-shadow-state").unwrap();

    let _home = EnvGuard::set_safe("HOME", &user_home);
    let _codex_home = EnvGuard::remove_safe("CODEX_HOME");
    let _sqlite_home = EnvGuard::remove_safe("CODEX_SQLITE_HOME");

    let (env, _, _) =
        build_repo_home_env(Provider::Codex, &repo, false, false, Some(&repo)).unwrap();

    assert_eq!(
        env.get(OsStr::new("CODEX_SQLITE_HOME")),
        Some(&OsString::from(&real_codex_home))
    );
    assert_eq!(
        fs::read(&legacy_state).unwrap(),
        b"legacy-shadow-state",
        "regular shadow-owned databases are recoverable legacy state"
    );
}

#[cfg(unix)]
#[test]
#[serial]
fn non_codex_shadow_home_does_not_receive_codex_sqlite_home() {
    let tmp = TempDir::new().unwrap();
    let user_home = tmp.path().join("home");
    let repo = tmp.path().join("repo");
    fs::create_dir_all(user_home.join(".claude")).unwrap();
    fs::create_dir_all(&repo).unwrap();

    let _home = EnvGuard::set_safe("HOME", &user_home);
    let _sqlite_home = EnvGuard::remove_safe("CODEX_SQLITE_HOME");

    let (env, _, _) =
        build_repo_home_env(Provider::Claude, &repo, true, false, Some(&repo)).unwrap();

    assert!(!env.contains_key(OsStr::new("CODEX_SQLITE_HOME")));
}

#[test]
fn codex_repo_prompts_source_prefers_codex_dir_then_claude_commands() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    let claude_commands = repo_root.join(".claude/commands");
    let codex_prompts = repo_root.join(".codex/prompts");

    fs::create_dir_all(&claude_commands).unwrap();
    assert_eq!(
        codex_repo_prompts_source(repo_root).as_deref(),
        Some(claude_commands.as_path())
    );

    fs::create_dir_all(&codex_prompts).unwrap();
    assert_eq!(
        codex_repo_prompts_source(repo_root).as_deref(),
        Some(codex_prompts.as_path())
    );
}

#[cfg(unix)]
#[test]
fn materialize_repo_scoped_resources_merges_user_and_repo_prompts() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path().join("repo");
    let shadow_home = tmp.path().join("shadow/.codex");
    let original_home = tmp.path().join("home/.codex");
    let user_prompts = original_home.join("prompts");
    let claude_commands = repo_root.join(".claude/commands");
    let prompts_dir = shadow_home.join("prompts");
    let repo_review = claude_commands.join("review.md");
    let user_review = user_prompts.join("review.md");
    let user_commit = user_prompts.join("commit.md");

    fs::create_dir_all(&claude_commands).unwrap();
    fs::create_dir_all(&user_prompts).unwrap();
    fs::create_dir_all(&shadow_home).unwrap();
    fs::write(&user_review, "user review").unwrap();
    fs::write(&user_commit, "user commit").unwrap();
    fs::write(&repo_review, "repo review").unwrap();
    fs::create_dir_all(claude_commands.join("nested")).unwrap();
    fs::write(claude_commands.join("nested/plan.md"), "repo nested").unwrap();

    materialize_repo_scoped_resources(
        Provider::Codex,
        &shadow_home,
        &original_home,
        &repo_root,
        false,
    )
    .unwrap();

    assert_eq!(
        fs::read_link(prompts_dir.join("review.md")).unwrap(),
        repo_review
    );
    assert_eq!(
        fs::read_link(prompts_dir.join("commit.md")).unwrap(),
        user_commit
    );
    assert_eq!(
        fs::read_link(prompts_dir.join("nested/plan.md")).unwrap(),
        claude_commands.join("nested/plan.md")
    );
}

#[cfg(unix)]
#[test]
fn materialize_repo_scoped_resources_repo_only_uses_repo_prompts() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path().join("repo");
    let shadow_home = tmp.path().join("shadow/.codex");
    let original_home = tmp.path().join("home/.codex");
    let user_prompts = original_home.join("prompts");
    let claude_commands = repo_root.join(".claude/commands");
    let prompts_dir = shadow_home.join("prompts");

    fs::create_dir_all(&user_prompts).unwrap();
    fs::create_dir_all(&claude_commands).unwrap();
    fs::create_dir_all(&shadow_home).unwrap();
    fs::write(user_prompts.join("commit.md"), "user commit").unwrap();
    fs::write(claude_commands.join("review.md"), "repo review").unwrap();

    materialize_repo_scoped_resources(
        Provider::Codex,
        &shadow_home,
        &original_home,
        &repo_root,
        true,
    )
    .unwrap();

    assert!(fs::symlink_metadata(prompts_dir.join("commit.md")).is_err());
    assert_eq!(
        fs::read_link(prompts_dir.join("review.md")).unwrap(),
        claude_commands.join("review.md")
    );
}

#[cfg(unix)]
#[test]
fn materialize_repo_scoped_resources_replaces_existing_repo_overlay_when_switching_repos() {
    let tmp = TempDir::new().unwrap();
    let first_repo = tmp.path().join("repo-one");
    let second_repo = tmp.path().join("repo-two");
    let original_home = tmp.path().join("home/.codex");
    let shadow_home = tmp.path().join("shadow/.codex");
    let prompts_dir = shadow_home.join("prompts");
    let first_review = first_repo.join(".claude/commands/review.md");
    let second_review = second_repo.join(".claude/commands/review.md");

    fs::create_dir_all(first_review.parent().unwrap()).unwrap();
    fs::create_dir_all(second_review.parent().unwrap()).unwrap();
    fs::create_dir_all(&shadow_home).unwrap();
    fs::create_dir_all(original_home.join("prompts")).unwrap();
    fs::write(&first_review, "repo one").unwrap();
    fs::write(&second_review, "repo two").unwrap();

    materialize_repo_scoped_resources(
        Provider::Codex,
        &shadow_home,
        &original_home,
        &first_repo,
        false,
    )
    .unwrap();
    assert_eq!(
        fs::read_link(prompts_dir.join("review.md")).unwrap(),
        first_review
    );

    materialize_repo_scoped_resources(
        Provider::Codex,
        &shadow_home,
        &original_home,
        &second_repo,
        false,
    )
    .unwrap();
    assert_eq!(
        fs::read_link(prompts_dir.join("review.md")).unwrap(),
        second_review
    );
}

#[cfg(unix)]
#[test]
fn materialize_root_level_state_preserves_claude_global_state() {
    let tmp = TempDir::new().unwrap();
    let user_home = tmp.path().join("home");
    let shadow_home_root = tmp.path().join("shadow");
    let source = user_home.join(".claude.json");
    let dest = shadow_home_root.join(".claude.json");

    fs::create_dir_all(&user_home).unwrap();
    fs::create_dir_all(&shadow_home_root).unwrap();
    fs::write(&source, "{\"hasCompletedOnboarding\":true}").unwrap();

    materialize_root_level_state(Provider::Claude, &user_home, &shadow_home_root).unwrap();

    assert_eq!(fs::read_link(&dest).unwrap(), source);
}

#[cfg(unix)]
#[test]
fn materialize_root_level_state_replaces_stale_shadow_file() {
    let tmp = TempDir::new().unwrap();
    let user_home = tmp.path().join("home");
    let shadow_home_root = tmp.path().join("shadow");
    let source = user_home.join(".claude.json");
    let dest = shadow_home_root.join(".claude.json");

    fs::create_dir_all(&user_home).unwrap();
    fs::create_dir_all(&shadow_home_root).unwrap();
    fs::write(&source, "{\"userID\":\"real\"}").unwrap();
    fs::write(&dest, "{\"userID\":\"stale\"}").unwrap();

    materialize_root_level_state(Provider::Claude, &user_home, &shadow_home_root).unwrap();

    assert_eq!(fs::read_link(&dest).unwrap(), source);
}

#[test]
fn needs_shadow_home_supplied_effective_root_used_for_codex_detection() {
    let tmp = TempDir::new().unwrap();
    let with_prompts = tmp.path().join("with-prompts");
    let without_prompts = tmp.path().join("without-prompts");
    fs::create_dir_all(with_prompts.join(".codex/prompts")).unwrap();
    fs::create_dir_all(&without_prompts).unwrap();

    // When the supplied effective root contains prompts, Codex needs a
    // shadow home even if cwd lives somewhere without prompts.
    assert!(
        needs_shadow_home(Provider::Codex, &without_prompts, false, Some(&with_prompts)),
        "expected true when effective_root has codex prompts"
    );

    // When the supplied effective root lacks prompts, Codex does not need
    // a shadow home (repo_only is false).
    assert!(
        !needs_shadow_home(Provider::Codex, &with_prompts, false, Some(&without_prompts)),
        "expected false when effective_root has no codex prompts"
    );

    // Non-Codex providers are never affected by repo-local prompt detection.
    assert!(
        !needs_shadow_home(Provider::Claude, &with_prompts, false, Some(&with_prompts)),
        "expected false for non-Codex regardless of effective_root"
    );
}

#[test]
fn needs_shadow_home_repo_only_short_circuits_regardless_of_effective_root() {
    let tmp = TempDir::new().unwrap();
    let empty = tmp.path().join("empty");
    fs::create_dir_all(&empty).unwrap();

    // repo_only=true forces shadow home for every provider.
    assert!(needs_shadow_home(Provider::Codex, &empty, true, Some(&empty)));
    assert!(needs_shadow_home(Provider::Claude, &empty, true, Some(&empty)));
    assert!(needs_shadow_home(Provider::OpenCode, &empty, true, None));
}

/// Proves that `build_repo_home_env` materializes Codex repo prompts from
/// the supplied `effective_root` even when `cwd` points to a different
/// directory (or repo). This is the core Phase 3/4 contract: the caller
/// threads a pre-resolved launch-child root through the shadow-HOME API
/// so the redundant `resolve_repo_root(cwd)` sniff walk is skipped.
#[cfg(unix)]
#[test]
#[serial]
fn build_repo_home_env_uses_supplied_effective_root_not_cwd() {
    let tmp = TempDir::new().unwrap();
    let fake_home = tmp.path().join("home");
    let launch_repo = tmp.path().join("launch-repo");
    let source_repo = tmp.path().join("source-repo");

    fs::create_dir_all(fake_home.join(".codex")).unwrap();
    fs::create_dir_all(launch_repo.join(".claude/commands")).unwrap();
    fs::create_dir_all(source_repo.join(".claude/commands")).unwrap();
    fs::write(launch_repo.join(".claude/commands/launch.md"), "launch").unwrap();
    fs::write(source_repo.join(".claude/commands/source.md"), "source").unwrap();

    let old_home = std::env::var_os("HOME");
    unsafe { std::env::set_var("HOME", &fake_home) };

    let (_env, shadow_path, _timings) = build_repo_home_env(
        Provider::Codex,
        &source_repo, // cwd points to source repo (simulates metadata root)
        false,
        false,
        Some(&launch_repo), // effective_root is launch repo (simulates child_cwd)
    )
    .unwrap();

    // Restore HOME so later tests see a clean environment.
    match old_home {
        Some(v) => unsafe { std::env::set_var("HOME", v) },
        None => unsafe { std::env::remove_var("HOME") },
    }

    let shadow_path = shadow_path.expect("shadow home path must be returned");
    let prompts_dir = shadow_path.join("prompts");

    // Prompt materialization must follow effective_root (launch repo),
    // not cwd (source repo).
    assert!(
        fs::symlink_metadata(prompts_dir.join("launch.md")).is_ok(),
        "expected launch.md from effective_root in shadow home"
    );
    assert!(
        fs::symlink_metadata(prompts_dir.join("source.md")).is_err(),
        "expected source.md from cwd NOT in shadow home"
    );
}

/// Proves that `build_repo_home_env(..., None)` falls back to the legacy
/// `resolve_repo_root(cwd)` behavior, which walks up to the *git root* — not
/// the literal `cwd`. The repo-local prompt lives at the repository root,
/// while `cwd` is a nested subdirectory with no `.claude/commands` of its
/// own. The prompt only materializes if `resolve_repo_root` ascends to the
/// root, so this fails if the fallback degrades to `cwd.to_path_buf()`.
#[cfg(unix)]
#[test]
#[serial]
fn build_repo_home_env_fallback_resolves_repo_root_from_nested_cwd() {
    let tmp = TempDir::new().unwrap();
    let fake_home = tmp.path().join("home");
    let repo = tmp.path().join("repo");
    let nested_cwd = repo.join("crate/src/deep");

    fs::create_dir_all(fake_home.join(".codex")).unwrap();
    fs::create_dir_all(repo.join(".claude/commands")).unwrap();
    fs::create_dir_all(&nested_cwd).unwrap();
    fs::write(repo.join(".claude/commands/review.md"), "review").unwrap();

    if !init_git_repo(&repo) {
        // Skip when git is unavailable: without a detectable repo root the
        // fallback cannot distinguish itself from direct cwd reuse.
        return;
    }

    let old_home = std::env::var_os("HOME");
    unsafe { std::env::set_var("HOME", &fake_home) };

    // cwd is a nested subdir with no .claude/commands; only repo-root
    // resolution can locate the root-level prompt.
    let (_env, shadow_path, _timings) =
        build_repo_home_env(Provider::Codex, &nested_cwd, false, false, None).unwrap();

    match old_home {
        Some(v) => unsafe { std::env::set_var("HOME", v) },
        None => unsafe { std::env::remove_var("HOME") },
    }

    let shadow_path = shadow_path.expect("shadow home path must be returned");
    let prompts_dir = shadow_path.join("prompts");

    assert!(
        fs::symlink_metadata(prompts_dir.join("review.md")).is_ok(),
        "expected root-level review.md to materialize via resolve_repo_root from nested cwd"
    );
}
