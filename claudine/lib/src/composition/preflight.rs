//! Pre-flight shell command approval for provider sessions.
//!
//! Scans all sources of shell commands (template directives and lifecycle
//! stacks), checks them against the whitelist, prompts the user for any that
//! need approval, and returns the full pre-approved set.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::{Expr, ExpressionFinder, ResolutionContext, parse};
use darkmatter::markdown::compose::subtree::SubtreeCompose;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, EffectiveStateBuilder};

use crate::composition::error::CompositionError;
use crate::composition::lifecycle::{
    LATE_BINDING_ROOTS, LifecycleConfig, LifecycleSignal, collect_lifecycle_shell_commands,
    collect_lifecycle_shell_commands_for,
};
use crate::composition::lifecycle_actions::LifecycleActionKind;
use crate::harness::shell::{ShellApprovalOptions, tokenize_words_strict};

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
/// 2. Lifecycle stack shell commands (from every reachable `action: shell`
///    across all seven lifecycle events)
///
/// ## Errors
///
/// - `ShellCommandDenied` if the user denies any command
/// - `PreFlightDiscoveryFailed` if Darkmatter's document graph walk fails
/// - `PreFlightFailed` for blacklisted commands or missing approval handler
///
/// ## Arguments
///
/// * `markdown` — composed Markdown for template `::shell` discovery.
/// * `compose_options` — Darkmatter compose options for the template walker.
/// * `approval_options` — shell approval policy, handler, and cache.
/// * `lifecycle` — parsed lifecycle configuration; when present, every
///   reachable `action: shell` command (and `on_error` command) across all
///   seven lifecycle events is collected and audited alongside the other
///   sources. `lifecycle_source_path` names the composition source file for
///   diagnostics.
/// * `lifecycle_source_path` — source path for lifecycle-stack shell
///   commands; ignored when `lifecycle` is `None`.
pub fn resolve_shell_approvals(
    markdown: Option<&Markdown>,
    compose_options: Option<&ComposeOptions>,
    approval_options: &ShellApprovalOptions,
    lifecycle: Option<&LifecycleConfig>,
    lifecycle_source_path: Option<&std::path::Path>,
) -> Result<PreFlightResult, CompositionError> {
    let mut all_commands: Vec<(String, std::path::PathBuf, usize)> = Vec::new();

    // -- Source 1: Template ::shell directives ---------------------------------
    // Darkmatter discovers condition-blind: every command that could run under
    // any document state (including dead branches). Claudine authorizes the
    // union of these plus lifecycle stack commands below.
    if let (Some(md), Some(opts)) = (markdown, compose_options) {
        let preflight = md
            .compose_preflight(opts)
            .map_err(CompositionError::PreFlightDiscoveryFailed)?;
        for entry in &preflight.entries {
            all_commands.push((
                entry.normalized.clone(),
                entry.source_file.clone(),
                entry.origin.line_number(),
            ));
        }
    }

    // -- Source 2: Lifecycle stack shell commands ------------------------------
    // Condition-blind, matching the template posture: every reachable
    // `action: shell` (and its `on_error`) is gathered regardless of the
    // `when:` guard, because the guard may evaluate differently under
    // document states the audit cannot predict.
    if let Some(lifecycle_config) = lifecycle {
        let source_file = lifecycle_source_path
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| std::path::PathBuf::from("<lifecycle-stack>"));
        for (command, _property_path) in collect_lifecycle_shell_commands(lifecycle_config) {
            all_commands.push((command, source_file.clone(), 0));
        }
    }

    approve_discovered_commands(all_commands, approval_options)
}

/// Audit only the shell commands reachable from `signals`.
///
/// The narrow bootstrap gate uses this with `[LifecycleSignal::Initialize]`: a
/// freshly adopted document runs its `initialize` *before* the full audit can
/// run, because `initialize` may mutate the very document the full audit would
/// have to read. "Initialize before full pre-flight" must never mean "execute
/// unapproved shell", so the commands that event could select are approved
/// first, on their own.
///
/// Approvals land in `approval_options.approval_cache`, keyed on the normalized
/// command string, so the post-stabilization audit over
/// [`LifecycleSignal::ALL`] reuses them instead of prompting a second time.
///
/// `lifecycle` must be the C3-resolved config from canonical preparation — the
/// approved bytes are the executed bytes only when the `{{ }}` spans have
/// already been stamped.
///
/// ## Errors
///
/// The same typed failures as [`resolve_shell_approvals`]: a denial, a
/// blacklisted command, or a missing approval handler.
pub fn resolve_lifecycle_shell_approvals(
    lifecycle: &LifecycleConfig,
    lifecycle_source_path: &std::path::Path,
    signals: &[LifecycleSignal],
    approval_options: &ShellApprovalOptions,
) -> Result<PreFlightResult, CompositionError> {
    let commands = collect_lifecycle_shell_commands_for(lifecycle, signals)
        .into_iter()
        .map(|(command, _property)| (command, lifecycle_source_path.to_path_buf(), 0))
        .collect();
    approve_discovered_commands(commands, approval_options)
}

/// Deduplicate `all_commands` and run each survivor through shell policy.
fn approve_discovered_commands(
    all_commands: Vec<(String, std::path::PathBuf, usize)>,
    approval_options: &ShellApprovalOptions,
) -> Result<PreFlightResult, CompositionError> {
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
        let parts: Vec<String> =
            tokenize_words_strict(normalized).unwrap_or_else(|_| vec![normalized.clone()]);

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
                // No handler -- cannot get approval. Under `--dry-run` the
                // CI/non-TTY gate names the offending command and points at
                // the two ways to proceed (spec: Non-TTY Behavior).
                if approval_options.dry_run {
                    return Err(CompositionError::PreFlightFailed(format!(
                        "Cannot dry-run: shell command '{command}' requires interactive approval. \
                         Run with --yolo to auto-approve, or pre-approve the command in your \
                         configuration."
                    )));
                }
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

/// Resolves the `{{ }}` interpolation inside every lifecycle `shell` command
/// at pre-flight (C3), stamping the resolved bytes back into the parsed
/// command so the approved command equals the executed command.
///
/// Shell commands live inside the deferred lifecycle subtree (see
/// [`LATE_BINDING_ROOTS`] and `ComposeOptions::with_exclude_keys`), so after
/// main compose they still carry their authored `{{ }}` spans. Unlike the
/// communication/action surfaces — which interpolate at event-time (C2) —
/// `shell` commands are approved at pre-flight, before any event fires, and so
/// resolve against an **early-binding-only** lookup: `doc.*`, `ctx.*`, `env.*`,
/// and read-side functions. A late-binding reference (`err`/`timing`/`current`)
/// is rejected with [`CompositionError::LifecycleShellResolution`] because its
/// value does not yet exist.
///
/// Positional `shell` actions (`shell: "..."`) and key/value `shell` actions
/// (`{ action: shell, command: ... }`) with any `on_error:` text are covered.
/// Non-string command expressions (e.g. a bare `command: "ctx.repo"` parsed as a
/// variable) and literals with no interpolation span are left untouched — there
/// is nothing to stamp.
///
/// ## Errors
///
/// Returns [`CompositionError::LifecycleShellResolution`] when a command's
/// interpolation references a late-binding global, fails to parse, references
/// an unknown root (a typo), or calls an unknown function. The error names the
/// dotted property path (e.g. `failure.stack[0].action[1].command`) and the
/// raw command string.
pub fn resolve_lifecycle_shell_commands(
    lifecycle: &mut LifecycleConfig,
    effective_frontmatter: &serde_json::Value,
    context: &ComposeContext,
    source_path: &Path,
    file_ref_fallback_dir: Option<&Path>,
) -> Result<(), CompositionError> {
    let frontmatter: HashMap<String, serde_json::Value> = effective_frontmatter
        .as_object()
        .map(|map| map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();

    let state = EffectiveStateBuilder::new()
        .with_frontmatter(frontmatter)
        .with_context(context.clone())
        // A deferred lifecycle subtree never defines `ctx`; downgrade any
        // pathological `ctx` shape to a warning rather than aborting pre-flight.
        .with_allow_ctx_override(true)
        .build()
        .map_err(|e| {
            CompositionError::PreFlightFailed(format!(
                "lifecycle shell pre-flight: building early-binding state failed: {e}"
            ))
        })?;

    // Read-side functions (`parent_dir`, `dirname`, `file_exists`, …) resolve
    // against the prompt's parent directory first (document-first contract),
    // then fall back to the launch-area anchor when supplied — matching the
    // event-time lifecycle lookup so a launch-relative read-side reference
    // (`file_exists(spec)`) resolves identically at pre-flight and event-time
    // instead of depending on the prompt-only anchor.
    let mut resolution_ctx = ResolutionContext::new(
        source_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| std::path::PathBuf::from(".")),
    );
    if let Some(fallback) = file_ref_fallback_dir {
        resolution_ctx = resolution_ctx.with_file_ref_fallback_dir(fallback.to_path_buf());
    }

    for signal in LifecycleSignal::ALL {
        let event_name = signal.property_name();
        let Some(stack) = lifecycle.stack_mut(signal) else {
            continue;
        };
        for (idx, item) in stack.iter_mut().enumerate() {
            for (action_idx, action) in item.actions.iter_mut().enumerate() {
                let LifecycleActionKind::Shell(shell) = &mut action.kind else {
                    continue;
                };
                let prefix = format!("{event_name}.stack[{idx}].action[{action_idx}]");
                resolve_shell_command_expr(
                    &mut shell.command,
                    &state,
                    &resolution_ctx,
                    &format!("{prefix}.command"),
                    source_path,
                )?;
                if let Some(on_error) = &mut shell.on_error {
                    resolve_shell_command_expr(
                        on_error,
                        &state,
                        &resolution_ctx,
                        &format!("{prefix}.on_error"),
                        source_path,
                    )?;
                }
            }
        }
    }
    Ok(())
}

/// Resolves the `{{ }}` spans inside one shell-command [`Expr`] and stamps the
/// resolved string literal back in place.
///
/// Only string-literal expressions carrying an interpolation span are touched.
/// Late-binding references are rejected before resolution so the diagnostic
/// names the offending global rather than a generic "unknown root".
fn resolve_shell_command_expr(
    expr: &mut Expr,
    state: &darkmatter::markdown::compose::EffectiveState,
    resolution_ctx: &ResolutionContext,
    property: &str,
    source_path: &Path,
) -> Result<(), CompositionError> {
    let Expr::StringLiteral(raw) = expr else {
        return Ok(());
    };
    if !raw.contains("{{") {
        return Ok(());
    }

    if let Some(root) = first_late_binding_root(raw) {
        return Err(CompositionError::LifecycleShellResolution {
            source_path: source_path.to_path_buf(),
            property: property.to_string(),
            raw: raw.clone(),
            message: format!(
                "late-binding reference `{root}` is not available in shell commands; \
                 shell commands are resolved at pre-flight (before any event fires), so only \
                 early-binding values (`doc.*`, `ctx.*`, `env.*`, read-side functions) may be \
                 used here"
            ),
        });
    }

    let value = serde_json::Value::String(raw.clone());
    let resolved = SubtreeCompose::new(&value, state)
        .with_resolution_context(resolution_ctx.clone())
        .strict()
        .compose()
        .map_err(|e| CompositionError::LifecycleShellResolution {
            source_path: source_path.to_path_buf(),
            property: property.to_string(),
            raw: raw.clone(),
            message: e.to_string(),
        })?;

    let resolved = match resolved {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    };
    *expr = Expr::StringLiteral(resolved);
    Ok(())
}

/// Returns the first late-binding root (`err`/`timing`/`current`) referenced by
/// any `{{ }}` span in `raw`, or `None` when the command uses only
/// early-binding values.
fn first_late_binding_root(raw: &str) -> Option<String> {
    for loc in ExpressionFinder::find_all_plain(raw) {
        let Ok(expr) = parse(&loc.expression) else {
            continue;
        };
        if let Some(root) = late_binding_root_in_expr(&expr) {
            return Some(root);
        }
    }
    None
}

/// Walks an expression tree for the first variable whose root is a late-binding
/// global. `doc.*` is exempt: it reaches a literal frontmatter property, not a
/// lifecycle global.
fn late_binding_root_in_expr(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Variable(path) => {
            let root = path.split('.').next().unwrap_or(path);
            LATE_BINDING_ROOTS
                .contains(&root)
                .then(|| root.to_string())
        }
        Expr::MemberAccess { base, .. } => {
            if let Expr::Variable(base_path) = base.as_ref() {
                let root = base_path.split('.').next().unwrap_or(base_path);
                if root == "doc" {
                    return None;
                }
            }
            late_binding_root_in_expr(base)
        }
        Expr::UnaryNot(inner) | Expr::UnaryMinus(inner) | Expr::Paren(inner) => {
            late_binding_root_in_expr(inner)
        }
        Expr::Binary { left, right, .. } | Expr::Comparison { left, right, .. } => {
            late_binding_root_in_expr(left).or_else(|| late_binding_root_in_expr(right))
        }
        Expr::Index { base, index } => {
            late_binding_root_in_expr(base).or_else(|| late_binding_root_in_expr(index))
        }
        Expr::FunctionCall { args, .. } => args.iter().find_map(late_binding_root_in_expr),
        Expr::Fallback { primary, fallback } => {
            late_binding_root_in_expr(primary).or_else(|| late_binding_root_in_expr(fallback))
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => late_binding_root_in_expr(condition)
            .or_else(|| late_binding_root_in_expr(then_branch))
            .or_else(|| late_binding_root_in_expr(else_branch)),
        Expr::StringLiteral(_) | Expr::NumberLiteral(_) | Expr::BoolLiteral(_) => None,
    }
}

#[cfg(test)]
mod tests;
