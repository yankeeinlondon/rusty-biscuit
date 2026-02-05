use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use super::discovery::DiscoveredSkill;

/// Status of a skill across providers after analysis.
#[derive(Debug, Clone)]
pub enum SkillSyncStatus {
    /// Skill exists in one provider -- candidate for linking to others.
    LinkCandidate {
        /// The source skill.
        source: DiscoveredSkill,
        /// Provider names that should receive a link.
        target_providers: Vec<String>,
    },
    /// Skill exists in multiple providers with identical content hash.
    InSync {
        /// Skill name.
        name: String,
        /// All providers that have this skill with their paths and hashes.
        providers: Vec<(String, PathBuf, u64)>,
    },
    /// Skill exists in multiple providers with different content hashes.
    Conflict {
        /// Skill name.
        name: String,
        /// Each provider's path and hash.
        versions: Vec<(String, PathBuf, u64)>,
    },
    /// Skill is already a symlink in all destination providers.
    AlreadyLinked {
        /// Skill name.
        name: String,
        /// The non-symlink source.
        source_provider: String,
    },
}

/// Mapping from provider name to directories it reads from other providers.
pub type AlsoReadsFrom = HashMap<String, Vec<PathBuf>>;

/// Build an `AlsoReadsFrom` map from a `ProviderSkillPaths`.
pub fn build_also_reads_from(
    paths: &super::paths::ProviderSkillPaths,
    providers: &[&str],
) -> AlsoReadsFrom {
    let mut map = AlsoReadsFrom::new();
    for &name in providers {
        let reads = paths.also_reads_from(name);
        if !reads.is_empty() {
            map.insert(name.to_string(), reads.to_vec());
        }
    }
    map
}

/// Analyze discovered skills and classify each by sync status.
///
/// Groups skills by name, then for each group:
/// - 1 provider (non-symlink) -> `LinkCandidate`
/// - Multiple providers, same hash -> `InSync`
/// - Multiple providers, different hashes -> `Conflict`
/// - All except one are symlinks -> `AlreadyLinked`
///
/// The `also_reads_from` map prevents redundant linking (e.g., OpenCode
/// reads Claude's skill directory, so we skip creating a link from Claude
/// to OpenCode).
pub fn analyze_skills(
    skills: Vec<DiscoveredSkill>,
    all_provider_names: &[&str],
    also_reads_from: &AlsoReadsFrom,
) -> Vec<SkillSyncStatus> {
    // Group by skill name
    let mut groups: BTreeMap<String, Vec<DiscoveredSkill>> = BTreeMap::new();
    for skill in skills {
        groups.entry(skill.name.clone()).or_default().push(skill);
    }

    let mut results = Vec::new();

    for (name, group) in groups {
        // Check if all non-source entries are symlinks
        let non_symlinks: Vec<&DiscoveredSkill> = group.iter().filter(|s| !s.is_symlink).collect();
        let symlinks: Vec<&DiscoveredSkill> = group.iter().filter(|s| s.is_symlink).collect();

        // If exactly one non-symlink and the rest are symlinks -> AlreadyLinked
        if non_symlinks.len() == 1 && !symlinks.is_empty() {
            results.push(SkillSyncStatus::AlreadyLinked {
                name,
                source_provider: non_symlinks[0].provider.clone(),
            });
            continue;
        }

        // Only one provider has this skill -> LinkCandidate
        if group.len() == 1 {
            let source = group.into_iter().next().unwrap();
            let source_provider = source.provider.clone();

            // Determine target providers (all except the source)
            let target_providers: Vec<String> = all_provider_names
                .iter()
                .filter(|&&p| p != source_provider)
                .filter(|&&target| {
                    // Skip if target reads from source's directories
                    let reads = also_reads_from
                        .get(target)
                        .map(|v| v.as_slice())
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
                .map(|p| p.to_string())
                .collect();

            results.push(SkillSyncStatus::LinkCandidate {
                source,
                target_providers,
            });
            continue;
        }

        // Multiple providers have this skill -- compare hashes
        let hashes: Vec<u64> = group.iter().filter_map(|s| s.hash).collect();

        if hashes.is_empty() {
            results.push(SkillSyncStatus::Conflict {
                name,
                versions: group
                    .into_iter()
                    .map(|s| (s.provider, s.path, s.hash.unwrap_or(0)))
                    .collect(),
            });
            continue;
        }

        let first_hash = hashes[0];
        let all_same = hashes.iter().all(|&h| h == first_hash);

        if all_same {
            results.push(SkillSyncStatus::InSync {
                name,
                providers: group
                    .into_iter()
                    .map(|s| (s.provider, s.path, s.hash.unwrap_or(0)))
                    .collect(),
            });
        } else {
            results.push(SkillSyncStatus::Conflict {
                name,
                versions: group
                    .into_iter()
                    .map(|s| (s.provider, s.path, s.hash.unwrap_or(0)))
                    .collect(),
            });
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_skill(name: &str, provider: &str, hash: u64, is_symlink: bool) -> DiscoveredSkill {
        DiscoveredSkill {
            name: name.to_string(),
            path: PathBuf::from(format!("/home/.{provider}/skills/{name}")),
            provider: provider.to_string(),
            is_symlink,
            hash: Some(hash),
        }
    }

    static ALL_PROVIDERS: &[&str] = &["claude", "gemini", "codex", "opencode"];

    fn no_reads() -> AlsoReadsFrom {
        AlsoReadsFrom::new()
    }

    #[test]
    fn single_provider_is_link_candidate() {
        let skills = vec![make_skill("clap", "claude", 123, false)];

        let results = analyze_skills(skills, ALL_PROVIDERS, &no_reads());

        assert_eq!(results.len(), 1);
        match &results[0] {
            SkillSyncStatus::LinkCandidate {
                source,
                target_providers,
            } => {
                assert_eq!(source.name, "clap");
                assert_eq!(source.provider, "claude");
                assert_eq!(target_providers.len(), 3);
            }
            other => panic!("expected LinkCandidate, got {other:?}"),
        }
    }

    #[test]
    fn same_hash_is_in_sync() {
        let skills = vec![
            make_skill("tokio", "claude", 999, false),
            make_skill("tokio", "gemini", 999, false),
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
            make_skill("react", "claude", 111, false),
            make_skill("react", "gemini", 222, false),
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
            make_skill("serde", "claude", 500, false),
            make_skill("serde", "gemini", 500, true),
        ];

        let results = analyze_skills(skills, ALL_PROVIDERS, &no_reads());

        assert_eq!(results.len(), 1);
        match &results[0] {
            SkillSyncStatus::AlreadyLinked {
                name,
                source_provider,
            } => {
                assert_eq!(name, "serde");
                assert_eq!(source_provider, "claude");
            }
            other => panic!("expected AlreadyLinked, got {other:?}"),
        }
    }

    #[test]
    fn opencode_skipped_when_source_is_claude() {
        let skill = DiscoveredSkill {
            name: "chrono".to_string(),
            path: PathBuf::from("/home/.claude/skills/chrono"),
            provider: "claude".to_string(),
            is_symlink: false,
            hash: Some(100),
        };
        let skills = vec![skill];

        let mut reads = AlsoReadsFrom::new();
        reads.insert(
            "opencode".to_string(),
            vec![PathBuf::from("/home/.claude/skills")],
        );

        let results = analyze_skills(skills, ALL_PROVIDERS, &reads);

        assert_eq!(results.len(), 1);
        match &results[0] {
            SkillSyncStatus::LinkCandidate {
                target_providers, ..
            } => {
                assert!(
                    !target_providers.contains(&"opencode".to_string()),
                    "opencode should be excluded because it reads from claude's path"
                );
                assert!(target_providers.contains(&"gemini".to_string()));
                assert!(target_providers.contains(&"codex".to_string()));
            }
            other => panic!("expected LinkCandidate, got {other:?}"),
        }
    }

    #[test]
    fn opencode_included_when_no_reads_from() {
        let skills = vec![make_skill("clap", "claude", 123, false)];

        let results = analyze_skills(skills, ALL_PROVIDERS, &no_reads());

        assert_eq!(results.len(), 1);
        match &results[0] {
            SkillSyncStatus::LinkCandidate {
                target_providers, ..
            } => {
                assert!(target_providers.contains(&"opencode".to_string()));
            }
            other => panic!("expected LinkCandidate, got {other:?}"),
        }
    }

    #[test]
    fn multiple_skills_analyzed_independently() {
        let skills = vec![
            make_skill("alpha", "claude", 1, false),
            make_skill("beta", "claude", 2, false),
            make_skill("beta", "gemini", 2, false),
            make_skill("gamma", "claude", 3, false),
            make_skill("gamma", "gemini", 4, false),
        ];

        let results = analyze_skills(skills, ALL_PROVIDERS, &no_reads());

        assert_eq!(results.len(), 3);

        // alpha: LinkCandidate (one provider)
        assert!(matches!(&results[0], SkillSyncStatus::LinkCandidate { .. }));
        // beta: InSync (same hash)
        assert!(matches!(&results[1], SkillSyncStatus::InSync { .. }));
        // gamma: Conflict (different hashes)
        assert!(matches!(&results[2], SkillSyncStatus::Conflict { .. }));
    }
}
