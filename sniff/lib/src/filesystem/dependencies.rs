use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::Result;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyReport {
    pub detected_managers: Vec<PackageManager>,
    pub packages: Vec<()>, // Stub: empty for v1
}

/// Detect package managers present in a directory
///
/// This is a stub implementation for v1. Full dependency analysis
/// will be implemented in v2.
pub fn detect_dependencies(root: &Path) -> Result<DependencyReport> {
    let mut managers = Vec::new();

    // Check for various package manager files
    if root.join("Cargo.toml").exists() {
        managers.push(PackageManager::Cargo);
    }
    if root.join("package.json").exists() {
        // Check which JS package manager
        if root.join("pnpm-lock.yaml").exists() {
            managers.push(PackageManager::Pnpm);
        } else if root.join("yarn.lock").exists() {
            managers.push(PackageManager::Yarn);
        } else if root.join("bun.lockb").exists() {
            managers.push(PackageManager::Bun);
        } else {
            managers.push(PackageManager::Npm);
        }
    }
    if root.join("pyproject.toml").exists() {
        if root.join("poetry.lock").exists() {
            managers.push(PackageManager::Poetry);
        } else if root.join("pdm.lock").exists() {
            managers.push(PackageManager::Pdm);
        } else if root.join("uv.lock").exists() {
            managers.push(PackageManager::Uv);
        } else {
            managers.push(PackageManager::Pip);
        }
    } else if root.join("requirements.txt").exists() {
        managers.push(PackageManager::Pip);
    }
    if root.join("Gemfile").exists() {
        managers.push(PackageManager::Bundler);
    }
    if root.join("composer.json").exists() {
        managers.push(PackageManager::Composer);
    }
    if root.join("pom.xml").exists() {
        managers.push(PackageManager::Maven);
    }
    if root.join("build.gradle").exists() || root.join("build.gradle.kts").exists() {
        managers.push(PackageManager::Gradle);
    }
    if root.join("go.mod").exists() {
        managers.push(PackageManager::GoMod);
    }

    Ok(DependencyReport {
        detected_managers: managers,
        packages: vec![],
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
        assert!(result.packages.is_empty());
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
    }

    #[test]
    fn test_detects_pnpm() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "lockfileVersion: 9").unwrap();

        let result = detect_dependencies(dir.path()).unwrap();
        assert!(result.detected_managers.contains(&PackageManager::Pnpm));
    }
}
