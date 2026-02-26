//! Output formatting for remote repository reports.
//!
//! Uses biscuit-terminal's Prose, Table, and UnorderedList components for
//! rich terminal rendering with styled headings, structured tables, and
//! categorized lists.

use biscuit_terminal::prelude::*;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::terminal::{TerminalOptions, write_terminal};

use sniff::remote::{
    CiCdInfo, DocumentCategory, DocumentRef, IssueInfo, PullRequestInfo, RemoteReport,
};

use super::format_number;

/// Print remote report as formatted text using biscuit-terminal components.
///
/// When `readme_content` is provided, renders it as markdown after the
/// standard report output using darkmatter's terminal renderer.
pub fn print_remote_text(report: &RemoteReport, readme_content: Option<&str>) {
    let term = Terminal::default();
    let meta = &report.metadata;

    // === Header ===
    let header = format!(
        "<b>{}</b> <dim>on {}</dim>",
        meta.full_name,
        report.provider.display_name()
    );
    print!("{}", Prose::new(&header).display(&term));

    if let Some(ref desc) = meta.description {
        print!("{}", Prose::new(desc).display(&term));
    }

    // === Stats line ===
    let mut stats_parts = Vec::new();
    if let Some(stars) = meta.stars {
        stats_parts.push(format!(
            "<yellow>★</yellow> {}",
            format_number(stars as usize)
        ));
    }
    if let Some(forks) = meta.forks {
        stats_parts.push(format!("<dim>⑂</dim> {}", format_number(forks as usize)));
    }
    if let Some(issues) = meta.open_issues {
        stats_parts.push(format!(
            "<dim>◎</dim> {} open",
            format_number(issues as usize)
        ));
    }
    if !stats_parts.is_empty() {
        print!("{}", Prose::new(stats_parts.join("  ")).display(&term));
    }

    // === Metadata details ===
    let mut detail_items = Vec::new();

    if let Some(ref lang) = meta.language {
        detail_items.push(format!("<b>Language:</b> {lang}"));
    }
    if let Some(ref license) = meta.license {
        let name = license
            .spdx_id
            .as_deref()
            .unwrap_or(license.name.as_str());
        detail_items.push(format!("<b>License:</b> {name}"));
    }
    detail_items.push(format!("<b>Branch:</b> {}", meta.default_branch));
    if meta.archived {
        detail_items.push("<yellow><b>Archived</b></yellow>".to_string());
    }
    if meta.private {
        detail_items.push("<dim><b>Private</b></dim>".to_string());
    }

    if !detail_items.is_empty() {
        let rendered: Vec<String> = detail_items
            .iter()
            .map(|item| Prose::new(item).fallback_render(&term))
            .collect();
        let list = UnorderedList::new(rendered).with_bullet("  ");
        print!("{}", list.display(&term));
    }

    // === Topics ===
    if !meta.topics.is_empty() {
        let topics_str = meta
            .topics
            .iter()
            .map(|t| format!("<cyan>{t}</cyan>"))
            .collect::<Vec<_>>()
            .join("  ");
        println!();
        print!("{}", Prose::new(&topics_str).display(&term));
    }

    // === Documents ===
    print_documents(&report.documents, &term);

    // === CI/CD ===
    print_cicd(&report.ci_cd, &term);

    // === Pull Requests ===
    print_pull_requests(&report.pull_requests, &term);

    // === Issues ===
    print_issues(&report.issues, &term);

    // === Tags & Releases ===
    print_tags(&report.tags_and_releases.tags, &term);

    // === Key URLs ===
    print_key_urls(report, &term);

    // === README ===
    if let Some(content) = readme_content {
        println!();
        let md: Markdown = content.into();
        let mut stdout = std::io::stdout().lock();
        let _ = write_terminal(&mut stdout, &md, TerminalOptions::default());
    }
}

/// Print document references as a categorized list.
fn print_documents(docs: &[DocumentRef], term: &Terminal) {
    if docs.is_empty() {
        return;
    }

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

    let mut items = Vec::new();

    for doc in &readmes {
        items.push(Prose::new(format!("<b>{}</b>", doc.path)).fallback_render(term));
    }
    if !doc_folder.is_empty() {
        items.push(
            Prose::new(format!("<dim>docs/</dim> ({} files)", doc_folder.len()))
                .fallback_render(term),
        );
    }
    for doc in &other_docs {
        if !doc.path.contains('/') {
            items.push(Prose::new(format!("<dim>{}</dim>", doc.path)).fallback_render(term));
        }
    }

    if !items.is_empty() {
        println!();
        print!("{}", Prose::new("<b><u>Documents</u></b>").display(term));
        print!("{}", UnorderedList::new(items).display(term));
    }
}

/// Print CI/CD information.
fn print_cicd(cicd: &[CiCdInfo], term: &Terminal) {
    if cicd.is_empty() {
        return;
    }

    println!();
    print!("{}", Prose::new("<b><u>CI/CD</u></b>").display(term));

    let items: Vec<String> = cicd
        .iter()
        .map(|ci| {
            let path_info = ci
                .config_path
                .as_ref()
                .map(|p| format!(" <dim>({})</dim>", p))
                .unwrap_or_default();
            Prose::new(format!("<b>{}</b>{}", ci.provider, path_info)).fallback_render(term)
        })
        .collect();

    print!("{}", UnorderedList::new(items).display(term));
}

/// Print recent pull requests as a table.
fn print_pull_requests(prs: &[PullRequestInfo], term: &Terminal) {
    if prs.is_empty() {
        return;
    }

    println!();
    let heading = format!("<b><u>Pull Requests</u></b> <dim>({})</dim>", prs.len());
    print!("{}", Prose::new(&heading).display(term));

    let columns = vec![
        TableColumn::new("#").with_min_width(4),
        TableColumn::new("Title"),
        TableColumn::new("State").with_min_width(6),
        TableColumn::new("Author").with_min_width(8),
    ];

    let mut table = Table::new().with_columns(columns);

    for pr in prs.iter().take(10) {
        let state_display = pr_state_display(pr);
        let title = if pr.draft {
            format!("{} [draft]", pr.title)
        } else {
            pr.title.clone()
        };

        table.add_row(vec![
            format!("#{}", pr.number).into(),
            title.into(),
            state_display.into(),
            pr.author.clone().into(),
        ]);
    }

    print!("{}", table.display(term));
}

/// Print recent issues as a table.
fn print_issues(issues: &[IssueInfo], term: &Terminal) {
    if issues.is_empty() {
        return;
    }

    println!();
    let heading = format!("<b><u>Issues</u></b> <dim>({})</dim>", issues.len());
    print!("{}", Prose::new(&heading).display(term));

    let columns = vec![
        TableColumn::new("#").with_min_width(4),
        TableColumn::new("Title"),
        TableColumn::new("State").with_min_width(6),
        TableColumn::new("Author").with_min_width(8),
    ];

    let mut table = Table::new().with_columns(columns);

    for issue in issues.iter().take(10) {
        table.add_row(vec![
            format!("#{}", issue.number).into(),
            issue.title.clone().into(),
            issue.state.clone().into(),
            issue.author.clone().into(),
        ]);
    }

    print!("{}", table.display(term));
}

/// Print recent tags.
fn print_tags(tags: &[sniff::remote::TagInfo], term: &Terminal) {
    if tags.is_empty() {
        return;
    }

    println!();
    let heading = format!("<b><u>Tags</u></b> <dim>({})</dim>", tags.len());
    print!("{}", Prose::new(&heading).display(term));

    let items: Vec<String> = tags
        .iter()
        .take(5)
        .map(|tag| {
            let annotated = if tag.annotated {
                " <dim>annotated</dim>"
            } else {
                ""
            };
            Prose::new(format!("<b>{}</b>{}", tag.name, annotated)).fallback_render(term)
        })
        .collect();

    print!("{}", UnorderedList::new(items).display(term));
}

/// Print key URLs as a styled list.
fn print_key_urls(report: &RemoteReport, term: &Terminal) {
    let urls = &report.key_urls;
    let mut items = Vec::new();

    items.push(
        Prose::new(format!(
            "<b>Repository:</b> <a href=\"{}\">{}</a>",
            urls.repo, urls.repo
        ))
        .fallback_render(term),
    );

    if let Some(ref homepage) = urls.homepage {
        items.push(
            Prose::new(format!(
                "<b>Homepage:</b> <a href=\"{homepage}\">{homepage}</a>"
            ))
            .fallback_render(term),
        );
    }
    if let Some(ref issues) = urls.issues {
        items.push(
            Prose::new(format!("<b>Issues:</b> <a href=\"{issues}\">{issues}</a>"))
                .fallback_render(term),
        );
    }
    if let Some(ref prs) = urls.pull_requests {
        items.push(
            Prose::new(format!(
                "<b>Pull Requests:</b> <a href=\"{prs}\">{prs}</a>"
            ))
            .fallback_render(term),
        );
    }
    if let Some(ref releases) = urls.releases {
        items.push(
            Prose::new(format!(
                "<b>Releases:</b> <a href=\"{releases}\">{releases}</a>"
            ))
            .fallback_render(term),
        );
    }
    if let Some(ref wiki) = urls.wiki {
        items.push(
            Prose::new(format!("<b>Wiki:</b> <a href=\"{wiki}\">{wiki}</a>"))
                .fallback_render(term),
        );
    }

    if !items.is_empty() {
        println!();
        print!("{}", Prose::new("<b><u>URLs</u></b>").display(term));
        print!("{}", UnorderedList::new(items).display(term));
    }
}

/// Format PR state with semantic meaning.
fn pr_state_display(pr: &PullRequestInfo) -> String {
    match pr.state.as_str() {
        "open" => "open".to_string(),
        "closed" if pr.merged_at.is_some() => "merged".to_string(),
        "closed" => "closed".to_string(),
        other => other.to_string(),
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
    use sniff::remote::{GitProvider, KeyUrls, RepoMetadata, TagsAndReleases};

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
