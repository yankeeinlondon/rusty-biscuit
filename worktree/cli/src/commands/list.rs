use biscuit_terminal::components::compose::Compose;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable as _;
use biscuit_terminal::terminal::Terminal;
use worktree::worktree::{list_worktrees, WorktreeStatus};
use worktree::WorktreeError;

pub fn run() -> Result<(), WorktreeError> {
    let statuses = list_worktrees()?;
    let terminal = Terminal::default();

    let mut compose = Compose::default();
    compose.add_text("");

    for status in &statuses {
        let line = format_status_line(status);
        compose.add_prose(Prose::new(line));
    }

    compose.add_text("");
    eprintln!("{}", compose.render(&terminal));

    Ok(())
}

fn format_status_line(status: &WorktreeStatus) -> String {
    let name = status
        .entry
        .branch
        .as_deref()
        .unwrap_or("(detached)");

    let marker = if status.entry.is_current { "▸ " } else { "  " };

    let name_styled = if status.entry.is_current {
        format!("<bold><cyan>{name}</cyan></bold>")
    } else if status.entry.is_main {
        format!("<bold>{name}</bold>")
    } else {
        name.to_string()
    };

    let mut parts = vec![format!("{marker}{name_styled}")];

    if !status.entry.is_main {
        let merge_indicator = if status.is_clean {
            "<green>clean</green>".to_string()
        } else {
            "<red>conflict</red>".to_string()
        };
        parts.push(merge_indicator);

        if status.ahead > 0 || status.behind > 0 {
            let mut counts = Vec::new();
            if status.ahead > 0 {
                counts.push(format!("<green>+{}</green>", status.ahead));
            }
            if status.behind > 0 {
                counts.push(format!("<yellow>-{}</yellow>", status.behind));
            }
            parts.push(counts.join(" "));
        }
    }

    parts.join("  ")
}
