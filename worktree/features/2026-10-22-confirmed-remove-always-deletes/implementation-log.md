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
    - sniff
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
source_files_during_phase_4:
    - sniff/lib/Cargo.toml
    - sniff/lib/src/filesystem/query/ancestry.rs
    - sniff/lib/src/filesystem/query/backend.rs
    - sniff/lib/src/filesystem/query/linux.rs
    - sniff/lib/src/filesystem/query/linux_tests.rs
    - sniff/lib/src/filesystem/query/macos.rs
    - sniff/lib/src/filesystem/query/macos_tests.rs
    - sniff/lib/src/filesystem/query/mod.rs
    - sniff/lib/src/filesystem/query/process.rs
    - sniff/lib/src/filesystem/query/report.rs
    - sniff/lib/src/filesystem/query/tests.rs
    - sniff/lib/src/filesystem/query/win32.rs
    - sniff/lib/src/filesystem/query/win32_tests.rs
    - sniff/lib/src/performance/counters.rs
    - sniff/cli/tests/l1/filesystem_query.rs
    - Cargo.lock
    - worktree/lib/Cargo.toml
    - worktree/lib/src/git_metadata.rs
    - worktree/lib/src/remove/facts.rs
    - worktree/lib/src/remove/facts/tests.rs
    - worktree/lib/src/remove/submodules.rs
    - worktree/lib/src/remove/submodules/tests.rs
    - worktree/lib/src/remove/mod.rs
    - worktree/lib/src/remove/usage.rs
    - worktree/lib/src/remove/usage/tests.rs
    - worktree/lib/src/remove/lifecycle/tests.rs
    - worktree/lib/src/remove/mounts.rs
    - worktree/lib/src/remove/mounts/macos.rs
    - worktree/lib/src/remove/mounts/mountinfo.rs
    - worktree/lib/src/remove/mounts/windows.rs
    - worktree/lib/src/remove/delete/mod.rs
    - worktree/lib/src/remove/delete/tests.rs
    - worktree/lib/src/remove/quarantine/association.rs
    - worktree/lib/src/remove/quarantine/placement/tests.rs
docs_updated_during_phase_4:
    - sniff/docs/topics/filesystem-query.md
    - sniff/docs/dependencies.md
    - docs/dependencies.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/sniff/SKILL.md
    - .claude/skills/worktree/SKILL.md
    - .claude/skills/worktree/deletion.md
    - .claude/skills/worktree/warnings.md
source_files_during_phase_5:
    - worktree/lib/src/remove/confirmed.rs
    - worktree/lib/src/remove/confirmed/branches.rs
    - worktree/lib/src/remove/confirmed/late.rs
    - worktree/lib/src/remove/confirmed/resolve.rs
    - worktree/lib/src/remove/confirmed/resume.rs
    - worktree/lib/src/remove/confirmed/tests.rs
    - worktree/lib/src/remove/mod.rs
    - worktree/lib/src/remove/handoff.rs
    - worktree/lib/src/remove/quarantine/association.rs
    - worktree/lib/src/remove/recovery/store.rs
    - worktree/lib/src/remove/recovery/store/compact.rs
    - worktree/lib/src/remove/delete/mod.rs
    - worktree/lib/src/remove/test_support.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/worktree/SKILL.md
    - .claude/skills/worktree/confirmed.md
    - .claude/skills/os/windows.md
message_to_agent: |-
    Phase 5 built the new lifecycle BESIDE the old one, as
    remove::confirmed (lib/src/remove/confirmed.rs + confirmed/). The CLI
    still calls remove::lifecycle; nothing in production calls confirmed.
    - Phase 6 wiring: confirmed::resolve(base, name) -> Live | Pending(id) |
      Ambiguous (reads compact AND legacy records). Live: facts::gather,
      ask, then confirmed::remove(&Request{base, entry, requested_name,
      facts, branches: &[ApprovedBranch{action, approval}]}, &Env).
      Pending: confirmed::finish_pending(base, &id, &Env), no question.
      Handoff (R18): first run confirmed::save_approval(&Request,&Env) ->
      OperationId, write handoff::OperationHandoff (format 5, 10 min TTL);
      second run handoff::consume_operation + confirmed::finish_handoff.
    - Env needs: usage (SniffUsage), mounts (NativeMounts), clock
      (HostClock), faults (NoFaults), cancelled (Ctrl-C), revalidate (impl
      confirmed::Revalidate with the CLI's reconfirm_branch), notify (impl
      confirmed::Notify: removal_running for R16, waiting(&Wait) for R22).
    - Render Finished: checkout (CheckoutResult; Display is the one plain
      sentence for R7/R29/stop), entries (Option, "unknown" when None),
      late_changes (R14 line), original_kept, branches (BranchOutcome),
      warnings. Removal::exit_class() is N5 (Blocked=4 only for R15).
    - Missing/Vanished/file/link targets save no record (logged ruling).
    - plan_confirmed_move lifts Locked/Submodules; plan_move is unchanged
      for the old lifecycle. Phase 7 deletes remove::lifecycle and may
      rename confirmed -> lifecycle. Skill: .claude/skills/worktree/confirmed.md.
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

## Phase 4

### What was built

Phase 4 gathers everything the confirmation question will show. Nothing in
production calls it yet; Phase 5 (lifecycle) and Phase 6 (CLI) wire it in.
Three parallel subagents did Wave 7. The orchestrator wrote the shared
submodule interface first, so the classifier and the inspector were built
against the same types, and then did the process-context enrichment and
the `gix-index` fix after merging.

- **Sniff process context (R27), in `sniff/lib/src/filesystem/query/`.**
  `ProcessRecord` gains:
    - `parent_pid`;
    - `app_name` (macOS: `CFBundleName` of the enclosing `.app`, then
      `CFBundleDisplayName`, then the bundle name; Windows: the
      `FileDescription`; Linux: always `None`);
    - `ancestors: Vec<Ancestor>`, nearest first, bounded by
      `MAX_ANCESTORS = 8`, built from per-PID reads with a start-time
      recheck, so a reused PID ends the chain;
    - `ancestry_end: AncestryEnd`, the explicit "unavailable" value.

  It also adds the `filesystem.query.parent_lookups` counter and the
  `LimitationKind::ContextUnavailable` limitation, and raises
  `SCHEMA_VERSION` to 3. The new fields are `#[serde(default)]` because
  `wt`'s legacy recovery records persist schema-1 reports. The chain
  builder is `ancestry.rs`. `plist` is a new macOS-only dependency; it was
  already in `Cargo.lock`.
- **Submodule inspector (R2), `worktree/lib/src/remove/submodules.rs`.**
    - It lists gitlinks in-process from `<admin>/index` (so a broken checkout
      `.git` file hides nothing), then recurses into each submodule's index.
    - Each initialized submodule costs two Git processes:
      `status --porcelain=v1 -z --untracked-files=normal
      --ignore-submodules=dirty` and N6's `rev-list --count HEAD --branches
      --not --remotes`. Remote presence is read in-process.
    - Any failure is `Finding::Unchecked`. A `.git` pointing outside
      `<admin>/modules` and outside the submodule's own checkout is
      `GitDirOwner::External`.
- **State classifier, `worktree/lib/src/remove/facts.rs`.**
  `gather(&Sources) -> Facts` returns:
    - `Target`: `Checkout`, `Link`, `Missing`, `Vanished`, `NotADirectory`,
      or `Uninspectable`;
    - ordered `Warning`s (lock first);
    - `Note`s (external submodule Git data, which needs no `--force`);
    - `Blocker::ShellInside` (Windows R15);
    - the questioned `Inventory`;
    - the one usage report.

  `needs_force()` is true when there is any warning (N8). It makes one
  usage query and one `git status`. `Mount` gains `fs_type` and
  `super_options`, plus `Mount::is_drvfs` and `mounts::containing`. The
  drvfs rule comes from a real WSL2 `/mnt/c` mountinfo line: fstype `9p`
  with `aname=drvfs`.
- **Process-context rendering (orchestrator).** `usage::Holder` gains
  `start_time`, `app_name`, and `ancestors`, and `HolderEvidence` gains a
  per-handle `writing`. The subagent's per-process `ProcessUse.writing` is
  removed: it marked every open file of a process as written when one
  was.
    - `facts::describe_process` renders, for example, `cargo (pid 4123,
      started 14:02, run from WezTerm) is writing target/out`.
    - The app name leads when it differs from the process name. The start
      time is local; it is `HH:MM` today and gets a date otherwise.
    - "Run from" is the nearest ancestor on `KNOWN_ORIGINS` (terminals,
      editors, agents), else the nearest ancestor with an app name, else
      nothing.

  `worktree/lib` gains a direct `chrono` dependency (already in the
  lockfile).
- **A defect fixed outside the plan: a `gix-index` panic.** gix-index 0.52
  panics ("attempt to subtract with overflow") on an index file shorter
  than its checksum. The new submodule inspector reads `<admin>/index`
  before the question, so a truncated index would have crashed `wt
  remove`. `git_metadata::staged_entries` now hands any file under 32
  bytes to Git, which reports it as corrupt.

### Rulings and departures

- **macOS parent reads.** The Phase 1 audit named `pbi_ppid` from
  `PROC_PIDTBSDINFO`. That call works only for the same user, so the chain
  would stop at a root-owned `login` before reaching the terminal. Sniff
  uses `proc_bsdshortinfo` plus `sysctl kern.proc.pid`, which need no
  same-user permission.
- **Windows write access** stays `None`: the file-side holder query
  returns PIDs only, with no granted-access value. So on Windows an open
  handle always reads "has … open", which is what the spec asks for when
  writing is unknown.
- **The Windows blocker** replaces every warning: when it is set, nothing
  more is gathered.
- **A locked worktree whose directory is gone** is not marked `prunable` by
  Git, so `availability::classify` calls it healthy. `gather` checks the
  path itself and treats it as missing. Phase 5's record removal must do
  the same.
- **A link not leading to this worktree's own checkout** is not inspected:
  no usage query, inventory, or mount check. Only the link and its record
  are involved.
- **The R14 sentence** ("files created or changed before deletion finishes
  are included") is `facts::LATE_CHANGES_NOTE`, for the CLI to show under
  the warnings. It is not part of any one warning.
- **A failed `git status`** usually adds a second warning, because the
  submodule inspector reads the same index and reports "could not be
  checked". Phase 6 may fold the two together.

### Left for later phases

- **Phase 5:**
    - `plan_move` still refuses `Locked` and `Submodules`, and that must
      be lifted. Unlocking goes through `git worktree unlock` after the
      record is saved (N7).
    - A submodule checkout's own `.git` link is relative, so it may need
      in-place deletion.
    - It calls `facts::gather` with `repair::Git`, `SniffUsage`,
      `NativeMounts`, and `Host::detect()`.
- **Phase 6:** the caller's `shell` name (N11: the parent process name) and
  `shell_wrapper` come from the CLI. Escaping happens there, over the whole
  rendered warning text.
- **Sniff:** `sniff filesystem query --plain` does not print the new
  fields; JSON does.

### Tests (requirement → test)

All L1. The classifier tests are rung F2: real repositories in temp dirs,
with usage, mounts, caller, and host injected. Tests in
`worktree::remove::facts::tests` unless named otherwise.

| Requirement | Test |
|---|---|
| A clean checkout has no warnings and needs no `--force`, after exactly one usage query | `a_clean_checkout_has_no_warnings` |
| R3 / N7: the lock and its reason (none, empty, or gone directory) come first; the mtime is never shown | `lock` tests (3), including locked with the directory gone |
| R1 / R27: each activity kind is worded as observed | `each_process_is_named_with_what_it_is_doing` |
| R27: name, start time (today or another day), and "run from" (known program, else nearest app, else nothing) | `a_process_is_named_with_its_start_and_the_known_program_it_was_run_from`, `the_origin_is_a_known_program_else_the_nearest_app_else_nothing` |
| R27: the app name leads; only an observed write reads as writing, per handle | `an_app_name_leads_and_only_an_observed_write_reads_as_writing` |
| R27: Sniff's context reaches the rendered warning through the real usage policy | `sniffs_process_context_reaches_the_warning` |
| N8: a failed or partial usage query is a warning; R17: a checkout that vanished is already removed | usage failed / partial / vanished tests (3) |
| Uncommitted files are named up to a bound; R13: a failed `git status` gives its reason and no count | `uncommitted_files_are_named_up_to_a_bound`, corrupt-index test |
| R10: an unverifiable repair and a prunable entry whose `.git` exists warn; a missing `.git` is still repaired | 3 tests |
| R11: a file at the path, and an uninspectable path (Unix) | 2 tests |
| R12: staged changes unknown, and staged changes listed | 2 tests |
| R9: a link to its own checkout, an unrelated directory, another worktree, or the main checkout (only the first counts as own) | 4 tests |
| R15: a Unix warning, a Windows blocker, and no warning with the wrapper | 3 tests (cfg-gated) |
| R6 synced folder; R4 drvfs; R7 nested and same-device bind mounts; an unreadable mount table | 4 tests |
| R2: the submodule-report mapping | mapping test over a hand-built `SubmoduleReport` |
| R2 (spec L1): a submodule with an uncommitted file and an unpushed commit warns about both; a clean one beside it adds nothing | `a_submodule_with_work_only_there_warns_and_a_clean_one_does_not`, `a_worktree_whose_submodules_are_clean_has_no_warnings` |
| Checkpoint 4: a combined fixture renders every item, lock first | the combined test |
| R2 inspector: clean/owned, at risk, no remote, uninitialized, nested, external, failed, unlisted, status parsing | `remove::submodules::tests::*` (9) |
| drvfs from the real WSL2 line; `fs_type` and options kept; `containing` and `is_drvfs` | `remove::mounts::…` (3 new) |
| The `gix-index` panic: a truncated index (0, 4, or 31 bytes) goes to Git and fails with Git's error | `git_metadata::tests::a_truncated_index_is_left_to_git_rather_than_panicking`. Confirmed to fail with the guard removed. |
| Sniff: the chain builder (bound, reused PID, cycle, vanished, unreadable, budget), `/proc` stat parent PID, plist and version-info reads, serde of absent, null, and present, and live parent and ancestor checks per OS | `sniff` `ancestry::tests` (11), `linux::stat_tests` (3), `tests.rs` (6), `macos_tests` (7), `win32_tests` (5), `linux_tests::live` |

**Input robustness.** The only new reader of a persisted format is Sniff's
report deserializer, for the new fields. Its serde test covers absent,
null, and present, plus a missing `Ancestor` name and an unknown
`ancestry_end`, both rejected. The new fields are `#[serde(default)]` on
purpose: schema-1 reports saved in legacy recovery records must still load.
The mountinfo parser is not configuration.

### Gates

- `just lint` (`worktree/`): clean. `just lint` (`sniff/`): clean.
- `just test` (`worktree/`): 1,759 passed, 15 skipped. The one slow test
  is the pre-existing `recovery::journal::…64_mib_ceiling…`, which Phase 7
  deletes.
- `just test` (`sniff/`): 3,522 passed, 33 skipped.
- `just check-tier-coverage worktree`: 0 stranded.
- `just cross-check worktree --os windows`: 1,027 of 1,027 passed.
- `just cross-check worktree --os linux remove::`: 501 of 501 passed.
- The Sniff subagent also ran `filesystem::query` on `build-linux` (168 of
  168) and `build-win-native` (128 of 128, then `win32_tests` 42 of 42).
- Dependents: only `worktree` reads `PathUsageReport`, `ProcessRecord`, or
  Sniff's counters (grep across the repo). `sniff`'s lint compiled
  `sniff-cli` and `darkmatter`, and the new fields are additive.
- **Not run:** WSL2 (the Linux code path was covered on `build-linux`).
  `cargo clippy -p sniff --all-targets -- -D warnings` outside `just lint`
  reports lints that were already there, outside `filesystem/query`
  (`space.rs:378` and others).

## Phase 5

### Departure from the plan: the new lifecycle is a new module, beside the old

The plan says "rewrite `lifecycle.rs`". The CLI still calls the journal
lifecycle (`remove::lifecycle`) and its ~140 CLI tests pin that behavior;
rewiring the CLI is Phase 6. Rewriting `lifecycle.rs` in place would have
broken `just test` for the area, or pulled Phase 6 into this phase. So,
as in Phases 2–4, the new lifecycle is built beside the old one:

- `lib/src/remove/confirmed.rs` plus `confirmed/{branches,late,resolve,resume,tests}.rs`.
- Nothing in production calls it yet. Phase 6 switches the CLI to
  `confirmed::{resolve, remove, finish_pending, finish_handoff}`; Phase 7
  deletes `remove::lifecycle` (with its `Refusal` variants, `observation`,
  the journal, and the free-space gate). Phase 7 may rename `confirmed` to
  `lifecycle` once the old module is gone.
- The old lifecycle is unchanged in behavior. `plan_move` still refuses
  locks and submodules for it.

### What was built

- **`confirmed::remove(&Request, &Env) -> Removal`.** `Request` carries the
  `Facts` the question showed and the approved branch steps
  (`ApprovedBranch { action, approval }`). It runs, recording each step in
  the compact record:
    1. `plan_confirmed_move` (new in `quarantine/association.rs`): like
       `plan_move`, but a lock does not refuse at planning (the move itself
       still checks it, after the unlock) and submodules never refuse. A
       `LinkNotMovable` refusal, or a nested mount, means in place from the
       start.
    2. the lock (`take_lock`): if another removal holds it, `Notify::removal_running`
       and `lock_waiting` (R16); Ctrl-C ends the wait.
    3. `store.approve` of the compact record. A save failure is
       `Refusal::NotSaved` (exit 1, "nothing was deleted"). `Pending(id)`
       (an earlier run of this worktree exited) resumes that operation
       instead of saving a second one.
    4. `git worktree unlock` when the question showed a lock (N7).
    5. `place_with`: moved, or `PendingRemoval::mark_in_place` (new) for
       drvfs, a Windows sharing violation, a nested mount, or a link that
       cannot be kept.
    6. R14: `late::unseen` runs `git status` once and lists dirty paths the
       question did not show (`Finished::late_changes`).
    7. `delete::delete_tree` with `UsageHolders`, re-spelled nested mounts,
       and `Notify::waiting` for R22/R24 waits.
    8. `git worktree remove <absent registered path>` (deletes `<admin>`,
       its `modules/` too), then the copy record.
    9. branch steps (`confirmed::branches`), then `PendingRemoval::finish`.
- **`confirmed::resume`**: `finish_pending` and `finish_handoff`. The start
  step is read from the paths and `<admin>/gitdir`, not the marker (table in
  `.claude/skills/worktree/confirmed.md`). A crash between the rename and the
  association is moved back and placed again. A replaced quarantine refuses
  and keeps the record; a replaced in-place root closes it.
- **`confirmed::resolve`**: the name resolver over compact and legacy
  records (the old one reads v1 only). A live registration at the original
  path is the operation's own when that path holds the recorded checkout or
  nothing.
- **`confirmed::branches`**: local steps are bound to their tip; an absent
  branch is done; a conditional step is rechecked through `Revalidate` on
  any run but the approving one; a remote step's endpoint is rechecked
  against `push_endpoints` on a later run, then pushed under its lease. A
  kept branch never asks about the checkout; it makes the run exit 1. A
  legacy unbound step is `NotCarriedOver` (exit 1).
- **R18 handoff**: `confirmed::save_approval` is the first run (saves the
  approval and returns its id, nothing else); `finish_handoff` is the
  second. In `handoff.rs`, `OperationHandoff` (format 5) carries the
  operation id, the repository, and the target; `OPERATION_HANDOFF_TTL` is 10
  minutes; `consume_operation` is single-use; `verify_operation` checks only
  that the caller left the target and the repository is the same.
- **`ExitClass` (N5)** in `confirmed`: 0 success; 1 for a retained mount, a
  permanent failure, a kept branch, an unsaved approval, Ctrl-C, a stopped
  step; 3 for a replaced target, a main checkout, an unusable record, a
  refused move; 4 only for the Windows R15 blocker.
- **Engine:** `delete::Progress::finished(deleted)` is called once at the
  end, so the final line can say "N entries" (`Finished::entries`).

### Rulings and departures

- **Targets with no checkout directory save no record.** `Missing`,
  `Vanished`, `NotADirectory`, `Uninspectable`, and both kinds of `Link` run
  `without_record`: unlock if approved, delete the entry (for an own link,
  the real checkout in place first), `git worktree remove`, branch steps. The
  compact record's `checkout_identity` has nothing to bind for them, and each
  step is idempotent, so a crash leaves a state that the next question
  describes (it asks again rather than silently resuming). This departs from
  "save before the first unlock" for those targets only; a full checkout
  always saves first.
- **The checkout's identity is bound when the approval is saved**, after the
  question, by `plan_confirmed_move`. `Facts` does not carry an identity.
  The plan's association checks still require the directory's `.git` to lead
  to this worktree's own record, so a replacement can only be another
  checkout of the same record.
- **A refused move after the unlock** (for example a collision at the hidden
  name) closes the record and returns `Refusal::Move`; the approved unlock
  stays done. Retrying with the same operation id would collide again.
- **A replaced root found by the engine during a fresh run** closes the
  record in both modes (exit 3). On resume, a replaced quarantine keeps the
  record, so each later run explains the path again.
- **Remote "already absent"** is not detected: the lease fails and the step
  is kept with Git's reason (exit 1). Detecting it needs a live `ls-remote`.
- **The R14 status runs after placement**, on the deletion path (the
  quarantine, which Git reads after the move), only in the run that showed
  the question.

### Left for later phases

- **Phase 6** wires the CLI: resolve with `confirmed::resolve`; gather
  `Facts`; ask; call `confirmed::remove` (or `finish_pending` for a pending
  name, `finish_handoff` for `--handoff`); implement `Revalidate` with the
  existing `reconfirm_branch`; implement `Notify` on stderr; render
  `Finished` (`late_changes`, `entries`, `original_kept`, branch results,
  the `CheckoutResult` sentence). For R18 the first run calls
  `confirmed::save_approval` (saves the compact record, changes nothing
  else, returns the operation id) and writes an `OperationHandoff`; the
  second run calls `confirmed::finish_handoff`.
- **Phase 7** deletes `remove::lifecycle` and the old `plan_move` callers.

### Tests (requirement → test)

All L1, rung F2: real repositories in temp dirs, with the usage source,
mount table, clock, and faults injected. Tests are in
`worktree::remove::confirmed::tests` unless named otherwise.

| Requirement | Test |
|---|---|
| One run deletes a dirty, untracked, ignored checkout, unregisters it, deletes the branch, leaves no record/journal/report/hidden directory; entries counted | `a_confirmed_checkout_is_deleted_in_one_run_and_leaves_nothing` |
| R3 / N7: the shown lock is unlocked through Git and the worktree deleted | `a_locked_worktree_is_unlocked_and_deleted` |
| R3: a lock added after the question is never removed (exit 3, nothing changed) | `a_lock_added_after_the_question_is_never_removed` |
| R2: a submodule worktree is deleted with `<admin>/modules/<name>`; the source repository untouched | `a_worktree_with_a_submodule_is_deleted_with_its_module_data` |
| R14: a file created after the question is deleted and named | `a_file_created_after_the_question_is_deleted_and_named` |
| R29: an unsaved approval deletes and unlocks nothing; "nothing was deleted"; exit 1 | `an_approval_that_cannot_be_saved_changes_nothing` |
| R29: a permanent failure deletes the rest, no wait, one plain sentence, exit 1; a later run finishes silently | `a_permanent_failure_deletes_the_rest_and_a_later_run_finishes` |
| R22: a held file is waited on after the rest, the wait shown once, finished in the same run | `a_held_file_is_waited_on_and_the_removal_finishes_in_the_same_run` |
| R7: a nested mount (injected, bind-mount style) is kept with registration, record, and branch; exit 1 with an unmount sentence; a later run finishes | `a_nested_mount_is_kept_until_unmounted_and_then_finished` |
| R4: no exclusive rename → in place under the same record (Unix) | `a_filesystem_without_exclusive_rename_deletes_in_place` |
| R15 Windows blocker → exit 4 | `the_shell_inside_blocker_refuses_with_exit_4` |
| Main checkout never removed | `the_main_checkout_is_never_removed` |
| Checkpoint 5: crash at Begun, Unlocked, Moved, ContentRemoved, Unregistered, BranchAction(0), BranchesDone, each finished without a question | `a_crash_at_every_step_is_finished_by_the_next_run` |
| Crash between rename and reassociation is finished | `a_crash_between_the_rename_and_the_registration_is_finished` |
| A recreated original is kept and reported; the moved checkout deleted | `a_recreated_original_is_kept_and_the_moved_checkout_deleted` |
| A replaced quarantine is never deleted; record kept; path named | `a_replaced_quarantine_is_never_deleted` |
| A replaced in-place root is kept; the operation closed (Unix) | `a_replaced_in_place_root_is_kept_and_the_operation_closed` |
| A reused worktree record is never resumed into | `a_replaced_worktree_record_is_never_resumed` |
| A moved branch is kept after the checkout is deleted; exit 1 | `a_branch_that_moved_is_kept_after_the_checkout_is_deleted` |
| An absent branch completes its step | `an_absent_branch_completes_its_step` |
| A conditional step is rechecked by a later run (kept / deleted) | `a_conditional_branch_is_rechecked_by_a_later_run` |
| R16: the waiter is told, waits, then continues what the owner left | `a_second_run_waits_for_the_running_removal_and_continues_it` |
| R16: the lock wait honors Ctrl-C | `the_wait_for_a_running_removal_honors_cancellation` |
| A fresh run continues an exited run's operation | `a_fresh_run_continues_the_removal_an_earlier_run_left` |
| A name matching a pending and another live worktree is ambiguous; alias resolves | `a_name_matching_a_pending_removal_and_another_worktree_is_ambiguous` |
| R18: the second run finishes once the caller left, with no content comparison | `the_handoff_second_run_finishes_once_the_caller_left` |
| R12 / R3: a missing locked worktree is unlocked and its record removed | `a_missing_locked_worktree_has_its_record_removed` |
| R11: a file at the path is deleted with the record | `a_file_at_the_path_is_deleted_with_the_record` |
| R9: an own link deletes both; a foreign link only the link and record | `a_link_to_the_own_checkout_deletes_both`, `a_link_to_another_directory_deletes_only_the_link_and_the_record` |
| R19: legacy v1 + report v6 finished without a question; own branch deleted; unbound remote step left undone, exit 1 | `a_legacy_operation_is_finished_and_its_unbound_remote_step_left_undone` |
| R18 handoff format: round trip twice, single use, 10-minute expiry, old format / bad id / unknown field refused, caller and repository checks | `remove::handoff::tests::{an_operation_handoff_round_trips_then_replay_is_missing, an_operation_handoff_expires_after_ten_minutes, only_the_operation_format_is_an_operation_handoff, the_second_run_checks_the_caller_and_the_repository_only}` |

Targeted run: `cargo nextest run -p worktree -E 'test(/remove::confirmed::|remove::handoff::/)'`
passed 44 of 44 (the handoff test goes through `save_approval`).

**Input robustness.** The one new loader is `consume_operation`
(`OperationHandoff`, JSON through serde with `deny_unknown_fields`). Its
default outcome is `HandoffError::Missing` with the file deleted. Covered
cells: the control (round trip), another `format_version`, the old
`HandoffRecord` format, a malformed operation id, an unknown field, and
corrupt bytes via the existing loader test. No field is `Option`, and none
uses `#[serde(default)]`, `.ok()`, or `unwrap_or_default()`.

### A Windows-only defect found by the cross-check, and fixed

The first `just cross-check worktree --os windows` failed 2 of 1,060: both
R9 link tests. On Windows, `symlink_metadata` reports a junction as a
symlink, not a directory, so `remove_link` called `remove_file`, which
fails with `ERROR_ACCESS_DENIED`. It now checks `FILE_ATTRIBUTE_DIRECTORY`
and uses `remove_dir`, which removes the link and never its target. The
fact is added to `.claude/skills/os/windows.md` ("Removing a link without
touching its target"). The fix took one attempt; `spec.md`'s `os_failing`
is not set.

### Gates

- `just lint` (`worktree/`): clean, on the final code.
  `cargo clippy -p worktree --all-targets --target x86_64-pc-windows-gnu -- -D warnings`:
  clean.
- `just test` (`worktree/`): 1,794 passed, 15 skipped, none failed. The
  only change after this run is inside `#[cfg(windows)]`.
- `just check-tier-coverage worktree`: 0 stranded. Every new test is L1
  (no tier marker), in the library's unit test target.
- `just cross-check worktree --os windows`: the first run, on the whole
  package, gave 1,058 of 1,060 passed (the defect above). After the fix,
  `just cross-check worktree --os windows remove::confirmed::` passed 29 of
  29. The two Unix-only tests (drvfs `EINVAL`, the replaced in-place root)
  are compiled out on Windows.
- Dependents: only `worktree-cli` depends on `worktree`, and `just test`
  runs it. `sniff` was not changed in this phase.
- Pre-existing or load-sensitive failures: none hit.
- **Not run:** Linux and WSL2 cross-checks. The new code adds no
  Linux-specific branch; Phase 8 covers WSL2 drvfs.
