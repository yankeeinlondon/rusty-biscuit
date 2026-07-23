use super::model::{
    ClassificationSource, FileAssociation, FileAssociationBreakdown, FileAssociationStats,
    FileInventory, FrameworkKind, FrameworkStats, LanguageSummary, ProgrammingLanguage,
    ProgrammingLanguageStats,
};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

const DIRECT_LANGUAGE_WEIGHT: f64 = 1.0;
const EXPLICIT_FRAMEWORK_WEIGHT: f64 = 1.0;
const INFERRED_FRAMEWORK_WEIGHT: f64 = 0.75;

#[derive(Default)]
pub(crate) struct LanguageAccumulator {
    pub(crate) direct_files: Vec<PathBuf>,
    pub(crate) framework_files: Vec<PathBuf>,
    pub(crate) explicit_framework_count: usize,
    pub(crate) inferred_framework_count: usize,
}

#[derive(Default)]
pub(crate) struct FrameworkAccumulator {
    pub(crate) files: Vec<PathBuf>,
    pub(crate) explicit_file_count: usize,
    pub(crate) inferred_file_count: usize,
    pub(crate) related_languages: BTreeSet<ProgrammingLanguage>,
}

pub fn summarize_file_inventory(
    inventory: &FileInventory,
) -> (FileAssociationBreakdown, LanguageSummary) {
    let lang_summary = summarize_languages(inventory);
    let breakdown = FileAssociationBreakdown {
        total_files: inventory.total_files_scanned,
        by_association: summarize_associations(inventory),
        by_language: lang_summary.languages.clone(),
        by_framework: lang_summary.frameworks.clone(),
        truncated: inventory.truncated,
        limit: inventory.limit,
    };
    (breakdown, lang_summary)
}

pub fn summarize_languages(inventory: &FileInventory) -> LanguageSummary {
    let mut language_map: HashMap<ProgrammingLanguage, LanguageAccumulator> = HashMap::new();
    let mut framework_map: HashMap<FrameworkKind, FrameworkAccumulator> = HashMap::new();

    for classification in inventory.classifications.iter() {
        accumulate_language_classification(classification, &mut language_map, &mut framework_map);
    }

    build_language_summary(language_map, framework_map, inventory.total_files_scanned)
        .with_completeness_of(inventory)
}

/// Accumulate a single classification into language and framework maps.
pub(crate) fn accumulate_language_classification(
    classification: &super::model::FileClassification,
    language_map: &mut HashMap<ProgrammingLanguage, LanguageAccumulator>,
    framework_map: &mut HashMap<FrameworkKind, FrameworkAccumulator>,
) {
    if classification.association == FileAssociation::ProgrammingLanguage
        && let Some(language) = classification.language
    {
        language_map
            .entry(language)
            .or_default()
            .direct_files
            .push(classification.path.clone());
    }

    if classification.association == FileAssociation::FrameworkFile
        && let Some(framework) = classification.framework
    {
        let framework_acc = framework_map.entry(framework).or_default();
        framework_acc.files.push(classification.path.clone());

        let explicit = classification.source == ClassificationSource::EmbeddedLanguageHint;
        if explicit {
            framework_acc.explicit_file_count += 1;
        } else {
            framework_acc.inferred_file_count += 1;
        }

        for language in &classification.related_languages {
            framework_acc.related_languages.insert(*language);
            let lang_acc = language_map.entry(*language).or_default();
            lang_acc.framework_files.push(classification.path.clone());
            if explicit {
                lang_acc.explicit_framework_count += 1;
            } else {
                lang_acc.inferred_framework_count += 1;
            }
        }
    }
}

/// Build a [`LanguageSummary`] from accumulated maps.
pub(crate) fn build_language_summary(
    language_map: HashMap<ProgrammingLanguage, LanguageAccumulator>,
    framework_map: HashMap<FrameworkKind, FrameworkAccumulator>,
    total_files_scanned: usize,
) -> LanguageSummary {
    let total_language_files = language_map
        .values()
        .map(|entry| entry.direct_files.len() + entry.framework_files.len())
        .sum();

    let mut languages: Vec<ProgrammingLanguageStats> = language_map
        .into_iter()
        .map(|(language, entry)| {
            let direct_file_count = entry.direct_files.len();
            let framework_file_count = entry.framework_files.len();
            let total_file_count = direct_file_count + framework_file_count;
            let signal = direct_file_count as f64 * DIRECT_LANGUAGE_WEIGHT
                + entry.explicit_framework_count as f64 * EXPLICIT_FRAMEWORK_WEIGHT
                + entry.inferred_framework_count as f64 * INFERRED_FRAMEWORK_WEIGHT;

            ProgrammingLanguageStats {
                language,
                language_type: language.language_type(),
                direct_file_count,
                framework_file_count,
                total_file_count,
                signal,
                percentage: if total_language_files > 0 {
                    (total_file_count as f64 / total_language_files as f64) * 100.0
                } else {
                    0.0
                },
                direct_files: entry.direct_files,
                framework_files: entry.framework_files,
            }
        })
        .collect();

    languages.sort_by(|left, right| {
        right
            .signal
            .total_cmp(&left.signal)
            .then(right.direct_file_count.cmp(&left.direct_file_count))
            .then(right.framework_file_count.cmp(&left.framework_file_count))
            .then_with(|| left.language.as_str().cmp(right.language.as_str()))
    });

    let primary = languages.first().map(|stats| stats.language);
    let secondary = languages
        .iter()
        .skip(1)
        .map(|stats| stats.language)
        .collect();

    let mut frameworks: Vec<FrameworkStats> = framework_map
        .into_iter()
        .map(|(framework, entry)| FrameworkStats {
            framework,
            file_count: entry.files.len(),
            explicit_file_count: entry.explicit_file_count,
            inferred_file_count: entry.inferred_file_count,
            related_languages: entry.related_languages.into_iter().collect(),
            files: entry.files,
        })
        .collect();

    frameworks.sort_by(|left, right| {
        right
            .file_count
            .cmp(&left.file_count)
            .then_with(|| left.framework.as_str().cmp(right.framework.as_str()))
    });

    // Completeness belongs to the inventory a summary was projected from, and
    // this builder is also called with maps accumulated outside any inventory
    // (per-package summaries in `repo::detection`). Callers that do have an
    // inventory stamp the pair via `LanguageSummary::with_completeness_of`.
    LanguageSummary {
        primary,
        secondary,
        total_files_scanned,
        total_language_files,
        languages,
        frameworks,
        truncated: false,
        limit: None,
    }
}

fn summarize_associations(inventory: &FileInventory) -> Vec<FileAssociationStats> {
    let mut associations: HashMap<FileAssociation, Vec<PathBuf>> = HashMap::new();

    for classification in inventory.classifications.iter() {
        associations
            .entry(classification.association)
            .or_default()
            .push(classification.path.clone());
    }

    build_association_breakdown(associations, inventory.total_files_scanned)
}

/// Build a [`Vec<FileAssociationStats>`] from an association map.
pub(crate) fn build_association_breakdown(
    associations: HashMap<FileAssociation, Vec<PathBuf>>,
    total_files: usize,
) -> Vec<FileAssociationStats> {
    let mut stats: Vec<FileAssociationStats> = associations
        .into_iter()
        .map(|(association, files)| {
            let file_count = files.len();
            FileAssociationStats {
                association,
                file_count,
                percentage: if total_files > 0 {
                    (file_count as f64 / total_files as f64) * 100.0
                } else {
                    0.0
                },
                files,
            }
        })
        .collect();

    stats.sort_by(|left, right| {
        right
            .file_count
            .cmp(&left.file_count)
            .then_with(|| left.association.as_str().cmp(right.association.as_str()))
    });
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::file_types::classify::scan_file_inventory;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn language_summary_ignores_configuration_and_docs() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("lib.rs"), "pub fn hi() {}").unwrap();
        fs::write(dir.path().join("README.md"), "# docs").unwrap();
        fs::write(dir.path().join("config.toml"), "name = 'x'").unwrap();

        let inventory = scan_file_inventory(dir.path()).unwrap();
        let summary = summarize_languages(&inventory);

        assert_eq!(summary.primary, Some(ProgrammingLanguage::Rust));
        assert_eq!(summary.total_language_files, 1);
        assert_eq!(summary.languages.len(), 1);
    }

    #[test]
    fn framework_default_language_counts_toward_js() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Component.vue"),
            "<template><div>Hello</div></template>",
        )
        .unwrap();

        let inventory = scan_file_inventory(dir.path()).unwrap();
        let summary = summarize_languages(&inventory);

        assert_eq!(summary.primary, Some(ProgrammingLanguage::JavaScript));
        assert_eq!(summary.languages[0].framework_file_count, 1);
        assert_eq!(summary.frameworks[0].framework, FrameworkKind::Vue);
    }

    #[test]
    fn summarize_file_inventory_language_data_matches_summarize_languages() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("lib.rs"), "pub fn hi() {}").unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(
            dir.path().join("Component.vue"),
            "<template><div>Hello</div></template>",
        )
        .unwrap();

        let inventory = scan_file_inventory(dir.path()).unwrap();
        let lang_summary = summarize_languages(&inventory);
        let (breakdown, returned_lang_summary) = summarize_file_inventory(&inventory);

        // Language data in the breakdown must match the standalone summarize_languages output
        assert_eq!(breakdown.by_language.len(), lang_summary.languages.len());
        assert_eq!(breakdown.by_framework.len(), lang_summary.frameworks.len());
        for (breakdown_lang, lang) in breakdown.by_language.iter().zip(&lang_summary.languages) {
            assert_eq!(breakdown_lang.language, lang.language);
            assert_eq!(breakdown_lang.total_file_count, lang.total_file_count);
        }

        // The returned LanguageSummary must be identical to the standalone result
        assert_eq!(returned_lang_summary.primary, lang_summary.primary);
        assert_eq!(
            returned_lang_summary.total_language_files,
            lang_summary.total_language_files
        );
        assert_eq!(
            returned_lang_summary.languages.len(),
            lang_summary.languages.len()
        );
        assert_eq!(
            returned_lang_summary.frameworks.len(),
            lang_summary.frameworks.len()
        );
    }
}
