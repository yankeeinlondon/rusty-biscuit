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
message_to_agent: |-
    Phase 1 adopted rulings N1-N12 as proposed (yolo plan; the author never
    saw them). Read the plan's "Phase 1 notes" before starting: they correct
    later phases.
    - cli/src/commands/remove/report.rs is the status report printed before
      the question, not report.json. report.json is recovery/store.rs
      (REPORT_FILE, save_report) plus outcome::OperationReport.
    - The free-space gate is recovery/space.rs plus store.rs ensure_space and
      its StoreError variants, plus WT_TEST_AVAILABLE_SPACE_BYTES in
      cli/src/env.rs. It is not in storage.rs.
    - observation/inventory.rs (DeletionInventory) is used by usage.rs (tree
      index), store.rs, lifecycle.rs, and resume.rs.
    New seams in worktree/lib/src/remove: clock.rs (Clock, HostClock,
    Backoff), mounts.rs (Mount, MountInfo, mounts_within), and faults.rs
    (FsOp, FsFaults, NoFaults). Their test doubles (ManualClock, FixedMounts,
    ScriptedFaults) are in remove/test_support.rs. No new lifecycle Boundary
    variants were added; add each one in the phase whose step emits it (the
    list is in the plan notes).
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
