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
fn full_approval_reads_both_policy_files() {
    let dir = tempfile::TempDir::new().unwrap();
    let whitelist = dir.path().join(".darkmatter-shell-whitelist");
    let blacklist = dir.path().join(".darkmatter-shell-blacklist");
    std::fs::write(&whitelist, "exact echo hello\n").unwrap();
    std::fs::write(&blacklist, "prefix curl\n").unwrap();
    let options = ShellApprovalOptions { policy_root: Some(dir.path().to_path_buf()), ..Default::default() };
    let reads = policy_reads::observe_this_thread(&[]);

    let approved = validate_and_approve_command("echo hello", &options);
    let blacklisted = validate_and_approve_command("curl x", &options);

    assert!(approved.is_ok(), "{approved:?}");
    assert!(matches!(blacklisted, Err(HarnessError::ShellCommandBlacklisted { .. })), "{blacklisted:?}");
    assert_eq!(reads.reads(), [whitelist.as_path(), &blacklist, &whitelist, &blacklist]);
}

#[test]
fn full_approval_denies_an_underivable_policy_root() {
    let options = ShellApprovalOptions {
        policy_root: Some(PathBuf::from("relative-policy-root")),
        approval_handler: Some(Arc::new(CountingApprovalHandler { approvals: AtomicUsize::new(0) })),
        ..Default::default()
    };
    let reads = policy_reads::observe_this_thread(&[]);

    let result = validate_and_approve_command("echo hello", &options);

    assert!(matches!(result, Err(HarnessError::ShellCommandDenied { .. })), "{result:?}");
    assert!(reads.reads().is_empty());
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
