//! Filesystem section output formatting (Git, Repo, Languages, Docs).

use std::fmt::Write;
use std::path::{Path, PathBuf};

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::git::{ConventionalCommit, FileAction, FileStatus, RefKind};
use sniff::filesystem::repo::Package;

use super::{TextOutput, format_number, relative_path};
use path_format::format_git_status_filepath;

/// Parsed repo filter with support for negation (`!`) and area matching (`@`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepoFilter {
    pub query: String,
    pub by_area: bool,
    pub negate: bool,
}

impl RepoFilter {
    /// Parse a filter string, stripping `!` and `@` prefixes in any order.
    pub fn parse(input: &str) -> Self {
        let mut by_area = false;
        let mut negate = false;
        let mut rest = input;

        loop {
            if let Some(stripped) = rest.strip_prefix('!') {
                negate = true;
                rest = stripped;
            } else if let Some(stripped) = rest.strip_prefix('@') {
                by_area = true;
                rest = stripped;
            } else {
                break;
            }
        }

        Self {
            query: rest.to_string(),
            by_area,
            negate,
        }
    }

    /// Check whether a package matches this filter.
    pub fn matches(&self, pkg: &Package) -> bool {
        let haystack = if self.by_area {
            &pkg.package_area
        } else {
            &pkg.name
        };
        let hit = haystack.to_lowercase().contains(&self.query.to_lowercase());
        if self.negate { !hit } else { hit }
    }
}

/// Apply an optional filter to a package list, returning matching packages.
pub(crate) fn filter_packages<'a>(packages: &'a [Package], filters: &[String]) -> Vec<&'a Package> {
    if filters.is_empty() {
        packages.iter().collect()
    } else {
        let parsed: Vec<RepoFilter> = filters.iter().map(|f| RepoFilter::parse(f)).collect();
        packages
            .iter()
            .filter(|p| parsed.iter().any(|f| f.matches(p)))
            .collect()
    }
}

mod deps;
mod docs;
mod files;
mod language;
mod package_areas;
mod packages;
mod path_format;
mod repo;

// Re-exports from submodules
pub use deps::{render_repo_deps_svg, render_repo_deps_text, render_repo_deps_visual};
pub use docs::render_docs_output;
pub(crate) use files::filter_file_breakdown;
pub use files::{PathListFormat, render_files_section, render_path_list};
pub(crate) use language::primary_language_name;
pub use language::{render_language_section, render_repo_language};
pub use package_areas::{
    collect_repo_package_area_names, render_dirty_package_areas, render_repo_area,
    render_repo_package_area, render_repo_package_area_root, render_repo_package_areas_formatted,
    render_staged_package_areas, render_unstaged_package_areas,
};
pub(crate) use package_areas::{
    select_dirty_package_area_names, select_staged_package_area_names,
    select_unstaged_package_area_names,
};
pub use packages::{
    collect_repo_package_names, render_dirty_packages, render_repo_package,
    render_repo_package_root, render_repo_packages_formatted, render_staged_packages,
    render_unstaged_packages,
};
pub(crate) use packages::{
    select_dirty_package_names, select_repo_packages, select_staged_package_names,
    select_unstaged_package_names,
};
pub(crate) use repo::format_monorepo_label;
pub use repo::{
    render_filesystem_section, render_repo_default_verbose, render_repo_name, render_repo_section,
};

/// Format a commit datetime to a relative date string and 12hr time string.
///
/// Returns a tuple of (date_string, time_string, use_on) where:
/// - date_string is "Today", "Yesterday", or "YYYY-MM-DD"
/// - time_string is in 12hr format with am/pm (e.g., "2:30pm")
/// - use_on is true for full dates (use "on 2026-01-31"), false for relative (just "Today")
fn format_commit_datetime(timestamp: &chrono::DateTime<chrono::Utc>) -> (String, String, bool) {
    use chrono::{Local, Timelike};

    // Convert to local timezone
    let local_time = timestamp.with_timezone(&Local);
    let today = Local::now().date_naive();
    let commit_date = local_time.date_naive();

    // Determine relative date and whether to use "on"
    let (date_str, use_on) = if commit_date == today {
        ("Today".to_string(), false)
    } else if commit_date == today.pred_opt().unwrap_or(today) {
        ("Yesterday".to_string(), false)
    } else {
        (commit_date.format("%Y-%m-%d").to_string(), true)
    };

    // Format time in 12hr format
    let hour = local_time.hour();
    let minute = local_time.minute();
    let (hour_12, period) = if hour == 0 {
        (12, "am")
    } else if hour < 12 {
        (hour, "am")
    } else if hour == 12 {
        (12, "pm")
    } else {
        (hour - 12, "pm")
    };
    let time_str = format!("{}:{:02}{}", hour_12, minute, period);

    (date_str, time_str, use_on)
}

/// Format ref decorations for a commit (branches, tags, remote tracking refs).
///
/// Returns a formatted string like `(HEAD -> main, origin/main, v1.0.0)` with
/// appropriate colors:
/// - Cyan for local branches (HEAD indicator in bold)
/// - Green for remote tracking branches
/// - Yellow for tags
fn format_ref_decorations(refs: &[sniff::filesystem::git::RefDecoration]) -> String {
    if refs.is_empty() {
        return String::new();
    }

    let parts: Vec<String> = refs
        .iter()
        .map(|r| {
            let name = &r.name;
            match r.kind {
                RefKind::LocalBranch => {
                    if r.is_head {
                        format!("<cyan><b>HEAD -></b> {}</cyan>", name)
                    } else {
                        format!("<cyan>{}</cyan>", name)
                    }
                }
                RefKind::RemoteBranch => {
                    format!("<green>{}</green>", name)
                }
                RefKind::Tag => {
                    format!("<yellow>{}</yellow>", name)
                }
            }
        })
        .collect();

    format!(" <dim>(</dim>{}<dim>)</dim>", parts.join("<dim>, </dim>"))
}

/// Parse a git remote URL to extract owner/repo and browsable URL.
///
/// Handles both SSH (`git@github.com:owner/repo.git`) and HTTPS
/// (`https://github.com/owner/repo.git`) formats.
///
/// Returns (owner/repo, browsable_url) tuple.
fn parse_git_url(
    url: &str,
    provider: &sniff::filesystem::git::GitHostingProvider,
) -> (Option<String>, Option<String>) {
    // Try to extract owner/repo from URL
    let owner_repo = if url.contains('@') && url.contains(':') {
        // SSH format: git@github.com:owner/repo.git
        url.split(':')
            .next_back()
            .map(|s| s.trim_end_matches(".git").to_string())
    } else if url.contains("://") {
        // HTTPS format: https://github.com/owner/repo.git
        url.split('/')
            .skip(3) // Skip https://hostname/
            .collect::<Vec<_>>()
            .join("/")
            .trim_end_matches(".git")
            .to_string()
            .into()
    } else {
        None
    };

    // Build browsable URL based on provider
    let browse_url = owner_repo.as_ref().and_then(|repo| {
        provider
            .browser_base_url()
            .map(|base| format!("{}/{}", base, repo))
    });

    (owner_repo, browse_url)
}

/// Build the commit URL base from the preferred remote (usually "origin").
///
/// Returns `(browse_url, provider, remote_name)` if a browsable remote is
/// found, or `None` if no remote has a resolvable browse URL.
fn build_commit_url_base(
    git: &sniff::filesystem::git::GitInfo,
) -> Option<(String, sniff::filesystem::git::GitHostingProvider, String)> {
    // Prefer "origin", fall back to the first remote with a URL
    let remote = git
        .remotes
        .iter()
        .find(|r| r.name == "origin")
        .or_else(|| git.remotes.first())?;
    let url = remote.url.as_ref()?;
    let (_, browse_url) = parse_git_url(url, &remote.provider);
    browse_url.map(|base| (base, remote.provider, remote.name.clone()))
}

/// Split a path into directory and filename components.
fn split_path(path: &str) -> (String, String) {
    if let Some(pos) = path.rfind('/') {
        let dir = &path[..=pos];
        let name = &path[pos + 1..];
        (dir.to_string(), name.to_string())
    } else {
        (String::new(), path.to_string())
    }
}

/// Formats diff stats as ` - <green-500>N added</green-500>, <red-500>N removed</red-500>`.
/// Returns an empty string if both counts are zero.
fn format_diff_stats(added: usize, removed: usize) -> String {
    if added == 0 && removed == 0 {
        return String::new();
    }
    format!(
        " - <dim><green-500>{added} <i>added</i></green-500>, <red-500>{removed} <i>removed</i></red-500></dim>"
    )
}

/// Formats ahead/behind counts relative to a base branch as styled markup.
///
/// Includes the base branch name in the output. Returns phrases like
/// `"3 ahead of main"`, `"2 behind main"`, or `"up to date with main"`.
fn format_ahead_behind_of(ahead: usize, behind: usize, base: &str) -> String {
    match (ahead, behind) {
        (0, 0) => format!("up to date with <b>{base}</b>"),
        (a, 0) => format!("<green-500>{a} ahead</green-500> of <b>{base}</b>"),
        (0, b) => format!("<red-500>{b} behind</red-500> <b>{base}</b>"),
        (a, b) => format!(
            "<green-500>{a} ahead</green-500>, <red-500>{b} behind</red-500> of <b>{base}</b>"
        ),
    }
}

/// Format a single commit as a styled one-liner.
///
/// Parses conventional commit format and includes SHA, timestamp, ref decorations,
/// and optionally the author (when `verbose > 0`).
///
/// When `commit_url` is `Some`, the SHA is rendered as an OSC8 hyperlink.
fn format_commit_line(
    commit: &sniff::filesystem::git::CommitInfo,
    verbose: u8,
    commit_url: Option<&str>,
) -> String {
    let cc = ConventionalCommit::parse(&commit.message);
    let (date_str, time_str, use_on) = format_commit_datetime(&commit.timestamp);
    let short_sha = &commit.sha[0..7];
    let sha_display = match commit_url {
        Some(url) => format!("<a href=\"{url}\"><b>{short_sha}</b></a>"),
        None => format!("<dim><i>{short_sha}</i></dim>"),
    };
    let date_prefix = if use_on { "<i>on</i> " } else { "" };
    let refs_part = format_ref_decorations(&commit.refs);
    let user_part = if verbose > 0 {
        format!(
            " <dim><i>by </i></dim><b><indigo-500>{}</indigo-500></b>",
            commit.author
        )
    } else {
        String::new()
    };

    if let Some(ref op) = cc.operation {
        let scope_part = cc
            .scope
            .as_ref()
            .map(|s| format!("(<dim>{}</dim>)", s))
            .unwrap_or_default();
        format!(
            "[{}] <b><yellow>{}</yellow></b>{} <i>at</i> <blue><b>{}</b></blue> {}<blue>{}</blue>{}{}: <dim>{}</dim>",
            sha_display,
            op,
            scope_part,
            time_str,
            date_prefix,
            date_str,
            refs_part,
            user_part,
            cc.description
        )
    } else {
        // Non-conventional commit
        let first_line = commit.message.lines().next().unwrap_or("");
        let truncated = if first_line.len() > 50 {
            format!("{}...", &first_line[..47])
        } else {
            first_line.to_string()
        };
        format!(
            "[{}] <dim>{}</dim> <i>at</i> <blue><b>{}</b></blue> {}<blue><b>{}</b></blue>{}{}",
            sha_display, truncated, time_str, date_prefix, date_str, refs_part, user_part,
        )
    }
}

/// Format the commit header for the `hash` subcommand.
///
/// Includes the SHA, conventional commit type/scope, timestamp, and author,
/// but not the commit description. The SHA is rendered as an OSC8 hyperlink
/// when `commit_url` is `Some`.
fn format_commit_hash_header(
    commit: &sniff::filesystem::git::CommitInfo,
    commit_url: Option<&str>,
) -> String {
    let cc = ConventionalCommit::parse(&commit.message);
    let (date_str, time_str, use_on) = format_commit_datetime(&commit.timestamp);
    let short_sha = &commit.sha[0..7];
    let sha_display = match commit_url {
        Some(url) => format!("<a href=\"{url}\"><b>{short_sha}</b></a>"),
        None => format!("<dim><i>{short_sha}</i></dim>"),
    };
    let date_prefix = if use_on { "<i>on</i> " } else { "" };
    let refs_part = format_ref_decorations(&commit.refs);
    let user_part = format!(
        " <dim><i>by </i></dim><b><indigo-500>{}</indigo-500></b>",
        commit.author
    );

    if let Some(ref op) = cc.operation {
        let scope_part = cc
            .scope
            .as_ref()
            .map(|s| format!("(<dim>{}</dim>)", s))
            .unwrap_or_default();
        format!(
            "[{}] <b><yellow>{}</yellow></b>{} <i>at</i> <blue><b>{}</b></blue> {}<blue>{}</blue>{}{}",
            sha_display, op, scope_part, time_str, date_prefix, date_str, refs_part, user_part,
        )
    } else {
        // Non-conventional commit
        format!(
            "[{}] <i>at</i> <blue><b>{}</b></blue> {}<blue><b>{}</b></blue>{}{}",
            sha_display, time_str, date_prefix, date_str, refs_part, user_part,
        )
    }
}

/// Render detailed information for a single commit looked up by `hash` subcommand.
///
/// Shows the commit header, description, body bullet points, and files changed.
pub fn render_hash_section(
    commit: &sniff::filesystem::git::CommitInfo,
    files: &[(std::path::PathBuf, sniff::filesystem::git::DeltaKind)],
    _verbose: u8,
    commit_url: Option<&str>,
) -> String {
    use sniff::filesystem::git::DeltaKind;

    let mut out = String::new();
    let terminal = Terminal::default();

    // === Commit Section ===
    let status_title = Prose::new("<b><u>Commit</u></b>");
    writeln!(out, "\n{}\n", status_title.render(&terminal)).unwrap();

    // Header line: [sha] type(scope) at time date by author
    let header = format_commit_hash_header(commit, commit_url);
    writeln!(out, "{}", Prose::new(header.as_str()).render(&terminal)).unwrap();

    // First line of the commit message (description)
    let cc = ConventionalCommit::parse(&commit.message);
    let description = cc.description.trim();
    if !description.is_empty() {
        writeln!(out).unwrap();
        writeln!(out, "{}", Prose::new(description).render(&terminal)).unwrap();
    }

    // === Body bullet points (commit message body) ===
    let (_, bullet_points) = sniff::filesystem::git::parse_commit_message(&commit.message);
    if !bullet_points.is_empty() {
        writeln!(out).unwrap();
        let items: Vec<String> = bullet_points
            .iter()
            .map(|bp| Prose::new(bp.as_str()).render(&terminal))
            .collect();
        let list = UnorderedList::new(items).with_bullet("  - ");
        writeln!(out, "{}", list.render(&terminal)).unwrap();
    }

    // === Files Section ===
    if files.is_empty() {
        return out;
    }

    let files_title = Prose::new("<b><u>Files changed</u></b>");
    writeln!(out, "\n{}\n", files_title.render(&terminal)).unwrap();

    let file_items: Vec<String> = files
        .iter()
        .map(|(path, kind)| {
            let path_str = path.display().to_string();
            let (dir, name) = split_path(&path_str);
            let dir_part = if dir.is_empty() { String::new() } else { dir };
            match kind {
                DeltaKind::Added => {
                    format!("<lime><i>{}</i></lime> {}<b>{}</b>", kind, dir_part, name)
                }
                DeltaKind::Modified => {
                    format!("<yellow><i>{}</i></yellow> {}<b>{}</b>", kind, dir_part, name)
                }
                DeltaKind::Deleted => {
                    format!("<red><i>{}</i></red> {}<b>{}</b>", kind, dir_part, name)
                }
                DeltaKind::Renamed | DeltaKind::Copied => {
                    format!("<cyan><i>{}</i></cyan> {}<b>{}</b>", kind, dir_part, name)
                }
            }
        })
        .collect();

    let rendered_items: Vec<String> = file_items
        .iter()
        .map(|item| Prose::new(item.as_str()).render(&terminal))
        .collect();
    let list = UnorderedList::new(rendered_items);
    writeln!(out, "{}", list.render(&terminal)).unwrap();

    out
}

/// Print git information with rich terminal formatting.
///
/// Uses biscuit-terminal's Prose component for styled output with two sections:
/// - **Status**: Recent commits (with conventional commit parsing), staged/modified/untracked files
/// - **Meta**: Remote tracking status, branches, git config
///
fn build_git_status_items(
    git: &sniff::filesystem::git::GitInfo,
    history_count: usize,
    verbose: u8,
) -> Vec<String> {
    // Build commit URL base from the preferred remote (usually "origin").
    let commit_url_base = build_commit_url_base(git);
    let url_remote_prefix = commit_url_base
        .as_ref()
        .map(|(_, _, name)| format!("{name}/"));

    let mut status_items: Vec<String> = Vec::new();

    // Recent commits with conventional commit parsing (oldest first, so most recent is at bottom).
    // `git.recent` is newest-first. A commit is considered pushed once we encounter
    // (walking newest→oldest) a commit with a remote-tracking ref decoration for the
    // URL-providing remote; that commit and all older ones are pushed. This is robust
    // to `--branch <other>` queries where `git.tracking` reflects the checked-out
    // branch, not the queried one.
    let commits: Vec<_> = git.recent.iter().take(history_count).collect();
    let unpushed_count = url_remote_prefix
        .as_deref()
        .map(|prefix| {
            commits
                .iter()
                .take_while(|c| {
                    !c.refs
                        .iter()
                        .any(|r| r.kind == RefKind::RemoteBranch && r.name.starts_with(prefix))
                })
                .count()
        })
        .unwrap_or(0);

    for (display_index, commit) in commits.iter().rev().enumerate() {
        // display_index 0 = oldest displayed commit, last = most recent.
        // The most recent `unpushed_count` commits (at the end) are unpushed.
        let is_pushed = display_index < commits.len().saturating_sub(unpushed_count);
        let commit_url = if is_pushed {
            commit_url_base.as_ref().map(|(base, provider, _)| {
                format!("{}/{}/{}", base, provider.commit_path_segment(), commit.sha)
            })
        } else {
            None
        };
        status_items.push(format_commit_line(commit, verbose, commit_url.as_deref()));
    }

    let staged: Vec<_> = git
        .file_changes
        .iter()
        .filter(|f| f.status == FileStatus::Staged || f.status == FileStatus::Both)
        .collect();
    let modified: Vec<_> = git
        .file_changes
        .iter()
        .filter(|f| f.status == FileStatus::Modified || f.status == FileStatus::Both)
        .collect();
    let untracked: Vec<_> = git
        .file_changes
        .iter()
        .filter(|f| f.status == FileStatus::Untracked)
        .collect();

    // Add staged files
    for file in &staged {
        let path = file.path.display().to_string();
        let absolute = git.repo_root.join(&file.path).display().to_string();
        let linked_path = format_git_status_filepath(&path, &absolute);
        let action = file.action.label();
        // Only show diff stats for modified files (not created/deleted)
        let diff_stats = if file.action == FileAction::Modified {
            format_diff_stats(file.lines_added, file.lines_removed)
        } else {
            String::new()
        };
        let line =
            format!("<lime>staged(<dim><i>{action}</i></dim>): {linked_path}</lime>{diff_stats}");
        status_items.push(line);
    }

    // Add unstaged files
    for file in &modified {
        let path = file.path.display().to_string();
        let absolute = git.repo_root.join(&file.path).display().to_string();
        let linked_path = format_git_status_filepath(&path, &absolute);
        let action = file.action.label();
        let diff_stats = format_diff_stats(file.lines_added, file.lines_removed);
        let line = format!(
            "<yellow>unstaged(<dim><i>{action}</i></dim>): {linked_path}</yellow>{diff_stats}"
        );
        status_items.push(line);
    }

    for file in &untracked {
        let path = file.path.display().to_string();
        let absolute = git.repo_root.join(&file.path).display().to_string();
        let linked_path = format_git_status_filepath(&path, &absolute);
        let line = format!("<dim>untracked: {linked_path}</dim>");
        status_items.push(line);
    }

    // Conflicted files render last so they're easy to spot at the bottom of the list.
    let conflicted: Vec<_> = git
        .file_changes
        .iter()
        .filter(|f| f.status == FileStatus::Conflicted)
        .collect();
    for file in &conflicted {
        let path = file.path.display().to_string();
        let absolute = git.repo_root.join(&file.path).display().to_string();
        let linked_path = format_git_status_filepath(&path, &absolute);
        let line = format!("<red>conflicted: {linked_path}</red>");
        status_items.push(line);
    }

    status_items
}

/// ## Arguments
///
/// * `git` - Git repository information
/// * `history_count` - Number of recent commits to display
/// * `target_branch` - When `Some`, annotates the Status heading with the
///   branch whose commits are being shown (used by `--branch <name>`).
/// * `target_worktree` - When `Some((name, branch))`, annotates the heading
///   to indicate the commits and working-tree state are scoped to the named
///   linked worktree (used by `--worktree <name>`). Takes precedence over
///   `target_branch` when both are provided.
///
/// Renders a section header with double-underline styling.
///
/// Emits `<b><uu>{title}</uu></b>` which degrades gracefully on terminals
/// that do not support double underline via `biscuit-terminal`.
fn render_header(title: &str, terminal: &Terminal) -> String {
    Prose::new(format!("<b><uu>{title}</uu></b>")).render(terminal)
}

/// Formats a worktree directory path as a blue OSC8 hyperlink.
///
/// The href is the absolute path; the visible label is computed relative to
/// the current worktree directory so sibling or parent layouts read as `..`,
/// `../project`, or `.` instead of a home-abbreviated absolute path.
fn worktree_path_link(path: &std::path::Path, current_worktree: &std::path::Path) -> String {
    let absolute = path.display().to_string();
    let label = relative_path_between(current_worktree, path);
    format!("<blue><a href=\"{absolute}\">{label}</a></blue>")
}

/// Formats a worktree directory path as a blue OSC8 hyperlink whose href is the
/// full absolute path and whose visible label is an aliased, compact form (see
/// [`alias_path`]).
///
/// Used for the current worktree's own "located at" line, where a label
/// relative to itself would degenerate to `.` and tell the reader nothing.
fn worktree_path_link_absolute(path: &std::path::Path) -> String {
    let absolute = path.display().to_string();
    let label = alias_path(path);
    format!("<blue><a href=\"{absolute}\">{label}</a></blue>")
}

/// Computes a compact display label for an absolute path by offsetting it
/// against an environment variable or the home directory.
///
/// Resolution ladder (first match wins):
/// 1. The environment variable whose value is the longest path-prefix of
///    `path` renders as `${VAR}/<rest>`. Ties on prefix length are broken by
///    the lexicographically-first variable name so output is deterministic.
/// 2. A path under `$HOME` renders as `~/<rest>`.
/// 3. Anything else renders as its absolute path.
///
/// An env-var offset wins only when it is a strictly longer prefix than
/// `$HOME`; otherwise the `~` form is preferred, since it reads better for the
/// home directory itself.
fn alias_path(path: &Path) -> String {
    let vars: Vec<(std::ffi::OsString, std::ffi::OsString)> = std::env::vars_os().collect();
    // `dirs::home_dir()` resolves the platform home directory (USERPROFILE /
    // known-folder on Windows), not just `$HOME`, so the `~` form works there.
    let home = dirs::home_dir();
    alias_path_with(path, &vars, home.as_deref())
}

/// Environment variables whose values are absolute paths that must never be
/// surfaced as a `${VAR}` alias.
///
/// `PWD` / `OLDPWD` denote *transient shell position* (the current / previous
/// directory): offsetting against them yields a label that changes on every
/// `cd`. On Windows, `USERPROFILE` mirrors the home directory (already rendered
/// as `~` via [`dirs::home_dir`]) and `HOMEDRIVE` / `HOMEPATH` are bare or
/// partial roots (`C:\`, `\Users\ken`) that would alias almost every path to a
/// confusing `${HOMEDRIVE}\...`. Variables whose values are not absolute paths
/// (`TERM`, `LANG`, `SHLVL`, ...) are already filtered by the `is_absolute`
/// check below and need no entry here.
const SKIP_ALIAS_VARS: &[&str] = &["PWD", "OLDPWD", "USERPROFILE", "HOMEDRIVE", "HOMEPATH"];

/// Compares two environment variable names for equality.
///
/// Windows treats environment variable names case-insensitively, so matching is
/// ASCII-case-insensitive there; Unix-like hosts keep exact, case-sensitive
/// matching. The flag is passed explicitly rather than read from `cfg!` so
/// Windows behavior is exercisable in tests on any host.
fn env_name_eq(a: &str, b: &str, case_insensitive: bool) -> bool {
    if case_insensitive {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

/// Pure core of [`alias_path`], with the environment supplied explicitly so it
/// can be tested without mutating global process state. Env-var-name case
/// sensitivity follows the host: case-insensitive on Windows, exact elsewhere.
fn alias_path_with(
    path: &Path,
    vars: &[(std::ffi::OsString, std::ffi::OsString)],
    home: Option<&Path>,
) -> String {
    alias_path_with_case(path, vars, home, cfg!(windows))
}

/// Pure core of [`alias_path_with`] with env-var-name case sensitivity supplied
/// explicitly so Windows behavior is testable on any host.
fn alias_path_with_case(
    path: &Path,
    vars: &[(std::ffi::OsString, std::ffi::OsString)],
    home: Option<&Path>,
    case_insensitive_names: bool,
) -> String {
    // "Longest prefix" is measured in path components, not bytes, so a trailing
    // slash in an env value can't spuriously outrank a real ancestor.
    let home_components = home.map(|h| h.components().count()).unwrap_or(0);

    let mut best: Option<(String, PathBuf, usize)> = None;
    for (name, value) in vars {
        let name = name.to_string_lossy();
        if SKIP_ALIAS_VARS
            .iter()
            .any(|v| env_name_eq(name.as_ref(), v, case_insensitive_names))
        {
            continue;
        }
        let value = PathBuf::from(value);
        if !value.is_absolute() || !path.starts_with(&value) {
            continue;
        }
        let components = value.components().count();
        let better = match &best {
            None => true,
            Some((best_name, _, best_components)) => {
                components > *best_components
                    || (components == *best_components && name.as_ref() < best_name.as_str())
            }
        };
        if better {
            best = Some((name.into_owned(), value, components));
        }
    }

    if let Some((name, value, components)) = best
        && components > home_components
        && let Ok(rel) = path.strip_prefix(&value)
    {
        return join_alias(&format!("${{{name}}}"), rel);
    }

    if let Some(home) = home
        && let Ok(rel) = path.strip_prefix(home)
    {
        return join_alias("~", rel);
    }

    path.display().to_string()
}

/// Joins an alias prefix (`~` or `${VAR}`) with the remaining relative path,
/// collapsing to the bare prefix when the path is an exact match.
fn join_alias(prefix: &str, rel: &Path) -> String {
    if rel.as_os_str().is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}/{}", rel.display())
    }
}

/// Computes a relative path from `base` to `target`.
///
/// Returns `.` when the two paths are the same, otherwise a sequence of `..`
/// segments and/or the remaining target components. Falls back to the target's
/// absolute display when the paths do not share a common prefix.
fn relative_path_between(base: &std::path::Path, target: &std::path::Path) -> String {
    use std::path::{Component, MAIN_SEPARATOR_STR};

    if let Ok(rel) = target.strip_prefix(base) {
        return if rel.as_os_str().is_empty() {
            ".".to_string()
        } else {
            rel.display().to_string()
        };
    }

    if let Ok(rel) = base.strip_prefix(target) {
        let ups = rel.components().count();
        return std::iter::repeat_n("..", ups)
            .collect::<Vec<_>>()
            .join(MAIN_SEPARATOR_STR);
    }

    let base_components: Vec<_> = base.components().collect();
    let target_components: Vec<_> = target.components().collect();
    let common = base_components
        .iter()
        .zip(target_components.iter())
        .take_while(|(a, b)| a == b)
        .count();

    // When the paths diverge at (or before) the filesystem root — e.g. a
    // worktree under `/Users/...` next to one under `/Volumes/...` — the only
    // shared component is the root, and a relative label degenerates into a
    // long `../` chain back to root. The absolute path is shorter and clearer.
    let common_is_root_only = common <= 1
        && base_components
            .first()
            .is_some_and(|c| matches!(c, Component::RootDir | Component::Prefix(_)));

    if common == 0 || common_is_root_only {
        return target.display().to_string();
    }

    let ups = base_components.len().saturating_sub(common);
    let mut parts: Vec<String> = std::iter::repeat_n("..".to_string(), ups).collect();
    parts.extend(
        target_components[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );

    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join(MAIN_SEPARATOR_STR)
    }
}

pub fn render_git_section(
    git: &sniff::filesystem::git::GitInfo,
    history_count: usize,
    verbose: u8,
    compact: bool,
    target_branch: Option<&str>,
    target_worktree: Option<(&str, &str)>,
) -> String {
    let mut out = String::new();
    let terminal = Terminal::default();

    // === Status Section ===
    out.push('\n');
    let status_title = match (target_worktree, target_branch) {
        (Some((wt, branch)), _) => render_header(
            &format!(
                "Status (<dim>worktree: <green>{wt}</green> <i>on</i> <green><i>{branch}</i></green> <i>branch</i></dim>)"
            ),
            &terminal,
        ),
        (None, Some(branch)) => {
            render_header(&format!("Status (branch: <i>{branch}</i>)"), &terminal)
        }
        (None, None) => render_header("Status", &terminal),
    };
    writeln!(out, "{}", status_title).unwrap();
    out.push('\n');

    let status_items = build_git_status_items(git, history_count, verbose);

    // Render status items as list
    if !status_items.is_empty() {
        let rendered_items: Vec<String> = status_items
            .iter()
            .map(|item| Prose::new(item.as_str()).render(&terminal))
            .collect();
        let list = UnorderedList::new(rendered_items);
        writeln!(out, "{}", list.render(&terminal)).unwrap();
    } else {
        let clean = Prose::new("<dim>No changes</dim>");
        writeln!(out, "  {}", clean.render(&terminal)).unwrap();
    }

    if compact {
        return out;
    }

    // === Worktrees Section ===
    // The dedicated Worktrees section is only rendered when the repo has at
    // least one linked worktree. When we are in the main worktree and none are
    // defined there is nothing to enumerate, so instead of an empty section two
    // trailing bullets are appended to the Status list above (a blank-line-
    // separated repo-location line and a "no worktrees" note).
    //
    // Case selection is by *physical location* (`in_worktree`): are we running
    // inside the main worktree or a linked one? Branch spelling (`== "main"`)
    // misclassifies detached HEAD, `master`-default repos, and any non-main
    // branch checked out in the main worktree — none of which have a usable
    // entry in the linked-worktree-only `worktrees` map. Running inside a linked
    // worktree (`in_worktree`) always implies at least one worktree exists.
    if !git.in_worktree && git.worktrees.is_empty() {
        out.push('\n');
        let repo_link = worktree_path_link_absolute(&git.repo_root);
        let mut extra = UnorderedList::empty();
        extra.add(Prose::new(format!("this repo is located at {repo_link}")));
        extra.add(Prose::new("no <i>worktrees</i> are defined".to_string()));
        writeln!(out, "{}", extra.render(&terminal)).unwrap();
    } else {
        out.push('\n');
        writeln!(out, "{}", render_header("Worktrees", &terminal)).unwrap();
        out.push('\n');

        let mut wt_list = UnorderedList::empty();

        if git.in_worktree {
            // Case A: running inside a linked worktree. Show where main lives,
            // then this worktree's own details, then a count of the rest.
            let main_root = git.base_repo_root.as_deref().unwrap_or(&git.repo_root);
            let main_path = worktree_path_link(main_root, &git.repo_root);
            wt_list.add(Prose::new(format!(
                "<b>main:</b> <i>the main worktree for this repo is located at </i>{main_path}"
            )));

            // The current worktree's display name is its directory basename,
            // not its branch — they routinely differ (e.g. `login-fix` on
            // `feature/login`).
            let current_name = git
                .repo_root
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("worktree");
            let current_path = worktree_path_link_absolute(&git.repo_root);
            wt_list.add(Prose::new("<b>Current Worktree:</b>".to_string()));

            let mut current_list = UnorderedList::empty();
            current_list.add(Prose::new(format!(
                "you are in the <b>{current_name}</b> worktree located at {current_path}"
            )));

            // ahead/behind comes from the current worktree's own map entry,
            // which detection always computes in full because it is `is_current`.
            let current_entry = git.worktrees.values().find(|wt| wt.is_current);
            let branch_label = git
                .current_branch
                .clone()
                .unwrap_or_else(|| "detached HEAD".to_string());
            let ab = match current_entry {
                Some(wt) => format_ahead_behind_of(wt.ahead, wt.behind, &wt.base_branch),
                None => format_ahead_behind_of(0, 0, "main"),
            };
            current_list.add(Prose::new(format!(
                "this worktree is on the <b>{branch_label}</b> branch and is {ab}"
            )));
            wt_list.add(current_list);

            let other_count = git.worktrees.values().filter(|wt| !wt.is_current).count();
            wt_list.add(Prose::new("<b>Other Worktrees:</b>".to_string()));
            let mut other_list = UnorderedList::empty();
            other_list.add(Prose::new(format!(
                "there are {other_count} other active worktrees in this repo"
            )));
            wt_list.add(other_list);
        } else {
            // Case B: running inside the main worktree. The main worktree never
            // appears in the linked-worktree map, so every entry is "other".
            let main_path = worktree_path_link_absolute(&git.repo_root);
            wt_list.add(Prose::new("<b>Current Worktree:</b>".to_string()));

            let mut current_list = UnorderedList::empty();
            current_list.add(Prose::new(format!(
                "you are in the <b>main</b> worktree located at {main_path}"
            )));
            wt_list.add(current_list);

            let other_count = git.worktrees.len();
            wt_list.add(Prose::new("<b>Other Worktrees:</b>".to_string()));
            let mut other_list = UnorderedList::empty();
            other_list.add(Prose::new(format!(
                "there are {other_count} other active worktrees in this repo"
            )));
            wt_list.add(other_list);
        }

        writeln!(out, "{}", wt_list.render(&terminal)).unwrap();
    }

    // === Meta Section ===
    out.push('\n');
    writeln!(out, "{}", render_header("Meta", &terminal)).unwrap();
    out.push('\n');

    let mut meta_list = UnorderedList::empty();

    // --- Local ---
    if let Some(ref current) = git.current_branch {
        let local_header: RenderableTerminalContent = Prose::new("<b>Local:</b>").into();
        if verbose > 0 {
            // Verbose: nested list with current branch + other branches
            let mut local_list = UnorderedList::empty();

            // Identity-only `GitInfo` carries no status; render no dirty marker
            // rather than asserting cleanliness (or panicking) on absent status.
            let dirty = if git.status.as_ref().is_some_and(|s| s.is_dirty) {
                "<red>+</red>"
            } else {
                ""
            };

            // Find current branch's short hash
            let current_hash = git
                .branches
                .iter()
                .find(|b| b.name == *current)
                .map(|b| b.short_hash.as_str())
                .unwrap_or("");

            local_list.add(Prose::new(format!(
                "<b><blue>{}{}</blue></b> [<dim>{}</dim>] (<dim><i>current</i></dim>)",
                current, dirty, current_hash
            )));

            for branch in git.branches.iter().filter(|b| b.name != *current) {
                let ab = format_ahead_behind(branch.ahead, branch.behind, terminal.is_nerd_font);
                local_list.add(Prose::new(format!(
                    "{} [<dim>{}</dim>] - {}",
                    branch.name, branch.short_hash, ab
                )));
            }

            let branches_header: RenderableTerminalContent = Prose::new("<b>Branches:</b>").into();
            let mut local_wrapper = UnorderedList::empty();
            local_wrapper.add(branches_header);
            local_wrapper.add(local_list);

            meta_list.add(local_header);
            meta_list.add(local_wrapper);
        } else {
            // Normal: single line with current branch and others in parens
            let other_branches: Vec<_> = git
                .branches
                .iter()
                .filter(|b| b.name != *current)
                .take(3)
                .map(|b| b.name.clone())
                .collect();

            let branch_line = if other_branches.is_empty() {
                format!("<b>Branches:</b> <blue>{}</blue>", current)
            } else {
                let others = other_branches.join(", ");
                let more = if git.branches.len() > 4 {
                    format!(", +{} more", git.branches.len() - 4)
                } else {
                    String::new()
                };
                format!(
                    "<b>Branches:</b> <blue>{}</blue> (<dim>{}{}</dim>)",
                    current, others, more
                )
            };

            let mut local_list = UnorderedList::empty();
            local_list.add(Prose::new(branch_line));

            meta_list.add(local_header);
            meta_list.add(local_list);
        }
    }

    // --- Remotes ---
    if !git.tracking.is_empty() || !git.remotes.is_empty() {
        let remotes_header: RenderableTerminalContent = Prose::new("<b>Remotes:</b>").into();
        let mut remotes_list = UnorderedList::empty();

        for remote in &git.remotes {
            // Build ahead/behind with arrows
            let tracking_part = git
                .tracking
                .iter()
                .find(|t| t.remote == remote.name)
                .map(|t| format_ahead_behind(t.ahead, t.behind, terminal.is_nerd_font))
                .unwrap_or_default();

            // Parse owner/repo from URL and build display string with link
            let repo_link = remote
                .url
                .as_ref()
                .map(|url| {
                    let (owner_repo, browse_url) = parse_git_url(url, &remote.provider);
                    let provider_label = remote.provider.display_name();
                    if let Some(ref repo_path) = owner_repo {
                        let link_url = browse_url.unwrap_or_else(|| url.clone());
                        format!(
                            " - <a href=\"{}\"><blue>{}</blue></a> <i>on</i> <b>{}</b>",
                            link_url, repo_path, provider_label
                        )
                    } else {
                        format!(" <i>on</i> <b>{}</b>", provider_label)
                    }
                })
                .unwrap_or_default();

            // Only show a remote default branch when refreshed remote data is available.
            let branch_part = remote
                .default_branch
                .as_ref()
                .map(|b| format!(" <i>of</i> {}", b))
                .unwrap_or_default();

            let line = if tracking_part.is_empty() {
                format!("<b>{}:</b>{}{}", remote.name, branch_part, repo_link)
            } else {
                format!(
                    "<b>{}:</b> {}{}{}",
                    remote.name, tracking_part, branch_part, repo_link
                )
            };

            remotes_list.add(Prose::new(line));

            // Verbose: show remote branches as nested list, excluding the default branch
            if verbose > 0
                && let Some(ref branches) = remote.branches
            {
                let default = remote.default_branch.as_deref();
                let non_default: Vec<_> = branches
                    .iter()
                    .filter(|b| default.is_none_or(|d| b.as_str() != d))
                    .collect();
                if !non_default.is_empty() {
                    let mut branch_list = UnorderedList::empty();
                    for branch in non_default {
                        branch_list.add(Prose::new(format!("<dim>{}</dim>", branch)));
                    }
                    remotes_list.add(branch_list);
                }
            }
        }

        meta_list.add(remotes_header);
        meta_list.add(remotes_list);
    }

    // --- Config ---
    if git.config.user_name.is_some() {
        let config_header: RenderableTerminalContent = Prose::new("<b>Config:</b>").into();
        let mut config_list = UnorderedList::empty();

        if let Some(ref name) = git.config.user_name {
            let email_part = git
                .config
                .user_email
                .as_ref()
                .map(|e| format!(" ⟨<dim>{}</dim>⟩", e))
                .unwrap_or_default();
            config_list.add(Prose::new(format!(
                "<b>User Info:</b> <blue>{}</blue>{}",
                name, email_part
            )));
        }

        // Crypto subsection (verbose only)
        if verbose > 0 {
            let crypto_header: RenderableTerminalContent = Prose::new("<b>Crypto</b>").into();
            let mut crypto_list = UnorderedList::empty();

            let agent = git
                .config
                .gpg_use_agent
                .map(|v| if v { "true" } else { "false" })
                .unwrap_or("<dim><i>undefined</i></dim>");
            let program = git
                .config
                .gpg_program
                .as_deref()
                .unwrap_or("<dim><i>undefined</i></dim>");
            let helper = git
                .config
                .credential_helper
                .as_deref()
                .unwrap_or("<dim><i>undefined</i></dim>");
            crypto_list.add(Prose::new(format!(
                "<b>GPG:</b> use-agent: <blue>{}</blue>, program: <blue>{}</blue>, helper: <blue>{}</blue>",
                agent, program, helper
            )));

            let key = git
                .config
                .signing_key
                .as_deref()
                .unwrap_or("<dim><i>undefined</i></dim>");
            crypto_list.add(Prose::new(format!("<b>GPG Key:</b> <blue>{}</blue>", key)));

            let commit_sign = git
                .config
                .commit_sign
                .map(|v| if v { "true" } else { "false" })
                .unwrap_or("<dim><i>undefined</i></dim>");
            let tag_sign = git
                .config
                .tag_sign
                .map(|v| if v { "true" } else { "false" })
                .unwrap_or("<dim><i>undefined</i></dim>");
            crypto_list.add(Prose::new(format!(
                "<b>Signing:</b> commit: <blue>{}</blue>, tags: <blue>{}</blue>",
                commit_sign, tag_sign
            )));

            config_list.add(crypto_header);
            config_list.add(crypto_list);
        }

        // Pager subsection (verbose only)
        if verbose > 0 {
            let pager_value = git
                .config
                .pager
                .as_deref()
                .unwrap_or("<dim><i>undefined</i></dim>");
            let pager_line = Prose::new(format!("<b>Pager:</b> <blue>{}</blue>", pager_value));

            if git.config.pager.as_deref() == Some("delta") {
                let theme = git
                    .config
                    .delta_syntax_theme
                    .as_deref()
                    .unwrap_or("<dim><i>undefined</i></dim>");
                let light = git
                    .config
                    .delta_light
                    .map(|v| if v { "true" } else { "false" })
                    .unwrap_or("false");
                let side_by_side = git
                    .config
                    .delta_side_by_side
                    .map(|v| if v { "true" } else { "false" })
                    .unwrap_or("<dim><i>undefined</i></dim>");

                let mut pager_details = UnorderedList::empty();
                pager_details.add(Prose::new(format!("theme: <dim>{}</dim>", theme)));
                pager_details.add(Prose::new(format!("light-mode: <dim>{}</dim>", light)));
                pager_details.add(Prose::new(format!(
                    "side-by-side: <dim>{}</dim>",
                    side_by_side
                )));

                config_list.add(pager_line);
                config_list.add(pager_details);
            } else {
                config_list.add(pager_line);
            }
        }

        meta_list.add(config_header);
        meta_list.add(config_list);
    }

    writeln!(out, "{}", meta_list.render(&terminal)).unwrap();
    out
}

/// Format ahead/behind counts with directional arrows.
///
/// When the terminal uses a Nerd Font, uses  (U+F0737) for ahead and
///  (U+F072E) for behind. Otherwise arrows are omitted.
/// Arrows are always omitted when the corresponding count is 0.
fn format_ahead_behind(ahead: usize, behind: usize, nerd_font: Option<bool>) -> String {
    let is_nerd = nerd_font == Some(true);
    let ahead_arrow = if ahead > 0 && is_nerd {
        "\u{F0737} "
    } else {
        ""
    };
    let behind_arrow = if behind > 0 && is_nerd {
        "\u{F072E} "
    } else {
        ""
    };
    format!(
        "<green>{}{} ahead</green>, <red>{}{} behind</red>",
        ahead_arrow, ahead, behind_arrow, behind
    )
}

/// Render the root directory of the repository.
///
/// Returns empty string if no repository is found.
pub fn render_repo_root(result: &sniff::SniffResult) -> String {
    result
        .filesystem
        .as_ref()
        .and_then(|fs| fs.repo.as_ref())
        .map(|repo| repo.root.display().to_string())
        .unwrap_or_default()
}

/// Pure helper: returns whether the current package area has uncommitted
/// changes, or `None` when the area cannot be resolved.
///
/// Checks all dirty/untracked files against the area prefix, not just
/// package-scoped files. JSON and text/exit-code call sites both consume this.
///
/// Pulls dirty paths from `git.file_changes` (always populated) plus the
/// diff-rich `git.status.dirty` / `git.status.untracked` arrays (deep mode
/// only). Without the `file_changes` source, non-deep callers would always
/// see `false` because `status.dirty`/`status.untracked` are empty unless
/// `--refresh-remotes` is set.
pub(crate) fn current_package_area_is_dirty(
    result: &sniff::SniffResult,
    base_dir: Option<&Path>,
) -> Option<bool> {
    let fs = result.filesystem.as_ref()?;
    let repo = fs.repo.as_ref()?;
    let dir = resolve_dir(base_dir);
    let area = repo.package_area_for_dir(&dir)?;
    let git = fs.git.as_ref()?;

    let area_prefix = if area == "root" { "" } else { area };

    // Without computed status (identity-only request) dirtiness is
    // indeterminate, so return `None` like the other missing-data early exits.
    let status = git.status.as_ref()?;
    let has_dirty = status
        .dirty
        .iter()
        .map(|d| d.filepath.to_str().unwrap_or(""))
        .chain(
            status
                .untracked
                .iter()
                .map(|u| u.filepath.to_str().unwrap_or("")),
        )
        .chain(
            git.file_changes
                .iter()
                .map(|fc| fc.path.to_str().unwrap_or("")),
        )
        .any(|path| {
            if area_prefix.is_empty() {
                // Root area: dirty if file is not inside any non-root area
                !repo.packages.as_ref().is_some_and(|pkgs| {
                    pkgs.iter()
                        .any(|p| p.package_area != "root" && path.starts_with(&p.package_area))
                })
            } else {
                path.starts_with(area_prefix)
            }
        });

    Some(has_dirty)
}

/// Exit 0 if the current package area has uncommitted changes, exit 1 otherwise.
///
/// No text output is produced — only the exit code signals the result.
/// Checks all dirty/untracked files against the area prefix, not just package-scoped files.
pub fn print_current_package_area_dirty(result: &sniff::SniffResult, base_dir: Option<&Path>) {
    match current_package_area_is_dirty(result, base_dir) {
        Some(true) => std::process::exit(0),
        Some(false) | None => std::process::exit(1),
    }
}

/// Returns true if a file path has a source code extension.
///
/// Delegates to the shared library helper.
fn is_source_code_file(path: &str) -> bool {
    sniff::filesystem::path_kind::is_source_code_path(Path::new(path))
}

/// Pure helper: returns `(has_changes, count, area_name)` for the current
/// package area, or `None` when the area cannot be resolved.
///
/// JSON consumers only need the boolean. The text/verbose path uses the count
/// and area name to print a human-readable summary.
///
/// ## Notes
///
/// Pulls dirty paths from `git.file_changes` (always populated) plus the
/// diff-rich `git.status.dirty` / `git.status.untracked` arrays (deep mode
/// only). Without the `file_changes` source, non-deep callers would always
/// see a count of `0` because `status.dirty`/`status.untracked` are empty
/// unless `--refresh-remotes` is set. This mirrors the same chain used by
/// [`current_package_area_is_dirty`].
pub(crate) fn package_area_source_code_change_count(
    result: &sniff::SniffResult,
    base_dir: Option<&Path>,
) -> Option<(bool, usize, String)> {
    let fs = result.filesystem.as_ref()?;
    let repo = fs.repo.as_ref()?;
    let dir = resolve_dir(base_dir);
    let area = repo.package_area_for_dir(&dir)?;
    let git = fs.git.as_ref()?;

    let area_prefix = if area == "root" { "" } else { area };

    // Without computed status (identity-only request) the change count is
    // indeterminate, so return `None` like the other missing-data early exits.
    let status = git.status.as_ref()?;

    let count = status
        .dirty
        .iter()
        .map(|d| d.filepath.to_str().unwrap_or(""))
        .chain(
            status
                .untracked
                .iter()
                .map(|u| u.filepath.to_str().unwrap_or("")),
        )
        .chain(
            git.file_changes
                .iter()
                .map(|fc| fc.path.to_str().unwrap_or("")),
        )
        .filter(|path| {
            let in_area = if area_prefix.is_empty() {
                !repo.packages.as_ref().is_some_and(|pkgs| {
                    pkgs.iter()
                        .any(|p| p.package_area != "root" && path.starts_with(&p.package_area))
                })
            } else {
                path.starts_with(area_prefix)
            };
            in_area && is_source_code_file(path)
        })
        .count();

    Some((count > 0, count, area.to_string()))
}

/// Exit 0 if the current package area has source code file changes, exit 1 otherwise.
///
/// With `verbose >= 1`, prints a human-readable message before exiting.
pub fn print_package_area_has_source_code_changes(
    result: &sniff::SniffResult,
    base_dir: Option<&Path>,
    verbose: u8,
) {
    let Some((has_changes, count, area)) = package_area_source_code_change_count(result, base_dir)
    else {
        std::process::exit(1);
    };

    if verbose > 0 {
        if count > 0 {
            println!(
                "{} source file{} changed in the {} package area",
                count,
                if count == 1 { "" } else { "s" },
                area,
            );
        } else {
            println!("no source files changed in the {} package area", area);
        }
    }

    if has_changes {
        std::process::exit(0);
    }
    std::process::exit(1);
}

/// Resolve the effective directory from `--base` or fall back to CWD.
fn resolve_dir(base_dir: Option<&Path>) -> PathBuf {
    base_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::deps::build_deps_dot;
    use super::repo::build_area_hierarchy;
    use super::*;
    use chrono::Utc;
    use sniff::filesystem::docs::MarkdownMeta;
    use sniff::filesystem::git::{
        CommitInfo, FileAction, FileChange, FileStatus, GitConfig, GitInfo, RepoStatus,
    };
    use sniff::filesystem::repo::Package;
    use sniff::filesystem::{TitleSource, UpdatedSource};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_package(name: &str, area: &str, depends_on: &[&str]) -> Package {
        Package {
            path: PathBuf::from(format!("/repo/{area}/{name}")),
            relative: format!("{area}/{name}"),
            package_area: area.to_string(),
            name: name.to_string(),
            ecosystem: sniff::filesystem::repo::PackageEcosystem::Unknown,
            standard: sniff::filesystem::repo::MonorepoStandard::Unknown,
            provenance: sniff::filesystem::repo::PackageProvenance::ManifestScan,
            nested_packages: vec![],
            primary_language: None,
            secondary_languages: vec![],
            languages: vec![],
            frameworks: vec![],
            file_associations: vec![],
            configuration: vec![],
            documentation: vec![],
            editor_config: None,
            command_runner: vec![],
            package_managers: vec![],
            test_runners: vec![],
            version: None,
            features: vec![],
            depends_on: depends_on.iter().map(|s| s.to_string()).collect(),
            used_by: vec![],
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            is_updatable: None,
            has_major_update: None,
            is_excluded: false,
        }
    }

    fn make_git_info(file_changes: Vec<FileChange>) -> GitInfo {
        let staged_count = file_changes
            .iter()
            .filter(|f| f.status == FileStatus::Staged || f.status == FileStatus::Both)
            .count();
        let unstaged_count = file_changes
            .iter()
            .filter(|f| f.status == FileStatus::Modified || f.status == FileStatus::Both)
            .count();
        let untracked_count = file_changes
            .iter()
            .filter(|f| f.status == FileStatus::Untracked)
            .count();

        GitInfo {
            repo_root: PathBuf::from("/repo"),
            org: None,
            repo: None,
            current_branch: Some("main".to_string()),
            head_id: None,
            branches: vec![],
            in_worktree: false,
            base_repo_root: None,
            recent: vec![CommitInfo {
                sha: "1234567890abcdef".to_string(),
                message: "feat: add status output".to_string(),
                author: "Test User".to_string(),
                timestamp: Utc::now(),
                remotes: None,
                refs: vec![],
            }],
            status: Some(RepoStatus {
                is_dirty: !file_changes.is_empty(),
                staged_count,
                unstaged_count,
                untracked_count,
                dirty: vec![],
                untracked: vec![],
                is_behind: None,
            }),
            remotes: vec![],
            worktrees: HashMap::new(),
            config: GitConfig::default(),
            tracking: vec![],
            file_changes,
            aggregate: None,
        }
    }

    mod git_status_rendering {
        use super::*;

        #[test]
        fn conflicted_files_render_last_and_untracked_is_dimmed() {
            let git = make_git_info(vec![
                FileChange {
                    path: PathBuf::from("src/main.rs"),
                    status: FileStatus::Staged,
                    action: FileAction::Modified,
                    lines_added: 3,
                    lines_removed: 1,
                },
                FileChange {
                    path: PathBuf::from("conflict.txt"),
                    status: FileStatus::Conflicted,
                    action: FileAction::Modified,
                    lines_added: 0,
                    lines_removed: 0,
                },
                FileChange {
                    path: PathBuf::from("notes.md"),
                    status: FileStatus::Untracked,
                    action: FileAction::Created,
                    lines_added: 0,
                    lines_removed: 0,
                },
            ]);

            let items = build_git_status_items(&git, 10, 0);

            let last = items.last().expect("at least one status item");
            assert!(last.starts_with("<red>conflicted: <a href=\"/repo/conflict.txt\">"));
            assert!(last.contains("<b>conflict.txt</b></a>"));

            let staged_index = items
                .iter()
                .position(|item| {
                    item.starts_with("<lime>staged(")
                        && item.contains("<a href=\"/repo/src/main.rs\">src/<b>main.rs</b></a>")
                })
                .expect("staged item present");
            let untracked_index = items
                .iter()
                .position(|item| {
                    item.starts_with("<dim>untracked: <a href=\"/repo/notes.md\">")
                        && item.contains("<b>notes.md</b></a>")
                })
                .expect("untracked item present");
            let conflicted_index = items.len() - 1;

            assert!(staged_index < conflicted_index);
            assert!(untracked_index < conflicted_index);
        }

        #[test]
        fn compact_git_status_omits_meta_section() {
            let git = make_git_info(vec![FileChange {
                path: PathBuf::from("conflict.txt"),
                status: FileStatus::Conflicted,
                action: FileAction::Modified,
                lines_added: 0,
                lines_removed: 0,
            }]);

            let output = render_git_section(&git, 10, 0, true, None, None);

            assert!(output.contains("Status"));
            assert!(!output.contains("\x1b[1m\x1b[4mMeta"));
            assert!(!output.contains("Worktrees"));
        }

        #[test]
        fn git_status_headers_use_double_underline_markup() {
            let git = make_git_info(vec![]);
            let output = render_git_section(&git, 10, 0, false, None, None);
            // The header is produced via Prose with <b><uu>Status</uu></b>.
            // In a TTY with double-underline support this renders as \x1b[4:2m;
            // otherwise it degrades to regular underline or plain text.  We
            // simply verify the header text appears and the section renders
            // without panicking.
            assert!(output.contains("Status"));
        }

        #[test]
        fn git_status_sections_have_single_blank_line_separator() {
            let mut git = make_git_info(vec![]);
            git.worktrees.insert(
                "feature".to_string(),
                sniff::filesystem::git::WorktreeInfo {
                    branch: "feature".to_string(),
                    filepath: PathBuf::from("/repo/feature"),
                    sha: "abc123".to_string(),
                    dirty: false,
                    ahead: 1,
                    behind: 0,
                    base_branch: "main".to_string(),
                    has_conflicts: false,
                    merged: false,
                    changed_files: 0,
                    is_current: true,
                },
            );
            git.base_repo_root = Some(PathBuf::from("/repo"));
            git.in_worktree = true;
            git.current_branch = Some("feature".to_string());

            let output = render_git_section(&git, 10, 0, false, None, None);
            let lines: Vec<&str> = output.lines().collect();

            // Find section headers (they contain the rendered header text)
            let status_idx = lines
                .iter()
                .position(|l| l.contains("Status"))
                .expect("Status header present");
            let worktrees_idx = lines
                .iter()
                .position(|l| l.contains("Worktrees"))
                .expect("Worktrees header present");
            let meta_idx = lines
                .iter()
                .position(|l| l.contains("Meta"))
                .expect("Meta header present");

            // Exactly one blank line before each header
            assert!(
                lines[status_idx - 1].trim().is_empty(),
                "blank line before Status"
            );
            assert!(
                lines[worktrees_idx - 1].trim().is_empty(),
                "blank line before Worktrees"
            );
            assert!(
                lines[meta_idx - 1].trim().is_empty(),
                "blank line before Meta"
            );

            // Exactly one blank line after each header
            assert!(
                lines[status_idx + 1].trim().is_empty(),
                "blank line after Status"
            );
            assert!(
                lines[worktrees_idx + 1].trim().is_empty(),
                "blank line after Worktrees"
            );
            assert!(
                lines[meta_idx + 1].trim().is_empty(),
                "blank line after Meta"
            );

            // EXACTLY one — not two. The line two rows out from each header (the
            // content side) must be non-blank, proving there is no second blank
            // row padding the separation. Status is the first section, so only
            // its trailing side has a preceding content row to check.
            assert!(
                !lines[status_idx + 2].trim().is_empty(),
                "no double blank line after Status: {:?}",
                &lines[status_idx..=status_idx + 2]
            );
            assert!(
                !lines[worktrees_idx - 2].trim().is_empty(),
                "no double blank line before Worktrees: {:?}",
                &lines[worktrees_idx - 2..=worktrees_idx]
            );
            assert!(
                !lines[worktrees_idx + 2].trim().is_empty(),
                "no double blank line after Worktrees: {:?}",
                &lines[worktrees_idx..=worktrees_idx + 2]
            );
            assert!(
                !lines[meta_idx - 2].trim().is_empty(),
                "no double blank line before Meta: {:?}",
                &lines[meta_idx - 2..=meta_idx]
            );
            assert!(
                !lines[meta_idx + 2].trim().is_empty(),
                "no double blank line after Meta: {:?}",
                &lines[meta_idx..=meta_idx + 2]
            );
        }

        #[test]
        fn git_status_case_a_linked_worktree_shows_main_and_current() {
            let mut git = make_git_info(vec![]);
            git.repo_root = PathBuf::from("/repo/feature");
            git.base_repo_root = Some(PathBuf::from("/repo"));
            git.in_worktree = true;
            git.current_branch = Some("feature".to_string());

            git.worktrees.insert(
                "feature".to_string(),
                sniff::filesystem::git::WorktreeInfo {
                    branch: "feature".to_string(),
                    filepath: PathBuf::from("/repo/feature"),
                    sha: "abc123".to_string(),
                    dirty: false,
                    ahead: 2,
                    behind: 1,
                    base_branch: "main".to_string(),
                    has_conflicts: false,
                    merged: false,
                    changed_files: 0,
                    is_current: true,
                },
            );
            git.worktrees.insert(
                "hotfix".to_string(),
                sniff::filesystem::git::WorktreeInfo {
                    branch: "hotfix".to_string(),
                    filepath: PathBuf::from("/repo/hotfix"),
                    sha: "def456".to_string(),
                    dirty: false,
                    ahead: 0,
                    behind: 0,
                    base_branch: "main".to_string(),
                    has_conflicts: false,
                    merged: false,
                    changed_files: 0,
                    is_current: false,
                },
            );

            let output = render_git_section(&git, 10, 0, false, None, None);

            assert!(
                output.contains("main:"),
                "Case A should show main worktree label"
            );
            assert!(
                output.contains("Current Worktree:"),
                "Case A should show current worktree header"
            );
            assert!(
                output.contains("feature"),
                "Case A should mention the current branch"
            );
            assert!(
                output.contains("2 ahead"),
                "Case A should show ahead/behind for current worktree"
            );
            assert!(
                output.contains("Other Worktrees:"),
                "Case A should show other worktrees header"
            );
            assert!(
                output.contains("1 other active worktrees in this repo"),
                "Case A should count other worktrees"
            );
        }

        #[test]
        fn git_status_case_b_main_branch_shows_current_and_others() {
            let mut git = make_git_info(vec![]);
            git.repo_root = PathBuf::from("/repo");
            git.in_worktree = false;
            git.current_branch = Some("main".to_string());

            git.worktrees.insert(
                "feature".to_string(),
                sniff::filesystem::git::WorktreeInfo {
                    branch: "feature".to_string(),
                    filepath: PathBuf::from("/repo/feature"),
                    sha: "abc123".to_string(),
                    dirty: false,
                    ahead: 0,
                    behind: 0,
                    base_branch: "main".to_string(),
                    has_conflicts: false,
                    merged: false,
                    changed_files: 0,
                    is_current: false,
                },
            );

            let output = render_git_section(&git, 10, 0, false, None, None);

            assert!(
                output.contains("Current Worktree:"),
                "Case B should show current worktree header"
            );
            assert!(output.contains("main"), "Case B should mention main branch");
            assert!(
                output.contains("Other Worktrees:"),
                "Case B should show other worktrees header"
            );
            assert!(
                output.contains("1 other active worktree"),
                "Case B should count other worktrees"
            );
            assert!(
                !output.contains("main:"),
                "Case B should NOT show a separate main label"
            );
        }

        #[test]
        fn git_status_no_worktrees_omits_section_and_adds_status_bullets() {
            // A repo with no linked worktrees is the common case. The dedicated
            // Worktrees section is suppressed; the repo location and a "no
            // worktrees" note are appended as trailing Status bullets instead.
            let mut git = make_git_info(vec![]);
            git.worktrees = HashMap::new();
            git.current_branch = Some("main".to_string());
            git.in_worktree = false;

            let output = render_git_section(&git, 10, 0, false, None, None);

            assert!(
                !output.contains("Worktrees"),
                "Worktrees section must be omitted with no linked worktrees: {output}"
            );
            assert!(
                !output.contains("Current Worktree:"),
                "no Current Worktree heading when the section is omitted: {output}"
            );
            assert!(
                output.contains("this repo is located at"),
                "repo location bullet is present: {output}"
            );
            // "worktrees" carries `<i>…</i>` markup, so assert the surrounding
            // plain fragments rather than the styled word.
            assert!(
                output.contains("are defined"),
                "'no worktrees are defined' bullet is present: {output}"
            );
            assert!(
                output.contains("Status"),
                "Status section should still render"
            );
            assert!(output.contains("Meta"), "Meta section should still render");
        }

        #[test]
        fn git_status_case_a_uses_directory_name_and_relative_path() {
            // The current worktree's display name is its directory basename, and
            // the path label is relative to the current worktree directory. A
            // branch named differently from the directory must not be substituted
            // for the name.
            let wt_dir = "/tmp/demo/login-fix";
            let main_dir = "/tmp/demo/project";

            let mut git = make_git_info(vec![]);
            git.repo_root = PathBuf::from(wt_dir);
            git.base_repo_root = Some(PathBuf::from(main_dir));
            git.in_worktree = true;
            git.current_branch = Some("feature/login".to_string());
            git.worktrees.insert(
                "feature/login".to_string(),
                sniff::filesystem::git::WorktreeInfo {
                    branch: "feature/login".to_string(),
                    filepath: PathBuf::from(wt_dir),
                    sha: "abc123".to_string(),
                    dirty: false,
                    ahead: 3,
                    behind: 0,
                    base_branch: "main".to_string(),
                    has_conflicts: false,
                    merged: false,
                    changed_files: 0,
                    is_current: true,
                },
            );

            let output = render_git_section(&git, 10, 0, false, None, None);

            assert!(
                output.contains("login-fix"),
                "current worktree must be named by its directory: {output}"
            );
            assert!(
                output.contains("feature/login"),
                "current branch must still be shown: {output}"
            );
            // Main worktree is a sibling directory, so its visible label is
            // `../project`. The current worktree shows its own absolute path —
            // a label relative to itself (`.`) would tell the reader nothing.
            assert!(
                output.contains("[../project](file://"),
                "main worktree label must be relative to the current directory: {output}"
            );
            // The visible absolute label may word-wrap at the terminal width,
            // so assert on the OSC8 href, which is emitted intact.
            assert!(
                output.contains("(file:///tmp/demo/login-fix)"),
                "current worktree must link to its absolute path: {output}"
            );
        }

        #[test]
        fn relative_path_between_labels_worktree_paths() {
            assert_eq!(
                relative_path_between(
                    &PathBuf::from("/tmp/demo/login-fix"),
                    &PathBuf::from("/tmp/demo/project")
                ),
                "../project"
            );
            assert_eq!(
                relative_path_between(
                    &PathBuf::from("/tmp/demo/login-fix"),
                    &PathBuf::from("/tmp/demo/login-fix")
                ),
                "."
            );
            assert_eq!(
                relative_path_between(&PathBuf::from("/repo/feature"), &PathBuf::from("/repo")),
                ".."
            );
            assert_eq!(
                relative_path_between(&PathBuf::from("/repo"), &PathBuf::from("/repo/feature")),
                "feature"
            );
            // Paths that share only the filesystem root fall back to the
            // absolute target — a `../` chain back to root reads worse than the
            // plain absolute path (e.g. a `/Users/...` worktree next to a
            // `/Volumes/...` checkout).
            assert_eq!(
                relative_path_between(
                    &PathBuf::from("/repo/feature"),
                    &PathBuf::from("/other/project")
                ),
                "/other/project"
            );
        }

        /// Builds an owned env-var list for `alias_path_with` from string pairs.
        fn env(pairs: &[(&str, &str)]) -> Vec<(std::ffi::OsString, std::ffi::OsString)> {
            pairs
                .iter()
                .map(|(k, v)| (std::ffi::OsString::from(k), std::ffi::OsString::from(v)))
                .collect()
        }

        #[test]
        fn alias_path_offsets_against_env_var() {
            let path = PathBuf::from("/Users/ken/.claudine/worktrees/rusty-biscuit/sniff");
            let vars = env(&[("CLAUDINE_WT", "/Users/ken/.claudine/worktrees")]);
            assert_eq!(
                alias_path_with(&path, &vars, Some(Path::new("/Users/ken"))),
                "${CLAUDINE_WT}/rusty-biscuit/sniff"
            );
        }

        #[test]
        fn alias_path_falls_back_to_home_tilde() {
            let path = PathBuf::from("/Users/ken/.claudine/worktrees/rusty-biscuit/sniff");
            assert_eq!(
                alias_path_with(&path, &[], Some(Path::new("/Users/ken"))),
                "~/.claudine/worktrees/rusty-biscuit/sniff"
            );
        }

        #[test]
        fn alias_path_prefers_tilde_over_shorter_env_offset() {
            // An env var that is a *shorter* prefix than $HOME must yield to the
            // `~` form, which reads better for paths inside the home directory.
            let path = PathBuf::from("/Users/ken/project/src");
            let vars = env(&[("ROOT", "/Users")]);
            assert_eq!(
                alias_path_with(&path, &vars, Some(Path::new("/Users/ken"))),
                "~/project/src"
            );
        }

        #[test]
        fn alias_path_picks_longest_prefix_then_name_for_ties() {
            let path = PathBuf::from("/a/b/c/d");
            // `DEEP` is the longer prefix and wins over `SHALLOW`.
            let vars = env(&[("SHALLOW", "/a"), ("DEEP", "/a/b/c")]);
            assert_eq!(alias_path_with(&path, &vars, None), "${DEEP}/d");

            // Equal-length prefixes tie-break on the lexicographically-first name.
            let vars = env(&[("ZED", "/a/b"), ("ACE", "/a/b")]);
            assert_eq!(alias_path_with(&path, &vars, None), "${ACE}/c/d");
        }

        #[test]
        fn alias_path_collapses_exact_match_to_bare_prefix() {
            let path = PathBuf::from("/srv/data");
            let vars = env(&[("DATA", "/srv/data")]);
            assert_eq!(alias_path_with(&path, &vars, None), "${DATA}");

            assert_eq!(
                alias_path_with(Path::new("/Users/ken"), &[], Some(Path::new("/Users/ken"))),
                "~"
            );
        }

        #[test]
        fn alias_path_skips_positional_shell_vars() {
            // PWD/OLDPWD often equal (an exact prefix of) the target, but alias
            // to a transient label — they must be ignored in favor of `~`.
            let path = PathBuf::from("/Users/ken/.claudine/worktrees/rusty-biscuit/sniff");
            let vars = env(&[
                (
                    "OLDPWD",
                    "/Users/ken/.claudine/worktrees/rusty-biscuit/sniff",
                ),
                ("PWD", "/Users/ken/.claudine/worktrees/rusty-biscuit/sniff"),
            ]);
            assert_eq!(
                alias_path_with(&path, &vars, Some(Path::new("/Users/ken"))),
                "~/.claudine/worktrees/rusty-biscuit/sniff"
            );
        }

        #[test]
        fn alias_path_ignores_non_absolute_and_falls_back_to_absolute() {
            let path = PathBuf::from("/opt/tool/bin");
            // Relative and empty env values must never match.
            let vars = env(&[("REL", "opt/tool"), ("EMPTY", "")]);
            assert_eq!(
                alias_path_with(&path, &vars, Some(Path::new("/home/other"))),
                "/opt/tool/bin"
            );
        }

        // The Windows-shaped tests below simulate drive letters with a leading
        // `/c/` or `/d/` component so the prefix logic is exercised identically
        // on Unix and Windows hosts (`std::path` separators are host-specific,
        // but the component-prefix comparisons are not). Case sensitivity is
        // passed explicitly via `alias_path_with_case` so the Windows branch is
        // testable on this macOS host.

        #[test]
        fn alias_path_windows_drive_letter_path_uses_home_tilde() {
            let path = PathBuf::from("/c/Users/ken/project");
            assert_eq!(
                alias_path_with_case(&path, &[], Some(Path::new("/c/Users/ken")), true),
                "~/project"
            );
        }

        #[test]
        fn alias_path_windows_skips_userprofile_alias() {
            // USERPROFILE mirrors the home dir; even with home unknown it must
            // not surface as `${USERPROFILE}`, falling back to the absolute path.
            let path = PathBuf::from("/c/Users/ken/project");
            let vars = env(&[("USERPROFILE", "/c/Users/ken")]);
            assert_eq!(
                alias_path_with_case(&path, &vars, None, true),
                "/c/Users/ken/project"
            );
        }

        #[test]
        fn alias_path_windows_env_var_names_are_case_insensitive() {
            // On Windows `oldpwd` and `OLDPWD` name the same variable, so the
            // lowercase spelling must be skipped too, yielding `~` rather than
            // `${oldpwd}`. The same input is case-sensitive on Unix.
            let path = PathBuf::from("/c/Users/ken/wt/proj");
            let vars = env(&[("oldpwd", "/c/Users/ken/wt/proj")]);
            assert_eq!(
                alias_path_with_case(&path, &vars, Some(Path::new("/c/Users/ken")), true),
                "~/wt/proj"
            );
            // Case-sensitive (Unix) host: `oldpwd` is a distinct, longer prefix
            // than home, so it offsets as `${oldpwd}`.
            assert_eq!(
                alias_path_with_case(&path, &vars, Some(Path::new("/c/Users/ken")), false),
                "${oldpwd}"
            );
        }

        #[test]
        fn alias_path_windows_does_not_alias_across_drives() {
            // Home on the D: drive must not alias a path on the C: drive.
            let path = PathBuf::from("/c/projects/app");
            let vars = env(&[("PROJ", "/d/projects")]);
            assert_eq!(
                alias_path_with_case(&path, &vars, Some(Path::new("/d/Users/ken")), true),
                "/c/projects/app"
            );
        }

        #[test]
        fn git_status_main_worktree_on_non_main_branch_is_case_b() {
            // Regression: a non-main branch checked out in the MAIN worktree
            // (in_worktree == false) must render Case B, not vanish and not be
            // misclassified as Case A. Selecting by branch spelling produced
            // empty output here. A linked worktree is present so the Worktrees
            // section renders (the empty case is covered separately).
            let mut git = make_git_info(vec![]);
            git.repo_root = PathBuf::from("/repo");
            git.in_worktree = false;
            git.current_branch = Some("feature-x".to_string());
            git.worktrees.insert(
                "linked".to_string(),
                sniff::filesystem::git::WorktreeInfo {
                    branch: "linked".to_string(),
                    filepath: PathBuf::from("/repo-linked"),
                    sha: "abc123".to_string(),
                    dirty: false,
                    ahead: 0,
                    behind: 0,
                    base_branch: "main".to_string(),
                    has_conflicts: false,
                    merged: false,
                    changed_files: 0,
                    is_current: false,
                },
            );

            let output = render_git_section(&git, 10, 0, false, None, None);

            assert!(
                output.contains("Current Worktree:"),
                "main worktree on a non-main branch must still show Case B: {output}"
            );
            assert!(
                output.contains("there are 1 other active worktrees in this repo"),
                "must report the other-worktree count: {output}"
            );
            assert!(
                !output.contains("main:"),
                "Case B must not show a separate main location line: {output}"
            );
        }

        #[test]
        fn git_status_master_default_main_worktree_is_case_b() {
            // A repository whose primary branch is `master` is still the main
            // worktree (in_worktree == false) and must render Case B. A linked
            // worktree is present so the Worktrees section renders.
            let mut git = make_git_info(vec![]);
            git.repo_root = PathBuf::from("/repo");
            git.in_worktree = false;
            git.current_branch = Some("master".to_string());
            git.worktrees.insert(
                "linked".to_string(),
                sniff::filesystem::git::WorktreeInfo {
                    branch: "linked".to_string(),
                    filepath: PathBuf::from("/repo-linked"),
                    sha: "abc123".to_string(),
                    dirty: false,
                    ahead: 0,
                    behind: 0,
                    base_branch: "master".to_string(),
                    has_conflicts: false,
                    merged: false,
                    changed_files: 0,
                    is_current: false,
                },
            );

            let output = render_git_section(&git, 10, 0, false, None, None);

            assert!(
                output.contains("Current Worktree:") && !output.contains("main:"),
                "master-default repo must render Case B: {output}"
            );
        }

        #[test]
        fn git_status_detached_head_in_linked_worktree_renders_case_a() {
            // Detached HEAD (current_branch == None) inside a linked worktree
            // must still render Case A with a sensible branch label.
            let mut git = make_git_info(vec![]);
            git.repo_root = PathBuf::from("/repo/wt");
            git.base_repo_root = Some(PathBuf::from("/repo"));
            git.in_worktree = true;
            git.current_branch = None;
            git.worktrees.insert(
                "wt".to_string(),
                sniff::filesystem::git::WorktreeInfo {
                    branch: "wt".to_string(),
                    filepath: PathBuf::from("/repo/wt"),
                    sha: "abc123".to_string(),
                    dirty: false,
                    ahead: 0,
                    behind: 0,
                    base_branch: "main".to_string(),
                    has_conflicts: false,
                    merged: false,
                    changed_files: 0,
                    is_current: true,
                },
            );

            let output = render_git_section(&git, 10, 0, false, None, None);

            assert!(
                output.contains("main:"),
                "Case A shows main location: {output}"
            );
            assert!(
                output.contains("detached HEAD"),
                "detached HEAD must be labeled rather than blank: {output}"
            );
        }

        #[test]
        fn git_status_single_linked_worktree_counts_correctly() {
            let mut git = make_git_info(vec![]);
            git.repo_root = PathBuf::from("/repo");
            git.in_worktree = false;
            git.current_branch = Some("main".to_string());

            git.worktrees.insert(
                "feature".to_string(),
                sniff::filesystem::git::WorktreeInfo {
                    branch: "feature".to_string(),
                    filepath: PathBuf::from("/repo/feature"),
                    sha: "abc123".to_string(),
                    dirty: false,
                    ahead: 0,
                    behind: 0,
                    base_branch: "main".to_string(),
                    has_conflicts: false,
                    merged: false,
                    changed_files: 0,
                    is_current: false,
                },
            );

            let output = render_git_section(&git, 10, 0, false, None, None);

            assert!(
                output.contains("Current Worktree:"),
                "Case B should show current worktree header"
            );
            assert!(
                output.contains("Other Worktrees:"),
                "Case B should show other worktrees header"
            );
            assert!(
                output.contains("1 other active worktrees in this repo"),
                "Single worktree should be counted correctly"
            );
        }

        #[test]
        fn merge_commit_line_includes_time_segment() {
            // Merge subjects do not parse as conventional commits, so they take
            // the non-conventional branch of `format_commit_line`. That branch
            // must still render the `at <time>` segment.
            let commit = CommitInfo {
                sha: "3e2ca7f1234567".to_string(),
                message: "Merge branch 'claudine'".to_string(),
                author: "Test User".to_string(),
                timestamp: Utc::now(),
                remotes: None,
                refs: vec![],
            };

            let line = format_commit_line(&commit, 0, None);

            // Derive the expected time string the same way the formatter does to
            // avoid timezone/relative-day flakiness across machines.
            let (_, time_str, _) = format_commit_datetime(&commit.timestamp);

            assert!(line.contains("<i>at</i>"), "missing `at` segment: {line}");
            assert!(line.contains(&time_str), "missing time string: {line}");
            assert!(
                line.contains("Merge branch 'claudine'"),
                "missing merge subject: {line}"
            );
        }
    }

    mod repo_filter_parse {
        use super::*;

        #[test]
        fn simple_name() {
            let f = RepoFilter::parse("biscuit");
            assert_eq!(f.query, "biscuit");
            assert!(!f.by_area);
            assert!(!f.negate);
        }

        #[test]
        fn negated() {
            let f = RepoFilter::parse("!biscuit");
            assert_eq!(f.query, "biscuit");
            assert!(!f.by_area);
            assert!(f.negate);
        }

        #[test]
        fn area() {
            let f = RepoFilter::parse("@sniff");
            assert_eq!(f.query, "sniff");
            assert!(f.by_area);
            assert!(!f.negate);
        }

        #[test]
        fn negated_area() {
            let f = RepoFilter::parse("!@sniff");
            assert_eq!(f.query, "sniff");
            assert!(f.by_area);
            assert!(f.negate);
        }

        #[test]
        fn area_negated() {
            let f = RepoFilter::parse("@!sniff");
            assert_eq!(f.query, "sniff");
            assert!(f.by_area);
            assert!(f.negate);
        }
    }

    mod area_hierarchy {
        use super::*;

        #[test]
        fn nests_child_areas_under_present_parents() {
            let areas = vec![
                "homelab".to_string(),
                "homelab/server".to_string(),
                "sniff".to_string(),
            ];

            let (top, children) = build_area_hierarchy(&areas);

            assert_eq!(top, vec!["homelab".to_string(), "sniff".to_string()]);
            assert_eq!(
                children.get("homelab"),
                Some(&vec!["homelab/server".to_string()])
            );
        }

        #[test]
        fn keeps_area_top_level_when_parent_is_missing() {
            let areas = vec!["apps/browser".to_string()];

            let (top, children) = build_area_hierarchy(&areas);

            assert_eq!(top, vec!["apps/browser".to_string()]);
            assert!(children.is_empty());
        }
    }

    mod package_area_root {
        use super::*;

        #[test]
        fn root_sentinel_returns_repo_root() {
            let mut pkg = make_package("model_id", "root", &[]);
            pkg.relative = "model_id".to_string();
            pkg.path = PathBuf::from("/repo/model_id");

            assert_eq!(super::super::package_areas::package_area_root(&pkg), ".");
        }

        #[test]
        fn normal_area_returns_package_area() {
            let pkg = make_package("cli", "sniff", &[]);

            assert_eq!(
                super::super::package_areas::package_area_root(&pkg),
                "sniff"
            );
        }

        #[test]
        fn multi_segment_area_returns_full_package_area() {
            let pkg = make_package("my_package", "apps/browser", &[]);

            assert_eq!(
                super::super::package_areas::package_area_root(&pkg),
                "apps/browser"
            );
        }

        #[test]
        fn mismatched_area_falls_back_to_relative_first_component() {
            let mut pkg = make_package("pkg", "weird", &[]);
            pkg.relative = "actual/path".to_string();
            pkg.path = PathBuf::from("/repo/actual/path");

            assert_eq!(
                super::super::package_areas::package_area_root(&pkg),
                "actual"
            );
        }
    }

    mod repo_filter_matches {
        use super::*;

        #[test]
        fn name_substring_match() {
            let pkg = make_package("biscuit-hash", "biscuit-hash", &[]);
            let f = RepoFilter::parse("biscuit");
            assert!(f.matches(&pkg));
        }

        #[test]
        fn name_substring_no_match() {
            let pkg = make_package("sniff-cli", "sniff", &[]);
            let f = RepoFilter::parse("biscuit");
            assert!(!f.matches(&pkg));
        }

        #[test]
        fn case_insensitive() {
            let pkg = make_package("Biscuit-Hash", "biscuit-hash", &[]);
            let f = RepoFilter::parse("biscuit");
            assert!(f.matches(&pkg));
        }

        #[test]
        fn area_match() {
            let pkg = make_package("sniff-cli", "sniff", &[]);
            let f = RepoFilter::parse("@sniff");
            assert!(f.matches(&pkg));
        }

        #[test]
        fn area_no_match() {
            let pkg = make_package("sniff-cli", "sniff", &[]);
            let f = RepoFilter::parse("@biscuit");
            assert!(!f.matches(&pkg));
        }

        #[test]
        fn negated_excludes() {
            let pkg = make_package("biscuit-hash", "biscuit-hash", &[]);
            let f = RepoFilter::parse("!biscuit");
            assert!(!f.matches(&pkg));
        }

        #[test]
        fn negated_includes_non_matching() {
            let pkg = make_package("sniff-cli", "sniff", &[]);
            let f = RepoFilter::parse("!biscuit");
            assert!(f.matches(&pkg));
        }
    }

    mod filter_packages_tests {
        use super::*;

        #[test]
        fn no_filter_returns_all() {
            let packages = vec![
                make_package("alpha", "area-a", &[]),
                make_package("beta", "area-b", &[]),
            ];
            let result = filter_packages(&packages, &[]);
            assert_eq!(result.len(), 2);
        }

        #[test]
        fn name_filter() {
            let packages = vec![
                make_package("biscuit-hash", "biscuit-hash", &[]),
                make_package("sniff-cli", "sniff", &[]),
                make_package("biscuit-file", "biscuit-file", &[]),
            ];
            let result = filter_packages(&packages, &["biscuit".to_string()]);
            assert_eq!(result.len(), 2);
            assert!(result.iter().all(|p| p.name.contains("biscuit")));
        }

        #[test]
        fn area_filter() {
            let packages = vec![
                make_package("sniff-cli", "sniff", &[]),
                make_package("sniff-lib", "sniff", &[]),
                make_package("biscuit-hash", "biscuit-hash", &[]),
            ];
            let result = filter_packages(&packages, &["@sniff".to_string()]);
            assert_eq!(result.len(), 2);
            assert!(result.iter().all(|p| p.package_area == "sniff"));
        }

        #[test]
        fn multiple_filters_with_or_logic() {
            let packages = vec![
                make_package("biscuit-hash", "biscuit-hash", &[]),
                make_package("sniff-cli", "sniff", &[]),
                make_package("darkmatter-lib", "darkmatter", &[]),
                make_package("playa-cli", "playa", &[]),
            ];
            let result = filter_packages(
                &packages,
                &["biscuit".to_string(), "darkmatter".to_string()],
            );
            assert_eq!(result.len(), 2);
            let names: Vec<&str> = result.iter().map(|p| p.name.as_str()).collect();
            assert!(names.contains(&"biscuit-hash"));
            assert!(names.contains(&"darkmatter-lib"));
        }
    }

    mod select_package_family {
        use super::*;
        use sniff::SniffResult;
        use sniff::filesystem::FilesystemInfo;
        use sniff::filesystem::git::types::DirtyFile;
        use sniff::filesystem::repo::types::RepoInfo;

        fn make_dirty_file(path: &str) -> DirtyFile {
            DirtyFile {
                filepath: PathBuf::from(path),
                absolute_filepath: PathBuf::from(format!("/repo/{path}")),
                diff: String::new(),
                last_local_commit: String::new(),
                origin_commit: None,
            }
        }

        fn make_repo(packages: Vec<Package>, is_monorepo: bool) -> RepoInfo {
            RepoInfo {
                is_monorepo,
                root: PathBuf::from("/repo"),
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                monorepo_standards: Vec::new(),
                monorepo_layers: Vec::new(),
                packages: if packages.is_empty() {
                    None
                } else {
                    Some(packages)
                },
            }
        }

        fn build_result(repo: RepoInfo, mut git: GitInfo) -> SniffResult {
            git.repo_root = repo.root.clone();
            let filesystem = FilesystemInfo {
                repo: Some(repo),
                git: Some(git),
                ..Default::default()
            };
            SniffResult {
                os: None,
                hardware: None,
                network: None,
                filesystem: Some(filesystem),
                performance: None,
            }
        }

        #[test]
        fn dirty_packages_returns_empty_when_not_monorepo() {
            let packages = vec![make_package("alpha", "alpha", &[])];
            let repo = make_repo(packages, false);
            let mut git = make_git_info(vec![]);
            git.status.as_mut().unwrap().dirty = vec![make_dirty_file("alpha/src/main.rs")];
            let result = build_result(repo, git);

            let names = select_dirty_package_names(&result, &[], None, None);
            assert!(
                names.is_empty(),
                "expected empty for non-monorepo, got {names:?}"
            );
        }

        #[test]
        fn dirty_packages_picks_up_modified_files() {
            let packages = vec![
                make_package("alpha", "area-a", &[]),
                make_package("beta", "area-b", &[]),
            ];
            // Adjust relative paths so prefix matching works
            let mut packages = packages;
            packages[0].relative = "area-a/alpha".to_string();
            packages[1].relative = "area-b/beta".to_string();

            let repo = make_repo(packages, true);
            let mut git = make_git_info(vec![]);
            git.status.as_mut().unwrap().dirty = vec![make_dirty_file("area-a/alpha/src/main.rs")];
            let result = build_result(repo, git);

            let names = select_dirty_package_names(&result, &[], None, None);
            assert_eq!(names, vec!["alpha".to_string()]);
        }

        #[test]
        fn dirty_package_areas_picks_up_modified_files() {
            let mut packages = vec![
                make_package("alpha", "area-a", &[]),
                make_package("beta", "area-b", &[]),
            ];
            packages[0].relative = "area-a/alpha".to_string();
            packages[1].relative = "area-b/beta".to_string();

            let repo = make_repo(packages, true);
            let mut git = make_git_info(vec![]);
            git.status.as_mut().unwrap().dirty = vec![make_dirty_file("area-a/alpha/src/main.rs")];
            let result = build_result(repo, git);

            let areas = select_dirty_package_area_names(&result, &[], None, None);
            assert_eq!(areas, vec!["area-a".to_string()]);
        }

        #[test]
        fn dirty_packages_honors_filter() {
            let mut packages = vec![
                make_package("alpha", "area-a", &[]),
                make_package("beta", "area-b", &[]),
            ];
            packages[0].relative = "area-a/alpha".to_string();
            packages[1].relative = "area-b/beta".to_string();

            let repo = make_repo(packages, true);
            let mut git = make_git_info(vec![]);
            git.status.as_mut().unwrap().dirty = vec![
                make_dirty_file("area-a/alpha/src/main.rs"),
                make_dirty_file("area-b/beta/src/lib.rs"),
            ];
            let result = build_result(repo, git);

            let names = select_dirty_package_names(&result, &["@area-a".to_string()], None, None);
            assert_eq!(names, vec!["alpha".to_string()]);
        }

        #[test]
        fn staged_packages_picks_up_staged_changes() {
            let mut packages = vec![make_package("alpha", "area-a", &[])];
            packages[0].relative = "area-a/alpha".to_string();

            let repo = make_repo(packages, true);
            let git = make_git_info(vec![FileChange {
                path: PathBuf::from("area-a/alpha/src/main.rs"),
                status: FileStatus::Staged,
                action: FileAction::Modified,
                lines_added: 1,
                lines_removed: 0,
            }]);
            let result = build_result(repo, git);

            let names = select_staged_package_names(&result, &[], None, None);
            assert_eq!(names, vec!["alpha".to_string()]);

            let areas = select_staged_package_area_names(&result, &[], None, None);
            assert_eq!(areas, vec!["area-a".to_string()]);
        }

        #[test]
        fn unstaged_packages_picks_up_modified_changes() {
            let mut packages = vec![make_package("alpha", "area-a", &[])];
            packages[0].relative = "area-a/alpha".to_string();

            let repo = make_repo(packages, true);
            let git = make_git_info(vec![FileChange {
                path: PathBuf::from("area-a/alpha/src/main.rs"),
                status: FileStatus::Modified,
                action: FileAction::Modified,
                lines_added: 1,
                lines_removed: 0,
            }]);
            let result = build_result(repo, git);

            let names = select_unstaged_package_names(&result, &[], None, None);
            assert_eq!(names, vec!["alpha".to_string()]);

            let areas = select_unstaged_package_area_names(&result, &[], None, None);
            assert_eq!(areas, vec!["area-a".to_string()]);
        }

        #[test]
        fn staged_and_unstaged_return_empty_when_not_monorepo() {
            let mut packages = vec![make_package("alpha", "area-a", &[])];
            packages[0].relative = "area-a/alpha".to_string();
            let repo = make_repo(packages, false);
            let git = make_git_info(vec![FileChange {
                path: PathBuf::from("area-a/alpha/src/main.rs"),
                status: FileStatus::Both,
                action: FileAction::Modified,
                lines_added: 1,
                lines_removed: 0,
            }]);
            let result = build_result(repo, git);

            assert!(select_staged_package_names(&result, &[], None, None).is_empty());
            assert!(select_unstaged_package_names(&result, &[], None, None).is_empty());
            assert!(select_staged_package_area_names(&result, &[], None, None).is_empty());
            assert!(select_unstaged_package_area_names(&result, &[], None, None).is_empty());
        }
    }

    mod boolean_helpers {
        //! Phase 4 — pure helpers that back `is-current-package-area-dirty`
        //! and `package-area-has-source-code-changes`.
        //!
        //! The text/exit-code call sites map `None`/`Some(false)` to
        //! `exit(1)` and `Some(true)` to `exit(0)`. These tests pin the
        //! pure data path so the JSON arms can rely on it without
        //! re-implementing scope detection.

        use super::*;
        use sniff::SniffResult;
        use sniff::filesystem::FilesystemInfo;
        use sniff::filesystem::git::types::DirtyFile;
        use sniff::filesystem::repo::types::RepoInfo;

        fn make_repo(packages: Vec<Package>) -> RepoInfo {
            RepoInfo {
                is_monorepo: true,
                root: PathBuf::from("/repo"),
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
                monorepo_standards: Vec::new(),
                monorepo_layers: Vec::new(),
                packages: if packages.is_empty() {
                    None
                } else {
                    Some(packages)
                },
            }
        }

        fn dirty_file(path: &str) -> DirtyFile {
            DirtyFile {
                filepath: PathBuf::from(path),
                absolute_filepath: PathBuf::from(format!("/repo/{path}")),
                diff: String::new(),
                last_local_commit: String::new(),
                origin_commit: None,
            }
        }

        fn build_result(repo: RepoInfo, dirty_paths: &[&str]) -> SniffResult {
            let mut git = make_git_info(vec![]);
            git.repo_root = repo.root.clone();
            git.status.as_mut().unwrap().dirty =
                dirty_paths.iter().map(|p| dirty_file(p)).collect();
            let filesystem = FilesystemInfo {
                repo: Some(repo),
                git: Some(git),
                ..Default::default()
            };
            SniffResult {
                os: None,
                hardware: None,
                network: None,
                filesystem: Some(filesystem),
                performance: None,
            }
        }

        /// Build a `SniffResult` whose `GitInfo` is identity-only: `status`
        /// is `None` (as produced by `GitRequest::identity()`), mirroring a
        /// valid library state the CLI helpers must tolerate without panicking.
        fn build_identity_only_result(repo: RepoInfo) -> SniffResult {
            let mut git = make_git_info(vec![]);
            git.repo_root = repo.root.clone();
            git.status = None;
            git.head_id = Some("1234567890abcdef".to_string());
            let filesystem = FilesystemInfo {
                repo: Some(repo),
                git: Some(git),
                ..Default::default()
            };
            SniffResult {
                os: None,
                hardware: None,
                network: None,
                filesystem: Some(filesystem),
                performance: None,
            }
        }

        #[test]
        fn identity_only_git_info_yields_indeterminate_not_panic() {
            let mut packages = vec![make_package("alpha", "area-a", &[])];
            packages[0].relative = "area-a/alpha".to_string();
            let repo = make_repo(packages);
            let result = build_identity_only_result(repo);
            let area_dir = PathBuf::from("/repo/area-a/alpha");

            // Selection helpers report "indeterminate" (None / empty) rather
            // than silently claiming clean — and never panic on absent status.
            assert_eq!(
                current_package_area_is_dirty(&result, Some(&area_dir)),
                None
            );
            assert_eq!(
                package_area_source_code_change_count(&result, Some(&area_dir)),
                None
            );

            let fs = result.filesystem.as_ref().unwrap();
            let git = fs.git.as_ref().unwrap();
            assert!(super::packages::dirty_package_names(&result).is_empty());

            // Status-oriented renderers must produce output without panicking.
            let _ = render_git_section(git, 10, 1, false, None, None);
            let _ = super::repo::render_filesystem_section(fs, 1, Some(&git.repo_root), false);
        }

        #[test]
        fn current_package_area_is_dirty_some_true_when_dirty_in_area() {
            let mut packages = vec![make_package("alpha", "area-a", &[])];
            packages[0].relative = "area-a/alpha".to_string();
            let repo = make_repo(packages);
            let result = build_result(repo, &["area-a/alpha/src/main.rs"]);

            let answer =
                current_package_area_is_dirty(&result, Some(&PathBuf::from("/repo/area-a/alpha")));
            assert_eq!(answer, Some(true));
        }

        #[test]
        fn current_package_area_is_dirty_some_false_when_clean() {
            let mut packages = vec![make_package("alpha", "area-a", &[])];
            packages[0].relative = "area-a/alpha".to_string();
            let repo = make_repo(packages);
            let result = build_result(repo, &[]);

            let answer =
                current_package_area_is_dirty(&result, Some(&PathBuf::from("/repo/area-a/alpha")));
            assert_eq!(answer, Some(false));
        }

        #[test]
        fn current_package_area_is_dirty_none_when_outside_repo() {
            let mut packages = vec![make_package("alpha", "area-a", &[])];
            packages[0].relative = "area-a/alpha".to_string();
            let repo = make_repo(packages);
            let result = build_result(repo, &["area-a/alpha/src/main.rs"]);

            let answer =
                current_package_area_is_dirty(&result, Some(&PathBuf::from("/somewhere-else")));
            assert_eq!(answer, None);
        }

        #[test]
        fn package_area_source_code_change_count_counts_source_files_only() {
            let mut packages = vec![make_package("alpha", "area-a", &[])];
            packages[0].relative = "area-a/alpha".to_string();
            let repo = make_repo(packages);
            let result = build_result(
                repo,
                &[
                    "area-a/alpha/src/main.rs",
                    "area-a/alpha/README.md",
                    "area-a/alpha/src/lib.rs",
                ],
            );

            let answer = package_area_source_code_change_count(
                &result,
                Some(&PathBuf::from("/repo/area-a/alpha")),
            );
            let (has, count, area) = answer.expect("area should resolve");
            assert!(has);
            assert_eq!(count, 2);
            assert_eq!(area, "area-a");
        }

        #[test]
        fn package_area_source_code_change_count_false_when_only_docs_dirty() {
            let mut packages = vec![make_package("alpha", "area-a", &[])];
            packages[0].relative = "area-a/alpha".to_string();
            let repo = make_repo(packages);
            let result = build_result(repo, &["area-a/alpha/README.md"]);

            let (has, count, _area) = package_area_source_code_change_count(
                &result,
                Some(&PathBuf::from("/repo/area-a/alpha")),
            )
            .expect("area should resolve");
            assert!(!has);
            assert_eq!(count, 0);
        }

        #[test]
        fn package_area_source_code_change_count_none_when_outside_repo() {
            let mut packages = vec![make_package("alpha", "area-a", &[])];
            packages[0].relative = "area-a/alpha".to_string();
            let repo = make_repo(packages);
            let result = build_result(repo, &["area-a/alpha/src/main.rs"]);

            let answer = package_area_source_code_change_count(
                &result,
                Some(&PathBuf::from("/somewhere-else")),
            );
            assert!(answer.is_none());
        }

        /// Build a `SniffResult` whose dirty paths arrive solely via
        /// `git.file_changes` — i.e. the normal (non-deep) CLI path where
        /// `git.status.dirty` and `git.status.untracked` are empty.
        fn build_result_with_file_changes(
            repo: RepoInfo,
            file_changes: Vec<FileChange>,
        ) -> SniffResult {
            let mut git = make_git_info(file_changes);
            git.repo_root = repo.root.clone();
            // Force status.dirty / status.untracked empty so the test
            // exclusively exercises the file_changes branch.
            git.status.as_mut().unwrap().dirty = Vec::new();
            git.status.as_mut().unwrap().untracked = Vec::new();
            let filesystem = FilesystemInfo {
                repo: Some(repo),
                git: Some(git),
                ..Default::default()
            };
            SniffResult {
                os: None,
                hardware: None,
                network: None,
                filesystem: Some(filesystem),
                performance: None,
            }
        }

        #[test]
        fn package_area_source_code_change_count_counts_file_changes_source_files() {
            let mut packages = vec![make_package("alpha", "area-a", &[])];
            packages[0].relative = "area-a/alpha".to_string();
            let repo = make_repo(packages);
            let file_changes = vec![FileChange {
                path: PathBuf::from("area-a/alpha/src/lib.rs"),
                status: FileStatus::Modified,
                action: FileAction::Modified,
                lines_added: 1,
                lines_removed: 0,
            }];
            let result = build_result_with_file_changes(repo, file_changes);

            let (has, count, area) = package_area_source_code_change_count(
                &result,
                Some(&PathBuf::from("/repo/area-a/alpha")),
            )
            .expect("area should resolve");
            assert!(has);
            assert_eq!(count, 1);
            assert_eq!(area, "area-a");
        }

        #[test]
        fn package_area_source_code_change_count_ignores_file_changes_docs() {
            let mut packages = vec![make_package("alpha", "area-a", &[])];
            packages[0].relative = "area-a/alpha".to_string();
            let repo = make_repo(packages);
            let file_changes = vec![FileChange {
                path: PathBuf::from("area-a/alpha/README.md"),
                status: FileStatus::Modified,
                action: FileAction::Modified,
                lines_added: 1,
                lines_removed: 0,
            }];
            let result = build_result_with_file_changes(repo, file_changes);

            let (has, count, _area) = package_area_source_code_change_count(
                &result,
                Some(&PathBuf::from("/repo/area-a/alpha")),
            )
            .expect("area should resolve");
            assert!(!has);
            assert_eq!(count, 0);
        }
    }

    mod path_list_rendering {
        use super::*;

        #[test]
        fn lines_format_one_path_per_line() {
            let repo_root = PathBuf::from("/repo");
            let paths = vec![PathBuf::from("src/main.rs"), PathBuf::from("src/lib.rs")];
            let output = render_path_list(&repo_root, &paths, PathListFormat::Lines, false);
            let lines: Vec<&str> = output.trim().lines().collect();
            assert_eq!(lines.len(), 2);
        }

        #[test]
        fn bullet_list_format_has_bullet_prefix() {
            let repo_root = PathBuf::from("/repo");
            let paths = vec![PathBuf::from("src/main.rs")];
            let output = render_path_list(&repo_root, &paths, PathListFormat::BulletList, false);
            // UnorderedList uses bullet characters
            assert!(!output.is_empty());
        }

        #[test]
        fn csv_format_comma_separated() {
            let repo_root = PathBuf::from("/repo");
            let paths = vec![PathBuf::from("src/a.rs"), PathBuf::from("src/b.rs")];
            let output = render_path_list(&repo_root, &paths, PathListFormat::Csv, false);
            assert!(output.contains(", "));
        }

        #[test]
        fn no_path_shows_basename_only() {
            let repo_root = PathBuf::from("/repo");
            let paths = vec![PathBuf::from("deeply/nested/file.rs")];
            let output = render_path_list(&repo_root, &paths, PathListFormat::Lines, true);
            // With no_path, should not contain the directory segments in display text
            // (though they may be in the OSC8 link target)
            assert!(output.contains("file.rs"));
        }

        #[test]
        fn empty_paths_produces_empty_output() {
            let repo_root = PathBuf::from("/repo");
            let paths: Vec<PathBuf> = vec![];
            let output = render_path_list(&repo_root, &paths, PathListFormat::Lines, false);
            assert!(output.is_empty());
        }
    }

    mod docs_output_rendering {
        use super::*;
        use chrono::Utc;

        fn make_doc(relative: &str, title: &str, title_source: TitleSource) -> MarkdownMeta {
            MarkdownMeta {
                filepath: PathBuf::from(format!("/repo/{relative}")),
                relative: relative.to_string(),
                package: None,
                title: title.to_string(),
                title_source,
                model: None,
                prompt: None,
                last_updated: Utc::now(),
                updated_source: UpdatedSource::FileMetadata,
                content_hash: "abc123".to_string(),
                has_blast_radius: false,
                blast_radius: None,
                frontmatter_keys: vec!["title".to_string()],
            }
        }

        #[test]
        fn non_verbose_output_has_header_on_stderr() {
            let docs = vec![make_doc(
                "docs/readme.md",
                "Readme",
                TitleSource::FrontmatterTitle,
            )];
            let output = render_docs_output(&docs, 0);
            assert!(output.stderr.contains("Docs"));
            assert!(output.stderr.contains("1 document"));
        }

        #[test]
        fn non_verbose_output_has_footer_on_stderr() {
            let docs = vec![make_doc(
                "docs/readme.md",
                "Readme",
                TitleSource::FrontmatterTitle,
            )];
            let output = render_docs_output(&docs, 0);
            assert!(output.stderr.contains("--verbose"));
            assert!(output.stderr.contains("metadata for documents"));
        }

        #[test]
        fn non_verbose_document_list_on_stdout() {
            let docs = vec![make_doc(
                "docs/readme.md",
                "Readme",
                TitleSource::FrontmatterTitle,
            )];
            let output = render_docs_output(&docs, 0);
            assert!(output.stdout.contains("readme.md"));
        }

        #[test]
        fn verbose_output_includes_title_with_provenance() {
            let docs = vec![make_doc(
                "docs/guide.md",
                "Guide",
                TitleSource::FrontmatterTitle,
            )];
            let output = render_docs_output(&docs, 1);
            assert!(output.stdout.contains("title:"));
            assert!(output.stdout.contains("title property"));
        }

        #[test]
        fn verbose_h1_title_provenance() {
            let docs = vec![make_doc("docs/guide.md", "Guide", TitleSource::H1Heading)];
            let output = render_docs_output(&docs, 1);
            assert!(output.stdout.contains("H1 heading"));
        }

        #[test]
        fn verbose_none_title_has_provenance() {
            let docs = vec![make_doc("docs/empty.md", "", TitleSource::None)];
            let output = render_docs_output(&docs, 1);
            // Should show "none" provenance, not drop it
            assert!(output.stdout.contains("none"));
        }

        #[test]
        fn verbose_includes_updated_with_provenance() {
            let docs = vec![make_doc(
                "docs/guide.md",
                "Guide",
                TitleSource::FrontmatterTitle,
            )];
            let output = render_docs_output(&docs, 1);
            assert!(output.stdout.contains("updated:"));
            assert!(output.stdout.contains("file metadata"));
        }

        #[test]
        fn verbose_includes_frontmatter_properties() {
            let mut doc = make_doc("docs/guide.md", "Guide", TitleSource::FrontmatterTitle);
            doc.frontmatter_keys = vec!["blast_radius".to_string(), "title".to_string()];
            let output = render_docs_output(&[doc], 1);
            assert!(output.stdout.contains("frontmatter properties:"));
            assert!(output.stdout.contains("blast_radius"));
        }

        #[test]
        fn prompt_count_shown_in_header() {
            let mut doc = make_doc("docs/guide.md", "Guide", TitleSource::FrontmatterTitle);
            doc.prompt = Some("Generate a summary".to_string());
            let output = render_docs_output(&[doc], 0);
            assert!(output.stderr.contains("with prompts"));
        }
    }

    mod deps_dot {
        use super::*;

        #[test]
        fn returns_none_when_no_edges() {
            let packages = vec![
                make_package("alpha", "area-a", &[]),
                make_package("beta", "area-b", &[]),
            ];
            assert!(build_deps_dot(&packages, None).is_none());
        }

        #[test]
        fn generates_digraph_with_edges() {
            let packages = vec![
                make_package("cli", "sniff", &["lib"]),
                make_package("lib", "sniff", &[]),
            ];
            let result = build_deps_dot(&packages, None).unwrap();
            assert!(result.starts_with("digraph G {"));
            assert!(result.contains("subgraph cluster_"));
            assert!(result.contains("label=\"sniff\""));
            assert!(result.contains("n0 -> n1;"));
        }

        #[test]
        fn groups_packages_by_area() {
            let packages = vec![
                make_package("speaks-cli", "biscuit-speaks", &["speaks-lib"]),
                make_package("speaks-lib", "biscuit-speaks", &[]),
                make_package("sniff-cli", "sniff", &["sniff-lib"]),
                make_package("sniff-lib", "sniff", &[]),
            ];
            let result = build_deps_dot(&packages, None).unwrap();
            assert!(result.contains("label=\"biscuit-speaks\""));
            assert!(result.contains("label=\"sniff\""));
        }

        #[test]
        fn cross_area_edges() {
            let packages = vec![
                make_package("app", "apps", &["core"]),
                make_package("core", "libs", &[]),
            ];
            let result = build_deps_dot(&packages, None).unwrap();
            assert!(result.contains("n0 -> n1;"));
        }

        #[test]
        fn focus_groups_focus_subgraph_and_floats_external_nodes() {
            // dm-cli depends on dm-lib (both in darkmatter area);
            // dm-lib depends on biscuit-terminal (external);
            // claudine (external) depends on dm-cli.
            let packages = vec![
                make_package("dm-cli", "darkmatter", &["dm-lib"]),
                make_package("dm-lib", "darkmatter", &["biscuit-terminal"]),
                make_package("biscuit-terminal", "biscuit-terminal", &[]),
                make_package("claudine", "claudine", &["dm-cli"]),
            ];
            let focus: std::collections::HashSet<&str> = ["dm-cli", "dm-lib"].into_iter().collect();
            let result = build_deps_dot(&packages, Some(&focus)).unwrap();

            // Focus area is the only cluster (count `subgraph cluster_` occurrences).
            assert_eq!(
                result.matches("subgraph cluster_").count(),
                1,
                "expected exactly one cluster wrapping the focus area"
            );
            assert!(result.contains("label=\"darkmatter\""));

            // External packages still appear as bare nodes, drawn dashed
            assert!(result.contains("n2 [label=\"biscuit-terminal\", style=dashed];"));
            assert!(result.contains("n3 [label=\"claudine\", style=dashed];"));

            // Edges touching focus are emitted
            assert!(
                result.contains("n0 -> n1;"),
                "dm-cli -> dm-lib edge missing"
            );
            assert!(
                result.contains("n1 -> n2;"),
                "dm-lib -> biscuit-terminal edge missing"
            );
            assert!(
                result.contains("n3 -> n0;"),
                "claudine -> dm-cli edge missing"
            );
        }

        #[test]
        fn focus_omits_edges_between_external_only() {
            // Both biscuit-terminal and biscuit-file are 1-hop external context for
            // the darkmatter focus. Their internal edge must NOT be drawn because
            // it doesn't touch the focus area.
            let packages = vec![
                make_package(
                    "dm-lib",
                    "darkmatter",
                    &["biscuit-terminal", "biscuit-file"],
                ),
                make_package("biscuit-terminal", "biscuit-terminal", &["biscuit-file"]),
                make_package("biscuit-file", "biscuit-file", &[]),
            ];
            let focus: std::collections::HashSet<&str> = ["dm-lib"].into_iter().collect();
            let result = build_deps_dot(&packages, Some(&focus)).unwrap();

            assert!(
                result.contains("n0 -> n1;"),
                "focus -> external edge missing"
            );
            assert!(
                result.contains("n0 -> n2;"),
                "focus -> external edge missing"
            );
            // External-only edge: biscuit-terminal (n1) -> biscuit-file (n2)
            assert!(
                !result.contains("n1 -> n2;"),
                "external-only edge should not be drawn"
            );
        }
    }

    mod monorepo_label {
        use super::*;
        use sniff::filesystem::repo::MonorepoStandard;

        #[test]
        fn label_is_authority_alone_with_no_orchestrators() {
            let label = format_monorepo_label(MonorepoStandard::PnpmWorkspaces, &[]);
            assert_eq!(label, "pnpm workspaces");
        }

        #[test]
        fn label_wraps_single_orchestrator_with_authority() {
            let label =
                format_monorepo_label(MonorepoStandard::PnpmWorkspaces, &[MonorepoStandard::Nx]);
            assert_eq!(label, "Nx (using pnpm workspaces)");
        }

        #[test]
        fn label_joins_every_orchestrator_in_layer_order() {
            // A layer may carry multiple orchestrators (e.g. Nx + Lerna over a
            // pnpm workspace). Every orchestrator must surface in the text,
            // joined deterministically in the order the topology layer carries
            // them (the same order the JSON `orchestrators` array emits).
            let label = format_monorepo_label(
                MonorepoStandard::PnpmWorkspaces,
                &[MonorepoStandard::Nx, MonorepoStandard::Lerna],
            );
            assert_eq!(label, "Nx + Lerna (using pnpm workspaces)");
        }
    }

    mod hash_section {
        use super::*;

        fn commit_with_message(message: &str) -> CommitInfo {
            CommitInfo {
                sha: "1234567890abcdef".to_string(),
                message: message.to_string(),
                author: "Test User".to_string(),
                timestamp: Utc::now(),
                remotes: None,
                refs: vec![],
            }
        }

        #[test]
        fn renders_body_bullet_points() {
            let msg = "feat: add thing\n\n- first bullet\n- second bullet";
            let commit = commit_with_message(msg);
            let out = render_hash_section(&commit, &[], 0, None);
            assert!(
                out.contains("first bullet"),
                "expected first bullet in output:\n{out}"
            );
            assert!(
                out.contains("second bullet"),
                "expected second bullet in output:\n{out}"
            );
        }

        #[test]
        fn renders_commit_header_with_author() {
            let msg = "feat: add thing\n\n- first bullet";
            let commit = commit_with_message(msg);
            let out = render_hash_section(&commit, &[], 0, None);
            assert!(
                out.contains("Test User"),
                "expected author attribution in output:\n{out}"
            );
            assert!(
                out.contains("add thing"),
                "expected subject in output:\n{out}"
            );
            assert!(
                out.contains("first bullet"),
                "expected first bullet in output:\n{out}"
            );
        }

        #[test]
        fn omits_bullet_section_when_no_body() {
            let commit = commit_with_message("feat: add thing");
            let out = render_hash_section(&commit, &[], 0, None);
            assert!(
                out.contains("add thing"),
                "expected subject in output:\n{out}"
            );
        }
    }
}
