use super::languages::detect_languages;
use crate::{Result, SniffError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

/// The type/category of a dependency.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    /// Normal runtime dependency
    Normal,
    /// Development-only dependency (testing, building docs, etc.)
    Dev,
    /// Build script dependency (Cargo's build-dependencies)
    Build,
    /// Optional dependency (enabled via features)
    Optional,
    /// Target-specific dependency (e.g., platform-specific)
    Target,
}

/// A single dependency entry with version information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DependencyEntry {
    /// The package/crate name
    pub name: String,
    /// The kind of dependency
    pub kind: DependencyKind,
    /// Version requirement as specified in the manifest (e.g., "^1.0", ">=2.0, <3.0")
    pub version_req: String,
    /// Actual resolved version from the lockfile (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_version: Option<String>,
    /// Target specification for target-specific dependencies (e.g., "cfg(target_os = \"macos\")")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Whether this dependency is optional (feature-gated)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    /// Features enabled for this dependency
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
}

/// Information about a detected repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    /// Whether this is a monorepo
    pub is_monorepo: bool,
    /// The tool managing the monorepo (if is_monorepo is true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monorepo_tool: Option<MonorepoTool>,
    /// Root directory of the repository
    pub root: PathBuf,
    /// Dependencies (for non-monorepo projects only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<DependencyEntry>>,
    /// Dev dependencies (for non-monorepo projects only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<Vec<DependencyEntry>>,
    /// Peer dependencies (for non-monorepo projects only, JS ecosystem)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_dependencies: Option<Vec<DependencyEntry>>,
    /// Optional dependencies (for non-monorepo projects only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_dependencies: Option<Vec<DependencyEntry>>,
    /// Package locations within the monorepo (only present when is_monorepo is true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packages: Option<Vec<PackageLocation>>,
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
    /// Dependencies for this package
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<DependencyEntry>>,
    /// Dev dependencies for this package
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<Vec<DependencyEntry>>,
    /// Peer dependencies for this package (JS ecosystem)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_dependencies: Option<Vec<DependencyEntry>>,
    /// Optional dependencies for this package
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_dependencies: Option<Vec<DependencyEntry>>,
}

/// Detect repository configuration in the given directory.
///
/// ## Examples
///
/// ```no_run
/// use std::path::Path;
/// use sniff_lib::filesystem::repo::detect_repo;
///
/// let root = Path::new("/path/to/project");
/// if let Some(info) = detect_repo(root).unwrap() {
///     if info.is_monorepo {
///         println!("Monorepo tool: {:?}", info.monorepo_tool);
///         if let Some(ref packages) = info.packages {
///             println!("Packages: {}", packages.len());
///         }
///     }
/// }
/// ```
///
/// ## Returns
///
/// - `Ok(Some(RepoInfo))` if a repository is detected
/// - `Ok(None)` if no repository configuration is found
/// - `Err(SniffError)` if there's an error reading files
pub fn detect_repo(root: &Path) -> Result<Option<RepoInfo>> {
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

fn detect_cargo_workspace(root: &Path) -> Result<Option<RepoInfo>> {
    let cargo_toml = root.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&cargo_toml)?;
    let parsed: toml::Value = toml::from_str(&content).map_err(|e| SniffError::SystemInfo {
        domain: "repo",
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

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: Some(MonorepoTool::CargoWorkspace),
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages: Some(packages),
    }))
}

fn detect_pnpm_workspace(root: &Path) -> Result<Option<RepoInfo>> {
    let pnpm_workspace = root.join("pnpm-workspace.yaml");
    if !pnpm_workspace.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&pnpm_workspace)?;
    let parsed: serde_yaml::Value =
        serde_yaml::from_str(&content).map_err(|e| SniffError::SystemInfo {
            domain: "repo",
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

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: Some(MonorepoTool::PnpmWorkspaces),
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages: Some(package_locations),
    }))
}

fn detect_npm_workspace(root: &Path) -> Result<Option<RepoInfo>> {
    let package_json = root.join("package.json");
    if !package_json.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&package_json)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| SniffError::SystemInfo {
            domain: "repo",
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

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: Some(MonorepoTool::NpmWorkspaces),
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages: Some(packages),
    }))
}

fn detect_nx(root: &Path) -> Result<Option<RepoInfo>> {
    let nx_json = root.join("nx.json");
    if !nx_json.exists() {
        return Ok(None);
    }

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: Some(MonorepoTool::Nx),
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages: Some(vec![]), // Nx projects would need deeper parsing
    }))
}

fn detect_turborepo(root: &Path) -> Result<Option<RepoInfo>> {
    let turbo_json = root.join("turbo.json");
    if !turbo_json.exists() {
        return Ok(None);
    }

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: Some(MonorepoTool::Turborepo),
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages: Some(vec![]), // Would need to parse package.json workspaces
    }))
}

fn detect_lerna(root: &Path) -> Result<Option<RepoInfo>> {
    let lerna_json = root.join("lerna.json");
    if !lerna_json.exists() {
        return Ok(None);
    }

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: Some(MonorepoTool::Lerna),
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages: Some(vec![]),
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
                                dependencies: None,
                                dev_dependencies: None,
                                peer_dependencies: None,
                                optional_dependencies: None,
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
                    dependencies: None,
                    dev_dependencies: None,
                    peer_dependencies: None,
                    optional_dependencies: None,
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
        let result = detect_repo(dir.path()).unwrap();
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

        let result = detect_repo(dir.path()).unwrap();
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.is_monorepo);
        assert_eq!(info.monorepo_tool, Some(MonorepoTool::CargoWorkspace));
        assert_eq!(info.packages.as_ref().unwrap().len(), 2);
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

        let result = detect_repo(dir.path()).unwrap();
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.is_monorepo);
        assert_eq!(info.monorepo_tool, Some(MonorepoTool::PnpmWorkspaces));
    }

    #[test]
    fn test_nx_detected() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("nx.json"), "{}").unwrap();

        let result = detect_repo(dir.path()).unwrap();
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.is_monorepo);
        assert_eq!(info.monorepo_tool, Some(MonorepoTool::Nx));
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

        let result = detect_repo(dir.path()).unwrap();
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.is_monorepo);
        assert_eq!(info.monorepo_tool, Some(MonorepoTool::CargoWorkspace));
        assert_eq!(info.packages.as_ref().unwrap().len(), 2);
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

        let result = detect_repo(dir.path()).unwrap();
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.is_monorepo);
        assert_eq!(info.monorepo_tool, Some(MonorepoTool::NpmWorkspaces));
    }

    #[test]
    fn test_turborepo_detected() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("turbo.json"), "{}").unwrap();

        let result = detect_repo(dir.path()).unwrap();
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.is_monorepo);
        assert_eq!(info.monorepo_tool, Some(MonorepoTool::Turborepo));
    }

    #[test]
    fn test_lerna_detected() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("lerna.json"), "{}").unwrap();

        let result = detect_repo(dir.path()).unwrap();
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.is_monorepo);
        assert_eq!(info.monorepo_tool, Some(MonorepoTool::Lerna));
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
        fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"test\"",
        )
        .unwrap();

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

        let result = detect_repo(dir.path()).unwrap();
        assert!(result.is_some());
        let info = result.unwrap();
        let packages = info.packages.unwrap();
        assert_eq!(packages.len(), 2);

        // Find the rust package
        let rust_package = packages
            .iter()
            .find(|p| p.name == "rust-pkg")
            .expect("rust-pkg should be found");
        assert_eq!(rust_package.primary_language, Some("Rust".to_string()));
        assert!(
            rust_package
                .detected_managers
                .contains(&"cargo".to_string())
        );

        // Find the node package
        let node_package = packages
            .iter()
            .find(|p| p.name == "node-pkg")
            .expect("node-pkg should be found");
        assert_eq!(
            node_package.primary_language,
            Some("JavaScript".to_string())
        );
        assert!(node_package.detected_managers.contains(&"npm".to_string()));
    }

    #[test]
    fn test_repo_info_has_optional_dependency_fields() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("nx.json"), "{}").unwrap();

        let result = detect_repo(dir.path()).unwrap();
        let info = result.unwrap();

        // Non-monorepo dependency fields should be None for monorepos
        assert!(info.dependencies.is_none());
        assert!(info.dev_dependencies.is_none());
        assert!(info.peer_dependencies.is_none());
        assert!(info.optional_dependencies.is_none());
        // Packages should be present for monorepos
        assert!(info.packages.is_some());
    }

    #[test]
    fn test_package_location_has_optional_dependency_fields() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"pkg\"]\n",
        )
        .unwrap();
        fs::create_dir(dir.path().join("pkg")).unwrap();

        let result = detect_repo(dir.path()).unwrap();
        let info = result.unwrap();
        let packages = info.packages.unwrap();
        let pkg = &packages[0];

        // Dependency fields should be None (not populated yet - Phase 5 work)
        assert!(pkg.dependencies.is_none());
        assert!(pkg.dev_dependencies.is_none());
        assert!(pkg.peer_dependencies.is_none());
        assert!(pkg.optional_dependencies.is_none());
    }
}
