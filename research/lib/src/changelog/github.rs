//! GitHub Releases API client for fetching release information.
//!
//! This module provides functionality to fetch version history from GitHub Releases,
//! including support for authentication, rate limiting, and parsing release information.

use super::types::{ChangelogError, ChangelogSource, VersionInfo, VersionSignificance};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

/// GitHub API response for a single release.
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    /// Tag name (e.g., "v1.2.3")
    tag_name: String,
    /// Release name
    name: Option<String>,
    /// Release body (markdown)
    body: Option<String>,
    /// Published timestamp
    published_at: Option<String>,
    /// Is this a prerelease?
    prerelease: bool,
    /// Is this a draft?
    draft: bool,
}

/// GitHub API error response.
#[derive(Debug, Deserialize)]
struct GitHubError {
    message: String,
}

/// Extract owner and repository from various GitHub URL formats.
///
/// Supports:
/// - `https://github.com/owner/repo`
/// - `https://github.com/owner/repo.git`
/// - `git@github.com:owner/repo.git`
/// - `https://raw.githubusercontent.com/owner/repo/...`
///
/// ## Examples
///
/// ```
/// use research_lib::changelog::github::parse_github_url;
///
/// let (owner, repo) = parse_github_url("https://github.com/rust-lang/rust").unwrap();
/// assert_eq!(owner, "rust-lang");
/// assert_eq!(repo, "rust");
///
/// let (owner, repo) = parse_github_url("git@github.com:tokio-rs/tokio.git").unwrap();
/// assert_eq!(owner, "tokio-rs");
/// assert_eq!(repo, "tokio");
/// ```
pub fn parse_github_url(url: &str) -> Option<(String, String)> {
    let url = url.trim();

    // Handle git@ SSH URLs
    if url.starts_with("git@github.com:") {
        let rest = url.strip_prefix("git@github.com:")?;
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() >= 2 {
            let owner = parts[0];
            let repo = parts[1].trim_end_matches(".git");
            return Some((owner.to_string(), repo.to_string()));
        }
    }

    // Handle HTTPS URLs (github.com or raw.githubusercontent.com)
    if url.contains("github.com") || url.contains("githubusercontent.com") {
        // Extract path part after domain
        let parts: Vec<&str> = url.split('/').collect();

        // Find the index of github.com or githubusercontent.com
        let domain_idx = parts
            .iter()
            .position(|&p| p.contains("github.com") || p.contains("githubusercontent.com"))?;

        // Owner and repo come after the domain
        if parts.len() > domain_idx + 2 {
            let owner = parts[domain_idx + 1];
            let repo = parts[domain_idx + 2].trim_end_matches(".git");
            return Some((owner.to_string(), repo.to_string()));
        }
    }

    // Handle test URLs (localhost/127.0.0.1 with /owner/repo path)
    // This is for wiremock tests
    if url.contains("localhost") || url.contains("127.0.0.1") {
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() >= 2 {
            // Find owner/repo from the end of the URL
            let owner = parts.get(parts.len() - 2)?;
            let repo = parts.last()?.trim_end_matches(".git");
            return Some((owner.to_string(), repo.to_string()));
        }
    }

    None
}

/// Fetch releases from GitHub Releases API.
///
/// ## Arguments
///
/// * `client` - HTTP client for making requests
/// * `repo_url` - GitHub repository URL (e.g., `https://github.com/owner/repo`)
/// * `limit` - Maximum number of releases to fetch (max: 100 per page)
///
/// ## Returns
///
/// Vector of `VersionInfo` from GitHub releases, sorted newest to oldest.
///
/// ## Errors
///
/// - `ChangelogError::UrlParse` - Invalid GitHub URL format
/// - `ChangelogError::RateLimitExceeded` - API rate limit exceeded
/// - `ChangelogError::Http` - Network or HTTP error
/// - `ChangelogError::JsonParse` - Invalid JSON response
///
/// ## Examples
///
/// ```rust,no_run
/// use reqwest::Client;
/// use research_lib::changelog::github::fetch_github_releases;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new();
/// let releases = fetch_github_releases(
///     &client,
///     "https://github.com/tokio-rs/tokio",
///     10
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn fetch_github_releases(
    client: &Client,
    repo_url: &str,
    limit: usize,
) -> Result<Vec<VersionInfo>, ChangelogError> {
    let (owner, repo) = parse_github_url(repo_url)
        .ok_or_else(|| ChangelogError::UrlParse(format!("Invalid GitHub URL: {}", repo_url)))?;

    // Determine base URL (for testing support)
    let base_url = if repo_url.contains("localhost") || repo_url.contains("127.0.0.1") {
        // Extract base URL for tests (e.g., "http://127.0.0.1:1234" from "http://127.0.0.1:1234/owner/repo")
        if let Some(pos) = repo_url.find("//") {
            let after_slash = &repo_url[pos + 2..];
            if let Some(slash_pos) = after_slash.find('/') {
                format!("{}{}", &repo_url[..pos + 2], &after_slash[..slash_pos])
            } else {
                repo_url.to_string()
            }
        } else {
            repo_url.to_string()
        }
    } else {
        "https://api.github.com".to_string()
    };

    let api_url = format!(
        "{}/repos/{}/{}/releases?per_page={}",
        base_url,
        owner,
        repo,
        limit.min(100)
    );

    let mut request = client
        .get(&api_url)
        .header("User-Agent", "research-lib")
        .header("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(10));

    // Add authentication if GITHUB_TOKEN is available
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request.send().await?;

    // Check for rate limiting
    if response.status().as_u16() == 429 {
        return Err(ChangelogError::RateLimitExceeded);
    }

    // Check rate limit headers
    if let Some(remaining) = response.headers().get("X-RateLimit-Remaining")
        && let Ok(remaining_str) = remaining.to_str()
        && let Ok(remaining_count) = remaining_str.parse::<u32>()
        && remaining_count == 0
    {
        return Err(ChangelogError::RateLimitExceeded);
    }

    // Handle error responses
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        if let Ok(gh_error) = serde_json::from_str::<GitHubError>(&error_text) {
            return Err(ChangelogError::UrlParse(format!(
                "GitHub API error ({}): {}",
                status, gh_error.message
            )));
        }
        return Err(ChangelogError::UrlParse(format!(
            "GitHub API error ({}): {}",
            status, error_text
        )));
    }

    let releases: Vec<GitHubRelease> = response.json().await?;

    let mut versions = Vec::new();
    for release in releases {
        // Skip drafts
        if release.draft {
            continue;
        }

        // Parse version from tag name (strip 'v' prefix if present)
        let version_str = release
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&release.tag_name);

        // Create VersionInfo
        let mut version_info = match VersionInfo::from_version_str(version_str) {
            Ok(info) => info,
            Err(_) => {
                // If semver parsing fails, create a basic version
                let significance = if release.prerelease {
                    VersionSignificance::Prerelease
                } else {
                    VersionSignificance::Major
                };
                VersionInfo::new(version_str, significance)
            }
        };

        // Add source
        version_info.add_source(ChangelogSource::GitHubRelease);

        // Parse release date
        if let Some(published_at) = release.published_at
            && let Ok(date) = super::types::parse_flexible_date(&published_at)
        {
            version_info.release_date = Some(date);
        }

        // Add summary from release name
        if let Some(name) = release.name
            && !name.is_empty()
            && name != release.tag_name
        {
            version_info.summary = Some(name);
        }

        // Parse release body for breaking changes and features
        if let Some(body) = release.body {
            parse_release_body(&body, &mut version_info);
        }

        versions.push(version_info);
    }

    Ok(versions)
}

/// Parse release body for breaking changes and features.
///
/// Looks for common markdown patterns:
/// - Breaking changes: "BREAKING", "Breaking Changes", etc.
/// - Features: "Features", "New Features", etc.
fn parse_release_body(body: &str, version_info: &mut VersionInfo) {
    let lines: Vec<&str> = body.lines().collect();
    let mut current_section = None;

    for line in lines {
        let line_lower = line.to_lowercase();
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Detect section headers (must contain heading marker or be a distinct header)
        if line_lower.contains("breaking") && (line.starts_with('#') || trimmed.ends_with(':')) {
            current_section = Some("breaking");
            continue;
        } else if line_lower.contains("feature")
            && (line.starts_with('#') || trimmed.ends_with(':'))
        {
            current_section = Some("features");
            continue;
        } else if line.starts_with('#') {
            // Reset section on new header that's not breaking/features
            if !line_lower.contains("breaking") && !line_lower.contains("feature") {
                current_section = None;
            }
            continue;
        }

        // Parse list items in current section
        if let Some(section) = current_section
            && let Some(item) = trimmed
                .strip_prefix('-')
                .or_else(|| trimmed.strip_prefix('*'))
        {
            let item = item.trim();
            if !item.is_empty() {
                match section {
                    "breaking" => version_info.breaking_changes.push(item.to_string()),
                    "features" => version_info.new_features.push(item.to_string()),
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_parse_github_url_https() {
        let (owner, repo) = parse_github_url("https://github.com/rust-lang/rust").unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(repo, "rust");
    }

    #[test]
    fn test_parse_github_url_https_with_git() {
        let (owner, repo) = parse_github_url("https://github.com/tokio-rs/tokio.git").unwrap();
        assert_eq!(owner, "tokio-rs");
        assert_eq!(repo, "tokio");
    }

    #[test]
    fn test_parse_github_url_ssh() {
        let (owner, repo) = parse_github_url("git@github.com:serde-rs/serde.git").unwrap();
        assert_eq!(owner, "serde-rs");
        assert_eq!(repo, "serde");
    }

    #[test]
    fn test_parse_github_url_raw() {
        let (owner, repo) =
            parse_github_url("https://raw.githubusercontent.com/clap-rs/clap/master/README.md")
                .unwrap();
        assert_eq!(owner, "clap-rs");
        assert_eq!(repo, "clap");
    }

    #[test]
    fn test_parse_github_url_invalid() {
        assert!(parse_github_url("https://gitlab.com/owner/repo").is_none());
        assert!(parse_github_url("not a url").is_none());
        assert!(parse_github_url("").is_none());
    }

    #[tokio::test]
    async fn test_fetch_github_releases_success() {
        let mock_server = MockServer::start().await;

        let response_body = r###"[
            {
                "tag_name": "v1.2.3",
                "name": "Release 1.2.3",
                "body": "## Features\n- New feature A\n- New feature B\n\n## Breaking Changes\n- Breaking change 1",
                "published_at": "2024-01-15T10:30:00Z",
                "prerelease": false,
                "draft": false
            },
            {
                "tag_name": "v1.2.2",
                "name": "Release 1.2.2",
                "body": "Bug fixes",
                "published_at": "2024-01-10T10:30:00Z",
                "prerelease": false,
                "draft": false
            }
        ]"###;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/releases"))
            .and(header("User-Agent", "research-lib"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = Client::new();
        let repo_url = format!("{}/owner/repo", mock_server.uri());
        let versions = fetch_github_releases(&client, &repo_url, 10).await.unwrap();

        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version, "1.2.3");
        assert_eq!(versions[1].version, "1.2.2");
        assert!(
            versions[0]
                .sources
                .contains(&ChangelogSource::GitHubRelease)
        );
        assert_eq!(versions[0].new_features.len(), 2);
        assert_eq!(versions[0].breaking_changes.len(), 1);
    }

    #[tokio::test]
    async fn test_fetch_github_releases_rate_limit_429() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/releases"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        let client = Client::new();
        let repo_url = format!("{}/owner/repo", mock_server.uri());
        let result = fetch_github_releases(&client, &repo_url, 10).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ChangelogError::RateLimitExceeded
        ));
    }

    #[tokio::test]
    async fn test_fetch_github_releases_rate_limit_header() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/releases"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("[]")
                    .insert_header("X-RateLimit-Remaining", "0"),
            )
            .mount(&mock_server)
            .await;

        let client = Client::new();
        let repo_url = format!("{}/owner/repo", mock_server.uri());
        let result = fetch_github_releases(&client, &repo_url, 10).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ChangelogError::RateLimitExceeded
        ));
    }

    #[tokio::test]
    async fn test_fetch_github_releases_invalid_url() {
        let client = Client::new();
        let result = fetch_github_releases(&client, "not a github url", 10).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ChangelogError::UrlParse(_)));
    }

    #[tokio::test]
    async fn test_fetch_github_releases_network_error() {
        let client = Client::new();
        // Use a URL that will fail to connect
        let result = fetch_github_releases(&client, "https://localhost:1/owner/repo", 10).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ChangelogError::Http(_)));
    }

    #[tokio::test]
    async fn test_fetch_github_releases_malformed_json() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/releases"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&mock_server)
            .await;

        let client = Client::new();
        let repo_url = format!("{}/owner/repo", mock_server.uri());
        let result = fetch_github_releases(&client, &repo_url, 10).await;

        assert!(result.is_err());
        // reqwest returns Http error for JSON parsing failures, not JsonParse
        assert!(matches!(result.unwrap_err(), ChangelogError::Http(_)));
    }

    #[tokio::test]
    async fn test_fetch_github_releases_skips_drafts() {
        let mock_server = MockServer::start().await;

        let response_body = r##"[
            {
                "tag_name": "v1.2.3",
                "name": "Release 1.2.3",
                "body": "",
                "published_at": "2024-01-15T10:30:00Z",
                "prerelease": false,
                "draft": true
            },
            {
                "tag_name": "v1.2.2",
                "name": "Release 1.2.2",
                "body": "",
                "published_at": "2024-01-10T10:30:00Z",
                "prerelease": false,
                "draft": false
            }
        ]"##;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/releases"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = Client::new();
        let repo_url = format!("{}/owner/repo", mock_server.uri());
        let versions = fetch_github_releases(&client, &repo_url, 10).await.unwrap();

        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, "1.2.2");
    }

    #[tokio::test]
    async fn test_fetch_github_releases_prerelease() {
        let mock_server = MockServer::start().await;

        let response_body = r##"[
            {
                "tag_name": "v1.2.3-beta.1",
                "name": "Beta Release",
                "body": "",
                "published_at": "2024-01-15T10:30:00Z",
                "prerelease": true,
                "draft": false
            }
        ]"##;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/releases"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = Client::new();
        let repo_url = format!("{}/owner/repo", mock_server.uri());
        let versions = fetch_github_releases(&client, &repo_url, 10).await.unwrap();

        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, "1.2.3-beta.1");
        assert_eq!(versions[0].significance, VersionSignificance::Prerelease);
    }

    #[test]
    fn test_parse_release_body_breaking_changes() {
        let body = r#"
## Breaking Changes

- Removed deprecated API
- Changed function signature

## Features

- Added new feature
"#;

        let mut version_info = VersionInfo::new("1.0.0", VersionSignificance::Major);
        parse_release_body(body, &mut version_info);

        assert_eq!(version_info.breaking_changes.len(), 2);
        assert!(version_info.breaking_changes[0].contains("Removed deprecated"));
        assert_eq!(version_info.new_features.len(), 1);
        assert!(version_info.new_features[0].contains("Added new feature"));
    }

    #[test]
    fn test_parse_release_body_no_sections() {
        let body = "Just a simple release note";

        let mut version_info = VersionInfo::new("1.0.0", VersionSignificance::Major);
        parse_release_body(body, &mut version_info);

        assert_eq!(version_info.breaking_changes.len(), 0);
        assert_eq!(version_info.new_features.len(), 0);
    }

    #[tokio::test]
    async fn test_fetch_github_releases_without_v_prefix() {
        let mock_server = MockServer::start().await;

        let response_body = r##"[
            {
                "tag_name": "1.2.3",
                "name": "Release 1.2.3",
                "body": "",
                "published_at": "2024-01-15T10:30:00Z",
                "prerelease": false,
                "draft": false
            }
        ]"##;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/releases"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = Client::new();
        let repo_url = format!("{}/owner/repo", mock_server.uri());
        let versions = fetch_github_releases(&client, &repo_url, 10).await.unwrap();

        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, "1.2.3");
    }

    #[tokio::test]
    async fn test_fetch_github_releases_invalid_semver() {
        let mock_server = MockServer::start().await;

        let response_body = r##"[
            {
                "tag_name": "invalid-version",
                "name": "Invalid",
                "body": "",
                "published_at": "2024-01-15T10:30:00Z",
                "prerelease": false,
                "draft": false
            }
        ]"##;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/releases"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let client = Client::new();
        let repo_url = format!("{}/owner/repo", mock_server.uri());
        let versions = fetch_github_releases(&client, &repo_url, 10).await.unwrap();

        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, "invalid-version");
        assert_eq!(versions[0].significance, VersionSignificance::Major);
    }
}
