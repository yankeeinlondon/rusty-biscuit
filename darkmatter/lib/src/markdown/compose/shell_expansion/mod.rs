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
pub mod discovery;
pub mod executor;
pub mod parser;
pub mod policy;
pub mod store;
pub mod tokenize;
pub mod types;

pub use alias::{ResolvedAlias, resolve_alias};
pub use discovery::collect_shell_commands;
pub use executor::{execute_command, resolve_working_directory};
pub use parser::parse_directives;
pub use policy::{
    check_builtin_blacklist, check_user_blacklist, check_whitelist, normalize_command,
};
pub use store::resolve_policy_paths;
pub use types::{
    ErrorHandling, ErrorHandlingOutcome, ShellApprovalDecision, ShellApprovalHandler,
    ShellApprovalRequest, ShellCommandEntry, ShellCommandOrigin, ShellDirective,
    ShellExpansionError, ShellExpansionOptions, ShellExpansionRuntime, ShellPolicyPaths,
    ShellRuleSet, ShellTimeoutBehavior,
};

use crate::markdown::compose::ComposeOptions;
use crate::markdown::compose::types::ComposeWarning;

/// A directive that has passed alias resolution, policy checks, and approval.
#[derive(Debug, Clone)]
pub(crate) struct PreparedShellDirective {
    pub effective: ShellDirective,
    pub display_command: String,
}

/// Detailed output from executing a prepared shell directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectiveExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub warnings: Vec<ComposeWarning>,
}

impl DirectiveExecutionResult {
    /// Returns stdout and stderr combined using the body-shell contract.
    pub fn combined_output(&self) -> String {
        let mut output = self.stdout.clone();
        if !self.stderr.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&self.stderr);
        }
        output
    }
}

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
/// use darkmatter::markdown::compose::shell_expansion::{execute_directive, types::{ErrorHandling, ShellCommandOrigin, ShellDirective, ShellExpansionRuntime, ShellPolicyPaths}};
/// use darkmatter::markdown::compose::ComposeOptions;
/// use std::path::PathBuf;
///
/// let directive = ShellDirective {
///     raw_command: "echo hello".to_string(),
///     executable: "echo".to_string(),
///     args: vec!["hello".to_string()],
///     span: 0..10,
///     origin: ShellCommandOrigin::Body { line: 1 },
///     error_handling: ErrorHandling::default(),
///     timeout_override: None,
///     pipeline: None,
/// };
/// let options = ComposeOptions::new();
/// let policy_paths = ShellPolicyPaths {
///     whitelist: PathBuf::from("/tmp/whitelist"),
///     blacklist: PathBuf::from("/tmp/blacklist"),
/// };
/// let mut runtime = ShellExpansionRuntime::new();
/// // let output = execute_directive(&directive, &options, &policy_paths, &mut runtime).unwrap();
/// ```
pub fn execute_directive(
    directive: &ShellDirective,
    options: &ComposeOptions,
    policy_paths: &ShellPolicyPaths,
    shell_runtime: &mut ShellExpansionRuntime,
) -> Result<String, ShellExpansionError> {
    Ok(
        execute_directive_detailed(directive, options, policy_paths, shell_runtime)?
            .combined_output(),
    )
}

/// Executes a shell directive with policy enforcement and returns stdout,
/// stderr, and any non-fatal warnings.
pub(crate) fn execute_directive_detailed(
    directive: &ShellDirective,
    options: &ComposeOptions,
    policy_paths: &ShellPolicyPaths,
    shell_runtime: &mut ShellExpansionRuntime,
) -> Result<DirectiveExecutionResult, ShellExpansionError> {
    let prepared = prepare_directive(directive, options, policy_paths, shell_runtime)?;
    execute_prepared_directive(&prepared, options)
}

/// Resolves aliases, applies policy checks, and records approval decisions
/// without executing the command yet.
pub(crate) fn prepare_directive(
    directive: &ShellDirective,
    options: &ComposeOptions,
    policy_paths: &ShellPolicyPaths,
    shell_runtime: &mut ShellExpansionRuntime,
) -> Result<PreparedShellDirective, ShellExpansionError> {
    let (effective, alias_name) = resolve_or_passthrough(directive);
    let display_command = display_command(directive, alias_name.as_deref());

    // Collect all normalized commands from the pipeline (or single command)
    let normalized_commands = collect_normalized_commands(&effective);

    // ── Pre-approved fast path ───────────────────────────────────────
    if let Some(ref approved) = options.pre_approved_commands {
        for normalized in &normalized_commands {
            if !approved.contains(normalized) {
                return Err(ShellExpansionError::NotPreApproved {
                    command: display_command.clone(),
                    origin: directive.origin.clone(),
                    source_desc: match &options.source {
                        crate::markdown::compose::ComposeSource::File(p) => {
                            format!(" (in {})", p.display())
                        }
                        _ => String::new(),
                    },
                });
            }
        }
        return Ok(PreparedShellDirective {
            effective,
            display_command,
        });
    }

    let runtime_snapshot = shell_runtime.snapshot();

    // 1. Check built-in blacklist for all commands
    for (exe, args) in executables_with_args(&effective) {
        if let Some(reason) = check_builtin_blacklist(&exe, &args) {
            return Err(ShellExpansionError::Blacklisted {
                command: display_command.clone(),
                reason,
                origin: directive.origin.clone(),
            });
        }
    }

    // 2. Check user blacklist for all commands
    for (i, normalized) in normalized_commands.iter().enumerate() {
        let (exe, args) = executable_and_args_at(&effective, i);
        if check_user_blacklist(
            &runtime_snapshot.user_blacklist,
            &exe,
            &args,
            normalized,
        ) {
            return Err(ShellExpansionError::Blacklisted {
                command: display_command.clone(),
                reason: "user blacklist".to_string(),
                origin: directive.origin.clone(),
            });
        }
    }

    // 3. Check whitelist for all commands
    let all_whitelisted = normalized_commands.iter().enumerate().all(|(i, normalized)| {
        let (exe, _) = executable_and_args_at(&effective, i);
        check_whitelist(&runtime_snapshot.whitelist, &exe, normalized)
            || runtime_snapshot.allow_once.contains(normalized)
    });

    if all_whitelisted {
        return Ok(PreparedShellDirective {
            effective,
            display_command,
        });
    }

    // 5. Request approval for the entire chain
    if let Some(ref handler) = options.shell_approval_handler {
        // Reserve all commands for allow-once handling before calling the handler
        let mut all_reserved = true;
        for normalized in &normalized_commands {
            if !shell_runtime.try_reserve_allow_once(normalized) {
                all_reserved = false;
                break;
            }
        }

        if !all_reserved {
            // Some commands already approved — treat entire chain as approved
            return Ok(PreparedShellDirective {
                effective,
                display_command,
            });
        }

        // Build the unique list of chain executables for the prompt. Empty
        // when the request is a single command.
        let chain_executables = if effective.is_chain() {
            let mut seen = std::collections::HashSet::new();
            executables_with_args(&effective)
                .into_iter()
                .filter_map(|(exe, _)| seen.insert(exe.clone()).then_some(exe))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        // Present the entire chain for approval
        let request = ShellApprovalRequest {
            source: options.source.clone(),
            origin: directive.origin.clone(),
            raw_command: effective.raw_command.clone(),
            executable: effective.executable.clone(),
            args: effective.args.clone(),
            normalized_exact: normalized_commands.join(" && "),
            whitelist_path: policy_paths.whitelist.clone(),
            blacklist_path: policy_paths.blacklist.clone(),
            alias_name: alias_name.clone(),
            chain_executables,
        };

        match handler.approve(request)? {
            ShellApprovalDecision::AllowExactPersist => {
                for normalized in &normalized_commands {
                    shell_runtime.complete_allow_once(normalized, false);
                    store::append_whitelist_exact(policy_paths, normalized)?;
                    shell_runtime.persist_whitelist_exact(normalized.clone());
                }
                Ok(PreparedShellDirective {
                    effective,
                    display_command,
                })
            }
            ShellApprovalDecision::AllowCommandPersist => {
                for normalized in &normalized_commands {
                    shell_runtime.complete_allow_once(normalized, false);
                }
                // Persist a prefix entry for every unique executable in the
                // chain so the next run does not re-prompt for later actions.
                let mut persisted = std::collections::HashSet::new();
                for (exe, _) in executables_with_args(&effective) {
                    if persisted.insert(exe.clone()) {
                        store::append_whitelist_prefix(policy_paths, &exe)?;
                        shell_runtime.persist_whitelist_prefix(exe);
                    }
                }
                Ok(PreparedShellDirective {
                    effective,
                    display_command,
                })
            }
            ShellApprovalDecision::AllowOnce => {
                for normalized in &normalized_commands {
                    shell_runtime.complete_allow_once(normalized, true);
                }
                shell_runtime.approvals_used += 1;
                Ok(PreparedShellDirective {
                    effective,
                    display_command,
                })
            }
            ShellApprovalDecision::Deny => {
                for normalized in &normalized_commands {
                    shell_runtime.complete_allow_once(normalized, false);
                }
                Err(ShellExpansionError::Denied {
                    command: display_command,
                    origin: directive.origin.clone(),
                })
            }
            ShellApprovalDecision::BlacklistPersist => {
                for normalized in &normalized_commands {
                    shell_runtime.complete_allow_once(normalized, false);
                    store::append_blacklist_exact(policy_paths, normalized)?;
                    shell_runtime.persist_blacklist_exact(normalized.clone());
                }
                Err(ShellExpansionError::Blacklisted {
                    command: display_command,
                    reason: "user blacklisted".to_string(),
                    origin: directive.origin.clone(),
                })
            }
        }
    } else {
        Err(ShellExpansionError::ApprovalRequired {
            command: display_command,
            whitelist_path: policy_paths.whitelist.clone(),
            blacklist_path: policy_paths.blacklist.clone(),
            origin: directive.origin.clone(),
        })
    }
}

/// Executes a previously prepared directive and converts timeout fallbacks into
/// compose warnings.
pub(crate) fn execute_prepared_directive(
    prepared: &PreparedShellDirective,
    options: &ComposeOptions,
) -> Result<DirectiveExecutionResult, ShellExpansionError> {
    let execution = execute_and_handle_errors(
        &prepared.effective,
        options,
        &prepared.effective.error_handling,
    )?;

    let mut warnings = Vec::new();
    if let Some(timeout) = execution.timeout_fallback {
        let warning = ComposeWarning::new(
            "shell_expansion",
            format!(
                "Shell command timed out after {timeout:?} at {}: '{}'; replaced with an empty string",
                prepared.effective.origin, prepared.display_command
            ),
        );
        warnings.push(match prepared.effective.origin {
            ShellCommandOrigin::Body { line } => warning.at_line(line),
            ShellCommandOrigin::Frontmatter { .. } => warning,
            ShellCommandOrigin::ShellBlock { command_line, .. } => warning.at_line(command_line),
        });
    }

    Ok(DirectiveExecutionResult {
        stdout: execution.stdout,
        stderr: execution.stderr,
        warnings,
    })
}

/// Executes a command and applies error handling rules to `ExecutionFailed` errors.
///
/// If the directive has error handling options and the command fails with a
/// non-zero exit code, the error may be suppressed (replaced with text) or
/// enriched with additional context.
fn execute_and_handle_errors(
    effective: &ShellDirective,
    options: &ComposeOptions,
    error_handling: &types::ErrorHandling,
) -> Result<executor::CommandExecution, ShellExpansionError> {
    let shell_opts = options.shell_options();
    let result = executor::execute_directive_impl(effective, &shell_opts, &options.source);

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
                types::ErrorHandlingOutcome::Replace(text) => Ok(
                    executor::CommandExecution::from_streams(text, String::new()),
                ),
                types::ErrorHandlingOutcome::Enrich(enrichment) => {
                    // Re-construct the error with enrichment appended to stderr
                    match result {
                        Err(ShellExpansionError::ExecutionFailed {
                            command,
                            code,
                            stdout,
                            stderr,
                            origin,
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
                                origin,
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
    // For pipeline chains, resolve each action's executable independently
    if let Some(ref pipeline) = directive.pipeline
        && (pipeline.actions.len() > 1
            || (pipeline.actions.len() == 1
                && pipeline.actions[0].command.redirection != types::RedirectionConfig::default()))
    {
        let mut any_resolved = false;
        let mut alias_name: Option<String> = None;
        let mut new_pipeline = pipeline.clone();

        for action in &mut new_pipeline.actions {
            if which::which(&action.command.executable).is_ok() {
                continue;
            }
            if let Some(resolved) = alias::resolve_alias(&action.command.executable) {
                let mut merged_args = resolved.args;
                merged_args.extend_from_slice(&action.command.args);
                action.command.executable = resolved.executable;
                action.command.args = merged_args;
                if alias_name.is_none() {
                    alias_name = Some(resolved.alias_name);
                }
                any_resolved = true;
            }
        }

        if any_resolved || alias_name.is_some() {
            let raw = new_pipeline.display_string();
            let exe = new_pipeline.actions[0].command.executable.clone();
            let args = new_pipeline.actions[0].command.args.clone();
            let effective = ShellDirective {
                raw_command: raw,
                executable: exe,
                args,
                span: directive.span.clone(),
                origin: directive.origin.clone(),
                error_handling: directive.error_handling.clone(),
                timeout_override: directive.timeout_override,
                pipeline: Some(new_pipeline),
            };
            return (effective, alias_name);
        }
    }

    // Standard single-command path
    if which::which(&directive.executable).is_ok() {
        return (directive.clone(), None);
    }

    if let Some(resolved) = alias::resolve_alias(&directive.executable) {
        let mut merged_args = resolved.args;
        merged_args.extend_from_slice(&directive.args);

        let raw = if directive.args.is_empty() {
            resolved.definition.clone()
        } else {
            format!("{} {}", resolved.definition, directive.args.join(" "))
        };

        let mut new_pipeline = None;
        if let Some(ref pipeline) = directive.pipeline {
            let mut p = pipeline.clone();
            p.actions[0].command.executable = resolved.executable.clone();
            p.actions[0].command.args = merged_args.clone();
            new_pipeline = Some(p);
        }

        let effective = ShellDirective {
            raw_command: raw,
            executable: resolved.executable,
            args: merged_args,
            span: directive.span.clone(),
            origin: directive.origin.clone(),
            error_handling: directive.error_handling.clone(),
            timeout_override: directive.timeout_override,
            pipeline: new_pipeline,
        };

        return (effective, Some(resolved.alias_name));
    }

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
/// use darkmatter::markdown::compose::shell_expansion::apply_replacements_in_reverse;
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
    replacements.sort_by_key(|b| std::cmp::Reverse(b.0.start));
    for (span, replacement) in replacements {
        content.replace_range(span, &replacement);
    }
}

/// Collects normalized command strings for all actions in the pipeline.
fn collect_normalized_commands(directive: &ShellDirective) -> Vec<String> {
    if let Some(ref pipeline) = directive.pipeline {
        pipeline
            .actions
            .iter()
            .map(|a| normalize_command(&a.command.executable, &a.command.args))
            .collect()
    } else {
        vec![normalize_command(&directive.executable, &directive.args)]
    }
}

/// Returns (executable, args) pairs for all actions in the pipeline.
fn executables_with_args(directive: &ShellDirective) -> Vec<(String, Vec<String>)> {
    if let Some(ref pipeline) = directive.pipeline {
        pipeline
            .actions
            .iter()
            .map(|a| (a.command.executable.clone(), a.command.args.clone()))
            .collect()
    } else {
        vec![(directive.executable.clone(), directive.args.clone())]
    }
}

fn executable_and_args_at(directive: &ShellDirective, index: usize) -> (String, Vec<String>) {
    if let Some(ref pipeline) = directive.pipeline
        && let Some(action) = pipeline.actions.get(index)
    {
        return (action.command.executable.clone(), action.command.args.clone());
    }
    (directive.executable.clone(), directive.args.clone())
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::markdown::Markdown;
    use crate::markdown::compose::{ComposeOperation, ComposeOptions};
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

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("hello"));
        assert!(!composed.content().contains("::shell"));
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

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();

        // Only the "outside" directive should be replaced
        assert!(composed.content().contains("outside"));
        assert!(composed.content().contains("::shell echo inside")); // Still in code block
        assert_eq!(report.shell_expansions_applied, 1);
    }

    #[test]
    fn pipeline_fails_with_blacklisted_command() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell rm -rf /\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let result = md.compose_with(options);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Blacklisted") || err.to_string().contains("dangerous"));
    }

    #[test]
    fn pipeline_fails_without_approval_handler() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell echo hello\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: None,
                ..Default::default()
            });

        let result = md.compose_with(options);
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

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: None, // No handler needed - whitelisted
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("hello"));
        assert_eq!(report.shell_expansions_applied, 1);
        assert_eq!(report.shell_approvals_used, 0); // No approval needed
    }

    #[test]
    fn pipeline_report_counts_are_correct() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell echo hello\n::shell echo world\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("hello"));
        assert!(composed.content().contains("world"));
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

        let options = ComposeOptions::new()
            .only(&[
                ComposeOperation::Interpolation,
                ComposeOperation::ShellExpansion,
            ])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();

        // The {{ name }} should be interpolated to "world" before shell execution
        assert!(composed.content().contains("world"));
        assert!(!composed.content().contains("::shell"));
        assert!(!composed.content().contains("{{ name }}"));
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
        let options =
            ComposeOptions::new()
                .with_source_file(&root)
                .with_shell(ShellExpansionOptions {
                    policy_root: Some(temp_dir.path().to_path_buf()),
                    approval_handler: Some(handler.clone()),
                    ..Default::default()
                });

        let (composed, report) = Markdown::try_from(root.as_path())
            .unwrap()
            .compose_with(options)
            .unwrap();

        assert_eq!(handler.approvals(), 1);
        assert_eq!(report.shell_approvals_used, 1);
        assert_eq!(composed.content().matches("hello").count(), 2);
    }

    #[test]
    fn allow_once_persists_across_sibling_transclusions() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().join("root.md");
        let child_a = temp_dir.path().join("child-a.md");
        let child_b = temp_dir.path().join("child-b.md");

        std::fs::write(&root, "::file ./child-a.md\n\n::file ./child-b.md\n").unwrap();
        std::fs::write(&child_a, "## A\n\n::shell echo hello\n").unwrap();
        std::fs::write(&child_b, "## B\n\n::shell echo hello\n").unwrap();

        let handler = Arc::new(CountingApprovalHandler::new(
            ShellApprovalDecision::AllowOnce,
        ));
        let options =
            ComposeOptions::new()
                .with_source_file(&root)
                .with_shell(ShellExpansionOptions {
                    policy_root: Some(temp_dir.path().to_path_buf()),
                    approval_handler: Some(handler.clone()),
                    ..Default::default()
                });

        let (composed, report) = Markdown::try_from(root.as_path())
            .unwrap()
            .compose_with(options)
            .unwrap();

        assert_eq!(handler.approvals(), 1);
        assert_eq!(report.shell_approvals_used, 1);
        assert_eq!(composed.content().matches("hello").count(), 2);
    }

    #[test]
    fn shell_approval_counts_aggregate_from_multiple_children() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().join("root.md");
        let child_a = temp_dir.path().join("child-a.md");
        let child_b = temp_dir.path().join("child-b.md");

        std::fs::write(&root, "::file ./child-a.md\n\n::file ./child-b.md\n").unwrap();
        std::fs::write(&child_a, "## A\n\n::shell echo alpha\n").unwrap();
        std::fs::write(&child_b, "## B\n\n::shell echo beta\n").unwrap();

        let handler = Arc::new(CountingApprovalHandler::new(
            ShellApprovalDecision::AllowOnce,
        ));
        let options =
            ComposeOptions::new()
                .with_source_file(&root)
                .with_shell(ShellExpansionOptions {
                    policy_root: Some(temp_dir.path().to_path_buf()),
                    approval_handler: Some(handler.clone()),
                    ..Default::default()
                });

        let (_composed, report) = Markdown::try_from(root.as_path())
            .unwrap()
            .compose_with(options)
            .unwrap();

        assert_eq!(handler.approvals(), 2);
        assert_eq!(report.shell_approvals_used, 2);
        assert_eq!(report.shell_expansions_applied, 2);
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

        let options = ComposeOptions::new()
            .with_source_file(&file_path)
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("hello"));
        assert_eq!(report.shell_expansions_applied, 1);
    }

    /// Denied commands produce a hard error, not a warning.
    #[test]
    fn pipeline_denied_command_produces_error() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell echo hello\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::Deny,
                })),
                ..Default::default()
            });

        let err = md.compose_with(options).unwrap_err();
        assert!(err.to_string().contains("denied") || err.to_string().contains("Denied"));
    }

    /// AllowCommandPersist on a chain writes a prefix entry for every unique
    /// executable in the chain so later runs do not re-prompt. Regression for
    /// review-3.
    #[test]
    fn pipeline_allow_command_persist_writes_each_chain_executable() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell echo ok && pwd\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowCommandPersist,
                })),
                ..Default::default()
            });

        let (composed, _) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("ok"));

        let whitelist =
            std::fs::read_to_string(temp_dir.path().join(".darkmatter-shell-whitelist")).unwrap();
        assert!(
            whitelist.contains("prefix echo"),
            "whitelist missing 'prefix echo': {whitelist:?}"
        );
        assert!(
            whitelist.contains("prefix pwd"),
            "whitelist missing 'prefix pwd': {whitelist:?}"
        );
    }

    /// Repeated executables in a chain are only persisted once.
    #[test]
    fn pipeline_allow_command_persist_dedupes_repeated_chain_executables() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell echo a && echo b\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowCommandPersist,
                })),
                ..Default::default()
            });

        let _ = md.compose_with(options).unwrap();

        let whitelist =
            std::fs::read_to_string(temp_dir.path().join(".darkmatter-shell-whitelist")).unwrap();
        let count = whitelist.matches("prefix echo").count();
        assert_eq!(count, 1, "expected exactly one 'prefix echo' line: {whitelist:?}");
    }

    /// The approval request exposes every unique executable in the chain so
    /// the CLI prompt can describe option 2 accurately.
    #[test]
    fn approval_request_exposes_chain_executables() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell echo a && pwd && echo b\n";
        let md: Markdown = content.into();

        let handler = Arc::new(RecordingApprovalHandler::new(
            ShellApprovalDecision::AllowOnce,
        ));

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(handler.clone()),
                ..Default::default()
            });

        let _ = md.compose_with(options).unwrap();

        let requests = handler.requests();
        assert_eq!(requests.len(), 1);
        // Chain is a, pwd, b — unique entries in first-occurrence order.
        assert_eq!(
            requests[0].chain_executables,
            vec!["echo".to_string(), "pwd".to_string()]
        );
    }

    /// Single-command requests leave `chain_executables` empty so the CLI
    /// keeps the original wording.
    #[test]
    fn approval_request_chain_executables_empty_for_single_command() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell echo hi\n";
        let md: Markdown = content.into();

        let handler = Arc::new(RecordingApprovalHandler::new(
            ShellApprovalDecision::AllowOnce,
        ));

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(handler.clone()),
                ..Default::default()
            });

        let _ = md.compose_with(options).unwrap();

        let requests = handler.requests();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].chain_executables.is_empty());
    }

    /// AllowExactPersist writes the normalized command to the whitelist file.
    #[test]
    fn pipeline_allow_exact_persist_writes_whitelist() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell echo hello\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowExactPersist,
                })),
                ..Default::default()
            });

        let (composed, _) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("hello"));

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

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::BlacklistPersist,
                })),
                ..Default::default()
            });

        let err = md.compose_with(options).unwrap_err();
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

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(!composed.content().contains("::shell"));
        assert!(composed.content().contains("After"));
        assert_eq!(report.shell_expansions_applied, 1);
    }

    #[test]
    fn shell_errors_remain_hard_failures_when_fail_fast_is_false() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell rm -rf /\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .with_fail_fast(false)
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let err = md.compose_with(options).unwrap_err();
        assert!(err.to_string().contains("Blacklisted") || err.to_string().contains("dangerous"));
    }

    /// `--when-error` suppresses non-zero exit and replaces with fallback text.
    #[test]
    fn when_error_replaces_failed_command_with_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n::shell --when-error \"fallback\" false\nAfter\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("fallback"));
        assert!(!composed.content().contains("::shell"));
        assert!(composed.content().contains("After"));
        assert_eq!(report.shell_expansions_applied, 1);
    }

    /// `--when-error` does not interfere with successful commands.
    #[test]
    fn when_error_does_not_affect_successful_commands() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell --when-error \"fallback\" echo success\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, _) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("success"));
        assert!(!composed.content().contains("fallback"));
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

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, _) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("caught 42"));
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

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let result = md.compose_with(options);
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

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, _) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("caught"));
    }

    /// `--enrich-error` adds context to the error message.
    #[test]
    fn enrich_error_adds_context_to_failure() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell --enrich-error \"Check PATH\" false\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let err = md.compose_with(options).unwrap_err();
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

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, _) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("warnings found"));
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

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, _) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("non-fatal"));
    }

    #[test]
    fn pre_approved_commands_bypass_approval_flow() {
        let content = "# Test\n::shell echo hello\n::shell echo world\n";
        let md: Markdown = content.into();

        let mut approved = std::collections::HashSet::new();
        approved.insert("echo hello".to_string());
        approved.insert("echo world".to_string());

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_pre_approved_commands(approved);

        // No approval handler, no whitelist — should succeed via pre-approved set
        let (composed, report) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("hello"));
        assert!(composed.content().contains("world"));
        assert_eq!(report.shell_expansions_applied, 2);
        assert_eq!(report.shell_approvals_used, 0);
    }

    #[test]
    fn pre_approved_rejects_unknown_commands() {
        let content = "# Test\n::shell echo hello\n::shell echo sneaky\n";
        let md: Markdown = content.into();

        let mut approved = std::collections::HashSet::new();
        approved.insert("echo hello".to_string());
        // "echo sneaky" is NOT pre-approved

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_pre_approved_commands(approved);

        let err = md.compose_with(options).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not pre-approved"), "got: {msg}");
        assert!(msg.contains("echo sneaky"), "got: {msg}");
    }

    /// Helper to find python3 for integration tests.
    fn find_test_python() -> Option<String> {
        ["python3", "python"].into_iter().find_map(|candidate| {
            which::which(candidate)
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        })
    }

    /// Approval handler that records the requests it receives so tests can
    /// inspect what the user would have seen.
    struct RecordingApprovalHandler {
        decision: ShellApprovalDecision,
        requests: std::sync::Mutex<Vec<ShellApprovalRequest>>,
    }

    impl RecordingApprovalHandler {
        fn new(decision: ShellApprovalDecision) -> Self {
            Self {
                decision,
                requests: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn requests(&self) -> Vec<ShellApprovalRequest> {
            self.requests.lock().unwrap().clone()
        }
    }

    impl ShellApprovalHandler for RecordingApprovalHandler {
        fn approve(
            &self,
            request: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, ShellExpansionError> {
            self.requests.lock().unwrap().push(request);
            Ok(self.decision.clone())
        }
    }

    /// Regression test: when a whitelist contains only a prefix entry for the
    /// FIRST command in a chain, later commands in the same chain must still
    /// require approval. Previously, the check used the first executable for
    /// every action, allowing later commands to slip through.
    #[test]
    fn pipeline_whitelist_prefix_does_not_authorize_later_chain_actions() {
        let temp_dir = TempDir::new().unwrap();
        let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
        std::fs::write(&whitelist_path, "prefix echo\n").unwrap();

        // Whitelist allows `echo`, but `pwd` must trigger approval.
        let content = "::shell echo ok && pwd\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: None, // No approval handler -- must error
                ..Default::default()
            });

        let err = md.compose_with(options).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Approval required") || msg.contains("approval"),
            "expected approval-required error for `pwd`, got: {msg}"
        );
    }

    /// When every action in a chain is independently whitelisted, the chain
    /// runs without prompting for approval.
    #[test]
    fn pipeline_whitelist_authorizes_chain_when_each_action_matches() {
        let temp_dir = TempDir::new().unwrap();
        let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
        std::fs::write(&whitelist_path, "prefix echo\nprefix pwd\n").unwrap();

        let content = "::shell echo ok && pwd\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: None,
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("ok"));
        assert_eq!(report.shell_approvals_used, 0);
    }

    /// Approval request preserves redirection tokens in the displayed command.
    #[test]
    fn approval_request_preserves_redirections() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell echo hidden > /dev/null\n";
        let md: Markdown = content.into();

        let handler = Arc::new(RecordingApprovalHandler::new(
            ShellApprovalDecision::AllowOnce,
        ));

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(handler.clone()),
                ..Default::default()
            });

        let _ = md.compose_with(options).unwrap();

        let requests = handler.requests();
        assert_eq!(requests.len(), 1);
        assert!(
            requests[0].raw_command.contains("> /dev/null"),
            "expected raw_command to include redirection, got: {}",
            requests[0].raw_command
        );
    }

    /// Approval request shows every command in a chain, joined by `&&` for
    /// the normalized form.
    #[test]
    fn approval_request_includes_full_chain() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell echo a && echo b\n";
        let md: Markdown = content.into();

        let handler = Arc::new(RecordingApprovalHandler::new(
            ShellApprovalDecision::AllowOnce,
        ));

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(handler.clone()),
                ..Default::default()
            });

        let _ = md.compose_with(options).unwrap();

        let requests = handler.requests();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].raw_command.contains("&&"));
        assert!(requests[0].normalized_exact.contains("&&"));
        assert!(requests[0].normalized_exact.contains("echo a"));
        assert!(requests[0].normalized_exact.contains("echo b"));
    }

    /// `A && B` runs B when A succeeds; combined output reflects both commands.
    #[test]
    fn chain_and_runs_second_when_first_succeeds() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell echo first && echo second\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, _) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("first"));
        assert!(composed.content().contains("second"));
    }

    /// `A && B` skips B when A fails -- but the chain still propagates the
    /// failure.
    #[test]
    fn chain_and_skips_second_when_first_fails() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell false && echo unreachable\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        // No `||` recovery, so the chain should fail
        let result = md.compose_with(options);
        assert!(result.is_err());
    }

    /// `A || B` runs B when A fails; B's output is rendered.
    #[test]
    fn chain_or_runs_second_when_first_fails() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell false || echo recovered\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, _) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("recovered"));
    }

    /// `A || B` skips B when A succeeds.
    #[test]
    fn chain_or_skips_second_when_first_succeeds() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell echo primary || echo fallback\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, _) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("primary"));
        assert!(!composed.content().contains("fallback"));
    }

    /// Complex `A && B || C` chain: when A fails, C executes (not B).
    #[test]
    fn chain_and_or_runs_recovery_branch() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell false && echo middle || echo tail\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, _) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("tail"));
        assert!(!composed.content().contains("middle"));
    }

    /// `> /dev/null` discards stdout: nothing is captured for substitution.
    #[test]
    fn redirection_stdout_null_drops_captured_output() {
        let temp_dir = TempDir::new().unwrap();
        let content = "Before\n::shell echo silenced > /dev/null\nAfter\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, _) = md.compose_with(options).unwrap();
        assert!(!composed.content().contains("silenced"));
        assert!(composed.content().contains("Before"));
        assert!(composed.content().contains("After"));
    }

    /// `2> /dev/null` suppresses stderr but keeps stdout.
    #[test]
    fn redirection_stderr_null_drops_stderr() {
        let temp_dir = TempDir::new().unwrap();
        let python = find_test_python();
        let Some(ref python) = python else { return };
        let content = format!(
            "::shell {python} -c \"import sys; sys.stdout.write('OUT'); sys.stderr.write('ERR')\" 2> /dev/null\n"
        );
        let md: Markdown = content.as_str().into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, _) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("OUT"));
        assert!(!composed.content().contains("ERR"));
    }

    /// `2>&1` merges stderr into stdout: both streams appear in captured
    /// output.
    #[test]
    fn redirection_stderr_to_stdout_merges_streams() {
        let temp_dir = TempDir::new().unwrap();
        let python = find_test_python();
        let Some(ref python) = python else { return };
        let content = format!(
            "::shell {python} -c \"import sys; sys.stdout.write('OUT'); sys.stderr.write('ERR')\" 2>&1\n"
        );
        let md: Markdown = content.as_str().into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, _) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("OUT"));
        assert!(composed.content().contains("ERR"));
    }

    /// `2>&1` preserves emission order: a child that writes ERR before OUT
    /// must surface as `ERROUT`, not `OUTERR`. Regression for review-3.
    #[test]
    fn redirection_stderr_to_stdout_preserves_emission_order() {
        let temp_dir = TempDir::new().unwrap();
        let python = find_test_python();
        let Some(ref python) = python else { return };
        // Write ERR first, flush, then OUT — both go to the merged pipe.
        let content = format!(
            "::shell {python} -c \"import sys; sys.stderr.write('ERR'); sys.stderr.flush(); sys.stdout.write('OUT'); sys.stdout.flush()\" 2>&1\n"
        );
        let md: Markdown = content.as_str().into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, _) = md.compose_with(options).unwrap();
        // The merged stream must contain ERR before OUT in source order.
        let body = composed.content();
        let err_pos = body.find("ERR").expect("expected ERR in merged output");
        let out_pos = body.find("OUT").expect("expected OUT in merged output");
        assert!(
            err_pos < out_pos,
            "expected ERR to precede OUT under 2>&1, got: {body:?}"
        );
    }

    /// `>&2` preserves emission order in the merged stream and the body-shell
    /// contract still surfaces both pieces.
    #[test]
    fn redirection_stdout_to_stderr_preserves_emission_order() {
        let temp_dir = TempDir::new().unwrap();
        let python = find_test_python();
        let Some(ref python) = python else { return };
        let content = format!(
            "::shell {python} -c \"import sys; sys.stdout.write('A'); sys.stdout.flush(); sys.stderr.write('B'); sys.stderr.flush()\" >&2\n"
        );
        let md: Markdown = content.as_str().into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, _) = md.compose_with(options).unwrap();
        let body = composed.content();
        let a_pos = body.find('A').expect("expected A in merged output");
        let b_pos = body.find('B').expect("expected B in merged output");
        assert!(
            a_pos < b_pos,
            "expected A to precede B under >&2, got: {body:?}"
        );
    }

    /// `>&2` routes stdout to stderr; combined output still surfaces it via
    /// the body shell contract that concatenates stdout and stderr.
    #[test]
    fn redirection_stdout_to_stderr_routes_output() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell echo routed >&2\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, _) = md.compose_with(options).unwrap();
        // body-shell contract concatenates stdout + stderr, so the line is
        // still present even after redirection
        assert!(composed.content().contains("routed"));
    }

    /// Trailing chain operator at the body level produces a parse error.
    #[test]
    fn body_directive_trailing_operator_is_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell echo ok &&\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let result = md.compose_with(options);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Missing command after chain operator"),
            "got: {msg}"
        );
    }
}
