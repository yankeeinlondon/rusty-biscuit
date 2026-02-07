//! Filesystem section output formatting (Git, Repo, Languages, Docs).

use std::path::Path;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use sniff::filesystem::docs::MarkdownMeta;
use sniff::filesystem::git::{BehindStatus, ConventionalCommit, FileStatus, RefKind};

use super::{format_number, relative_path};

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

/// Format a hosting provider as a display string.
fn format_provider(provider: &sniff::filesystem::git::HostingProvider) -> &'static str {
    use sniff::filesystem::git::HostingProvider;
    match provider {
        HostingProvider::GitHub => "GitHub",
        HostingProvider::GitLab => "GitLab",
        HostingProvider::Bitbucket => "Bitbucket",
        HostingProvider::AzureDevOps => "Azure DevOps",
        HostingProvider::AwsCodeCommit => "AWS CodeCommit",
        HostingProvider::Gitea => "Gitea",
        HostingProvider::Forgejo => "Forgejo",
        HostingProvider::SourceHut => "SourceHut",
        HostingProvider::SelfHosted => "Self-Hosted",
        HostingProvider::Unknown | _ => "Unknown",
    }
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
    use sniff::filesystem::git::HostingProvider;

    // Try to extract owner/repo from URL
    let owner_repo = if url.contains('@') && url.contains(':') {
        // SSH format: git@github.com:owner/repo.git
        url.split(':')
            .last()
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
        let base = match provider {
            HostingProvider::GitHub => Some("https://github.com"),
            HostingProvider::GitLab => Some("https://gitlab.com"),
            HostingProvider::Bitbucket => Some("https://bitbucket.org"),
            HostingProvider::SourceHut => Some("https://sr.ht"),
            _ => None,
        };
        base.map(|b| format!("{}/{}", b, repo))
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
pub fn print_git_section(git: &sniff::filesystem::git::GitInfo, history_count: usize) {
    // === Status Section ===
    let status_title = Prose::new("<b><u>Status</u></b>");
    println!("\n{}\n", status_title.render(None));

    let mut status_items: Vec<String> = Vec::new();

    // Recent commits with conventional commit parsing
    for commit in git.recent.iter().take(history_count) {
        let cc = ConventionalCommit::parse(&commit.message);
        let (date_str, time_str, use_on) = format_commit_datetime(&commit.timestamp);
        let sha = commit.sha[0..7].to_string();
        let date_prefix = if use_on { "<i>on</i> " } else { "" };
        let refs_part = format_ref_decorations(&commit.refs);

        let commit_line = if let Some(ref op) = cc.operation {
            let scope_part = cc
                .scope
                .as_ref()
                .map(|s| format!("(<dim>{}</dim>)", s))
                .unwrap_or_default();
            format!(
                "[<b>{}</b>] <b><yellow>{}</yellow></b>{} <i>at</i> <blue><b>{}</b></blue> {}<blue>{}</blue>{}: <dim>{}</dim>",
                sha, op, scope_part, time_str, date_prefix, date_str, refs_part, cc.description
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
                "[<b>{}</b>] <dim>{}</dim> {}<blue><b>{}</b></blue>{}",
                sha, truncated, date_prefix, date_str, refs_part,
            )
        };
        status_items.push(commit_line);
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
            format!("<red>modified: <b>{}</b></red>", name)
        } else {
            format!("<red>modified: {}<b>{}</b></red>", dir, name)
        };
        status_items.push(line);
    }

    // Add untracked files
    for file in &untracked {
        let path = file.path.display().to_string();
        let (dir, name) = split_path(&path);
        let line = if dir.is_empty() {
            format!("<yellow>untracked: <b>{}</b></yellow>", name)
        } else {
            format!("<yellow>untracked: {}<b>{}</b></yellow>", dir, name)
        };
        status_items.push(line);
    }

    // Render status items as list
    if !status_items.is_empty() {
        let rendered_items: Vec<String> = status_items
            .iter()
            .map(|item| Prose::new(item.as_str()).render(None))
            .collect();
        let list = UnorderedList::new(rendered_items);
        println!("{}", list.render(None));
    } else {
        let clean = Prose::new("<dim>No changes</dim>");
        println!("  {}", clean.render(None));
    }

    // === Meta Section ===
    let meta_title = Prose::new("<b><u>Meta</u></b>");
    println!("\n{}\n", meta_title.render(None));

    let mut meta_items: Vec<String> = Vec::new();

    // Remote tracking status with heading - render as list under "Remotes:" header
    if !git.tracking.is_empty() || !git.remotes.is_empty() {
        // Build remote items - match tracking with remote info
        let remote_items: Vec<String> = git
            .remotes
            .iter()
            .map(|remote| {
                // Find tracking info for this remote
                let tracking_part = git
                    .tracking
                    .iter()
                    .find(|t| t.remote == remote.name)
                    .map(|t| {
                        format!(
                            "<green>{} ahead</green>, <red>{} behind</red>",
                            t.ahead, t.behind
                        )
                    })
                    .unwrap_or_default();

                // Parse owner/repo from URL and build display string with link
                let repo_link = remote
                    .url
                    .as_ref()
                    .map(|url| {
                        let (owner_repo, browse_url) = parse_git_url(url, &remote.provider);
                        let provider_name = format_provider(&remote.provider);
                        if let Some(ref repo_path) = owner_repo {
                            let link_url = browse_url.unwrap_or_else(|| url.clone());
                            format!(
                                " - <a href=\"{}\"><blue>{}</blue></a> <i>on</i> {}",
                                link_url, repo_path, provider_name
                            )
                        } else {
                            format!(" <i>on</i> {}", provider_name)
                        }
                    })
                    .unwrap_or_default();

                let line = if tracking_part.is_empty() {
                    format!("<b>{}</b>{}", remote.name, repo_link)
                } else {
                    format!("<b>{}</b>: {}{}", remote.name, tracking_part, repo_link)
                };

                Prose::new(&line).render(None)
            })
            .collect();

        // Print "Remotes:" header followed by the list
        if !remote_items.is_empty() {
            let header = Prose::new("<b>Remotes:</b>");
            println!("{}", header.render(None));
            let remote_list = UnorderedList::new(remote_items);
            println!("{}", remote_list.render(None));
        }
    }

    // Branch info with heading
    if let Some(ref current) = git.current_branch {
        let other_branches: Vec<_> = git
            .branches
            .iter()
            .filter(|b| *b != current)
            .take(3)
            .cloned()
            .collect();

        let branch_line = if other_branches.is_empty() {
            format!("<b>Branches:</b> <b>{}</b>", current)
        } else {
            let others = other_branches.join(", ");
            let more = if git.branches.len() > 4 {
                format!(", +{} more", git.branches.len() - 4)
            } else {
                String::new()
            };
            format!(
                "<b>Branches:</b> <b>{}</b> <dim>({}{})</dim>",
                current, others, more
            )
        };
        meta_items.push(branch_line);
    }

    // Git config with heading - format email with angle brackets
    // Use mathematical angle brackets (⟨ and ⟩) to avoid HTML parsing issues
    if let Some(ref name) = git.config.user_name {
        let email_part = git
            .config
            .user_email
            .as_ref()
            .map(|e| format!(" <dim>⟨{}⟩</dim>", e))
            .unwrap_or_default();
        meta_items.push(format!(
            "<b>Git Config:</b> <cyan>{}</cyan>{}",
            name, email_part
        ));
    }

    // Render meta items as list
    if !meta_items.is_empty() {
        let rendered_items: Vec<String> = meta_items
            .iter()
            .map(|item| Prose::new(item.as_str()).render(None))
            .collect();
        let list = UnorderedList::new(rendered_items);
        println!("{}", list.render(None));
    }

    println!();
}

pub fn print_repo_section(
    repo: &sniff::filesystem::repo::RepoInfo,
    verbose: u8,
    repo_root: Option<&Path>,
) {
    println!("=== Repository ===");

    if repo.is_monorepo {
        if let Some(ref tool) = repo.monorepo_tool {
            println!("Monorepo: {:?}", tool);
        }
        if let Some(ref packages) = repo.packages {
            println!("Packages: {}", packages.len());
            let show_count = if verbose > 0 { packages.len() } else { 5 };
            for pkg in packages.iter().take(show_count) {
                let path_str = relative_path(&pkg.path, repo_root);
                let lang_info = pkg
                    .primary_language
                    .as_ref()
                    .map(|l| format!(" [{}]", l))
                    .unwrap_or_default();
                println!("  {} ({}){}", pkg.name, path_str, lang_info);

                if verbose > 0 && !pkg.detected_managers.is_empty() {
                    println!("    Managers: {}", pkg.detected_managers.join(", "));
                }
                if verbose > 1 && !pkg.languages.is_empty() {
                    println!("    Languages: {}", pkg.languages.join(", "));
                }
            }
            if packages.len() > show_count {
                println!("  ... and {} more", packages.len() - show_count);
            }
        }
    } else {
        println!("Single-package repository");
        println!("Root: {}", repo.root.display());
    }
    println!();
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
        if let Some(ref tool) = repo.monorepo_tool {
            println!("Monorepo: {:?}", tool);
        }
        if let Some(ref packages) = repo.packages {
            println!("  Packages: {}", packages.len());
            let show_count = if verbose > 0 { packages.len() } else { 5 };
            for pkg in packages.iter().take(show_count) {
                let path_str = relative_path(&pkg.path, repo_root);
                let lang_info = pkg
                    .primary_language
                    .as_ref()
                    .map(|l| format!(" [{}]", l))
                    .unwrap_or_default();
                println!("    {} ({}){}", pkg.name, path_str, lang_info);

                // Show package details at verbose level 1+
                if verbose > 0 {
                    if !pkg.detected_managers.is_empty() {
                        println!("      Managers: {}", pkg.detected_managers.join(", "));
                    }
                    if verbose > 1 && !pkg.languages.is_empty() {
                        println!("      Languages: {}", pkg.languages.join(", "));
                    }
                }
            }
            if packages.len() > show_count {
                println!("    ... and {} more", packages.len() - show_count);
            }
        }
    }
}

/// Print markdown documents section.
pub(crate) fn print_docs_section(docs: &[MarkdownMeta]) {
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
    println!("\n{}\n", Prose::new(&header).render(None));

    for doc in docs {
        let pkg_display = doc.package.as_deref().unwrap_or("(root)");

        let date_str = doc.last_updated.format("%Y-%m-%d").to_string();

        let mut details = vec![
            format!("<dim>Package:</dim> {pkg_display}"),
            format!("<dim>Updated:</dim> {date_str}"),
        ];

        if !doc.title.is_empty() {
            details.insert(0, format!("<dim>Title:</dim> {}", doc.title));
        }

        if doc.prompt.is_some() {
            details.push("<dim>Prompt:</dim> <green>yes</green>".to_string());
        }
        if let Some(ref model) = doc.model {
            details.push(format!("<dim>Model:</dim> {model}"));
        }

        println!(
            "  {}",
            Prose::new(&format!("<cyan>{}</cyan>", doc.relative)).render(None)
        );
        for detail in &details {
            println!("    {}", Prose::new(detail).render(None));
        }
        println!();
    }
}
