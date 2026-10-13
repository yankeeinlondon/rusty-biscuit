# `wt remove` lifecycle, recovery, and retry (`remove::lifecycle`)

Implementation facts for the contention-aware removal. User-facing behavior
is in `worktree/docs/cli/remove.md` ("Deleting the checkout", "What `wt
remove` refuses to move", "Recovery records and retry", and the exit-code
table); a change to an exit class or a refusal updates those sections and the
README's exit-code table together. The pieces it calls are in
[remove.md](remove.md) (store, journal, move) and
[observation.md](observation.md) (observer, inventory, deletion engine).

## Entry points

| Call | Use |
| --- | --- |
| `remove_checkout(&RemovalRequest, &Environment) -> Removal` | first attempt on a live, linked, healthy checkout |
| `resolve_target(base, name) -> Target` | `Live`, `Pending`, or `Ambiguous`, from live registrations plus pending records |
| `inspect(base, &id) -> Inspection` | what a pending operation left, with a `Resolution` |
| `resume(&ResumeRequest, &Environment) -> Removal` | retry `Quarantined`, `ContentRemoved`, `CheckoutRemoved`; anything else is `Refusal::Recovery` |
| `release_unstarted(base, &id)` | close a `NotStarted` record, or move a `MovedUnassociated` checkout back first |
| `consent_state(base, checkout, branch)` | the `(Inventory, fingerprint)` an `Approval` binds; runs `git status` |

`Environment` injects the `UsageSource`, the interrupt poll, and the observer
starter (`native_observation`, or `Observation::injected*` in tests).
`Removal::exit_class()` gives `Success`/`Failure`/`Refused`/`Blocked`
(0/1/3/4); the CLI maps that, not the outcome.

## Order (do not reorder)

`plan_move` → no pending record for this admin name → observer start →
`DeletionInventory::capture` → `register_handles` + `check_usage` →
`recheck` (tip, then `consent_state` fingerprint and dirty list) → drain +
`confirm_unchanged(.., admin_allowed = true)` → `bind_index` → `begin` →
drain + `confirm_unchanged(.., admin_allowed = false)` → `move_to_quarantine`
→ `moved_to_quarantine()` → drain and explain the move's own events →
`delete_content` → `unregister_quarantine` → original's copy record → branch
actions → `Operation::complete`.

- **The recheck runs `git status`, which may rewrite the index**, so its admin
  events are drained before `bind_index` and allowed; any admin event after
  the binding refuses (before the move) or stops (after it).
- **`confirm_unchanged` skips `Original("")`** (the tree root itself); a
  replaced root is caught by `open_root`'s identity check inside `confirm`.
- **The move's own events** are `Original("")` with `own_move` kinds and the
  original absent, `Quarantine("")`/`Tree("")` with the quarantine holding the
  checkout's identity, and `Admin("gitdir")` with
  `MovePlan::back_reference_rewritten()`. Everything else is activity.
- **A refusal after `begin` that changed nothing calls `Operation::discard`**
  (deletes the operation directory). If that fails, `inspect` resolves the
  leftover record as `NotStarted`.
- **Every write failure after the move stops with `cleanup_failed`** and
  saves what it can; the record keeps its last durable phase.

## Recovery is decided from the filesystem and Git, never the phase

`inspect` reads `Presence` at both paths (`Absent`, `Checkout` = real
directory with the recorded identity, `Other`, `Uninspectable`) and the
registration from the admin `gitdir` (`registration_state`), then:

| Original | Quarantine | Registration | Resolution |
| --- | --- | --- | --- |
| `Other` | any | any | `OriginalRecreated` |
| `Checkout` | `Absent` | at original | `NotStarted` |
| `Absent` | `Checkout` | at original | `MovedUnassociated` (crash between rename and `gitdir` rewrite) |
| `Absent` | `Checkout` | at quarantine, `.git` not a file | `PartialCheckout` |
| `Absent` | `Checkout` | at quarantine | `Quarantined` |
| `Absent` | `Absent` | at quarantine | `ContentRemoved` |
| `Absent` | `Absent` | removed | `CheckoutRemoved` |
| anything else | | | `Unresumable(reason)` |

- **An admin directory with another identity under the recorded name** means
  Git reused the name for a new worktree: `Unresumable`, and the record's own
  identity is used for `recover`'s binding check so the record stays
  readable.
- **The journal summary is `None` when the quarantine is not the moved
  checkout**: nothing in a stranger's directory is checked.

## Retry

- **Resolution dedupes only the same operation's checkout:** a live entry at
  the record's quarantine (its branch still matches the name), or at the
  original with the recorded identity (`NotStarted`). Any other double match
  is `Ambiguous`. Pending records match by `requested_name`, `aliases`, and
  the original's directory name (dasherized too). Unreadable records match
  nothing here, but `remove_checkout` refuses while any exists.
- **`resume` re-collects everything.** `Quarantined` needs an `Approval` read
  from the quarantine (`consent_state(base, quarantine, branch)`), starts the
  observer with `moved = true`, takes a fresh inventory, queries usage of the
  quarantine, rechecks, binds the index, then `RecoveryStore::reopen`, which
  keeps the old journal as `journal-<n>.jsonl` and starts a new one with the
  fresh inventory. `ContentRemoved` needs a `RecordApproval` (the
  `inspect_missing` result shown to the user) and goes through
  `remove_missing_record`. `CheckoutRemoved` runs only the branch actions in
  the request; saved approvals are never reused.
- **A completed record** is `StoreError::Completed` from `inspect`,
  `resume`, and `reopen`, and `pending()` skips it.

## How `wt remove` drives it (`cli/src/commands/remove/`)

- **Resolution:** `run` calls `resolve_target`, not `find_worktree`. Only-live
  ambiguity becomes the old `WorktreeError::AmbiguousWorktree` (exit 1); any
  ambiguity involving a pending record is `recovery::ambiguous_markup`
  (exit 3). For `Pending`, `inspect` first: `NotStarted`/`MovedUnassociated`
  → `release_unstarted`, then resolve again as live; `Quarantined`/
  `ContentRemoved` → the live entry at the record's quarantine (found by
  `canonical` path) with `Facts::retry` set; `CheckoutRemoved` → a synthetic
  `WorktreeEntry` at the original (`Facts::branch_only`); anything else →
  `recovery::inspection_markup` (exit 3). A retry never repairs (an unlinked
  quarantine refuses before `prepare`) and never hands off (a cwd inside the
  quarantine is exit 4).
- **Every existing checkout is fingerprinted** before the report
  (`Facts::reported`), because `Approval` needs one; `Approval.tip` is
  `Facts::head`. The old `files_unchanged` recheck is gone: the lifecycle's
  `recheck` does it after discovery.
- **`execute`:** a first-attempt missing directory goes to
  `execute_record_only` (no Sniff, no lifecycle). Everything else builds
  `BranchActions` (local only for `BranchStep::Delete`, remote only for
  `RemoteState::Present` with `--force-remote`) and calls `remove_checkout`,
  or `resume` with `approval` (a quarantine) or `record` (a `Missing`
  quarantine). A `RemoteState` other than `Present` is still reported by
  `delete_on_origin(.., already_deleted = true)` after a successful removal
  (exit 1 for `Unavailable`, as before).
- **Output:** `Removal::Refused` → `recovery::refusal_error` (usage refusals
  through `usage::usage_refusal`); `Finished` with `ExitClass::Success` → the
  old completion lines; anything else → `WorktreeError::RemovalFailed`
  with `recovery::stopped_markup`. Nothing reaches stdout, so no `cd:` or
  `remove-handoff:` follows a failure. The handoff's second run sets
  `Facts::reported` from the verified fresh state, then runs the same
  `execute`, so the decisive usage query is in the second run.
- **`Environment::interrupted` is `|| false`:** no Ctrl-C handler is installed
  (no signal crate is a dependency of `worktree-cli`).
- **Binary tests:** `cli/tests/remove_recovery.rs` stops a real removal with a
  `git` shim on `PATH` that writes once a `.wt-remove-*` sibling exists (Git
  runs association and verification calls right after the move). A writer at
  `git worktree remove` or `diff-index` is too late: the last original/
  quarantine check is at the start of `unregister_quarantine`, so such a write
  passes as the documented concurrent-write limit. Use
  `assert_cmd::cargo::cargo_bin("wt")`, not `env!("CARGO_BIN_EXE_wt")`:
  cross-check and CI run nextest archives, where the compile-time path does
  not exist.

## Tests (`lifecycle/tests.rs`)

- Policy cases use `Observers` (injected observer per start; the last
  `Injector` is reachable) and `Usage` (a fake `UsageSource` with a holder,
  an outcome, `exhausted`, and `fail_after`).
- **Crash injection:** `run_with(.., hook)` returns `Err(Crashed)` when the
  hook returns `true` at a `Boundary` (`Begun`, `Moved`, `ContentRemoved`,
  `Unregistered`, `BranchesDone`). `DeletionBatch(n)` cannot crash in-process;
  the real-kill test blocks there instead.
- **Real process kill:** `native_a_killed_removal_…` re-runs the test binary
  with `remove::lifecycle::tests::lifecycle_child_blocks_mid_deletion --exact
  --ignored --nocapture` (the sniff precedent for subprocess fixtures), waits
  for its ready line on stdout, and kills it.
- The uncertain-entry case drops the journal's last results line by hand; the
  window between an unlink and its result cannot be reached by a hook.
