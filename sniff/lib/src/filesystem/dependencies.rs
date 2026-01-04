use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyReport {
    /// Package managers detected (deduplicated list)
    pub detected_managers: Vec<PackageManager>,
    /// All manifest file locations found in the directory tree
    pub manifests: Vec<ManifestLocation>,
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
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    return !EXCLUDED_DIRS.contains(&name);
                }
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

    Ok(DependencyReport {
        detected_managers,
        manifests,
    })
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
}
