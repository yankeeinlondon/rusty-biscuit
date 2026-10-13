# Building Worktree Test Repositories

Load before writing or changing a fixture that builds a Git repository.
The cost of a worktree test is its `git` processes, and most of them are
spent in fixture setup. A spawn costs far more than a file write, and the
gap is widest on Windows CI under contention. Spawns are the measure; a
wall-clock time on a shared host is noise.

## Helpers

| Need | Use | Not |
| ---- | --- | --- |
| Repository settings (`user.*`, `commit.gpgsign`, `gc.auto`, `remote.origin.*`, …) | `crate::test_support::configure(repo, &[(key, value)])` (lib unit tests), `crate::commands::test_support::configure` (cli unit tests), `perf_support::append_git_config(config_file, …)` (integration tests) | one `git config` per key |
| `git remote add origin <url>` in setup | the same helper with `remote.origin.url` and `remote.origin.fetch = +refs/heads/*:refs/remotes/origin/*` | a `git remote` process |
| Many commits that only need parents and dates (graph fixtures) | `BulkCommits` in `graph/tests.rs` (lib) and `git_graph/tests.rs` (cli): one streaming `git fast-import`; `commit` returns each SHA at once | one `commit-tree` per commit |
| Several ref moves | `update_refs` (`git update-ref --stdin`) in the graph tests | one `update-ref` per ref |
| Many identical repositories in one test | build one, copy it per case (`commands::list::tests::gather::Fixture::copied_from`, which skips `git init`'s `*.sample` hooks) | rebuilding per case |
| A commit's SHA right after `git commit`/`git merge` | `head(path)` in the graph tests (lib: `git_metadata::reference_target`; cli: the loose ref file) | a `rev-parse HEAD` per commit |
| A root commit, or a whole graph fixture with no checkout | `BulkCommits::commit(&[], …)` (an empty tree), then `update_refs` | `init` + `add` + `commit` |
| PR-badge listings through the real `wt` | `MixedFixture::small()` (one behind, `divergent-0..2`, one fast-forward) | `MixedFixture::new()`'s ten worktrees, which every listing measures |
| A tracking ref ahead of `main` in a graph test | `commit-tree` + `update-ref refs/remotes/origin/main` | a bare `origin`, a clone, push, and fetch |
| Fixture self-checks | one `rev-list --no-walk=unsorted --parents a b …`, one `rev-parse r1 r2 …`, one `cat-file --batch-check` | a process per commit or ref |
| Someone else's commit landing in a bare `origin` | `crate::test_support::commit_to_origin(origin, branch, file)` (lib unit tests), `remote_fixture::Fixture::commit_and_push` (cli): one `git fast-import` into the bare repository | a clone, commit, and push (about a dozen processes, half of them `receive-pack`'s helpers) |
| A ref or tag created in `origin` | `crate::test_support::fast_import(origin, "reset refs/tags/v2\nfrom refs/heads/main^0\n\n")` | a `push` |
| Any `git` process a fixture starts | `crate::git::command()` / `worktree::git::command()` | `Command::new("git")`, which on Windows pays for Git for Windows' launcher on every call |
| A `MixedFixture` for a test about the worker, flags, or PR badges | `MixedFixture::small()` (five worktrees) | `MixedFixture::new()` (ten), unless the test is about the table's size |

- The config helpers append to `<repo>/.git/config` and double-quote every
  value, because a Windows path's backslashes are escapes in a config file.
  They panic when `.git` is not a directory.
- **A linked worktree shares the main repository's config.** Configuring it
  again only rewrites the same file (`TestRepo::add_worktree` no longer
  does).
- `MixedFixture::new` builds its whole history (base, `behind-*`,
  `divergent-*`, the advanced `main`, `fast-forward-*`) with one
  `fast-import` and then `reset --hard main`. Its branches therefore have no
  reflog. `wt` reads only the tracking ref's reflog, so that is safe; a
  fixture whose test reads a branch's reflog must use real commits.
- **Copy a template only when it has no linked worktree.** A linked
  worktree's admin files hold absolute paths, so a copy would point back at
  the template. A plain repository holds none.
- `cli/tests/remove.rs` has its own helpers:
  - `configure` and `add_origin` append to `.git/config`.
  - `Fixture::commit` returns the new tip from the loose ref the commit
    wrote (`head_after_commit`), asking `git rev-parse` only when that file
    is not there.
  - **One template per test binary build.** `template()` builds `repo` (one
    commit) and a bare `origin.git` once, beside the test binary (keyed by
    its path, size, and mtime; built in a temporary directory and renamed
    into place, so racing test processes are safe). Each `Fixture` copies
    it. That replaced six processes per test (`init`, `add`, `commit`,
    `clone --bare` and its `upload-pack`, `update-ref`). Nextest runs each
    test in its own process, so a `OnceLock` alone would not share anything.
    The clone's `remote.origin.url` is cut so the template holds no absolute
    path, which keeps the rule above about linked worktrees.
  - `Fixture::push(commit, branch)` / `push_upstream(branch)` /
    `publish(bare, commit, branch)` stand in for a setup `git push`. They
    copy the loose objects the bare repository lacks, write its branch and
    (for `push`) the tracking ref, and fall back to `git push` when the
    repository has a pack. A push starts seven processes. Origin may end up
    with a superset of the pushed objects; `wt` reads only a remote's refs.
  - `Fixture::others_push(branch, commit)` is someone else's push: one
    `update-ref` in origin, where a `pusher` clone and a push cost about
    nine. A new commit on origin still goes through `pusher`.
  - `add_worktree` writes the branch ref first, so `worktree add` skips its
    `git branch` child.
- `perf_support`'s `serve_gitea_origin_one_commit_ahead` builds the served
  repository with a local `clone --bare` and writes refs as files (3
  processes, was 11).
- `perf_support::worker_processes` reads command lines and working
  directories only for `wt` processes. Reading them for every process on the
  host made each 20 ms poll of a waiting test a full enumeration (13 ms on
  an idle `build-win-native`).
- `fast-import` needs `from refs/heads/<branch>^0` to extend a branch it
  did not create in the same stream; without `^0` it refuses a branch
  starting from itself. A commit given only a `committer` gets the same
  author.
- `remote_fixture`'s isolated global config sets `maintenance.auto = false`:
  every commit and fetch otherwise starts a detached
  `git maintenance run --auto`, one more process that outlives the command.
- Keep every setting a fixture had. Settings like `gc.auto=0`,
  `core.fsmonitor=false`, and `core.commitGraph=false` stop background Git
  work that nextest reports as a leak.

## Measuring spawns

Put a `git` shim first on `PATH` that logs its arguments and runs the real
Git, then run one test with `-j 1`:

```sh
#!/bin/sh
echo "$1 $2" >> "${GIT_SHIM_LOG:-/dev/null}"
exec /opt/homebrew/bin/git "$@"
```

```sh
GIT_SHIM_LOG=/tmp/log PATH=<shim dir>:$PATH \
  cargo nextest run -p worktree -p worktree-cli -j 1 -E 'test(/<name>/)'
awk '{print $1}' /tmp/log | sort | uniq -c | sort -rn
```

A count covers both copies of a cli unit test (bin and lib targets). Product
calls mostly appear as `-C <path>` or `-c <key>`, and fixture calls as
the subcommand.

To rank a whole suite in one parallel run, log `$NEXTEST_TEST_NAME` first
(`echo "$NEXTEST_TEST_NAME $1 $2"`): nextest sets it in every test process
and Git inherits it, so `awk '{print $1}' | sort | uniq -c | sort -rn` gives
spawns per test. On `build-win-native` a spawn costs ~25–30 ms, so a test
over ~60 spawns is already near the 2.5 s target there.
