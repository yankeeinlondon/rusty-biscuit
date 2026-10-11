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
| Many identical repositories in one test | build one, copy it per case (`list::tests::gather::Fixture::copied_from`) | rebuilding per case |

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
