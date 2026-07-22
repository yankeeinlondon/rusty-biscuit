//! Parse git remote URLs into provider, owner, repo, and base URL components.
//!
//! Supports multiple URL formats:
//! - HTTPS: `https://github.com/owner/repo`, `https://github.com/owner/repo.git`
//! - SSH: `git@github.com:owner/repo.git`, `ssh://git@github.com/owner/repo`
//! - GitLab with groups: `https://gitlab.com/group/subgroup/repo`
//! - Self-hosted: `https://git.example.com/owner/repo`
//! - Bitbucket: `https://bitbucket.org/workspace/repo`

use super::GitProvider;
use crate::error::SniffError;

/// A parsed remote URL with provider, owner, repository, and base URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRemoteUrl {
    /// The detected git hosting provider.
    pub provider: GitProvider,
    /// The owner, organization, workspace, or group.
    /// For GitLab nested groups, this includes the full path (e.g., "group/subgroup").
    pub owner: String,
    /// The repository name (without `.git` suffix).
    pub repo: String,
    /// The API base URL (for self-hosted providers like Gitea).
    /// For well-known providers (GitHub, GitLab, Bitbucket), this is `None`.
    pub base_url: Option<String>,
}

/// Parse a git remote URL into its components.
///
/// Extracts the provider, owner, repository name, and base URL from various
/// git remote URL formats.
///
/// ## Examples
///
/// ```
/// use sniff::remote::parse_remote_url;
///
/// // GitHub HTTPS
/// let parsed = parse_remote_url("https://github.com/rust-lang/cargo").unwrap();
/// assert_eq!(parsed.owner, "rust-lang");
/// assert_eq!(parsed.repo, "cargo");
///
/// // SSH with .git suffix
/// let parsed = parse_remote_url("git@github.com:owner/repo.git").unwrap();
/// assert_eq!(parsed.repo, "repo"); // .git is stripped
/// ```
///
/// ## Errors
///
/// Returns `SniffError::UnsupportedProvider` if the URL cannot be parsed
/// or points to an unsupported hosting provider.
pub fn parse_remote_url(url: &str) -> Result<ParsedRemoteUrl, SniffError> {
    // Try SSH format first: git@host:path or ssh://git@host/path
    if let Some(parsed) = try_parse_ssh(url) {
        return Ok(parsed);
    }

    // Try HTTPS format: https://host/path
    if let Some(parsed) = try_parse_https(url) {
        return Ok(parsed);
    }

    // Try git:// protocol
    if let Some(parsed) = try_parse_git_protocol(url) {
        return Ok(parsed);
    }

    Err(SniffError::UnsupportedProvider {
        url: url.to_string(),
    })
}

/// Try to parse an SSH-style remote URL.
///
/// Supports:
/// - `git@github.com:owner/repo.git`
/// - `ssh://git@github.com/owner/repo`
/// - `ssh://git@github.com:22/owner/repo`
fn try_parse_ssh(url: &str) -> Option<ParsedRemoteUrl> {
    // ssh:// protocol format
    if url.starts_with("ssh://") {
        let without_protocol = url.strip_prefix("ssh://")?;
        // Remove user@ prefix if present
        let without_user = if let Some(at_pos) = without_protocol.find('@') {
            &without_protocol[at_pos + 1..]
        } else {
            without_protocol
        };

        // Split host and path
        // Handle optional port: ssh://git@github.com:22/owner/repo
        let (host, path) = {
            let slash_pos = without_user.find('/')?;
            let host_part = &without_user[..slash_pos];
            // Remove port if present (e.g., "github.com:22" -> "github.com")
            let host = host_part.split(':').next().unwrap_or(host_part);
            let path = &without_user[slash_pos + 1..];
            (host, path)
        };

        return parse_host_and_path(host, path);
    }

    // SCP-style format: git@github.com:owner/repo.git
    if let Some(at_pos) = url.find('@') {
        let after_at = &url[at_pos + 1..];
        if let Some(colon_pos) = after_at.find(':') {
            let host = &after_at[..colon_pos];
            let path = &after_at[colon_pos + 1..];

            // Ensure path doesn't look like a port (all digits followed by /)
            // This distinguishes "git@github.com:22/path" from "git@github.com:owner/repo"
            let first_segment = path.split('/').next().unwrap_or(path);
            if first_segment.chars().all(|c| c.is_ascii_digit()) {
                // This is likely ssh://git@host:port/path without the ssh:// prefix
                // Skip the port and parse the rest
                if let Some(slash_pos) = path.find('/') {
                    let actual_path = &path[slash_pos + 1..];
                    return parse_host_and_path(host, actual_path);
                }
                return None;
            }

            return parse_host_and_path(host, path);
        }
    }

    None
}

/// Try to parse an HTTPS-style remote URL.
///
/// Supports:
/// - `https://github.com/owner/repo`
/// - `https://github.com/owner/repo.git`
/// - `http://github.com/owner/repo` (also supported)
fn try_parse_https(url: &str) -> Option<ParsedRemoteUrl> {
    let without_protocol = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;

    // Split host and path
    let slash_pos = without_protocol.find('/')?;
    let host = &without_protocol[..slash_pos];
    let path = &without_protocol[slash_pos + 1..];

    parse_host_and_path(host, path)
}

/// Try to parse a git:// protocol URL.
///
/// Supports:
/// - `git://github.com/owner/repo.git`
fn try_parse_git_protocol(url: &str) -> Option<ParsedRemoteUrl> {
    let without_protocol = url.strip_prefix("git://")?;

    // Split host and path
    let slash_pos = without_protocol.find('/')?;
    let host = &without_protocol[..slash_pos];
    let path = &without_protocol[slash_pos + 1..];

    parse_host_and_path(host, path)
}

/// Parse host and path into a `ParsedRemoteUrl`.
///
/// Detects the provider from the host and extracts owner/repo from the path.
fn parse_host_and_path(host: &str, path: &str) -> Option<ParsedRemoteUrl> {
    // Remove trailing slashes from path
    let path = path.trim_end_matches('/');

    if path.is_empty() {
        return None;
    }

    // Detect provider from host
    let (provider, base_url) = detect_provider(host);

    // Split path into segments
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if segments.is_empty() {
        return None;
    }

    // Extract owner and repo based on provider
    let (owner, repo) = match provider {
        GitProvider::GitLab => {
            // GitLab supports nested groups: group/subgroup/repo
            // The last segment is the repo, everything before is the owner/group path
            if segments.len() < 2 {
                return None;
            }
            let repo = segments.last()?;
            let owner = segments[..segments.len() - 1].join("/");
            (owner, *repo)
        }
        _ => {
            // GitHub, Bitbucket, Gitea: owner/repo format
            if segments.len() < 2 {
                return None;
            }
            (segments[0].to_string(), segments[1])
        }
    };

    // Strip .git suffix from repo name
    let repo = repo.strip_suffix(".git").unwrap_or(repo).to_string();

    if owner.is_empty() || repo.is_empty() {
        return None;
    }

    Some(ParsedRemoteUrl {
        provider,
        owner,
        repo,
        base_url,
    })
}

/// Detect the git hosting provider from the host name.
///
/// Returns the provider and an optional base URL for self-hosted instances.
fn detect_provider(host: &str) -> (GitProvider, Option<String>) {
    let host_lower = host.to_lowercase();

    // Well-known providers (no base URL needed)
    if host_lower == "github.com" || host_lower == "www.github.com" {
        return (GitProvider::GitHub, None);
    }

    if host_lower == "gitlab.com" || host_lower == "www.gitlab.com" {
        return (GitProvider::GitLab, None);
    }

    if host_lower == "bitbucket.org" || host_lower == "www.bitbucket.org" {
        return (GitProvider::Bitbucket, None);
    }

    // Self-hosted GitLab detection (common subdomain patterns)
    if host_lower.starts_with("gitlab.") || host_lower.contains(".gitlab.") {
        let base_url = format!("https://{}/api/v4", host);
        return (GitProvider::GitLab, Some(base_url));
    }

    // Self-hosted Gitea/Forgejo detection (common patterns)
    if host_lower.starts_with("gitea.")
        || host_lower.starts_with("forgejo.")
        || host_lower.contains(".gitea.")
        || host_lower.contains(".forgejo.")
        || host_lower.starts_with("git.")
        || host_lower.contains("codeberg.org")
    {
        let base_url = format!("https://{}/api/v1", host);
        return (GitProvider::Gitea, Some(base_url));
    }

    // Default to Gitea for unknown self-hosted instances
    // (most common self-hosted git server)
    let base_url = format!("https://{}/api/v1", host);
    (GitProvider::Gitea, Some(base_url))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────
    // GitHub URL Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn github_https_basic() {
        let parsed = parse_remote_url("https://github.com/rust-lang/cargo").unwrap();
        assert_eq!(parsed.provider, GitProvider::GitHub);
        assert_eq!(parsed.owner, "rust-lang");
        assert_eq!(parsed.repo, "cargo");
        assert_eq!(parsed.base_url, None);
    }

    #[test]
    fn github_https_with_git_suffix() {
        let parsed = parse_remote_url("https://github.com/rust-lang/cargo.git").unwrap();
        assert_eq!(parsed.provider, GitProvider::GitHub);
        assert_eq!(parsed.owner, "rust-lang");
        assert_eq!(parsed.repo, "cargo");
    }

    #[test]
    fn github_https_with_trailing_slash() {
        let parsed = parse_remote_url("https://github.com/rust-lang/cargo/").unwrap();
        assert_eq!(parsed.provider, GitProvider::GitHub);
        assert_eq!(parsed.owner, "rust-lang");
        assert_eq!(parsed.repo, "cargo");
    }

    #[test]
    fn github_ssh_scp_style() {
        let parsed = parse_remote_url("git@github.com:rust-lang/cargo.git").unwrap();
        assert_eq!(parsed.provider, GitProvider::GitHub);
        assert_eq!(parsed.owner, "rust-lang");
        assert_eq!(parsed.repo, "cargo");
        assert_eq!(parsed.base_url, None);
    }

    #[test]
    fn github_ssh_scp_style_no_git_suffix() {
        let parsed = parse_remote_url("git@github.com:rust-lang/cargo").unwrap();
        assert_eq!(parsed.provider, GitProvider::GitHub);
        assert_eq!(parsed.owner, "rust-lang");
        assert_eq!(parsed.repo, "cargo");
    }

    #[test]
    fn github_ssh_protocol() {
        let parsed = parse_remote_url("ssh://git@github.com/rust-lang/cargo").unwrap();
        assert_eq!(parsed.provider, GitProvider::GitHub);
        assert_eq!(parsed.owner, "rust-lang");
        assert_eq!(parsed.repo, "cargo");
    }

    #[test]
    fn github_ssh_protocol_with_port() {
        let parsed = parse_remote_url("ssh://git@github.com:22/rust-lang/cargo").unwrap();
        assert_eq!(parsed.provider, GitProvider::GitHub);
        assert_eq!(parsed.owner, "rust-lang");
        assert_eq!(parsed.repo, "cargo");
    }

    #[test]
    fn github_git_protocol() {
        let parsed = parse_remote_url("git://github.com/rust-lang/cargo.git").unwrap();
        assert_eq!(parsed.provider, GitProvider::GitHub);
        assert_eq!(parsed.owner, "rust-lang");
        assert_eq!(parsed.repo, "cargo");
    }

    #[test]
    fn github_http_protocol() {
        let parsed = parse_remote_url("http://github.com/rust-lang/cargo").unwrap();
        assert_eq!(parsed.provider, GitProvider::GitHub);
        assert_eq!(parsed.owner, "rust-lang");
        assert_eq!(parsed.repo, "cargo");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // GitLab URL Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn gitlab_https_basic() {
        let parsed = parse_remote_url("https://gitlab.com/inkscape/inkscape").unwrap();
        assert_eq!(parsed.provider, GitProvider::GitLab);
        assert_eq!(parsed.owner, "inkscape");
        assert_eq!(parsed.repo, "inkscape");
        assert_eq!(parsed.base_url, None);
    }

    #[test]
    fn gitlab_https_nested_groups() {
        let parsed = parse_remote_url("https://gitlab.com/group/subgroup/project").unwrap();
        assert_eq!(parsed.provider, GitProvider::GitLab);
        assert_eq!(parsed.owner, "group/subgroup");
        assert_eq!(parsed.repo, "project");
    }

    #[test]
    fn gitlab_https_deeply_nested_groups() {
        let parsed = parse_remote_url("https://gitlab.com/a/b/c/d/repo.git").unwrap();
        assert_eq!(parsed.provider, GitProvider::GitLab);
        assert_eq!(parsed.owner, "a/b/c/d");
        assert_eq!(parsed.repo, "repo");
    }

    #[test]
    fn gitlab_ssh_scp_style() {
        let parsed = parse_remote_url("git@gitlab.com:inkscape/inkscape.git").unwrap();
        assert_eq!(parsed.provider, GitProvider::GitLab);
        assert_eq!(parsed.owner, "inkscape");
        assert_eq!(parsed.repo, "inkscape");
    }

    #[test]
    fn gitlab_ssh_nested_groups() {
        let parsed = parse_remote_url("git@gitlab.com:group/subgroup/repo.git").unwrap();
        assert_eq!(parsed.provider, GitProvider::GitLab);
        assert_eq!(parsed.owner, "group/subgroup");
        assert_eq!(parsed.repo, "repo");
    }

    #[test]
    fn gitlab_self_hosted() {
        let parsed = parse_remote_url("https://gitlab.example.com/team/project").unwrap();
        assert_eq!(parsed.provider, GitProvider::GitLab);
        assert_eq!(parsed.owner, "team");
        assert_eq!(parsed.repo, "project");
        assert_eq!(
            parsed.base_url,
            Some("https://gitlab.example.com/api/v4".to_string())
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Bitbucket URL Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn bitbucket_https_basic() {
        let parsed = parse_remote_url("https://bitbucket.org/atlassian/python-bitbucket").unwrap();
        assert_eq!(parsed.provider, GitProvider::Bitbucket);
        assert_eq!(parsed.owner, "atlassian");
        assert_eq!(parsed.repo, "python-bitbucket");
        assert_eq!(parsed.base_url, None);
    }

    #[test]
    fn bitbucket_https_with_git_suffix() {
        let parsed =
            parse_remote_url("https://bitbucket.org/atlassian/python-bitbucket.git").unwrap();
        assert_eq!(parsed.provider, GitProvider::Bitbucket);
        assert_eq!(parsed.owner, "atlassian");
        assert_eq!(parsed.repo, "python-bitbucket");
    }

    #[test]
    fn bitbucket_ssh_scp_style() {
        let parsed = parse_remote_url("git@bitbucket.org:atlassian/python-bitbucket.git").unwrap();
        assert_eq!(parsed.provider, GitProvider::Bitbucket);
        assert_eq!(parsed.owner, "atlassian");
        assert_eq!(parsed.repo, "python-bitbucket");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Gitea / Self-hosted URL Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn gitea_self_hosted() {
        let parsed = parse_remote_url("https://gitea.example.com/org/project").unwrap();
        assert_eq!(parsed.provider, GitProvider::Gitea);
        assert_eq!(parsed.owner, "org");
        assert_eq!(parsed.repo, "project");
        assert_eq!(
            parsed.base_url,
            Some("https://gitea.example.com/api/v1".to_string())
        );
    }

    #[test]
    fn forgejo_self_hosted() {
        let parsed = parse_remote_url("https://forgejo.example.com/user/repo").unwrap();
        assert_eq!(parsed.provider, GitProvider::Gitea);
        assert_eq!(parsed.owner, "user");
        assert_eq!(parsed.repo, "repo");
    }

    #[test]
    fn codeberg_url() {
        let parsed = parse_remote_url("https://codeberg.org/forgejo/forgejo").unwrap();
        assert_eq!(parsed.provider, GitProvider::Gitea);
        assert_eq!(parsed.owner, "forgejo");
        assert_eq!(parsed.repo, "forgejo");
        assert_eq!(
            parsed.base_url,
            Some("https://codeberg.org/api/v1".to_string())
        );
    }

    #[test]
    fn git_subdomain_self_hosted() {
        let parsed = parse_remote_url("https://git.example.com/team/project").unwrap();
        assert_eq!(parsed.provider, GitProvider::Gitea);
        assert_eq!(parsed.owner, "team");
        assert_eq!(parsed.repo, "project");
    }

    #[test]
    fn unknown_host_defaults_to_gitea() {
        let parsed = parse_remote_url("https://code.mycompany.com/dev/app").unwrap();
        assert_eq!(parsed.provider, GitProvider::Gitea);
        assert_eq!(parsed.owner, "dev");
        assert_eq!(parsed.repo, "app");
        assert_eq!(
            parsed.base_url,
            Some("https://code.mycompany.com/api/v1".to_string())
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // .git Suffix Handling Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn strips_git_suffix_https() {
        let parsed = parse_remote_url("https://github.com/owner/repo.git").unwrap();
        assert_eq!(parsed.repo, "repo");
    }

    #[test]
    fn strips_git_suffix_ssh() {
        let parsed = parse_remote_url("git@github.com:owner/repo.git").unwrap();
        assert_eq!(parsed.repo, "repo");
    }

    #[test]
    fn preserves_repo_with_git_in_name() {
        // Repo name that contains "git" but doesn't end with ".git"
        let parsed = parse_remote_url("https://github.com/owner/git-hooks").unwrap();
        assert_eq!(parsed.repo, "git-hooks");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Error Cases
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn error_empty_url() {
        let result = parse_remote_url("");
        assert!(result.is_err());
    }

    #[test]
    fn error_invalid_url() {
        let result = parse_remote_url("not-a-url");
        assert!(result.is_err());
    }

    #[test]
    fn error_missing_repo() {
        let result = parse_remote_url("https://github.com/owner");
        assert!(result.is_err());
    }

    #[test]
    fn error_missing_owner_and_repo() {
        let result = parse_remote_url("https://github.com/");
        assert!(result.is_err());
    }

    #[test]
    fn error_only_host() {
        let result = parse_remote_url("https://github.com");
        assert!(result.is_err());
    }

    #[test]
    fn error_malformed_ssh() {
        let result = parse_remote_url("git@github.com");
        assert!(result.is_err());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Edge Cases
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn handles_www_prefix() {
        let parsed = parse_remote_url("https://www.github.com/owner/repo").unwrap();
        assert_eq!(parsed.provider, GitProvider::GitHub);
        assert_eq!(parsed.owner, "owner");
        assert_eq!(parsed.repo, "repo");
    }

    #[test]
    fn case_insensitive_host() {
        let parsed = parse_remote_url("https://GITHUB.COM/owner/repo").unwrap();
        assert_eq!(parsed.provider, GitProvider::GitHub);
    }

    #[test]
    fn preserves_owner_case() {
        let parsed = parse_remote_url("https://github.com/MyOrg/MyRepo").unwrap();
        assert_eq!(parsed.owner, "MyOrg");
        assert_eq!(parsed.repo, "MyRepo");
    }
}
