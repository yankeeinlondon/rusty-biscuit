//! Pre-flight shell command approval for provider sessions.
//!
//! Scans all sources of shell commands (template directives and lifecycle
//! stacks), checks them against the whitelist, prompts the user for any that
//! need approval, and returns the full pre-approved set.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use darkmatter::lifecycle::preflight::{
    LifecycleShellSites, ShellAuditBindings, ShellWalkOptions, collect_lifecycle_shell_audit,
};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::{BindingView, ExpressionError, RuntimeBinding};
use darkmatter::markdown::compose::{ComposeContext, ComposeRequest};

use crate::composition::error::{CompositionError, ShellApprovalFailure};
use crate::composition::lifecycle::{LifecycleConfig, LifecycleSignal};
use crate::composition::lifecycle::bindings::{
    EnclosingLoop, LifecycleScope, LifecycleValues, approval_diagnostics, runtime_bindings_enclosed,
};
use crate::harness::shell::{ResolvedSiteApproval, ShellApprovalOptions, tokenize_words_strict};
use darkmatter::markdown::compose::shell_expansion::template::{
    ModeRequirement, ShellCandidate, ShellPreApprovals,
};

/// Result of pre-flight shell command approval.
#[derive(Debug)]
pub struct PreFlightResult {
    /// The approvals granted: normalized resolved bytes, and template shapes
    /// for sites approved by shape.
    pub approved_commands: ShellPreApprovals,
    /// Total commands discovered across all sources.
    pub total_discovered: usize,
    /// Commands that were already whitelisted.
    pub already_whitelisted: usize,
    /// Commands the user approved interactively.
    pub user_approved: usize,
}

/// Scans all sources of shell commands, checks whitelist, prompts user,
/// returns the full pre-approved set: resolved bytes, and the shapes of
/// document sites approved by a template rule or as templates.
///
/// ## Sources
///
/// 1. Template `::shell` directives (via Darkmatter's document graph walker)
/// 2. Lifecycle stack shell commands (from every reachable `shell` action
///    across the six shell-capable lifecycle events). A lifecycle command with
///    a placeholder is a site like a document command, and the approval records
///    on it whether it runs its resolved bytes or its template.
///
/// ## Errors
///
/// - `ShellCommandDenied` if the user denies any command
/// - `PreFlightDiscoveryFailed` if Darkmatter's document graph walk fails
///   (`RemovedGrammar` when it failed on a retired `_loop_*` read)
/// - `ShellApprovalUnavailable` for blacklisted commands or missing approval handler
/// - `PreFlightShellAuditFailed` for a shell-audit failure outside the approval
///   family, carrying the raised `HarnessError` as its typed source
///
/// ## Arguments
///
/// * `markdown` — Markdown source for condition-blind template `::shell`
///   discovery, which can dereference body includes. For staged live entry,
///   pass the stabilized reread after initialization.
/// * `compose_request` — the prepared Darkmatter request for the template
///   walker.
/// * `approval_options` — shell approval policy, handler, and cache.
/// * `lifecycle` — parsed lifecycle configuration; when present, every
///   reachable `shell` action command (and `on_error` command) across all
///   six shell-capable lifecycle events is collected and audited alongside the other
///   sources. `lifecycle_source_path` names the composition source file for
///   diagnostics.
/// * `lifecycle_source_path` — source path for lifecycle-stack shell
///   commands; ignored when `lifecycle` is `None`.
pub fn resolve_shell_approvals(
    markdown: Option<&Markdown>,
    compose_request: Option<&ComposeRequest>,
    approval_options: &ShellApprovalOptions,
    lifecycle: Option<&LifecycleConfig>,
    lifecycle_source_path: Option<&std::path::Path>,
) -> Result<PreFlightResult, CompositionError> {
    let mut all_commands: Vec<(String, std::path::PathBuf, usize)> = Vec::new();
    let mut candidates: Vec<ShellCandidate> = Vec::new();

    // -- Source 1: Template ::shell directives ---------------------------------
    // Darkmatter discovers condition-blind: every command that could run under
    // any document state (including dead branches). Claudine authorizes the
    // union of these plus lifecycle stack commands below.
    if let (Some(md), Some(opts)) = (markdown, compose_request) {
        let preflight = md.compose_preflight(opts).map_err(|error| {
            let source_path = match md.source() {
                Some(darkmatter::markdown::compose::ComposeSource::File(path)) => path.as_path(),
                _ => lifecycle_source_path.unwrap_or(Path::new("")),
            };
            discovery_failed(source_path, error)
        })?;
        for entry in &preflight.entries {
            all_commands.push((
                entry.normalized.clone(),
                entry.source_file.clone(),
                entry.origin.line_number(),
            ));
        }
        candidates.extend(preflight.candidates);
    }

    // -- Source 2: Lifecycle stack shell commands ------------------------------
    // Condition-blind, matching the template posture: every reachable
    // `shell` action is gathered regardless of the
    // `when:` guard, because the guard may evaluate differently under
    // document states the audit cannot predict.
    if let Some(lifecycle_config) = lifecycle {
        let source_file = lifecycle_source_path
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| std::path::PathBuf::from("<lifecycle-stack>"));
        let audit = collect_lifecycle_shell_audit(lifecycle_config, &LifecycleSignal::ALL, &source_file);
        for (command, _property_path) in audit.commands {
            all_commands.push((command, source_file.clone(), 0));
        }
        candidates.extend(audit.candidates);
    }

    let result = approve_commands(all_commands, &candidates, approval_options)?;
    if let Some(lifecycle_config) = lifecycle {
        settle_lifecycle_shell_modes(
            lifecycle_config,
            &LifecycleSignal::ALL,
            &result.approved_commands,
            approval_options.policy_root.as_deref(),
        );
    }
    Ok(result)
}

/// Approve every shell command a sequence's preflight graph can reach.
///
/// Three sources feed the gate, and all are condition-blind:
///
/// 1. the graph's resolved-byte commands — the first task's `shell:`
///    commands and `setup:`/`teardown:` shell sites whose values all exist
///    now, which preflight resolved once; their valid templates also permit
///    choosing shape approval instead;
/// 2. the graph's typed candidates — first-task sites eligible for either
///    mode, and every later task command with a placeholder or late read
///    approved only by its typed shape;
/// 3. every referenced prompt document's shell directives, frontmatter shell
///    values, and lifecycle stacks, discovered once per reference with that reference's late roots and
///    parameters, so a document composed at its turn can never stop
///    mid-sequence for approval. A later task's document has every command
///    with a placeholder approved as a template.
///
/// ## Errors
///
/// As [`resolve_shell_approvals`]: denial, a missing handler, a blacklisted
/// command, or a document-graph walk failure.
pub fn resolve_graph_shell_approvals(
    graph: &crate::composition::sequence::preflight::PreflightGraph,
    approval_options: &ShellApprovalOptions,
    prompt_compose_request: &dyn Fn(
        &crate::composition::sequence::preflight::PromptReference,
    ) -> Result<ComposeRequest, CompositionError>,
) -> Result<PreFlightResult, CompositionError> {
    graph.record_shell_authorization(ShellPreApprovals::new(), None);
    let mut all_commands: Vec<(String, std::path::PathBuf, usize)> = Vec::new();
    let mut candidates: Vec<ShellCandidate> = graph.template_candidates.clone();

    for command in &graph.shell_commands {
        all_commands.push((command.command.clone(), command.source_file.clone(), 0));
    }

    for reference in &graph.prompt_references {
        let Some(document) = graph.prompt_document(&reference.path) else {
            continue;
        };
        let options = prompt_compose_request(reference)?;
        let preflight = document
            .markdown
            .compose_preflight(&options)
            .map_err(|error| discovery_failed(&document.path, error))?;
        for entry in &preflight.entries {
            all_commands.push((
                entry.normalized.clone(),
                entry.source_file.clone(),
                entry.origin.line_number(),
            ));
        }
        candidates.extend(preflight.candidates);
        // Resolve only frontmatter data; discovery never executes document shells.
        let roots = lifecycle_document_roots(&document.markdown, &reference.deferred_shell_roots());
        let excluded: Vec<String> = super::lifecycle::claudine_registry().authored_event_keys().into_iter().map(str::to_string)
            .chain(roots.iter().map(str::to_string)).collect();
        let passive = options.clone().map_options(|options| options.only(
            &[darkmatter::markdown::compose::ComposeOperation::FrontmatterInterpolation])
            .with_exclude_keys(excluded));
        let composed = document.markdown.compose_with(&passive)
            .map_err(|error| discovery_failed(&document.path, error))?.0;
        let effective = super::prepare::frontmatter_to_value(composed.frontmatter());
        let mut lifecycle = super::lifecycle::parse_lifecycle_config_with_orders(&effective, &document.path, Some(composed.frontmatter()))?;
        let mut lifecycle_options = super::PrepareOptions::new(options.resolution_context().clone());
        lifecycle_options.deferred_shell_roots = roots;
        let source = super::ResolvedCompositionSource { original_ref: document.path.display().to_string(),
            resolved_path: document.path.clone(), original_text: document.markdown.as_string(), markdown: document.markdown.clone() };
        let schema = super::schema::load_authored_schema(&source, None, options.resolution_context(), &Default::default())?
            .and_then(|schema| schema.simplified);
        let enclosing = if effective.get("loop").is_some_and(serde_json::Value::is_object) { EnclosingLoop::Pending } else { EnclosingLoop::Outside };
        resolve_lifecycle_shell_commands_in_sequence(
            &mut lifecycle, &effective, options.context(), &document.path,
            options.resolution_context(), None, enclosing, schema.as_ref(), Some(&lifecycle_options),
        )?;
        let audit = collect_lifecycle_shell_audit(&lifecycle, &LifecycleSignal::ALL, &document.path);
        all_commands.extend(audit.commands.into_iter().map(|(command, _)| (command, document.path.clone(), 0)));
        candidates.extend(audit.candidates);
    }

    let result = approve_commands(all_commands, &candidates, approval_options)?;
    graph.record_shell_authorization(result.approved_commands.clone(), approval_options.policy_root.clone());
    Ok(result)
}

/// Approve every document command site, then the remaining resolved bytes.
///
/// A site that may run only as a template is approved by its shape. A
/// resolved-byte site whose bytes no `exact` rule covers, but whose shape a
/// `template` rule covers, is approved by its shape too: an exact rule wins,
/// then a template rule, then a prefix rule. A resolved-byte site with a
/// placeholder that no rule covers is asked about as a site, so its prompt
/// offers the shape beside the exact bytes. The bytes of a site approved by
/// its shape are still checked against the blacklists; they join the exact
/// set only when another site needs them as bytes.
fn approve_commands(
    all_commands: Vec<(String, std::path::PathBuf, usize)>,
    candidates: &[ShellCandidate],
    approval_options: &ShellApprovalOptions,
) -> Result<PreFlightResult, CompositionError> {
    if let Some(approved) = &approval_options.closed_approvals {
        let uncovered = candidates.iter().find(|candidate| !candidate.authorized_by(approved))
            .map(|candidate| candidate.display_command())
            .or_else(|| all_commands.iter().find(|(command, _, _)| {
                !approved.contains_exact(command) && !candidates.iter().any(|candidate| {
                    candidate.authorized_by(approved) && candidate.resolved.as_ref().is_some_and(|keys| keys.contains(command))
                })
            }).map(|(command, _, _)| command.clone()));
        if let Some(command) = uncovered {
            return Err(CompositionError::SequenceShellResolution {
                command, task: "sequence descendant".into(),
                message: "introduced an unapproved command after sequence approval closed".into(), source: None,
            });
        }
        return Ok(PreFlightResult { approved_commands: approved.clone(),
            total_discovered: all_commands.len() + candidates.len(), already_whitelisted: all_commands.len(), user_approved: 0 });
    }
    let mut templates = ShellPreApprovals::new();
    let mut by_shape: HashSet<String> = HashSet::new();
    let mut by_bytes: HashSet<String> = HashSet::new();
    let mut prompted_bytes: HashSet<String> = HashSet::new();
    for candidate in candidates {
        let line = candidate.site.origin.line_number();
        let source_file = &candidate.site.source_file;
        let audit = |error| audit_failure(error, candidate.display_command(), source_file, line, approval_options);
        if candidate.requirement == ModeRequirement::TemplateRequired {
            let shape = crate::harness::shell::validate_and_approve_template_site(candidate, approval_options)
                .map_err(audit)?;
            templates.insert_template(shape);
            continue;
        }
        let Some(keys) = &candidate.resolved else {
            continue;
        };
        let shape = match crate::harness::shell::template_rule_covers(candidate, approval_options).map_err(audit)? {
            Some(shape) => Some(shape),
            None => {
                let prompted =
                    crate::harness::shell::prompt_resolved_site(candidate, approval_options).map_err(audit)?;
                if prompted.is_some() {
                    prompted_bytes.extend(keys.iter().cloned());
                }
                match prompted {
                    Some(ResolvedSiteApproval::Shape(shape)) => Some(shape),
                    Some(ResolvedSiteApproval::Bytes) | None => None,
                }
            }
        };
        match shape {
            Some(shape) => {
                templates.insert_template(shape);
                by_shape.extend(keys.iter().cloned());
            }
            None => by_bytes.extend(keys.iter().cloned()),
        }
    }
    let shape_only: HashSet<String> = by_shape.difference(&by_bytes).cloned().collect();
    let mut result = approve_discovered_commands_with(all_commands, &shape_only, approval_options)?;
    // A site answered at its own prompt leaves its bytes looking whitelisted
    // to the byte pass, which runs after the answer was recorded.
    result.user_approved += prompted_bytes.len();
    result.already_whitelisted = result.already_whitelisted.saturating_sub(prompted_bytes.len());
    result.total_discovered += candidates
        .iter()
        .filter(|candidate| candidate.requirement == ModeRequirement::TemplateRequired)
        .count();
    result.approved_commands.extend(templates);
    Ok(result)
}

/// Audit only the shell commands reachable from `signals`.
///
/// Initialization is shell-free. Audit later events only after initialization
/// and the stabilized reread; cached approvals remain invocation-scoped.
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
    let audit = collect_lifecycle_shell_audit(lifecycle, signals, lifecycle_source_path);
    let commands = audit
        .commands
        .into_iter()
        .map(|(command, _property)| (command, lifecycle_source_path.to_path_buf(), 0))
        .collect();
    let result = approve_commands(commands, &audit.candidates, approval_options)?;
    settle_lifecycle_shell_modes(
        lifecycle,
        signals,
        &result.approved_commands,
        approval_options.policy_root.as_deref(),
    );
    Ok(result)
}

/// Record on each lifecycle shell site the mode `approved` grants it: an
/// exact member runs its bytes, a shape member its template, and a site that
/// is neither runs in no mode.
pub(crate) fn settle_lifecycle_shell_modes(
    lifecycle: &LifecycleConfig,
    signals: &[LifecycleSignal],
    approved: &ShellPreApprovals,
    policy_root: Option<&Path>,
) {
    darkmatter::lifecycle::preflight::settle_lifecycle_shell_modes(lifecycle, signals, approved, policy_root);
}

/// Deduplicate `all_commands` and run each survivor through shell policy.
/// The bytes in `shape_only` belong only to sites already approved by their
/// shape: they are checked against the blacklists and neither prompted for
/// nor added to the exact set.
fn approve_discovered_commands_with(
    all_commands: Vec<(String, std::path::PathBuf, usize)>,
    shape_only: &HashSet<String>,
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
    let mut approved = ShellPreApprovals::new();

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

        if shape_only.contains(normalized) {
            crate::harness::shell::check_command_blacklists(&parts, approval_options).map_err(|error| {
                audit_failure(error, normalized.clone(), source_file, *line, approval_options)
            })?;
            continue;
        }

        match crate::harness::shell::validate_and_approve_command_parts(
            &parts,
            approval_options,
            Some(source_file.as_path()),
            Some(*line),
        ) {
            Ok(_) => {
                approved.insert_exact(normalized.clone());
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
                let failure = if approval_options.dry_run {
                    ShellApprovalFailure::DryRun
                } else {
                    ShellApprovalFailure::NoHandler
                };
                return Err(CompositionError::ShellApprovalUnavailable {
                    command,
                    source_file: source_file.clone(),
                    line: *line,
                    failure,
                });
            }
            Err(crate::harness::error::HarnessError::ShellCommandBlacklisted {
                command,
                reason,
            }) => {
                return Err(CompositionError::ShellApprovalUnavailable {
                    command,
                    source_file: source_file.clone(),
                    line: *line,
                    failure: ShellApprovalFailure::Blacklisted(reason),
                });
            }
            Err(e) => {
                return Err(CompositionError::PreFlightShellAuditFailed { source: e });
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

/// The typed preflight failure for a site or command the shell audit refused.
fn audit_failure(
    error: crate::harness::error::HarnessError,
    command: String,
    source_file: &Path,
    line: usize,
    approval_options: &ShellApprovalOptions,
) -> CompositionError {
    use crate::harness::error::HarnessError;
    match error {
        HarnessError::ShellCommandDenied { .. } if approval_options.approval_handler.is_some() => {
            CompositionError::ShellCommandDenied {
                command,
                source_file: source_file.to_path_buf(),
                line,
            }
        }
        HarnessError::ShellCommandDenied { .. } => CompositionError::ShellApprovalUnavailable {
            command,
            source_file: source_file.to_path_buf(),
            line,
            failure: if approval_options.dry_run {
                ShellApprovalFailure::DryRun
            } else {
                ShellApprovalFailure::NoHandler
            },
        },
        HarnessError::ShellCommandBlacklisted { reason, .. } => CompositionError::ShellApprovalUnavailable {
            command,
            source_file: source_file.to_path_buf(),
            line,
            failure: ShellApprovalFailure::Blacklisted(reason),
        },
        other => CompositionError::PreFlightShellAuditFailed { source: other },
    }
}

/// Resolves the `{{ }}` interpolation inside every lifecycle `shell` command
/// and `set` shell value at pre-flight (C3), stamping the resolved bytes back
/// so the approved command equals the executed command.
///
/// Darkmatter's [`LifecycleShellSites`] fixes each site; Claudine supplies the
/// binding catalog: the lifecycle shell approval scope, where `loop` follows
/// `enclosing_loop`, and a late-binding reference (`err`, `timing`, `group`,
/// `current`) is refused.
///
/// ## Errors
///
/// Returns [`CompositionError::LifecycleShellResolution`] when a command's
/// interpolation references a late-binding global, fails to parse, calls an
/// unknown function, or reads a missing value no enclosing guard proves
/// present, and when such a guarded command is not a valid template. The error names the
/// authored property path (e.g. `failure[0].else[1].shell.command`) and the
/// raw command string. Commands in both branches of every conditional are
/// resolved: approval never depends on which branch runs.
#[allow(clippy::too_many_arguments)]
pub fn resolve_lifecycle_shell_commands(
    lifecycle: &mut LifecycleConfig,
    effective_frontmatter: &serde_json::Value,
    context: &ComposeContext,
    source_path: &Path,
    file_resolution_context: &biscuit_file::FileResolutionContext,
    file_ref_fallback_dir: Option<&Path>,
    enclosing_loop: EnclosingLoop<'_>,
    owner_schema: Option<&darkmatter::markdown::schemas::SimplifiedSchema>,
) -> Result<(), CompositionError> {
    resolve_lifecycle_shell_commands_in_sequence(lifecycle, effective_frontmatter, context, source_path,
        file_resolution_context, file_ref_fallback_dir, enclosing_loop, owner_schema, None)
}

/// [`resolve_lifecycle_shell_commands`] for a document a sequence references:
/// `options` carries its late roots, lexical group and sequence bindings, and
/// the sequence's closed approvals.
#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve_lifecycle_shell_commands_in_sequence(
    lifecycle: &mut LifecycleConfig,
    effective_frontmatter: &serde_json::Value,
    context: &ComposeContext,
    source_path: &Path,
    file_resolution_context: &biscuit_file::FileResolutionContext,
    file_ref_fallback_dir: Option<&Path>,
    enclosing_loop: EnclosingLoop<'_>,
    owner_schema: Option<&darkmatter::markdown::schemas::SimplifiedSchema>,
    options: Option<&super::PrepareOptions>,
) -> Result<(), CompositionError> {
    let sites = LifecycleShellSites {
        effective_frontmatter,
        context,
        source_path,
        registry: super::lifecycle::claudine_registry(),
        owner_schema,
        bindings: &ClaudineShellAuditBindings { enclosing_loop },
        options: options.map(|options| ShellWalkOptions {
            deferred_shell_roots: &options.deferred_shell_roots,
            lifecycle_group: options.lifecycle_group.as_ref(),
            sequence_binding: options.sequence_binding.as_ref(),
            pre_approved_commands: options.pre_approved_commands.as_ref(),
            validate_deferred_shell_values: options.validate_deferred_shell_values,
            document_shell_values: options.document_shell_values.as_ref(),
        }),
    };
    sites.stamp(lifecycle, || {
        super::document_expression_resolution_context(source_path, Some(context), file_resolution_context, file_ref_fallback_dir)
    })?;
    Ok(())
}

/// Claudine's binding catalog as the lifecycle shell walk reads it.
struct ClaudineShellAuditBindings<'a> {
    enclosing_loop: EnclosingLoop<'a>,
}

impl ShellAuditBindings for ClaudineShellAuditBindings<'_> {
    fn approval_findings(&self, raw: &str) -> Vec<ExpressionError> {
        approval_diagnostics(raw, LifecycleScope::LifecycleShellApproval)
            .map(|diagnostics| diagnostics.into_iter().map(|diagnostic| diagnostic.error).collect())
            .unwrap_or_default()
    }

    fn approval_bindings(&self) -> (Arc<BindingView>, HashMap<String, RuntimeBinding<'static>>) {
        runtime_bindings_enclosed(LifecycleScope::LifecycleShellApproval, LifecycleValues::default(), self.enclosing_loop)
    }

    fn task_setup_bindings(
        &self,
        group: Option<&serde_json::Map<String, serde_json::Value>>,
        seq: Option<&serde_json::Value>,
    ) -> (Arc<BindingView>, HashMap<String, RuntimeBinding<'static>>) {
        runtime_bindings_enclosed(LifecycleScope::TaskSetup, LifecycleValues { group, seq, ..Default::default() }, self.enclosing_loop)
    }

    fn loop_pending(&self) -> bool {
        matches!(self.enclosing_loop, EnclosingLoop::Pending)
    }
}

/// A document-graph walk failure as the error the caller reports.
///
/// The walk composes transcluded documents, so it can be the first to meet a
/// retired `_loop_*` read that only a transclusion introduces; that read is
/// [`CompositionError::RemovedGrammar`], exactly as when the same read fails
/// the document's composition.
fn discovery_failed(
    source_path: &Path,
    error: darkmatter::markdown::MarkdownError,
) -> CompositionError {
    crate::composition::lifecycle::bindings::removed_loop_read_in_composition(source_path, &error)
        .unwrap_or(CompositionError::PreFlightDiscoveryFailed(error))
}

pub(crate) use darkmatter::lifecycle::preflight::approval_failure;

pub mod checks;

#[cfg(test)]
mod tests;

/// Keep derived inputs late after frontmatter interpolation replaces their expressions.
pub(crate) fn lifecycle_document_roots(
    markdown: &Markdown,
    roots: &darkmatter::markdown::compose::shell_expansion::template::DeferredShellRoots,
) -> darkmatter::markdown::compose::shell_expansion::template::DeferredShellRoots {
    darkmatter::lifecycle::preflight::lifecycle_document_roots(markdown, roots)
}
