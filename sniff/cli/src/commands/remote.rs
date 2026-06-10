use sniff::SniffError;
use sniff::remote::PullRequestState;
use sniff::remote::{DocumentCategory, GitRemote, RemoteRepoProvider, RemoteReport};

use crate::output;
use crate::perf::CliPerf;

/// Handle `owner/repo` shorthand by probing configured providers.
pub(super) async fn handle_shorthand(
    shorthand: &str,
    json: bool,
    plain: bool,
    verbose: u8,
    perf: &CliPerf,
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, repo) = shorthand.split_once('/').expect("already validated");
    let remote = GitRemote::from_shorthand(owner, repo).await?;
    let report = remote.fetch_report(owner, repo).await?;

    if json {
        output::print_remote_json(&report, perf.build_report().as_ref())?;
    } else {
        let readme = fetch_readme(&report, &remote, owner, repo, verbose).await;
        let rendered = output::render_remote_text(&report, readme.as_deref());
        output::emit_text(&rendered, plain);
    }

    Ok(())
}

/// Handle remote URL inspection (from `sniff git <remote>`).
///
/// Parses the URL, detects the provider, fetches the report, and outputs it.
pub(super) async fn handle_remote_url(
    url: &str,
    json: bool,
    plain: bool,
    verbose: u8,
    perf: &CliPerf,
) -> Result<(), Box<dyn std::error::Error>> {
    let remote = GitRemote::from_url(url)?;
    let parsed = GitRemote::parse_url(url)?;
    let report = remote.fetch_report(&parsed.owner, &parsed.repo).await?;

    if json {
        output::print_remote_json(&report, perf.build_report().as_ref())?;
    } else {
        let readme = fetch_readme(&report, &remote, &parsed.owner, &parsed.repo, verbose).await;
        let rendered = output::render_remote_text(&report, readme.as_deref());
        output::emit_text(&rendered, plain);
    }

    Ok(())
}

/// Handle `sniff repo pr` — list pull requests for the current repo's remote.
///
/// Discovers the git repo, resolves the preferred remote, fetches PRs, and
/// renders them as JSON or text (table / verbose block).
pub(super) async fn handle_pr_command(
    status: PullRequestState,
    json: bool,
    plain: bool,
    verbose: u8,
    base_dir: Option<&std::path::Path>,
    perf: &CliPerf,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = base_dir.unwrap_or_else(|| std::path::Path::new("."));

    // 1. Resolve the preferred remote URL (origin first, then first configured
    //    remote) via the backend-neutral library API.
    let remote_url = sniff::filesystem::preferred_remote_url(dir)?
        .ok_or("No git remotes found for this repository")?;

    // 3. Parse the remote URL and construct the provider
    let parsed = GitRemote::parse_url(&remote_url).map_err(|e| {
        format!(
            "Unsupported provider for remote URL '{}': {}",
            remote_url, e
        )
    })?;
    let remote = GitRemote::from_url(&remote_url).map_err(|e| {
        format!(
            "Unsupported provider for remote URL '{}': {}",
            remote_url, e
        )
    })?;

    // 4. Fetch pull requests
    let prs = match remote
        .list_pull_requests(&parsed.owner, &parsed.repo, status)
        .await
    {
        Ok(prs) => prs,
        Err(SniffError::MissingCredentials { provider, env_var }) => {
            return Err(format!(
                "{} requires credentials for this resource: set the {} environment variable",
                provider, env_var
            )
            .into());
        }
        Err(SniffError::InvalidCredentials { provider, message }) => {
            return Err(format!(
                "{} rejected credentials: {} (check your token environment variable)",
                provider, message
            )
            .into());
        }
        Err(SniffError::RateLimited {
            provider,
            retry_after,
        }) => {
            let retry_msg = retry_after
                .map(|s| format!(", retry after {}s", s))
                .unwrap_or_default();
            return Err(format!("Rate limited by {} API{}", provider, retry_msg).into());
        }
        Err(SniffError::RemoteApi {
            status,
            provider,
            message,
        }) => {
            return Err(format!("{} API error (HTTP {}): {}", provider, status, message).into());
        }
        Err(SniffError::RemoteInit { provider, message }) => {
            return Err(format!(
                "Failed to initialize {} remote provider: {}",
                provider, message
            )
            .into());
        }
        Err(e) => {
            return Err(format!(
                "Failed to list pull requests from {}: {}",
                parsed.provider.display_name(),
                e
            )
            .into());
        }
    };

    // 5. Output
    if json {
        output::print_json_value(serde_json::to_value(&prs)?, perf.build_report().as_ref());
    } else {
        if prs.is_empty() {
            let rendered = output::render_pull_requests_empty(status);
            output::emit_text(&rendered, plain);
        } else if verbose > 0 {
            let rendered = output::render_pull_requests_verbose(&prs);
            output::emit_text(&rendered, plain);
        } else {
            let rendered = output::render_pull_requests_table(&prs);
            output::emit_text(&rendered, plain);
        }
        perf.emit_for_json(None);
    }

    Ok(())
}

/// Fetch the README content when verbose mode is enabled.
///
/// Looks for the first `DocumentCategory::Readme` entry in the report's
/// documents and fetches its content via the provider API.
pub(super) async fn fetch_readme(
    report: &RemoteReport,
    remote: &GitRemote,
    owner: &str,
    repo: &str,
    verbose: u8,
) -> Option<String> {
    if verbose == 0 {
        return None;
    }
    let readme_path = report
        .documents
        .iter()
        .find(|d| d.category == DocumentCategory::Readme)
        .map(|d| d.path.as_str())?;
    remote.get_file_content(owner, repo, readme_path).await.ok()
}
