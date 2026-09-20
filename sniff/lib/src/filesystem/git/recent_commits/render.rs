//! Text reports for [`RecentCommits`].
//!
//! One layout walk produces styled [`Line`]s; `to_prose`, `to_markdown`, and
//! `to_plain` are folds over that layout that differ only in how a style or
//! link is written. The prose-tag vocabulary is the `biscuit-terminal` Prose
//! grammar (`biscuit-terminal/docs/components/prose.md`).

use std::path::Path;

use chrono::{DateTime, Duration, FixedOffset, Utc};

use super::options::{RecentCommitsOptions, RecentCommitsProjection, RecentCommitsVerbosity};
use super::payload::{RecentCommit, RecentCommitFile, RecentCommitFileKind, RecentCommits};
use crate::filesystem::path_kind::{ChangeCategory, classify_path};

impl RecentCommits {
    /// A copy keeping only the files `projection` selects and the commits
    /// left with at least one file.
    ///
    /// A `moved` file is kept when either its path or its original path is in
    /// the projection. Commit-level facts (`file_types`, package attribution,
    /// links) still describe the whole commit. [`RecentCommitsProjection::All`]
    /// keeps every commit, including no-change merges with no files.
    pub fn projected(&self, projection: RecentCommitsProjection) -> RecentCommits {
        let Some(category) = projection_category(projection) else {
            return self.clone();
        };
        let commits = self
            .commits()
            .iter()
            .filter_map(|commit| {
                let files: Vec<RecentCommitFile> = commit
                    .files
                    .iter()
                    .filter(|file| file_in_category(file, category))
                    .cloned()
                    .collect();
                (!files.is_empty()).then(|| RecentCommit {
                    files,
                    ..commit.clone()
                })
            })
            .collect();
        let projected = RecentCommits::from_commits(commits);
        match self.repo_root() {
            Some(root) => projected.with_repo_root(root.to_path_buf()),
            None => projected,
        }
    }

    /// The report as Prose: style tags, a remote link on the hash when one
    /// exists, and `file://` links on changed files.
    ///
    /// Render it to a terminal with `biscuit-terminal`'s `Prose` component.
    /// An empty report is an empty string.
    pub fn to_prose(&self, options: &RecentCommitsOptions) -> String {
        self.render(options, Utc::now(), Format::Prose)
    }

    /// The report as portable Markdown: links and bold/italic emphasis, no
    /// color.
    pub fn to_markdown(&self, options: &RecentCommitsOptions) -> String {
        self.render(options, Utc::now(), Format::Markdown)
    }

    /// The report as bare text with no styling, markup, or links.
    pub fn to_plain(&self, options: &RecentCommitsOptions) -> String {
        self.render(options, Utc::now(), Format::Plain)
    }

    /// One plain block per commit, in report order.
    ///
    /// These are the per-commit units of [`to_plain`](Self::to_plain): the
    /// blocks it separates with a blank line (in every verbosity but
    /// `Compact`, which runs them together). A consumer that wants commits as
    /// individual array elements — Darkmatter's `ctx.recent_commits` — takes
    /// them here rather than splitting `to_plain` on blank lines, which the
    /// verbose layout also uses inside a block. Projection applies exactly as
    /// it does to `to_plain`, so a commit the projection drops has no block.
    pub fn plain_blocks(&self, options: &RecentCommitsOptions) -> Vec<String> {
        let now = Utc::now();
        let report = self.projected(options.projection);
        report
            .commits()
            .iter()
            .map(|commit| {
                commit_lines(commit, options, now, report.repo_root())
                    .iter()
                    .map(|line| Format::Plain.line(line))
                    .collect()
            })
            .collect()
    }

    fn render(&self, options: &RecentCommitsOptions, now: DateTime<Utc>, format: Format) -> String {
        layout(self, options, now)
            .iter()
            .map(|line| format.line(line))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Prose,
    Markdown,
    Plain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Style {
    Plain,
    Bold,
    Italic,
    Operation,
    Scope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Span {
    text: String,
    style: Style,
    link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Line {
    Blank,
    Heading(String),
    Text(Vec<Span>),
}

fn span(text: impl Into<String>, style: Style) -> Span {
    Span {
        text: text.into(),
        style,
        link: None,
    }
}

fn plain(text: impl Into<String>) -> Span {
    span(text, Style::Plain)
}

fn projection_category(projection: RecentCommitsProjection) -> Option<ChangeCategory> {
    match projection {
        RecentCommitsProjection::All => None,
        RecentCommitsProjection::SourceCode => Some(ChangeCategory::SourceCode),
        RecentCommitsProjection::Documentation => Some(ChangeCategory::Documentation),
    }
}

fn file_in_category(file: &RecentCommitFile, category: ChangeCategory) -> bool {
    std::iter::once(file.path.as_str())
        .chain(file.original_path.as_deref())
        .any(|path| classify_path(Path::new(path)) == category)
}

fn layout(commits: &RecentCommits, options: &RecentCommitsOptions, now: DateTime<Utc>) -> Vec<Line> {
    let report = commits.projected(options.projection);
    let mut lines = Vec::new();
    if report.is_empty() {
        return lines;
    }

    let heading = match options.projection {
        RecentCommitsProjection::All => None,
        RecentCommitsProjection::SourceCode => Some("Source Code Changes"),
        RecentCommitsProjection::Documentation => Some("Documentation Changes"),
    };
    if let Some(heading) = heading {
        lines.push(Line::Heading(heading.to_string()));
        lines.push(Line::Blank);
    }

    for (index, commit) in report.commits().iter().enumerate() {
        if index > 0 && options.verbosity != RecentCommitsVerbosity::Compact {
            lines.push(Line::Blank);
        }
        lines.extend(commit_lines(commit, options, now, report.repo_root()));
    }
    lines
}

/// The lines of one commit's block: its header, then whatever the verbosity
/// adds. This is the unit [`RecentCommits::plain_blocks`] exposes, so it must
/// not include the blank line `layout` places *between* commits.
fn commit_lines(
    commit: &RecentCommit,
    options: &RecentCommitsOptions,
    now: DateTime<Utc>,
    repo_root: Option<&Path>,
) -> Vec<Line> {
    let mut lines = vec![Line::Text(header(commit, options, now))];
    match options.verbosity {
        RecentCommitsVerbosity::Compact => {}
        RecentCommitsVerbosity::Normal => {
            files_block(&mut lines, &commit.files, repo_root);
        }
        RecentCommitsVerbosity::Verbose => {
            let mut has_commentary = false;
            if !commit.description.is_empty() {
                lines.push(Line::Text(vec![plain("  "), plain(&commit.description)]));
                has_commentary = true;
            }
            if !commit.bullet_points.is_empty() {
                lines.push(Line::Blank);
                lines.push(label_line("Details:"));
                lines.push(Line::Blank);
                for bullet in &commit.bullet_points {
                    lines.push(Line::Text(vec![plain("  - "), plain(bullet)]));
                }
                has_commentary = true;
            }
            if has_commentary {
                lines.push(Line::Blank);
            }
            files_block(&mut lines, &commit.files, repo_root);
        }
    }
    lines
}

fn header(commit: &RecentCommit, options: &RecentCommitsOptions, now: DateTime<Utc>) -> Vec<Span> {
    let short_hash: String = commit.hash.chars().take(7).collect();
    let mut spans = vec![
        plain("- ["),
        Span {
            text: short_hash,
            style: Style::Bold,
            link: commit.commit_url.clone(),
        },
        plain("] "),
    ];
    if let Some(operation) = &commit.operation {
        spans.push(span(operation, Style::Operation));
        if let Some(scope) = &commit.scope {
            spans.push(span("(", Style::Operation));
            spans.push(span(scope, Style::Scope));
            spans.push(span(")", Style::Operation));
        }
        spans.push(plain(" "));
    }
    if options.show_author {
        spans.push(plain(format!("by {} ", author_label(commit))));
    }
    spans.push(span("at", Style::Italic));
    spans.push(plain(" "));
    spans.push(span(time_label(commit.datetime, now, options.timezone), Style::Bold));
    spans.push(plain(": "));
    spans.push(plain(&commit.heading));
    spans
}

fn author_label(commit: &RecentCommit) -> &str {
    if commit.author.name.is_empty() {
        &commit.author.email
    } else {
        &commit.author.name
    }
}

/// `1:01pm Today`, `9:30am Yesterday`, or `9:30am 2026-04-01`, with the day
/// judged in `timezone` so a label always agrees with calendar selection.
fn time_label(datetime: DateTime<Utc>, now: DateTime<Utc>, timezone: FixedOffset) -> String {
    let local = datetime.with_timezone(&timezone);
    let today = now.with_timezone(&timezone).date_naive();
    let date = local.date_naive();
    let day = if date == today {
        "Today".to_string()
    } else if date == today - Duration::days(1) {
        "Yesterday".to_string()
    } else {
        date.format("%Y-%m-%d").to_string()
    };
    format!("{} {day}", local.format("%-I:%M%P"))
}

fn label_line(label: &str) -> Line {
    Line::Text(vec![plain("  "), span(label, Style::Bold)])
}

fn files_block(lines: &mut Vec<Line>, files: &[RecentCommitFile], repo_root: Option<&Path>) {
    if files.is_empty() {
        lines.push(Line::Text(vec![
            plain("  "),
            span("Files Impacted:", Style::Bold),
            plain(" none"),
        ]));
        return;
    }
    lines.push(label_line("Files Impacted:"));
    for file in files {
        let kind = match file.kind {
            RecentCommitFileKind::Modified => "modified",
            RecentCommitFileKind::Added => "added",
            RecentCommitFileKind::Deleted => "deleted",
            RecentCommitFileKind::Moved => "moved",
        };
        // A deleted file no longer exists in the working tree to link to.
        let link = match (file.kind, repo_root) {
            (RecentCommitFileKind::Deleted, _) | (_, None) => None,
            (_, Some(root)) => file_url(&root.join(&file.path)),
        };
        let mut spans = vec![
            plain(format!("  - {kind}: ")),
            Span {
                text: file.path.clone(),
                style: Style::Plain,
                link,
            },
        ];
        if let Some(original) = &file.original_path {
            spans.push(plain(" (from "));
            spans.push(plain(original));
            spans.push(plain(")"));
        }
        lines.push(Line::Text(spans));
    }
}

fn file_url(path: &Path) -> Option<String> {
    url::Url::from_file_path(path).ok().map(String::from)
}

impl Format {
    fn line(self, line: &Line) -> String {
        let mut out = match line {
            Line::Blank => String::new(),
            Line::Heading(text) => match self {
                Format::Prose => format!("<bold>{}</bold>", escape(text, self)),
                Format::Markdown => format!("## {}", escape(text, self)),
                Format::Plain => text.clone(),
            },
            Line::Text(spans) => spans.iter().map(|span| self.span(span)).collect(),
        };
        out.push('\n');
        out
    }

    fn span(self, span: &Span) -> String {
        if self == Format::Plain {
            return span.text.clone();
        }
        let text = escape(&span.text, self);
        let styled = match (self, span.style) {
            (_, Style::Plain) => text,
            (Format::Prose, Style::Bold) => format!("<bold>{text}</bold>"),
            (Format::Prose, Style::Italic) => format!("<italic>{text}</italic>"),
            (Format::Prose, Style::Operation) => format!("<blue>{text}</blue>"),
            (Format::Prose, Style::Scope) => format!("<blue><dim>{text}</dim></blue>"),
            (Format::Markdown, Style::Bold) => format!("**{text}**"),
            (Format::Markdown, Style::Italic) => format!("_{text}_"),
            (Format::Markdown, Style::Operation | Style::Scope) => text,
            (Format::Plain, _) => unreachable!("plain spans return before styling"),
        };
        match &span.link {
            Some(url) => format!("[{styled}]({})", link_target(url)),
            None => styled,
        }
    }
}

/// Backslash-escape characters that `format` would read as markup. Prose has
/// no inline code and renders the backslash of an escaped backtick, so only
/// Markdown escapes backticks.
fn escape(text: &str, format: Format) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        let markup = matches!(character, '\\' | '*' | '_' | '[' | ']' | '<' | '>')
            || (character == '`' && format == Format::Markdown);
        if markup {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

/// Percent-encode characters that would end a `[text](url)` target early.
fn link_target(url: &str) -> String {
    url.replace('(', "%28")
        .replace(')', "%29")
        .replace(' ', "%20")
        .replace('<', "%3C")
        .replace('>', "%3E")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::git::recent_commits::payload::{
        RecentCommitAuthor, RecentCommitFileTypes,
    };
    use chrono::TimeZone;
    use std::path::PathBuf;

    const URL: &str = "https://github.com/o/r/commit/0123456789abcdef";

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 17, 18, 0, 0).unwrap()
    }

    fn utc_offset() -> FixedOffset {
        FixedOffset::east_opt(0).unwrap()
    }

    fn file(kind: RecentCommitFileKind, path: &str) -> RecentCommitFile {
        RecentCommitFile {
            kind,
            path: path.to_string(),
            original_path: None,
            added: None,
            removed: None,
        }
    }

    fn repo_root() -> PathBuf {
        if cfg!(windows) {
            PathBuf::from(r"C:\repo")
        } else {
            PathBuf::from("/repo")
        }
    }

    /// `file:///repo/…` on Unix, `file:///C:/repo/…` on Windows.
    fn file_link(path: &str) -> String {
        if cfg!(windows) {
            format!("file:///C:/repo/{path}")
        } else {
            format!("file:///repo/{path}")
        }
    }

    fn conventional() -> RecentCommit {
        RecentCommit {
            hash: "0123456789abcdef".to_string(),
            datetime: Utc.with_ymd_and_hms(2026, 9, 17, 13, 1, 0).unwrap(),
            author: RecentCommitAuthor {
                name: "Ada Lovelace".to_string(),
                email: "ada@example.com".to_string(),
            },
            operation: Some("feat".to_string()),
            scope: Some("sniff".to_string()),
            heading: "add reports".to_string(),
            description: "Longer prose.".to_string(),
            bullet_points: vec!["first".to_string(), "second".to_string()],
            files: vec![
                file(RecentCommitFileKind::Modified, "src/lib.rs"),
                RecentCommitFile {
                    original_path: Some("docs/old.md".to_string()),
                    ..file(RecentCommitFileKind::Moved, "docs/new.md")
                },
                file(RecentCommitFileKind::Deleted, "README.md"),
            ],
            file_types: RecentCommitFileTypes::default(),
            attribution: None,
            remote: Some(true),
            commit_url: Some(URL.to_string()),
        }
    }

    fn non_conventional() -> RecentCommit {
        RecentCommit {
            hash: "fedcba9876543210".to_string(),
            datetime: Utc.with_ymd_and_hms(2026, 9, 16, 9, 30, 0).unwrap(),
            operation: None,
            scope: None,
            heading: "Merge branch main".to_string(),
            description: String::new(),
            bullet_points: Vec::new(),
            files: Vec::new(),
            remote: Some(false),
            commit_url: None,
            ..conventional()
        }
    }

    fn report() -> RecentCommits {
        RecentCommits::from_commits(vec![conventional(), non_conventional()])
            .with_repo_root(repo_root())
    }

    fn options(verbosity: RecentCommitsVerbosity) -> RecentCommitsOptions {
        RecentCommitsOptions::new()
            .verbosity(verbosity)
            .timezone(utc_offset())
    }

    mod layouts {
        use super::*;

        #[test]
        fn compact_prose_is_one_styled_header_per_commit() {
            let prose =
                report().render(&options(RecentCommitsVerbosity::Compact), now(), Format::Prose);
            assert_eq!(
                prose,
                format!(
                    "- \\[[<bold>0123456</bold>]({URL})\\] <blue>feat</blue><blue>(</blue><blue><dim>sniff</dim></blue><blue>)</blue> <italic>at</italic> <bold>1:01pm Today</bold>: add reports\n\
                     - \\[<bold>fedcba9</bold>\\] <italic>at</italic> <bold>9:30am Yesterday</bold>: Merge branch main\n"
                )
            );
        }

        #[test]
        fn normal_prose_lists_files_with_links_and_marks_empty_merges() {
            let prose =
                report().render(&options(RecentCommitsVerbosity::Normal), now(), Format::Prose);
            assert_eq!(
                prose,
                format!(
                    "- \\[[<bold>0123456</bold>]({URL})\\] <blue>feat</blue><blue>(</blue><blue><dim>sniff</dim></blue><blue>)</blue> <italic>at</italic> <bold>1:01pm Today</bold>: add reports\n\
                     \x20 <bold>Files Impacted:</bold>\n\
                     \x20 - modified: [src/lib.rs]({})\n\
                     \x20 - moved: [docs/new.md]({}) (from docs/old.md)\n\
                     \x20 - deleted: README.md\n\
                     \n\
                     - \\[<bold>fedcba9</bold>\\] <italic>at</italic> <bold>9:30am Yesterday</bold>: Merge branch main\n\
                     \x20 <bold>Files Impacted:</bold> none\n",
                    file_link("src/lib.rs"),
                    file_link("docs/new.md"),
                )
            );
        }

        #[test]
        fn verbose_prose_adds_description_and_details_before_files() {
            let prose =
                report().render(&options(RecentCommitsVerbosity::Verbose), now(), Format::Prose);
            assert_eq!(
                prose,
                format!(
                    "- \\[[<bold>0123456</bold>]({URL})\\] <blue>feat</blue><blue>(</blue><blue><dim>sniff</dim></blue><blue>)</blue> <italic>at</italic> <bold>1:01pm Today</bold>: add reports\n\
                     \x20 Longer prose.\n\
                     \n\
                     \x20 <bold>Details:</bold>\n\
                     \n\
                     \x20 - first\n\
                     \x20 - second\n\
                     \n\
                     \x20 <bold>Files Impacted:</bold>\n\
                     \x20 - modified: [src/lib.rs]({})\n\
                     \x20 - moved: [docs/new.md]({}) (from docs/old.md)\n\
                     \x20 - deleted: README.md\n\
                     \n\
                     - \\[<bold>fedcba9</bold>\\] <italic>at</italic> <bold>9:30am Yesterday</bold>: Merge branch main\n\
                     \x20 <bold>Files Impacted:</bold> none\n",
                    file_link("src/lib.rs"),
                    file_link("docs/new.md"),
                )
            );
        }

        #[test]
        fn verbose_markdown_uses_links_and_emphasis_without_color_tags() {
            let markdown = report().render(
                &options(RecentCommitsVerbosity::Verbose),
                now(),
                Format::Markdown,
            );
            assert_eq!(
                markdown,
                format!(
                    "- \\[[**0123456**]({URL})\\] feat(sniff) _at_ **1:01pm Today**: add reports\n\
                     \x20 Longer prose.\n\
                     \n\
                     \x20 **Details:**\n\
                     \n\
                     \x20 - first\n\
                     \x20 - second\n\
                     \n\
                     \x20 **Files Impacted:**\n\
                     \x20 - modified: [src/lib.rs]({})\n\
                     \x20 - moved: [docs/new.md]({}) (from docs/old.md)\n\
                     \x20 - deleted: README.md\n\
                     \n\
                     - \\[**fedcba9**\\] _at_ **9:30am Yesterday**: Merge branch main\n\
                     \x20 **Files Impacted:** none\n",
                    file_link("src/lib.rs"),
                    file_link("docs/new.md"),
                )
            );
            assert!(!markdown.contains("<blue>") && !markdown.contains("<bold>"));
        }

        #[test]
        fn verbose_plain_has_the_same_text_without_markup_or_links() {
            let plain = report().render(
                &options(RecentCommitsVerbosity::Verbose),
                now(),
                Format::Plain,
            );
            assert_eq!(
                plain,
                "- [0123456] feat(sniff) at 1:01pm Today: add reports\n\
                 \x20 Longer prose.\n\
                 \n\
                 \x20 Details:\n\
                 \n\
                 \x20 - first\n\
                 \x20 - second\n\
                 \n\
                 \x20 Files Impacted:\n\
                 \x20 - modified: src/lib.rs\n\
                 \x20 - moved: docs/new.md (from docs/old.md)\n\
                 \x20 - deleted: README.md\n\
                 \n\
                 - [fedcba9] at 9:30am Yesterday: Merge branch main\n\
                 \x20 Files Impacted: none\n"
            );
            for marker in ["**", "](", "<", "\x1b", "file://", "https://"] {
                assert!(!plain.contains(marker), "{marker:?} in {plain}");
            }
        }

        #[test]
        fn verbose_without_commentary_matches_normal() {
            let commit = RecentCommit {
                description: String::new(),
                bullet_points: Vec::new(),
                ..conventional()
            };
            let commits = RecentCommits::from_commits(vec![commit]).with_repo_root(repo_root());
            assert_eq!(
                commits.render(&options(RecentCommitsVerbosity::Verbose), now(), Format::Plain),
                commits.render(&options(RecentCommitsVerbosity::Normal), now(), Format::Plain),
            );
        }

        #[test]
        fn description_without_bullets_is_followed_by_one_blank_line() {
            let commit = RecentCommit {
                bullet_points: Vec::new(),
                files: vec![file(RecentCommitFileKind::Added, "a.rs")],
                ..conventional()
            };
            let plain = RecentCommits::from_commits(vec![commit]).render(
                &options(RecentCommitsVerbosity::Verbose),
                now(),
                Format::Plain,
            );
            assert_eq!(
                plain,
                "- [0123456] feat(sniff) at 1:01pm Today: add reports\n\
                 \x20 Longer prose.\n\
                 \n\
                 \x20 Files Impacted:\n\
                 \x20 - added: a.rs\n"
            );
        }

        #[test]
        fn show_author_adds_the_name_to_the_header_only() {
            let with_author = options(RecentCommitsVerbosity::Compact).show_author(true);
            let prose = report().render(&with_author, now(), Format::Prose);
            assert!(
                prose.contains("<blue>)</blue> by Ada Lovelace <italic>at</italic>"),
                "{prose}"
            );
            assert!(prose.contains(r"\] by Ada Lovelace <italic>at</italic>"), "{prose}");
            assert_eq!(
                report().render(&with_author, now(), Format::Plain),
                "- [0123456] feat(sniff) by Ada Lovelace at 1:01pm Today: add reports\n\
                 - [fedcba9] by Ada Lovelace at 9:30am Yesterday: Merge branch main\n"
            );

            let nameless = RecentCommit {
                author: RecentCommitAuthor {
                    name: String::new(),
                    email: "bot@example.com".to_string(),
                },
                ..non_conventional()
            };
            assert_eq!(
                RecentCommits::from_commits(vec![nameless]).render(
                    &with_author,
                    now(),
                    Format::Plain
                ),
                "- [fedcba9] by bot@example.com at 9:30am Yesterday: Merge branch main\n"
            );
        }

        #[test]
        fn operation_without_scope_has_no_parentheses() {
            let commit = RecentCommit {
                scope: None,
                ..conventional()
            };
            let collection = RecentCommits::from_commits(vec![commit]);
            let compact = options(RecentCommitsVerbosity::Compact);
            assert!(
                collection
                    .render(&compact, now(), Format::Prose)
                    .contains(r"\] <blue>feat</blue> <italic>at</italic>")
            );
            assert!(
                collection
                    .render(&compact, now(), Format::Plain)
                    .starts_with("- [0123456] feat at ")
            );
        }

        #[test]
        fn deserialized_collections_render_files_without_links() {
            let json = serde_json::to_string(&report()).unwrap();
            let read: RecentCommits = serde_json::from_str(&json).unwrap();
            let prose = read.render(&options(RecentCommitsVerbosity::Normal), now(), Format::Prose);
            assert!(prose.contains("  - modified: src/lib.rs\n"), "{prose}");
            assert!(!prose.contains("file://"), "{prose}");
            assert!(prose.contains(&format!("]({URL})")), "hash link survives: {prose}");
        }

        #[test]
        fn empty_collection_renders_nothing_in_every_format() {
            for format in [Format::Prose, Format::Markdown, Format::Plain] {
                assert_eq!(
                    RecentCommits::default().render(
                        &options(RecentCommitsVerbosity::Verbose),
                        now(),
                        format
                    ),
                    ""
                );
            }
        }
    }

    mod dates {
        use super::*;

        #[test]
        fn day_labels_use_the_options_timezone() {
            // 02:00 UTC on the 17th is 18:00 on the 16th at -08:00, where "now"
            // (18:00 UTC) is 10:00 on the 17th.
            let commit = RecentCommit {
                datetime: Utc.with_ymd_and_hms(2026, 9, 17, 2, 0, 0).unwrap(),
                ..non_conventional()
            };
            let collection = RecentCommits::from_commits(vec![commit]);
            let label = |offset_hours: i32| {
                let options = options(RecentCommitsVerbosity::Compact)
                    .timezone(FixedOffset::east_opt(offset_hours * 3600).unwrap());
                collection.render(&options, now(), Format::Plain)
            };

            assert_eq!(label(0), "- [fedcba9] at 2:00am Today: Merge branch main\n");
            assert_eq!(label(-8), "- [fedcba9] at 6:00pm Yesterday: Merge branch main\n");
            assert_eq!(label(3), "- [fedcba9] at 5:00am Today: Merge branch main\n");
        }

        #[test]
        fn older_commits_show_the_local_date() {
            let commit = RecentCommit {
                datetime: Utc.with_ymd_and_hms(2026, 4, 1, 23, 30, 0).unwrap(),
                ..non_conventional()
            };
            let options = options(RecentCommitsVerbosity::Compact)
                .timezone(FixedOffset::east_opt(5 * 3600 + 1800).unwrap());
            assert_eq!(
                RecentCommits::from_commits(vec![commit]).render(&options, now(), Format::Plain),
                "- [fedcba9] at 5:00am 2026-04-02: Merge branch main\n"
            );
        }
    }

    mod escaping {
        use super::*;

        fn hostile() -> RecentCommits {
            RecentCommits::from_commits(vec![RecentCommit {
                operation: Some("fix".to_string()),
                scope: Some("a_b".to_string()),
                heading: "handle <red>tags</red> and [x](y) and **stars** and _em_".to_string(),
                description: String::new(),
                bullet_points: vec!["use `code` \\ here".to_string()],
                files: vec![file(RecentCommitFileKind::Added, "dir (copy)/_notes_.md")],
                ..conventional()
            }])
            .with_repo_root(repo_root())
        }

        #[test]
        fn prose_and_markdown_escape_dynamic_text_and_plain_keeps_it_verbatim() {
            let verbose = options(RecentCommitsVerbosity::Verbose);

            let prose = hostile().render(&verbose, now(), Format::Prose);
            assert!(
                prose.contains(
                    r"handle \<red\>tags\</red\> and \[x\](y) and \*\*stars\*\* and \_em\_"
                ),
                "{prose}"
            );
            assert!(prose.contains(r"<blue><dim>a\_b</dim></blue>"), "{prose}");
            assert!(prose.contains(r"  - use `code` \\ here"), "{prose}");

            let markdown = hostile().render(&verbose, now(), Format::Markdown);
            assert!(
                markdown.contains(
                    r"handle \<red\>tags\</red\> and \[x\](y) and \*\*stars\*\* and \_em\_"
                ),
                "{markdown}"
            );
            assert!(markdown.contains(r"  - use \`code\` \\ here"), "{markdown}");

            let plain = hostile().render(&verbose, now(), Format::Plain);
            assert!(
                plain.contains("handle <red>tags</red> and [x](y) and **stars** and _em_"),
                "{plain}"
            );
            assert!(plain.contains("  - added: dir (copy)/_notes_.md\n"), "{plain}");
        }

        #[test]
        fn link_targets_cannot_close_early() {
            let prose =
                hostile().render(&options(RecentCommitsVerbosity::Normal), now(), Format::Prose);
            let expected_target = file_link("dir%20%28copy%29/_notes_.md");
            assert!(
                prose.contains(&format!(r"[dir (copy)/\_notes\_.md]({expected_target})")),
                "{prose}"
            );
        }
    }

    mod projections {
        use super::*;

        #[test]
        fn projected_prunes_files_and_drops_commits_left_empty() {
            let source = report().projected(RecentCommitsProjection::SourceCode);
            assert_eq!(source.len(), 1);
            let paths: Vec<&str> = source.commits()[0]
                .files
                .iter()
                .map(|file| file.path.as_str())
                .collect();
            assert_eq!(paths, ["src/lib.rs"]);
            assert_eq!(source.repo_root(), Some(repo_root().as_path()));

            let docs = report().projected(RecentCommitsProjection::Documentation);
            let paths: Vec<&str> = docs.commits()[0]
                .files
                .iter()
                .map(|file| file.path.as_str())
                .collect();
            assert_eq!(paths, ["docs/new.md", "README.md"]);
            // Commit-level facts are untouched by pruning.
            assert_eq!(docs.commits()[0].commit_url.as_deref(), Some(URL));
        }

        #[test]
        fn moved_file_counts_when_only_its_original_path_is_in_the_projection() {
            let commit = RecentCommit {
                files: vec![RecentCommitFile {
                    original_path: Some("docs/guide.md".to_string()),
                    ..file(RecentCommitFileKind::Moved, "data/guide.bin")
                }],
                ..conventional()
            };
            let projected = RecentCommits::from_commits(vec![commit])
                .projected(RecentCommitsProjection::Documentation);
            assert_eq!(projected.len(), 1);
        }

        #[test]
        fn all_projection_keeps_no_change_merges() {
            assert_eq!(report().projected(RecentCommitsProjection::All), report());
            assert!(
                report()
                    .projected(RecentCommitsProjection::SourceCode)
                    .commits()
                    .iter()
                    .all(|commit| commit.hash != non_conventional().hash)
            );
        }

        #[test]
        fn projection_reports_carry_a_heading_and_only_projected_files() {
            let options = options(RecentCommitsVerbosity::Normal)
                .projection(RecentCommitsProjection::SourceCode);

            assert_eq!(
                report().render(&options, now(), Format::Plain),
                "Source Code Changes\n\
                 \n\
                 - [0123456] feat(sniff) at 1:01pm Today: add reports\n\
                 \x20 Files Impacted:\n\
                 \x20 - modified: src/lib.rs\n"
            );
            assert!(
                report()
                    .render(&options, now(), Format::Prose)
                    .starts_with("<bold>Source Code Changes</bold>\n\n- \\[")
            );
            let documentation = options.projection(RecentCommitsProjection::Documentation);
            assert!(
                report()
                    .render(&documentation, now(), Format::Markdown)
                    .starts_with("## Documentation Changes\n\n- \\[")
            );
        }

        #[test]
        fn projection_with_no_matching_files_renders_nothing() {
            let only_merge = RecentCommits::from_commits(vec![non_conventional()]);
            let options = options(RecentCommitsVerbosity::Normal)
                .projection(RecentCommitsProjection::Documentation);
            assert_eq!(only_merge.render(&options, now(), Format::Prose), "");
        }
    }
}
