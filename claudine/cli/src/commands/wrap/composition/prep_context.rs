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

use claudine::composition::{
    InstalledProviderSnapshot, LaunchWorkspaceContext, build_installed_snapshot,
    detect_installed_providers,
};
use claudine::diagnostics::DiagnosticSnapshot;
use claudine::error::ClaudineError;
use claudine::events::{EnvironmentContext, environment_context_from_sniff_result};
use claudine::provider::Provider;
use claudine::system_prompt::LaunchContext;
use color_eyre::eyre::Result;
use sniff::programs::InstalledAiClients;
use sniff::request::*;

use super::{SelectionConfig, env, load_selection_config_for_repo};

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
    /// Launch-CWD `LaunchWorkspaceContext`, derived from the same shared
    /// sniff scan. Threaded into `execute_composition_request_inner` so
    /// both the header env plan and the child env build reuse it instead
    /// of calling `resolve_launch_workspace_context` again.
    pub launch_workspace: LaunchWorkspaceContext,
    /// Diagnostic snapshot of the shared sniff scan failure, if it failed.
    ///
    /// The shared scan uses `unwrap_or_default()` to keep prep best-
    /// effort, but the legacy non-prep path treated launch-context
    /// detection failure as a hard error when `--repo` was set. The
    /// executor reads this field to preserve that contract: when
    /// `--repo` is set and the sniff scan failed, the run is aborted
    /// with the captured error rather than silently launching with an
    /// empty launch context. `None` means the scan succeeded.
    ///
    /// The typed `sniff::SniffError` is retained through
    /// `ClaudineError::LaunchContextDetection` and projected once into a
    /// [`DiagnosticSnapshot`] (the §D9 recovery-record boundary) so the
    /// `Clone` record can carry the facets without holding a non-`Clone`
    /// `ClaudineError`.
    pub launch_detection_error: Option<DiagnosticSnapshot>,
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
    /// launch CWD covering git summary only (no os/hw/net/repo), and reuses
    /// its result for both [`LaunchContext`] and [`EnvironmentContext`] so
    /// downstream phases never re-scan. Workspace (repo) structure is
    /// detected in a separate, bounded `detect_repo_structure` call gated on
    /// the scan's discovered git root — never as part of the launch-CWD scan
    /// — so a non-repo launch directory (e.g. `$HOME`) can never trigger an
    /// unbounded home-tree walk. The source's parent directory is probed
    /// separately with the much cheaper `detect_git` to discover
    /// `source_repo_root` because the markdown file may live in a different
    /// repo than the launch CWD.
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
        //
        // The scan requests git summary only — never repo structure. Repo
        // structure is detected separately below, bounded to the discovered
        // git root, because sniff bounds its package-enumeration walk to the
        // git root *when one exists* but otherwise walks `base_dir` unbounded.
        // Launched from a non-repo directory (e.g. `$HOME`) that unbounded
        // walk would recurse the entire home tree (`~/Library`, `~/Documents`,
        // …) and appear to hang. The git root the scan already discovers is
        // the sole gate for whether a package concept exists at all — the same
        // gate `FileReference::with_package_area_magic_path` applies before
        // consulting `cargo_metadata`.
        let (launch_context, env_context, git_root, repo, launch_detection_error) = {
            let _span = tracing::info_span!("compose_prep.shared_sniff").entered();
            let plan = DetectionPlan::new()
                .base_dir(cwd.clone())
                .without_os()
                .without_hardware()
                .without_network()
                .filesystem(
                    FilesystemRequest::new()
                        .git(GitRequest::summary())
                        .without_repo()
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
                Err(error) => (
                    sniff::SniffResult::default(),
                    Some(DiagnosticSnapshot::from_diagnostic(
                        &ClaudineError::LaunchContextDetection(Box::new(error)),
                    )),
                ),
            };
            let launch_context = LaunchContext::from_sniff_result(&sniff_result, &cwd);

            // The launch git root, discovered once by the scan above. This is
            // the single guard for repo-scoped work: no git root means no
            // repo, hence no workspace packages to enumerate.
            let git_root = sniff_result
                .filesystem
                .as_ref()
                .and_then(|f| f.git.as_ref().map(|g| g.repo_root.clone()));

            // Detect workspace structure only when inside a repo, bounded to
            // the git root so the package-enumeration walk can never escape
            // into an unbounded directory tree. Best-effort: a structure-probe
            // failure leaves `repo` as `None` rather than aborting prep.
            let repo = git_root.as_deref().and_then(|root| {
                sniff::filesystem::repo::detect_repo_structure(root)
                    .ok()
                    .flatten()
            });

            let env_context = environment_context_from_sniff_result(sniff_result);
            (
                launch_context,
                env_context,
                git_root,
                repo,
                launch_detection_error,
            )
        };

        // Resolve source_repo_root: in the 99% case the markdown file
        // lives inside the launch repo, so we can short-circuit the
        // expensive `detect_git(source_parent)` (which probes branch /
        // upstream / commit summary on top of finding `.git`). Falls back
        // to a full probe only when the source lives outside the launch
        // repo (e.g., user is composing a file from a sibling clone).
        //
        // Resolved BEFORE `launch_workspace` so the workspace context can
        // honour the legacy split contract: `repo_root` follows the
        // source repo (metadata for guardrails, MCP defaults, harness path
        // resolution), while `child_cwd` keeps following the launch repo
        // (where the spawned provider process actually runs).
        let source_repo_root = {
            let _span = tracing::info_span!("compose_prep.source_repo_root").entered();
            resolve_source_repo_root(
                source_parent.as_deref(),
                launch_context.repo_root.as_deref(),
            )
        };

        let launch_workspace = env::launch_workspace_context_from_repo_info(
            &cwd,
            git_root.as_deref(),
            repo.as_ref(),
            source_repo_root.as_deref(),
        );

        let selection_config = {
            let _span = tracing::info_span!("compose_prep.selection_config").entered();
            load_selection_config_for_repo(source_repo_root.as_deref().or(Some(cwd.as_path())))
        };

        let installed_snapshot = {
            let _span = tracing::info_span!("compose_prep.installed_clients").entered();
            let clients = InstalledAiClients::new();
            let installed = detect_installed_providers(&clients);
            build_installed_snapshot(&installed, excluded, &clients)
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
            launch_workspace,
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

    struct ScopedCwd {
        previous: PathBuf,
    }

    impl ScopedCwd {
        fn enter(path: &Path) -> Self {
            let previous = std::env::current_dir().expect("read current directory");
            std::env::set_current_dir(path).expect("enter fixture repository");
            Self { previous }
        }
    }

    impl Drop for ScopedCwd {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.previous);
        }
    }

    /// Site C: the captured launch-detection failure is now a
    /// `DiagnosticSnapshot` projected from the typed `sniff::SniffError`
    /// (retained through `ClaudineError::LaunchContextDetection`), so the
    /// `Clone` prep/request record carries the facets and message rather than a
    /// flattened `String`. The snapshot must clone and round-trip through serde.
    #[test]
    fn launch_detection_failure_projects_a_clonable_serializable_snapshot() {
        let sniff_error = sniff::SniffError::NotARepository(PathBuf::from("/no/such/repo"));
        let snapshot = DiagnosticSnapshot::from_diagnostic(&ClaudineError::LaunchContextDetection(
            Box::new(sniff_error),
        ));

        // The typed facets survive: `LaunchContextDetection` classifies as
        // `internal.bug`, and its message retains the sniff error text.
        assert_eq!(snapshot.code, "internal.bug");
        assert!(
            snapshot.message.contains("/no/such/repo"),
            "snapshot message must retain the sniff error text, got: {}",
            snapshot.message
        );

        // The record field is `Option<DiagnosticSnapshot>`; the `.clone()` sites
        // in `compose/prep.rs` and `sequence/iterate.rs` rely on this.
        let stored: Option<DiagnosticSnapshot> = Some(snapshot.clone());
        assert_eq!(stored.clone(), Some(snapshot.clone()));

        // Serialize round-trip (the §D9 snapshot boundary).
        let json = serde_json::to_string(&snapshot).expect("snapshot serializes");
        let restored: DiagnosticSnapshot =
            serde_json::from_str(&json).expect("snapshot round-trips");
        assert_eq!(restored, snapshot);
    }

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

    /// W0 regression: building `CompositionPrepContext` must populate
    /// `launch_workspace` without falling through to the legacy
    /// `env::resolve_launch_workspace_context` (which would re-scan the
    /// repo via `detect_git`/`detect_repo`). We verify this by checking
    /// that the `launch_workspace` matches the cheap, no-walk
    /// `launch_workspace_context_from_repo_info` output computed from the
    /// same inputs we passed in. If a future change reintroduces the
    /// scanning fallback inside `CompositionPrepContext::new`, the
    /// computed values would diverge (e.g., `child_cwd` would point at a
    /// freshly detected ancestor rather than the launch CWD).
    #[test]
    #[serial_test::serial]
    fn prep_context_launch_workspace_avoids_redundant_walks() {
        // Create a source markdown file that lives outside any git repo so
        // `source_repo_root` is `None` and the fast `repo_root` value is
        // entirely determined by the launch CWD's sniff result.
        let source_dir = tempfile::tempdir().unwrap();
        let source_file = source_dir.path().join("prompt.md");
        std::fs::write(&source_file, "# Test\n").unwrap();

        let excluded = BTreeSet::new();
        let ctx = CompositionPrepContext::new("prompt.md", &source_file, &excluded).unwrap();

        // The launch_workspace must mirror the data the precomputed
        // helper would have produced from the launch CWD's sniff result.
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(
            ctx.launch_workspace.launch_cwd, cwd,
            "launch_cwd should mirror the ambient CWD captured at construction"
        );
        // child_cwd must NOT be the source_dir (which would happen if a
        // later scan replaced the precomputed value with one anchored at
        // the source's parent).
        assert_ne!(
            ctx.launch_workspace.child_cwd,
            source_dir.path(),
            "child_cwd must follow the launch CWD, not the source-repo location"
        );
    }

    /// W0 regression: when the prompt source lives in a different repo
    /// than the launch CWD, `launch_workspace.repo_root` (metadata) must
    /// follow the source's repo root while `child_cwd` must continue to
    /// follow the launch repo. This exercises the full
    /// `launch_workspace_context_from_repo_info` contract through
    /// `CompositionPrepContext` rather than the helper in isolation.
    #[test]
    fn prep_context_launch_workspace_split_contract_unit() {
        let launch_cwd = PathBuf::from("/repo-a/sub");
        let launch_git = PathBuf::from("/repo-a");
        let source_repo = PathBuf::from("/repo-b");

        let ws = super::env::launch_workspace_context_from_repo_info(
            &launch_cwd,
            Some(&launch_git),
            None,
            Some(&source_repo),
        );
        assert_eq!(ws.repo_root.as_deref(), Some(source_repo.as_path()));
        assert_eq!(ws.child_cwd, launch_git);
    }

    #[test]
    #[serial_test::serial]
    fn prep_context_uses_cwd_repo_when_source_repo_root_is_none() {
        let repo = tempfile::tempdir().unwrap();
        let git_status = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(repo.path())
            .status()
            .expect("git should initialize the fixture repository");
        assert!(git_status.success());
        let repo_root = std::fs::canonicalize(repo.path()).unwrap();

        let source_dir = tempfile::tempdir().unwrap();
        let source_file = source_dir.path().join("prompt.md");
        std::fs::write(&source_file, "# Test\n").unwrap();

        let _cwd = ScopedCwd::enter(&repo_root);
        let excluded = BTreeSet::new();
        let ctx = CompositionPrepContext::new("prompt.md", &source_file, &excluded).unwrap();

        assert!(
            ctx.source_repo_root.is_none(),
            "source outside git should have no repo root"
        );
        assert_eq!(
            std::fs::canonicalize(ctx.effective_root()).unwrap(),
            repo_root,
            "selection fallback should use the launch CWD repository"
        );
        assert_eq!(
            ctx.launch_workspace
                .repo_root
                .as_deref()
                .map(std::fs::canonicalize)
                .transpose()
                .unwrap(),
            Some(repo_root),
            "launch workspace should retain the CWD repository root"
        );
    }

    /// Regression (compose hangs from `$HOME` / any non-repo directory):
    /// building the prep context from a launch CWD that is **not** inside a
    /// git repository must not trigger sniff's package-enumeration walk.
    /// Sniff bounds that walk to the git root when one exists but otherwise
    /// recurses `base_dir` unbounded, so launching from `$HOME` would walk
    /// the entire home tree (`~/Library`, …) and appear to hang.
    ///
    /// The launch CWD is a non-repo tempdir that *also* carries a planted
    /// `[workspace]` `Cargo.toml`. If repo-structure detection were (wrongly)
    /// rooted at this non-repo CWD, sniff would discover that workspace and
    /// `launch_workspace.repo_root` would surface as `Some(tempdir)`. The fix
    /// gates structure detection on the scan's discovered git root — absent
    /// here — so the workspace is never enumerated and `repo_root` stays
    /// `None`. That single assertion is what distinguishes the fixed path
    /// from the unbounded-walk regression.
    #[test]
    #[serial_test::serial]
    fn prep_context_outside_repo_skips_package_enumeration() {
        let launch = tempfile::tempdir().unwrap();
        // macOS canonicalizes `/var` → `/private/var`; `current_dir()` (and
        // therefore `launch_workspace.child_cwd`) returns the canonical form.
        let launch_canon = std::fs::canonicalize(launch.path()).unwrap();

        // Precondition: the tempdir must resolve to no git root. If an
        // ancestor of `$TMPDIR` unexpectedly carries a `.git`, the scan would
        // bound to that root and this test could not exercise the non-repo
        // path — fail loudly rather than silently mis-assert.
        assert!(
            sniff::filesystem::git::repo_root(&launch_canon)
                .ok()
                .flatten()
                .is_none(),
            "test requires a non-repo launch dir; an ancestor unexpectedly has a .git"
        );

        // Plant a workspace that only a recursive walk rooted at the launch
        // CWD would discover.
        std::fs::write(
            launch_canon.join("Cargo.toml"),
            "[workspace]\nmembers = [\"pkg_a\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(launch_canon.join("pkg_a")).unwrap();
        std::fs::write(
            launch_canon.join("pkg_a/Cargo.toml"),
            "[package]\nname = \"pkg_a\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
        )
        .unwrap();

        let source_file = launch_canon.join("prompt.md");
        std::fs::write(&source_file, "# Test\nTell a joke\n").unwrap();

        let prior_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&launch_canon).unwrap();

        let excluded = BTreeSet::new();
        let result = CompositionPrepContext::new("prompt.md", &source_file, &excluded);

        // Restore the process CWD before asserting so a failed assert cannot
        // leave the shared test process pointed at a deleted tempdir.
        let _ = std::env::set_current_dir(&prior_cwd);

        let ctx = result.expect("prep context must build outside a repo");

        // No git anywhere up the tree → no launch repo root.
        assert!(
            ctx.launch_context.repo_root.is_none(),
            "non-repo launch CWD must yield no launch repo root"
        );
        assert!(
            ctx.source_repo_root.is_none(),
            "non-repo source must yield no source repo root"
        );
        // The child process runs in the launch CWD itself when there is no
        // repo to anchor it.
        assert_eq!(
            ctx.launch_workspace.child_cwd, launch_canon,
            "child_cwd must follow the launch CWD outside a repo"
        );
        // KEY regression assertion: the planted workspace must NOT have been
        // enumerated. A reintroduced `.repo(RepoRequest::structure())` rooted
        // at the non-repo CWD would surface it here as `Some(launch_canon)`.
        assert!(
            ctx.launch_workspace.repo_root.is_none(),
            "package enumeration must be skipped outside a repo; \
             a planted workspace leaked into launch_workspace.repo_root"
        );
        assert!(
            ctx.launch_workspace.package_context.is_none(),
            "no package context should be derived outside a repo"
        );
    }
}
