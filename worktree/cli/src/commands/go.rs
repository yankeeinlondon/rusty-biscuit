use worktree::git::repo_info;
use worktree::worktree::find_worktree;
use worktree::WorktreeError;

pub fn run(name: &str) -> Result<(), WorktreeError> {
    let entry = find_worktree(name)?;

    // Preserve relative path within the worktree
    let info = repo_info()?;
    let target = entry.path.join(&info.relative_path);

    // Output cd: protocol for the shell wrapper
    println!("cd:{}", target.display());

    Ok(())
}
