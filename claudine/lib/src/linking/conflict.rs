use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use crate::events::Provider;

use super::capabilities::LinkableResource;
use super::discovery::DiscoveredSkill;
use super::paths::{ProviderSkillPaths, ResourceScope};

/// Status of a resource across providers after analysis.
#[derive(Debug, Clone)]
pub enum SkillSyncStatus {
    /// Resource exists in one provider and can be linked to others.
    LinkCandidate {
        /// The source resource.
        source: DiscoveredSkill,
        /// Providers that should receive a link.
        target_providers: Vec<Provider>,
    },
    /// Resource exists in multiple providers with identical content hash.
    InSync {
        /// Resource name.
        name: String,
        /// Providers and their paths/hashes.
        providers: Vec<(Provider, PathBuf, u64)>,
    },
    /// Resource exists in multiple providers with different content hashes.
    Conflict {
        /// Resource name.
        name: String,
        /// Providers and their paths/hashes.
        versions: Vec<(Provider, PathBuf, u64)>,
    },
    /// Resource is already linked from one source to other providers.
    AlreadyLinked {
        /// Resource name.
        name: String,
        /// Source provider.
        source_provider: Provider,
    },
}

/// Mapping from provider to directories it also reads from.
pub type AlsoReadsFrom = HashMap<Provider, Vec<PathBuf>>;

/// Build an `AlsoReadsFrom` map from capability-backed paths.
pub fn build_also_reads_from(
    paths: &ProviderSkillPaths,
    providers: &[Provider],
    resource: LinkableResource,
    scope: ResourceScope,
) -> AlsoReadsFrom {
    let mut map = AlsoReadsFrom::new();
    for provider in providers {
        let reads = paths.also_reads_from(*provider, resource, scope);
        if !reads.is_empty() {
            map.insert(*provider, reads);
        }
    }
    map
}

/// Analyze discovered resources and classify each by sync status.
pub fn analyze_skills(
    skills: Vec<DiscoveredSkill>,
    all_provider_names: &[Provider],
    also_reads_from: &AlsoReadsFrom,
) -> Vec<SkillSyncStatus> {
    let mut groups: BTreeMap<String, Vec<DiscoveredSkill>> = BTreeMap::new();
    for skill in skills {
        groups.entry(skill.name.clone()).or_default().push(skill);
    }

    let mut results = Vec::new();

    for (name, group) in groups {
        let non_symlinks: Vec<&DiscoveredSkill> =
            group.iter().filter(|skill| !skill.is_symlink).collect();
        let symlinks: Vec<&DiscoveredSkill> =
            group.iter().filter(|skill| skill.is_symlink).collect();

        if non_symlinks.len() == 1 && !symlinks.is_empty() {
            results.push(SkillSyncStatus::AlreadyLinked {
                name,
                source_provider: non_symlinks[0].provider,
            });
            continue;
        }

        if group.len() == 1 {
            let source = group.into_iter().next().expect("single-item group");
            let source_provider = source.provider;

            let target_providers: Vec<Provider> = all_provider_names
                .iter()
                .copied()
                .filter(|provider| *provider != source_provider)
                .filter(|target| {
                    let reads = also_reads_from
                        .get(target)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]);
                    !reads.iter().any(|read_path| {
                        source
                            .path
                            .parent()
                            .map(|parent| {
                                parent.starts_with(read_path)
                                    || read_path.starts_with(parent)
                                    || parent == read_path.as_path()
                            })
                            .unwrap_or(false)
                    })
                })
                .collect();

            results.push(SkillSyncStatus::LinkCandidate {
                source,
                target_providers,
            });
            continue;
        }

        let hashes: Vec<u64> = group.iter().filter_map(|skill| skill.hash).collect();

        if hashes.is_empty() {
            results.push(SkillSyncStatus::Conflict {
                name,
                versions: group
                    .into_iter()
                    .map(|skill| (skill.provider, skill.path, skill.hash.unwrap_or(0)))
                    .collect(),
            });
            continue;
        }

        let first_hash = hashes[0];
        let all_same = hashes.iter().all(|hash| *hash == first_hash);

        if all_same {
            results.push(SkillSyncStatus::InSync {
                name,
                providers: group
                    .into_iter()
                    .map(|skill| (skill.provider, skill.path, skill.hash.unwrap_or(0)))
                    .collect(),
            });
        } else {
            results.push(SkillSyncStatus::Conflict {
                name,
                versions: group
                    .into_iter()
                    .map(|skill| (skill.provider, skill.path, skill.hash.unwrap_or(0)))
                    .collect(),
            });
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_skill(name: &str, provider: Provider, hash: u64, is_symlink: bool) -> DiscoveredSkill {
        DiscoveredSkill {
            name: name.to_string(),
            path: PathBuf::from(format!("/home/.{}/skills/{name}", provider.as_slug())),
            provider,
            is_symlink,
            hash: Some(hash),
        }
    }

    static ALL_PROVIDERS: &[Provider] = &[
        Provider::Claude,
        Provider::Codex,
        Provider::Gemini,
        Provider::OpenCode,
    ];

    fn no_reads() -> AlsoReadsFrom {
        AlsoReadsFrom::new()
    }

    #[test]
    fn single_provider_is_link_candidate() {
        let skills = vec![make_skill("clap", Provider::Claude, 123, false)];

        let results = analyze_skills(skills, ALL_PROVIDERS, &no_reads());

        assert_eq!(results.len(), 1);
        match &results[0] {
            SkillSyncStatus::LinkCandidate {
                source,
                target_providers,
            } => {
                assert_eq!(source.name, "clap");
                assert_eq!(source.provider, Provider::Claude);
                assert_eq!(target_providers.len(), 3);
            }
            other => panic!("expected LinkCandidate, got {other:?}"),
        }
    }

    #[test]
    fn same_hash_is_in_sync() {
        let skills = vec![
            make_skill("tokio", Provider::Claude, 999, false),
            make_skill("tokio", Provider::Gemini, 999, false),
        ];

        let results = analyze_skills(skills, ALL_PROVIDERS, &no_reads());

        assert_eq!(results.len(), 1);
        match &results[0] {
            SkillSyncStatus::InSync { name, providers } => {
                assert_eq!(name, "tokio");
                assert_eq!(providers.len(), 2);
            }
            other => panic!("expected InSync, got {other:?}"),
        }
    }

    #[test]
    fn different_hash_is_conflict() {
        let skills = vec![
            make_skill("react", Provider::Claude, 111, false),
            make_skill("react", Provider::Gemini, 222, false),
        ];

        let results = analyze_skills(skills, ALL_PROVIDERS, &no_reads());

        assert_eq!(results.len(), 1);
        match &results[0] {
            SkillSyncStatus::Conflict { name, versions } => {
                assert_eq!(name, "react");
                assert_eq!(versions.len(), 2);
                assert_ne!(versions[0].2, versions[1].2);
            }
            other => panic!("expected Conflict, got {other:?}"),
        }
    }

    #[test]
    fn symlinks_detected_as_already_linked() {
        let skills = vec![
            make_skill("serde", Provider::Claude, 500, false),
            make_skill("serde", Provider::Gemini, 500, true),
        ];

        let results = analyze_skills(skills, ALL_PROVIDERS, &no_reads());

        assert_eq!(results.len(), 1);
        match &results[0] {
            SkillSyncStatus::AlreadyLinked {
                name,
                source_provider,
            } => {
                assert_eq!(name, "serde");
                assert_eq!(*source_provider, Provider::Claude);
            }
            other => panic!("expected AlreadyLinked, got {other:?}"),
        }
    }

    #[test]
    fn opencode_skipped_when_source_is_claude() {
        let skill = DiscoveredSkill {
            name: "chrono".to_string(),
            path: PathBuf::from("/home/.claude/skills/chrono"),
            provider: Provider::Claude,
            is_symlink: false,
            hash: Some(100),
        };
        let skills = vec![skill];

        let mut reads = AlsoReadsFrom::new();
        reads.insert(
            Provider::OpenCode,
            vec![PathBuf::from("/home/.claude/skills")],
        );

        let results = analyze_skills(skills, ALL_PROVIDERS, &reads);

        assert_eq!(results.len(), 1);
        match &results[0] {
            SkillSyncStatus::LinkCandidate {
                target_providers, ..
            } => {
                assert!(
                    !target_providers.contains(&Provider::OpenCode),
                    "opencode should be excluded because it reads from claude's path"
                );
                assert!(target_providers.contains(&Provider::Gemini));
                assert!(target_providers.contains(&Provider::Codex));
            }
            other => panic!("expected LinkCandidate, got {other:?}"),
        }
    }

    #[test]
    fn opencode_included_when_no_reads_from() {
        let skills = vec![make_skill("clap", Provider::Claude, 123, false)];

        let results = analyze_skills(skills, ALL_PROVIDERS, &no_reads());

        assert_eq!(results.len(), 1);
        match &results[0] {
            SkillSyncStatus::LinkCandidate {
                target_providers, ..
            } => {
                assert!(target_providers.contains(&Provider::OpenCode));
            }
            other => panic!("expected LinkCandidate, got {other:?}"),
        }
    }

    #[test]
    fn multiple_skills_analyzed_independently() {
        let skills = vec![
            make_skill("alpha", Provider::Claude, 1, false),
            make_skill("beta", Provider::Claude, 2, false),
            make_skill("beta", Provider::Gemini, 2, false),
            make_skill("gamma", Provider::Claude, 3, false),
            make_skill("gamma", Provider::Gemini, 4, false),
        ];

        let results = analyze_skills(skills, ALL_PROVIDERS, &no_reads());

        assert_eq!(results.len(), 3);
        assert!(matches!(&results[0], SkillSyncStatus::LinkCandidate { .. }));
        assert!(matches!(&results[1], SkillSyncStatus::InSync { .. }));
        assert!(matches!(&results[2], SkillSyncStatus::Conflict { .. }));
    }
}
