//! Dependency entry types shared across filesystem and package modules.
//!
//! This module provides [`DependencyEntry`] and [`DependencyKind`] as a neutral
//! leaf so that both `filesystem/repo` and `package/network` can import them
//! without creating a layer inversion.

use serde::{Deserialize, Serialize};

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
