//! Phase 1c — per-step compose with schema validation and aggregated
//! missing-property handling for the sequence orchestrator.
//!
//! Each step is composed (and, for inline sequences, prepared for
//! body write-back) with `$schema` validation. Missing-property failures
//! are aggregated across all steps so the user can fix the full sequence in
//! one edit, matching the direct `compose` path's interactive collection.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use claudine::composition::sequence::build_step_overlay;
use claudine::composition::{
    self, CompositionError, LIFECYCLE_EVENT_KEYS, MissingProperty, PrepareOptions,
    PreparedComposition, ResolvedCompositionSource, SequenceMissingPropertiesStep, SequencePlan,
};
use claudine::harness::ShellApprovalOptions;
use color_eyre::eyre::{Result, eyre};

use crate::commands::compose::SharedComposeArgs;
use crate::commands::schema_interactive::{
    collect_missing_values, emit_dropped_optional_warnings, render_status_report,
    resolve_interactive_options,
};
use crate::log;

/// Context data for a single step in the sequence.
///
/// Captures the per-step environment overrides and the prepared composition
/// so that Phase 2 execution can reuse the work done during Phase 1 pre-flight.
pub(super) struct StepContext {
    pub(super) env_overrides: BTreeMap<String, String>,
    pub(super) prepared: PreparedComposition,
}

/// Run Phase 1c (per-step compose) with schema validation and aggregated
/// missing-property handling.
///
/// On the first pass each step is composed via
/// [`composition::prepare_direct_with_schema`]. Missing-property errors
/// are collected per-step; all other failures short-circuit immediately
/// (including [`CompositionError::SchemaValidation`] for invalid required
/// values). When at least one step reports missing properties and
/// Interactive Mode is allowed, the deduplicated missing set is collected
/// via `biscuit-tui` prompts and the loop re-runs with the merged
/// overrides. When Interactive Mode is not allowed, an aggregated
/// [`CompositionError::SequenceMissingProperties`] is returned so the
/// user can fix the full sequence in one edit.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_phase_1c_with_schema(
    source: &ResolvedCompositionSource,
    plan: &SequencePlan,
    resolved_targets: &[Option<claudine::composition::ResolvedExecutionTarget>],
    user_set_overrides: &Option<serde_json::Value>,
    source_repo_root: Option<&std::path::Path>,
    child_cwd: &std::path::Path,
    launch_area: Option<&std::path::Path>,
    shared: &SharedComposeArgs,
    effective_fail_fast: bool,
    inline_mode: bool,
    shared_approval_cache: composition::SharedApprovalCache,
    initial_cumulative_approved: HashSet<String>,
    interrupted: &Arc<AtomicBool>,
    silent: bool,
) -> Result<(Vec<StepContext>, HashSet<String>)> {
    let mut overrides = user_set_overrides.clone();
    // At most one interactive collection pass — collected values are
    // shared across all steps that need the same property, and a second
    // attempt should always succeed (or fail with a different error).
    for attempt in 0..2 {
        let attempt_result = run_phase_1c_attempt(
            source,
            plan,
            resolved_targets,
            &overrides,
            source_repo_root,
            child_cwd,
            launch_area,
            shared,
            effective_fail_fast,
            inline_mode,
            Arc::clone(&shared_approval_cache),
            initial_cumulative_approved.clone(),
            interrupted,
        )?;

        match attempt_result {
            Phase1cAttempt::Interrupted => {
                return Ok((Vec::new(), initial_cumulative_approved));
            }
            Phase1cAttempt::Success(contexts, approved) => {
                return Ok((contexts, approved));
            }
            Phase1cAttempt::Missing(contexts) => {
                if attempt > 0 {
                    // We already collected values once. If we're still
                    // seeing missing values, surface them.
                    return Err(into_sequence_missing(contexts).into());
                }
                // Honor `interactive.allowed()` first so non-TTY runs
                // produce the aggregated per-step report — matching the
                // direct `compose` path in
                // `pre_validate_with_interactive_collection`. Promoting
                // unsupported shapes ahead of this check would force
                // every non-TTY sequence with e.g. `object(required)` to
                // surface as `UnsupportedInteractiveSchema` instead of
                // the actionable aggregated `MissingProperties` report.
                let interactive = resolve_interactive_options(shared.silent);
                if !interactive.allowed() {
                    return Err(into_sequence_missing(contexts).into());
                }
                // Interactive Mode is allowed: now promote the first
                // missing property whose schema shape cannot be collected
                // via `biscuit-tui` (raw JSON Schema, `object`, `any`,
                // property-level union) so the user sees a targeted
                // `UnsupportedInteractiveSchema` error rather than a
                // generic aggregated report we can't satisfy.
                if let Some((source_path, property, shape)) =
                    find_first_unsupported(&failures_slice(&contexts))
                {
                    return Err(CompositionError::UnsupportedInteractiveSchema {
                        source_path,
                        property,
                        shape,
                    }
                    .into());
                }
                let collected = match collect_sequence_missing_values(&contexts, silent, launch_area) {
                    Ok(values) => values,
                    Err(_) => {
                        // Ctrl-C / Esc / unsupported shape — surface the
                        // aggregated error so the user sees the actionable
                        // non-TTY report.
                        return Err(into_sequence_missing(contexts).into());
                    }
                };
                if collected.is_empty() {
                    return Err(into_sequence_missing(contexts).into());
                }
                overrides = Some(merge_overrides(overrides.as_ref(), collected));
            }
        }
    }
    // Loop bound enforces two attempts; the inner match returns from
    // either branch so this point is unreachable.
    unreachable!("phase 1c attempt loop exited without resolving")
}

/// Per-step missing-properties context.
///
/// Pairs the typed `SequenceMissingPropertiesStep` (returned to the user
/// as part of `CompositionError::SequenceMissingProperties`) with the
/// post-overlay effective override map for that step. The effective
/// overrides are needed by [`build_schema_status_report`] so the
/// pre-prompt diagnostic reflects what the prepare pipeline will
/// validate against — values supplied by reserved per-step overlay keys
/// and CLI `--set` / shorthand setters must show as `Valid` (or omitted)
/// rather than as `Missing`.
struct StepMissingContext {
    failure: SequenceMissingPropertiesStep,
    effective_overrides: Option<serde_json::Value>,
}

enum Phase1cAttempt {
    Success(Vec<StepContext>, HashSet<String>),
    Missing(Vec<StepMissingContext>),
    Interrupted,
}

/// Borrow just the failure records out of a `[StepMissingContext]` slice
/// for helpers that already operate on `&[SequenceMissingPropertiesStep]`.
fn failures_slice(contexts: &[StepMissingContext]) -> Vec<SequenceMissingPropertiesStep> {
    contexts.iter().map(|c| c.failure.clone()).collect()
}

/// Convert per-step contexts into the typed aggregated error variant.
fn into_sequence_missing(contexts: Vec<StepMissingContext>) -> CompositionError {
    let failures: Vec<SequenceMissingPropertiesStep> =
        contexts.into_iter().map(|c| c.failure).collect();
    CompositionError::SequenceMissingProperties {
        failure_count: failures.len(),
        failures,
    }
}

#[allow(clippy::too_many_arguments)]
fn run_phase_1c_attempt(
    source: &ResolvedCompositionSource,
    plan: &SequencePlan,
    resolved_targets: &[Option<claudine::composition::ResolvedExecutionTarget>],
    user_set_overrides: &Option<serde_json::Value>,
    source_repo_root: Option<&std::path::Path>,
    child_cwd: &std::path::Path,
    launch_area: Option<&std::path::Path>,
    shared: &SharedComposeArgs,
    effective_fail_fast: bool,
    inline_mode: bool,
    shared_approval_cache: composition::SharedApprovalCache,
    initial_cumulative_approved: HashSet<String>,
    interrupted: &Arc<AtomicBool>,
) -> Result<Phase1cAttempt> {
    let total_steps = plan.steps.len();
    let mut cumulative_approved = initial_cumulative_approved;
    let mut step_contexts: Vec<StepContext> = Vec::with_capacity(total_steps);
    let mut missing_contexts: Vec<StepMissingContext> = Vec::new();

    for step_index in 0..total_steps {
        if interrupted.load(Ordering::SeqCst) {
            return Ok(Phase1cAttempt::Interrupted);
        }
        let overlay = build_step_overlay(plan, step_index);
        let step_set_overrides = overlay.as_set_overrides(user_set_overrides.clone());

        let mut env_overrides: BTreeMap<String, String> = BTreeMap::new();
        env_overrides.insert(
            "CLAUDINE_FAIL_FAST".to_string(),
            effective_fail_fast.to_string(),
        );
        let target = resolved_targets
            .get(step_index)
            .ok_or_else(|| eyre!("missing resolved target for step {}", step_index + 1))?;
        // A `--dry-run` step with an unresolved agent state has no target;
        // leave `AGENT` unset so composition matches the direct compose
        // --dry-run path (`{{env.AGENT}}` resolves to empty for those states).
        if let Some(target) = target {
            env_overrides.insert("AGENT".to_string(), target.provider.as_slug().to_string());
            if let Some(ref model) = target.model {
                env_overrides.insert("MODEL".to_string(), model.clone());
            }
        }
        env_overrides.insert("YOLO".to_string(), shared.yolo.to_string());

        // Per-step schema pre-validation BEFORE preflight. If a step's
        // effective frontmatter (source + overlay overrides) is missing
        // required schema values, capture them for aggregation and
        // continue — do NOT let Darkmatter's preflight compose pass
        // surface a raw `SchemaValidationFailed` here, because that
        // would short-circuit aggregation across remaining steps.
        let (step_source, step_overrides) =
            match composition::pre_validate_schema(source, Some(&step_set_overrides), launch_area) {
                Ok(pre) => {
                    emit_dropped_optional_warnings(&pre.dropped_optionals);
                    (
                        pre.source,
                        pre.set_overrides
                            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
                    )
                }
                Err(CompositionError::MissingProperties {
                    source_path,
                    missing,
                    frontmatter_description,
                    pointer_paths,
                }) => {
                    let step = &plan.steps[step_index];
                    missing_contexts.push(StepMissingContext {
                        failure: SequenceMissingPropertiesStep {
                            step: step_index + 1,
                            step_name: step.name.clone(),
                            source_path,
                            missing,
                            frontmatter_description,
                            pointer_paths,
                        },
                        effective_overrides: Some(step_set_overrides.clone()),
                    });
                    continue;
                }
                Err(other) => return Err(other.into()),
            };

        // Capture the step's early-binding context ONCE, anchored on the
        // step's explicit launch CWD (never the process CWD), and reuse the
        // same snapshot for the template shell preflight below AND the final
        // `PrepareOptions.prepared_context`. This is the sequence-step
        // equivalent of the compose path's single `prepared_context`
        // (`compose::prep::execute_loop_or_single`): the commands the preflight
        // audits are then the commands the step's execution expands, even when
        // the wrapper has moved the process CWD to the repo root (R5/R9).
        let step_prepared_context = build_step_prepared_context(
            launch_area,
            &step_source.resolved_path,
            &step_source.markdown,
            &env_overrides,
        );

        let compose_options = build_template_preflight_options(
            &step_prepared_context,
            &step_source.resolved_path,
            &step_overrides,
            launch_area,
        );

        let approval_options = super::super::apply_composition_shell_overrides(
            super::super::build_harness_shell_options_with_cache(
                &step_source.resolved_path,
                source_repo_root,
                Some(Arc::clone(&shared_approval_cache)),
            ),
            shared.dry_run,
            shared.yolo,
        );

        let template_preflight = composition::resolve_shell_approvals(
            Some(&step_source.markdown),
            Some(&compose_options),
            &approval_options,
            None,
            None,
        )?;
        cumulative_approved.extend(template_preflight.approved_commands.iter().cloned());

        let prepare_options = PrepareOptions {
            set_overrides: Some(step_overrides.clone()),
            pre_approved_commands: Some(cumulative_approved.clone()),
            env_overrides: env_overrides.clone(),
            perf_enabled: shared.perf,
            source_repo_root: source_repo_root.map(std::path::Path::to_path_buf),
            shell_working_directory: Some(child_cwd.to_path_buf()),
            // Reuse the exact snapshot the shell preflight audited against, so
            // `ctx.*` resolves identically at audit and execution time (R5/R9).
            prepared_context: Some(step_prepared_context.clone()),
            file_ref_fallback_dir: launch_area.map(std::path::Path::to_path_buf),
            defer_schema_verdict: false,
        };

        // Inline steps prepare via `prepare_inline_with_schema` so the
        // composed `prompt` frontmatter becomes the agent prompt and the
        // prepared closure is `Inline` (drives body write-back in Phase 2).
        // Compose steps keep the body-as-prompt behavior.
        let prepare_result = if inline_mode {
            composition::prepare_inline_with_schema(&step_source, prepare_options)
        } else {
            composition::prepare_direct_with_schema(&step_source, prepare_options)
        };
        let prepared = match prepare_result {
            Ok(prepared) => {
                emit_dropped_optional_warnings(&prepared.dropped_optionals);
                prepared
            }
            Err(CompositionError::MissingProperties {
                source_path,
                missing,
                frontmatter_description,
                pointer_paths,
            }) => {
                let step = &plan.steps[step_index];
                missing_contexts.push(StepMissingContext {
                    failure: SequenceMissingPropertiesStep {
                        step: step_index + 1,
                        step_name: step.name.clone(),
                        source_path,
                        missing,
                        frontmatter_description,
                        pointer_paths,
                    },
                    effective_overrides: Some(step_overrides.clone()),
                });
                // Skip harness preflight for the failed step; continue so
                // we accumulate every step's missing properties.
                continue;
            }
            Err(other) => return Err(other.into()),
        };

        // Audit the resolved lifecycle commands during fleet-wide preflight,
        // before any sequence step starts. Runtime performs the same check
        // defensively, using the shared approval cache populated here.
        let lifecycle_preflight = composition::resolve_shell_approvals(
            None,
            None,
            &approval_options,
            Some(&prepared.lifecycle),
            Some(&prepared.resolved_path),
        )?;
        cumulative_approved.extend(lifecycle_preflight.approved_commands);

        step_contexts.push(StepContext {
            env_overrides,
            prepared,
        });
    }

    if !missing_contexts.is_empty() {
        return Ok(Phase1cAttempt::Missing(missing_contexts));
    }
    Ok(Phase1cAttempt::Success(step_contexts, cumulative_approved))
}

/// Dedupe missing properties across all failed steps and prompt for each
/// unique `(name, type_label, description)` exactly once.
///
/// Returns a JSON map of `{ property_name: collected_value }` suitable for
/// merging into the user `--set` overrides. The first failure that
/// declared the property supplies the metadata used for the prompt.
///
/// The per-step status report uses `StepMissingContext::effective_overrides`
/// so a required property already satisfied by a reserved per-step overlay
/// value or by a CLI `--set` / shorthand setter is not reported as
/// `Missing` while the user is being prompted for a different property
/// (matches the direct `compose` status-drift fix from review-5).
fn collect_sequence_missing_values(
    contexts: &[StepMissingContext],
    silent: bool,
    launch_area: Option<&std::path::Path>,
) -> std::io::Result<serde_json::Map<String, serde_json::Value>> {
    if !silent {
        let term = log::terminal();
        // Render one status report per step so the user sees the same
        // diagnostic they get for direct compose.
        for ctx in contexts {
            let failure = &ctx.failure;
            // Build a minimal source view from the recorded path for the
            // status report. The library helper expects a resolved source
            // so we re-resolve here; if that fails we fall through with
            // a header-only note.
            if let Ok(source) =
                composition::resolve_composition_source(&failure.source_path.display().to_string())
                && let Ok(Some(report)) = composition::build_schema_status_report(
                    &source,
                    ctx.effective_overrides.as_ref(),
                    launch_area,
                )
            {
                let status = Status::from_prose(format!(
                    "<b>Step {}:</b> <cyan>{}</cyan>",
                    failure.step, failure.step_name
                ))
                .state(StatusState::Info);
                log::message(&status.render(&term));
                render_status_report(&report, &term);
            }
        }
    }

    // Build a deduped list, keyed by (name, type_label, description).
    let mut seen_keys: HashSet<(String, Option<String>, Option<String>)> = HashSet::new();
    let mut unique: Vec<MissingProperty> = Vec::new();
    for ctx in contexts {
        for prop in &ctx.failure.missing {
            let key = (
                prop.name.clone(),
                prop.type_label.clone(),
                prop.description.clone(),
            );
            if seen_keys.insert(key) {
                unique.push(prop.clone());
            }
        }
    }

    collect_missing_values(&unique)
}

/// Locate the first missing property across all sequence step failures
/// whose schema shape cannot be collected interactively.
///
/// Returns `(source_path, property_name, shape_label)` so the caller can
/// build a typed [`CompositionError::UnsupportedInteractiveSchema`] that
/// matches the direct `compose` path. The shape label falls back to
/// `(unknown)` when the property has no recorded type label.
pub(super) fn find_first_unsupported(
    failures: &[SequenceMissingPropertiesStep],
) -> Option<(std::path::PathBuf, String, String)> {
    for failure in failures {
        for prop in &failure.missing {
            if prop.interactive_shape.is_none() {
                return Some((
                    failure.source_path.clone(),
                    prop.name.clone(),
                    prop.type_label
                        .clone()
                        .unwrap_or_else(|| "(unknown)".to_string()),
                ));
            }
        }
    }
    None
}

/// Capture a sequence step's early-binding [`ComposeContext`] ONCE, anchored on
/// the step's explicit launch CWD rather than ambient process state (R5).
///
/// The anchor precedence mirrors the library's `derive_compose_context`: the
/// launch area, then the document's own directory, then `"."` — but never
/// `std::env::current_dir()`. The wrapper deliberately moves the parent CWD to
/// the repo root before dispatch, so an ambient `ComposeContext::capture()`
/// would resolve `ctx.*` against a different document's location than the step
/// being prepared. `capture_for_document` is demand-driven over the step's
/// frontmatter and body, so a step that never mentions `ctx.*` pays for no host
/// scan.
///
/// The returned snapshot is reused for both the template shell preflight and the
/// step's final `PrepareOptions.prepared_context`, so the commands the audit
/// discovers are the commands the execution expands.
fn build_step_prepared_context(
    launch_area: Option<&std::path::Path>,
    source_path: &std::path::Path,
    markdown: &darkmatter::markdown::Markdown,
    env_overrides: &BTreeMap<String, String>,
) -> darkmatter::markdown::compose::ComposeContext {
    let anchor = launch_area
        .map(std::path::Path::to_path_buf)
        .or_else(|| source_path.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let mut ctx =
        darkmatter::markdown::compose::ComposeContext::capture_for_document(&anchor, markdown);
    for (key, value) in env_overrides {
        ctx.env_mut().insert(key.clone(), value.clone());
    }
    ctx
}

/// Build the Darkmatter `ComposeOptions` for a step's template SHELL preflight.
///
/// Takes the step's already-captured, launch-anchored context (see
/// [`build_step_prepared_context`]) rather than capturing ambiently, so `ctx.*`
/// resolves against the launch CWD both here and at execution time.
///
/// The launch-area fallback is anchored on `file`-typed schema validation and
/// read-side interpolation so an area-relative path resolves document-first then
/// launch-area — matching this step's `pre_validate_schema` call and the final
/// `PrepareOptions.file_ref_fallback_dir`. Without it the preflight compose would
/// fall back to the (already-mutated) process CWD and could discover different
/// shell commands or pass/fail schema validation inconsistently with the
/// corrected prepare path. The fallback is applied only when `launch_area` is
/// present, matching `PrepareOptions`'s `launch_area.map(...)`.
fn build_template_preflight_options(
    prepared_context: &darkmatter::markdown::compose::ComposeContext,
    source_path: &std::path::Path,
    set_overrides: &serde_json::Value,
    launch_area: Option<&std::path::Path>,
) -> darkmatter::markdown::compose::ComposeOptions {
    let mut opts =
        darkmatter::markdown::compose::ComposeOptions::new_with_context(prepared_context.clone())
            .with_source_file(source_path)
            // Defer the lifecycle event keys (DM1), matching the main prepare pass.
            // The preflight compose exists only to discover template `::shell`
            // directives; without the exclusion it resolves the deferred lifecycle
            // subtree at compose time, so a `success`/`failure` read-side file
            // reference (a file a later event creates) trips the fatal file-ref
            // check before that event fires. Lifecycle shell commands are audited
            // separately via `collect_lifecycle_shell_commands`.
            .with_exclude_keys(LIFECYCLE_EVENT_KEYS.iter().copied());
    if let Some(launch_area) = launch_area {
        opts = opts.with_file_ref_fallback_dir(launch_area.to_path_buf());
    }
    opts = opts.with_set_overrides(set_overrides.clone());
    opts
}

fn merge_overrides(
    base: Option<&serde_json::Value>,
    collected: serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let mut out = match base {
        Some(serde_json::Value::Object(map)) => map.clone(),
        _ => serde_json::Map::new(),
    };
    for (k, v) in collected {
        out.insert(k, v);
    }
    serde_json::Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::{build_step_prepared_context, build_template_preflight_options};
    use std::collections::BTreeMap;

    use darkmatter::markdown::Markdown;

    use claudine::composition::resolve_shell_approvals;
    use claudine::harness::ShellApprovalOptions;

    /// RAII guard that switches the process CWD and restores it on drop
    /// (including on panic). Tests using it are serialized to avoid racing on
    /// process-global CWD with other CWD-mutating tests.
    struct CwdGuard {
        prior: std::path::PathBuf,
    }

    impl CwdGuard {
        fn enter(dir: &std::path::Path) -> Self {
            let prior = std::env::current_dir().expect("read CWD");
            std::env::set_current_dir(dir).expect("set CWD");
            Self { prior }
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.prior);
        }
    }

    /// Initialize a real git repository at `dir` so `ctx.repo_root` resolves to
    /// it. A bare `.git` directory is not a valid gix repository, so the context
    /// capture's `GitRepo::discover` needs an actual `git init`.
    fn init_git_repo(dir: &std::path::Path) {
        let ok = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "git init failed in {}", dir.display());
    }

    /// A step's template shell preflight must resolve `ctx.*` against the
    /// explicit launch CWD, never the ambient process CWD (R5/R9).
    ///
    /// `launch_dir` and `unrelated` are distinct git repositories. The step's
    /// `::shell` argument reads `ctx.repo_root`. With the process CWD moved to
    /// `unrelated` (as the wrapper moves it to the repo root before dispatch),
    /// the approved command must still carry `launch_dir`'s root — the anchor the
    /// step's execution will also use. Before the fix the preflight captured
    /// ambiently and would approve `unrelated`'s root instead, so the audited
    /// bytes could differ from the executed bytes.
    #[test]
    #[serial_test::serial(preflight_cwd)]
    fn preflight_context_anchors_on_launch_cwd_not_process_cwd() {
        let launch_dir = tempfile::TempDir::new().unwrap();
        let unrelated = tempfile::TempDir::new().unwrap();
        let doc_dir = tempfile::TempDir::new().unwrap();
        init_git_repo(launch_dir.path());
        init_git_repo(unrelated.path());

        let source_path = doc_dir.path().join("prompt.md");
        std::fs::write(&source_path, "::shell echo {{ ctx.repo_root }}\n").unwrap();
        std::fs::write(
            doc_dir.path().join(".darkmatter-shell-whitelist"),
            "prefix echo\n",
        )
        .unwrap();

        let md = Markdown::try_from(source_path.as_path()).unwrap();
        let approval_options = ShellApprovalOptions {
            policy_root: Some(doc_dir.path().to_path_buf()),
            approval_handler: None,
            ..Default::default()
        };

        // Move the process CWD to the unrelated repo, standing in for the
        // wrapper's chdir to the repo root.
        let _cwd = CwdGuard::enter(unrelated.path());

        let env_overrides: BTreeMap<String, String> = BTreeMap::new();
        let ctx =
            build_step_prepared_context(Some(launch_dir.path()), &source_path, &md, &env_overrides);
        let opts = build_template_preflight_options(
            &ctx,
            &source_path,
            &serde_json::json!({}),
            Some(launch_dir.path()),
        );
        let result =
            resolve_shell_approvals(Some(&md), Some(&opts), &approval_options, None, None).unwrap();

        // The tempdir's unique basename survives macOS `/var`→`/private/var`
        // canonicalization of the discovered repo root, so match on it rather
        // than the full path.
        let launch_name = launch_dir.path().file_name().unwrap().to_str().unwrap();
        let unrelated_name = unrelated.path().file_name().unwrap().to_str().unwrap();
        assert!(
            result
                .approved_commands
                .iter()
                .any(|c| c.contains(launch_name)),
            "preflight ctx.repo_root must anchor on the launch CWD ({launch_name}); approved: {:?}",
            result.approved_commands,
        );
        assert!(
            !result
                .approved_commands
                .iter()
                .any(|c| c.contains(unrelated_name)),
            "preflight ctx.repo_root must NOT reflect the process CWD ({unrelated_name}); \
             approved: {:?}",
            result.approved_commands,
        );
    }

    /// Regression for the Phase 1c template-preflight fallback omission.
    ///
    /// A sequence step with a `::shell` directive whose `{{ … }}` argument
    /// depends on a read-side `file_exists` against a launch-area-relative file
    /// must see
    /// that file during the template SHELL preflight — exactly as the per-step
    /// `pre_validate_schema` and the final `PrepareOptions.file_ref_fallback_dir`
    /// already do. The launch-area-only file is reachable ONLY via the threaded
    /// fallback, and the resolved/approved command must be CWD-independent.
    #[test]
    #[serial_test::serial(preflight_cwd)]
    fn template_preflight_resolves_via_launch_area_fallback() {
        let doc_dir = tempfile::TempDir::new().unwrap();
        let launch_dir = tempfile::TempDir::new().unwrap();
        let unrelated = tempfile::TempDir::new().unwrap();

        // `spec.md` exists ONLY under the launch-area fallback — not the prompt
        // (document) directory, not the ambient CWD.
        std::fs::write(launch_dir.path().join("spec.md"), "# Spec\n").unwrap();

        let source_path = doc_dir.path().join("prompt.md");
        std::fs::write(
            &source_path,
            "::shell echo {{ file_exists(spec) }}\n",
        )
        .unwrap();

        // Whitelist `echo` so preflight approves without an interactive handler.
        std::fs::write(
            doc_dir.path().join(".darkmatter-shell-whitelist"),
            "prefix echo\n",
        )
        .unwrap();

        let md = Markdown::try_from(source_path.as_path()).unwrap();
        let overrides = serde_json::json!({ "spec": "spec.md" });
        let approval_options = ShellApprovalOptions {
            policy_root: Some(doc_dir.path().to_path_buf()),
            approval_handler: None,
            ..Default::default()
        };

        // Switch ambient CWD elsewhere to prove resolution is independent of any
        // post-launch chdir.
        let _cwd = CwdGuard::enter(unrelated.path());

        let env_overrides: BTreeMap<String, String> = BTreeMap::new();
        let ctx =
            build_step_prepared_context(Some(launch_dir.path()), &source_path, &md, &env_overrides);
        let opts = build_template_preflight_options(
            &ctx,
            &source_path,
            &overrides,
            Some(launch_dir.path()),
        );
        let result =
            resolve_shell_approvals(Some(&md), Some(&opts), &approval_options, None, None).unwrap();

        assert!(
            result.approved_commands.contains("echo true"),
            "template preflight must resolve file_exists(spec) via the launch-area \
             fallback; approved: {:?}",
            result.approved_commands,
        );
    }

    /// Companion proving the assertion above turns on the fallback specifically.
    /// WITHOUT the fallback (the pre-fix behavior), the launch-only `spec.md` is
    /// unreachable from the prompt dir or the unrelated CWD, so `file_exists`
    /// resolves to `false` and the approved command differs — the exact
    /// preflight/prepare disagreement the fix closes.
    #[test]
    #[serial_test::serial(preflight_cwd)]
    fn template_preflight_without_fallback_misses_launch_area_file() {
        let doc_dir = tempfile::TempDir::new().unwrap();
        let launch_dir = tempfile::TempDir::new().unwrap();
        let unrelated = tempfile::TempDir::new().unwrap();

        std::fs::write(launch_dir.path().join("spec.md"), "# Spec\n").unwrap();

        let source_path = doc_dir.path().join("prompt.md");
        std::fs::write(
            &source_path,
            "::shell echo {{ file_exists(spec) }}\n",
        )
        .unwrap();
        std::fs::write(
            doc_dir.path().join(".darkmatter-shell-whitelist"),
            "prefix echo\n",
        )
        .unwrap();

        let md = Markdown::try_from(source_path.as_path()).unwrap();
        let overrides = serde_json::json!({ "spec": "spec.md" });
        let approval_options = ShellApprovalOptions {
            policy_root: Some(doc_dir.path().to_path_buf()),
            approval_handler: None,
            ..Default::default()
        };

        let _cwd = CwdGuard::enter(unrelated.path());

        let env_overrides: BTreeMap<String, String> = BTreeMap::new();
        let ctx = build_step_prepared_context(None, &source_path, &md, &env_overrides);
        let opts = build_template_preflight_options(&ctx, &source_path, &overrides, None);
        let result =
            resolve_shell_approvals(Some(&md), Some(&opts), &approval_options, None, None).unwrap();

        assert!(
            result.approved_commands.contains("echo false"),
            "without the fallback the launch-only spec.md is unreachable, so \
             file_exists(spec) resolves false; approved: {:?}",
            result.approved_commands,
        );
        assert!(
            !result.approved_commands.contains("echo true"),
            "no-fallback path must NOT see the launch-area file; approved: {:?}",
            result.approved_commands,
        );
    }
}
