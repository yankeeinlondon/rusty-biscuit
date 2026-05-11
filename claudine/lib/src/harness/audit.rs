//! Shell-command discovery and policy preflight.
//!
//! Collects all auditable commands from a `HarnessPlan` and optional source
//! page text, then runs each through shell policy without executing it.

use tracing::{debug, info_span};

use crate::harness::error::HarnessError;
use crate::harness::model::{
    AuditedCommand, AuditedCommandSource, HandlerAction, HarnessPlan, ShellAuditOutcome,
    ShellAuditReport, ValidationKind,
};
use crate::harness::report::prose_escape;
use crate::harness::shell::ShellApprovalOptions;

/// Collect all shell commands that must pass audit before the run proceeds.
pub fn collect_auditable_commands(
    plan: &HarnessPlan,
    source_text: Option<&str>,
) -> Result<Vec<AuditedCommand>, HarnessError> {
    let mut commands = Vec::new();

    // 1. Pre-checks — ShellCommand variants
    for rule in &plan.pre_checks {
        if let ValidationKind::ShellCommand { command, .. } = &rule.kind {
            commands.push(AuditedCommand {
                source: AuditedCommandSource::PreCheck(rule.id),
                raw: command.raw.clone(),
                executable: command.executable.clone(),
                args: command.args.clone(),
            });
        }
    }

    // 2. Post-checks — ShellCommand variants
    for rule in &plan.post_checks {
        if let ValidationKind::ShellCommand { command, .. } = &rule.kind {
            commands.push(AuditedCommand {
                source: AuditedCommandSource::PostCheck(rule.id),
                raw: command.raw.clone(),
                executable: command.executable.clone(),
                args: command.args.clone(),
            });
        }
    }

    // 3. Programmatic handle
    if let Some(cmd) = &plan.programmatic_handler {
        commands.push(AuditedCommand {
            source: AuditedCommandSource::ProgrammaticHandle,
            raw: cmd.raw.clone(),
            executable: cmd.executable.clone(),
            args: cmd.args.clone(),
        });
    }

    // 4. Declarative deviate handlers
    for rule in plan
        .handlers
        .exact
        .iter()
        .chain(plan.handlers.generic.iter())
    {
        if let HandlerAction::Deviate { command, .. } = &rule.action {
            commands.push(AuditedCommand {
                source: AuditedCommandSource::DeclarativeHandler {
                    event: rule.event.clone(),
                    subject_key: rule.subject_key.clone(),
                },
                raw: command.raw.clone(),
                executable: command.executable.clone(),
                args: command.args.clone(),
            });
        }
    }

    // 5. Source page ::shell directives
    if let Some(text) = source_text {
        let ctx = biscuit_terminal::errors::SourceContext::new(
            std::path::PathBuf::from("<harness-audit>"),
            std::path::PathBuf::from("<harness-audit>"),
            text.to_string(),
        );
        let directives =
            darkmatter::markdown::compose::shell_expansion::parse_directives(text, ctx).map_err(
                |e| HarnessError::ShellAuditParseError {
                    detail: e.to_string(),
                },
            )?;
        for directive in directives {
            commands.push(AuditedCommand {
                source: AuditedCommandSource::ComposeSourceLine {
                    line: directive.origin.line_number(),
                },
                raw: directive.raw_command.clone(),
                executable: directive.executable.clone(),
                args: directive.args.clone(),
            });
        }
    }

    Ok(commands)
}

/// Audit all collected commands against shell policy.
///
/// Reuses `validate_and_approve_command_parts` for each command.
/// Does not execute any commands.
pub fn audit_shell_commands(
    commands: &[AuditedCommand],
    options: &ShellApprovalOptions,
) -> ShellAuditReport {
    let command_count = commands.len();
    let _span = info_span!("harness_audit", command_count).entered();

    let outcomes: Vec<_> = commands
        .iter()
        .map(|cmd| {
            let parts: Vec<String> = std::iter::once(cmd.executable.clone())
                .chain(cmd.args.iter().cloned())
                .collect();

            let result = crate::harness::shell::validate_and_approve_command_parts(
                &parts, options, None, None,
            );

            match result {
                Ok(_) => {
                    debug!(command = %cmd.raw, "audit approved");
                    ShellAuditOutcome {
                        command: cmd.clone(),
                        passed: true,
                        message: format!(
                            "<green-500>{}</green-500> approved",
                            prose_escape(&cmd.raw)
                        ),
                    }
                }
                Err(HarnessError::ShellCommandDenied { .. }) => {
                    debug!(command = %cmd.raw, "audit denied by policy");
                    ShellAuditOutcome {
                        command: cmd.clone(),
                        passed: false,
                        message: format!(
                            "<red-500>{}</red-500> denied by policy",
                            prose_escape(&cmd.raw)
                        ),
                    }
                }
                Err(HarnessError::ShellCommandBlacklisted { reason, .. }) => {
                    debug!(command = %cmd.raw, reason = %reason, "audit blacklisted");
                    ShellAuditOutcome {
                        command: cmd.clone(),
                        passed: false,
                        message: format!(
                            "<red-500>{}</red-500> blacklisted: {}",
                            prose_escape(&cmd.raw),
                            prose_escape(&reason),
                        ),
                    }
                }
                Err(e) => {
                    debug!(command = %cmd.raw, error = %e, "audit error");
                    ShellAuditOutcome {
                        command: cmd.clone(),
                        passed: false,
                        message: format!(
                            "<red-500>{}</red-500> audit error: {}",
                            prose_escape(&cmd.raw),
                            prose_escape(&e.to_string()),
                        ),
                    }
                }
            }
        })
        .collect();

    let passed_count = outcomes.iter().filter(|o| o.passed).count();
    let failed_count = outcomes.len() - passed_count;
    debug!(
        total = outcomes.len(),
        passed = passed_count,
        failed = failed_count,
        "audit complete"
    );

    ShellAuditReport { outcomes }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::model::{
        ApprovedRuntimeCommand, HandlerRule, HandlerTable, ValidationEvent, ValidationPhase,
        ValidationRule, ValidationRuleId,
    };

    fn make_shell_rule(id: u32, phase_checks: &str) -> (ValidationRule, bool) {
        let cmd = ApprovedRuntimeCommand {
            raw: "echo hello".to_string(),
            executable: "echo".to_string(),
            args: vec!["hello".to_string()],
        };
        let rule = ValidationRule {
            id: ValidationRuleId(id),
            event: ValidationEvent::ShellCommand,
            phase: ValidationPhase::Both,
            kind: ValidationKind::ShellCommand {
                command: cmd,
                show_stdout: false,
                show_stderr: false,
            },
            message_template: None,
            subject_key: None,
            source: None,
        };
        (rule, phase_checks == "pre")
    }

    fn empty_plan() -> HarnessPlan {
        HarnessPlan {
            source_path: std::path::PathBuf::from("/tmp/source.md"),
            timeout: None,
            step_timeout: None,
            timeout_warn: None,
            step_timeout_warn: None,
            pre_checks: Vec::new(),
            post_checks: Vec::new(),
            handlers: HandlerTable::default(),
            programmatic_handler: None,
        }
    }

    #[test]
    fn empty_plan_returns_empty_vec() {
        let plan = empty_plan();
        let commands = collect_auditable_commands(&plan, None).unwrap();
        assert!(commands.is_empty());
    }

    #[test]
    fn collects_pre_check_shell_commands() {
        let mut plan = empty_plan();
        let (rule, _) = make_shell_rule(0, "pre");
        plan.pre_checks.push(rule);

        let commands = collect_auditable_commands(&plan, None).unwrap();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0].source,
            AuditedCommandSource::PreCheck(_)
        ));
    }

    #[test]
    fn collects_post_check_shell_commands() {
        let mut plan = empty_plan();
        let (rule, _) = make_shell_rule(0, "post");
        plan.post_checks.push(rule);

        let commands = collect_auditable_commands(&plan, None).unwrap();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0].source,
            AuditedCommandSource::PostCheck(_)
        ));
    }

    #[test]
    fn collects_programmatic_handler() {
        let mut plan = empty_plan();
        plan.programmatic_handler = Some(ApprovedRuntimeCommand {
            raw: "my-handler".to_string(),
            executable: "my-handler".to_string(),
            args: vec![],
        });

        let commands = collect_auditable_commands(&plan, None).unwrap();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0].source,
            AuditedCommandSource::ProgrammaticHandle
        ));
    }

    #[test]
    fn collects_deviate_handlers() {
        let mut plan = empty_plan();
        plan.handlers.generic.push(HandlerRule {
            event: crate::harness::model::FailureEvent::AgentFailure,
            subject_key: None,
            action: HandlerAction::Deviate {
                command: ApprovedRuntimeCommand {
                    raw: "fix-it".to_string(),
                    executable: "fix-it".to_string(),
                    args: vec![],
                },
                set: None,
                msg: None,
                say: None,
            },
        });

        let commands = collect_auditable_commands(&plan, None).unwrap();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0].source,
            AuditedCommandSource::DeclarativeHandler { .. }
        ));
    }

    #[test]
    fn ignores_non_deviate_handlers() {
        let mut plan = empty_plan();
        plan.handlers.generic.push(HandlerRule {
            event: crate::harness::model::FailureEvent::AgentFailure,
            subject_key: None,
            action: HandlerAction::Retry {
                prompt_suffix: None,
                set: None,
                msg: None,
                say: None,
                retries: None,
            },
        });

        let commands = collect_auditable_commands(&plan, None).unwrap();
        assert!(commands.is_empty());
    }

    #[test]
    fn collects_source_page_shell_directives() {
        let source = "# Test\n::shell echo hello world\nSome text\n";
        let plan = empty_plan();
        let commands = collect_auditable_commands(&plan, Some(source)).unwrap();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0].source,
            AuditedCommandSource::ComposeSourceLine { .. }
        ));
        assert_eq!(commands[0].executable, "echo");
    }

    // -- audit_shell_commands tests --

    fn make_audited_command(executable: &str, args: &[&str]) -> AuditedCommand {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let raw = std::iter::once(executable.to_string())
            .chain(args.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");
        AuditedCommand {
            source: AuditedCommandSource::PreCheck(ValidationRuleId(0)),
            raw,
            executable: executable.to_string(),
            args,
        }
    }

    fn permissive_options() -> ShellApprovalOptions {
        // No policy root, no approval handler → commands on PATH are approved
        ShellApprovalOptions::default()
    }

    #[test]
    fn audit_approved_command() {
        let cmd = make_audited_command("echo", &["hello"]);
        let report = audit_shell_commands(&[cmd], &permissive_options());
        assert_eq!(report.outcomes.len(), 1);
        assert!(report.outcomes[0].passed);
        assert!(report.outcomes[0].message.contains("approved"));
        assert!(report.all_passed());
        assert!(report.failures().is_empty());
    }

    #[test]
    fn audit_blacklisted_command() {
        let cmd = make_audited_command("rm", &["-rf", "/"]);
        let report = audit_shell_commands(&[cmd], &permissive_options());
        assert_eq!(report.outcomes.len(), 1);
        assert!(!report.outcomes[0].passed);
        assert!(report.outcomes[0].message.contains("blacklisted"));
        assert!(!report.all_passed());
        assert_eq!(report.failures().len(), 1);
    }

    #[test]
    fn audit_empty_commands_returns_empty_report() {
        let report = audit_shell_commands(&[], &permissive_options());
        assert!(report.outcomes.is_empty());
        assert!(report.all_passed());
        assert!(report.failures().is_empty());
    }

    #[test]
    fn audit_mixed_commands() {
        let good = make_audited_command("echo", &["ok"]);
        let bad = make_audited_command("rm", &["-rf", "/"]);
        let report = audit_shell_commands(&[good, bad], &permissive_options());
        assert_eq!(report.outcomes.len(), 2);
        assert!(report.outcomes[0].passed);
        assert!(!report.outcomes[1].passed);
        assert!(!report.all_passed());
        assert_eq!(report.failures().len(), 1);
    }

    #[test]
    fn audit_policy_denied_command() {
        // Use an isolated policy root with no whitelist and no approval handler.
        // A non-blacklisted command ("echo") is denied by policy, not blacklisted.
        let dir = tempfile::TempDir::new().unwrap();
        let options = ShellApprovalOptions {
            policy_root: Some(dir.path().to_path_buf()),
            approval_handler: None,
            ..Default::default()
        };
        let cmd = make_audited_command("echo", &["hello"]);
        let report = audit_shell_commands(&[cmd], &options);
        assert_eq!(report.outcomes.len(), 1);
        assert!(!report.outcomes[0].passed);
        assert!(
            report.outcomes[0].message.contains("denied by policy"),
            "expected 'denied by policy' but got: {}",
            report.outcomes[0].message
        );
        assert!(!report.all_passed());
        assert_eq!(report.failures().len(), 1);
    }

    #[test]
    fn audit_message_escapes_command_text() {
        // Command with markup characters should be escaped in the message
        let cmd = make_audited_command("echo", &["<b>bold</b>"]);
        let report = audit_shell_commands(&[cmd], &permissive_options());
        // The raw text should be escaped (no unescaped <b>)
        assert!(!report.outcomes[0].message.contains("<b>bold</b>"));
    }

    fn empty_plan_with_pre_check(executable: &str, args: &[&str]) -> HarnessPlan {
        let mut plan = HarnessPlan {
            source_path: std::path::PathBuf::from("/tmp/test.md"),
            timeout: None,
            step_timeout: None,
            timeout_warn: None,
            step_timeout_warn: None,
            pre_checks: Vec::new(),
            post_checks: Vec::new(),
            handlers: HandlerTable::default(),
            programmatic_handler: None,
        };
        plan.pre_checks.push(ValidationRule {
            id: ValidationRuleId(0),
            event: ValidationEvent::ShellCommand,
            phase: ValidationPhase::Both,
            kind: ValidationKind::ShellCommand {
                command: ApprovedRuntimeCommand {
                    raw: format!("{} {}", executable, args.join(" ")),
                    executable: executable.to_string(),
                    args: args.iter().map(|s| s.to_string()).collect(),
                },
                show_stdout: false,
                show_stderr: false,
            },
            message_template: None,
            subject_key: None,
            source: None,
        });
        plan
    }

    #[test]
    fn collect_auditable_commands_excludes_source_directives_when_none() {
        let plan = empty_plan_with_pre_check("echo", &["preflight"]);

        // With source text: includes both harness + source-page commands
        let with_source =
            collect_auditable_commands(&plan, Some("# Test\n::shell echo hidden\n")).unwrap();
        let source_page_count = with_source
            .iter()
            .filter(|c| matches!(c.source, AuditedCommandSource::ComposeSourceLine { .. }))
            .count();
        assert_eq!(
            source_page_count, 1,
            "source text should produce source-page commands"
        );

        // Without source text: only harness commands
        let without_source = collect_auditable_commands(&plan, None).unwrap();
        let source_page_count = without_source
            .iter()
            .filter(|c| matches!(c.source, AuditedCommandSource::ComposeSourceLine { .. }))
            .count();
        assert_eq!(
            source_page_count, 0,
            "None source_text must produce zero source-page commands"
        );

        // Harness commands still present in both
        assert!(
            without_source
                .iter()
                .any(|c| matches!(c.source, AuditedCommandSource::PreCheck(_))),
            "harness pre-check commands should still be collected"
        );
    }

    #[test]
    fn source_scan_finds_shell_hidden_by_false_block() {
        let plan = empty_plan();
        let source = "# Title\n::block when=\"false\"\n::shell curl https://example.com\n::end-block\nRegular text\n";

        // Raw source scan (Passthrough mode) picks up ::shell despite the
        // enclosing ::block when="false" because parse_directives does
        // line-level scanning without block context.
        let with_source = collect_auditable_commands(&plan, Some(source)).unwrap();
        let source_count = with_source
            .iter()
            .filter(|c| matches!(c.source, AuditedCommandSource::ComposeSourceLine { .. }))
            .count();
        assert_eq!(
            source_count, 1,
            "raw source scan should pick up ::shell inside ::block when=\"false\" (line-level scan)"
        );

        // Composition mode passes None — no source-page re-audit.
        // This is the fix from ef6e3cf2: composition flows must not re-parse raw
        // source because the false ::block hides the directive at the Darkmatter
        // level but not at the raw-text level.
        let without_source = collect_auditable_commands(&plan, None).unwrap();
        let source_count = without_source
            .iter()
            .filter(|c| matches!(c.source, AuditedCommandSource::ComposeSourceLine { .. }))
            .count();
        assert_eq!(
            source_count, 0,
            "composition mode (None source_text) must not find source-page directives"
        );
    }
}
