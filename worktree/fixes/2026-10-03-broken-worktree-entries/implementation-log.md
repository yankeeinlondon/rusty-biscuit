---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-wt-skill/worktree/fixes/2026-10-03-broken-worktree-entries/spec.md"
plan: "worktree/fixes/2026-10-03-broken-worktree-entries/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/worktree/remove.md
packages:
  - worktree
---

# Implementation Log for 2026-10-03-broken-worktree-entries (6 phases)

## Phase 1

Phase 1 is rulings, a Git spike, and a baseline. No source code changed.

### Baseline (macOS host, before any change)

- `just test` (worktree/): **958 passed, 32 skipped, 0 failed** (68 s). No pre-existing failures.
- `just lint` (worktree/): clean, exit 0, no warnings.
- The staged/modified files in `git status` are from the previous fix
  (`2026-10-03-list-overlap-and-keyless-notice`) and are not part of this work.

### Spike A — Git 2.56.0, macOS, scratch repos under `/tmp/wt-spike*` (no live checkout touched)

Setup: base repo plus linked worktrees made Missing (`rm -rf <wt>`), Unlinked
(`rm <wt>/.git`, branch and detached), Locked+Missing, and one with a garbage
`.git` file.

1. **Porcelain `prunable` line.** Every Missing and Unlinked entry printed
   `prunable gitdir file points to non-existent location`, after `branch`/`detached`,
   with and without `-z`. A bare `prunable` (no reason) was **not** observed.
   The reason strings in the 2.56 binary are all non-empty: `not a valid directory`,
   `gitdir file does not exist`, `unable to read gitdir file (%s)`,
   `invalid gitdir file`, `gitdir file points to non-existent location`. The
   parser must still accept a bare marker (`Some("")`), as the spec requires.
2. **Admin layout** `<common>/worktrees/<id>/`: `commondir gitdir HEAD index logs ORIG_HEAD refs`.
   `gitdir` holds the absolute `<path>/.git` (macOS spells it `/private/tmp/...`).
   **The `index` survives deleting the checkout** for Missing and Unlinked
   entries. `HEAD` holds `ref: refs/heads/<b>` or a SHA. `git rev-parse --git-common-dir`
   from the base returned the relative `.git`, so request `--path-format=absolute`.
3. **Repair restores all postconditions but is not targeted.** Running
   `git -C <base> worktree repair <a>` printed
   `error: unable to locate repository; .git file broken: <a>/.git` and exited **1**,
   yet recreated `<a>/.git` = `gitdir: <base>/.git/worktrees/a`. The back-reference
   stayed `<a>/.git`, `rev-parse --show-toplevel` in `<a>` gave `<a>`, and the
   listing showed the same branch with no `prunable`. On a detached entry the same
   command exited 0. **It also recreated `.git` for another unlinked worktree `b`
   that was not named** (output line `repair: .git file broken: <b>`). From the main
   checkout, Git repairs every broken worktree-side link in addition to the named
   path. A second repair of a healthy entry exited 0 with no output. This is raised
   as ruling **R9** below and set for human review.
4. **Locked + Missing.** A locked entry whose directory is gone is **not** marked
   `prunable`. The listing shows `locked <reason>` only. `git worktree remove <p>`
   and `git worktree remove --force <p>` both refused with exit 128
   (`cannot remove a locked working tree ... use 'remove -f -f'`). One `--force` does
   not bypass a lock, which confirms the plan's "no second `--force`". Note for
   Phase 2/5: because such an entry is not prunable, the classifier never sees it. It
   takes the ordinary path, where status fails, so it lists as `?`, and remove gets
   the R5 error-context path.
5. **Index check via `GIT_INDEX_FILE`.** With the Missing entry's staged file:
   `GIT_INDEX_FILE=<admin>/index git diff-index --cached --name-status refs/heads/<b>`
   printed `A f`, and `--quiet` exited 1. An unchanged index gave `--quiet` exit 0.
   **Trap: an absent index file is silently read as an empty index.** `diff-index`
   exited 0 with no output, because the HEAD was an empty commit. Index presence
   must therefore be proved separately (`symlink_metadata`) before trusting the diff.
   This confirms R2's "missing index with a present admin dir counts as not
   disposable".
6. **Staged work loss is real.** `git worktree remove <missing>` with a staged file
   in the surviving index exited 0 silently and deleted the admin dir with the
   index. R1 protects against exactly this.
7. **A garbage `.git` file is not prunable.** `.git` containing `garbage` was listed
   without `prunable`, and `git status` failed with `fatal: invalid gitfile format`.
   Such entries reach the ordinary path and list as `?` (Unknown), not `✕`. In
   practice the classifier's "`.git` present but damaged" Other case arises only for
   prunable entries.
8. **Parent-repository discovery.** For an unlinked worktree nested inside the
   base (`base/inner/wt`), `git -C base/inner/wt rev-parse --show-toplevel` returned
   **the base**. Today `dirty_status` on such an entry would report the parent's
   status as the worktree's. The repair postcondition "top-level is the target" is
   load-bearing, and skipping status for prunable entries removes the
   misattribution.

### Path helpers (for R4 and Phase 4)

- `worktree::remove::handoff::canonical(path)` wraps `biscuit_file::canonicalize_simplified`
  (`dunce::canonicalize`). It resolves macOS `/private` aliases and Windows short
  names (via `GetFinalPathNameByHandle`) and strips the `\\?\` verbatim prefix. It
  returns the input unchanged when the path does not exist, so it never canonicalizes
  a missing path into something else.
- `handoff::same_path(a, b)` is private (`canonical(a) == canonical(b)`).
  `handoff::is_within(path, dir)` is public. Phase 4 should make `same_path`
  `pub(crate)` (or move it beside `canonical`) and not write a second comparator.
- `copy_record::canonical_worktree_path` (private) canonicalizes the longest
  existing ancestor and re-appends missing components. It is the model to use if a
  comparison must include a missing tail.
- `lib/src/util.rs` holds only `dasherize`, with no path helpers.
- Git runners in `lib/src/git.rs`: `git_command_in(dir, args)` (what `dirty_status`
  uses), `git_from(base, dir, args)`, `git_from_raw`, `git_from_bytes`, and
  `git_from_bytes_allow_no_match`.
- Name resolution: `worktree::worktree::resolve_worktree(entries, name)` is a pure
  function over parsed entries. R6 calls it directly for each candidate (branch,
  then basename) and accepts a candidate only if it returns `Ok(e)` with `e.path`
  equal to this entry's path.

### Inventory of `DirtyStatus` and `WorktreeEntry` uses

`DirtyStatus` matches and defaults:
- `lib/src/worktree.rs:36` enum; `:49` `WorktreeStatus.dirty`; `:299` `gather_dirtiness`
  (spawns `dirty_status` per entry, `:303`); `:388` `gather_local`; `:443` `commit`;
  `:488-491` `refresh_dirty_status` (used after `--ff`, called from
  `cli/src/commands/list.rs:597`); `:505-535` `dirty_status` (returns `Clean` at
  `:514` on failure — the defect).
- `cli/src/commands/list_table.rs:483-485` legend; `:658` row builder; `:776-780`
  `dirty_dot` (exhaustive match, so it breaks at compile time when `Unknown` is added).
- Tests: `lib/src/listing.rs:1211-1224`; `cli/src/commands/list/tests/pipeline.rs:25,318,646-647`;
  `cli/tests/list_table.rs:21,64,90-97`.
- `cli/src/commands/git_graph*` and `lib/src/fast_forward.rs` do not read
  dirtiness. There is no `unwrap_or(DirtyStatus::Clean)` anywhere.
  `remove/inventory.rs` `dirty` is a separate `Vec<DirtyEntry>`, not `DirtyStatus`.

`WorktreeEntry` struct literals (they break when `prunable` is added):
- `lib/src/worktree.rs:145` and `:170` (the parser's two flushes); `:1593` (test closure).
- `lib/src/listing.rs:505` (test helper `entry`).
- `cli/tests/list_table.rs:66` (test helper `status`).

Other porcelain parsers that ignore `prunable`:
- `lib/src/fast_forward.rs:121` `holder_of` parses porcelain itself and must learn
  `prunable` in Phase 3 (or switch to `parse_worktree_list`).

### Rulings (R1–R9) — plan defaults adopted under `yolo: true`

- **R1** — adopted the spec's recommendation. Inspect the surviving admin index;
  ordinary discard consent is required when it differs from the recorded HEAD;
  failed inspection refuses. Spike item 6 shows the loss is real. The spec stays
  `draft-spec` until the author confirms.
- **R2** — adopted, with one addition from spike item 5: check `<admin>/index` with
  `symlink_metadata` before running `GIT_INDEX_FILE=… git diff-index --cached --quiet <HEAD>`,
  because Git reads an absent index as empty. Exit 0 means disposable, exit 1 means
  staged paths (list them with `--name-status -z`), and any other exit refuses.
  Absent admin dir plus absent checkout is a bare record and disposable.
- **R3** — adopted as written.
- **R4** — adopted. Enumerate `<git-common-dir>/worktrees/*/gitdir`, with the common
  dir from `git -C <base> rev-parse --path-format=absolute --git-common-dir` (spike
  item 2). Compare the back-reference to `<recorded-path>/.git` with
  `handoff::canonical`. Exactly one match is required.
- **R5** — adopted as written. Spike items 4 and 7 identify the cases that reach it
  (locked+missing, garbage `.git`).
- **R6** — adopted. Call `resolve_worktree` per candidate; no copy of its logic.
- **R7** — adopted (no perf work).
- **R8** — adopted (injected inspector seam for Windows reparse points).
- **R9 (new, needs author input before Phase 4)** — `git worktree repair <path>`
  run from the base also repairs every other unlinked worktree's `.git`
  (spike item 3). The spec calls the repair "targeted". Proposed default:
  accept Git's behavior, keep verifying only the target, and say in the refusal or
  report text and docs that repair "may have changed Git metadata for this or other
  worktrees". The options are recorded in the spec's `human_review_items`. Phases 2
  and 3 do not depend on R9.

### Checkpoint 1

The spike notes, rulings R1–R9, and the baseline are recorded above. No source,
docs, or skill-reference files changed except the worktree skill's `remove.md`
Git-facts note: a new section, "Git facts about broken worktree records",
recording spike items 1, 3–8. These facts describe Git, so the note is true
before any code lands.

Gates: `just test` and `just lint` were run as the baseline above. No code
changed afterward, so no rerun was needed. Environment exercised: macOS only. No
cross-OS run, because Phase 1 has no code.
