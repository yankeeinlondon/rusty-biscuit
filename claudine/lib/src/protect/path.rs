use std::path::PathBuf;

/// Warn once that path-based protection does not function on Windows.
///
/// This is instrumentation for a known gap, not a fix. Every matcher in this
/// module and in `permissions::matchers` hardcodes `/` as the path-segment
/// boundary, while a Windows path uses `\`. Because each matcher returns `bool`
/// with no error channel, a separator mismatch is indistinguishable from a
/// legitimate "this path is not under that prefix" — so the negative branch is
/// taken, and for these particular checks the negative branch is the *permissive*
/// one:
///
/// - a sensitive-path check returning `false` means "not sensitive", so
///   `~/.ssh`, `~/.aws`, `~/.gnupg`, and `~/.claude` are unprotected;
/// - a *deny* rule that fails to match is a denial that never applies.
///
/// An allow rule failing to match is merely inconvenient; the two above are not.
/// Until the real fix lands, a Windows user is told rather than left to assume
/// the control is working. A red CI cell would never reach them.
#[cfg(windows)]
pub(crate) fn warn_windows_path_matching_is_broken() {
    use std::sync::OnceLock;
    static WARNED: OnceLock<()> = OnceLock::new();
    WARNED.get_or_init(|| {
        tracing::warn!(
            "path matching does not function on Windows: sensitive-path \
             classification and directory-scoped permission rules compare against \
             a hardcoded '/' separator. `~/.ssh`, `~/.aws`, `~/.gnupg` and \
             `~/.claude` are NOT classified sensitive, and directory-scoped deny \
             rules do NOT apply. See claudine/fixes/2026-07-29-windows-paths."
        );
    });
}

#[cfg(not(windows))]
pub(crate) fn warn_windows_path_matching_is_broken() {}

/// Check if `path` is exactly `prefix` or starts with `prefix/`.
///
/// ## Notes
///
/// The `/` boundary is hardcoded, so this returns `false` for every Windows path
/// regardless of whether it is genuinely under `prefix`. See
/// [`warn_windows_path_matching_is_broken`].
fn is_prefix_match(path: &str, prefix: &str) -> bool {
    path == prefix || (path.starts_with(prefix) && path.as_bytes().get(prefix.len()) == Some(&b'/'))
}

/// Prefixes for absolute sensitive paths.
///
/// On macOS, several paths are symlinks to /private/* equivalents.
/// We include both forms to ensure canonicalized paths are caught.
/// /System/Library is the actual macOS system directory; /System/Volumes/Data
/// is the firmlink root for user data and should not be blocked.
const SENSITIVE_PREFIXES: &[&str] = &[
    "/bin",
    "/boot",
    "/dev",
    "/etc",
    "/opt",
    "/private/etc",
    "/private/var",
    "/proc",
    "/root",
    "/sbin",
    "/sys",
    "/System/Library",
    "/System/Applications",
    "/Library/LaunchDaemons",
    "/usr",
    "/var",
];

/// Home-relative sensitive prefixes (checked after ~ expansion).
const SENSITIVE_HOME_PREFIXES: &[&str] = &[
    ".aws",
    ".claude",
    ".codex",
    ".config/gh",
    ".docker/config.json",
    ".gemini",
    ".git-credentials",
    ".gnupg",
    ".goose",
    ".kube",
    ".netrc",
    ".npmrc",
    ".opencode",
    ".qwen",
    ".roo",
    ".ssh",
];

/// Checks whether a file path targets a sensitive system location.
#[derive(Debug, Clone)]
pub struct SensitivePathChecker {
    home_dir: Option<PathBuf>,
}

impl SensitivePathChecker {
    pub fn new() -> Self {
        Self {
            home_dir: dirs::home_dir(),
        }
    }

    /// Returns true if the path is under a sensitive prefix.
    ///
    /// ## Notes
    ///
    /// On Windows this currently returns `false` for every home-relative
    /// sensitive path, because the prefix is spliced with a hardcoded `/` against
    /// a `\`-separated path. The caller is warned once rather than left assuming
    /// the check succeeded — see [`warn_windows_path_matching_is_broken`].
    pub fn is_sensitive(&self, path: &str) -> bool {
        warn_windows_path_matching_is_broken();
        let normalized = normalize_path(path);
        let path_str = normalized.to_string_lossy();

        for prefix in SENSITIVE_PREFIXES {
            if is_prefix_match(&path_str, prefix) {
                return true;
            }
        }

        if let Some(home) = &self.home_dir {
            let home_str = home.to_string_lossy();
            for prefix in SENSITIVE_HOME_PREFIXES {
                let full_prefix = format!("{home_str}/{prefix}");
                if is_prefix_match(&path_str, &full_prefix) {
                    return true;
                }
            }
        }

        false
    }
}

impl Default for SensitivePathChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalize a path: expand ~, resolve . and .. lexically.
pub fn normalize_path(path: &str) -> PathBuf {
    let expanded = if let Some(rest) = path.strip_prefix("~/") {
        match dirs::home_dir() {
            Some(home) => home.join(rest),
            None => PathBuf::from(path),
        }
    } else if path == "~" {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from(path))
    } else {
        PathBuf::from(path)
    };

    let mut normalized = PathBuf::new();
    for component in expanded.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }
    normalized
}

/// Canonicalize by resolving the deepest existing ancestor.
///
/// Walks from the full path toward the root, finds the deepest component
/// that exists on the filesystem, canonicalizes that prefix, then appends
/// the remaining (non-existent) suffix. Falls back to the input path
/// unchanged if no ancestor can be resolved.
pub fn canonicalize_existing_ancestor(path: &std::path::Path) -> PathBuf {
    // Try the full path first
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }

    // Walk ancestors until we find one that exists and can be canonicalized
    let mut suffix_components = Vec::new();
    let mut current = path.to_path_buf();

    while let Some(parent) = current.parent() {
        if let Some(file_name) = current.file_name() {
            suffix_components.push(file_name.to_os_string());
        }
        if parent.exists()
            && let Ok(canonical_parent) = parent.canonicalize()
        {
            let mut result = canonical_parent;
            for component in suffix_components.into_iter().rev() {
                result.push(component);
            }
            return result;
        }
        current = parent.to_path_buf();
    }

    path.to_path_buf()
}

/// Extract target path operands from a shell command string.
///
/// Skips sudo, the command name, and flag arguments (starting with -).
pub fn extract_target_paths(command: &str) -> Vec<String> {
    let words: Vec<&str> = command.split_whitespace().collect();
    let mut targets = Vec::new();
    let mut i = 0;

    if words.first() == Some(&"sudo") {
        i = 1;
    }
    i += 1; // skip command name

    while i < words.len() {
        let word = words[i];
        if word.starts_with('-') {
            i += 1;
            continue;
        }
        targets.push(word.to_string());
        i += 1;
    }
    targets
}

/// Check whether all target paths are in the allow list.
///
/// Returns false if targets is empty.
pub fn all_targets_allowed(targets: &[String], allow_paths: &[String]) -> bool {
    if targets.is_empty() {
        return false;
    }
    targets
        .iter()
        .all(|target| is_path_allowed(target, allow_paths))
}

/// Check whether `target` is allowed by any entry in `allow_paths`.
///
/// Relative allow entries are matched as an anchored component-sequence prefix
/// of the target, so `node_modules` allows `node_modules/foo` but not
/// `/etc/build/passwd`. Absolute allow entries use boundary-aware prefix
/// semantics so `/var/tmp` does not allow `/var/tmpevil`.
pub fn is_path_allowed(target: &str, allow_paths: &[String]) -> bool {
    let target = target.trim_start_matches("./");
    let target_components: Vec<&str> = target.split('/').collect();

    for allowed in allow_paths {
        let allowed = allowed.trim_start_matches("./");
        if allowed.starts_with('/') {
            let allowed_normalized = normalize_path(allowed);
            let allowed_canonical = canonicalize_existing_ancestor(&allowed_normalized);
            let allowed_str = allowed_canonical.to_string_lossy();
            let target_normalized = if target.starts_with('/') {
                canonicalize_existing_ancestor(&normalize_path(target))
                    .to_string_lossy()
                    .to_string()
            } else {
                target.to_string()
            };
            if target_normalized == allowed_str.as_ref()
                || target_normalized.starts_with(&format!("{}/", allowed_str.as_ref()))
            {
                return true;
            }
        } else {
            let allowed_components: Vec<&str> = allowed.split('/').collect();
            if allowed_components.is_empty() {
                continue;
            }
            if target_components.starts_with(&allowed_components) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_sensitive_paths_are_detected() {
        let checker = SensitivePathChecker::new();
        assert!(checker.is_sensitive("/etc/passwd"));
        assert!(checker.is_sensitive("/var/log/syslog"));
        assert!(checker.is_sensitive("/usr/bin/something"));
        assert!(checker.is_sensitive("/boot/vmlinuz"));
        assert!(checker.is_sensitive("/dev/sda"));
        assert!(checker.is_sensitive("/proc/1/status"));
        assert!(checker.is_sensitive("/sys/class/net"));
    }

    #[test]
    fn macos_system_path_is_sensitive() {
        let checker = SensitivePathChecker::new();
        assert!(checker.is_sensitive("/System/Library/Extensions/something"));
        assert!(checker.is_sensitive("/System/Applications/Safari.app"));
    }

    #[test]
    fn home_relative_sensitive_paths_are_detected() {
        let checker = SensitivePathChecker::new();
        let home = dirs::home_dir().unwrap();
        let ssh_config = home.join(".ssh/config");
        assert!(checker.is_sensitive(ssh_config.to_str().unwrap()));
        let gnupg = home.join(".gnupg/pubring.kbx");
        assert!(checker.is_sensitive(gnupg.to_str().unwrap()));
    }

    #[test]
    fn repo_internal_path_is_not_sensitive() {
        let checker = SensitivePathChecker::new();
        assert!(!checker.is_sensitive("src/main.rs"));
        assert!(!checker.is_sensitive("/home/user/project/src/lib.rs"));
        assert!(!checker.is_sensitive("./node_modules/something"));
    }

    #[test]
    fn tilde_path_is_expanded() {
        let normalized = normalize_path("~/.ssh/config");
        let home = dirs::home_dir().unwrap();
        assert_eq!(normalized, home.join(".ssh/config"));
    }

    #[test]
    fn extract_targets_from_rm_command() {
        let targets = extract_target_paths("rm -rf node_modules target");
        assert_eq!(targets, vec!["node_modules", "target"]);
    }

    #[test]
    fn extract_targets_skips_sudo_and_flags() {
        let targets = extract_target_paths("sudo rm -rf /var/log");
        assert_eq!(targets, vec!["/var/log"]);
    }

    #[test]
    fn all_targets_allowed_suppresses_match() {
        let allow = vec!["node_modules".to_string(), "target".to_string()];
        assert!(all_targets_allowed(
            &["node_modules".to_string(), "target".to_string()],
            &allow
        ));
    }

    #[test]
    fn partial_allowed_does_not_suppress() {
        let allow = vec!["node_modules".to_string()];
        assert!(!all_targets_allowed(
            &["node_modules".to_string(), "/etc/passwd".to_string()],
            &allow
        ));
    }

    #[test]
    fn nested_allowed_path_matches_top_level_only() {
        let allow = vec!["node_modules".to_string()];
        assert!(all_targets_allowed(&["./node_modules".to_string()], &allow));
        assert!(
            !all_targets_allowed(
                &["packages/foo/node_modules".to_string()],
                &allow
            ),
            "relative allow entries must match as an anchored prefix"
        );
    }

    #[test]
    fn relative_allow_does_not_match_absolute_target() {
        let allow = vec!["build".to_string()];
        assert!(
            !is_path_allowed("/etc/build/passwd", &allow),
            "relative 'build' must not allow /etc/build/passwd"
        );
    }

    #[test]
    fn absolute_allow_respects_boundary() {
        let allow = vec!["/var/tmp".to_string()];
        assert!(is_path_allowed("/var/tmp/file.txt", &allow));
        assert!(
            !is_path_allowed("/var/tmpevil", &allow),
            "absolute allow must use boundary-aware prefix matching"
        );
    }

    #[test]
    fn empty_targets_does_not_suppress() {
        let allow = vec!["node_modules".to_string()];
        assert!(!all_targets_allowed(&[], &allow));
    }

    #[test]
    fn exact_sensitive_directory_roots_are_detected() {
        let checker = SensitivePathChecker::new();
        assert!(checker.is_sensitive("/etc"), "/etc should be sensitive");
        assert!(checker.is_sensitive("/var"), "/var should be sensitive");
        assert!(checker.is_sensitive("/usr"), "/usr should be sensitive");
        assert!(checker.is_sensitive("/boot"), "/boot should be sensitive");
        assert!(checker.is_sensitive("/dev"), "/dev should be sensitive");
        assert!(checker.is_sensitive("/proc"), "/proc should be sensitive");
        assert!(checker.is_sensitive("/sys"), "/sys should be sensitive");
        assert!(
            checker.is_sensitive("/System/Library"),
            "/System/Library should be sensitive"
        );
        assert!(
            checker.is_sensitive("/System/Applications"),
            "/System/Applications should be sensitive"
        );
    }

    #[test]
    fn exact_home_sensitive_directory_roots_are_detected() {
        let checker = SensitivePathChecker::new();
        let home = dirs::home_dir().unwrap();
        assert!(
            checker.is_sensitive(&format!("{}/.ssh", home.display())),
            "~/.ssh should be sensitive"
        );
        assert!(
            checker.is_sensitive(&format!("{}/.gnupg", home.display())),
            "~/.gnupg should be sensitive"
        );
    }

    #[test]
    fn tilde_exact_sensitive_directory_roots_are_detected() {
        let checker = SensitivePathChecker::new();
        assert!(checker.is_sensitive("~/.ssh"), "~/.ssh should be sensitive");
        assert!(
            checker.is_sensitive("~/.gnupg"),
            "~/.gnupg should be sensitive"
        );
    }

    // Unix-only for the *fixture*, not the behavior: creating a symlink on
    // Windows needs SeCreateSymbolicLinkPrivilege (developer mode or elevation),
    // which a test host cannot assume. `canonicalize_existing_ancestor` itself is
    // cross-platform and covered on every host by the sibling tests.
    #[cfg(unix)]
    #[test]
    fn canonicalize_existing_ancestor_resolves_symlink_parent() {
        let tmp = tempfile::tempdir().unwrap();
        let real_dir = tmp.path().join("real");
        std::fs::create_dir_all(&real_dir).unwrap();
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(&real_dir, &link).unwrap();

        let input = link.join("nonexistent.txt");
        let result = canonicalize_existing_ancestor(&input);

        // Canonicalize the expected path too, since tmp.path() might itself
        // be under a symlink (e.g., /var -> /private/var on macOS)
        let expected = real_dir.canonicalize().unwrap().join("nonexistent.txt");
        assert_eq!(result, expected);
    }

    #[test]
    fn canonicalize_existing_ancestor_returns_input_when_nothing_exists() {
        let result =
            canonicalize_existing_ancestor(std::path::Path::new("/nonexistent/deeply/nested/path"));
        assert!(result.to_string_lossy().contains("nonexistent"));
    }

    #[test]
    fn home_credential_paths_are_sensitive() {
        let checker = SensitivePathChecker::new();
        let home = dirs::home_dir().unwrap();
        assert!(checker.is_sensitive(&format!("{}/.aws/credentials", home.display())));
        assert!(checker.is_sensitive(&format!("{}/.kube/config", home.display())));
        assert!(checker.is_sensitive(&format!("{}/.docker/config.json", home.display())));
        assert!(checker.is_sensitive(&format!("{}/.netrc", home.display())));
        assert!(checker.is_sensitive(&format!("{}/.npmrc", home.display())));
        assert!(checker.is_sensitive(&format!("{}/.git-credentials", home.display())));
        assert!(checker.is_sensitive(&format!("{}/.config/gh/hosts.yml", home.display())));
        assert!(checker.is_sensitive(&format!("{}/.claude/config.json", home.display())));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn unix_absolute_sensitive_paths_are_detected() {
        let checker = SensitivePathChecker::new();
        assert!(checker.is_sensitive("/bin/bash"));
        assert!(checker.is_sensitive("/sbin/init"));
        assert!(checker.is_sensitive("/root/.bashrc"));
        assert!(checker.is_sensitive("/opt/someapp/config"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_launch_daemon_path_is_sensitive() {
        let checker = SensitivePathChecker::new();
        assert!(checker.is_sensitive("/Library/LaunchDaemons/com.example.plist"));
    }
}
