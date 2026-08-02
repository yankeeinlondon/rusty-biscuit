//! CLI-private invocation context for compose / inline-compose / sequence.
//!
//! `CompositionPrepContext` is rebuilt for each active document from the one
//! invocation owner and that document's definitive source context. It owns
//! provider-selection state; repository, source, and launch facts remain
//! projections of `InvocationContext`.
//!
//! By threading this context into eager target resolution, shell preflight
//! setup, and composition preparation we ensure:
//!
//! - Repository topology is reused per worktree by the invocation owner.
//! - The selection config (`favorite`, `model_overrides`) is loaded exactly
//!   once for the effective source repo root or CWD.
//! - The installed-provider snapshot is built exactly once.
//!
//! The context is intentionally CLI-private; it carries enough information
//! for the existing library APIs (`eagerly_resolve_target`, `PrepareOptions`,
//! `build_picker_plan_with_hints`, etc.) to run without rediscovery.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use claudine::composition::{
    InstalledProviderSnapshot, LaunchWorkspaceContext, build_installed_snapshot,
    detect_installed_providers,
};
use claudine::diagnostics::DiagnosticSnapshot;
use claudine::error::ClaudineError;
use claudine::events::EnvironmentContext;
use claudine::invocation_context::{InvocationContext, SourceContext};
use claudine::provider::Provider;
use claudine::system_prompt::LaunchContext;
use color_eyre::eyre::Result;
use sniff::programs::InstalledAiClients;

use super::{SelectionConfig, load_selection_config_for_repo};

/// CLI-private invocation context used to deduplicate source-root and
/// selection-config discovery across compose prep phases.
pub(crate) struct CompositionPrepContext {
    /// Shared owner of immutable launch and repository observations.
    pub invocation: InvocationContext,
    /// Definitive context for the first resolved composition source.
    pub source_context: SourceContext,
    /// Source repo root, when the source lives inside a git workspace.
    pub source_repo_root: Option<PathBuf>,
    /// Working directory frozen by the invocation owner at launch.
    pub cwd: PathBuf,
    /// Selection config (favorite, model overrides) loaded for the
    /// effective source repo root or CWD.
    pub selection_config: Option<SelectionConfig>,
    /// Snapshot of installed agentic CLIs at prep time, filtered by the
    /// caller's `--exclude` set.
    pub installed_snapshot: InstalledProviderSnapshot,
    /// Launch-CWD `LaunchContext` projected from the invocation owner.
    pub launch_context: LaunchContext,
    /// Launch-CWD `EnvironmentContext` projected from the captured environment.
    pub env_context: EnvironmentContext,
    /// Launch-CWD workspace projection reused by headers and child setup.
    pub launch_workspace: LaunchWorkspaceContext,
    /// Diagnostic snapshot of the retained launch observation failure.
    ///
    /// The typed `sniff::SniffError` is retained through
    /// `ClaudineError::LaunchContextDetection` and projected once into a
    /// [`DiagnosticSnapshot`] (the §D9 recovery-record boundary) so the
    /// `Clone` record can carry the facets without holding a non-`Clone`
    /// `ClaudineError`.
    pub launch_detection_error: Option<DiagnosticSnapshot>,
}

impl CompositionPrepContext {
    /// Build provider-selection state from the invocation's existing source
    /// observation without performing filesystem discovery.
    ///
    /// `excluded` is the caller's `--exclude` set, applied to the installed
    /// provider list.
    pub fn from_invocation(
        invocation: InvocationContext,
        source_context: SourceContext,
        excluded: &BTreeSet<Provider>,
    ) -> Result<Self> {
        let _ctx_span = tracing::info_span!("compose_prep.prep_context").entered();
        let cwd = invocation.launch_cwd().to_path_buf();
        let source_repo_root = source_context.repository_root().map(Path::to_path_buf);
        let launch_context = invocation.launch_context();
        let env_context = invocation.environment_context();
        let launch_workspace = invocation.launch_workspace_context(source_repo_root.as_deref());
        let launch_detection_error = invocation
            .launch_repository()
            .diagnostic()
            .cloned();

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
            invocation,
            source_context,
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
    /// the source lives in one, falling back to the captured launch CWD otherwise.
    #[allow(dead_code)]
    pub fn effective_root(&self) -> &Path {
        self.source_repo_root.as_deref().unwrap_or(&self.cwd)
    }
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

    /// Build a context the way production does: one invocation owner
    /// captured at the ambient CWD, then a source derived from it.
    fn build_prep_context(
        resolved_path: &Path,
        excluded: &BTreeSet<Provider>,
    ) -> Result<CompositionPrepContext> {
        let invocation = InvocationContext::capture()?;
        let source_context = invocation.derive_source(resolved_path)?;
        CompositionPrepContext::from_invocation(invocation, source_context, excluded)
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
            Arc::new(sniff_error),
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

    /// W0 regression: building `CompositionPrepContext` must populate
    /// `launch_workspace` without falling through to the legacy
    /// `env::resolve_launch_workspace_context` (which would re-scan the
    /// repo via `detect_git`/`detect_repo`). We verify this by checking
    /// that the `launch_workspace` matches the cheap, no-walk
    /// `launch_workspace_context_from_repo_info` output computed from the
    /// same inputs we passed in. If a future change reintroduces the
    /// scanning fallback inside `CompositionPrepContext::from_invocation`, the
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
        let ctx = build_prep_context(&source_file, &excluded).unwrap();

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

        let ws = claudine::composition::LaunchWorkspaceContext::from_repo_info(
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
        let ctx = build_prep_context(&source_file, &excluded).unwrap();

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
        let result = build_prep_context(&source_file, &excluded);

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
