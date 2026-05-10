//! CLI-private invocation context for compose / inline-compose / sequence.
//!
//! `CompositionPrepContext` is built once per command invocation immediately
//! after `composition::resolve_composition_source` returns. It owns the
//! repo/source/CWD facts that earlier code paths rediscovered independently
//! (see `2026-05-09-slow-prep` Phase 2).
//!
//! By threading this context into eager target resolution, shell preflight
//! setup, and composition preparation we ensure:
//!
//! - The source repo root is detected exactly once.
//! - The selection config (`favorite`, `model_overrides`) is loaded exactly
//!   once for the effective source repo root or CWD.
//! - The installed-provider snapshot is built exactly once.
//!
//! The context is intentionally CLI-private; it carries enough information
//! for the existing library APIs (`eagerly_resolve_target`, `PrepareOptions`,
//! `build_picker_plan_with_hints`, etc.) to run without rediscovery.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use claudine::composition::{InstalledProviderSnapshot, build_installed_snapshot};
use claudine::events::{EnvironmentContext, environment_context_from_sniff_result};
use claudine::provider::{PROVIDERS_DISPLAY_ORDER, Provider};
use claudine::system_prompt::LaunchContext;
use color_eyre::eyre::Result;
use sniff::programs::InstalledAiClients;
use sniff::request::*;

use super::{SelectionConfig, load_selection_config_for_repo};

/// CLI-private invocation context used to deduplicate source-root and
/// selection-config discovery across compose prep phases.
pub(crate) struct CompositionPrepContext {
    /// Original CLI file argument (relative or absolute, as the user typed).
    #[allow(dead_code)]
    pub original_ref: String,
    /// Resolved absolute source path (output of `biscuit-file` resolution).
    #[allow(dead_code)]
    pub resolved_path: PathBuf,
    /// Parent directory of the resolved source path, when one exists.
    #[allow(dead_code)]
    pub source_parent: Option<PathBuf>,
    /// Source repo root, when the source lives inside a git workspace.
    pub source_repo_root: Option<PathBuf>,
    /// Ambient working directory at command invocation time.
    pub cwd: PathBuf,
    /// Selection config (favorite, model overrides) loaded for the
    /// effective source repo root or CWD.
    pub selection_config: Option<SelectionConfig>,
    /// Snapshot of installed agentic CLIs at prep time, filtered by the
    /// caller's `--exclude` set.
    pub installed_snapshot: InstalledProviderSnapshot,
    /// Launch-CWD `LaunchContext`, derived from the single shared sniff
    /// scan in [`Self::new`]. Threaded into `composition_prepare` so the
    /// per-invocation `LaunchContext::from_cwd` re-scan is skipped.
    pub launch_context: LaunchContext,
    /// Launch-CWD `EnvironmentContext`, derived from the same shared
    /// sniff scan. Threaded into `composition_preflight` so the
    /// per-invocation `detect_environment_fast` re-scan is skipped when
    /// it would have run against the same root.
    pub env_context: EnvironmentContext,
    /// Captured error message from the shared sniff scan, if it failed.
    ///
    /// The shared scan uses `unwrap_or_default()` to keep prep best-
    /// effort, but the legacy non-prep path treated launch-context
    /// detection failure as a hard error when `--repo` was set. The
    /// executor reads this field to preserve that contract: when
    /// `--repo` is set and the sniff scan failed, the run is aborted
    /// with the captured error rather than silently launching with an
    /// empty launch context. `None` means the scan succeeded.
    pub launch_detection_error: Option<String>,
}

impl CompositionPrepContext {
    /// Build a fresh context for one compose / inline-compose / sequence
    /// invocation.
    ///
    /// `original_ref` is the raw CLI file argument; `resolved_path` is the
    /// absolute path produced by `composition::resolve_composition_source`.
    /// `excluded` is the caller's `--exclude` set, applied to the installed
    /// provider list.
    ///
    /// Performs one shared `sniff::detect_with_plan` scan rooted at the
    /// launch CWD covering git summary + repo structure (no os/hw/net),
    /// and reuses its result for both [`LaunchContext`] and
    /// [`EnvironmentContext`] so downstream phases never re-scan. The
    /// source's parent directory is probed separately with the much
    /// cheaper `detect_git` to discover `source_repo_root` because the
    /// markdown file may live in a different repo than the launch CWD.
    pub fn new(
        original_ref: &str,
        resolved_path: &Path,
        excluded: &BTreeSet<Provider>,
    ) -> Result<Self> {
        // Phase 3 (2026-05-09-slow-prep): instrument the discoveries owned
        // by this context so trace inspection can confirm each runs exactly
        // once per compose invocation.
        let _ctx_span = tracing::info_span!("compose_prep.prep_context").entered();
        let cwd = std::env::current_dir()?;
        let source_parent = resolved_path.parent().map(Path::to_path_buf);

        // Single shared sniff scan rooted at the launch CWD. Replaces the
        // historical `LaunchContext::from_cwd` (in composition_prepare) and
        // `detect_environment_fast` (in composition_preflight) duplicate
        // calls. Both consumers now read from the cached results below.
        // Run this FIRST so `source_repo_root` resolution can reuse the
        // launch repo root in the common case.
        let (launch_context, env_context, launch_detection_error) = {
            let _span = tracing::info_span!("compose_prep.shared_sniff").entered();
            let plan = DetectionPlan::new()
                .base_dir(cwd.clone())
                .without_os()
                .without_hardware()
                .without_network()
                .filesystem(
                    FilesystemRequest::new()
                        .git(GitRequest::summary())
                        .repo(RepoRequest::structure())
                        .without_file_inventory()
                        .without_docs()
                        .without_formatting(),
                );
            // Preserve the sniff error so callers with strict launch-
            // context requirements (currently `--repo`) can fail hard
            // instead of inheriting a silent empty default. Best-effort
            // consumers continue to read the defaulted contexts below.
            let (sniff_result, launch_detection_error) = match sniff::detect_with_plan(plan) {
                Ok(result) => (result, None),
                Err(error) => (sniff::SniffResult::default(), Some(error.to_string())),
            };
            let launch_context = LaunchContext::from_sniff_result(&sniff_result, &cwd);
            let env_context = environment_context_from_sniff_result(sniff_result);
            (launch_context, env_context, launch_detection_error)
        };

        // Resolve source_repo_root: in the 99% case the markdown file
        // lives inside the launch repo, so we can short-circuit the
        // expensive `detect_git(source_parent)` (which probes branch /
        // upstream / commit summary on top of finding `.git`). Falls back
        // to a full probe only when the source lives outside the launch
        // repo (e.g., user is composing a file from a sibling clone).
        let source_repo_root = {
            let _span = tracing::info_span!("compose_prep.source_repo_root").entered();
            resolve_source_repo_root(
                source_parent.as_deref(),
                launch_context.repo_root.as_deref(),
            )
        };

        let selection_config = {
            let _span = tracing::info_span!("compose_prep.selection_config").entered();
            load_selection_config_for_repo(source_repo_root.as_deref().or(Some(cwd.as_path())))
        };

        let installed_snapshot = {
            let _span = tracing::info_span!("compose_prep.installed_clients").entered();
            let clients = InstalledAiClients::new();
            let installed: Vec<Provider> = PROVIDERS_DISPLAY_ORDER
                .into_iter()
                .filter(|p| clients.path(p.sniff_ai_cli()).is_some())
                .collect();
            build_installed_snapshot(&installed, excluded)
        };

        Ok(Self {
            original_ref: original_ref.to_string(),
            resolved_path: resolved_path.to_path_buf(),
            source_parent,
            source_repo_root,
            cwd,
            selection_config,
            installed_snapshot,
            launch_context,
            env_context,
            launch_detection_error,
        })
    }

    /// Effective root used for selection-config / model-override scoping.
    ///
    /// Mirrors the legacy `source_repo_root.unwrap_or(&cwd)` precedence so
    /// catalog overrides keyed off the source repo continue to apply when
    /// the source lives in one, falling back to the ambient CWD otherwise.
    #[allow(dead_code)]
    pub fn effective_root(&self) -> &Path {
        self.source_repo_root.as_deref().unwrap_or(&self.cwd)
    }
}

/// Resolve the markdown source's enclosing git repo root.
///
/// Fast path: when the source's parent directory is inside the launch
/// repo root (which the shared sniff scan already discovered), reuse the
/// launch root and skip the expensive `sniff::filesystem::git::detect_git`
/// probe entirely. That probe reads HEAD, branch, upstream, and a commit
/// summary — work we don't need just to identify the repo root, and which
/// observed at ~609 ms in the wild on the rusty-biscuit worktree.
///
/// Slow path: when the source lives outside the launch repo (sibling
/// clone, source pulled from elsewhere, no launch repo), fall back to a
/// full `detect_git` probe — that case is rare enough to absorb the cost.
fn resolve_source_repo_root(
    source_parent: Option<&Path>,
    launch_repo_root: Option<&Path>,
) -> Option<PathBuf> {
    let parent = source_parent?;
    if let Some(launch_root) = launch_repo_root
        && parent.starts_with(launch_root)
    {
        return Some(launch_root.to_path_buf());
    }
    sniff::filesystem::git::detect_git(parent, false, 1)
        .ok()
        .flatten()
        .map(|info| info.repo_root)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use claudine::config::claudine_config::ClaudineConfig;

    #[test]
    fn resolve_source_repo_root_reuses_launch_root_when_source_inside() {
        // Fast path: source lives inside the launch repo, so we should
        // skip the expensive detect_git probe and reuse the launch root.
        let launch_root = PathBuf::from("/repo");
        let source_parent = PathBuf::from("/repo/prompts");

        let result = super::resolve_source_repo_root(Some(&source_parent), Some(&launch_root));
        assert_eq!(result, Some(launch_root));
    }

    #[test]
    fn resolve_source_repo_root_returns_none_when_no_source_parent() {
        let launch_root = PathBuf::from("/repo");
        let result = super::resolve_source_repo_root(None, Some(&launch_root));
        assert!(result.is_none());
    }

    #[test]
    #[serial_test::serial]
    fn prep_context_loads_cwd_config_when_source_repo_root_is_none() {
        let home = tempfile::tempdir().unwrap();
        let claudine_dir = home.path().join(".claudine");
        std::fs::create_dir_all(&claudine_dir).unwrap();

        let config = ClaudineConfig {
            preferred_agent: Some(Provider::Codex),
            ..ClaudineConfig::default()
        };
        let config_path = claudine_dir.join("config.json");
        claudine::dispatch::loader::save_claudine_config(&config, &config_path).unwrap();

        let old_home = std::env::var("HOME").ok();
        unsafe {
            std::env::set_var("HOME", home.path());
        }

        let source_dir = tempfile::tempdir().unwrap();
        let source_file = source_dir.path().join("prompt.md");
        std::fs::write(&source_file, "# Test\n").unwrap();

        let excluded = BTreeSet::new();
        let ctx = CompositionPrepContext::new("prompt.md", &source_file, &excluded).unwrap();

        unsafe {
            if let Some(old) = old_home {
                std::env::set_var("HOME", old);
            } else {
                std::env::remove_var("HOME");
            }
        }

        assert!(
            ctx.source_repo_root.is_none(),
            "source outside git should have no repo root"
        );
        let cfg = ctx
            .selection_config
            .expect("should fall back to CWD and load config");
        assert_eq!(cfg.favorite, Some(Provider::Codex));
    }
}
