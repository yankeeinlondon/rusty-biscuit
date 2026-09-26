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
    Lists the worktrees (along with the base repo checkout) which currently exist. See [`docs/cli/list.md`](./docs/cli/list.md) for the full output.

    - a caption comparing the local default branch with `origin/<default>` (in sync, ahead, behind, or diverged)
    - a table with one row per worktree; the current row is highlighted
        - **Worktree**: a dot for uncommitted files (none, other files, or source files) and the directory name (`base repo` for the main checkout)
        - **Branch**: a tree of which branch was forked from which (recorded by `wt create`); deleted parents are struck through
        - **`-> {default}`**: `already in`, `clean`, or `conflicts` against the default branch (local or `origin/<default>`, whichever contains the other)
        - **`-> parent`**: the same against the branch's fork parent
        - open PRs from this repository as badges, placed by the PR's target; the request waits at most 300 ms and its answer is reused for 60 s
    - a legend, then, on image-capable terminals, a branch graph (see [`docs/git-graph.md`](./docs/git-graph.md))

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
        - Without the [shell integration](#shell-integration) the worktree is still created (exit 0), but `wt` says it could not move the shell
    - Fork base (`--from <base>`)
        - a new branch forks from the current branch by default; `--from <base>` forks it from another **local** branch instead (e.g. `wt create fix/x --from feat/theme`)
        - `--from` with a branch that already exists is an error, since the flag would be ignored; without `--from`, an existing branch is reused as-is with a notice
        - a `--from` value that is not a local branch (missing, remote-only, or a tag) is an error naming it
        - on a detached HEAD there is no current branch to fork from, so a new branch needs `--from`
        - the fork base is recorded per repository in the user cache (`<repo hash>.fork-origins.json`, beside the comparison cache); reusing an existing branch records nothing

- `wt go <worktree|base>`

    - moves the user to the specified worktree (or base)
    - communicates the change in directories with:
        - `\nYou've been moved into the <blue-500>{worktree}</blue-500> <i>worktree</i> of <yellow>{{repo}}</yellow> at the same relative location (<dim>{relative-path}</dim>)`
        - `\nYou've been moved into the <blue-500>base</blue-500> <i>checkout</i> of <yellow>{{repo}}</yellow> at the same relative location (<dim>{relative-path}</dim>)`

    - name resolution (shared with `wt remove`):
        - `base` is always the main checkout, whatever branch it has checked out
        - any other name matches a worktree's branch name or its directory name (the input is dasherized for the directory comparison), so `wt go main` goes to wherever `main` is actually checked out
        - a name that matches more than one worktree is an error listing each one's branch, directory, and path; `wt` never picks one
    - needs the [shell integration](#shell-integration); without it `wt go` changes nothing and exits 4

    > **Note:** the shell completions for `wt` are dynamic: they offer `base`, the base checkout's branch, and each worktree's branch name and directory name

- `wt remove <name> [--force-worktree] [--force-branch] [--force-remote]`

    Removes a worktree by branch or directory name and, when its commits are safe elsewhere, its local branch. It starts from one question: would removing this lose work?

    - **report first**: before asking or removing anything it prints the worktree's uncommitted files (a tree of up to 10, colored by kind, otherwise a bold red count), its ignored entries grouped by top-level folder (their contents are deleted too), the branch's safety tier and the ref that proves it, ahead/behind against the default branch, its copy on origin (as of your last fetch), and its PR; each question then starts after one blank line
    - **the worktree**: removed when it has no uncommitted or ignored files, or with `--force-worktree`; otherwise an interactive run asks (default No) and a non-interactive run removes nothing and exits 3
    - **the local branch**, by where its last commit is found:
        - *Safe*: on the default branch (local or `origin/<default>`), or the exact head of an open or merged PR from this repository; deleted
        - *Pretty safe*: on another local branch, a tag, or an `origin/*` branch whose live head `wt` checked with `git ls-remote` (3 s deadline, never prompting for credentials); deleted, and the report names where the commits still live
        - *Not safe*: nowhere else, or it could not be checked; an interactive run lists the commits and offers "keep" (default) or "delete"; a non-interactive run keeps it with a warning and still exits 0. `--force-branch` deletes it anyway (`git branch -D`)
    - **the branch on origin**: deleted only with `--force-remote` (which also closes any open PR), under a lease on the head the report showed, so a push made in between fails the deletion instead of losing commits. The report, the live check, and the deletion all use origin's push URL, and a move-first removal refuses if that URL changed before the second run; an origin with several push URLs, or whose push URL git would not take literally (a `url.<base>.insteadOf`/`pushInsteadOf` rule would rewrite it again, or it is also the name of a configured remote, as a relative path like `approved` can be), is refused before anything is removed. The tiers then ignore the branch's own origin copy, even when its upstream is the default branch on origin, and its open PR. If origin cannot be reached, the local removal still happens and `wt` exits 1 with the `git push origin --delete <branch>` that finishes the job
    - `--force-branch` on a worktree with files but without `--force-worktree` is a conflict (exit 3 when non-interactive)
    - **standing in the worktree**: with the [shell integration](#shell-integration) the first run asks everything, then moves your shell to the fork parent's worktree (or the base repo) and the wrapper finishes the removal from there, keeping your subdirectory when it exists there. If anything it would delete changed between the two steps (including files inside a nested repository, which git lists as one directory), or the handoff is over a minute old, nothing is removed; a file it cannot read stops it too (exit 4). Without the wrapper `wt remove` refuses (exit 4)
    - on Windows, a worktree folder another program holds (a cmd window or terminal tab opened in it, an open file) is detected before anything is deleted and refused with exit 4
    - the main checkout cannot be removed (use plain `git` for that), whether it is named `base` or by its branch
    - an interactive run needs stdin and stderr to be terminals and `CI` to be unset or empty

- `wt help`

    - shows the help system
    - the `help` command is not listed along with the others
    - alternatively running `wt --help` will also show the help system


## Shell Integration

A program cannot change its parent shell's directory, so `wt go` and `wt create` (and `wt remove` of the worktree you stand in) rely on a `wt` shell function generated by `wt --completions <shell>`. It also registers the dynamic completions.

| Shell | Add to | Line |
|---|---|---|
| bash | `~/.bashrc` | `source <(wt --completions bash)` |
| zsh | `~/.zshrc` | `source <(wt --completions zsh)` |
| fish | `config.fish` | `wt --completions fish \| source` |
| PowerShell | `$PROFILE` | `wt --completions powershell \| Out-String \| Invoke-Expression` |

The protocol between `wt` and the wrapper:

- the wrapper sets `WT_SHELL_WRAPPER=1` for exactly the `wt` calls it makes; `wt` prints protocol lines only when it sees that variable (a captured stdout is not proof, because scripts and agents capture it too)
- `cd:<path>` on stdout: the wrapper changes directory after `wt` exits successfully, and stops with an error if the change fails
- `remove-handoff:<token>` on stdout: after a successful `cd`, the wrapper runs the fixed command `wt remove --handoff <token>` with the token as one argument
- every other stdout line is printed unchanged; the wrapper never evaluates `wt`'s output as shell code
- the PowerShell wrapper calls the `wt` executable by the absolute path it was generated from (Windows Terminal installs its own `wt.exe` alias), decodes its output as UTF-8, and sets both `Set-Location` and `[Environment]::CurrentDirectory`

cmd.exe has no wrapper.

## Exit Codes

| Code | Meaning |
|---|---|
| 0 | Done, including a prompt the caller cancelled |
| 1 | Something failed |
| 2 | Invalid arguments (clap) |
| 3 | Refused to avoid losing work; nothing was changed |
| 4 | Blocked by the environment (for example, no shell wrapper); nothing was changed |

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

### Comparison Cache

`wt list` caches each `(ahead, behind, is_clean)` comparison by the pair of commit SHAs it compared (target tip and branch tip), so one cache serves the caption, the `-> {default}` column, and the `-> parent` column. Cache files live under the user cache directory in a `worktree` subdirectory, beside the fork-origin records and the PR store; the full path and invalidation rules are documented in [`docs/performance-testing.md`](./docs/performance-testing.md).

The cache self-invalidates when either tip changes. `CACHE_FORMAT_VERSION` forces invalidation when the on-disk shape or semantics change, and working-tree dirtiness is still measured live on every run.

See [`docs/performance-testing.md`](./docs/performance-testing.md) for the full performance contract.
