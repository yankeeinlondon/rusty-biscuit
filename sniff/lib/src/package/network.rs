//! Network-based package registry lookups.
//!
//! This module provides implementations of [`PackageManagerShape`] that query
//! package registries over HTTP to fetch version information.
//!
//! ## Feature Flag
//!
//! This module is only available when the `network` feature is enabled.
//!
//! ## Supported Registries
//!
//! - **crates.io** - Rust packages via Cargo
//! - **npm registry** - JavaScript packages via npm/pnpm/yarn/bun

#[cfg(feature = "network")]
use reqwest::Client;
use serde::Deserialize;

use super::{BoxFuture, LanguagePackageManager, PackageManagerShape};
use crate::os::{command_exists_in_path, get_path_dirs};
use crate::package::stubs::PackageInfo;
use crate::Result;

// ============================================================================
// crates.io API Types
// ============================================================================

/// Response from crates.io API for a single crate.
#[derive(Debug, Deserialize)]
struct CratesIoResponse {
    #[serde(rename = "crate")]
    crate_info: CratesIoCrate,
}

/// Crate information from crates.io.
#[derive(Debug, Deserialize)]
struct CratesIoCrate {
    name: String,
    max_version: String,
    description: Option<String>,
}

// ============================================================================
// npm Registry API Types
// ============================================================================

/// Response from npm registry API for a package.
#[derive(Debug, Deserialize)]
struct NpmRegistryResponse {
    name: String,
    description: Option<String>,
    #[serde(rename = "dist-tags")]
    dist_tags: Option<NpmDistTags>,
}

/// Distribution tags from npm registry.
#[derive(Debug, Deserialize)]
struct NpmDistTags {
    latest: Option<String>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Checks if an executable exists in the system PATH.
fn executable_exists(name: &str) -> bool {
    let path_dirs = get_path_dirs();
    command_exists_in_path(name, &path_dirs).is_some()
}

// ============================================================================
// Cargo Implementation (crates.io)
// ============================================================================

/// Network-enabled implementation for Cargo using crates.io API.
///
/// Queries the crates.io API to fetch package information and latest versions.
///
/// ## API Endpoint
///
/// `https://crates.io/api/v1/crates/{name}`
#[derive(Debug, Clone, Default)]
pub struct CargoNetwork {
    #[cfg(feature = "network")]
    client: Option<Client>,
}

impl CargoNetwork {
    /// Creates a new CargoNetwork instance.
    #[cfg(feature = "network")]
    pub fn new() -> Self {
        Self {
            client: Some(
                Client::builder()
                    .user_agent("sniff-lib (https://github.com/anthropics/dockhand)")
                    .build()
                    .ok(),
            )
            .flatten(),
        }
    }

    #[cfg(not(feature = "network"))]
    pub fn new() -> Self {
        Self {}
    }
}

impl PackageManagerShape for CargoNetwork {
    fn executable_name(&self) -> &'static str {
        LanguagePackageManager::Cargo.executable_name()
    }

    fn is_available(&self) -> bool {
        executable_exists(self.executable_name())
    }

    fn find_package(&self, name: &str) -> BoxFuture<'_, Result<Option<PackageInfo>>> {
        #[cfg(feature = "network")]
        {
            let name = name.to_string();
            Box::pin(async move {
                let Some(ref client) = self.client else {
                    return Ok(None);
                };

                let url = format!("https://crates.io/api/v1/crates/{name}");
                let response = client.get(&url).send().await;

                match response {
                    Ok(resp) if resp.status().is_success() => {
                        match resp.json::<CratesIoResponse>().await {
                            Ok(data) => Ok(Some(PackageInfo {
                                name: data.crate_info.name,
                                version: data.crate_info.max_version,
                                description: data.crate_info.description,
                            })),
                            Err(_) => Ok(None),
                        }
                    }
                    Ok(_) => Ok(None), // Non-success status (404, etc.)
                    Err(_) => Ok(None), // Network error - graceful degradation
                }
            })
        }

        #[cfg(not(feature = "network"))]
        {
            let _ = name;
            Box::pin(async { Ok(None) })
        }
    }

    fn latest_version(&self, name: &str) -> BoxFuture<'_, Result<Option<String>>> {
        #[cfg(feature = "network")]
        {
            let name = name.to_string();
            Box::pin(async move {
                let Some(ref client) = self.client else {
                    return Ok(None);
                };

                let url = format!("https://crates.io/api/v1/crates/{name}");
                let response = client.get(&url).send().await;

                match response {
                    Ok(resp) if resp.status().is_success() => {
                        match resp.json::<CratesIoResponse>().await {
                            Ok(data) => Ok(Some(data.crate_info.max_version)),
                            Err(_) => Ok(None),
                        }
                    }
                    Ok(_) => Ok(None),
                    Err(_) => Ok(None),
                }
            })
        }

        #[cfg(not(feature = "network"))]
        {
            let _ = name;
            Box::pin(async { Ok(None) })
        }
    }
}

// ============================================================================
// npm Implementation (npm registry)
// ============================================================================

/// Network-enabled implementation for npm using npm registry API.
///
/// Queries the npm registry API to fetch package information and latest versions.
/// This implementation works for npm, pnpm, yarn, and bun since they all use
/// the same registry.
///
/// ## API Endpoint
///
/// `https://registry.npmjs.org/{name}`
#[derive(Debug, Clone, Default)]
pub struct NpmNetwork {
    #[cfg(feature = "network")]
    client: Option<Client>,
}

impl NpmNetwork {
    /// Creates a new NpmNetwork instance.
    #[cfg(feature = "network")]
    pub fn new() -> Self {
        Self {
            client: Some(
                Client::builder()
                    .user_agent("sniff-lib (https://github.com/anthropics/dockhand)")
                    .build()
                    .ok(),
            )
            .flatten(),
        }
    }

    #[cfg(not(feature = "network"))]
    pub fn new() -> Self {
        Self {}
    }
}

impl PackageManagerShape for NpmNetwork {
    fn executable_name(&self) -> &'static str {
        LanguagePackageManager::Npm.executable_name()
    }

    fn is_available(&self) -> bool {
        executable_exists(self.executable_name())
    }

    fn find_package(&self, name: &str) -> BoxFuture<'_, Result<Option<PackageInfo>>> {
        #[cfg(feature = "network")]
        {
            let name = name.to_string();
            Box::pin(async move {
                let Some(ref client) = self.client else {
                    return Ok(None);
                };

                let url = format!("https://registry.npmjs.org/{name}");
                let response = client.get(&url).send().await;

                match response {
                    Ok(resp) if resp.status().is_success() => {
                        match resp.json::<NpmRegistryResponse>().await {
                            Ok(data) => {
                                let version = data
                                    .dist_tags
                                    .and_then(|dt| dt.latest)
                                    .unwrap_or_else(|| "unknown".to_string());
                                Ok(Some(PackageInfo {
                                    name: data.name,
                                    version,
                                    description: data.description,
                                }))
                            }
                            Err(_) => Ok(None),
                        }
                    }
                    Ok(_) => Ok(None),
                    Err(_) => Ok(None),
                }
            })
        }

        #[cfg(not(feature = "network"))]
        {
            let _ = name;
            Box::pin(async { Ok(None) })
        }
    }

    fn latest_version(&self, name: &str) -> BoxFuture<'_, Result<Option<String>>> {
        #[cfg(feature = "network")]
        {
            let name = name.to_string();
            Box::pin(async move {
                let Some(ref client) = self.client else {
                    return Ok(None);
                };

                let url = format!("https://registry.npmjs.org/{name}");
                let response = client.get(&url).send().await;

                match response {
                    Ok(resp) if resp.status().is_success() => {
                        match resp.json::<NpmRegistryResponse>().await {
                            Ok(data) => Ok(data.dist_tags.and_then(|dt| dt.latest)),
                            Err(_) => Ok(None),
                        }
                    }
                    Ok(_) => Ok(None),
                    Err(_) => Ok(None),
                }
            })
        }

        #[cfg(not(feature = "network"))]
        {
            let _ = name;
            Box::pin(async { Ok(None) })
        }
    }
}

// ============================================================================
// Shared npm-based implementation for pnpm/yarn/bun
// ============================================================================

/// Network-enabled implementation for pnpm using npm registry API.
#[derive(Debug, Clone, Default)]
pub struct PnpmNetwork(NpmNetwork);

impl PnpmNetwork {
    pub fn new() -> Self {
        Self(NpmNetwork::new())
    }
}

impl PackageManagerShape for PnpmNetwork {
    fn executable_name(&self) -> &'static str {
        LanguagePackageManager::Pnpm.executable_name()
    }

    fn is_available(&self) -> bool {
        executable_exists(self.executable_name())
    }

    fn find_package(&self, name: &str) -> BoxFuture<'_, Result<Option<PackageInfo>>> {
        self.0.find_package(name)
    }

    fn latest_version(&self, name: &str) -> BoxFuture<'_, Result<Option<String>>> {
        self.0.latest_version(name)
    }
}

/// Network-enabled implementation for Yarn using npm registry API.
#[derive(Debug, Clone, Default)]
pub struct YarnNetwork(NpmNetwork);

impl YarnNetwork {
    pub fn new() -> Self {
        Self(NpmNetwork::new())
    }
}

impl PackageManagerShape for YarnNetwork {
    fn executable_name(&self) -> &'static str {
        LanguagePackageManager::Yarn.executable_name()
    }

    fn is_available(&self) -> bool {
        executable_exists(self.executable_name())
    }

    fn find_package(&self, name: &str) -> BoxFuture<'_, Result<Option<PackageInfo>>> {
        self.0.find_package(name)
    }

    fn latest_version(&self, name: &str) -> BoxFuture<'_, Result<Option<String>>> {
        self.0.latest_version(name)
    }
}

/// Network-enabled implementation for Bun using npm registry API.
#[derive(Debug, Clone, Default)]
pub struct BunNetwork(NpmNetwork);

impl BunNetwork {
    pub fn new() -> Self {
        Self(NpmNetwork::new())
    }
}

impl PackageManagerShape for BunNetwork {
    fn executable_name(&self) -> &'static str {
        LanguagePackageManager::Bun.executable_name()
    }

    fn is_available(&self) -> bool {
        executable_exists(self.executable_name())
    }

    fn find_package(&self, name: &str) -> BoxFuture<'_, Result<Option<PackageInfo>>> {
        self.0.find_package(name)
    }

    fn latest_version(&self, name: &str) -> BoxFuture<'_, Result<Option<String>>> {
        self.0.latest_version(name)
    }
}

// ============================================================================
// Dependency Enrichment
// ============================================================================

use crate::filesystem::repo::DependencyEntry;

/// Enriches a dependency entry with the latest version from its registry.
///
/// This function queries the appropriate package registry based on the
/// `package_manager` field to fetch the latest available version.
///
/// ## Network Failure Handling
///
/// On network failure:
/// - The function logs at DEBUG level (if tracing is enabled)
/// - Returns the entry unchanged (latest_version remains None)
/// - Does NOT fail the operation
///
/// ## Supported Package Managers
///
/// - `cargo` - queries crates.io
/// - `npm`, `pnpm`, `yarn`, `bun` - queries npm registry
#[cfg(feature = "network")]
pub async fn enrich_dependency(mut entry: DependencyEntry) -> DependencyEntry {
    let Some(ref manager) = entry.package_manager else {
        return entry;
    };

    let latest = match manager.as_str() {
        "cargo" => {
            let cargo = CargoNetwork::new();
            cargo.latest_version(&entry.name).await.ok().flatten()
        }
        "npm" | "pnpm" | "yarn" | "bun" => {
            let npm = NpmNetwork::new();
            npm.latest_version(&entry.name).await.ok().flatten()
        }
        _ => None,
    };

    entry.latest_version = latest;
    entry
}

/// Enriches a list of dependency entries with latest versions in parallel.
///
/// ## Examples
///
/// ```ignore
/// use sniff_lib::package::enrich_dependencies;
///
/// let deps = vec![/* ... */];
/// let enriched = enrich_dependencies(deps).await;
/// ```
#[cfg(feature = "network")]
pub async fn enrich_dependencies(entries: Vec<DependencyEntry>) -> Vec<DependencyEntry> {
    use futures::future::join_all;

    let futures = entries.into_iter().map(enrich_dependency);
    join_all(futures).await
}

/// Stub for when network feature is disabled.
#[cfg(not(feature = "network"))]
pub async fn enrich_dependency(entry: DependencyEntry) -> DependencyEntry {
    entry
}

/// Stub for when network feature is disabled.
#[cfg(not(feature = "network"))]
pub async fn enrich_dependencies(entries: Vec<DependencyEntry>) -> Vec<DependencyEntry> {
    entries
}

#[cfg(all(test, feature = "network"))]
mod tests {
    use super::*;
    use crate::filesystem::repo::DependencyKind;

    #[tokio::test]
    async fn test_enrich_dependency_cargo() {
        let entry = DependencyEntry {
            name: "serde".to_string(),
            kind: DependencyKind::Normal,
            targeted_version: "1.0".to_string(),
            actual_version: None,
            package_manager: Some("cargo".to_string()),
            latest_version: None,
            target: None,
            optional: false,
            features: vec![],
        };

        let enriched = enrich_dependency(entry).await;
        assert!(enriched.latest_version.is_some());
        assert!(enriched.latest_version.unwrap().starts_with('1'));
    }

    #[tokio::test]
    async fn test_enrich_dependency_npm() {
        let entry = DependencyEntry {
            name: "lodash".to_string(),
            kind: DependencyKind::Normal,
            targeted_version: "^4.0.0".to_string(),
            actual_version: None,
            package_manager: Some("npm".to_string()),
            latest_version: None,
            target: None,
            optional: false,
            features: vec![],
        };

        let enriched = enrich_dependency(entry).await;
        assert!(enriched.latest_version.is_some());
        assert!(enriched.latest_version.unwrap().starts_with('4'));
    }

    #[tokio::test]
    async fn test_enrich_dependency_no_manager() {
        let entry = DependencyEntry {
            name: "unknown".to_string(),
            kind: DependencyKind::Normal,
            targeted_version: "1.0".to_string(),
            actual_version: None,
            package_manager: None,
            latest_version: None,
            target: None,
            optional: false,
            features: vec![],
        };

        let enriched = enrich_dependency(entry).await;
        assert!(enriched.latest_version.is_none());
    }

    #[tokio::test]
    async fn test_cargo_network_find_package() {
        let cargo = CargoNetwork::new();
        let result = cargo.find_package("serde").await;
        assert!(result.is_ok());
        if let Ok(Some(info)) = result {
            assert_eq!(info.name, "serde");
            assert!(!info.version.is_empty());
        }
    }

    #[tokio::test]
    async fn test_cargo_network_latest_version() {
        let cargo = CargoNetwork::new();
        let result = cargo.latest_version("serde").await;
        assert!(result.is_ok());
        if let Ok(Some(version)) = result {
            // Version should look like "1.x.y"
            assert!(version.starts_with('1'));
        }
    }

    #[tokio::test]
    async fn test_cargo_network_nonexistent_package() {
        let cargo = CargoNetwork::new();
        let result = cargo.find_package("this-crate-definitely-does-not-exist-xyz123").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_npm_network_find_package() {
        let npm = NpmNetwork::new();
        let result = npm.find_package("lodash").await;
        assert!(result.is_ok());
        if let Ok(Some(info)) = result {
            assert_eq!(info.name, "lodash");
            assert!(!info.version.is_empty());
        }
    }

    #[tokio::test]
    async fn test_npm_network_latest_version() {
        let npm = NpmNetwork::new();
        let result = npm.latest_version("lodash").await;
        assert!(result.is_ok());
        if let Ok(Some(version)) = result {
            // Lodash version should look like "4.x.y"
            assert!(version.starts_with('4'));
        }
    }

    #[tokio::test]
    async fn test_npm_network_nonexistent_package() {
        let npm = NpmNetwork::new();
        let result = npm.find_package("this-package-definitely-does-not-exist-xyz123").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
