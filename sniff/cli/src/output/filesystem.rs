//! Filesystem section output formatting (Git, Repo, Languages, Docs).

use std::path::{Path, PathBuf};
use std::rc::Rc;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::mermaid::MermaidRenderer;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::docs::MarkdownMeta;
use sniff::filesystem::git::{BehindStatus, ConventionalCommit, FileStatus, RefKind};

use sniff::filesystem::repo::Package;

use super::{format_number, relative_path};

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
pub(crate) fn filter_packages<'a>(
    packages: &'a [Package],
    repo_filter: Option<&str>,
) -> Vec<&'a Package> {
    match repo_filter {
        Some(f) => {
            let filter = RepoFilter::parse(f);
            packages.iter().filter(|p| filter.matches(p)).collect()
        }
        None => packages.iter().collect(),
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
    provider: &sniff::filesystem::git::HostingProvider,
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

/// Format a single commit as a styled one-liner.
///
/// Parses conventional commit format and includes SHA, timestamp, ref decorations,
/// and optionally the author (when `verbose > 0`).
fn format_commit_line(
    commit: &sniff::filesystem::git::CommitInfo,
    verbose: u8,
) -> String {
    let cc = ConventionalCommit::parse(&commit.message);
    let (date_str, time_str, use_on) = format_commit_datetime(&commit.timestamp);
    let sha = commit.sha[0..7].to_string();
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
            "[<b>{}</b>] <b><yellow>{}</yellow></b>{} <i>at</i> <blue><b>{}</b></blue> {}<blue>{}</blue>{}{}: <dim>{}</dim>",
            sha,
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
            "[<b>{}</b>] <dim>{}</dim> {}<blue><b>{}</b></blue>{}{}",
            sha, truncated, date_prefix, date_str, refs_part, user_part,
        )
    }
}

/// Print detailed information for a single commit looked up by `--hash`.
///
/// Shows the commit as a one-liner followed by a list of files changed.
pub fn print_hash_section(
    commit: &sniff::filesystem::git::CommitInfo,
    files: &[(std::path::PathBuf, sniff::filesystem::git::DeltaKind)],
    verbose: u8,
) {
    use sniff::filesystem::git::DeltaKind;

    let terminal = Terminal::default();

    // === Commit Section ===
    let status_title = Prose::new("<b><u>Commit</u></b>");
    println!("\n{}\n", status_title.render(&terminal));

    let commit_line = format_commit_line(commit, verbose);
    let rendered = Prose::new(commit_line.as_str()).render(&terminal);
    let list = UnorderedList::new(vec![rendered]);
    println!("{}", list.render(&terminal));

    // === Files Section ===
    if files.is_empty() {
        return;
    }

    let files_title = Prose::new("<b><u>Files changed</u></b>");
    println!("{}\n", files_title.render(&terminal));

    let file_items: Vec<String> = files
        .iter()
        .map(|(path, kind)| {
            let path_str = path.display().to_string();
            let (dir, name) = split_path(&path_str);
            let dir_part = if dir.is_empty() {
                String::new()
            } else {
                dir
            };
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
    println!("{}", list.render(&terminal));
}

/// Print git information with rich terminal formatting.
///
/// Uses biscuit-terminal's Prose component for styled output with two sections:
/// - **Status**: Recent commits (with conventional commit parsing), staged/modified/untracked files
/// - **Meta**: Remote tracking status, branches, git config
///
/// ## Arguments
///
/// * `git` - Git repository information
/// * `history_count` - Number of recent commits to display
pub fn print_git_section(git: &sniff::filesystem::git::GitInfo, history_count: usize, verbose: u8) {
    let terminal = Terminal::default();

    // === Status Section ===
    let status_title = Prose::new("<b><u>Status</u></b>");
    println!("\n{}\n", status_title.render(&terminal));

    let mut status_items: Vec<String> = Vec::new();

    // Recent commits with conventional commit parsing (oldest first, so most recent is at bottom)
    let commits: Vec<_> = git.recent.iter().take(history_count).collect();
    for commit in commits.iter().rev() {
        status_items.push(format_commit_line(commit, verbose));
    }

    // File changes grouped by status
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
        let (dir, name) = split_path(&path);
        let line = if dir.is_empty() {
            format!("<lime>staged: <b>{}</b></lime>", name)
        } else {
            format!("<lime>staged: {}<b>{}</b></lime>", dir, name)
        };
        status_items.push(line);
    }

    // Add modified files
    for file in &modified {
        let path = file.path.display().to_string();
        let (dir, name) = split_path(&path);
        let line = if dir.is_empty() {
            format!("<yellow>modified: <b>{}</b></yellow>", name)
        } else {
            format!("<yellow>modified: {}<b>{}</b></yellow>", dir, name)
        };
        status_items.push(line);
    }

    // Add untracked files
    for file in &untracked {
        let path = file.path.display().to_string();
        let (dir, name) = split_path(&path);
        let line = if dir.is_empty() {
            format!("<red>untracked: <b>{}</b></red>", name)
        } else {
            format!("<red>untracked: {}<b>{}</b></red>", dir, name)
        };
        status_items.push(line);
    }

    // Render status items as list
    if !status_items.is_empty() {
        let rendered_items: Vec<String> = status_items
            .iter()
            .map(|item| Prose::new(item.as_str()).render(&terminal))
            .collect();
        let list = UnorderedList::new(rendered_items);
        println!("{}", list.render(&terminal));
    } else {
        let clean = Prose::new("<dim>No changes</dim>");
        println!("  {}", clean.render(&terminal));
    }

    // === Worktrees Section (only if worktrees exist) ===
    if !git.worktrees.is_empty() {
        let wt_title = Prose::new("<b><u>Worktrees</u></b>");
        println!("{}\n", wt_title.render(&terminal));

        let mut wt_list = UnorderedList::empty();
        wt_list.add(Prose::new(format!(
            "The <i>base repo</i> is located at <blue>{}</blue>",
            git.repo_root.display()
        )));
        for (branch, info) in &git.worktrees {
            wt_list.add(Prose::new(format!(
                "<b>{}:</b> <i>the {} worktree is located at</i> <blue>{}</blue>",
                branch,
                branch,
                info.filepath.display()
            )));
        }
        println!("{}", wt_list.render(&terminal));
    }

    // === Meta Section ===
    let meta_title = Prose::new("<b><u>Meta</u></b>");
    println!("{}\n", meta_title.render(&terminal));

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
                    let provider_label = format!(
                        "<dim>{}</dim> {}",
                        remote.provider.symbol(),
                        remote.provider.display_name()
                    );
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

            // Use remote's default branch, falling back to current branch
            let branch_part = remote
                .default_branch
                .as_ref()
                .or(git.current_branch.as_ref())
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

    println!("{}", meta_list.render(&terminal));
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

/// Render a single package as a styled list item string.
///
/// Format a single package as a list of renderable items.
///
/// The first item is the main line (name, version, path, etc.).
/// Subsequent items are verbose details rendered as child bullets.
fn format_package_items(pkg: &sniff::filesystem::repo::Package, verbose: u8) -> Vec<String> {
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
            items.push(format!("<dim>langs:</dim> {}", pkg.languages.join(", ")));
        }
    }

    items
}

/// Print package names as a comma-separated plain text list.
///
/// Writes to stderr and exits if the repo is not a monorepo.
pub fn print_repo_packages(result: &sniff::SniffResult, repo_filter: Option<&str>) {
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    match repo {
        Some(repo) if repo.is_monorepo => {
            if let Some(ref packages) = repo.packages {
                let filtered = filter_packages(packages, repo_filter);
                let names: Vec<&str> = filtered.iter().map(|p| p.name.as_str()).collect();
                println!("{}", names.join(", "));
            }
        }
        _ => {
            eprintln!("- the \"--packages\" switch is only intended to be used in a monorepo");
        }
    }
}

/// Print the package name for the given directory, or exit 1 if not in a package.
///
/// With `verbose >= 1`, appends the package root directory on the same line.
pub fn print_repo_package(result: &sniff::SniffResult, base_dir: Option<&Path>, verbose: u8) {
    let dir = resolve_dir(base_dir);
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    if let Some(pkg) = repo.and_then(|r| r.package_for_dir(&dir)) {
        if verbose > 0 {
            let terminal = Terminal::default();
            let line = format!(
                "{} (<i>located in</i> <blue>{}</blue>)",
                pkg.name,
                pkg.relative
            );
            print!("{}", Prose::new(&line).display(&terminal));
        } else {
            println!("{}", pkg.name);
        }
    } else {
        println!();
        std::process::exit(1);
    }
}

/// Print the package area for the given directory, or exit 1 if not in a package area.
pub fn print_repo_package_area(result: &sniff::SniffResult, base_dir: Option<&Path>) {
    let dir = resolve_dir(base_dir);
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    if let Some(area) = repo.and_then(|r| r.package_area_for_dir(&dir)) {
        println!("{}", area);
    } else {
        println!();
        std::process::exit(1);
    }
}

/// Resolve the effective directory from `--base` or fall back to CWD.
fn resolve_dir(base_dir: Option<&Path>) -> PathBuf {
    base_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

pub fn print_repo_section(
    repo: &sniff::filesystem::repo::RepoInfo,
    verbose: u8,
    _repo_root: Option<&Path>,
    repo_filter: Option<&str>,
) {
    if !repo.is_monorepo {
        let title = Prose::new("<b><u>Repository</u></b>");
        println!("\n{}\n", title.render_optimistic(None));
        let items = vec![
            Prose::new("<b>Type:</b> Single-package").render_optimistic(None),
            Prose::new(format!("<b>Root:</b> {}", repo.root.display())).render_optimistic(None),
        ];
        let list = UnorderedList::new(items);
        println!("{}", list.render_optimistic(None));
        return;
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

        let title_suffix = if repo_filter.is_some() && showing_count != total_count {
            format!(
                " <dim>({} / showing {} of {} packages)</dim>",
                tool_name, showing_count, total_count,
            )
        } else {
            format!(" <dim>({} / {} packages)</dim>", tool_name, total_count)
        };

        let title = Prose::new(format!("<b><u>Repository</u></b>{}", title_suffix));
        println!("\n{}\n", title.render_optimistic(None));

        // Track whether any package has updatable deps or excluded packages for the key
        let has_updatable = filtered.iter().any(|pkg| pkg.is_updatable == Some(true));
        let has_excluded = filtered.iter().any(|pkg| pkg.is_excluded);

        // Group packages by area, preserving discovery order
        let mut areas: Vec<String> = Vec::new();
        let mut area_packages: std::collections::HashMap<&str, Vec<&&Package>> =
            std::collections::HashMap::new();
        for pkg in &filtered {
            let area = pkg.package_area.as_str();
            if !area_packages.contains_key(area) {
                areas.push(area.to_string());
            }
            area_packages.entry(area).or_default().push(pkg);
        }

        let mut outer_items: Vec<RenderableContent> = Vec::new();
        for area in &areas {
            // Area heading in blue
            let label = Prose::new(format!("<blue><b>{}</b></blue>", area)).render_optimistic(None);
            outer_items.push(RenderableContent::String(label));

            // Nested package list with left margin
            let mut inner_items: Vec<RenderableContent> = Vec::new();
            for pkg in &area_packages[area.as_str()] {
                let items = format_package_items(pkg, verbose);
                // First item is the main package line
                let main = Prose::new(&items[0]).render_optimistic(None);
                inner_items.push(RenderableContent::String(main));
                // Additional items are verbose details shown as a nested child list
                if items.len() > 1 {
                    let detail_items: Vec<String> = items[1..]
                        .iter()
                        .map(|s| Prose::new(s).render_optimistic(None))
                        .collect();
                    let detail_list = UnorderedList::new(detail_items).with_bullet("  ");
                    inner_items.push(RenderableContent::Component(Rc::new(detail_list)));
                }
            }
            let inner_list = UnorderedList::from(inner_items);
            outer_items.push(RenderableContent::Component(Rc::new(inner_list)));
        }

        let list = UnorderedList::from(outer_items).with_indent_children(Some(4));
        println!("{}", list.render_optimistic(None));

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
            println!("{}", Prose::new(&legend).render_optimistic(None));
        }
    } else {
        let title = Prose::new(format!(
            "<b><u>Repository</u></b> <dim>({} / {} packages)</dim>",
            tool_name, total_count,
        ));
        println!("\n{}\n", title.render_optimistic(None));
    }
}

pub fn print_language_section(
    langs: &sniff::filesystem::languages::LanguageBreakdown,
    verbose: u8,
) {
    println!("=== Languages ===");
    println!("Files analyzed: {}", format_number(langs.total_files));
    if let Some(ref primary) = langs.primary {
        println!("Primary: {}", primary);
    }
    let show_count = if verbose > 0 { 10 } else { 5 };
    for lang in langs.languages.iter().take(show_count) {
        println!(
            "{}: {} files ({:.1}%)",
            lang.language,
            format_number(lang.file_count),
            lang.percentage
        );
        if verbose > 1 && !lang.files.is_empty() {
            let file_show_count = 3.min(lang.files.len());
            for file in lang.files.iter().take(file_show_count) {
                println!("  - {}", file.display());
            }
            if lang.files.len() > file_show_count {
                println!(
                    "  ... and {} more files",
                    lang.files.len() - file_show_count
                );
            }
        }
    }
    if langs.languages.len() > show_count {
        println!("... and {} more", langs.languages.len() - show_count);
    }
    println!();
}

pub fn print_filesystem_section(fs: &sniff::FilesystemInfo, verbose: u8, repo_root: Option<&Path>) {
    println!("=== Filesystem ===");

    // Print EditorConfig formatting info at verbose level 2+
    if verbose > 1
        && let Some(ref formatting) = fs.formatting
    {
        println!("EditorConfig: {}", formatting.config_path.display());
        for section in &formatting.sections {
            println!("  [{}]", section.pattern);
            if let Some(style) = &section.indent_style {
                println!("    indent_style: {}", style);
            }
            if let Some(size) = section.indent_size {
                println!("    indent_size: {}", size);
            }
        }
        println!();
    }

    if let Some(ref langs) = fs.languages {
        println!(
            "Languages ({} files analyzed):",
            format_number(langs.total_files)
        );
        if let Some(ref primary) = langs.primary {
            println!("  Primary: {}", primary);
        }
        let show_count = if verbose > 0 { 10 } else { 5 };
        for lang in langs.languages.iter().take(show_count) {
            println!(
                "  {}: {} files ({:.1}%)",
                lang.language,
                format_number(lang.file_count),
                lang.percentage
            );
            // Show file list at verbose level 2+
            if verbose > 1 && !lang.files.is_empty() {
                let file_show_count = 3.min(lang.files.len());
                for file in lang.files.iter().take(file_show_count) {
                    println!("    - {}", file.display());
                }
                if lang.files.len() > file_show_count {
                    println!(
                        "    ... and {} more files",
                        lang.files.len() - file_show_count
                    );
                }
            }
        }
        if langs.languages.len() > show_count {
            println!("  ... and {} more", langs.languages.len() - show_count);
        }
    }
    println!();

    if let Some(ref git) = fs.git {
        println!("Git Repository:");
        let root_str = relative_path(&git.repo_root, repo_root);
        println!(
            "  Root: {}",
            if root_str.is_empty() {
                ".".to_string()
            } else {
                root_str
            }
        );
        if let Some(ref branch) = git.current_branch {
            println!("  Branch: {}", branch);
        }

        // Show in_worktree indicator when true
        if git.in_worktree {
            println!("  In Worktree: yes");
        }

        // Show HEAD commit (first recent commit)
        if let Some(commit) = git.recent.first() {
            println!("  HEAD: {} ({})", &commit.sha[..8], commit.author);
            println!("  Message: {}", commit.message.lines().next().unwrap_or(""));
            // Show which remotes have this commit (deep mode)
            if let Some(ref remotes) = commit.remotes {
                println!("    Synced to: {}", remotes.join(", "));
            }
        }

        let dirty = if git.status.is_dirty {
            "dirty"
        } else {
            "clean"
        };
        println!(
            "  Status: {} ({} staged, {} unstaged, {} untracked)",
            dirty, git.status.staged_count, git.status.unstaged_count, git.status.untracked_count
        );

        // Show is_behind status (deep mode only)
        if let Some(ref behind) = git.status.is_behind {
            match behind {
                BehindStatus::NotBehind => println!("  Behind: no"),
                BehindStatus::Behind(remotes) => {
                    println!("  Behind: {}", remotes.join(", "));
                }
            }
        }

        // Show more recent commits at verbose level 1+
        if verbose > 0 && git.recent.len() > 1 {
            println!("  Recent commits:");
            for commit in git.recent.iter().skip(1).take(5) {
                let short_msg = commit.message.lines().next().unwrap_or("");
                let truncated = if short_msg.len() > 50 {
                    format!("{}...", &short_msg[..47])
                } else {
                    short_msg.to_string()
                };
                print!("    {} - {}", &commit.sha[..8], truncated);
                // Show commit remotes at verbose level 2+ with deep
                if verbose > 1
                    && let Some(ref remotes) = commit.remotes
                {
                    print!(" [{}]", remotes.join(", "));
                }
                println!();
            }
            if git.recent.len() > 6 {
                println!("    ... and {} more", git.recent.len() - 6);
            }
        }

        // Show dirty file details at verbose level 1+
        if verbose > 0 && !git.status.dirty.is_empty() {
            println!("  Dirty files:");
            for dirty_file in &git.status.dirty {
                println!("    - {}", dirty_file.filepath.display());
                // Show diff at verbose level 2+
                if verbose > 1 && !dirty_file.diff.is_empty() {
                    for line in dirty_file.diff.lines().take(5) {
                        println!("      {}", line);
                    }
                    let line_count = dirty_file.diff.lines().count();
                    if line_count > 5 {
                        println!("      ... ({} more lines)", line_count - 5);
                    }
                }
            }
        }

        // Show untracked files at verbose level 1+
        if verbose > 0 && !git.status.untracked.is_empty() {
            println!("  Untracked files:");
            let show_count = 5.min(git.status.untracked.len());
            for untracked in git.status.untracked.iter().take(show_count) {
                println!("    - {}", untracked.filepath.display());
            }
            if git.status.untracked.len() > show_count {
                println!(
                    "    ... and {} more",
                    git.status.untracked.len() - show_count
                );
            }
        }

        // Show worktrees at verbose level 1+
        if verbose > 0 && !git.worktrees.is_empty() {
            println!("  Worktrees:");
            for (branch, info) in &git.worktrees {
                let dirty_indicator = if info.dirty { " (dirty)" } else { "" };
                println!("    {} @ {}{}", branch, &info.sha[..8], dirty_indicator);
                if verbose > 1 {
                    println!("      Path: {}", info.filepath.display());
                }
            }
        }

        // Show remotes with enhanced branch info
        for remote in &git.remotes {
            print!("  Remote {}: {:?}", remote.name, remote.provider);
            // Show branch count in deep mode
            if let Some(ref branches) = remote.branches {
                print!(" ({} branches)", branches.len());
            }
            println!();
            // Show branches at verbose level 2+ with deep
            if verbose > 1
                && let Some(ref branches) = remote.branches
            {
                let show_count = 5.min(branches.len());
                for branch in branches.iter().take(show_count) {
                    println!("    - {}", branch);
                }
                if branches.len() > show_count {
                    println!("    ... and {} more", branches.len() - show_count);
                }
            }
        }
    }
    println!();

    if let Some(ref repo) = fs.repo
        && repo.is_monorepo
    {
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
        println!("{}", header.render_optimistic(None));

        if let Some(ref packages) = repo.packages {
            let items: Vec<String> = packages
                .iter()
                .map(|pkg| {
                    let markup = &format_package_items(pkg, verbose)[0];
                    Prose::new(markup).render_optimistic(None)
                })
                .collect();

            let list = UnorderedList::new(items);
            println!("{}", list.render_optimistic(None));
        }
    }
}

/// Print markdown documents section.
pub(crate) fn print_docs_section(docs: &[MarkdownMeta], verbose: u8) {
    let terminal = Terminal::default();
    let prompt_count = docs.iter().filter(|d| d.prompt.is_some()).count();

    let header = if prompt_count > 0 {
        format!(
            "<b>Docs</b> <dim>({} documents, {} with prompts)</dim>",
            docs.len(),
            prompt_count
        )
    } else {
        format!("<b>Docs</b> <dim>({} documents)</dim>", docs.len())
    };
    eprintln!("\n{}\n", Prose::new(&header).render(&terminal));

    let items: Vec<String> = docs
        .iter()
        .map(|doc| {
            let file_link = format_doc_filepath(&doc.relative, &doc.filepath.display().to_string());

            if verbose > 0 {
                let date_str = doc.last_updated.format("%Y-%m-%d").to_string();
                let mut meta_parts = Vec::new();
                if !doc.title.is_empty() {
                    meta_parts.push(format!("title: <dim>{}</dim>", doc.title));
                }
                meta_parts.push(format!("updated: <dim>{date_str}</dim>"));
                format!("{file_link} ({meta})", meta = meta_parts.join(", "))
            } else {
                file_link
            }
        })
        .map(|item| Prose::new(&item).render(&terminal))
        .collect();

    let list = UnorderedList::new(items);
    println!("{}", list.render(&terminal));

    if verbose == 0 {
        eprintln!(
            "{}",
            Prose::new(
                "<dim>Use <blue>--verbose</blue> / <blue>-v</blue> to include title and last updated</dim>"
            )
            .render(&terminal)
        );
    }
}

/// Format a document filepath with dim directory and bold filename,
/// wrapped in an OSC8 hyperlink.
fn format_doc_filepath(relative: &str, absolute: &str) -> String {
    match relative.rsplit_once('/') {
        Some((dir, file)) => {
            format!("<a href=\"{absolute}\"><blue><dim>{dir}/</dim><b>{file}</b></blue></a>")
        }
        None => {
            // No directory prefix, just the filename
            format!("<a href=\"{absolute}\"><blue><b>{relative}</b></blue></a>")
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
pub fn print_repo_deps_visual(repo: &sniff::filesystem::repo::RepoInfo, repo_filter: Option<&str>) {
    if !repo.is_monorepo {
        eprintln!("--deps requires a monorepo (no workspace packages found)");
        return;
    }

    let packages = match repo.packages {
        Some(ref pkgs) => pkgs,
        None => {
            eprintln!("No packages found in workspace");
            return;
        }
    };

    let filtered: Vec<Package> = filter_packages(packages, repo_filter)
        .into_iter()
        .cloned()
        .collect();

    let mermaid = match build_deps_mermaid(&filtered) {
        Some(m) => m,
        None => {
            eprintln!("No internal dependencies found between workspace packages");
            return;
        }
    };

    let renderer = MermaidRenderer::for_terminal(&mermaid);
    match renderer.render_for_terminal() {
        Ok(()) => {}
        Err(_) => {
            renderer.print_fallback();
        }
    }
}

/// Render an internal dependency list for the repository as styled text.
///
/// Each package with dependencies or dependents is shown as a top-level item
/// with `depends-on` and `used-by` sub-items. Isolates (packages with neither)
/// are omitted unless an explicit filter is set.
pub fn print_repo_deps_text(repo: &sniff::filesystem::repo::RepoInfo, repo_filter: Option<&str>) {
    if !repo.is_monorepo {
        eprintln!("--deps requires a monorepo (no workspace packages found)");
        return;
    }

    let packages = match repo.packages {
        Some(ref pkgs) => pkgs,
        None => {
            eprintln!("No packages found in workspace");
            return;
        }
    };

    let filtered = filter_packages(packages, repo_filter);
    let has_explicit_filter = repo_filter.is_some();

    // Collect only packages that participate in dependency relationships
    // (unless an explicit filter is set, in which case show all matched)
    let relevant: Vec<&&Package> = filtered
        .iter()
        .filter(|pkg| has_explicit_filter || !pkg.depends_on.is_empty() || !pkg.used_by.is_empty())
        .collect();

    if relevant.is_empty() {
        eprintln!("No internal dependencies found between workspace packages");
        return;
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
    eprintln!("\n{}\n", Prose::new(&title).render(&term));

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
    print!("{}", list.render(&term));

    eprintln!(
        "\n{}",
        Prose::new(
            "<dim><i>use the <blue>--ui</blue> CLI switch to show this in a visual format</i></dim>"
        )
        .render(&term)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use sniff::filesystem::repo::Package;
    use std::path::PathBuf;

    fn make_package(name: &str, area: &str, depends_on: &[&str]) -> Package {
        Package {
            path: PathBuf::from(format!("/repo/{area}/{name}")),
            relative: format!("{area}/{name}"),
            package_area: area.to_string(),
            name: name.to_string(),
            primary_language: None,
            languages: vec![],
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
            let result = filter_packages(&packages, None);
            assert_eq!(result.len(), 2);
        }

        #[test]
        fn name_filter() {
            let packages = vec![
                make_package("biscuit-hash", "biscuit-hash", &[]),
                make_package("sniff-cli", "sniff", &[]),
                make_package("biscuit-file", "biscuit-file", &[]),
            ];
            let result = filter_packages(&packages, Some("biscuit"));
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
            let result = filter_packages(&packages, Some("@sniff"));
            assert_eq!(result.len(), 2);
            assert!(result.iter().all(|p| p.package_area == "sniff"));
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
