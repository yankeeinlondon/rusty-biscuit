# `wt remove`'s confirmed lifecycle (`remove::confirmed`)

The lifecycle that replaces the journal-based one in `remove::lifecycle`
([lifecycle.md](lifecycle.md)): after "yes" (or `--force`) the checkout is
deleted. `wt remove` (`cli/src/commands/remove/mod.rs`) runs it; the old
lifecycle is still compiled but nothing in production calls it.

## Entry points

| Call | Use |
| ---- | --- |
| `confirmed::remove(&Request, &Env)` | a fresh removal after the question; `Request.facts` is the `facts::gather` result the question showed |
| `confirmed::finish_pending(base, &id, &Env)` | a pending operation a name resolved to (R19); asks nothing |
| `confirmed::save_approval(&Request, &Env)` | the shell wrapper's first run (R18): saves the approval, changes nothing else, returns the operation id for the `OperationHandoff` |
| `confirmed::finish_handoff(base, &OperationHandoff, cwd, &Env)` | the shell wrapper's second run (R18) |
| `confirmed::resolve(base, name)` | `Live`, `Pending(id)`, or `Ambiguous`, over compact **and** legacy records |

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
- **Plan with `plan_confirmed_move`, not `plan_move`.** It skips the lock
  check at planning (the unlock comes after the save) and drops Git's
  submodule refusal; the move itself (`move_with`) still refuses a lock, so a
  lock added after the question is never removed.
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

## Tests (`confirmed/tests.rs`)

- Crash with `Seams { hook, moves }` (`run_with`); `Boundary` is in
  flowchart order. `DeletionBatch` is passed but cannot crash in process.
- Inject a rename failure through `Seams.moves.rename_instead` (`EINVAL` →
  drvfs in place); a store save failure through
  `recovery::store::compact_fault::inject`.
- `test_support::ClearUsage` is a usage source with no holders.
- Compare record paths with `same_location`: Git's listing spells temp dirs
  as `/private/var/...` on macOS, the fixture as `/var/...`.
