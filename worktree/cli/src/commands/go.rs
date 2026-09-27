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

    if !crate::env::shell_wrapper_active() {
        return Err(WorktreeError::BlockedByEnvironment(format!(
            "\n<red><b>Shell wrapper not active.</b></red> The directory cannot be changed.\n{}",
            wrapper_setup_help()
        )));
    }

    eprintln!("{}", Prose::new(msg).render(&terminal));
    println!("cd:{}", target.display());

    Ok(())
}

/// How to activate the shell wrapper, for every shell `wt --completions`
/// supports. Shared by every command that moves the caller's shell.
pub(crate) fn wrapper_setup_help() -> String {
    "Run the line for your shell to activate it, then try again:\n\n\
    <dim>bash</dim>        source <(wt --completions bash)\n\
    <dim>zsh</dim>         source <(wt --completions zsh)\n\
    <dim>fish</dim>        wt --completions fish | source\n\
    <dim>PowerShell</dim>  wt --completions powershell | Out-String | Invoke-Expression\n\n\
    Add it to your shell's startup file (<dim>~/.bashrc</dim>, <dim>~/.zshrc</dim>, \
    <dim>config.fish</dim>, or <dim>$PROFILE</dim>) to make it permanent."
        .to_string()
}
