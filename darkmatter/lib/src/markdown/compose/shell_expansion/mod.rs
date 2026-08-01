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
use crate::markdown::compose::ComposeWarning;

/// Owns the allow-once reservations taken for one directive's approval flow and
/// releases any still held when that flow leaves [`prepare_directive`] by ANY
/// path — including an approval-handler error and a policy-store write failure.
///
/// A leaked reservation is not a cosmetic set entry: the next composition of
/// that same command blocks for the full `RESERVATION_WAIT_TIMEOUT` and then
/// fails as an approval conflict the user never caused.
///
/// Reservations consumed on the success path are handed off explicitly via
/// [`ReservationGuard::release`], which removes them from the guard, so `Drop`
/// releases only what is genuinely still held.
struct ReservationGuard<'a> {
    runtime: &'a mut ShellExpansionRuntime,
    held: Vec<String>,
}

impl<'a> ReservationGuard<'a> {
    fn new(runtime: &'a mut ShellExpansionRuntime) -> Self {
        Self {
            runtime,
            held: Vec::new(),
        }
    }

    fn reserve(&mut self, normalized: &str, may_wait: bool) -> types::ReserveOutcome {
        let outcome = self.runtime.reserve_allow_once(normalized, may_wait);
        if outcome == types::ReserveOutcome::Reserved {
            self.held.push(normalized.to_string());
        }
        outcome
    }

    fn holds_none(&self) -> bool {
        self.held.is_empty()
    }

    /// Completes the reservation for `normalized` if this guard owns it.
    ///
    /// A command this guard never reserved is already authorized — by the
    /// whitelist or by a peer's completed allow-once — so it is left alone
    /// rather than released out from under whoever does own it.
    fn release(&mut self, normalized: &str, approved: bool) {
        if let Some(index) = self.held.iter().position(|held| held == normalized) {
            self.held.swap_remove(index);
            self.runtime.complete_allow_once(normalized, approved);
        }
    }
}

impl Drop for ReservationGuard<'_> {
    fn drop(&mut self) {
        for normalized in std::mem::take(&mut self.held) {
            self.runtime.complete_allow_once(&normalized, false);
        }
    }
}

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
/// use biscuit_terminal::errors::SourceContext;
/// use darkmatter::markdown::compose::shell_expansion::{execute_directive, types::{ErrorHandling, ShellCommandOrigin, ShellDirective, ShellExpansionRuntime, ShellPolicyPaths}};
/// use darkmatter::markdown::compose::ComposeOptions;
/// use std::path::PathBuf;
///
/// let ctx = SourceContext::new(PathBuf::from("/t"), PathBuf::from("t"), "");
/// let directive = ShellDirective {
///     raw_command: "echo hello".to_string(),
///     executable: "echo".to_string(),
///     args: vec!["hello".to_string()],
///     span: 0..10,
///     indent: String::new(),
///     origin: ShellCommandOrigin::Body { line: 1 },
///     error_handling: ErrorHandling::default(),
///     timeout_override: None,
///     no_cache: false,
///     pipeline: None,
///     ctx,
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
    execute_prepared_directive(&prepared, options, shell_runtime)
}

/// Resolves aliases, applies policy checks, and records approval decisions
/// without executing the command yet.
///
/// ## Notes
///
/// Each directive is judged against a policy view taken at its own entry, so a
/// rule persisted by an approval earlier in the same stage suppresses a later
/// directive's prompt — the user-observable prompt frequency callers rely on.
/// The view is an [`Arc`]-shared borrow of the runtime's rule collections
/// ([`ShellExpansionRuntime::snapshot`]), not a copy of them, so taking it per
/// directive costs three refcount bumps.
///
/// The policy mutex is held only for those refcount bumps — never across
/// parsing, approval, or command execution.
///
/// Allow-once reservations taken here are owned by a [`ReservationGuard`], so
/// every exit — handler error, policy-store write failure, conflict, or success
/// — releases whatever is still reserved and wakes any same-command waiter.
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
                    ctx: Box::new(directive.ctx.clone()),
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
                ctx: Box::new(directive.ctx.clone()),
                command: display_command.clone(),
                reason,
                origin: directive.origin.clone(),
            });
        }
    }

    // 2. Check user blacklist for all commands
    for (i, normalized) in normalized_commands.iter().enumerate() {
        let (exe, args) = executable_and_args_at(&effective, i);
        if check_user_blacklist(&runtime_snapshot.user_blacklist, &exe, &args, normalized) {
            return Err(ShellExpansionError::Blacklisted {
                ctx: Box::new(directive.ctx.clone()),
                command: display_command.clone(),
                reason: "user blacklist".to_string(),
                origin: directive.origin.clone(),
            });
        }
    }

    // 3. Check whitelist for all commands
    let all_whitelisted = normalized_commands
        .iter()
        .enumerate()
        .all(|(i, normalized)| {
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
        // Reserve only the commands that are not already authorized. Commands
        // already in the snapshot whitelist or already in `allow_once` are
        // skipped. If another thread holds a pending reservation for any
        // command in this chain, treat it as a conflict and require fresh
        // approval rather than implicitly approving the unrelated commands.
        let mut guard = ReservationGuard::new(shell_runtime);
        let mut pending_conflict = false;

        for (i, normalized) in normalized_commands.iter().enumerate() {
            let (exe, _) = executable_and_args_at(&effective, i);
            if check_whitelist(&runtime_snapshot.whitelist, &exe, normalized) {
                continue;
            }
            // Block on a same-command peer approval only while we hold no
            // reservations of our own; otherwise a hold-and-wait cycle across
            // cross-ordered chains could deadlock.
            let may_wait = guard.holds_none();
            match guard.reserve(normalized, may_wait) {
                types::ReserveOutcome::Reserved | types::ReserveOutcome::AlreadyAllowed => {}
                types::ReserveOutcome::Pending => {
                    pending_conflict = true;
                    break;
                }
            }
        }

        if pending_conflict {
            // Release reservations we just made so the conflicting thread can
            // make progress, then surface the conflict as an approval-required
            // error instead of silently approving the rest of the chain.
            drop(guard);
            return Err(ShellExpansionError::ApprovalRequired {
                ctx: Box::new(directive.ctx.clone()),
                command: display_command,
                whitelist_path: Box::new(policy_paths.whitelist.clone()),
                blacklist_path: Box::new(policy_paths.blacklist.clone()),
                origin: directive.origin.clone(),
            });
        }

        if guard.holds_none() {
            // Every command in the chain is now authorized (whitelist or a
            // concurrently-completed allow-once). No prompt is needed.
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
                    guard.release(normalized, false);
                    store::append_whitelist_exact(policy_paths, normalized)?;
                    guard.runtime.persist_whitelist_exact(normalized.clone());
                }
                Ok(PreparedShellDirective {
                    effective,
                    display_command,
                })
            }
            ShellApprovalDecision::AllowCommandPersist => {
                for normalized in &normalized_commands {
                    guard.release(normalized, false);
                }
                // Persist a prefix entry for every unique executable in the
                // chain so the next run does not re-prompt for later actions.
                let mut persisted = std::collections::HashSet::new();
                for (exe, _) in executables_with_args(&effective) {
                    if persisted.insert(exe.clone()) {
                        store::append_whitelist_prefix(policy_paths, &exe)?;
                        guard.runtime.persist_whitelist_prefix(exe);
                    }
                }
                Ok(PreparedShellDirective {
                    effective,
                    display_command,
                })
            }
            ShellApprovalDecision::AllowOnce => {
                for normalized in &normalized_commands {
                    guard.release(normalized, true);
                }
                guard.runtime.approvals_used += 1;
                Ok(PreparedShellDirective {
                    effective,
                    display_command,
                })
            }
            ShellApprovalDecision::Deny => {
                for normalized in &normalized_commands {
                    guard.release(normalized, false);
                }
                Err(ShellExpansionError::Denied {
                    ctx: Box::new(directive.ctx.clone()),
                    command: display_command,
                    origin: directive.origin.clone(),
                })
            }
            ShellApprovalDecision::BlacklistPersist => {
                for normalized in &normalized_commands {
                    guard.release(normalized, false);
                    store::append_blacklist_exact(policy_paths, normalized)?;
                    guard.runtime.persist_blacklist_exact(normalized.clone());
                }
                Err(ShellExpansionError::Blacklisted {
                    ctx: Box::new(directive.ctx.clone()),
                    command: display_command,
                    reason: "user blacklisted".to_string(),
                    origin: directive.origin.clone(),
                })
            }
        }
    } else {
        Err(ShellExpansionError::ApprovalRequired {
            ctx: Box::new(directive.ctx.clone()),
            command: display_command,
            whitelist_path: Box::new(policy_paths.whitelist.clone()),
            blacklist_path: Box::new(policy_paths.blacklist.clone()),
            origin: directive.origin.clone(),
        })
    }
}

/// Built-in allowlist of read-only-but-nondeterministic executables.
///
/// A cached (collapsed) repeat of one of these is the foot-gun the cache
/// discoverability warning targets; `--no-cache` (or `::no-cache` /
/// `no_cache=true`) opts a volatile command back into fresh execution at each
/// occurrence.
const VOLATILE_EXECUTABLES: &[&str] = &["uuidgen", "date", "openssl"];

/// Returns the normalized cache key for a directive.
///
/// The key is the canonical pipeline shape: each action is rendered with
/// [`normalize_command`] (so quoting is canonical), joined by its real chain
/// operator (`&&`/`||`), with redirection tokens appended. This prevents two
/// directives that differ only in chain operator or redirection from sharing
/// one cache entry — `false || echo fallback` and `false && echo fallback`
/// have different execution semantics and must not collide.
fn cache_key(effective: &ShellDirective) -> String {
    if let Some(ref pipeline) = effective.pipeline {
        let mut key = String::new();
        for action in &pipeline.actions {
            let op_str = match action.operator {
                types::ChainOperator::None => "",
                types::ChainOperator::And => " && ",
                types::ChainOperator::Or => " || ",
            };
            key.push_str(op_str);
            key.push_str(&normalize_command(
                &action.command.executable,
                &action.command.args,
            ));
            key.push_str(&types::render_redirection(&action.command.redirection));
        }
        key
    } else {
        normalize_command(&effective.executable, &effective.args)
    }
}

/// Returns `true` if any executable in the directive is on the volatile
/// allowlist, so silently collapsing repeats could hide an intended fresh value.
fn directive_is_volatile(effective: &ShellDirective) -> bool {
    executables_with_args(effective).iter().any(|(exe, _)| {
        let base = std::path::Path::new(exe)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(exe.as_str());
        VOLATILE_EXECUTABLES.contains(&base)
    })
}

/// Builds the discoverability warning emitted the first time a volatile command
/// is served from cache.
fn volatile_cache_warning(effective: &ShellDirective, display: &str) -> ComposeWarning {
    let warning = ComposeWarning::new(
        "shell_expansion",
        format!(
            "'{display}' is a volatile command that appears more than once; it was executed \
             once and its result reused (cached). Add `--no-cache` (body), `::no-cache` \
             (frontmatter), or `no_cache=true` (shell block) for a fresh value each time."
        ),
    );
    match effective.origin {
        ShellCommandOrigin::Body { line } => warning.at_line(line),
        ShellCommandOrigin::Frontmatter { .. } => warning,
        ShellCommandOrigin::ShellBlock { command_line, .. } => warning.at_line(command_line),
    }
}

/// Executes a previously prepared directive, applying the per-compose command
/// cache.
///
/// Identical normalized commands execute once per compose: the first occurrence
/// runs and memoizes its output; later occurrences reuse it. A directive marked
/// `no_cache` (via `--no-cache` / `::no-cache` / `no_cache=true`) bypasses the
/// cache entirely — it neither reads nor writes it and runs fresh every time.
pub(crate) fn execute_prepared_directive(
    prepared: &PreparedShellDirective,
    options: &ComposeOptions,
    shell_runtime: &ShellExpansionRuntime,
) -> Result<DirectiveExecutionResult, ShellExpansionError> {
    if prepared.effective.no_cache {
        return run_prepared_fresh(prepared, options);
    }

    let key = cache_key(&prepared.effective);

    if let Some((stdout, stderr)) = shell_runtime.cache_lookup(&key) {
        // Warnings from the original execution (e.g. a timeout fallback) belong
        // to that occurrence and are not replayed; a one-shot discoverability
        // warning may fire for a volatile command the cache just collapsed.
        let mut warnings = Vec::new();
        if directive_is_volatile(&prepared.effective)
            && shell_runtime.note_volatile_cache_hit(&key)
        {
            warnings.push(volatile_cache_warning(
                &prepared.effective,
                &prepared.display_command,
            ));
        }
        return Ok(DirectiveExecutionResult {
            stdout,
            stderr,
            warnings,
        });
    }

    let result = run_prepared_fresh(prepared, options)?;
    shell_runtime.cache_store(&key, &result.stdout, &result.stderr);
    Ok(result)
}

/// Executes a prepared directive without consulting the cache and converts
/// timeout fallbacks into compose warnings.
fn run_prepared_fresh(
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
                            ctx,
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
                                ctx,
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
                indent: directive.indent.clone(),
                origin: directive.origin.clone(),
                error_handling: directive.error_handling.clone(),
                timeout_override: directive.timeout_override,
                no_cache: directive.no_cache,
                pipeline: Some(new_pipeline),
                ctx: directive.ctx.clone(),
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
            indent: directive.indent.clone(),
            origin: directive.origin.clone(),
            error_handling: directive.error_handling.clone(),
            timeout_override: directive.timeout_override,
            no_cache: directive.no_cache,
            pipeline: new_pipeline,
            ctx: directive.ctx.clone(),
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
        return (
            action.command.executable.clone(),
            action.command.args.clone(),
        );
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

    /// Composes `content` through the shell-expansion stage with an allow-once
    /// approval handler, returning the composed body.
    fn compose_shell_only(content: &str) -> String {
        let temp_dir = TempDir::new().unwrap();
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
        let (composed, _report) = md.compose_with(options).unwrap();
        composed.content().to_string()
    }

    /// Like [`compose_shell_only`] but also returns the compose report so cache
    /// discoverability warnings can be asserted.
    fn compose_shell_with_report(content: &str) -> (String, crate::markdown::compose::ComposeReport) {
        let temp_dir = TempDir::new().unwrap();
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
        (composed.content().to_string(), report)
    }

    /// Returns the non-blank output lines of a body composed through the shell
    /// stage — one per `uuidgen` directive.
    fn non_blank_lines(body: &str) -> Vec<String> {
        body.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// T5 — identical pure commands execute once per compose: both occurrences
    /// of `uuidgen` collapse to a single execution and share one value.
    #[test]
    fn cache_default_collapses_identical_commands() {
        if which::which("uuidgen").is_err() {
            return;
        }
        let (body, _) = compose_shell_with_report("::shell uuidgen\n\n::shell uuidgen\n");
        let ids = non_blank_lines(&body);
        assert_eq!(ids.len(), 2, "expected two output lines: {body:?}");
        assert_eq!(
            ids[0], ids[1],
            "cached command must yield one consistent value at both sites"
        );
    }

    /// T6 — `--no-cache` fully bypasses the cache: each occurrence executes
    /// fresh and yields a distinct value.
    #[test]
    fn no_cache_body_flag_executes_fresh_each_time() {
        if which::which("uuidgen").is_err() {
            return;
        }
        let (body, _) =
            compose_shell_with_report("::shell --no-cache uuidgen\n\n::shell --no-cache uuidgen\n");
        let ids = non_blank_lines(&body);
        assert_eq!(ids.len(), 2, "expected two output lines: {body:?}");
        assert_ne!(ids[0], ids[1], "--no-cache must yield distinct values");
    }

    /// Cache keys must preserve chain operators. `false || echo fallback` and
    /// `false && echo fallback` normalize to the same per-action commands
    /// (`false`, `echo fallback`) but have different execution semantics. The
    /// `&&` chain must not be served from the `||` chain's cache entry.
    #[test]
    fn cache_key_preserves_chain_operators() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell false || echo fallback\n\n::shell false && echo fallback\n";
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
        let err = result.expect_err(
            "the `&&` chain must fail (exit 1) instead of reusing the `||` chain's cached success",
        );
        assert!(
            err.to_string().contains("exit 1"),
            "expected ExecutionFailed exit 1, got: {err}"
        );
    }

    /// Cache keys must preserve redirection. `echo a` and `echo a > /dev/null`
    /// differ only in redirection and must produce distinct cache entries.
    #[test]
    fn cache_key_preserves_redirection() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell echo visible > /dev/null && echo cached\n\n::shell echo cached\n";
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
        let body = composed.content();
        // The first directive suppresses `echo visible` (> /dev/null) and only
        // emits `echo cached`. The second directive emits `echo cached` too.
        // If redirection were dropped from the key, the second directive would
        // reuse the first's cache entry (still "cached") — same output, so
        // assert the suppressed command never leaks into the body instead.
        assert!(
            !body.contains("visible"),
            "redirection must suppress output; got: {body:?}"
        );
        assert_eq!(body.matches("cached").count(), 2, "got: {body:?}");
    }

    /// T9 — a repeated volatile command without `--no-cache` emits a single
    /// discoverability warning.
    #[test]
    fn volatile_repeated_command_warns_once() {
        if which::which("uuidgen").is_err() {
            return;
        }
        let (_body, report) =
            compose_shell_with_report("::shell uuidgen\n\n::shell uuidgen\n\n::shell uuidgen\n");
        let warnings: Vec<_> = report
            .warnings
            .iter()
            .filter(|w| w.message.contains("volatile") && w.message.contains("--no-cache"))
            .collect();
        assert_eq!(
            warnings.len(),
            1,
            "expected exactly one discoverability warning, got: {:?}",
            report.warnings
        );
    }

    /// T2 — execution is condition-aware: a `::shell` in a `when:`-false page
    /// block is pruned before the shell stage and never executes (the side
    /// effect — creating the sentinel file — must not happen).
    #[test]
    fn dead_page_block_shell_command_never_executes() {
        let temp_dir = TempDir::new().unwrap();
        let sentinel = temp_dir.path().join("ran.txt");
        let content = format!(
            "---\nenabled: false\n---\n::block when=\"enabled == true\"\n::shell touch {}\n::end-block\n",
            sentinel.display()
        );
        let md: Markdown = content.as_str().into();
        let options = ComposeOptions::new()
            .only(&[ComposeOperation::PageBlocks, ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });
        let _ = md.compose_with(options).unwrap();
        assert!(
            !sentinel.exists(),
            "dead-branch command must not execute under condition-aware execution"
        );
    }

    #[test]
    fn indented_shell_output_under_list_item_is_reindented() {
        let content = "- intro\n\n    ::shell printf 'one\\ntwo\\n'\n\n- next\n";
        let body = compose_shell_only(content);
        // Every output line keeps the directive's 4-space indent so the lines
        // remain nested under the list item instead of becoming siblings.
        assert!(body.contains("    one\n    two\n"), "got: {body:?}");
        assert!(!body.contains("\none\n"), "got: {body:?}");
    }

    #[test]
    fn tab_indented_shell_output_is_reindented() {
        let content = "- intro\n\n\t::shell printf 'one\\ntwo\\n'\n\n- next\n";
        let body = compose_shell_only(content);
        assert!(body.contains("\tone\n\ttwo\n"), "got: {body:?}");
    }

    #[test]
    fn indented_shell_blank_output_lines_receive_indent() {
        let content = "- intro\n\n    ::shell printf 'one\\n\\ntwo\\n'\n\n- next\n";
        let body = compose_shell_only(content);
        // The blank separator line is indented too, otherwise it would break
        // out of the surrounding list.
        assert!(body.contains("    one\n    \n    two\n"), "got: {body:?}");
    }

    #[test]
    fn root_level_shell_output_has_no_indent() {
        let content = "intro\n\n::shell printf 'one\\ntwo\\n'\n\nnext\n";
        let body = compose_shell_only(content);
        assert!(body.contains("one\ntwo\n"), "got: {body:?}");
        assert!(!body.contains("    one"), "got: {body:?}");
    }

    #[test]
    fn indented_shell_trailing_newline_does_not_become_indent_only_line() {
        // Output ending in a trailing newline keeps that newline bare: the final
        // content line is indented, but the trailing newline is NOT followed by
        // an indentation-only `"    "` line. There is no further output line for
        // such a prefix to keep nested, and a trailing `""` vs `"    "` are
        // CommonMark-equivalent at the end of a container.
        let content = "- intro\n\n    ::shell printf 'one\\n'\n\n- next\n";
        let body = compose_shell_only(content);
        assert!(body.contains("    one\n\n- next"), "got: {body:?}");
        assert!(!body.contains("    one\n    "), "got: {body:?}");
    }

    #[test]
    fn blockquote_shell_output_is_prefixed_with_markers() {
        // A `::shell` directive inside a block quote replays the `> ` markers on
        // every emitted line — including the blank separator — so the output
        // stays inside the quote.
        let content = "> intro\n>\n> ::shell printf 'one\\n\\ntwo\\n'\n";
        let body = compose_shell_only(content);
        assert!(body.contains("> one\n> \n> two\n"), "got: {body:?}");
        assert!(!body.contains("\none\n"), "got: {body:?}");
    }

    #[test]
    fn nested_blockquote_shell_output_is_prefixed_with_nested_markers() {
        let content = "> > ::shell printf 'one\\ntwo\\n'\n";
        let body = compose_shell_only(content);
        assert!(body.contains("> > one\n> > two\n"), "got: {body:?}");
    }

    /// Composes `content` through the shell stage, then renders the result to
    /// HTML so the splice can be validated against the CommonMark block
    /// structure rather than raw substrings.
    fn compose_shell_to_html(content: &str) -> String {
        let body = compose_shell_only(content);
        let md: Markdown = body.as_str().into();
        md.as_html(crate::markdown::output::HtmlOptions::default())
            .unwrap()
    }

    #[test]
    fn indented_shell_output_is_nested_under_list_item_in_commonmark() {
        // Re-indented output is a continuation paragraph of the first list item,
        // so the whole document is a single list with two items — proving the
        // generated lines are children of the parent <li>, not siblings.
        let content = "- intro\n\n    ::shell printf 'one\\ntwo\\n'\n\n- next\n";
        let html = compose_shell_to_html(content);
        assert_eq!(html.matches("<ul>").count(), 1, "got: {html}");
        assert_eq!(html.matches("<li>").count(), 2, "got: {html}");
    }

    #[test]
    fn root_level_shell_output_is_a_sibling_block_in_commonmark() {
        // The column-1 baseline is intentionally a top-level sibling: the spliced
        // paragraph splits the surrounding list into two separate <ul> blocks.
        let content = "- intro\n\n::shell printf 'one\\ntwo\\n'\n\n- next\n";
        let html = compose_shell_to_html(content);
        assert_eq!(html.matches("<ul>").count(), 2, "got: {html}");
    }

    #[test]
    fn blockquote_shell_output_is_nested_inside_blockquote_in_commonmark() {
        // The `> ` markers keep the spliced output inside the quote, so the whole
        // document renders as a single <blockquote> rather than the directive
        // breaking out of it.
        let content = "> intro\n>\n> ::shell printf 'one\\ntwo\\n'\n";
        let html = compose_shell_to_html(content);
        assert_eq!(html.matches("<blockquote>").count(), 1, "got: {html}");
        assert!(html.contains("one"), "got: {html}");
        assert!(html.contains("two"), "got: {html}");
    }

    #[test]
    fn pipeline_timeout_fallback_emits_warning() {
        let temp_dir = TempDir::new().unwrap();
        // `execute_pipeline_detailed` spends one timeout budget on every `&&`
        // segment, so `echo after` must spawn and exit inside it too. The sleep
        // therefore has to overshoot the budget by a wide margin while the
        // budget stays wide enough for a slow spawn under parallel-suite load.
        // The sleeping child is killed at the timeout, so its length is free.
        let content = "::shell sleep 10 && echo after\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                timeout: std::time::Duration::from_secs(2),
                timeout_behavior: ShellTimeoutBehavior::EmptyString,
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(MockApprovalHandler {
                    decision: ShellApprovalDecision::AllowOnce,
                })),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("after"));
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].message.contains("timed out"));
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

    /// F32, half 1 of the policy-visibility contract — a rule persisted by an
    /// approval in one stage IS policy input for a *subsequent* stage. The root
    /// body stage persists `prefix echo`; the transcluded child's body stage
    /// sees it and never prompts.
    #[test]
    fn persisted_whitelist_from_one_stage_is_policy_input_for_the_next_stage() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().join("root.md");
        let child = temp_dir.path().join("child.md");

        std::fs::write(&root, "::shell echo alpha\n\n::file ./child.md\n").unwrap();
        std::fs::write(&child, "## Child\n\n::shell echo beta\n").unwrap();

        let handler = Arc::new(CountingApprovalHandler::new(
            ShellApprovalDecision::AllowCommandPersist,
        ));
        let options =
            ComposeOptions::new()
                .with_source_file(&root)
                .with_shell(ShellExpansionOptions {
                    policy_root: Some(temp_dir.path().to_path_buf()),
                    approval_handler: Some(handler.clone()),
                    ..Default::default()
                });

        let (composed, _report) = Markdown::try_from(root.as_path())
            .unwrap()
            .compose_with(options)
            .unwrap();

        assert_eq!(
            handler.approvals(),
            1,
            "the child stage's fresh snapshot must see the whitelist entry the root stage persisted"
        );
        assert!(composed.content().contains("alpha"));
        assert!(composed.content().contains("beta"));
    }

    /// F32, half 2 of the policy-visibility contract — a rule persisted
    /// mid-stage IS policy input for later directives in that *same* stage.
    /// The first `echo` prompts and persists `prefix echo`; the second matches
    /// that rule and must not prompt.
    ///
    /// This pins the user-observable prompt **frequency**: exactly one approval
    /// for the pair. Finding 32's rule-set sharing must not change it.
    #[test]
    fn persistence_mid_stage_is_policy_input_for_the_same_stage() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell echo alpha\n\n::shell echo beta\n";
        let md: Markdown = content.into();

        let handler = Arc::new(CountingApprovalHandler::new(
            ShellApprovalDecision::AllowCommandPersist,
        ));
        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(handler.clone()),
                ..Default::default()
            });

        let (composed, _report) = md.compose_with(options).unwrap();

        assert_eq!(
            handler.approvals(),
            1,
            "the second directive must observe the `prefix echo` rule the first one persisted, \
             and not prompt again"
        );
        let whitelist = std::fs::read_to_string(temp_dir.path().join(".darkmatter-shell-whitelist"))
            .unwrap();
        assert!(
            whitelist.contains("prefix echo"),
            "approval must still persist its rule; got: {whitelist:?}"
        );
        assert!(composed.content().contains("alpha"));
        assert!(composed.content().contains("beta"));
    }

    /// F32 — an exact-command persist mid-stage likewise suppresses a later
    /// prompt for that same command, while a *different* command in the same
    /// stage still prompts. Guards against a fix that preserves the frequency
    /// count by over-authorizing rather than by refreshing the policy view.
    #[test]
    fn exact_persist_mid_stage_suppresses_only_the_same_command() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell --no-cache echo alpha\n\n::shell --no-cache echo alpha\n\n::shell --no-cache printf beta\n";
        let md: Markdown = content.into();

        let handler = Arc::new(CountingApprovalHandler::new(
            ShellApprovalDecision::AllowExactPersist,
        ));
        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(handler.clone()),
                ..Default::default()
            });

        let (_composed, _report) = md.compose_with(options).unwrap();

        assert_eq!(
            handler.approvals(),
            2,
            "the repeated `echo alpha` must reuse the persisted exact rule, but `printf beta` \
             must still prompt on its own"
        );
    }

    /// F32 — the runtime shares its rule collections with a snapshot by pointer
    /// instead of deep-copying them, and a write copies-on-write so an
    /// outstanding snapshot keeps observing the rules it was taken with.
    ///
    /// This is the structural bound behind the per-directive snapshot: taking a
    /// view per directive is affordable only because it copies no rules.
    #[test]
    fn snapshot_shares_rule_collections_by_pointer() {
        let mut runtime = ShellExpansionRuntime::new();

        let first = runtime.snapshot();
        let second = runtime.snapshot();
        assert!(
            Arc::ptr_eq(&first.whitelist, &second.whitelist),
            "two snapshots of an unchanged runtime must share one whitelist allocation"
        );
        assert!(
            Arc::ptr_eq(&first.user_blacklist, &second.user_blacklist),
            "two snapshots of an unchanged runtime must share one blacklist allocation"
        );
        assert!(
            Arc::ptr_eq(&first.allow_once, &second.allow_once),
            "two snapshots of an unchanged runtime must share one allow-once allocation"
        );

        runtime.persist_whitelist_prefix("echo".to_string());

        let third = runtime.snapshot();
        assert!(
            !Arc::ptr_eq(&first.whitelist, &third.whitelist),
            "persisting a rule must not mutate a snapshot taken before it"
        );
        assert!(
            first.whitelist.entries.is_empty(),
            "the pre-persist snapshot must still observe the rules it was taken with"
        );
        assert!(
            third.whitelist.matches_prefix("echo"),
            "a snapshot taken after the persist must observe the new rule"
        );
    }

    /// F32 — allow-once approvals are arbitrated live through
    /// `reserve_allow_once` against shared runtime state rather than through a
    /// snapshot, so one approval covers a repeat of that exact command later in
    /// the SAME stage (and across concurrent sibling transclusions).
    #[test]
    fn allow_once_still_dedupes_within_a_single_stage() {
        let temp_dir = TempDir::new().unwrap();
        let content = "::shell --no-cache echo same\n\n::shell --no-cache echo same\n";
        let md: Markdown = content.into();

        let handler = Arc::new(CountingApprovalHandler::new(
            ShellApprovalDecision::AllowOnce,
        ));
        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(handler.clone()),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(
            handler.approvals(),
            1,
            "one allow-once approval must cover the repeat of that exact command"
        );
        assert_eq!(report.shell_expansions_applied, 2);
        assert_eq!(composed.content().matches("same").count(), 2);
    }

    /// F32 — the policy mutex must not be held across approval. A handler that
    /// touches the same runtime (here: by taking a snapshot of it) while the
    /// stage is mid-approval would deadlock if `prepare_directive` still held
    /// the lock. The test's own timeout is the failure signal.
    #[test]
    fn policy_mutex_is_not_held_across_approval() {
        struct RuntimeTouchingHandler {
            runtime: ShellExpansionRuntime,
        }

        impl ShellApprovalHandler for RuntimeTouchingHandler {
            fn approve(
                &self,
                _request: ShellApprovalRequest,
            ) -> Result<ShellApprovalDecision, ShellExpansionError> {
                // Reaching into the shared policy state from inside the approval
                // callback must not block.
                let _ = self.runtime.snapshot();
                Ok(ShellApprovalDecision::AllowOnce)
            }
        }

        let temp_dir = TempDir::new().unwrap();
        let runtime = ShellExpansionRuntime::new();
        let content = "::shell echo hello\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(Arc::new(RuntimeTouchingHandler {
                    runtime: runtime.clone_for_child(),
                })),
                ..Default::default()
            });

        let (composed, _report) = md.compose_with(options).unwrap();
        assert!(composed.content().contains("hello"));
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

    /// Regression for review-6: a chain whose first action was previously
    /// approved as allow-once must NOT bypass approval for the remaining
    /// un-approved actions in the chain. Every command in the chain must be
    /// presented to the approval handler.
    #[test]
    fn pipeline_partial_allow_once_overlap_in_chain_re_prompts() {
        let temp_dir = TempDir::new().unwrap();
        let content = "# Test\n\n::shell echo allowed\n\n::shell echo allowed && echo unapproved\n";
        let md: Markdown = content.into();

        let handler = Arc::new(CountingApprovalHandler::new(
            ShellApprovalDecision::AllowOnce,
        ));

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellExpansion])
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                approval_handler: Some(handler.clone()),
                ..Default::default()
            });

        let (composed, report) = md.compose_with(options).unwrap();

        // Both directives produced their output.
        assert!(composed.content().contains("allowed"));
        assert!(composed.content().contains("unapproved"));

        // The handler must have been called for BOTH directives — once for the
        // first single-action directive, and once for the second chain because
        // it contains an un-approved action ("echo unapproved"). Prior to the
        // fix, the second chain silently approved itself because "echo allowed"
        // was already in `allow_once`.
        assert_eq!(handler.approvals(), 2);
        assert_eq!(report.shell_approvals_used, 2);
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
        assert_eq!(
            count, 1,
            "expected exactly one 'prefix echo' line: {whitelist:?}"
        );
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
            let path = which::which(candidate).ok()?;
            std::process::Command::new(&path)
                .arg("--version")
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|_| path.to_string_lossy().to_string())
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

    /// `> /dev/null 2>&1` suppresses BOTH stdout and stderr.
    ///
    /// Per shell semantics, `2>&1` after `>/dev/null` dups stderr to the
    /// already-redirected stdout (now /dev/null), so both streams vanish.
    #[test]
    fn redirection_stdout_null_then_merge_suppresses_both_streams() {
        let temp_dir = TempDir::new().unwrap();
        let python = find_test_python();
        let Some(ref python) = python else { return };
        let content = format!(
            "Before\n::shell {python} -c \"import sys; sys.stdout.write('OUT'); sys.stderr.write('ERR')\" > /dev/null 2>&1\nAfter\n"
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
        assert!(body.contains("Before"));
        assert!(body.contains("After"));
        assert!(
            !body.contains("OUT"),
            "stdout should be suppressed: {body:?}"
        );
        assert!(
            !body.contains("ERR"),
            "stderr should be suppressed: {body:?}"
        );
    }

    /// `2>&1 > /dev/null` differs from `> /dev/null 2>&1`: stderr keeps a
    /// reference to the ORIGINAL stdout (capture), so only stdout is
    /// suppressed and stderr remains visible.
    #[test]
    fn redirection_merge_then_stdout_null_only_suppresses_stdout() {
        let temp_dir = TempDir::new().unwrap();
        let python = find_test_python();
        let Some(ref python) = python else { return };
        let content = format!(
            "::shell {python} -c \"import sys; sys.stdout.write('OUT'); sys.stderr.write('ERR')\" 2>&1 > /dev/null\n"
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
        assert!(
            !body.contains("OUT"),
            "stdout should be suppressed: {body:?}"
        );
        assert!(
            body.contains("ERR"),
            "stderr should remain visible: {body:?}"
        );
    }

    /// `> /dev/null 2> /dev/null` independently nulls both streams.
    #[test]
    fn redirection_both_streams_null_suppresses_both() {
        let temp_dir = TempDir::new().unwrap();
        let python = find_test_python();
        let Some(ref python) = python else { return };
        let content = format!(
            "Before\n::shell {python} -c \"import sys; sys.stdout.write('OUT'); sys.stderr.write('ERR')\" > /dev/null 2> /dev/null\nAfter\n"
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
        assert!(body.contains("Before"));
        assert!(body.contains("After"));
        assert!(!body.contains("OUT"));
        assert!(!body.contains("ERR"));
    }

    /// `2> /dev/null >&2` first nulls stderr, then routes stdout to the
    /// already-nulled stderr — both streams vanish.
    #[test]
    fn redirection_stderr_null_then_route_stdout_to_stderr_suppresses_both() {
        let temp_dir = TempDir::new().unwrap();
        let python = find_test_python();
        let Some(ref python) = python else { return };
        let content = format!(
            "Before\n::shell {python} -c \"import sys; sys.stdout.write('OUT'); sys.stderr.write('ERR')\" 2> /dev/null >&2\nAfter\n"
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
        assert!(body.contains("Before"));
        assert!(body.contains("After"));
        assert!(!body.contains("OUT"));
        assert!(!body.contains("ERR"));
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

    /// Every early return from `prepare_directive`'s approval flow must release
    /// the allow-once reservations it took.
    ///
    /// A leak is precisely a normalized command left in the runtime's
    /// `pending_allow_once` set once the flow that reserved it has returned; its
    /// user-observable cost is the *next* composition of that command blocking
    /// for `RESERVATION_WAIT_TIMEOUT` (30s) and then failing as an approval
    /// conflict the user never caused. These tests therefore assert the
    /// reservation set itself and then compose the same command again to pin the
    /// user-observable consequence. Neither oracle measures elapsed time: a
    /// composition's own preparation cost is unrelated to whether a reservation
    /// was released, so timing one conflates the two.
    mod reservation_cleanup {
        use super::*;
        use biscuit_terminal::errors::SourceContext;
        use std::path::{Path, PathBuf};
        use std::sync::{Condvar, Mutex};
        use std::time::{Duration, Instant};

        /// Ceiling for a waiter that the failing flow's release must wake.
        ///
        /// Unlike a composition's total cost, the two outcomes here are far
        /// apart and neither depends on host speed: a notified waiter resumes in
        /// microseconds, an un-notified one parks for the full 30s
        /// `RESERVATION_WAIT_TIMEOUT`. `prepare_directive` performs no I/O on
        /// this path, so contention cannot push a healthy waiter near this bound.
        const NOTIFICATION_BUDGET: Duration = Duration::from_secs(10);

        struct FixedHandler(ShellApprovalDecision);

        impl ShellApprovalHandler for FixedHandler {
            fn approve(
                &self,
                _request: ShellApprovalRequest,
            ) -> Result<ShellApprovalDecision, ShellExpansionError> {
                Ok(self.0.clone())
            }
        }

        /// Fails the way an interactive prompt does when its terminal read
        /// errors — the handler is user code and may return any error.
        struct FailingHandler;

        impl ShellApprovalHandler for FailingHandler {
            fn approve(
                &self,
                _request: ShellApprovalRequest,
            ) -> Result<ShellApprovalDecision, ShellExpansionError> {
                Err(handler_error())
            }
        }

        fn handler_error() -> ShellExpansionError {
            ShellExpansionError::PolicyIo {
                path: PathBuf::from("/approval-handler"),
                source: std::io::Error::other("approval handler failed"),
            }
        }

        fn directive(command: &str) -> ShellDirective {
            let content = format!("::shell {command}\n");
            let ctx = SourceContext::new(
                PathBuf::from("/test.md"),
                PathBuf::from("test.md"),
                Arc::from(content.as_str()),
            );
            parse_directives(&content, ctx, 0)
                .unwrap()
                .pop()
                .expect("fixture must parse to exactly one directive")
        }

        fn options(decision: ShellApprovalDecision) -> ComposeOptions {
            ComposeOptions::new().with_shell_approval_handler(Arc::new(FixedHandler(decision)))
        }

        fn policy_paths(temp: &TempDir) -> ShellPolicyPaths {
            ShellPolicyPaths {
                whitelist: temp.path().join(".darkmatter-shell-whitelist"),
                blacklist: temp.path().join(".darkmatter-shell-blacklist"),
            }
        }

        /// Makes a policy file reject an append the way a read-only checkout or
        /// a root-owned policy file does, so the persistence arms fail for real
        /// rather than through a test-only seam.
        fn make_unwritable(path: &Path) {
            std::fs::write(path, "").unwrap();
            let mut perms = std::fs::metadata(path).unwrap().permissions();
            perms.set_readonly(true);
            std::fs::set_permissions(path, perms).unwrap();
        }

        /// Asserts the failed flow released every reservation it took, then that
        /// composing `command` again still prompts and is approved rather than
        /// rejected as a conflict.
        ///
        /// The reservation-set assertion is the oracle and runs first: it is the
        /// exact definition of the defect, and failing on it reports the leaked
        /// command by name instead of letting the second composition stall for
        /// the full 30s `RESERVATION_WAIT_TIMEOUT`.
        fn assert_reservations_released(
            runtime: &mut ShellExpansionRuntime,
            paths: &ShellPolicyPaths,
            command: &str,
        ) {
            let leaked = runtime.pending_allow_once_for_test();
            assert!(
                leaked.is_empty(),
                "the failed flow left {leaked:?} reserved: the next composition of each would \
                 block for RESERVATION_WAIT_TIMEOUT and then fail as an approval conflict the \
                 user never caused"
            );

            if let Err(err) = prepare_directive(
                &directive(command),
                &options(ShellApprovalDecision::AllowOnce),
                paths,
                runtime,
            ) {
                panic!("a released command must not surface an approval conflict; got: {err}");
            }
        }

        /// Error class 1 — the approval handler returns an error before any
        /// reservation is completed.
        #[test]
        fn approval_handler_error_releases_every_reservation() {
            let temp = TempDir::new().unwrap();
            let paths = policy_paths(&temp);
            let mut runtime = ShellExpansionRuntime::new();

            let failing =
                ComposeOptions::new().with_shell_approval_handler(Arc::new(FailingHandler));
            let err = prepare_directive(
                &directive("echo alpha && echo beta"),
                &failing,
                &paths,
                &mut runtime,
            )
            .expect_err("the handler's error must propagate unchanged");
            assert!(
                matches!(err, ShellExpansionError::PolicyIo { .. }),
                "got: {err}"
            );

            assert_reservations_released(&mut runtime, &paths, "echo beta");
        }

        /// Error class 2 — `append_whitelist_exact` fails mid-chain. `echo
        /// alpha` is released and its append fails, so `echo beta` is still
        /// reserved when the error propagates.
        #[test]
        fn whitelist_exact_write_failure_releases_the_rest_of_the_chain() {
            let temp = TempDir::new().unwrap();
            let paths = policy_paths(&temp);
            make_unwritable(&paths.whitelist);
            let mut runtime = ShellExpansionRuntime::new();

            let err = prepare_directive(
                &directive("echo alpha && echo beta"),
                &options(ShellApprovalDecision::AllowExactPersist),
                &paths,
                &mut runtime,
            )
            .expect_err("an unwritable whitelist must fail the persist");
            assert!(
                matches!(err, ShellExpansionError::PolicyIo { .. }),
                "got: {err}"
            );

            assert_reservations_released(&mut runtime, &paths, "echo beta");
        }

        /// Error class 3 — `append_whitelist_prefix` fails. This arm releases
        /// the whole chain before its first fallible write, so it holds nothing
        /// to leak; the test pins that ordering against a future edit that
        /// interleaves release and persist the way the exact arm does.
        #[test]
        fn whitelist_prefix_write_failure_leaves_no_reservation() {
            let temp = TempDir::new().unwrap();
            let paths = policy_paths(&temp);
            make_unwritable(&paths.whitelist);
            let mut runtime = ShellExpansionRuntime::new();

            let err = prepare_directive(
                &directive("echo alpha && pwd"),
                &options(ShellApprovalDecision::AllowCommandPersist),
                &paths,
                &mut runtime,
            )
            .expect_err("an unwritable whitelist must fail the persist");
            assert!(
                matches!(err, ShellExpansionError::PolicyIo { .. }),
                "got: {err}"
            );

            assert_reservations_released(&mut runtime, &paths, "pwd");
        }

        /// Error class 4 — `append_blacklist_exact` fails mid-chain.
        #[test]
        fn blacklist_exact_write_failure_releases_the_rest_of_the_chain() {
            let temp = TempDir::new().unwrap();
            let paths = policy_paths(&temp);
            make_unwritable(&paths.blacklist);
            let mut runtime = ShellExpansionRuntime::new();

            let err = prepare_directive(
                &directive("echo alpha && echo beta"),
                &options(ShellApprovalDecision::BlacklistPersist),
                &paths,
                &mut runtime,
            )
            .expect_err("an unwritable blacklist must fail the persist");
            assert!(
                matches!(err, ShellExpansionError::PolicyIo { .. }),
                "got: {err}"
            );

            assert_reservations_released(&mut runtime, &paths, "echo beta");
        }

        /// Longest the approver will spin waiting for the peer to park before
        /// declaring the test setup broken.
        ///
        /// This bounds a correctness precondition, not the behavior under test:
        /// the waiter's fixtures are already built, so it reaches
        /// `reserve_allow_once` and increments the parked-waiter count in
        /// microseconds. Exceeding this means the waiter never parked — the
        /// synchronization the test depends on failed — so we panic loudly here
        /// rather than release early and silently stop exercising notification.
        const WAITER_PARK_DEADLINE: Duration = Duration::from_secs(5);

        /// The release must also notify a peer already parked in
        /// `reserve_allow_once`, not merely leave a clean set behind for the
        /// next caller.
        ///
        /// The approver reserves `echo shared`, enters its handler, and does not
        /// return its error until `parked_waiters_for_test` proves the waiter is
        /// enqueued on `reservation_done`. Because that count is incremented
        /// under the shared mutex immediately before `wait_timeout_while`'s
        /// atomic park, a positive reading is proof of parking — not the
        /// inference from a sleep the previous version relied on. Only then does
        /// the handler error, releasing the reservation and firing `notify_all`;
        /// a waiter that was merely late (having reserved the command itself)
        /// can no longer make this test pass, so it cannot silently stop
        /// exercising the notification path.
        #[test]
        fn handler_error_notifies_a_waiter_blocked_on_the_same_command() {
            struct SignalingHandler {
                entered: Arc<(Mutex<bool>, Condvar)>,
                probe: ShellExpansionRuntime,
            }

            impl ShellApprovalHandler for SignalingHandler {
                fn approve(
                    &self,
                    _request: ShellApprovalRequest,
                ) -> Result<ShellApprovalDecision, ShellExpansionError> {
                    let (lock, signal) = &*self.entered;
                    *lock.lock().unwrap() = true;
                    signal.notify_all();
                    // Hold the reservation open until the peer is *provably*
                    // parked on it, observed through the shared runtime the
                    // probe handle points at. `parked_waiters_for_test` reads
                    // the count under the same mutex the waiter incremented
                    // before its atomic park, so a positive reading cannot
                    // precede the park. Releasing before that would let the
                    // waiter reserve the command itself and pass without ever
                    // exercising notification — the defect this test exists to
                    // catch.
                    let deadline = Instant::now() + WAITER_PARK_DEADLINE;
                    while self.probe.parked_waiters_for_test("echo shared") == 0 {
                        assert!(
                            Instant::now() < deadline,
                            "the waiter never parked within {WAITER_PARK_DEADLINE:?}: the test \
                             can no longer prove release wakes a parked peer, so it is failing \
                             rather than silently degrading into a no-op"
                        );
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Err(handler_error())
                }
            }

            let temp = TempDir::new().unwrap();
            let paths = policy_paths(&temp);
            let runtime = ShellExpansionRuntime::new();
            let entered = Arc::new((Mutex::new(false), Condvar::new()));

            // Build the waiter's fixtures up front. `ComposeOptions::new`
            // captures a runtime context — host and repo discovery costing
            // seconds. Paying it after the signal below would delay the waiter's
            // park and needlessly widen `WAITER_PARK_DEADLINE`; it also keeps
            // that cost out of `NOTIFICATION_BUDGET`.
            let waiter_directive = directive("echo shared");
            let waiter_options = options(ShellApprovalDecision::AllowOnce);

            let mut approver_runtime = runtime.clone_for_child();
            let approver_paths = paths.clone();
            let approver_entered = Arc::clone(&entered);
            // The probe shares the same `Arc<Mutex<..>>` as every other handle
            // (via `clone_for_child`), so `parked_waiters_for_test` on it sees
            // the count the waiter's own handle increments.
            let approver_probe = runtime.clone_for_child();
            let approver = std::thread::spawn(move || {
                let handler = Arc::new(SignalingHandler {
                    entered: approver_entered,
                    probe: approver_probe,
                });
                let opts = ComposeOptions::new().with_shell_approval_handler(handler);
                prepare_directive(
                    &directive("echo shared"),
                    &opts,
                    &approver_paths,
                    &mut approver_runtime,
                )
                .expect_err("the handler's error must propagate unchanged")
            });

            // Wait until the approver provably holds the reservation before
            // starting the waiter, so the waiter parks on the approver's
            // reservation rather than winning the race and reserving first.
            {
                let (lock, signal) = &*entered;
                let mut in_handler = lock.lock().unwrap();
                while !*in_handler {
                    in_handler = signal.wait(in_handler).unwrap();
                }
            }

            // The waiter runs on its own thread so a missed notification is
            // reported here rather than stalling the test thread for the full
            // 30s `RESERVATION_WAIT_TIMEOUT` — long enough that nextest's
            // terminate-after ceiling would kill the run first and report a
            // timeout instead of the real defect. Its fixtures are already
            // built, so it reaches `reserve_allow_once` and parks immediately;
            // the approver's handler is blocked until that park is observed.
            let (outcome_tx, outcome_rx) = std::sync::mpsc::channel();
            let mut waiter_runtime = runtime.clone_for_child();
            let waiter_paths = paths.clone();
            std::thread::spawn(move || {
                let prepared = prepare_directive(
                    &waiter_directive,
                    &waiter_options,
                    &waiter_paths,
                    &mut waiter_runtime,
                );
                let _ = outcome_tx.send(prepared.map(|_| ()).map_err(|err| err.to_string()));
            });

            match outcome_rx.recv_timeout(NOTIFICATION_BUDGET) {
                Ok(Ok(())) => {}
                Ok(Err(err)) => panic!(
                    "the waiter must be notified and approved, not left to time out into a \
                     conflict; got: {err}"
                ),
                Err(_) => panic!(
                    "the waiter was still parked after {NOTIFICATION_BUDGET:?}: the failed flow \
                     released its reservation without waking it, so the waiter is sitting out the \
                     full RESERVATION_WAIT_TIMEOUT"
                ),
            }
            approver.join().unwrap();

            assert!(
                runtime.pending_allow_once_for_test().is_empty(),
                "the notified waiter's own approval must leave the reservation set clean"
            );
        }
    }
}
