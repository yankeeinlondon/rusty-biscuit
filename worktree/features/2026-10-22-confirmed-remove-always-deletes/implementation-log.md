---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-remove-large-checkout/worktree/features/2026-10-22-confirmed-remove-always-deletes/spec.md"
plan: "worktree/features/2026-10-22-confirmed-remove-always-deletes/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - worktree/lib/src/remove/clock.rs
    - worktree/lib/src/remove/mounts.rs
    - worktree/lib/src/remove/faults.rs
    - worktree/lib/src/remove/test_support.rs
    - worktree/lib/src/remove/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/worktree/testing.md
packages:
    - worktree
human_review: false
source_files_during_phase_2:
    - worktree/lib/src/remove/recovery/compact.rs
    - worktree/lib/src/remove/recovery/compact/tests.rs
    - worktree/lib/src/remove/recovery/legacy.rs
    - worktree/lib/src/remove/recovery/legacy/tests.rs
    - worktree/lib/src/remove/recovery/store/compact.rs
    - worktree/lib/src/remove/recovery/store/compact/tests.rs
    - worktree/lib/src/remove/recovery/store.rs
    - worktree/lib/src/remove/recovery/mod.rs
    - worktree/lib/src/remove/recovery/record.rs
    - worktree/lib/src/remove/lifecycle.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/worktree/lifecycle.md
message_to_agent: |-
    Phase 3 added the plain deletion engine and quarantine placement BESIDE
    the old watcher path; nothing in production calls them yet (Phase 5).
    - Engine: remove/delete/ -> delete_tree(Target{root, identity}, &Env{clock,
      faults, mounts, holders, cancelled}, &mut dyn Progress) -> Deletion::{Done,
      RetainedMounts, Permanent{failures, retained_mounts}, Cancelled{waiting_on},
      RootReplaced}. Failure displays the R29 sentence. Waits happen inside the
      engine (Progress::waiting fires only on change); Progress::batch(n) every
      1024 deletions is the hook for Boundary::DeletionBatch(n).
    - Production holders: delete::UsageHolders(&SniffUsage). Mounts:
      mounts::NativeMounts reports REAL paths (/private/var on macOS): pass a
      root resolved the same way (biscuit_file::canonicalize_simplified).
    - Placement: quarantine::placement::place(base, &plan, mounts,
      activity_observed) -> Placement::{Moved, InPlace(reason)}; also apply
      in_place_for_refusal() to plan_move's own error (LinkNotMovable has no
      plan). Nested mount -> in place before any rename.
    - plan_move still refuses Locked and Submodules (R2/R3 belong to
      Phase 4/5). R8 (.git rewrite) is already live in the old lifecycle.
    - Linux trap (os skill linux.md): cap-std dir handles are O_PATH; run
      `just cross-check worktree --os linux remove::` (from the repo root)
      after touching permission code. A -E filter with parentheses breaks
      the cross-check's remote quoting; use a substring filter.
    - Skill page for the engine: .claude/skills/worktree/deletion.md.
source_files_during_phase_3:
    - worktree/lib/src/remove/delete/mod.rs
    - worktree/lib/src/remove/delete/classify.rs
    - worktree/lib/src/remove/delete/wait.rs
    - worktree/lib/src/remove/delete/platform.rs
    - worktree/lib/src/remove/delete/tests.rs
    - worktree/lib/src/remove/mounts.rs
    - worktree/lib/src/remove/mounts/mountinfo.rs
    - worktree/lib/src/remove/mounts/linux.rs
    - worktree/lib/src/remove/mounts/macos.rs
    - worktree/lib/src/remove/mounts/windows.rs
    - worktree/lib/src/remove/quarantine/placement.rs
    - worktree/lib/src/remove/quarantine/placement/tests.rs
    - worktree/lib/src/remove/quarantine/association.rs
    - worktree/lib/src/remove/quarantine/association/tests.rs
    - worktree/lib/src/remove/quarantine/mod.rs
    - worktree/lib/src/remove/test_support.rs
    - worktree/lib/src/remove/mod.rs
docs_updated_during_phase_3:
    - worktree/docs/cli/remove.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/worktree/SKILL.md
    - .claude/skills/worktree/remove.md
    - .claude/skills/worktree/deletion.md
    - .claude/skills/os/linux.md
---

# Implementation Log for 2026-10-22-confirmed-remove-always-deletes (9 phases)

## Phase 1

### Rulings (N1–N12)

- **Adopted as proposed.** The plan is `yolo: true` and this run was
  headless, so the author did not see them. The full status is in the plan's
  "Phase 1 notes".
- **N11 checked against the code:** `wt` inherits the shell's working
  directory, and the existing check compares it with the target
  (`cli/src/commands/remove/mod.rs:356`). The Windows refusal needs no new
  Sniff API.

### Wave 1 audits

Three read-only subagents ran in parallel. Their condensed outputs are in the
plan's "Phase 1 notes". Findings that change later phases:

- **`cli/src/commands/remove/report.rs` is not `report.json`.** It renders the
  status report printed before the question, and it is kept. `report.json` is
  `recovery/store.rs` (`REPORT_FILE`, `save_report`) plus
  `outcome::OperationReport`.
- **The free-space gate is not in `storage.rs`.** It is `recovery/space.rs`,
  `store.rs::ensure_space` and its errors, and `WT_TEST_AVAILABLE_SPACE_BYTES`
  in `cli/src/env.rs`.
- **`observation/inventory.rs` (`DeletionInventory`) is used outside the
  watcher.** The usage tree index, the store, the lifecycle, and resume use
  it. Its survivors must move out before `observation/` is deleted.
- **`OwnedHandles`, the Windows Search recheck, and `SLOW_STARTUP`** exist
  only for the watcher.
- **`windows-sys` features:**
    - keep `Foundation`, `Storage_FileSystem`, and `Security` (needed by
      `CreateFileW`; `docs/dependencies.md` wrongly attributes it to the
      observer);
    - drop `System_IO`, `System_Threading`, and `System_SystemInformation`.
- **`inotify` / `fsevent-sys`:** only `worktree` uses them.
- **`.config/nextest.toml`:** the `fsevents-l1` group needs dropping or
  narrowing in Phase 7.
- **Sniff:** of R27's fields, only start time exists. Phase 4 adds:
    - a parent pid and a bounded parent chain, from the native per-pid reads;
    - the macOS app name and the Windows FileDescription;
    - Windows write access;
    - an explicit unavailable value for each.

  No new work-budget knob is needed.

### Wave 2: test seams

New modules, in the `worktree` library:

- **`lib/src/remove/clock.rs`:**
    - `Clock` (`sleep(duration, cancelled) -> Slept`);
    - `HostClock`, which checks cancellation every 50 ms;
    - `Backoff`, the N3 schedule: 50 ms doubling to a 2 s cap, with `reset`.
- **`lib/src/remove/mounts.rs`:** `Mount`, `MountInfo`, and `mounts_within`.
  It finds nested mounts by mount point, so a same-device bind mount counts,
  and returns them outermost first.
- **`lib/src/remove/faults.rs`:** `FsOp`, `FsFaults`, and `NoFaults`.
- **`lib/src/remove/test_support.rs`:** the test doubles `ManualClock`,
  `FixedMounts`, and `ScriptedFaults` (`times`, `always`, `fired`).
- **`lib/src/remove/mod.rs`:** declares the modules, and its module doc
  gained one sentence.

Two seams already existed and were not redefined:

- **`usage::UsageSource`.** The test sources `Usage` and `Sequence` already
  inject partial and failed reports.
- **The crash `Hook`/`Boundary`.**

**Departure from the plan:** no new `Boundary` variants were added. A variant
that no step constructs fails `-D warnings`, and Phase 5 rewrites
`lifecycle.rs` anyway. The plan's notes list the boundaries the rewired
lifecycle must emit.

Spike decision: no spikes. The record is in the plan.

### Tests (requirement → test)

The tests are L1, rung F1: pure in-process logic, with no crate, CLI, or
filesystem boundary crossed beyond tempdir path construction.

| Requirement | Test |
|---|---|
| N3 backoff schedule (50 ms doubling, 2 s cap) | `remove::clock::tests::backoff_doubles_from_fifty_milliseconds_to_a_two_second_cap` |
| Backoff restarts after progress | `remove::clock::tests::backoff_starts_over_after_a_reset` |
| Cancellation ends a wait (injected clock) | `remove::clock::tests::a_cancelled_backoff_reports_cancellation` |
| Host clock: cancelled before the wait | `remove::clock::tests::the_host_clock_never_waits_once_cancelled` |
| Host clock: cancelled during the wait | `remove::clock::tests::the_host_clock_notices_cancellation_while_it_sleeps` |
| Host clock: uncancelled sleep | `remove::clock::tests::the_host_clock_sleeps_the_whole_duration_when_not_cancelled` |
| Nested mounts only strictly inside the checkout, outermost first; a sibling with a longer name excluded | `remove::mounts::tests::only_mounts_strictly_inside_the_checkout_are_nested` |
| Same-device bind mount detected (spec: device IDs alone insufficient) | `remove::mounts::tests::a_same_device_bind_mount_is_nested` |
| Production fault seam injects nothing | `remove::faults::tests::no_faults_injects_nothing` |
| Temporary vs permanent injected faults, scoped by operation and path | `remove::faults::tests::scripted_faults_fire_for_their_operation_and_path_only` |

Targeted run: `cargo nextest run -p worktree -E 'test(/remove::(clock|mounts|faults)::/)'`
passed 10 of 10. The input robustness matrix does not apply: no loader
changed in this phase. It applies in Phase 2.

### Skill update

`.claude/skills/worktree/testing.md` now has a bullet listing the new seams
and their doubles.

### Gates

All gates were run in `worktree/`, after every subagent had finished:

- **`just lint`:** passed for `worktree` and `worktree-cli`.
- **`cargo clippy -p worktree --all-targets -- -D warnings`:** passed. This
  covers the new test doubles.
- **`just test`:** 1,631 passed, 15 skipped, none failed. No test failed or
  had to be rerun.

The only package that depends on `worktree` is `worktree-cli`, and
`just test` covers it.

A cross-OS check was not run. The new code is pure logic:

- path-prefix matching with `Path::starts_with`;
- `std::thread::sleep` slices;
- an error factory.

It uses no OS-specific API, so CI covers it.

## Phase 2

### Departure from the plan: v2 lives beside v1, not in `record.rs`

The plan says "new `recovery/record.rs` shape". The journal-based lifecycle
still builds and reads the v1 `RecoveryRecord` until Phase 5 rewires it, and
Phase 2's checkpoint requires the old path to keep compiling. So the compact
record is a new module, and v1 stays where it is:

- `lib/src/remove/recovery/compact.rs` holds `CompactRecord`
  (`record_version` 2).
- `recovery/record.rs` keeps v1. It is now the legacy format, read only
  through `recovery/legacy.rs`. Its module doc now states the new contract:
  its checkout intent is continued approval, but only through migration, only
  while its bindings match, and never for a remote step.
- The v2 store API is in `recovery/store/compact.rs`, as `impl RecoveryStore`
  plus `PendingRemoval`. The journal API (`begin`, `reopen`, `recover`,
  `pending`) is unchanged.
- In Phase 7, `record.rs` folds into `legacy.rs` and `store/compact.rs`
  replaces the journal half of `store.rs`.

### What was built

- **`CompactRecord`.** Its fields cover R25:
    - operation id, `requested_name`, `aliases`;
    - `original`, `deletion_path`, and `deletion_mode` (`moved` or
      `in_place`);
    - `checkout_identity` and `repository` (repository identity plus admin
      record binding);
    - `unlock_needed` (N7);
    - `branch_steps`, each with its action, approval, and state:
        - action `local {name, expected_tip}`,
          `remote {remote, destination, endpoint, expected_head}`, or
          `unbound {target}`;
        - approval `explicit` or `conditional`;
        - state `pending`, `done`, or `kept`;
    - `progress`: `approved` → `unlocked` → `placed` → `content_removed` →
      `unregistered`.

  It has no expiry. Completion is the operation directory being gone, not a
  progress value.
- **`legacy::read`.** It parses record v1 with the existing strict parser.
  The report (versions 1 to 6) is read separately and is informational only.
  A report that is absent, corrupt, of an unknown version, or for another
  operation is `LegacyReport::Ignored`/`Absent`, never an error.
- **`to_compact` mapping.**
    - Progress maps conservatively: Moved, Associated, and Deleting all become
      `placed`. BranchActions becomes `unregistered`, with every step
      re-pending; an absent branch then completes its step.
    - A local step for the checkout's own branch is rebuilt as `conditional`,
      with the v1 `approved_tip`.
    - Every remote step, and any local step for another branch, becomes
      `unbound`/`kept`.
- **The store.**
    - `approve` checks the lock and refuses while another operation on the
      same worktree is stored (either format) or while any record is
      unreadable. It builds the directory under a hidden name, writes the
      record durably (temp, fsync, rename, fsync dir), and renames the
      directory into place.
    - Any save failure is the new `StoreError::NotSaved`, whose message says
      nothing was deleted. Its exit class is `Failure` (exit 1, N5).
    - `resume` checks the binding and the lock. For a v1 record it builds the
      compact record and checks that it reads back as itself, then publishes
      it over `record.json` in one atomic replacement. Only after that are the
      journals and report deleted, unread.
    - A completed v1 record has its directory deleted and returns
      `StoreError::Completed`.
    - `lock_waiting` retries the lock with the N3 backoff on an injected
      `Clock`. A cancelled wait is the new `StoreError::Cancelled` (exit
      class `Failure`, which is how the existing `Interrupted` maps).
    - `PendingRemoval::{advance, finish_step, finish}`: `finish` renames the
      directory to a hidden name, then deletes it, so a finished operation
      leaves nothing in the store.

### Not done here (for later phases)

- **The copy record.** Removing it at completion is the lifecycle's step G
  (Phase 5); the store deletes only its own directory.
- **"An unstarted operation resumes only for the same bound checkout."** The
  store checks the repository and admin-record binding. The checkout identity
  is in the record (`checkout_identity`), but comparing it with the directory
  on disk is the lifecycle's job (Phase 5).
- **No production code calls the new API yet.** The old `pending()` sees a
  v2 directory as `UnsupportedVersion(2)`, and so refuses it. That cannot
  happen until Phase 5 starts writing v2 records.

### Tests (requirement → test)

The record and legacy tests are L1, rung F1: in-process parsing through the
public `compact::parse` and `legacy::read`. The store tests are L1, rung F2:
a real filesystem in a temp dir, real OS locks, and an injected clock and
faults.

| Requirement | Test |
|---|---|
| v2 control round trip (read, write, read) | `remove::recovery::compact::tests::the_control_record_round_trips_through_its_own_bytes_twice` |
| Field shapes of every load-bearing v2 field, plus document shapes | `remove::recovery::compact::tests::compact_record_robustness_matrix` |
| Unknown version refused before other fields (N1) | `remove::recovery::compact::tests::an_unknown_version_is_refused_before_anything_else_is_read` |
| In-place deletion path is the original; a moved one is a hidden sibling (N10) | `remove::recovery::compact::tests::an_in_place_deletion_deletes_the_original_and_nothing_else` |
| Branch steps finish only after unregistering | `remove::recovery::compact::tests::branch_steps_finish_only_after_the_registration_is_gone` |
| Unbound steps never run; at most one local and one remote step | `remove::recovery::compact::tests::an_unbound_step_is_always_kept_and_counts_toward_its_kind` |
| Binding check by identity | `remove::recovery::compact::tests::a_binding_matches_only_the_same_repository_and_worktree_record` |
| v1 + report v6 migrates; remote step left undone | `remove::recovery::legacy::tests::record_v1_with_report_v6_migrates_its_checkout_approval_and_keeps_its_remote_step_undone` |
| v1 alone migrates identically | `remove::recovery::legacy::tests::record_v1_alone_migrates_the_same_as_with_its_report` |
| Report shapes (v4, v5, v6, null id, v7, v0, text version, absent version, other operation, absent id, duplicate key, not JSON, truncated) never change authority | `remove::recovery::legacy::tests::a_report_never_changes_what_migration_authorizes` |
| A report version alone grants nothing (malformed, unknown-version, empty record) | `remove::recovery::legacy::tests::a_report_version_alone_grants_nothing_without_a_valid_record` |
| Every v1 phase maps conservatively and validates | `remove::recovery::legacy::tests::every_legacy_phase_maps_to_progress_no_further_than_it_proves` |
| A local step for another branch is not rebuilt | `remove::recovery::legacy::tests::a_local_step_for_another_branch_than_the_checkouts_is_never_rebuilt` |
| Durable owner-only save; record only; a few KB (R25) | `remove::recovery::store::compact::tests::an_approval_is_saved_alone_owner_only_and_reads_back` |
| Record-write failure (injected fail or crash) leaves no operation; "nothing was deleted" (R29) | `remove::recovery::store::compact::tests::an_approval_that_cannot_be_saved_leaves_no_operation_and_says_nothing_was_deleted` |
| Real write failure (read-only store, Unix) | `remove::recovery::store::compact::tests::an_unwritable_store_is_an_approval_not_saved` |
| A stored op of either format blocks the same worktree only | `remove::recovery::store::compact::tests::a_stored_operation_of_either_format_blocks_a_new_approval_of_the_same_worktree_only` |
| An unreadable record blocks every approval | `remove::recovery::store::compact::tests::an_unreadable_record_blocks_every_new_approval` |
| Lock and progress preconditions | `remove::recovery::store::compact::tests::approve_needs_the_worktrees_own_lock_and_an_unstarted_record` |
| Progress and steps persist, forward only | `remove::recovery::store::compact::tests::progress_and_branch_steps_persist_and_only_move_forward` |
| A completed op leaves nothing (R26) | `remove::recovery::store::compact::tests::a_finished_operation_leaves_nothing_in_the_store` |
| Concurrent waiter: acquires once released, then reads state (completion, or the exited owner's record) (R16) | `remove::recovery::store::compact::tests::a_waiter_takes_the_lock_once_released_and_then_reads_what_its_owner_left` |
| Lock wait backs off (50, 100, 200 ms) and honors cancellation | `remove::recovery::store::compact::tests::a_lock_wait_backs_off_and_ends_on_cancellation_without_the_lock` |
| Stale lock and crashed owner taken over at once | `remove::recovery::store::compact::tests::a_stale_lock_file_and_a_crashed_owners_record_are_taken_over_at_once` |
| Replaced admin record or repository refused, nothing changed | `remove::recovery::store::compact::tests::resume_refuses_a_replaced_worktree_record_or_repository_and_changes_nothing` |
| Corrupt, unknown-version, foreign, and malformed records explained (id + reason) and kept | `remove::recovery::store::compact::tests::corrupt_foreign_and_unknown_version_records_are_explained_and_kept` |
| A leftover journal or report is deleted unread | `remove::recovery::store::compact::tests::resuming_a_compact_record_deletes_leftover_journals_and_reports_without_reading_them` |
| Migration publishes v2 and drops the journals and report; a partly deleted `.git` is untouched | `remove::recovery::store::compact::tests::a_legacy_operation_migrates_to_a_compact_record_and_drops_its_journals_and_report` |
| Migration crash or failure before publishing leaves the originals, and can be re-run | `remove::recovery::store::compact::tests::a_migration_stopped_before_publishing_leaves_the_legacy_files_and_can_run_again` |
| Migration crash after publishing resumes from v2 and clears leftovers | `remove::recovery::store::compact::tests::a_migration_stopped_after_publishing_resumes_from_the_compact_record` |
| No readable journal or report is needed | `remove::recovery::store::compact::tests::a_legacy_operation_migrates_without_a_readable_journal_or_report` |
| A mismatched legacy binding is never migrated | `remove::recovery::store::compact::tests::a_legacy_operation_of_a_replaced_worktree_record_is_never_migrated` |
| A completed legacy op is deleted, not resumed | `remove::recovery::store::compact::tests::a_completed_legacy_operation_is_deleted_rather_than_resumed` |

Targeted run:
`cargo nextest run -p worktree -E 'test(/recovery::(compact|legacy|store::compact)::/)'`
passed 33 of 33.

**Input robustness.** The default outcome is that every invalid shape is
refused with `RecordInvalid` (located by field in its message) and nothing
changes on disk. The exceptions are:

- `aliases: []` and `branch_steps: []` are valid;
- the report is optional, and its invalid shapes are `Ignored`, never an
  error;
- for the report's `operation_id`, null is valid (the stop came before a
  record existed) and absent is `Ignored`.

Document shapes (duplicate key at the top level and inside a nested value,
trailing content, unknown field, not JSON, array) are tested once on the v2
loader. v1 keeps its existing matrix in `store/tests.rs`.

The CLI-spawn wiring cells (one control and one corrupt record per loader)
wait for Phase 6, because no CLI path reaches the new loader yet.

**Smell grep** over the new files:

- no `#[serde(default)]` and no `filter_map(.. as_str())`;
- `.ok()` appears once, on the optional legacy report read. It is deliberate,
  since an unreadable report grants nothing;
- `unwrap_or_default()` appears once, on a directory name, which is not a
  parse.

### Gates

- **`just lint`:** passed. `cargo clippy -p worktree --all-targets -D
  warnings` also passed, after `Stored`'s variants were boxed and one test
  type was aliased.
- **`just test`:** the first run stopped at fail-fast on
  `remove::observation::fsevents::tests::the_ceiling_spans_every_marker_round_of_a_busy_drain`.
    - It passed when rerun alone, at load average about 30 from other
      sessions on the host.
    - It is a watcher timing test that this phase does not touch, so it is
      load-sensitive. Phase 7 deletes it.
    - The second full run: 1,664 passed, 15 skipped, none failed.
- **Dependents:** only `worktree-cli` depends on `worktree`, and `just test`
  covers it.
- **Skill:** `.claude/skills/worktree/lifecycle.md` gained the section
  "Compact record and store (`record_version` 2)".
- **Cross-OS:** `just cross-check worktree --os windows` passed: 934 tests
  run, 934 passed, 1 skipped. It covers the new store's directory renames,
  file deletions, and lock waits on native Windows.

## Phase 3

### What was built

Phase 3 adds a plain deletion engine and the quarantine placement beside
the old watcher path. Nothing in production calls the engine yet; Phase 5
wires it in. One user-visible change is already live: the R8 `.git` link
rewrite (see "Quarantine placement" below).

- **Engine: `lib/src/remove/delete/`.**
    - `mod.rs` (`delete_tree`, `Target`, `Env`, `Progress`, `Deletion`,
      `Failure`);
    - `classify.rs` (N4);
    - `wait.rs` (`Holders`, `HolderReport`, `Holder`, `Wait`, `WaitCause`,
      and the `UsageHolders` adapter over `usage::UsageSource`);
    - `platform.rs`: Unix `unlinkat`/`fchmod`/`fchmodat(NOFOLLOW)` through
      cap-std handles, and Windows deletion by handle.
- **Native mount tables: `lib/src/remove/mounts.rs` plus `mounts/`.**
  Written by a subagent. `NativeMounts` reads:
    - Linux: `/proc/self/mountinfo`, with octal escapes decoded and bind
      mounts included. A malformed line fails the whole table, because
      skipping one could hide a mount.
    - macOS: `getfsstat`.
    - Windows: `FindFirstVolumeW` plus `GetVolumePathNamesForVolumeNameW`.
    - Other Unix: `Unsupported`.

  Native tables report real paths, so the caller must pass a root resolved
  the same way. The `MountInfo` doc now says so.
- **Quarantine placement: `lib/src/remove/quarantine/placement.rs`.** Written
  by a subagent. `place(base, &plan, mounts, activity_observed) ->
  Placement::{Moved, InPlace(InPlaceReason)}`:
    - a nested mount → in place, with no rename attempted;
    - `RenameFailure::Unsupported` (drvfs, R4) and `Sharing` (Windows) → in
      place, with the checkout and its registration untouched;
    - `MoveRefusal::LinkNotMovable` → in place;
    - everything else stays an error. A collision at the hidden name replaces
      nothing.

  Tests inject a rename error with `Faults::rename_instead`.
- **R8 (`association.rs`).** A relative `.git` link that breaks after the
  move (a `..` after a named component) is no longer refused:
    - `plan_move` plans `gitdir: <absolute admin dir>`, keeping the original
      line ending;
    - `move_with` replaces `<quarantine>/.git` atomically after the rename;
    - rollback restores the original bytes before moving back;
    - `verify_quarantined` accepts the rewritten form.

  This is live in the current lifecycle, because `plan_move` no longer
  refuses. `worktree/docs/cli/remove.md` was updated to match.

### Rulings and departures from the plan

- **Result enum.** The plan named `Done | RetainedMount | Waiting(blocked) |
  Permanent{path, cause}`. The engine waits internally, so the results are:
    - `Done`, `RetainedMounts(Vec<Mount>)`;
    - `Permanent { failures, retained_mounts }`, since both can happen in
      one run;
    - `Cancelled { waiting_on }`, which is the plan's "Waiting(blocked)" as
      seen by a caller whose wait was cancelled;
    - `RootReplaced`, for in-place deletion whose root is no longer the bound
      directory, a link, or a file.
- **N4 table.** It gains a fourth class, `Denied`:
    - **Unix:** the engine makes the parent directory writable through its
      handle and tries once more; a second denial is Permanent.
    - **Windows:** `ERROR_ACCESS_DENIED` is also how a running program's
      image refuses deletion. It becomes a wait only when `Holders` names a
      program at that path; otherwise it is Permanent.

  Unix `EBUSY` is Permanent ("the system is using it (it may be a mount
  point)"), so an undetected mount never waits forever. Unknown errors are
  Permanent.
- **Mount boundary.** It is the mount table, by path, OR a different
  device/volume than the root. The second check runs before each descent,
  so a tree mounted during deletion is never traversed; it is what the spec
  means by "recheck a directory boundary before descending".
- **Windows read-only.** Deletion uses `FileDispositionInfoEx` with
  `POSIX_SEMANTICS | IGNORE_READONLY_ATTRIBUTE` on a handle opened with
  `OPEN_REPARSE_POINT` and checked against the identity seen just before. It
  never clears the attribute, so another hard link's attributes never change.
  The fallback for filesystems without those flags clears read-only only when
  `nNumberOfLinks == 1`.
- **Nested mounts and placement.** Any nested mount means in-place deletion,
  decided before the move instead of after a failed rename. This is
  deterministic and testable with an injected table. The retained mount then
  stays at the original path, which is the path the user knows.
- **Refill (N2).** After three failed `remove_dir` calls on the same
  directory, with the directory relisted and emptied between them, the engine
  waits, reusing the R22 wait. Each pass after a wait gets three more tries.
- **Waits and holder queries.** `Holders::query` runs only to explain a wait:
  once when it starts, and again when the blocked set changes, never on a
  timer. `Progress::waiting` fires only when the `Wait` value changes. The
  backoff resets when the blocked set shrinks.
- **The root's `.git` is deleted last**, as the old engine did, so an
  interrupted run still leaves a checkout Git recognizes.
- `Progress::batch(n)` every 1024 deletions is where the lifecycle's
  `DeletionBatch(n)` crash boundary will hook in Phase 5.

### Left for later phases

- **Phase 5** calls, in this order:
    - `NativeMounts` (with a canonical root);
    - `placement::place` (plus `in_place_for_refusal` on `plan_move`'s own
      error);
    - `delete_tree` with `UsageHolders(&SniffUsage)`.

  It also maps `Deletion` to exit classes (N5: Permanent or RetainedMounts →
  exit 1).
- **`plan_move` still refuses** locked worktrees and submodules (R2 and R3
  are Phase 4/5). A submodule checkout's own `.git` link is relative, so
  moving a checkout that contains one keeps it resolvable only at the same
  depth; Phase 5 should decide on in-place deletion for submodule checkouts
  if that causes trouble.
- **Wording.** The `Wait` text is engine-level ("<path> is in use: cargo (pid
  4123) has a file open. Waiting until it is released (Ctrl-C to stop).").
  Phase 4 (R27 context) and Phase 6 (CLI) may enrich it.
- **The WSL side of drvfs** (a file held on the Windows side) is named by its
  path, with "the program using it could not be identified", as R4 asks. Real
  drvfs was not exercised; only an injected `EINVAL`.

### A Linux-only defect found by the cross-check, and fixed

On Linux, cap-std holds directory handles as `O_PATH` descriptors.
`fchmod` rejects them (`EBADF`), and opening an unlistable directory
succeeds; the denial comes only when it is listed. So read-only and
unlistable directories became permanent failures on Linux, while every
macOS test passed. The fix:

- `chmod_handle` falls back to `/proc/self/fd/<fd>`.
- `make_traversable` on Linux opens the child with `O_PATH | O_NOFOLLOW |
  O_DIRECTORY`, checks its device and inode against the entry it looked at,
  and changes the mode through that descriptor.
- `clear` makes a directory writable through its own handle and lists it
  once more when listing is denied.

I added this trap to `.claude/skills/worktree/deletion.md`.

### Tests (requirement → test)

The engine tests are L1, rung F2: real trees in a temp dir, with the clock,
faults, mounts, and holders injected. All are in `remove::delete::…`, apart
from the per-module ones named in full.

| Requirement | Test |
|---|---|
| A whole checkout is deleted; nothing outside it is touched | `tests::a_whole_checkout_is_deleted_with_its_root_and_nothing_outside_it` |
| Resume: a partial root is finished, and an absent root is Done | `tests::a_root_already_gone_is_done_and_a_partial_one_is_finished` |
| An in-place root replaced by another directory or a link is never deleted | `tests::a_root_that_is_another_directory_or_a_link_is_never_deleted` |
| Links are removed as entries; their targets are untouched | `tests::links_inside_the_checkout_are_removed_and_their_targets_never_touched` |
| A directory swapped for a link mid-traversal exposes nothing (Checkpoint 3) | `tests::a_directory_swapped_for_a_link_during_traversal_exposes_nothing_behind_the_link` |
| R5: a FIFO and a socket are unlinked without being opened | `tests::fifos_and_sockets_are_unlinked_without_being_opened` (Unix) |
| R13: read-only and unlistable directories; a hard link's outside data and mode are unchanged (Checkpoint 3) | `tests::read_only_directories_are_made_deletable_without_changing_a_hard_links_shared_permissions` (Unix) |
| Windows read-only files are deleted; a hard link's attribute is unchanged | `tests::read_only_files_are_deleted_without_changing_a_hard_links_shared_attributes` (Windows) |
| R29: a permanent failure deletes everything else, is never retried or waited on, and reads as one plain sentence (Checkpoint 3) | `tests::a_permanent_failure_deletes_everything_else_and_is_never_retried` |
| A denial nobody explains is permanent after one fix | `tests::a_denial_nobody_explains_is_permanent_after_one_fix` |
| Windows: a denial with a named holder (a running image) is waited on | `tests::a_windows_denial_with_a_named_holder_is_waited_on` (Windows) |
| R22: the rest is deleted first, the holder named once, and the backoff is 50, 100, 200 ms on the injected clock (Checkpoint 3) | `tests::a_held_file_is_waited_on_after_everything_else_is_deleted` |
| A wait is re-explained and re-reported only on change; the backoff resets on progress | `tests::a_wait_is_reported_again_only_when_its_blocked_entries_change` |
| Cancellation during a wait (Checkpoint 3) | `tests::cancelling_a_wait_ends_deletion_and_names_what_it_waited_on` |
| Cancellation during traversal (Checkpoint 3) | `tests::cancelling_during_traversal_stops_at_the_next_entry` |
| R24: a refill within the threshold is relisted with no wait | `tests::a_directory_that_refills_a_few_times_is_relisted_and_deleted_without_waiting` |
| R24 / N2: a persistent refill becomes a wait, and its writer is named as writing | `tests::a_directory_that_keeps_refilling_is_waited_on_and_its_writer_named` |
| R7: a nested mount is never crossed and everything else goes (Checkpoint 3) | `tests::a_nested_mount_is_never_crossed_and_everything_else_is_deleted` |
| A permanent failure and a retained mount are both reported | `tests::a_permanent_failure_and_a_retained_mount_are_both_reported` |
| The root's `.git` goes last; batch progress is reported | `tests::the_roots_git_file_goes_last_and_batches_are_reported` |
| N4 Unix table and plain causes | `classify::tests::the_unix_table_retries_only_what_the_os_asks_to_repeat`, `classify::tests::unix_causes_read_as_plain_words` |
| N4 Windows table and plain causes | `classify::tests::the_windows_table_waits_on_sharing_and_lock_violations_only`, `classify::tests::windows_causes_read_as_plain_words` |
| Errors with no OS code are classified by kind; unknown is Permanent | `classify::tests::an_error_without_an_os_code_is_classified_by_its_kind_and_unknown_kinds_are_permanent` |
| Wait wording: "has a file open" unless writing was seen; an unidentified holder is said plainly | `wait::tests::*` (3 tests) |
| Native mount tables and the mountinfo parser (escapes, optional fields, bind mounts, malformed and truncated lines, non-UTF-8 bytes) | `remove::mounts::tests::*`, `remove::mounts::mountinfo::tests::*` (13 on macOS) |
| Placement for every `RenameFailure` and `MoveRefusal`; Unsupported → in place with nothing changed; nested mount → in place with no rename; a collision and another volume stay errors; a Windows held file → in place | `remove::quarantine::placement::tests::*` |
| R8: the link is rewritten for the move, rollback restores it, and a link to another record still refuses | `remove::quarantine::association::tests::{a_link_through_the_checkout_is_rewritten_for_the_move, a_rewritten_link_is_restored_when_the_move_is_rolled_back, a_link_through_the_checkout_to_another_record_refuses, the_link_rewrite_names_the_record_absolutely}`; the old R8 refusal pin in `unsupported_shapes_refuse_before_anything_moves` was replaced, not skipped |

The tiers are all L1 (no marker prefixes). The engine fixtures are small,
and the largest test (1,024 files) runs in under 0.1 s.

**Input robustness matrix.** The only new reader is the mountinfo parser,
and the mount table is not configuration. A malformed or truncated line
fails the whole table with a located `InvalidData` error ("mountinfo line N
is malformed"), and the tests cover that. Phase 3 adds no record or
configuration loader.

### Gates

- Targeted runs: `cargo nextest run -p worktree -E 'test(/remove::delete::/)'`
  passed 24 of 24; `test(/remove::mounts::/)` 13 of 13;
  `test(/remove::quarantine::/)` 47 of 47.
- `just test` (in `worktree/`, on the final code): 1,710 passed, 15 skipped.
- `just lint`: clean. `cargo clippy --all-targets --target
  x86_64-pc-windows-gnu -- -D warnings`: clean.
- `just cross-check worktree --os windows` (whole package, before the Linux
  fix, which changes only Unix code and a branch disabled on Windows): pass,
  980 of 980, including all 4 Windows-only new tests.
- `just cross-check worktree --os linux remove::`: the first run failed with
  the 2 defects above; after the fix it passed 453 of 453.
- Dependents: only `worktree-cli`, which `just test` covers.
- Pre-existing load-sensitive test: none hit in this phase.
- **Not run:** WSL2 and real drvfs (only an injected `EINVAL`); a native
  mount (L1 uses injected tables, as the spec allows).
