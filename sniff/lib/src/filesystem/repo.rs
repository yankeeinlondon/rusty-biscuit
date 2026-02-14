use super::languages::detect_languages;
use crate::{Result, SniffError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    /// Normal runtime dependency
    #[default]
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
    /// The kind of dependency (internal use only, hidden from JSON)
    #[serde(skip)]
    pub kind: DependencyKind,
    /// Version requirement as specified in the manifest (e.g., "^1.0", ">=2.0, <3.0")
    #[serde(alias = "version_req")]
    pub targeted_version: String,
    /// Actual resolved version from the lockfile (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_version: Option<String>,
    /// The package manager used for this dependency (e.g., "cargo", "npm")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<String>,
    /// Latest version available from the registry (only populated with --deep flag)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    /// Target specification for target-specific dependencies (e.g., "cfg(target_os = \"macos\")")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Whether this dependency is optional (feature-gated)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    /// Features enabled for this dependency
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    /// Whether this dependency can be updated (latest != actual)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_updatable: bool,
    /// Whether the available update is a major version bump.
    ///
    /// Only set when `is_updatable` is true and both versions follow
    /// semantic versioning (`major.minor.patch`). Considered major when:
    /// - The major version is 0 and a newer minor version exists, or
    /// - A newer major version exists.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub has_major_update: bool,
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
    /// Packages within the monorepo (only present when is_monorepo is true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packages: Option<Vec<Package>>,
}

/// A package within a monorepo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// Absolute path to the package
    pub path: PathBuf,
    /// Path relative to the repo root (e.g., "sniff/lib")
    pub relative: String,
    /// Directory path between repo root and package root (e.g., "sniff" for "sniff/lib",
    /// "apps/browser" for "apps/browser/my_package", "root" for top-level packages)
    pub package_area: String,
    /// Native package name from manifest (Cargo.toml [package].name or package.json name)
    pub name: String,
    /// The primary programming language detected in this package
    pub primary_language: Option<String>,
    /// All programming languages detected in this package
    pub languages: Vec<String>,
    /// Configuration files found in the package root (JSON, TOML, YAML, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub configuration: Vec<PathBuf>,
    /// Documentation files found in the package root (MD, TXT, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documentation: Vec<PathBuf>,
    /// EditorConfig file path (if present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_config: Option<PathBuf>,
    /// Command runner files (justfile, Makefile, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command_runner: Vec<PathBuf>,
    /// Detected package managers (e.g., "cargo", "npm", "pnpm")
    pub package_managers: Vec<String>,
    /// Package version from manifest (Cargo.toml [package].version or package.json version)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Feature flags defined by this package (e.g., Cargo [features])
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    /// Names of workspace-internal packages this package depends on
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    /// Names of workspace-internal packages that depend on this package
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub used_by: Vec<String>,
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
    /// Whether any dependency can be updated (deep-mode only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_updatable: Option<bool>,
    /// Whether any dependency has a major version update available (deep-mode only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_major_update: Option<bool>,
}

/// Deprecated type alias for backward compatibility.
#[deprecated(note = "Use `Package` instead")]
pub type PackageLocation = Package;

/// Categorized files found in a package directory.
struct PackageFiles {
    configuration: Vec<PathBuf>,
    documentation: Vec<PathBuf>,
    editor_config: Option<PathBuf>,
    command_runner: Vec<PathBuf>,
}

/// Resolved versions from Cargo.lock.
pub(crate) struct CargoLockVersions {
    versions: HashMap<String, Vec<String>>,
}

impl CargoLockVersions {
    /// Parse a Cargo.lock file and extract package versions.
    pub fn parse(lock_path: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(lock_path).ok()?;
        let parsed: toml::Value = toml::from_str(&content).ok()?;

        let mut versions: HashMap<String, Vec<String>> = HashMap::new();

        if let Some(packages) = parsed.get("package").and_then(|p| p.as_array()) {
            for pkg in packages {
                if let (Some(name), Some(version)) = (
                    pkg.get("name").and_then(|n| n.as_str()),
                    pkg.get("version").and_then(|v| v.as_str()),
                ) {
                    versions.entry(name.to_string()).or_default().push(version.to_string());
                }
            }
        }

        Some(Self {
            versions,
        })
    }

    /// Resolve the version for a dependency name.
    ///
    /// Returns the first resolved version if available.
    pub fn resolve(&self, name: &str) -> Option<String> {
        self.versions.get(name).and_then(|v| v.first()).cloned()
    }
}

/// Detect repository configuration in the given directory.
///
/// ## Examples
///
/// ```no_run
/// use std::path::Path;
/// use sniff::filesystem::repo::detect_repo;
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

    // Parse Cargo.lock once for version resolution
    let lock_versions = CargoLockVersions::parse(&root.join("Cargo.lock"));

    // Expand globs and collect packages with dependencies
    let mut packages = expand_glob_patterns_with_deps(
        root,
        &members,
        MonorepoTool::CargoWorkspace,
        &lock_versions,
    );

    // Resolve internal dependency graph
    resolve_internal_deps(&mut packages);

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

/// Parses Cargo.toml dependencies into DependencyEntry structs.
fn parse_cargo_dependencies(
    toml_path: &Path,
    lock_versions: &Option<CargoLockVersions>,
) -> Option<(Vec<DependencyEntry>, Vec<DependencyEntry>, Vec<DependencyEntry>)> {
    let content = std::fs::read_to_string(toml_path).ok()?;
    let parsed: toml::Value = toml::from_str(&content).ok()?;

    let normal_deps =
        parse_cargo_dep_section(&parsed, "dependencies", DependencyKind::Normal, lock_versions);
    let dev_deps =
        parse_cargo_dep_section(&parsed, "dev-dependencies", DependencyKind::Dev, lock_versions);
    let build_deps = parse_cargo_dep_section(
        &parsed,
        "build-dependencies",
        DependencyKind::Build,
        lock_versions,
    );

    Some((normal_deps, dev_deps, build_deps))
}

/// Parses a single dependencies section from Cargo.toml.
fn parse_cargo_dep_section(
    parsed: &toml::Value,
    section: &str,
    kind: DependencyKind,
    lock_versions: &Option<CargoLockVersions>,
) -> Vec<DependencyEntry> {
    let Some(deps) = parsed.get(section).and_then(|d| d.as_table()) else {
        return Vec::new();
    };

    deps.iter()
        .map(|(name, value)| {
            let (version_req, features, optional) = match value {
                toml::Value::String(v) => (v.clone(), Vec::new(), false),
                toml::Value::Table(t) => {
                    let version =
                        t.get("version").and_then(|v| v.as_str()).unwrap_or("*").to_string();
                    let features = t
                        .get("features")
                        .and_then(|f| f.as_array())
                        .map(|arr| {
                            arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
                        })
                        .unwrap_or_default();
                    let optional = t.get("optional").and_then(|o| o.as_bool()).unwrap_or(false);
                    (version, features, optional)
                }
                _ => ("*".to_string(), Vec::new(), false),
            };

            let actual_version = lock_versions.as_ref().and_then(|lv| lv.resolve(name));

            DependencyEntry {
                name: name.clone(),
                kind,
                targeted_version: version_req,
                actual_version,
                package_manager: Some("cargo".to_string()),
                latest_version: None,
                target: None,
                optional,
                features,
                is_updatable: false,
                has_major_update: false,
            }
        })
        .collect()
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

    let mut package_locations = expand_glob_patterns(root, &packages, MonorepoTool::PnpmWorkspaces);
    resolve_internal_deps(&mut package_locations);

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

    let mut packages = expand_glob_patterns(root, &workspaces, MonorepoTool::NpmWorkspaces);
    resolve_internal_deps(&mut packages);

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

// ============================================================================
// Package name reading helpers
// ============================================================================

/// Reads the package name from a Cargo.toml file.
fn read_cargo_package_name(cargo_toml: &Path) -> Option<String> {
    let content = std::fs::read_to_string(cargo_toml).ok()?;
    let parsed: toml::Value = toml::from_str(&content).ok()?;
    parsed.get("package").and_then(|p| p.get("name")).and_then(|n| n.as_str()).map(String::from)
}

/// Reads the package version from a Cargo.toml file.
fn read_cargo_package_version(cargo_toml: &Path) -> Option<String> {
    let content = std::fs::read_to_string(cargo_toml).ok()?;
    let parsed: toml::Value = toml::from_str(&content).ok()?;
    parsed.get("package").and_then(|p| p.get("version")).and_then(|v| v.as_str()).map(String::from)
}

/// Reads the package name from a package.json file.
fn read_npm_package_name(package_json: &Path) -> Option<String> {
    let content = std::fs::read_to_string(package_json).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    parsed.get("name").and_then(|n| n.as_str()).map(String::from)
}

/// Reads the package version from a package.json file.
fn read_npm_package_version(package_json: &Path) -> Option<String> {
    let content = std::fs::read_to_string(package_json).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    parsed.get("version").and_then(|v| v.as_str()).map(String::from)
}

/// Reads the feature flag names from a Cargo.toml `[features]` section.
fn read_cargo_features(cargo_toml: &Path) -> Vec<String> {
    let content = match std::fs::read_to_string(cargo_toml) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let parsed: toml::Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let Some(features) = parsed.get("features").and_then(|f| f.as_table()) else {
        return Vec::new();
    };
    let mut names: Vec<String> = features.keys().cloned().collect();
    names.sort();
    names
}

// ============================================================================
// File categorization
// ============================================================================

/// Categorizes files in a package root directory into configuration, documentation,
/// editor config, and command runner files.
///
/// All paths are stored relative to the repo root for portability.
/// Only performs a shallow scan of the package root directory (no recursion).
fn detect_package_files(path: &Path, repo_root: &Path) -> PackageFiles {
    let config_extensions = [
        "json", "toml", "yaml", "yml", "ini", "cfg", "conf",
    ];
    let doc_extensions = [
        "md", "txt", "rst", "adoc",
    ];
    let command_runners = [
        "justfile",
        "Justfile",
        "Makefile",
        "makefile",
        "Taskfile.yml",
        "Rakefile",
    ];

    let mut configuration = Vec::new();
    let mut documentation = Vec::new();
    let mut editor_config = None;
    let mut command_runner = Vec::new();

    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => {
            return PackageFiles {
                configuration,
                documentation,
                editor_config,
                command_runner,
            };
        }
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        if !entry_path.is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();

        // Convert to relative path from repo root
        let rel_path = entry_path.strip_prefix(repo_root).unwrap_or(&entry_path).to_path_buf();

        // Check for .editorconfig
        if file_name == ".editorconfig" {
            editor_config = Some(rel_path);
            continue;
        }

        // Check for command runners
        if command_runners.contains(&file_name.as_str()) {
            command_runner.push(rel_path);
            continue;
        }

        // Check extension-based categories
        if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            if config_extensions.contains(&ext_lower.as_str()) {
                configuration.push(rel_path);
            } else if doc_extensions.contains(&ext_lower.as_str()) {
                documentation.push(rel_path);
            }
        }
    }

    // Sort for deterministic output
    configuration.sort();
    documentation.sort();
    command_runner.sort();

    PackageFiles {
        configuration,
        documentation,
        editor_config,
        command_runner,
    }
}

// ============================================================================
// Package name resolution
// ============================================================================

/// Determines the native package name based on monorepo tool type.
fn resolve_package_name(path: &Path, root: &Path, tool: MonorepoTool) -> String {
    match tool {
        MonorepoTool::CargoWorkspace => {
            let cargo_toml = path.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Some(name) = read_cargo_package_name(&cargo_toml) {
                    return name;
                }
            }
        }
        MonorepoTool::NpmWorkspaces
        | MonorepoTool::PnpmWorkspaces
        | MonorepoTool::YarnWorkspaces => {
            let package_json = path.join("package.json");
            if package_json.exists() {
                if let Some(name) = read_npm_package_name(&package_json) {
                    return name;
                }
            }
        }
        _ => {}
    }
    // Fallback: relative path from root
    make_relative_path(path, root)
}

/// Determines the package version based on monorepo tool type.
fn resolve_package_version(path: &Path, tool: MonorepoTool) -> Option<String> {
    match tool {
        MonorepoTool::CargoWorkspace => {
            let cargo_toml = path.join("Cargo.toml");
            if cargo_toml.exists() {
                return read_cargo_package_version(&cargo_toml);
            }
        }
        MonorepoTool::NpmWorkspaces
        | MonorepoTool::PnpmWorkspaces
        | MonorepoTool::YarnWorkspaces => {
            let package_json = path.join("package.json");
            if package_json.exists() {
                return read_npm_package_version(&package_json);
            }
        }
        _ => {}
    }
    None
}

// ============================================================================
// Internal dependency graph
// ============================================================================

/// Resolves internal dependencies between packages in a workspace.
///
/// Two-pass algorithm:
/// 1. Scan each package's dependency lists for names matching other package names → `depends_on`
/// 2. Invert the relationship → `used_by`
fn resolve_internal_deps(packages: &mut [Package]) {
    // Collect all package names
    let package_names: Vec<String> = packages.iter().map(|p| p.name.clone()).collect();

    // Pass 1: populate depends_on
    for pkg in packages.iter_mut() {
        let mut internal_deps = Vec::new();
        for dep_list in [
            pkg.dependencies.as_ref(),
            pkg.dev_dependencies.as_ref(),
            pkg.optional_dependencies.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            for dep in dep_list {
                if package_names.contains(&dep.name) && dep.name != pkg.name {
                    if !internal_deps.contains(&dep.name) {
                        internal_deps.push(dep.name.clone());
                    }
                }
            }
        }
        internal_deps.sort();
        pkg.depends_on = internal_deps;
    }

    // Pass 2: invert to populate used_by
    // Collect depends_on relationships first to avoid borrow issues
    let dep_pairs: Vec<(String, Vec<String>)> =
        packages.iter().map(|p| (p.name.clone(), p.depends_on.clone())).collect();

    for pkg in packages.iter_mut() {
        let mut used_by = Vec::new();
        for (other_name, other_deps) in &dep_pairs {
            if other_deps.contains(&pkg.name) {
                used_by.push(other_name.clone());
            }
        }
        used_by.sort();
        pkg.used_by = used_by;
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Creates a relative path string from root.
fn make_relative_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root).ok().and_then(|rel| rel.to_str()).map(|s| s.to_string()).unwrap_or_else(
        || path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
    )
}

/// Derives the package area from a relative path.
///
/// The area is the directory path between the repo root and the package directory.
/// Returns "root" when the package sits directly under the repo root.
fn make_package_area(relative: &str) -> String {
    let path = Path::new(relative);
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_string_lossy().to_string(),
        _ => "root".to_string(),
    }
}

/// Detects programming languages in a package directory.
fn detect_package_languages(path: &Path) -> (Option<String>, Vec<String>) {
    match detect_languages(path) {
        Ok(breakdown) => {
            let languages: Vec<String> =
                breakdown.languages.iter().map(|s| s.language.clone()).collect();
            (breakdown.primary, languages)
        }
        Err(_) => (None, Vec::new()),
    }
}

/// Detects dependency managers present in a package directory.
fn detect_package_managers(path: &Path) -> Vec<String> {
    let mut managers = Vec::new();

    if path.join("Cargo.toml").exists() {
        managers.push("cargo".to_string());
    }

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

    if path.join("requirements.txt").exists() || path.join("pyproject.toml").exists() {
        managers.push("pip".to_string());
    }

    if path.join("go.mod").exists() {
        managers.push("go".to_string());
    }

    managers
}

fn expand_glob_patterns(root: &Path, patterns: &[&str], tool: MonorepoTool) -> Vec<Package> {
    let mut packages = Vec::new();

    for pattern in patterns {
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if let Some(prefix) = parts.first() {
                let search_dir = root.join(prefix.trim_end_matches('/'));
                if let Ok(entries) = std::fs::read_dir(&search_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        if entry.path().is_dir() {
                            let path = entry.path();
                            packages.push(create_package(&path, root, tool, &None));
                        }
                    }
                }
            }
        } else {
            let path = root.join(pattern);
            if path.exists() {
                packages.push(create_package(&path, root, tool, &None));
            }
        }
    }

    packages
}

/// Expand glob patterns and parse dependencies for Cargo workspaces.
fn expand_glob_patterns_with_deps(
    root: &Path,
    patterns: &[&str],
    tool: MonorepoTool,
    lock_versions: &Option<CargoLockVersions>,
) -> Vec<Package> {
    let mut packages = Vec::new();

    for pattern in patterns {
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if let Some(prefix) = parts.first() {
                let search_dir = root.join(prefix.trim_end_matches('/'));
                if let Ok(entries) = std::fs::read_dir(&search_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        if entry.path().is_dir() {
                            let path = entry.path();
                            packages.push(create_package(&path, root, tool, lock_versions));
                        }
                    }
                }
            }
        } else {
            let path = root.join(pattern);
            if path.exists() {
                packages.push(create_package(&path, root, tool, lock_versions));
            }
        }
    }

    packages
}

/// Creates a Package with all metadata and parsed dependencies.
fn create_package(
    path: &Path,
    root: &Path,
    tool: MonorepoTool,
    lock_versions: &Option<CargoLockVersions>,
) -> Package {
    let relative = make_relative_path(path, root);
    let package_area = make_package_area(&relative);
    let name = resolve_package_name(path, root, tool);
    let version = resolve_package_version(path, tool);
    let (primary_language, languages) = detect_package_languages(path);
    let package_managers = detect_package_managers(path);
    let files = detect_package_files(path, root);

    // Read feature flags (Cargo only for now)
    let cargo_toml = path.join("Cargo.toml");
    let features = if cargo_toml.exists() {
        read_cargo_features(&cargo_toml)
    } else {
        Vec::new()
    };

    // Try to parse Cargo.toml dependencies (for Cargo workspaces)
    let (dependencies, dev_dependencies, optional_dependencies) = if cargo_toml.exists() {
        if let Some((normal, dev, build)) = parse_cargo_dependencies(&cargo_toml, lock_versions) {
            let mut all_deps = normal;
            all_deps.extend(build);

            let (optional, regular): (Vec<_>, Vec<_>) =
                all_deps.into_iter().partition(|d| d.optional);

            let deps = if regular.is_empty() {
                None
            } else {
                Some(regular)
            };
            let dev_deps = if dev.is_empty() {
                None
            } else {
                Some(dev)
            };
            let opt_deps = if optional.is_empty() {
                None
            } else {
                Some(optional)
            };

            (deps, dev_deps, opt_deps)
        } else {
            (None, None, None)
        }
    } else {
        (None, None, None)
    };

    Package {
        path: path.to_path_buf(),
        relative,
        package_area,
        name,
        primary_language,
        languages,
        configuration: files.configuration,
        documentation: files.documentation,
        editor_config: files.editor_config,
        command_runner: files.command_runner,
        package_managers,
        version,
        features,
        depends_on: Vec::new(),
        used_by: Vec::new(),
        dependencies,
        dev_dependencies,
        peer_dependencies: None,
        optional_dependencies,
        is_updatable: None,
        has_major_update: None,
    }
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
        fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = [\"pkg1\", \"pkg2\"]\n")
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
        fs::write(dir.path().join("pnpm-workspace.yaml"), "packages:\n  - 'packages/*'\n").unwrap();
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
        fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = [\"packages/*\"]\n")
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
        fs::write(dir.path().join("package.json"), r#"{"workspaces": ["packages/*"]}"#).unwrap();
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
        assert_eq!(managers, vec!["pnpm"]);
    }

    #[test]
    fn test_detect_package_managers_yarn() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("yarn.lock"), "").unwrap();

        let managers = detect_package_managers(dir.path());
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
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();

        let managers = detect_package_managers(dir.path());
        assert_eq!(
            managers,
            vec![
                "cargo", "npm"
            ]
        );
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
        fs::write(
            rust_pkg.join("Cargo.toml"),
            "[package]\nname = \"rust-pkg\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
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

        // Find the rust package (by native name from Cargo.toml)
        let rust_package =
            packages.iter().find(|p| p.name == "rust-pkg").expect("rust-pkg should be found");
        assert_eq!(rust_package.primary_language, Some("Rust".to_string()));
        assert!(rust_package.package_managers.contains(&"cargo".to_string()));
        assert_eq!(rust_package.version, Some("0.1.0".to_string()));

        // Find the node package (falls back to relative path since no name in package.json)
        let node_package =
            packages.iter().find(|p| p.name == "node-pkg").expect("node-pkg should be found");
        assert_eq!(node_package.primary_language, Some("JavaScript".to_string()));
        assert!(node_package.package_managers.contains(&"npm".to_string()));
    }

    #[test]
    fn test_repo_info_has_optional_dependency_fields() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("nx.json"), "{}").unwrap();

        let result = detect_repo(dir.path()).unwrap();
        let info = result.unwrap();

        assert!(info.dependencies.is_none());
        assert!(info.dev_dependencies.is_none());
        assert!(info.peer_dependencies.is_none());
        assert!(info.optional_dependencies.is_none());
        assert!(info.packages.is_some());
    }

    #[test]
    fn test_package_has_optional_dependency_fields() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = [\"pkg\"]\n").unwrap();
        fs::create_dir(dir.path().join("pkg")).unwrap();

        let result = detect_repo(dir.path()).unwrap();
        let info = result.unwrap();
        let packages = info.packages.unwrap();
        let pkg = &packages[0];

        assert!(pkg.dependencies.is_none());
        assert!(pkg.dev_dependencies.is_none());
        assert!(pkg.peer_dependencies.is_none());
        assert!(pkg.optional_dependencies.is_none());
    }

    #[test]
    fn test_read_cargo_package_name() {
        let dir = TempDir::new().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]\nname = \"my-crate\"\nversion = \"1.0.0\"\n").unwrap();

        assert_eq!(read_cargo_package_name(&cargo_toml), Some("my-crate".to_string()));
    }

    #[test]
    fn test_read_cargo_package_version() {
        let dir = TempDir::new().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]\nname = \"my-crate\"\nversion = \"2.3.4\"\n").unwrap();

        assert_eq!(read_cargo_package_version(&cargo_toml), Some("2.3.4".to_string()));
    }

    #[test]
    fn test_read_npm_package_name() {
        let dir = TempDir::new().unwrap();
        let pkg_json = dir.path().join("package.json");
        fs::write(&pkg_json, r#"{"name": "@scope/my-pkg", "version": "1.0.0"}"#).unwrap();

        assert_eq!(read_npm_package_name(&pkg_json), Some("@scope/my-pkg".to_string()));
    }

    #[test]
    fn test_read_npm_package_version() {
        let dir = TempDir::new().unwrap();
        let pkg_json = dir.path().join("package.json");
        fs::write(&pkg_json, r#"{"name": "pkg", "version": "3.2.1"}"#).unwrap();

        assert_eq!(read_npm_package_version(&pkg_json), Some("3.2.1".to_string()));
    }

    #[test]
    fn test_read_cargo_features() {
        let dir = TempDir::new().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            "[package]\nname = \"x\"\n\n[features]\ndefault = [\"net\"]\nnet = [\"dep:reqwest\"]\nfull = [\"net\"]\n",
        )
        .unwrap();

        let features = read_cargo_features(&cargo_toml);
        assert_eq!(
            features,
            vec![
                "default", "full", "net"
            ]
        );
    }

    #[test]
    fn test_read_cargo_features_empty() {
        let dir = TempDir::new().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]\nname = \"x\"\n").unwrap();

        let features = read_cargo_features(&cargo_toml);
        assert!(features.is_empty());
    }

    #[test]
    fn test_package_features_populated() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = [\"pkg\"]\n").unwrap();
        let pkg_dir = dir.path().join("pkg");
        fs::create_dir(&pkg_dir).unwrap();
        fs::write(
            pkg_dir.join("Cargo.toml"),
            "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\n\n[features]\ndefault = []\nfast = []\n",
        )
        .unwrap();

        let result = detect_repo(dir.path()).unwrap().unwrap();
        let packages = result.packages.unwrap();
        let pkg = &packages[0];
        assert_eq!(
            pkg.features,
            vec![
                "default", "fast"
            ]
        );
    }

    #[test]
    fn test_detect_package_files() {
        let dir = TempDir::new().unwrap();
        // Simulate repo_root/pkg/ structure
        let pkg_dir = dir.path().join("pkg");
        fs::create_dir(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("config.toml"), "").unwrap();
        fs::write(pkg_dir.join("settings.json"), "{}").unwrap();
        fs::write(pkg_dir.join("README.md"), "# Hello").unwrap();
        fs::write(pkg_dir.join("CHANGELOG.txt"), "").unwrap();
        fs::write(pkg_dir.join(".editorconfig"), "root = true").unwrap();
        fs::write(pkg_dir.join("justfile"), "build:").unwrap();
        fs::write(pkg_dir.join("main.rs"), "fn main() {}").unwrap();

        let files = detect_package_files(&pkg_dir, dir.path());
        assert_eq!(files.configuration.len(), 2);
        assert_eq!(files.documentation.len(), 2);
        assert!(files.editor_config.is_some());
        assert_eq!(files.command_runner.len(), 1);

        // Verify paths are relative to repo root
        for p in &files.configuration {
            assert!(p.starts_with("pkg/"), "config path should be relative: {:?}", p);
        }
        for p in &files.documentation {
            assert!(p.starts_with("pkg/"), "doc path should be relative: {:?}", p);
        }
        assert!(
            files.editor_config.as_ref().unwrap().starts_with("pkg/"),
            "editor_config should be relative"
        );
        for p in &files.command_runner {
            assert!(p.starts_with("pkg/"), "command_runner path should be relative: {:?}", p);
        }
    }

    #[test]
    fn test_resolve_internal_deps() {
        let dir = TempDir::new().unwrap();

        // Create a 3-package workspace where core is used by both app and lib
        fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"core\", \"lib\", \"app\"]\n",
        )
        .unwrap();

        // core: no deps on other workspace members
        let core_dir = dir.path().join("core");
        fs::create_dir(&core_dir).unwrap();
        fs::write(
            core_dir.join("Cargo.toml"),
            "[package]\nname = \"core\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1.0\"\n",
        )
        .unwrap();

        // lib: depends on core
        let lib_dir = dir.path().join("lib");
        fs::create_dir(&lib_dir).unwrap();
        fs::write(
            lib_dir.join("Cargo.toml"),
            "[package]\nname = \"lib\"\nversion = \"0.1.0\"\n\n[dependencies]\ncore = { path = \"../core\" }\n",
        )
        .unwrap();

        // app: depends on both core and lib
        let app_dir = dir.path().join("app");
        fs::create_dir(&app_dir).unwrap();
        fs::write(
            app_dir.join("Cargo.toml"),
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\ncore = { path = \"../core\" }\nlib = { path = \"../lib\" }\n",
        )
        .unwrap();

        let result = detect_repo(dir.path()).unwrap().unwrap();
        let packages = result.packages.unwrap();

        let core_pkg = packages.iter().find(|p| p.name == "core").unwrap();
        let lib_pkg = packages.iter().find(|p| p.name == "lib").unwrap();
        let app_pkg = packages.iter().find(|p| p.name == "app").unwrap();

        // core depends on nothing internal
        assert!(core_pkg.depends_on.is_empty());
        // core is used by lib and app
        assert!(core_pkg.used_by.contains(&"lib".to_string()));
        assert!(core_pkg.used_by.contains(&"app".to_string()));

        // lib depends on core
        assert_eq!(lib_pkg.depends_on, vec!["core"]);
        // lib is used by app
        assert_eq!(lib_pkg.used_by, vec!["app"]);

        // app depends on core and lib
        assert!(app_pkg.depends_on.contains(&"core".to_string()));
        assert!(app_pkg.depends_on.contains(&"lib".to_string()));
        // app is not used by anyone
        assert!(app_pkg.used_by.is_empty());
    }

    #[test]
    fn test_cargo_lock_versions_parse() {
        let dir = TempDir::new().unwrap();
        let lock_content = r#"
[[package]]
name = "serde"
version = "1.0.210"

[[package]]
name = "tokio"
version = "1.48.0"

[[package]]
name = "serde_json"
version = "1.0.128"
"#;
        let lock_path = dir.path().join("Cargo.lock");
        fs::write(&lock_path, lock_content).unwrap();

        let versions = CargoLockVersions::parse(&lock_path).unwrap();
        assert_eq!(versions.resolve("serde"), Some("1.0.210".to_string()));
        assert_eq!(versions.resolve("tokio"), Some("1.48.0".to_string()));
        assert_eq!(versions.resolve("serde_json"), Some("1.0.128".to_string()));
        assert_eq!(versions.resolve("nonexistent"), None);
    }

    #[test]
    fn test_cargo_lock_versions_missing_file() {
        let result = CargoLockVersions::parse(Path::new("/nonexistent/Cargo.lock"));
        assert!(result.is_none());
    }

    #[test]
    fn test_package_has_relative_path() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = [\"pkg\"]\n").unwrap();
        let pkg_dir = dir.path().join("pkg");
        fs::create_dir(&pkg_dir).unwrap();
        fs::write(
            pkg_dir.join("Cargo.toml"),
            "[package]\nname = \"my-pkg\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let result = detect_repo(dir.path()).unwrap().unwrap();
        let packages = result.packages.unwrap();
        let pkg = &packages[0];

        assert_eq!(pkg.relative, "pkg");
        assert_eq!(pkg.name, "my-pkg");
        assert_eq!(pkg.version, Some("0.1.0".to_string()));
    }

    #[test]
    fn test_dependency_kind_hidden_from_json() {
        let dep = DependencyEntry {
            name: "serde".to_string(),
            kind: DependencyKind::Normal,
            targeted_version: "1.0".to_string(),
            actual_version: None,
            package_manager: Some("cargo".to_string()),
            latest_version: None,
            target: None,
            optional: false,
            features: vec![],
            is_updatable: false,
            has_major_update: false,
        };

        let json = serde_json::to_string(&dep).unwrap();
        assert!(!json.contains("kind"), "kind field should be hidden from JSON");
        assert!(!json.contains("is_updatable"), "is_updatable=false should be skipped");
    }

    #[test]
    fn test_is_updatable_shown_when_true() {
        let dep = DependencyEntry {
            name: "serde".to_string(),
            kind: DependencyKind::Normal,
            targeted_version: "1.0".to_string(),
            actual_version: Some("1.0.200".to_string()),
            package_manager: Some("cargo".to_string()),
            latest_version: Some("1.0.210".to_string()),
            target: None,
            optional: false,
            features: vec![],
            is_updatable: true,
            has_major_update: false,
        };

        let json = serde_json::to_string(&dep).unwrap();
        assert!(json.contains("is_updatable"), "is_updatable=true should be serialized");
    }

    #[test]
    fn test_make_package_area_nested() {
        assert_eq!(make_package_area("sniff/lib"), "sniff");
        assert_eq!(make_package_area("sniff/cli"), "sniff");
    }

    #[test]
    fn test_make_package_area_deeply_nested() {
        assert_eq!(make_package_area("apps/browser/my_package"), "apps/browser");
    }

    #[test]
    fn test_make_package_area_top_level() {
        assert_eq!(make_package_area("my_package"), "root");
    }

    #[test]
    fn test_make_package_area_empty() {
        assert_eq!(make_package_area(""), "root");
    }

    #[test]
    fn test_cargo_workspace_packages_have_area() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/foo\", \"bar\"]\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("crates/foo")).unwrap();
        fs::create_dir(dir.path().join("bar")).unwrap();

        let info = detect_repo(dir.path()).unwrap().unwrap();
        let packages = info.packages.unwrap();

        let foo = packages.iter().find(|p| p.relative == "crates/foo").unwrap();
        assert_eq!(foo.package_area, "crates");

        let bar = packages.iter().find(|p| p.relative == "bar").unwrap();
        assert_eq!(bar.package_area, "root");
    }
}
