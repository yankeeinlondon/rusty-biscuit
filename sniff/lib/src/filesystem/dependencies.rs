use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::Result;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PackageManager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Cargo,
    Pip,
    Poetry,
    Pdm,
    Uv,
    Bundler,
    Composer,
    Maven,
    Gradle,
    GoMod,
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

impl PackageManager {
    pub fn primary_language(&self) -> &'static str {
        match self {
            Self::Npm | Self::Pnpm | Self::Yarn | Self::Bun => "JavaScript",
            Self::Cargo => "Rust",
            Self::Pip | Self::Poetry | Self::Pdm | Self::Uv => "Python",
            Self::Bundler => "Ruby",
            Self::Composer => "PHP",
            Self::Maven | Self::Gradle => "Java",
            Self::GoMod => "Go",
        }
    }

    pub fn registry_url(&self) -> Option<&'static str> {
        match self {
            Self::Npm | Self::Pnpm | Self::Yarn | Self::Bun => Some("https://registry.npmjs.org"),
            Self::Cargo => Some("https://crates.io"),
            Self::Pip | Self::Poetry | Self::Pdm | Self::Uv => Some("https://pypi.org"),
            Self::Bundler => Some("https://rubygems.org"),
            Self::Composer => Some("https://packagist.org"),
            Self::Maven => Some("https://repo1.maven.org/maven2"),
            Self::GoMod => Some("https://pkg.go.dev"),
            Self::Gradle => None, // Can use various repos
        }
    }

    pub fn lockfile_name(&self) -> Option<&'static str> {
        match self {
            Self::Npm => Some("package-lock.json"),
            Self::Pnpm => Some("pnpm-lock.yaml"),
            Self::Yarn => Some("yarn.lock"),
            Self::Bun => Some("bun.lockb"),
            Self::Cargo => Some("Cargo.lock"),
            Self::Pip => Some("requirements.txt"),
            Self::Poetry => Some("poetry.lock"),
            Self::Pdm => Some("pdm.lock"),
            Self::Uv => Some("uv.lock"),
            Self::Bundler => Some("Gemfile.lock"),
            Self::Composer => Some("composer.lock"),
            Self::Maven => Some("pom.xml"),
            Self::Gradle => Some("gradle.lockfile"),
            Self::GoMod => Some("go.sum"),
        }
    }
}

/// Location where a package manager manifest was found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestLocation {
    /// The package manager type
    pub manager: PackageManager,
    /// Relative path to the manifest file from the root
    pub path: PathBuf,
}

/// Dependencies parsed from a single manifest file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDependencies {
    /// Package name (from manifest)
    pub name: Option<String>,
    /// Relative path to the manifest file
    pub manifest_path: PathBuf,
    /// The package manager used
    pub manager: PackageManager,
    /// All dependencies from this manifest
    pub dependencies: Vec<DependencyEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyReport {
    /// Package managers detected (deduplicated list)
    pub detected_managers: Vec<PackageManager>,
    /// All manifest file locations found in the directory tree
    pub manifests: Vec<ManifestLocation>,
    /// Parsed dependencies per package (for monorepos, includes all packages)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<PackageDependencies>,
}

/// Directories to skip when scanning for package manifests.
const EXCLUDED_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "vendor",
    "dist",
    "build",
    "__pycache__",
    ".venv",
    "venv",
];

/// Detect package managers present in a directory tree.
///
/// Walks the entire directory tree (excluding common build/dependency
/// directories) to find all package manager manifest files. This is
/// essential for monorepos where each package may have its own manifest.
///
/// ## Returns
///
/// Returns a `DependencyReport` containing:
/// - `detected_managers`: Deduplicated list of package manager types found
/// - `manifests`: All manifest file locations with their types
pub fn detect_dependencies(root: &Path) -> Result<DependencyReport> {
    let mut manifests = Vec::new();
    let mut manager_set: HashSet<PackageManager> = HashSet::new();

    // Walk the directory tree
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            // Skip excluded directories
            if e.file_type().is_dir()
                && let Some(name) = e.file_name().to_str() {
                    return !EXCLUDED_DIRS.contains(&name);
                }
            true
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
    {
        let dir = entry.path();
        let rel_path = dir.strip_prefix(root).unwrap_or(dir);

        // Check for Cargo.toml
        if dir.join("Cargo.toml").exists() {
            manifests.push(ManifestLocation {
                manager: PackageManager::Cargo,
                path: rel_path.join("Cargo.toml"),
            });
            manager_set.insert(PackageManager::Cargo);
        }

        // Check for package.json (JavaScript)
        if dir.join("package.json").exists() {
            let js_manager = if dir.join("pnpm-lock.yaml").exists() {
                PackageManager::Pnpm
            } else if dir.join("yarn.lock").exists() {
                PackageManager::Yarn
            } else if dir.join("bun.lockb").exists() {
                PackageManager::Bun
            } else {
                PackageManager::Npm
            };
            manifests.push(ManifestLocation {
                manager: js_manager,
                path: rel_path.join("package.json"),
            });
            manager_set.insert(js_manager);
        }

        // Check for pyproject.toml (Python)
        if dir.join("pyproject.toml").exists() {
            let py_manager = if dir.join("poetry.lock").exists() {
                PackageManager::Poetry
            } else if dir.join("pdm.lock").exists() {
                PackageManager::Pdm
            } else if dir.join("uv.lock").exists() {
                PackageManager::Uv
            } else {
                PackageManager::Pip
            };
            manifests.push(ManifestLocation {
                manager: py_manager,
                path: rel_path.join("pyproject.toml"),
            });
            manager_set.insert(py_manager);
        } else if dir.join("requirements.txt").exists() {
            manifests.push(ManifestLocation {
                manager: PackageManager::Pip,
                path: rel_path.join("requirements.txt"),
            });
            manager_set.insert(PackageManager::Pip);
        }

        // Check for Gemfile (Ruby)
        if dir.join("Gemfile").exists() {
            manifests.push(ManifestLocation {
                manager: PackageManager::Bundler,
                path: rel_path.join("Gemfile"),
            });
            manager_set.insert(PackageManager::Bundler);
        }

        // Check for composer.json (PHP)
        if dir.join("composer.json").exists() {
            manifests.push(ManifestLocation {
                manager: PackageManager::Composer,
                path: rel_path.join("composer.json"),
            });
            manager_set.insert(PackageManager::Composer);
        }

        // Check for pom.xml (Maven)
        if dir.join("pom.xml").exists() {
            manifests.push(ManifestLocation {
                manager: PackageManager::Maven,
                path: rel_path.join("pom.xml"),
            });
            manager_set.insert(PackageManager::Maven);
        }

        // Check for build.gradle or build.gradle.kts (Gradle)
        if dir.join("build.gradle").exists() {
            manifests.push(ManifestLocation {
                manager: PackageManager::Gradle,
                path: rel_path.join("build.gradle"),
            });
            manager_set.insert(PackageManager::Gradle);
        } else if dir.join("build.gradle.kts").exists() {
            manifests.push(ManifestLocation {
                manager: PackageManager::Gradle,
                path: rel_path.join("build.gradle.kts"),
            });
            manager_set.insert(PackageManager::Gradle);
        }

        // Check for go.mod (Go)
        if dir.join("go.mod").exists() {
            manifests.push(ManifestLocation {
                manager: PackageManager::GoMod,
                path: rel_path.join("go.mod"),
            });
            manager_set.insert(PackageManager::GoMod);
        }
    }

    // Convert HashSet to sorted Vec for consistent output
    let mut detected_managers: Vec<PackageManager> = manager_set.into_iter().collect();
    detected_managers.sort_by_key(|m| format!("{:?}", m));

    // Parse dependencies from Cargo.toml files
    let lockfile_versions = parse_cargo_lockfile(root);
    let mut packages = Vec::new();
    for manifest in &manifests {
        if manifest.manager == PackageManager::Cargo
            && let Some(pkg_deps) = parse_cargo_toml(root, &manifest.path, &lockfile_versions) {
                packages.push(pkg_deps);
            }
    }

    Ok(DependencyReport {
        detected_managers,
        manifests,
        packages,
    })
}

/// Parse Cargo.lock to get a mapping of package names to resolved versions.
fn parse_cargo_lockfile(root: &Path) -> HashMap<String, String> {
    let lockfile_path = root.join("Cargo.lock");
    if !lockfile_path.exists() {
        return HashMap::new();
    }

    let content = match std::fs::read_to_string(&lockfile_path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };

    let parsed: toml::Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };

    let mut versions = HashMap::new();
    if let Some(packages) = parsed.get("package").and_then(|p| p.as_array()) {
        for pkg in packages {
            if let (Some(name), Some(version)) = (
                pkg.get("name").and_then(|n| n.as_str()),
                pkg.get("version").and_then(|v| v.as_str()),
            ) {
                // Note: In a lockfile, the same package might appear multiple times
                // with different versions. We take the first occurrence which is
                // typically the one used by the workspace.
                versions.entry(name.to_string()).or_insert_with(|| version.to_string());
            }
        }
    }

    versions
}

/// Parse a Cargo.toml file to extract dependencies.
fn parse_cargo_toml(
    root: &Path,
    manifest_rel_path: &Path,
    lockfile_versions: &HashMap<String, String>,
) -> Option<PackageDependencies> {
    let manifest_path = root.join(manifest_rel_path);
    let content = std::fs::read_to_string(&manifest_path).ok()?;
    let parsed: toml::Value = toml::from_str(&content).ok()?;

    // Get package name
    let name = parsed
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string());

    let mut dependencies = Vec::new();

    // Parse [dependencies]
    if let Some(deps) = parsed.get("dependencies").and_then(|d| d.as_table()) {
        parse_dependency_table(deps, DependencyKind::Normal, None, lockfile_versions, &mut dependencies);
    }

    // Parse [dev-dependencies]
    if let Some(deps) = parsed.get("dev-dependencies").and_then(|d| d.as_table()) {
        parse_dependency_table(deps, DependencyKind::Dev, None, lockfile_versions, &mut dependencies);
    }

    // Parse [build-dependencies]
    if let Some(deps) = parsed.get("build-dependencies").and_then(|d| d.as_table()) {
        parse_dependency_table(deps, DependencyKind::Build, None, lockfile_versions, &mut dependencies);
    }

    // Parse [target.'cfg(...)'.dependencies]
    if let Some(targets) = parsed.get("target").and_then(|t| t.as_table()) {
        for (target_spec, target_table) in targets {
            if let Some(table) = target_table.as_table() {
                if let Some(deps) = table.get("dependencies").and_then(|d| d.as_table()) {
                    parse_dependency_table(deps, DependencyKind::Target, Some(target_spec.clone()), lockfile_versions, &mut dependencies);
                }
                if let Some(deps) = table.get("dev-dependencies").and_then(|d| d.as_table()) {
                    // Target-specific dev dependencies
                    let mut target_deps = Vec::new();
                    parse_dependency_table(deps, DependencyKind::Dev, Some(target_spec.clone()), lockfile_versions, &mut target_deps);
                    dependencies.extend(target_deps);
                }
            }
        }
    }

    // Sort dependencies by name for consistent output
    dependencies.sort_by(|a, b| a.name.cmp(&b.name));

    Some(PackageDependencies {
        name,
        manifest_path: manifest_rel_path.to_path_buf(),
        manager: PackageManager::Cargo,
        dependencies,
    })
}

/// Parse a TOML dependency table into DependencyEntry items.
fn parse_dependency_table(
    table: &toml::map::Map<String, toml::Value>,
    kind: DependencyKind,
    target: Option<String>,
    lockfile_versions: &HashMap<String, String>,
    out: &mut Vec<DependencyEntry>,
) {
    for (name, value) in table {
        let (version_req, optional, features) = match value {
            // Simple form: dep = "1.0"
            toml::Value::String(v) => (v.clone(), false, Vec::new()),
            // Table form: dep = { version = "1.0", features = [...], optional = true }
            toml::Value::Table(t) => {
                let version = t
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("*")
                    .to_string();
                let optional = t
                    .get("optional")
                    .and_then(|o| o.as_bool())
                    .unwrap_or(false);
                let features = t
                    .get("features")
                    .and_then(|f| f.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                (version, optional, features)
            }
            _ => continue,
        };

        let actual_version = lockfile_versions.get(name).cloned();
        let dep_kind = if optional { DependencyKind::Optional } else { kind };

        out.push(DependencyEntry {
            name: name.clone(),
            kind: dep_kind,
            version_req,
            actual_version,
            target: target.clone(),
            optional,
            features,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_empty_dir_returns_empty_report() {
        let dir = TempDir::new().unwrap();
        let result = detect_dependencies(dir.path()).unwrap();
        assert!(result.detected_managers.is_empty());
        assert!(result.manifests.is_empty());
    }

    #[test]
    fn test_package_manager_languages() {
        assert_eq!(PackageManager::Cargo.primary_language(), "Rust");
        assert_eq!(PackageManager::Npm.primary_language(), "JavaScript");
        assert_eq!(PackageManager::Poetry.primary_language(), "Python");
    }

    #[test]
    fn test_package_manager_registries() {
        assert_eq!(PackageManager::Cargo.registry_url(), Some("https://crates.io"));
        assert_eq!(PackageManager::Npm.registry_url(), Some("https://registry.npmjs.org"));
    }

    #[test]
    fn test_detects_cargo() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        let result = detect_dependencies(dir.path()).unwrap();
        assert!(result.detected_managers.contains(&PackageManager::Cargo));
        assert_eq!(result.manifests.len(), 1);
        assert_eq!(result.manifests[0].path, PathBuf::from("Cargo.toml"));
    }

    #[test]
    fn test_detects_pnpm() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "lockfileVersion: 9").unwrap();

        let result = detect_dependencies(dir.path()).unwrap();
        assert!(result.detected_managers.contains(&PackageManager::Pnpm));
    }

    #[test]
    fn test_scans_nested_directories() {
        let dir = TempDir::new().unwrap();
        // Root Cargo.toml
        fs::write(dir.path().join("Cargo.toml"), "[workspace]").unwrap();
        // Nested package
        fs::create_dir_all(dir.path().join("packages/foo")).unwrap();
        fs::write(
            dir.path().join("packages/foo/Cargo.toml"),
            "[package]\nname = \"foo\"",
        )
        .unwrap();
        // Another nested package
        fs::create_dir_all(dir.path().join("packages/bar")).unwrap();
        fs::write(
            dir.path().join("packages/bar/Cargo.toml"),
            "[package]\nname = \"bar\"",
        )
        .unwrap();

        let result = detect_dependencies(dir.path()).unwrap();
        // Should find all 3 Cargo.toml files
        assert_eq!(result.manifests.len(), 3);
        // But only one detected manager type
        assert_eq!(result.detected_managers.len(), 1);
        assert!(result.detected_managers.contains(&PackageManager::Cargo));
    }

    #[test]
    fn test_excludes_node_modules() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::create_dir_all(dir.path().join("node_modules/some-pkg")).unwrap();
        fs::write(
            dir.path().join("node_modules/some-pkg/package.json"),
            "{}",
        )
        .unwrap();

        let result = detect_dependencies(dir.path()).unwrap();
        // Should only find the root package.json, not the one in node_modules
        assert_eq!(result.manifests.len(), 1);
        assert_eq!(result.manifests[0].path, PathBuf::from("package.json"));
    }

    #[test]
    fn test_detects_multiple_manager_types() {
        let dir = TempDir::new().unwrap();
        // Rust
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        // JavaScript
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        // Python
        fs::write(dir.path().join("requirements.txt"), "requests").unwrap();

        let result = detect_dependencies(dir.path()).unwrap();
        assert_eq!(result.detected_managers.len(), 3);
        assert!(result.detected_managers.contains(&PackageManager::Cargo));
        assert!(result.detected_managers.contains(&PackageManager::Npm));
        assert!(result.detected_managers.contains(&PackageManager::Pip));
    }

    // Regression tests for dependency parsing bug fix:
    // Previously, dependencies only showed manifest locations (paths to Cargo.toml),
    // not the actual dependency information (name, version, type, etc.)

    #[test]
    fn test_parses_cargo_dependencies() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[package]
name = "test-pkg"

[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }
"#,
        )
        .unwrap();

        let result = detect_dependencies(dir.path()).unwrap();

        // Should have one package with dependencies
        assert_eq!(result.packages.len(), 1);
        let pkg = &result.packages[0];
        assert_eq!(pkg.name, Some("test-pkg".to_string()));
        assert_eq!(pkg.dependencies.len(), 2);

        // Check serde dependency
        let serde = pkg.dependencies.iter().find(|d| d.name == "serde").unwrap();
        assert_eq!(serde.kind, DependencyKind::Normal);
        assert_eq!(serde.version_req, "1.0");

        // Check tokio dependency with features
        let tokio = pkg.dependencies.iter().find(|d| d.name == "tokio").unwrap();
        assert_eq!(tokio.kind, DependencyKind::Normal);
        assert_eq!(tokio.version_req, "1.0");
        assert!(tokio.features.contains(&"full".to_string()));
    }

    #[test]
    fn test_parses_dev_dependencies() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[package]
name = "test-pkg"

[dependencies]
serde = "1.0"

[dev-dependencies]
tempfile = "3"
proptest = { version = "1.0" }
"#,
        )
        .unwrap();

        let result = detect_dependencies(dir.path()).unwrap();
        let pkg = &result.packages[0];

        // Check dev dependencies
        let tempfile = pkg.dependencies.iter().find(|d| d.name == "tempfile").unwrap();
        assert_eq!(tempfile.kind, DependencyKind::Dev);
        assert_eq!(tempfile.version_req, "3");

        let proptest = pkg.dependencies.iter().find(|d| d.name == "proptest").unwrap();
        assert_eq!(proptest.kind, DependencyKind::Dev);
    }

    #[test]
    fn test_parses_build_dependencies() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[package]
name = "test-pkg"

[build-dependencies]
cc = "1.0"
"#,
        )
        .unwrap();

        let result = detect_dependencies(dir.path()).unwrap();
        let pkg = &result.packages[0];

        let cc = pkg.dependencies.iter().find(|d| d.name == "cc").unwrap();
        assert_eq!(cc.kind, DependencyKind::Build);
        assert_eq!(cc.version_req, "1.0");
    }

    #[test]
    fn test_parses_target_specific_dependencies() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[package]
name = "test-pkg"

[target.'cfg(target_os = "macos")'.dependencies]
metal = "0.33"

[target.'cfg(windows)'.dependencies]
winapi = "0.3"
"#,
        )
        .unwrap();

        let result = detect_dependencies(dir.path()).unwrap();
        let pkg = &result.packages[0];

        // Check metal (macOS)
        let metal = pkg.dependencies.iter().find(|d| d.name == "metal").unwrap();
        assert_eq!(metal.kind, DependencyKind::Target);
        assert_eq!(metal.target, Some("cfg(target_os = \"macos\")".to_string()));
        assert_eq!(metal.version_req, "0.33");

        // Check winapi (Windows)
        let winapi = pkg.dependencies.iter().find(|d| d.name == "winapi").unwrap();
        assert_eq!(winapi.kind, DependencyKind::Target);
        assert_eq!(winapi.target, Some("cfg(windows)".to_string()));
    }

    #[test]
    fn test_parses_optional_dependencies() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[package]
name = "test-pkg"

[dependencies]
serde = { version = "1.0", optional = true }
normal_dep = "1.0"
"#,
        )
        .unwrap();

        let result = detect_dependencies(dir.path()).unwrap();
        let pkg = &result.packages[0];

        // Check optional dependency
        let serde = pkg.dependencies.iter().find(|d| d.name == "serde").unwrap();
        assert_eq!(serde.kind, DependencyKind::Optional);
        assert!(serde.optional);

        // Check normal dependency
        let normal = pkg.dependencies.iter().find(|d| d.name == "normal_dep").unwrap();
        assert_eq!(normal.kind, DependencyKind::Normal);
        assert!(!normal.optional);
    }

    #[test]
    fn test_resolves_versions_from_lockfile() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[package]
name = "test-pkg"

[dependencies]
serde = "1.0"
"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("Cargo.lock"),
            r#"
# This file is automatically @generated by Cargo.
version = 4

[[package]]
name = "serde"
version = "1.0.228"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "test-pkg"
version = "0.1.0"
"#,
        )
        .unwrap();

        let result = detect_dependencies(dir.path()).unwrap();
        let pkg = &result.packages[0];

        let serde = pkg.dependencies.iter().find(|d| d.name == "serde").unwrap();
        assert_eq!(serde.version_req, "1.0");
        assert_eq!(serde.actual_version, Some("1.0.228".to_string()));
    }

    #[test]
    fn test_monorepo_parses_all_package_dependencies() {
        let dir = TempDir::new().unwrap();

        // Root workspace
        fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = [\"pkg-a\", \"pkg-b\"]").unwrap();

        // Package A
        fs::create_dir(dir.path().join("pkg-a")).unwrap();
        fs::write(
            dir.path().join("pkg-a/Cargo.toml"),
            r#"
[package]
name = "pkg-a"

[dependencies]
serde = "1.0"
"#,
        )
        .unwrap();

        // Package B
        fs::create_dir(dir.path().join("pkg-b")).unwrap();
        fs::write(
            dir.path().join("pkg-b/Cargo.toml"),
            r#"
[package]
name = "pkg-b"

[dependencies]
tokio = "1.0"
"#,
        )
        .unwrap();

        let result = detect_dependencies(dir.path()).unwrap();

        // Should have 3 packages (workspace root + 2 member packages)
        assert_eq!(result.packages.len(), 3);

        // Find pkg-a and check its dependencies
        let pkg_a = result.packages.iter().find(|p| p.name == Some("pkg-a".to_string())).unwrap();
        assert!(pkg_a.dependencies.iter().any(|d| d.name == "serde"));

        // Find pkg-b and check its dependencies
        let pkg_b = result.packages.iter().find(|p| p.name == Some("pkg-b".to_string())).unwrap();
        assert!(pkg_b.dependencies.iter().any(|d| d.name == "tokio"));
    }

    #[test]
    fn test_dependencies_are_sorted_by_name() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[package]
name = "test-pkg"

[dependencies]
zlib = "1.0"
aaa = "1.0"
middle = "1.0"
"#,
        )
        .unwrap();

        let result = detect_dependencies(dir.path()).unwrap();
        let pkg = &result.packages[0];

        let names: Vec<&str> = pkg.dependencies.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, vec!["aaa", "middle", "zlib"]);
    }

    #[test]
    fn test_workspace_without_package_section() {
        // Workspace-only Cargo.toml (no [package] section, common for root of monorepo)
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[workspace]
members = ["pkg"]
"#,
        )
        .unwrap();

        let result = detect_dependencies(dir.path()).unwrap();
        let pkg = &result.packages[0];

        // Package name should be None for workspace-only manifest
        assert_eq!(pkg.name, None);
        assert!(pkg.dependencies.is_empty());
    }

    #[test]
    fn test_dependency_with_all_fields() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[package]
name = "test-pkg"

[dependencies]
complex = { version = "1.0", features = ["a", "b"], optional = true }
"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("Cargo.lock"),
            r#"
version = 4

[[package]]
name = "complex"
version = "1.0.5"
"#,
        )
        .unwrap();

        let result = detect_dependencies(dir.path()).unwrap();
        let pkg = &result.packages[0];
        let dep = &pkg.dependencies[0];

        assert_eq!(dep.name, "complex");
        assert_eq!(dep.kind, DependencyKind::Optional);
        assert_eq!(dep.version_req, "1.0");
        assert_eq!(dep.actual_version, Some("1.0.5".to_string()));
        assert!(dep.optional);
        assert_eq!(dep.features, vec!["a", "b"]);
    }
}
