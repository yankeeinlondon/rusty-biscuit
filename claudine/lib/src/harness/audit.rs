//! Shell-command discovery and policy preflight.
//!
//! Collects all auditable commands from a harness source page, then runs each
//! through shell policy without executing it.

use tracing::{debug, info_span};

use crate::harness::error::HarnessError;
use crate::harness::model::{
    AuditedCommand, AuditedCommandSource, ShellAuditOutcome, ShellAuditReport,
};
use crate::harness::report::prose_escape;
use crate::harness::shell::ShellApprovalOptions;

/// Collect all shell commands that must pass audit before the run proceeds.
///
/// Only source-page `::shell` directives are collected here. Lifecycle stack
/// shell commands are audited during composition preflight.
pub fn collect_auditable_commands(
    source_text: Option<&str>,
) -> Result<Vec<AuditedCommand>, HarnessError> {
    let mut commands = Vec::new();

    if let Some(text) = source_text {
        let ctx = biscuit_terminal::errors::SourceContext::new(
            std::path::PathBuf::from("<harness-audit>"),
            std::path::PathBuf::from("<harness-audit>"),
            text.to_string(),
        );
        let directives = darkmatter::markdown::compose::shell_expansion::parse_directives(
            text, ctx, 0,
        )
        .map_err(|error| HarnessError::ShellAuditParseError {
            detail: error.to_string(),
            source: Box::new(error),
        })?;
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

    #[test]
    fn empty_source_text_returns_empty_vec() {
        let commands = collect_auditable_commands(None).unwrap();
        assert!(commands.is_empty());
    }

    #[test]
    fn collects_source_page_shell_directives() {
        let source = "# Test\n::shell echo hello world\nSome text\n";
        let commands = collect_auditable_commands(Some(source)).unwrap();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0].source,
            AuditedCommandSource::ComposeSourceLine { .. }
        ));
        assert_eq!(commands[0].executable, "echo");
    }

    // -- audit_shell_commands tests --

    fn make_audited_command(source: AuditedCommandSource) -> AuditedCommand {
        AuditedCommand {
            source,
            raw: "echo hello".to_string(),
            executable: "echo".to_string(),
            args: vec!["hello".to_string()],
        }
    }

    fn permissive_options() -> ShellApprovalOptions {
        // No policy root, no approval handler → commands on PATH are approved
        ShellApprovalOptions::default()
    }

    #[test]
    fn audit_approved_command() {
        let cmd = make_audited_command(AuditedCommandSource::ComposeSourceLine { line: 1 });
        let report = audit_shell_commands(&[cmd], &permissive_options());
        assert_eq!(report.outcomes.len(), 1);
        assert!(report.outcomes[0].passed);
        assert!(report.outcomes[0].message.contains("approved"));
        assert!(report.all_passed());
        assert!(report.failures().is_empty());
    }

    #[test]
    fn audit_blacklisted_command() {
        let cmd = AuditedCommand {
            source: AuditedCommandSource::ComposeSourceLine { line: 1 },
            raw: "rm -rf /".to_string(),
            executable: "rm".to_string(),
            args: vec!["-rf".to_string(), "/".to_string()],
        };
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
        let good = make_audited_command(AuditedCommandSource::ComposeSourceLine { line: 1 });
        let bad = AuditedCommand {
            source: AuditedCommandSource::ComposeSourceLine { line: 2 },
            raw: "rm -rf /".to_string(),
            executable: "rm".to_string(),
            args: vec!["-rf".to_string(), "/".to_string()],
        };
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
        let cmd = make_audited_command(AuditedCommandSource::ComposeSourceLine { line: 1 });
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
        let cmd = AuditedCommand {
            source: AuditedCommandSource::ComposeSourceLine { line: 1 },
            raw: "echo <b>bold</b>".to_string(),
            executable: "echo".to_string(),
            args: vec!["<b>bold</b>".to_string()],
        };
        let report = audit_shell_commands(&[cmd], &permissive_options());
        // The raw text should be escaped (no unescaped <b>)
        assert!(!report.outcomes[0].message.contains("<b>bold</b>"));
    }

    #[test]
    fn source_scan_finds_shell_hidden_by_false_block() {
        let source = "# Title\n::block when=\"false\"\n::shell curl https://example.com\n::end-block\nRegular text\n";

        // Raw source scan (Passthrough mode) picks up ::shell despite the
        // enclosing ::block when="false" because parse_directives does
        // line-level scanning without block context.
        let with_source = collect_auditable_commands(Some(source)).unwrap();
        let source_count = with_source.len();
        assert_eq!(
            source_count, 1,
            "raw source scan should pick up ::shell inside ::block when=\"false\" (line-level scan)"
        );

        // Composition mode passes None — no source-page re-audit.
        // This is the fix from ef6e3cf2: composition flows must not re-parse raw
        // source because the false ::block hides the directive at the Darkmatter
        // level but not at the raw-text level.
        let without_source = collect_auditable_commands(None).unwrap();
        assert_eq!(
            without_source.len(),
            0,
            "composition mode (None source_text) must not find source-page directives"
        );
    }
}
