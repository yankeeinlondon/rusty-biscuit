use std::io::IsTerminal as _;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use worktree::WorktreeError;
use worktree::git::repo_info;
use worktree::worktree::find_worktree;

pub fn run(name: &str) -> Result<(), WorktreeError> {
    let entry = find_worktree(name)?;
    let info = repo_info()?;

    let relative = &info.relative_path;
    let preferred = entry.path.join(relative);
    let (target, path_adjusted) = if relative.as_os_str().is_empty() || preferred.exists() {
        (preferred, false)
    } else {
        (entry.path.clone(), true)
    };

    let terminal = Terminal::default();
    let relative_display = relative.to_string_lossy();
    let repo = &info.name;

    // Already in the target worktree — nothing to do.
    if entry.is_current {
        let kind = if entry.is_main {
            "<blue-500>base</blue-500> <i>checkout</i>".to_string()
        } else {
            let worktree_name = entry.branch.as_deref().unwrap_or(name);
            format!("<blue-500>{worktree_name}</blue-500> <i>worktree</i>")
        };
        let msg = format!("\nAlready in the {kind} of <yellow>{repo}</yellow>.");
        eprintln!("{}", Prose::new(msg).render(&terminal));
        return Ok(());
    }

    let location = if relative.as_os_str().is_empty() {
        "<dim><i>repo root</i></dim>".to_string()
    } else {
        format!("<dim>{relative_display}</dim>")
    };
    let location_msg = format!("at the same relative location ({location})");

    let msg = if entry.is_main {
        if path_adjusted {
            format!(
                "\nYou've been moved into the <blue-500>base</blue-500> <i>checkout</i> of <yellow>{repo}</yellow> <dim>(<b>{relative_display}</b> doesn't exist here — moved to <i>root</i>)</dim>"
            )
        } else {
            format!(
                "\nYou've been moved into the <blue-500>base</blue-500> <i>checkout</i> of <yellow>{repo}</yellow> {location_msg}"
            )
        }
    } else {
        let worktree_name = entry.branch.as_deref().unwrap_or(name);
        if path_adjusted {
            format!(
                "\nYou've been moved into the <blue-500>{worktree_name}</blue-500> <i>worktree</i> of <yellow>{repo}</yellow> <dim>(<b>{relative_display}</b> doesn't exist here — moved to <i>root</i>)</dim>"
            )
        } else {
            format!(
                "\nYou've been moved into the <blue-500>{worktree_name}</blue-500> <i>worktree</i> of <yellow>{repo}</yellow> {location_msg}"
            )
        }
    };

    // If stdout is a TTY the shell wrapper is not active — the cd: line would
    // print raw to the terminal and the directory would never change.
    if std::io::stdout().is_terminal() {
        let warn = "\n<red><b>Shell wrapper not active.</b></red> The directory cannot be changed.\n\
            Run this to activate it, then try again:\n\n\
            <dim>source <(wt --completions zsh)</dim>\n\n\
            Add that line to <dim>~/.zshrc</dim> to make it permanent.";
        eprintln!("{}", Prose::new(warn).render(&terminal));
        return Ok(());
    }

    eprintln!("{}", Prose::new(msg).render(&terminal));
    println!("cd:{}", target.display());

    Ok(())
}
