use std::io::IsTerminal as _;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::mermaid::MermaidDiagram;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::components::terminal_image::{ImageWidth, parse_width_spec};
use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Alignment;
use worktree::WorktreeError;
use worktree::worktree::{DirtyStatus, WorktreeStatus, default_branch, list_worktrees};

use super::git_graph;

const MIN_GRAPH_TERMINAL_WIDTH: u32 = 80;

pub fn run(width_spec: Option<&str>, verbose: bool) -> Result<(), WorktreeError> {
    let statuses = list_worktrees()?;
    let terminal = Terminal::default();

    let table = build_status_table(&statuses, &terminal);
    eprintln!("\n{}", table.render(&terminal));

    // Generate graph instructions first so we can size the width based on commit count
    if let Some(instructions) = graph_instructions(&statuses) {
        let commit_count = instructions
            .lines()
            .filter(|l| l.contains("commit id:"))
            .count();

        let graph_width = width_spec
            .and_then(|s| parse_width_spec(s).ok())
            .unwrap_or_else(|| default_graph_width(commit_count, terminal.width()));

        // For a percentage, the graph always fits; for characters, check the terminal.
        let fits = match &graph_width {
            ImageWidth::Percent(_) | ImageWidth::Fill => true,
            ImageWidth::Characters(_) => terminal.width() >= MIN_GRAPH_TERMINAL_WIDTH,
        };

        if fits {
            let img_term = image_terminal(&terminal);
            let diagram = MermaidDiagram::new(instructions).with_width(graph_width);
            eprint!("{}", diagram.render(&img_term));
        }
    }

    if verbose {
        render_verbose(&statuses, &terminal);
    }

    Ok(())
}

fn graph_instructions(statuses: &[WorktreeStatus]) -> Option<String> {
    let default = default_branch().ok()?;
    let current = statuses.iter().find(|s| s.entry.is_current)?;

    if current.entry.is_main {
        let branch_names: Vec<String> = statuses
            .iter()
            .filter_map(|s| s.entry.branch.clone())
            .collect();
        git_graph::base_graph(&branch_names, &default)
    } else {
        let branch = current.entry.branch.as_deref().unwrap_or("HEAD");
        git_graph::worktree_graph(branch, &default)
    }
}

/// Choose a default graph width based on how many commits are in the graph.
fn default_graph_width(commit_count: usize, terminal_width: u32) -> ImageWidth {
    if commit_count <= 4 {
        ImageWidth::Characters(60)
    } else if commit_count <= 8 {
        ImageWidth::Characters(80)
    } else if commit_count <= 15 {
        if terminal_width > 120 {
            ImageWidth::Characters(120)
        } else {
            ImageWidth::Percent(100.0)
        }
    } else if terminal_width >= 160 {
        ImageWidth::Characters(160)
    } else {
        ImageWidth::Percent(100.0)
    }
}

fn render_verbose(statuses: &[WorktreeStatus], terminal: &Terminal) {
    let Ok(default) = default_branch() else {
        return;
    };

    let current = statuses.iter().find(|s| s.entry.is_current);
    let Some(current) = current else {
        return;
    };
    if current.entry.is_main {
        return;
    }
    let branch = current.entry.branch.as_deref().unwrap_or("HEAD");

    // Main section: the commit where the worktree branched from
    let main_heading = Prose::new(format!("<b><blue-500>{default}</blue-500></b>"));
    eprintln!("{}", main_heading.render(terminal));

    if let Some(commit) = git_graph::merge_base_commit(&default, branch) {
        let mut main_list = UnorderedList::empty();
        main_list.add(Prose::new(git_graph::format_commit(&commit)));
        eprintln!("{}", main_list.render(terminal));
    }

    // Worktree section: all commits since the branch point
    let branch_heading = Prose::new(format!("<b><yellow-500>{branch}</yellow-500></b>"));
    eprintln!("{}", branch_heading.render(terminal));

    let branch_commits = git_graph::branch_commits_detail(branch, &default);
    if branch_commits.is_empty() {
        let mut empty_list = UnorderedList::empty();
        empty_list.add(Prose::new(
            "<dim>no commits since branching</dim>".to_string(),
        ));
        eprintln!("{}", empty_list.render(terminal));
    } else {
        let mut branch_list = UnorderedList::empty();
        for commit in &branch_commits {
            branch_list.add(Prose::new(git_graph::format_commit(commit)));
        }
        eprintln!("{}", branch_list.render(terminal));
    }
}

/// Build a Terminal suitable for rendering images to stderr.
///
/// `Terminal::default()` calls `is_tty()` which checks stdout. When the shell
/// wrapper captures stdout via `$()`, stdout is a pipe and image support is
/// suppressed. This builds a terminal that uses stderr for the TTY check and
/// detects image support from `$TERM_PROGRAM` env vars instead.
fn image_terminal(_base: &Terminal) -> Terminal {
    let stderr_is_tty = std::io::stderr().is_terminal();
    let img_support = if stderr_is_tty {
        detect_image_support_from_env()
    } else {
        ImageSupport::None
    };
    Terminal::builder()
        .is_tty(stderr_is_tty)
        .image_support(img_support)
        .build()
}

fn detect_image_support_from_env() -> ImageSupport {
    match std::env::var("TERM_PROGRAM").as_deref() {
        Ok("ghostty") | Ok("kitty") | Ok("WezTerm") | Ok("Warp") | Ok("WarpTerminal")
        | Ok("konsole") | Ok("wast") => ImageSupport::Kitty,
        Ok("iTerm.app") => ImageSupport::ITerm,
        _ => {
            if std::env::var("KITTY_WINDOW_ID").is_ok() {
                ImageSupport::Kitty
            } else {
                ImageSupport::None
            }
        }
    }
}

/// Build the worktree-status table with one row per worktree.
fn build_status_table(statuses: &[WorktreeStatus], terminal: &Terminal) -> Table {
    let columns = vec![
        TableColumn::new("Worktree").with_alignment(Alignment::Center),
        TableColumn::new("Worktree Name"),
        TableColumn::new("Branch"),
        TableColumn::new("Merge").with_alignment(Alignment::Center),
        TableColumn::new("Commits").with_alignment(Alignment::Right),
    ];

    let mut table = Table::new()
        .with_columns(columns)
        .prefer_cursor_alignment();

    for status in statuses {
        table.add_row(status_row(status, terminal));
    }

    table
}

fn status_row(status: &WorktreeStatus, terminal: &Terminal) -> Vec<TableCellContent> {
    vec![
        prose_cell(&dirty_badge(status.dirty), terminal),
        prose_cell(&wt_name_markup(status), terminal),
        prose_cell(&branch_markup(status), terminal),
        prose_cell(&merge_markup(status), terminal),
        prose_cell(&commits_markup(status), terminal),
    ]
}

fn prose_cell(markup: &str, terminal: &Terminal) -> TableCellContent {
    Prose::new(markup.to_string()).render(terminal).into()
}

fn wt_name_markup(status: &WorktreeStatus) -> String {
    let name = status
        .entry
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "(unknown)".to_string());
    if status.entry.is_main {
        let branch = status.entry.branch.as_deref().unwrap_or("HEAD");
        return format!("<dim>{branch}::(<i>{name}</i>)</dim>");
    }
    if status.entry.is_current {
        format!("<b>{name}</b>")
    } else {
        format!("<dim>{name}</dim>")
    }
}

fn branch_markup(status: &WorktreeStatus) -> String {
    let branch = status.entry.branch.as_deref().unwrap_or("(detached)");
    if status.entry.is_current {
        format!("<b>{branch}</b>")
    } else {
        format!("<dim>{branch}</dim>")
    }
}

fn merge_markup(status: &WorktreeStatus) -> String {
    if status.entry.is_main {
        return String::new();
    }
    match (status.entry.is_current, status.is_clean) {
        (true, true) => "<green>clean</green>".to_string(),
        (true, false) => "<red>conflict</red>".to_string(),
        (false, true) => "<dim>clean</dim>".to_string(),
        (false, false) => "<dim>conflict</dim>".to_string(),
    }
}

fn commits_markup(status: &WorktreeStatus) -> String {
    if status.entry.is_main || (status.ahead == 0 && status.behind == 0) {
        return String::new();
    }
    let mut parts = Vec::new();
    if status.ahead > 0 {
        parts.push(if status.entry.is_current {
            format!("<green>+{}</green>", status.ahead)
        } else {
            format!("<dim>+{}</dim>", status.ahead)
        });
    }
    if status.behind > 0 {
        parts.push(if status.entry.is_current {
            format!("<yellow>-{}</yellow>", status.behind)
        } else {
            format!("<dim>-{}</dim>", status.behind)
        });
    }
    parts.join(" ")
}

/// Render a colored badge for a worktree's working-tree dirtiness.
///
/// All three badges have the same visible width (` Clean ` / ` Dirty `) so the
/// name column lines up across rows.
fn dirty_badge(status: DirtyStatus) -> String {
    match status {
        DirtyStatus::Clean => {
            "<bg-green-900><green-300><b> Clean </b></green-300></bg-green-900>".to_string()
        }
        DirtyStatus::DirtyNonSource => {
            "<bg-yellow-900><yellow-300><b> Dirty </b></yellow-300></bg-yellow-900>".to_string()
        }
        DirtyStatus::DirtySource => {
            "<bg-red-900><red-300><b> Dirty </b></red-300></bg-red-900>".to_string()
        }
    }
}
