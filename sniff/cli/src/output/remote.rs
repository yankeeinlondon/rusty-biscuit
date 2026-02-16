//! Output formatting for remote repository reports.

use sniff::remote::{CiCdInfo, DocumentCategory, DocumentRef, PullRequestInfo, RemoteReport};

use super::format_number;

/// Print remote report as formatted text.
pub fn print_remote_text(report: &RemoteReport) {
    let meta = &report.metadata;

    // Header: Repository name and provider
    println!("Repository: {}", meta.full_name);
    println!("Provider: {}", report.provider);

    if let Some(ref desc) = meta.description {
        println!("Description: {}", desc);
    }

    // Stats line
    let mut stats = Vec::new();
    if let Some(stars) = meta.stars {
        stats.push(format!("Stars: {}", format_number(stars as usize)));
    }
    if let Some(forks) = meta.forks {
        stats.push(format!("Forks: {}", format_number(forks as usize)));
    }
    if let Some(issues) = meta.open_issues {
        stats.push(format!("Issues: {}", format_number(issues as usize)));
    }
    if !stats.is_empty() {
        println!("{}", stats.join("  "));
    }

    // License
    if let Some(ref license) = meta.license {
        println!("License: {}", license.name);
    }

    // Topics
    if !meta.topics.is_empty() {
        println!("Topics: {}", meta.topics.join(", "));
    }

    // Blank line before sections
    println!();

    // Documents section
    print_documents(&report.documents);

    // CI/CD section
    print_cicd(&report.ci_cd);

    // Recent PRs
    print_pull_requests(&report.pull_requests);

    // Key URLs
    print_key_urls(report);
}

/// Print document references.
fn print_documents(docs: &[DocumentRef]) {
    if docs.is_empty() {
        return;
    }

    // Group by category
    let readmes: Vec<_> = docs
        .iter()
        .filter(|d| d.category == DocumentCategory::Readme)
        .collect();
    let doc_folder: Vec<_> = docs
        .iter()
        .filter(|d| d.category == DocumentCategory::DocsFolder)
        .collect();
    let other_docs: Vec<_> = docs
        .iter()
        .filter(|d| d.category == DocumentCategory::Other)
        .collect();

    let mut parts = Vec::new();

    // List READMEs by name
    for doc in &readmes {
        parts.push(doc.path.clone());
    }

    // Add docs folder indicator
    if !doc_folder.is_empty() {
        parts.push(format!("docs/ ({} files)", doc_folder.len()));
    }

    // List other docs by name (e.g., CHANGELOG.md, CONTRIBUTING.md)
    for doc in &other_docs {
        if doc.path.contains('/') {
            continue; // Skip nested files
        }
        parts.push(doc.path.clone());
    }

    if !parts.is_empty() {
        println!("Documents: {}", parts.join(", "));
    }
}

/// Print CI/CD information.
fn print_cicd(cicd: &[CiCdInfo]) {
    if cicd.is_empty() {
        return;
    }

    let ci = &cicd[0];
    let path_info = ci
        .config_path
        .as_ref()
        .map(|p| format!(" ({})", p))
        .unwrap_or_default();

    println!("CI/CD: {}{}", ci.provider, path_info);
}

/// Print recent pull requests.
fn print_pull_requests(prs: &[PullRequestInfo]) {
    if prs.is_empty() {
        return;
    }

    println!();
    println!("Recent PRs ({}):", prs.len());

    for pr in prs.iter().take(5) {
        let state = match pr.state.as_str() {
            "open" => "open",
            "closed" if pr.merged_at.is_some() => "merged",
            "closed" => "closed",
            _ => &pr.state,
        };

        let draft = if pr.draft { " [draft]" } else { "" };
        println!("  #{} {}{} ({})", pr.number, pr.title, draft, state);
    }
}

/// Print key URLs section.
fn print_key_urls(report: &RemoteReport) {
    let urls = &report.key_urls;

    println!();
    println!("URLs:");
    println!("  Repository: {}", urls.repo);

    if let Some(ref homepage) = urls.homepage {
        println!("  Homepage: {}", homepage);
    }
    if let Some(ref issues) = urls.issues {
        println!("  Issues: {}", issues);
    }
    if let Some(ref prs) = urls.pull_requests {
        println!("  Pull Requests: {}", prs);
    }
}

/// Print remote report as JSON.
pub fn print_remote_json(report: &RemoteReport) -> serde_json::Result<()> {
    println!("{}", serde_json::to_string_pretty(report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sniff::remote::{
        GitProvider, KeyUrls, RepoMetadata, TagsAndReleases,
    };

    fn make_test_report() -> RemoteReport {
        RemoteReport {
            provider: GitProvider::GitHub,
            metadata: RepoMetadata {
                name: "repo".to_string(),
                full_name: "owner/repo".to_string(),
                description: Some("A test repository".to_string()),
                private: false,
                default_branch: "main".to_string(),
                language: Some("Rust".to_string()),
                stars: Some(1234),
                forks: Some(56),
                open_issues: Some(12),
                archived: false,
                created_at: None,
                updated_at: None,
                pushed_at: None,
                license: None,
                topics: vec!["rust".to_string(), "cli".to_string()],
                has_issues: Some(true),
                has_wiki: Some(false),
                homepage: None,
                html_url: "https://github.com/owner/repo".to_string(),
            },
            org_info: None,
            documents: vec![],
            pull_requests: vec![],
            issues: vec![],
            tags_and_releases: TagsAndReleases::default(),
            ci_cd: vec![],
            org_repos: vec![],
            key_urls: KeyUrls {
                repo: "https://github.com/owner/repo".to_string(),
                homepage: None,
                docs: None,
                issues: Some("https://github.com/owner/repo/issues".to_string()),
                pull_requests: Some("https://github.com/owner/repo/pulls".to_string()),
                wiki: None,
                ci_cd: None,
                insights: None,
                releases: None,
                settings: None,
            },
        }
    }

    #[test]
    fn test_print_remote_json_succeeds() {
        let report = make_test_report();
        let result = print_remote_json(&report);
        assert!(result.is_ok());
    }
}
