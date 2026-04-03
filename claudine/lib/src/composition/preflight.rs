//! Pre-flight shell command approval for provider sessions.
//!
//! Scans all sources of shell commands (template directives, harness
//! pre/post checks, handlers), checks them against the whitelist,
//! prompts the user for any that need approval, and returns the full
//! pre-approved set.

use std::collections::HashSet;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use darkmatter::markdown::compose::shell_expansion::discovery::collect_shell_commands;
use darkmatter::markdown::compose::shell_expansion::policy::normalize_command;
use darkmatter::markdown::compose::shell_expansion::tokenize::tokenize;

use crate::composition::error::CompositionError;
use crate::harness::audit::collect_auditable_commands;
use crate::harness::model::HarnessPlan;
use crate::harness::shell::ShellApprovalOptions;

/// Result of pre-flight shell command approval.
#[derive(Debug)]
pub struct PreFlightResult {
    /// Normalized command strings approved for execution.
    pub approved_commands: HashSet<String>,
    /// Total commands discovered across all sources.
    pub total_discovered: usize,
    /// Commands that were already whitelisted.
    pub already_whitelisted: usize,
    /// Commands the user approved interactively.
    pub user_approved: usize,
}

/// Scans all sources of shell commands, checks whitelist, prompts user,
/// returns the full pre-approved set.
///
/// ## Sources
///
/// 1. Template `::shell` directives (via Darkmatter's document graph walker)
/// 2. Harness pre/post checks and handlers (via existing audit infrastructure)
///
/// ## Errors
///
/// - `ShellCommandDenied` if the user denies any command
/// - `PreFlightDiscoveryFailed` if Darkmatter's document graph walk fails
/// - `PreFlightFailed` for blacklisted commands or missing approval handler
pub fn resolve_shell_approvals(
    markdown: Option<&Markdown>,
    compose_options: Option<&ComposeOptions>,
    harness_plan: Option<&HarnessPlan>,
    approval_options: &ShellApprovalOptions,
) -> Result<PreFlightResult, CompositionError> {
    let mut all_commands: Vec<(String, std::path::PathBuf, usize)> = Vec::new();

    // -- Source 1: Template ::shell directives ---------------------------------
    if let (Some(md), Some(opts)) = (markdown, compose_options) {
        let entries = collect_shell_commands(md, opts)
            .map_err(|e| CompositionError::PreFlightDiscoveryFailed(e.to_string()))?;
        for entry in &entries {
            all_commands.push((
                entry.normalized.clone(),
                entry.source_file.clone(),
                entry.line,
            ));
        }
    }

    // -- Source 2 & 3: Harness commands ----------------------------------------
    if let Some(plan) = harness_plan {
        // Pass None for source_text since we already collected template
        // directives via Darkmatter's graph walker above.
        let auditable = collect_auditable_commands(plan, None)
            .map_err(|e| CompositionError::PreFlightFailed(e.to_string()))?;
        for cmd in &auditable {
            let normalized = normalize_command(&cmd.executable, &cmd.args);
            all_commands.push((normalized, plan.source_path.clone(), 0));
        }
    }

    // -- Deduplicate -----------------------------------------------------------
    let unique: Vec<(String, std::path::PathBuf, usize)> = {
        let mut seen = HashSet::new();
        all_commands
            .into_iter()
            .filter(|(normalized, _, _)| seen.insert(normalized.clone()))
            .collect()
    };

    let total_discovered = unique.len();
    let mut approved = HashSet::new();

    // Snapshot the approval cache size before we start so we can distinguish
    // whitelisted commands from interactively-approved ones.
    let cache_size_before = approval_options
        .approval_cache
        .lock()
        .map(|c| c.len())
        .unwrap_or(0);

    // -- Check each command against policy -------------------------------------
    for (normalized, source_file, line) in &unique {
        // Split normalized command back into parts for the existing validator.
        let parts: Vec<String> = tokenize(normalized).unwrap_or_else(|_| vec![normalized.clone()]);

        match crate::harness::shell::validate_and_approve_command_parts(
            &parts,
            approval_options,
            Some(source_file.as_path()),
            Some(*line),
        ) {
            Ok(_) => {
                approved.insert(normalized.clone());
            }
            Err(crate::harness::error::HarnessError::ShellCommandDenied { command }) => {
                if approval_options.approval_handler.is_some() {
                    // Handler was available but user denied
                    return Err(CompositionError::ShellCommandDenied {
                        command,
                        source_file: source_file.clone(),
                        line: *line,
                    });
                }
                // No handler -- cannot get approval
                let location = if *line > 0 {
                    format!("{}:{}", source_file.display(), line)
                } else {
                    source_file.display().to_string()
                };
                return Err(CompositionError::PreFlightFailed(format!(
                    "Shell command '{command}' at {location} requires approval but no approval handler \
                     is available. Add to whitelist or run interactively."
                )));
            }
            Err(crate::harness::error::HarnessError::ShellCommandBlacklisted {
                command,
                reason,
            }) => {
                let location = if *line > 0 {
                    format!("{}:{}", source_file.display(), line)
                } else {
                    source_file.display().to_string()
                };
                return Err(CompositionError::PreFlightFailed(format!(
                    "Shell command '{command}' at {location} is blacklisted: {reason}"
                )));
            }
            Err(e) => {
                return Err(CompositionError::PreFlightFailed(e.to_string()));
            }
        }
    }

    // Count how many were newly approved by the handler vs already whitelisted.
    let cache_size_after = approval_options
        .approval_cache
        .lock()
        .map(|c| c.len())
        .unwrap_or(0);
    let user_approved = cache_size_after.saturating_sub(cache_size_before);
    let already_whitelisted = total_discovered.saturating_sub(user_approved);

    Ok(PreFlightResult {
        approved_commands: approved,
        total_discovered,
        already_whitelisted,
        user_approved,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::model::{
        ApprovedRuntimeCommand, HandlerTable, HarnessPlan, ValidationEvent, ValidationKind,
        ValidationPhase, ValidationRule, ValidationRuleId,
    };
    use darkmatter::markdown::compose::shell_expansion::types::{
        ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
    };
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    struct CapturingHandler {
        captured: Arc<Mutex<Vec<ShellApprovalRequest>>>,
    }

    impl CapturingHandler {
        fn new() -> Self {
            Self {
                captured: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn captured_requests(&self) -> Vec<ShellApprovalRequest> {
            self.captured.lock().unwrap().clone()
        }
    }

    impl ShellApprovalHandler for CapturingHandler {
        fn approve(
            &self,
            request: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, ShellExpansionError> {
            self.captured.lock().unwrap().push(request);
            Ok(ShellApprovalDecision::AllowOnce)
        }
    }

    struct MockApprovalHandler {
        decision: ShellApprovalDecision,
        call_count: Arc<Mutex<usize>>,
    }

    impl MockApprovalHandler {
        fn new(decision: ShellApprovalDecision) -> Self {
            Self {
                decision,
                call_count: Arc::new(Mutex::new(0)),
            }
        }

        fn calls(&self) -> usize {
            *self.call_count.lock().unwrap()
        }
    }

    impl ShellApprovalHandler for MockApprovalHandler {
        fn approve(
            &self,
            _request: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, ShellExpansionError> {
            *self.call_count.lock().unwrap() += 1;
            Ok(self.decision.clone())
        }
    }

    fn empty_plan() -> HarnessPlan {
        HarnessPlan {
            source_path: PathBuf::from("/tmp/test.md"),
            timeout: None,
            pre_checks: Vec::new(),
            post_checks: Vec::new(),
            handlers: HandlerTable::default(),
            programmatic_handler: None,
        }
    }

    /// Creates a temp dir with a whitelist file that allows commands prefixed
    /// with the given executables.
    fn approval_options_with_whitelist(
        prefixes: &[&str],
    ) -> (tempfile::TempDir, ShellApprovalOptions) {
        let dir = tempfile::TempDir::new().unwrap();
        let whitelist_content: String = prefixes.iter().map(|p| format!("prefix {p}\n")).collect();
        std::fs::write(
            dir.path().join(".darkmatter-shell-whitelist"),
            whitelist_content,
        )
        .unwrap();
        let options = ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: None,
            ..Default::default()
        };
        (dir, options)
    }

    #[test]
    fn empty_sources_returns_empty_approved_set() {
        let options = ShellApprovalOptions::default();
        let result = resolve_shell_approvals(None, None, None, &options).unwrap();

        assert!(result.approved_commands.is_empty());
        assert_eq!(result.total_discovered, 0);
        assert_eq!(result.already_whitelisted, 0);
        assert_eq!(result.user_approved, 0);
    }

    #[test]
    fn discovers_commands_from_template() {
        let md: Markdown = "# Test\n::shell echo hello\n".into();
        let compose_options = ComposeOptions::new();
        let (_dir, approval_options) = approval_options_with_whitelist(&["echo"]);

        let result =
            resolve_shell_approvals(Some(&md), Some(&compose_options), None, &approval_options)
                .unwrap();

        assert_eq!(result.total_discovered, 1);
        assert!(result.approved_commands.contains("echo hello"));
    }

    #[test]
    fn discovers_commands_from_harness_plan() {
        let mut plan = empty_plan();
        plan.pre_checks.push(ValidationRule {
            id: ValidationRuleId(0),
            event: ValidationEvent::ShellCommand,
            phase: ValidationPhase::Both,
            kind: ValidationKind::ShellCommand {
                command: ApprovedRuntimeCommand {
                    raw: "echo world".to_string(),
                    executable: "echo".to_string(),
                    args: vec!["world".to_string()],
                },
                show_stdout: false,
                show_stderr: false,
            },
            message_template: None,
            subject_key: None,
        });

        let (_dir, approval_options) = approval_options_with_whitelist(&["echo"]);

        let result = resolve_shell_approvals(None, None, Some(&plan), &approval_options).unwrap();

        assert_eq!(result.total_discovered, 1);
        assert!(result.approved_commands.contains("echo world"));
    }

    #[test]
    fn blacklisted_command_returns_error() {
        let md: Markdown = "# Test\n::shell rm -rf /\n".into();
        let compose_options = ComposeOptions::new();
        let (_dir, approval_options) = approval_options_with_whitelist(&["rm"]);

        let result =
            resolve_shell_approvals(Some(&md), Some(&compose_options), None, &approval_options);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, CompositionError::PreFlightFailed(ref msg) if msg.contains("blacklisted")),
            "expected PreFlightFailed with blacklisted, got: {err}"
        );
    }

    #[test]
    fn deduplicates_commands_across_sources() {
        // Both the template and the harness plan provide "echo hello".
        let md: Markdown = "# Test\n::shell echo hello\n".into();
        let compose_options = ComposeOptions::new();

        let mut plan = empty_plan();
        plan.pre_checks.push(ValidationRule {
            id: ValidationRuleId(0),
            event: ValidationEvent::ShellCommand,
            phase: ValidationPhase::Both,
            kind: ValidationKind::ShellCommand {
                command: ApprovedRuntimeCommand {
                    raw: "echo hello".to_string(),
                    executable: "echo".to_string(),
                    args: vec!["hello".to_string()],
                },
                show_stdout: false,
                show_stderr: false,
            },
            message_template: None,
            subject_key: None,
        });

        let (_dir, approval_options) = approval_options_with_whitelist(&["echo"]);

        let result = resolve_shell_approvals(
            Some(&md),
            Some(&compose_options),
            Some(&plan),
            &approval_options,
        )
        .unwrap();

        assert_eq!(result.total_discovered, 1, "duplicate should be collapsed");
        assert_eq!(result.approved_commands.len(), 1);
    }

    #[test]
    fn full_flow_template_with_whitelisted_commands() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        std::fs::write(&file_path, "# Test\n::shell echo hello\n").unwrap();

        // Create whitelist with echo prefix
        let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
        std::fs::write(&whitelist_path, "prefix echo\n").unwrap();

        let md = Markdown::try_from(file_path.as_path()).unwrap();
        let compose_opts = ComposeOptions::new().with_source_file(&file_path);

        let options = ShellApprovalOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: None,
            ..Default::default()
        };

        // Pre-flight should discover and approve "echo hello"
        let result =
            resolve_shell_approvals(Some(&md), Some(&compose_opts), None, &options).unwrap();
        assert!(result.approved_commands.contains("echo hello"));

        // Now compose with the pre-approved set — should succeed
        let compose_with_approval = ComposeOptions::new()
            .with_source_file(&file_path)
            .with_pre_approved_commands(result.approved_commands);
        let (composed, _) = md.compose_with(compose_with_approval).unwrap();
        assert!(composed.content().contains("hello"));
    }

    #[test]
    fn full_flow_blacklisted_command_aborts_preflight() {
        let content = "# Test\n::shell rm -rf /\n";
        let md: Markdown = content.into();
        let compose_opts = ComposeOptions::new();

        let temp_dir = tempfile::TempDir::new().unwrap();
        let options = ShellApprovalOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: None,
            ..Default::default()
        };

        let err = resolve_shell_approvals(Some(&md), Some(&compose_opts), None, &options);
        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(msg.contains("blacklisted"), "got: {msg}");
        // Compose was never called — session would never start
    }

    #[test]
    fn full_flow_unapproved_command_rejected_at_compose_time() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        std::fs::write(
            &file_path,
            "# Test\n::shell echo hello\n::shell echo sneaky\n",
        )
        .unwrap();

        // Only pre-approve "echo hello", not "echo sneaky"
        let mut approved = std::collections::HashSet::new();
        approved.insert("echo hello".to_string());

        let options = ComposeOptions::new()
            .with_source_file(&file_path)
            .with_pre_approved_commands(approved);

        let md = Markdown::try_from(file_path.as_path()).unwrap();
        let err = md.compose_with(options).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not pre-approved"), "got: {msg}");
    }

    #[test]
    fn allow_once_populates_cache_without_persisting() {
        let md: Markdown = "# Test\n::shell echo test-once\n".into();
        let compose_options = ComposeOptions::new();

        let dir = tempfile::TempDir::new().unwrap();
        let handler = Arc::new(MockApprovalHandler::new(ShellApprovalDecision::AllowOnce));
        let options = ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: Some(handler.clone()),
            ..Default::default()
        };

        let result =
            resolve_shell_approvals(Some(&md), Some(&compose_options), None, &options).unwrap();

        assert_eq!(result.total_discovered, 1);
        assert_eq!(result.user_approved, 1);
        assert!(result.approved_commands.contains("echo test-once"));
    }

    #[test]
    fn deny_returns_shell_command_denied_error() {
        let md: Markdown = "# Test\n::shell echo test-deny\n".into();
        let compose_options = ComposeOptions::new();

        let dir = tempfile::TempDir::new().unwrap();
        let handler = Arc::new(MockApprovalHandler::new(ShellApprovalDecision::Deny));
        let options = ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: Some(handler.clone()),
            ..Default::default()
        };

        let result = resolve_shell_approvals(Some(&md), Some(&compose_options), None, &options);

        assert!(result.is_err());
        assert!(
            matches!(
                result.unwrap_err(),
                CompositionError::ShellCommandDenied { .. }
            ),
            "expected ShellCommandDenied"
        );
    }

    #[test]
    fn warm_cache_prevents_second_handler_invocation() {
        let md: Markdown = "# Test\n::shell echo cached\n".into();
        let compose_options = ComposeOptions::new();

        let dir = tempfile::TempDir::new().unwrap();
        let handler = Arc::new(MockApprovalHandler::new(ShellApprovalDecision::AllowOnce));
        let options = ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: Some(handler.clone()),
            ..Default::default()
        };

        // First call: handler invoked
        let result1 =
            resolve_shell_approvals(Some(&md), Some(&compose_options), None, &options).unwrap();
        assert_eq!(result1.user_approved, 1);
        assert_eq!(handler.calls(), 1);

        // Second call: cache hit, handler NOT invoked
        let result2 =
            resolve_shell_approvals(Some(&md), Some(&compose_options), None, &options).unwrap();
        assert_eq!(result2.total_discovered, 1);
        assert_eq!(
            handler.calls(),
            1,
            "handler should not be called again — cache hit"
        );
    }

    #[test]
    fn interactive_handler_is_invoked_for_non_whitelisted_command() {
        let md: Markdown = "# Test\n::shell curl https://example.com\n".into();
        let compose_options = ComposeOptions::new();

        let dir = tempfile::TempDir::new().unwrap();
        let handler = Arc::new(MockApprovalHandler::new(ShellApprovalDecision::AllowOnce));
        let options = ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: Some(handler.clone()),
            ..Default::default()
        };

        let result =
            resolve_shell_approvals(Some(&md), Some(&compose_options), None, &options).unwrap();

        assert_eq!(
            handler.calls(),
            1,
            "handler must be invoked for non-whitelisted command"
        );
        assert!(
            result
                .approved_commands
                .contains("curl https://example.com")
        );
        assert_eq!(result.user_approved, 1);
        assert_eq!(result.already_whitelisted, 0);
    }

    #[test]
    fn approval_request_carries_real_source_provenance() {
        use darkmatter::markdown::compose::ComposeSource;

        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("template.md");
        std::fs::write(&file_path, "# Test\n::shell curl https://example.com\n").unwrap();

        let md = Markdown::try_from(file_path.as_path()).unwrap();
        let compose_opts = ComposeOptions::new().with_source_file(&file_path);

        let handler = Arc::new(CapturingHandler::new());
        let options = ShellApprovalOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(handler.clone()),
            ..Default::default()
        };

        let _result =
            resolve_shell_approvals(Some(&md), Some(&compose_opts), None, &options).unwrap();

        let requests = handler.captured_requests();
        assert_eq!(requests.len(), 1, "handler should be called once");

        let req = &requests[0];
        // The source should be the real file, not a dummy path
        match &req.source {
            ComposeSource::File(path) => {
                assert_eq!(
                    path, &file_path,
                    "source should be the template file, not a dummy"
                );
            }
            other => panic!("expected File source, got: {other:?}"),
        }
        assert!(req.line > 0, "line should be the real line number, not 0");
    }
}
