---
description: Durable, non-obvious git mechanics for committing staged changes in this monorepo.
---
# Committing Staged Changes

This file holds only git mechanics that an agent would get wrong without being
told and that generalize beyond one batch. The commit prompt owns the workflow
and the commit-message conventions. No entry names a commit hash, a feature,
or a fix; a rule that needs an example to be understood is not a rule yet.

## Scope

- Commit only what the caller staged. Never `add`, `reset`, `restore`,
  `stash`, `checkout`, or otherwise touch the index or working tree to shape
  a commit. Extra staged paths belong to a sibling agent; leave them.
- Path-limit every commit: `git commit --only -F <msg> -- <paths>`. The `--`
  is mandatory, and `-F <msg>` must come before it, otherwise git resolves the
  message path against the worktree root and rejects a file under `/tmp`.
- `--only` commits the **working-tree** content of the named paths, not the
  staged blob. For an `AM`/`MM` path, run `git diff -- <path>` first; if the
  working tree carries edits that must not ship, copy the staged blob over
  the working tree for the commit and restore it afterwards
  (`git show :<path> > /tmp/staged; cp <path> /tmp/wt; cp /tmp/staged <path>;
  commit; cp /tmp/wt <path>`). Save the staged blob **before** the first
  `--only`, because `--only` rewrites the index entry to what it committed.
- Do not use temp-index plumbing (`update-index` → `write-tree` →
  `commit-tree`). It writes a tree containing only the captured paths, so the
  commit deletes every other file in the repository, and every later `--only`
  commit inherits the loss.
- A staged rename is two index entries. Name **both** the old and the new
  path in the pathspec, or only the addition lands. Verify with
  `git show --name-status <hash>` showing an `R` row, and `git ls-files
  <old-path>` returning nothing.
- `--pathspec-from-file` silently drops rename destinations. Use it only for
  batches with no renames. For renames use the inline `-- <old> <new> …`
  form; it copes with hundreds of pairs, so check `wc -c` of the argument
  list against `getconf ARG_MAX` rather than guessing a limit.
- For large non-rename batches, `git diff --cached --name-only <glob> >
  /tmp/paths.txt` then `--pathspec-from-file=/tmp/paths.txt`. Never pass a
  glob to `git commit` unquoted, and never loop `git commit` per path.
- Use `git log` for history; there is no `sniff git commits`.

## Inspect First

- Never pre-flight signing with `git commit --allow-empty`: it still commits
  whatever is staged. Read-only checks are `git log -5 --pretty='%G? %s'`
  or `gpg-connect-agent 'getinfo passphrase' /bye`.
- Scan full staged blobs (`git show :<path>`) for conflict markers, including
  the diff3 base marker `|||||||`. `git diff --check` misses markers outside
  the changed hunks. On a hit, refuse, leave the path staged, report
  `file:line`.
- `git grep --cached <pattern> -- <path>`: the index flag goes **before** the
  pattern, or git parses it as a revision.
- `Cargo.lock` is coupled to the manifest that declares the dependency; commit
  them together. A lockfile can shrink when a bump turns default features off,
  so audit removals as deliberate, not just additions as leaks.
- `git commit --only` and every pathspec-restricted commit is refused while
  `.git/MERGE_HEAD` exists. Check for it before dispatching; a wrapper's
  `git pull` may already have landed the staged files as a merge commit.

## Messages

- Never pass the message with `-m`. Bodies contain backticks and `$`, which
  the shell evaluates even inside double quotes. Write the full message to a
  file, subject on line 1, blank line 2, bullets after, and pass `-F`.
  Verify with `git log -1 --format=%B <hash>`; a body file with no subject
  line collapses every bullet into one giant subject.
- Give every concurrent agent its own message file name. A shared
  `/tmp/commit_msg.txt` ships the last writer's body under another commit's
  subject.
- Do not put `$$` in a file name written with a tool: the tool stores it
  literally and the shell later expands it.
- Under zsh, `$var:path` parses `:` as a modifier; use `"${rev}":path`.
  zsh does not word-split unquoted expansions, so a space-joined path list
  in one variable is one argument; pass paths as separate words or run the
  commit through `bash -c`. Avoid the variable names `status` and `path`.

## Signing

- Never disable or override signing (`commit.gpgsign`, `gpg.program`,
  `-c commit.gpgsign=false`). If signing hangs or fails, stop and report.
- `commit-tree` ignores `commit.gpgsign`; `filter-branch` strips signatures.
  Both are reasons not to use them.
- A zero exit covers the index update, not the signature. Follow every
  commit with `git verify-commit <hash>`.

## Concurrency

- `index.lock` / `refs/heads/<branch>.lock` failures are transient: wait one
  to three seconds and retry the identical command, up to five times. Never
  remove another process's lock. The one exception is your own orphaned lock
  after a fatal write error (disk full) with no git process running
  (`pgrep -f 'git (commit|status)'`).
- Never `--no-verify`, override `core.hooksPath`, `--amend`, or add fixup
  commits mid-batch. Report and let the orchestrator decide. `--amend`
  always targets HEAD, whatever pathspec you give it.
- Parallel groups need disjoint paths. Producer commits before consumer; a
  consumer whose brief names a sibling's commit polls `git log --oneline`
  for it before committing rather than assuming it landed.
- A new variant on a non-`#[non_exhaustive]` enum couples the producer with
  every matching consumer across package areas; that is one commit with no
  scope.
- Before dispatch, diff the sorted union of all group pathspecs against the
  sorted staged set; after the batch, reconcile `git status --short` against
  the original staged set. Anything left belongs to a failed or unassigned
  group. A working-tree-only ` M` that was not in the original set is the
  developer's live edit: leave it alone and mention it.
- A sub-agent brief must enumerate the hunks the message describes, and the
  sub-agent must refuse if the staged diff and the body disagree. A
  sub-agent that "completes the job" by editing beyond the staged snapshot
  commits a tree the message does not describe.
- An empty sub-agent report is not success. Verify each agent's commit by
  hash while its pathspec is still in context.

## Verification and Recovery

- Capture the hash from the `[branch abbrev] subject` banner in `git commit`
  stdout. In a batch neither `HEAD`, `git log -1`, nor `git reflog -1` is
  authoritative, because a sibling may land in between. If stdout was lost,
  `git reflog --grep '<subject-substring>' -1`.
- Verify with `git show --name-status <hash>` against the brief's path list
  and `git verify-commit <hash>`.
- The only sanctioned undo is `git update-ref HEAD <new> <old>`: a
  compare-and-swap soft reset that keeps the index and working tree, so the
  paths reappear staged for a single recommit. Capture the pre-batch SHA
  (`git rev-parse HEAD`) before dispatch so you have `<old>`. If the undo
  severs a sibling's commit that sat on top, `git cherry-pick` it back
  afterwards; it re-signs, so its hash changes.
- A blob lost from the index after a bad `--only` is recoverable with
  `git fsck --dangling` and `git cat-file -p <hash>`.

## Grouping Rules That Are Not Obvious From Paths

- A single-file-to-module split (one `D`, a new `mod.rs`, N sub-module `A`s,
  and the call-site `M`s) is one commit; every partial state fails to
  compile.
- Two docs deleted and one synthesized replacement added is not a rename;
  commit the `D`s and the `A` together so the replacement has an antecedent.
- Zero-byte placeholder files that arrive with a new docs subtree are
  intentional scaffolding; commit them with their siblings.
- A CLI whose `--help` is a hand-rolled registry needs the registry row in
  the same commit as the new subcommand. Check the registry file's diff
  against the new name before dispatching the group.
- A test that deserializes a library type ships with that type's file even
  when the conventional `lib/` versus `cli/` boundary says otherwise.
