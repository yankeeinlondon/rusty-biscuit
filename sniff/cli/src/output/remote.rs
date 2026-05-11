//! Output formatting for remote repository reports.
//!
//! Uses biscuit-terminal's Prose, Table, and UnorderedList components for
//! rich terminal rendering with styled headings, structured tables, and
//! categorized lists.

use std::fmt::Write;

use biscuit_terminal::prelude::*;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::terminal::{TerminalOptions, write_terminal};

use sniff::remote::{
    CiCdInfo, DocumentCategory, DocumentRef, IssueInfo, PullRequestInfo, RemoteReport,
};

use super::format_number;

/// Render remote report as formatted text using biscuit-terminal components.
///
/// When `readme_content` is provided, renders it as markdown after the
/// standard report output using darkmatter's terminal renderer.
///
/// ## Returns
///
/// A String containing the complete formatted report.
pub fn render_remote_text(report: &RemoteReport, readme_content: Option<&str>) -> String {
    let mut out = String::new();
    let term = Terminal::default();
    let meta = &report.metadata;

    // === Header ===
    let header = format!(
        "<b>{}</b> <dim>on {}</dim>",
        meta.full_name,
        report.provider.display_name()
    );
    write!(out, "{}", Prose::new(&header).display(&term)).unwrap();

    if let Some(ref desc) = meta.description {
        write!(out, "{}", Prose::new(desc).display(&term)).unwrap();
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
        write!(out, "{}", Prose::new(stats_parts.join("  ")).display(&term)).unwrap();
    }

    // === Metadata details ===
    let mut detail_items = Vec::new();

    if let Some(ref lang) = meta.language {
        detail_items.push(format!("<b>Language:</b> {lang}"));
    }
    if let Some(ref license) = meta.license {
        let name = license.spdx_id.as_deref().unwrap_or(license.name.as_str());
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
            .map(|item| Prose::new(item).render(&term))
            .collect();
        let list = UnorderedList::new(rendered).with_bullet("  ");
        write!(out, "{}", list.display(&term)).unwrap();
    }

    // === Topics ===
    if !meta.topics.is_empty() {
        let topics_str = meta
            .topics
            .iter()
            .map(|t| format!("<cyan>{t}</cyan>"))
            .collect::<Vec<_>>()
            .join("  ");
        writeln!(out).unwrap();
        write!(out, "{}", Prose::new(&topics_str).display(&term)).unwrap();
    }

    // === Documents ===
    out.push_str(&render_documents(&report.documents, &term));

    // === CI/CD ===
    out.push_str(&render_cicd(&report.ci_cd, &term));

    // === Pull Requests ===
    out.push_str(&render_pull_requests(&report.pull_requests, &term));

    // === Issues ===
    out.push_str(&render_issues(&report.issues, &term));

    // === Tags & Releases ===
    out.push_str(&render_tags(&report.tags_and_releases.tags, &term));

    // === Key URLs ===
    out.push_str(&render_key_urls(report, &term));

    // === README ===
    if let Some(content) = readme_content {
        writeln!(out).unwrap();
        let md: Markdown = content.into();
        let mut buffer = Vec::new();
        let _ = write_terminal(&mut buffer, &md, TerminalOptions::default());
        out.push_str(&String::from_utf8_lossy(&buffer));
    }

    out
}

/// Render document references as a categorized list.
fn render_documents(docs: &[DocumentRef], term: &Terminal) -> String {
    if docs.is_empty() {
        return String::new();
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
        items.push(Prose::new(format!("<b>{}</b>", doc.path)).render(term));
    }
    if !doc_folder.is_empty() {
        items.push(
            Prose::new(format!("<dim>docs/</dim> ({} files)", doc_folder.len())).render(term),
        );
    }
    for doc in &other_docs {
        if !doc.path.contains('/') {
            items.push(Prose::new(format!("<dim>{}</dim>", doc.path)).render(term));
        }
    }

    if !items.is_empty() {
        let mut out = String::new();
        writeln!(out).unwrap();
        write!(
            out,
            "{}",
            Prose::new("<b><u>Documents</u></b>").display(term)
        )
        .unwrap();
        write!(out, "{}", UnorderedList::new(items).display(term)).unwrap();
        out
    } else {
        String::new()
    }
}

/// Render CI/CD information.
fn render_cicd(cicd: &[CiCdInfo], term: &Terminal) -> String {
    if cicd.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    writeln!(out).unwrap();
    write!(out, "{}", Prose::new("<b><u>CI/CD</u></b>").display(term)).unwrap();

    let items: Vec<String> = cicd
        .iter()
        .map(|ci| {
            let path_info = ci
                .config_path
                .as_ref()
                .map(|p| format!(" <dim>({})</dim>", p))
                .unwrap_or_default();
            Prose::new(format!("<b>{}</b>{}", ci.provider, path_info)).render(term)
        })
        .collect();

    write!(out, "{}", UnorderedList::new(items).display(term)).unwrap();
    out
}

/// Render recent pull requests as a table.
fn render_pull_requests(prs: &[PullRequestInfo], term: &Terminal) -> String {
    if prs.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    writeln!(out).unwrap();
    let heading = format!("<b><u>Pull Requests</u></b> <dim>({})</dim>", prs.len());
    write!(out, "{}", Prose::new(&heading).display(term)).unwrap();

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

    write!(out, "{}", table.display(term)).unwrap();
    out
}

/// Render recent issues as a table.
fn render_issues(issues: &[IssueInfo], term: &Terminal) -> String {
    if issues.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    writeln!(out).unwrap();
    let heading = format!("<b><u>Issues</u></b> <dim>({})</dim>", issues.len());
    write!(out, "{}", Prose::new(&heading).display(term)).unwrap();

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

    write!(out, "{}", table.display(term)).unwrap();
    out
}

/// Render recent tags.
fn render_tags(tags: &[sniff::remote::TagInfo], term: &Terminal) -> String {
    if tags.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    writeln!(out).unwrap();
    let heading = format!("<b><u>Tags</u></b> <dim>({})</dim>", tags.len());
    write!(out, "{}", Prose::new(&heading).display(term)).unwrap();

    let items: Vec<String> = tags
        .iter()
        .take(5)
        .map(|tag| {
            let annotated = if tag.annotated {
                " <dim>annotated</dim>"
            } else {
                ""
            };
            Prose::new(format!("<b>{}</b>{}", tag.name, annotated)).render(term)
        })
        .collect();

    write!(out, "{}", UnorderedList::new(items).display(term)).unwrap();
    out
}

/// Render key URLs as a styled list.
fn render_key_urls(report: &RemoteReport, term: &Terminal) -> String {
    let urls = &report.key_urls;
    let mut items = Vec::new();

    items.push(
        Prose::new(format!(
            "<b>Repository:</b> <a href=\"{}\">{}</a>",
            urls.repo, urls.repo
        ))
        .render(term),
    );

    if let Some(ref homepage) = urls.homepage {
        items.push(
            Prose::new(format!(
                "<b>Homepage:</b> <a href=\"{homepage}\">{homepage}</a>"
            ))
            .render(term),
        );
    }
    if let Some(ref issues) = urls.issues {
        items.push(
            Prose::new(format!("<b>Issues:</b> <a href=\"{issues}\">{issues}</a>")).render(term),
        );
    }
    if let Some(ref prs) = urls.pull_requests {
        items.push(
            Prose::new(format!("<b>Pull Requests:</b> <a href=\"{prs}\">{prs}</a>")).render(term),
        );
    }
    if let Some(ref releases) = urls.releases {
        items.push(
            Prose::new(format!(
                "<b>Releases:</b> <a href=\"{releases}\">{releases}</a>"
            ))
            .render(term),
        );
    }
    if let Some(ref wiki) = urls.wiki {
        items.push(Prose::new(format!("<b>Wiki:</b> <a href=\"{wiki}\">{wiki}</a>")).render(term));
    }

    if !items.is_empty() {
        let mut out = String::new();
        writeln!(out).unwrap();
        write!(out, "{}", Prose::new("<b><u>URLs</u></b>").display(term)).unwrap();
        write!(out, "{}", UnorderedList::new(items).display(term)).unwrap();
        out
    } else {
        String::new()
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

/// Render a list of pull requests as a compact table for `sniff repo pr`.
///
/// Columns: ID, Title, Author, State.
pub fn render_pull_requests_table(prs: &[PullRequestInfo]) -> String {
    if prs.is_empty() {
        return String::new();
    }

    let term = Terminal::default();
    let mut out = String::new();

    let heading = format!("<b><u>Pull Requests</u></b> <dim>({})</dim>", prs.len());
    write!(out, "{}", Prose::new(&heading).display(&term)).unwrap();

    let columns = vec![
        TableColumn::new("#").with_min_width(4),
        TableColumn::new("Title"),
        TableColumn::new("State").with_min_width(6),
        TableColumn::new("Author").with_min_width(8),
    ];

    let mut table = Table::new().with_columns(columns);

    for pr in prs.iter().take(50) {
        let state_display = pr_state_display(pr);
        let title = if pr.draft {
            format!("{} <dim>[draft]</dim>", pr.title)
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

    write!(out, "{}", table.display(&term)).unwrap();
    out
}

/// Render a list of pull requests as verbose blocks for `sniff repo pr -v`.
///
/// Each PR is rendered as a block with number/title, author, normalized status
/// with draft marker, source and target branches, labels, created date, URL,
/// and description body when present.
pub fn render_pull_requests_verbose(prs: &[PullRequestInfo]) -> String {
    if prs.is_empty() {
        return String::new();
    }

    let term = Terminal::default();
    let mut out = String::new();

    let heading = format!("<b><u>Pull Requests</u></b> <dim>({})</dim>", prs.len());
    write!(out, "{}", Prose::new(&heading).display(&term)).unwrap();

    for pr in prs.iter().take(50) {
        writeln!(out).unwrap();

        // Title line
        let draft_marker = if pr.draft { " <dim>[draft]</dim>" } else { "" };
        let title_line = format!("<b>#{}</b> {}{}", pr.number, pr.title, draft_marker);
        write!(out, "{}", Prose::new(&title_line).display(&term)).unwrap();

        // Meta line
        let state_display = pr_state_display(pr);
        let mut meta_parts = vec![
            format!("<dim>by</dim> <b>{}</b>", pr.author),
            format!("<dim>{}</dim>", state_display),
        ];

        if let Some(ref src) = pr.source_branch
            && let Some(ref tgt) = pr.target_branch
        {
            meta_parts.push(format!("<dim>{} → {}</dim>", src, tgt));
        }

        meta_parts.push(format!("<dim>{}</dim>", pr.created_at));
        write!(out, "{}", Prose::new(meta_parts.join("  ")).display(&term)).unwrap();

        // Labels (omitted entirely when empty, mirroring how the body is handled)
        if !pr.labels.is_empty() {
            let labels_str = pr
                .labels
                .iter()
                .map(|l| format!("<cyan>{l}</cyan>"))
                .collect::<Vec<_>>()
                .join(", ");
            let labels_line = format!("<b>Labels:</b> {labels_str}");
            write!(out, "{}", Prose::new(&labels_line).display(&term)).unwrap();
        }

        // URL
        write!(
            out,
            "{}",
            Prose::new(format!("<a href=\"{}\">{}</a>", pr.html_url, pr.html_url)).display(&term)
        )
        .unwrap();

        // Body
        if let Some(ref body) = pr.body
            && !body.trim().is_empty()
        {
            let preview: String = body.lines().take(10).collect::<Vec<_>>().join("\n");
            let md: Markdown = preview.into();
            let mut buffer = Vec::new();
            let _ = write_terminal(&mut buffer, &md, TerminalOptions::default());
            out.push_str(&String::from_utf8_lossy(&buffer));
        }
    }

    out
}

/// Render a clear message when no pull requests match the filter.
pub fn render_pull_requests_empty(state: sniff::remote::PullRequestState) -> String {
    let term = Terminal::default();
    let msg = format!("No {} pull requests found", state.as_str());
    Prose::new(&msg).render(&term)
}

/// Print remote report as JSON.
pub fn print_remote_json(
    report: &RemoteReport,
    performance: Option<&sniff::PerformanceReport>,
) -> serde_json::Result<()> {
    let value = serde_json::to_value(report)?;
    crate::output::print_json_value(value, performance);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sniff::remote::{GitProvider, KeyUrls, PullRequestState, RepoMetadata, TagsAndReleases};

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

    fn make_test_pr(
        number: u64,
        state: &str,
        draft: bool,
        merged_at: Option<String>,
    ) -> PullRequestInfo {
        PullRequestInfo {
            number,
            title: format!("PR #{number}"),
            state: state.to_string(),
            author: "testuser".to_string(),
            draft,
            source_branch: Some("feature-branch".to_string()),
            target_branch: Some("main".to_string()),
            labels: vec![],
            body: None,
            created_at: "2024-01-15T10:00:00Z".to_string(),
            updated_at: None,
            merged_at,
            html_url: format!("https://github.com/owner/repo/pull/{number}"),
        }
    }

    fn make_test_pr_with_labels_and_body(number: u64) -> PullRequestInfo {
        PullRequestInfo {
            number,
            title: format!("PR #{number}"),
            state: "open".to_string(),
            author: "testuser".to_string(),
            draft: false,
            source_branch: Some("feature-branch".to_string()),
            target_branch: Some("main".to_string()),
            labels: vec!["bug".to_string(), "urgent".to_string()],
            body: Some("This is the PR description.\nIt has multiple lines.".to_string()),
            created_at: "2024-01-15T10:00:00Z".to_string(),
            updated_at: None,
            merged_at: None,
            html_url: format!("https://github.com/owner/repo/pull/{number}"),
        }
    }

    #[test]
    fn test_print_remote_json_succeeds() {
        let report = make_test_report();
        let result = print_remote_json(&report, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_pull_requests_table_empty() {
        let prs: Vec<PullRequestInfo> = vec![];
        let rendered = render_pull_requests_table(&prs);
        assert!(rendered.is_empty());
    }

    #[test]
    fn test_render_pull_requests_table_basic() {
        let prs = vec![
            make_test_pr(1, "open", false, None),
            make_test_pr(2, "closed", false, None),
        ];
        let rendered = render_pull_requests_table(&prs);
        assert!(rendered.contains("Pull Requests"));
        assert!(rendered.contains("#1"));
        assert!(rendered.contains("PR #1"));
        assert!(rendered.contains("open"));
        assert!(rendered.contains("testuser"));
    }

    #[test]
    fn test_render_pull_requests_table_draft_marker() {
        let prs = vec![make_test_pr(1, "open", true, None)];
        let rendered = render_pull_requests_table(&prs);
        assert!(rendered.contains("[draft]"));
    }

    #[test]
    fn test_render_pull_requests_table_merged_state() {
        let prs = vec![make_test_pr(
            1,
            "closed",
            false,
            Some("2024-01-20T10:00:00Z".to_string()),
        )];
        let rendered = render_pull_requests_table(&prs);
        assert!(rendered.contains("merged"));
    }

    #[test]
    fn test_render_pull_requests_table_closed_state() {
        let prs = vec![make_test_pr(1, "closed", false, None)];
        let rendered = render_pull_requests_table(&prs);
        assert!(rendered.contains("closed"));
        assert!(!rendered.contains("merged"));
    }

    #[test]
    fn test_render_pull_requests_verbose_empty() {
        let prs: Vec<PullRequestInfo> = vec![];
        let rendered = render_pull_requests_verbose(&prs);
        assert!(rendered.is_empty());
    }

    #[test]
    fn test_render_pull_requests_verbose_basic() {
        let prs = vec![make_test_pr(1, "open", false, None)];
        let rendered = render_pull_requests_verbose(&prs);
        assert!(rendered.contains("Pull Requests"));
        assert!(rendered.contains("#1"));
        assert!(rendered.contains("PR #1"));
        assert!(rendered.contains("testuser"));
        assert!(rendered.contains("open"));
        assert!(rendered.contains("feature-branch"));
        assert!(rendered.contains("main"));
        assert!(rendered.contains("2024-01-15"));
    }

    #[test]
    fn test_render_pull_requests_verbose_with_labels_and_body() {
        let prs = vec![make_test_pr_with_labels_and_body(42)];
        let rendered = render_pull_requests_verbose(&prs);
        assert!(rendered.contains("#42"));
        assert!(
            rendered.contains("Labels:"),
            "verbose output should include the 'Labels:' row"
        );
        assert!(rendered.contains("bug"));
        assert!(rendered.contains("urgent"));
        // Body is rendered through markdown terminal renderer which wraps words
        // in ANSI escape codes, so we check for individual words rather than phrases.
        assert!(rendered.contains("description"));
        assert!(rendered.contains("multiple"));
    }

    #[test]
    fn test_render_pull_requests_verbose_omits_labels_when_empty() {
        // PR with no labels and no body — Labels: row should be absent (consistent
        // with how Option fields like body are omitted entirely when missing).
        let prs = vec![make_test_pr(7, "open", false, None)];
        let rendered = render_pull_requests_verbose(&prs);
        assert!(rendered.contains("#7"));
        assert!(
            !rendered.contains("Labels:"),
            "verbose output should omit 'Labels:' row when labels are empty"
        );
    }

    #[test]
    fn test_render_pull_requests_verbose_draft_marker() {
        let prs = vec![make_test_pr(1, "open", true, None)];
        let rendered = render_pull_requests_verbose(&prs);
        assert!(rendered.contains("[draft]"));
    }

    #[test]
    fn test_render_pull_requests_verbose_merged_state() {
        let prs = vec![make_test_pr(
            1,
            "closed",
            false,
            Some("2024-01-20T10:00:00Z".to_string()),
        )];
        let rendered = render_pull_requests_verbose(&prs);
        assert!(rendered.contains("merged"));
    }

    #[test]
    fn test_render_pull_requests_empty_message_open() {
        let rendered = render_pull_requests_empty(PullRequestState::Open);
        assert!(rendered.contains("No open pull requests found"));
    }

    #[test]
    fn test_render_pull_requests_empty_message_merged() {
        let rendered = render_pull_requests_empty(PullRequestState::Merged);
        assert!(rendered.contains("No merged pull requests found"));
    }

    #[test]
    fn test_render_pull_requests_empty_message_all() {
        let rendered = render_pull_requests_empty(PullRequestState::All);
        assert!(rendered.contains("No all pull requests found"));
    }

    #[test]
    fn test_pr_state_display_open() {
        let pr = make_test_pr(1, "open", false, None);
        assert_eq!(pr_state_display(&pr), "open");
    }

    #[test]
    fn test_pr_state_display_closed() {
        let pr = make_test_pr(1, "closed", false, None);
        assert_eq!(pr_state_display(&pr), "closed");
    }

    #[test]
    fn test_pr_state_display_merged() {
        let pr = make_test_pr(1, "closed", false, Some("2024-01-20T10:00:00Z".to_string()));
        assert_eq!(pr_state_display(&pr), "merged");
    }

    #[test]
    fn test_pr_state_display_unknown() {
        let pr = make_test_pr(1, "unknown", false, None);
        assert_eq!(pr_state_display(&pr), "unknown");
    }
}
