use super::*;
use serial_test::serial;
use tempfile::TempDir;

#[test]
fn explicit_append_file_resolves() {
    let tmp = TempDir::new().unwrap();
    let file_path = tmp.path().join("custom.md");
    std::fs::write(&file_path, "# Custom Prompt\nAppend me").unwrap();

    let args = SystemPromptArgs {
        append_file: Some(file_path.display().to_string()),
        ..Default::default()
    };
    let context = LaunchContext {
        agent: None,
        cwd: tmp.path().to_path_buf(),
        repo_root: None,
        package_area_root: None,
        package_root: None,
    };

    let result = resolve_system_prompt_source(&args, &context).unwrap();
    assert!(result.is_some());
    let (source, text) = result.unwrap();

    match source {
        SystemPromptSource::ExplicitFile { path, mode } => {
            assert_eq!(path, file_path);
            assert!(matches!(mode, SystemPromptMode::Append));
        }
        _ => panic!("Expected ExplicitFile source"),
    }
    assert_eq!(text, "# Custom Prompt\nAppend me");
}

#[test]
fn explicit_replace_file_resolves() {
    let tmp = TempDir::new().unwrap();
    let file_path = tmp.path().join("replacement.md");
    std::fs::write(&file_path, "# Replace Prompt\nReplace with this").unwrap();

    let args = SystemPromptArgs {
        replace_file: Some(file_path.display().to_string()),
        ..Default::default()
    };
    let context = LaunchContext {
        agent: None,
        cwd: tmp.path().to_path_buf(),
        repo_root: None,
        package_area_root: None,
        package_root: None,
    };

    let result = resolve_system_prompt_source(&args, &context).unwrap();
    assert!(result.is_some());
    let (source, text) = result.unwrap();

    match source {
        SystemPromptSource::ExplicitFile { path, mode } => {
            assert_eq!(path, file_path);
            assert!(matches!(mode, SystemPromptMode::Replace));
        }
        _ => panic!("Expected ExplicitFile source"),
    }
    assert_eq!(text, "# Replace Prompt\nReplace with this");
}

#[test]
fn explicit_file_not_found_errors() {
    let tmp = TempDir::new().unwrap();
    let nonexistent = tmp.path().join("does-not-exist.md");

    let args = SystemPromptArgs {
        append_file: Some(nonexistent.display().to_string()),
        ..Default::default()
    };
    let context = LaunchContext {
        agent: None,
        cwd: tmp.path().to_path_buf(),
        repo_root: None,
        package_area_root: None,
        package_root: None,
    };

    let result = resolve_system_prompt_source(&args, &context);
    assert!(result.is_err());
    match result.unwrap_err() {
        crate::error::ClaudineError::SystemPromptFileNotFound(_) => {}
        e => panic!("Expected SystemPromptFileNotFound, got {:?}", e),
    }
}

/// A `@`-prefixed `--append-system-prompt` reference is a magic-root search,
/// resolving under the supplied repository root — the migration added `@`
/// support the former `cwd.join` grammar lacked.
#[test]
fn explicit_append_at_prefix_searches_repository_root() {
    let repo = TempDir::new().unwrap();
    let target = repo.path().join("prompts").join("sys.md");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, "# repo sys\n").unwrap();
    let sub = repo.path().join("area");
    std::fs::create_dir_all(&sub).unwrap();

    let args = SystemPromptArgs {
        append_file: Some("@prompts/sys.md".to_string()),
        ..Default::default()
    };
    let context = LaunchContext {
        agent: None,
        cwd: sub,
        repo_root: Some(repo.path().to_path_buf()),
        package_area_root: None,
        package_root: None,
    };

    let (source, text) = resolve_system_prompt_source(&args, &context)
        .unwrap()
        .expect("magic reference resolves under the repository root");
    match source {
        SystemPromptSource::ExplicitFile { path, .. } => {
            assert_eq!(path.canonicalize().unwrap(), target.canonicalize().unwrap());
        }
        _ => panic!("expected ExplicitFile source, got {source:?}"),
    }
    assert_eq!(text, "# repo sys\n");
}

/// A `~`-prefixed `--append-system-prompt` reference is home-pinned through the
/// shared `Home` kind — support the former grammar lacked.
#[test]
fn explicit_append_tilde_resolves_against_home() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("sys.md"), "# home sys\n").unwrap();

    let args = SystemPromptArgs {
        append_file: Some("~/sys.md".to_string()),
        ..Default::default()
    };
    let context = LaunchContext {
        agent: None,
        cwd: tmp.path().to_path_buf(),
        repo_root: None,
        package_area_root: None,
        package_root: None,
    };
    let result = resolve_system_prompt_source_with_home(&args, &context, Some(&home));

    let (source, text) = result.unwrap().expect("~ reference resolves against HOME");
    match source {
        SystemPromptSource::ExplicitFile { path, .. } => {
            assert_eq!(path, home.join("sys.md"));
        }
        _ => panic!("expected ExplicitFile source, got {source:?}"),
    }
    assert_eq!(text, "# home sys\n");
}

/// A bare (implicit) `--append-system-prompt` reference is launch-relative
/// first: with the same basename at the repository root and launch directory,
/// the launch-directory copy wins.
#[test]
fn explicit_append_implicit_is_launch_relative_first() {
    let repo = TempDir::new().unwrap();
    std::fs::write(repo.path().join("sys.md"), "# repo\n").unwrap();
    let sub = repo.path().join("area");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("sys.md"), "# area\n").unwrap();

    let args = SystemPromptArgs {
        append_file: Some("sys.md".to_string()),
        ..Default::default()
    };
    let context = LaunchContext {
        agent: None,
        cwd: sub,
        repo_root: Some(repo.path().to_path_buf()),
        package_area_root: None,
        package_root: None,
    };

    let (_source, text) = resolve_system_prompt_source(&args, &context)
        .unwrap()
        .expect("implicit reference resolves");
    assert_eq!(
        text, "# area\n",
        "implicit reference must resolve from the launch directory first",
    );
}

#[test]
fn explicit_skips_standard_discovery() {
    let tmp = TempDir::new().unwrap();
    let package_root = tmp.path().join("package");
    std::fs::create_dir_all(&package_root).unwrap();

    // Create standard discovery file
    let standard_file = package_root.join("system-prompt.md");
    std::fs::write(
        &standard_file,
        "# Standard Discovery\nThis should be skipped",
    )
    .unwrap();

    // Create explicit file
    let explicit_file = tmp.path().join("explicit.md");
    std::fs::write(&explicit_file, "# Explicit File\nUse this instead").unwrap();

    let args = SystemPromptArgs {
        append_file: Some(explicit_file.display().to_string()),
        ..Default::default()
    };
    let context = LaunchContext {
        agent: None,
        cwd: tmp.path().to_path_buf(),
        repo_root: Some(tmp.path().to_path_buf()),
        package_area_root: None,
        package_root: Some(package_root.clone()),
    };

    let result = resolve_system_prompt_source(&args, &context).unwrap();
    assert!(result.is_some());
    let (source, text) = result.unwrap();

    match source {
        SystemPromptSource::ExplicitFile { path, .. } => {
            assert_eq!(path, explicit_file);
        }
        _ => panic!("Expected ExplicitFile source, got {:?}", source),
    }
    assert_eq!(text, "# Explicit File\nUse this instead");
}

#[test]
fn standard_discovery_package_wins() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    let area = repo.join("area");
    let package = area.join("package");
    std::fs::create_dir_all(&package).unwrap();

    std::fs::write(package.join("system-prompt.md"), "# Package").unwrap();
    std::fs::write(area.join("system-prompt.md"), "# Area").unwrap();
    std::fs::write(repo.join("system-prompt.md"), "# Repo").unwrap();

    let args = SystemPromptArgs::default();
    let context = LaunchContext {
        agent: None,
        cwd: package.clone(),
        repo_root: Some(repo.clone()),
        package_area_root: Some(area.clone()),
        package_root: Some(package.clone()),
    };

    let result = resolve_system_prompt_source(&args, &context).unwrap();
    assert!(result.is_some());
    let (source, text) = result.unwrap();

    match source {
        SystemPromptSource::StandardDiscovered { scope, .. } => {
            assert!(matches!(scope, StandardPromptScope::Package));
        }
        _ => panic!("Expected StandardDiscovered source"),
    }
    assert_eq!(text, "# Package");
}

#[test]
fn standard_discovery_package_area_wins() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    let area = repo.join("area");
    let package = area.join("package");
    std::fs::create_dir_all(&package).unwrap();

    // No file in package root
    std::fs::write(area.join("system-prompt.md"), "# Area").unwrap();
    std::fs::write(repo.join("system-prompt.md"), "# Repo").unwrap();

    let args = SystemPromptArgs::default();
    let context = LaunchContext {
        agent: None,
        cwd: package.clone(),
        repo_root: Some(repo.clone()),
        package_area_root: Some(area.clone()),
        package_root: Some(package.clone()),
    };

    let result = resolve_system_prompt_source(&args, &context).unwrap();
    assert!(result.is_some());
    let (source, text) = result.unwrap();

    match source {
        SystemPromptSource::StandardDiscovered { scope, .. } => {
            assert!(matches!(scope, StandardPromptScope::PackageArea));
        }
        _ => panic!("Expected StandardDiscovered source"),
    }
    assert_eq!(text, "# Area");
}

#[test]
fn standard_discovery_repo_fallback() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    let area = repo.join("area");
    let package = area.join("package");
    std::fs::create_dir_all(&package).unwrap();

    // Only file at repo root
    std::fs::write(repo.join("system-prompt.md"), "# Repo").unwrap();

    let args = SystemPromptArgs::default();
    let context = LaunchContext {
        agent: None,
        cwd: package.clone(),
        repo_root: Some(repo.clone()),
        package_area_root: Some(area.clone()),
        package_root: Some(package.clone()),
    };

    let result = resolve_system_prompt_source(&args, &context).unwrap();
    assert!(result.is_some());
    let (source, text) = result.unwrap();

    match source {
        SystemPromptSource::StandardDiscovered { scope, .. } => {
            assert!(matches!(scope, StandardPromptScope::Repo));
        }
        _ => panic!("Expected StandardDiscovered source"),
    }
    assert_eq!(text, "# Repo");
}

#[test]
fn standard_discovery_user_home_fallback() {
    // Note: This test has limitations because we cannot easily mock dirs::home_dir().
    // We test the case where no local files exist, which should either:
    // 1. Return None if ~/.claudine/system-prompt.md doesn't exist (normal test env)
    // 2. Return Some with User scope if it does exist (real user environment)
    //
    // We verify the fallback logic works by checking that when no local files exist,
    // the result is deterministic based on the user's home directory state.

    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();

    let args = SystemPromptArgs::default();
    let context = LaunchContext {
        agent: None,
        cwd: repo.clone(),
        repo_root: Some(repo.clone()),
        package_area_root: None,
        package_root: None,
    };

    let result = resolve_system_prompt_source(&args, &context).unwrap();

    // The result depends on whether the user actually has ~/.claudine/system-prompt.md
    if let Some((source, _)) = result {
        match source {
            SystemPromptSource::StandardDiscovered { scope, .. } => {
                assert!(matches!(scope, StandardPromptScope::User));
            }
            _ => panic!("Expected StandardDiscovered source with User scope"),
        }
    }
    // If None, that's also valid - it means no user-home file exists
}

/// The user-home leg of standard discovery, pinned through the explicit-home
/// seam so it asserts on every platform.
///
/// [`standard_discovery_user_home_fallback`] can only observe whatever the
/// real profile happens to hold, and the CLI subprocess fixture that covered
/// this end to end is Unix-only because `HOME` does not redirect Windows
/// profile discovery.
#[test]
fn standard_discovery_user_home_is_injectable() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::create_dir_all(home.join(".claudine")).unwrap();
    std::fs::write(home.join(".claudine/system-prompt.md"), "# User\n").unwrap();

    let args = SystemPromptArgs::default();
    let context = LaunchContext {
        agent: None,
        cwd: repo.clone(),
        repo_root: Some(repo),
        package_area_root: None,
        package_root: None,
    };

    let (source, text) = resolve_system_prompt_source_with_home(&args, &context, Some(&home))
        .unwrap()
        .expect("the injected home supplies the user-global prompt");
    match source {
        SystemPromptSource::StandardDiscovered { path, scope } => {
            assert!(matches!(scope, StandardPromptScope::User));
            assert_eq!(path, home.join(".claudine/system-prompt.md"));
        }
        _ => panic!("Expected StandardDiscovered source with User scope, got {source:?}"),
    }
    assert_eq!(text, "# User\n");
}

#[test]
fn standard_discovery_none() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();

    let args = SystemPromptArgs::default();
    let context = LaunchContext {
        agent: None,
        cwd: repo.clone(),
        repo_root: Some(repo.clone()),
        package_area_root: None,
        package_root: None,
    };

    let result = resolve_system_prompt_source(&args, &context).unwrap();

    // Should be None unless user has ~/.claudine/system-prompt.md
    // Since we can't control that in tests, we just verify no error occurs
    if let Some((source, _)) = result {
        // If we got a result, it must be from user home
        match source {
            SystemPromptSource::StandardDiscovered { scope, .. } => {
                assert!(matches!(scope, StandardPromptScope::User));
            }
            _ => panic!("Unexpected source type"),
        }
    }
}

#[test]
fn standard_discovery_dedupes_overlapping_roots() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();

    // Place one file where package_root == repo_root
    std::fs::write(repo.join("system-prompt.md"), "# Single File").unwrap();

    let args = SystemPromptArgs::default();
    let context = LaunchContext {
        agent: None,
        cwd: repo.clone(),
        repo_root: Some(repo.clone()),
        package_area_root: None,
        package_root: Some(repo.clone()), // Same as repo_root
    };

    let result = resolve_system_prompt_source(&args, &context).unwrap();
    assert!(result.is_some());
    let (source, text) = result.unwrap();

    match source {
        SystemPromptSource::StandardDiscovered { scope, .. } => {
            // Package scope has precedence, so even though it's the same path,
            // the first match (Package) wins
            assert!(matches!(scope, StandardPromptScope::Package));
        }
        _ => panic!("Expected StandardDiscovered source"),
    }
    assert_eq!(text, "# Single File");
}

#[test]
fn outside_repo_searches_cwd() {
    let tmp = TempDir::new().unwrap();
    let cwd = tmp.path().join("standalone");
    std::fs::create_dir_all(&cwd).unwrap();

    std::fs::write(cwd.join("system-prompt.md"), "# CWD Prompt").unwrap();

    let args = SystemPromptArgs::default();
    let context = LaunchContext {
        agent: None,
        cwd: cwd.clone(),
        repo_root: None, // Not in a repo
        package_area_root: None,
        package_root: None,
    };

    let result = resolve_system_prompt_source(&args, &context).unwrap();
    assert!(result.is_some());
    let (source, text) = result.unwrap();

    match source {
        SystemPromptSource::StandardDiscovered { scope, .. } => {
            assert!(matches!(scope, StandardPromptScope::CurrentDirectory));
        }
        _ => panic!("Expected StandardDiscovered source"),
    }
    assert_eq!(text, "# CWD Prompt");
}

#[test]
fn non_interactive_candidates_prefer_repo_then_home_then_builtin() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    let home = tmp.path().join("home");
    std::fs::create_dir_all(repo.join(".claudine")).unwrap();
    std::fs::create_dir_all(home.join(".claudine")).unwrap();
    std::fs::write(
        repo.join(".claudine").join("non-interactive.md"),
        "Repo appendix",
    )
    .unwrap();
    std::fs::write(
        home.join(".claudine").join("non-interactive.md"),
        "Home appendix",
    )
    .unwrap();

    let context = LaunchContext {
        agent: None,
        cwd: repo.clone(),
        repo_root: Some(repo.clone()),
        package_area_root: None,
        package_root: None,
    };

    let candidates = resolve_non_interactive_candidates_with_home(&context, Some(&home)).unwrap();

    assert_eq!(candidates.len(), 3);
    assert!(matches!(
        candidates[0].0,
        SystemPromptSource::NonInteractiveFile {
            scope: StandardPromptScope::Repo,
            ..
        }
    ));
    assert_eq!(candidates[0].1, "Repo appendix");
    assert!(matches!(
        candidates[1].0,
        SystemPromptSource::NonInteractiveFile {
            scope: StandardPromptScope::User,
            ..
        }
    ));
    assert_eq!(candidates[1].1, "Home appendix");
    assert!(matches!(
        candidates[2].0,
        SystemPromptSource::BuiltInNonInteractive
    ));
    assert_eq!(candidates[2].1, DEFAULT_NON_INTERACTIVE_SYSTEM_PROMPT);
}

#[test]
#[serial]
fn non_interactive_candidates_use_builtin_when_no_files_exist() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();

    unsafe {
        std::env::set_var("HOME", &home);
    }

    let context = LaunchContext {
        agent: None,
        cwd: tmp.path().to_path_buf(),
        repo_root: None,
        package_area_root: None,
        package_root: None,
    };

    let candidates = resolve_non_interactive_candidates(&context).unwrap();

    unsafe {
        std::env::remove_var("HOME");
    }

    assert_eq!(candidates.len(), 1);
    assert!(matches!(
        candidates[0].0,
        SystemPromptSource::BuiltInNonInteractive
    ));
    assert_eq!(candidates[0].1, DEFAULT_NON_INTERACTIVE_SYSTEM_PROMPT);
}
