# Worktree

**Worktree** is a simple CLI for making working with [git worktrees](https://git-scm.com/docs/git-worktree) easier.

<img src="../assets/Worktree-512.png" style="width: 250px" />

## Packages

The **worktree** package area, like many in this monorepo, is composed of both a library and a CLI:

- **Library**(./lib)

    - the library encapsulates all the business logic and git functionality
    - this allows other library callers to use this functionality programmatically

- **CLI**(./cli)

    - provides a binary which allows anyone at the termianl to be able to leverage the features the library exposes
    - we leverage the `clap` and `clap_complete` crates to provide a high quality CLI with:
        - help system
        - shell completions

## CLI Commands

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
            - if WT is not set we will look for a `~/.worktree.json` file:

                The `~/.worktree.json` file is structured like:

                ```ts
                type worktree {
                    base_dir: string; 
                }
                ```

                - if the file exists and the file's structure is valid, the directory it points to is a valid directory, and that directory is NOT part of a git repo, we will use that directory as the base
                - if the file doesn't exist we will ask the user to choose a "base directory" from a set of choices:
                    - the following directories will be "considered": `~/worktrees`, `~/wt`, `~/.claudine/worktrees`
                    - each of the "considered" directories will be evaluated to make sure that they are NOT part of a git repo (as this would cause conflicts); any which are a part of a repo will be eliminated for consideration. 
                    - if these directories do NOT exist that is fine, they're considered "available"
                    - if we have at least one available option then we will use the `inquire` crate to create a select input for the user to choose between the found options, plus a "other" option
                    - if the "other" option is chosen then we will provide a text input for them to specify the directory manually
                - if the file _exists_ but has the wrong structure then we will provide an error which describes the problem and describes what the correct format is. Provide the user with the path to the config file (e.g., `~/.worktree.config`) as an OSC8 link (Prose struct in biscuit-terminal supports this with an easy syntax) and tell the user to edit the file or alternatively to delete the file and run `wt create` again for an interactive process.
                - if the file _exists_ but points to a directory which is in a git repo, we will provide an error which describes the problem and then provides the filepath to the config file (as an OSC8 link) and suggests the user to update this file. In this case, we should also just suggest some common choices are ... and list the same "considered" directories which passed from above.
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
    - communicates the change in directories with:
        - `\nYou've been moved into the <blue-500>{worktree}</blue-500> <i>worktree</i> of <yellow>{{repo}}</yellow> at the same relative location (<dim>{relative-path}</dim>)`
        - `\nYou've been moved into the <blue-500>base</blue-500> <i>checkout</i> of <yellow>{{repo}}</yellow> at the same relative location (<dim>{relative-path}</dim>)`

    > **Note:** the shell completions for `wt` will be updated dynamically and able to resolve the valid worktree names without the user needing to type them out

- `wt remove <name> [-f | -ff] [-b]`

    Removes a worktree by name (or branch name).

    - safety semantics:
        - clean worktrees prompt for confirmation (no force flag)
        - dirty worktrees (with uncommitted files) prompt for confirmation
        - `-f` / `--force` skips confirmation when safe (clean, or fewer than 10 non-source files)
        - `-ff` removes immediately regardless of state, skipping all confirmation
    - source-code awareness: uncommitted source files (e.g. `.rs`, `.ts`, `.py`) trigger a stronger warning than non-source files (e.g. `.md`, `.txt`)
    - `-b` / `--branch` also attempts a soft delete (`git branch -d`) of the worktree's branch after removal; if the branch is not fully merged, a warning is shown with a hint to use `git branch -D` to force-delete it
    - the main checkout cannot be removed (use plain `git` for that)

- `wt help`

    - shows the help system
    - the `help` command is not listed along with the others
    - alternatively running `wt --help` will also show the help system


> Important: always try to use `biscuit-terminal` renderable components to produce a nice looking output. `Prose` is the most commonly used component but `UnorderedList`, `BlockQuote`, or `Table` can also be very helpful.

## Tech Stack

- uses `clap` and `clap_complete` to provide the core CLI functionality (and shell completions)
- uses the `biscuit-terminal` library's _composable_ components like `UnorderedList`, `Prose`, etc. to render to the terminal beautifully
- all underlying git commands use the host's git program via shell commands (detects the absence of `git` when missing)

> **Note:** business logic,  shell command orchestration, and `git` detection are all provided as a small library. The primary consumer of that library being the CLI.

## Performance

### Runtime diagnostic

Run `wt list --perf` to emit a per-stage timing report to stderr after the command completes. The report is rendered as a reconciling tree: recorded stages plus an `unattributed` node sum to the total wall-clock time. Only stages that actually ran are shown, so on a non-image terminal the graph-related stages are omitted.

### Dev-time benchmarks

`just bench` runs Criterion benches for the library-owned `list_worktrees()` gather stage. The HTML report is written to `target/criterion/report/index.html`.

Use `just bench-save` to capture a host-derived baseline before a change, then `just bench-compare` after the change to see the delta. The shared bench helpers run a preflight check (battery, memory, load) and use a host-derived baseline ID so comparisons stay on the same machine.

See [`docs/performance-testing.md`](./docs/performance-testing.md) for the full performance contract.
