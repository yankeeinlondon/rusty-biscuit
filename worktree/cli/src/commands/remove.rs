use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use inquire::{Confirm, InquireError};
use worktree::WorktreeError;
use worktree::worktree::{
    DeleteBranchOutcome, DirtyFiles, delete_branch, find_worktree, list_dirty_files,
    remove_worktree,
};

use super::dirty_tree;

/// Threshold (per spec): below this count, a non-source-only dirtiness still
/// allows `-f` to skip the confirmation prompt.
const FORCE_BYPASS_FILE_LIMIT: usize = 10;

pub fn run(name: &str, force: u8, delete_branch_flag: bool) -> Result<(), WorktreeError> {
    let terminal = Terminal::default();

    let entry = find_worktree(name)?;

    // Disallow removing the main checkout — git itself rejects this, but
    // surface a friendlier error before we run any prompts.
    if entry.is_main {
        return Err(WorktreeError::GitCommand(format!(
            "refusing to remove the main checkout '{name}'. Use plain git for that."
        )));
    }

    let dirty = list_dirty_files(&entry.path);
    let display_name = entry.branch.clone().unwrap_or_else(|| name.to_string());

    // Decide whether to prompt, based on `force` count + dirty state.
    let should_prompt = decide_prompt(force, &dirty);

    if should_prompt {
        render_dirty_summary(&terminal, &display_name, &dirty);
        let prompt_msg = build_prompt_message(&display_name, &dirty);
        if !confirm(&prompt_msg)? {
            let cancelled = format!("<dim>Cancelled. Worktree <blue>{display_name}</blue> was not removed.</dim>");
            eprintln!("{}", Prose::new(cancelled).render(&terminal));
            return Ok(());
        }
    }

    // Anything past this point removes the worktree. Force the underlying git
    // call when the user passed any -f or when dirty files exist (so git
    // doesn't reject the removal in confirmed flows).
    let force_git = force > 0 || dirty.status() != worktree::worktree::DirtyStatus::Clean;
    remove_worktree(&entry.path, force_git)?;

    let removed = format!(
        "\n<green>Removed worktree</green> <bold>{display_name}</bold> at <dim>{}</dim>",
        entry.path.display()
    );
    eprintln!("{}", Prose::new(removed).render(&terminal));

    if delete_branch_flag {
        if let Some(branch) = entry.branch.as_deref() {
            match delete_branch(branch) {
                DeleteBranchOutcome::Deleted => {
                    let msg = format!("<green>Deleted branch</green> <bold>{branch}</bold>");
                    eprintln!("{}", Prose::new(msg).render(&terminal));
                }
                DeleteBranchOutcome::Preserved { reason } => {
                    let msg = format!(
                        "<yellow><b>Warning:</b></yellow> branch <bold>{branch}</bold> was preserved: <dim>{reason}</dim>\n  \
                        <dim>Run <i>git branch -D {branch}</i> if you want to force-delete it.</dim>"
                    );
                    eprintln!("{}", Prose::new(msg).render(&terminal));
                }
            }
        } else {
            let msg = format!(
                "<yellow><b>Warning:</b></yellow> no branch associated with <bold>{display_name}</bold>; skipping branch cleanup."
            );
            eprintln!("{}", Prose::new(msg).render(&terminal));
        }
    } else if let Some(branch) = entry.branch.as_deref() {
        // Plain remove keeps the branch. Say so explicitly — a silently retained
        // branch is what a later `wt create <name>` reuses as-is (stale commit).
        let notice = build_branch_kept_notice(branch, entry.head_sha.as_deref());
        eprintln!("{}", Prose::new(notice).render(&terminal));
    }

    Ok(())
}

/// Notice shown when a plain `remove` deleted the worktree but kept its branch.
/// Surfaces the commit so the lingering branch is not a silent surprise when a
/// later `create` of the same name reuses it as-is.
fn build_branch_kept_notice(branch: &str, head_sha: Option<&str>) -> String {
    let at = head_sha
        .map(|s| format!(" (<dim>{}</dim>)", &s[..s.len().min(9)]))
        .unwrap_or_default();
    format!(
        "<yellow><b>Note:</b></yellow> branch <bold>{branch}</bold>{at} was kept. \
        Re-running <i>wt create {branch}</i> reuses it as-is.\n  \
        <dim>Delete it with <i>wt remove {branch} -b</i> or <i>git branch -d {branch}</i>.</dim>"
    )
}

/// Decide whether to show the confirmation prompt, per the spec matrix.
///
/// `force` is the count of `-f` flags:
/// - 0 (no flag): prompt whenever dirty (any kind).
/// - 1 (`-f` / `--force`): skip prompt when clean OR (non-source AND < 10 files).
/// - 2+ (`-ff`): never prompt.
fn decide_prompt(force: u8, dirty: &DirtyFiles) -> bool {
    if force >= 2 {
        return false;
    }
    if dirty.paths.is_empty() {
        // Clean worktree: prompt only when no force flag is given.
        return force == 0;
    }
    if force == 0 {
        return true;
    }
    // force == 1: bypass when no source files AND under the file-count threshold.
    if !dirty.has_source && dirty.paths.len() < FORCE_BYPASS_FILE_LIMIT {
        return false;
    }
    true
}

fn render_dirty_summary(terminal: &Terminal, display_name: &str, dirty: &DirtyFiles) {
    if dirty.paths.is_empty() {
        return;
    }
    let header = format!(
        "\n<b>Worktree <blue>{display_name}</blue> has {} uncommitted file(s):</b>",
        dirty.paths.len()
    );
    eprintln!("{}", Prose::new(header).render(terminal));
    let tree = dirty_tree::render_markup(&dirty.paths);
    eprint!("{}", Prose::new(tree).render(terminal));
}

fn build_prompt_message(display_name: &str, dirty: &DirtyFiles) -> String {
    let count = dirty.paths.len();
    if dirty.has_source {
        format!(
            "- the <blue>{display_name}</blue> worktree has source code files in it which \
            have not been committed to <b>git</b>! Are you sure you want to remove this \
            worktree? All file changes will be lost."
        )
    } else if count > 0 {
        format!(
            "- the <blue>{display_name}</blue> has {count} non-source files which have not been \
            committed to <b>git</b>! Are you sure you want to remove this worktree? \
            All file changes will be lost."
        )
    } else {
        format!("- remove worktree <blue>{display_name}</blue>?")
    }
}

fn confirm(message_markup: &str) -> Result<bool, WorktreeError> {
    let terminal = Terminal::default();
    let rendered = Prose::new(message_markup.to_string()).render(&terminal);
    Confirm::new(&rendered)
        .with_default(false)
        .prompt()
        .map_err(map_inquire_err)
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
    use std::path::PathBuf;

    fn dirty_with(paths: Vec<&str>, has_source: bool) -> DirtyFiles {
        DirtyFiles {
            paths: paths.into_iter().map(PathBuf::from).collect(),
            has_source,
        }
    }

    #[test]
    fn no_force_clean_prompts() {
        assert!(decide_prompt(0, &DirtyFiles::default()));
    }

    #[test]
    fn no_force_dirty_prompts() {
        let d = dirty_with(vec!["README.md"], false);
        assert!(decide_prompt(0, &d));
    }

    #[test]
    fn force_one_clean_skips() {
        assert!(!decide_prompt(1, &DirtyFiles::default()));
    }

    #[test]
    fn force_one_few_non_source_skips() {
        let d = dirty_with(vec!["a.txt", "b.md"], false);
        assert!(!decide_prompt(1, &d));
    }

    #[test]
    fn force_one_many_non_source_prompts() {
        let paths: Vec<&str> = (0..FORCE_BYPASS_FILE_LIMIT)
            .map(|_| "x.txt")
            .collect();
        let d = dirty_with(paths, false);
        assert!(decide_prompt(1, &d));
    }

    #[test]
    fn force_one_source_prompts() {
        let d = dirty_with(vec!["src/lib.rs"], true);
        assert!(decide_prompt(1, &d));
    }

    #[test]
    fn force_two_never_prompts() {
        let d = dirty_with(vec!["src/lib.rs", "a.md", "b.md"], true);
        assert!(!decide_prompt(2, &d));
    }

    #[test]
    fn prompt_message_source_variant() {
        let d = dirty_with(vec!["src/lib.rs"], true);
        let msg = build_prompt_message("feat-x", &d);
        assert!(msg.contains("source code files"));
        assert!(msg.contains("<blue>feat-x</blue>"));
    }

    #[test]
    fn prompt_message_non_source_variant() {
        let d = dirty_with(vec!["a.md", "b.md"], false);
        let msg = build_prompt_message("feat-x", &d);
        assert!(msg.contains("has 2 non-source files"));
        assert!(msg.contains("<blue>feat-x</blue>"));
    }

    #[test]
    fn branch_kept_notice_includes_branch_and_short_sha() {
        let msg = build_branch_kept_notice("w-cli-heretic", Some("d4ea98a40942d4afb"));
        assert!(msg.contains("w-cli-heretic"));
        assert!(msg.contains("d4ea98a40")); // SHA truncated to 9 chars
        assert!(msg.contains("wt create w-cli-heretic"));
        assert!(msg.contains("wt remove w-cli-heretic -b"));
    }

    #[test]
    fn branch_kept_notice_without_sha_omits_parens() {
        let msg = build_branch_kept_notice("feat-x", None);
        assert!(msg.contains("feat-x"));
        assert!(!msg.contains("()"));
    }
}
