use std::io::IsTerminal as _;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::mermaid::MermaidDiagram;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable as _;
use biscuit_terminal::components::terminal_image::ImageWidth;
use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::terminal::Terminal;
use worktree::worktree::{default_branch, list_worktrees, WorktreeStatus};
use worktree::WorktreeError;

use super::git_graph;

const GRAPH_WIDTH: u32 = 55;

pub fn run() -> Result<(), WorktreeError> {
    let statuses = list_worktrees()?;
    let terminal = Terminal::default();

    let mut list = UnorderedList::empty();
    for status in &statuses {
        list.add(Prose::new(format_status_line(status)));
    }

    eprintln!("\n{}", list.render(&terminal));

    // Render git graph if terminal is wide enough
    if terminal.width() >= GRAPH_WIDTH {
        render_graph(&statuses, &terminal);
    }

    Ok(())
}

fn render_graph(statuses: &[WorktreeStatus], terminal: &Terminal) {
    let Ok(default) = default_branch() else {
        return;
    };

    let current = statuses.iter().find(|s| s.entry.is_current);

    let instructions = if let Some(current) = current {
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
    } else {
        None
    };

    if let Some(instructions) = instructions {
        let img_term = image_terminal(terminal);
        let diagram =
            MermaidDiagram::new(instructions).with_width(ImageWidth::Characters(GRAPH_WIDTH));
        eprint!("{}", diagram.render(&img_term));
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
