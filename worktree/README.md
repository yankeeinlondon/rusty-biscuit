# Worktree

**Worktree** is a simple CLI for making working with [git worktrees](https://git-scm.com/docs/git-worktree) easier.


## Commands

- `wt list`
    Lists the worktree's (along with the base repo checkout) which currently exist.

    - the base/worktree the user is currently in is highlighted
    - an indication of whether the each worktree is "clean" (aka, could be merged back into main/master without any conflicts) or "conflict"
    - an indication of how many commits each worktree is ahead/behind of main/master

    > The **list** command is the default command so it will be run if a user types only `wt`

- `wt create <branch>`

    Allows you to create a new worktree by just providing the branch name you want to use.

    - Directory Resolution
        - Establishing the base directory we `wt` will use for all worktrees it creates:
            - if the ENV variable `WT` is set and points to a valid directory (which is NOT a git repo) we will use that
            - if WT is _not_set we will look for a `~/.worktree.json` file:

                The `~/.worktree.json` file is structured like:

                ```ts
                type worktree {
                    base_dir: string; 
                }
                ```

                - if the file exists and the directory it points to is a valid directory (which is NOT a git repo) we will use that directory as the base
                - if either the file doesn't exist or it points to an invalid directory we will report a descriptive error
        - Repo Info:
            - we will detect if the current working directory is inside a git repo and if it's not return with a descriptive error
            - if we are in a git repo, we will detect:
                - repo name
                - relative path in repo
        - We now have enough information to fully execute the git command and are able to place the new git worktree in the right location
        - The location we'll put worktrees is: `{base}/{repo-name}/{dasherized-branch-name}/`
        - Once we've created the worktree we will change the current working directory to the same relative path in the repo but inside the new worktree path
        - We will report to the user that they have been moved into the new worktree
        - An optional `--stay` switch will create the worktree but not move into it

- `wt go <worktree|base>`

    - moves the user to the specified worktree (or base)

    > **Note:** the shell completions for `wt` will be updated dynamically and able to resolve the valid worktree names without the user needing to type them out

- `wt help`

    - shows the help system
    - the `help` command is not listed along with the others
    - alternatively running `wt --help` will also show the help system

## Tech Stack

- uses `clap` and `clap_complete` to provide the core CLI functionality (and shell completions)
- uses the `biscuit-terminal` library's _composable_ components like `UnorderedList`, `Prose`, etc. to render to the terminal beautifully
- all underlying git commands use the host's git program via shell commands (detects the absence of `git` when missing)

> **Note:** business logic,  shell command orchestration, and `git` detection are all provided as a small library. The primary consumer of that library being the CLI.
