use std::path::PathBuf;

/// Check if `path` is exactly `prefix` or starts with `prefix/`.
fn is_prefix_match(path: &str, prefix: &str) -> bool {
    path == prefix || (path.starts_with(prefix) && path.as_bytes().get(prefix.len()) == Some(&b'/'))
}

/// Prefixes for absolute sensitive paths.
const SENSITIVE_PREFIXES: &[&str] = &[
    "/etc", "/var", "/usr", "/boot", "/dev", "/proc", "/sys", "/System",
];

/// Home-relative sensitive prefixes (checked after ~ expansion).
const SENSITIVE_HOME_PREFIXES: &[&str] = &[".ssh", ".gnupg"];

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
    pub fn is_sensitive(&self, path: &str) -> bool {
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

fn is_path_allowed(target: &str, allow_paths: &[String]) -> bool {
    let target = target.trim_start_matches("./");
    for allowed in allow_paths {
        if allowed.starts_with('/') {
            if target.starts_with(allowed.as_str()) {
                return true;
            }
        } else {
            let parts: Vec<&str> = target.split('/').collect();
            if parts.contains(&allowed.as_str()) {
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
        assert!(checker.is_sensitive("/System/Library/something"));
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
    fn nested_allowed_path_matches() {
        let allow = vec!["node_modules".to_string()];
        assert!(all_targets_allowed(&["./node_modules".to_string()], &allow));
        assert!(all_targets_allowed(
            &["packages/foo/node_modules".to_string()],
            &allow
        ));
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
            checker.is_sensitive("/System"),
            "/System should be sensitive"
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
}
