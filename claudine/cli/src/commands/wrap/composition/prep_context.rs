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
use claudine::provider::{PROVIDERS_DISPLAY_ORDER, Provider};
use color_eyre::eyre::Result;
use sniff::programs::InstalledAiClients;

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
    /// Performs at most one `sniff::filesystem::git::detect_git` call on the
    /// source's parent directory. The selection config is loaded against
    /// the discovered root (or `None` when the source has no git ancestor),
    /// avoiding the additional `detect_git` that the legacy
    /// `load_selection_config(path)` helper performed.
    pub fn new(
        original_ref: &str,
        resolved_path: &Path,
        excluded: &BTreeSet<Provider>,
    ) -> Result<Self> {
        // Phase 3 (2026-05-09-slow-prep): instrument the three discoveries
        // owned by this context (source-repo root, selection config,
        // installed-provider snapshot) so trace inspection can confirm each
        // runs exactly once per compose invocation.
        let _ctx_span = tracing::info_span!("compose_prep.prep_context").entered();
        let cwd = std::env::current_dir()?;
        let source_parent = resolved_path.parent().map(Path::to_path_buf);

        let source_repo_root = {
            let _span = tracing::info_span!("compose_prep.source_repo_root").entered();
            source_parent.as_deref().and_then(|parent| {
                sniff::filesystem::git::detect_git(parent, false, 1)
                    .ok()
                    .flatten()
                    .map(|info| info.repo_root)
            })
        };

        let selection_config = {
            let _span = tracing::info_span!("compose_prep.selection_config").entered();
            load_selection_config_for_repo(source_repo_root.as_deref())
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
