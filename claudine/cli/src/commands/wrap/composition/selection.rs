//! Provider selection configuration loading.
//!
//! [`load_selection_config`] resolves the user's preferred agent (favorite
//! provider) and per-provider model overrides from the Claudine config file.
//! The hot path uses [`load_selection_config_for_repo`] with a pre-detected
//! repo root to avoid a redundant `detect_git` walk on the compose path.

use std::collections::HashMap;
use std::path::Path;

use claudine::config::claudine_config::ProviderModelOverride;
use claudine::provider::Provider;

/// Resolved provider-selection configuration: the user's favorite agent and
/// per-provider model overrides.
pub(crate) struct SelectionConfig {
    pub favorite: Option<Provider>,
    #[allow(dead_code)]
    pub model_overrides: HashMap<Provider, ProviderModelOverride>,
}

pub(crate) fn load_selection_config(cwd: &Path) -> Option<SelectionConfig> {
    let repo_root = sniff::filesystem::git::detect_git(cwd, false, 1)
        .ok()
        .flatten()
        .map(|info| info.repo_root);
    load_selection_config_for_repo(repo_root.as_deref())
}

/// Load selection config against a pre-detected repo root.
///
/// Skips the internal `sniff::filesystem::git::detect_git` call that
/// [`load_selection_config`] performs, so callers that already discovered
/// the source repo root (typically via [`super::CompositionPrepContext`]) avoid a
/// redundant filesystem walk on the compose hot path.
pub(crate) fn load_selection_config_for_repo(repo_root: Option<&Path>) -> Option<SelectionConfig> {
    load_selection_config_from_paths(None, repo_root)
}

pub(super) fn load_selection_config_from_paths(
    user_path: Option<&Path>,
    repo_root: Option<&Path>,
) -> Option<SelectionConfig> {
    let config = claudine::dispatch::loader::load_claudine_config(user_path, repo_root).ok()?;
    Some(SelectionConfig {
        favorite: config.preferred_agent,
        model_overrides: config.models,
    })
}
