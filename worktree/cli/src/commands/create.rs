use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable as _;
use biscuit_terminal::terminal::Terminal;
use worktree::worktree::create_worktree;
use worktree::WorktreeError;

pub fn run(branch: &str, stay: bool) -> Result<(), WorktreeError> {
    let result = create_worktree(branch)?;
    let terminal = Terminal::default();

    let msg = format!(
        "\n<green>Created worktree</green> <bold>{}</bold> at <dim>{}</dim>\n",
        result.branch,
        result.worktree_path.display()
    );
    eprintln!("{}", Prose::new(msg).render(&terminal));

    if !stay {
        // Output cd: protocol for the shell wrapper
        println!("cd:{}", result.target_cwd.display());
    }

    Ok(())
}
