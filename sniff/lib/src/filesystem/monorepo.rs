use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::{Result, SniffError};
use super::languages::detect_languages;

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
    /// The primary programming language detected in this package
    pub primary_language: Option<String>,
    /// All programming languages detected in this package
    pub languages: Vec<String>,
    /// Detected dependency managers in this package (e.g., "cargo", "npm", "pnpm")
    pub detected_managers: Vec<String>,
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

/// Creates a namespaced package name from a path relative to the repo root.
///
/// For example, `/repo/sniff/lib` becomes `sniff/lib` to avoid name collisions
/// when multiple packages have the same directory name (like "lib" or "cli").
fn make_namespaced_name(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .and_then(|rel| rel.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        })
}

/// Detects programming languages in a package directory.
///
/// Uses the `detect_languages` function scoped to the package directory
/// to identify the primary language and all languages present.
///
/// ## Returns
///
/// A tuple of (primary_language, all_languages) where:
/// - `primary_language` is the most common programming language (excluding markup/config)
/// - `all_languages` is a list of all detected programming languages sorted by frequency
fn detect_package_languages(path: &Path) -> (Option<String>, Vec<String>) {
    match detect_languages(path) {
        Ok(breakdown) => {
            let languages: Vec<String> = breakdown
                .languages
                .iter()
                .map(|s| s.language.clone())
                .collect();
            (breakdown.primary, languages)
        }
        Err(_) => (None, Vec::new()),
    }
}

/// Detects dependency managers present in a package directory.
///
/// Checks for the presence of various package manager configuration files
/// to determine which dependency managers are used in the package.
///
/// ## Detected Managers
///
/// - `cargo` - Rust (Cargo.toml)
/// - `npm` - Node.js with npm (package.json without pnpm-lock.yaml or yarn.lock)
/// - `pnpm` - Node.js with pnpm (pnpm-lock.yaml)
/// - `yarn` - Node.js with Yarn (yarn.lock)
/// - `pip` - Python (requirements.txt or pyproject.toml)
/// - `go` - Go (go.mod)
fn detect_package_managers(path: &Path) -> Vec<String> {
    let mut managers = Vec::new();

    // Rust: Cargo.toml
    if path.join("Cargo.toml").exists() {
        managers.push("cargo".to_string());
    }

    // Node.js package managers
    let has_package_json = path.join("package.json").exists();
    let has_pnpm_lock = path.join("pnpm-lock.yaml").exists();
    let has_yarn_lock = path.join("yarn.lock").exists();

    if has_pnpm_lock {
        managers.push("pnpm".to_string());
    } else if has_yarn_lock {
        managers.push("yarn".to_string());
    } else if has_package_json {
        managers.push("npm".to_string());
    }

    // Python: requirements.txt or pyproject.toml
    if path.join("requirements.txt").exists() || path.join("pyproject.toml").exists() {
        managers.push("pip".to_string());
    }

    // Go: go.mod
    if path.join("go.mod").exists() {
        managers.push("go".to_string());
    }

    managers
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
                            let path = entry.path();
                            let name = make_namespaced_name(&path, root);
                            let (primary_language, languages) = detect_package_languages(&path);
                            let detected_managers = detect_package_managers(&path);
                            packages.push(PackageLocation {
                                name,
                                path,
                                primary_language,
                                languages,
                                detected_managers,
                            });
                        }
                    }
                }
            }
        } else {
            // Direct path
            let path = root.join(pattern);
            if path.exists() {
                let name = make_namespaced_name(&path, root);
                let (primary_language, languages) = detect_package_languages(&path);
                let detected_managers = detect_package_managers(&path);
                packages.push(PackageLocation {
                    name,
                    path,
                    primary_language,
                    languages,
                    detected_managers,
                });
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

    #[test]
    fn test_detect_package_managers_cargo() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        let managers = detect_package_managers(dir.path());
        assert_eq!(managers, vec!["cargo"]);
    }

    #[test]
    fn test_detect_package_managers_npm() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();

        let managers = detect_package_managers(dir.path());
        assert_eq!(managers, vec!["npm"]);
    }

    #[test]
    fn test_detect_package_managers_pnpm() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();

        let managers = detect_package_managers(dir.path());
        // pnpm takes precedence over npm when pnpm-lock.yaml exists
        assert_eq!(managers, vec!["pnpm"]);
    }

    #[test]
    fn test_detect_package_managers_yarn() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("yarn.lock"), "").unwrap();

        let managers = detect_package_managers(dir.path());
        // yarn takes precedence over npm when yarn.lock exists
        assert_eq!(managers, vec!["yarn"]);
    }

    #[test]
    fn test_detect_package_managers_pip_requirements() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("requirements.txt"), "requests==2.31.0").unwrap();

        let managers = detect_package_managers(dir.path());
        assert_eq!(managers, vec!["pip"]);
    }

    #[test]
    fn test_detect_package_managers_pip_pyproject() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("pyproject.toml"), "[project]\nname = \"test\"").unwrap();

        let managers = detect_package_managers(dir.path());
        assert_eq!(managers, vec!["pip"]);
    }

    #[test]
    fn test_detect_package_managers_go() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("go.mod"), "module example.com/test").unwrap();

        let managers = detect_package_managers(dir.path());
        assert_eq!(managers, vec!["go"]);
    }

    #[test]
    fn test_detect_package_managers_multiple() {
        let dir = TempDir::new().unwrap();
        // A package with both Rust and Node.js
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();

        let managers = detect_package_managers(dir.path());
        assert_eq!(managers, vec!["cargo", "npm"]);
    }

    #[test]
    fn test_detect_package_managers_empty() {
        let dir = TempDir::new().unwrap();

        let managers = detect_package_managers(dir.path());
        assert!(managers.is_empty());
    }

    #[test]
    fn test_detect_package_languages_rust() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("lib.rs"), "pub fn foo() {}").unwrap();

        let (primary, languages) = detect_package_languages(dir.path());
        assert_eq!(primary, Some("Rust".to_string()));
        assert!(languages.contains(&"Rust".to_string()));
    }

    #[test]
    fn test_detect_package_languages_javascript() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("index.js"), "console.log('hello')").unwrap();

        let (primary, languages) = detect_package_languages(dir.path());
        assert_eq!(primary, Some("JavaScript".to_string()));
        assert!(languages.contains(&"JavaScript".to_string()));
    }

    #[test]
    fn test_detect_package_languages_empty() {
        let dir = TempDir::new().unwrap();

        let (primary, languages) = detect_package_languages(dir.path());
        assert!(primary.is_none());
        assert!(languages.is_empty());
    }

    #[test]
    fn test_cargo_workspace_with_languages_and_managers() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"rust-pkg\", \"node-pkg\"]\n",
        )
        .unwrap();

        // Create a Rust package
        let rust_pkg = dir.path().join("rust-pkg");
        fs::create_dir(&rust_pkg).unwrap();
        fs::write(rust_pkg.join("Cargo.toml"), "[package]").unwrap();
        fs::write(rust_pkg.join("main.rs"), "fn main() {}").unwrap();

        // Create a Node.js package
        let node_pkg = dir.path().join("node-pkg");
        fs::create_dir(&node_pkg).unwrap();
        fs::write(node_pkg.join("package.json"), "{}").unwrap();
        fs::write(node_pkg.join("index.js"), "console.log('hi')").unwrap();

        let result = detect_monorepo(dir.path()).unwrap();
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.packages.len(), 2);

        // Find the rust package
        let rust_package = info
            .packages
            .iter()
            .find(|p| p.name == "rust-pkg")
            .expect("rust-pkg should be found");
        assert_eq!(rust_package.primary_language, Some("Rust".to_string()));
        assert!(rust_package.detected_managers.contains(&"cargo".to_string()));

        // Find the node package
        let node_package = info
            .packages
            .iter()
            .find(|p| p.name == "node-pkg")
            .expect("node-pkg should be found");
        assert_eq!(node_package.primary_language, Some("JavaScript".to_string()));
        assert!(node_package.detected_managers.contains(&"npm".to_string()));
    }
}
