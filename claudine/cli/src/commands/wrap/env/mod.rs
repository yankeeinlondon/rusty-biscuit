use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, WrapErr, eyre};

use super::profile::WrapperProfile;
use super::repo_home;

pub(crate) use claudine::composition::{LaunchWorkspaceContext, PackageContext};

mod package_context;
mod sanitize;

// Re-exported so the assembly path here, the `tests` module (`use super::*`),
// and external callers (`env::*`) all reach the moved helpers through this
// module's namespace.
pub(crate) use package_context::{
    canonical_or_self, launch_workspace_context_from_repo_info, package_candidates_for_area,
    resolve_launch_workspace_context, select_package_area_for_cwd, select_package_for_cwd,
};
pub(crate) use sanitize::{
    admits_sensitive_key, ambient_sensitive_env, is_sensitive_key, redact_sensitive_args,
    sanitize_process_env, validate_include_names,
};

/// Shared startup detection results for the direct wrap path.
///
/// Projected from one request-scoped invocation owner and retained for all
/// downstream wrapper stages.
pub(crate) struct WrapStartupDetection {
    pub(crate) invocation: claudine::invocation_context::InvocationContext,
    pub(crate) env_context: claudine::events::EnvironmentContext,
    pub(crate) launch_context: claudine::system_prompt::LaunchContext,
    pub(crate) launch_workspace: LaunchWorkspaceContext,
}

/// Capture one reusable filesystem observation and build every startup context.
///
/// On a cold filesystem cache in a large monorepo the previous pipeline
/// walked the tree 3-5 times (once in `detect_environment_fast`, once in
/// `LaunchContext::from_cwd`, and twice inside `build_child_env`). This
/// helper collapses that into one retained observation and projects each
/// consumer context without rediscovery.
pub(crate) fn detect_wrap_startup(
    cwd: &Path,
    capture_git_status: bool,
) -> Result<WrapStartupDetection> {
    let invocation =
        claudine::invocation_context::InvocationContext::capture_for_wrapper(
            cwd,
            capture_git_status,
        );
    let mut launch_context = invocation.launch_context();
    let mut launch_workspace = invocation.launch_workspace_context(None);
    let launch_repository = invocation.launch_repository();
    let promptless_at_repo_root = !capture_git_status
        && launch_repository
            .repo_root()
            .is_some_and(|root| canonical_or_self(root) == canonical_or_self(cwd))
        && launch_repository.repo_info().is_none();

    if promptless_at_repo_root
        && let Some(repo_root) = launch_repository.repo_root().map(Path::to_path_buf)
    {
        launch_context.package_area_root = Some(repo_root);
        launch_workspace.package_context = Some(PackageContext {
            package_area: "root".to_string(),
            package: None,
            candidates: Vec::new(),
        });
    }

    let env_context = invocation.environment_context();

    Ok(WrapStartupDetection {
        invocation,
        env_context,
        launch_context,
        launch_workspace,
    })
}

/// Run startup detection and fall back to a minimal context on failure.
///
/// When `repo_requested` is true, detection failure is propagated as an
/// error because `--repo` depends on repo/package discovery. Otherwise
/// the failure is logged as a deferred warning and the wrapper continues
/// without repo context.
pub(crate) fn detect_wrap_startup_or_fallback(
    cwd: &Path,
    capture_git_status: bool,
    repo_requested: bool,
    deferred_warnings: &mut Vec<String>,
) -> Result<WrapStartupDetection> {
    let startup = detect_wrap_startup(cwd, capture_git_status)?;
    if let Some(captured) = startup.invocation.launch_repository().diagnostic().cloned() {
        if repo_requested {
            return Err(claudine::diagnostics::RestoredDiagnostic::new(captured)
                .with_context("--repo requires startup repo detection")
                .into());
        }
        deferred_warnings.push(format!(
            "startup detection failed; continuing without repo/package context: {}",
            captured.message
        ));
    }
    Ok(startup)
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub(crate) struct EnvPlan {
    pub(crate) env: HashMap<OsString, OsString>,
    pub(crate) removed: Vec<String>,
    pub(crate) included: Vec<String>,
    pub(crate) added: Vec<(String, String)>,
    pub(crate) package_context: Option<PackageContext>,
    pub(crate) repo_root: Option<PathBuf>,
    // The actual working directory used when launching the child process.
    // Callers must consume this directly and must not recompute it from
    // `repo_root`.
    pub(crate) child_cwd: PathBuf,
    pub(crate) warnings: Vec<String>,
    pub(crate) shadow_home_path: Option<PathBuf>,
    /// Measured breakdown of the child-env build cost, for `--perf`. Empty
    /// unless the caller requested perf timing; when no effective root is
    /// supplied the dominant cost is the shadow-HOME `repo root detect` sniff
    /// git walk (~hundreds of ms under `--repo`). When the caller threads a
    /// known root through `build_child_env_with_launch`, that cost collapses
    /// to microseconds. The caller attaches these as `Breakdown` children of
    /// the `child env build` substage.
    pub(crate) perf_substages: Vec<crate::perf::SubstageTiming>,
}

/// The child-visible interactivity markers one session mode implies.
///
/// `INTERACTIVE` is the `"true"`/`"false"` value wrapped providers and
/// downstream processes read. `CLAUDINE_INTERACTIVE` is a deliberately distinct
/// `1`/`0` gate in the `CLAUDINE_` correlation namespace that the hook
/// subprocess (`claudine handle`) reads: the idle (Trigger 2) producer gates on
/// it, because a non-interactive turn-complete is the agent auto-proceeding
/// rather than waiting on a human.
///
/// Both are a pure function of the mode, and this is the only place that
/// function lives. The invocation stamps them into its base child environment
/// from the *opening* mode; a per-attempt rebuild whose refreshed document moved
/// `interactive:` restates them from here rather than deriving its own pair,
/// which is what stops a retry shipping refreshed argv alongside a stale
/// `INTERACTIVE`.
pub(crate) fn interactivity_env(interactive: bool) -> [(&'static str, &'static str); 2] {
    [
        ("INTERACTIVE", if interactive { "true" } else { "false" }),
        ("CLAUDINE_INTERACTIVE", if interactive { "1" } else { "0" }),
    ]
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)] // kept for tests and legacy callers; production paths use `build_child_env_with_launch`
pub(crate) fn build_child_env(
    profile: &dyn WrapperProfile,
    provider: claudine::provider::Provider,
    include: &[String],
    yolo: bool,
    interactive: bool,
    agent_params: &[String],
    cwd: &Path,
    env_overrides: &[(String, String)],
    repo: bool,
    force_shadow_home: bool,
    repo_root_hint: Option<&Path>,
) -> Result<EnvPlan> {
    let launch_ctx = resolve_launch_workspace_context(cwd, repo_root_hint);
    build_child_env_with_launch(
        profile,
        provider,
        include,
        yolo,
        interactive,
        agent_params,
        env_overrides,
        repo,
        force_shadow_home,
        launch_ctx,
        false,
    )
}

/// Same as `build_child_env`, but takes a pre-computed
/// [`LaunchWorkspaceContext`] instead of re-running the sniff-based
/// workspace/package detection internally.
///
/// Canonical callers pass the launch workspace projected by the invocation
/// owner alongside its launch and environment projections, so this function
/// performs no filesystem discovery.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_child_env_with_launch(
    profile: &dyn WrapperProfile,
    provider: claudine::provider::Provider,
    include: &[String],
    yolo: bool,
    interactive: bool,
    agent_params: &[String],
    env_overrides: &[(String, String)],
    repo: bool,
    force_shadow_home: bool,
    launch_ctx: LaunchWorkspaceContext,
    perf: bool,
) -> Result<EnvPlan> {
    // `child env build` is dominated by the shadow-HOME branch under `--repo`;
    // when perf is requested we time `env sanitize` and `shadow home sync` so
    // the substage breakdown points at the real cost. On the production path the
    // already-resolved launch-child root is threaded through, so `repo root
    // detect` collapses to microseconds and the shadow sync's filesystem linking
    // is what remains; only the fallback (no supplied root) still pays the sniff
    // git walk.
    let sanitize_start = perf.then(std::time::Instant::now);
    let include_set = validate_include_names(include)?;
    let auto_include: HashSet<String> = profile
        .allowed_env_keys()
        .iter()
        .map(|k| (*k).to_string())
        .collect();
    let (mut env, removed, included, mut warnings) =
        sanitize_process_env(&include_set, &auto_include);
    let mut added = BTreeMap::new();
    let sanitize_elapsed = sanitize_start.map(|t| t.elapsed());

    let redacted_params = redact_sensitive_args(agent_params);
    let encoded_agent_params = serde_json::to_string(&redacted_params)?;
    set_added_env(
        &mut env,
        &mut added,
        "AGENT",
        profile.agent_env().to_string(),
    );
    set_added_env(
        &mut env,
        &mut added,
        "YOLO",
        if yolo {
            "true".to_string()
        } else {
            "false".to_string()
        },
    );
    for (key, value) in interactivity_env(interactive) {
        set_added_env(&mut env, &mut added, key, value.to_string());
    }
    set_added_env(&mut env, &mut added, "AGENT_PARAMS", encoded_agent_params);
    set_added_env(
        &mut env,
        &mut added,
        "CLAUDINE_SESSION_ID",
        uuid::Uuid::new_v4().to_string(),
    );
    // Stamp Claudine's own PID so wrapped providers and downstream
    // consumers (logs, reports) can correlate back to the wrapper
    // process. Set before provider-specific overrides so profiles can
    // never accidentally strip it.
    set_added_env(
        &mut env,
        &mut added,
        "CLAUDINE_PID",
        std::process::id().to_string(),
    );

    for (key, value) in env_overrides {
        if key == "OPENCODE_CONFIG_CONTENT" {
            // The OpenCode inline config is contributed by several producers
            // (system prompt here, MCP and YOLO later). Merge into any value
            // already present — which may be a user-supplied config preserved by
            // sanitize — instead of overwriting it.
            let current = env.get(std::ffi::OsStr::new(key)).map(|v| v.as_os_str());
            let overlay: serde_json::Value = serde_json::from_str(value)
                .wrap_err("invalid OPENCODE_CONFIG_CONTENT override")?;
            let merged = claudine::opencode_config::merge_overlay(current, overlay)
                .wrap_err("failed to merge OPENCODE_CONFIG_CONTENT")?;
            set_added_env(&mut env, &mut added, key, merged);
        } else {
            set_added_env(&mut env, &mut added, key, value.clone());
        }
    }

    warnings.extend(launch_ctx.warnings.iter().cloned());

    let mut shadow_home_path = None;
    let needs_shadow_home = force_shadow_home
        || repo_home::needs_shadow_home(
            provider,
            &launch_ctx.child_cwd,
            repo,
            Some(launch_ctx.child_cwd.as_path()),
        );

    let mut shadow_breakdown: Option<repo_home::RepoHomeTimings> = None;

    // Use a shadow HOME when repo-only isolation is requested, or when Codex
    // needs repo-local prompt overlay because custom prompts are user-scoped.
    // The already-resolved launch-child root is passed through so the shadow-HOME
    // pipeline does not re-run repo-root detection.
    if needs_shadow_home {
        match repo_home::build_repo_home_env(
            provider,
            &launch_ctx.child_cwd,
            repo,
            perf,
            Some(launch_ctx.child_cwd.as_path()),
        ) {
            Ok((shadow_env, shadow_path, timings)) => {
                for (key, value) in shadow_env {
                    env.insert(key, value);
                }
                shadow_home_path = shadow_path;
                shadow_breakdown = timings;
            }
            Err(e) => {
                warnings.push(format!("failed to create shadow HOME: {}", e));
                // Fall back to original behavior
                set_added_env(&mut env, &mut added, "HOME", "/dev/null".to_string());
            }
        }
    }

    if let Some(package_ctx) = launch_ctx.package_context.clone() {
        // PACKAGE metadata is derived from the original launch directory,
        // not from the child's repo-root working directory.
        set_added_env(
            &mut env,
            &mut added,
            "PACKAGE_AREA",
            package_ctx.package_area.clone(),
        );
        if let Some(ref package) = package_ctx.package {
            set_added_env(&mut env, &mut added, "PACKAGE", package.clone());
        }
    }

    // Sync `PWD` to `child_cwd`. The parent shell sets `PWD` to wherever
    // the user invoked claudine from (often a package subdirectory of the
    // worktree). Rust's `set_current_dir` calls `chdir(2)` but does not
    // update `PWD`, so without this override the spawned child inherits
    // the stale shell `PWD`. Several downstream tools resolve project
    // directory from `process.env.PWD` before `process.cwd()` (notably
    // OpenCode's `cli/cmd/run.ts:276`), which is the standard shell
    // convention. Inject the corrected `PWD` so the child's project /
    // git resolution agrees with the cwd we chose.
    set_added_env(
        &mut env,
        &mut added,
        "PWD",
        launch_ctx.child_cwd.display().to_string(),
    );
    claudine::child_environment::contribute_child_environment(&mut env)?;
    let agent_cwd = env
        .get(std::ffi::OsStr::new(
            claudine::child_environment::AGENT_CWD_ENV,
        ))
        .expect("shared child-environment contribution must set AGENT_CWD")
        .to_string_lossy()
        .into_owned();
    added.insert(
        claudine::child_environment::AGENT_CWD_ENV.to_string(),
        agent_cwd,
    );

    let perf_substages = build_env_perf_substages(sanitize_elapsed, shadow_breakdown);

    Ok(EnvPlan {
        env,
        removed,
        included,
        added: added.into_iter().collect(),
        package_context: launch_ctx.package_context,
        repo_root: launch_ctx.repo_root,
        child_cwd: launch_ctx.child_cwd,
        warnings,
        shadow_home_path,
        perf_substages,
    })
}

/// Assemble the `child env build` perf breakdown from the measured phases.
///
/// Returns empty when perf was not requested (`sanitize_elapsed` is `None`).
/// `shadow home sync` is only present when the shadow-HOME branch ran (i.e.
/// under `--repo` or a Codex prompt overlay); it carries `repo root detect`
/// as its own child, which is either microsecond-scale local work when a
/// known root was supplied or the sniff git walk that dominates the substage
/// when falling back.
fn build_env_perf_substages(
    sanitize_elapsed: Option<std::time::Duration>,
    shadow: Option<repo_home::RepoHomeTimings>,
) -> Vec<crate::perf::SubstageTiming> {
    let Some(sanitize) = sanitize_elapsed else {
        return Vec::new();
    };
    let mut out = vec![crate::perf::SubstageTiming::new("env sanitize", sanitize)];
    if let Some(t) = shadow {
        out.push(crate::perf::SubstageTiming {
            name: "shadow home sync",
            elapsed: t.total,
            children: vec![crate::perf::SubstageTiming::new(
                "repo root detect",
                t.repo_root_detect,
            )],
        });
    }
    out
}

fn set_added_env(
    env: &mut HashMap<OsString, OsString>,
    added: &mut BTreeMap<String, String>,
    key: &str,
    value: String,
) {
    env.insert(OsString::from(key), OsString::from(value.clone()));
    added.insert(key.to_string(), value);
}

#[cfg(test)]
mod tests;
