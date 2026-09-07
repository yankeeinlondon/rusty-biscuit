//! Policy file discovery, loading, and persistence.

use super::types::{
    ShellExpansionError, ShellExpansionOptions, ShellPolicyPaths, ShellRuleEntry, ShellRuleSet,
};
use crate::markdown::compose::ComposeSource;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Resolves the paths to whitelist and blacklist policy files.
///
/// Resolution order:
/// 1. If `policy_root` is set in options, use it
/// 2. If source is a file, use its parent directory
/// 3. Otherwise use current working directory
/// 4. Walk up to find `.git` directory if inside a git repo
/// 5. Fall back to HOME directory if no git repo found
///
/// ## Examples
///
/// ```no_run
/// use darkmatter::markdown::compose::shell_expansion::store::resolve_policy_paths;
/// use darkmatter::markdown::compose::shell_expansion::types::ShellExpansionOptions;
/// use darkmatter::markdown::compose::ComposeSource;
/// use std::path::PathBuf;
///
/// let options = ShellExpansionOptions::default();
/// let source = ComposeSource::File(PathBuf::from("/path/to/file.md"));
/// let paths = resolve_policy_paths(&options, &source).unwrap();
/// ```
pub fn resolve_policy_paths(
    shell_opts: &ShellExpansionOptions,
    source: &ComposeSource,
) -> Result<ShellPolicyPaths, ShellExpansionError> {
    // If policy_root is explicitly set, use it directly
    let policy_dir = if let Some(ref policy_root) = shell_opts.policy_root {
        policy_root.clone()
    } else {
        // Determine base directory from source
        let base_dir = match source {
            ComposeSource::File(path) => {
                if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                    parent.to_path_buf()
                } else {
                    std::env::current_dir().map_err(|e| ShellExpansionError::PolicyIo {
                        path: PathBuf::from("."),
                        source: e,
                    })?
                }
            }
            ComposeSource::Url(_) | ComposeSource::Unknown => {
                std::env::current_dir().map_err(|e| ShellExpansionError::PolicyIo {
                    path: PathBuf::from("."),
                    source: e,
                })?
            }
        };

        shell_opts
            .file_resolution_context
            .as_ref()
            .and_then(|context| context.repository_root().map(Path::to_path_buf))
            .or_else(|| {
                shell_opts
                    .file_resolution_context
                    .as_ref()
                    .and_then(|context| context.home_dir().map(Path::to_path_buf))
            })
            .unwrap_or(base_dir)
    };

    let whitelist = policy_dir.join(".darkmatter-shell-whitelist");
    let blacklist = policy_dir.join(".darkmatter-shell-blacklist");

    Ok(ShellPolicyPaths {
        whitelist,
        blacklist,
    })
}

/// Loads a ruleset from a policy file.
///
/// The file format is line-based:
/// - `exact <command>` - exact command match
/// - `prefix <command>` - executable prefix match
/// - Lines starting with `#` are comments
/// - Blank lines are ignored
/// - Malformed lines are ignored and emitted as `tracing::warn!` diagnostics
/// - Malformed non-comment lines are skipped and logged with `tracing::warn!`
///
/// If the file does not exist, returns an empty ruleset (not an error).
///
/// ## Examples
///
/// ```no_run
/// use darkmatter::markdown::compose::shell_expansion::store::load_ruleset;
/// use std::path::Path;
///
/// let ruleset = load_ruleset(Path::new("/path/to/whitelist")).unwrap();
/// ```
pub fn load_ruleset(path: &Path) -> Result<ShellRuleSet, ShellExpansionError> {
    // If file doesn't exist, return empty ruleset
    if !path.exists() {
        return Ok(ShellRuleSet::default());
    }

    let content = std::fs::read_to_string(path).map_err(|e| ShellExpansionError::PolicyIo {
        path: path.to_path_buf(),
        source: e,
    })?;

    let mut entries = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip blank lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Parse "exact <command>" or "prefix <command>"
        if let Some(command) = trimmed.strip_prefix("exact ") {
            entries.push(ShellRuleEntry::Exact(command.to_string()));
        } else if let Some(command) = trimmed.strip_prefix("prefix ") {
            entries.push(ShellRuleEntry::Prefix(command.to_string()));
        } else {
            // Warn about malformed lines that don't match expected format
            tracing::warn!(
                "Ignoring malformed policy line in {}: '{}'",
                path.display(),
                trimmed
            );
        }
    }

    // Deduplicate entries
    entries.sort_by(|a, b| match (a, b) {
        (ShellRuleEntry::Exact(a), ShellRuleEntry::Exact(b)) => a.cmp(b),
        (ShellRuleEntry::Prefix(a), ShellRuleEntry::Prefix(b)) => a.cmp(b),
        (ShellRuleEntry::Exact(_), ShellRuleEntry::Prefix(_)) => std::cmp::Ordering::Less,
        (ShellRuleEntry::Prefix(_), ShellRuleEntry::Exact(_)) => std::cmp::Ordering::Greater,
    });
    entries.dedup();

    Ok(ShellRuleSet { entries })
}

/// Appends an exact-match rule to the whitelist file.
///
/// ## Examples
///
/// ```no_run
/// use darkmatter::markdown::compose::shell_expansion::store::append_whitelist_exact;
/// use darkmatter::markdown::compose::shell_expansion::types::ShellPolicyPaths;
/// use std::path::PathBuf;
///
/// let paths = ShellPolicyPaths {
///     whitelist: PathBuf::from("/path/to/whitelist"),
///     blacklist: PathBuf::from("/path/to/blacklist"),
/// };
/// append_whitelist_exact(&paths, "echo hello").unwrap();
/// ```
pub fn append_whitelist_exact(
    paths: &ShellPolicyPaths,
    normalized: &str,
) -> Result<(), ShellExpansionError> {
    append_rule(&paths.whitelist, "exact", normalized)
}

/// Appends a prefix-match rule to the whitelist file.
///
/// ## Examples
///
/// ```no_run
/// use darkmatter::markdown::compose::shell_expansion::store::append_whitelist_prefix;
/// use darkmatter::markdown::compose::shell_expansion::types::ShellPolicyPaths;
/// use std::path::PathBuf;
///
/// let paths = ShellPolicyPaths {
///     whitelist: PathBuf::from("/path/to/whitelist"),
///     blacklist: PathBuf::from("/path/to/blacklist"),
/// };
/// append_whitelist_prefix(&paths, "ls").unwrap();
/// ```
pub fn append_whitelist_prefix(
    paths: &ShellPolicyPaths,
    executable: &str,
) -> Result<(), ShellExpansionError> {
    append_rule(&paths.whitelist, "prefix", executable)
}

/// Appends an exact-match rule to the blacklist file.
///
/// ## Examples
///
/// ```no_run
/// use darkmatter::markdown::compose::shell_expansion::store::append_blacklist_exact;
/// use darkmatter::markdown::compose::shell_expansion::types::ShellPolicyPaths;
/// use std::path::PathBuf;
///
/// let paths = ShellPolicyPaths {
///     whitelist: PathBuf::from("/path/to/whitelist"),
///     blacklist: PathBuf::from("/path/to/blacklist"),
/// };
/// append_blacklist_exact(&paths, "dangerous-command").unwrap();
/// ```
pub fn append_blacklist_exact(
    paths: &ShellPolicyPaths,
    normalized: &str,
) -> Result<(), ShellExpansionError> {
    append_rule(&paths.blacklist, "exact", normalized)
}

/// Appends a prefix-match rule to the blacklist file.
///
/// Mirrors [`append_whitelist_prefix`] for deny rules so the append API stays
/// symmetric across whitelist and blacklist policy files.
///
/// ## Examples
///
/// ```no_run
/// use darkmatter::markdown::compose::shell_expansion::store::append_blacklist_prefix;
/// use darkmatter::markdown::compose::shell_expansion::types::ShellPolicyPaths;
/// use std::path::PathBuf;
///
/// let paths = ShellPolicyPaths {
///     whitelist: PathBuf::from("/path/to/whitelist"),
///     blacklist: PathBuf::from("/path/to/blacklist"),
/// };
/// append_blacklist_prefix(&paths, "dangerous-tool").unwrap();
/// ```
pub fn append_blacklist_prefix(
    paths: &ShellPolicyPaths,
    executable: &str,
) -> Result<(), ShellExpansionError> {
    append_rule(&paths.blacklist, "prefix", executable)
}

/// Internal helper to append a rule to a policy file.
fn append_rule(path: &Path, rule_type: &str, value: &str) -> Result<(), ShellExpansionError> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| ShellExpansionError::PolicyIo {
            path: path.to_path_buf(),
            source: e,
        })?;

    writeln!(file, "{} {}", rule_type, value).map_err(|e| ShellExpansionError::PolicyIo {
        path: path.to_path_buf(),
        source: e,
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::markdown::compose::find_git_root_from;
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn resolve_policy_paths_with_policy_root() {
        let temp_dir = TempDir::new().unwrap();
        let options = ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            ..Default::default()
        };
        let source = ComposeSource::Unknown;

        let paths = resolve_policy_paths(&options, &source).unwrap();
        assert_eq!(
            paths.whitelist,
            temp_dir.path().join(".darkmatter-shell-whitelist")
        );
        assert_eq!(
            paths.blacklist,
            temp_dir.path().join(".darkmatter-shell-blacklist")
        );
    }

    #[test]
    fn resolve_policy_paths_from_file_source() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        std::fs::write(&file_path, "test").unwrap();

        let options = ShellExpansionOptions::default();
        let source = ComposeSource::File(file_path);

        let paths = resolve_policy_paths(&options, &source).unwrap();
        // Should use the parent directory (temp_dir) or walk up to find git root
        assert!(
            paths
                .whitelist
                .to_string_lossy()
                .contains(".darkmatter-shell-whitelist")
        );
    }

    #[test]
    fn load_ruleset_nonexistent_file() {
        let path = PathBuf::from("/nonexistent/path/to/whitelist");
        let ruleset = load_ruleset(&path).unwrap();
        assert_eq!(ruleset.entries.len(), 0);
    }

    #[test]
    fn load_ruleset_with_exact_and_prefix() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("whitelist");
        std::fs::write(
            &file_path,
            "exact echo hello\nprefix ls\n# comment\n\nexact pwd\n",
        )
        .unwrap();

        let ruleset = load_ruleset(&file_path).unwrap();
        assert_eq!(ruleset.entries.len(), 3);
        assert!(
            ruleset
                .entries
                .contains(&ShellRuleEntry::Exact("echo hello".to_string()))
        );
        assert!(
            ruleset
                .entries
                .contains(&ShellRuleEntry::Prefix("ls".to_string()))
        );
        assert!(
            ruleset
                .entries
                .contains(&ShellRuleEntry::Exact("pwd".to_string()))
        );
    }

    #[test]
    fn load_ruleset_deduplicates() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("whitelist");
        std::fs::write(
            &file_path,
            "exact echo hello\nexact echo hello\nprefix ls\nprefix ls\n",
        )
        .unwrap();

        let ruleset = load_ruleset(&file_path).unwrap();
        assert_eq!(ruleset.entries.len(), 2);
    }

    #[test]
    fn load_ruleset_warns_and_ignores_invalid_lines() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("whitelist");
        std::fs::write(&file_path, "exact echo hello\ninvalid line\nprefix ls\n").unwrap();

        let ruleset = load_ruleset(&file_path).unwrap();
        assert_eq!(ruleset.entries.len(), 2);
        assert!(
            ruleset
                .entries
                .contains(&ShellRuleEntry::Exact("echo hello".to_string()))
        );
        assert!(
            ruleset
                .entries
                .contains(&ShellRuleEntry::Prefix("ls".to_string()))
        );
    }

    #[test]
    fn append_whitelist_exact_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let paths = ShellPolicyPaths {
            whitelist: temp_dir.path().join("whitelist"),
            blacklist: temp_dir.path().join("blacklist"),
        };

        append_whitelist_exact(&paths, "echo hello").unwrap();

        let content = std::fs::read_to_string(&paths.whitelist).unwrap();
        assert_eq!(content.trim(), "exact echo hello");
    }

    #[test]
    fn append_whitelist_prefix_appends() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("whitelist");
        std::fs::write(&file_path, "exact echo hello\n").unwrap();

        let paths = ShellPolicyPaths {
            whitelist: file_path.clone(),
            blacklist: temp_dir.path().join("blacklist"),
        };

        append_whitelist_prefix(&paths, "ls").unwrap();

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "exact echo hello\nprefix ls\n");
    }

    #[test]
    fn append_blacklist_exact_works() {
        let temp_dir = TempDir::new().unwrap();
        let paths = ShellPolicyPaths {
            whitelist: temp_dir.path().join("whitelist"),
            blacklist: temp_dir.path().join("blacklist"),
        };

        append_blacklist_exact(&paths, "dangerous-command").unwrap();

        let content = std::fs::read_to_string(&paths.blacklist).unwrap();
        assert_eq!(content.trim(), "exact dangerous-command");
    }

    #[test]
    fn append_blacklist_prefix_works() {
        let temp_dir = TempDir::new().unwrap();
        let paths = ShellPolicyPaths {
            whitelist: temp_dir.path().join("whitelist"),
            blacklist: temp_dir.path().join("blacklist"),
        };

        append_blacklist_prefix(&paths, "dangerous-command").unwrap();

        let content = std::fs::read_to_string(&paths.blacklist).unwrap();
        assert_eq!(content.trim(), "prefix dangerous-command");
    }

    #[test]
    fn append_blacklist_prefix_appends() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("blacklist");
        std::fs::write(&file_path, "exact dangerous-command\n").unwrap();

        let paths = ShellPolicyPaths {
            whitelist: temp_dir.path().join("whitelist"),
            blacklist: file_path.clone(),
        };

        append_blacklist_prefix(&paths, "dangerous-tool").unwrap();

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "exact dangerous-command\nprefix dangerous-tool\n");
    }

    #[test]
    fn find_git_root_finds_git_directory() {
        let temp_dir = TempDir::new().unwrap();
        let git_dir = temp_dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();

        let subdir = temp_dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        let root = find_git_root_from(&subdir);
        assert_eq!(root, Some(temp_dir.path().to_path_buf()));
    }

    #[test]
    fn find_git_root_returns_none_if_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let root = find_git_root_from(temp_dir.path());
        assert!(root.is_none());
    }
}
