//! Package and package-area selectors resolved to a repository path prefix.
//!
//! Shared by focused `repo git-status` preparation and the CLI commands that
//! validate `--package` / `--package-area` before rendering. Every failure is a
//! [`SniffError::RepositorySelection`] whose message lists the valid names.

use super::types::Package;
use crate::{Result, SniffError};

/// Resolve and intersect optional package and package-area selectors.
///
/// Returns `Ok(None)` when both selectors are `None`. A package matches its
/// name exactly, ignoring case; an area matches by case-insensitive prefix,
/// keeping the catalog's spelling. The returned prefix always ends with `/`,
/// so `starts_with` filtering stops at whole path segments. With both
/// selectors, the package must lie inside the area and the narrower package
/// prefix is returned.
///
/// ## Errors
///
/// [`SniffError::RepositorySelection`] when a selector matches nothing (also
/// when the repository has no package catalog), or when the package lies in
/// another area. Validation runs even when no Git facts were collected.
pub fn resolve_package_and_area(
    packages: Option<&[Package]>,
    package: Option<&str>,
    area: Option<&str>,
) -> Result<Option<String>> {
    match (package, area) {
        (None, None) => Ok(None),
        (Some(name), None) => resolve_package_path(packages, name).map(Some),
        (None, Some(area)) => resolve_package_area_path(packages, area).map(Some),
        (Some(name), Some(area)) => {
            let pkg_path = resolve_package_path(packages, name)?;
            let area_path = resolve_package_area_path(packages, area)?;
            if !pkg_path.starts_with(&area_path) {
                let lower_name = name.to_lowercase();
                let real_area = packages
                    .and_then(|pkgs| pkgs.iter().find(|p| p.name.to_lowercase() == lower_name))
                    .map(|p| p.package_area.clone())
                    .unwrap_or_else(|| pkg_path.trim_end_matches('/').to_string());
                let requested = area_path_label(packages, area).unwrap_or_else(|| area.to_string());
                return Err(SniffError::RepositorySelection(format!(
                    "Package '{name}' is in area '{real_area}', not '{requested}'"
                )));
            }
            Ok(Some(pkg_path))
        }
    }
}

fn resolve_package_path(packages: Option<&[Package]>, pkg_name: &str) -> Result<String> {
    let Some(packages) = packages else {
        return Err(no_packages());
    };

    let lower = pkg_name.to_lowercase();

    if let Some(pkg) = packages.iter().find(|p| p.name.to_lowercase() == lower) {
        return Ok(format!("{}/", pkg.relative));
    }

    let mut names: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
    names.sort();
    names.dedup();

    Err(SniffError::RepositorySelection(format!(
        "Package '{}' not found.\n\nValid package names: {}",
        pkg_name,
        names.join(", "),
    )))
}

fn resolve_package_area_path(packages: Option<&[Package]>, area: &str) -> Result<String> {
    let Some(packages) = packages else {
        return Err(no_packages());
    };

    let lower = area.to_lowercase();

    // The top-level `""` area has no directory prefix to filter by.
    if let Some(pkg) = packages
        .iter()
        .filter(|p| !p.package_area.is_empty())
        .find(|p| p.package_area.to_lowercase().starts_with(&lower))
    {
        return Ok(format!("{}/", pkg.package_area));
    }

    let mut areas: Vec<&str> = packages
        .iter()
        .map(|p| p.package_area.as_str())
        .filter(|area| !area.is_empty())
        .collect();
    areas.sort();
    areas.dedup();

    Err(SniffError::RepositorySelection(format!(
        "Package area '{}' not found.\n\nValid package areas: {}",
        area,
        areas.join(", "),
    )))
}

/// The catalog's spelling of the first area the input prefixes.
fn area_path_label(packages: Option<&[Package]>, area: &str) -> Option<String> {
    let packages = packages?;
    let lower = area.to_lowercase();
    packages
        .iter()
        .find(|p| p.package_area.to_lowercase().starts_with(&lower))
        .map(|p| p.package_area.clone())
}

fn no_packages() -> SniffError {
    SniffError::RepositorySelection("No packages found in this repository".to_string())
}
