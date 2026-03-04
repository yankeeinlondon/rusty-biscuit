use super::languages::detect_languages;
use crate::{Result, SniffError};
use biscuit_file::serde_yaml_ng;
use biscuit_file::toml_crate;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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
    /// Whether this package is excluded from the workspace
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_excluded: bool,
}

impl RepoInfo {
    /// Find the package whose directory tree contains `dir`.
    ///
    /// Returns `None` when `dir` is not inside any package.
    pub fn package_for_dir(&self, dir: &Path) -> Option<&Package> {
        let packages = self.packages.as_ref()?;
        let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());

        packages.iter().find(|pkg| {
            let pkg_path =
                std::fs::canonicalize(&pkg.path).unwrap_or_else(|_| pkg.path.clone());
            dir.starts_with(&pkg_path)
        })
    }

    /// Find the package area that contains `dir`.
    ///
    /// First checks if `dir` is inside a specific package, then falls back to
    /// checking whether it sits anywhere within a package area directory.
    /// Returns `None` when `dir` is outside every known package area.
    pub fn package_area_for_dir(&self, dir: &Path) -> Option<&str> {
        let packages = self.packages.as_ref()?;
        let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());

        // Check if inside a specific package first
        if let Some(pkg) = packages.iter().find(|pkg| {
            let pkg_path =
                std::fs::canonicalize(&pkg.path).unwrap_or_else(|_| pkg.path.clone());
            dir.starts_with(&pkg_path)
        }) {
            return Some(&pkg.package_area);
        }

        // Fall back to checking package area directories
        let root = std::fs::canonicalize(&self.root).unwrap_or_else(|_| self.root.clone());
        let areas: HashSet<&str> = packages
            .iter()
            .map(|p| p.package_area.as_str())
            .filter(|a| *a != "root")
            .collect();

        for area in &areas {
            let area_path = root.join(area);
            if dir.starts_with(&area_path) {
                // Return a reference with the right lifetime by finding the
                // original &str in the packages vec
                return packages
                    .iter()
                    .find(|p| p.package_area == *area)
                    .map(|p| p.package_area.as_str());
            }
        }

        None
    }
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
        let parsed: toml_crate::Value = toml_crate::from_str(&content).ok()?;

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
    if let Some(info) = detect_yarn_workspace(root)? {
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
    let parsed: toml_crate::Value =
        toml_crate::from_str(&content).map_err(|e| SniffError::SystemInfo {
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
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>())
        .unwrap_or_default();

    if members.is_empty() {
        return Ok(None);
    }

    // Parse Cargo.lock once for version resolution
    let lock_versions = CargoLockVersions::parse(&root.join("Cargo.lock"));

    let excludes = workspace
        .get("exclude")
        .and_then(|m| m.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>())
        .unwrap_or_default();

    // Expand globs and collect packages with dependencies
    let mut packages = expand_glob_patterns_with_deps(
        root,
        &members,
        MonorepoTool::CargoWorkspace,
        &lock_versions,
    );

    // Expand excluded patterns and mark them
    let mut excluded_packages = expand_glob_patterns_with_deps(
        root,
        &excludes,
        MonorepoTool::CargoWorkspace,
        &lock_versions,
    );
    for pkg in &mut excluded_packages {
        pkg.is_excluded = true;
    }
    packages.extend(excluded_packages);

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
    let parsed: toml_crate::Value = toml_crate::from_str(&content).ok()?;

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
    parsed: &toml_crate::Value,
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
                toml_crate::Value::String(v) => (v.clone(), Vec::new(), false),
                toml_crate::Value::Table(t) => {
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

/// Parses a single dependency section from package.json.
fn parse_package_json_dep_section(
    parsed: &serde_json::Value,
    section: &str,
    kind: DependencyKind,
    package_manager: &str,
    optional: bool,
) -> Vec<DependencyEntry> {
    let Some(deps) = parsed.get(section).and_then(|d| d.as_object()) else {
        return Vec::new();
    };

    deps.iter()
        .map(|(name, value)| {
            let targeted_version = value
                .as_str()
                .map(String::from)
                .or_else(|| value.get("version").and_then(|v| v.as_str()).map(String::from))
                .unwrap_or_else(|| "*".to_string());

            DependencyEntry {
                name: name.clone(),
                kind,
                targeted_version,
                actual_version: None,
                package_manager: Some(package_manager.to_string()),
                latest_version: None,
                target: None,
                optional,
                features: Vec::new(),
                is_updatable: false,
                has_major_update: false,
            }
        })
        .collect()
}

/// Parses package.json dependencies into category-specific vectors.
#[allow(clippy::type_complexity)]
fn parse_package_json_dependencies(
    package_json_path: &Path,
    package_manager: &str,
) -> Option<(Vec<DependencyEntry>, Vec<DependencyEntry>, Vec<DependencyEntry>, Vec<DependencyEntry>)>
{
    let content = std::fs::read_to_string(package_json_path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;

    let deps = parse_package_json_dep_section(
        &parsed,
        "dependencies",
        DependencyKind::Normal,
        package_manager,
        false,
    );
    let dev_deps = parse_package_json_dep_section(
        &parsed,
        "devDependencies",
        DependencyKind::Dev,
        package_manager,
        false,
    );
    let peer_deps = parse_package_json_dep_section(
        &parsed,
        "peerDependencies",
        DependencyKind::Normal,
        package_manager,
        false,
    );
    let optional_deps = parse_package_json_dep_section(
        &parsed,
        "optionalDependencies",
        DependencyKind::Optional,
        package_manager,
        true,
    );

    Some((deps, dev_deps, peer_deps, optional_deps))
}

/// Extracts package name from a PEP 508 requirement string.
fn parse_python_requirement_name(requirement: &str) -> Option<String> {
    let without_comment = requirement.split('#').next()?.trim();
    if without_comment.is_empty() {
        return None;
    }

    let before_marker = without_comment.split(';').next().unwrap_or(without_comment).trim();
    if before_marker.is_empty() {
        return None;
    }

    let before_at = before_marker.split('@').next().unwrap_or(before_marker).trim();
    if before_at.is_empty() {
        return None;
    }

    let end = before_at
        .find(|c: char| {
            c == '['
                || c == '<'
                || c == '>'
                || c == '='
                || c == '!'
                || c == '~'
                || c.is_whitespace()
        })
        .unwrap_or(before_at.len());
    let name = before_at[..end].trim();

    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// Parses pyproject.toml dependencies from PEP 621 `[project]` sections.
fn parse_pyproject_dependencies(
    pyproject_path: &Path,
) -> Option<(Vec<DependencyEntry>, Vec<DependencyEntry>)> {
    let content = std::fs::read_to_string(pyproject_path).ok()?;
    let parsed: toml_crate::Value = toml_crate::from_str(&content).ok()?;
    let project = parsed.get("project")?;

    let dependencies = project
        .get("dependencies")
        .and_then(|v| v.as_array())
        .map(|deps| {
            deps.iter()
                .filter_map(|entry| entry.as_str())
                .filter_map(|req| parse_python_requirement_name(req).map(|name| (name, req)))
                .map(|(name, req)| DependencyEntry {
                    name,
                    kind: DependencyKind::Normal,
                    targeted_version: req.to_string(),
                    actual_version: None,
                    package_manager: Some("pip".to_string()),
                    latest_version: None,
                    target: None,
                    optional: false,
                    features: Vec::new(),
                    is_updatable: false,
                    has_major_update: false,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let optional_dependencies = project
        .get("optional-dependencies")
        .and_then(|v| v.as_table())
        .map(|groups| {
            groups
                .values()
                .filter_map(|value| value.as_array())
                .flat_map(|entries| entries.iter())
                .filter_map(|entry| entry.as_str())
                .filter_map(|req| parse_python_requirement_name(req).map(|name| (name, req)))
                .map(|(name, req)| DependencyEntry {
                    name,
                    kind: DependencyKind::Optional,
                    targeted_version: req.to_string(),
                    actual_version: None,
                    package_manager: Some("pip".to_string()),
                    latest_version: None,
                    target: None,
                    optional: true,
                    features: Vec::new(),
                    is_updatable: false,
                    has_major_update: false,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Some((dependencies, optional_dependencies))
}

/// Parses requirements.txt dependency lines.
fn parse_requirements_txt_dependencies(requirements_path: &Path) -> Option<Vec<DependencyEntry>> {
    let content = std::fs::read_to_string(requirements_path).ok()?;
    let mut deps = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some(name) = parse_python_requirement_name(trimmed) else {
            continue;
        };

        deps.push(DependencyEntry {
            name,
            kind: DependencyKind::Normal,
            targeted_version: trimmed.to_string(),
            actual_version: None,
            package_manager: Some("pip".to_string()),
            latest_version: None,
            target: None,
            optional: false,
            features: Vec::new(),
            is_updatable: false,
            has_major_update: false,
        });
    }

    if deps.is_empty() {
        None
    } else {
        Some(deps)
    }
}

/// Parses go.mod `require` entries.
fn parse_go_mod_dependencies(go_mod_path: &Path) -> Option<Vec<DependencyEntry>> {
    let content = std::fs::read_to_string(go_mod_path).ok()?;
    let mut deps = Vec::new();
    let mut in_require_block = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        if in_require_block {
            if trimmed.starts_with(')') {
                in_require_block = false;
                continue;
            }

            let line_without_comment = trimmed.split("//").next().unwrap_or(trimmed).trim();
            let mut parts = line_without_comment.split_whitespace();
            let Some(name) = parts.next() else {
                continue;
            };
            let Some(version) = parts.next() else {
                continue;
            };

            deps.push(DependencyEntry {
                name: name.to_string(),
                kind: DependencyKind::Normal,
                targeted_version: version.to_string(),
                actual_version: None,
                package_manager: Some("go".to_string()),
                latest_version: None,
                target: None,
                optional: false,
                features: Vec::new(),
                is_updatable: false,
                has_major_update: false,
            });

            continue;
        }

        if trimmed.starts_with("require (") {
            in_require_block = true;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("require ") {
            let line_without_comment = rest.split("//").next().unwrap_or(rest).trim();
            let mut parts = line_without_comment.split_whitespace();
            let Some(name) = parts.next() else {
                continue;
            };
            let Some(version) = parts.next() else {
                continue;
            };

            deps.push(DependencyEntry {
                name: name.to_string(),
                kind: DependencyKind::Normal,
                targeted_version: version.to_string(),
                actual_version: None,
                package_manager: Some("go".to_string()),
                latest_version: None,
                target: None,
                optional: false,
                features: Vec::new(),
                is_updatable: false,
                has_major_update: false,
            });
        }
    }

    if deps.is_empty() {
        None
    } else {
        Some(deps)
    }
}

fn detect_pnpm_workspace(root: &Path) -> Result<Option<RepoInfo>> {
    let pnpm_workspace = root.join("pnpm-workspace.yaml");
    if !pnpm_workspace.exists() {
        return Ok(None);
    }

    let packages = parse_pnpm_workspace_patterns(&pnpm_workspace)?;

    if packages.is_empty() {
        return Ok(None);
    }

    let lock_versions = None;
    let mut package_locations = expand_glob_patterns_with_deps(
        root,
        &packages,
        MonorepoTool::PnpmWorkspaces,
        &lock_versions,
    );
    package_locations = dedupe_packages(package_locations);
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

    let workspaces = parse_package_json_workspace_patterns(&package_json)?.unwrap_or_default();

    if workspaces.is_empty() {
        return Ok(None);
    }

    let lock_versions = None;
    let mut packages = expand_glob_patterns_with_deps(
        root,
        &workspaces,
        MonorepoTool::NpmWorkspaces,
        &lock_versions,
    );
    packages = dedupe_packages(packages);
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

fn detect_yarn_workspace(root: &Path) -> Result<Option<RepoInfo>> {
    if !root.join("yarn.lock").exists() {
        return Ok(None);
    }

    let package_json = root.join("package.json");
    if !package_json.exists() {
        return Ok(None);
    }

    let workspaces = parse_package_json_workspace_patterns(&package_json)?.unwrap_or_default();

    if workspaces.is_empty() {
        return Ok(None);
    }

    let lock_versions = None;
    let mut packages = expand_glob_patterns_with_deps(
        root,
        &workspaces,
        MonorepoTool::YarnWorkspaces,
        &lock_versions,
    );
    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: Some(MonorepoTool::YarnWorkspaces),
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

    let mut patterns = collect_default_workspace_patterns(root);
    patterns.extend(parse_nx_layout_patterns(&nx_json));
    patterns = dedupe_patterns(patterns);

    let lock_versions = None;
    let mut packages = if patterns.is_empty() {
        discover_packages_from_manifests(root, MonorepoTool::Nx, &lock_versions)
    } else {
        expand_glob_patterns_with_deps(root, &patterns, MonorepoTool::Nx, &lock_versions)
    };
    if packages.is_empty() {
        packages = discover_packages_from_manifests(root, MonorepoTool::Nx, &lock_versions);
    }
    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: Some(MonorepoTool::Nx),
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages: Some(packages),
    }))
}

fn detect_turborepo(root: &Path) -> Result<Option<RepoInfo>> {
    let turbo_json = root.join("turbo.json");
    if !turbo_json.exists() {
        return Ok(None);
    }

    let patterns = collect_default_workspace_patterns(root);
    let lock_versions = None;
    let mut packages = if patterns.is_empty() {
        discover_packages_from_manifests(root, MonorepoTool::Turborepo, &lock_versions)
    } else {
        expand_glob_patterns_with_deps(root, &patterns, MonorepoTool::Turborepo, &lock_versions)
    };
    if packages.is_empty() {
        packages = discover_packages_from_manifests(root, MonorepoTool::Turborepo, &lock_versions);
    }
    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: Some(MonorepoTool::Turborepo),
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages: Some(packages),
    }))
}

fn detect_lerna(root: &Path) -> Result<Option<RepoInfo>> {
    let lerna_json = root.join("lerna.json");
    if !lerna_json.exists() {
        return Ok(None);
    }

    let mut patterns = parse_lerna_workspace_patterns(&lerna_json).unwrap_or_default();
    patterns.extend(collect_default_workspace_patterns(root));
    patterns = dedupe_patterns(patterns);

    let lock_versions = None;
    let mut packages = if patterns.is_empty() {
        discover_packages_from_manifests(root, MonorepoTool::Lerna, &lock_versions)
    } else {
        expand_glob_patterns_with_deps(root, &patterns, MonorepoTool::Lerna, &lock_versions)
    };
    if packages.is_empty() {
        packages = discover_packages_from_manifests(root, MonorepoTool::Lerna, &lock_versions);
    }
    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: Some(MonorepoTool::Lerna),
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages: Some(packages),
    }))
}

fn parse_pnpm_workspace_patterns(pnpm_workspace_path: &Path) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(pnpm_workspace_path)?;
    let parsed: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&content).map_err(|e| SniffError::SystemInfo {
            domain: "repo",
            message: e.to_string(),
        })?;

    Ok(parsed
        .get("packages")
        .and_then(|p| p.as_sequence())
        .map(|seq| seq.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>())
        .unwrap_or_default())
}

fn parse_package_json_workspace_patterns(package_json_path: &Path) -> Result<Option<Vec<String>>> {
    let content = std::fs::read_to_string(package_json_path)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| SniffError::SystemInfo {
            domain: "repo",
            message: e.to_string(),
        })?;
    let Some(workspaces) = parsed.get("workspaces") else {
        return Ok(None);
    };

    if let Some(arr) = workspaces.as_array() {
        return Ok(Some(arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()));
    }

    if let Some(obj) = workspaces.as_object() {
        return Ok(Some(
            obj.get("packages")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
        ));
    }

    Ok(Some(Vec::new()))
}

fn parse_lerna_workspace_patterns(lerna_json_path: &Path) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(lerna_json_path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;

    parsed
        .get("packages")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
}

fn parse_nx_layout_patterns(nx_json_path: &Path) -> Vec<String> {
    let content = match std::fs::read_to_string(nx_json_path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };
    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(parsed) => parsed,
        Err(_) => return Vec::new(),
    };

    let apps_dir = parsed
        .get("workspaceLayout")
        .and_then(|v| v.get("appsDir"))
        .and_then(|v| v.as_str())
        .unwrap_or("apps");
    let libs_dir = parsed
        .get("workspaceLayout")
        .and_then(|v| v.get("libsDir"))
        .and_then(|v| v.as_str())
        .unwrap_or("libs");

    vec![
        format!("{apps_dir}/*"),
        format!("{libs_dir}/*"),
    ]
}

fn collect_default_workspace_patterns(root: &Path) -> Vec<String> {
    let mut patterns = Vec::new();

    if let Ok(Some(package_json_patterns)) =
        parse_package_json_workspace_patterns(&root.join("package.json"))
    {
        patterns.extend(package_json_patterns);
    }

    if let Ok(pnpm_patterns) = parse_pnpm_workspace_patterns(&root.join("pnpm-workspace.yaml")) {
        patterns.extend(pnpm_patterns);
    }

    if let Some(lerna_patterns) = parse_lerna_workspace_patterns(&root.join("lerna.json")) {
        patterns.extend(lerna_patterns);
    }

    dedupe_patterns(patterns)
}

// ============================================================================
// Package name reading helpers
// ============================================================================

/// Reads the package name from a Cargo.toml file.
fn read_cargo_package_name(cargo_toml: &Path) -> Option<String> {
    let content = std::fs::read_to_string(cargo_toml).ok()?;
    let parsed: toml_crate::Value = toml_crate::from_str(&content).ok()?;
    parsed.get("package").and_then(|p| p.get("name")).and_then(|n| n.as_str()).map(String::from)
}

/// Reads the package version from a Cargo.toml file.
fn read_cargo_package_version(cargo_toml: &Path) -> Option<String> {
    let content = std::fs::read_to_string(cargo_toml).ok()?;
    let parsed: toml_crate::Value = toml_crate::from_str(&content).ok()?;
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

/// Reads package name from a pyproject.toml `[project].name`.
fn read_pyproject_package_name(pyproject_toml: &Path) -> Option<String> {
    let content = std::fs::read_to_string(pyproject_toml).ok()?;
    let parsed: toml_crate::Value = toml_crate::from_str(&content).ok()?;
    parsed.get("project").and_then(|p| p.get("name")).and_then(|n| n.as_str()).map(String::from)
}

/// Reads package version from a pyproject.toml `[project].version`.
fn read_pyproject_package_version(pyproject_toml: &Path) -> Option<String> {
    let content = std::fs::read_to_string(pyproject_toml).ok()?;
    let parsed: toml_crate::Value = toml_crate::from_str(&content).ok()?;
    parsed.get("project").and_then(|p| p.get("version")).and_then(|v| v.as_str()).map(String::from)
}

/// Reads module name from a go.mod file (module directive).
fn read_go_module_name(go_mod: &Path) -> Option<String> {
    let content = std::fs::read_to_string(go_mod).ok()?;
    content.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed.strip_prefix("module ").map(|value| value.trim().to_string())
    })
}

/// Reads the feature flag names from a Cargo.toml `[features]` section.
fn read_cargo_features(cargo_toml: &Path) -> Vec<String> {
    let content = match std::fs::read_to_string(cargo_toml) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let parsed: toml_crate::Value = match toml_crate::from_str(&content) {
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
            if cargo_toml.exists()
                && let Some(name) = read_cargo_package_name(&cargo_toml)
            {
                return name;
            }
        }
        MonorepoTool::NpmWorkspaces
        | MonorepoTool::PnpmWorkspaces
        | MonorepoTool::YarnWorkspaces
        | MonorepoTool::Nx
        | MonorepoTool::Turborepo
        | MonorepoTool::Lerna => {
            let package_json = path.join("package.json");
            if package_json.exists()
                && let Some(name) = read_npm_package_name(&package_json)
            {
                return name;
            }
        }
        _ => {}
    }

    let pyproject_toml = path.join("pyproject.toml");
    if pyproject_toml.exists()
        && let Some(name) = read_pyproject_package_name(&pyproject_toml)
    {
        return name;
    }

    let go_mod = path.join("go.mod");
    if go_mod.exists()
        && let Some(name) = read_go_module_name(&go_mod)
    {
        return name;
    }

    // Fallback: relative path from root
    make_relative_path(path, root)
}

/// Determines the package version based on monorepo tool type.
fn resolve_package_version(path: &Path, root: &Path, tool: MonorepoTool) -> Option<String> {
    match tool {
        MonorepoTool::CargoWorkspace => {
            let cargo_toml = path.join("Cargo.toml");
            if cargo_toml.exists() {
                return read_cargo_package_version(&cargo_toml);
            }
        }
        MonorepoTool::NpmWorkspaces
        | MonorepoTool::PnpmWorkspaces
        | MonorepoTool::YarnWorkspaces
        | MonorepoTool::Nx
        | MonorepoTool::Turborepo
        | MonorepoTool::Lerna => {
            let package_json = path.join("package.json");
            if package_json.exists() {
                if let Some(version) = read_npm_package_version(&package_json) {
                    return Some(version);
                }
                if path != root {
                    let root_package_json = root.join("package.json");
                    if root_package_json.exists() {
                        return read_npm_package_version(&root_package_json);
                    }
                }
            }
        }
        _ => {}
    }

    let pyproject_toml = path.join("pyproject.toml");
    if pyproject_toml.exists()
        && let Some(version) = read_pyproject_package_version(&pyproject_toml)
    {
        return Some(version);
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
            pkg.peer_dependencies.as_ref(),
            pkg.optional_dependencies.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            for dep in dep_list {
                if package_names.contains(&dep.name)
                    && dep.name != pkg.name
                    && !internal_deps.contains(&dep.name)
                {
                    internal_deps.push(dep.name.clone());
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

fn resolve_js_package_manager(
    tool: MonorepoTool,
    root: &Path,
    package_managers: &[String],
) -> &'static str {
    match tool {
        MonorepoTool::PnpmWorkspaces => return "pnpm",
        MonorepoTool::YarnWorkspaces => return "yarn",
        _ => {}
    }

    if package_managers.iter().any(|manager| manager == "pnpm")
        || root.join("pnpm-lock.yaml").exists()
    {
        return "pnpm";
    }
    if package_managers.iter().any(|manager| manager == "yarn") || root.join("yarn.lock").exists() {
        return "yarn";
    }
    if root.join("bun.lock").exists() || root.join("bun.lockb").exists() {
        return "bun";
    }

    "npm"
}

fn dedupe_patterns(patterns: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for pattern in patterns {
        if seen.insert(pattern.clone()) {
            deduped.push(pattern);
        }
    }

    deduped
}

fn dedupe_packages(packages: Vec<Package>) -> Vec<Package> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for package in packages {
        if seen.insert(package.relative.clone()) {
            deduped.push(package);
        }
    }

    deduped
}

fn discover_packages_from_manifests(
    root: &Path,
    tool: MonorepoTool,
    lock_versions: &Option<CargoLockVersions>,
) -> Vec<Package> {
    let mut discovered_dirs = HashSet::new();

    let walker =
        walkdir::WalkDir::new(root).follow_links(false).into_iter().filter_entry(|entry| {
            if !entry.file_type().is_dir() {
                return true;
            }

            let name = entry.file_name().to_string_lossy();
            name != ".git"
                && name != "node_modules"
                && name != "target"
                && name != ".turbo"
                && name != "dist"
                && name != "build"
        });

    for entry in walker.filter_map(|entry| entry.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy();
        let is_manifest = matches!(
            file_name.as_ref(),
            "Cargo.toml" | "package.json" | "pyproject.toml" | "go.mod"
        );
        if !is_manifest {
            continue;
        }

        let Some(parent) = entry.path().parent() else {
            continue;
        };
        if parent == root {
            continue;
        }
        discovered_dirs.insert(parent.to_path_buf());
    }

    let mut dirs: Vec<PathBuf> = discovered_dirs.into_iter().collect();
    dirs.sort();

    dirs.iter().map(|path| create_package(path, root, tool, lock_versions)).collect()
}

#[allow(dead_code)]
/// Expand glob patterns and parse dependencies for Cargo workspaces.
fn expand_glob_patterns_with_deps(
    root: &Path,
    patterns: &[String],
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
    let (primary_language, languages) = detect_package_languages(path);
    let package_managers = detect_package_managers(path);
    let version = resolve_package_version(path, root, tool);
    let files = detect_package_files(path, root);

    // Read feature flags (Cargo only for now)
    let cargo_toml = path.join("Cargo.toml");
    let features = if cargo_toml.exists() {
        read_cargo_features(&cargo_toml)
    } else {
        Vec::new()
    };

    let mut dependencies = Vec::new();
    let mut dev_dependencies = Vec::new();
    let mut peer_dependencies = Vec::new();
    let mut optional_dependencies = Vec::new();

    if cargo_toml.exists()
        && let Some((normal, dev, build)) = parse_cargo_dependencies(&cargo_toml, lock_versions)
    {
        let mut all_deps = normal;
        all_deps.extend(build);

        let (optional, regular): (Vec<_>, Vec<_>) = all_deps.into_iter().partition(|d| d.optional);

        dependencies.extend(regular);
        dev_dependencies.extend(dev);
        optional_dependencies.extend(optional);
    }

    // Parse package.json dependency sections when available.
    let package_json = path.join("package.json");
    if package_json.exists() {
        let js_package_manager = resolve_js_package_manager(tool, root, &package_managers);
        if let Some((normal, dev, peer, optional)) =
            parse_package_json_dependencies(&package_json, js_package_manager)
        {
            dependencies.extend(normal);
            dev_dependencies.extend(dev);
            peer_dependencies.extend(peer);
            optional_dependencies.extend(optional);
        }
    }

    // Parse Python dependencies from pyproject.toml / requirements.txt.
    let pyproject_toml = path.join("pyproject.toml");
    if pyproject_toml.exists()
        && let Some((normal, optional)) = parse_pyproject_dependencies(&pyproject_toml)
    {
        dependencies.extend(normal);
        optional_dependencies.extend(optional);
    }
    let requirements_txt = path.join("requirements.txt");
    if requirements_txt.exists()
        && let Some(req_deps) = parse_requirements_txt_dependencies(&requirements_txt)
    {
        dependencies.extend(req_deps);
    }

    // Parse Go module dependencies when available.
    let go_mod = path.join("go.mod");
    if go_mod.exists()
        && let Some(go_deps) = parse_go_mod_dependencies(&go_mod)
    {
        dependencies.extend(go_deps);
    };

    let dependencies = if dependencies.is_empty() {
        None
    } else {
        Some(dependencies)
    };
    let dev_dependencies = if dev_dependencies.is_empty() {
        None
    } else {
        Some(dev_dependencies)
    };
    let peer_dependencies = if peer_dependencies.is_empty() {
        None
    } else {
        Some(peer_dependencies)
    };
    let optional_dependencies = if optional_dependencies.is_empty() {
        None
    } else {
        Some(optional_dependencies)
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
        peer_dependencies,
        optional_dependencies,
        is_updatable: None,
        has_major_update: None,
        is_excluded: false,
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
    fn test_cargo_workspace_excludes_detected() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"pkg1\"]\nexclude = [\"excluded1\"]\n",
        )
        .unwrap();
        fs::create_dir(dir.path().join("pkg1")).unwrap();
        fs::create_dir(dir.path().join("excluded1")).unwrap();

        let result = detect_repo(dir.path()).unwrap();
        assert!(result.is_some());
        let info = result.unwrap();
        assert!(info.is_monorepo);
        let packages = info.packages.as_ref().unwrap();
        assert_eq!(packages.len(), 2);

        let member = packages.iter().find(|p| p.name == "pkg1").unwrap();
        assert!(!member.is_excluded);

        let excluded = packages.iter().find(|p| p.name == "excluded1").unwrap();
        assert!(excluded.is_excluded);
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
    fn test_pnpm_workspace_resolves_internal_dependencies_from_package_json() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("pnpm-workspace.yaml"), "packages:\n  - 'packages/*'\n").unwrap();
        fs::write(dir.path().join("package.json"), r#"{"private": true}"#).unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "lockfileVersion: '9.0'").unwrap();

        let app_dir = dir.path().join("packages/app");
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(
            app_dir.join("package.json"),
            r#"{
  "name": "@scope/app",
  "version": "1.2.3",
  "dependencies": {
    "@scope/lib": "workspace:*",
    "lodash": "^4.17.21"
  },
  "devDependencies": {
    "@scope/lib": "workspace:*"
  }
}"#,
        )
        .unwrap();

        let lib_dir = dir.path().join("packages/lib");
        fs::create_dir_all(&lib_dir).unwrap();
        fs::write(
            lib_dir.join("package.json"),
            r#"{
  "name": "@scope/lib",
  "version": "2.0.0"
}"#,
        )
        .unwrap();

        let result = detect_repo(dir.path()).unwrap().unwrap();
        assert_eq!(result.monorepo_tool, Some(MonorepoTool::PnpmWorkspaces));
        let packages = result.packages.unwrap();

        let app = packages.iter().find(|p| p.name == "@scope/app").unwrap();
        assert_eq!(app.version.as_deref(), Some("1.2.3"));
        assert_eq!(app.depends_on, vec!["@scope/lib".to_string()]);
        assert_eq!(app.dependencies.as_ref().unwrap().len(), 2);
        assert!(app.dev_dependencies.is_some());

        let lib = packages.iter().find(|p| p.name == "@scope/lib").unwrap();
        assert!(lib.used_by.contains(&"@scope/app".to_string()));
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
    fn test_nx_detected_with_workspace_packages() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("nx.json"), "{}").unwrap();
        fs::write(dir.path().join("package.json"), r#"{"workspaces": ["apps/*", "libs/*"]}"#)
            .unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "lockfileVersion: '9.0'").unwrap();

        let app_dir = dir.path().join("apps/web");
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(
            app_dir.join("package.json"),
            r#"{
  "name": "@scope/web",
  "version": "0.1.0",
  "dependencies": {"@scope/shared": "workspace:*"}
}"#,
        )
        .unwrap();

        let shared_dir = dir.path().join("libs/shared");
        fs::create_dir_all(&shared_dir).unwrap();
        fs::write(
            shared_dir.join("package.json"),
            r#"{"name": "@scope/shared", "version": "0.1.0"}"#,
        )
        .unwrap();

        let result = detect_repo(dir.path()).unwrap().unwrap();
        assert_eq!(result.monorepo_tool, Some(MonorepoTool::Nx));
        let packages = result.packages.unwrap();
        assert_eq!(packages.len(), 2);

        let web = packages.iter().find(|p| p.name == "@scope/web").unwrap();
        assert_eq!(web.depends_on, vec!["@scope/shared".to_string()]);
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
    fn test_npm_workspace_detected_with_object_syntax() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"workspaces": {"packages": ["modules/*"]}}"#,
        )
        .unwrap();
        let pkg = dir.path().join("modules/types");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(pkg.join("package.json"), r#"{"name": "@scope/types", "version": "0.5.0"}"#)
            .unwrap();

        let result = detect_repo(dir.path()).unwrap().unwrap();
        assert_eq!(result.monorepo_tool, Some(MonorepoTool::NpmWorkspaces));
        let packages = result.packages.unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "@scope/types");
        assert_eq!(packages[0].version.as_deref(), Some("0.5.0"));
    }

    #[test]
    fn test_yarn_workspace_detected() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("yarn.lock"), "").unwrap();
        fs::write(dir.path().join("package.json"), r#"{"workspaces": ["packages/*"]}"#).unwrap();
        let pkg = dir.path().join("packages/app");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(pkg.join("package.json"), r#"{"name": "@scope/app", "version": "1.0.0"}"#)
            .unwrap();

        let result = detect_repo(dir.path()).unwrap().unwrap();
        assert_eq!(result.monorepo_tool, Some(MonorepoTool::YarnWorkspaces));
        assert_eq!(result.packages.unwrap().len(), 1);
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
    fn test_turborepo_detects_packages_from_workspaces() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("turbo.json"), "{}").unwrap();
        fs::write(dir.path().join("pnpm-workspace.yaml"), "packages:\n  - 'packages/*'\n").unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "lockfileVersion: '9.0'").unwrap();

        let app_dir = dir.path().join("packages/app");
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(app_dir.join("package.json"), r#"{"name": "@scope/app", "version": "1.0.0"}"#)
            .unwrap();

        let result = detect_repo(dir.path()).unwrap().unwrap();
        assert_eq!(result.monorepo_tool, Some(MonorepoTool::Turborepo));
        let packages = result.packages.unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "@scope/app");
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
    fn test_lerna_detects_packages_from_lerna_json() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("lerna.json"),
            r#"{"packages": ["packages/*"], "version": "independent"}"#,
        )
        .unwrap();

        let app_dir = dir.path().join("packages/app");
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(app_dir.join("package.json"), r#"{"name": "@scope/app", "version": "0.9.0"}"#)
            .unwrap();

        let result = detect_repo(dir.path()).unwrap().unwrap();
        assert_eq!(result.monorepo_tool, Some(MonorepoTool::Lerna));
        let packages = result.packages.unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "@scope/app");
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
    fn test_read_pyproject_name_and_version() {
        let dir = TempDir::new().unwrap();
        let pyproject = dir.path().join("pyproject.toml");
        fs::write(
            &pyproject,
            "[project]\nname = \"py-pkg\"\nversion = \"0.4.0\"\ndependencies = [\"httpx>=0.27\"]\n",
        )
        .unwrap();

        assert_eq!(read_pyproject_package_name(&pyproject), Some("py-pkg".to_string()));
        assert_eq!(read_pyproject_package_version(&pyproject), Some("0.4.0".to_string()));
    }

    #[test]
    fn test_parse_pyproject_dependencies() {
        let dir = TempDir::new().unwrap();
        let pyproject = dir.path().join("pyproject.toml");
        fs::write(
            &pyproject,
            r#"[project]
name = "py-pkg"
version = "0.4.0"
dependencies = ["httpx>=0.27", "pydantic~=2.8"]

[project.optional-dependencies]
dev = ["pytest>=8.0"]
"#,
        )
        .unwrap();

        let (normal, optional) = parse_pyproject_dependencies(&pyproject).unwrap();
        assert_eq!(normal.len(), 2);
        assert!(normal.iter().any(|dep| dep.name == "httpx"));
        assert!(optional.iter().any(|dep| dep.name == "pytest"));
    }

    #[test]
    fn test_parse_go_mod_dependencies() {
        let dir = TempDir::new().unwrap();
        let go_mod = dir.path().join("go.mod");
        fs::write(
            &go_mod,
            r#"module example.com/my-service

go 1.22

require (
    github.com/gorilla/mux v1.8.1
    golang.org/x/net v0.37.0 // indirect
)
"#,
        )
        .unwrap();

        let deps = parse_go_mod_dependencies(&go_mod).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.iter().any(|dep| dep.name == "github.com/gorilla/mux"));
        assert_eq!(read_go_module_name(&go_mod), Some("example.com/my-service".to_string()));
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
