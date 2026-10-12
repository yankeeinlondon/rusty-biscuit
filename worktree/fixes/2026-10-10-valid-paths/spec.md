---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: draft-spec
implemented: false
reviewed: false
review_iterations: 0
---

# Valid base-directory paths

## Outcome

`wt` only uses a worktree base directory that is an **absolute path on the
current host**. A path written for a different operating system, such as
`b:\wt` on Linux or `/home/me/wt` on Windows, is rejected before anything is
created on disk or saved to `~/.worktree.json`. The error names the problem,
where the value came from (`WT` or `~/.worktree.json`), and how to fix it.

## Incident

On a WSL2 host, the user ran `wt create fix/wsl` with `WT` unset and no
`~/.worktree.json`, so the interactive setup ran. The user thought they were on
native Windows, chose "Other", and entered `b:\wt`.

1. `prompt_for_base_dir` (`cli/src/commands/create.rs`) turned the input into a
   `PathBuf`. On a POSIX host `b:\wt` is a valid but **relative** file name
   with a colon and a backslash in it.
2. `create_dir_all` created `./b:\wt/` relative to the current directory, which
   was the root of the repository.
3. `save_config` saved `"base_dir": "b:\\wt"`.
4. `create_worktree` added a real worktree at
   `<repo>/b:\wt/rusty-biscuit/fix-wsl` on the new branch `fix/wsl`. It showed
   up as an untracked `b:\wt/` directory in the main checkout.
5. After the directory was removed, every `wt create` failed with
   `base directory 'b:\wt' does not exist`. That message doesn't say the value
   comes from `~/.worktree.json`, so the user couldn't tell what to fix.

A relative base directory is wrong even when it uses the host's own syntax
(for example `wt`, or `~/wt`, which Rust doesn't expand). Where it resolves
depends on the directory `wt` happens to run in, and it usually lands inside
the repository.

## Current behavior

| Source | Checks today | Missing |
| --- | --- | --- |
| `WT` env var (`resolve_base_dir`) | exists; is not a repo root | is absolute; source named in errors |
| `~/.worktree.json` (`resolve_base_dir`) | parses; exists; is not a repo root | is absolute; "not found" error doesn't name the file |
| Interactive prompt (`prompt_for_base_dir`) | is not a repo root (after `create_dir_all`) | is absolute, checked **before** `create_dir_all` and `save_config` |
| `save_config` | none | is absolute |

## Required behavior

### One validation rule, in the library

Add one function to `worktree::config` that every path source goes through:

```rust
pub fn validate_base_dir(raw: &str, source: BaseDirSource) -> Result<PathBuf, WorktreeError>
```

`BaseDirSource` is `Env`, `ConfigFile(PathBuf)`, or `Prompt`. The function
checks the rules below in order and returns the first one that fails:

1. **Not empty** after trimming whitespace.
2. **Not a foreign-OS path.** The path is classified by the host's path style
   (see below), and a path that is only absolute on the other OS family gets
   its own error and hint.
3. **Absolute on this host** (`Path::is_absolute`). This also catches plain
   relative paths like `wt` and `./wt`.

Checks that need the filesystem (it exists, it is not a git repository) stay
where they are and run after these.

`resolve_base_dir` calls it for both `WT` and `~/.worktree.json`.
`prompt_for_base_dir` calls it on the selected or typed value **before**
`create_dir_all`. If the typed value is rejected, the prompt shows the error
and asks again instead of exiting. `save_config` also calls it, so a library
caller can't save an invalid path.

### Recognizing a foreign-OS path

Classification is a pure function over the string and a `PathStyle`
(`Posix` | `Windows`), so tests can cover both styles on any host:

```rust
fn classify(raw: &str, style: PathStyle) -> PathKind // Absolute | Relative | ForeignAbsolute
```

| Input | On a POSIX host | On a Windows host |
| --- | --- | --- |
| `/home/me/wt` | Absolute | ForeignAbsolute (no drive or UNC prefix) |
| `b:\wt`, `B:/wt` | ForeignAbsolute (drive letter) | Absolute |
| `\\server\share\wt` | ForeignAbsolute (UNC) | Absolute |
| `b:wt` (drive-relative) | ForeignAbsolute | Relative |
| `wt`, `./wt`, `~/wt` | Relative | Relative |

The host style comes from `cfg!(windows)`. WSL2 is a POSIX host.

### Errors

Add variants to `WorktreeError` and keep `source` on all of them so the message
can name where the value came from:

- `BaseDirectoryForeignPath { value, source, hint: Option<String> }`
- `BaseDirectoryNotAbsolute { value, source }`

`BaseDirectoryNotFound` also gets a `source`. Each `wt create` render:

- names the source: "from the `WT` environment variable", or "from
  `~/.worktree.json`" with the file as an OSC 8 link, the same way the existing
  invalid-format error does,
- says how to fix it: change `WT`, edit the file, or delete the file and run
  `wt create <branch>` again to go through setup.

**WSL hint.** When the host is WSL (detected with `sniff`, which `worktree`
already depends on) and the value is a drive-letter path, the hint suggests the
mounted spelling. For example, `b:\wt` suggests `/mnt/b/wt`. Otherwise
`hint` is `None`.

### Existing invalid configuration

`wt` never rewrites `~/.worktree.json` on its own. A file that already holds an
invalid `base_dir` (like the incident's `b:\wt`) produces the error above on
the next `wt create`.

## Non-goals

- Translating paths between operating systems automatically. The WSL hint is
  only a suggestion.
- Making the base-directory check reject paths *inside* a git repository
  instead of only a repository root (see Open Questions).
- Changing where worktrees go under the base directory.

## Testing

L1 unit tests in `lib/src/config.rs`:

- `classify` against every row of the table above, for both `PathStyle`s.
  These run on every host.
- `validate_base_dir` rejects empty, relative, and foreign values from each
  source, and each error carries the right `source`.
- `resolve_base_dir` with `WT=b:\wt` (`#[cfg(unix)]`) and with
  `WT=/home/me/wt` (`#[cfg(windows)]`) returns `BaseDirectoryForeignPath`,
  using the existing `serial(wt_env)` guard.
- `resolve_base_dir` with a config file holding a relative or foreign
  `base_dir` returns the matching error with `ConfigFile(path)`. This needs the
  config location to be injectable or redirected so the test doesn't touch the
  real `~/.worktree.json`. Follow `rust-testing`'s fixture-isolation guidance.
- `save_config` refuses a relative path and writes nothing.

L1 CLI tests (`cli/`):

- With a temporary `HOME` holding a config whose `base_dir` is `b:\wt`,
  `wt create x` exits non-zero, mentions `~/.worktree.json`, and creates
  **no** directory under the current working directory. This is the incident's
  regression test.

The interactive re-prompt loop is covered by keeping the decision logic in a
testable function. No L2 terminal test is required.

## Documentation

- `README.md` "Directory Resolution": state the absolute-path rule, the
  foreign-path rejection, and that the prompt asks again on invalid input.
- Add a `docs/` page for base-directory configuration (`WT`,
  `~/.worktree.json`, setup, validation rules, and the error each rule gives).
  No such page exists yet, and the `docs/` tree has to describe this behavior.
- `worktree` skill: add the rule, plus the note that WSL2 is a POSIX host.

## Open Questions

1. **Expand `~`?** People naturally type `~/wt`, both in the prompt and in the
   JSON file. Recommendation: expand a leading `~` or `~/` to the home
   directory before validating. Otherwise reject it with a hint showing the
   expanded path.
2. **Reject base directories inside a repository?** Today only a repository
   *root* (one with `.git` directly in it) is rejected. An absolute path like
   `/home/me/coding/rusty-biscuit/wt` passes and nests worktrees inside a
   checkout. Recommendation: check it in this fix (walk up looking for `.git`)
   because it is the same kind of mistake. Or split it into its own fix.
3. **Re-run setup when the configured directory is missing?** Today a missing
   directory from `~/.worktree.json` is a hard error. Offering the
   interactive setup instead (saving only after confirmation) would have let
   the user recover without editing the file. Recommendation: no for now. The
   improved error message is enough, and re-prompting on a typo'd path could
   hide a mount that is only temporarily unavailable.
