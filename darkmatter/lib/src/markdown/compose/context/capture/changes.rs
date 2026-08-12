use super::*;
use std::path::Path;

use sniff::filesystem::blast_radius;
use sniff::filesystem::repo::Package;

use super::datetime::string_array;
use super::snapshot::ContextCapture;

pub(super) const KEYS: &[&str] = &[
    "dirty_files", "dirty_source_code_files", "staged_files", "untracked_files",
    "dirty_packages", "dirty_package_areas", "staged_packages", "staged_package_areas",
    "current_package_has_staged_files", "current_package_area_has_staged_files",
    "current_package_has_dirty_files", "current_package_area_has_dirty_files",
];

pub(super) fn populate_file_changes(cap: &ContextCapture, values: &mut Map<String, Value>) {
    // Dirty files
    let mut dirty: Vec<String> = cap
        .dirty_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    dirty.sort();

    // Dirty source code files
    let dirty_source: Vec<String> = dirty
        .iter()
        .filter(|p| blast_radius::is_source_code_path(Path::new(p.as_str())))
        .cloned()
        .collect();

    // Staged files
    let mut staged: Vec<String> = cap
        .staged_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    staged.sort();

    // Untracked files
    let mut untracked: Vec<String> = cap
        .untracked_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    untracked.sort();

    values.insert("dirty_files".into(), string_array(dirty));
    values.insert(
        "dirty_source_code_files".into(),
        string_array(dirty_source),
    );
    values.insert("staged_files".into(), string_array(staged));
    values.insert("untracked_files".into(), string_array(untracked));
}

// ── Package/area dirty/staged context ─────────────────────────────

pub(super) fn populate_package_changes(cap: &ContextCapture, values: &mut Map<String, Value>) {
    let repo = cap.repo_info.as_ref();
    let packages = repo.and_then(|r| r.packages.as_ref());
    let is_mono = repo.is_some_and(|r| r.is_monorepo);

    if !is_mono || packages.is_none() {
        // No packages: empty arrays (the variables are required, so not null).
        values.insert("dirty_packages".into(), string_array(Vec::new()));
        values.insert("dirty_package_areas".into(), string_array(Vec::new()));
        values.insert("staged_packages".into(), string_array(Vec::new()));
        values.insert("staged_package_areas".into(), string_array(Vec::new()));
        values.insert(
            "current_package_has_staged_files".into(),
            Value::Bool(false),
        );
        values.insert(
            "current_package_area_has_staged_files".into(),
            Value::Bool(false),
        );
        values.insert("current_package_has_dirty_files".into(), Value::Bool(false));
        values.insert(
            "current_package_area_has_dirty_files".into(),
            Value::Bool(false),
        );
        return;
    }

    let pkgs = packages.unwrap();

    // Helper: check if a repo-relative path belongs to a package
    let path_in_package = |path: &Path, pkg: &Package| -> bool {
        let pkg_rel = &pkg.relative;
        let path_str = path.to_string_lossy();
        path_str.starts_with(pkg_rel)
    };

    // Dirty packages
    let mut dirty_pkg_names: Vec<String> = pkgs
        .iter()
        .filter(|pkg| cap.dirty_paths.iter().any(|p| path_in_package(p, pkg)))
        .map(|p| p.name.clone())
        .collect();
    dirty_pkg_names.sort();
    dirty_pkg_names.dedup();

    let mut dirty_area_names: Vec<String> = pkgs
        .iter()
        .filter(|pkg| cap.dirty_paths.iter().any(|p| path_in_package(p, pkg)))
        .map(|p| p.package_area.clone())
        .collect();
    dirty_area_names.sort();
    dirty_area_names.dedup();

    values.insert("dirty_packages".into(), string_array(dirty_pkg_names.clone()));
    values.insert(
        "dirty_package_areas".into(),
        string_array(dirty_area_names.clone()),
    );

    // Staged packages
    let mut staged_pkg_names: Vec<String> = pkgs
        .iter()
        .filter(|pkg| cap.staged_paths.iter().any(|p| path_in_package(p, pkg)))
        .map(|p| p.name.clone())
        .collect();
    staged_pkg_names.sort();
    staged_pkg_names.dedup();

    let mut staged_area_names: Vec<String> = pkgs
        .iter()
        .filter(|pkg| cap.staged_paths.iter().any(|p| path_in_package(p, pkg)))
        .map(|p| p.package_area.clone())
        .collect();
    staged_area_names.sort();
    staged_area_names.dedup();

    values.insert(
        "staged_packages".into(),
        string_array(staged_pkg_names.clone()),
    );
    values.insert(
        "staged_package_areas".into(),
        string_array(staged_area_names.clone()),
    );

    // Current package/area boolean flags
    let cur_pkg_name = cap.current_package.as_ref().map(|p| &p.name);
    let cur_area = cap.current_package_area.as_deref();

    values.insert(
        "current_package_has_staged_files".into(),
        Value::Bool(cur_pkg_name.is_some_and(|name| staged_pkg_names.contains(name))),
    );
    values.insert(
        "current_package_area_has_staged_files".into(),
        Value::Bool(cur_area.is_some_and(|area| staged_area_names.iter().any(|a| a == area))),
    );
    values.insert(
        "current_package_has_dirty_files".into(),
        Value::Bool(cur_pkg_name.is_some_and(|name| dirty_pkg_names.contains(name))),
    );
    values.insert(
        "current_package_area_has_dirty_files".into(),
        Value::Bool(cur_area.is_some_and(|area| dirty_area_names.iter().any(|a| a == area))),
    );
}

// ── Programming language and package manager ──────────────────────
