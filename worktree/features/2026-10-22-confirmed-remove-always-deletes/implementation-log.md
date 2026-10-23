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
    - worktree-cli
human_review: true
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
source_files_during_phase_6:
    - worktree/cli/src/args.rs
    - worktree/cli/src/main.rs
    - worktree/cli/src/env.rs
    - worktree/cli/src/commands/list_table.rs
    - worktree/cli/src/commands/remove/mod.rs
    - worktree/cli/src/commands/remove/policy.rs
    - worktree/cli/src/commands/remove/report.rs
    - worktree/cli/src/commands/remove/outcome.rs
    - worktree/cli/src/commands/remove/interrupt.rs
    - worktree/cli/src/commands/remove/recovery.rs
    - worktree/cli/src/commands/remove/usage.rs
    - worktree/cli/tests/remove.rs
    - worktree/cli/tests/remove_recovery.rs
    - worktree/cli/tests/wrapper_protocol.rs
    - worktree/cli/tests/level2_remove.rs
    - worktree/cli/tests/level2_powershell_remove.rs
    - worktree/cli/tests/powershell_wrapper_exec.rs
    - worktree/cli/Cargo.toml
    - worktree/cli/tests/remove_recovery_report.rs
    - worktree/cli/tests/remove_usage_refusal.rs
    - worktree/cli/tests/snapshots/remove_recovery_report__*.snap
    - worktree/cli/tests/snapshots/remove_usage_refusal__*.snap
    - worktree/lib/src/remove/confirmed.rs
    - worktree/lib/src/remove/confirmed/branches.rs
    - worktree/lib/src/remove/confirmed/resume.rs
    - worktree/lib/src/remove/confirmed/tests.rs
    - worktree/lib/src/remove/facts.rs
    - worktree/lib/src/remove/facts/tests.rs
    - worktree/lib/src/remove/quarantine/rename.rs
    - worktree/lib/src/remove/quarantine/rename/tests.rs
    - worktree/lib/src/remove/remote.rs
docs_updated_during_phase_6:
    - worktree/README.md
    - worktree/docs/cli/remove.md
    - worktree/docs/cli/list.md
    - docs/dependencies.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/worktree/confirmed.md
    - .claude/skills/worktree/warnings.md
    - .claude/skills/worktree/cli-contracts.md
    - .claude/skills/worktree/remove.md
    - .claude/skills/worktree/testing.md
    - .claude/skills/os/macos.md
message_to_agent: |-
    Phase 8 (verification) changed two behaviors Phase 9's docs must state:
    - An unreadable pending record (corrupt, unknown version, another
      operation's) is now explained: `wt remove <name>` that matches
      nothing while such a record exists exits 1 with "Nothing was removed.
      The record of an earlier removal could not be read: removal record
      <id> cannot be used: <reason>." (lifecycle/resolve.rs).
    - On WSL drvfs (/mnt/<drive>) the removal store skips the owner-only
      mode-bit check (bits are always 0o777 there and chmod is ignored);
      ownership is still checked. Before this, every --force removal on
      /mnt/c failed with "other users can access it" (recovery/storage.rs).
      Verified on build-win: warns, deletes, exit 0, nothing left.
    - Open decision (human_review_items): on a ~500k-entry checkout Sniff's
      2 s usage budget is spent walking the tree, so the question always
      shows "Some programs could not be checked". Do not describe the
      process warning as reliable on very large checkouts until that is
      decided; check the log's Phase 8 "Large-checkout acceptance" section.
    - worktree/docs/cli/remove.md below "Deleting the checkout" is still the
      old watcher/journal text (Phase 9 owns it). Code names: the plan's
      `confirmed` is `remove::lifecycle` since Phase 7.
source_files_during_phase_7:
    - worktree/lib/Cargo.toml
    - Cargo.lock
    - .config/nextest.toml
    - worktree/lib/src/test_support.rs
    - worktree/lib/src/remove/mod.rs
    - worktree/lib/src/remove/lifecycle.rs
    - worktree/lib/src/remove/lifecycle/branches.rs
    - worktree/lib/src/remove/lifecycle/late.rs
    - worktree/lib/src/remove/lifecycle/resolve.rs
    - worktree/lib/src/remove/lifecycle/resume.rs
    - worktree/lib/src/remove/lifecycle/tests.rs
    - worktree/lib/src/remove/confirmed.rs
    - worktree/lib/src/remove/confirmed/*.rs
    - worktree/lib/src/remove/lifecycle/tests/*.rs
    - worktree/lib/src/remove/observation/**
    - worktree/lib/src/remove/outcome.rs
    - worktree/lib/src/remove/recovery/journal.rs
    - worktree/lib/src/remove/recovery/space.rs
    - worktree/lib/src/remove/recovery/mod.rs
    - worktree/lib/src/remove/recovery/record.rs
    - worktree/lib/src/remove/recovery/storage.rs
    - worktree/lib/src/remove/recovery/store.rs
    - worktree/lib/src/remove/recovery/store/tests.rs
    - worktree/lib/src/remove/recovery/store/compact.rs
    - worktree/lib/src/remove/recovery/store/compact/tests.rs
    - worktree/lib/src/remove/quarantine/mod.rs
    - worktree/lib/src/remove/quarantine/association.rs
    - worktree/lib/src/remove/quarantine/association/tests.rs
    - worktree/lib/src/remove/quarantine/placement.rs
    - worktree/lib/src/remove/quarantine/placement/tests.rs
    - worktree/lib/src/remove/quarantine/unregister.rs
    - worktree/lib/src/remove/quarantine/unregister/tests.rs
    - worktree/lib/src/remove/usage.rs
    - worktree/lib/src/remove/usage/tests.rs
    - worktree/lib/src/remove/facts.rs
    - worktree/lib/src/remove/facts/tests.rs
    - worktree/lib/src/remove/delete/wait.rs
    - worktree/lib/src/remove/handoff.rs
    - worktree/lib/src/remove/included.rs
    - worktree/lib/src/remove/inventory.rs
    - worktree/lib/src/remove/missing.rs
    - worktree/lib/src/remove/test_support.rs
    - worktree/cli/src/commands/remove/mod.rs
    - worktree/cli/src/commands/remove/outcome.rs
    - worktree/cli/src/commands/remove/interrupt.rs
docs_updated_during_phase_7:
    - docs/dependencies.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
    - .claude/skills/worktree/SKILL.md
    - .claude/skills/worktree/lifecycle.md
    - .claude/skills/worktree/confirmed.md
    - .claude/skills/worktree/observation.md
    - .claude/skills/worktree/remove.md
    - .claude/skills/worktree/deletion.md
    - .claude/skills/os/linux.md
    - .claude/skills/os/macos.md
    - .claude/skills/os/windows.md
source_files_during_phase_8:
    - worktree/lib/src/remove/lifecycle/resolve.rs
    - worktree/lib/src/remove/lifecycle/tests.rs
    - worktree/lib/src/remove/recovery/storage.rs
    - worktree/cli/tests/remove_recovery.rs
docs_updated_during_phase_8: []
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
    - .claude/skills/worktree/lifecycle.md
    - .claude/skills/worktree/warnings.md
    - .claude/skills/os/wsl.md
    - .claude/skills/os/SKILL.md
human_review_items:
    - |-
      **Should `wt remove` give its "which programs are using this worktree" check more time on very large checkouts?**

      Why now: the next phase writes the user documentation for the removal question. What it says about the program warning depends on this choice.

      What happened: before it asks "Delete it?", `wt` asks the system which programs are using files in the worktree. That check has a fixed 2-second allowance and has to look at every file in the worktree first. On a test worktree with 500,000 files (a large Rust or Node build folder is in this range) just looking at the files took about 3.5 seconds. The allowance ran out before any program was checked, so the question always says "Some programs could not be checked". A script that runs `wt remove` without a terminal is then refused unless it adds `--force`. Deleting still works.

      - **A. Leave it as it is.** Pros: the question appears after at most about 2 seconds, and nothing changes. Cons: on big worktrees the program check never runs, and scripts always need `--force` there.
      - **B. Give the check a longer allowance, for example 10 seconds.** Pros: a one-line change, and the program check works on big worktrees. Cons: on very big worktrees the question takes a few seconds longer to appear (about 3.5 seconds at 500,000 files).
      - **C. Change Sniff (the library that does the check) so that listing programs does not depend on first looking at every file**, for example by giving the two steps separate time allowances or by matching open files by folder. Pros: fast and complete. Cons: a larger change to a shared library, with its own tests on all four operating systems.

      Recommendation: **B now, C later if it is ever needed.** It removes the misleading warning for a small, predictable wait, and it keeps the rule of asking the system only once.
    - |-
      **Please confirm a security trade-off made for WSL drives (`/mnt/c` and similar).**

      Why now: it changes what `wt` trusts, and the next phase documents it.

      What happened: `wt` keeps a small recovery file for an unfinished removal and refuses to use it if other users could change it, judged by Unix permission bits. On Windows drives opened from WSL those bits are always "everyone can do everything" and cannot be changed, so every removal there failed with "other users can access it". The spec says such worktrees must be removable. I changed the check so that on these drives only the owner is checked, not the bits. Native Windows already works this way (it relies on the folder's Windows permissions).

      - **A. Keep the change** (owner is checked, bits are ignored on WSL Windows drives only). Pros: the feature works on WSL Windows drives and matches native Windows. Cons: on that one kind of drive, protection relies on Windows permissions, which `wt` cannot see from WSL.
      - **B. Refuse removals on WSL Windows drives with a clear message** to run `wt` from Windows instead. Pros: no reliance on permissions `wt` cannot see. Cons: goes against the spec's decision that these worktrees are supported.

      Recommendation: **A.** It is the behavior the spec asked for, it matches native Windows, and the recovery file only ever lets `wt` finish deleting the same worktree it already recorded.
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

## Phase 6

### What was built

- **`wt remove` runs the confirmed lifecycle.** `cli/src/commands/remove/mod.rs`
  is rewritten:
    - `confirmed::resolve_in` resolves the name. `Pending(id)` is finished
      with `confirmed::finish_pending`, with no report and no question (R19).
    - `Live` runs `facts::gather` (one usage query, one `git status`), then
      the branch facts (tier, PR, remote preflight as before), the report,
      `policy::decide`, and `confirmed::remove`.
    - For a wrapper caller standing in a checkout directory,
      `confirmed::save_approval` plus `handoff::write_operation`
      (`OperationHandoff`, 10-minute TTL). `run_handoff` is
      `consume_operation` plus `confirmed::finish_handoff`.
    - `Env`: `SniffUsage`, `NativeMounts`, `HostClock`, `NoFaults`, the
      Ctrl-C window, `Reconfirm` (the first run's `reconfirm` proof, as
      `Revalidate`), `Stderr` (`Notify`: R16 "another removal is running",
      R22/R24 waits).
- **`policy.rs`**: `Flags { force, force_branch, force_remote }`. One
  checkout question for every interactive run without `--force`; a
  non-interactive run with any warning and no `--force` is
  `Decision::NeedsForce` (exit 3); a clean one proceeds. Branch questions are
  unchanged; `--force` never implies `--force-branch`/`--force-remote`.
  The matrix test is rebuilt: tier × warned × flags × mode (448 cases).
- **`report.rs`**: the report/question. A heading (`Remove worktree X
  (path)?` when asking, `Removing worktree X at path` otherwise), each
  warning as a `⚠ ` `UnorderedList` item, notes, the late-change sentence
  when there are warnings or a question, then the branch section as before.
  `Processes` and `NestedMounts` are rendered from their parts so no
  process name or mount source can start a line; everything else goes
  through `visible` (control characters escaped, Prose markup escaped).
- **`outcome.rs`** (new; replaces `recovery.rs`): the final line
  `Removed X (N entries)[ and deleted branch B][ (n commits lost)][; also
  deleted N files changed after you answered: …].`, "unknown number of
  entries" when not counted, the already-gone record line, kept-branch
  sentences (exit 1), the plain `CheckoutResult` sentence for a retained
  mount, a permanent failure, a cancel or a stop (exit 1), refusals mapped
  by `ExitClass` (`error_for`), and the ambiguity list. Git's multi-line
  errors are folded to one line (`one_line`).
- **Args (R28)**: `--force-worktree` is gone (clap error, exit 2); `--force`
  is added; help text and examples updated. Completions are clap's dynamic
  ones, so bash/zsh/fish/PowerShell follow the flag set with no other
  change.
- **Deleted**: `cli/src/commands/remove/{recovery,usage}.rs`,
  `cli/tests/{remove_recovery_report,remove_usage_refusal}.rs` and their 19
  snapshots, `env::TEST_AVAILABLE_SPACE_VAR` (the CLI no longer reads free
  space), and `list_table::unavailable_reason` (used only by the old
  refusal).

### Rulings and departures

- **Only a real miss of the usage query warns.** Phase 4 made any
  incomplete coverage a `UsageIncomplete` warning (N8, "failed or
  partial"). On macOS every query reports FSEvents subscriptions and
  polling watchers as unsupported and open handles as partly checked
  (other users' processes), so every removal warned and every
  non-interactive removal refused. That contradicts the spec's "a clean
  checkout with no warnings still removes without a question in a
  non-interactive run". `facts::missed` now counts discovery unavailable or
  unsupported, an exhausted budget, and a mechanism `Failed` or
  `NotAttempted`; `Unsupported` and `Partial` mechanisms are the host's
  ordinary limits. Test: `remove::facts::tests::only_a_missed_usage_mechanism_warns`.
- **Included-file consent is kept as a warning.** The old flow asked
  consent for `.worktreeinclude` copies that were new, changed, or could not
  be compared; Phase 4's facts dropped it, so a non-interactive run would
  have deleted an edited `.env` copy with no `--force`. `gather` now runs
  `classify_included`: `Warning::IncludedFiles`, `Warning::IncludedUnchecked`
  (unreadable rules, formerly a failure before removal), and
  `Note::IncludedCheck` for the assessment's messages.
- **The report is printed on every run** (heading plus branch section), not
  only when a question is asked: the branch tiers are not about the
  checkout, and a script's log keeps them. A clean non-interactive run
  prints the report and the final line, and no question.
- **The caller's own shell is not listed as a program using the worktree.**
  The first run now queries usage (the old first run did not), so a wrapper
  caller's shell, standing in the worktree, was listed and a clean
  non-interactive wrapper run refused. `Caller::parent` (Unix:
  `std::os::unix::process::parent_id`; Windows: one `sysinfo` lookup) drops
  that PID when all it does is stand in a directory of the checkout: a
  working directory, or on Windows an open handle on a directory, which is
  how Sniff sees a shell's working directory there.
- **A worktree reached through a link to its own checkout is not handed
  off.** It has no compact record to hand off (Phase 5 ruling), so a wrapper
  caller standing in it gets the deletion in the first run and a bare `cd:`
  line after success.
- **`Revalidate::still_safe` takes `remote_deleted`.** A resumed or
  handed-off conditional branch is rechecked with the first run's rule:
  when the operation also deletes the branch on origin, that copy and its PR
  are no proof.
- **Exit codes (N5)**: an unknown, malformed, or expired handoff token is
  exit 3 (was 4); the second run's "shell still inside" is exit 3; 4 is
  only the Windows shell-inside blocker.
- **`sysinfo` is now a Windows-only normal dependency of `worktree-cli`**
  (it was a dev-dependency, already in the lockfile): std has no parent PID
  on Windows, and Sniff's ancestry reader is crate-private. The same lookup
  names the shell in the Windows R15 blocker (`Caller::shell`, N11).
  `docs/dependencies.md` updated.

### Product defects the test rewrite found, and fixed

- **R10/R13: `--force` refused a checkout Git cannot read** (an unverified
  repair, a garbage `.git` file): `plan_confirmed_move` refused the `.git`
  link. When the question warned `GitUnreadable` or `Unchecked`, a move
  refusal about the link (`Unsupported`, `Uninspectable`, `Association`) now
  binds the checkout through its admin record's back-reference and deletes
  in place (`confirmed::unreadable_by_git`). Tests:
  `a_repair_that_is_not_verified_is_a_warning_and_force_deletes_it`,
  `a_status_failure_is_a_warning_with_gits_reason_and_force_deletes_it`.
- **A remote branch already deleted by someone else failed its step**
  (lease push error, exit 1, advice that would fail). After a failed lease
  push the step asks the endpoint once (`remote::remote_branch_absent`); an
  absent branch is `AlreadyAbsent` and its stale tracking ref is dropped.
  Test: `an_origin_branch_deleted_between_the_runs_does_not_stop_the_removal`.
- **Windows: `--force` refused a worktree a program stands in.** Opening the
  root for the move fails with `ERROR_SHARING_VIOLATION`, which
  `rename::classify` mapped to `Permission` (a refusal). At `Stage::Open` it
  is now `Sharing`, so the checkout is deleted in place and the held root is
  waited on (R22). Unit row updated in
  `quarantine::rename::tests::windows_error_codes_classify_by_stage`; the
  binary case is `level2_powershell_with_force_deletes_a_worktree_another_program_holds_once_it_exits`
  (Windows L2, not re-run after the fix; see Gates).
- **Windows: the user's own PowerShell blocked a wrapper removal.** Found by
  the Windows cross-check (`powershell_wrapper_exec.rs`, a Windows-only L1
  file the rewrite agents could not compile on macOS): with the wrapper, the
  parent PowerShell was listed as `has the worktree open`, so the
  non-interactive first run refused (exit 3). Fixed by the parent lookup
  above. Tests: `powershell_wrapper_exec::move_first_removes_the_worktree_powershell_was_launched_inside`
  (Windows), `facts::tests::the_callers_own_shell_standing_in_the_checkout_is_not_listed`
  (the Windows open-handle form). The same file's refusal test now pins the
  warning: `a_directory_held_by_another_program_is_a_warning_and_refuses_without_force`.
- **Internal state names leaked into a resume refusal** (`Other`,
  `Original` from `Debug`). `resume::inspect` now says what is at the path
  and where Git records the worktree in words.

### Known and left for later phases

- **Hyphenated paths.** `biscuit-terminal`'s wrap hyphenates a word wider
  than the line, so a path longer than the terminal gets a `-` inserted.
  It only affects such paths; changing it is a `biscuit-terminal` decision.
- **Sniff's 2 s usage budget has no test override.** Under heavy load a
  no-`--force` binary test can see "the time allowed for the check ran
  out" (a warning, so exit 3). Seen once during the rewrite on a loaded
  host, not since. Recorded in `.claude/skills/worktree/testing.md`.
- **An interrupt between "yes" and the move does not stop the move**
  (`place_with` gets `&|| false`); the run still ends at the first deletion
  entry with the record kept. Ctrl-C while the question is open ends `wt`
  with nothing changed.
- **`docs/cli/remove.md` below "Deleting the checkout"** still describes
  the watcher, the journal, stop reports, and retry; the top of the page
  (usage, the report and question, `--force`, the final line, an unfinished
  removal, standing in the worktree, links, exit codes) is rewritten for the
  new behavior. Phase 9 rewrites the rest; Phase 7 deletes the machinery
  those sections describe.
- `lib/src/remove/usage.rs`'s module doc still describes the refusal; it
  goes with the refusal API in Phase 7.

### Tests (requirement → test)

L1 unless marked. CLI binary tests are rung F3 (the real `wt` binary,
real repositories in temp dirs); library tests are F2 with injected usage
and mounts.

| Requirement | Test |
|---|---|
| R28: `--force-worktree` is exit 2; help lists `--force`, `--force-branch`, `--force-remote` | `cli/tests/remove.rs::retired_flags_are_clap_errors`, `help_lists_the_three_force_flags_and_hides_handoff` |
| R28: `--force` alone keeps a Not-safe branch | `force_alone_keeps_a_branch_whose_commits_exist_nowhere_else`; `policy::tests::force_alone_keeps_an_unsafe_branch` |
| Policy matrix (one question; NeedsForce; clean proceeds; branch flags never implied) | `commands::remove::policy::tests::policy_matrix_follows_the_rules` (+3 examples) |
| Non-interactive with warnings: exit 3, advice on its own line, nothing removed (incl. `CI` set) | `wrapper_protocol::exit_3_when_removal_would_lose_files_and_nobody_can_confirm`, `example_dirty_files_without_a_terminal_refuse_with_nothing_removed` |
| Clean non-interactive: removed, quiet final line, no hidden name/`Retry:`/report | `example_clean_and_merged_removes_worktree_and_branch` |
| Question rendering: lock first, `⚠` items, late-change note, heading forms | `report::tests::the_rendered_question_lists_warnings_lock_first_and_the_late_change_note`, `a_clean_report_without_a_question_has_no_warnings_or_late_change_note` |
| Untrusted text literal (lock reason, path, note with ESC/markup) | `report::tests::untrusted_text_renders_literally`; `remove.rs::a_lock_reason_with_terminal_controls_and_markup_is_shown_literally` (Unix) |
| R3: lock reason shown; `--force` unlocks and deletes | `a_locked_worktree_shows_its_reason_and_force_unlocks_and_deletes_it`; missing + locked: `a_locked_worktree_whose_directory_is_gone_is_unlocked_and_its_record_removed_with_force` |
| R1: a process is a warning with its pid; `--force` deletes | `a_process_standing_in_the_worktree_is_a_warning_and_force_deletes_it` |
| R1/R15: the caller's own shell is not listed | `facts::tests::the_callers_own_shell_standing_in_the_checkout_is_not_listed`; `remove.rs::the_callers_own_shell_inside_is_not_listed_and_the_wrapper_run_hands_off` (fails without the filter: checked) |
| Only a real usage miss warns | `facts::tests::only_a_missed_usage_mechanism_warns` |
| R15 Unix warning; Windows exit 4 | `standing_inside_without_the_wrapper_is_a_warning_and_force_deletes`; `standing_inside_without_the_wrapper_on_windows_refuses_with_exit_4` (Windows) |
| R9 own link deletes both; foreign link only link + record; wrapper caller via own link gets `cd:` | `a_checkout_replaced_by_a_link_to_itself_is_a_warning_and_force_deletes_both`, `a_worktree_replaced_by_a_foreign_link_deletes_only_the_link_and_its_record`, `a_wrapper_caller_inside_a_checkout_reached_through_its_link_is_moved_out_after_the_removal` |
| R10/R11/R12/R13 warnings then deletion | `a_repair_that_is_not_verified_is_a_warning_and_force_deletes_it`, `a_worktree_replaced_by_a_file_is_a_warning_and_force_deletes_it_and_its_record`, `a_missing_directory_whose_index_is_gone_is_a_warning_and_force_removes_the_record`, `a_status_failure_is_a_warning_with_gits_reason_and_force_deletes_it` |
| Included copies warn; invalid rules warn | `changed_included_copy_requires_force_and_preserves_state_on_refusal`, `invalid_include_rules_are_a_warning_and_force_deletes_the_checkout`, `unchanged_included_copy_and_other_ignored_files_remove_without_force` |
| R14: a file created after the report is deleted and named in the final line | `remove.rs::a_file_created_after_the_report_is_deleted_and_named_in_the_final_line`, `remove_recovery.rs::a_file_created_after_the_report_is_deleted_and_named_in_the_final_line` |
| R18: second run compares no content (≈20 former refusals now deletions) | e.g. `a_restaged_version_between_the_runs_is_deleted_by_the_second_run`, `mod a_same_size_same_mtime_edit_between_the_runs_is_deleted_by_the_second_run`, `changed_include_rules_between_the_runs_are_deleted_by_the_second_run`, `executable_bit::…_is_deleted_by_the_second_run`, `non_utf8_paths::…_is_deleted_by_the_second_run` |
| R18: branch/remote changes between runs: checkout deleted, branch kept, exit 1 | `a_changed_branch_tip_between_the_runs_deletes_the_checkout_and_keeps_the_branch`, `a_branch_that_stops_being_safe_between_the_runs_is_kept_after_the_checkout_is_deleted`, `an_origin_branch_moved_between_the_runs_is_kept_by_the_lease`, `a_changed_origin_url_between_the_runs_keeps_the_origin_branch_after_the_checkout_is_deleted` |
| Remote already absent completes | `an_origin_branch_deleted_between_the_runs_does_not_stop_the_removal` |
| R18 network: second run makes no `ls-remote`, one `push` | `a_force_remote_second_run_runs_no_ls_remote_only_the_deletion_push`, `keeping_the_branch_makes_no_network_request_in_the_second_run` |
| R18 token: missing/malformed/expired exit 3; expired approval finished later | `a_missing_or_malformed_token_refuses_with_exit_3`, `an_expired_token_refuses_with_exit_3_and_a_later_remove_finishes_it` |
| R19: interrupt then retry by name finishes silently | `remove_recovery.rs::an_interrupt_after_the_move_cancels_and_a_retry_by_name_finishes_silently`, `a_retry_of_an_interrupted_removal_needs_no_consent_for_uncommitted_files` |
| R20: a write into the moved checkout is deleted | `remove_recovery.rs::a_write_into_the_moved_checkout_just_before_deletion_is_deleted`, `a_retry_deletes_new_work_written_into_the_moved_checkout_after_the_interrupt` |
| Recreated original kept; ambiguous name | `recreation_at_the_original_path_is_kept_and_the_moved_checkout_deleted`, `a_new_worktree_at_the_original_path_is_ambiguous_with_the_pending_removal` |
| Final line wording (counts, late changes, one-line Git errors) | `outcome::tests::{entry_counts_are_grouped_or_unknown, late_changes_name_at_most_five_files, gits_multi_line_errors_read_as_one_line, a_pending_sentence_reads_as_one_sentence}` |
| Git processes of a clean removal: 10 → 7 | `a_clean_removal_starts_7_git_processes` |
| L2: the interactive question with a process warning, answered yes, in tmux | `level2_remove.rs::level2_a_process_warning_in_the_question_answered_yes_deletes_the_worktree` |
| L2: one `Delete it?` question after one blank line, default No; included warnings; handoff edit deleted | `level2_remove_delete_question_follows_one_blank_line_and_defaults_to_no`, `level2_a_changed_included_file_warns_in_the_question_and_defaults_to_no`, `level2_an_included_edit_between_handoff_runs_is_deleted_with_the_checkout` |
| Windows L2 (PowerShell): program warning refuses without `--force`; `--force` deletes once it exits | `level2_powershell_without_force_or_a_terminal_names_the_program_and_refuses_with_exit_3`, `level2_powershell_with_force_deletes_a_worktree_another_program_holds_once_it_exits` |

Deleted test: `remove_recovery.rs::an_interrupt_during_a_quarantine_retry_stops_with_a_report`
(a retry makes no Git call during its deletion, so the shim cannot deliver
the interrupt there; the library covers cancellation during deletion in
`delete/tests.rs`). `too_little_space_for_the_journal_refuses_before_the_move`
went with the CLI's free-space variable.

Input robustness: no loader changed in this phase (the handoff loader is
Phase 5's; the CLI only writes `OperationHandoff`).

### Incident outside this worktree

A test-rewrite subagent ran a scratch script with a relative `cd repo`;
the exported `CDPATH` includes `~/coding/personal`, so it ran in
`/Users/ken/coding/personal/repo`, committed there (`d2b020d`, sweeping in
uncommitted changes to `src/analysis.ts` and `src/types.ts`), and added a
worktree. The subagent undid it (`git reset HEAD~1`, removed the worktree
and branch). Verified afterward: HEAD is `3f2f5c4`, the two files are
modified and unstaged, no `feat/x` branch or `wts/` directory, and an
untracked `README.md` containing `r` (created 2026-10-22 19:45) remains;
it was left for the author to delete. Whether those two files had staged
changes before is unknown. The trap is now in `.claude/skills/os/macos.md`.

### Gates

- `just lint` (`worktree/`): clean on the final code.
  `cargo clippy -p worktree -p worktree-cli --all-targets --target x86_64-pc-windows-gnu -- -D warnings`:
  clean.
- `just test` (`worktree/`), final code: 1,782 run, 1,781 passed, 15
  skipped, 1 timed out:
  `worktree-cli::list_prs::a_detached_workers_answer_replaces_the_stale_one_on_the_next_list`
  (the `wt list` PR worker; not touched by this phase). Rerun alone it
  passed in 0.7 s; the host's load average was 42 at the time (other
  sessions). Recorded as load-sensitive. The previous full run, before the
  last Windows-parent change, passed 1,782 of 1,782.
- `just check-tier-coverage worktree`: 0 stranded. New tests are L1 except
  `level2_a_process_warning_in_the_question_answered_yes_deletes_the_worktree`
  (L2, tmux) and the PowerShell L2 case.
- `level2_remove` (L2, tmux, macOS): 12 of 12 passed, run by the test
  subagent with `BISCUIT_TEST_REQUIRED_BACKENDS=tmux`, before the final
  library fixes; those fixes touch no interactive path the L2 file drives.
- Windows (`build-win-native`):
    - `just cross-check worktree --os windows`: 1,062 of 1,062 passed; after
      the final `facts` change, `remove::facts::` 36 of 36.
    - `just cross-check worktree-cli --os windows`: attempt 1, 640 of 642
      (the two `powershell_wrapper_exec.rs` failures above); attempt 2,
      after the fix, 642 of 642.
    - `level2_powershell_remove` (PowerShell 5.1 in ConPTY): 4 of 4,
      including `level2_powershell_with_force_deletes_a_worktree_another_program_holds_once_it_exits`,
      which failed before the sharing-violation fix.
    - `spec.md`'s `os_failing` is not set.
- Dependents: only `worktree-cli` depends on `worktree`; `just test` runs
  it. `sniff` was not changed.
- **Not run:** Linux and WSL2 cross-checks. No Linux-specific branch was
  added; `Caller::parent` uses the same Unix call on macOS and Linux.


## Phase 7

### What was removed

- **The activity watcher and checked deletion** (R23): all of
  `lib/src/remove/observation/` (FSEvents, inotify, `ReadDirectoryChangesW`
  adapters, `wait`, `injected`, `diagnostics`, the deletion inventory and
  checked engine, ~140 tests).
- **The journal-based lifecycle**: the old `lib/src/remove/lifecycle.rs`,
  `lifecycle/{resolve,resume}.rs`, and `lifecycle/tests/{large_checkout,
  resume_consent,startup_warning}.rs`.
- **The journal, the free-space gate, and `report.json`** (R25, R26):
  `recovery/{journal,space}.rs`, `outcome.rs` (`OperationReport`), and
  everything in `recovery/store.rs` that served them (`begin`, `reopen`,
  `recover`, `pending`, `read_report`, `Operation`, `ensure_space`,
  `StoreError::{InsufficientSpace, SpaceUnknown, Journal, JournalWrite,
  JournalSize, InventoryMismatch}`, the reopen fault seam). `store.rs` now
  opens the store and takes locks; `store/compact.rs` does the rest.
  `REPORT_FILE` stays only so a legacy operation's report is read for
  migration. `storage::open_append` went; `temp_beside`, `publish`,
  `create_private_file` became private to `write_durable`.
- **The usage refusal**: `usage::{check_usage, assess, UsageRefusal, InUse,
  UsageClearance, OwnedHandles, SearchRecheck, SEARCH_RECHECK_WAIT,
  WINDOWS_SEARCH_IMAGES, is_windows_search}` and
  `UsageSource::{system_directory, pause}`. `UsageSource::query` lost its
  tree-index argument. New `usage::holders(&report)` lists every process
  with evidence; `facts::gather` uses it.
- **Content comparison between handoff runs** (R18): the version 4
  `HandoffRecord`/`HandoffState`/`Approvals`/`write_record`/`consume`/
  `verify`/`checkout_git_dir`, `Inventory::fingerprint`,
  `inventory::bind_excluded`, `Inventory::{needs_consent,
  consent_sensitive_paths, has_source}`, and `included::BaselineBinding`
  plus the rules' content digest. An earlier `wt`'s v4 handoff now reads as
  missing (test below).
- **The old checked unregister**: `quarantine/unregister.rs` (+ tests),
  `missing::{remove_missing_record, remove_listed_record, IndexBinding,
  bind_index}` and the index digests on `MissingCheckout`, and
  `MissingRefusal::{IndexChanged, HeadChanged, Changed, Reappeared,
  PathUninspectable, RecordRemoval}`. Inspection (`inspect_missing`) stays.
- **The old move planner**: `association::{plan_move (old), move_to_quarantine,
  verify_quarantined, MovePlan::back_reference_rewritten,
  check_no_submodules}`, `MoveRefusal::Submodules`, the `confirmed` flag,
  and the `activity_observed` argument of `move_with`, `roll_back`,
  `placement::{place, place_with}` (its rollback guard was the watcher's).
  `plan_confirmed_move` is now `plan_move`. `RegistrationState` moved from
  `outcome.rs` to `association.rs`.
- **Dependencies**: `inotify` (with `inotify-sys`) and `fsevent-sys` left
  `worktree/lib/Cargo.toml` and `Cargo.lock`; `windows-sys` keeps
  `Win32_Foundation`, `Win32_Security`, `Win32_Storage_FileSystem` and drops
  `Win32_System_IO`, `Win32_System_Threading`,
  `Win32_System_SystemInformation`. `docs/dependencies.md` updated (both
  entries removed, the `worktree` paragraph rewritten). No per-area
  `worktree/docs/dependencies.md` exists.
- **Recipes and perf gates**: no `worktree/justfile` recipe or `perf_` test
  was watcher-specific (Phase 1 audit, re-checked). `.config/nextest.toml`
  lost the `fsevents-l1` group and its `remove::.*::native_` /
  `remove::observation::fsevents::` override (no such test remains).
- `crate::test_support::package_dir` (used only by watcher tests).

### Rename: `remove::confirmed` → `remove::lifecycle`

With the old lifecycle gone, the only lifecycle took its name
(`lib/src/remove/lifecycle.rs`, `lifecycle/{branches,late,resolve,resume,
tests}.rs`). CLI imports and doc links follow. The skill page
`confirmed.md` became `lifecycle.md` (old `lifecycle.md` and
`observation.md` deleted). This is a departure from the plan's wording,
which still says `confirmed`; the previous phase's agent suggested it.

### Drift found and resolved

Code is assumed correct; each comment below was fixed or deleted:

- `remove/mod.rs` module doc described the watcher, the outcome report, and
  "the lifecycle that replaces it" → rewritten for what exists.
- `recovery/mod.rs` described the journal layout "still written by the
  journal-based lifecycle" and its crash guarantees → rewritten for the
  compact record only, with legacy migration.
- `recovery/record.rs`: `RECORD_VERSION` "the only version this build reads
  or writes" and `consent_fingerprint` linking to the deleted
  `Inventory::fingerprint` → legacy-only wording; `to_json` is now
  `#[cfg(test)]`.
- `usage.rs` module doc (stale since Phase 6) → rewritten.
- `handoff.rs` module doc described the content-comparing handoff and the
  60 s TTL → the operation handoff, 10 min.
- `inventory.rs`, `included.rs`, `missing.rs` module docs spoke of consent
  bindings and removal re-reads → inspection only.
- `association.rs` module and item docs named `plan_confirmed_move`,
  `move_to_quarantine`, and the activity guard → updated.
- `quarantine/mod.rs` named `unregister` → removed.
- CLI `commands/remove/mod.rs` "confirmed lifecycle" → "removal lifecycle".

### Success-criteria grep

`grep -ri "fsevent\|inotify\|ReadDirectoryChanges\|CoverageLost\|journal"
worktree/` (excluding `features/`, `fixes/`): code hits remain only where
they are correct, which the plan's "finds only docs" did not foresee:

- legacy migration deletes `journal.jsonl`/`journal-<n>.jsonl` unread, so
  its docs and tests name them (`recovery/store/compact.rs`, its tests,
  `lifecycle/tests.rs`'s legacy case, `recovery/legacy/tests.rs`);
- `facts.rs` labels Sniff's own coverage mechanisms (`Mechanism::Fsevents`,
  `Mechanism::Inotify`) when describing a missed usage query, and
  `usage/tests.rs` builds such reports.
- `worktree/docs/cli/remove.md` still describes the watcher; its rewrite is
  Phase 9's (unchanged here).

### Tests (requirement → test)

All L1, `worktree` library (F2: real repositories in temp dirs, injected
usage/mounts/clock).

| Requirement | Test |
|---|---|
| Large-checkout coverage that still applies: a crash part-way through deleting a multi-batch checkout is finished by the next run, asking nothing, counting only its own deletions | `remove::lifecycle::tests::a_crash_part_way_through_deleting_a_built_checkout_is_finished_by_the_next_run` (new; 2,148 built files, > 2 batches) |
| Legacy (v1) record loader keeps its full robustness matrix after `recover` was deleted | `remove::recovery::store::tests::record_robustness_matrix` (ported to assert through `RecoveryStore::resume`; unknown version is now 99 since 2 is the compact version) |
| A linked operation directory is never read | `remove::recovery::store::tests::a_linked_operation_directory_is_never_read` (ported to `operations()` + `resume`) |
| R25/R26: journals and a report beside a record are deleted unread; legacy v1 + report v6 finishes without a question | existing `store::compact::tests::{resuming_a_compact_record_deletes_leftover_journals_and_reports_without_reading_them, a_legacy_operation_migrates_to_a_compact_record_and_drops_its_journals_and_report}`, `lifecycle::tests::a_legacy_operation_is_finished_and_its_unbound_remote_step_left_undone` |
| Usage is facts, not a refusal | `remove::usage::tests::{every_evidence_kind_makes_a_holder, no_match_has_no_holders_whatever_the_coverage, a_match_in_a_partial_report_is_a_holder_and_its_gaps_are_summarized, locations_report_aliases_and_identity_only_evidence_as_facts, a_real_query_names_a_process_working_in_the_checkout, a_real_query_of_a_vanished_checkout_is_an_error}` (rewritten) |
| R18: an earlier `wt`'s content-comparing handoff (v4) is never used | `remove::handoff::tests::only_the_operation_format_is_an_operation_handoff` (now writes raw v4 JSON); `paths_compare_canonical` ported to `verify_operation` |
| A lock does not refuse planning but the move still refuses and keeps it | `remove::quarantine::association::tests::a_locked_worktree_plans_but_refuses_the_move` (rewritten) |
| Submodules never refuse the move (R2) | `remove::quarantine::association::tests::a_checkout_with_a_submodule_moves` (rewritten from the refusal test) |
| Missing checkout inspection unchanged; split index read by Git | `remove::missing::tests::{a_record_without_staged_work_needs_no_consent, staged_work_in_a_split_index_is_reported, a_split_index_whose_shared_file_is_corrupt_cannot_be_inspected}` (rewritten from removal tests) |

Deleted with the code they tested (no behavior left to test): the observer
suites, journal/space/report tests, `store/tests.rs`'s journal matrix and
begin/reopen tests, usage exclusion and Windows Search tests,
`quarantine/unregister/tests.rs`, `missing.rs`'s removal and split-index
re-read tests, `inventory.rs`'s 12 fingerprint tests, the old handoff's
record tests, `association`'s observed-activity rollback test, and the old
lifecycle's tests (whose surviving behaviors Phase 5 re-covered in
`lifecycle/tests.rs`).

Input robustness: no loader was added or changed. The legacy record
loader's matrix is preserved (ported, all cells kept, control row
included); the handoff loader is unchanged.

### Gates

- `just lint` (`worktree/`): clean.
- `just test` (`worktree/`): 1,524 run, 1,524 passed, 14 skipped (was 1,782;
  the difference is the deleted suites).
- Targeted run before the full suite: 67 of 67 (`store::tests`, `usage`,
  `handoff`, `missing`, `inventory`, `quarantine`, `included`, the new
  crash test).
- `just check-tier-coverage worktree`: 0 stranded.
- `cargo check -p worktree -p worktree-cli --target x86_64-pc-windows-gnu
  --all-targets`: clean (Windows-only tests compile after the
  `move_with`/`place` signature change).
- rustdoc intra-doc links in `lib/src/remove/`: no new broken link (one
  pre-existing in `association.rs`'s module doc, `rename_no_replace`).
- Dependents: only `worktree-cli` depends on `worktree`; `just test` covers
  it.
- Windows (`build-win-native`), attempt 1, final code (before a comment-only
  rewrap in `association.rs`):
    - `just cross-check worktree --os windows`: 816 of 816 passed.
    - `just cross-check worktree-cli --os windows`: 642 of 642 passed
      (44 skipped).
    - `spec.md`'s `os_failing` is not set.
- **Not run:** Linux and WSL2 cross-checks. No Linux-specific code was
  changed; the removed `inotify` adapter was self-contained, and the
  remaining `cfg(target_os = "linux")` code (`rename`, `delete/platform`,
  `mounts/linux`, `include`) is untouched. No local Linux cross toolchain
  exists for `cargo check --target x86_64-unknown-linux-gnu` (`aws-lc-sys`
  needs `x86_64-linux-gnu-gcc`).
- CI scope calculator after all edits
  (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs
  python3 scripts/ci/affected_scope.py`): `packages` = `worktree`,
  `worktree-cli`, `test-toolkit`, `darkmatter`, `sniff`; `change_class` =
  `package`. The last three come from the shared `.config/nextest.toml` and
  `Cargo.lock` edits (a `worktree`-only nextest group and two
  `worktree`-only crates removed), so their suites were not run locally;
  the gates for `worktree` were run.

## Phase 8

### What was done

- **Ruling coverage audit** (table below): every ruling R1–R29 maps to at
  least one L1 test. Phases 1–7 had already covered almost all of them; this
  phase filled three gaps and found one product defect.
- **Defect found and fixed: an unreadable pending record was never
  explained.** Name resolution (`remove::lifecycle::resolve_in`) skipped a
  record it could not read, so `wt remove feat-x` after an interrupted
  removal whose record was later corrupted, of an unknown version, or
  another operation's said only that no worktree is named `feat-x`. The
  spec asks for "a plain explanation and no speculative deletion". Now,
  when a name matches nothing and the store holds an unreadable record,
  resolution returns that record's `ResolveError::Store`, which the CLI
  already renders as `Nothing was removed. The record of an earlier removal
  could not be read: removal record <id> cannot be used: …` (exit 1).
  A name that matches a live worktree or a readable operation still
  resolves as before (an unreadable record still blocks a new approval, as
  it did). Doc comments of `resolve` and `pending_operations` updated.
- **Gaps filled:** a submodule with an uncommitted file *and* a commit on
  no remote, asserted end to end (warning content, then deletion with
  `modules/<name>`); files *changed* and *staged* while the question is open
  (only *created* was covered), including that a file the question already
  showed is not named again; the unreadable-record case above, in the
  library and through the binary.
- **L2** (`just test-l2`, macOS, tmux, detached): 42 of 42, including
  `level2_a_process_warning_in_the_question_answered_yes_deletes_the_worktree`.
  No window took focus (tmux panes only).
- **Defect found by the WSL2 run and fixed: removal on drvfs always failed.**
  On `/mnt/c`, `wt remove a --force` showed the drvfs warning and then
  exited 1 with `Nothing was removed: …/.git/wt-removal cannot be used:
  other users can access it`. drvfs (mounted `9p`, `aname=drvfs`, no
  `metadata` option) reports every entry as `0o777` and ignores `chmod`,
  so the store's owner-only check could never pass and R4 was unreachable.
  `recovery/storage.rs`'s Unix `check_owner` now skips the open-bits
  refusal only when the path's containing mount is drvfs
  (`mounts::Mount::is_drvfs`), still checking ownership; the mount table is
  read only on that failing branch. This mirrors native Windows, which
  relies on the inherited ACL. Flagged for human confirmation
  (`human_review_items`).
- **Departure from the spec, not fixed here (human decision): on a very
  large checkout the question always says "Some programs could not be
  checked".** Since Phase 7 no tree index is supplied to Sniff, so its usage
  query walks the checkout inside its one 2 s budget. On the 500,000-entry
  checkout the walk exhausted the budget before any process was
  enumerated (reproduced three times; `sniff filesystem query` of the same
  tree: 2.06 s with the default budget, `budget_exhausted`, `unavailable`;
  3.52 s and `usable` with a 60 s budget). Deletion is unaffected; a
  non-interactive run needs `--force` there. Options and a recommendation
  are in `human_review_items`.

### Ruling coverage audit (R1–R29 → tests)

L1 unless marked. Library tests are F2 (real repositories in temp dirs;
usage, mounts, clock, faults injected); `cli/tests/*` are F3 (the real `wt`
binary). New or changed in this phase: **bold**.

| Ruling | Tests |
|---|---|
| R1 process warning; yes deletes; non-interactive refuses without `--force` | `facts::tests::each_process_is_named_with_what_it_is_doing`; `cli/tests/remove.rs::a_process_standing_in_the_worktree_is_a_warning_and_force_deletes_it`; L2 `level2_remove::level2_a_process_warning_in_the_question_answered_yes_deletes_the_worktree` |
| R2 submodules: work only there warns (uncommitted + unpushed), deleted with `modules/<name>`; clean adds no warning | **`lifecycle::tests::a_worktree_with_a_submodule_is_deleted_with_its_module_data`** (now with an unpushed commit and the warning asserted); `facts::tests::{a_submodule_with_work_only_there_warns_and_a_clean_one_does_not, a_worktree_whose_submodules_are_clean_has_no_warnings}`; `submodules::tests::*` |
| R3 lock reason first; yes unlocks and deletes; missing + locked | `facts::tests::{a_lock_reason_is_shown_first, a_lock_is_warned_about_when_the_directory_is_gone}`; `lifecycle::tests::{a_locked_worktree_is_unlocked_and_deleted, a_lock_added_after_the_question_is_never_removed, a_missing_locked_worktree_has_its_record_removed}`; `remove.rs::a_locked_worktree_shows_its_reason_and_force_unlocks_and_deletes_it` |
| R4 drvfs: warning; in place when no exclusive rename; collision never replaced | `facts::tests::a_checkout_on_drvfs_warns_and_suggests_windows`; `lifecycle::tests::a_filesystem_without_exclusive_rename_deletes_in_place`; `placement::tests::a_collision_at_the_hidden_name_stays_an_error_and_replaces_nothing`; **`storage::tests::open_permissions_are_accepted_only_on_drvfs`**; manual WSL2 run below |
| R5 FIFOs/sockets unlinked unopened | `delete::tests::fifos_and_sockets_are_unlinked_without_being_opened` |
| R6 reparse points deleted, never traversed; synced-folder warning | `delete::tests::links_inside_the_checkout_are_removed_and_their_targets_never_touched` (junctions on Windows take the same reparse-point branch as placeholders); `facts::tests::a_checkout_inside_a_synced_folder_warns_about_cloud_copies`. A real cloud placeholder cannot be made without a sync provider: not tested |
| R7 nested mount kept (incl. same-device bind), exit 1, later run finishes | `lifecycle::tests::a_nested_mount_is_kept_until_unmounted_and_then_finished`; `mounts::tests::a_same_device_bind_mount_is_nested`; `mountinfo::tests::a_same_device_bind_mount_is_listed`; `delete::tests::a_nested_mount_is_never_crossed_and_everything_else_is_deleted` |
| R8 relative `.git` link rewritten for the move | `association::tests::{a_link_through_the_checkout_is_rewritten_for_the_move, a_rewritten_link_is_restored_when_the_move_is_rolled_back, a_relative_record_keeps_its_format}` |
| R9 own link deletes both; foreign link only link + record | `lifecycle::tests::{a_link_to_the_own_checkout_deletes_both, a_link_to_another_directory_deletes_only_the_link_and_the_record}`; `remove.rs` own/foreign link cases |
| R10–R13 unreadable Git, file at path, index gone, unchecked files | `facts::tests::{an_unverifiable_repair_warns_that_git_cannot_read_it, a_file_at_the_path_says_what_is_there, a_missing_directory_without_its_index_cannot_check_staged_changes, a_failed_status_is_unchecked_with_its_reason}`; `lifecycle::tests::a_file_at_the_path_is_deleted_with_the_record`; `remove.rs` R10–R13 `--force` cases; read-only entries: `delete::tests::read_only_*` |
| R14 created/changed/staged after the question deleted and named | `lifecycle::tests::a_file_created_after_the_question_is_deleted_and_named`, **`files_changed_and_staged_while_the_question_was_open_are_deleted_and_named`**; `remove_recovery.rs::a_file_created_after_the_report_is_deleted_and_named_in_the_final_line` |
| R15 shell inside: Unix warning, Windows exit 4 | `facts::tests::a_shell_inside_without_the_wrapper_{warns_on_unix,refuses_on_windows}`; `lifecycle::tests::the_shell_inside_blocker_refuses_with_exit_4`; `remove.rs` R15 cases |
| R16 concurrent removal waits, continues; cancellable; stale lock | `lifecycle::tests::{a_second_run_waits_for_the_running_removal_and_continues_it, the_wait_for_a_running_removal_honors_cancellation}`; `store::compact::tests::a_stale_lock_file_and_a_crashed_owners_record_are_taken_over_at_once` |
| R17 vanished checkout is already removed | `facts::tests::a_checkout_that_vanishes_during_the_usage_query_is_already_removed` |
| R18 handoff second run: no content comparison, 10 min, single use | `lifecycle::tests::the_handoff_second_run_finishes_once_the_caller_left`; `handoff::tests::*`; `remove.rs` R18 cases |
| R19 crash at every edge finished silently; legacy v1 + report v6; unbound remote step undone; replaced roots/records/recreated originals never deleted; malformed/unknown records explained | `lifecycle::tests::{a_crash_at_every_step_is_finished_by_the_next_run, a_crash_part_way_through_deleting_a_built_checkout_is_finished_by_the_next_run, a_crash_between_the_rename_and_the_registration_is_finished, a_recreated_original_is_kept_and_the_moved_checkout_deleted, a_replaced_quarantine_is_never_deleted, a_replaced_in_place_root_is_kept_and_the_operation_closed, a_replaced_worktree_record_is_never_resumed, a_legacy_operation_is_finished_and_its_unbound_remote_step_left_undone}`, **`an_unreadable_record_of_a_pending_removal_is_explained_and_nothing_is_deleted`**; **`remove_recovery.rs::an_unreadable_record_of_an_interrupted_removal_is_explained_and_nothing_is_deleted`**; `store::compact::tests::{corrupt_foreign_and_unknown_version_records_are_explained_and_kept, a_migration_stopped_before_publishing_leaves_the_legacy_files_and_can_run_again, …}`; `legacy::tests::*` |
| R20 activity during deletion: continues | `remove_recovery.rs::a_write_into_the_moved_checkout_just_before_deletion_is_deleted` |
| R21 / R23 watcher gone | deleted with the code in Phase 7 (no test remains to keep) |
| R22 held file waited on, named, finished in the same run; cancellable | `lifecycle::tests::a_held_file_is_waited_on_and_the_removal_finishes_in_the_same_run`; `delete::tests::{a_held_file_is_waited_on_after_everything_else_is_deleted, a_windows_denial_with_a_named_holder_is_waited_on, cancelling_a_wait_ends_deletion_and_names_what_it_waited_on}`; Windows L2 `level2_powershell_with_force_deletes_a_worktree_another_program_holds_once_it_exits` |
| R24 refilling directory relisted, then waited on with its writer named | `delete::tests::{a_directory_that_refills_a_few_times_is_relisted_and_deleted_without_waiting, a_directory_that_keeps_refilling_is_waited_on_and_its_writer_named}` |
| R25 / R26 compact record only; nothing left after completion | `lifecycle::tests::a_confirmed_checkout_is_deleted_in_one_run_and_leaves_nothing`; `store::compact::tests::a_finished_operation_leaves_nothing_in_the_store`; `compact::tests::*` |
| R27 process context | `facts::tests::{a_process_is_named_with_its_start_and_the_known_program_it_was_run_from, sniffs_process_context_reaches_the_warning}` |
| R28 `--force` only; `--force-worktree` exit 2; `--force` keeps an unsafe branch | `remove.rs::{retired_flags_are_clap_errors, help_lists_the_three_force_flags_and_hides_handoff, force_alone_keeps_a_branch_whose_commits_exist_nowhere_else}`; `policy::tests::*` |
| R29 permanent failure: rest deleted, one sentence, exit 1, no retry, later run finishes; unsaveable approval deletes nothing | `lifecycle::tests::{a_permanent_failure_deletes_the_rest_and_a_later_run_finishes, an_approval_that_cannot_be_saved_changes_nothing}`; `delete::tests::{a_permanent_failure_deletes_everything_else_and_is_never_retried, a_denial_nobody_explains_is_permanent_after_one_fix}` |
| Non-interactive: refusal without `--force`, `CI` set, clean removes silently | `wrapper_protocol::exit_3_when_removal_would_lose_files_and_nobody_can_confirm` (`CI` unset and `true`); `remove.rs::{example_dirty_files_without_a_terminal_refuse_with_nothing_removed, example_clean_and_merged_removes_worktree_and_branch}` |
| Branch regressions: moved tip, endpoint, lease | `lifecycle::tests::{a_branch_that_moved_is_kept_after_the_checkout_is_deleted, a_conditional_branch_is_rechecked_by_a_later_run}`; `remove.rs` origin-moved/URL-changed/lease cases; `remote::tests::*` |

Each new test fails against the code it guards: the two unreadable-record
tests got `NotFound` before the `resolve` fix (checked); the drvfs storage
test's accepted cells returned `OpenPermissions` before the
`storage.rs` fix; the R14 test asserts names the previous test set never
produced (changed and staged paths).

**Input robustness.** No loader changed. The unreadable-record tests drive
three record shapes (malformed JSON, `record_version` 3, another
operation's id) through the public lifecycle and CLI path; the loader's
full matrix stays in `store::compact::tests` and `compact::tests`.

### Large-checkout acceptance (macOS, once)

Release `wt`, a disposable repository in `/tmp`, one worktree with
500,000 ignored entries under `target/` (500 directories × 999 files, built
in 25 s) plus one untracked file so the question is asked. Run in a
detached tmux pane (no window), answered `y`:

- One run, exit 0, no retry, no wait. The pane showed only the question and
  `Removed big (500,007 entries) and deleted branch feat/big.` (no hidden
  name, `Retry:`, report path, watcher, or journal text).
- Question shown 2.1 s after start; **18.9 s** from "yes" to exit.
- **Compact record: 1,202 bytes** (largest size seen while polling the
  store every 5 ms). After the run: no checkout, no hidden directory, an
  empty store, no branch, only the main worktree registered.
- The question also carried the "Some programs could not be checked"
  warning described above (the budget finding).
- A first attempt under `script(1)` with "y" piped in hung at the prompt:
  the terminal capability queries consumed the input. It was killed (nothing
  changed) and the tmux run above replaced it.

### Windows and WSL2

- **Windows (`build-win-native`):** `just cross-check worktree --os windows`
  (before the `storage.rs` change, whose Windows branch only gained an
  unused path parameter): 819 of 819. `./scripts/cross-check.sh --os windows
  worktree-cli --all-features level2_powershell` (native mode, PowerShell 5.1
  in ConPTY): 4 of 4, 6.4–7.2 s each (they ran), including
  `level2_powershell_with_force_deletes_a_worktree_another_program_holds_once_it_exits`
  (a held directory is waited on, the program named, and the removal
  finishes after it exits). `cargo check -p worktree -p worktree-cli
  --all-targets --target x86_64-pc-windows-gnu` clean after the last
  change. `os_failing` not set.
- **WSL2 (`build-win`):** `just cross-check worktree --os wsl remove::recovery
  remove::lifecycle`: first attempt 76 of 77. The failure was this phase's
  submodule test committing without an identity (the rig runs with no
  global Git config); fixed with `append_config` on the module's config, as
  `facts::tests` does. Second attempt: 77 of 77.
- **WSL2 drvfs, manual** (`wt` built in the guest from this tree, repository
  under `/mnt/c/Users/Public`):
    - without `--force`, no terminal: the uncommitted-files and drvfs
      warnings, exit 3, nothing changed;
    - with `--force`: first attempt exit 1 "other users can access it"
      (the defect above); after the fix `Removed a (54 entries) and deleted
      branch feat/a.`, exit 0, 4.2 s, no hidden directory, registration and
      branch gone;
    - `renameat2(RENAME_NOREPLACE)` on drvfs onto an occupied name: `EEXIST`,
      the occupant intact; onto an absent name: `EINVAL` (so `wt` deletes in
      place there and never creates a hidden name). A collision through `wt`
      itself cannot be staged on drvfs: the hidden name carries a fresh
      token. The collision rule is covered by the L1 placement test.
- **Not run:** a full `worktree-cli` L1 cross-check on Windows (this
  phase's CLI change is a Unix-only test file) and a Linux cross-check
  (`storage.rs`'s new branch is Linux-reachable only on drvfs, exercised on
  the WSL2 guest).

### Gates

- `just test` (`worktree/`), final code: 1,528 of 1,528 passed, 14
  skipped. An earlier full run (before the `storage.rs` fix) had one
  timeout, `worktree-cli::list_prs::a_failed_assertion_while_a_pr_request_is_held_still_reaps_the_worker_before_the_fixture_goes`
  (the `wt list` PR worker, not touched here); it passed alone in 0.6 s,
  load average 11 at the time. Recorded as load-sensitive.
- `just lint` (`worktree/`): clean (one `type_complexity` in a new test,
  fixed).
- `just test-l2` (`worktree/`): 42 of 42.
- `just check-tier-coverage worktree`: 0 stranded.
- Dependents: only `worktree-cli` depends on `worktree`; `just test` runs it.
- Release build warnings seen, pre-existing and outside this feature:
  `TEST_WAIT_BUDGET_VAR` and `TEST_REFRESH_DEADLINE_VAR` unused in
  `cli/src/env.rs` (`wt list` test seams; used only under `debug_assertions`).
- Windows durations to watch (5 s L1 budget): pre-existing
  `facts::tests::a_submodule_with_work_only_there_warns_and_a_clean_one_does_not`
  took 5.1 s in the full Windows run. This phase's first version of the R2
  test (two submodules) took 5.9 s there; it was folded into the existing
  one-submodule test instead (3.2 s there before the fold).
- CI scope calculator after all edits (`git status --porcelain
  --untracked-files=all | awk '{print $NF}' | xargs python3
  scripts/ci/affected_scope.py`): `packages` = `worktree`, `worktree-cli`,
  `test-toolkit`, `darkmatter`, `sniff`; `change_class` = `package`, so the
  gates were run. The working tree still holds Phase 7's uncommitted
  `Cargo.lock` and `.config/nextest.toml` edits, which bring in the last
  three; this phase changed only `worktree` and `worktree-cli` files, and
  their suites were not run locally.
