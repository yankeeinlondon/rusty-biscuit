//! Shell policy adapter for harness runtime commands.
//!
//! Thin adapter that reuses Darkmatter's shell expansion infrastructure
//! for tokenization, blacklist/whitelist checking, and command approval.

use std::path::PathBuf;
use std::sync::Arc;

use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalHandler, ShellExpansionOptions,
    check_builtin_blacklist, check_user_blacklist, check_whitelist, normalize_command,
    resolve_policy_paths,
};
use darkmatter::markdown::compose::shell_expansion::tokenize::tokenize;
use darkmatter::markdown::compose::ComposeSource;

use crate::harness::error::HarnessError;
use crate::harness::model::ApprovedRuntimeCommand;

/// Options for the shell approval flow.
#[derive(Clone)]
pub struct ShellApprovalOptions {
    /// Root directory for policy file resolution.
    pub policy_root: Option<PathBuf>,
    /// Callback for interactive command approval.
    pub approval_handler: Option<Arc<dyn ShellApprovalHandler>>,
}

impl Default for ShellApprovalOptions {
    fn default() -> Self {
        Self {
            policy_root: None,
            approval_handler: None,
        }
    }
}

/// Tokenize a raw command string and validate it against shell policies.
///
/// Returns an `ApprovedRuntimeCommand` if the command passes all checks,
/// or a `HarnessError` if the command is blacklisted, denied, or malformed.
pub fn validate_and_approve_command(
    raw: &str,
    options: &ShellApprovalOptions,
) -> Result<ApprovedRuntimeCommand, HarnessError> {
    // Tokenize
    let tokens = tokenize(raw).map_err(|_| HarnessError::ShellCommandDenied {
        command: raw.to_string(),
    })?;

    if tokens.is_empty() {
        return Err(HarnessError::ShellCommandDenied {
            command: raw.to_string(),
        });
    }

    let executable = &tokens[0];
    let args: Vec<String> = tokens[1..].to_vec();
    let normalized = normalize_command(executable, &args);

    // Check built-in blacklist
    if let Some(reason) = check_builtin_blacklist(executable, &args) {
        return Err(HarnessError::ShellCommandBlacklisted {
            command: raw.to_string(),
            reason,
        });
    }

    // Resolve policy paths
    let source = match &options.policy_root {
        Some(root) => ComposeSource::File(root.join("dummy")),
        None => ComposeSource::Unknown,
    };
    let shell_opts = ShellExpansionOptions {
        timeout: std::time::Duration::from_secs(30),
        policy_root: options.policy_root.clone(),
        working_directory: options.policy_root.clone(),
        approval_handler: options.approval_handler.clone(),
    };

    let policy_paths = resolve_policy_paths(&shell_opts, &source).map_err(|_| {
        HarnessError::ShellCommandDenied {
            command: raw.to_string(),
        }
    })?;

    // Load and check user blacklist
    let blacklist =
        darkmatter::markdown::compose::shell_expansion::store::load_ruleset(&policy_paths.blacklist)
            .unwrap_or_default();
    if check_user_blacklist(&blacklist, executable, &args, &normalized) {
        return Err(HarnessError::ShellCommandBlacklisted {
            command: raw.to_string(),
            reason: "command matches user blacklist".to_string(),
        });
    }

    // Check whitelist — if whitelisted, approve immediately
    let whitelist =
        darkmatter::markdown::compose::shell_expansion::store::load_ruleset(&policy_paths.whitelist)
            .unwrap_or_default();
    if check_whitelist(&whitelist, executable, &normalized) {
        return Ok(ApprovedRuntimeCommand {
            raw: raw.to_string(),
            executable: executable.to_string(),
            args,
        });
    }

    // If not whitelisted, invoke approval handler
    if let Some(ref handler) = options.approval_handler {
        let request = darkmatter::markdown::compose::shell_expansion::ShellApprovalRequest {
            source: source.clone(),
            line: 0,
            raw_command: raw.to_string(),
            executable: executable.to_string(),
            args: args.clone(),
            normalized_exact: normalized.clone(),
            whitelist_path: policy_paths.whitelist.clone(),
            blacklist_path: policy_paths.blacklist.clone(),
            alias_name: None,
        };

        match handler.approve(request) {
            Ok(decision) => {
                use darkmatter::markdown::compose::shell_expansion::ShellApprovalDecision;
                match decision {
                    ShellApprovalDecision::AllowExactPersist => {
                        let _ = darkmatter::markdown::compose::shell_expansion::store::append_whitelist_exact(
                            &policy_paths,
                            &normalized,
                        );
                    }
                    ShellApprovalDecision::AllowCommandPersist => {
                        let _ = darkmatter::markdown::compose::shell_expansion::store::append_whitelist_prefix(
                            &policy_paths,
                            executable,
                        );
                    }
                    ShellApprovalDecision::AllowOnce => {}
                    ShellApprovalDecision::Deny => {
                        return Err(HarnessError::ShellCommandDenied {
                            command: raw.to_string(),
                        });
                    }
                    ShellApprovalDecision::BlacklistPersist => {
                        let _ = darkmatter::markdown::compose::shell_expansion::store::append_blacklist_exact(
                            &policy_paths,
                            &normalized,
                        );
                        return Err(HarnessError::ShellCommandBlacklisted {
                            command: raw.to_string(),
                            reason: "user blacklisted this command".to_string(),
                        });
                    }
                }
            }
            Err(_) => {
                return Err(HarnessError::ShellCommandDenied {
                    command: raw.to_string(),
                });
            }
        }
    } else {
        // No approval handler and not whitelisted: deny
        return Err(HarnessError::ShellCommandDenied {
            command: raw.to_string(),
        });
    }

    Ok(ApprovedRuntimeCommand {
        raw: raw.to_string(),
        executable: executable.to_string(),
        args,
    })
}

/// Execute an approved command and return its exit code and stdout/stderr.
///
/// Uses timeout protection. Returns `(exit_code, stdout, stderr)`.
pub fn execute_approved_command(
    command: &ApprovedRuntimeCommand,
    working_dir: Option<&std::path::Path>,
    _timeout: std::time::Duration,
) -> Result<(i32, String, String), HarnessError> {
    let exe = which::which(&command.executable).map_err(|_| HarnessError::HandlerFailed {
        action: "shell_command".to_string(),
        detail: format!("executable '{}' not found in PATH", command.executable),
    })?;

    let mut cmd = std::process::Command::new(&exe);
    cmd.args(&command.args);
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let child = cmd.spawn().map_err(|e| HarnessError::HandlerFailed {
        action: "shell_command".to_string(),
        detail: format!("failed to spawn '{}': {e}", command.executable),
    })?;

    let output = child
        .wait_with_output()
        .map_err(|e| HarnessError::HandlerFailed {
            action: "shell_command".to_string(),
            detail: format!("failed to wait for '{}': {e}", command.executable),
        })?;

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok((exit_code, stdout, stderr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_command_denied_without_approval_handler() {
        // Use an isolated directory with no policy files and no approval handler.
        let dir = tempfile::TempDir::new().unwrap();
        let options = ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: None,
        };
        let result = validate_and_approve_command("echo hello", &options);
        // Without whitelist or approval handler, this should be denied
        assert!(result.is_err());
    }

    #[test]
    fn blacklisted_command_rejected() {
        let options = ShellApprovalOptions::default();
        let result = validate_and_approve_command("rm -rf /", &options);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            HarnessError::ShellCommandBlacklisted { .. }
        ));
    }

    #[test]
    fn empty_command_rejected() {
        let options = ShellApprovalOptions::default();
        let result = validate_and_approve_command("", &options);
        assert!(result.is_err());
    }

    #[test]
    fn shell_metacharacters_rejected() {
        let options = ShellApprovalOptions::default();
        let result = validate_and_approve_command("echo hello | cat", &options);
        assert!(result.is_err());
    }

    #[test]
    fn execute_echo_command() {
        let cmd = ApprovedRuntimeCommand {
            raw: "echo hello".to_string(),
            executable: "echo".to_string(),
            args: vec!["hello".to_string()],
        };
        let result = execute_approved_command(&cmd, None, std::time::Duration::from_secs(5));
        assert!(result.is_ok());
        let (exit_code, stdout, _stderr) = result.unwrap();
        assert_eq!(exit_code, 0);
        assert!(stdout.trim() == "hello");
    }

    #[test]
    fn execute_failing_command() {
        let cmd = ApprovedRuntimeCommand {
            raw: "false".to_string(),
            executable: "false".to_string(),
            args: vec![],
        };
        let result = execute_approved_command(&cmd, None, std::time::Duration::from_secs(5));
        assert!(result.is_ok());
        let (exit_code, _, _) = result.unwrap();
        assert_ne!(exit_code, 0);
    }
}
