use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::system_prompt::context::LaunchContext;
use crate::system_prompt::types::*;

const STANDARD_FILENAME: &str = "system-prompt.md";

/// Resolve the effective system prompt source.
///
/// If explicit args are provided, resolves only the named file and
/// skips standard discovery. Otherwise searches the launch context
/// hierarchy for `system-prompt.md`.
///
/// ## Returns
///
/// Returns `Ok(Some((source, text)))` if a prompt file is found, `Ok(None)` if
/// no file exists in the search path, or `Err` if file I/O fails.
///
/// ## Errors
///
/// Returns `ClaudineError::SystemPromptFileNotFound` if an explicit file path
/// does not exist. Returns `ClaudineError::Io` if reading any file fails.
///
/// ## Examples
///
/// ```no_run
/// use claudine::system_prompt::context::LaunchContext;
/// use claudine::system_prompt::types::SystemPromptArgs;
/// use claudine::system_prompt::resolve::resolve_system_prompt_source;
/// use std::path::PathBuf;
///
/// let args = SystemPromptArgs::default();
/// let context = LaunchContext {
///     cwd: PathBuf::from("/workspace"),
///     repo_root: Some(PathBuf::from("/workspace")),
///     package_area_root: None,
///     package_root: None,
/// };
///
/// let result = resolve_system_prompt_source(&args, &context)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn resolve_system_prompt_source(
    args: &SystemPromptArgs,
    context: &LaunchContext,
) -> Result<Option<(SystemPromptSource, String)>, crate::error::ClaudineError> {
    // 1. Explicit --append-system-prompt
    if let Some(ref file) = args.append_file {
        let path = resolve_file_ref(file, &context.cwd)?;
        let text = std::fs::read_to_string(&path)?;
        return Ok(Some((
            SystemPromptSource::ExplicitFile {
                path,
                mode: SystemPromptMode::Append,
            },
            text,
        )));
    }

    // 2. Explicit --replace-system-prompt
    if let Some(ref file) = args.replace_file {
        let path = resolve_file_ref(file, &context.cwd)?;
        let text = std::fs::read_to_string(&path)?;
        return Ok(Some((
            SystemPromptSource::ExplicitFile {
                path,
                mode: SystemPromptMode::Replace,
            },
            text,
        )));
    }

    // 3. Standard discovery
    discover_standard_file(context)
}

fn discover_standard_file(
    context: &LaunchContext,
) -> Result<Option<(SystemPromptSource, String)>, crate::error::ClaudineError> {
    // Search local scopes in precedence order
    let scope_dirs = build_scope_list(context);
    for (dir, scope) in &scope_dirs {
        let candidate = dir.join(STANDARD_FILENAME);
        if candidate.is_file() {
            let text = std::fs::read_to_string(&candidate)?;
            return Ok(Some((
                SystemPromptSource::StandardDiscovered {
                    path: candidate,
                    scope: *scope,
                },
                text,
            )));
        }
    }

    // User-home fallback
    let user_home = dirs::home_dir().map(|h| h.join(".claudine").join(STANDARD_FILENAME));
    if let Some(ref home_path) = user_home
        && home_path.is_file()
    {
        let text = std::fs::read_to_string(home_path)?;
        return Ok(Some((
            SystemPromptSource::StandardDiscovered {
                path: home_path.clone(),
                scope: StandardPromptScope::User,
            },
            text,
        )));
    }

    Ok(None)
}

/// Build the local scope search list from the launch context.
///
/// When inside a repo/monorepo, returns package -> package-area -> repo.
/// When outside a repo, returns CWD only.
fn build_scope_list(context: &LaunchContext) -> Vec<(PathBuf, StandardPromptScope)> {
    if context.repo_root.is_some() {
        let mut list = Vec::with_capacity(3);
        let mut seen = HashSet::new();
        if let Some(ref p) = context.package_root
            && seen.insert(p.clone())
        {
            list.push((p.clone(), StandardPromptScope::Package));
        }
        if let Some(ref p) = context.package_area_root
            && seen.insert(p.clone())
        {
            list.push((p.clone(), StandardPromptScope::PackageArea));
        }
        if let Some(ref p) = context.repo_root
            && seen.insert(p.clone())
        {
            list.push((p.clone(), StandardPromptScope::Repo));
        }
        list
    } else {
        vec![(context.cwd.clone(), StandardPromptScope::CurrentDirectory)]
    }
}

/// Resolve a file reference relative to the CWD.
fn resolve_file_ref(file_ref: &str, cwd: &Path) -> Result<PathBuf, crate::error::ClaudineError> {
    let path = Path::new(file_ref);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    if !resolved.is_file() {
        return Err(crate::error::ClaudineError::SystemPromptFileNotFound(
            resolved.display().to_string(),
        ));
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn explicit_skips_standard_discovery() {
        let tmp = TempDir::new().unwrap();
        let package_root = tmp.path().join("package");
        std::fs::create_dir_all(&package_root).unwrap();

        // Create standard discovery file
        let standard_file = package_root.join("system-prompt.md");
        std::fs::write(&standard_file, "# Standard Discovery\nThis should be skipped").unwrap();

        // Create explicit file
        let explicit_file = tmp.path().join("explicit.md");
        std::fs::write(&explicit_file, "# Explicit File\nUse this instead").unwrap();

        let args = SystemPromptArgs {
            append_file: Some(explicit_file.display().to_string()),
            ..Default::default()
        };
        let context = LaunchContext {
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

    #[test]
    fn standard_discovery_none() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();

        let args = SystemPromptArgs::default();
        let context = LaunchContext {
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
}
