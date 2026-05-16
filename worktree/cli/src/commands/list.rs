use std::io::IsTerminal as _;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::mermaid::MermaidDiagram;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::components::terminal_image::{ImageWidth, parse_width_spec};
use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::terminal::Terminal;
use worktree::WorktreeError;
use worktree::worktree::{WorktreeStatus, default_branch, list_worktrees};

use super::git_graph;

const MIN_GRAPH_TERMINAL_WIDTH: u32 = 80;

pub fn run(width_spec: Option<&str>, verbose: bool) -> Result<(), WorktreeError> {
    let statuses = list_worktrees()?;
    let terminal = Terminal::default();

    let mut list = UnorderedList::empty();
    for status in &statuses {
        list.add(Prose::new(format_status_line(status)));
    }

    eprintln!("\n{}", list.render(&terminal));

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

fn format_status_line(status: &WorktreeStatus) -> String {
    let name = status.entry.branch.as_deref().unwrap_or("(detached)");

    let name_styled = if status.entry.is_current {
        format!("<b>{name}</b>")
    } else {
        format!("<dim>{name}</dim>")
    };

    let mut parts = vec![name_styled];

    if !status.entry.is_main {
        let merge_indicator = if status.entry.is_current {
            if status.is_clean {
                "<green>clean</green>".to_string()
            } else {
                "<red>conflict</red>".to_string()
            }
        } else if status.is_clean {
            "<dim>clean</dim>".to_string()
        } else {
            "<dim>conflict</dim>".to_string()
        };
        parts.push(merge_indicator);

        if status.ahead > 0 || status.behind > 0 {
            let mut counts = Vec::new();
            if status.ahead > 0 {
                counts.push(if status.entry.is_current {
                    format!("<green>+{}</green>", status.ahead)
                } else {
                    format!("<dim>+{}</dim>", status.ahead)
                });
            }
            if status.behind > 0 {
                counts.push(if status.entry.is_current {
                    format!("<yellow>-{}</yellow>", status.behind)
                } else {
                    format!("<dim>-{}</dim>", status.behind)
                });
            }
            parts.push(counts.join(" "));
        }
    }

    parts.join("  ")
}
