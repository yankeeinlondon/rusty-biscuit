use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable as _;
use biscuit_terminal::terminal::Terminal;
use worktree::worktree::{list_worktrees, WorktreeStatus};
use worktree::WorktreeError;

pub fn run() -> Result<(), WorktreeError> {
    let statuses = list_worktrees()?;
    let terminal = Terminal::default();

    let mut list = UnorderedList::empty();
    for status in &statuses {
        list.add(Prose::new(format_status_line(status)));
    }

    eprintln!("\n{}", list.render(&terminal));

    Ok(())
}

fn format_status_line(status: &WorktreeStatus) -> String {
    let name = status
        .entry
        .branch
        .as_deref()
        .unwrap_or("(detached)");

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
