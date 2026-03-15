//! Shell command expansion for markdown documents.
//!
//! This module provides support for `::shell` directives that execute shell commands
//! and insert their output into the rendered document.
//!
//! ## Security
//!
//! Shell expansion is inherently dangerous and requires careful security controls:
//!
//! - Built-in blacklist blocks dangerous commands (rm, dd, chmod, etc.)
//! - User whitelist and blacklist files (`.darkmatter-shell-whitelist`, `.darkmatter-shell-blacklist`)
//! - Approval handler callback for interactive prompting
//! - Command validation and normalization
//! - Timeout protection
//! - No shell metacharacters or redirections allowed
//!
//! ## Examples
//!
//! ```markdown
//! # System Information
//!
//! Current date: ::shell date +%Y-%m-%d
//!
//! Files in current directory:
//! ::shell ls -la
//! ```

pub mod executor;
pub mod parser;
pub mod policy;
pub mod store;
pub mod tokenize;
pub mod types;

pub use executor::{execute_command, resolve_working_directory};
pub use parser::parse_directives;
pub use policy::{check_builtin_blacklist, check_user_blacklist, check_whitelist, normalize_command};
pub use store::resolve_policy_paths;
pub use types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellDirective,
    ShellExpansionError, ShellExpansionOptions, ShellPolicyPaths, ShellRuleSet,
    ShellExpansionRuntime,
};

use crate::markdown::transform::TransformOptions;

/// Executes a shell directive with policy enforcement and approval flow.
///
/// ## Returns
///
/// The command's stdout+stderr output if successful.
///
/// ## Errors
///
/// - `Blacklisted` if the command matches built-in or user blacklist
/// - `ApprovalRequired` if approval handler is not set
/// - `Denied` if approval handler denies the command
/// - Propagates errors from `execute_command`
///
/// ## Examples
///
/// ```no_run
/// use darkmatter::markdown::transform::shell_expansion::{execute_directive, types::{ShellDirective, ShellExpansionRuntime, ShellPolicyPaths}};
/// use darkmatter::markdown::transform::TransformOptions;
/// use std::path::PathBuf;
///
/// let directive = ShellDirective {
///     raw_command: "echo hello".to_string(),
///     executable: "echo".to_string(),
///     args: vec!["hello".to_string()],
///     span: 0..10,
///     line: 1,
/// };
/// let options = TransformOptions::new();
/// let policy_paths = ShellPolicyPaths {
///     whitelist: PathBuf::from("/tmp/whitelist"),
///     blacklist: PathBuf::from("/tmp/blacklist"),
/// };
/// let mut runtime = ShellExpansionRuntime::new();
/// // let output = execute_directive(&directive, &options, &policy_paths, &mut runtime).unwrap();
/// ```
pub fn execute_directive(
    directive: &ShellDirective,
    options: &TransformOptions,
    policy_paths: &ShellPolicyPaths,
    shell_runtime: &mut ShellExpansionRuntime,
) -> Result<String, ShellExpansionError> {
    let normalized = normalize_command(&directive.executable, &directive.args);

    // 1. Check built-in blacklist
    if let Some(reason) = check_builtin_blacklist(&directive.executable, &directive.args) {
        return Err(ShellExpansionError::Blacklisted {
            command: directive.raw_command.clone(),
            reason,
            line: directive.line,
        });
    }

    // 2. Check user blacklist
    if check_user_blacklist(
        &shell_runtime.user_blacklist,
        &directive.executable,
        &directive.args,
        &normalized,
    ) {
        return Err(ShellExpansionError::Blacklisted {
            command: directive.raw_command.clone(),
            reason: "user blacklist".to_string(),
            line: directive.line,
        });
    }

    // 3. Check exact whitelist
    if check_whitelist(&shell_runtime.whitelist, &directive.executable, &normalized) {
        return executor::execute_command(directive, &options.shell, &options.transclusion.source);
    }

    // 4. Check allow-once
    if shell_runtime.allow_once.contains(&normalized) {
        return executor::execute_command(directive, &options.shell, &options.transclusion.source);
    }

    // 5. Request approval or fail
    if let Some(ref handler) = options.shell.approval_handler {
        let request = ShellApprovalRequest {
            source: options.transclusion.source.clone(),
            line: directive.line,
            raw_command: directive.raw_command.clone(),
            executable: directive.executable.clone(),
            args: directive.args.clone(),
            normalized_exact: normalized.clone(),
            whitelist_path: policy_paths.whitelist.clone(),
            blacklist_path: policy_paths.blacklist.clone(),
        };

        match handler.approve(request)? {
            ShellApprovalDecision::AllowExactPersist => {
                store::append_whitelist_exact(policy_paths, &normalized)?;
                shell_runtime.whitelist.entries.push(types::ShellRuleEntry::Exact(normalized));
                executor::execute_command(directive, &options.shell, &options.transclusion.source)
            }
            ShellApprovalDecision::AllowCommandPersist => {
                store::append_whitelist_prefix(policy_paths, &directive.executable)?;
                shell_runtime.whitelist.entries.push(types::ShellRuleEntry::Prefix(directive.executable.clone()));
                executor::execute_command(directive, &options.shell, &options.transclusion.source)
            }
            ShellApprovalDecision::AllowOnce => {
                shell_runtime.allow_once.insert(normalized);
                shell_runtime.approvals_used += 1;
                executor::execute_command(directive, &options.shell, &options.transclusion.source)
            }
            ShellApprovalDecision::Deny => {
                Err(ShellExpansionError::Denied {
                    command: directive.raw_command.clone(),
                    line: directive.line,
                })
            }
            ShellApprovalDecision::BlacklistPersist => {
                store::append_blacklist_exact(policy_paths, &normalized)?;
                shell_runtime.user_blacklist.entries.push(types::ShellRuleEntry::Exact(normalized));
                Err(ShellExpansionError::Blacklisted {
                    command: directive.raw_command.clone(),
                    reason: "user blacklisted".to_string(),
                    line: directive.line,
                })
            }
        }
    } else {
        Err(ShellExpansionError::ApprovalRequired {
            command: directive.raw_command.clone(),
            whitelist_path: policy_paths.whitelist.clone(),
            blacklist_path: policy_paths.blacklist.clone(),
            line: directive.line,
        })
    }
}

/// Applies string replacements from highest byte offset to lowest.
///
/// This ensures that earlier replacements don't invalidate the byte offsets
/// of later replacements.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::transform::shell_expansion::apply_replacements_in_reverse;
///
/// let mut content = "::shell echo hello\n::shell pwd\n".to_string();
/// let replacements = vec![
///     (0..19, "hello\n".to_string()),
///     (19..31, "/tmp\n".to_string()),
/// ];
/// apply_replacements_in_reverse(&mut content, replacements);
/// assert_eq!(content, "hello\n/tmp\n");
/// ```
pub fn apply_replacements_in_reverse(
    content: &mut String,
    mut replacements: Vec<(std::ops::Range<usize>, String)>,
) {
    replacements.sort_by(|a, b| b.0.start.cmp(&a.0.start));
    for (span, replacement) in replacements {
        content.replace_range(span, &replacement);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::markdown::Markdown;
    use crate::markdown::transform::{Stage1Stages, TransformOptions, TransformSource};
    use std::sync::Arc;
    use tempfile::TempDir;

    struct MockApprovalHandler {
        decision: ShellApprovalDecision,
    }

    impl ShellApprovalHandler for MockApprovalHandler {
        fn approve(
            &self,
            _request: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, ShellExpansionError> {
            Ok(self.decision.clone())
        }
    }

    #[test]
    fn pipeline_replaces_shell_directive_with_output() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell echo hello\nSome text\n";
        let mut md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_stages(Stage1Stages {
                shell_expansion: true,
                ..Stage1Stages::none()
            })
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            })
;

        let (transformed, report) = md.transform_with(options).unwrap();

        assert!(transformed.content().contains("hello"));
        assert!(!transformed.content().contains("::shell"));
        assert_eq!(report.shell_expansions_applied, 1);
        assert_eq!(report.shell_approvals_used, 1);
    }

    #[test]
    fn pipeline_ignores_directive_in_code_block() {
        let temp_dir = TempDir::new().unwrap();
        let content = r#"
# Test
::shell echo outside

```bash
::shell echo inside
```
"#;
        let mut md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_stages(Stage1Stages {
                shell_expansion: true,
                ..Stage1Stages::none()
            })
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            })
;

        let (transformed, report) = md.transform_with(options).unwrap();

        // Only the "outside" directive should be replaced
        assert!(transformed.content().contains("outside"));
        assert!(transformed.content().contains("::shell echo inside")); // Still in code block
        assert_eq!(report.shell_expansions_applied, 1);
    }

    #[test]
    fn pipeline_fails_with_blacklisted_command() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell rm -rf /\n";
        let mut md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_stages(Stage1Stages {
                shell_expansion: true,
                ..Stage1Stages::none()
            })
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            })
;

        let result = md.transform_with(options);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Blacklisted") || err.to_string().contains("dangerous"));
    }

    #[test]
    fn pipeline_fails_without_approval_handler() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell echo hello\n";
        let mut md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_stages(Stage1Stages {
                shell_expansion: true,
                ..Stage1Stages::none()
            })
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: None,
                ..Default::default()
            })
;

        let result = md.transform_with(options);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Approval required"));
    }

    #[test]
    fn pipeline_uses_whitelist() {
        let temp_dir = TempDir::new().unwrap();
        let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
        std::fs::write(&whitelist_path, "prefix echo\n").unwrap();

        let content = "# Test\n::shell echo hello\n";
        let mut md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_stages(Stage1Stages {
                shell_expansion: true,
                ..Stage1Stages::none()
            })
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: None, // No handler needed - whitelisted
                ..Default::default()
            })
;

        let (transformed, report) = md.transform_with(options).unwrap();

        assert!(transformed.content().contains("hello"));
        assert_eq!(report.shell_expansions_applied, 1);
        assert_eq!(report.shell_approvals_used, 0); // No approval needed
    }

    #[test]
    fn pipeline_report_counts_are_correct() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell echo hello\n::shell echo world\n";
        let mut md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_stages(Stage1Stages {
                shell_expansion: true,
                ..Stage1Stages::none()
            })
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            })
;

        let (transformed, report) = md.transform_with(options).unwrap();

        assert!(transformed.content().contains("hello"));
        assert!(transformed.content().contains("world"));
        assert_eq!(report.shell_expansions_applied, 2);
        assert_eq!(report.shell_approvals_used, 2);
    }
}
