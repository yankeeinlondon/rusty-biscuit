//! Pre-flight shell command approval for provider sessions.
//!
//! Scans all sources of shell commands (template directives and lifecycle
//! stacks), checks them against the whitelist, prompts the user for any that
//! need approval, and returns the full pre-approved set.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::{Expr, ResolutionContext};
use darkmatter::markdown::compose::subtree::SubtreeCompose;
use darkmatter::markdown::compose::{ComposeContext, ComposeRequest, EffectiveStateBuilder};

use crate::composition::error::{CompositionError, ShellApprovalFailure};
use crate::composition::lifecycle::{
    LifecycleConfig, LifecycleSignal, collect_lifecycle_shell_commands,
    collect_lifecycle_shell_commands_for,
};
use crate::composition::lifecycle::bindings::{
    EnclosingLoop, LifecycleScope, LifecycleValues, approval_diagnostics, runtime_bindings_enclosed,
    unavailable_root,
};
use crate::composition::lifecycle_actions::{LifecycleActionKind, for_each_action_mut};
use crate::composition::lifecycle::shell_template::{LifecycleShellTemplate, reads_as_command, typed_template};
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
        for (command, _property_path) in collect_lifecycle_shell_commands(lifecycle_config) {
            all_commands.push((command, source_file.clone(), 0));
        }
        candidates.extend(lifecycle_shell_candidates(lifecycle_config, &LifecycleSignal::ALL, &source_file));
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
        let excluded: Vec<String> = super::lifecycle::LIFECYCLE_EVENT_KEYS.iter().map(|key| (*key).to_string())
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
        all_commands.extend(collect_lifecycle_shell_commands(&lifecycle).into_iter()
            .map(|(command, _)| (command, document.path.clone(), 0)));
        candidates.extend(lifecycle_shell_candidates(&lifecycle, &LifecycleSignal::ALL, &document.path));
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
    let commands = collect_lifecycle_shell_commands_for(lifecycle, signals)
        .into_iter()
        .map(|(command, _property)| (command, lifecycle_source_path.to_path_buf(), 0))
        .collect();
    let candidates = lifecycle_shell_candidates(lifecycle, signals, lifecycle_source_path);
    let result = approve_commands(commands, &candidates, approval_options)?;
    settle_lifecycle_shell_modes(
        lifecycle,
        signals,
        &result.approved_commands,
        approval_options.policy_root.as_deref(),
    );
    Ok(result)
}

/// Every lifecycle shell site reachable from `signals` that holds a
/// placeholder, as an approval candidate: its resolved bytes (when they still
/// read as a command) and its typed shape.
fn lifecycle_shell_candidates(
    lifecycle: &LifecycleConfig,
    signals: &[LifecycleSignal],
    source_file: &Path,
) -> Vec<ShellCandidate> {
    let mut candidates = Vec::new();
    for_each_lifecycle_shell_site(lifecycle, signals, &mut |site, property| {
        candidates.push(site.candidate(source_file, &property));
    });
    for signal in signals {
        crate::composition::lifecycle_actions::for_each_action(lifecycle.stack(*signal), &mut |action| {
            let LifecycleActionKind::Shell(shell) = &action.kind else { return; };
            let (None, Some(authored), Expr::StringLiteral(bytes)) =
                (&shell.template, &shell.authored_command, &shell.command) else { return; };
            if !authored.contains("{{") { return; }
            let parts = tokenize_words_strict(bytes).unwrap_or_else(|_| vec![bytes.clone()]);
            let Some((program, args)) = parts.split_first() else { return; };
            candidates.push(ShellCandidate {
                authored_command: authored.clone(),
                site: darkmatter::markdown::compose::shell_expansion::template::CommandSite {
                    source_file: source_file.to_path_buf(),
                    origin: darkmatter::markdown::compose::ShellCommandOrigin::Frontmatter {
                        key: action.path.field("command").to_string(), line: None,
                    },
                    owner: None,
                },
                template: darkmatter::markdown::compose::shell_expansion::template::parse_authored(authored),
                resolved: Some(vec![darkmatter::markdown::compose::shell_expansion::normalize_command(program, args)]),
                requirement: ModeRequirement::AnyMode,
            });
        });
    }
    candidates
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
    for_each_lifecycle_shell_site(lifecycle, signals, &mut |site, _property| {
        site.settle(approved, policy_root);
    });
}

/// Visit every `shell` command and `set` value with a typed template,
/// condition-blind, with its authored property.
fn for_each_lifecycle_shell_site(
    lifecycle: &LifecycleConfig,
    signals: &[LifecycleSignal],
    visit: &mut impl FnMut(&LifecycleShellTemplate, String),
) {
    for signal in LifecycleSignal::ALL.into_iter().filter(|signal| signals.contains(signal)) {
        crate::composition::lifecycle_actions::for_each_action(lifecycle.stack(signal), &mut |action| {
            match &action.kind {
                LifecycleActionKind::Shell(shell) => {
                    if let Some(site) = &shell.template {
                        visit(site, action.path.field("command").to_string());
                    }
                }
                LifecycleActionKind::RuntimeSet(set) => {
                    for (key, value) in set.shell_values() {
                        if let Some(site) = &value.template {
                            visit(site, action.path.map_key(key.as_str()).to_string());
                        }
                    }
                }
                _ => {}
            }
        });
    }
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
/// at pre-flight (C3), stamping the resolved bytes back into the parsed
/// command so the approved command equals the executed command.
///
/// Shell commands live inside the deferred lifecycle subtree (see
/// `ComposeOptions::with_exclude_keys`), so after
/// main compose they still carry their authored `{{ }}` spans. Unlike the
/// communication/action surfaces — which interpolate at event-time (C2) —
/// `shell` commands are approved at pre-flight, before any event fires, and so
/// resolve in the lifecycle shell approval scope of the binding catalog
/// ([`LifecycleScope::LifecycleShellApproval`]): `fm.*`, `ctx.*`, `env.*`,
/// document properties, read-side functions, and `loop` as `enclosing_loop`
/// supplies it. A late-binding reference
/// (`err`, `timing`, `group`, `current`) is rejected with
/// [`CompositionError::LifecycleShellResolution`], in any branch, because its
/// value does not yet exist.
///
/// Each resolved action is marked [`ShellAction::pre_resolved`], so the
/// executor runs the stamped bytes as they stand instead of interpolating the
/// data they now contain a second time. A command with a placeholder also
/// keeps its authored text as a template typed against `owner_schema`
/// ([`ShellAction::template`]); the lifecycle audit then settles which of the
/// two runs (see [`LifecycleShellTemplate`]).
///
/// [`ShellAction::pre_resolved`]: super::lifecycle::ShellAction::pre_resolved
/// [`ShellAction::template`]: super::lifecycle::ShellAction::template
///
/// Short-form `shell` actions (`shell: "..."`) and long-form `shell` actions
/// (`shell: { command: ... }`) with any `on_error:` text are covered,
/// and so is every whole-value `$( … )` in a `set` mapping, whose command
/// bytes Darkmatter fixes here ([`resolve_set_shell_value`]).
/// Non-string command expressions (e.g. a bare `command: "ctx.repo"` parsed as a
/// variable) and literals with no interpolation span are left untouched — there
/// is nothing to stamp.
///
/// ## Errors
///
/// Returns [`CompositionError::LifecycleShellResolution`] when a command's
/// interpolation references a late-binding global, fails to parse, or calls an
/// unknown function. The error names the
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
    let frontmatter: HashMap<String, serde_json::Value> = effective_frontmatter
        .as_object()
        .map(|map| map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();

    let state = EffectiveStateBuilder::new()
        .with_frontmatter(frontmatter.clone())
        .with_context(context.clone())
        // A deferred lifecycle subtree never defines `ctx`; downgrade any
        // pathological `ctx` shape to a warning rather than aborting pre-flight.
        .with_allow_ctx_override(true)
        .build()
        .map_err(|e| CompositionError::PreFlightStateBuildFailed { source: e })?;

    let resolution_ctx = super::document_expression_resolution_context(
        source_path,
        Some(context),
        file_resolution_context,
        file_ref_fallback_dir,
    )?;

    for signal in LifecycleSignal::ALL {
        let mut result = Ok(());
        for_each_action_mut(lifecycle.stack_mut(signal), &mut |action| {
            if result.is_err() {
                return;
            }
            let prefix = action.path.to_string();
            result = match &mut action.kind {
                LifecycleActionKind::RuntimeSet(set) => set.shell_values_mut().try_for_each(|(key, shell)| {
                    if let Some(site) = sequence_lifecycle_template(&shell.authored, Some(key), &format!("{prefix}.{key}"),
                        source_path, owner_schema, options, &state, &resolution_ctx, enclosing_loop)? {
                        shell.resolved = darkmatter::markdown::compose::ResolvedShellValue::template_only(
                            key, &shell.authored, "sequence template", context);
                        shell.template = Some(site);
                        return Ok(());
                    }
                    resolve_set_shell_value(
                        key,
                        shell,
                        &frontmatter,
                        context,
                        &state,
                        &resolution_ctx,
                        &format!("{prefix}.{key}"),
                        source_path,
                        enclosing_loop,
                        owner_schema,
                    )
                }),
                LifecycleActionKind::Shell(shell) => (|| {
                    if shell.authored_command.is_none()
                        && let Expr::StringLiteral(raw) = &shell.command
                    {
                        shell.authored_command = Some(raw.clone());
                    }
                    if let Expr::StringLiteral(raw) = &shell.command
                        && let Some(site) = sequence_lifecycle_template(raw, None, &format!("{prefix}.command"),
                            source_path, owner_schema, options, &state, &resolution_ctx, enclosing_loop)? {
                        shell.template = Some(site);
                        shell.pre_resolved = false;
                        return Ok(());
                    }
                    let authored = match &shell.command {
                        Expr::StringLiteral(raw) => typed_template(raw, owner_schema),
                        _ => None,
                    };
                    let resolved = resolve_shell_command_expr(
                        &mut shell.command,
                        &state,
                        &resolution_ctx,
                        &format!("{prefix}.command"),
                        source_path,
                        enclosing_loop,
                    )
                    .and_then(|()| match &mut shell.on_error {
                        Some(on_error) => resolve_shell_command_expr(
                            on_error,
                            &state,
                            &resolution_ctx,
                            &format!("{prefix}.on_error"),
                            source_path,
                            enclosing_loop,
                        ),
                        None => Ok(()),
                    });
                    if resolved.is_ok() {
                        shell.pre_resolved = true;
                        // The bytes run only when approval proves them; a
                        // shape approval runs the authored template instead.
                        shell.template = authored.map(|template| {
                            let bytes = match &shell.command {
                                Expr::StringLiteral(bytes) if reads_as_command(bytes) => Some(vec![bytes.clone()]),
                                _ => None,
                            };
                            LifecycleShellTemplate::new(template, bytes)
                        });
                    }
                    resolved
                })(),
                _ => Ok(()),
            };
        });
        result?;
    }
    Ok(())
}

/// Fixes one lifecycle `set` shell value's command bytes (R9).
///
/// The authored text resolves against the same early-binding lookup as a
/// `shell` action's command, with late-binding roots refused, and Darkmatter
/// then fixes every pipeline the value can run. Those bytes are what approval
/// sees and what the action runs; nothing re-interpolates them later.
///
/// A value whose command holds a placeholder also keeps its authored command
/// as a typed template, so approval by shape can run it as one. When the
/// resolved text no longer reads as a command (a value inserted command
/// syntax), it has no bytes and may run only as that template.
#[allow(clippy::too_many_arguments)]
fn resolve_set_shell_value(
    key: &str,
    shell: &mut crate::composition::lifecycle::actions::SetShellValue,
    frontmatter: &HashMap<String, serde_json::Value>,
    context: &ComposeContext,
    state: &darkmatter::markdown::compose::EffectiveState,
    resolution_ctx: &ResolutionContext,
    property: &str,
    source_path: &Path,
    enclosing_loop: EnclosingLoop<'_>,
    owner_schema: Option<&darkmatter::markdown::schemas::SimplifiedSchema>,
) -> Result<(), CompositionError> {
    let mut resolved = Expr::StringLiteral(shell.authored.clone());
    resolve_shell_command_expr(
        &mut resolved,
        state,
        resolution_ctx,
        property,
        source_path,
        enclosing_loop,
    )?;
    let Expr::StringLiteral(resolved) = resolved else {
        unreachable!("a string literal stays a string literal");
    };
    let template = darkmatter::markdown::compose::authored_shell_template(
        &shell.authored,
        key,
        &biscuit_terminal::errors::SourceContext::new(
            source_path.to_path_buf(),
            source_path.to_path_buf(),
            shell.authored.clone(),
        ),
    )
    .and_then(|(inner, _)| typed_template(&inner, owner_schema));
    let resolution_failure = |error: darkmatter::markdown::compose::ShellExpansionError| {
        CompositionError::LifecycleShellResolution {
            source_path: source_path.to_path_buf(),
            property: property.to_string(),
            raw: shell.authored.clone(),
            message: crate::composition::lifecycle::actions::shell_value_error_message(&error),
            source: Some(Box::new(darkmatter::markdown::MarkdownError::ShellExpansion(Box::new(error)))),
        }
    };
    let (value, site) = match darkmatter::markdown::compose::ResolvedShellValue::resolve(
        key,
        &shell.authored,
        &resolved,
        frontmatter.clone(),
        context,
        resolution_ctx.clone(),
    ) {
        Ok(value) => {
            let site = template.map(|template| LifecycleShellTemplate::new(template, Some(value.commands())));
            (value, site)
        }
        Err(error) => {
            let template_only = template.as_ref().and_then(|_| {
                darkmatter::markdown::compose::ResolvedShellValue::template_only(
                    key,
                    &shell.authored,
                    &crate::composition::lifecycle::actions::shell_value_error_message(&error),
                    context,
                )
            });
            match (template_only, template) {
                (Some(value), Some(template)) => (value, Some(LifecycleShellTemplate::new(template, None))),
                _ => return Err(resolution_failure(error)),
            }
        }
    };
    shell.resolved = Some(value);
    shell.template = site;
    Ok(())
}

/// Resolves the `{{ }}` spans inside one shell-command [`Expr`] and stamps the
/// resolved string literal back in place.
///
/// Only string-literal expressions carrying an interpolation span are touched.
/// The command is first checked passively against the lifecycle shell approval
/// scope, in every branch, so a late global (`err`, `timing`, `group`) or a
/// late namespace (`current`) is refused by name before
/// anything is evaluated; the bytes are then resolved through a session in the
/// same scope, where `loop` follows `enclosing_loop`: a shell's bytes are fixed
/// per iteration, so an iteration's values are known.
fn resolve_shell_command_expr(
    expr: &mut Expr,
    state: &darkmatter::markdown::compose::EffectiveState,
    resolution_ctx: &ResolutionContext,
    property: &str,
    source_path: &Path,
    enclosing_loop: EnclosingLoop<'_>,
) -> Result<(), CompositionError> {
    let Expr::StringLiteral(raw) = expr else {
        return Ok(());
    };
    if !raw.contains("{{") {
        return Ok(());
    }

    let scope = LifecycleScope::LifecycleShellApproval;
    // A parse failure is left to the resolution below, which reports it typed.
    if let Ok(diagnostics) = approval_diagnostics(raw, scope)
        && let Some(diagnostic) = diagnostics.into_iter().next()
    {
        let message = match unavailable_root(&diagnostic.error) {
            Some(root) => format!(
                "late-binding reference `{root}` is not available in shell commands; \
                 shell commands are resolved at pre-flight (before any event fires), so only \
                 early-binding values (`fm.*`, `ctx.*`, `env.*`, read-side functions) may be \
                 used here"
            ),
            None => diagnostic.error.to_string(),
        };
        return Err(CompositionError::LifecycleShellResolution {
            source_path: source_path.to_path_buf(),
            property: property.to_string(),
            raw: raw.clone(),
            message,
            source: Some(Box::new(approval_failure(raw, diagnostic.error))),
        });
    }

    let (view, globals) = runtime_bindings_enclosed(scope, LifecycleValues::default(), enclosing_loop);

    // A shell command never runs with a missing value. The command and each
    // of its placeholders, as a whole value, resolve in one subtree so the
    // placeholders keep their types: a `null` refuses the command, while an
    // empty string, zero, and `false` are present.
    let placeholders: Vec<String> = darkmatter::markdown::compose::expression::ExpressionFinder::scan_plain(raw)
        .expressions
        .into_iter()
        .map(|location| location.expression)
        .collect();
    let value = serde_json::Value::Array(
        std::iter::once(serde_json::Value::String(raw.clone()))
            .chain(
                placeholders
                    .iter()
                    .map(|expression| serde_json::Value::String(format!("{{{{ {expression} }}}}"))),
            )
            .collect(),
    );
    let composed = SubtreeCompose::new(&value, state)
        .with_resolution_context(resolution_ctx.clone())
        .with_globals(globals)
        .with_binding_view(view)
        .compose()
        .map_err(|e| CompositionError::LifecycleShellResolution {
            source_path: source_path.to_path_buf(),
            property: property.to_string(),
            raw: raw.clone(),
            message: e.to_string(),
            source: Some(Box::new(e)),
        })?;
    let serde_json::Value::Array(mut composed) = composed else {
        unreachable!("an array subtree composes to an array");
    };
    // Before a loop's first iteration `loop` has no value yet: it is unknown
    // here, not missing, and each iteration's preparation checks it.
    let pending_loop = matches!(enclosing_loop, EnclosingLoop::Pending)
        .then(|| darkmatter::markdown::compose::shell_expansion::template::DeferredShellRoots::new(["loop"]));
    if let Some(expression) = placeholders
        .iter()
        .zip(composed.iter().skip(1))
        .filter(|(expression, _)| {
            pending_loop
                .as_ref()
                .is_none_or(|roots| roots.late_roots_of(expression, &Default::default()).is_none())
        })
        .find_map(|(expression, read)| read.is_null().then_some(expression))
    {
        let error = darkmatter::markdown::compose::ShellExpansionError::MissingValue {
            ctx: Box::new(biscuit_terminal::errors::SourceContext::new(
                source_path.to_path_buf(),
                source_path.to_path_buf(),
                raw.clone(),
            )),
            command: raw.clone(),
            expression: expression.clone(),
            origin: darkmatter::markdown::compose::ShellCommandOrigin::Frontmatter {
                key: property.to_string(),
                line: None,
            },
        };
        return Err(CompositionError::LifecycleShellResolution {
            source_path: source_path.to_path_buf(),
            property: property.to_string(),
            raw: raw.clone(),
            message: format!(
                "shell command reads `{expression}`, which is missing; a shell command never runs with a missing value"
            ),
            source: Some(Box::new(darkmatter::markdown::MarkdownError::ShellExpansion(Box::new(error)))),
        });
    }
    let resolved = composed.swap_remove(0);

    let resolved = match resolved {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    };
    *expr = Expr::StringLiteral(resolved);
    Ok(())
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

/// A passive approval finding as the Darkmatter error composition would have
/// raised for `raw`, so callers read one typed cause either way.
pub(crate) fn approval_failure(
    raw: &str,
    cause: darkmatter::markdown::compose::expression::ExpressionError,
) -> darkmatter::markdown::MarkdownError {
    darkmatter::markdown::MarkdownError::Interpolation {
        key: None,
        expression: raw.to_string(),
        source: Box::new(darkmatter::markdown::SourceRef::Effective {
            rendered: raw.to_string(),
            origin_key: None,
        }),
        cause: Box::new(cause),
    }
}

#[cfg(test)]
mod tests;

/// Sequence lifecycle sites retain authored expressions and the step barrier's values.
#[allow(clippy::too_many_arguments)]
fn sequence_lifecycle_template(
    raw: &str, set_key: Option<&str>, property: &str, source: &Path,
    schema: Option<&darkmatter::markdown::schemas::SimplifiedSchema>,
    options: Option<&super::PrepareOptions>, state: &darkmatter::markdown::compose::EffectiveState,
    resolution: &ResolutionContext, enclosing_loop: EnclosingLoop<'_>,
) -> Result<Option<LifecycleShellTemplate>, CompositionError> {
    use darkmatter::markdown::compose::shell_expansion::template::{ArgPart, HoleTiming, LookupEvaluator, resolve_late_values, fill_template};
    let Some(options) = options.filter(|options| !options.deferred_shell_roots.is_empty()) else { return Ok(None) };
    let context = biscuit_terminal::errors::SourceContext::new(source.to_path_buf(), source.to_path_buf(), raw.to_string());
    let inner = match set_key {
        Some(key) => darkmatter::markdown::compose::authored_shell_template(raw, key, &context).map(|(inner, _)| inner),
        None => Some(raw.to_string()),
    };
    let Some(inner) = inner else { return Ok(None) };
    if !inner.contains("{{") { return Ok(None) }
    if let Ok(diagnostics) = approval_diagnostics(&inner, LifecycleScope::LifecycleShellApproval)
        && let Some(diagnostic) = diagnostics.into_iter().find(|diagnostic| {
            unavailable_root(&diagnostic.error).is_none_or(|root| !options.deferred_shell_roots.contains(root))
        }) {
        return Err(CompositionError::LifecycleShellResolution { source_path: source.to_path_buf(), property: property.to_string(),
            raw: raw.to_string(), message: diagnostic.error.to_string(), source: Some(Box::new(approval_failure(raw, diagnostic.error))) });
    }
    let derived = darkmatter::markdown::compose::shell_expansion::template::derived_deferred_keys(
        state.data().iter(), &options.deferred_shell_roots, &Default::default());
    // Resolved-byte eligibility must not depend on the narrower template grammar.
    let reads_late = darkmatter::markdown::compose::expression::ExpressionFinder::scan_plain(&inner)
        .expressions.iter().any(|location| options.deferred_shell_roots
            .late_roots_of(&location.expression, &derived).is_some());
    if !options.deferred_shell_roots.placeholders_template_required() && !reads_late {
        return Ok(None);
    }
    let Some(mut template) = typed_template(&inner, schema) else {
        return Err(CompositionError::LifecycleShellResolution { source_path: source.to_path_buf(), property: property.to_string(),
            raw: raw.to_string(), message: "invalid shell template grammar".to_string(), source: None });
    };
    for segment in &mut template.segments {
        for arg in &mut segment.args {
            for part in &mut arg.parts {
                if let ArgPart::Hole(hole) = part
                    && let Some(roots) = options.deferred_shell_roots.late_roots_of(&hole.expression, &derived) {
                    hole.timing = HoleTiming::Deferred { roots };
                }
            }
        }
    }
    let mut site = LifecycleShellTemplate::new(template, None);
    let candidate = site.candidate(source, property);
    if let Some(approved) = &options.pre_approved_commands
        && !candidate.authorized_by(approved) {
        return Err(CompositionError::LifecycleShellResolution { source_path: source.to_path_buf(), property: property.to_string(),
            raw: raw.to_string(), message: "the authored template was not approved at sequence preflight".to_string(), source: None });
    }
    let origin = candidate.site.origin.clone();
    let fail = |error: darkmatter::markdown::compose::shell_expansion::template::HoleError| {
        let message = error.to_string();
        CompositionError::LifecycleShellResolution { source_path: source.to_path_buf(), property: property.to_string(),
            raw: raw.to_string(), message,
            source: Some(Box::new(darkmatter::markdown::MarkdownError::ShellExpansion(Box::new(error.into_shell_error(&context, raw, &origin))))) }
    };
    let (view, globals) = runtime_bindings_enclosed(LifecycleScope::TaskSetup, LifecycleValues {
        group: options.lifecycle_group.as_ref(), seq: options.sequence_binding.as_ref(), ..Default::default()
    }, enclosing_loop);
    let session = darkmatter::markdown::compose::expression::EvaluationSession::associate(view, state, globals)
        .expect("task bindings agree").with_resolution_context(resolution.clone());
    let evaluator = LookupEvaluator::new(&session);
    let mut known = site.template().clone();
    for segment in &mut known.segments {
        for arg in &mut segment.args {
            for part in &mut arg.parts {
                if let ArgPart::Hole(hole) = part {
                    hole.timing = match hole.timing {
                        HoleTiming::Deferred { .. } => HoleTiming::ApprovalTime,
                        HoleTiming::ApprovalTime => HoleTiming::Deferred { roots: Vec::new() },
                    };
                }
            }
        }
    }
    resolve_late_values([(property, &known)], &evaluator).map_err(|error| fail(error.error))?;
    let saved = options.document_shell_values.as_ref().and_then(|values| values.get(&candidate));
    let values = if options.validate_deferred_shell_values {
        let late = resolve_late_values([(property, site.template())], &evaluator).map_err(|error| fail(error.error))?;
        let values = late.site(property).cloned().unwrap_or_default();
        if let Some(snapshot) = &options.document_shell_values { snapshot.insert(&candidate, values.clone()); }
        Some(values)
    } else { saved };
    // Validate known values without evaluating deferred holes during discovery.
    if options.validate_deferred_shell_values {
        fill_template(site.template(), &evaluator, values.as_ref()).map_err(fail)?;
    }
    if let Some(values) = values { site = site.with_late_values(values); }
    Ok(Some(site))
}

/// Keep derived inputs late after frontmatter interpolation replaces their expressions.
pub(crate) fn lifecycle_document_roots(
    markdown: &Markdown,
    roots: &darkmatter::markdown::compose::shell_expansion::template::DeferredShellRoots,
) -> darkmatter::markdown::compose::shell_expansion::template::DeferredShellRoots {
    use darkmatter::markdown::compose::shell_expansion::template::{DeferredShellRoots, derived_deferred_keys};
    let authored = super::prepare::frontmatter_to_value(markdown.frontmatter());
    let derived = authored.as_object().map(|map| derived_deferred_keys(map.iter(), roots, &Default::default())).unwrap_or_default();
    let extended = DeferredShellRoots::new(roots.iter().map(str::to_string).chain(derived));
    if roots.placeholders_template_required() { extended.with_placeholders_template_required() } else { extended }
}
