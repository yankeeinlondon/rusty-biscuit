# `wt create`: Fork Origins, `.worktreeinclude`, Copy Records

Load before changing `worktree::fork_origin`, `worktree::include`,
`worktree::compare`, `worktree::copy_record`, or `create_worktree`.

## Fork-origin records (`worktree::fork_origin`)

- Stores `{ base_branch, base_sha, created_at }` per branch in
  `<repo hash>.fork-origins.json`, beside the comparison cache
  (`cache::repo_cache_file`), keyed by the main worktree's path.
- `create_worktree(branch, base, from)` records **only newly created branches**,
  as a best-effort write that never fails a create.
- `--from` must name a local branch (`refs/heads/…`); a detached HEAD requires
  `--from`.
- `ForkOriginStore::prune(exists)` drops the records `exists` rejects. The
  listing drops records of deleted branches and keeps records whose parent was
  deleted.
- Prune races with `wt create`:
  - it prunes the file **as reread at prune time**, not the copy it loaded
    before its remote wait;
  - a record created at or after the listing's ref read
    (`WorktreeList::refs_read_at`) is kept unless `rev-parse --verify` finds no
    branch.
  - Proof:
    `listing::repo_tests::a_fork_record_written_while_the_listing_waits_survives_its_prune`.
- The git graph reads `base_sha` as a lane cutoff; see [git-graph.md](git-graph.md).

## Include resolution (`worktree::include`)

- Membership comes from Git's byte-oriented
  `ls-files --others --ignored --exclude-from` intersected with
  `check-ignore --stdin -z --verbose --non-matching`.
- `IncludeRules::locate` distinguishes missing, empty, present, and
  indeterminate rules.
- `guarded_kind` rejects nested repositories, linked directories, and Windows
  reparse ancestors.
- Paths stay Git's native bytes until filesystem access.
- Git for Windows rejects verbatim exclude paths, so the resolver passes a
  simplified canonical spelling.

## Comparison (`worktree::compare`)

- Hashes every same-size included file with
  `biscuit_hash::blake3_hash_reader`; a size change avoids the read.
- A missing digest means the copy baseline is untrusted.

## Copying (`worktree::include::copy`)

- Holds source-file and destination-directory handles through copying and
  publication, so a parent link swap cannot move the copy outside the checkout.
- Clones from the open source handle where supported; byte-copies on
  unsupported clone errors.
- Publishes regular files without replacing existing destinations.
- `CopyOutcome::copied` carries `None` instead of an observation when the source
  changed mid-copy.

## Copy records (`worktree::copy_record`)

- Persists per-worktree baselines under a 16-hex-digit canonical-path key, bound
  to a random marker in Git's admin directory.
- `record_path` resolves missing destination suffixes so its key is stable
  before and after `git worktree add`.
- `load` treats corruption and identity mismatch as untrusted.

## `create_worktree` flow

1. Chooses the checkout holding the fork branch, or the main checkout when no
   checkout holds it or when reusing a branch.
2. Clears a stale destination record before adding the worktree.
3. Copies included files and writes trusted observations. Failures after the
   add are warnings in `CreateResult.include`.

## Record lifecycle elsewhere

- `fill_worktree_statuses` (`wt list`) prunes records for removed worktrees
  after a successful listing. A record goes only when its worktree is missing
  from the listing's `worktree list` **and** its admin directory no longer holds
  the record's marker — that list predates any worktree created during the
  remote wait.
- Removal deletes a record immediately after the directory is removed; cleanup
  failures are warnings.
