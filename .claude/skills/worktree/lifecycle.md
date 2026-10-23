# `wt remove`'s removal lifecycle (`remove::lifecycle`)

After "yes" (or `--force`) the checkout is deleted. `wt remove`
(`cli/src/commands/remove/mod.rs`) runs this lifecycle; there is no other.
The journal, the activity observer, the checked deletion engine, the
free-space gate, and `report.json` are gone (2026-10-23).

## Entry points

| Call | Use |
| ---- | --- |
| `lifecycle::remove(&Request, &Env)` | a fresh removal after the question; `Request.facts` is the `facts::gather` result the question showed |
| `lifecycle::finish_pending(base, &id, &Env)` | a pending operation a name resolved to (R19); asks nothing |
| `lifecycle::save_approval(&Request, &Env)` | the shell wrapper's first run (R18): saves the approval, changes nothing else, returns the operation id for the `OperationHandoff` |
| `lifecycle::finish_handoff(base, &OperationHandoff, cwd, &Env)` | the shell wrapper's second run (R18) |
| `lifecycle::resolve(base, name)` | `Live`, `Pending(id)`, or `Ambiguous`, over compact **and** legacy records. An unreadable record matches no name, so a name that matches nothing while one exists is `ResolveError::Store` with that record's explanation, never a bare not-found |

`Env` injects the usage source (only to name holders in a deletion wait),
the mount table, the clock, `FsFaults`, cancellation, `Revalidate` (rechecks
a conditional branch on a later run), and `Notify` (R16 "another removal is
running", R22 waits). `Removal::exit_class()` is N5.

## How the CLI drives it

- `run`: `resolve_in` → `Pending(id)` is `finish_pending` with no report;
  `Live` → `facts::gather` → report → `policy::decide` → `remove`, or for a
  wrapper caller inside a checkout directory `save_approval` +
  `OperationHandoff` (only a `Target::Checkout` is handed off; an own link
  is deleted at once and the run prints a bare `cd:`).
- `Env` in the CLI: `SniffUsage`, `NativeMounts`, `HostClock`, `NoFaults`,
  the Ctrl-C `interrupt::Window`, `Reconfirm` (the first run's
  `reconfirm` proof) and `Stderr` (`Notify`).
- `Revalidate::still_safe(name, tip, remote_deleted)`: `remote_deleted`
  says the operation also has a remote step, so the branch's origin copy
  and its PR are no proof (the first run's `force_remote` rule).
- A remote step whose lease push fails asks the endpoint once
  (`remote::remote_branch_absent`); a branch already gone is `AlreadyAbsent`.

## Order (do not reorder)

save approval (`store.approve`) → `git worktree unlock` (only when the
question showed the lock) → place (`placement::place_with`: move, or
`mark_in_place`) → R14 `git status` (`late::unseen`) → `delete::delete_tree`
→ `git worktree remove <absent registered path>` + copy record → branch steps
→ `PendingRemoval::finish`.

- **A checkout Git cannot read is deleted in place.** When the question
  warned `GitUnreadable` or `Unchecked` (R10, R13), a move refusal about the
  `.git` link (`Unsupported`, `Uninspectable`, `Association`) binds through
  the admin record's back-reference (`bind_in_place`) instead of refusing.
- **`plan_move` skips the lock check at planning** (the unlock comes after
  the save) and never refuses submodules; the move itself (`move_with`)
  still refuses a lock, so a lock added after the question is never
  removed.
- **`git worktree remove` of the absent checkout deletes `<admin>`,
  `modules/` included** (R2's module data). Never `prune`, never `--force`.
- **Nested mounts are re-spelled.** Native tables spell real paths
  (`/private/var` on macOS); `nested_mounts` resolves the checkout and maps
  each nested mount back under the spelled root before placement and
  deletion compare paths.
- **`Operation::run(from, …)` takes the start step from inspection**, not the
  record's marker; only the unlock uses the marker (it is idempotent).

## Resume decides from the paths and Git

`resume::inspect` reads what is at the original and deletion paths (by the
recorded identity, without following links) and where `<admin>/gitdir`
points. Moved mode: checkout at the quarantine and registered there →
delete; registered still at the original (crash between rename and
association) → `rename_no_replace` back, plan again; quarantine absent →
unregister; registration gone → branch steps. A different directory at the
quarantine refuses and **keeps** the record. In place: a replaced root
**closes** the record (a later run asks afresh). A reused `<admin>` (same
name, other identity) refuses in `current_binding`.

The resolver treats a live registration at the original path as the
operation's own when that path holds the recorded checkout **or nothing**;
otherwise a crash between rename and association reads as ambiguous.

## Targets without a checkout directory

`Missing`, `Vanished`, `NotADirectory`, `Uninspectable`, and both `Link`
kinds run `without_record`: unlock if approved, delete the entry (an own
link's real checkout first, in place), `git worktree remove`, branch steps.
No compact record is saved: its `checkout_identity` would have nothing to
bind, and each step is idempotent, so an interrupted run leaves a state the
next question describes.

## Compact record and store (`record_version` 2)

- **`recovery::compact::CompactRecord`** is the approval. It has no expiry,
  and `progress` (`Approved` → `Unlocked` → `Placed` → `ContentRemoved` →
  `Unregistered`) is a hint, never proof. Branch steps carry their own
  `StepState` and may finish only at `Unregistered`. `deletion_mode: in_place`
  requires `deletion_path == original`. `compact::record_version(bytes)` is
  read first, so an unknown version is refused before any other field.
- **`recovery::legacy`** reads a v1 record and, separately, its optional
  report (≤ v6). A report never changes what migration authorizes.
  `to_compact` keeps a local step only for the checkout's own branch
  (`Conditional`, with the v1 `approved_tip`). Every other step, and every
  remote step, becomes `BranchAction::Unbound` with `StepState::Kept`: it is
  never run, and it ends in exit 1.
- **Store API (`store/compact.rs`):**
    - `approve` saves the record before any mutation. Any failure to save
      is `StoreError::NotSaved` ("nothing was deleted", exit 1).
    - `resume` migrates a v1 record by one atomic replacement of
      `record.json`, then deletes the journals and report unread.
    - `lock_waiting` backs off on a `Clock` and ends in
      `StoreError::Cancelled`.
    - `PendingRemoval::finish` deletes the whole operation directory.
    - `operations()` lists both formats.
- **Owner-only storage (`recovery/storage.rs`).** On Unix the store's
  directories and files must be the user's with no group/other bits. WSL's
  drvfs (`/mnt/<drive>`) shows every entry as `0o777` and ignores `chmod`,
  so open bits there pass (ownership is still checked), as native Windows
  relies on the inherited ACL. The mount table is read only when the bits
  are open, so other filesystems pay nothing.
- **Crash injection:** `store::compact_fault::inject` stops at a `SaveStep`
  (`Staged`, `Migrated`, `MigrationPublished`).

## Tests (`lifecycle/tests.rs`)

- Crash with `Seams { hook, moves }` (`run_with`); `Boundary` is in
  flowchart order. `DeletionBatch` is passed but cannot crash in process:
  a crash during deletion is reproduced as a crash at `Moved` plus files
  removed from the placed checkout
  (`a_crash_part_way_through_deleting_a_built_checkout_is_finished_by_the_next_run`).
- Inject a rename failure through `Seams.moves.rename_instead` (`EINVAL` →
  drvfs in place); a store save failure through
  `recovery::store::compact_fault::inject`.
- `test_support::ClearUsage` is a usage source with no holders.
- Compare record paths with `same_location`: Git's listing spells temp dirs
  as `/private/var/...` on macOS, the fixture as `/var/...`.
