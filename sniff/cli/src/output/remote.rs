//! Output formatting for remote repository reports.
//!
//! Uses biscuit-terminal's Prose, Table, and UnorderedList components for
//! rich terminal rendering with styled headings, structured tables, and
//! categorized lists.

use std::fmt::Write;

use biscuit_terminal::prelude::*;
use chrono::{DateTime, Utc};
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
            "<dim>◎</dim> {} issues",
            format_number(issues as usize)
        ));
    }
    stats_parts.push(format!(
        "<dim>⇄</dim> {} PRs",
        format_number(report.pull_requests.len())
    ));
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
///
/// When entries have workflow run details (`started_at` is present), renders
/// a compact table with conclusion glyphs, workflow name, branch, event, and
/// relative start time. Falls back to an unordered list for presence-only
/// detection.
fn render_cicd(cicd: &[CiCdInfo], term: &Terminal) -> String {
    if cicd.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    writeln!(out).unwrap();

    // Check if we have "run-shaped" entries with actual workflow run data
    let run_shaped: Vec<&CiCdInfo> = cicd.iter().filter(|ci| ci.started_at.is_some()).collect();

    if !run_shaped.is_empty() {
        let shown = run_shaped.len().min(10);
        let heading = format!("<b><u>CI/CD</u></b> <dim>(last {shown} runs)</dim>");
        write!(out, "{}", Prose::new(&heading).display(term)).unwrap();

        let columns = vec![
            TableColumn::new("").with_fixed_width(2),
            TableColumn::new("Workflow"),
            TableColumn::new("Branch").with_min_width(8),
            TableColumn::new("Event").with_min_width(6),
            TableColumn::new("Time").with_min_width(10),
        ];

        let mut table = Table::new().with_columns(columns);
        let now = Utc::now();

        for ci in run_shaped.iter().take(10) {
            let glyph = cicd_conclusion_glyph(ci);
            let time_str = ci
                .started_at
                .as_deref()
                .map(|t| relative_time(t, now))
                .unwrap_or_default();

            table.add_row(vec![
                glyph.into(),
                ci.name.clone().into(),
                ci.head_branch.clone().unwrap_or_default().into(),
                ci.event.clone().unwrap_or_default().into(),
                time_str.into(),
            ]);
        }

        write!(out, "{}", table.display(term)).unwrap();
    } else {
        // Presence-only fallback: render as unordered list
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
    }

    out
}

/// Map a CI/CD run conclusion/status to a glyph string.
fn cicd_conclusion_glyph(ci: &CiCdInfo) -> String {
    // Active runs: use a spinner-like glyph
    match ci.status.as_str() {
        "in_progress" => return "⟳".to_string(),
        "queued" | "waiting" | "pending" => return "◌".to_string(),
        _ => {}
    }

    // Completed runs: map conclusion to glyph
    match ci.conclusion.as_deref() {
        Some("success") => "✓".to_string(),
        Some("failure") => "✗".to_string(),
        Some("cancelled") => "⊘".to_string(),
        Some("skipped") => "⊝".to_string(),
        Some("timed_out") => "⏱".to_string(),
        Some("action_required") => "⚠".to_string(),
        Some("neutral") => "○".to_string(),
        Some(other) => other.chars().next().unwrap_or('?').to_string(),
        None => "?".to_string(),
    }
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

/// Convert an ISO 8601 timestamp to a compact relative time string.
///
/// Examples: "just now", "8m ago", "2h ago", "3d ago", "2w ago", "5mo ago",
/// "2y ago".
pub fn relative_time(iso: &str, now: DateTime<Utc>) -> String {
    let parsed = match DateTime::parse_from_rfc3339(iso) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => return iso.to_string(),
    };

    let duration = now.signed_duration_since(parsed);
    let seconds = duration.num_seconds();

    if seconds < 60 {
        return "just now".to_string();
    }

    let minutes = duration.num_minutes();
    if minutes < 60 {
        return format!("{minutes}m ago");
    }

    let hours = duration.num_hours();
    if hours < 24 {
        return format!("{hours}h ago");
    }

    let days = duration.num_days();
    if days < 7 {
        return format!("{days}d ago");
    }

    let weeks = days / 7;
    if weeks < 4 {
        return format!("{weeks}w ago");
    }

    let months = days / 30;
    if months < 12 {
        return format!("{months}mo ago");
    }

    let years = days / 365;
    format!("{years}y ago")
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

    #[test]
    fn test_relative_time_just_now() {
        let now = Utc::now();
        let iso = now.to_rfc3339();
        assert_eq!(relative_time(&iso, now), "just now");
    }

    #[test]
    fn test_relative_time_minutes_ago() {
        let now = Utc::now();
        let then = now - chrono::Duration::minutes(5);
        assert_eq!(relative_time(&then.to_rfc3339(), now), "5m ago");
    }

    #[test]
    fn test_relative_time_one_minute_ago() {
        let now = Utc::now();
        let then = now - chrono::Duration::minutes(1);
        assert_eq!(relative_time(&then.to_rfc3339(), now), "1m ago");
    }

    #[test]
    fn test_relative_time_hours_ago() {
        let now = Utc::now();
        let then = now - chrono::Duration::hours(3);
        assert_eq!(relative_time(&then.to_rfc3339(), now), "3h ago");
    }

    #[test]
    fn test_relative_time_one_hour_ago() {
        let now = Utc::now();
        let then = now - chrono::Duration::hours(1);
        assert_eq!(relative_time(&then.to_rfc3339(), now), "1h ago");
    }

    #[test]
    fn test_relative_time_days_ago() {
        let now = Utc::now();
        let then = now - chrono::Duration::days(2);
        assert_eq!(relative_time(&then.to_rfc3339(), now), "2d ago");
    }

    #[test]
    fn test_relative_time_one_day_ago() {
        let now = Utc::now();
        let then = now - chrono::Duration::days(1);
        assert_eq!(relative_time(&then.to_rfc3339(), now), "1d ago");
    }

    #[test]
    fn test_relative_time_weeks_ago() {
        let now = Utc::now();
        let then = now - chrono::Duration::days(14);
        assert_eq!(relative_time(&then.to_rfc3339(), now), "2w ago");
    }

    #[test]
    fn test_relative_time_one_week_ago() {
        let now = Utc::now();
        let then = now - chrono::Duration::days(7);
        assert_eq!(relative_time(&then.to_rfc3339(), now), "1w ago");
    }

    #[test]
    fn test_relative_time_months_ago() {
        let now = Utc::now();
        let then = now - chrono::Duration::days(60);
        assert_eq!(relative_time(&then.to_rfc3339(), now), "2mo ago");
    }

    #[test]
    fn test_relative_time_one_month_ago() {
        let now = Utc::now();
        let then = now - chrono::Duration::days(30);
        assert_eq!(relative_time(&then.to_rfc3339(), now), "1mo ago");
    }

    #[test]
    fn test_relative_time_years_ago() {
        let now = Utc::now();
        let then = now - chrono::Duration::days(730);
        assert_eq!(relative_time(&then.to_rfc3339(), now), "2y ago");
    }

    #[test]
    fn test_relative_time_one_year_ago() {
        let now = Utc::now();
        let then = now - chrono::Duration::days(365);
        assert_eq!(relative_time(&then.to_rfc3339(), now), "1y ago");
    }

    #[test]
    fn test_relative_time_invalid_iso_returns_original() {
        let now = Utc::now();
        assert_eq!(relative_time("not-a-date", now), "not-a-date");
    }

    #[test]
    fn test_render_remote_text_issues_relabeled() {
        let report = make_test_report();
        let rendered = render_remote_text(&report, None);
        assert!(rendered.contains("◎") && rendered.contains("12 issues"));
        assert!(!rendered.contains("12 open"));
    }

    #[test]
    fn test_render_remote_text_shows_pr_count() {
        let mut report = make_test_report();
        report.pull_requests = vec![
            make_test_pr(1, "open", false, None),
            make_test_pr(2, "open", false, None),
        ];
        let rendered = render_remote_text(&report, None);
        assert!(rendered.contains("⇄") && rendered.contains("2 PRs"));
    }

    #[test]
    fn test_render_remote_text_shows_zero_pr_count_when_empty() {
        let report = make_test_report();
        let rendered = render_remote_text(&report, None);
        assert!(rendered.contains("⇄") && rendered.contains("0 PRs"));
    }

    fn make_test_cicd(
        name: &str,
        status: &str,
        conclusion: Option<&str>,
        started_at: Option<&str>,
        head_branch: Option<&str>,
        event: Option<&str>,
    ) -> CiCdInfo {
        CiCdInfo {
            provider: "GitHub Actions".to_string(),
            config_path: None,
            name: name.to_string(),
            status: status.to_string(),
            conclusion: conclusion.map(|s| s.to_string()),
            html_url: None,
            started_at: started_at.map(|s| s.to_string()),
            head_branch: head_branch.map(|s| s.to_string()),
            event: event.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_cicd_conclusion_glyph_success() {
        let ci = make_test_cicd("CI", "completed", Some("success"), None, None, None);
        assert_eq!(cicd_conclusion_glyph(&ci), "✓");
    }

    #[test]
    fn test_cicd_conclusion_glyph_failure() {
        let ci = make_test_cicd("CI", "completed", Some("failure"), None, None, None);
        assert_eq!(cicd_conclusion_glyph(&ci), "✗");
    }

    #[test]
    fn test_cicd_conclusion_glyph_cancelled() {
        let ci = make_test_cicd("CI", "completed", Some("cancelled"), None, None, None);
        assert_eq!(cicd_conclusion_glyph(&ci), "⊘");
    }

    #[test]
    fn test_cicd_conclusion_glyph_skipped() {
        let ci = make_test_cicd("CI", "completed", Some("skipped"), None, None, None);
        assert_eq!(cicd_conclusion_glyph(&ci), "⊝");
    }

    #[test]
    fn test_cicd_conclusion_glyph_timed_out() {
        let ci = make_test_cicd("CI", "completed", Some("timed_out"), None, None, None);
        assert_eq!(cicd_conclusion_glyph(&ci), "⏱");
    }

    #[test]
    fn test_cicd_conclusion_glyph_action_required() {
        let ci = make_test_cicd("CI", "completed", Some("action_required"), None, None, None);
        assert_eq!(cicd_conclusion_glyph(&ci), "⚠");
    }

    #[test]
    fn test_cicd_conclusion_glyph_neutral() {
        let ci = make_test_cicd("CI", "completed", Some("neutral"), None, None, None);
        assert_eq!(cicd_conclusion_glyph(&ci), "○");
    }

    #[test]
    fn test_cicd_conclusion_glyph_in_progress() {
        let ci = make_test_cicd("CI", "in_progress", None, None, None, None);
        assert_eq!(cicd_conclusion_glyph(&ci), "⟳");
    }

    #[test]
    fn test_cicd_conclusion_glyph_queued() {
        let ci = make_test_cicd("CI", "queued", None, None, None, None);
        assert_eq!(cicd_conclusion_glyph(&ci), "◌");
    }

    #[test]
    fn test_cicd_conclusion_glyph_waiting() {
        let ci = make_test_cicd("CI", "waiting", None, None, None, None);
        assert_eq!(cicd_conclusion_glyph(&ci), "◌");
    }

    #[test]
    fn test_cicd_conclusion_glyph_pending() {
        let ci = make_test_cicd("CI", "pending", None, None, None, None);
        assert_eq!(cicd_conclusion_glyph(&ci), "◌");
    }

    #[test]
    fn test_cicd_conclusion_glyph_unknown_conclusion() {
        let ci = make_test_cicd("CI", "completed", Some("weird"), None, None, None);
        assert_eq!(cicd_conclusion_glyph(&ci), "w");
    }

    #[test]
    fn test_cicd_conclusion_glyph_no_conclusion() {
        let ci = make_test_cicd("CI", "completed", None, None, None, None);
        assert_eq!(cicd_conclusion_glyph(&ci), "?");
    }

    #[test]
    fn test_render_cicd_empty() {
        let term = Terminal::default();
        let rendered = render_cicd(&[], &term);
        assert!(rendered.is_empty());
    }

    #[test]
    fn test_render_cicd_presence_only_fallback() {
        let term = Terminal::default();
        let cicd = vec![CiCdInfo {
            provider: "GitHub Actions".to_string(),
            config_path: Some(".github/workflows".to_string()),
            name: "GitHub Actions".to_string(),
            status: "detected".to_string(),
            conclusion: None,
            html_url: None,
            started_at: None,
            head_branch: None,
            event: None,
        }];
        let rendered = render_cicd(&cicd, &term);
        assert!(rendered.contains("CI/CD"));
        assert!(rendered.contains("GitHub Actions"));
        assert!(rendered.contains(".github/workflows"));
    }

    #[test]
    fn test_render_cicd_run_shaped_table() {
        let term = Terminal::default();
        let now = Utc::now();
        let started = (now - chrono::Duration::minutes(5)).to_rfc3339();
        let cicd = vec![make_test_cicd(
            "Test Workflow",
            "completed",
            Some("success"),
            Some(&started),
            Some("main"),
            Some("push"),
        )];
        let rendered = render_cicd(&cicd, &term);
        assert!(rendered.contains("CI/CD"));
        assert!(rendered.contains("last 1 runs"));
        assert!(rendered.contains("Test Workflow"));
        assert!(rendered.contains("main"));
        assert!(rendered.contains("push"));
        assert!(rendered.contains("5m ago"));
        // Table rendering uses box-drawing characters
        assert!(rendered.contains('│') || rendered.contains('┌'));
    }

    #[test]
    fn test_render_cicd_run_shaped_multiple_runs() {
        let term = Terminal::default();
        let now = Utc::now();
        let started1 = (now - chrono::Duration::hours(1)).to_rfc3339();
        let started2 = (now - chrono::Duration::days(1)).to_rfc3339();
        let cicd = vec![
            make_test_cicd(
                "Build",
                "completed",
                Some("success"),
                Some(&started1),
                Some("main"),
                Some("push"),
            ),
            make_test_cicd(
                "Lint",
                "completed",
                Some("failure"),
                Some(&started2),
                Some("feature"),
                Some("pull_request"),
            ),
        ];
        let rendered = render_cicd(&cicd, &term);
        assert!(rendered.contains("last 2 runs"));
        assert!(rendered.contains("Build"));
        assert!(rendered.contains("Lint"));
        assert!(rendered.contains("feature"));
        assert!(rendered.contains("pull_request"));
    }

    #[test]
    fn test_render_cicd_mixed_run_and_presence_prefers_table() {
        let term = Terminal::default();
        let now = Utc::now();
        let started = (now - chrono::Duration::minutes(10)).to_rfc3339();
        let cicd = vec![
            make_test_cicd(
                "Test",
                "completed",
                Some("success"),
                Some(&started),
                Some("main"),
                Some("push"),
            ),
            CiCdInfo {
                provider: "GitHub Actions".to_string(),
                config_path: Some(".github/workflows".to_string()),
                name: "GitHub Actions".to_string(),
                status: "detected".to_string(),
                conclusion: None,
                html_url: None,
                started_at: None,
                head_branch: None,
                event: None,
            },
        ];
        let rendered = render_cicd(&cicd, &term);
        // When at least one run-shaped entry exists, table mode is used
        assert!(rendered.contains("Test"));
        assert!(rendered.contains("main"));
        assert!(rendered.contains("push"));
    }

    #[test]
    fn test_render_remote_text_includes_cicd_table() {
        let mut report = make_test_report();
        let now = Utc::now();
        let started = (now - chrono::Duration::minutes(15)).to_rfc3339();
        report.ci_cd = vec![make_test_cicd(
            "CI",
            "completed",
            Some("success"),
            Some(&started),
            Some("main"),
            Some("push"),
        )];
        let rendered = render_remote_text(&report, None);
        assert!(rendered.contains("CI/CD"));
        assert!(rendered.contains("last 1 runs"));
        assert!(rendered.contains("main"));
    }
}
