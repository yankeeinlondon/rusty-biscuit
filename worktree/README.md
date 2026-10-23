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

Run repository commands from inside a Git repository or one of its worktrees.
Outside a repository, `wt` explains where to run it and exits with code 1.

- `wt list`
    Lists the worktrees (along with the base repo checkout) which currently exist. See [`docs/cli/list.md`](./docs/cli/list.md) for the full output.

    - first, two questions for `origin`, asked side by side by a background process: the default branch's current commit (through the provider API, falling back to `git ls-remote`; when it differs from `origin/<default>`, that one tracking ref is fetched), and the open pull requests. An ordinary listing waits up to 3 s for both, with a spinner on terminals; anything still running carries on in the background and the listing says so. `-r` and `--ff` wait for both to finish, up to 75 s. The local default branch is never moved unless you ask with `--ff`
    - meanwhile the local work runs: `git status` in every worktree Git can read, the branch comparisons, and the graph. Afterwards the branch tips are read again; if a fetch or `--ff` moved any of them, the comparisons and graph are worked out again from the new tips, so everything shown describes one state. Uncommitted-file status is not measured again, except in a checkout whose files `--ff` just moved
    - a one-sentence caption comparing the local default branch with `origin/<default>` (in sync, ahead, behind, or diverged), followed in dim italics by what this run learned (`updated from origin just now`, `still checking in the background`, `couldn't check origin`, …; nothing when a fresh check found the tracking ref current). Without an `origin` there is no caption
    - a table with one row per worktree; the current row is highlighted
        - **Worktree**: a dot for uncommitted files (none, other files, or source files), `✕` when Git can't read the worktree or its path is a link that may have replaced it (a closing note says why and how to recover), or `?` when `git status` failed, and the directory name (`base repo` for the main checkout). `wt list` never repairs or removes a worktree
        - **Branch**: a tree of which branch was forked from which (recorded by `wt create`); deleted parents are struck through
        - **`-> {default}`**: `clean` or `conflicts` against the default branch (local or `origin/<default>`, whichever contains the other); from 100 columns, followed by `+ahead` and `-behind` commit counts
        - **`-> parent`**: the same against the branch's fork parent (its local branch)
        - open PRs from this repository as badges, placed by the PR's target. Every listing asks again and shows the answer that arrives within its wait; otherwise it shows the last stored answer for the current `origin`, with a dim item saying how old it is (`PRs as of N min ago`) or that this run `couldn't refresh` it (`couldn't get open PRs` when nothing is stored)
    - a legend, then, on image-capable terminals, a branch graph (see [`docs/git-graph.md`](./docs/git-graph.md))
    - notes when they apply: one credentials line, either a warning when a missing or rejected API key (or a rate limit) stopped the check or the PR request, or, failing that, a notice that the provider answered without an API key (with the variables to set), a hint to run again when the wait ran out, and closing notes suggesting `wt --ff` when the default branch is behind or explaining the `ls-remote` fallback
    - flags (listing only): `-r`/`--refresh` waits for a full update from `origin`; `--ignore-api` makes this repository use `git ls-remote` only, recorded in `~/.wt.json`; `--ff`/`--fast-forward` fast-forwards the local default branch to `origin/<default>`

    > The **list** command is the default command so it will be run if a user types only `wt`

- `wt create <branch>`

    Allows you to create a new worktree by just providing the branch name you want to use.

    - Directory Resolution (full rules in [`docs/base-directory.md`](./docs/base-directory.md))
        - the base directory comes from the `WT` environment variable, or, when `WT` is unset or exactly empty, from `base_dir` in `~/.worktree.json` (`{"base_dir": "/home/me/wt"}`)
        - any other `WT` value is used or rejected; an invalid `WT` never falls back to the config file
        - the value must be an absolute path on this host (`/home/me/wt` on Linux and WSL, `C:\Users\me\wt` on Windows) naming an existing directory that does not have `.git` directly inside it; directories deeper inside a repository are allowed
        - nothing is expanded or made relative to the current directory: `wt`, `./wt`, and `~/wt` are rejected, and a path written for another operating system (`b:\wt` on Linux, macOS, or WSL; `/home/me/wt` on Windows) is rejected as foreign syntax. Under WSL, a drive path also gets a `/mnt/b/wt` suggestion
        - every rejection exits 1 before anything is created, naming the value, its source (`WT` or the config file, linked), and the fix; a missing directory is never recreated, since it may be an unmounted drive
        - a malformed `~/.worktree.json` (not UTF-8 JSON with one string `base_dir`, a repeated key, trailing text) exits 1 with the expected format
        - when neither source is set:
            - in an interactive terminal (stdin and stderr are terminals and `CI` is unset or empty), `wt` offers `~/worktrees`, `~/wt`, `~/.claudine/worktrees` (minus repository roots) and "Other" for a typed path. An unusable choice shows why and asks again; nothing is created or saved until the path passes the rules above. A missing directory is created, checked, then saved to `~/.worktree.json`. A creation failure exits 1; cancelling exits 0
            - otherwise `wt create` exits 1 with instructions to set `WT` or write `~/.worktree.json`, and never prompts
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
    - Ignored files selected by a root `.worktreeinclude` are copied from the checkout holding the fork source branch, or from the base checkout if that branch has no checkout. Reusing a branch always copies from the base checkout. See [Included ignored files](#included-ignored-files).

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

- `wt remove <name> [--force] [--force-branch] [--force-remote]`

    Removes a worktree by branch or directory name and, when its commits are safe elsewhere, its local branch. See [`docs/cli/remove.md`](./docs/cli/remove.md) for the full rules and examples.

    - **one question**: it first prints everything that makes the removal risky (a `git worktree lock`, programs using the worktree, uncommitted files, changed included files, links, mounts, submodule work) and the branch's safety tier, then asks once, default No. Yes deletes the checkout in that run, including files created while it runs; the final line says how many entries went and names uncommitted files that changed after you answered
    - **scripts**: a non-interactive run (stdin or stderr not a terminal, or `CI` set) removes a worktree with no warnings without asking, and refuses with exit 3 when there is anything to warn about. `--force` answers yes; it never implies `--force-branch` or `--force-remote`
    - **the local branch** is deleted when it is *Safe* (on the default branch, or the head of an open or merged PR) or *Pretty safe* (on another branch, a tag, or a live `origin/*` copy); a *Not safe* branch is offered for deletion interactively (default keep), kept with a warning otherwise, and deleted by `--force-branch`. A branch that moved after it was checked is kept and the run exits 1
    - **the branch on origin** is deleted only with `--force-remote` (which also closes any open PR), through origin's push URL and under a lease on the head the report showed
    - **standing in the worktree**: with the [shell integration](#shell-integration) the first run asks, then moves your shell out and the wrapper finishes the removal; without it, macOS and Linux warn and delete, and Windows refuses (exit 4)
    - **an interrupted removal** (Ctrl-C, a crash, a file that could not be deleted) is finished by the next `wt remove <name>` without asking again
    - the main checkout is never removed, even with `--force`

- `wt help`

    - shows the help system
    - the `help` command is not listed along with the others
    - alternatively running `wt --help` will also show the help system


## Included ignored files

Put a `.worktreeinclude` file at the repository root to copy local files that a fresh checkout needs. This filename is shared with [Claude Code](https://code.claude.com/docs/en/worktrees) and [Worktrunk](https://worktrunk.dev/step/#wt-step-copy-ignored); their other lifecycle policies may differ. Commit the pattern file, not the secrets it names. A file is selected only when it matches both `.worktreeinclude` and Git's normal ignore rules. Tracked files and ordinary untracked files are never copied this way.

Patterns use Git's `.gitignore` syntax: comments, blank lines, `/` anchoring, `**/`, directory patterns, and ordered `!` negation. Only the root `.worktreeinclude` is read. A negated child cannot recover a directory excluded earlier by the same pattern set. For example, `config/` followed by `!config/secrets.json` still excludes `config/secrets.json`; use `config/*` followed by `!config/secrets.json` when you need that exception. See [Git's pattern rules](https://git-scm.com/docs/gitignore).

`wt create` copies selected files, preserving link targets and Unix permissions. It tries copy-on-write cloning where the filesystem supports it, then falls back to a byte copy. The new worktree and source can be edited independently. Copy failures warn and leave the new worktree in place. No `.worktreeinclude` means no files are copied.

At removal, the target worktree's own `.worktreeinclude` decides which ignored files are protected. If it is missing, `wt` uses the base checkout's rules against the target's ignored files. An empty file selects nothing and suppresses that fallback. Changing or deleting patterns can make previously protected files disposable. Rules that can't be read are a warning in the removal question.

| Included file at removal | Warning |
|---|---|
| New, changed since copying, or impossible to compare | Yes; a non-interactive run needs `--force` |
| Unchanged since copying | No |
| Other ignored file, such as `target/` | No |

`wt` keeps a per-worktree copy record in the user cache. A size difference proves a change without reading the file; every same-size included file is compared by its full content, regardless of size or modification time. Links are compared by kind and target. Without a trusted record, `wt` compares with a distinct copy source; if that cannot be determined or read, consent is required. The cache holds no file contents and is not a backup. An unchanged copy is removed without a question even if it has become the last copy of its original contents because the source was later edited or deleted.

Prefer specific files such as `.env` and `config/secrets.json` over large ignored trees such as `node_modules/`: Git must enumerate matching files, and removal must read every same-size selected file.

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
| 1 | Something failed, including a `wt remove` that could not finish (a file that could not be deleted, a mount still mounted, a branch that was kept); the next `wt remove` of that name finishes it |
| 2 | Invalid arguments (clap) |
| 3 | Refused to avoid losing work (for example, `wt remove` without a terminal and without `--force` when there is something to warn about); nothing was removed, though `wt remove` may have repaired a worktree's `.git` link first (the message says so) |
| 4 | Blocked by the environment (for example, `wt go` without the shell wrapper, or on Windows `wt remove` of the worktree the shell stands in); nothing was removed |

> Important: always try to use `biscuit-terminal` renderable components to produce a nice looking output. `Prose` is the most commonly used component but `UnorderedList`, `BlockQuote`, or `Table` can also be very helpful.

## Tech Stack

- uses `clap` and `clap_complete` to provide the core CLI functionality (and shell completions)
- uses the `biscuit-terminal` library's _composable_ components like `UnorderedList`, `Prose`, etc. to render to the terminal beautifully
- all underlying git commands use the host's git program via shell commands (detects the absence of `git` when missing)

> **Note:** business logic,  shell command orchestration, and `git` detection are all provided as a small library. The primary consumer of that library being the CLI.

## Performance

### Runtime diagnostic

Run `wt list --perf` to emit a per-stage timing report to stderr after the listing. The report is a reconciling tree of named steps (for example `read worktrees and refs`, `graph history`, `table render`): each level's sequential steps plus a generated `unattributed` row (hidden under 1 ms) or `over-attributed` row sum to their parent, and a step that starts `git` shows its count, as in `[12 git]`. Work that runs at the same time is one concurrent group (`refresh worker ‖ local reads`, or `local reads` when no wait ran), whose children show their own durations with no share. Only steps that actually ran are shown. When the listing followed the background refresh, a separate section, `Refresh worker (diagnostic, measured in the worker)`, lists each launch's own measurements (its setup, then the PR and live-head halves side by side) with no share of the total, since they ran in another process; they reconcile with the same `unattributed` and `over-attributed` rows. Its heading says whether they are `complete`, `partial` (a half stopped early), `missing` (the listing ended before the worker reported), `invalid`, `adopted` (another run's check was followed), or `origin changed`. `wt list --perf=json` instead ends stderr with one line, `WT_PERF_JSON ` followed by the same report as a versioned JSON document, for scripts and tests; read only the final nonempty line. The measuring is the library's: `worktree::list::gather` with `ListOptions::timings` set returns the same document (scope `library`, without the CLI's startup and render steps) in `Listing::timings`.

### Dev-time benchmarks

`just bench` runs Criterion benches for the local half of the listing, the library's `worktree::list::gather` with no refresh worker, graph, or timings. The HTML report is written to `target/criterion/report/index.html`.

Use `just bench-save` to capture a host-derived baseline before a change, then `just bench-compare` after the change to see the delta. The shared bench helpers run a preflight check (battery, memory, load) and use a host-derived baseline ID so comparisons stay on the same machine.

### Comparison Cache

`wt list` caches each `(ahead, behind, is_clean)` comparison by the pair of commit SHAs it compared (target tip and branch tip), so one cache serves the caption, the `-> {default}` column, and the `-> parent` column. Cache files live under the user cache directory in a `worktree` subdirectory, beside the fork-origin records and the PR store; the full path and invalidation rules are documented in [`docs/performance-testing.md`](./docs/performance-testing.md).

The cache self-invalidates when either tip changes. `CACHE_FORMAT_VERSION` forces invalidation when the on-disk shape or semantics change, and working-tree dirtiness is still measured live on every run.

See [`docs/performance-testing.md`](./docs/performance-testing.md) for the full performance contract.
