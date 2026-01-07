//! Source aggregation and deduplication for changelog version history.
//!
//! This module combines version information from multiple sources (GitHub Releases,
//! package registries, changelog files) into a unified version history. It handles
//! deduplication, confidence calculation, and graceful degradation when sources fail.
//!
//! ## Aggregation Strategy
//!
//! 1. **Parallel Fetching**: All sources are queried concurrently using `tokio::join!`
//! 2. **Deduplication**: Versions with the same version string are merged
//! 3. **Source Precedence**: For conflicting data, ChangelogFile > GitHubRelease > RegistryVersion
//! 4. **Confidence Calculation**: Based on which sources successfully returned data
//!
//! ## Examples
//!
//! ```rust,no_run
//! use research_lib::changelog::aggregator::aggregate_version_history;
//! use reqwest::Client;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new();
//! let history = aggregate_version_history(
//!     &client,
//!     "tokio",
//!     "crates.io",
//!     "https://github.com/tokio-rs/tokio"
//! ).await?;
//!
//! println!("Latest version: {}", history.latest_version);
//! println!("Confidence: {:?}", history.confidence);
//! # Ok(())
//! # }
//! ```

use super::discovery::discover_changelog_file;
use super::github::fetch_github_releases;
use super::registry::fetch_registry_versions;
use super::types::{
    ChangelogError, ChangelogSource, ConfidenceLevel, VersionHistory, VersionInfo,
};
use reqwest::Client as HttpClient;
use std::collections::HashMap;

/// Aggregate changelog information from all available sources.
///
/// This function fetches version information from multiple sources in parallel,
/// deduplicates versions, calculates confidence, and returns a unified history.
///
/// ## Parameters
///
/// - `client`: HTTP client for making requests
/// - `library_name`: Name of the package/library (e.g., "tokio", "express")
/// - `package_manager`: Registry identifier (e.g., "crates.io", "npm", "PyPI")
/// - `repo_url`: GitHub repository URL for releases and changelog discovery
///
/// ## Returns
///
/// A `VersionHistory` containing:
/// - Deduplicated versions sorted newest to oldest
/// - Latest version string
/// - Sources that contributed data
/// - Confidence level based on available sources
///
/// ## Errors
///
/// Returns `ChangelogError::NoSources` if all sources fail to return data.
/// Individual source failures are logged but don't fail the entire operation
/// (graceful degradation).
///
/// ## Examples
///
/// ```rust,no_run
/// # use research_lib::changelog::aggregator::aggregate_version_history;
/// # use reqwest::Client;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new();
/// let history = aggregate_version_history(
///     &client,
///     "clap",
///     "crates.io",
///     "https://github.com/clap-rs/clap"
/// ).await?;
///
/// assert!(!history.versions.is_empty());
/// assert!(!history.latest_version.is_empty());
/// # Ok(())
/// # }
/// ```
pub async fn aggregate_version_history(
    client: &HttpClient,
    library_name: &str,
    package_manager: &str,
    repo_url: &str,
) -> Result<VersionHistory, ChangelogError> {
    // 1. Parallel fetch from all sources (up to 50 GitHub releases, 100 registry versions)
    let (github_result, registry_result, file_result) = tokio::join!(
        fetch_github_releases(client, repo_url, 50),
        fetch_registry_versions(client, package_manager, library_name, 100),
        discover_changelog_file(client, repo_url),
    );

    // Collect versions from all successful sources
    let mut all_versions: Vec<Vec<VersionInfo>> = Vec::new();
    let mut sources_used: Vec<ChangelogSource> = Vec::new();

    // GitHub Releases
    if let Ok(github_versions) = github_result
        && !github_versions.is_empty()
    {
        all_versions.push(github_versions);
        sources_used.push(ChangelogSource::GitHubRelease);
    }

    // Registry Versions
    if let Ok(registry_versions) = registry_result
        && !registry_versions.is_empty()
    {
        all_versions.push(registry_versions);
        sources_used.push(ChangelogSource::RegistryVersion);
    }

    // Changelog File
    if let Ok(Some(changelog_content)) = file_result {
        // Try parsing with different parsers
        let changelog_versions = parse_changelog_file(&changelog_content);
        if !changelog_versions.is_empty() {
            all_versions.push(changelog_versions);
            sources_used.push(ChangelogSource::ChangelogFile);
        }
    }

    // Check if we have any sources
    if all_versions.is_empty() {
        return Err(ChangelogError::NoSources);
    }

    // 2. Merge and deduplicate
    let merged_versions = merge_version_info(all_versions);

    // 3. Calculate confidence level
    let confidence = calculate_confidence(&sources_used);

    // 4. Build final version history
    let mut history = VersionHistory {
        latest_version: String::new(),
        versions: merged_versions,
        sources_used,
        confidence,
    };

    // 5. Sort and determine latest version
    history.sort_versions();
    history.update_latest();

    Ok(history)
}

/// Parse changelog file content using multiple parsers.
///
/// Tries Keep a Changelog, Conventional Changelog, and Generic parsers
/// in order, returning the result from the first parser that finds versions.
fn parse_changelog_file(content: &str) -> Vec<VersionInfo> {
    use super::discovery::{
        parse_conventional_changelog, parse_generic_changelog, parse_keep_a_changelog,
    };

    // Try Keep a Changelog format first
    let versions = parse_keep_a_changelog(content);
    if !versions.is_empty() {
        return versions;
    }

    // Try Conventional Changelog format
    let versions = parse_conventional_changelog(content);
    if !versions.is_empty() {
        return versions;
    }

    // Fall back to generic parser
    parse_generic_changelog(content)
}

/// Merge version info from multiple sources, preferring structured sources.
///
/// Versions with the same version string are merged into a single `VersionInfo`
/// with combined sources, features, and breaking changes. Date conflicts are
/// resolved using source precedence.
///
/// ## Source Precedence (for date conflicts)
///
/// 1. ChangelogFile (most curated)
/// 2. GitHubRelease (official releases)
/// 3. RegistryVersion (registry publication time)
/// 4. LlmKnowledge (least reliable)
///
/// ## Examples
///
/// ```
/// use research_lib::changelog::types::{VersionInfo, VersionSignificance, ChangelogSource};
/// use research_lib::changelog::aggregator::merge_version_info;
///
/// let mut v1 = VersionInfo::new("1.0.0", VersionSignificance::Major);
/// v1.add_source(ChangelogSource::GitHubRelease);
/// v1.new_features.push("Feature A".to_string());
///
/// let mut v2 = VersionInfo::new("1.0.0", VersionSignificance::Major);
/// v2.add_source(ChangelogSource::RegistryVersion);
/// v2.new_features.push("Feature B".to_string());
///
/// let merged = merge_version_info(vec![vec![v1], vec![v2]]);
/// assert_eq!(merged.len(), 1);
/// assert_eq!(merged[0].sources.len(), 2);
/// assert_eq!(merged[0].new_features.len(), 2);
/// ```
pub fn merge_version_info(versions: Vec<Vec<VersionInfo>>) -> Vec<VersionInfo> {
    let mut version_map: HashMap<String, VersionInfo> = HashMap::new();

    // Flatten all versions
    for version_list in versions {
        for version in version_list {
            let key = version.version.clone();

            version_map
                .entry(key)
                .and_modify(|existing| {
                    merge_single_version(existing, &version);
                })
                .or_insert(version);
        }
    }

    // Convert map back to sorted vector
    let mut result: Vec<VersionInfo> = version_map.into_values().collect();
    result.sort();
    result
}

/// Merge a single version into an existing version entry.
///
/// Combines sources, features, breaking changes, and resolves date conflicts
/// based on source precedence.
fn merge_single_version(existing: &mut VersionInfo, incoming: &VersionInfo) {
    // Check date precedence BEFORE merging sources (important!)
    // We need to compare the incoming source precedence against the existing sources
    // before they're merged together.
    let should_update_date =
        incoming.release_date.is_some() && should_prefer_incoming_date(existing, incoming);

    // Merge sources
    for source in &incoming.sources {
        existing.add_source(source.clone());
    }

    // Merge breaking changes (deduplicate)
    for change in &incoming.breaking_changes {
        if !existing.breaking_changes.contains(change) {
            existing.breaking_changes.push(change.clone());
        }
    }

    // Merge new features (deduplicate)
    for feature in &incoming.new_features {
        if !existing.new_features.contains(feature) {
            existing.new_features.push(feature.clone());
        }
    }

    // Update date if we determined it should be updated
    if should_update_date {
        existing.release_date = incoming.release_date;
    }

    // Merge summaries (prefer non-empty)
    if existing.summary.is_none() && incoming.summary.is_some() {
        existing.summary = incoming.summary.clone();
    }
}

/// Determine if incoming date should replace existing date based on source precedence.
///
/// Source precedence: ChangelogFile > GitHubRelease > RegistryVersion > LlmKnowledge
///
/// Returns true if:
/// - Existing has no date, OR
/// - Incoming has higher precedence (lower number) than existing
fn should_prefer_incoming_date(existing: &VersionInfo, incoming: &VersionInfo) -> bool {
    // If existing has no date, always prefer incoming
    if existing.release_date.is_none() {
        return true;
    }

    let existing_precedence = get_highest_source_precedence(&existing.sources);
    let incoming_precedence = get_highest_source_precedence(&incoming.sources);
    incoming_precedence < existing_precedence
}

/// Get the highest source precedence value (lower is better).
///
/// Returns:
/// - 0: ChangelogFile
/// - 1: GitHubRelease
/// - 2: RegistryVersion
/// - 3: LlmKnowledge
/// - 99: Unknown (fallback)
fn get_highest_source_precedence(sources: &[ChangelogSource]) -> u8 {
    sources
        .iter()
        .map(|s| match s {
            ChangelogSource::ChangelogFile => 0,
            ChangelogSource::GitHubRelease => 1,
            ChangelogSource::RegistryVersion => 2,
            ChangelogSource::LlmKnowledge => 3,
        })
        .min()
        .unwrap_or(99)
}

/// Calculate confidence level based on sources used.
///
/// ## Confidence Levels
///
/// - **High**: GitHub Releases + (Registry OR ChangelogFile)
/// - **Medium**: One structured source OR LLM-enriched structured data
/// - **Low**: LLM knowledge only
///
/// ## Examples
///
/// ```
/// use research_lib::changelog::types::{ChangelogSource, ConfidenceLevel};
/// use research_lib::changelog::aggregator::calculate_confidence;
///
/// // High confidence: GitHub + Registry
/// let sources = vec![ChangelogSource::GitHubRelease, ChangelogSource::RegistryVersion];
/// assert_eq!(calculate_confidence(&sources), ConfidenceLevel::High);
///
/// // Medium confidence: Single source
/// let sources = vec![ChangelogSource::GitHubRelease];
/// assert_eq!(calculate_confidence(&sources), ConfidenceLevel::Medium);
///
/// // Low confidence: LLM only
/// let sources = vec![ChangelogSource::LlmKnowledge];
/// assert_eq!(calculate_confidence(&sources), ConfidenceLevel::Low);
/// ```
pub fn calculate_confidence(sources: &[ChangelogSource]) -> ConfidenceLevel {
    let has_github = sources.contains(&ChangelogSource::GitHubRelease);
    let has_registry = sources.contains(&ChangelogSource::RegistryVersion);
    let has_changelog = sources.contains(&ChangelogSource::ChangelogFile);
    let has_llm = sources.contains(&ChangelogSource::LlmKnowledge);

    // High: GitHub + (Registry OR Changelog)
    if has_github && (has_registry || has_changelog) {
        return ConfidenceLevel::High;
    }

    // Medium: Any single structured source (or multiple structured sources without GitHub)
    if has_github || has_registry || has_changelog {
        return ConfidenceLevel::Medium;
    }

    // Low: LLM only or no sources
    if has_llm {
        ConfidenceLevel::Low
    } else {
        // No sources at all - this shouldn't happen due to NoSources error,
        // but default to Low just in case
        ConfidenceLevel::Low
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changelog::types::VersionSignificance;

    #[test]
    fn test_merge_version_info_single_source() {
        let mut v1 = VersionInfo::new("1.0.0", VersionSignificance::Major);
        v1.add_source(ChangelogSource::GitHubRelease);

        let merged = merge_version_info(vec![vec![v1.clone()]]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].version, "1.0.0");
    }

    #[test]
    fn test_merge_version_info_duplicate_versions() {
        let mut v1 = VersionInfo::new("1.0.0", VersionSignificance::Major);
        v1.add_source(ChangelogSource::GitHubRelease);
        v1.new_features.push("Feature A".to_string());

        let mut v2 = VersionInfo::new("1.0.0", VersionSignificance::Major);
        v2.add_source(ChangelogSource::RegistryVersion);
        v2.new_features.push("Feature B".to_string());

        let merged = merge_version_info(vec![vec![v1], vec![v2]]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].sources.len(), 2);
        assert_eq!(merged[0].new_features.len(), 2);
        assert!(merged[0]
            .sources
            .contains(&ChangelogSource::GitHubRelease));
        assert!(merged[0]
            .sources
            .contains(&ChangelogSource::RegistryVersion));
    }

    #[test]
    fn test_merge_version_info_different_versions() {
        let v1 = VersionInfo::new("1.0.0", VersionSignificance::Major);
        let v2 = VersionInfo::new("2.0.0", VersionSignificance::Major);
        let v3 = VersionInfo::new("1.5.0", VersionSignificance::Minor);

        let merged = merge_version_info(vec![vec![v1], vec![v2], vec![v3]]);
        assert_eq!(merged.len(), 3);

        // Should be sorted newest to oldest
        assert_eq!(merged[0].version, "2.0.0");
        assert_eq!(merged[1].version, "1.5.0");
        assert_eq!(merged[2].version, "1.0.0");
    }

    #[test]
    fn test_merge_version_info_deduplicates_features() {
        let mut v1 = VersionInfo::new("1.0.0", VersionSignificance::Major);
        v1.new_features.push("Feature A".to_string());
        v1.new_features.push("Feature B".to_string());

        let mut v2 = VersionInfo::new("1.0.0", VersionSignificance::Major);
        v2.new_features.push("Feature B".to_string()); // Duplicate
        v2.new_features.push("Feature C".to_string());

        let merged = merge_version_info(vec![vec![v1], vec![v2]]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].new_features.len(), 3);
        assert!(merged[0].new_features.contains(&"Feature A".to_string()));
        assert!(merged[0].new_features.contains(&"Feature B".to_string()));
        assert!(merged[0].new_features.contains(&"Feature C".to_string()));
    }

    #[test]
    fn test_merge_version_info_deduplicates_breaking_changes() {
        let mut v1 = VersionInfo::new("2.0.0", VersionSignificance::Major);
        v1.breaking_changes.push("Breaking A".to_string());

        let mut v2 = VersionInfo::new("2.0.0", VersionSignificance::Major);
        v2.breaking_changes.push("Breaking A".to_string()); // Duplicate
        v2.breaking_changes.push("Breaking B".to_string());

        let merged = merge_version_info(vec![vec![v1], vec![v2]]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].breaking_changes.len(), 2);
    }

    #[test]
    fn test_calculate_confidence_high() {
        // GitHub + Registry
        let sources = vec![
            ChangelogSource::GitHubRelease,
            ChangelogSource::RegistryVersion,
        ];
        assert_eq!(calculate_confidence(&sources), ConfidenceLevel::High);

        // GitHub + Changelog
        let sources = vec![
            ChangelogSource::GitHubRelease,
            ChangelogSource::ChangelogFile,
        ];
        assert_eq!(calculate_confidence(&sources), ConfidenceLevel::High);

        // GitHub + Both
        let sources = vec![
            ChangelogSource::GitHubRelease,
            ChangelogSource::RegistryVersion,
            ChangelogSource::ChangelogFile,
        ];
        assert_eq!(calculate_confidence(&sources), ConfidenceLevel::High);
    }

    #[test]
    fn test_calculate_confidence_medium() {
        // Single GitHub
        let sources = vec![ChangelogSource::GitHubRelease];
        assert_eq!(calculate_confidence(&sources), ConfidenceLevel::Medium);

        // Single Registry
        let sources = vec![ChangelogSource::RegistryVersion];
        assert_eq!(calculate_confidence(&sources), ConfidenceLevel::Medium);

        // Single Changelog
        let sources = vec![ChangelogSource::ChangelogFile];
        assert_eq!(calculate_confidence(&sources), ConfidenceLevel::Medium);

        // Registry + Changelog (no GitHub)
        let sources = vec![
            ChangelogSource::RegistryVersion,
            ChangelogSource::ChangelogFile,
        ];
        assert_eq!(calculate_confidence(&sources), ConfidenceLevel::Medium);
    }

    #[test]
    fn test_calculate_confidence_low() {
        // LLM only
        let sources = vec![ChangelogSource::LlmKnowledge];
        assert_eq!(calculate_confidence(&sources), ConfidenceLevel::Low);

        // Empty sources
        let sources = vec![];
        assert_eq!(calculate_confidence(&sources), ConfidenceLevel::Low);
    }

    #[test]
    fn test_get_highest_source_precedence() {
        let sources = vec![
            ChangelogSource::GitHubRelease,
            ChangelogSource::ChangelogFile,
        ];
        assert_eq!(get_highest_source_precedence(&sources), 0); // ChangelogFile wins

        let sources = vec![
            ChangelogSource::RegistryVersion,
            ChangelogSource::GitHubRelease,
        ];
        assert_eq!(get_highest_source_precedence(&sources), 1); // GitHubRelease wins

        let sources = vec![ChangelogSource::LlmKnowledge];
        assert_eq!(get_highest_source_precedence(&sources), 3);

        let sources = vec![];
        assert_eq!(get_highest_source_precedence(&sources), 99); // Fallback
    }

    #[test]
    fn test_merge_single_version_date_precedence() {
        use chrono::{TimeZone, Utc};

        // ChangelogFile should override GitHubRelease date
        let mut existing = VersionInfo::new("1.0.0", VersionSignificance::Major);
        existing.add_source(ChangelogSource::GitHubRelease);
        let existing_date = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();
        existing.release_date = Some(existing_date);

        let mut incoming = VersionInfo::new("1.0.0", VersionSignificance::Major);
        incoming.add_source(ChangelogSource::ChangelogFile);
        let incoming_date = Utc.with_ymd_and_hms(2024, 1, 14, 10, 0, 0).unwrap();
        incoming.release_date = Some(incoming_date);

        merge_single_version(&mut existing, &incoming);
        assert_eq!(existing.release_date, Some(incoming_date));

        // RegistryVersion should NOT override GitHubRelease date
        let mut existing = VersionInfo::new("2.0.0", VersionSignificance::Major);
        existing.add_source(ChangelogSource::GitHubRelease);
        let existing_date = Utc.with_ymd_and_hms(2024, 2, 1, 10, 0, 0).unwrap();
        existing.release_date = Some(existing_date);

        let mut incoming = VersionInfo::new("2.0.0", VersionSignificance::Major);
        incoming.add_source(ChangelogSource::RegistryVersion);
        let incoming_date = Utc.with_ymd_and_hms(2024, 1, 31, 10, 0, 0).unwrap();
        incoming.release_date = Some(incoming_date);

        merge_single_version(&mut existing, &incoming);
        assert_eq!(existing.release_date, Some(existing_date)); // Unchanged
    }

    #[test]
    fn test_merge_single_version_summary_precedence() {
        let mut existing = VersionInfo::new("1.0.0", VersionSignificance::Major);
        existing.summary = None;

        let mut incoming = VersionInfo::new("1.0.0", VersionSignificance::Major);
        incoming.summary = Some("Summary from incoming".to_string());

        merge_single_version(&mut existing, &incoming);
        assert_eq!(
            existing.summary,
            Some("Summary from incoming".to_string())
        );

        // Existing summary should not be overwritten
        let mut existing = VersionInfo::new("2.0.0", VersionSignificance::Major);
        existing.summary = Some("Existing summary".to_string());

        let mut incoming = VersionInfo::new("2.0.0", VersionSignificance::Major);
        incoming.summary = Some("Incoming summary".to_string());

        merge_single_version(&mut existing, &incoming);
        assert_eq!(existing.summary, Some("Existing summary".to_string()));
    }

    #[tokio::test]
    async fn test_aggregate_version_history_no_sources() {
        let client = HttpClient::new();

        // Use invalid URLs that will fail all fetches
        let result = aggregate_version_history(
            &client,
            "nonexistent-package",
            "unknown-registry",
            "https://localhost:1/invalid/repo",
        )
        .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ChangelogError::NoSources));
    }

    #[test]
    fn test_parse_changelog_file_keep_a_changelog() {
        let content = r#"
# Changelog
## [1.0.0] - 2024-01-15
### Added
- New feature
"#;

        let versions = parse_changelog_file(content);
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, "1.0.0");
        assert!(versions[0]
            .sources
            .contains(&ChangelogSource::ChangelogFile));
    }

    #[test]
    fn test_parse_changelog_file_conventional() {
        let content = r#"
# Changelog
## 1.0.0 (2024-01-15)
### Features
* feat: new feature
"#;

        let versions = parse_changelog_file(content);
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, "1.0.0");
    }

    #[test]
    fn test_parse_changelog_file_generic() {
        let content = r#"
# Changelog
## Version 1.0.0 - Released 2024-01-15
"#;

        let versions = parse_changelog_file(content);
        assert!(versions.len() >= 1);
        assert!(versions.iter().any(|v| v.version == "1.0.0"));
    }

    #[test]
    fn test_version_history_sorting() {
        let mut v1 = VersionInfo::new("1.0.0", VersionSignificance::Major);
        v1.add_source(ChangelogSource::GitHubRelease);

        let mut v2 = VersionInfo::new("2.0.0", VersionSignificance::Major);
        v2.add_source(ChangelogSource::GitHubRelease);

        let merged = merge_version_info(vec![vec![v1], vec![v2]]);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].version, "2.0.0"); // Newest first
        assert_eq!(merged[1].version, "1.0.0");
    }
}
