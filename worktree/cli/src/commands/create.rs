use std::path::{Path, PathBuf};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use inquire::{InquireError, Select, Text};
use worktree::WorktreeError;
use worktree::config::{config_path, considered_dirs, resolve_base_dir, save_config};
use worktree::git::repo_info;
use worktree::worktree::create_worktree;

pub fn run(branch: &str, stay: bool) -> Result<(), WorktreeError> {
    let terminal = Terminal::default();

    let base = match resolve_base_dir() {
        Ok(path) => path,
        Err(WorktreeError::BaseDirectoryNotConfigured) => prompt_for_base_dir(&terminal)?,
        Err(WorktreeError::ConfigInvalidFormat {
            config_path,
            message,
        }) => {
            render_invalid_format_error(&terminal, &config_path, &message, branch);
            return Ok(());
        }
        Err(WorktreeError::ConfigBaseDirIsGitRepo { config_path, dir }) => {
            render_git_repo_error(&terminal, &config_path, &dir);
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    let result = create_worktree(branch, &base)?;

    let relative = repo_info().map(|i| i.relative_path).unwrap_or_default();
    let location = if relative.as_os_str().is_empty() {
        "<dim><i>repo root</i></dim>".to_string()
    } else {
        format!("<dim>{}</dim>", relative.to_string_lossy())
    };
    let msg = format!(
        "\n<green>Created worktree</green> <bold>{}</bold> at <dim>{}</dim>\n  <dim>- you have been moved <i>into</i> the worktree at the same relative path ({})</dim>\n",
        result.branch,
        result.worktree_path.display(),
        location,
    );
    eprintln!("{}", Prose::new(msg).render(&terminal));

    if let Some(commit) = &result.reused_branch_at {
        let notice = build_reuse_notice(&result.branch, commit);
        eprintln!("{}", Prose::new(notice).render(&terminal));
    }

    if !stay {
        println!("cd:{}", result.target_cwd.display());
    }

    Ok(())
}

/// Notice shown when `create` re-attached a pre-existing branch instead of
/// forking a fresh one. Surfaces the commit so a stale branch is not reused
/// silently.
fn build_reuse_notice(branch: &str, commit: &str) -> String {
    format!(
        "<yellow><b>Note:</b></yellow> branch <bold>{branch}</bold> already existed and was \
        reused at <dim>{commit}</dim> — <b>not</b> forked from your current branch.\n  \
        <dim>Delete it first (<i>git branch -d {branch}</i>) if you wanted a fresh fork.</dim>"
    )
}

fn render_invalid_format_error(
    terminal: &Terminal,
    config_path: &Path,
    message: &str,
    branch: &str,
) {
    let path_str = config_path.display().to_string();
    let msg = format!(
        "\n<red><b>Error:</b></red> <b>~/.worktree.json</b> has an invalid format: {message}\n\n\
        The file must be structured as:\n\n\
        <dim>{{</dim>\n\
        <dim>  \"base_dir\": \"/path/to/your/worktrees\"</dim>\n\
        <dim>}}</dim>\n\n\
        <a href=\"{path_str}\">Edit ~/.worktree.json</a> to fix the format, \
        or delete the file and run <dim>wt create {branch}</dim> again for an interactive setup."
    );
    eprintln!("{}", Prose::new(msg).render(terminal));
}

fn render_git_repo_error(terminal: &Terminal, config_path: &Path, dir: &str) {
    let path_str = config_path.display().to_string();
    let suggestions: Vec<String> = considered_dirs()
        .into_iter()
        .filter(|p| !p.join(".git").exists())
        .map(|p| format!("  • <dim>{}</dim>", p.display()))
        .collect();
    let suggestions_str = if suggestions.is_empty() {
        String::new()
    } else {
        format!("\n\nCommon choices:\n{}", suggestions.join("\n"))
    };
    let msg = format!(
        "\n<red><b>Error:</b></red> The configured base directory <b>{dir}</b> is itself a \
        git repository. Worktrees cannot be stored inside a git repo.\n\n\
        <a href=\"{path_str}\">Edit ~/.worktree.json</a> to point to a plain directory.{suggestions_str}"
    );
    eprintln!("{}", Prose::new(msg).render(terminal));
}

fn prompt_for_base_dir(terminal: &Terminal) -> Result<PathBuf, WorktreeError> {
    let intro = "\n<b>No worktree base directory configured.</b> Let's set one up.\n\n\
        This is where <dim>wt</dim> will store all worktrees, organized as \
        <dim>{base}/{repo}/{branch}/</dim>. Your choice will be saved to \
        <dim>~/.worktree.json</dim>.";
    eprintln!("{}", Prose::new(intro).render(terminal));

    let candidates = considered_dirs();

    // Filter out directories that are inside a git repo
    let valid: Vec<PathBuf> = candidates
        .into_iter()
        .filter(|p| !p.join(".git").exists())
        .collect();

    const OTHER: &str = "Other (specify manually)";
    let mut options: Vec<String> = valid.iter().map(|p| p.display().to_string()).collect();
    options.push(OTHER.to_string());

    let selection = Select::new("Choose a base directory:", options)
        .prompt()
        .map_err(map_inquire_err)?;

    let path = if selection == OTHER {
        let input = Text::new("Base directory path:")
            .prompt()
            .map_err(map_inquire_err)?;
        PathBuf::from(input.trim())
    } else {
        PathBuf::from(&selection)
    };

    // Create the directory if it doesn't already exist
    if !path.exists() {
        std::fs::create_dir_all(&path)?;
    }

    // Validate: must not itself be a git repo
    if path.join(".git").exists() {
        let msg = format!(
            "\n<red><b>Error:</b></red> <b>{}</b> is itself a git repository. \
            Worktrees cannot be stored inside a git repo. Please choose a different directory.",
            path.display()
        );
        eprintln!("{}", Prose::new(msg).render(terminal));
        return Err(WorktreeError::BaseDirectoryIsGitRepo(
            path.display().to_string(),
        ));
    }

    save_config(&path)?;

    let config_str = config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "~/.worktree.json".to_string());
    let confirm = format!(
        "\n<green>Saved</green> base directory <dim>{}</dim> to <dim>{config_str}</dim>.",
        path.display()
    );
    eprintln!("{}", Prose::new(confirm).render(terminal));

    Ok(path)
}

fn map_inquire_err(e: InquireError) -> WorktreeError {
    match e {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => {
            WorktreeError::Cancelled
        }
        e => WorktreeError::Io(std::io::Error::other(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reuse_notice_names_branch_commit_and_flags_no_fork() {
        let msg = build_reuse_notice("w-cli-heretic", "d4ea98a");
        assert!(msg.contains("w-cli-heretic"));
        assert!(msg.contains("d4ea98a"));
        assert!(msg.contains("not"));
    }
}
