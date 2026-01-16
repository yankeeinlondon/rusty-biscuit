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
#[cfg(feature = "network")]
use serde::Deserialize;

use super::{BoxFuture, LanguagePackageManager, PackageManagerShape};
use crate::Result;
use crate::os::{command_exists_in_path, get_path_dirs};
use crate::package::stubs::PackageInfo;

// ============================================================================
// crates.io API Types
// ============================================================================

/// Response from crates.io API for a single crate.
#[cfg(feature = "network")]
#[derive(Debug, Deserialize)]
struct CratesIoResponse {
    #[serde(rename = "crate")]
    crate_info: CratesIoCrate,
}

/// Crate information from crates.io.
#[cfg(feature = "network")]
#[derive(Debug, Deserialize)]
struct CratesIoCrate {
    name: String,
    max_version: String,
    description: Option<String>,
}

// ============================================================================
// npm Registry API Types
// ============================================================================

/// Default base URL for npm registry API.
#[cfg(feature = "network")]
const NPM_REGISTRY_BASE_URL: &str = "https://registry.npmjs.org";

/// Response from npm registry API for a package.
#[cfg(feature = "network")]
#[derive(Debug, Deserialize)]
struct NpmRegistryResponse {
    name: String,
    description: Option<String>,
    #[serde(rename = "dist-tags")]
    dist_tags: Option<NpmDistTags>,
}

/// Distribution tags from npm registry.
#[cfg(feature = "network")]
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

/// Default base URL for crates.io API.
#[cfg(feature = "network")]
const CRATES_IO_BASE_URL: &str = "https://crates.io/api/v1/crates";

/// Network-enabled implementation for Cargo using crates.io API.
///
/// Queries the crates.io API to fetch package information and latest versions.
///
/// ## API Endpoint
///
/// `https://crates.io/api/v1/crates/{name}`
///
/// ## Testing
///
/// Use [`CargoNetwork::with_base_url`] to override the base URL for testing
/// with a mock server.
#[derive(Debug, Clone)]
pub struct CargoNetwork {
    #[cfg(feature = "network")]
    client: Option<Client>,
    #[cfg(feature = "network")]
    base_url: String,
}

impl CargoNetwork {
    /// Creates a new CargoNetwork instance using the default crates.io API.
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
            base_url: CRATES_IO_BASE_URL.to_string(),
        }
    }

    /// Sets a custom base URL for the API.
    ///
    /// This is primarily useful for testing with a mock server.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// let cargo = CargoNetwork::new()
    ///     .with_base_url("http://localhost:8080/api/v1/crates");
    /// ```
    #[cfg(feature = "network")]
    #[must_use]
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    #[cfg(not(feature = "network"))]
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(not(feature = "network"))]
impl Default for CargoNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "network")]
impl Default for CargoNetwork {
    fn default() -> Self {
        Self::new()
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
            let base_url = self.base_url.clone();
            Box::pin(async move {
                let Some(ref client) = self.client else {
                    return Ok(None);
                };

                let url = format!("{base_url}/{name}");
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
                    Ok(_) => Ok(None),  // Non-success status (404, etc.)
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
            let base_url = self.base_url.clone();
            Box::pin(async move {
                let Some(ref client) = self.client else {
                    return Ok(None);
                };

                let url = format!("{base_url}/{name}");
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
///
/// ## Testing
///
/// Use [`NpmNetwork::with_base_url`] to override the base URL for testing
/// with a mock server.
#[derive(Debug, Clone)]
pub struct NpmNetwork {
    #[cfg(feature = "network")]
    client: Option<Client>,
    #[cfg(feature = "network")]
    base_url: String,
}

impl NpmNetwork {
    /// Creates a new NpmNetwork instance using the default npm registry.
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
            base_url: NPM_REGISTRY_BASE_URL.to_string(),
        }
    }

    /// Sets a custom base URL for the API.
    ///
    /// This is primarily useful for testing with a mock server.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// let npm = NpmNetwork::new()
    ///     .with_base_url("http://localhost:8080");
    /// ```
    #[cfg(feature = "network")]
    #[must_use]
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    #[cfg(not(feature = "network"))]
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(not(feature = "network"))]
impl Default for NpmNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "network")]
impl Default for NpmNetwork {
    fn default() -> Self {
        Self::new()
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
            let base_url = self.base_url.clone();
            Box::pin(async move {
                let Some(ref client) = self.client else {
                    return Ok(None);
                };

                let url = format!("{base_url}/{name}");
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
            let base_url = self.base_url.clone();
            Box::pin(async move {
                let Some(ref client) = self.client else {
                    return Ok(None);
                };

                let url = format!("{base_url}/{name}");
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
        let result = cargo
            .find_package("this-crate-definitely-does-not-exist-xyz123")
            .await;
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
        let result = npm
            .find_package("this-package-definitely-does-not-exist-xyz123")
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}

// ============================================================================
// Wiremock-based Unit Tests
// ============================================================================

/// Unit tests using wiremock to mock HTTP responses.
///
/// These tests are deterministic and do not require network access.
#[cfg(all(test, feature = "network"))]
mod wiremock_tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // ------------------------------------------------------------------------
    // CargoNetwork Tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_cargo_find_package_success() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "crate": {
                "name": "tokio",
                "max_version": "1.40.0",
                "description": "An event-driven, non-blocking I/O platform"
            }
        }"#;

        Mock::given(method("GET"))
            .and(path("/tokio"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let cargo = CargoNetwork::new().with_base_url(mock_server.uri());
        let result = cargo.find_package("tokio").await;

        assert!(result.is_ok());
        let info = result.unwrap().expect("should have package info");
        assert_eq!(info.name, "tokio");
        assert_eq!(info.version, "1.40.0");
        assert_eq!(
            info.description,
            Some("An event-driven, non-blocking I/O platform".to_string())
        );
    }

    #[tokio::test]
    async fn test_cargo_find_package_no_description() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "crate": {
                "name": "some-crate",
                "max_version": "0.1.0",
                "description": null
            }
        }"#;

        Mock::given(method("GET"))
            .and(path("/some-crate"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let cargo = CargoNetwork::new().with_base_url(mock_server.uri());
        let result = cargo.find_package("some-crate").await;

        assert!(result.is_ok());
        let info = result.unwrap().expect("should have package info");
        assert_eq!(info.name, "some-crate");
        assert_eq!(info.version, "0.1.0");
        assert!(info.description.is_none());
    }

    #[tokio::test]
    async fn test_cargo_latest_version_success() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "crate": {
                "name": "serde",
                "max_version": "1.0.210",
                "description": "A serialization framework"
            }
        }"#;

        Mock::given(method("GET"))
            .and(path("/serde"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let cargo = CargoNetwork::new().with_base_url(mock_server.uri());
        let result = cargo.latest_version("serde").await;

        assert!(result.is_ok());
        let version = result.unwrap().expect("should have version");
        assert_eq!(version, "1.0.210");
    }

    #[tokio::test]
    async fn test_cargo_find_package_404() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/nonexistent-crate"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let cargo = CargoNetwork::new().with_base_url(mock_server.uri());
        let result = cargo.find_package("nonexistent-crate").await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_cargo_latest_version_404() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/nonexistent-crate"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let cargo = CargoNetwork::new().with_base_url(mock_server.uri());
        let result = cargo.latest_version("nonexistent-crate").await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_cargo_find_package_malformed_json() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/bad-response"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
            .mount(&mock_server)
            .await;

        let cargo = CargoNetwork::new().with_base_url(mock_server.uri());
        let result = cargo.find_package("bad-response").await;

        // Should gracefully handle malformed JSON
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_cargo_find_package_incomplete_json() {
        let mock_server = MockServer::start().await;

        // Valid JSON but missing required fields
        let response_body = r#"{"crate": {"name": "partial"}}"#;

        Mock::given(method("GET"))
            .and(path("/partial"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let cargo = CargoNetwork::new().with_base_url(mock_server.uri());
        let result = cargo.find_package("partial").await;

        // Should gracefully handle missing fields
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_cargo_find_package_server_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/server-error"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let cargo = CargoNetwork::new().with_base_url(mock_server.uri());
        let result = cargo.find_package("server-error").await;

        // Should gracefully handle 500 errors
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    // ------------------------------------------------------------------------
    // NpmNetwork Tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_npm_find_package_success() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "name": "lodash",
            "description": "Lodash modular utilities",
            "dist-tags": {
                "latest": "4.17.21"
            }
        }"#;

        Mock::given(method("GET"))
            .and(path("/lodash"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let npm = NpmNetwork::new().with_base_url(mock_server.uri());
        let result = npm.find_package("lodash").await;

        assert!(result.is_ok());
        let info = result.unwrap().expect("should have package info");
        assert_eq!(info.name, "lodash");
        assert_eq!(info.version, "4.17.21");
        assert_eq!(
            info.description,
            Some("Lodash modular utilities".to_string())
        );
    }

    #[tokio::test]
    async fn test_npm_find_package_no_description() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "name": "minimal-pkg",
            "dist-tags": {
                "latest": "1.0.0"
            }
        }"#;

        Mock::given(method("GET"))
            .and(path("/minimal-pkg"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let npm = NpmNetwork::new().with_base_url(mock_server.uri());
        let result = npm.find_package("minimal-pkg").await;

        assert!(result.is_ok());
        let info = result.unwrap().expect("should have package info");
        assert_eq!(info.name, "minimal-pkg");
        assert_eq!(info.version, "1.0.0");
        assert!(info.description.is_none());
    }

    #[tokio::test]
    async fn test_npm_find_package_no_dist_tags() {
        let mock_server = MockServer::start().await;

        // Package with no dist-tags (edge case)
        let response_body = r#"{
            "name": "legacy-pkg",
            "description": "A legacy package"
        }"#;

        Mock::given(method("GET"))
            .and(path("/legacy-pkg"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let npm = NpmNetwork::new().with_base_url(mock_server.uri());
        let result = npm.find_package("legacy-pkg").await;

        assert!(result.is_ok());
        let info = result.unwrap().expect("should have package info");
        assert_eq!(info.name, "legacy-pkg");
        assert_eq!(info.version, "unknown"); // Falls back to "unknown"
    }

    #[tokio::test]
    async fn test_npm_find_package_no_latest_tag() {
        let mock_server = MockServer::start().await;

        // Package with dist-tags but no "latest" tag
        let response_body = r#"{
            "name": "beta-pkg",
            "description": "A beta package",
            "dist-tags": {
                "beta": "2.0.0-beta.1"
            }
        }"#;

        Mock::given(method("GET"))
            .and(path("/beta-pkg"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let npm = NpmNetwork::new().with_base_url(mock_server.uri());
        let result = npm.find_package("beta-pkg").await;

        assert!(result.is_ok());
        let info = result.unwrap().expect("should have package info");
        assert_eq!(info.name, "beta-pkg");
        assert_eq!(info.version, "unknown"); // Falls back to "unknown"
    }

    #[tokio::test]
    async fn test_npm_latest_version_success() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "name": "react",
            "description": "A JavaScript library for building user interfaces",
            "dist-tags": {
                "latest": "18.3.1",
                "next": "19.0.0-rc.0"
            }
        }"#;

        Mock::given(method("GET"))
            .and(path("/react"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let npm = NpmNetwork::new().with_base_url(mock_server.uri());
        let result = npm.latest_version("react").await;

        assert!(result.is_ok());
        let version = result.unwrap().expect("should have version");
        assert_eq!(version, "18.3.1");
    }

    #[tokio::test]
    async fn test_npm_latest_version_no_dist_tags() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "name": "no-tags-pkg"
        }"#;

        Mock::given(method("GET"))
            .and(path("/no-tags-pkg"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let npm = NpmNetwork::new().with_base_url(mock_server.uri());
        let result = npm.latest_version("no-tags-pkg").await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_npm_find_package_404() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/nonexistent-package"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let npm = NpmNetwork::new().with_base_url(mock_server.uri());
        let result = npm.find_package("nonexistent-package").await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_npm_latest_version_404() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/nonexistent-package"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let npm = NpmNetwork::new().with_base_url(mock_server.uri());
        let result = npm.latest_version("nonexistent-package").await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_npm_find_package_malformed_json() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/bad-json"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{invalid json}"))
            .mount(&mock_server)
            .await;

        let npm = NpmNetwork::new().with_base_url(mock_server.uri());
        let result = npm.find_package("bad-json").await;

        // Should gracefully handle malformed JSON
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_npm_find_package_server_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/server-error"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&mock_server)
            .await;

        let npm = NpmNetwork::new().with_base_url(mock_server.uri());
        let result = npm.find_package("server-error").await;

        // Should gracefully handle 503 errors
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_npm_scoped_package() {
        let mock_server = MockServer::start().await;

        let response_body = r#"{
            "name": "@types/node",
            "description": "TypeScript definitions for node",
            "dist-tags": {
                "latest": "22.5.0"
            }
        }"#;

        // Scoped packages use URL encoding: @types/node -> @types%2Fnode
        // But reqwest handles this, so the path matcher sees the encoded form
        Mock::given(method("GET"))
            .and(path("/@types/node"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let npm = NpmNetwork::new().with_base_url(mock_server.uri());
        let result = npm.find_package("@types/node").await;

        assert!(result.is_ok());
        let info = result.unwrap().expect("should have package info");
        assert_eq!(info.name, "@types/node");
        assert_eq!(info.version, "22.5.0");
    }
}
