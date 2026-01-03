use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::{Result, SniffError};

/// Supported monorepo tools and package managers
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MonorepoTool {
    /// Rust Cargo workspace
    CargoWorkspace,
    /// npm workspaces
    NpmWorkspaces,
    /// pnpm workspaces
    PnpmWorkspaces,
    /// Yarn workspaces
    YarnWorkspaces,
    /// Nx monorepo tool
    Nx,
    /// Turborepo
    Turborepo,
    /// Lerna
    Lerna,
    /// Unknown monorepo tool
    Unknown,
}

/// Information about a detected monorepo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonorepoInfo {
    /// Whether this is a monorepo
    pub is_monorepo: bool,
    /// The tool managing the monorepo
    pub tool: MonorepoTool,
    /// Root directory of the monorepo
    pub root: PathBuf,
    /// Package locations within the monorepo
    pub packages: Vec<PackageLocation>,
}

/// A package within a monorepo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageLocation {
    /// Package name
    pub name: String,
    /// Path to the package
    pub path: PathBuf,
}

/// Detect monorepo configuration in the given directory.
///
/// ## Examples
///
/// ```no_run
/// use std::path::Path;
/// use sniff_lib::filesystem::monorepo::detect_monorepo;
///
/// let root = Path::new("/path/to/project");
/// if let Some(info) = detect_monorepo(root).unwrap() {
///     println!("Monorepo tool: {:?}", info.tool);
///     println!("Packages: {}", info.packages.len());
/// }
/// ```
///
/// ## Returns
///
/// - `Ok(Some(MonorepoInfo))` if a monorepo is detected
/// - `Ok(None)` if no monorepo configuration is found
/// - `Err(SniffError)` if there's an error reading files
pub fn detect_monorepo(root: &Path) -> Result<Option<MonorepoInfo>> {
    // Check in priority order (more specific tools first)
    if let Some(info) = detect_cargo_workspace(root)? {
        return Ok(Some(info));
    }
    if let Some(info) = detect_nx(root)? {
        return Ok(Some(info));
    }
    if let Some(info) = detect_turborepo(root)? {
        return Ok(Some(info));
    }
    if let Some(info) = detect_pnpm_workspace(root)? {
        return Ok(Some(info));
    }
    if let Some(info) = detect_npm_workspace(root)? {
        return Ok(Some(info));
    }
    if let Some(info) = detect_lerna(root)? {
        return Ok(Some(info));
    }
    Ok(None)
}

fn detect_cargo_workspace(root: &Path) -> Result<Option<MonorepoInfo>> {
    let cargo_toml = root.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&cargo_toml)?;
    let parsed: toml::Value = toml::from_str(&content).map_err(|e| SniffError::SystemInfo {
        domain: "monorepo",
        message: e.to_string(),
    })?;

    let workspace = match parsed.get("workspace") {
        Some(w) => w,
        None => return Ok(None),
    };

    let members = workspace
        .get("members")
        .and_then(|m| m.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();

    if members.is_empty() {
        return Ok(None);
    }

    // Expand globs and collect packages
    let packages = expand_glob_patterns(root, &members);

    Ok(Some(MonorepoInfo {
        is_monorepo: true,
        tool: MonorepoTool::CargoWorkspace,
        root: root.to_path_buf(),
        packages,
    }))
}

fn detect_pnpm_workspace(root: &Path) -> Result<Option<MonorepoInfo>> {
    let pnpm_workspace = root.join("pnpm-workspace.yaml");
    if !pnpm_workspace.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&pnpm_workspace)?;
    let parsed: serde_yaml::Value =
        serde_yaml::from_str(&content).map_err(|e| SniffError::SystemInfo {
            domain: "monorepo",
            message: e.to_string(),
        })?;

    let packages = parsed
        .get("packages")
        .and_then(|p| p.as_sequence())
        .map(|seq| seq.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();

    if packages.is_empty() {
        return Ok(None);
    }

    let package_locations = expand_glob_patterns(root, &packages);

    Ok(Some(MonorepoInfo {
        is_monorepo: true,
        tool: MonorepoTool::PnpmWorkspaces,
        root: root.to_path_buf(),
        packages: package_locations,
    }))
}

fn detect_npm_workspace(root: &Path) -> Result<Option<MonorepoInfo>> {
    let package_json = root.join("package.json");
    if !package_json.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&package_json)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| SniffError::SystemInfo {
            domain: "monorepo",
            message: e.to_string(),
        })?;

    let workspaces = parsed
        .get("workspaces")
        .and_then(|w| w.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();

    if workspaces.is_empty() {
        return Ok(None);
    }

    let packages = expand_glob_patterns(root, &workspaces);

    Ok(Some(MonorepoInfo {
        is_monorepo: true,
        tool: MonorepoTool::NpmWorkspaces,
        root: root.to_path_buf(),
        packages,
    }))
}

fn detect_nx(root: &Path) -> Result<Option<MonorepoInfo>> {
    let nx_json = root.join("nx.json");
    if !nx_json.exists() {
        return Ok(None);
    }

    Ok(Some(MonorepoInfo {
        is_monorepo: true,
        tool: MonorepoTool::Nx,
        root: root.to_path_buf(),
        packages: vec![], // Nx projects would need deeper parsing
    }))
}

fn detect_turborepo(root: &Path) -> Result<Option<MonorepoInfo>> {
    let turbo_json = root.join("turbo.json");
    if !turbo_json.exists() {
        return Ok(None);
    }

    Ok(Some(MonorepoInfo {
        is_monorepo: true,
        tool: MonorepoTool::Turborepo,
        root: root.to_path_buf(),
        packages: vec![], // Would need to parse package.json workspaces
    }))
}

fn detect_lerna(root: &Path) -> Result<Option<MonorepoInfo>> {
    let lerna_json = root.join("lerna.json");
    if !lerna_json.exists() {
        return Ok(None);
    }

    Ok(Some(MonorepoInfo {
        is_monorepo: true,
        tool: MonorepoTool::Lerna,
        root: root.to_path_buf(),
        packages: vec![],
    }))
}

fn expand_glob_patterns(root: &Path, patterns: &[&str]) -> Vec<PackageLocation> {
    let mut packages = Vec::new();

    for pattern in patterns {
        // Simple glob expansion - check if pattern contains *
        if pattern.contains('*') {
            // Get the directory part before the *
            let parts: Vec<&str> = pattern.split('*').collect();
            if let Some(prefix) = parts.first() {
                let search_dir = root.join(prefix.trim_end_matches('/'));
                if let Ok(entries) = std::fs::read_dir(&search_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        if entry.path().is_dir() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            packages.push(PackageLocation {
                                name: name.clone(),
                                path: entry.path(),
                            });
                        }
                    }
                }
            }
        } else {
            // Direct path
            let path = root.join(pattern);
            if path.exists() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                packages.push(PackageLocation { name, path });
            }
        }
    }

    packages
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_non_monorepo_returns_none() {
        let dir = TempDir::new().unwrap();
        let result = detect_monorepo(dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_cargo_workspace_detected() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"pkg1\", \"pkg2\"]\n",
        )
        .unwrap();
        fs::create_dir(dir.path().join("pkg1")).unwrap();
        fs::create_dir(dir.path().join("pkg2")).unwrap();

        let result = detect_monorepo(dir.path()).unwrap();
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.tool, MonorepoTool::CargoWorkspace);
        assert_eq!(info.packages.len(), 2);
    }

    #[test]
    fn test_pnpm_workspace_detected() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("packages/app")).unwrap();

        let result = detect_monorepo(dir.path()).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().tool, MonorepoTool::PnpmWorkspaces);
    }

    #[test]
    fn test_nx_detected() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("nx.json"), "{}").unwrap();

        let result = detect_monorepo(dir.path()).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().tool, MonorepoTool::Nx);
    }

    #[test]
    fn test_cargo_workspace_with_glob() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"packages/*\"]\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("packages")).unwrap();
        fs::create_dir(dir.path().join("packages/foo")).unwrap();
        fs::create_dir(dir.path().join("packages/bar")).unwrap();

        let result = detect_monorepo(dir.path()).unwrap();
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.tool, MonorepoTool::CargoWorkspace);
        assert_eq!(info.packages.len(), 2);
    }

    #[test]
    fn test_npm_workspace_detected() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"workspaces": ["packages/*"]}"#,
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("packages/app")).unwrap();

        let result = detect_monorepo(dir.path()).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().tool, MonorepoTool::NpmWorkspaces);
    }

    #[test]
    fn test_turborepo_detected() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("turbo.json"), "{}").unwrap();

        let result = detect_monorepo(dir.path()).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().tool, MonorepoTool::Turborepo);
    }

    #[test]
    fn test_lerna_detected() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("lerna.json"), "{}").unwrap();

        let result = detect_monorepo(dir.path()).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().tool, MonorepoTool::Lerna);
    }
}
