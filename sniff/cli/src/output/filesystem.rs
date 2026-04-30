//! Filesystem section output formatting (Git, Repo, Languages, Docs).

use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::mermaid::MermaidDiagram;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::docs::MarkdownMeta;
use sniff::filesystem::git::{BehindStatus, ConventionalCommit, FileAction, FileStatus, RefKind};
use sniff::filesystem::repo::{DependencyEntry, Package, RepoInfo};
use sniff::filesystem::{
    FileAssociationBreakdown, FileAssociationStats, FrameworkStats, TitleSource, UpdatedSource,
};

use super::{TextOutput, format_number, relative_path};
use crate::args::{FilesFilter, PackagesFormat};

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

fn area_parent(area: &str) -> Option<String> {
    if area == "root" {
        return None;
    }

    let parent = Path::new(area).parent()?;
    if parent.as_os_str().is_empty() {
        None
    } else {
        Some(parent.to_string_lossy().to_string())
    }
}

fn build_area_hierarchy(
    areas: &[String],
) -> (Vec<String>, std::collections::HashMap<String, Vec<String>>) {
    let area_set: std::collections::HashSet<&str> = areas.iter().map(String::as_str).collect();
    let mut top_areas = Vec::new();
    let mut children: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for area in areas {
        match area_parent(area) {
            Some(parent) if area_set.contains(parent.as_str()) => {
                children.entry(parent).or_default().push(area.clone());
            }
            _ => top_areas.push(area.clone()),
        }
    }

    (top_areas, children)
}

fn append_package_items(items: &mut Vec<RenderableContent>, pkg: &Package, verbose: u8) {
    let formatted = format_package_items(pkg, verbose);
    let main = Prose::new(&formatted[0]).render_optimistic(None);
    items.push(RenderableContent::String(main));

    if formatted.len() > 1 {
        let detail_items: Vec<String> = formatted[1..]
            .iter()
            .map(|s| Prose::new(s).render_optimistic(None))
            .collect();
        let detail_list = UnorderedList::new(detail_items).with_bullet("  ");
        items.push(RenderableContent::Component(Rc::new(detail_list)));
    }
}

fn append_area_section(
    output: &mut Vec<RenderableContent>,
    area: &str,
    area_packages: &std::collections::HashMap<String, Vec<&Package>>,
    area_children: &std::collections::HashMap<String, Vec<String>>,
    verbose: u8,
) {
    let label = Prose::new(format!("<blue><b>{}</b></blue>", area)).render_optimistic(None);
    output.push(RenderableContent::String(label));

    let mut inner_items: Vec<RenderableContent> = Vec::new();
    if let Some(packages) = area_packages.get(area) {
        for pkg in packages {
            append_package_items(&mut inner_items, pkg, verbose);
        }
    }

    if let Some(children) = area_children.get(area) {
        for child in children {
            append_area_section(
                &mut inner_items,
                child,
                area_packages,
                area_children,
                verbose,
            );
        }
    }

    if !inner_items.is_empty() {
        let inner_list = UnorderedList::from(inner_items);
        output.push(RenderableContent::Component(Rc::new(inner_list)));
    }
}

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
/// Returns `(browse_url, provider)` if a browsable remote is found, or `None`
/// if no remote has a resolvable browse URL.
fn build_commit_url_base(
    git: &sniff::filesystem::git::GitInfo,
) -> Option<(String, sniff::filesystem::git::GitHostingProvider)> {
    // Prefer "origin", fall back to the first remote with a URL
    let remote = git
        .remotes
        .iter()
        .find(|r| r.name == "origin")
        .or_else(|| git.remotes.first())?;
    let url = remote.url.as_ref()?;
    let (_, browse_url) = parse_git_url(url, &remote.provider);
    browse_url.map(|base| (base, remote.provider))
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
        None => format!("<b>{short_sha}</b>"),
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
            "[{}] <dim>{}</dim> {}<blue><b>{}</b></blue>{}{}",
            sha_display, truncated, date_prefix, date_str, refs_part, user_part,
        )
    }
}

/// Render detailed information for a single commit looked up by `hash` subcommand.
///
/// Shows the commit as a one-liner followed by a list of files changed.
pub fn render_hash_section(
    commit: &sniff::filesystem::git::CommitInfo,
    files: &[(std::path::PathBuf, sniff::filesystem::git::DeltaKind)],
    verbose: u8,
    commit_url: Option<&str>,
) -> String {
    use sniff::filesystem::git::DeltaKind;

    let mut out = String::new();
    let terminal = Terminal::default();

    // === Commit Section ===
    let status_title = Prose::new("<b><u>Commit</u></b>");
    writeln!(out, "\n{}\n", status_title.render(&terminal)).unwrap();

    let commit_line = format_commit_line(commit, verbose, commit_url);
    let rendered = Prose::new(commit_line.as_str()).render(&terminal);
    let list = UnorderedList::new(vec![rendered]);
    writeln!(out, "{}", list.render(&terminal)).unwrap();

    // === Files Section ===
    if files.is_empty() {
        return out;
    }

    let files_title = Prose::new("<b><u>Files changed</u></b>");
    writeln!(out, "{}\n", files_title.render(&terminal)).unwrap();

    let file_items: Vec<String> = files
        .iter()
        .map(|(path, kind)| {
            let path_str = path.display().to_string();
            let (dir, name) = split_path(&path_str);
            let dir_part = if dir.is_empty() { String::new() } else { dir };
            match kind {
                DeltaKind::Added => format!("<lime>{}: {}<b>{}</b></lime>", kind, dir_part, name),
                DeltaKind::Modified => {
                    format!("<yellow>{}: {}<b>{}</b></yellow>", kind, dir_part, name)
                }
                DeltaKind::Deleted => format!("<red>{}: {}<b>{}</b></red>", kind, dir_part, name),
                DeltaKind::Renamed | DeltaKind::Copied => {
                    format!("<cyan>{}: {}<b>{}</b></cyan>", kind, dir_part, name)
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

    // Determine how many of the most recent commits are unpushed.
    // Use the "origin" tracking ahead count; if unavailable, assume all are pushed.
    let unpushed_count = git
        .tracking
        .iter()
        .find(|t| t.remote == "origin")
        .map(|t| t.ahead)
        .unwrap_or(0);

    let mut status_items: Vec<String> = Vec::new();

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

    // Recent commits with conventional commit parsing (oldest first, so most recent is at bottom)
    let commits: Vec<_> = git.recent.iter().take(history_count).collect();
    for (display_index, commit) in commits.iter().rev().enumerate() {
        // display_index 0 = oldest displayed commit, last = most recent.
        // The most recent `unpushed_count` commits (at the end) are unpushed.
        let is_pushed = display_index < commits.len().saturating_sub(unpushed_count);
        let commit_url = if is_pushed {
            commit_url_base.as_ref().map(|(base, provider)| {
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

    status_items
}

/// ## Arguments
///
/// * `git` - Git repository information
/// * `history_count` - Number of recent commits to display
pub fn render_git_section(
    git: &sniff::filesystem::git::GitInfo,
    history_count: usize,
    verbose: u8,
    compact: bool,
) -> String {
    let mut out = String::new();
    let terminal = Terminal::default();

    // === Status Section ===
    let status_title = Prose::new("<b><u>Status</u></b>");
    writeln!(out, "\n{}\n", status_title.render(&terminal)).unwrap();

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

    // === Worktrees Section (only if worktrees exist) ===
    if !git.worktrees.is_empty() {
        let wt_title = Prose::new("<b><u>Worktrees</u></b>");
        writeln!(out, "{}\n", wt_title.render(&terminal)).unwrap();

        let mut wt_list = UnorderedList::empty();

        // Base repo line: varies based on whether we're in the base repo or a worktree
        if git.in_worktree {
            if let Some(ref base_root) = git.base_repo_root {
                wt_list.add(Prose::new(format!(
                    "Base Repo: <dim>the base repo is located at <blue-500>{}</blue-500></dim>",
                    base_root.display()
                )));
            }
        } else if let Some(ref branch) = git.current_branch {
            wt_list.add(Prose::new(format!(
                "<b>Base Repo:</b> you are in the base repo which is on the <blue-500>{branch}</blue-500> branch"
            )));
        }

        // Worktree lines: varies based on whether we're inside that worktree
        for info in git.worktrees.values() {
            let branch = &info.branch;
            let status = if info.merged && info.ahead == 0 {
                format!("merged into <b>{}</b>", &info.base_branch)
            } else {
                format_ahead_behind_of(info.ahead, info.behind, &info.base_branch)
            };
            let merge_status = if info.has_conflicts {
                " · <red-500><b>conflicts</b></red-500>"
            } else {
                " · <green-500>clean</green-500>"
            };

            let uncommitted = if info.changed_files > 0 {
                format!(
                    " <dim><i>merge</i></dim> · <red-500>{}</red-500> <dim><i>uncommitted {}</i></dim>",
                    info.changed_files,
                    if info.changed_files == 1 {
                        "file"
                    } else {
                        "files"
                    }
                )
            } else {
                String::new()
            };

            // Check if we're inside this particular worktree
            let is_current = git.in_worktree
                && git.repo_root.canonicalize().ok() == info.filepath.canonicalize().ok();

            if is_current {
                wt_list.add(Prose::new(format!(
                    "<b>{branch}:</b> you are {status}{merge_status}{uncommitted}"
                )));
            } else {
                wt_list.add(Prose::new(format!(
                    "{branch}: <dim>is {status}</dim>{merge_status}{uncommitted}"
                )));
            }
        }
        writeln!(out, "{}", wt_list.render(&terminal)).unwrap();
    }

    // === Meta Section ===
    let meta_title = Prose::new("<b><u>Meta</u></b>");
    writeln!(out, "{}\n", meta_title.render(&terminal)).unwrap();

    let mut meta_list = UnorderedList::empty();

    // --- Local ---
    if let Some(ref current) = git.current_branch {
        let local_header: RenderableContent = Prose::new("<b>Local:</b>").into();
        if verbose > 0 {
            // Verbose: nested list with current branch + other branches
            let mut local_list = UnorderedList::empty();

            let dirty = if git.status.is_dirty {
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

            let branches_header: RenderableContent = Prose::new("<b>Branches:</b>").into();
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
        let remotes_header: RenderableContent = Prose::new("<b>Remotes:</b>").into();
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
        let config_header: RenderableContent = Prose::new("<b>Config:</b>").into();
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
            let crypto_header: RenderableContent = Prose::new("<b>Crypto</b>").into();
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

/// Format a MonorepoTool for display.
fn format_monorepo_tool(tool: &sniff::filesystem::repo::MonorepoTool) -> &'static str {
    use sniff::filesystem::repo::MonorepoTool;
    match tool {
        MonorepoTool::CargoWorkspace => "Cargo Workspace",
        MonorepoTool::NpmWorkspaces => "npm Workspaces",
        MonorepoTool::PnpmWorkspaces => "pnpm Workspaces",
        MonorepoTool::YarnWorkspaces => "Yarn Workspaces",
        MonorepoTool::Nx => "Nx",
        MonorepoTool::Turborepo => "Turborepo",
        MonorepoTool::Lerna => "Lerna",
        _ => "Unknown",
    }
}

#[derive(Debug, Clone, Default)]
struct UpdateSummary {
    checked_packages: usize,
    packages_with_updates: usize,
    packages_with_major_updates: usize,
    dependency_updates: usize,
    major_dependency_updates: usize,
    sample_transitions: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct PackageUpdateSummary {
    checked: bool,
    dependency_updates: usize,
    major_dependency_updates: usize,
    sample_transitions: Vec<String>,
}

fn collect_dependency_updates(
    target: &mut UpdateSummary,
    package_name: Option<&str>,
    deps: &Option<Vec<DependencyEntry>>,
) {
    let Some(deps) = deps else {
        return;
    };

    for dep in deps {
        if !dep.is_updatable {
            continue;
        }

        target.dependency_updates += 1;
        if dep.has_major_update {
            target.major_dependency_updates += 1;
        }

        if target.sample_transitions.len() >= 5 {
            continue;
        }

        let Some(latest) = dep.latest_version.as_deref() else {
            continue;
        };

        let current = dep
            .actual_version
            .as_deref()
            .unwrap_or(dep.targeted_version.as_str());
        let prefix = package_name
            .map(|name| format!("{name}: "))
            .unwrap_or_default();
        target
            .sample_transitions
            .push(format!("{}{} {} -> {}", prefix, dep.name, current, latest));
    }
}

fn summarize_package_updates(pkg: &Package) -> PackageUpdateSummary {
    let mut summary = PackageUpdateSummary {
        checked: pkg.is_updatable.is_some(),
        ..PackageUpdateSummary::default()
    };

    let mut rollup = UpdateSummary::default();
    for deps in [
        &pkg.dependencies,
        &pkg.dev_dependencies,
        &pkg.peer_dependencies,
        &pkg.optional_dependencies,
    ] {
        collect_dependency_updates(&mut rollup, None, deps);
    }

    summary.dependency_updates = rollup.dependency_updates;
    summary.major_dependency_updates = rollup.major_dependency_updates;
    summary.sample_transitions = rollup.sample_transitions;
    summary
}

fn summarize_repo_updates(
    repo: &RepoInfo,
    filtered_packages: Option<&[&Package]>,
    latest_versions_requested: bool,
) -> Option<UpdateSummary> {
    let mut summary = UpdateSummary::default();

    if let Some(packages) = filtered_packages {
        for pkg in packages {
            if pkg.is_updatable.is_some() {
                summary.checked_packages += 1;
            }
            if pkg.is_updatable == Some(true) {
                summary.packages_with_updates += 1;
            }
            if pkg.has_major_update == Some(true) {
                summary.packages_with_major_updates += 1;
            }

            for deps in [
                &pkg.dependencies,
                &pkg.dev_dependencies,
                &pkg.peer_dependencies,
                &pkg.optional_dependencies,
            ] {
                collect_dependency_updates(&mut summary, Some(pkg.name.as_str()), deps);
            }
        }

        if summary.checked_packages > 0 || latest_versions_requested {
            return Some(summary);
        }
        return None;
    }

    let has_dependency_lists = [
        &repo.dependencies,
        &repo.dev_dependencies,
        &repo.peer_dependencies,
        &repo.optional_dependencies,
    ]
    .iter()
    .any(|deps| deps.as_ref().is_some_and(|entries| !entries.is_empty()));

    if !has_dependency_lists && !latest_versions_requested {
        return None;
    }

    summary.checked_packages = usize::from(has_dependency_lists && latest_versions_requested);
    for deps in [
        &repo.dependencies,
        &repo.dev_dependencies,
        &repo.peer_dependencies,
        &repo.optional_dependencies,
    ] {
        collect_dependency_updates(&mut summary, None, deps);
    }
    summary.packages_with_updates = usize::from(summary.dependency_updates > 0);
    summary.packages_with_major_updates = usize::from(summary.major_dependency_updates > 0);

    Some(summary)
}

fn package_word(count: usize) -> &'static str {
    if count == 1 { "package" } else { "packages" }
}

fn update_word(count: usize) -> &'static str {
    if count == 1 { "update" } else { "updates" }
}

fn render_update_summary(
    summary: &UpdateSummary,
    latest_versions_requested: bool,
    verbose: u8,
    term: &Terminal,
) -> String {
    if !latest_versions_requested {
        return String::new();
    }

    let mut out = String::new();

    let summary_line = format!(
        "<dim>Registry check:</dim> {} {} checked, {} with updates, {} with major updates",
        summary.checked_packages,
        package_word(summary.checked_packages),
        summary.packages_with_updates,
        summary.packages_with_major_updates,
    );
    writeln!(out, "{}", Prose::new(&summary_line).render(term)).unwrap();

    if verbose > 1 && !summary.sample_transitions.is_empty() {
        let samples = summary
            .sample_transitions
            .iter()
            .map(|sample| Prose::new(format!("<dim>{}</dim>", sample)).render(term))
            .collect::<Vec<_>>();
        let list = UnorderedList::new(samples);
        writeln!(out, "{}", list.render(term)).unwrap();
    }

    out
}

/// Render a single package as a styled list item string.
///
/// Format a single package as a list of renderable items.
///
/// The first item is the main line (name, version, path, etc.).
/// Subsequent items are verbose details rendered as child bullets.
fn format_package_items(pkg: &sniff::filesystem::repo::Package, verbose: u8) -> Vec<String> {
    let update_summary = summarize_package_updates(pkg);
    let version_part = pkg
        .version
        .as_ref()
        .map(|v| format!(" <dim>v{}</dim>", v))
        .unwrap_or_default();

    let lang_part = pkg
        .primary_language
        .as_ref()
        .map(|l| format!(" <dim>[{}]</dim>", l))
        .unwrap_or_default();

    let updatable_part = match (pkg.is_updatable, pkg.has_major_update) {
        (Some(true), Some(true)) => " <red>*</red>",
        (Some(true), _) => " <yellow>*</yellow>",
        _ => "",
    };

    let name_part = if pkg.is_excluded {
        format!("<orange>{}</orange>", pkg.name)
    } else {
        format!("<b>{}</b>", pkg.name)
    };

    let main_line = format!(
        "{}{} <dim>({})</dim>{}{}",
        name_part, version_part, pkg.relative, lang_part, updatable_part
    );

    let mut items = vec![main_line];

    if verbose > 0 {
        if !pkg.features.is_empty() {
            items.push(format!("<dim>features:</dim> {}", pkg.features.join(", ")));
        }
        if !pkg.depends_on.is_empty() {
            items.push(format!(
                "<dim>depends on:</dim> {}",
                pkg.depends_on.join(", ")
            ));
        }
    }

    if verbose > 1 {
        if !pkg.used_by.is_empty() {
            items.push(format!("<dim>used by:</dim> {}", pkg.used_by.join(", ")));
        }
        if !pkg.languages.is_empty() {
            let languages = pkg
                .languages
                .iter()
                .map(|language| language.language.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            items.push(format!("<dim>langs:</dim> {}", languages));
        }
        if !pkg.frameworks.is_empty() {
            let frameworks = pkg
                .frameworks
                .iter()
                .map(|framework| framework.framework.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            items.push(format!("<dim>frameworks:</dim> {}", frameworks));
        }
    }

    if verbose > 0 && update_summary.checked {
        items.push(format!(
            "<dim>updates:</dim> {} {}, {} major",
            update_summary.dependency_updates,
            update_word(update_summary.dependency_updates),
            update_summary.major_dependency_updates,
        ));
    }

    if verbose > 1 && !update_summary.sample_transitions.is_empty() {
        items.push(format!(
            "<dim>latest:</dim> {}",
            update_summary.sample_transitions.join(", ")
        ));
    }

    items
}

/// Render package names as a comma-separated plain text list.
///
/// Returns an error message if the repo is not a monorepo.
/// Filter a package list by name/area filters and an optional package-area scope.
fn select_repo_packages<'a>(
    packages: &'a [Package],
    repo_filter: &[String],
    package_area: Option<&str>,
) -> Vec<&'a Package> {
    let mut filtered = filter_packages(packages, repo_filter);
    if let Some(area) = package_area {
        let needle = area.to_lowercase();
        filtered.retain(|p| p.package_area.to_lowercase() == needle);
    }
    filtered
}

/// Collect package names matching the given filters and area scope.
///
/// Returns an empty vec when the repo is not a monorepo.
pub fn collect_repo_package_names<'a>(
    repo: &'a RepoInfo,
    repo_filter: &[String],
    package_area: Option<&str>,
) -> Vec<&'a str> {
    if !repo.is_monorepo {
        return Vec::new();
    }
    let Some(packages) = repo.packages.as_ref() else {
        return Vec::new();
    };
    select_repo_packages(packages, repo_filter, package_area)
        .into_iter()
        .map(|p| p.name.as_str())
        .collect()
}

/// Render a styled package entry; with `verbose > 0` appends the dimmed/italic
/// repo-relative root directory (e.g. `name(<dim><i>./relative</i></dim>)`).
fn package_entry_markup(pkg: &Package, verbose: u8) -> String {
    if verbose > 0 {
        format!(
            "{}(<dim><i>./{}</i></dim>)",
            pkg.name,
            pkg.relative.trim_start_matches("./")
        )
    } else {
        pkg.name.clone()
    }
}

/// Render the package list for `sniff repo packages` in the requested format.
///
/// Honors `--md` (Markdown unordered list), `--list` (one entry per line), and
/// the default csv form. With `verbose > 0`, each entry is annotated with the
/// dimmed package root directory.
pub fn render_repo_packages_formatted(
    repo: &RepoInfo,
    repo_filter: &[String],
    package_area: Option<&str>,
    format: PackagesFormat,
    verbose: u8,
) -> String {
    if !repo.is_monorepo {
        return String::from(
            "- the \"packages\" subcommand is only intended to be used in a monorepo",
        );
    }

    let Some(packages) = repo.packages.as_ref() else {
        return String::new();
    };

    let filtered = select_repo_packages(packages, repo_filter, package_area);
    if filtered.is_empty() {
        return String::new();
    }

    let term = Terminal::default();
    let entries: Vec<String> = filtered
        .iter()
        .map(|pkg| {
            let markup = package_entry_markup(pkg, verbose);
            Prose::new(markup).render(&term)
        })
        .collect();

    match format {
        PackagesFormat::Csv => entries.join(", "),
        PackagesFormat::Markdown => entries
            .iter()
            .map(|e| format!("- {e}"))
            .collect::<Vec<_>>()
            .join("\n"),
        PackagesFormat::List => entries.join("\n"),
    }
}

/// Collect unique package area names, honoring the optional scope and filters.
fn select_repo_package_areas<'a>(
    packages: &'a [Package],
    repo_filter: &[String],
    package_area: Option<&str>,
) -> Vec<&'a str> {
    select_repo_package_areas_with_roots(packages, repo_filter, package_area)
        .into_iter()
        .map(|(area, _)| area)
        .collect()
}

/// Compute the repo-relative area root directory for a given package.
///
/// For a package whose `package_area` is `"root"` (a top-level package living
/// directly at the repo root, such as `model_id` in this workspace), returns
/// `"."` — the repo root itself.
///
/// Otherwise, when `pkg.relative` starts with `pkg.package_area` (the common
/// case — including multi-segment areas such as `apps/browser`), the area root
/// is the `package_area` value verbatim. This preserves correctness for nested
/// monorepo layouts where an area name can legitimately contain `/`.
///
/// If neither of those holds, falls back to the first path component of
/// `pkg.relative`, which matches `Package::package_area` for the overwhelming
/// majority of this workspace's layouts.
///
/// Returns a borrowed `&str` to avoid allocation in the hot render loop.
fn package_area_root(pkg: &Package) -> &str {
    if pkg.package_area == "root" {
        return ".";
    }

    let relative = pkg.relative.trim_start_matches("./");

    // Prefer `pkg.package_area` when it prefixes `relative` at a path boundary.
    // This handles multi-segment areas like `apps/browser/my_package` where the
    // area is `apps/browser` — naive `split('/').next()` would incorrectly
    // return `apps`.
    if relative == pkg.package_area {
        return &pkg.package_area;
    }
    if let Some(rest) = relative.strip_prefix(pkg.package_area.as_str())
        && rest.starts_with('/')
    {
        return &pkg.package_area;
    }

    relative.split('/').next().unwrap_or(relative)
}

/// Same selection logic as [`select_repo_package_areas`] but also returns the
/// repo-relative area root directory derived from each area's first package.
fn select_repo_package_areas_with_roots<'a>(
    packages: &'a [Package],
    repo_filter: &[String],
    package_area: Option<&str>,
) -> Vec<(&'a str, &'a str)> {
    // Capture the first package encountered for each area (deterministic via
    // BTreeMap ordering) so we can derive the area root once.
    let mut seen: std::collections::BTreeMap<&str, &Package> = std::collections::BTreeMap::new();
    for pkg in packages {
        seen.entry(pkg.package_area.as_str()).or_insert(pkg);
    }

    let scope = package_area.map(str::to_lowercase);
    let filters: Vec<RepoFilter> = if repo_filter.is_empty() {
        Vec::new()
    } else {
        repo_filter.iter().map(|f| RepoFilter::parse(f)).collect()
    };

    seen.into_iter()
        .filter(|(area, _)| {
            if let Some(needle) = scope.as_deref()
                && area.to_lowercase() != needle
            {
                return false;
            }
            if filters.is_empty() {
                return true;
            }
            let lower = area.to_lowercase();
            filters.iter().any(|f| {
                let hit = lower.contains(&f.query.to_lowercase());
                if f.negate { !hit } else { hit }
            })
        })
        .map(|(area, pkg)| (area, package_area_root(pkg)))
        .collect()
}

/// Collect unique package area names matching the given filters and scope.
///
/// Returns an empty vec when the repo is not a monorepo.
pub fn collect_repo_package_area_names<'a>(
    repo: &'a RepoInfo,
    repo_filter: &[String],
    package_area: Option<&str>,
) -> Vec<&'a str> {
    if !repo.is_monorepo {
        return Vec::new();
    }
    let Some(packages) = repo.packages.as_ref() else {
        return Vec::new();
    };
    select_repo_package_areas(packages, repo_filter, package_area)
}

/// Render the unique package area list for `sniff repo package-areas` in the
/// requested format.
///
/// Honors `--md` (Markdown unordered list), `--list` (one entry per line), and
/// the default csv form. With `verbose > 0`, each entry is annotated with the
/// dimmed repo-relative area directory.
pub fn render_repo_package_areas_formatted(
    repo: &RepoInfo,
    repo_filter: &[String],
    package_area: Option<&str>,
    format: PackagesFormat,
    verbose: u8,
) -> String {
    if !repo.is_monorepo {
        return String::from(
            "- the \"package-areas\" subcommand is only intended to be used in a monorepo",
        );
    }

    let Some(packages) = repo.packages.as_ref() else {
        return String::new();
    };

    let areas = select_repo_package_areas_with_roots(packages, repo_filter, package_area);
    if areas.is_empty() {
        return String::new();
    }

    let term = Terminal::default();
    let entries: Vec<String> = areas
        .iter()
        .map(|(area, root)| {
            let markup = if verbose > 0 {
                // Special-case the "root" area so the annotation reads
                // "root (./)" rather than "root (./root)" (a non-existent
                // directory). Every other area renders as "./{root}".
                let dir_label = if *root == "." {
                    String::from("./")
                } else {
                    format!("./{root}")
                };
                // Note the SPACE before the open paren — spec requires
                // "{package-area} (<dim><i>{dir}</i></dim>)".
                format!("{area} (<dim><i>{dir_label}</i></dim>)")
            } else {
                (*area).to_string()
            };
            Prose::new(markup).render(&term)
        })
        .collect();

    match format {
        PackagesFormat::Csv => entries.join(", "),
        PackagesFormat::Markdown => entries
            .iter()
            .map(|e| format!("- {e}"))
            .collect::<Vec<_>>()
            .join("\n"),
        PackagesFormat::List => entries.join("\n"),
    }
}

/// Collect package names that have uncommitted changes (dirty or untracked files).
///
/// Cross-references git status dirty/untracked file paths with package relative
/// paths to determine which packages are affected.
fn dirty_package_names(result: &sniff::SniffResult) -> Vec<String> {
    let fs = match result.filesystem.as_ref() {
        Some(fs) => fs,
        None => return vec![],
    };

    let packages = match fs.repo.as_ref().and_then(|r| r.packages.as_ref()) {
        Some(p) => p,
        None => return vec![],
    };

    let git = match fs.git.as_ref() {
        Some(g) => g,
        None => return vec![],
    };

    // Collect all dirty file paths (relative to repo root)
    let dirty_paths: Vec<&str> = git
        .status
        .dirty
        .iter()
        .map(|d| d.filepath.to_str().unwrap_or(""))
        .chain(
            git.status
                .untracked
                .iter()
                .map(|u| u.filepath.to_str().unwrap_or("")),
        )
        .collect();

    if dirty_paths.is_empty() {
        return vec![];
    }

    let mut names: Vec<String> = packages
        .iter()
        .filter(|pkg| {
            let prefix = &pkg.relative;
            dirty_paths.iter().any(|path| {
                if prefix.is_empty() {
                    // Root package: file is dirty if it's not inside any other package
                    !packages.iter().any(|other| {
                        !other.relative.is_empty() && path.starts_with(&other.relative)
                    })
                } else {
                    path.starts_with(prefix)
                }
            })
        })
        .map(|pkg| pkg.name.clone())
        .collect();

    names.sort();
    names.dedup();
    names
}

/// Pure selector returning dirty package names, honoring the repo `filter`.
///
/// Returns an empty vector when the repo is not a monorepo, when no packages
/// are detected, or when no dirty paths exist. The result is sorted and
/// de-duplicated.
///
/// ## Notes
///
/// This is the JSON-friendly counterpart to [`render_dirty_packages`]: it
/// never produces prose error strings, so JSON consumers see an empty array
/// instead of the "only intended to be used in a monorepo" message.
pub(crate) fn select_dirty_package_names(
    result: &sniff::SniffResult,
    repo_filter: &[String],
) -> Vec<String> {
    let Some(repo) = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref()) else {
        return Vec::new();
    };
    if !repo.is_monorepo {
        return Vec::new();
    }

    let names = dirty_package_names(result);
    if repo_filter.is_empty() {
        return names;
    }

    let Some(packages) = repo.packages.as_ref() else {
        return Vec::new();
    };
    let filtered = filter_packages(packages, repo_filter);
    let filtered_names: std::collections::HashSet<&str> =
        filtered.iter().map(|p| p.name.as_str()).collect();
    names
        .into_iter()
        .filter(|n| filtered_names.contains(n.as_str()))
        .collect()
}

/// Pure selector returning dirty package area names, honoring the repo `filter`.
///
/// Returns an empty vector when the repo is not a monorepo or when no
/// packages are detected. The result is sorted and de-duplicated.
pub(crate) fn select_dirty_package_area_names(
    result: &sniff::SniffResult,
    repo_filter: &[String],
) -> Vec<String> {
    let Some(repo) = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref()) else {
        return Vec::new();
    };
    if !repo.is_monorepo {
        return Vec::new();
    }
    let Some(packages) = repo.packages.as_ref() else {
        return Vec::new();
    };

    let dirty_names = dirty_package_names(result);
    let dirty_set: std::collections::HashSet<&str> =
        dirty_names.iter().map(|n| n.as_str()).collect();

    let filtered = filter_packages(packages, repo_filter);
    let mut areas: Vec<String> = filtered
        .iter()
        .filter(|p| dirty_set.contains(p.name.as_str()))
        .map(|p| p.package_area.clone())
        .collect();
    areas.sort();
    areas.dedup();
    areas
}

/// Render package names with uncommitted changes as a comma-separated list.
///
/// Returns an error message if the repo is not a monorepo.
pub fn render_dirty_packages(result: &sniff::SniffResult, repo_filter: &[String]) -> String {
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    match repo {
        Some(repo) if repo.is_monorepo => {
            select_dirty_package_names(result, repo_filter).join(", ")
        }
        _ => String::from(
            "- the \"--dirty-packages\" switch is only intended to be used in a monorepo",
        ),
    }
}

/// Render package area names with uncommitted changes as a comma-separated list.
///
/// Returns an error message if the repo is not a monorepo.
pub fn render_dirty_package_areas(result: &sniff::SniffResult, repo_filter: &[String]) -> String {
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    match repo {
        Some(repo) if repo.is_monorepo => {
            select_dirty_package_area_names(result, repo_filter).join(", ")
        }
        _ => String::from(
            "- the \"--dirty-package-areas\" switch is only intended to be used in a monorepo",
        ),
    }
}

/// Collect package names that have staged files (in index).
fn staged_package_names(result: &sniff::SniffResult) -> Vec<String> {
    let fs = match result.filesystem.as_ref() {
        Some(fs) => fs,
        None => return vec![],
    };

    let packages = match fs.repo.as_ref().and_then(|r| r.packages.as_ref()) {
        Some(p) => p,
        None => return vec![],
    };

    let git = match fs.git.as_ref() {
        Some(g) => g,
        None => return vec![],
    };

    let staged_paths: Vec<&str> = git
        .file_changes
        .iter()
        .filter(|f| {
            f.status == sniff::filesystem::git::FileStatus::Staged
                || f.status == sniff::filesystem::git::FileStatus::Both
        })
        .map(|f| f.path.to_str().unwrap_or(""))
        .collect();

    if staged_paths.is_empty() {
        return vec![];
    }

    let mut names: Vec<String> = packages
        .iter()
        .filter(|pkg| {
            let prefix = &pkg.relative;
            staged_paths.iter().any(|path| {
                if prefix.is_empty() {
                    !packages.iter().any(|other| {
                        !other.relative.is_empty() && path.starts_with(&other.relative)
                    })
                } else {
                    path.starts_with(prefix)
                }
            })
        })
        .map(|pkg| pkg.name.clone())
        .collect();

    names.sort();
    names.dedup();
    names
}

/// Pure selector returning staged package names, honoring the repo `filter`.
///
/// Returns an empty vector when the repo is not a monorepo or when nothing
/// is staged. The result is sorted and de-duplicated.
pub(crate) fn select_staged_package_names(
    result: &sniff::SniffResult,
    repo_filter: &[String],
) -> Vec<String> {
    let Some(repo) = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref()) else {
        return Vec::new();
    };
    if !repo.is_monorepo {
        return Vec::new();
    }

    let names = staged_package_names(result);
    if repo_filter.is_empty() {
        return names;
    }

    let Some(packages) = repo.packages.as_ref() else {
        return Vec::new();
    };
    let filtered = filter_packages(packages, repo_filter);
    let filtered_names: std::collections::HashSet<&str> =
        filtered.iter().map(|p| p.name.as_str()).collect();
    names
        .into_iter()
        .filter(|n| filtered_names.contains(n.as_str()))
        .collect()
}

/// Pure selector returning staged package area names, honoring the repo `filter`.
pub(crate) fn select_staged_package_area_names(
    result: &sniff::SniffResult,
    repo_filter: &[String],
) -> Vec<String> {
    let Some(repo) = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref()) else {
        return Vec::new();
    };
    if !repo.is_monorepo {
        return Vec::new();
    }
    let Some(packages) = repo.packages.as_ref() else {
        return Vec::new();
    };

    let staged_names = staged_package_names(result);
    let staged_set: std::collections::HashSet<&str> =
        staged_names.iter().map(|n| n.as_str()).collect();

    let filtered = filter_packages(packages, repo_filter);
    let mut areas: Vec<String> = filtered
        .iter()
        .filter(|p| staged_set.contains(p.name.as_str()))
        .map(|p| p.package_area.clone())
        .collect();
    areas.sort();
    areas.dedup();
    areas
}

/// Render package names with staged files as a comma-separated list.
///
/// Returns an error message if the repo is not a monorepo.
pub fn render_staged_packages(result: &sniff::SniffResult, repo_filter: &[String]) -> String {
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    match repo {
        Some(repo) if repo.is_monorepo => {
            select_staged_package_names(result, repo_filter).join(", ")
        }
        _ => String::from(
            "- the \"staged-packages\" subcommand is only intended to be used in a monorepo",
        ),
    }
}

/// Render package area names with staged files as a comma-separated list.
///
/// Returns an error message if the repo is not a monorepo.
pub fn render_staged_package_areas(result: &sniff::SniffResult, repo_filter: &[String]) -> String {
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    match repo {
        Some(repo) if repo.is_monorepo => {
            select_staged_package_area_names(result, repo_filter).join(", ")
        }
        _ => String::from(
            "- the \"staged-package-areas\" subcommand is only intended to be used in a monorepo",
        ),
    }
}

/// Collect package names that have unstaged changes (modified in working tree).
///
/// Cross-references git file_changes with `Modified` or `Both` status against
/// package relative paths.
fn unstaged_package_names(result: &sniff::SniffResult) -> Vec<String> {
    let fs = match result.filesystem.as_ref() {
        Some(fs) => fs,
        None => return vec![],
    };

    let packages = match fs.repo.as_ref().and_then(|r| r.packages.as_ref()) {
        Some(p) => p,
        None => return vec![],
    };

    let git = match fs.git.as_ref() {
        Some(g) => g,
        None => return vec![],
    };

    let unstaged_paths: Vec<&str> = git
        .file_changes
        .iter()
        .filter(|f| {
            f.status == sniff::filesystem::git::FileStatus::Modified
                || f.status == sniff::filesystem::git::FileStatus::Both
        })
        .map(|f| f.path.to_str().unwrap_or(""))
        .collect();

    if unstaged_paths.is_empty() {
        return vec![];
    }

    let mut names: Vec<String> = packages
        .iter()
        .filter(|pkg| {
            let prefix = &pkg.relative;
            unstaged_paths.iter().any(|path| {
                if prefix.is_empty() {
                    !packages.iter().any(|other| {
                        !other.relative.is_empty() && path.starts_with(&other.relative)
                    })
                } else {
                    path.starts_with(prefix)
                }
            })
        })
        .map(|pkg| pkg.name.clone())
        .collect();

    names.sort();
    names.dedup();
    names
}

/// Pure selector returning unstaged package names, honoring the repo `filter`.
pub(crate) fn select_unstaged_package_names(
    result: &sniff::SniffResult,
    repo_filter: &[String],
) -> Vec<String> {
    let Some(repo) = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref()) else {
        return Vec::new();
    };
    if !repo.is_monorepo {
        return Vec::new();
    }

    let names = unstaged_package_names(result);
    if repo_filter.is_empty() {
        return names;
    }

    let Some(packages) = repo.packages.as_ref() else {
        return Vec::new();
    };
    let filtered = filter_packages(packages, repo_filter);
    let filtered_names: std::collections::HashSet<&str> =
        filtered.iter().map(|p| p.name.as_str()).collect();
    names
        .into_iter()
        .filter(|n| filtered_names.contains(n.as_str()))
        .collect()
}

/// Pure selector returning unstaged package area names, honoring the repo `filter`.
pub(crate) fn select_unstaged_package_area_names(
    result: &sniff::SniffResult,
    repo_filter: &[String],
) -> Vec<String> {
    let Some(repo) = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref()) else {
        return Vec::new();
    };
    if !repo.is_monorepo {
        return Vec::new();
    }
    let Some(packages) = repo.packages.as_ref() else {
        return Vec::new();
    };

    let unstaged_names = unstaged_package_names(result);
    let unstaged_set: std::collections::HashSet<&str> =
        unstaged_names.iter().map(|n| n.as_str()).collect();

    let filtered = filter_packages(packages, repo_filter);
    let mut areas: Vec<String> = filtered
        .iter()
        .filter(|p| unstaged_set.contains(p.name.as_str()))
        .map(|p| p.package_area.clone())
        .collect();
    areas.sort();
    areas.dedup();
    areas
}

/// Render package names with unstaged changes as a comma-separated list.
///
/// Returns an error message if the repo is not a monorepo.
pub fn render_unstaged_packages(result: &sniff::SniffResult, repo_filter: &[String]) -> String {
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    match repo {
        Some(repo) if repo.is_monorepo => {
            select_unstaged_package_names(result, repo_filter).join(", ")
        }
        _ => String::from(
            "- the \"unstaged-packages\" subcommand is only intended to be used in a monorepo",
        ),
    }
}

/// Render package area names with unstaged changes as a comma-separated list.
///
/// Returns an error message if the repo is not a monorepo.
pub fn render_unstaged_package_areas(
    result: &sniff::SniffResult,
    repo_filter: &[String],
) -> String {
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    match repo {
        Some(repo) if repo.is_monorepo => {
            select_unstaged_package_area_names(result, repo_filter).join(", ")
        }
        _ => String::from(
            "- the \"unstaged-package-areas\" subcommand is only intended to be used in a monorepo",
        ),
    }
}

/// Render the package name for the given directory.
///
/// With `verbose >= 1`, appends the package root directory on the same line.
/// Returns empty string if not in a package.
pub fn render_repo_package(
    result: &sniff::SniffResult,
    base_dir: Option<&Path>,
    verbose: u8,
) -> String {
    let dir = resolve_dir(base_dir);
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    if let Some(pkg) = repo.and_then(|r| r.package_for_dir(&dir)) {
        if verbose > 0 {
            let terminal = Terminal::default();
            let line = format!(
                "{} (<i>located in</i> <blue>{}</blue>)",
                pkg.name, pkg.relative
            );
            Prose::new(&line).display(&terminal).to_string()
        } else {
            pkg.name.clone()
        }
    } else {
        String::new()
    }
}

/// Render the package area for the given directory.
///
/// Returns empty string if not in a package area.
pub fn render_repo_package_area(result: &sniff::SniffResult, base_dir: Option<&Path>) -> String {
    let dir = resolve_dir(base_dir);
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    repo.and_then(|r| r.package_area_for_dir(&dir))
        .unwrap_or_default()
        .to_string()
}

/// Render the root directory of the package containing the given directory.
///
/// Returns empty string if not in a package.
pub fn render_repo_package_root(result: &sniff::SniffResult, base_dir: Option<&Path>) -> String {
    let dir = resolve_dir(base_dir);
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    repo.and_then(|r| r.package_for_dir(&dir))
        .map(|pkg| pkg.path.display().to_string())
        .unwrap_or_default()
}

/// Render the root directory of the package area containing the given directory.
///
/// Returns empty string if not in a package area. Root-level packages (area
/// `"root"`) are not considered to be inside a package area directory, so this
/// also returns empty for them.
pub fn render_repo_package_area_root(
    result: &sniff::SniffResult,
    base_dir: Option<&Path>,
) -> String {
    let dir = resolve_dir(base_dir);
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    if let Some(area) = repo.and_then(|r| r.package_area_for_dir(&dir)) {
        if area == "root" {
            // Root-level packages have no real package area directory
            String::new()
        } else {
            repo.unwrap().root.join(area).display().to_string()
        }
    } else {
        String::new()
    }
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

    let has_dirty = git
        .status
        .dirty
        .iter()
        .map(|d| d.filepath.to_str().unwrap_or(""))
        .chain(
            git.status
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
    sniff::filesystem::blast_radius::is_source_code_path(Path::new(path))
}

/// Pure helper: returns `(has_changes, count, area_name)` for the current
/// package area, or `None` when the area cannot be resolved.
///
/// JSON consumers only need the boolean. The text/verbose path uses the count
/// and area name to print a human-readable summary.
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

    let count = git
        .status
        .dirty
        .iter()
        .map(|d| d.filepath.to_str().unwrap_or(""))
        .chain(
            git.status
                .untracked
                .iter()
                .map(|u| u.filepath.to_str().unwrap_or("")),
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

pub fn render_repo_section(
    repo: &sniff::filesystem::repo::RepoInfo,
    verbose: u8,
    _repo_root: Option<&Path>,
    repo_filter: &[String],
    latest_versions_requested: bool,
) -> String {
    let mut out = String::new();
    let terminal = Terminal::default();

    if !repo.is_monorepo {
        let title = Prose::new("<b><u>Repository</u></b>");
        writeln!(out, "\n{}\n", title.render(&terminal)).unwrap();
        let items = vec![
            Prose::new("<b>Type:</b> Single-package").render(&terminal),
            Prose::new(format!("<b>Root:</b> {}", repo.root.display())).render(&terminal),
        ];
        let list = UnorderedList::new(items);
        writeln!(out, "{}", list.render(&terminal)).unwrap();

        if let Some(summary) = summarize_repo_updates(repo, None, latest_versions_requested) {
            write!(
                out,
                "{}",
                render_update_summary(&summary, latest_versions_requested, verbose, &terminal)
            )
            .unwrap();
        }
        return out;
    }

    // Monorepo heading
    let tool_name = repo
        .monorepo_tool
        .as_ref()
        .map(format_monorepo_tool)
        .unwrap_or("Unknown");

    let total_count = repo.packages.as_ref().map(|p| p.len()).unwrap_or(0);

    if let Some(ref packages) = repo.packages {
        let filtered = filter_packages(packages, repo_filter);
        let showing_count = filtered.len();

        let title_suffix = if !repo_filter.is_empty() && showing_count != total_count {
            format!(
                " <dim>({} / showing {} of {} packages)</dim>",
                tool_name, showing_count, total_count,
            )
        } else {
            format!(" <dim>({} / {} packages)</dim>", tool_name, total_count)
        };

        let title = Prose::new(format!("<b><u>Repository</u></b>{}", title_suffix));
        writeln!(out, "\n{}\n", title.render(&terminal)).unwrap();

        let update_summary =
            summarize_repo_updates(repo, Some(filtered.as_slice()), latest_versions_requested);

        // Track whether any package has updatable deps or excluded packages for the key
        let has_updatable = filtered.iter().any(|pkg| pkg.is_updatable == Some(true));
        let has_excluded = filtered.iter().any(|pkg| pkg.is_excluded);

        // Group packages by area, preserving discovery order
        let mut areas: Vec<String> = Vec::new();
        let mut area_packages: std::collections::HashMap<String, Vec<&Package>> =
            std::collections::HashMap::new();
        for pkg in &filtered {
            let area = pkg.package_area.clone();
            if !area_packages.contains_key(&area) {
                areas.push(area.clone());
            }
            area_packages.entry(area).or_default().push(*pkg);
        }
        let (top_areas, area_children) = build_area_hierarchy(&areas);

        let mut outer_items: Vec<RenderableContent> = Vec::new();
        for area in &top_areas {
            append_area_section(
                &mut outer_items,
                area,
                &area_packages,
                &area_children,
                verbose,
            );
        }

        let list = UnorderedList::from(outer_items).with_indent_children(Some(4));
        writeln!(out, "{}", list.render(&terminal)).unwrap();

        if let Some(summary) = update_summary {
            write!(
                out,
                "{}",
                render_update_summary(&summary, latest_versions_requested, verbose, &terminal)
            )
            .unwrap();
        }

        // Legend for indicators
        if has_updatable || has_excluded {
            let mut legend = String::from("\n<dim>");
            if has_updatable {
                legend.push_str("<yellow>*</yellow> dependency updates available");
                let has_major = filtered
                    .iter()
                    .any(|pkg| pkg.has_major_update == Some(true));
                if has_major {
                    legend.push_str("  <red>*</red> major version update available");
                }
                if has_excluded {
                    legend.push_str("  ");
                }
            }
            if has_excluded {
                legend.push_str(
                    "packages in <orange>orange</orange> are excluded from the workspace",
                );
            }
            legend.push_str("</dim>");
            writeln!(out, "{}", Prose::new(&legend).render(&terminal)).unwrap();
        }
    } else {
        let title = Prose::new(format!(
            "<b><u>Repository</u></b> <dim>({} / {} packages)</dim>",
            tool_name, total_count,
        ));
        writeln!(out, "\n{}\n", title.render(&terminal)).unwrap();
    }

    out
}

/// Categorize a language name for display styling.
fn render_language_name(
    name: &impl std::fmt::Display,
    is_primary: bool,
    term: &Terminal,
) -> String {
    let markup = if is_primary {
        format!("<b>{}</b>", name)
    } else {
        name.to_string()
    };
    Prose::new(&markup).render(term)
}

fn render_language_usage(
    lang: &sniff::filesystem::languages::LanguageStats,
    term: &Terminal,
) -> String {
    Prose::new(format!(
        "{} direct, {} framework ({:.1}%)",
        format_number(lang.direct_file_count),
        format_number(lang.framework_file_count),
        lang.percentage
    ))
    .render(term)
}

fn render_framework_summary(frameworks: &[FrameworkStats]) -> String {
    frameworks
        .iter()
        .map(|framework| format!("{} ({})", framework.framework, framework.file_count))
        .collect::<Vec<_>>()
        .join(", ")
}

enum LanguageContext<'a> {
    MonorepoRoot(&'a RepoInfo),
    PackageArea(&'a RepoInfo, &'a str),
    Package(&'a RepoInfo, &'a Package),
    NonMonorepo,
}

fn resolve_language_context<'a>(
    result: &'a sniff::SniffResult,
    base_dir: Option<&Path>,
) -> LanguageContext<'a> {
    let dir = resolve_dir(base_dir);
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    let Some(repo) = repo else {
        return LanguageContext::NonMonorepo;
    };

    if !repo.is_monorepo {
        return LanguageContext::NonMonorepo;
    }

    if let Some(pkg) = repo.package_for_dir(&dir) {
        return LanguageContext::Package(repo, pkg);
    }

    if let Some(area) = repo.package_area_for_dir(&dir) {
        return LanguageContext::PackageArea(repo, area);
    }

    LanguageContext::MonorepoRoot(repo)
}

fn render_language_table_for_package(pkg: &Package, verbose: u8, term: &Terminal) -> String {
    use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
    use biscuit_terminal::utils::layout::{Alignment, Margin};

    let mut out = String::new();

    if pkg.languages.is_empty() {
        let msg = Prose::new("<dim><i>No language data available</i></dim>").render(term);
        writeln!(out, "{}", msg).unwrap();
        return out;
    }

    if let Some(primary) = pkg.primary_language {
        let line = Prose::new(format!(
            "<b>{}</b> is the primary language in this package",
            primary
        ))
        .render(term);
        writeln!(out, "{}", line).unwrap();
    }
    if !pkg.secondary_languages.is_empty() {
        let line = Prose::new(format!(
            "Secondary languages: {}",
            pkg.secondary_languages
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ))
        .render(term);
        writeln!(out, "{}", line).unwrap();
    }

    let mut table = Table::new()
        .with_columns(vec![
            TableColumn::new("Language").with_min_width(16),
            TableColumn::new("Usage").with_alignment(Alignment::Right),
            TableColumn::new("Signal").with_alignment(Alignment::Right),
        ])
        .prefer_cursor_alignment();

    table.layout_mut().left_margin = Margin::Chars(1);
    table.layout_mut().top_margin = Margin::Chars(1);
    table.layout_mut().bottom_margin = Margin::Chars(1);

    for lang in &pkg.languages {
        table.add_row(vec![
            TableCellContent::Text(render_language_name(
                &lang.language,
                pkg.primary_language == Some(lang.language),
                term,
            )),
            TableCellContent::Text(render_language_usage(lang, term)),
            TableCellContent::Text(Prose::new(format!("{:.2}", lang.signal)).render(term)),
        ]);
    }

    writeln!(out).unwrap();
    write!(out, "{}", table.display(term)).unwrap();
    writeln!(out).unwrap();

    if verbose > 0 && !pkg.frameworks.is_empty() {
        writeln!(
            out,
            "Frameworks: {}",
            render_framework_summary(&pkg.frameworks)
        )
        .unwrap();
    }

    out
}

fn repo_display_name(result: &sniff::SniffResult) -> String {
    result
        .filesystem
        .as_ref()
        .and_then(|fs| fs.git.as_ref())
        .and_then(|git| git.repo.clone())
        .or_else(|| {
            result
                .filesystem
                .as_ref()
                .and_then(|fs| fs.git.as_ref())
                .map(|git| {
                    git.repo_root
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                })
        })
        .unwrap_or_default()
}

pub fn render_language_section(
    result: &sniff::SniffResult,
    verbose: u8,
    base_dir: Option<&Path>,
) -> String {
    use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
    use biscuit_terminal::utils::layout::{Alignment, Margin};

    let mut out = String::new();
    let term = Terminal::default();
    let ctx = resolve_language_context(result, base_dir);

    match ctx {
        LanguageContext::MonorepoRoot(repo) => {
            let packages = match repo.packages.as_ref() {
                Some(p) if !p.is_empty() => p,
                _ => return out,
            };
            let repo_name = repo_display_name(result);
            let header = Prose::new(format!(
                "In the <yellow>{}</yellow> monorepo, {} packages defined:",
                repo_name,
                format_number(packages.len())
            ))
            .render(&term);
            writeln!(out, "{}", header).unwrap();

            let mut items: Vec<RenderableContent> = Vec::new();
            for pkg in packages {
                let title = Prose::new(format!("<b>{}</b>", pkg.name)).render(&term);
                let body = render_language_table_for_package(pkg, verbose, &term);
                items.push(RenderableContent::String(title));
                items.push(RenderableContent::String(body));
            }

            let list = UnorderedList::from(items).with_indent_children(Some(4));
            write!(out, "{}", list.render(&term)).unwrap();
        }
        LanguageContext::PackageArea(repo, area) => {
            let packages = match repo.packages.as_ref() {
                Some(p) if !p.is_empty() => p,
                _ => return out,
            };
            let area_packages: Vec<&Package> =
                packages.iter().filter(|p| p.package_area == area).collect();
            let repo_name = repo_display_name(result);
            let header = Prose::new(format!(
                "In the <yellow>{}</yellow> monorepo's \"{}\" package area, {} packages defined:",
                repo_name,
                area,
                format_number(area_packages.len())
            ))
            .render(&term);
            writeln!(out, "{}", header).unwrap();

            let mut items: Vec<RenderableContent> = Vec::new();
            for pkg in &area_packages {
                let title = Prose::new(format!("<b>{}</b>", pkg.name)).render(&term);
                let body = render_language_table_for_package(pkg, verbose, &term);
                items.push(RenderableContent::String(title));
                items.push(RenderableContent::String(body));
            }

            let list = UnorderedList::from(items).with_indent_children(Some(4));
            write!(out, "{}", list.render(&term)).unwrap();
        }
        LanguageContext::Package(repo, pkg) => {
            let total = repo.packages.as_ref().map(|p| p.len()).unwrap_or(1);
            let repo_name = repo_display_name(result);
            let header = Prose::new(format!(
                "In the <yellow>{}</yellow> monorepo's <b>{}</b> package (<dim><i>1 of {} packages</i></dim>):",
                repo_name,
                pkg.name,
                format_number(total)
            ))
            .render(&term);
            writeln!(out, "{}", header).unwrap();

            let body = render_language_table_for_package(pkg, verbose, &term);
            let items = vec![RenderableContent::String(body)];
            let list = UnorderedList::from(items).with_indent_children(Some(4));
            write!(out, "{}", list.render(&term)).unwrap();
        }
        LanguageContext::NonMonorepo => {
            let langs = match result
                .filesystem
                .as_ref()
                .and_then(|fs| fs.languages.as_ref())
            {
                Some(l) => l,
                None => return out,
            };

            let repo_name = repo_display_name(result);
            let header = if repo_name.is_empty() {
                Prose::new(
                    "Evaluating programming languages:".to_string(),
                )
            } else {
                Prose::new(format!(
                    "Evaluating the <yellow>{}</yellow> (<dim><i>a non-monorepo repo</i></dim>) programming languages:",
                    repo_name
                ))
            }
            .render(&term);
            writeln!(out, "{}", header).unwrap();

            let primary = langs.primary;
            if let Some(primary) = primary {
                let line = Prose::new(format!(
                    "<b>{}</b> is the primary language in this repo",
                    primary
                ))
                .render(&term);
                let items = vec![RenderableContent::String(line)];
                let list = UnorderedList::from(items).with_indent_children(Some(4));
                write!(out, "{}", list.render(&term)).unwrap();
            }

            let mut table = Table::new()
                .with_columns(vec![
                    TableColumn::new("Language").with_min_width(16),
                    TableColumn::new("Usage").with_alignment(Alignment::Right),
                    TableColumn::new("Signal").with_alignment(Alignment::Right),
                ])
                .prefer_cursor_alignment();

            table.layout_mut().left_margin = Margin::Chars(1);
            table.layout_mut().top_margin = Margin::Chars(1);
            table.layout_mut().bottom_margin = Margin::Chars(1);

            for lang in &langs.languages {
                table.add_row(vec![
                    TableCellContent::Text(render_language_name(
                        &lang.language,
                        Some(lang.language) == primary,
                        &term,
                    )),
                    TableCellContent::Text(render_language_usage(lang, &term)),
                    TableCellContent::Text(Prose::new(format!("{:.2}", lang.signal)).render(&term)),
                ]);
            }

            writeln!(out).unwrap();
            write!(out, "{}", table.display(&term)).unwrap();
            writeln!(out).unwrap();

            if verbose > 0 && !langs.frameworks.is_empty() {
                writeln!(
                    out,
                    "Frameworks: {}",
                    render_framework_summary(&langs.frameworks)
                )
                .unwrap();
            }

            let footer = Prose::new(format!(
                "<dim><i>- analyzed </i></dim><yellow>{}</yellow><dim><i> files, </i></dim><yellow>{}</yellow><dim><i> contributed to language selection</i></dim>",
                format_number(langs.total_files_scanned),
                format_number(langs.total_language_files)
            ))
            .render(&term);
            writeln!(out, "{}", footer).unwrap();
        }
    }

    out
}

pub(crate) fn filter_file_breakdown(
    breakdown: &FileAssociationBreakdown,
    filter: &FilesFilter,
) -> FileAssociationBreakdown {
    let Some(association) = filter.association else {
        return breakdown.clone();
    };

    let by_association: Vec<FileAssociationStats> = breakdown
        .by_association
        .iter()
        .filter(|stats| stats.association == association)
        .cloned()
        .collect();
    let total_files = by_association.iter().map(|stats| stats.file_count).sum();

    FileAssociationBreakdown {
        total_files,
        by_association,
        by_language: if association == sniff::filesystem::FileAssociation::ProgrammingLanguage {
            breakdown.by_language.clone()
        } else {
            Vec::new()
        },
        by_framework: if association == sniff::filesystem::FileAssociation::FrameworkFile {
            breakdown.by_framework.clone()
        } else {
            Vec::new()
        },
    }
}

pub fn render_files_section(
    files: &FileAssociationBreakdown,
    verbose: u8,
    filter: &FilesFilter,
) -> String {
    use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
    use biscuit_terminal::utils::layout::{Alignment, Margin};

    let mut out = String::new();
    let filtered = filter_file_breakdown(files, filter);
    let term = Terminal::default();

    let mut table = Table::new()
        .with_columns(vec![
            TableColumn::new("Association").with_min_width(18),
            TableColumn::new("Count").with_alignment(Alignment::Right),
        ])
        .prefer_cursor_alignment();
    table.layout_mut().left_margin = Margin::Chars(1);
    table.layout_mut().top_margin = Margin::Chars(1);
    table.layout_mut().bottom_margin = Margin::Chars(1);

    for stats in &filtered.by_association {
        table.add_row(vec![
            TableCellContent::Text(stats.association.to_string()),
            TableCellContent::Text(format!("{} ({:.1}%)", stats.file_count, stats.percentage)),
        ]);
    }

    writeln!(out).unwrap();
    write!(out, "{}", table.display(&term)).unwrap();
    writeln!(out).unwrap();

    if verbose > 0 && !filtered.by_framework.is_empty() {
        writeln!(
            out,
            "Frameworks: {}",
            render_framework_summary(&filtered.by_framework)
        )
        .unwrap();
    }
    if verbose > 0 && !filtered.by_language.is_empty() {
        writeln!(
            out,
            "Languages: {}",
            filtered
                .by_language
                .iter()
                .map(|language| format!("{} ({})", language.language, language.total_file_count))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .unwrap();
    }

    out
}

pub fn render_filesystem_section(
    fs: &sniff::FilesystemInfo,
    verbose: u8,
    repo_root: Option<&Path>,
    latest_versions_requested: bool,
) -> String {
    let mut out = String::new();
    writeln!(out, "=== Filesystem ===").unwrap();

    // Print EditorConfig formatting info at verbose level 2+
    if verbose > 1
        && let Some(ref formatting) = fs.formatting
    {
        writeln!(out, "EditorConfig: {}", formatting.config_path.display()).unwrap();
        for section in &formatting.sections {
            writeln!(out, "  [{}]", section.pattern).unwrap();
            if let Some(style) = &section.indent_style {
                writeln!(out, "    indent_style: {}", style).unwrap();
            }
            if let Some(size) = section.indent_size {
                writeln!(out, "    indent_size: {}", size).unwrap();
            }
        }
        writeln!(out).unwrap();
    }

    if let Some(ref langs) = fs.languages {
        writeln!(
            out,
            "Languages ({} contributing files out of {} scanned):",
            format_number(langs.total_language_files),
            format_number(langs.total_files_scanned)
        )
        .unwrap();
        if let Some(ref primary) = langs.primary {
            writeln!(out, "  Primary: {}", primary).unwrap();
        }
        if !langs.secondary.is_empty() {
            writeln!(
                out,
                "  Secondary: {}",
                langs
                    .secondary
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
            .unwrap();
        }
        let show_count = if verbose > 0 { 10 } else { 5 };
        for lang in langs.languages.iter().take(show_count) {
            writeln!(
                out,
                "  {}: {} direct, {} framework ({:.1}%)",
                lang.language,
                format_number(lang.direct_file_count),
                format_number(lang.framework_file_count),
                lang.percentage
            )
            .unwrap();
            if verbose > 1 && !lang.direct_files.is_empty() {
                let file_show_count = 3.min(lang.direct_files.len());
                for file in lang.direct_files.iter().take(file_show_count) {
                    writeln!(out, "    - {}", file.display()).unwrap();
                }
                if lang.direct_files.len() > file_show_count {
                    writeln!(
                        out,
                        "    ... and {} more files",
                        lang.direct_files.len() - file_show_count
                    )
                    .unwrap();
                }
            }
        }
        if langs.languages.len() > show_count {
            writeln!(out, "  ... and {} more", langs.languages.len() - show_count).unwrap();
        }
        if verbose > 0 && !langs.frameworks.is_empty() {
            writeln!(
                out,
                "  Frameworks: {}",
                render_framework_summary(&langs.frameworks)
            )
            .unwrap();
        }
    }
    if let Some(ref files) = fs.files {
        writeln!(out, "Files ({} scanned):", format_number(files.total_files)).unwrap();
        let show_count = if verbose > 0 { 10 } else { 6 };
        for stats in files.by_association.iter().take(show_count) {
            writeln!(
                out,
                "  {}: {} files ({:.1}%)",
                stats.association,
                format_number(stats.file_count),
                stats.percentage
            )
            .unwrap();
        }
    }
    writeln!(out).unwrap();

    if let Some(ref git) = fs.git {
        writeln!(out, "Git Repository:").unwrap();
        let root_str = relative_path(&git.repo_root, repo_root);
        writeln!(
            out,
            "  Root: {}",
            if root_str.is_empty() {
                ".".to_string()
            } else {
                root_str
            }
        )
        .unwrap();
        if let Some(ref branch) = git.current_branch {
            writeln!(out, "  Branch: {}", branch).unwrap();
        }

        // Show in_worktree indicator when true
        if git.in_worktree {
            writeln!(out, "  In Worktree: yes").unwrap();
        }

        // Show HEAD commit (first recent commit)
        if let Some(commit) = git.recent.first() {
            writeln!(out, "  HEAD: {} ({})", &commit.sha[..8], commit.author).unwrap();
            writeln!(
                out,
                "  Message: {}",
                commit.message.lines().next().unwrap_or("")
            )
            .unwrap();
            // Show which remotes have this commit (deep mode)
            if let Some(ref remotes) = commit.remotes {
                writeln!(out, "    Synced to: {}", remotes.join(", ")).unwrap();
            }
        }

        let dirty = if git.status.is_dirty {
            "dirty"
        } else {
            "clean"
        };
        writeln!(
            out,
            "  Status: {} ({} staged, {} unstaged, {} untracked)",
            dirty, git.status.staged_count, git.status.unstaged_count, git.status.untracked_count
        )
        .unwrap();

        // Show is_behind status (deep mode only)
        if let Some(ref behind) = git.status.is_behind {
            match behind {
                BehindStatus::NotBehind => writeln!(out, "  Behind: no").unwrap(),
                BehindStatus::Behind(remotes) => {
                    writeln!(out, "  Behind: {}", remotes.join(", ")).unwrap();
                }
            }
        }

        // Show more recent commits at verbose level 1+
        if verbose > 0 && git.recent.len() > 1 {
            writeln!(out, "  Recent commits:").unwrap();
            for commit in git.recent.iter().skip(1).take(5) {
                let short_msg = commit.message.lines().next().unwrap_or("");
                let truncated = if short_msg.len() > 50 {
                    format!("{}...", &short_msg[..47])
                } else {
                    short_msg.to_string()
                };
                write!(out, "    {} - {}", &commit.sha[..8], truncated).unwrap();
                // Show commit remotes at verbose level 2+ with deep
                if verbose > 1
                    && let Some(ref remotes) = commit.remotes
                {
                    write!(out, " [{}]", remotes.join(", ")).unwrap();
                }
                writeln!(out).unwrap();
            }
            if git.recent.len() > 6 {
                writeln!(out, "    ... and {} more", git.recent.len() - 6).unwrap();
            }
        }

        // Show dirty file details at verbose level 1+
        if verbose > 0 && !git.status.dirty.is_empty() {
            writeln!(out, "  Dirty files:").unwrap();
            for dirty_file in &git.status.dirty {
                writeln!(out, "    - {}", dirty_file.filepath.display()).unwrap();
                // Show diff at verbose level 2+
                if verbose > 1 && !dirty_file.diff.is_empty() {
                    for line in dirty_file.diff.lines().take(5) {
                        writeln!(out, "      {}", line).unwrap();
                    }
                    let line_count = dirty_file.diff.lines().count();
                    if line_count > 5 {
                        writeln!(out, "      ... ({} more lines)", line_count - 5).unwrap();
                    }
                }
            }
        }

        // Show untracked files at verbose level 1+
        if verbose > 0 && !git.status.untracked.is_empty() {
            writeln!(out, "  Untracked files:").unwrap();
            let show_count = 5.min(git.status.untracked.len());
            for untracked in git.status.untracked.iter().take(show_count) {
                writeln!(out, "    - {}", untracked.filepath.display()).unwrap();
            }
            if git.status.untracked.len() > show_count {
                writeln!(
                    out,
                    "    ... and {} more",
                    git.status.untracked.len() - show_count
                )
                .unwrap();
            }
        }

        // Show worktrees at verbose level 1+
        if verbose > 0 && !git.worktrees.is_empty() {
            writeln!(out, "  Worktrees:").unwrap();
            for (branch, info) in &git.worktrees {
                let dirty_indicator = if info.dirty { " (dirty)" } else { "" };
                writeln!(
                    out,
                    "    {} @ {}{}",
                    branch,
                    &info.sha[..8],
                    dirty_indicator
                )
                .unwrap();
                if verbose > 1 {
                    writeln!(out, "      Path: {}", info.filepath.display()).unwrap();
                }
            }
        }

        // Show remotes with enhanced branch info
        for remote in &git.remotes {
            write!(out, "  Remote {}: {:?}", remote.name, remote.provider).unwrap();
            if let Some(ref default_branch) = remote.default_branch {
                write!(out, " (default: {})", default_branch).unwrap();
            }
            // Show branch count in deep mode
            if let Some(ref branches) = remote.branches {
                write!(out, " ({} branches)", branches.len()).unwrap();
            }
            writeln!(out).unwrap();
            // Show branches at verbose level 2+ with deep
            if verbose > 1
                && let Some(ref branches) = remote.branches
            {
                let show_count = 5.min(branches.len());
                for branch in branches.iter().take(show_count) {
                    writeln!(out, "    - {}", branch).unwrap();
                }
                if branches.len() > show_count {
                    writeln!(out, "    ... and {} more", branches.len() - show_count).unwrap();
                }
            }
        }
    }
    writeln!(out).unwrap();

    if let Some(ref repo) = fs.repo {
        let filtered_packages_storage = repo
            .packages
            .as_ref()
            .map(|packages| packages.iter().collect::<Vec<_>>());
        let update_summary = summarize_repo_updates(
            repo,
            filtered_packages_storage.as_deref(),
            latest_versions_requested,
        );

        if !repo.is_monorepo {
            writeln!(out, "Repository:").unwrap();
            writeln!(out, "  Type: Single-package").unwrap();
            writeln!(out, "  Root: {}", relative_path(&repo.root, repo_root)).unwrap();
            if let Some(summary) = update_summary {
                let terminal = Terminal::default();
                write!(
                    out,
                    "{}",
                    render_update_summary(&summary, latest_versions_requested, verbose, &terminal)
                )
                .unwrap();
            }
            return out;
        }

        let tool_name = repo
            .monorepo_tool
            .as_ref()
            .map(format_monorepo_tool)
            .unwrap_or("Unknown");
        let pkg_count = repo.packages.as_ref().map(|p| p.len()).unwrap_or(0);

        let header = Prose::new(format!(
            "<b>Packages:</b> <dim>({} / {} packages)</dim>",
            tool_name, pkg_count,
        ));
        writeln!(out, "{}", header.render_optimistic(None)).unwrap();

        if let Some(ref packages) = repo.packages {
            let mut items: Vec<RenderableContent> = Vec::new();
            for pkg in packages {
                let package_items = format_package_items(pkg, verbose);
                items.push(RenderableContent::String(
                    Prose::new(&package_items[0]).render_optimistic(None),
                ));
                if package_items.len() > 1 {
                    let detail_items = package_items[1..]
                        .iter()
                        .map(|item| Prose::new(item).render_optimistic(None))
                        .collect::<Vec<_>>();
                    let detail_list = UnorderedList::new(detail_items).with_bullet("  ");
                    items.push(RenderableContent::Component(Rc::new(detail_list)));
                }
            }

            let list = UnorderedList::from(items);
            writeln!(out, "{}", list.render_optimistic(None)).unwrap();
        }

        if let Some(summary) = update_summary {
            let terminal = Terminal::default();
            write!(
                out,
                "{}",
                render_update_summary(&summary, latest_versions_requested, verbose, &terminal)
            )
            .unwrap();
        }

        let has_updatable = repo
            .packages
            .as_ref()
            .is_some_and(|packages| packages.iter().any(|pkg| pkg.is_updatable == Some(true)));
        if has_updatable {
            writeln!(
                out,
                "{}",
                Prose::new(
                    "<dim><yellow>*</yellow> dependency updates available  <red>*</red> major version update available</dim>"
                )
                .render_optimistic(None)
            ).unwrap();
        }
    }

    out
}

/// Render markdown documents with split stdout/stderr output.
///
/// - **stderr**: header line + footer (when not verbose)
/// - **stdout**: document list
pub fn render_docs_output(docs: &[MarkdownMeta], verbose: u8) -> TextOutput {
    let terminal = Terminal::default();
    let prompt_count = docs.iter().filter(|d| d.prompt.is_some()).count();

    // --- stderr: header ---
    let mut stderr = String::new();
    let header = if prompt_count > 0 {
        format!(
            "<b>Docs</b> <dim>({} documents, {} with prompts)</dim>",
            docs.len(),
            prompt_count
        )
    } else {
        format!("<b>Docs</b> <dim>({} documents)</dim>", docs.len())
    };
    writeln!(stderr, "\n{}\n", Prose::new(&header).render(&terminal)).unwrap();

    // --- stdout: document list ---
    let mut stdout = String::new();

    let items: Vec<RenderableContent> = docs
        .iter()
        .flat_map(|doc| {
            let file_link =
                format_styled_filepath(&doc.relative, &doc.filepath.display().to_string());
            let main = Prose::new(&file_link).render(&terminal);
            let mut result = vec![RenderableContent::String(main)];

            if verbose > 0 {
                let mut details: Vec<String> = Vec::new();

                // title
                let title_source_label = match doc.title_source {
                    TitleSource::FrontmatterTitle => "title property",
                    TitleSource::H1Heading => "H1 heading",
                    TitleSource::H2Heading => "H2 heading",
                    TitleSource::H3Heading => "H3 heading",
                    TitleSource::None => "none",
                };
                if !doc.title.is_empty() {
                    let title_line = format!(
                        "<b>title:</b> {} <dim><i>(from {})</i></dim>",
                        doc.title, title_source_label
                    );
                    details.push(Prose::new(&title_line).render(&terminal));
                } else {
                    let title_line = format!(
                        "<b>title:</b> <yellow>none</yellow> <dim><i>(from {})</i></dim>",
                        title_source_label
                    );
                    details.push(Prose::new(&title_line).render(&terminal));
                }

                // updated
                let date_str = doc.last_updated.format("%Y-%m-%d").to_string();
                let updated_source_label = match doc.updated_source {
                    UpdatedSource::UpdatedProperty => "updated property",
                    UpdatedSource::FileMetadata => "file metadata",
                };
                let updated_line = format!(
                    "<b>updated:</b> {} <dim><i>(from {})</i></dim>",
                    date_str, updated_source_label
                );
                details.push(Prose::new(&updated_line).render(&terminal));

                // frontmatter properties
                if !doc.frontmatter_keys.is_empty() {
                    let props = doc.frontmatter_keys.join(", ");
                    let props_line = format!("<b>frontmatter properties:</b> <i>{props}</i>");
                    details.push(Prose::new(&props_line).render(&terminal));
                }

                let detail_list = UnorderedList::new(details).with_bullet("  ");
                result.push(RenderableContent::Component(Rc::new(detail_list)));
            }

            result
        })
        .collect();

    let list = UnorderedList::from(items);
    writeln!(stdout, "{}", list.render(&terminal)).unwrap();

    // --- stderr: footer ---
    if verbose == 0 {
        writeln!(
            stderr,
            "{}",
            Prose::new(
                "<dim>Use <blue>--verbose</blue> / <blue>-v</blue> to include metadata for documents</dim>"
            )
            .render(&terminal)
        )
        .unwrap();
    }

    TextOutput { stdout, stderr }
}

/// Format a filepath with dim directory and bold filename,
/// wrapped in an OSC8 hyperlink.
fn format_styled_filepath(relative: &str, absolute: &str) -> String {
    match relative.rsplit_once('/') {
        Some((dir, file)) => {
            format!("<a href=\"{absolute}\"><blue><dim>{dir}/</dim><b>{file}</b></blue></a>")
        }
        None => {
            format!("<a href=\"{absolute}\"><blue><b>{relative}</b></blue></a>")
        }
    }
}

/// Format a filepath showing only the basename, with an OSC8 hyperlink.
fn format_basename_filepath(relative: &str, absolute: &str) -> String {
    let basename = relative.rsplit_once('/').map_or(relative, |(_, f)| f);
    format!("<a href=\"{absolute}\"><blue>{basename}</blue></a>")
}

/// Format a git-status filepath preserving the existing visible text while
/// making the path clickable through Prose OSC8 support.
fn format_git_status_filepath(relative: &str, absolute: &str) -> String {
    match relative.rsplit_once('/') {
        Some((dir, file)) => format!("<a href=\"{absolute}\">{dir}/<b>{file}</b></a>"),
        None => format!("<a href=\"{absolute}\"><b>{relative}</b></a>"),
    }
}

// ---------------------------------------------------------------------------
// Shared path-list renderer
// ---------------------------------------------------------------------------

/// Output format for path lists.
pub enum PathListFormat {
    /// One path per line (default).
    Lines,
    /// Bullet list with `- ` prefix.
    BulletList,
    /// Comma-separated on a single line.
    Csv,
}

/// Render a list of repo-relative paths in the chosen format.
///
/// Paths are displayed with OSC8 hyperlinks (absolute target), dim directory
/// segments, and bold basenames. With `no_path`, only the basename is shown.
pub fn render_path_list(
    repo_root: &Path,
    paths: &[PathBuf],
    format: PathListFormat,
    no_path: bool,
) -> String {
    let terminal = Terminal::default();

    let format_one = |p: &PathBuf| -> String {
        let relative = p.display().to_string();
        let absolute = repo_root.join(p).display().to_string();
        let markup = if no_path {
            format_basename_filepath(&relative, &absolute)
        } else {
            format_styled_filepath(&relative, &absolute)
        };
        Prose::new(&markup).render(&terminal)
    };

    match format {
        PathListFormat::Lines => {
            let mut out = String::new();
            for p in paths {
                writeln!(out, "{}", format_one(p)).unwrap();
            }
            out
        }
        PathListFormat::BulletList => {
            let items: Vec<String> = paths.iter().map(format_one).collect();
            let list = UnorderedList::new(items);
            let mut out = String::new();
            writeln!(out, "{}", list.render(&terminal)).unwrap();
            out
        }
        PathListFormat::Csv => {
            let items: Vec<String> = paths.iter().map(format_one).collect();
            let mut out = items.join(", ");
            out.push('\n');
            out
        }
    }
}

/// Build a Mermaid flowchart from workspace package dependencies.
///
/// Packages are grouped into subgraphs by `package_area`. Edges are drawn
/// from each package to its `depends_on` entries.
///
/// Returns `None` when there are no internal dependency edges.
fn build_deps_mermaid(packages: &[sniff::filesystem::repo::Package]) -> Option<String> {
    use std::collections::HashMap;

    // Assign each package a stable node ID and build name→id lookup
    let mut node_ids: HashMap<&str, String> = HashMap::new();
    for (i, pkg) in packages.iter().enumerate() {
        node_ids.insert(pkg.name.as_str(), format!("n{i}"));
    }

    // Check if there are any edges at all
    let has_edges = packages.iter().any(|p| !p.depends_on.is_empty());
    if !has_edges {
        return None;
    }

    let mut lines = vec!["flowchart TD".to_string()];

    // Group packages by area, preserving discovery order
    let mut areas: Vec<&str> = Vec::new();
    let mut area_packages: HashMap<&str, Vec<&sniff::filesystem::repo::Package>> = HashMap::new();
    for pkg in packages {
        let area = pkg.package_area.as_str();
        if !area_packages.contains_key(area) {
            areas.push(area);
        }
        area_packages.entry(area).or_default().push(pkg);
    }

    // Emit subgraphs
    for area in &areas {
        let pkgs = &area_packages[area];
        if pkgs.len() == 1 && *area == "root" {
            // Single root-level package doesn't need a subgraph
            let pkg = pkgs[0];
            let id = &node_ids[pkg.name.as_str()];
            lines.push(format!("    {id}[\"{}\"]", pkg.name));
        } else {
            lines.push(format!("    subgraph {area}"));
            for pkg in pkgs {
                let id = &node_ids[pkg.name.as_str()];
                lines.push(format!("        {id}[\"{}\"]", pkg.name));
            }
            lines.push("    end".to_string());
        }
    }

    // Emit edges
    for pkg in packages {
        let from = &node_ids[pkg.name.as_str()];
        for dep_name in &pkg.depends_on {
            if let Some(to) = node_ids.get(dep_name.as_str()) {
                lines.push(format!("    {from} --> {to}"));
            }
        }
    }

    Some(lines.join("\n"))
}

/// Render an internal dependency diagram for the repository as a Mermaid image.
///
/// Builds a Mermaid flowchart from package dependency data and renders it
/// inline using `MermaidRenderer`. Falls back to a code block if the
/// terminal cannot display images or mmdc is not available.
pub fn render_repo_deps_visual(
    repo: &sniff::filesystem::repo::RepoInfo,
    repo_filter: &[String],
) -> String {
    if !repo.is_monorepo {
        return String::from("deps requires a monorepo (no workspace packages found)");
    }

    let packages = match repo.packages {
        Some(ref pkgs) => pkgs,
        None => {
            return String::from("No packages found in workspace");
        }
    };

    let filtered: Vec<Package> = filter_packages(packages, repo_filter)
        .into_iter()
        .cloned()
        .collect();

    let mermaid = match build_deps_mermaid(&filtered) {
        Some(m) => m,
        None => {
            return String::from("No internal dependencies found between workspace packages");
        }
    };

    let diagram = MermaidDiagram::new(&mermaid);
    let term = Terminal::default();
    diagram.render(&term)
}

/// Render an internal dependency list for the repository as styled text.
///
/// Each package with dependencies or dependents is shown as a top-level item
/// with `depends-on` and `used-by` sub-items. Isolates (packages with neither)
/// are omitted unless an explicit filter is set.
pub fn render_repo_deps_text(
    repo: &sniff::filesystem::repo::RepoInfo,
    repo_filter: &[String],
) -> String {
    let mut out = String::new();

    if !repo.is_monorepo {
        return String::from("deps requires a monorepo (no workspace packages found)");
    }

    let packages = match repo.packages {
        Some(ref pkgs) => pkgs,
        None => {
            return String::from("No packages found in workspace");
        }
    };

    let filtered = filter_packages(packages, repo_filter);
    let has_explicit_filter = !repo_filter.is_empty();

    // Collect only packages that participate in dependency relationships
    // (unless an explicit filter is set, in which case show all matched)
    let relevant: Vec<&&Package> = filtered
        .iter()
        .filter(|pkg| has_explicit_filter || !pkg.depends_on.is_empty() || !pkg.used_by.is_empty())
        .collect();

    if relevant.is_empty() {
        return String::from("No internal dependencies found between workspace packages");
    }

    let title = if has_explicit_filter {
        format!(
            "<b><u>Dependencies</u></b> <dim>(showing {} of {} packages)</dim>",
            filtered.len(),
            packages.len(),
        )
    } else {
        format!(
            "<b><u>Dependencies</u></b> <dim>({} packages with dependencies)</dim>",
            relevant.len(),
        )
    };
    let term = Terminal::default();
    writeln!(out, "\n{}\n", Prose::new(&title).render(&term)).unwrap();

    let mut outer_items: Vec<RenderableContent> = Vec::new();
    for pkg in &relevant {
        let label = Prose::new(format!("<b><blue>{}</blue></b>", pkg.name)).render(&term);
        outer_items.push(RenderableContent::String(label));

        let mut detail_items: Vec<String> = Vec::new();
        if !pkg.depends_on.is_empty() {
            detail_items.push(
                Prose::new(format!("<b>depends-on:</b> {}", pkg.depends_on.join(", ")))
                    .render(&term),
            );
        }
        if !pkg.used_by.is_empty() {
            detail_items.push(
                Prose::new(format!("<b>used-by:</b> {}", pkg.used_by.join(", "))).render(&term),
            );
        }

        if !detail_items.is_empty() {
            let detail_list = UnorderedList::new(detail_items).with_bullet("  ");
            outer_items.push(RenderableContent::Component(Rc::new(detail_list)));
        }
    }

    let list = UnorderedList::from(outer_items).with_indent_children(Some(4));
    write!(out, "{}", list.render(&term)).unwrap();

    writeln!(
        out,
        "\n{}",
        Prose::new(
            "<dim><i>use the <blue>--ui</blue> flag to show this in a visual format</i></dim>"
        )
        .render(&term)
    )
    .unwrap();

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sniff::filesystem::git::{
        CommitInfo, FileAction, FileChange, FileStatus, GitConfig, GitInfo, RepoStatus,
    };
    use sniff::filesystem::repo::Package;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_package(name: &str, area: &str, depends_on: &[&str]) -> Package {
        Package {
            path: PathBuf::from(format!("/repo/{area}/{name}")),
            relative: format!("{area}/{name}"),
            package_area: area.to_string(),
            name: name.to_string(),
            ecosystem: sniff::filesystem::repo::PackageEcosystem::Unknown,
            discovery_sources: vec![],
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
            status: RepoStatus {
                is_dirty: !file_changes.is_empty(),
                staged_count,
                unstaged_count,
                untracked_count,
                dirty: vec![],
                untracked: vec![],
                is_behind: None,
            },
            remotes: vec![],
            worktrees: HashMap::new(),
            config: GitConfig::default(),
            tracking: vec![],
            file_changes,
        }
    }

    mod git_status_rendering {
        use super::*;

        #[test]
        fn conflicted_files_render_before_commits_and_untracked_is_dimmed() {
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

            assert!(items[0].starts_with("<red>conflicted: <a href=\"/repo/conflict.txt\">"));
            assert!(items[0].contains("<b>conflict.txt</b></a>"));
            assert!(items.iter().any(|item| {
                item.starts_with("<dim>untracked: <a href=\"/repo/notes.md\">")
                    && item.contains("<b>notes.md</b></a>")
            }));
            assert!(items.iter().any(|item| {
                item.starts_with("<lime>staged(")
                    && item.contains("<a href=\"/repo/src/main.rs\">src/<b>main.rs</b></a>")
            }));
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

            let output = render_git_section(&git, 10, 0, true);

            assert!(output.contains("Status"));
            assert!(!output.contains("Meta"));
            assert!(!output.contains("Worktrees"));
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

            assert_eq!(super::super::package_area_root(&pkg), ".");
        }

        #[test]
        fn normal_area_returns_package_area() {
            let pkg = make_package("cli", "sniff", &[]);

            assert_eq!(super::super::package_area_root(&pkg), "sniff");
        }

        #[test]
        fn multi_segment_area_returns_full_package_area() {
            let pkg = make_package("my_package", "apps/browser", &[]);

            assert_eq!(super::super::package_area_root(&pkg), "apps/browser");
        }

        #[test]
        fn mismatched_area_falls_back_to_relative_first_component() {
            let mut pkg = make_package("pkg", "weird", &[]);
            pkg.relative = "actual/path".to_string();
            pkg.path = PathBuf::from("/repo/actual/path");

            assert_eq!(super::super::package_area_root(&pkg), "actual");
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
                monorepo_tool: None,
                workspace_tools: Vec::new(),
                root: PathBuf::from("/repo"),
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
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
            git.status.dirty = vec![make_dirty_file("alpha/src/main.rs")];
            let result = build_result(repo, git);

            let names = select_dirty_package_names(&result, &[]);
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
            git.status.dirty = vec![make_dirty_file("area-a/alpha/src/main.rs")];
            let result = build_result(repo, git);

            let names = select_dirty_package_names(&result, &[]);
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
            git.status.dirty = vec![make_dirty_file("area-a/alpha/src/main.rs")];
            let result = build_result(repo, git);

            let areas = select_dirty_package_area_names(&result, &[]);
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
            git.status.dirty = vec![
                make_dirty_file("area-a/alpha/src/main.rs"),
                make_dirty_file("area-b/beta/src/lib.rs"),
            ];
            let result = build_result(repo, git);

            let names = select_dirty_package_names(&result, &["@area-a".to_string()]);
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

            let names = select_staged_package_names(&result, &[]);
            assert_eq!(names, vec!["alpha".to_string()]);

            let areas = select_staged_package_area_names(&result, &[]);
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

            let names = select_unstaged_package_names(&result, &[]);
            assert_eq!(names, vec!["alpha".to_string()]);

            let areas = select_unstaged_package_area_names(&result, &[]);
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

            assert!(select_staged_package_names(&result, &[]).is_empty());
            assert!(select_unstaged_package_names(&result, &[]).is_empty());
            assert!(select_staged_package_area_names(&result, &[]).is_empty());
            assert!(select_unstaged_package_area_names(&result, &[]).is_empty());
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
                monorepo_tool: None,
                workspace_tools: Vec::new(),
                root: PathBuf::from("/repo"),
                dependencies: None,
                dev_dependencies: None,
                peer_dependencies: None,
                optional_dependencies: None,
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
            git.status.dirty = dirty_paths.iter().map(|p| dirty_file(p)).collect();
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

    mod deps_mermaid {
        use super::*;

        #[test]
        fn returns_none_when_no_edges() {
            let packages = vec![
                make_package("alpha", "area-a", &[]),
                make_package("beta", "area-b", &[]),
            ];
            assert!(build_deps_mermaid(&packages).is_none());
        }

        #[test]
        fn generates_flowchart_with_edges() {
            let packages = vec![
                make_package("cli", "sniff", &["lib"]),
                make_package("lib", "sniff", &[]),
            ];
            let result = build_deps_mermaid(&packages).unwrap();
            assert!(result.starts_with("flowchart TD"));
            assert!(result.contains("subgraph sniff"));
            assert!(result.contains("n0 --> n1"));
        }

        #[test]
        fn groups_packages_by_area() {
            let packages = vec![
                make_package("speaks-cli", "biscuit-speaks", &["speaks-lib"]),
                make_package("speaks-lib", "biscuit-speaks", &[]),
                make_package("sniff-cli", "sniff", &["sniff-lib"]),
                make_package("sniff-lib", "sniff", &[]),
            ];
            let result = build_deps_mermaid(&packages).unwrap();
            assert!(result.contains("subgraph biscuit-speaks"));
            assert!(result.contains("subgraph sniff"));
        }

        #[test]
        fn cross_area_edges() {
            let packages = vec![
                make_package("app", "apps", &["core"]),
                make_package("core", "libs", &[]),
            ];
            let result = build_deps_mermaid(&packages).unwrap();
            assert!(result.contains("n0 --> n1"));
        }
    }
}
