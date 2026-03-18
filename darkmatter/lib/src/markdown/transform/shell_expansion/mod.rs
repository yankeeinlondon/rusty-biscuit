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

pub mod alias;
pub mod executor;
pub mod parser;
pub mod policy;
pub mod store;
pub mod tokenize;
pub mod types;

pub use alias::{ResolvedAlias, resolve_alias};
pub use executor::{execute_command, resolve_working_directory};
pub use parser::parse_directives;
pub use policy::{
    check_builtin_blacklist, check_user_blacklist, check_whitelist, normalize_command,
};
pub use store::resolve_policy_paths;
pub use types::{
    ErrorHandling, ErrorHandlingOutcome, ShellApprovalDecision, ShellApprovalHandler,
    ShellApprovalRequest, ShellDirective, ShellExpansionError, ShellExpansionOptions,
    ShellExpansionRuntime, ShellPolicyPaths, ShellRuleSet,
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
/// use darkmatter::markdown::transform::shell_expansion::{execute_directive, types::{ErrorHandling, ShellDirective, ShellExpansionRuntime, ShellPolicyPaths}};
/// use darkmatter::markdown::transform::TransformOptions;
/// use std::path::PathBuf;
///
/// let directive = ShellDirective {
///     raw_command: "echo hello".to_string(),
///     executable: "echo".to_string(),
///     args: vec!["hello".to_string()],
///     span: 0..10,
///     line: 1,
///     error_handling: ErrorHandling::default(),
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
    // Resolve alias if the executable is not found on PATH
    let (effective, alias_name) = resolve_or_passthrough(directive);

    let normalized = normalize_command(&effective.executable, &effective.args);

    // 1. Check built-in blacklist (against resolved command)
    if let Some(reason) = check_builtin_blacklist(&effective.executable, &effective.args) {
        return Err(ShellExpansionError::Blacklisted {
            command: display_command(directive, alias_name.as_deref()),
            reason,
            line: directive.line,
        });
    }

    // 2. Check user blacklist (against resolved command)
    if check_user_blacklist(
        &shell_runtime.user_blacklist,
        &effective.executable,
        &effective.args,
        &normalized,
    ) {
        return Err(ShellExpansionError::Blacklisted {
            command: display_command(directive, alias_name.as_deref()),
            reason: "user blacklist".to_string(),
            line: directive.line,
        });
    }

    // 3. Check whitelist (against resolved command)
    if check_whitelist(&shell_runtime.whitelist, &effective.executable, &normalized) {
        return execute_and_handle_errors(
            &effective,
            options,
            &directive.error_handling,
        );
    }

    // 4. Check allow-once
    if shell_runtime.allow_once.contains(&normalized) {
        return execute_and_handle_errors(
            &effective,
            options,
            &directive.error_handling,
        );
    }

    // 5. Request approval or fail
    if let Some(ref handler) = options.shell.approval_handler {
        let request = ShellApprovalRequest {
            source: options.transclusion.source.clone(),
            line: directive.line,
            raw_command: effective.raw_command.clone(),
            executable: effective.executable.clone(),
            args: effective.args.clone(),
            normalized_exact: normalized.clone(),
            whitelist_path: policy_paths.whitelist.clone(),
            blacklist_path: policy_paths.blacklist.clone(),
            alias_name: alias_name.clone(),
        };

        match handler.approve(request)? {
            ShellApprovalDecision::AllowExactPersist => {
                store::append_whitelist_exact(policy_paths, &normalized)?;
                shell_runtime
                    .whitelist
                    .entries
                    .push(types::ShellRuleEntry::Exact(normalized));
                execute_and_handle_errors(&effective, options, &directive.error_handling)
            }
            ShellApprovalDecision::AllowCommandPersist => {
                store::append_whitelist_prefix(policy_paths, &effective.executable)?;
                shell_runtime
                    .whitelist
                    .entries
                    .push(types::ShellRuleEntry::Prefix(effective.executable.clone()));
                execute_and_handle_errors(&effective, options, &directive.error_handling)
            }
            ShellApprovalDecision::AllowOnce => {
                shell_runtime.allow_once.insert(normalized);
                shell_runtime.approvals_used += 1;
                execute_and_handle_errors(&effective, options, &directive.error_handling)
            }
            ShellApprovalDecision::Deny => Err(ShellExpansionError::Denied {
                command: display_command(directive, alias_name.as_deref()),
                line: directive.line,
            }),
            ShellApprovalDecision::BlacklistPersist => {
                store::append_blacklist_exact(policy_paths, &normalized)?;
                shell_runtime
                    .user_blacklist
                    .entries
                    .push(types::ShellRuleEntry::Exact(normalized));
                Err(ShellExpansionError::Blacklisted {
                    command: display_command(directive, alias_name.as_deref()),
                    reason: "user blacklisted".to_string(),
                    line: directive.line,
                })
            }
        }
    } else {
        Err(ShellExpansionError::ApprovalRequired {
            command: display_command(directive, alias_name.as_deref()),
            whitelist_path: policy_paths.whitelist.clone(),
            blacklist_path: policy_paths.blacklist.clone(),
            line: directive.line,
        })
    }
}

/// Executes a command and applies error handling rules to `ExecutionFailed` errors.
///
/// If the directive has error handling options and the command fails with a
/// non-zero exit code, the error may be suppressed (replaced with text) or
/// enriched with additional context.
fn execute_and_handle_errors(
    effective: &ShellDirective,
    options: &TransformOptions,
    error_handling: &types::ErrorHandling,
) -> Result<String, ShellExpansionError> {
    let result = executor::execute_command(effective, &options.shell, &options.transclusion.source);

    // Fast path: no error handling configured or command succeeded
    if error_handling.is_empty() || result.is_ok() {
        return result;
    }

    // Apply error handling rules to ExecutionFailed errors
    match result {
        Err(ShellExpansionError::ExecutionFailed {
            ref code,
            ref stderr,
            ..
        }) => {
            let outcome = error_handling.resolve(*code, stderr);
            match outcome {
                types::ErrorHandlingOutcome::Replace(text) => Ok(text),
                types::ErrorHandlingOutcome::Enrich(enrichment) => {
                    // Re-construct the error with enrichment appended to stderr
                    match result {
                        Err(ShellExpansionError::ExecutionFailed {
                            command,
                            code,
                            stdout,
                            stderr,
                            line,
                        }) => {
                            let enriched_stderr = if stderr.is_empty() {
                                enrichment
                            } else {
                                format!("{stderr}\n{enrichment}")
                            };
                            Err(ShellExpansionError::ExecutionFailed {
                                command,
                                code,
                                stdout,
                                stderr: enriched_stderr,
                                line,
                            })
                        }
                        _ => unreachable!(),
                    }
                }
                types::ErrorHandlingOutcome::Propagate => result,
            }
        }
        // Non-ExecutionFailed errors (Timeout, CommandNotFound, etc.) are not handled
        _ => result,
    }
}

/// Resolves a directive's executable, returning the effective directive and
/// an optional alias name if resolution occurred.
///
/// If the executable is already on PATH, returns the original directive.
/// If it resolves as a shell alias, returns a new directive with the resolved
/// command and merged arguments.
fn resolve_or_passthrough(directive: &ShellDirective) -> (ShellDirective, Option<String>) {
    // If the executable is on PATH, use as-is
    if which::which(&directive.executable).is_ok() {
        return (directive.clone(), None);
    }

    // Try to resolve as a shell alias
    if let Some(resolved) = alias::resolve_alias(&directive.executable) {
        let mut merged_args = resolved.args;
        merged_args.extend_from_slice(&directive.args);

        let raw = if directive.args.is_empty() {
            resolved.definition.clone()
        } else {
            format!("{} {}", resolved.definition, directive.args.join(" "))
        };

        let effective = ShellDirective {
            raw_command: raw,
            executable: resolved.executable,
            args: merged_args,
            span: directive.span.clone(),
            line: directive.line,
            error_handling: directive.error_handling.clone(),
        };

        return (effective, Some(resolved.alias_name));
    }

    // Not found and not an alias — will fail later at execute_command
    (directive.clone(), None)
}

/// Formats a command for display in error messages, including alias info.
fn display_command(directive: &ShellDirective, alias_name: Option<&str>) -> String {
    match alias_name {
        Some(name) => format!("{} (alias: {})", directive.raw_command, name),
        None => directive.raw_command.clone(),
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
    use crate::markdown::transform::{Stage1Stages, TransformOptions};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
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

    struct CountingApprovalHandler {
        decision: ShellApprovalDecision,
        approvals: AtomicUsize,
    }

    impl CountingApprovalHandler {
        fn new(decision: ShellApprovalDecision) -> Self {
            Self {
                decision,
                approvals: AtomicUsize::new(0),
            }
        }

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
            Ok(self.decision.clone())
        }
    }

    #[test]
    fn pipeline_replaces_shell_directive_with_output() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell echo hello\nSome text\n";
        let md: Markdown = content.into();

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
            });

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
        let md: Markdown = content.into();

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
            });

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
        let md: Markdown = content.into();

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
            });

        let result = md.transform_with(options);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Blacklisted") || err.to_string().contains("dangerous"));
    }

    #[test]
    fn pipeline_fails_without_approval_handler() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell echo hello\n";
        let md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_stages(Stage1Stages {
                shell_expansion: true,
                ..Stage1Stages::none()
            })
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: None,
                ..Default::default()
            });

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
        let md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_stages(Stage1Stages {
                shell_expansion: true,
                ..Stage1Stages::none()
            })
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: None, // No handler needed - whitelisted
                ..Default::default()
            });

        let (transformed, report) = md.transform_with(options).unwrap();

        assert!(transformed.content().contains("hello"));
        assert_eq!(report.shell_expansions_applied, 1);
        assert_eq!(report.shell_approvals_used, 0); // No approval needed
    }

    #[test]
    fn pipeline_report_counts_are_correct() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell echo hello\n::shell echo world\n";
        let md: Markdown = content.into();

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
            });

        let (transformed, report) = md.transform_with(options).unwrap();

        assert!(transformed.content().contains("hello"));
        assert!(transformed.content().contains("world"));
        assert_eq!(report.shell_expansions_applied, 2);
        assert_eq!(report.shell_approvals_used, 2);
    }

    #[test]
    fn pipeline_interpolation_feeds_into_shell_expansion() {
        let temp_dir = TempDir::new().unwrap();
        let content = r#"---
name: world
---
# Test
::shell echo {{ name }}
"#;
        let md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_stages(Stage1Stages {
                interpolation: true,
                shell_expansion: true,
                ..Stage1Stages::none()
            })
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (transformed, report) = md.transform_with(options).unwrap();

        // The {{ name }} should be interpolated to "world" before shell execution
        assert!(transformed.content().contains("world"));
        assert!(!transformed.content().contains("::shell"));
        assert!(!transformed.content().contains("{{ name }}"));
        assert_eq!(report.shell_expansions_applied, 1);
    }

    #[test]
    fn allow_once_persists_across_recursive_transclusion() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().join("root.md");
        let child = temp_dir.path().join("child.md");

        std::fs::write(&root, "# Root\n\n::shell echo hello\n\n::file ./child.md\n").unwrap();
        std::fs::write(&child, "## Child\n\n::shell echo hello\n").unwrap();

        let handler = Arc::new(CountingApprovalHandler::new(
            ShellApprovalDecision::AllowOnce,
        ));
        let options = TransformOptions::new()
            .with_source_file(&root)
            .with_stages(Stage1Stages {
                shell_expansion: true,
                ..Stage1Stages::default()
            })
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(handler.clone()),
                ..Default::default()
            });

        let (transformed, report) = Markdown::try_from(root.as_path())
            .unwrap()
            .transform_with(options)
            .unwrap();

        assert_eq!(handler.approvals(), 1);
        assert_eq!(report.shell_approvals_used, 1);
        assert_eq!(transformed.content().matches("hello").count(), 2);
    }

    /// Regression test: when the source file is a bare filename (no directory
    /// component), execution must still succeed — the previous bug set current_dir
    /// to an empty string, causing spawn to fail with exit -1.
    #[test]
    fn pipeline_works_with_bare_filename_source() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        std::fs::write(&file_path, "# Test\n::shell echo hello\n").unwrap();

        // Use a bare filename as the source (not an absolute path)
        let md = Markdown::try_from(file_path.as_path()).unwrap();

        let options = TransformOptions::new()
            .with_source_file(&file_path)
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
            });

        let (transformed, report) = md.transform_with(options).unwrap();
        assert!(transformed.content().contains("hello"));
        assert_eq!(report.shell_expansions_applied, 1);
    }

    /// Denied commands produce a hard error, not a warning.
    #[test]
    fn pipeline_denied_command_produces_error() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell echo hello\n";
        let md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_stages(Stage1Stages {
                shell_expansion: true,
                ..Stage1Stages::none()
            })
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::Deny,
                })),
                ..Default::default()
            });

        let err = md.transform_with(options).unwrap_err();
        assert!(err.to_string().contains("denied") || err.to_string().contains("Denied"));
    }

    /// AllowExactPersist writes the normalized command to the whitelist file.
    #[test]
    fn pipeline_allow_exact_persist_writes_whitelist() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell echo hello\n";
        let md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_stages(Stage1Stages {
                shell_expansion: true,
                ..Stage1Stages::none()
            })
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowExactPersist,
                })),
                ..Default::default()
            });

        let (transformed, _) = md.transform_with(options).unwrap();
        assert!(transformed.content().contains("hello"));

        let whitelist =
            std::fs::read_to_string(temp_dir.path().join(".darkmatter-shell-whitelist")).unwrap();
        assert!(whitelist.contains("exact echo hello"));
    }

    /// BlacklistPersist writes to the blacklist and errors.
    #[test]
    fn pipeline_blacklist_persist_writes_and_errors() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell echo hello\n";
        let md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_stages(Stage1Stages {
                shell_expansion: true,
                ..Stage1Stages::none()
            })
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::BlacklistPersist,
                })),
                ..Default::default()
            });

        let err = md.transform_with(options).unwrap_err();
        assert!(err.to_string().contains("Blacklisted") || err.to_string().contains("blacklist"));

        let blacklist =
            std::fs::read_to_string(temp_dir.path().join(".darkmatter-shell-blacklist")).unwrap();
        assert!(blacklist.contains("exact echo hello"));
    }

    /// Empty command output removes the directive line entirely.
    #[test]
    fn pipeline_empty_output_removes_directive() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell true\nAfter\n";
        let md: Markdown = content.into();

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
            });

        let (transformed, report) = md.transform_with(options).unwrap();
        assert!(!transformed.content().contains("::shell"));
        assert!(transformed.content().contains("After"));
        assert_eq!(report.shell_expansions_applied, 1);
    }

    #[test]
    fn shell_errors_remain_hard_failures_when_fail_fast_is_false() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell rm -rf /\n";
        let md: Markdown = content.into();

        let options = TransformOptions::new()
            .with_fail_fast(false)
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
            });

        let err = md.transform_with(options).unwrap_err();
        assert!(err.to_string().contains("Blacklisted") || err.to_string().contains("dangerous"));
    }

    /// `--when-error` suppresses non-zero exit and replaces with fallback text.
    #[test]
    fn when_error_replaces_failed_command_with_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell --when-error \"fallback\" false\nAfter\n";
        let md: Markdown = content.into();

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
            });

        let (transformed, report) = md.transform_with(options).unwrap();
        assert!(transformed.content().contains("fallback"));
        assert!(!transformed.content().contains("::shell"));
        assert!(transformed.content().contains("After"));
        assert_eq!(report.shell_expansions_applied, 1);
    }

    /// `--when-error` does not interfere with successful commands.
    #[test]
    fn when_error_does_not_affect_successful_commands() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell --when-error \"fallback\" echo success\n";
        let md: Markdown = content.into();

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
            });

        let (transformed, _) = md.transform_with(options).unwrap();
        assert!(transformed.content().contains("success"));
        assert!(!transformed.content().contains("fallback"));
    }

    /// `--when-exit-code` only matches the specified exit code.
    #[test]
    fn when_exit_code_matches_specific_code() {
        let temp_dir = TempDir::new().unwrap();
        let python = find_test_python();
        let Some(ref python) = python else { return };
        let content = format!(
            "::shell --when-exit-code 42 \"caught 42\" {} -c \"import sys; sys.exit(42)\"\n",
            python
        );
        let md: Markdown = content.as_str().into();

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
            });

        let (transformed, _) = md.transform_with(options).unwrap();
        assert!(transformed.content().contains("caught 42"));
    }

    /// `--when-exit-code` does not match a different code, so the error propagates.
    #[test]
    fn when_exit_code_does_not_match_wrong_code() {
        let temp_dir = TempDir::new().unwrap();
        let python = find_test_python();
        let Some(ref python) = python else { return };
        let content = format!(
            "::shell --when-exit-code 99 \"caught\" {} -c \"import sys; sys.exit(1)\"\n",
            python
        );
        let md: Markdown = content.as_str().into();

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
            });

        let result = md.transform_with(options);
        assert!(result.is_err());
    }

    /// `--except-exit-code` catches all codes except the specified one.
    #[test]
    fn except_exit_code_catches_other_codes() {
        let temp_dir = TempDir::new().unwrap();
        let python = find_test_python();
        let Some(ref python) = python else { return };
        // Exit code 1 is NOT 99, so it should be caught
        let content = format!(
            "::shell --except-exit-code 99 \"caught\" {} -c \"import sys; sys.exit(1)\"\n",
            python
        );
        let md: Markdown = content.as_str().into();

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
            });

        let (transformed, _) = md.transform_with(options).unwrap();
        assert!(transformed.content().contains("caught"));
    }

    /// `--enrich-error` adds context to the error message.
    #[test]
    fn enrich_error_adds_context_to_failure() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell --enrich-error \"Check PATH\" false\n";
        let md: Markdown = content.into();

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
            });

        let err = md.transform_with(options).unwrap_err();
        // The enrichment should appear in the error chain
        assert!(err.to_string().contains("failed") || err.to_string().contains("exit"));
    }

    /// `--stderr-contains` replaces when stderr matches.
    #[test]
    fn stderr_contains_replaces_on_match() {
        let temp_dir = TempDir::new().unwrap();
        let python = find_test_python();
        let Some(ref python) = python else { return };
        let content = format!(
            "::shell --stderr-contains \"warn\" \"warnings found\" {} -c \"import sys; sys.stderr.write('warning: something'); sys.exit(1)\"\n",
            python
        );
        let md: Markdown = content.as_str().into();

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
            });

        let (transformed, _) = md.transform_with(options).unwrap();
        assert!(transformed.content().contains("warnings found"));
    }

    /// `--stderr-lacks` replaces when stderr does NOT contain the string.
    #[test]
    fn stderr_lacks_replaces_when_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let python = find_test_python();
        let Some(ref python) = python else { return };
        // stderr says "error" but we're checking for lack of "fatal"
        let content = format!(
            "::shell --stderr-lacks \"fatal\" \"non-fatal\" {} -c \"import sys; sys.stderr.write('error: minor'); sys.exit(1)\"\n",
            python
        );
        let md: Markdown = content.as_str().into();

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
            });

        let (transformed, _) = md.transform_with(options).unwrap();
        assert!(transformed.content().contains("non-fatal"));
    }

    /// Helper to find python3 for integration tests.
    fn find_test_python() -> Option<String> {
        ["python3", "python"]
            .into_iter()
            .find_map(|candidate| {
                which::which(candidate)
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
            })
    }
}
