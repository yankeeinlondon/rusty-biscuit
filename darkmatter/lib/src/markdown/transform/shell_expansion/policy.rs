//! Policy enforcement for shell commands.
//!
//! Implements built-in blacklists, whitelist checking, and command normalization.

use super::types::{BlacklistRule, ShellRuleSet};

/// Built-in blacklist rules that are always enforced.
const BUILTIN_BLACKLIST: &[BlacklistRule] = &[
    // Destructive file operations
    BlacklistRule::Executable("rm"),
    BlacklistRule::Executable("rmdir"),
    BlacklistRule::Executable("shred"),
    // Disk operations
    BlacklistRule::ExecutablePrefix("mkfs"),
    BlacklistRule::Executable("fdisk"),
    BlacklistRule::Executable("parted"),
    BlacklistRule::Executable("dd"),
    // Permission changes
    BlacklistRule::Executable("chmod"),
    BlacklistRule::Executable("chown"),
    BlacklistRule::Executable("chgrp"),
    // System control
    BlacklistRule::Executable("shutdown"),
    BlacklistRule::Executable("reboot"),
    BlacklistRule::Executable("halt"),
    BlacklistRule::Executable("init"),
    BlacklistRule::Executable("systemctl"),
    // Package managers
    BlacklistRule::Executable("apt"),
    BlacklistRule::Executable("yum"),
    BlacklistRule::Executable("dnf"),
    BlacklistRule::Executable("brew"),
    BlacklistRule::Executable("pacman"),
    BlacklistRule::Executable("pip"),
    BlacklistRule::Executable("npm"),
    BlacklistRule::Executable("cargo"),
    // Git dangerous operations
    BlacklistRule::SubcommandPrefix {
        executable: "git",
        prefix: &["reset"],
    },
    BlacklistRule::SubcommandPrefix {
        executable: "git",
        prefix: &["clean"],
    },
    BlacklistRule::SubcommandPrefix {
        executable: "git",
        prefix: &["push", "--force"],
    },
    BlacklistRule::SubcommandPrefix {
        executable: "git",
        prefix: &["push", "-f"],
    },
    // Docker dangerous operations
    BlacklistRule::SubcommandPrefix {
        executable: "docker",
        prefix: &["system", "prune"],
    },
    BlacklistRule::SubcommandPrefix {
        executable: "docker",
        prefix: &["rm"],
    },
    BlacklistRule::SubcommandPrefix {
        executable: "docker",
        prefix: &["rmi"],
    },
    // SQL dangerous operations
    BlacklistRule::ArgPrefix {
        executable: "psql",
        arg_prefix: "DROP",
    },
    BlacklistRule::ArgPrefix {
        executable: "mysql",
        arg_prefix: "DROP",
    },
    // Network tools
    BlacklistRule::Executable("nc"),
    BlacklistRule::Executable("ncat"),
    BlacklistRule::ArgExact {
        executable: "curl",
        arg: "-X DELETE",
    },
    // Process control
    BlacklistRule::Executable("kill"),
    BlacklistRule::Executable("killall"),
    BlacklistRule::Executable("pkill"),
    // Interactive editors (can't work in non-interactive context)
    BlacklistRule::Executable("vi"),
    BlacklistRule::Executable("vim"),
    BlacklistRule::Executable("nano"),
    BlacklistRule::Executable("emacs"),
    // Shells (prevent shell injection)
    BlacklistRule::Executable("sh"),
    BlacklistRule::Executable("bash"),
    BlacklistRule::Executable("zsh"),
    BlacklistRule::Executable("fish"),
    BlacklistRule::Executable("cmd"),
    BlacklistRule::Executable("powershell"),
    BlacklistRule::Executable("pwsh"),
    // Shell interpreter wrappers
    BlacklistRule::SubcommandPrefix {
        executable: "sh",
        prefix: &["-c"],
    },
    BlacklistRule::SubcommandPrefix {
        executable: "bash",
        prefix: &["-c"],
    },
    BlacklistRule::SubcommandPrefix {
        executable: "bash",
        prefix: &["-lc"],
    },
    BlacklistRule::SubcommandPrefix {
        executable: "zsh",
        prefix: &["-c"],
    },
    BlacklistRule::SubcommandPrefix {
        executable: "fish",
        prefix: &["-c"],
    },
    BlacklistRule::SubcommandPrefix {
        executable: "cmd",
        prefix: &["/C"],
    },
    BlacklistRule::SubcommandPrefix {
        executable: "powershell",
        prefix: &["-Command"],
    },
    BlacklistRule::SubcommandPrefix {
        executable: "pwsh",
        prefix: &["-Command"],
    },
    // Redirection tokens
    BlacklistRule::RawToken(">"),
    BlacklistRule::RawToken(">>"),
    // find with delete
    BlacklistRule::ArgExact {
        executable: "find",
        arg: "-delete",
    },
];

/// Checks if a command matches the built-in blacklist.
///
/// ## Returns
///
/// `Some(reason)` if the command is blacklisted, `None` otherwise.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::transform::shell_expansion::policy::check_builtin_blacklist;
///
/// let reason = check_builtin_blacklist("rm", &["-rf".to_string(), "/".to_string()]);
/// assert!(reason.is_some());
///
/// let reason = check_builtin_blacklist("echo", &["hello".to_string()]);
/// assert!(reason.is_none());
/// ```
pub fn check_builtin_blacklist(executable: &str, args: &[String]) -> Option<String> {
    for rule in BUILTIN_BLACKLIST {
        match rule {
            BlacklistRule::Executable(cmd) => {
                if executable == *cmd {
                    return Some(format!("'{}' is a dangerous command", cmd));
                }
            }
            BlacklistRule::ExecutablePrefix(prefix) => {
                if executable.starts_with(prefix) {
                    return Some(format!("Commands starting with '{}' are dangerous", prefix));
                }
            }
            BlacklistRule::SubcommandPrefix {
                executable: cmd,
                prefix,
            } => {
                if executable == *cmd && args_start_with(args, prefix) {
                    return Some(format!(
                        "'{} {}' is a dangerous operation",
                        cmd,
                        prefix.join(" ")
                    ));
                }
            }
            BlacklistRule::ArgExact {
                executable: cmd,
                arg,
            } => {
                if executable == *cmd && args.iter().any(|a| a == *arg) {
                    return Some(format!("'{} {}' is a dangerous operation", cmd, arg));
                }
            }
            BlacklistRule::ArgPrefix {
                executable: cmd,
                arg_prefix,
            } => {
                if executable == *cmd
                    && args
                        .iter()
                        .any(|a| a.to_uppercase().starts_with(&arg_prefix.to_uppercase()))
                {
                    return Some(format!(
                        "'{} {}...' is a dangerous operation",
                        cmd, arg_prefix
                    ));
                }
            }
            BlacklistRule::RawToken(token) => {
                if executable == *token || args.iter().any(|a| a == *token) {
                    return Some(format!("Token '{}' is not allowed", token));
                }
            }
        }
    }
    None
}

/// Checks if a command matches the user blacklist.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::transform::shell_expansion::policy::{check_user_blacklist, normalize_command};
/// use darkmatter::markdown::transform::shell_expansion::types::{ShellRuleSet, ShellRuleEntry};
///
/// let mut ruleset = ShellRuleSet::default();
/// ruleset.entries.push(ShellRuleEntry::Prefix("curl".to_string()));
///
/// let args = vec!["http://example.com".to_string()];
/// let normalized = normalize_command("curl", &args);
/// assert!(check_user_blacklist(&ruleset, "curl", &args, &normalized));
/// ```
pub fn check_user_blacklist(
    ruleset: &ShellRuleSet,
    executable: &str,
    _args: &[String],
    normalized: &str,
) -> bool {
    ruleset.matches_exact(normalized) || ruleset.matches_prefix(executable)
}

/// Checks if a command matches the whitelist.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::transform::shell_expansion::policy::{check_whitelist, normalize_command};
/// use darkmatter::markdown::transform::shell_expansion::types::{ShellRuleSet, ShellRuleEntry};
///
/// let mut ruleset = ShellRuleSet::default();
/// ruleset.entries.push(ShellRuleEntry::Prefix("ls".to_string()));
///
/// let args = vec!["-la".to_string()];
/// let normalized = normalize_command("ls", &args);
/// assert!(check_whitelist(&ruleset, "ls", &normalized));
/// ```
pub fn check_whitelist(ruleset: &ShellRuleSet, executable: &str, normalized: &str) -> bool {
    ruleset.matches_exact(normalized) || ruleset.matches_prefix(executable)
}

/// Normalizes a command into a stable string representation.
///
/// Produces `executable arg1 arg2 ...` with proper quoting for args
/// that contain whitespace or special characters.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::transform::shell_expansion::policy::normalize_command;
///
/// let args = vec!["hello".to_string(), "world".to_string()];
/// let normalized = normalize_command("echo", &args);
/// assert_eq!(normalized, "echo hello world");
///
/// let args = vec!["hello world".to_string()];
/// let normalized = normalize_command("echo", &args);
/// assert_eq!(normalized, r#"echo "hello world""#);
/// ```
pub fn normalize_command(executable: &str, args: &[String]) -> String {
    let mut parts = vec![executable.to_string()];

    for arg in args {
        if needs_quoting(arg) {
            parts.push(format!(
                "\"{}\"",
                arg.replace('\\', "\\\\").replace('"', "\\\"")
            ));
        } else {
            parts.push(arg.clone());
        }
    }

    parts.join(" ")
}

/// Returns true if the argument needs quoting in normalized form.
fn needs_quoting(arg: &str) -> bool {
    arg.chars()
        .any(|ch| ch.is_whitespace() || ch == '"' || ch == '\'' || ch == '\\')
}

/// Returns true if args start with the given prefix.
fn args_start_with(args: &[String], prefix: &[&str]) -> bool {
    if args.len() < prefix.len() {
        return false;
    }
    args.iter()
        .zip(prefix.iter())
        .all(|(arg, expected)| arg == *expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::transform::shell_expansion::types::ShellRuleEntry;

    #[test]
    fn builtin_blacklist_rejects_rm() {
        let reason = check_builtin_blacklist("rm", &["-rf".to_string(), "/".to_string()]);
        assert!(reason.is_some());
        assert!(reason.unwrap().contains("dangerous"));
    }

    #[test]
    fn builtin_blacklist_rejects_mkfs_prefix() {
        let reason = check_builtin_blacklist("mkfs.ext4", &["/dev/sda1".to_string()]);
        assert!(reason.is_some());
    }

    #[test]
    fn builtin_blacklist_rejects_git_reset() {
        let reason = check_builtin_blacklist("git", &["reset".to_string(), "--hard".to_string()]);
        assert!(reason.is_some());
        assert!(reason.unwrap().contains("git reset"));
    }

    #[test]
    fn builtin_blacklist_rejects_git_push_force() {
        let reason = check_builtin_blacklist(
            "git",
            &[
                "push".to_string(),
                "--force".to_string(),
                "origin".to_string(),
            ],
        );
        assert!(reason.is_some());
    }

    #[test]
    fn builtin_blacklist_rejects_docker_system_prune() {
        let reason = check_builtin_blacklist(
            "docker",
            &["system".to_string(), "prune".to_string(), "-a".to_string()],
        );
        assert!(reason.is_some());
    }

    #[test]
    fn builtin_blacklist_rejects_psql_drop() {
        let reason = check_builtin_blacklist("psql", &["-c".to_string(), "DROP TABLE".to_string()]);
        assert!(reason.is_some());
    }

    #[test]
    fn builtin_blacklist_rejects_curl_delete() {
        let reason = check_builtin_blacklist(
            "curl",
            &["-X DELETE".to_string(), "http://example.com".to_string()],
        );
        assert!(reason.is_some());
    }

    #[test]
    fn builtin_blacklist_rejects_bash_c() {
        let reason = check_builtin_blacklist("bash", &["-c".to_string(), "echo test".to_string()]);
        assert!(reason.is_some());
    }

    #[test]
    fn builtin_blacklist_rejects_find_delete() {
        let reason = check_builtin_blacklist(
            "find",
            &[
                ".".to_string(),
                "-name".to_string(),
                "*.tmp".to_string(),
                "-delete".to_string(),
            ],
        );
        assert!(reason.is_some());
    }

    #[test]
    fn builtin_blacklist_allows_safe_commands() {
        assert!(check_builtin_blacklist("echo", &["hello".to_string()]).is_none());
        assert!(check_builtin_blacklist("ls", &["-la".to_string()]).is_none());
        assert!(check_builtin_blacklist("pwd", &[]).is_none());
        assert!(check_builtin_blacklist("date", &[]).is_none());
        assert!(check_builtin_blacklist("git", &["status".to_string()]).is_none());
    }

    #[test]
    fn user_blacklist_exact_match() {
        let mut ruleset = ShellRuleSet::default();
        ruleset
            .entries
            .push(ShellRuleEntry::Exact("echo hello".to_string()));

        assert!(check_user_blacklist(
            &ruleset,
            "echo",
            &["hello".to_string()],
            "echo hello"
        ));
        assert!(!check_user_blacklist(
            &ruleset,
            "echo",
            &["world".to_string()],
            "echo world"
        ));
    }

    #[test]
    fn user_blacklist_prefix_match() {
        let mut ruleset = ShellRuleSet::default();
        ruleset
            .entries
            .push(ShellRuleEntry::Prefix("curl".to_string()));

        assert!(check_user_blacklist(
            &ruleset,
            "curl",
            &["http://example.com".to_string()],
            "curl http://example.com"
        ));
        assert!(!check_user_blacklist(
            &ruleset,
            "wget",
            &["http://example.com".to_string()],
            "wget http://example.com"
        ));
    }

    #[test]
    fn whitelist_exact_match() {
        let mut ruleset = ShellRuleSet::default();
        ruleset
            .entries
            .push(ShellRuleEntry::Exact("ls -la".to_string()));

        assert!(check_whitelist(&ruleset, "ls", "ls -la"));
        assert!(!check_whitelist(&ruleset, "ls", "ls -l"));
    }

    #[test]
    fn whitelist_prefix_match() {
        let mut ruleset = ShellRuleSet::default();
        ruleset
            .entries
            .push(ShellRuleEntry::Prefix("git".to_string()));

        assert!(check_whitelist(&ruleset, "git", "git status"));
        assert!(check_whitelist(&ruleset, "git", "git log"));
        assert!(!check_whitelist(&ruleset, "ls", "ls -la"));
    }

    #[test]
    fn normalize_simple_command() {
        let normalized = normalize_command("echo", &["hello".to_string()]);
        assert_eq!(normalized, "echo hello");
    }

    #[test]
    fn normalize_multiple_args() {
        let normalized = normalize_command("ls", &["-la".to_string(), "/tmp".to_string()]);
        assert_eq!(normalized, "ls -la /tmp");
    }

    #[test]
    fn normalize_arg_with_whitespace() {
        let normalized = normalize_command("echo", &["hello world".to_string()]);
        assert_eq!(normalized, r#"echo "hello world""#);
    }

    #[test]
    fn normalize_arg_with_quotes() {
        let normalized = normalize_command("echo", &[r#"say "hi""#.to_string()]);
        assert_eq!(normalized, r#"echo "say \"hi\"""#);
    }

    #[test]
    fn normalize_arg_with_backslashes() {
        let normalized = normalize_command("echo", &[r"path\to\file".to_string()]);
        assert_eq!(normalized, r#"echo "path\\to\\file""#);
    }

    #[test]
    fn normalize_no_args() {
        let normalized = normalize_command("pwd", &[]);
        assert_eq!(normalized, "pwd");
    }

    #[test]
    fn needs_quoting_detects_whitespace() {
        assert!(needs_quoting("hello world"));
        assert!(!needs_quoting("hello"));
    }

    #[test]
    fn needs_quoting_detects_quotes() {
        assert!(needs_quoting(r#"say "hi""#));
        assert!(needs_quoting("say 'hi'"));
    }

    #[test]
    fn needs_quoting_detects_backslashes() {
        assert!(needs_quoting(r"path\to\file"));
    }

    #[test]
    fn args_start_with_empty_prefix() {
        assert!(args_start_with(&["a".to_string()], &[]));
    }

    #[test]
    fn args_start_with_matching_prefix() {
        assert!(args_start_with(
            &["reset".to_string(), "--hard".to_string()],
            &["reset"]
        ));
        assert!(args_start_with(
            &["push".to_string(), "--force".to_string()],
            &["push", "--force"]
        ));
    }

    #[test]
    fn args_start_with_non_matching() {
        assert!(!args_start_with(&["status".to_string()], &["reset"]));
        assert!(!args_start_with(
            &["push".to_string()],
            &["push", "--force"]
        ));
    }
}
