---
spike: S4
date: 2026-09-24
---

# Spike S4: `git status --ignored=matching` and the Windows lock probe

## Output shape (macOS, git 2.55.0, temporary repository)

`.gitignore`: `target/`, `.env`, `*.log`, `notes.md`. State: one modified, one staged, and one untracked
file; ignored `.env`, `notes.md`, `src/deep/trace.log` (under a tracked directory), `target/debug/a/b/f`,
`logs/x.log`, and `ignoredonly/{a,b}.log`.

```text
$ git status --porcelain --ignored=matching
 M src/lib.rs
A  staged.rs
?? untracked.txt
!! .env
!! ignoredonly/a.log
!! ignoredonly/b.log
!! logs/x.log
!! notes.md
!! src/deep/trace.log
!! target/
```

- A directory matched by a **directory pattern** (`target/`) is listed once and not descended.
- Files matched by a **file pattern** are listed one by one, even when a directory holds nothing else
  (`ignoredonly/a.log`, `ignoredonly/b.log`). `--ignored=traditional` would collapse that to `ignoredonly/`.
- Nested ignored entries under tracked directories appear (`src/deep/trace.log`).
- `-z` and `-uall` change nothing here; `git -C <path>` from another directory gives the same output.

## Real worktree (this checkout)

This repository ignores `**/target/*`, not `target/`, so the list shows the **children** of `target/`:

```text
!! .claudine/tmp/
!! .opencode/.gitignore
!! .opencode/node_modules/
!! .opencode/package-lock.json
!! .opencode/package.json
!! homelab/server/frontend/dist/
!! target/.rustc_info.json
!! target/CACHEDIR.TAG
!! target/debug/
!! target/flycheck0/
!! target/nextest/
!! target/tmp/
!! visualizer/src-tauri/gen/
```

Cost (12,968 tracked files, 31,531 files under `target/`, warm cache, three runs each):

| Command | Time |
|---|---|
| `git status --porcelain` | 0.03 s |
| `git status --porcelain --ignored=matching` | 0.07 s |
| `git status --porcelain --ignored=traditional` | 0.20 s |

`matching` keeps the remove report fast; `traditional` walks into ignored directories and costs about 3x.

## Windows rename lock probe (`$BUILD_WIN`, ReFS `B:`, 3,000 files in the directory)

| Case | Result |
|---|---|
| Unlocked: rename to a sibling and back, 20 pairs | 30 ms total (about 1.5 ms per pair) |
| Held (a process launched with the directory as its working directory) | first rename fails in 2 ms, "being used by another process"; the directory is untouched and no sibling name is left |
| A file inside opened without `FILE_SHARE_DELETE` | first rename fails, "Access to the path … is denied" |

The probe is cheap, and a held directory fails on the **first** rename, so the R5 rename-back failure
needs another program to take the lock inside the 1–2 ms between the two renames. R5's retry is still
worth keeping, but it guards a narrow race.

## Spec delta (needs a ruling)

Item 3 describes the ignored entries as "top-level matches … which does not descend into ignored folders,
e.g. `.env, notes.md, target/`". That holds only for directory patterns. With this repository's own
`**/target/*`, or a `*.log` pattern spread over many directories, the `matching` list is per file or per
child and can be long. Recommendation:

- The **fingerprint** hashes the full `matching` list (exact, cheap).
- The **report** groups the list by first path component, shows each group as `name/ (N entries)` or the
  file name, and applies the same 10-item cap and bold red total as the dirty-file list.
- Do not switch to `traditional`: it is 3x slower and still lists per file where a directory also holds
  tracked files.
