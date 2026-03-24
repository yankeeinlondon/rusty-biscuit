use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable as _;
use biscuit_terminal::terminal::Terminal;
use worktree::git::repo_info;
use worktree::worktree::find_worktree;
use worktree::WorktreeError;

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

    let msg = if entry.is_main {
        if path_adjusted {
            format!(
                "\nYou've been moved into the <blue-500>base</blue-500> <i>checkout</i> of <yellow>{repo}</yellow>. <dim>({relative_display} doesn't exist here — moved to root)</dim>"
            )
        } else {
            format!(
                "\nYou've been moved into the <blue-500>base</blue-500> <i>checkout</i> of <yellow>{repo}</yellow> at the same relative location (<dim>{relative_display}</dim>)"
            )
        }
    } else {
        let worktree_name = entry.branch.as_deref().unwrap_or(name);
        if path_adjusted {
            format!(
                "\nYou've been moved into the <blue-500>{worktree_name}</blue-500> <i>worktree</i> of <yellow>{repo}</yellow>. <dim>({relative_display} doesn't exist here — moved to root)</dim>"
            )
        } else {
            format!(
                "\nYou've been moved into the <blue-500>{worktree_name}</blue-500> <i>worktree</i> of <yellow>{repo}</yellow> at the same relative location (<dim>{relative_display}</dim>)"
            )
        }
    };

    eprintln!("{}", Prose::new(msg).render(&terminal));
    println!("cd:{}", target.display());

    Ok(())
}
