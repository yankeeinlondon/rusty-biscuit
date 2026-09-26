---
status: draft
created: 2026-07-29
area: worktree
packages:
  - worktree
  - worktree-cli
---

# Make `wt remove -b` Use the Default Branch as Its Merge Authority

## Summary

`wt list` and `wt remove -b` currently use different branches to decide whether
a worktree branch is safe relative to the repository:

- `wt list` compares every worktree branch with the resolved local default
  branch.
- `wt remove -b` delegates to `git branch -d <branch>`, which checks the
  branch's configured upstream or the caller's current `HEAD` when no upstream
  exists.

As a result, branch cleanup is dependent on the checkout from which the user
invokes `wt remove`. A branch that is fully contained in `main` can be preserved
as "not fully merged" merely because the user ran the command from an unrelated
branch that does not contain it.

The worktree removal itself succeeds and the branch is preserved, so this is a
safe false negative rather than data loss. It is nevertheless a correctness and
UX defect: the status command indicates that the branch has no commits absent
from the default branch, while the removal command applies an unrelated
reference and refuses cleanup.

## Reported Case

The `icon` row from `wt list` was:

```text
│  Clean   │ icon │ icon │ clean │ -2531 │
```

At the time of diagnosis:

| Fact | Observed value |
|---|---|
| Current branch | `docs/cross-platform-ci-plan` |
| Default branch | `main` |
| `icon` tip | `4550dbf4e` |
| Local `main` tip | `8fc3adc3ad18` |
| `origin/main` tip | `8fc3adc3ad18` |
| `icon` upstream | none |
| `main...icon` | 2,531 commits on `main`, 0 on `icon` |
| Is `icon` an ancestor of `main`? | yes |
| Is `icon` an ancestor of current `HEAD`? | no |

Running:

```sh
wt remove -b icon
```

removed the worktree, then preserved the branch:

```text
Warning: branch icon was preserved: error: the branch 'icon' is not fully merged
```

Invoking the same Git soft deletion from a checkout whose `HEAD` contains
`icon` would delete it. The result therefore changes with the caller's current
branch even though the target branch and default branch have not changed.

## Existing Status Semantics

The row contains three distinct facts:

1. The `Clean` badge describes uncommitted files in the linked worktree.
2. The merge column predicts whether the branch tip can merge into the local
   default-branch tip without conflicts.
3. `-2531` means the branch is zero commits ahead of, and 2,531 commits behind,
   the local default branch.

Neither `Clean` label independently promises that the branch has already been
merged. In the reported case, however, the zero-ahead commit relationship does
prove that the complete `icon` history is reachable from local `main`.

`wt list` discovers the default branch name from local repository metadata and
compares local refs. It does not fetch. That contract remains unchanged by this
fix.

## Root Cause

The library's `delete_branch` helper currently executes:

```rust
git branch -d <branch>
```

Git considers the branch fully merged when it is reachable from the branch's
configured upstream. If the branch has no upstream, Git falls back to the
current `HEAD`.

The remove command calls this helper after removing the linked worktree but
does not supply the default branch used by `wt list`. The deletion decision is
therefore made against implicit process state rather than an explicit
repository policy.

The existing `remove_with_branch_flag_deletes_branch` integration test does not
protect the intended outcome. Its stderr assertion accepts either
`Deleted branch` or `was preserved`, so a false preservation passes the test.
The unmerged-branch test covers only the ordinary case where the target has a
unique commit relative to the current checkout.

## Required Behavior

When `-b` or `--branch` is requested, `wt remove` MUST use the resolved local
default branch as the merge authority for soft deletion.

Given target branch `T` and resolved local default branch `D`:

| Relationship | Required outcome |
|---|---|
| `T` is fully reachable from `D` | Delete `T` |
| `T` has commits not reachable from `D` | Preserve `T` and warn |
| `D` cannot be resolved or inspected | Preserve `T` and report the reason |
| `T` is the default branch | Preserve `T` and report that the default branch is not deleted |
| `T` moved or disappeared during cleanup | Do not delete an unverified ref; report the resulting Git error |

The outcome MUST be independent of:

- the branch checked out in the process's current directory;
- whether the target branch has an unrelated configured upstream;
- which linked worktree was current when the command was invoked.

The command MUST continue to avoid network access. It uses the local default
branch and MUST NOT fetch, pull, or refresh remote-tracking refs.

### `D` failing to resolve is the CI default, not an edge case

The unresolvable-default row above will fire routinely, so treat it as a first-
class path rather than a defensive branch. `actions/checkout@v4` runs at default
depth, producing a single-branch shallow checkout with no `origin/HEAD`, no
`main`, and no `master` — so `default_branch()` returns `Err` there. That was
diagnosed while fixing `default_branch_detection`, which failed on macOS, Linux,
and Windows simultaneously for exactly this reason (a test asserting against the
ambient checkout rather than a repository it owned).

Two consequences:

- The warning for an unresolvable default branch must be actionable on a CI
  runner, where the cause is checkout depth rather than anything about the
  target branch.
- No test may depend on ambient default-branch resolution succeeding. A test
  that passes locally because the developer's checkout has `origin/HEAD` and
  fails on a runner because it does not is the failure mode this fix exists to
  remove, reproduced one level down.

## Proposed Implementation

Change the branch-deletion library boundary so its merge authority is explicit,
for example:

```rust
pub fn delete_branch(branch: &str, merged_into: &str) -> DeleteBranchOutcome
```

The CLI should resolve the same default branch used by list status and pass it
to the helper after the worktree has been removed successfully.

The preferred implementation keeps Git's soft-delete operation as the final
authority. Invoke `git branch -d` with command-scoped branch configuration that
treats the local default branch as the target branch's upstream for that one
subprocess. This retains Git's ref locking, checked-out-branch protections,
branch configuration cleanup, and reflog handling while preventing fallback to
the caller's `HEAD`.

The scoped configuration MUST:

- apply only to the child Git process;
- set the temporary upstream to the local repository and
  `refs/heads/<default>`;
- leave the target branch's persisted upstream configuration unchanged if the
  delete is refused;
- support all valid Git branch names, including names containing `/`, `.`, or
  `=`;
- be passed as process arguments/environment, never through a shell command.

Using a separate `merge-base --is-ancestor` check followed by
`git branch -D` is not the preferred design. It duplicates Git's soft-delete
policy and introduces a check/delete race in which the branch could move after
the ancestry check. If implementation constraints require an explicit
ancestry check, deletion MUST be conditional on the exact verified target SHA
and retain the cleanup behavior of `git branch -d`.

Default-branch resolution or branch deletion failure remains non-fatal to the
already completed worktree removal. The command should preserve the branch and
render a warning, as it does today.

## User-Facing Contract

Update CLI help and the worktree README so `-b` is described as:

> Also soft-delete the worktree branch when it is fully merged into the local
> default branch.

The warning should identify the merge authority when useful. For example:

```text
Warning: branch feature/x was preserved because it is not fully merged into main.
```

Do not recommend `git branch -D` merely because the caller's `HEAD` diverges;
that condition is no longer relevant. The force-delete hint remains appropriate
when the target actually has commits absent from the local default branch, but
the message must continue to make clear that forcing can discard the branch's
only reference to those commits.

No `wt list` calculation or cache format change is required. A small label or
documentation clarification may be made if needed, but redesigning the list
table is outside this fix.

## Test Plan

All tests in this fix are L1. They require temporary Git repositories and
subprocesses, but no real terminal, browser, device, or network resource.

### Library coverage

- A target branch fully contained in `main` is deleted while the current branch
  is divergent and does not contain the target.
- A target branch with a unique commit relative to `main` is preserved, even if
  it is merged into some other configured upstream.
- A branch with no upstream uses the explicit default branch rather than
  current `HEAD`.
- A refused deletion does not change the branch's persisted upstream
  configuration.
- Valid branch names containing `/`, `.`, and `=` are handled without shell or
  Git-config parsing errors.
- The default branch itself is never deleted through this helper.

### CLI regression coverage

Replace the permissive assertion in
`remove_with_branch_flag_deletes_branch` with a strict assertion that the
branch was deleted, then verify the ref no longer exists.

Add the reported topology:

1. Create `main`.
2. Create and advance the target worktree branch.
3. Fast-forward or merge the target into `main`.
4. Create a separate current branch from a point that does not contain the
   target.
5. Invoke `wt remove <target> -ff -b` from that separate branch.
6. Assert the worktree is removed, `Deleted branch` is reported, and the target
   ref is absent.

Retain and strengthen the unmerged case by asserting the target ref remains and
the warning names the local default branch as the failed merge authority.

Tests MUST disable repository background maintenance consistently with existing
temporary-repository fixtures. They must use ordinary cross-platform Git and
filesystem operations and avoid Unix-only shell scripts or path assumptions.

### Fixtures MUST name their default branch explicitly

`worktree/cli/tests/remove.rs:11` currently does `run_git(path, &["init"])` with
no `-b`, so every fixture in that file takes its default branch from the caller's
ambient `init.defaultBranch` — `main` on a developer machine that sets it,
`master` on a clean runner that does not.

That is load-bearing for this fix specifically. The whole premise is that
deletion is evaluated against *the resolved local default branch*, so a fixture
whose default branch name varies by host makes these tests environment-dependent
in precisely the way the fix is meant to eliminate. A green local run would prove
nothing about CI.

Every fixture touched by this work — new and existing — MUST create its default
branch explicitly (`git init -b main`, or an equivalent explicit rename), so the
branch the assertions reason about is the branch the test created. The nine other
`git init` sites under `worktree/**` already pass `-b main`; `remove.rs` and
`level2_dirty_tree.rs` are the outliers. Neither currently asserts on the branch
name, which is why the inconsistency has been inert — this fix makes it
load-bearing.

## Impact and Verification Scope

GitNexus reports LOW impact for `delete_branch`: one direct caller,
`worktree-cli`'s remove command, with no additional upstream consumers.

Expected implementation scope:

- `worktree/lib/src/worktree.rs`
- `worktree/cli/src/commands/remove.rs`
- `worktree/cli/src/args.rs`
- `worktree/cli/tests/remove.rs`
- `worktree/README.md`

No crate additions, dependency documentation changes, cache-format changes, or
downstream package areas are expected.

Final verification should use the worktree package-area recipes:

```sh
cd worktree
just build
just test
just lint
```

Do not use workspace-wide Cargo gates for this localized change.

## Non-goals

- Fetching the remote default branch before deletion.
- Deleting remote branches.
- Force-deleting a branch that has commits absent from the local default branch.
- Changing worktree dirtiness or merge-conflict prediction semantics.
- Changing the order in which worktree removal and optional branch cleanup are
  reported.
- Making branch cleanup failure undo a successfully completed worktree removal.

## Acceptance Criteria

- [ ] `wt remove <name> -b` evaluates branch deletion against the resolved local
      default branch.
- [ ] A target fully contained in the local default branch is deleted regardless
      of the caller's current branch.
- [ ] A target with commits absent from the local default branch is preserved.
- [ ] A target's configured upstream does not override the default-branch
      policy and is not mutated by a refused deletion.
- [ ] The local default branch itself cannot be deleted by this workflow.
- [ ] Failure to resolve or verify the default branch preserves the target and
      produces an actionable warning.
- [ ] No network operation is performed.
- [ ] The permissive deletion integration assertion is replaced with strict
      outcome and ref-existence assertions.
- [ ] The reported divergent-current-branch topology has regression coverage.
- [ ] CLI help and the README identify the local default branch as the soft
      deletion authority.
- [ ] The implementation compiles and the L1 tests pass on macOS, Windows, and
      Linux.
