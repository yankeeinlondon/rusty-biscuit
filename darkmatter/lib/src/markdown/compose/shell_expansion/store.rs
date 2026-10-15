//! Policy file discovery, loading, and persistence.

use super::template::TemplateShape;
use super::types::{
    ShellExpansionError, ShellExpansionOptions, ShellPolicyPaths, ShellRuleEntry, ShellRuleSet,
};
use crate::markdown::compose::ComposeSource;
use biscuit_file::FileResolutionContext;
use std::collections::HashSet;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Resolves the paths to whitelist and blacklist policy files.
///
/// Resolution order:
/// 1. If `policy_root` is set in options, use it
/// 2. The request context's repository root
/// 3. The request context's home directory
/// 4. The source file's directory, else the request directory
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
/// let context = biscuit_file::FileResolutionContext::from_snapshot("/path/to", None, Default::default());
/// let paths = resolve_policy_paths(&options, &source, &context).unwrap();
/// ```
pub fn resolve_policy_paths(
    shell_opts: &ShellExpansionOptions,
    source: &ComposeSource,
    context: &FileResolutionContext,
) -> Result<ShellPolicyPaths, ShellExpansionError> {
    // If policy_root is explicitly set, use it directly
    let policy_dir = if let Some(ref policy_root) = shell_opts.policy_root {
        policy_root.clone()
    } else {
        // A source without a directory of its own sits in the request's.
        let base_dir = match source {
            ComposeSource::File(path) => path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map_or_else(|| context.request_cwd().to_path_buf(), Path::to_path_buf),
            ComposeSource::Url(_) | ComposeSource::Unknown => context.request_cwd().to_path_buf(),
        };

        context
            .repository_root()
            .or_else(|| context.home_dir())
            .map_or(base_dir, Path::to_path_buf)
    };

    let whitelist = policy_dir.join(".darkmatter-shell-whitelist");
    let blacklist = policy_dir.join(".darkmatter-shell-blacklist");

    Ok(ShellPolicyPaths {
        whitelist,
        blacklist,
    })
}

/// A policy line the loader skipped, with where it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyLineWarning {
    pub path: PathBuf,
    /// 1-based line number.
    pub line: usize,
    /// The trimmed line as written.
    pub text: String,
    pub reason: String,
}

impl fmt::Display for PolicyLineWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Ignoring malformed policy line at {}:{}: {} ('{}')",
            self.path.display(),
            self.line,
            self.reason,
            self.text
        )
    }
}

/// Loads a ruleset from a policy file, emitting each skipped line as a
/// `tracing::warn!` diagnostic. See [`load_ruleset_with_warnings`] for the
/// format.
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
    let (ruleset, warnings) = load_ruleset_with_warnings(path)?;
    for warning in &warnings {
        tracing::warn!("{warning}");
    }
    Ok(ruleset)
}

/// Loads a ruleset from a policy file and returns the lines it skipped.
///
/// The file format is line-based UTF-8:
/// - `exact <command>` - exact command match
/// - `prefix <command>` - executable prefix match
/// - `template <shape>` - argument-list shape with typed `{{ type }}` holes
///   (see [`TemplateShape::parse_rule`])
/// - Lines starting with `#` are comments
/// - Blank lines are ignored
/// - Any other line, and a `template` line whose shape is rejected, is skipped
///   and reported with its file and line; the other rules still load
///
/// A repeated rule loads once; `template` rules compare as parsed shapes.
///
/// ## Errors
///
/// A missing file is an empty ruleset. An unreadable file or invalid UTF-8 is
/// [`ShellExpansionError::PolicyIo`], never an empty ruleset.
pub fn load_ruleset_with_warnings(
    path: &Path,
) -> Result<(ShellRuleSet, Vec<PolicyLineWarning>), ShellExpansionError> {
    if !path.exists() {
        return Ok((ShellRuleSet::default(), Vec::new()));
    }

    let content = std::fs::read_to_string(path).map_err(|e| ShellExpansionError::PolicyIo {
        path: path.to_path_buf(),
        source: e,
    })?;

    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    let mut warnings = Vec::new();

    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let entry = if let Some(command) = trimmed.strip_prefix("exact ") {
            Ok(ShellRuleEntry::Exact(command.to_string()))
        } else if let Some(command) = trimmed.strip_prefix("prefix ") {
            Ok(ShellRuleEntry::Prefix(command.to_string()))
        } else if let Some(shape) = trimmed.strip_prefix("template ") {
            TemplateShape::parse_rule(shape.trim())
                .map(ShellRuleEntry::Template)
                .map_err(|error| error.to_string())
        } else {
            Err("expected 'exact <command>', 'prefix <executable>', or 'template <shape>'".to_string())
        };

        match entry {
            Ok(entry) => {
                if seen.insert(entry.clone()) {
                    entries.push(entry);
                }
            }
            Err(reason) => warnings.push(PolicyLineWarning {
                path: path.to_path_buf(),
                line: index + 1,
                text: trimmed.to_string(),
                reason,
            }),
        }
    }

    Ok((ShellRuleSet { entries }, warnings))
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

/// Appends a `template` rule to the whitelist file unless an equal shape is
/// already there.
///
/// ## Errors
///
/// [`ShellExpansionError::PolicyIo`] when the file cannot be read or written,
/// or when the shape's rule text would not read back as the same shape.
pub fn append_whitelist_template(
    paths: &ShellPolicyPaths,
    shape: &TemplateShape,
) -> Result<(), ShellExpansionError> {
    append_template_rule(&paths.whitelist, shape)
}

/// Appends a `template` rule to the blacklist file unless an equal shape is
/// already there. Errors as [`append_whitelist_template`].
pub fn append_blacklist_template(
    paths: &ShellPolicyPaths,
    shape: &TemplateShape,
) -> Result<(), ShellExpansionError> {
    append_template_rule(&paths.blacklist, shape)
}

/// Writes the canonical rule text, deduplicated by parsed shape. Only the
/// shape is written: approval types, never a placeholder's value.
fn append_template_rule(path: &Path, shape: &TemplateShape) -> Result<(), ShellExpansionError> {
    let text = shape.canonical_text();
    if TemplateShape::parse_rule(&text).as_ref() != Ok(shape) {
        return Err(ShellExpansionError::PolicyIo {
            path: path.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("template rule '{text}' would not read back as the approved shape"),
            ),
        });
    }
    let (existing, _) = load_ruleset_with_warnings(path)?;
    if existing.entries.iter().any(|entry| matches!(entry, ShellRuleEntry::Template(rule) if rule == shape)) {
        return Ok(());
    }
    append_rule(path, "template", &text)
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
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn resolve_policy_paths_with_policy_root() {
        let temp_dir = TempDir::new().unwrap();
        let options = ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            ..Default::default()
        };
        let source = ComposeSource::Unknown;

        let paths = resolve_policy_paths(&options, &source, &FileResolutionContext::new(std::env::temp_dir())).unwrap();
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

        let paths = resolve_policy_paths(&options, &source, &FileResolutionContext::new(std::env::temp_dir())).unwrap();
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
