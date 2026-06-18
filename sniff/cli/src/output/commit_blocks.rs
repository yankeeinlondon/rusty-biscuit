//! Terminal-styled renderer for commit-centric output used by
//! `sniff repo recent-commits`, `sniff repo source-code-changes`, and
//! `sniff repo documentation-changes`.
//!
//! The library's [`CommitDescSet`] already produces clean markdown for
//! plain-text output. This module layers on top of the same data to emit
//! colored/bold/italic terminal output using `biscuit_terminal`'s `Prose`
//! and `UnorderedList`. The formatting policy applied here is:
//!
//! - short commit hash — **bold**, brackets left untouched
//! - conventional commit operation (e.g. `refactor`) — blue
//! - conventional commit scope (inside parens) — blue + dim; parens stay
//!   blue but NOT dim
//! - the literal word `at` — italic
//! - the time + relative-day label (e.g. `1:01pm Today`) — bold
//! - file paths — OSC8 hyperlinks via `<a href=…>` with fallback

use std::path::{Path, PathBuf};

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use chrono::{DateTime, Duration, Local, NaiveDate};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::terminal::TerminalOptions;

use sniff::filesystem::git::ConventionalCommit;
use sniff::filesystem::git::recent_commits::{CommitDesc, CommitDescSet, CommitFileChange};
use sniff::filesystem::path_kind::{is_documentation_path, is_source_code_path};

/// Which commits and which files to include in the output.
#[derive(Debug, Clone, Copy)]
pub(crate) enum CommitCentricFilter {
    /// Every commit with every file (used by `recent-commits`).
    All,
    /// Only commits that touched source-code files; non-source files are
    /// pruned from each commit's "Files Impacted" list.
    SourceCode,
    /// Only commits that touched documentation files; non-doc files are
    /// pruned from each commit's "Files Impacted" list.
    Documentation,
}

impl CommitCentricFilter {
    fn title(self) -> Option<&'static str> {
        match self {
            Self::All => None,
            Self::SourceCode => Some("Source Code Changes"),
            Self::Documentation => Some("Documentation Changes"),
        }
    }

    fn file_matches(self, path: &str) -> bool {
        let p = PathBuf::from(path);
        match self {
            Self::All => true,
            Self::SourceCode => is_source_code_path(&p),
            Self::Documentation => is_documentation_path(&p),
        }
    }
}

/// Filter a [`CommitDescSet`] in-place style by returning a new set whose
/// commits have been pruned to match `filter`.
///
/// For each commit in `set`, files are filtered by [`CommitCentricFilter::file_matches`].
/// Commits whose filtered file list is empty are dropped. `period_label`,
/// `repo_root`, and `packages` are preserved verbatim.
///
/// ## Notes
///
/// Used by the JSON path of `source-code-changes` and `documentation-changes`
/// to apply the same per-commit / per-file filtering that the styled and
/// plain text paths apply via [`render_commit_set_styled`] /
/// [`CommitDescSet::source_code_changes`].
pub(crate) fn filter_commit_set(set: &CommitDescSet, filter: CommitCentricFilter) -> CommitDescSet {
    let commits = set
        .commits
        .iter()
        .filter_map(|commit| {
            let files: Vec<CommitFileChange> = commit
                .files
                .iter()
                .filter(|f| filter.file_matches(&f.path))
                .cloned()
                .collect();
            if files.is_empty() {
                None
            } else {
                let mut filtered = commit.clone();
                filtered.files = files;
                Some(filtered)
            }
        })
        .collect();

    CommitDescSet {
        commits,
        period_label: set.period_label.clone(),
        repo_root: set.repo_root.clone(),
        packages: set.packages.clone(),
    }
}

/// Render the commit set for the terminal.
///
/// Returns an empty string when the filter is `SourceCode` or
/// `Documentation` and no commit in the set touched a matching file — the
/// CLI treats that like the library's "no results" signal.
pub(crate) fn render_commit_set_styled(
    set: &CommitDescSet,
    filter: CommitCentricFilter,
    term: &Terminal,
) -> String {
    let today = Local::now().date_naive();
    let mut out = String::new();
    let mut emitted = 0usize;

    for commit in &set.commits {
        let files: Vec<&CommitFileChange> = commit
            .files
            .iter()
            .filter(|f| filter.file_matches(&f.path))
            .collect();

        if files.is_empty() {
            continue;
        }

        if emitted == 0
            && let Some(title) = filter.title()
        {
            out.push_str(&render_section_heading(title, &set.period_label));
        }
        out.push_str(&render_commit_block(
            commit,
            &files,
            &set.repo_root,
            today,
            term,
        ));
        emitted += 1;
    }

    if emitted == 0
        && matches!(
            filter,
            CommitCentricFilter::SourceCode | CommitCentricFilter::Documentation
        )
    {
        return String::new();
    }

    out
}

/// Render the section heading via darkmatter so it matches the visual
/// treatment (block-char prefix, theme color) used by the rest of sniff's
/// markdown-driven output.
fn render_section_heading(title: &str, period_label: &str) -> String {
    let md = format!("### {} (_{}_)\n", title, period_label);
    match Markdown::from(md.as_str()).as_terminal(TerminalOptions::default()) {
        Ok(rendered) => rendered,
        Err(_) => md,
    }
}

fn render_commit_block(
    commit: &CommitDesc,
    files: &[&CommitFileChange],
    repo_root: &Path,
    today: NaiveDate,
    term: &Terminal,
) -> String {
    let mut out = String::new();

    // ------- header line -------
    let short_hash = &commit.hash[..commit.hash.len().min(7)];
    let time_label = commit_time_label(&commit.datetime, today);
    let parsed = ConventionalCommit::parse(&commit.description);

    let conventional_prefix = match (&parsed.operation, &parsed.scope) {
        (Some(op), Some(scope)) => {
            format!("<blue>{op}</blue><blue>(</blue><blue><dim>{scope}</dim></blue><blue>)</blue> ",)
        }
        (Some(op), None) => format!("<blue>{op}</blue> "),
        _ => String::new(),
    };
    let message = if parsed.operation.is_some() {
        parsed.description
    } else {
        commit.description.clone()
    };

    let header = format!(
        "[<b>{short_hash}</b>] {conventional_prefix}<i>at</i> <b>{time_label}</b>: {message}",
    );
    let rendered_header = Prose::new(header).render(term);
    let header_list = UnorderedList::new(vec![rendered_header]);
    out.push_str(&header_list.render(term));
    out.push('\n');

    // ------- Description block -------
    if !commit.bullet_points.is_empty() {
        out.push_str("    ");
        out.push_str(&Prose::new("<b>Description:</b>").render(term));
        out.push_str("\n\n");

        let items: Vec<String> = commit
            .bullet_points
            .iter()
            .map(|bp| Prose::new(bp.as_str()).render(term))
            .collect();
        let list = UnorderedList::new(items).with_bullet("    - ");
        out.push_str(&list.render(term));
        out.push('\n');
    }

    // ------- Files Impacted block -------
    out.push_str("    ");
    out.push_str(&Prose::new("<b>Files Impacted:</b>").render(term));
    out.push_str("\n\n");

    let items: Vec<String> = files
        .iter()
        .map(|f| {
            let url = file_url(&repo_root.join(&f.path));
            let content = format!("{}: <a href=\"{}\">{}</a>", f.kind, url, f.path);
            Prose::new(content).render(term)
        })
        .collect();
    let list = UnorderedList::new(items).with_bullet("    - ");
    out.push_str(&list.render(term));
    out.push('\n');

    out
}

/// Format a commit timestamp like `1:01pm Today` / `12:32pm Yesterday` /
/// `2026-04-01 at 9:30am`, converted to the viewer's local timezone so
/// relative-day labels match what the reader expects.
fn commit_time_label(datetime_rfc3339: &str, today_local: NaiveDate) -> String {
    let Ok(dt) = DateTime::parse_from_rfc3339(datetime_rfc3339) else {
        return datetime_rfc3339.to_string();
    };

    let local = dt.with_timezone(&Local);
    let time_str = local.format("%-I:%M%P").to_string();
    let commit_date = local.date_naive();
    let yesterday = today_local - Duration::days(1);

    if commit_date == today_local {
        format!("{} Today", time_str)
    } else if commit_date == yesterday {
        format!("{} Yesterday", time_str)
    } else {
        format!("{} at {}", commit_date.format("%Y-%m-%d"), time_str)
    }
}

fn file_url(path: &Path) -> String {
    url::Url::from_file_path(path)
        .map(|u| u.to_string())
        .unwrap_or_else(|()| format!("file://{}", path.to_string_lossy()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sniff::filesystem::git::discovery::DeltaKind;
    use std::path::PathBuf;

    fn change(path: &str) -> CommitFileChange {
        CommitFileChange {
            path: path.to_string(),
            kind: DeltaKind::Modified,
        }
    }

    fn commit(hash: &str, files: Vec<CommitFileChange>) -> CommitDesc {
        CommitDesc {
            hash: hash.to_string(),
            datetime: "2026-04-28T12:00:00+00:00".to_string(),
            packages: None,
            package_areas: None,
            files,
            description: format!("commit {hash}"),
            bullet_points: vec![],
        }
    }

    fn fixture_set() -> CommitDescSet {
        CommitDescSet {
            commits: vec![
                commit("aaaa", vec![change("src/main.rs"), change("README.md")]),
                commit("bbbb", vec![change("README.md")]),
            ],
            period_label: "last 3d".to_string(),
            repo_root: PathBuf::from("/tmp/repo"),
            packages: None,
        }
    }

    #[test]
    fn filter_commit_set_source_code_keeps_only_source_files() {
        let set = fixture_set();
        let filtered = filter_commit_set(&set, CommitCentricFilter::SourceCode);

        // Only the first commit has a source-code file, and only that file
        // should remain in the filtered output.
        assert_eq!(filtered.commits.len(), 1, "expected one commit kept");
        assert_eq!(filtered.commits[0].hash, "aaaa");
        let paths: Vec<&str> = filtered.commits[0]
            .files
            .iter()
            .map(|f| f.path.as_str())
            .collect();
        assert_eq!(paths, vec!["src/main.rs"]);
    }

    #[test]
    fn filter_commit_set_documentation_keeps_only_doc_files() {
        let set = fixture_set();
        let filtered = filter_commit_set(&set, CommitCentricFilter::Documentation);

        // Both commits touched README.md so both must survive, each with
        // only the docs file remaining.
        assert_eq!(filtered.commits.len(), 2);
        for commit in &filtered.commits {
            let paths: Vec<&str> = commit.files.iter().map(|f| f.path.as_str()).collect();
            assert_eq!(paths, vec!["README.md"]);
        }
    }

    #[test]
    fn filter_commit_set_all_returns_identical_data() {
        let set = fixture_set();
        let filtered = filter_commit_set(&set, CommitCentricFilter::All);

        assert_eq!(filtered.commits.len(), set.commits.len());
        for (a, b) in filtered.commits.iter().zip(set.commits.iter()) {
            assert_eq!(a.hash, b.hash);
            assert_eq!(a.files.len(), b.files.len());
        }
    }

    #[test]
    fn filter_commit_set_preserves_period_label_and_root() {
        let set = fixture_set();
        let filtered = filter_commit_set(&set, CommitCentricFilter::SourceCode);
        assert_eq!(filtered.period_label, set.period_label);
        assert_eq!(filtered.repo_root, set.repo_root);
    }
}
