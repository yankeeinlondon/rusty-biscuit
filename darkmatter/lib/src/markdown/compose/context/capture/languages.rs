use super::*;

use super::datetime::string_array;
use super::snapshot::ContextCapture;

pub(super) const KEYS: &[&str] = &[
    "programming_languages_in_repo", "programming_language", "package_manager",
];

pub(super) fn populate_languages(cap: &ContextCapture, values: &mut Map<String, Value>) {
    let repo = cap.repo_info.as_ref();

    if !cap.has_git {
        values.insert("programming_languages_in_repo".into(), Value::Null);
        values.insert("programming_language".into(), Value::Null);
        values.insert("package_manager".into(), Value::Null);
        return;
    }

    let packages = repo.and_then(|r| r.packages.as_ref());
    let is_mono = repo.is_some_and(|r| r.is_monorepo);

    // All languages across packages
    let all_langs: Vec<String> = cap
        .languages
        .as_ref()
        .map(|languages| {
            languages
                .languages
                .iter()
                .map(|language| format!("{:?}", language.language))
                .collect()
        })
        .or_else(|| packages.map(|pkgs| {
            let mut langs: Vec<String> = pkgs
                .iter()
                .filter_map(|p| p.primary_language.as_ref())
                .map(|l| format!("{l:?}"))
                .collect();
            langs.sort();
            langs.dedup();
            langs
        }))
        .unwrap_or_default();

    values.insert(
        "programming_languages_in_repo".into(),
        if all_langs.is_empty() {
            Value::Null
        } else {
            string_array(all_langs.clone())
        },
    );

    // programming_language
    let programming_language = if is_mono {
        if let Some(ref pkg) = cap.current_package {
            // In a package: that package's primary language
            pkg.primary_language.as_ref().map(|l| format!("{l:?}"))
        } else if let Some(ref area) = cap.current_package_area {
            // In a package area: unique languages across packages in that area
            packages.map(|pkgs| {
                let mut area_langs: Vec<String> = pkgs
                    .iter()
                    .filter(|p| &p.package_area == area)
                    .filter_map(|p| p.primary_language.as_ref())
                    .map(|l| format!("{l:?}"))
                    .collect();
                area_langs.sort();
                area_langs.dedup();
                area_langs.join(", ")
            })
        } else {
            // In monorepo root but not in a package/area
            if all_langs.len() == 1 {
                Some(all_langs[0].clone())
            } else {
                None
            }
        }
    } else {
        // Not monorepo: repo's primary language
        cap.languages
            .as_ref()
            .and_then(|languages| languages.primary.as_ref())
            .map(|language| format!("{language:?}"))
            .or_else(|| all_langs.first().cloned())
    };
    values.insert(
        "programming_language".into(),
        programming_language.map_or(Value::Null, Value::String),
    );

    // package_manager
    let package_manager = if is_mono {
        if let Some(ref pkg) = cap.current_package {
            pkg.package_managers.first().cloned()
        } else if let Some(ref area) = cap.current_package_area {
            let mut managers: Vec<String> = packages
                .map(|pkgs| {
                    pkgs.iter()
                        .filter(|p| &p.package_area == area)
                        .filter_map(|p| p.package_managers.first().cloned())
                        .collect()
                })
                .unwrap_or_default();
            managers.sort();
            managers.dedup();
            if managers.len() == 1 {
                Some(managers.remove(0))
            } else {
                None
            }
        } else {
            let mut managers: Vec<String> = packages
                .map(|pkgs| {
                    pkgs.iter()
                        .filter_map(|p| p.package_managers.first().cloned())
                        .collect()
                })
                .unwrap_or_default();
            managers.sort();
            managers.dedup();
            if managers.len() == 1 {
                Some(managers.remove(0))
            } else {
                None
            }
        }
    } else {
        packages.and_then(|pkgs| {
            pkgs.first()
                .and_then(|p| p.package_managers.first().cloned())
        })
    };
    values.insert(
        "package_manager".into(),
        package_manager.map_or(Value::Null, Value::String),
    );
}

// ── Document context ──────────────────────────────────────────────
