//! Shell policy adapter for harness runtime commands.
//!
//! Thin adapter that reuses Darkmatter's shell expansion infrastructure
//! for tokenization, blacklist/whitelist checking, and command approval.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::compose::ShellCommandOrigin;
use darkmatter::markdown::compose::shell_expansion::tokenize::{ShellToken, tokenize};
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalHandler, ShellExpansionError, ShellExpansionOptions, check_builtin_blacklist,
    check_user_blacklist, check_whitelist, normalize_command, resolve_policy_paths,
};

use tracing::{debug, info_span};

use crate::harness::error::{HarnessError, ShellExecCause};
use crate::harness::model::ApprovedRuntimeCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachedApprovalDecision {
    Allowed,
    Denied,
    Blacklisted,
}

/// Options for the shell approval flow.
#[derive(Clone)]
pub struct ShellApprovalOptions {
    /// Root directory for policy file resolution.
    pub policy_root: Option<PathBuf>,
    /// Callback for interactive command approval.
    pub approval_handler: Option<Arc<dyn ShellApprovalHandler>>,
    /// Session-local cache for repeated approval decisions.
    pub approval_cache: Arc<Mutex<HashMap<String, CachedApprovalDecision>>>,
    /// When set, a non-TTY unapproved command is reported with dry-run
    /// framing (a `Cannot dry-run: …` message pointing at `--yolo` /
    /// pre-approval) instead of the generic "no approval handler" guidance.
    pub dry_run: bool,
}

impl Default for ShellApprovalOptions {
    fn default() -> Self {
        Self {
            policy_root: None,
            approval_handler: None,
            approval_cache: Arc::new(Mutex::new(HashMap::new())),
            dry_run: false,
        }
    }
}

/// Tokenize a raw command string and validate it against shell policies.
///
/// Single-command contract: chain operators (`&&`, `||`) and redirections
/// (`> /dev/null`, `2>&1`, etc.) are rejected. Chains are split into
/// individual commands upstream by Darkmatter's discovery layer before
/// reaching this validator.
///
/// ## Errors
///
/// Returns `HarnessError::ShellCommandDenied` if the command fails to
/// tokenize, is empty, or contains chain operators or redirections.
pub fn validate_and_approve_command(
    raw: &str,
    options: &ShellApprovalOptions,
) -> Result<ApprovedRuntimeCommand, HarnessError> {
    let _span = info_span!("harness_shell_validate", command = %raw).entered();

    let tokens = tokenize_words_strict(raw).map_err(|_| HarnessError::ShellCommandDenied {
        command: raw.to_string(),
    })?;

    if tokens.is_empty() {
        return Err(HarnessError::ShellCommandDenied {
            command: raw.to_string(),
        });
    }

    validate_and_approve_command_parts(&tokens, options, None, None)
}

/// Tokenize a raw command string into Word-only parts.
///
/// Rejects chain operators and redirections so single-command callers
/// cannot accidentally validate a chain as if it were one command.
/// Chains are split per-command upstream by Darkmatter's discovery layer.
#[allow(clippy::result_large_err)]
pub(crate) fn tokenize_words_strict(raw: &str) -> Result<Vec<String>, ShellExpansionError> {
    let ctx = biscuit_terminal::errors::SourceContext::new(
        PathBuf::from("<harness-shell>"),
        PathBuf::from("<harness-shell>"),
        raw.to_string(),
    );
    let tokens = tokenize(raw, &ctx)?;
    let mut words = Vec::with_capacity(tokens.len());
    for tok in tokens {
        match tok {
            ShellToken::Word(w) => words.push(w),
            _ => {
                return Err(ShellExpansionError::ParseDirective {
                    ctx: Box::new(ctx.clone()),
                    origin: ShellCommandOrigin::Body { line: 0 },
                    message: format!(
                        "chain operators and redirections are not allowed here: {raw}"
                    ),
                });
            }
        }
    }
    Ok(words)
}

/// Validate an already-tokenized command against shell policies.
///
/// This is used for structured command inputs such as JSON arrays or expanded
/// objects where the caller has already separated the executable from its args.
pub fn validate_and_approve_command_parts(
    parts: &[String],
    options: &ShellApprovalOptions,
    source_file: Option<&Path>,
    source_line: Option<usize>,
) -> Result<ApprovedRuntimeCommand, HarnessError> {
    let raw = parts.join(" ");
    let _span = info_span!(
        "harness_shell_validate",
        command = %raw,
        ?source_file,
        source_line
    )
    .entered();

    if parts.is_empty() {
        return Err(HarnessError::ShellCommandDenied {
            command: String::new(),
        });
    }

    let executable = &parts[0];
    let args: Vec<String> = parts[1..].to_vec();
    let normalized = normalize_command(executable, &args);

    // Check built-in blacklist
    if let Some(reason) = check_builtin_blacklist(executable, &args) {
        debug!(reason = %reason, "shell command blocked by builtin blacklist");
        return Err(HarnessError::ShellCommandBlacklisted {
            command: raw.clone(),
            reason,
        });
    }

    // Resolve policy paths — needs a file under the policy root for path resolution.
    let policy_source = match &options.policy_root {
        Some(root) => ComposeSource::File(root.join("dummy")),
        None => ComposeSource::Unknown,
    };

    // Display source for approval prompts — use real provenance when available.
    let display_source = match source_file {
        Some(path) => ComposeSource::File(path.to_path_buf()),
        None => policy_source.clone(),
    };
    let display_origin = ShellCommandOrigin::Body {
        line: source_line.unwrap_or(0),
    };

    let shell_opts = ShellExpansionOptions {
        timeout: std::time::Duration::from_secs(30),
        policy_root: options.policy_root.clone(),
        working_directory: options.policy_root.clone(),
        approval_handler: options.approval_handler.clone(),
        ..Default::default()
    };

    let policy_paths = resolve_policy_paths(&shell_opts, &policy_source).map_err(|_| {
        HarnessError::ShellCommandDenied {
            command: raw.clone(),
        }
    })?;

    // Load and check user blacklist
    let blacklist = darkmatter::markdown::compose::shell_expansion::store::load_ruleset(
        &policy_paths.blacklist,
    )
    .unwrap_or_default();
    if check_user_blacklist(&blacklist, executable, &args, &normalized) {
        debug!("shell command blocked by user blacklist");
        return Err(HarnessError::ShellCommandBlacklisted {
            command: raw.clone(),
            reason: "command matches user blacklist".to_string(),
        });
    }

    // Check whitelist — if whitelisted, approve immediately
    let whitelist = darkmatter::markdown::compose::shell_expansion::store::load_ruleset(
        &policy_paths.whitelist,
    )
    .unwrap_or_default();
    if check_whitelist(&whitelist, executable, &normalized) {
        debug!("shell command approved via whitelist");
        return Ok(ApprovedRuntimeCommand {
            raw,
            executable: executable.to_string(),
            args,
        });
    }

    if let Some(decision) = options
        .approval_cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&normalized)
        .copied()
    {
        match decision {
            CachedApprovalDecision::Allowed => {
                debug!("shell command approved via cache");
                return Ok(ApprovedRuntimeCommand {
                    raw,
                    executable: executable.to_string(),
                    args,
                });
            }
            CachedApprovalDecision::Denied => {
                debug!("shell command denied via cache");
                return Err(HarnessError::ShellCommandDenied {
                    command: raw.clone(),
                });
            }
            CachedApprovalDecision::Blacklisted => {
                debug!("shell command blacklisted via cache");
                return Err(HarnessError::ShellCommandBlacklisted {
                    command: raw.clone(),
                    reason: "command was previously blacklisted during this session".to_string(),
                });
            }
        }
    }

    // If not whitelisted, invoke approval handler
    if let Some(ref handler) = options.approval_handler {
        let request = darkmatter::markdown::compose::shell_expansion::ShellApprovalRequest {
            source: display_source.clone(),
            origin: display_origin,
            raw_command: raw.clone(),
            executable: executable.to_string(),
            args: args.clone(),
            normalized_exact: normalized.clone(),
            whitelist_path: policy_paths.whitelist.clone(),
            blacklist_path: policy_paths.blacklist.clone(),
            alias_name: None,
            chain_executables: Vec::new(),
        };

        match handler.approve(request) {
            Ok(decision) => {
                use darkmatter::markdown::compose::shell_expansion::ShellApprovalDecision;
                match decision {
                    ShellApprovalDecision::AllowExactPersist => {
                        debug!("handler approved (AllowExactPersist), persisting to whitelist");
                        let _ = darkmatter::markdown::compose::shell_expansion::store::append_whitelist_exact(
                            &policy_paths,
                            &normalized,
                        );
                        cache_approval_decision(
                            &options.approval_cache,
                            &normalized,
                            CachedApprovalDecision::Allowed,
                        );
                    }
                    ShellApprovalDecision::AllowCommandPersist => {
                        debug!("handler approved (AllowCommandPersist), persisting to whitelist");
                        let _ = darkmatter::markdown::compose::shell_expansion::store::append_whitelist_prefix(
                            &policy_paths,
                            executable,
                        );
                        cache_approval_decision(
                            &options.approval_cache,
                            &normalized,
                            CachedApprovalDecision::Allowed,
                        );
                    }
                    ShellApprovalDecision::AllowOnce => {
                        debug!("handler approved (AllowOnce)");
                        cache_approval_decision(
                            &options.approval_cache,
                            &normalized,
                            CachedApprovalDecision::Allowed,
                        );
                    }
                    ShellApprovalDecision::Deny => {
                        debug!("handler denied command");
                        cache_approval_decision(
                            &options.approval_cache,
                            &normalized,
                            CachedApprovalDecision::Denied,
                        );
                        return Err(HarnessError::ShellCommandDenied {
                            command: raw.clone(),
                        });
                    }
                    ShellApprovalDecision::BlacklistPersist => {
                        debug!("handler blacklisted command");
                        let _ = darkmatter::markdown::compose::shell_expansion::store::append_blacklist_exact(
                            &policy_paths,
                            &normalized,
                        );
                        cache_approval_decision(
                            &options.approval_cache,
                            &normalized,
                            CachedApprovalDecision::Blacklisted,
                        );
                        return Err(HarnessError::ShellCommandBlacklisted {
                            command: raw.clone(),
                            reason: "user blacklisted this command".to_string(),
                        });
                    }
                }
            }
            Err(_) => {
                debug!("handler returned error, denying command");
                return Err(HarnessError::ShellCommandDenied {
                    command: raw.clone(),
                });
            }
        }
    } else {
        debug!("no approval handler and not whitelisted, denying command");
        return Err(HarnessError::ShellCommandDenied { command: raw });
    }

    debug!("shell command approved");
    Ok(ApprovedRuntimeCommand {
        raw,
        executable: executable.to_string(),
        args,
    })
}

fn cache_approval_decision(
    cache: &Arc<Mutex<HashMap<String, CachedApprovalDecision>>>,
    normalized: &str,
    decision: CachedApprovalDecision,
) {
    let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
    cache.insert(normalized.to_string(), decision);
}

/// Execute an approved command asynchronously and return its exit code and stdout/stderr.
///
/// Uses `tokio::process::Command` so the async runtime can await the child
/// process without busy-waiting or blocking a worker thread.
///
/// Enforces the given timeout: if the command does not complete within the
/// duration, it is killed with SIGKILL and an error is returned.
///
/// Returns `(exit_code, stdout, stderr)`.
pub async fn execute_approved_command(
    command: &ApprovedRuntimeCommand,
    working_dir: Option<&std::path::Path>,
    timeout: std::time::Duration,
) -> Result<(i32, String, String), HarnessError> {
    let _span = info_span!(
        "harness_shell_execute",
        command = %command.raw,
        timeout_secs = timeout.as_secs()
    )
    .entered();

    let start = Instant::now();
    let exe = which::which(&command.executable).map_err(|error| {
        HarnessError::ShellCommandExecutionFailed {
            detail: format!("executable '{}' not found in PATH", command.executable),
            source: ShellExecCause::Which(error),
        }
    })?;

    let mut cmd = tokio::process::Command::new(&exe);
    cmd.args(&command.args);
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }
    crate::child_environment::contribute_child_environment(&mut cmd).map_err(|error| {
        HarnessError::ShellCommandExecutionFailed {
            detail: format!("failed to prepare '{}': {error}", command.executable),
            source: ShellExecCause::Spawn(std::io::Error::other(error)),
        }
    })?;
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|error| HarnessError::ShellCommandExecutionFailed {
        detail: format!("failed to spawn '{}': {error}", command.executable),
        source: ShellExecCause::Spawn(error),
    })?;

    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();

    let result = tokio::time::timeout(timeout, child.wait()).await;

    match result {
        Ok(Ok(status)) => {
            let exit_code = status.code().unwrap_or(-1);
            let duration_ms = start.elapsed().as_millis() as u64;

            let stdout = match stdout_pipe {
                Some(mut pipe) => {
                    use tokio::io::AsyncReadExt;
                    let mut buf = Vec::new();
                    let _ = pipe.read_to_end(&mut buf).await;
                    String::from_utf8_lossy(&buf).to_string()
                }
                None => String::new(),
            };

            let stderr = match stderr_pipe {
                Some(mut pipe) => {
                    use tokio::io::AsyncReadExt;
                    let mut buf = Vec::new();
                    let _ = pipe.read_to_end(&mut buf).await;
                    String::from_utf8_lossy(&buf).to_string()
                }
                None => String::new(),
            };

            debug!(
                exit_code,
                duration_ms,
                stdout_len = stdout.len(),
                stderr_len = stderr.len(),
                "shell command completed"
            );
            Ok((exit_code, stdout, stderr))
        }
        Ok(Err(error)) => {
            debug!(error = %error, "shell command wait failed");
            Err(HarnessError::ShellCommandExecutionFailed {
                detail: format!("failed to wait for '{}': {error}", command.executable),
                source: ShellExecCause::Wait(error),
            })
        }
        Err(elapsed) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            debug!(timeout_secs = timeout.as_secs(), "shell command timed out");
            Err(HarnessError::ShellCommandExecutionFailed {
                detail: format!(
                    "command '{}' timed out after {}s",
                    command.raw,
                    timeout.as_secs()
                ),
                source: ShellExecCause::Timeout(elapsed),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkmatter::markdown::compose::shell_expansion::{
        ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
    };
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CapturingHandler {
        captured: Arc<Mutex<Option<ShellApprovalRequest>>>,
    }

    impl CapturingHandler {
        fn new() -> Self {
            Self {
                captured: Arc::new(Mutex::new(None)),
            }
        }

        fn captured_request(&self) -> ShellApprovalRequest {
            self.captured
                .lock()
                .unwrap()
                .clone()
                .expect("handler was never called")
        }
    }

    impl ShellApprovalHandler for CapturingHandler {
        fn approve(
            &self,
            request: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, ShellExpansionError> {
            *self.captured.lock().unwrap() = Some(request);
            Ok(ShellApprovalDecision::AllowOnce)
        }
    }

    struct CountingApprovalHandler {
        approvals: AtomicUsize,
    }

    impl CountingApprovalHandler {
        fn approvals(&self) -> usize {
            self.approvals.load(Ordering::SeqCst)
        }
    }

    impl ShellApprovalHandler for CountingApprovalHandler {
        fn approve(
            &self,
            _request: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, ShellExpansionError> {
            self.approvals.fetch_add(1, Ordering::SeqCst);
            Ok(ShellApprovalDecision::AllowOnce)
        }
    }

    #[test]
    fn unknown_command_denied_without_approval_handler() {
        // Use an isolated directory with no policy files and no approval handler.
        let dir = tempfile::TempDir::new().unwrap();
        let options = ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: None,
            ..Default::default()
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
    fn chain_operators_rejected_at_single_command_slot() {
        // Chains are split per-command upstream by Darkmatter discovery; a
        // raw chain reaching this validator means a misuse and must error.
        let options = ShellApprovalOptions::default();
        for raw in [
            "git status && git push",
            "make build || echo failed",
            "ls > /dev/null",
            "cmd 2>&1",
        ] {
            let result = validate_and_approve_command(raw, &options);
            assert!(
                matches!(result, Err(HarnessError::ShellCommandDenied { .. })),
                "expected denial for {raw:?}, got {result:?}"
            );
        }
    }

    #[test]
    fn approval_cache_avoids_prompting_for_same_command_twice() {
        let dir = tempfile::TempDir::new().unwrap();
        let handler = Arc::new(CountingApprovalHandler {
            approvals: AtomicUsize::new(0),
        });
        let options = ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: Some(handler.clone()),
            ..Default::default()
        };

        let first = validate_and_approve_command("echo hello", &options);
        let second = validate_and_approve_command("echo hello", &options);

        assert!(first.is_ok());
        assert!(second.is_ok());
        assert_eq!(handler.approvals(), 1);
    }

    #[tokio::test]
    async fn execute_echo_command() {
        let cmd = ApprovedRuntimeCommand {
            raw: "echo hello".to_string(),
            executable: "echo".to_string(),
            args: vec!["hello".to_string()],
        };
        let result = execute_approved_command(&cmd, None, std::time::Duration::from_secs(5)).await;
        assert!(result.is_ok());
        let (exit_code, stdout, _stderr) = result.unwrap();
        assert_eq!(exit_code, 0);
        assert!(stdout.trim() == "hello");
    }

    #[tokio::test]
    async fn execute_failing_command() {
        let cmd = ApprovedRuntimeCommand {
            raw: "false".to_string(),
            executable: "false".to_string(),
            args: vec![],
        };
        let result = execute_approved_command(&cmd, None, std::time::Duration::from_secs(5)).await;
        assert!(result.is_ok());
        let (exit_code, _, _) = result.unwrap();
        assert_ne!(exit_code, 0);
    }

    #[tokio::test]
    async fn approved_command_observes_agent_cwd() {
        let expected = crate::child_environment::initialize_process_launch_directory(
            crate::child_environment::LaunchDirectoryMode::Ordinary,
        )
        .unwrap()
        .to_string_lossy()
        .into_owned();
        let cmd = agent_cwd_command();
        let (exit_code, stdout, _) = execute_approved_command(
            &cmd,
            None,
            std::time::Duration::from_secs(5),
        )
        .await
        .unwrap();
        assert_eq!(exit_code, 0);
        assert_eq!(stdout.trim(), expected);
    }

    #[cfg(windows)]
    fn agent_cwd_command() -> ApprovedRuntimeCommand {
        ApprovedRuntimeCommand {
            raw: "echo %AGENT_CWD%".to_string(),
            executable: "cmd.exe".to_string(),
            args: vec!["/D".to_string(), "/C".to_string(), "echo %AGENT_CWD%".to_string()],
        }
    }

    #[cfg(not(windows))]
    fn agent_cwd_command() -> ApprovedRuntimeCommand {
        ApprovedRuntimeCommand {
            raw: "printf %s \"$AGENT_CWD\"".to_string(),
            executable: "sh".to_string(),
            args: vec!["-c".to_string(), "printf %s \"$AGENT_CWD\"".to_string()],
        }
    }

    #[test]
    fn provenance_is_passed_through_to_approval_request() {
        use darkmatter::markdown::compose::ComposeSource;

        let dir = tempfile::TempDir::new().unwrap();
        let handler = Arc::new(CapturingHandler::new());
        let options = ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: Some(handler.clone()),
            ..Default::default()
        };

        let source_file = PathBuf::from("/home/user/project/template.md");
        let source_line = 42usize;

        let _ = validate_and_approve_command_parts(
            &["echo".to_string(), "hello".to_string()],
            &options,
            Some(&source_file),
            Some(source_line),
        );

        let captured = handler.captured_request();
        assert_eq!(
            captured.source,
            ComposeSource::File(source_file),
            "request should carry the real source file, not a dummy path"
        );
        assert_eq!(
            captured.origin,
            darkmatter::markdown::compose::ShellCommandOrigin::Body { line: 42 },
            "request should carry the real line number"
        );
    }

    #[test]
    fn frozen_options_deny_new_commands_after_cache_populated() {
        let dir = tempfile::TempDir::new().unwrap();
        let handler = Arc::new(CountingApprovalHandler {
            approvals: AtomicUsize::new(0),
        });
        let mut options = ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: Some(handler.clone()),
            ..Default::default()
        };

        // Approve a command while the handler is active — populates the cache.
        let first = validate_and_approve_command("echo hello", &options);
        assert!(first.is_ok(), "should approve with active handler");
        assert_eq!(handler.approvals(), 1);

        // Freeze: strip the handler (simulates CachedHarnessLoopContext::freeze_shell_approvals).
        options.approval_handler = None;

        // Previously-approved command still passes via cache.
        let cached = validate_and_approve_command("echo hello", &options);
        assert!(
            cached.is_ok(),
            "cached command should still pass after freeze"
        );

        // A NEW command that was never approved is denied — no handler to prompt.
        let new_cmd = validate_and_approve_command("curl https://example.com", &options);
        assert!(
            new_cmd.is_err(),
            "new command must be denied after freeze — no handler available"
        );
        assert!(matches!(
            new_cmd.unwrap_err(),
            HarnessError::ShellCommandDenied { .. }
        ));

        // Handler was only called once (for the original "echo hello"), never for "curl".
        assert_eq!(
            handler.approvals(),
            1,
            "handler must not be called after freeze"
        );
    }
}
