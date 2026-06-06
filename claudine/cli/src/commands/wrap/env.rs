use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, bail};
use sniff::filesystem::git::detect_git;
use sniff::filesystem::repo::{Package, RepoInfo, detect_repo};

use super::profile::WrapperProfile;
use super::repo_home;

pub(crate) use claudine::composition::{LaunchWorkspaceContext, PackageContext};

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
/// Use this on hot paths where the caller has already resolved the
/// launch workspace from a shared `SniffResult` — e.g. the direct
/// provider wrapper in `run_provider_wrapper_inner`, which needs both
/// an `EnvironmentContext` and a `LaunchContext` from the same scan
/// and therefore also has the data to build a `LaunchWorkspaceContext`
/// without any additional filesystem walks.
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
    set_added_env(
        &mut env,
        &mut added,
        "INTERACTIVE",
        if interactive {
            "true".to_string()
        } else {
            "false".to_string()
        },
    );
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
        set_added_env(&mut env, &mut added, key, value.clone());
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

fn validate_include_names(include: &[String]) -> Result<HashSet<String>> {
    let mut unique = HashSet::new();
    for name in include {
        if !is_valid_env_name(name) {
            bail!("invalid --include env name '{}'", name);
        }
        unique.insert(name.clone());
    }
    Ok(unique)
}

fn is_valid_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }

    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn sanitize_process_env(
    include_set: &HashSet<String>,
    auto_include: &HashSet<String>,
) -> (
    HashMap<OsString, OsString>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
) {
    let mut kept = HashMap::new();
    let mut removed = BTreeSet::new();
    let mut included = BTreeSet::new();
    let mut present_keys = HashSet::new();

    for (key, value) in std::env::vars_os() {
        let key_display = key.to_string_lossy().to_string();
        present_keys.insert(key_display.clone());

        if is_sensitive_key(&key_display) {
            if include_set.contains(&key_display) || auto_include.contains(&key_display) {
                included.insert(key_display);
            } else {
                removed.insert(key_display);
                continue;
            }
        }

        kept.insert(key, value);
    }

    // Only warn about missing keys for explicit --include, not auto-included.
    let mut warnings = Vec::new();
    for include in include_set {
        if !present_keys.contains(include) {
            warnings.push(format!(
                "--include '{}' was requested but is not set in the current environment",
                include
            ));
        }
    }

    (
        kept,
        removed.into_iter().collect(),
        included.into_iter().collect(),
        warnings,
    )
}

fn is_sensitive_key(key: &str) -> bool {
    let uppercase = key.to_ascii_uppercase();
    uppercase.contains("API_KEY")
        || uppercase.contains("TOKEN")
        || uppercase.contains("PASSWORD")
        || uppercase.contains("SECRET")
        || uppercase.contains("PRIVATE_KEY")
        || uppercase.contains("CREDENTIAL")
        || uppercase.contains("ACCESS_KEY")
        || uppercase.contains("PASSPHRASE")
}

/// Redact values in CLI args that look like they contain secrets.
///
/// Scans for patterns like `--api-key=sk-...` or `--token sk-...` and
/// replaces the value portion with `****`.
fn redact_sensitive_args(args: &[String]) -> Vec<String> {
    let sensitive_prefixes: &[&str] = &[
        "--api-key",
        "--token",
        "--secret",
        "--password",
        "--credential",
        "--access-key",
        "--private-key",
        "--passphrase",
    ];

    let mut result = Vec::with_capacity(args.len());
    let mut redact_next = false;

    for arg in args {
        if redact_next {
            result.push("****".to_string());
            redact_next = false;
            continue;
        }

        // Check for --flag=value format
        let mut matched = false;
        for prefix in sensitive_prefixes {
            if let Some(rest) = arg.strip_prefix(prefix) {
                if rest.starts_with('=') {
                    result.push(format!("{prefix}=****"));
                    matched = true;
                    break;
                }
                if rest.is_empty() {
                    // Next arg is the value
                    result.push(arg.clone());
                    redact_next = true;
                    matched = true;
                    break;
                }
            }
        }

        if !matched {
            result.push(arg.clone());
        }
    }

    result
}

struct RepoContext {
    package_context: Option<PackageContext>,
    warnings: Vec<String>,
}

fn detect_repo_root(cwd: &Path) -> Option<PathBuf> {
    detect_git(cwd, false, 1)
        .ok()
        .flatten()
        .map(|info| info.repo_root)
        .or_else(|| detect_repo(cwd).ok().flatten().map(|repo| repo.root))
}

pub(crate) fn resolve_launch_workspace_context(
    launch_cwd: &Path,
    repo_root_hint: Option<&Path>,
) -> LaunchWorkspaceContext {
    // `repo_root` is metadata — guardrails, MCP defaults, harness path
    // resolution. When composing a document, the caller passes the
    // document's enclosing git root as a hint so those subsystems key
    // off the document's repo (e.g. `@`-references, per-repo guardrails).
    let repo_root = repo_root_hint
        .map(Path::to_path_buf)
        .or_else(|| detect_repo_root(launch_cwd));
    // `child_cwd` is the directory the spawned provider process will
    // run in. It must ALWAYS follow the user's launch directory, never
    // the document hint — otherwise a sequence that composes a markdown
    // file from an unrelated nested clone would jump OpenCode/Claude/etc.
    // into that clone and flag the user's real worktree as external.
    let child_cwd = detect_repo_root(launch_cwd).unwrap_or_else(|| launch_cwd.to_path_buf());

    match resolve_monorepo_package_context(launch_cwd) {
        Ok(repo_ctx) => LaunchWorkspaceContext {
            launch_cwd: launch_cwd.to_path_buf(),
            repo_root,
            child_cwd,
            package_context: repo_ctx.package_context,
            warnings: repo_ctx.warnings,
        },
        Err(error) => LaunchWorkspaceContext {
            launch_cwd: launch_cwd.to_path_buf(),
            repo_root,
            child_cwd,
            package_context: None,
            warnings: vec![format!(
                "failed to resolve monorepo package metadata for '{}': {}",
                launch_cwd.display(),
                error
            )],
        },
    }
}

/// Build a [`LaunchWorkspaceContext`] from data already produced by a
/// single `sniff::detect_with_plan` call, without performing any further
/// filesystem walks.
///
/// `git_root` should come from the filesystem git section of the shared
/// `SniffResult`; `repo` should come from its repo section. Callers that
/// have neither can pass `None` for both — the resulting context will
/// behave as if no repo was detected.
///
/// `source_repo_root_hint` preserves the legacy
/// [`resolve_launch_workspace_context`] split contract: when a composed
/// markdown source lives in a different repo than the launch CWD (sibling
/// clone, external prompt), the metadata-bearing `repo_root` should follow
/// the source's repo so guardrails, MCP defaults, and harness path
/// resolution key off the document. The `child_cwd` (where the spawned
/// provider process actually runs) must still follow the launch CWD's
/// repo root so the provider does not jump into an unrelated worktree.
pub(crate) fn launch_workspace_context_from_repo_info(
    launch_cwd: &Path,
    git_root: Option<&Path>,
    repo: Option<&RepoInfo>,
    source_repo_root_hint: Option<&Path>,
) -> LaunchWorkspaceContext {
    let launch_repo_root = git_root
        .map(Path::to_path_buf)
        .or_else(|| repo.map(|r| r.root.clone()));
    let repo_root = source_repo_root_hint
        .map(Path::to_path_buf)
        .or_else(|| launch_repo_root.clone());
    let child_cwd = launch_repo_root
        .clone()
        .unwrap_or_else(|| launch_cwd.to_path_buf());

    let (package_context, warnings) = match repo {
        Some(repo) if repo.is_monorepo => match repo.packages.as_deref() {
            Some(packages) => resolve_package_context_from_packages(launch_cwd, repo, packages),
            None => (
                None,
                vec![format!(
                    "monorepo detected at '{}' but no packages were reported",
                    repo.root.display()
                )],
            ),
        },
        _ => (None, Vec::new()),
    };

    LaunchWorkspaceContext {
        launch_cwd: launch_cwd.to_path_buf(),
        repo_root,
        child_cwd,
        package_context,
        warnings,
    }
}

fn resolve_package_context_from_packages(
    cwd: &Path,
    repo: &RepoInfo,
    packages: &[Package],
) -> (Option<PackageContext>, Vec<String>) {
    if let Some(package_ctx) = select_package_for_cwd(cwd, packages) {
        return (Some(package_ctx), Vec::new());
    }

    if let Some(package_area) = select_package_area_for_cwd(cwd, &repo.root, packages) {
        let candidates = package_candidates_for_area(&package_area, packages);
        return (
            Some(PackageContext {
                package_area,
                package: None,
                candidates,
            }),
            Vec::new(),
        );
    }

    (
        None,
        vec![format!(
            "monorepo detected at '{}' but no package area matched cwd '{}'",
            repo.root.display(),
            cwd.display()
        )],
    )
}

fn resolve_monorepo_package_context(cwd: &Path) -> Result<RepoContext> {
    let git_root = detect_git(cwd, false, 1)?.map(|info| info.repo_root);
    let repo_probe_root = git_root.clone().unwrap_or_else(|| cwd.to_path_buf());
    let Some(repo) = detect_repo(&repo_probe_root)? else {
        return Ok(RepoContext {
            package_context: None,
            warnings: Vec::new(),
        });
    };

    if !repo.is_monorepo {
        return Ok(RepoContext {
            package_context: None,
            warnings: Vec::new(),
        });
    }

    let Some(packages) = repo.packages else {
        return Ok(RepoContext {
            package_context: None,
            warnings: vec![format!(
                "monorepo detected at '{}' but no packages were reported",
                repo.root.display()
            )],
        });
    };

    if let Some(package_ctx) = select_package_for_cwd(cwd, &packages) {
        return Ok(RepoContext {
            package_context: Some(package_ctx),
            warnings: Vec::new(),
        });
    }

    if let Some(package_area) = select_package_area_for_cwd(cwd, &repo.root, &packages) {
        let candidates = package_candidates_for_area(&package_area, &packages);
        return Ok(RepoContext {
            package_context: Some(PackageContext {
                package_area,
                package: None,
                candidates,
            }),
            warnings: Vec::new(),
        });
    }

    Ok(RepoContext {
        package_context: None,
        warnings: vec![format!(
            "monorepo detected at '{}' but no package area matched cwd '{}'",
            repo.root.display(),
            cwd.display()
        )],
    })
}

fn select_package_for_cwd(cwd: &Path, packages: &[Package]) -> Option<PackageContext> {
    let cwd_normalized = canonical_or_self(cwd);

    packages
        .iter()
        .filter_map(|package| {
            let package_path = canonical_or_self(&package.path);
            if cwd_normalized.starts_with(&package_path) {
                Some((package_path.components().count(), package))
            } else {
                None
            }
        })
        .max_by_key(|(depth, _)| *depth)
        .map(|(_, package)| PackageContext {
            package_area: package.package_area.clone(),
            package: Some(package.name.clone()),
            candidates: vec![package.name.clone()],
        })
}

fn select_package_area_for_cwd(
    cwd: &Path,
    repo_root: &Path,
    packages: &[Package],
) -> Option<String> {
    let cwd_normalized = canonical_or_self(cwd);
    let repo_root_normalized = canonical_or_self(repo_root);

    packages
        .iter()
        .map(|package| {
            let area_root = if package.package_area == "root" {
                repo_root_normalized.clone()
            } else {
                repo_root_normalized.join(&package.package_area)
            };
            (area_root, package.package_area.clone())
        })
        .filter(|(area_root, _)| cwd_normalized.starts_with(area_root))
        .max_by_key(|(area_root, _)| area_root.components().count())
        .map(|(_, area)| area)
}

fn package_candidates_for_area(package_area: &str, packages: &[Package]) -> Vec<String> {
    let mut candidates: Vec<String> = packages
        .iter()
        .filter(|package| package.package_area == package_area)
        .map(|package| package.name.clone())
        .collect();
    candidates.sort();
    candidates.dedup();
    candidates
}

fn canonical_or_self(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::wrap::profile::profile_for_provider;
    use sniff::filesystem::repo::Package;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    #[test]
    fn sanitization_removes_sensitive_names_unless_included() {
        let include_set = HashSet::from(["OPENAI_API_KEY".to_string()]);
        let env = vec![
            ("OPENAI_API_KEY".to_string(), "keep".to_string()),
            ("SVC_TOKEN".to_string(), "remove".to_string()),
            ("DB_PASSWORD".to_string(), "remove".to_string()),
            ("APP_SECRET".to_string(), "remove".to_string()),
            ("NORMAL_VAR".to_string(), "ok".to_string()),
        ];

        let (kept, removed) = sanitize_env_for_test(&env, &include_set);
        let kept_names: HashSet<_> = kept.into_iter().map(|(name, _)| name).collect();

        assert!(kept_names.contains("OPENAI_API_KEY"));
        assert!(kept_names.contains("NORMAL_VAR"));
        assert_eq!(
            removed,
            vec![
                "APP_SECRET".to_string(),
                "DB_PASSWORD".to_string(),
                "SVC_TOKEN".to_string()
            ]
        );
    }

    #[test]
    fn sanitization_catches_new_sensitive_patterns() {
        let include_set = HashSet::new();
        let env = vec![
            ("SSH_PRIVATE_KEY".to_string(), "secret".to_string()),
            ("AWS_ACCESS_KEY_ID".to_string(), "secret".to_string()),
            ("DB_CREDENTIAL".to_string(), "secret".to_string()),
            ("KEY_PASSPHRASE".to_string(), "secret".to_string()),
            ("NORMAL_VAR".to_string(), "ok".to_string()),
        ];

        let (kept, removed) = sanitize_env_for_test(&env, &include_set);
        let kept_names: HashSet<_> = kept.into_iter().map(|(name, _)| name).collect();

        assert!(!kept_names.contains("SSH_PRIVATE_KEY"));
        assert!(!kept_names.contains("AWS_ACCESS_KEY_ID"));
        assert!(!kept_names.contains("DB_CREDENTIAL"));
        assert!(!kept_names.contains("KEY_PASSPHRASE"));
        assert!(kept_names.contains("NORMAL_VAR"));
        assert_eq!(removed.len(), 4);
    }

    #[test]
    fn include_names_must_be_valid_env_identifiers() {
        let includes = vec!["VALID_NAME".to_string(), "9INVALID".to_string()];
        let error = validate_include_names(&includes).unwrap_err();
        assert!(error.to_string().contains("invalid --include env name"));
    }

    #[test]
    fn redact_sensitive_args_hides_secret_values() {
        let args = vec![
            "--api-key=sk-12345".to_string(),
            "--token".to_string(),
            "bearer-abc".to_string(),
            "--model".to_string(),
            "gpt-4o".to_string(),
            "--password=hunter2".to_string(),
        ];

        let redacted = redact_sensitive_args(&args);
        assert_eq!(
            redacted,
            vec![
                "--api-key=****",
                "--token",
                "****",
                "--model",
                "gpt-4o",
                "--password=****",
            ]
        );
    }

    #[test]
    fn redact_sensitive_args_preserves_non_secret_args() {
        let args = vec![
            "--json".to_string(),
            "summarize".to_string(),
            "--model".to_string(),
            "gpt-4o".to_string(),
        ];

        let redacted = redact_sensitive_args(&args);
        assert_eq!(redacted, args);
    }

    #[test]
    fn package_selection_prefers_longest_matching_prefix() {
        let cwd = Path::new("/repo/apps/browser/my-app/src");
        let packages = vec![
            Package {
                path: PathBuf::from("/repo/apps/browser"),
                relative: "apps/browser".to_string(),
                package_area: "apps".to_string(),
                name: "browser-root".to_string(),
                ecosystem: Default::default(),
                discovery_sources: vec![],
                nested_packages: vec![],
                primary_language: None,
                secondary_languages: vec![],
                frameworks: vec![],
                file_associations: vec![],
                languages: vec![],
                configuration: vec![],
                documentation: vec![],
                editor_config: None,
                command_runner: vec![],
                package_managers: vec![],
                version: None,
                features: vec![],
                depends_on: vec![],
                used_by: vec![],
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                is_updatable: None,
                has_major_update: None,
                is_excluded: false,
            },
            Package {
                path: PathBuf::from("/repo/apps/browser/my-app"),
                relative: "apps/browser/my-app".to_string(),
                package_area: "apps/browser".to_string(),
                name: "my-app".to_string(),
                ecosystem: Default::default(),
                discovery_sources: vec![],
                nested_packages: vec![],
                primary_language: None,
                secondary_languages: vec![],
                frameworks: vec![],
                file_associations: vec![],
                languages: vec![],
                configuration: vec![],
                documentation: vec![],
                editor_config: None,
                command_runner: vec![],
                package_managers: vec![],
                version: None,
                features: vec![],
                depends_on: vec![],
                used_by: vec![],
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                is_updatable: None,
                has_major_update: None,
                is_excluded: false,
            },
        ];

        let selected = select_package_for_cwd(cwd, &packages).unwrap();
        assert_eq!(selected.package, Some("my-app".to_string()));
        assert_eq!(selected.package_area, "apps/browser");
    }

    #[test]
    fn package_area_selection_supports_area_root_without_package_match() {
        let cwd = Path::new("/repo/claudine");
        let repo_root = Path::new("/repo");
        let packages = vec![
            Package {
                path: PathBuf::from("/repo/claudine/lib"),
                relative: "claudine/lib".to_string(),
                package_area: "claudine".to_string(),
                name: "claudine".to_string(),
                ecosystem: Default::default(),
                discovery_sources: vec![],
                nested_packages: vec![],
                primary_language: None,
                secondary_languages: vec![],
                frameworks: vec![],
                file_associations: vec![],
                languages: vec![],
                configuration: vec![],
                documentation: vec![],
                editor_config: None,
                command_runner: vec![],
                package_managers: vec![],
                version: None,
                features: vec![],
                depends_on: vec![],
                used_by: vec![],
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                is_updatable: None,
                has_major_update: None,
                is_excluded: false,
            },
            Package {
                path: PathBuf::from("/repo/claudine/cli"),
                relative: "claudine/cli".to_string(),
                package_area: "claudine".to_string(),
                name: "claudine-cli".to_string(),
                ecosystem: Default::default(),
                discovery_sources: vec![],
                nested_packages: vec![],
                primary_language: None,
                secondary_languages: vec![],
                frameworks: vec![],
                file_associations: vec![],
                languages: vec![],
                configuration: vec![],
                documentation: vec![],
                editor_config: None,
                command_runner: vec![],
                package_managers: vec![],
                version: None,
                features: vec![],
                depends_on: vec![],
                used_by: vec![],
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                is_updatable: None,
                has_major_update: None,
                is_excluded: false,
            },
        ];

        assert!(select_package_for_cwd(cwd, &packages).is_none());
        let area = select_package_area_for_cwd(cwd, repo_root, &packages);
        assert_eq!(area, Some("claudine".to_string()));
        assert_eq!(
            package_candidates_for_area("claudine", &packages),
            vec!["claudine".to_string(), "claudine-cli".to_string()]
        );
    }

    fn fake_repo_info(root: &Path) -> RepoInfo {
        RepoInfo {
            is_monorepo: false,
            monorepo_tool: None,
            workspace_tools: vec![],
            root: root.to_path_buf(),
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            packages: None,
        }
    }

    #[test]
    fn launch_workspace_repo_root_follows_source_when_outside_launch_repo() {
        // Out-of-repo prompt case: launch CWD lives in repo A, prompt
        // markdown lives in repo B. `repo_root` (metadata) must follow
        // repo B so guardrails / MCP / harness key off the document repo,
        // but `child_cwd` must follow repo A so the spawned provider
        // process stays in the user's worktree.
        let launch_cwd = PathBuf::from("/repo-a/sub");
        let launch_git = PathBuf::from("/repo-a");
        let source_repo = PathBuf::from("/repo-b");
        let launch_repo_info = fake_repo_info(&launch_git);

        let ctx = launch_workspace_context_from_repo_info(
            &launch_cwd,
            Some(&launch_git),
            Some(&launch_repo_info),
            Some(&source_repo),
        );

        assert_eq!(ctx.repo_root.as_deref(), Some(source_repo.as_path()));
        assert_eq!(ctx.child_cwd, launch_git);
        assert_eq!(ctx.launch_cwd, launch_cwd);
    }

    #[test]
    fn launch_workspace_falls_back_to_launch_repo_when_no_source_hint() {
        // Common case (source inside launch repo, or direct wrapper with no
        // source at all): `repo_root` and `child_cwd` should both follow
        // the launch git root.
        let launch_cwd = PathBuf::from("/repo-a/sub");
        let launch_git = PathBuf::from("/repo-a");
        let launch_repo_info = fake_repo_info(&launch_git);

        let ctx = launch_workspace_context_from_repo_info(
            &launch_cwd,
            Some(&launch_git),
            Some(&launch_repo_info),
            None,
        );

        assert_eq!(ctx.repo_root.as_deref(), Some(launch_git.as_path()));
        assert_eq!(ctx.child_cwd, launch_git);
    }

    #[test]
    fn launch_workspace_source_hint_with_no_launch_repo_uses_source_for_meta_only() {
        // Edge case: launch CWD is not inside any git repo, but the
        // composed source lives in one. Metadata follows the source repo;
        // the child process still launches in the user's CWD because we
        // have no launch-repo root to anchor it.
        let launch_cwd = PathBuf::from("/tmp/scratch");
        let source_repo = PathBuf::from("/repo-b");

        let ctx =
            launch_workspace_context_from_repo_info(&launch_cwd, None, None, Some(&source_repo));

        assert_eq!(ctx.repo_root.as_deref(), Some(source_repo.as_path()));
        assert_eq!(ctx.child_cwd, launch_cwd);
    }

    /// Regression: the spawned child's `PWD` env var must equal
    /// `child_cwd`, not whatever the parent shell set as `PWD` before
    /// invoking claudine. OpenCode (and other shell-aware tooling)
    /// reads `process.env.PWD` BEFORE falling back to `process.cwd()`
    /// when resolving its project root; without this sync claudine
    /// silently leaks the user's pre-invocation shell PWD (often a
    /// package subdirectory of the worktree) into the child, causing
    /// git snapshot pathspec mismatches and external_directory false
    /// positives.
    #[test]
    fn build_child_env_overrides_pwd_to_match_child_cwd() {
        let profile = profile_for_provider(claudine::provider::Provider::OpenCode).unwrap();
        let cwd = tempfile::tempdir().unwrap();

        let plan = build_child_env(
            profile,
            claudine::provider::Provider::OpenCode,
            &[],
            false,
            false,
            &[],
            cwd.path(),
            &[],
            false,
            false,
            None,
        )
        .unwrap();

        let pwd = plan
            .env
            .get(std::ffi::OsStr::new("PWD"))
            .map(|v| v.to_string_lossy().into_owned())
            .expect("env plan must always set PWD for the spawned child");
        assert_eq!(
            std::path::PathBuf::from(&pwd),
            plan.child_cwd,
            "child PWD must equal child_cwd; got PWD={pwd:?} child_cwd={:?}",
            plan.child_cwd,
        );
    }

    #[test]
    fn build_child_env_uses_interactive_without_claudine_duplicate() {
        let profile = profile_for_provider(claudine::provider::Provider::Claude).unwrap();
        let cwd = tempfile::tempdir().unwrap();

        let plan = build_child_env(
            profile,
            claudine::provider::Provider::Claude,
            &[],
            false,
            true,
            &[],
            cwd.path(),
            &[],
            false,
            false,
            None,
        )
        .unwrap();

        let added: std::collections::HashMap<_, _> = plan.added.into_iter().collect();
        assert_eq!(added.get("INTERACTIVE").map(String::as_str), Some("true"));
        assert!(!added.contains_key("CLAUDINE_INTERACTIVE"));
    }

    /// `CLAUDINE_PID` must be stamped onto every wrapper env plan so the
    /// spawned provider can correlate back to the Claudine process. It
    /// is provider-agnostic, so a single assertion covers every profile.
    #[test]
    fn build_child_env_includes_claudine_pid_for_interactive_wrapper() {
        let profile = profile_for_provider(claudine::provider::Provider::Claude).unwrap();
        let cwd = tempfile::tempdir().unwrap();

        let plan = build_child_env(
            profile,
            claudine::provider::Provider::Claude,
            &[],
            false,
            true,
            &[],
            cwd.path(),
            &[],
            false,
            false,
            None,
        )
        .unwrap();

        let added: std::collections::HashMap<_, _> = plan.added.into_iter().collect();
        let pid_str = added
            .get("CLAUDINE_PID")
            .expect("CLAUDINE_PID must be added to every wrapper env plan");
        let pid: u32 = pid_str
            .parse()
            .expect("CLAUDINE_PID must be a valid u32 (matches std::process::id())");
        assert_eq!(pid, std::process::id());
        assert_eq!(
            plan.env
                .get(std::ffi::OsStr::new("CLAUDINE_PID"))
                .map(|v| v.to_string_lossy().into_owned())
                .as_deref(),
            Some(pid_str.as_str()),
            "CLAUDINE_PID must also be present in the child env map"
        );
    }

    /// Non-interactive wrapper runs (compose, inline-compose, sequence,
    /// harness attempts) must receive the same `CLAUDINE_PID` injection
    /// as interactive ones — the spec requires it regardless of mode.
    #[test]
    fn build_child_env_includes_claudine_pid_for_non_interactive_wrapper() {
        let profile = profile_for_provider(claudine::provider::Provider::OpenCode).unwrap();
        let cwd = tempfile::tempdir().unwrap();

        let plan = build_child_env(
            profile,
            claudine::provider::Provider::OpenCode,
            &[],
            false,
            false,
            &[],
            cwd.path(),
            &[],
            false,
            false,
            None,
        )
        .unwrap();

        let added: std::collections::HashMap<_, _> = plan.added.into_iter().collect();
        let pid_str = added
            .get("CLAUDINE_PID")
            .expect("CLAUDINE_PID must be added even in non-interactive mode");
        let pid: u32 = pid_str
            .parse()
            .expect("CLAUDINE_PID must parse as u32");
        assert_eq!(pid, std::process::id());
    }

    /// Regression for the source-repo vs launch-repo split through the real
    /// env wiring. When a composed source document lives in one repo (its
    /// enclosing git root becomes `repo_root`, the metadata anchor) but the
    /// user launched from a different repo (`child_cwd`), Codex shadow-HOME
    /// prompt materialization must follow `child_cwd`, NOT the source
    /// metadata root. This exercises `build_child_env_with_launch` ->
    /// `needs_shadow_home` -> `build_repo_home_env`, so a future change that
    /// accidentally threads `repo_root`/source metadata into the shadow-HOME
    /// call is caught here even if the low-level `repo_home` tests stay green.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn build_child_env_codex_shadow_home_uses_child_cwd_not_source_repo_root() {
        let tmp = tempfile::tempdir().unwrap();
        let fake_home = tmp.path().join("home");
        let launch_repo = tmp.path().join("launch-repo");
        let source_repo = tmp.path().join("source-repo");

        fs::create_dir_all(fake_home.join(".codex")).unwrap();
        fs::create_dir_all(launch_repo.join(".claude/commands")).unwrap();
        fs::create_dir_all(source_repo.join(".claude/commands")).unwrap();
        fs::write(launch_repo.join(".claude/commands/launch.md"), "launch").unwrap();
        fs::write(source_repo.join(".claude/commands/source.md"), "source").unwrap();

        let profile = profile_for_provider(claudine::provider::Provider::Codex).unwrap();
        // repo_root (metadata) follows the source document's repo; child_cwd
        // (where the spawned provider runs) follows the launch repo.
        let launch_ctx = LaunchWorkspaceContext {
            launch_cwd: launch_repo.clone(),
            repo_root: Some(source_repo.clone()),
            child_cwd: launch_repo.clone(),
            package_context: None,
            warnings: Vec::new(),
        };

        let old_home = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", &fake_home) };

        let plan = build_child_env_with_launch(
            profile,
            claudine::provider::Provider::Codex,
            &[],
            false,
            false,
            &[],
            &[],
            false,
            false,
            launch_ctx,
            false,
        );

        match old_home {
            Some(v) => unsafe { std::env::set_var("HOME", v) },
            None => unsafe { std::env::remove_var("HOME") },
        }

        let plan = plan.unwrap();
        let shadow_path = plan
            .shadow_home_path
            .expect("Codex repo-local prompts must trigger a shadow home");
        let prompts_dir = shadow_path.join("prompts");

        assert!(
            fs::symlink_metadata(prompts_dir.join("launch.md")).is_ok(),
            "shadow prompts must come from child_cwd (launch repo)"
        );
        assert!(
            fs::symlink_metadata(prompts_dir.join("source.md")).is_err(),
            "shadow prompts must NOT come from the source metadata repo_root"
        );
    }

    fn sanitize_env_for_test(
        env: &[(String, String)],
        include_set: &HashSet<String>,
    ) -> (Vec<(String, String)>, Vec<String>) {
        let auto_include = HashSet::new();
        sanitize_env_for_test_with_auto(env, include_set, &auto_include)
    }

    fn sanitize_env_for_test_with_auto(
        env: &[(String, String)],
        include_set: &HashSet<String>,
        auto_include: &HashSet<String>,
    ) -> (Vec<(String, String)>, Vec<String>) {
        let mut kept = Vec::new();
        let mut removed = BTreeSet::new();

        for (key, value) in env {
            if is_sensitive_key(key) && !include_set.contains(key) && !auto_include.contains(key) {
                removed.insert(key.clone());
            } else {
                kept.push((key.clone(), value.clone()));
            }
        }

        (kept, removed.into_iter().collect())
    }

    fn init_git_repo(path: &Path) -> bool {
        Command::new("git")
            .arg("init")
            .current_dir(path)
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[test]
    fn repo_root_hint_sets_metadata_but_not_child_cwd() {
        // The hint describes the composition source's enclosing repo
        // (used for guardrails, MCP, harness path resolution). The
        // child process must still spawn in the user's launch directory
        // — never in whatever repo the document happens to live in.
        let profile = profile_for_provider(claudine::provider::Provider::Claude).unwrap();
        let cwd = tempfile::tempdir().unwrap();
        let hint_dir = tempfile::tempdir().unwrap();

        let plan = build_child_env(
            profile,
            claudine::provider::Provider::Claude,
            &[],
            false,
            false,
            &[],
            cwd.path(),
            &[],
            false,
            false,
            Some(hint_dir.path()),
        )
        .unwrap();

        assert_eq!(plan.repo_root.as_deref(), Some(hint_dir.path()));
        assert_eq!(plan.child_cwd.as_path(), cwd.path());
    }

    #[test]
    fn repo_root_hint_none_falls_back_to_cwd_detection() {
        let profile = profile_for_provider(claudine::provider::Provider::Claude).unwrap();
        let cwd = tempfile::tempdir().unwrap();

        let plan = build_child_env(
            profile,
            claudine::provider::Provider::Claude,
            &[],
            false,
            false,
            &[],
            cwd.path(),
            &[],
            false,
            false,
            None,
        )
        .unwrap();

        // With None hint and a non-git tempdir, repo_root should be None
        // (CWD detection finds no git repo in a tempdir).
        assert_eq!(plan.repo_root, None);
        assert_eq!(plan.child_cwd.as_path(), cwd.path());
    }

    #[test]
    fn launch_workspace_context_keeps_repo_root_when_package_resolution_fails() {
        let repo_root = tempfile::tempdir().unwrap();
        let docs_dir = repo_root.path().join("docs");
        let package_dir = repo_root.path().join("claudine/cli");
        fs::create_dir_all(&docs_dir).unwrap();
        fs::create_dir_all(&package_dir).unwrap();

        fs::write(
            repo_root.path().join("Cargo.toml"),
            r#"[workspace]
members = ["claudine/cli"]
"#,
        )
        .unwrap();
        fs::write(
            package_dir.join("Cargo.toml"),
            r#"[package]
name = "claudine-cli"
version = "0.1.0"
edition = "2024"
"#,
        )
        .unwrap();

        if !init_git_repo(repo_root.path()) {
            eprintln!("Skipping integration test: git init unavailable");
            return;
        }

        let ctx = resolve_launch_workspace_context(&docs_dir, None);
        let canonical_repo_root = repo_root.path().canonicalize().unwrap();

        assert_eq!(ctx.launch_cwd, docs_dir);
        assert_eq!(
            ctx.repo_root.as_deref(),
            Some(canonical_repo_root.as_path())
        );
        assert_eq!(ctx.child_cwd.as_path(), canonical_repo_root.as_path());
        assert!(ctx.package_context.is_none());
        assert!(
            ctx.warnings
                .iter()
                .any(|warning| warning.contains("no package area matched cwd")),
            "expected package-context warning, got: {:?}",
            ctx.warnings
        );
    }
}
