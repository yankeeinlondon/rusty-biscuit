---
status: implemented
reviewed: false
completed: false
human_review: false
implemented: true
implemented_by: claude-code/claude-opus-5-5
related:
    - 2026-09-24-ux-improvements
---

# `wt remove` on a folder other programs are using

## What happened

`wt remove feat/reusable-path` on macOS printed git's raw error and left the
repository in a state no command explained:

```text
Error: failed to execute git command: error: failed to delete
'/Volumes/coding/wt/rusty-biscuit/feat-reusable-path': Directory not empty
```

Zed was still open on that worktree. Its rust-analyzer saw files disappear,
ran a check, and recreated `target/flycheck0/` while `git worktree remove
--force` was deleting the tree. Git's recursive delete stops at the first
directory it cannot remove, and then deletes the worktree's administrative
directory anyway. The result:

- 27 of 91 top-level tracked entries were gone, the rest were still there;
- the worktree was no longer registered, so `wt` could not find it to retry;
- the branch was kept (correctly) but nothing said so.

`remove_worktree` guards against this only on Windows (`check_not_in_use`,
the rename probe), because there a held directory makes the delete fail
outright. macOS and Linux delete happily under a process's feet, so the
failure is a race with whatever that process writes next.

## Decisions

### 1. Refuse while another process works in the folder (all OSes)

Before anything is deleted, `wt remove` lists the processes whose current
directory is the worktree or below it (`sniff::os::processes_working_in`,
one `sysinfo` process-table refresh reading names, ancestry, and current
directories). If any are found it refuses with exit 4, nothing changed, and
names them:

```text
Error: Nothing was removed: the folder /…/feat-x is in use by other programs:
rust-analyzer (pids 39871, 40846), claude (pid 29970). Close them, or move
them out of it, and run wt remove again.
```

- The caller and its ancestors are never listed: in the handoff's first run
  the caller's shell is inside the worktree and is about to be moved out.
- The check runs twice: early in `run`, before the report and any prompt,
  so the user is not asked questions and then refused; and again inside
  `remove_worktree`, immediately before git, which is the check that
  matters for the race (and the only one a handoff's second run makes).
- **No flag overrides it.** Exit 4 means "the environment blocks this and
  no `--force-*` flag helps" (Windows already behaves this way), and an
  override would reintroduce exactly the race above. The user closes the
  program or moves it out.
- Only current directories are visible. A process holding an open file
  without standing in the folder is not detected on macOS or Linux (on
  Windows the existing rename probe still catches it). A process whose
  current directory the OS hides (another user's) is not detected either.
- Windows keeps the rename probe after this check; the probe is what
  catches open files. Its refusal names no process.

`WorktreeError::DirectoryInUse` becomes `{ path, processes }`; `processes`
is empty for the Windows probe.

### 2. When git fails after unregistering, finish the delete or say so

When `git worktree remove` fails, `remove_worktree` asks git whether the
worktree is still registered.

- **Still registered:** git refused before deleting (a dirty tree without
  `--force`, a locked worktree). Return git's error as before.
- **Unregistered, folder gone:** success.
- **Unregistered, folder still there:** git had already committed to
  deleting it, so finish the job: `remove_dir_all`, up to three attempts
  200 ms apart. Success continues normally (copy record, branch, remote).
  If every attempt fails, return
  `WorktreeError::FolderNotFullyRemoved { path, reason }` (exit 1):

  ```text
  Error: git unregistered worktree /…/feat-x but could not delete all of its
  folder (Directory not empty). Nothing else was removed. Delete what is
  left with: rm -rf "/…/feat-x"
  ```

  On Windows the command shown is `Remove-Item -Recurse -Force "<path>"`.
  The branch and origin are left alone, because the user has not seen the
  removal finish. The copy record is pruned by the next `wt list`, as for
  any worktree removed outside `wt`.

## Tests

- sniff L1: a child process in a subdirectory is found by name; a sibling
  directory sharing the name prefix is not inside; the caller is never
  listed.
- worktree lib L1: a process standing in the worktree makes
  `remove_worktree` return `DirectoryInUse` naming it, with files and
  registration intact (all OSes); a worktree whose admin directory git
  removed but whose folder survives is finished by the recovery; a folder
  the recovery cannot delete returns `FolderNotFullyRemoved`.
- CLI L1: `wt remove` with a process inside the worktree exits 4, names the
  process, and asks nothing.
