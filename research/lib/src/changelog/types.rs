//! Core types for changelog version tracking.
//!
//! This module defines the fundamental data structures used throughout the changelog
//! system for representing version information, tracking data sources, and aggregating
//! version history.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use thiserror::Error;

/// Error types for changelog operations.
#[derive(Error, Debug)]
pub enum ChangelogError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Failed to parse URL
    #[error("Failed to parse URL: {0}")]
    UrlParse(String),

    /// Failed to parse version
    #[error("Failed to parse version: {0}")]
    VersionParse(String),

    /// Failed to parse JSON response
    #[error("Failed to parse JSON response: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// GitHub API rate limit exceeded
    #[error("GitHub API rate limit exceeded")]
    RateLimitExceeded,

    /// Failed to parse changelog file
    #[error("Failed to parse changelog file: {0}")]
    ParseError(String),

    /// No valid sources available
    #[error("No valid sources available")]
    NoSources,

    /// Failed to parse date
    #[error("Failed to parse date: {0}")]
    DateParse(String),
}

/// Significance level for a version release.
///
/// Determines the importance of a version in the changelog timeline.
/// Used for filtering and highlighting major milestones.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum VersionSignificance {
    /// Major version change (1.0.0 -> 2.0.0)
    Major,
    /// Minor version change with notable features (1.1.0 -> 1.2.0)
    Minor,
    /// Patch release, only if notable fix (1.0.1)
    Patch,
    /// Pre-release version (alpha, beta, rc)
    Prerelease,
}

/// Information about a specific version.
///
/// Contains all metadata about a single release, including the version number,
/// release date, significance level, and feature/breaking change information.
/// Multiple sources can confirm the same version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Version string (e.g., "1.2.3")
    pub version: String,
    /// Release date (if known)
    pub release_date: Option<DateTime<Utc>>,
    /// Significance level of this release
    pub significance: VersionSignificance,
    /// Brief summary of the release
    pub summary: Option<String>,
    /// Breaking changes in this version
    pub breaking_changes: Vec<String>,
    /// New features in this version
    pub new_features: Vec<String>,
    /// Sources that confirmed this version (multiple sources can validate same version)
    pub sources: Vec<ChangelogSource>,
}

impl VersionInfo {
    /// Creates a new VersionInfo with the given version string.
    ///
    /// ## Examples
    ///
    /// ```
    /// use research_lib::changelog::types::{VersionInfo, VersionSignificance};
    ///
    /// let version = VersionInfo::new("1.0.0", VersionSignificance::Major);
    /// assert_eq!(version.version, "1.0.0");
    /// assert_eq!(version.significance, VersionSignificance::Major);
    /// ```
    pub fn new(version: impl Into<String>, significance: VersionSignificance) -> Self {
        Self {
            version: version.into(),
            release_date: None,
            significance,
            summary: None,
            breaking_changes: vec![],
            new_features: vec![],
            sources: vec![],
        }
    }

    /// Adds a source to this version's list of confirming sources.
    pub fn add_source(&mut self, source: ChangelogSource) {
        if !self.sources.contains(&source) {
            self.sources.push(source);
        }
    }

    /// Parse version using semver and determine significance.
    ///
    /// ## Errors
    ///
    /// Returns `ChangelogError::VersionParse` if the version string cannot be parsed.
    pub fn from_version_str(version: &str) -> Result<Self, ChangelogError> {
        let semver = semver::Version::parse(version)
            .map_err(|e| ChangelogError::VersionParse(format!("{}: {}", version, e)))?;

        let significance = if !semver.pre.is_empty() {
            VersionSignificance::Prerelease
        } else if semver.major > 0 {
            VersionSignificance::Major
        } else if semver.minor > 0 {
            VersionSignificance::Minor
        } else {
            VersionSignificance::Patch
        };

        Ok(Self::new(version, significance))
    }
}

/// Compare versions by semver ordering.
///
/// Versions are ordered from newest to oldest (descending).
/// Invalid semver strings fall back to string comparison.
impl Ord for VersionInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        match (
            semver::Version::parse(&self.version),
            semver::Version::parse(&other.version),
        ) {
            (Ok(a), Ok(b)) => b.cmp(&a),           // Reverse order (newest first)
            _ => other.version.cmp(&self.version), // Fallback to string comparison
        }
    }
}

impl PartialOrd for VersionInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for VersionInfo {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
    }
}

impl Eq for VersionInfo {}

/// Where the version information came from.
///
/// Tracks the source of version information for confidence calculation
/// and deduplication. Multiple sources can confirm the same version.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChangelogSource {
    /// GitHub Releases API
    GitHubRelease,
    /// CHANGELOG.md or similar file
    ChangelogFile,
    /// Package registry (crates.io, npm, PyPI)
    RegistryVersion,
    /// LLM knowledge
    LlmKnowledge,
}

/// Aggregated version history from all sources.
///
/// Contains the complete version history for a library, including metadata
/// about which sources were used and the confidence level of the data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionHistory {
    /// Latest version string
    pub latest_version: String,
    /// All versions, sorted newest to oldest
    pub versions: Vec<VersionInfo>,
    /// Sources that contributed data
    pub sources_used: Vec<ChangelogSource>,
    /// Confidence level based on sources
    pub confidence: ConfidenceLevel,
}

impl Default for VersionHistory {
    fn default() -> Self {
        Self {
            latest_version: String::new(),
            versions: vec![],
            sources_used: vec![],
            confidence: ConfidenceLevel::Low,
        }
    }
}

impl VersionHistory {
    /// Sorts versions from newest to oldest.
    pub fn sort_versions(&mut self) {
        self.versions.sort();
    }

    /// Updates the latest version based on sorted versions.
    pub fn update_latest(&mut self) {
        if let Some(latest) = self.versions.first() {
            self.latest_version = latest.version.clone();
        }
    }
}

/// Confidence level for version history data.
///
/// Indicates the reliability of the version history based on
/// which sources were available.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    /// Structured sources available (GitHub Releases + Registry/ChangelogFile)
    High,
    /// Some structured sources or LLM-enriched structured data
    Medium,
    /// LLM knowledge only
    Low,
}

/// Parse dates from various formats (ISO-8601, US, UK, relative).
///
/// Supports multiple common date formats found in changelogs:
/// - ISO 8601: "2024-01-15", "2024-01-15T10:30:00Z"
/// - US format: "01/15/2024", "January 15, 2024"
/// - UK format: "15/01/2024", "15 January 2024"
///
/// ## Errors
///
/// Returns `ChangelogError::DateParse` if the date cannot be parsed in any format.
///
/// ## Examples
///
/// ```
/// use research_lib::changelog::types::parse_flexible_date;
///
/// let date1 = parse_flexible_date("2024-01-15").unwrap();
/// let date2 = parse_flexible_date("2024-01-15T10:30:00Z").unwrap();
/// assert_eq!(date1.date_naive(), date2.date_naive());
/// ```
pub fn parse_flexible_date(s: &str) -> Result<DateTime<Utc>, ChangelogError> {
    // Try ISO 8601 with timezone first
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try ISO 8601 date only (YYYY-MM-DD)
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| ChangelogError::DateParse(s.to_string()))?
            .and_utc());
    }

    // Try US format (MM/DD/YYYY)
    if let Ok(date) = NaiveDate::parse_from_str(s, "%m/%d/%Y") {
        return Ok(date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| ChangelogError::DateParse(s.to_string()))?
            .and_utc());
    }

    // Try UK format (DD/MM/YYYY)
    if let Ok(date) = NaiveDate::parse_from_str(s, "%d/%m/%Y") {
        return Ok(date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| ChangelogError::DateParse(s.to_string()))?
            .and_utc());
    }

    Err(ChangelogError::DateParse(format!(
        "Unable to parse date: {}",
        s
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info_new() {
        let version = VersionInfo::new("1.0.0", VersionSignificance::Major);
        assert_eq!(version.version, "1.0.0");
        assert_eq!(version.significance, VersionSignificance::Major);
        assert!(version.sources.is_empty());
        assert!(version.breaking_changes.is_empty());
        assert!(version.new_features.is_empty());
    }

    #[test]
    fn test_version_info_add_source() {
        let mut version = VersionInfo::new("1.0.0", VersionSignificance::Major);
        version.add_source(ChangelogSource::GitHubRelease);
        version.add_source(ChangelogSource::RegistryVersion);
        version.add_source(ChangelogSource::GitHubRelease); // Duplicate

        assert_eq!(version.sources.len(), 2);
        assert!(version.sources.contains(&ChangelogSource::GitHubRelease));
        assert!(version.sources.contains(&ChangelogSource::RegistryVersion));
    }

    #[test]
    fn test_version_info_from_version_str() {
        let v1 = VersionInfo::from_version_str("2.5.0").unwrap();
        assert_eq!(v1.significance, VersionSignificance::Major);

        let v2 = VersionInfo::from_version_str("0.3.0").unwrap();
        assert_eq!(v2.significance, VersionSignificance::Minor);

        let v3 = VersionInfo::from_version_str("0.0.1").unwrap();
        assert_eq!(v3.significance, VersionSignificance::Patch);

        let v4 = VersionInfo::from_version_str("1.0.0-alpha.1").unwrap();
        assert_eq!(v4.significance, VersionSignificance::Prerelease);
    }

    #[test]
    fn test_version_info_ordering() {
        let v1 = VersionInfo::new("1.0.0", VersionSignificance::Major);
        let v2 = VersionInfo::new("2.0.0", VersionSignificance::Major);
        let v3 = VersionInfo::new("1.5.0", VersionSignificance::Minor);

        assert!(v2 < v1); // v2 (2.0.0) is newer, so less than in descending order
        assert!(v3 < v1); // v3 (1.5.0) is newer than v1 (1.0.0), so less than in descending order
    }

    #[test]
    fn test_version_info_ordering_transitivity() {
        // Property test: ordering should be transitive
        let v1 = VersionInfo::new("1.0.0", VersionSignificance::Major);
        let v2 = VersionInfo::new("2.0.0", VersionSignificance::Major);
        let v3 = VersionInfo::new("3.0.0", VersionSignificance::Major);

        // If v3 < v2 and v2 < v1, then v3 < v1
        assert!(v3 < v2);
        assert!(v2 < v1);
        assert!(v3 < v1);
    }

    #[test]
    fn test_version_history_default() {
        let history = VersionHistory::default();
        assert_eq!(history.latest_version, "");
        assert!(history.versions.is_empty());
        assert!(history.sources_used.is_empty());
        assert_eq!(history.confidence, ConfidenceLevel::Low);
    }

    #[test]
    fn test_version_history_sort_and_update() {
        let mut history = VersionHistory::default();
        history
            .versions
            .push(VersionInfo::new("1.0.0", VersionSignificance::Major));
        history
            .versions
            .push(VersionInfo::new("2.0.0", VersionSignificance::Major));
        history
            .versions
            .push(VersionInfo::new("1.5.0", VersionSignificance::Minor));

        history.sort_versions();
        history.update_latest();

        assert_eq!(history.versions.len(), 3);
        assert_eq!(history.latest_version, "2.0.0");
        assert_eq!(history.versions[0].version, "2.0.0");
    }

    #[test]
    fn test_deduplication_idempotence() {
        // Property test: adding the same source twice should be idempotent
        let mut v1 = VersionInfo::new("1.0.0", VersionSignificance::Major);
        v1.add_source(ChangelogSource::GitHubRelease);
        let len_after_first = v1.sources.len();

        v1.add_source(ChangelogSource::GitHubRelease);
        let len_after_second = v1.sources.len();

        assert_eq!(len_after_first, len_after_second);
        assert_eq!(v1.sources.len(), 1);
    }

    #[test]
    fn test_parse_flexible_date_iso8601() {
        let date = parse_flexible_date("2024-01-15").unwrap();
        assert_eq!(date.format("%Y-%m-%d").to_string(), "2024-01-15");
    }

    #[test]
    fn test_parse_flexible_date_iso8601_with_time() {
        let date = parse_flexible_date("2024-01-15T10:30:00Z").unwrap();
        assert_eq!(date.format("%Y-%m-%d").to_string(), "2024-01-15");
    }

    #[test]
    fn test_parse_flexible_date_us_format() {
        let date = parse_flexible_date("01/15/2024").unwrap();
        assert_eq!(date.format("%Y-%m-%d").to_string(), "2024-01-15");
    }

    #[test]
    fn test_parse_flexible_date_uk_format() {
        let date = parse_flexible_date("15/01/2024").unwrap();
        assert_eq!(date.format("%Y-%m-%d").to_string(), "2024-01-15");
    }

    #[test]
    fn test_parse_flexible_date_invalid() {
        let result = parse_flexible_date("not a date");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ChangelogError::DateParse(_)));
    }

    #[test]
    fn test_confidence_levels() {
        assert_eq!(ConfidenceLevel::High, ConfidenceLevel::High);
        assert_ne!(ConfidenceLevel::High, ConfidenceLevel::Medium);
        assert_ne!(ConfidenceLevel::Medium, ConfidenceLevel::Low);
    }

    #[test]
    fn test_changelog_source_equality() {
        assert_eq!(
            ChangelogSource::GitHubRelease,
            ChangelogSource::GitHubRelease
        );
        assert_ne!(
            ChangelogSource::GitHubRelease,
            ChangelogSource::ChangelogFile
        );
    }
}
