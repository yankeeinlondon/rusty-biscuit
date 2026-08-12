use super::*;
use std::path::PathBuf;

use sniff::filesystem::blast_radius;
use sniff::filesystem::docs::MarkdownMeta;
use sniff::filesystem::repo::Package;

use super::datetime::string_array;
use super::snapshot::ContextCapture;

pub(super) const KEYS: &[&str] = &["docs_readme", "docs_blast_radius", "docs_drift", "docs_skill"];

pub(super) fn populate_docs(cap: &ContextCapture, values: &mut Map<String, Value>) {
    let docs = cap.docs.as_ref();
    let is_mono = cap.repo_info.as_ref().is_some_and(|r| r.is_monorepo);

    // Scope filter: filter docs to the active scope
    let scope_filter = |doc: &MarkdownMeta| -> bool {
        if !is_mono {
            return true;
        }
        if let Some(ref pkg) = cap.current_package {
            doc.package.as_deref() == Some(&pkg.name)
        } else if let Some(ref area) = cap.current_package_area {
            // In an area: include docs from packages in this area
            cap.repo_info
                .as_ref()
                .and_then(|r| r.packages.as_ref())
                .map(|pkgs| {
                    pkgs.iter()
                        .filter(|p| &p.package_area == area)
                        .any(|p| doc.package.as_deref() == Some(&p.name))
                })
                .unwrap_or(false)
        } else {
            true // repo-wide
        }
    };

    // docs_readme
    let readmes: Vec<String> = docs
        .map(|all_docs| {
            all_docs
                .iter()
                .filter(|d| scope_filter(d))
                .filter(|d| {
                    d.filepath
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.eq_ignore_ascii_case("readme.md"))
                        .unwrap_or(false)
                })
                .map(|d| d.relative.clone())
                .collect()
        })
        .unwrap_or_default();
    values.insert("docs_readme".into(), string_array(readmes));

    // docs_blast_radius: docs with blast_radius frontmatter
    let blast_radius_docs: Vec<String> = docs
        .map(|all_docs| {
            all_docs
                .iter()
                .filter(|d| scope_filter(d))
                .filter(|d| d.blast_radius.is_some())
                .map(|d| d.relative.clone())
                .collect()
        })
        .unwrap_or_default();
    values.insert(
        "docs_blast_radius".into(),
        string_array(blast_radius_docs),
    );

    // docs_drift: docs whose blast_radius intersects dirty source set.
    // Uses normalized PathBuf comparison consistent with sniff's
    // find_blast_radius_documents() rather than substring matching.
    let dirty_source: std::collections::HashSet<PathBuf> = cap
        .dirty_paths
        .iter()
        .filter(|p| blast_radius::is_source_code_path(p))
        .cloned()
        .collect();

    let drift_docs: Vec<String> = docs
        .map(|all_docs| {
            all_docs
                .iter()
                .filter(|d| scope_filter(d))
                .filter(|d| {
                    if let Some(ref br) = d.blast_radius {
                        br.iter().any(|p| dirty_source.contains(p))
                    } else {
                        false
                    }
                })
                .map(|d| d.relative.clone())
                .collect()
        })
        .unwrap_or_default();
    values.insert("docs_drift".into(), string_array(drift_docs));
}

// ── Skill context ─────────────────────────────────────────────────

pub(super) fn populate_skills(cap: &ContextCapture, values: &mut Map<String, Value>) {
    values.insert(
        "docs_skill".into(),
        cap.best_skill
            .clone()
            .map_or(Value::Null, Value::String),
    );
}

pub(super) fn find_best_skill(
    repo_root: &Path,
    current_package: Option<&Package>,
    current_area: Option<&str>,
) -> Option<String> {
    let skill_dirs = [".claude/skills", ".agents/skills"];

    for base in &skill_dirs {
        let skills_dir = repo_root.join(base);
        if !skills_dir.is_dir() {
            continue;
        }

        let entries: Vec<_> = std::fs::read_dir(&skills_dir)
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();

        // Try matching by package name, then area name, then repo name
        let target_names: Vec<&str> = [
            current_package.map(|p| p.name.as_str()),
            current_area,
            repo_root.file_name().and_then(|n| n.to_str()),
        ]
        .into_iter()
        .flatten()
        .collect();

        for target in &target_names {
            for entry in &entries {
                let dir_name = entry.file_name();
                let dir_name_str = dir_name.to_string_lossy();
                if dir_name_str == *target {
                    let skill_file = entry.path().join("SKILL.md");
                    if skill_file.exists() {
                        let relative = skill_file
                            .strip_prefix(repo_root)
                            .ok()?
                            .to_string_lossy()
                            .to_string();
                        return Some(relative);
                    }
                }
            }
        }
    }

    None
}

// ── OS context ────────────────────────────────────────────────────
