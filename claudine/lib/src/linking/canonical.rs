use std::path::{Path, PathBuf};

use crate::events::{CanonicalProviderSettings, Provider};

use super::capabilities::{LinkableResource, ResourceFormat, capabilities_for};
use super::paths::ResourceScope;

/// Result of canonical provider selection for a `(scope, resource)` pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalSelection {
    /// A canonical provider was selected.
    Selected {
        /// The selected provider.
        provider: Provider,
        /// Whether the selected provider already had valid assets.
        has_valid_assets: bool,
    },
    /// No provider remained after canonical filtering.
    NoMeaningfulLinkingService,
}

/// Number of explicit ranked choices init should ask for.
pub fn preference_prompt_count(installed_count: usize) -> usize {
    match installed_count {
        0 | 1 => 0,
        2 => 1,
        3 => 2,
        _ => 3,
    }
}

/// Build deterministic provider preference ordering.
///
/// Providers in `preferred` are honored first (if installed), and the
/// remaining installed providers are appended alphabetically.
pub fn ranked_provider_preferences(
    installed: &[Provider],
    preferred: &[Provider],
) -> Vec<Provider> {
    let mut installed_sorted = dedupe_providers(installed);
    installed_sorted.sort_by_key(|provider| provider.to_string());

    let mut ordered = Vec::new();

    for provider in preferred {
        if installed_sorted.contains(provider) && !ordered.contains(provider) {
            ordered.push(*provider);
        }
    }

    for provider in installed_sorted {
        if !ordered.contains(&provider) {
            ordered.push(provider);
        }
    }

    ordered
}

/// Select canonical provider according to design filtering rules.
pub fn select_canonical_provider(
    scope: ResourceScope,
    resource: LinkableResource,
    installed_providers: &[Provider],
    preference_order: &[Provider],
    home_dir: &Path,
    repo_root: &Path,
) -> CanonicalSelection {
    let ordered = ranked_provider_preferences(installed_providers, preference_order);
    if ordered.is_empty() {
        return CanonicalSelection::NoMeaningfulLinkingService;
    }

    let mut candidates = Vec::new();

    for provider in ordered {
        let caps = capabilities_for(provider);
        let support = caps.support_for(resource);

        // Canonical candidates must be markdown-backed custom resources.
        if !support.level.allows_custom() || support.format != Some(ResourceFormat::Markdown) {
            continue;
        }

        let Some(entrypoint) = entrypoint_path(provider, resource, scope, home_dir, repo_root)
        else {
            continue;
        };

        // Exclude providers whose entrypoint itself is a symlink.
        if is_symlink(&entrypoint) {
            continue;
        }

        candidates.push((provider, has_valid_assets(resource, &entrypoint)));
    }

    if candidates.is_empty() {
        return CanonicalSelection::NoMeaningfulLinkingService;
    }

    if let Some((provider, _)) = candidates.iter().find(|(_, has_assets)| *has_assets) {
        return CanonicalSelection::Selected {
            provider: *provider,
            has_valid_assets: true,
        };
    }

    CanonicalSelection::Selected {
        provider: candidates[0].0,
        has_valid_assets: false,
    }
}

/// Read canonical provider from persisted config slots.
pub fn canonical_provider(
    settings: &CanonicalProviderSettings,
    scope: ResourceScope,
    resource: LinkableResource,
) -> Option<Provider> {
    match (scope, resource) {
        (ResourceScope::User, LinkableResource::Skill) => settings.user_skill,
        (ResourceScope::User, LinkableResource::Command) => settings.user_command,
        (ResourceScope::User, LinkableResource::Agent) => settings.user_agent,
        (ResourceScope::User, LinkableResource::Script) => settings.user_script,
        (ResourceScope::Repo, LinkableResource::Skill) => settings.repo_skill,
        (ResourceScope::Repo, LinkableResource::Command) => settings.repo_command,
        (ResourceScope::Repo, LinkableResource::Agent) => settings.repo_agent,
        (ResourceScope::Repo, LinkableResource::Script) => settings.repo_script,
    }
}

/// Persist canonical provider into the proper `(scope, resource)` slot.
pub fn set_canonical_provider(
    settings: &mut CanonicalProviderSettings,
    scope: ResourceScope,
    resource: LinkableResource,
    provider: Provider,
) {
    match (scope, resource) {
        (ResourceScope::User, LinkableResource::Skill) => settings.user_skill = Some(provider),
        (ResourceScope::User, LinkableResource::Command) => settings.user_command = Some(provider),
        (ResourceScope::User, LinkableResource::Agent) => settings.user_agent = Some(provider),
        (ResourceScope::User, LinkableResource::Script) => settings.user_script = Some(provider),
        (ResourceScope::Repo, LinkableResource::Skill) => settings.repo_skill = Some(provider),
        (ResourceScope::Repo, LinkableResource::Command) => settings.repo_command = Some(provider),
        (ResourceScope::Repo, LinkableResource::Agent) => settings.repo_agent = Some(provider),
        (ResourceScope::Repo, LinkableResource::Script) => settings.repo_script = Some(provider),
    }
}

fn dedupe_providers(providers: &[Provider]) -> Vec<Provider> {
    let mut deduped = Vec::new();
    for provider in providers {
        if !deduped.contains(provider) {
            deduped.push(*provider);
        }
    }
    deduped
}

fn entrypoint_path(
    provider: Provider,
    resource: LinkableResource,
    scope: ResourceScope,
    home_dir: &Path,
    repo_root: &Path,
) -> Option<PathBuf> {
    let caps = capabilities_for(provider);
    let support = caps.support_for(resource);

    let base = match scope {
        ResourceScope::User => support.user_path.as_ref()?,
        ResourceScope::Repo => support.repo_path.as_ref()?,
    };

    if base.as_os_str().is_empty() {
        return None;
    }

    Some(match scope {
        ResourceScope::User => {
            if base.is_absolute() {
                base.clone()
            } else {
                home_dir.join(base)
            }
        }
        ResourceScope::Repo => {
            if base.is_absolute() {
                base.clone()
            } else {
                repo_root.join(base)
            }
        }
    })
}

fn is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
}

fn has_valid_assets(resource: LinkableResource, entrypoint: &Path) -> bool {
    if !entrypoint.exists() || !entrypoint.is_dir() {
        return false;
    }

    let entries = match std::fs::read_dir(entrypoint) {
        Ok(entries) => entries,
        Err(_) => return false,
    };

    match resource {
        LinkableResource::Skill => entries.filter_map(std::result::Result::ok).any(|entry| {
            let path = entry.path();
            let visible = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| !name.starts_with('.'))
                .unwrap_or(false);

            visible && path.is_dir() && path.join("SKILL.md").is_file()
        }),
        LinkableResource::Command => entries.filter_map(std::result::Result::ok).any(|entry| {
            let path = entry.path();
            let visible = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| !name.starts_with('.'))
                .unwrap_or(false);

            visible
                && path.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("md"))
                    .unwrap_or(false)
        }),
        LinkableResource::Agent | LinkableResource::Script => {
            entries.filter_map(std::result::Result::ok).any(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|name| !name.starts_with('.'))
                    .unwrap_or(false)
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn prompt_count_matches_design_rules() {
        assert_eq!(preference_prompt_count(0), 0);
        assert_eq!(preference_prompt_count(1), 0);
        assert_eq!(preference_prompt_count(2), 1);
        assert_eq!(preference_prompt_count(3), 2);
        assert_eq!(preference_prompt_count(4), 3);
        assert_eq!(preference_prompt_count(8), 3);
    }

    #[test]
    fn preference_order_appends_remaining_alphabetically() {
        let installed = [
            Provider::OpenCode,
            Provider::Claude,
            Provider::Codex,
            Provider::Gemini,
        ];
        let preferred = [Provider::Codex, Provider::OpenCode];

        let order = ranked_provider_preferences(&installed, &preferred);

        assert_eq!(order[0], Provider::Codex);
        assert_eq!(order[1], Provider::OpenCode);
        // Remaining providers sorted by display name: Claude, Gemini
        assert_eq!(order[2], Provider::Claude);
        assert_eq!(order[3], Provider::Gemini);
    }

    #[test]
    fn selects_first_provider_with_valid_assets() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let repo = temp.path().join("repo");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&repo).unwrap();

        create_skill_root(&home, ".claude/skills", false);
        create_skill_root(&home, ".codex/skills", true);
        create_skill_root(&home, ".gemini/skills", false);

        let installed = [Provider::Claude, Provider::Codex, Provider::Gemini];
        let preferred = [Provider::Claude, Provider::Codex, Provider::Gemini];

        let selected = select_canonical_provider(
            ResourceScope::User,
            LinkableResource::Skill,
            &installed,
            &preferred,
            &home,
            &repo,
        );

        assert_eq!(
            selected,
            CanonicalSelection::Selected {
                provider: Provider::Codex,
                has_valid_assets: true,
            }
        );
    }

    #[test]
    fn falls_back_to_first_candidate_when_no_assets_exist() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let repo = temp.path().join("repo");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&repo).unwrap();

        create_skill_root(&home, ".claude/skills", false);
        create_skill_root(&home, ".codex/skills", false);

        let installed = [Provider::Claude, Provider::Codex];
        let preferred = [Provider::Claude, Provider::Codex];

        let selected = select_canonical_provider(
            ResourceScope::User,
            LinkableResource::Skill,
            &installed,
            &preferred,
            &home,
            &repo,
        );

        assert_eq!(
            selected,
            CanonicalSelection::Selected {
                provider: Provider::Claude,
                has_valid_assets: false,
            }
        );
    }

    #[test]
    fn non_markdown_only_candidates_return_no_service() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let repo = temp.path().join("repo");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&repo).unwrap();

        // Gemini commands are TOML, so this should be filtered out for canonical.
        let installed = [Provider::Gemini];
        let selected = select_canonical_provider(
            ResourceScope::User,
            LinkableResource::Command,
            &installed,
            &installed,
            &home,
            &repo,
        );

        assert_eq!(selected, CanonicalSelection::NoMeaningfulLinkingService);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_entrypoint_is_filtered_from_candidates() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let repo = temp.path().join("repo");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&repo).unwrap();

        let claude_root = home.join(".claude/skills");
        let codex_root = home.join(".codex/skills");
        std::fs::create_dir_all(codex_root.join("serde")).unwrap();
        std::fs::write(codex_root.join("serde/SKILL.md"), "# serde\n").unwrap();

        std::fs::create_dir_all(claude_root.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&codex_root, &claude_root).unwrap();

        let installed = [Provider::Claude, Provider::Codex];
        let selected = select_canonical_provider(
            ResourceScope::User,
            LinkableResource::Skill,
            &installed,
            &installed,
            &home,
            &repo,
        );

        assert_eq!(
            selected,
            CanonicalSelection::Selected {
                provider: Provider::Codex,
                has_valid_assets: true,
            }
        );
    }

    #[test]
    fn canonical_slot_round_trip() {
        let mut settings = CanonicalProviderSettings::default();
        set_canonical_provider(
            &mut settings,
            ResourceScope::User,
            LinkableResource::Command,
            Provider::Codex,
        );
        set_canonical_provider(
            &mut settings,
            ResourceScope::Repo,
            LinkableResource::Skill,
            Provider::Claude,
        );

        assert_eq!(
            canonical_provider(&settings, ResourceScope::User, LinkableResource::Command),
            Some(Provider::Codex)
        );
        assert_eq!(
            canonical_provider(&settings, ResourceScope::Repo, LinkableResource::Skill),
            Some(Provider::Claude)
        );
    }

    fn create_skill_root(home: &Path, relative: &str, with_skill: bool) {
        let root = home.join(relative);
        std::fs::create_dir_all(&root).unwrap();
        if with_skill {
            let skill_dir = root.join("example");
            std::fs::create_dir_all(&skill_dir).unwrap();
            std::fs::write(skill_dir.join("SKILL.md"), "# example\n").unwrap();
        }
    }
}
