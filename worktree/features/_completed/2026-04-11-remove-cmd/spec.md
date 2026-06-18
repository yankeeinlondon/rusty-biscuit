# Remove command for Worktree CLI

## CLI Signature

`wt remove <name> [--force | -f] [--branch | -b]`

## Features

- Create a subcommand `wt remove <name>` to safely remove a worktree.
- **Uncommitted Changes Detection**:
    - Use the `sniff` library (specifically the `repo` module) to detect uncommitted changes and categorize "source code" files.
    - If uncommitted changes exist, present them using the `FileTree` component (from `darkmatter`/`biscuit-terminal`) to provide a clear hierarchical view.
- **Safety Dialogs**:
    - If the worktree has uncommitted changes, display the `FileTree` and a confirmation dialog.
    - If the worktree has no uncommitted changes, display a simple confirmation dialog.
- **Force Flags**:
    - `--force` / `-f`:
        - If no uncommitted changes: Remove without confirmation.
        - If < 10 files and no "source code" files (as defined by `sniff`): Remove without confirmation.
        - Otherwise: Show confirmation dialog with specific messages:
            - `"- the <blue>{worktree}</blue> has {#} files which have not been committed to <b>git</b>! Are you sure you want to remove this worktree? All file changes will be lost."`
            - `"- the <blue>{worktree}</blue> worktree has source code files in it which have not been committed to <b>git</b>! Are you sure you want to remove this worktree? All file changes will be lost."`
    - `-ff`: Remove immediately regardless of state or file types.
- **Branch Cleanup**:
    - If the `--branch` / `-b` flag is provided, attempt a **soft delete** (`git branch -d`) of the Git branch associated with the worktree after the worktree is successfully removed.
    - If the branch cannot be safely deleted (e.g., it is not merged into its upstream or HEAD), the branch should be preserved, and a warning should be displayed to the user.

## Success Criteria

- Worktree is removed from the filesystem and Git metadata.
- UI correctly renders uncommitted files in a tree format.
- `sniff` library is leveraged for reliable file categorization.
- Optional branch deletion works as expected.
