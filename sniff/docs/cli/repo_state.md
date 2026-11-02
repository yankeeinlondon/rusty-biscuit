---
blast_radius:
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/working_state.rs
  - sniff/lib/src/filesystem/git/working_state.rs
---

# The `sniff repo state` Subcommand

Prints an identity for the working tree's **content**: the `HEAD` commit plus a hash of every changed path. Two observations with the same id have the same commit and the same uncommitted content, whatever happened in between; any edit, new untracked file, or deletion changes the id, and undoing it restores the old id.

Use it to tell whether something you computed from the tree (a test run, a build, a report) still describes the tree as it is now: record the id with the result, and compare later.

```bash
sniff repo state
```

```text
1dbbae87177f1815
head: 3c18e031851ec31925eb9befe213d703c51ffd0b
changed: 2
  src/main.rs  file:bb13a608d48f790d
  old.rs  deleted
```

The id is alone on the first line, so `sniff repo state | head -1` is the id.

## What counts

- **Changed paths** are what Git reports against `HEAD`: staged, unstaged, and untracked files. Each one's current bytes are hashed (xxHash, through `biscuit-hash`); a deleted path counts as deleted, a symbolic link by its target, and a directory such as a submodule checkout by its presence only.
- **Ignored files never count.** Build output under `target/` can change freely without changing the id.
- **Timestamps never count.** Saving a file with the same bytes keeps the id.
- **The id is the same on every OS** for the same commit and bytes: paths enter the hash in Git's `/`-separated spelling. Line-ending conversion is not undone, so a checkout with CRLF files and one with LF files differ.

## Arguments and Flags

| Argument | Description |
|----------|-------------|
| `-b/--base <DIR>` | Analyze a specific directory instead of the current one |
| `--json` | Emit the object below |

## Errors

The command fails, rather than printing an id, outside a Git repository, in a bare repository, and while the index has merge conflicts (the conflicted paths are named).

## JSON Output (`--json`)

```bash
sniff --json repo state
```

```json
{
  "id": "1dbbae87177f1815",
  "head": "3c18e031851ec31925eb9befe213d703c51ffd0b",
  "changed": [
    { "path": "src/main.rs", "content": "file:bb13a608d48f790d" },
    { "path": "old.rs", "content": "absent" }
  ]
}
```

`head` is `null` on a branch with no commits yet. `content` is the token that entered the id: `file:<hash>`, `symlink:<hash>`, `directory`, or `absent`.

## Cost

One working-tree status walk plus one read of each changed path, counted as `git.status_walks` and `git.working_state.paths_hashed` under `--perf`. No history, diff, or network work is done.
