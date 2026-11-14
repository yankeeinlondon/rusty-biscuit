use super::*;
use super::test_support::{saved_rules, user_permissions};
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
    // An isolated home with no permissions and no approval handler.
    let dir = tempfile::TempDir::new().unwrap();
    let options = ShellApprovalOptions {
        permissions: user_permissions(dir.path(), &[]),
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
fn captured_permissions_preapprove_and_an_exclusion_requires_approval() {
    let dir = tempfile::TempDir::new().unwrap();
    let options = ShellApprovalOptions {
        permissions: user_permissions(dir.path(), &["exact echo hello", "prefix curl", "!exact curl secrets"]),
        ..Default::default()
    };

    let allowed = validate_and_approve_command("echo hello", &options);
    let prefixed = validate_and_approve_command("curl x", &options);
    let excluded = validate_and_approve_command("curl secrets", &options);

    assert!(allowed.is_ok(), "{allowed:?}");
    assert!(prefixed.is_ok(), "{prefixed:?}");
    // An exclusion removes pre-approval; it never blacklists.
    assert!(matches!(excluded, Err(HarnessError::ShellCommandDenied { .. })), "{excluded:?}");
}

#[test]
fn an_excluded_command_is_asked_about_naming_its_exclusion() {
    let dir = tempfile::TempDir::new().unwrap();
    let handler = Arc::new(CapturingHandler::new());
    let options = ShellApprovalOptions {
        permissions: user_permissions(dir.path(), &["prefix curl", "!exact curl secrets"]),
        approval_handler: Some(handler.clone()),
        ..Default::default()
    };

    let approved = validate_and_approve_command("curl secrets", &options);

    assert!(approved.is_ok(), "a run's approval overrides an exclusion: {approved:?}");
    let request = handler.captured_request();
    assert_eq!(request.exclusion.map(|exclusion| exclusion.entry).as_deref(), Some("!exact curl secrets"));
    assert_eq!(request.save_target, Some(dir.path().join(".darkmatter.yaml")));
}

#[test]
fn invalid_permissions_fail_every_command_without_asking() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join(".darkmatter.yaml"), "permissions:\n  shell-commands:\n    - frobnicate ls\n").unwrap();
    let load = darkmatter::config::load::ConfigSnapshot::load(
        &biscuit_file::FileResolutionContext::from_snapshot(dir.path(), Some(dir.path().to_path_buf()), Default::default()),
        None,
        darkmatter::config::load::ConfigLoadMode::Passive,
    )
    .map(Arc::new);
    let handler = Arc::new(CountingApprovalHandler { approvals: AtomicUsize::new(0) });
    let options = ShellApprovalOptions {
        permissions: CapturedPermissions::of_load(load),
        approval_handler: Some(handler.clone()),
        ..Default::default()
    };

    let result = validate_and_approve_command("echo hello", &options);
    let builtin = validate_and_approve_command("rm -rf /", &options);

    assert!(matches!(result, Err(HarnessError::ShellPermissionsInvalid { .. })), "{result:?}");
    assert!(matches!(builtin, Err(HarnessError::ShellCommandBlacklisted { .. })), "{builtin:?}");
    assert_eq!(handler.approvals(), 0);
}

#[test]
fn always_allow_saves_the_rule_to_the_user_permissions() {
    struct Persist;
    impl ShellApprovalHandler for Persist {
        fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
            Ok(ShellApprovalDecision::AllowCommandPersist)
        }
    }
    let dir = tempfile::TempDir::new().unwrap();
    let options = ShellApprovalOptions {
        permissions: user_permissions(dir.path(), &[]),
        approval_handler: Some(Arc::new(Persist)),
        ..Default::default()
    };

    let first = validate_and_approve_command("git status", &options);
    let frozen = ShellApprovalOptions { approval_handler: None, ..options.clone() };
    let later = validate_and_approve_command("git log", &frozen);

    assert!(first.is_ok(), "{first:?}");
    assert_eq!(saved_rules(dir.path()), ["prefix git"]);
    // The saved rule approves the rest of the run without reloading.
    assert!(later.is_ok(), "{later:?}");
}

#[test]
fn an_unsaved_approval_still_approves_the_run() {
    struct Persist;
    impl ShellApprovalHandler for Persist {
        fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
            Ok(ShellApprovalDecision::AllowExactPersist)
        }
    }
    let dir = tempfile::TempDir::new().unwrap();
    let options = ShellApprovalOptions {
        permissions: user_permissions(dir.path(), &["!exact curl secrets"]),
        approval_handler: Some(Arc::new(Persist)),
        ..Default::default()
    };

    let approved = validate_and_approve_command("curl secrets", &options);

    assert!(approved.is_ok(), "{approved:?}");
    // The exclusion still matches, so nothing is saved and it is not removed.
    assert_eq!(saved_rules(dir.path()), ["!exact curl secrets"]);
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
        permissions: user_permissions(dir.path(), &[]),
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
        permissions: user_permissions(dir.path(), &[]),
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
        permissions: user_permissions(dir.path(), &[]),
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
