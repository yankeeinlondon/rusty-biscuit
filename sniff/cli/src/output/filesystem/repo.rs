use std::fmt::Write;
use std::path::Path;
use std::rc::Rc;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{TerminalRenderable, RenderableTerminalContent};
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::git::BehindStatus;
use sniff::filesystem::repo::{DependencyEntry, Package, RepoIdentity, RepoInfo};

use super::language::render_framework_summary;
use super::packages::select_repo_packages;
use super::{format_number, relative_path};

/// Render `sniff repo name` output.
///
/// At verbosity `0`, emits just the bare repo name followed by a newline so
/// the command pipes cleanly. At verbosity `>= 1`, decorates the name with
/// version and a language-or-monorepo-package-count suffix, styled through
/// Prose so terminal-aware fallbacks apply.
pub fn render_repo_name(identity: &RepoIdentity, verbose: u8) -> String {
    if verbose == 0 {
        return format!("{}\n", identity.name);
    }

    let mut markup = format!("**{}**", identity.name);
    if let Some(version) = identity.version.as_deref() {
        markup.push_str(&format!(" v{version}"));
    }

    let suffix = if identity.is_monorepo {
        identity
            .package_count
            .map(|count| format!(" [<dim>{} package monorepo</dim>]", format_number(count)))
    } else {
        identity
            .language
            .as_deref()
            .map(|lang| format!(" [{lang}]"))
    };
    if let Some(suffix) = suffix {
        markup.push_str(&suffix);
    }

    let term = Terminal::default();
    let rendered = Prose::new(&markup).render(&term);
    if rendered.ends_with('\n') {
        rendered
    } else {
        format!("{rendered}\n")
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

pub(super) fn build_area_hierarchy(
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

fn append_package_items(items: &mut Vec<RenderableTerminalContent>, pkg: &Package, verbose: u8) {
    let formatted = format_package_items(pkg, verbose);
    let main = Prose::new(&formatted[0]).render_optimistic(None);
    items.push(RenderableTerminalContent::String(main));

    if formatted.len() > 1 {
        let detail_items: Vec<String> = formatted[1..]
            .iter()
            .map(|s| Prose::new(s).render_optimistic(None))
            .collect();
        let detail_list = UnorderedList::new(detail_items).with_bullet("  ");
        items.push(RenderableTerminalContent::Component(Rc::new(detail_list)));
    }
}

fn append_area_section(
    output: &mut Vec<RenderableTerminalContent>,
    area: &str,
    area_packages: &std::collections::HashMap<String, Vec<&Package>>,
    area_children: &std::collections::HashMap<String, Vec<String>>,
    verbose: u8,
) {
    let label = Prose::new(format!("<blue><b>{}</b></blue>", area)).render_optimistic(None);
    output.push(RenderableTerminalContent::String(label));

    let mut inner_items: Vec<RenderableTerminalContent> = Vec::new();
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
        output.push(RenderableTerminalContent::Component(Rc::new(inner_list)));
    }
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
pub fn render_repo_section(
    repo: &sniff::filesystem::repo::RepoInfo,
    verbose: u8,
    _repo_root: Option<&Path>,
    repo_filter: &[String],
    package: Option<&str>,
    package_area: Option<&str>,
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
        let filtered = select_repo_packages(packages, repo_filter, package, package_area);
        let showing_count = filtered.len();

        let any_scope = !repo_filter.is_empty() || package.is_some() || package_area.is_some();
        let title_suffix = if any_scope && showing_count != total_count {
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

        let mut outer_items: Vec<RenderableTerminalContent> = Vec::new();
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
            let mut items: Vec<RenderableTerminalContent> = Vec::new();
            for pkg in packages {
                let package_items = format_package_items(pkg, verbose);
                items.push(RenderableTerminalContent::String(
                    Prose::new(&package_items[0]).render_optimistic(None),
                ));
                if package_items.len() > 1 {
                    let detail_items = package_items[1..]
                        .iter()
                        .map(|item| Prose::new(item).render_optimistic(None))
                        .collect::<Vec<_>>();
                    let detail_list = UnorderedList::new(detail_items).with_bullet("  ");
                    items.push(RenderableTerminalContent::Component(Rc::new(detail_list)));
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
