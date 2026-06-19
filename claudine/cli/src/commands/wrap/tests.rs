//! Wrapper-orchestrator unit tests: cwd/PWD sync, binary resolution
//! preflight, and `package_name_display` rendering.

use super::*;
use super::exec::switch_process_cwd;
use std::collections::HashMap;

/// Regression: `switch_process_cwd` must update both the OS cwd
/// (`chdir(2)`) AND the `PWD` env var. Rust's `set_current_dir`
/// only does the former; the latter is the shell convention that
/// downstream tools (OpenCode, bash, fish, etc.) trust over the
/// real cwd. Leaving them out of sync produces spec-vs-reality
/// drift in child processes that resolve project / git roots from
/// `process.env.PWD`.
#[test]
fn switch_process_cwd_syncs_pwd_env_var() {
    // Test mutates process cwd and PWD; serialize it informally
    // by running synchronously and restoring before assert prints.
    let target = tempfile::tempdir().unwrap();
    // Canonicalize: macOS prefixes /private/ on /var paths and
    // `current_dir()` returns the canonical form.
    let target_canon = std::fs::canonicalize(target.path()).unwrap();

    let prior_cwd = std::env::current_dir().unwrap();
    let prior_pwd = std::env::var_os("PWD");
    // SAFETY: scoped mutation; we restore before returning.
    unsafe {
        std::env::set_var("PWD", "/definitely/not/the/target");
    }

    switch_process_cwd(&target_canon).unwrap();
    let observed_cwd = std::env::current_dir().unwrap();
    let observed_pwd = std::env::var_os("PWD");

    // Restore.
    let _ = std::env::set_current_dir(&prior_cwd);
    unsafe {
        match prior_pwd {
            Some(value) => std::env::set_var("PWD", value),
            None => std::env::remove_var("PWD"),
        }
    }

    assert_eq!(observed_cwd, target_canon, "chdir must take effect",);
    assert_eq!(
        observed_pwd.as_deref(),
        Some(target_canon.as_os_str()),
        "PWD env var must track child_cwd after switch_process_cwd \
         (the bug: chdir without PWD-sync lets child processes resolve \
         paths against stale shell PWD)",
    );
}

#[test]
fn missing_binary_preflight_has_actionable_message() {
    let clients = InstalledAiClients::default();
    let profile = profile::profile_for_provider(Provider::Codex).unwrap();

    let error = resolve_binary_path(profile, &clients).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("cannot run wrapped Codex session"));
    assert!(message.contains("docs:"));
}

/// W2 regression: when the prep snapshot already knows where the
/// provider binary lives, `resolve_binary_path_direct` must return
/// that path without touching `which::which`. We verify this by
/// seeding the snapshot with a synthetic path that does **not** exist
/// on `PATH`; if the function fell through to `which::which`, the
/// call would error with `binary_missing_error`.
#[test]
fn resolve_binary_path_direct_uses_snapshot_without_which_lookup() {
    use std::collections::{BTreeMap, BTreeSet};

    let synthetic = PathBuf::from("/nonexistent/cache/codex-bin-stub");
    let mut binary_paths = BTreeMap::new();
    binary_paths.insert(Provider::Codex, synthetic.clone());
    let snapshot = InstalledProviderSnapshot {
        runnable: vec![Provider::Codex],
        excluded: BTreeSet::new(),
        all_installed: vec![Provider::Codex],
        binary_paths,
    };

    let profile = profile::profile_for_provider(Provider::Codex).unwrap();
    let resolved =
        resolve_binary_path_direct(profile, Some(&snapshot)).expect("snapshot path wins");

    assert_eq!(
        resolved, synthetic,
        "snapshot path must be returned verbatim, not re-resolved via `which`"
    );
}

/// Companion: when the snapshot has no entry for the requested
/// provider, the legacy `which::which` fallback path is taken.
/// In an unhydrated environment (no real `codex` binary on PATH),
/// that path must still surface the actionable missing-binary error
/// rather than panic.
#[test]
fn resolve_binary_path_direct_falls_back_when_snapshot_lacks_provider() {
    use std::collections::{BTreeMap, BTreeSet};

    let snapshot = InstalledProviderSnapshot {
        runnable: vec![],
        excluded: BTreeSet::new(),
        all_installed: vec![],
        binary_paths: BTreeMap::new(),
    };
    let profile = profile::profile_for_provider(Provider::Codex).unwrap();

    // Force PATH to a directory we know contains no binaries so the
    // `which::which` fallback deterministically misses.
    let empty = tempfile::tempdir().unwrap();
    let prev_path = std::env::var_os("PATH");
    // SAFETY: tests in this binary are not parallelised across this
    // env var; the variable is restored before returning.
    unsafe {
        std::env::set_var("PATH", empty.path());
    }
    let result = resolve_binary_path_direct(profile, Some(&snapshot));
    unsafe {
        match prev_path {
            Some(p) => std::env::set_var("PATH", p),
            None => std::env::remove_var("PATH"),
        }
    }

    let error = result.expect_err("missing-binary fallback should error");
    assert!(
        error.to_string().contains("cannot run wrapped Codex"),
        "expected actionable error; got {error}"
    );
}

#[test]
fn package_name_display_shows_resolved_package_and_area() {
    let env_plan = env::EnvPlan {
        env: HashMap::new(),
        removed: Vec::new(),
        included: Vec::new(),
        added: Vec::new(),
        repo_root: None,
        child_cwd: PathBuf::from("/tmp"),
        package_context: Some(claudine::composition::PackageContext {
            package_area: "claudine".to_string(),
            package: Some("claudine-cli".to_string()),
            candidates: vec!["claudine-cli".to_string()],
        }),
        warnings: Vec::new(),
        shadow_home_path: None,
        perf_substages: Vec::new(),
    };

    let rendered = crate::output::package_name_display(&env_plan).unwrap();
    assert!(rendered.contains("claudine-cli"));
    assert!(rendered.contains("area: claudine"));
}

#[test]
fn package_name_display_is_hidden_when_package_is_ambiguous() {
    let env_plan = env::EnvPlan {
        env: HashMap::new(),
        removed: Vec::new(),
        included: Vec::new(),
        added: Vec::new(),
        repo_root: None,
        child_cwd: PathBuf::from("/tmp"),
        package_context: Some(claudine::composition::PackageContext {
            package_area: "claudine".to_string(),
            package: None,
            candidates: vec!["claudine".to_string(), "claudine-cli".to_string()],
        }),
        warnings: Vec::new(),
        shadow_home_path: None,
        perf_substages: Vec::new(),
    };

    assert!(crate::output::package_name_display(&env_plan).is_none());
}
