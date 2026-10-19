---
spec: /Volumes/coding/wt/rusty-biscuit/feat-pipeline/worktree/fixes/2026-10-18-remove-observation-under-load/spec.md
plan: worktree/fixes/2026-10-18-remove-observation-under-load/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
  - worktree/lib/src/remove/observation/wait.rs
  - worktree/lib/src/remove/observation/mod.rs
  - worktree/lib/src/remove/observation/tests.rs
  - worktree/lib/src/remove/observation/fsevents.rs
  - worktree/lib/src/remove/observation/read_directory_changes.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/worktree/observation.md
source_files_during_phase_2:
  - worktree/lib/src/remove/observation/wait.rs
  - worktree/lib/src/remove/observation/mod.rs
  - worktree/lib/src/remove/observation/fsevents.rs
  - worktree/lib/src/remove/observation/fsevents/tests.rs
  - worktree/lib/src/remove/observation/read_directory_changes.rs
  - worktree/lib/src/remove/observation/read_directory_changes/tests.rs
  - worktree/lib/src/remove/observation/inotify.rs
  - worktree/lib/src/remove/observation/inotify/tests.rs
  - worktree/lib/src/remove/observation/injected.rs
  - worktree/lib/src/remove/observation/tests.rs
  - worktree/lib/src/remove/observation/inventory/tests.rs
  - worktree/lib/src/remove/observation/deletion.rs
  - worktree/lib/src/remove/observation/deletion/tests.rs
  - worktree/lib/src/remove/lifecycle.rs
  - worktree/lib/src/remove/lifecycle/resume.rs
  - worktree/lib/src/remove/lifecycle/tests.rs
  - worktree/lib/src/remove/quarantine/unregister.rs
  - worktree/lib/src/remove/quarantine/unregister/tests.rs
docs_updated_during_phase_2:
  - worktree/docs/cli/remove.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/worktree/observation.md
  - .claude/skills/os/macos.md
packages:
  - worktree
  - worktree-cli
source_files_during_phase_3:
  - worktree/lib/src/remove/observation/diagnostics.rs
  - worktree/lib/src/remove/observation/mod.rs
  - worktree/lib/src/remove/observation/wait.rs
  - worktree/lib/src/remove/observation/injected.rs
  - worktree/lib/src/remove/observation/fsevents.rs
  - worktree/lib/src/remove/observation/fsevents/tests.rs
  - worktree/lib/src/remove/observation/read_directory_changes.rs
  - worktree/lib/src/remove/observation/read_directory_changes/tests.rs
  - worktree/lib/src/remove/observation/inotify.rs
  - worktree/lib/src/remove/observation/tests.rs
  - worktree/lib/src/remove/observation/deletion.rs
  - worktree/lib/src/remove/observation/deletion/tests.rs
  - worktree/lib/src/remove/quarantine/unregister.rs
  - worktree/lib/src/remove/outcome.rs
  - worktree/lib/src/remove/lifecycle.rs
  - worktree/lib/src/remove/lifecycle/resume.rs
  - worktree/lib/src/remove/lifecycle/tests.rs
  - worktree/cli/tests/remove_recovery_report.rs
docs_updated_during_phase_3:
  - worktree/docs/cli/remove.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/worktree/observation.md
  - .claude/skills/worktree/lifecycle.md
source_files_during_phase_4:
  - worktree/lib/src/remove/inventory.rs
  - worktree/lib/src/remove/lifecycle.rs
  - worktree/lib/src/remove/lifecycle/resume.rs
  - worktree/lib/src/remove/lifecycle/tests.rs
  - worktree/lib/src/remove/lifecycle/tests/resume_consent.rs
  - worktree/cli/src/commands/remove/mod.rs
  - worktree/cli/tests/remove_recovery.rs
docs_updated_during_phase_4:
  - worktree/docs/cli/remove.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/worktree/lifecycle.md
source_files_during_phase_5:
  - worktree/lib/src/remove/observation/mod.rs
  - worktree/lib/src/remove/observation/deletion.rs
  - worktree/lib/src/remove/lifecycle.rs
  - worktree/lib/src/remove/lifecycle/resume.rs
  - worktree/lib/src/remove/lifecycle/tests.rs
  - worktree/lib/src/remove/lifecycle/tests/startup_warning.rs
  - worktree/cli/src/commands/remove/recovery.rs
  - worktree/cli/src/commands/remove/mod.rs
  - worktree/cli/tests/remove_recovery.rs
  - worktree/cli/tests/remove_recovery_report.rs
  - worktree/cli/tests/snapshots/remove_recovery_report__a_macos_stall_after_the_move_points_at_fseventsd.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_slow_startup_warns_with_its_duration.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_stop_names_the_fsevents_flags_that_dropped_changes.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_watch_that_never_delivered_refuses_with_its_reason.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_windows_ceiling_names_the_watched_folder.snap
docs_updated_during_phase_5:
  - worktree/docs/cli/remove.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/worktree/observation.md
  - .claude/skills/worktree/lifecycle.md
  - .claude/skills/os/macos.md
  - .claude/skills/os/SKILL.md
source_files_during_phase_6:
  - worktree/lib/src/remove/observation/diagnostics.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/os/linux.md
source_code:
  - worktree/cli/src/commands/remove/mod.rs
  - worktree/cli/src/commands/remove/recovery.rs
  - worktree/cli/tests/remove_recovery.rs
  - worktree/cli/tests/remove_recovery_report.rs
  - worktree/cli/tests/snapshots/remove_recovery_report__a_macos_stall_after_the_move_points_at_fseventsd.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_slow_startup_warns_with_its_duration.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_stop_names_the_fsevents_flags_that_dropped_changes.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_watch_that_never_delivered_refuses_with_its_reason.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_windows_ceiling_names_the_watched_folder.snap
  - worktree/lib/src/remove/inventory.rs
  - worktree/lib/src/remove/lifecycle.rs
  - worktree/lib/src/remove/lifecycle/resume.rs
  - worktree/lib/src/remove/lifecycle/tests.rs
  - worktree/lib/src/remove/lifecycle/tests/resume_consent.rs
  - worktree/lib/src/remove/lifecycle/tests/startup_warning.rs
  - worktree/lib/src/remove/observation/deletion.rs
  - worktree/lib/src/remove/observation/deletion/tests.rs
  - worktree/lib/src/remove/observation/diagnostics.rs
  - worktree/lib/src/remove/observation/fsevents.rs
  - worktree/lib/src/remove/observation/fsevents/tests.rs
  - worktree/lib/src/remove/observation/injected.rs
  - worktree/lib/src/remove/observation/inotify.rs
  - worktree/lib/src/remove/observation/inotify/tests.rs
  - worktree/lib/src/remove/observation/inventory/tests.rs
  - worktree/lib/src/remove/observation/mod.rs
  - worktree/lib/src/remove/observation/read_directory_changes.rs
  - worktree/lib/src/remove/observation/read_directory_changes/tests.rs
  - worktree/lib/src/remove/observation/tests.rs
  - worktree/lib/src/remove/observation/wait.rs
  - worktree/lib/src/remove/outcome.rs
  - worktree/lib/src/remove/quarantine/unregister.rs
  - worktree/lib/src/remove/quarantine/unregister/tests.rs
documentation:
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/linux.md
  - .claude/skills/os/macos.md
  - .claude/skills/worktree/lifecycle.md
  - .claude/skills/worktree/observation.md
  - worktree/docs/cli/remove.md
completed_phase: 6
implemented: true
human_review: true
human_review_items:
  - |-
    **macOS: deleting 512 entries between checks did not always prevent lost change events**

    This fix makes `wt remove` on macOS stop deleting every 512 entries (it was 2,048) to check that
    the macOS change watcher (FSEvents) has caught up. The spec says that if a quick 100,000-entry
    sample still loses events, the change is unproven, and the author must decide what to do next
    before anyone claims it works.

    What happened on this Mac (two samples, 101,002 entries each):

    - **Sample 1** started about a minute after the 100,000 test files were created. It stopped
      after 18,600 deletions at about 2,900 per second, slower than either original incident. macOS
      reported `UserDropped` + `MustScanSubDirs`. Running `wt remove` again with no force flag
      finished the other 82,402 entries at about 8,200 per second with no loss.
    - **Sample 2** waited 90 seconds after creating the files on a quieter host. It finished all
      101,002 entries in 24.7 s (about 4,400 per second) with no loss.

    So deletion speed alone does not explain the loss. The most likely cause is that macOS was
    still processing the events from creating the files. That is unconfirmed. Full numbers are in
    the Phase 6 section of the implementation log.

    This must be decided before the fix is closed, because it determines whether the cadence part of
    the fix counts as done. The rest of the fix works either way. Sample 1 showed the new failure
    message naming the exact macOS flags, and the retry finishing without extra prompts.

    Options:

    - **A. Accept 512 as a mitigation and close the fix.** Write it down as "reduces but does not
      prevent" loss. The safety net already handles a loss: deletion stops, and a retry finishes
      without asking again.
      - *Pros:* no more work, and the behavior is safe and recoverable.
      - *Cons:* users can still see a stop on a large, freshly built checkout and need a retry.
    - **B. Open a follow-up to pace deletion by time, not by count.** For example, pause when the
      watcher reports it is falling behind, or cap deletions per second.
      - *Pros:* goes after the cause directly.
      - *Cons:* more design work. The spec warns against adapting to queue depth without a reliable
        signal from macOS.
    - **C. Have `wt remove` retry the deletion once by itself after an FSEvents drop.** The checks
      that run before a retry would run again.
      - *Pros:* most users would never see the stop.
      - *Cons:* it changes the "stop and let the person decide" safety model, so it needs its own
        spec.
    - **D. Run a few more controlled samples first** (settled versus freshly created trees, and a
      smaller interval such as 256) before deciding.
      - *Pros:* cheap, and tells whether the interval matters at all.
      - *Cons:* it delays the decision. The spec says a larger investigation is the author's call.

    **Recommendation: A, plus a small, unscheduled follow-up for D.** A loss is detected and
    recoverable, and the retry path was proven on a real tree in this sample. The data so far
    points at macOS's backlog rather than the interval, so a quick D will show whether B is worth
    designing.
---

# Implementation Log for 2026-10-18-remove-observation-under-load (6 phases)

## Phase 1

### What changed

- **Wait helper** `worktree/lib/src/remove/observation/wait.rs`: compiled on
  every platform. `WaitPolicy` (default stall 30 s, ceiling 5 min) and
  `WaitState<K>` keep one stall clock per awaited key plus one ceiling for the
  whole wait. `acknowledge` treats a marker number as progress only when it
  is above every earlier one (R3: duplicate or older acknowledgments are not
  progress). `check` reports the ceiling when it and a stall expire at the
  same check (R2). `Stalled.waited` is the time since that key's last
  progress (R2). Among several stalled keys, it reports the one silent
  longest. `until_next_deadline` gives the condvar timeout Phase 2 needs to
  wake for deadlines without spinning.
- **Error type**: `CoverageLost::DrainTimedOut` was replaced by
  `DeliveryStalled { waited, watch: Option<Root> }` and
  `DrainCeiling { waited, watch: Option<Root> }` (R6). The texts say
  confirmation could not be obtained and never say events were dropped. No
  match arm in the lib or CLI named the old variant; the CLI renders loss
  through `ActivityKind::CoverageLost` and `Display`.
- **Interim adapter mapping**: macOS `marker_round` and Windows `barrier` keep
  the 5 s `DRAIN_TIMEOUT` and now emit `DeliveryStalled`. macOS passes
  `watch: None`. Windows passes the first awaited root whose marker did not
  arrive.
- **Skill drift**: `.claude/skills/worktree/observation.md` named
  `DrainTimedOut` three times. All three now name the new variants. The 5 s
  wording there stays accurate until Phase 2.

### Departures from the plan

- The plan offered a `Clock` trait or closure. Instead, every `WaitState`
  method takes `now: Instant`. The behavior is the same and nothing needs
  mocking. Tests build instants as `start + n s` and never sleep.
- Added `acknowledge(key, number, now)` and `until_next_deadline(now)`
  beside the planned `progress`/`complete`/`check`. The monotonic marker
  rule now lives in one tested place, and Phase 2's wake-on-deadline wait
  needs a timeout to pass in.

### Requirement → test mapping (all L1, in-process; the boundary is the pure helper, so F1 is enough)

| Requirement | Test |
| --- | --- |
| Default policy is 30 s / 5 min | `wait::tests::default_policy_is_a_thirty_second_stall_and_five_minute_ceiling` |
| Delayed progress passes the old 5 s total | `wait::tests::delayed_progress_outlasts_the_old_five_second_total` |
| Silence stalls at 30 s (29 s continues) | `wait::tests::silence_stalls_at_the_stall_limit` |
| `waited` counts from last progress (R2) | `wait::tests::stall_waited_counts_from_the_last_progress_not_the_start` |
| Repeated progress cannot pass 5 min | `wait::tests::repeated_progress_cannot_pass_the_ceiling` |
| Duplicate/old markers do not reset (R3) | `wait::tests::duplicate_and_older_acknowledgments_do_not_reset_the_stall_clock` |
| Unrelated/cleanup activity and flush returns do not reset | `wait::tests::activity_reported_as_nothing_leaves_both_clocks_running` |
| Busy handle beside a stalled one reports the stalled one (R4) | `wait::tests::a_busy_handle_does_not_hide_a_stalled_one`, `wait::tests::progress_on_one_handle_is_not_progress_on_the_other` |
| Completed handle needs no progress | `wait::tests::a_completed_handle_needs_no_progress` |
| Ceiling wins a tie (R2) | `wait::tests::the_ceiling_wins_a_tie_with_a_stall` |
| Wake timeout is the nearest deadline | `wait::tests::next_deadline_is_the_nearest_of_the_stall_and_ceiling` |
| New loss texts: confirmation failed, not dropped; watch named when known | `observation::tests::unconfirmed_delivery_says_confirmation_failed_not_that_events_dropped` |

No parser or configuration reader changed, so the input robustness matrix
does not apply to this phase.

### Gates

- Targeted tests: `cargo nextest run -p worktree -E 'test(/observation::wait::/)'`
  (12 passed) and `-E 'test(unconfirmed_delivery_says)'` (1 passed).
- Windows adapter compile: `cargo check -p worktree --lib --target x86_64-pc-windows-gnu`
  finished clean. That covers the edited `read_directory_changes.rs`, which
  macOS does not compile.
- `just lint` in `worktree/`: clean.
- `just test` in `worktree/`: 1543 run, 1543 passed, 15 skipped (exit 0).
  There were no failures and no load-sensitive reruns.
- Dependents: only `worktree-cli` depends on `worktree`, and the area's
  `just test` covers it.
- Cross-OS: the only Windows-specific edit is a compile-level variant
  mapping, which the cross-target `cargo check` verified. I did not run
  `just cross-check`; this phase carries no runtime OS risk.

## Phase 2

### What changed

- **Portable wait loop** (`observation/wait.rs`): added `Delivery<K>` (what
  each awaited source has delivered: its highest marker, its in-scope event
  count, a generation counter, and a condvar), a `Source<K>` trait (time,
  reads, and blocking), and `await_marker`, which turns a higher marker or a
  new in-scope event into that key's progress and returns
  `Confirmed | Lost | Interrupted | Stalled | Ceiling`. Recorded loss and the
  interrupt are polled every `POLL_INTERVAL` (50 ms); progress wakes the wait
  at once through the condvar. `WaitPolicy` is now public and part of
  `ObserverOptions::wait`. `WaitState` gained `expect` (a further marker
  round restarts its key's stall clock but never the ceiling) and
  `awaiting`. `mod wait`'s blanket `allow(dead_code)` is narrowed to
  `cfg_attr(not(any(macos, windows)))`, and the "not yet called" comment is
  gone (Phase 1's handoff).
- **macOS** (`fsevents.rs`): `DRAIN_TIMEOUT` and the 1 ms `FlushSync` spin are
  gone. A drain's marker rounds share one `WaitState` under the session
  policy, so the ceiling spans every round. The callback reports marker
  acknowledgments and in-scope events (`EventQueue::push` now says whether
  an event classified) to `Shared::delivery`. Siblings, unreported admin
  files, and a marker's own cleanup are not progress (R3). The quiet-round
  rule is unchanged, except that it now counts in-scope events, not every
  located path. The blocking flush was replaced by `FSEventStreamFlushAsync`,
  re-issued every 1 ms while the round waits (`Flushing` source), so the wait
  can check its deadlines and Ctrl-C and no lock is held while waiting.
  Startup, `moved_to_quarantine`, and `release_admin` use the same barrier.
- **Windows** (`read_directory_changes.rs`): `DRAIN_TIMEOUT` and the single
  deadline are gone. `Shared::delivery` is a `Delivery<Root>` keyed per handle
  (Parent, Admin), and each awaited handle has its own stall clock (R4).
  `DeliveryStalled { watch: Some(root) }` names the silent handle, and the
  ceiling names the first handle still awaited (R6). The startup and
  admin-release checks use the same barrier (R5). An interrupted release keeps
  the handle open.
- **Interrupt plumbing (R1)**: `Observation::start`/`start_at_quarantine`,
  `drain`, `moved_to_quarantine`, and `release_admin` take the
  `&dyn Fn() -> bool` interrupt. `StartObservation` and `native_observation`
  pass it through, and `Backend` methods return `Waited::{Done, Interrupted}`.
  An interrupted wait records no loss and is reported as follows:
  - startup → `ObservationError::Interrupted` → `Refusal::Interrupted` (exit
    1, not exit 4);
  - a drain → `Drain::interrupted`; `confirm_unchanged` refuses before the
    move, while the deletion engine and the post-move drain stop as
    `Stop::Interrupted` (pending record and report);
  - `moved_to_quarantine` → reported by the next drain;
  - `release_admin` → `WaitInterrupted` → `UnregisterError::Interrupted`
    (CleanupFailed; registration kept).
  Linux accepts and ignores the signal. Once coverage is lost, a drain does
  not wait again.
- **Test seam**: `Observation::injected_confirming` runs the adapters' own
  `await_marker` on a virtual clock. `Injector::at(time, Scripted::{Acknowledge,
  Event, Overflow, Interrupt})` schedules deliveries, and a park jumps to
  the next one instead of sleeping. Both macOS and Windows gained a
  test-only `Shared::held` flag that holds delivery, set from the start by
  `FsEvents::start_held` and by `ReadDirectoryChanges::start_with_buffer(..,
  held)`; `Drop` clears it.

### Departures from the plan and spec

- **Flush cadence (found by a slow test).** My first macOS version issued one
  `FlushAsync` per round. `native_drain_delivers_every_change_kind` went from
  0.83 s (measured on a clean `HEAD` worktree) to 6.4 s, and
  `the_barrier_holds_for_a_checkout_on_another_volume` from 0.01 s to 5.2 s:
  each marker waited out the stream's 0.1 s latency. In production that is
  about 0.2 s more per drain, or about 40 s over 200 drains of a 100k-entry
  removal. The fix re-flushes every 1 ms while a round waits, and both tests
  are back under 1 s. Recorded in `.claude/skills/os/macos.md`.
- **Resumed attempts leave the interrupt to the deletion engine.** The full
  suite caught a regression:
  `worktree-cli::remove_recovery::an_interrupt_during_a_quarantine_retry_stops_with_a_report`
  failed (and failed alone too). Its pre-deletion drain now saw Ctrl-C and
  refused, printing "Nothing was removed… stopped before moving", which is
  false for a checkout an earlier attempt moved. The interrupted exit after
  the move must win (R1). `resume.rs`'s two pre-deletion drains
  (`checked_drain`) still check their events but clear `interrupted`, so the
  deletion engine's first drain, which waits on the same sticky interrupt,
  stops at once with the usual report. Nothing is deleted.
- **Known wording gap for Phase 5:** an interrupt during a *resumed* attempt's
  observation startup still refuses as `Refusal::Interrupted`. Its text says
  "before anything was moved", and the CLI renders every retry refusal as
  "stopped before moving". The pending record is kept and a retry continues,
  so this is wording only. It belongs to Phase 5's CLI-messages task.
- `docs/cli/remove.md`: corrected only the two "5 seconds" claims this phase
  made false (limits, per-watch counting on Windows, Ctrl-C). The fuller
  rewrite, and the marker-proves-everything claim, stay in Phase 5.

### Requirement → test mapping

The boundary is the observer contract: the in-process API that the lifecycle,
deletion engine, and unregistration call. The portable rules are proven with
F1 tests on the virtual clock, running the adapters' own `await_marker` on
every OS. Each adapter's real use of the policy is proven against a real
stream or real handles (F2), in L1. No parser or configuration reader changed,
so the input robustness matrix does not apply.

| Requirement | Test(s) |
| --- | --- |
| Delayed progress outlasts the old 5 s total | `observation::tests::a_drain_waits_through_delayed_progress_past_the_old_five_seconds` (events at 8 s and 20 s, marker at 45 s) |
| Silence stalls at 30 s, timed from last progress; loss is sticky and is not waited for again | `observation::tests::silence_stalls_a_drain_at_the_stall_limit` |
| Repeated progress cannot pass 5 min | `observation::tests::repeated_progress_cannot_hold_a_drain_past_the_ceiling` |
| Out-of-scope activity (sibling, unreported admin file) is not progress | `observation::tests::out_of_scope_activity_does_not_hold_off_a_stall` |
| Duplicate or older marker (cleanup) is not progress; generation wakes the park | `wait::tests::delivery_keeps_the_highest_marker_and_counts_events`, `wait::tests::a_report_after_reading_the_generation_ends_the_park_at_once` |
| Ceiling spans every macOS round; a round restarts only its stall clock | `wait::tests::a_repeated_round_restarts_its_stall_clock_but_not_the_ceiling`, `fsevents::tests::the_ceiling_spans_every_marker_round_of_a_busy_drain` (real stream) |
| Interrupt ends a wait at once, records no loss, and keeps prior events | `observation::tests::an_interrupt_ends_a_drain_without_loss`, `fsevents::tests::an_interrupt_ends_a_held_wait_without_loss`, `read_directory_changes::tests::an_interrupt_ends_a_held_wait_without_loss` |
| Loss during a wait ends it with that loss; prior events kept | `observation::tests::a_loss_during_the_wait_ends_it_with_that_loss` |
| macOS stall on a real stream (`watch: None`) | `fsevents::tests::a_held_stream_stalls_the_drain_at_the_session_policy` |
| Windows stall names the watch | `read_directory_changes::tests::a_held_watch_stalls_the_drain_and_names_the_watch` |
| Startup uses the policy and stops for an interrupt | `fsevents::tests::startup_waits_under_the_policy_and_stops_for_an_interrupt`, `read_directory_changes::tests::startup_waits_under_the_policy_and_stops_for_an_interrupt` |
| Admin release uses the policy; an interrupted release keeps the watch | `observation::tests::an_interrupted_admin_release_keeps_the_watch`, both adapters' `an_interrupt_ends_a_held_wait_without_loss` |
| Interrupted move labeling is reported by the next drain | `observation::tests::an_interrupted_move_labeling_is_reported_by_the_next_drain` |
| Before the move: startup or wait interrupt refuses (exit 1), nothing changed | `lifecycle::tests::an_interrupt_while_observation_starts_refuses_as_an_interrupt`, `lifecycle::tests::an_interrupt_during_a_wait_before_the_move_refuses_at_once` |
| After the move: interrupted wait stops with a pending record, nothing deleted | `lifecycle::tests::an_interrupt_during_a_wait_after_the_move_stops_with_a_pending_record` |
| Deletion: interrupted drain stops at once; stalled drain is lost coverage and blocks further deletion | `deletion::tests::an_interrupt_ends_a_drain_wait_and_nothing_more_is_deleted`, `deletion::tests::a_stalled_drain_stops_deletion_as_lost_coverage` |
| Unregistration: interrupted wait keeps the registration; uninterrupted retry unregisters | `quarantine::unregister::tests::an_interrupted_wait_keeps_the_registration` |
| Resumed attempt: Ctrl-C ends with the report, not a "nothing moved" refusal (regression) | existing `worktree-cli::remove_recovery::an_interrupt_during_a_quarantine_retry_stops_with_a_report` (F3, real binary) |
| Existing sticky-loss and first-loss behavior unchanged | existing `observation::tests::coverage_loss_is_typed_and_stays_lost`, `deletion::tests::lost_coverage_and_admin_changes_stop_deletion` |

macOS tests stay in `fsevents-l1`: the new ones live in `fsevents/tests.rs`.
Every new macOS adapter test finishes in under 1.5 s.

### Gates

- `just lint` (worktree area): clean. Clippy `--all-targets -D warnings` for
  `x86_64-pc-windows-gnu` was also clean.
- `just test` (worktree area, macOS): the first run had 1 failure (the
  resume regression above, which also failed alone and was fixed). Final run:
  **1564 passed, 15 skipped**, no load-sensitive reruns.
- `just check-tier-coverage worktree`: 0 stranded.
- Dependents: `worktree-cli` is the only one, and it is included in the area's
  `just test`.
- Cross-OS (behavioral evidence, rigs):
  - Windows (`build-win-native`): `worktree` 854 passed, 1 skipped; the 17
    `read_directory_changes` tests, including the 3 new ones, were run
    explicitly; `worktree-cli` 647 passed, 43 skipped.
  - Linux (`build-linux`): `worktree` 874 passed, 1 skipped.
  - WSL2: not run. It follows the Linux code path, whose only change is the
    ignored interrupt parameter, and the nightly covers it.

## Phase 3

### What changed

- **macOS loss payload (R7).** `CoverageLost::FsEventsDropped { raw, flags }`
  replaces the old `flags & LOST_FLAGS => Overflow` branch in
  `Shared::deliver`. `raw` is the event's whole flag mask, including bits
  without a name. `flags` lists every loss flag present (`KernelDropped`,
  `UserDropped`, `MustScanSubDirs`), in declaration order and with no
  precedence. `MustScanSubDirs` alone is still loss. Generic `Overflow`
  stays on Linux and Windows, and `QueueFull` is unchanged.
- **Deletion cadence.** `DEFAULT_DRAIN_EVERY` is
  `if cfg!(target_os = "macos") { 512 } else { 2048 }`. The comment tying
  2,048 to the queue was replaced by the batch-overshoot rule and the reason
  for macOS pacing, which lowers pressure but cannot rule out a drop. Depth
  drains, the 256-entry batch, and the check after each batch are unchanged.
- **Diagnostics (R8)** in a new `observation/diagnostics.rs`.
  `ObservationDiagnostics` holds `backend`, `startup_ms`,
  `last_round_trip_ms`, `longest_round_trip_ms`, and
  `loss: Option<ObservationLoss { stage, reason: CoverageLost, entries_since_drain }>`.
  It keeps one value per field and no samples.
  - `await_marker` now returns `Outcome::Confirmed { round_trip }`. Each
    adapter keeps `RoundTrips` (last and longest) and exposes them through
    `Backend::round_trips` (Linux: none). `Backend::backend()` names the
    adapter.
  - `Observation` refreshes a shared `DiagnosticsHandle` after every drain,
    move labeling, and admin release. The first loss a drain returns is
    recorded with the current stage. The stage is `pre_move` or `post_move`
    (from `moved`) until the deletion engine or unregistration calls
    `enter_stage`.
  - The deletion engine drains through `drain_after(entries)`, which records
    `entries_since_drain`. `Observation::start` and `start_at_quarantine`
    set `startup_ms`.
- **Startup failures.** A startup confirmation that loses coverage is now
  `ObservationError::NotDelivering(Box<ObservationDiagnostics>)`, with
  `stage: startup`, on both macOS and Windows. It replaces `Setup` with a
  text-only loss. It still refuses as `Refusal::Observation` (exit 4), and
  `ObservationError::diagnostics()` exposes the diagnostics to the CLI
  without a report.
- **Report persistence.** `CoverageLost`, `Root`, and `FsEventsLossFlag` now
  derive serde. `CoverageLost` is tagged by `kind`, and its durations are
  written as `waited_ms`. `OperationReport.observation` and
  `EarlierAttempt.observation` are `Option<ObservationDiagnostics>`, and
  `REPORT_VERSION` is 5. `continue_from` carries the earlier attempt's
  diagnostics. The lifecycle's `Attempt::observed_by(&observation)` (first
  attempt and resumed deletion) copies the snapshot into the report on every
  save and at completion. Resumes that start no observer (registration
  only, branches only) report `None`. The recovery record and the journal
  formats are untouched.
- **Docs and skills.**
  - `docs/cli/remove.md`: the drain interval line (512 on macOS), plus the
    new `observation` report field with an example.
  - `.claude/skills/worktree/observation.md`: loss list, diagnostics,
    interval.
  - `.claude/skills/worktree/lifecycle.md`: `REPORT_VERSION` 5.
  - The fuller CLI and doc rewrite stays in Phase 5.

### Departures and notes

- `CoverageLost` itself is the persisted loss reason. I did not add a
  parallel report enum; the enum's doc now says that changing a variant
  changes the report shape.
- The plan's "absent/null → unknown" is implemented without
  `#[serde(default)]`. A missing `Option` field reads as `None` under serde
  derive, and `deny_unknown_fields` still refuses unknown keys. A grep of the
  new code for `serde(default)` and `unwrap_or_default()` found nothing. The
  field's doc comment states the absent/null rule.
- `startup_ms` was not in R8's list. I added it because Phase 5's slow-start
  warning (over 1 s) needs it.
- **Found, not fixed (pre-existing; for review):** a loss recorded during
  `release_admin`'s barrier (macOS, Windows, and the injected observer) stays in the
  queue, and no later drain returns it. The admin directory is released and
  unregistration proceeds, protected only by `remove_listed_record`'s direct
  index/HEAD reread. The behavior is unchanged by this phase, and that loss
  is also not in the diagnostics.

### Requirement → test mapping

The input robustness matrix applies to the report reader (one new field
through serde's derive):

- control: present round-trips;
- absent → unknown, top level and earlier attempt;
- `null` → unknown, both levels;
- wrong type, whole field (`"fs_events"`) → error;
- wrong type, nested (`backend: 3`) → error;
- unknown nested key → error.

The reader is `RecoveryStore::read_report` (`serde_json::from_slice`), so the
cells are tested in-process on that deserialization. The lifecycle test reads
back through `read_report` itself.

| Requirement | Test(s) |
| --- | --- |
| Each loss flag alone and combined (with an unnamed bit) is loss keeping the raw mask and every name; the first loss is kept; earlier events kept | `fsevents::tests::every_dropped_or_rescan_flag_is_loss_with_its_raw_mask_and_names` (macOS, callback seam) |
| Flags (incl. `MustScanSubDirs` alone) survive into serialized diagnostics and read back | `fsevents::tests::a_loss_payload_survives_into_the_serialized_diagnostics` |
| Generic `Overflow` and `QueueFull` stay distinct; every reason round-trips through the report | `outcome::tests::every_loss_reason_round_trips_through_the_report`; existing `inotify`/`read_directory_changes` overflow tests unchanged |
| macOS default 512 / elsewhere 2,048, observed through deletion | `deletion::tests::the_default_drain_interval_is_512_on_macos_and_2048_elsewhere` |
| Overshoot ≤ one batch minus one entry | `deletion::tests::a_drain_overshoots_its_interval_by_at_most_one_batch_minus_one_entry` |
| Depth-level drains still happen | existing `deletion::tests::drains_follow_depth_levels_and_the_bound_not_entries` |
| Last/longest round trip, no samples, first loss + stage kept, handle outlives the session | `observation::tests::diagnostics_keep_the_last_and_longest_round_trip_and_the_first_loss` (virtual clock) |
| Stage labeling (pre-move, post-move, deletion with entries, unregistration without) | `observation::tests::a_loss_is_labeled_with_the_stage_that_received_it` |
| Real stream reports startup time and round trips | `fsevents::tests::a_started_session_reports_its_startup_and_marker_round_trips` (F2) |
| Startup failure carries diagnostics (stage `startup`, backend, reason) | `fsevents::tests::startup_waits_under_the_policy_and_stops_for_an_interrupt`, `read_directory_changes::tests::startup_waits_under_the_policy_and_stops_for_an_interrupt` |
| Saved report carries diagnostics; a retry carries them as the earlier attempt's | `lifecycle::tests::a_loss_during_deletion_is_reported_with_its_stage_and_carried_by_the_retry`, `lifecycle::tests::lost_coverage_after_the_move_stops_and_a_failed_diagnostic_is_recorded` |
| A v4 report stays readable, diagnostics unknown, earlier attempt preserved | `outcome::tests::a_version_4_report_reads_with_unknown_diagnostics_and_keeps_its_earlier_attempt` (frozen v4 JSON literal) |
| Absent / null / wrong type for the new field | `outcome::tests::the_observation_field_reads_present_absent_null_and_refuses_the_wrong_type` |
| No new required fields in recovery records or journals | the record and journal types are unchanged; the existing `recovery::store` tests pass |

### Gates

- Targeted runs while building: the `fsevents`, `deletion`, `observation`,
  `lifecycle`, and `outcome` tests listed above all passed.
- `just lint` (worktree area): clean. Clippy `--all-targets -D warnings` for
  `x86_64-pc-windows-gnu`: clean.
- `just test` (worktree area, macOS): **1574 passed, 15 skipped**, with no
  failures and no reruns.
- `just check-tier-coverage worktree`: 0 stranded.
- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' |
  xargs python3 scripts/ci/affected_scope.py` gave `packages: [worktree,
  worktree-cli, test-toolkit, darkmatter]` and `change_class: package` (the
  tree still holds Phase 2's uncommitted changes as well).
  - `darkmatter` comes from the skill edits, which its
    `shipped_schema_corpus` tests read: 8 passed.
  - `test-toolkit` is a lint-only `archive-path-guard` row:
    `just archive-path-guard` passed 2.
  - `worktree-cli` is covered by the area's `just test`.
- Cross-OS: `just cross-check worktree --os windows` (build-win-native):
  **862 passed, 1 skipped**. All 17 `read_directory_changes` tests ran,
  including the updated
  `startup_waits_under_the_policy_and_stops_for_an_interrupt`. Linux's only
  change is `Backend::backend()` for inotify. The interval there is unchanged
  (2,048), and the portable diagnostics tests run on Linux CI. I did not run
  a Linux or WSL2 rig check; Phase 6 owns that evidence.

## Phase 4

### What changed

- **Library calculation (R9, R10).** `lifecycle.rs` gains `ResumeConsent
  { raw, inventory, excluded, fingerprint }` beside `consent_state`, and a
  private `explain_deletions(raw, deleted, absent)`. It drops a dirty entry
  only when its porcelain status is exactly ` D`, its path is in the set of
  journal paths with a durable `Deleted` result, and `absent` confirms the path
  is gone. Paths are compared as `Path` (component equality: exact,
  case-sensitive, no prefix, and Git's `/` matches a Windows journal's `\`).
  A deleted directory explains nothing beneath it.
- **Public entry (resume.rs).** `resume_consent(base, &id)` opens the store,
  reads and validates the record, and then calls `resume_consent_in`. That
  function opens the quarantine through the verified-identity handle
  (`open_root`) and reads every attempt's journal with `store.recover`. An
  unreadable or invalid journal is `Refusal::Store` and enables no
  exclusion. It then runs `consent_state` on the quarantine with the
  record's branch, explains deletions by inspecting each path without
  following links through that handle (an inspection error is not absence),
  and binds the fingerprint.
- **Fingerprint binding.** `inventory::bind_excluded(raw_fingerprint,
  excluded)` is a length-prefixed BLAKE3 record of the raw
  `Inventory::fingerprint` (raw dirty entries, their content, and the whole
  index, as before) plus the sorted excluded paths. A retry's `Approval`
  carries the explained `inventory` and this fingerprint.
- **Lock-time recheck.** `recheck` now takes the fresh `(Inventory, String)`
  from a closure: `consent_state` of the original on a first attempt, and
  `resume_consent_in` in `Resumed::deletion`, which runs under the operation
  lock that `resume_with` takes before inspecting. The usage check,
  tip check, fresh `DeletionInventory` capture, and index binding run
  as before.
- **CLI wiring (mod.rs).** On a `Quarantined` retry, `run` calls
  `resume_consent` and replaces `facts.inventory`, `facts.reported`, and a
  new `facts.excluded` count. The report and policy read `facts.inventory`,
  so they use the explained view with no change of their own.
  `retry_markup` adds "N tracked file(s) this removal already deleted is/are
  not asked about again." A failure maps through `recovery::refusal_error`.
  First attempts and other retry resolutions are untouched; journal counts
  and reports are unchanged.
- **Docs and skill.** `docs/cli/remove.md`'s retry paragraph now states the
  exemption and its limits. `.claude/skills/worktree/lifecycle.md` corrects
  the stale instruction to approve a retry from `consent_state` of the
  quarantine, which is now refused, and adds the raw-versus-explained
  bullet. The library's `Approval` and `ResumeRequest` field docs were
  updated with the behavior.

### Departures from the plan

- The plan names `cli/src/commands/remove/{mod,policy,report}.rs`. Only
  `mod.rs` changed. `policy.rs` and `report.rs` already take their inventory
  from `Facts`, so they share the library calculation without edits.
- Existing resume tests in `lifecycle/tests.rs` approved a retry with
  `consent_state(quarantine)`. They now use a `resume_approval(&id)` fixture
  helper, since a bound fingerprint is required. Two tests that refuse
  before the recheck (a replaced quarantine and a recreated original) keep
  `approval_at`.
- The CLI's `Facts::local` still runs one `git status` on the quarantine
  before `resume_consent` runs its own. That is one extra Git call per
  retry, which I left in place to keep the change surgical.

### Requirement → test mapping

Library tests are L1 in-process against real repositories
(`lib/src/remove/lifecycle/tests/resume_consent.rs`, F2: real Git, real
filesystem, real journal; injected observer). The CLI tests are L1 through
the real `wt` binary (`cli/tests/remove_recovery.rs`, Unix-only like the
rest of that file).

| Requirement | Test |
| --- | --- |
| Stop after confirmed deletion, retry, no extra consent for the ` D` entries (spec's original case) | `a_retry_asks_nothing_for_tracked_files_its_own_deletions_removed` (raw shows ` D` for 3 paths, explained has none, `needs_consent` false; `resume` with `discard_files: false` completes; journal entry kept as attempt 1 `Deleted`) |
| Records spread over several attempts | `confirmed_deletions_from_several_attempts_are_all_excluded` (attempt 1 deletes `src/*`, attempt 2 deletes `README.md`, attempt 3 excludes all three and completes without consent; asserts attempts 1 and 2 in the report) |
| `ExternallyAbsent` and `Uncertain` keep consent; unreadable journal refuses | `only_a_confirmed_result_explains_a_missing_file_and_an_unreadable_journal_refuses` (control, `externally_absent` rewrite, journal cut before results, garbage journal leads to `Refusal::Store(Journal)` from both `resume_consent` and `resume`, exit class Refused; restored control) |
| Staged deletion, recreated file / directory / symlink, staged add with absent file keep consent | `staged_and_recreated_paths_keep_consent` (`D `, ` M`, ` D` with an empty directory at the path, which shows the no-follow inspection matters, ` T` (Unix), `AD`) |
| A changed path or index between the report and the recheck refuses | `a_change_between_the_report_and_the_retry_refuses` (recreated file, then a staged change, each `Refusal::Changed` with nothing deleted; the unchanged state then completes on the same approval) |
| Only exact ` D` is explained | `only_an_unstaged_deletion_is_explained` (13 porcelain shapes) |
| Exact per-path matching: no result, present, parent directory, other case, native separator | `a_path_is_explained_only_by_its_own_confirmed_result` (and only paths with a result are inspected) |
| CLI: noninteractive retry without `--force-worktree` completes | `a_retry_needs_no_consent_for_files_the_removal_already_deleted` (a real first `wt remove` deletes the tracked `deep/sub/tracked.txt`, then stops on an unwritable `locked/` one level up; the retry exits 0, prints the count line, does not list the file, and closes the record) |
| CLI: counterpart with new work refuses | `a_retry_with_new_work_beside_files_the_removal_already_deleted_refuses` (exit 3, lists `new-work.txt` only, keeps everything) |

Regression proof: with the four changed source files restored to `HEAD`
(test files kept), both CLI tests fail. The first fails on exit 3 instead of
0. Reapplying the saved patch restored them.

The CLI stop needs an unwritable directory, so both CLI tests probe and skip
when the user can write into a read-only directory (root). This follows
`a_repair_that_is_not_verified_refuses_even_with_every_force_flag`.

### Input robustness

The journal and porcelain loaders are unchanged, so no new loader matrix
applies. The new code has no `filter_map(.. as_str())`, no `.ok()` on a
journal or porcelain parse, and no `unwrap_or_default()`. The one
`matches!(present(relative), Ok(false))` treats an inspection error as
present (consent kept), as the spec requires.

### Gates

- Targeted while building: `remove::lifecycle::tests` (58 tests, including
  the 7 new ones) and `--test remove_recovery` (8) passed.
- `just lint` (worktree area): clean.
- `just test` (worktree area, macOS): **1583 passed, 15 skipped**, with no
  failures and no reruns.
- `just check-tier-coverage worktree`: 0 stranded.
- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' |
  xargs python3 scripts/ci/affected_scope.py` gave `packages: [worktree,
  worktree-cli, test-toolkit, darkmatter]` and `change_class: package`.
  - `darkmatter` (it reads the skill file):
    `cargo nextest run -E 'test(/shipped_schema_corpus/)'` passed 9.
  - `test-toolkit`, lint-only `archive-path-guard`: `just archive-path-guard`
    in `tools/test-toolkit` passed 2 (full-tree mode). A first attempt that
    fed the raw scope JSON as `BISCUIT_ARCHIVE_GUARD_PLAN` panicked on the
    plan format; that was my invocation, not a guard failure.
- Dependents: only `worktree-cli` depends on `worktree`, and the area's
  `just test` covers it.
- Cross-OS: path matching is Windows-sensitive, so I ran
  `just cross-check worktree --os windows`, which passed **869, 1 skipped**,
  all 7 `resume_consent` tests included (the multi-attempt test proves the
  `\` versus `/` match). `just cross-check worktree-cli --os windows`
  passed **647, 43 skipped**; the new CLI tests are Unix-only. No Linux or
  WSL2 rig run; Phase 6 owns that evidence.
- No skipped or pre-existing failures.

## Phase 5

### What changed

- **Slow-startup hook (library).** `lifecycle::SLOW_STARTUP` (1 s) and
  `Environment::slow_startup: &dyn Fn(Duration)`. `report_slow_startup` calls
  it at most once per attempt, right after observation starts: in `run_with`
  before the move, and in `Resumed::deletion` before further deletion. It
  fires only when `startup_ms` is over 1 s, and the removal goes on. The CLI
  prints `recovery::slow_startup_markup` to stderr. `Observation::started_after`
  is now `pub(crate)`, so tests can fake a startup time.
- **Loss rendering (CLI, `recovery.rs`).** Keyed on the `CoverageLost` and
  `ObservationStage` variants, never on `to_string()`.
  - A stop after the move adds `Watching stopped {stage}: {reason}.` with
    the FSEvents flag names and hex mask, entries since the previous drain,
    and the watched place.
  - Windows names the place: the checkout's path, the folder holding it, or
    Git's record.
  - A stall or ceiling adds "Nothing is known to have been lost" and the
    longest confirmed round trip. On macOS only, it also adds the fseventsd
    line: "may be delayed; wt does not restart it", pointing at the
    troubleshooting docs.
  - `NotDelivering` refusals (exit 4) show the same reason. A stall or
    ceiling gets a "run it again" remedy.
  - Locations, the retry line, and the `Full report:` line are unchanged.
  - The output is Prose markup rendered by biscuit-terminal's `Prose`, like
    the rest of this module. No list component was needed.
- **Retry wording (carried from Phases 2 and 4).**
  `refusal_error`/`refusal_markup` take `resumed`. A refusal on a resumed
  attempt (`resume_consent` failure, or a lifecycle refusal on a retry) says
  "stopped before deleting more of worktree X". `Refusal::Interrupted` reads
  "interrupted before it changed anything", which is true for both attempt
  kinds.
- **Docs and skills** (a subagent did the docs pass; I reviewed it).
  - `docs/cli/remove.md` gained these sections: waiting limits (table and
    Mermaid), the startup warning, the loss reasons, and fseventsd
    troubleshooting (`sudo killall fseventsd`, manual and system-wide).
  - It also has one "What a retry asks again" resume-consent section, which
    replaces the Phase 4 paragraph, and the report fields.
  - The claim that a macOS marker proves every earlier change arrived is
    corrected.
  - Skills: `observation.md` and `lifecycle.md`. `os/macos.md` has a new
    stuck-fseventsd section, and `os/SKILL.md` a symptom row for it.
  - README is unchanged: its overview is still accurate.
- **Comment drift found and fixed** (comment lines only):
  - `deletion.rs` `//!` said only a lost watch or a full queue stops
    deletion. Any `CoverageLost` does.
  - `observation/mod.rs` `//!`'s list of loss causes was missing the
    FSEvents flags and unconfirmed delivery.
  - `resume`'s doc did not mention the consent recheck under the lock.
  - I found no drift in `fsevents.rs` or `read_directory_changes.rs`.

### Departures and notes

- The plan put the slow-startup threshold under "CLI messages". The
  threshold lives in the library (`SLOW_STARTUP`), because the lifecycle runs
  on to the move without returning to the CLI. The CLI only renders.
- There is no admin path in the report, so Windows names Git's record in
  words. A startup refusal has no report yet, so it names "the checkout" or
  "the folder holding the checkout" without a path.
- **Tier trap:** the new library test module was first called
  `slow_startup`, and a segment starting with `slow_` takes a test out of L1.
  The Windows rig's L1 listing showed 869 tests, the same count as Phase 4.
  I renamed the module to `startup_warning`, and the rig then ran 872.
- **Load-sensitive, pre-existing:**
  `fsevents::tests::the_ceiling_spans_every_marker_round_of_a_busy_drain`
  failed once in `just test` at a load average of about 20 (other sessions
  on the host): "expected the ceiling, got None after 1039 events". It passed
  3/3 alone, and the next full `just test` was green. It is not touched by
  this phase.
- Not done (pre-existing, author's call): a loss recorded during the
  `release_admin` barrier stays queued and is not in the diagnostics.

### Requirement → test mapping

The boundary is the CLI's markup builders: library values are rendered
through biscuit-terminal `Prose`, F1, L1 (`cli/tests/remove_recovery_report.rs`).
The hook is tested at the lifecycle boundary, with real Git, a real
filesystem, and the injected observer (F2, L1,
`lib/src/remove/lifecycle/tests/startup_warning.rs`).

| Requirement | Test |
| --- | --- |
| A slow startup warns once, before the move, and does not refuse | `startup_warning::a_slow_startup_is_reported_once_before_the_move_and_the_removal_completes` (1500 ms; checkout untouched and not moved at call time; removal completes) |
| The 1 s boundary, a fast startup, and an untimed one do not warn | `startup_warning::a_startup_of_at_most_one_second_is_not_reported` (1000 ms, 5 ms, none) |
| A resumed attempt warns once, before further deletion | `startup_warning::a_resumed_attempts_slow_startup_is_reported_once_before_its_deletion` |
| Warning text and duration format | `a_slow_startup_warns_with_its_duration` (1.6 s, 12.3 s; snapshot) |
| macOS flags, mask, and stage are shown; no fseventsd line for a drop; retry and full-report lines kept; a stop with zero deletions is exit 1 | `a_stop_names_the_fsevents_flags_that_dropped_changes` |
| macOS stall: fseventsd line, "nothing known lost", round trip | `a_macos_stall_after_the_move_points_at_fseventsd` |
| Windows ceiling names the watched folder; no fseventsd line | `a_windows_ceiling_names_the_watched_folder` |
| No loss (absent diagnostics, or diagnostics with no loss) adds no line | `a_stop_without_a_loss_adds_no_watching_line` |
| Startup failure: reason, remedy, exit 4 (stall on FsEvents; Overflow keeps the old remedy) | `a_watch_that_never_delivered_refuses_with_its_reason` |
| A resumed refusal says "before deleting more" | `a_resumed_refusal_says_it_stopped_before_deleting_more` |
| Interrupt wording through the real CLI | `remove_recovery::an_interrupt_before_the_move_removes_nothing` (assertion updated) |

No parser, schema, or loader changed, so the input robustness matrix does
not apply.

### Gates

- `just lint` (worktree area): clean.
- `just test` (worktree area, macOS): first run had 1 failure, the
  load-sensitive fsevents test above. It passed 3/3 alone. The rerun gave
  **1593 passed, 15 skipped**, with the 3 `startup_warning` tests included.
- `just check-tier-coverage worktree`: 0 stranded.
- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' |
  xargs python3 scripts/ci/affected_scope.py` gave `packages: [worktree,
  worktree-cli, test-toolkit, darkmatter]` and `change_class: package`. The
  tree still holds Phase 4's uncommitted changes.
  - `darkmatter` (reads the skill files):
    `cargo nextest run -E 'test(/shipped_schema_corpus/)'` passed 9.
  - `test-toolkit`: `just archive-path-guard` passed 2.
  - `worktree-cli` is covered by the area's `just test`; it is the only
    dependent of `worktree`.
- Docs: `md validate refs` on the edited Markdown files found 0 issues.
  None of them has `kind` frontmatter.
- Cross-OS (Windows rig): `just cross-check worktree --os windows` passed
  **872, 1 skipped**, and `just cross-check worktree-cli --os windows` passed
  **654, 43 skipped**. No Linux or WSL2 run; Phase 6 owns that evidence.

## Phase 6

### What changed

- **Linux dead-code warning (fix).** The Linux cross-check build warned
  `method record is never used` for `RoundTrips::record`
  (`lib/src/remove/observation/diagnostics.rs`). Only the macOS and Windows
  adapters and the test-only injected observer confirm markers; the Linux
  adapter reads until empty. CI lints on Linux only, with `-D warnings`, so
  this would have failed the `lint` cell. The method is now
  `#[cfg(any(target_os = "macos", windows, test))]`. The rerun on
  `build-linux` built with no warning. The macOS lint cannot see it, so it
  slipped through Phases 3–5.
- **`os` skill.** `.claude/skills/os/linux.md`: on 2026-10-18,
  `--build-jobs 2` was not enough to keep `build-linux` from killing the
  `worktree-cli` link (signal 9); `--build-jobs 1` worked.
- No other code changed. No new CI environment, cell, fixture, or
  performance gate.

### macOS 100k-entry sample

Release `wt` (`cargo build --release -p worktree-cli`, built before the run,
nothing building during it). The generated checkout was a linked worktree,
`big`: 1,000 directories × 100 files plus a seed file, committed. That is
**101,002 entries**. Run from the base checkout with stdin closed.
fseventsd had been up 12 h, at about 2 % CPU before sample 1 and 0 % before
sample 2. Other sessions loaded the host; the load averages are given below.
The incident rates were about 6,600 and 7,000 deletions per second.

| Run | Conditions | Result | Elapsed | Rate | Loss / diagnostics |
| --- | --- | --- | --- | --- | --- |
| 1a | started within about 1 min of creating the 101k files; load average about 17 | paused, exit 3: two short-lived `git` processes were in the checkout (gone a second later); nothing moved | 0.9 s | — | — |
| 1b | rerun seconds later, same tree | **stopped, exit 1, coverage lost during deletion** after 18,600 entries; 82,402 left in the hidden directory | 8.1 s total; deletion 6.46 s | about 2,900/s | `observation.loss`: stage `deletion`, `fs_events_dropped`, raw `0x3` = `user_dropped` + `must_scan_sub_dirs`, `entries_since_drain` 600; `startup_ms` 17, `longest_round_trip_ms` 1012 |
| 1c | `wt remove big` retry, no `--force-worktree` | **completed**, exit 0: "18600 tracked files this removal already deleted are not asked about again", then the remaining 82,402 deleted with no loss | 12.5 s total; deletion 10.06 s | about 8,200/s | `loss: null`, `startup_ms` 14 |
| 2 | fresh tree (branch fast-forwarded into `main`); waited 90 s after creating it; load average about 4 (1 min), 10 (5 min); fseventsd 0 % CPU | **completed**, exit 0, branch deleted | 24.7 s total; deletion 23.2 s | about 4,400/s | `loss: null`, `startup_ms` 13, `longest_round_trip_ms` 1012 |

What this shows:

- The new diagnostics work on a real overrun. The CLI printed `Watching
  stopped during deletion, 600 entries after the previous check: macOS could
  not report every change (UserDropped, MustScanSubDirs; flags 0x3).`, and
  `report.json` `observation.loss` held the same data. Before this fix the
  report could not name the flags.
- Phase 4's resume consent worked on the real tree: the retry needed no
  `--force-worktree` for the 18,600 ` D` entries.
- **The 512 interval is not proven to prevent the overflow.** Run 1b lost
  coverage while deleting at about 2,900/s, well below both incident rates.
  Run 1c deleted faster (about 8,200/s) and run 2 at about 4,400/s, and
  neither lost coverage. So deletion rate alone does not explain the loss.
  A likely confound: run 1b started within a minute of creating the 101k
  files and committing them, so fseventsd was probably still working through
  that backlog. Run 2's 90 s settle removed it. These are two samples on one
  host, not a measurement.
- `longest_round_trip_ms` was 1012 in every run. That is the same value
  each time, and close to 1 s; it may be a bound in the marker round rather
  than a real delivery time. I did not investigate it.
- Per the plan, I am not claiming success for the cadence. The interval or
  pacing question is handed to the author (`human_review`).
- Clean-up: both sample directories under `/tmp/wt-sample.*` were removed.

### Cross-OS evidence

All runs used `just cross-check <pkg> --os <os>` on the tree with the
`diagnostics.rs` fix, except the first Linux `worktree` run, which found the
warning.

| Package | macOS (host) | Linux (`build-linux`) | Native Windows (`build-win-native`) | WSL2 (`build-win`) |
| --- | --- | --- | --- | --- |
| `worktree` + `worktree-cli` (area `just test`) | 1593 passed, 15 skipped | — | — | — |
| `worktree` | (in area run) | 892 passed, 1 skipped (warning gone on rerun) | 872 passed, 1 skipped | 892 passed, 1 skipped |
| `worktree-cli` | (in area run) | 693 passed, 55 skipped (at `--build-jobs 1`) | 654 passed, 43 skipped | 693 passed, 55 skipped |

- Linux `worktree-cli` failed to build three times (default jobs, then
  `--build-jobs 2`) with `ld terminated with signal 9`. That is the host
  killing the linker for memory, not a test failure. The run passed at
  `--build-jobs 1`.
- The Windows and WSL2 results come from the rigs. CI's nightly still owns
  their post-merge proof, under the fix-forward rule.

### Departures from the spec

- **Two samples instead of one.** The spec asks for one quick sample and
  does not require repeated trials. The first sample lost coverage, and its
  retry did not, at a higher rate. One more run with a settle period was the
  cheapest way to give the author a cleaner data point. It does not change
  the conclusion that 512 is unproven. No docs claim otherwise:
  `docs/cli/remove.md` and the skills describe 512 as the default interval,
  not as a guarantee.
- No docs needed correcting in this phase.

### Requirement → evidence mapping

| Requirement | Evidence |
| --- | --- |
| L1 evidence on Linux, native Windows, WSL2 for both packages | cross-check table above |
| macOS sample: completion, loss flags, elapsed time | sample table above |
| If 512 still loses coverage: record, don't claim success, hand off | run 1b; `human_review: true` |
| Linux build has no dead-code warning (CI lint is Linux-only, `-D warnings`) | `build-linux` rerun: no `warning:` in the `worktree` build |
| Sticky loss: no force flag bypasses it (done item) | force flags affect consent only and never reach `DeletionOptions` (`drain_every`, `cumulative_kinds`); existing `deletion::tests::lost_coverage_and_admin_changes_stop_deletion` and `observation::tests::coverage_loss_is_typed_and_stays_lost` |

The only code change is a `#[cfg]` on a method that already has callers and
tests (`injected.rs` exercises it through the round-trip tests), so it needs
no new test. The Linux build without the warning is the evidence. No parser,
schema, or loader changed, so the input robustness matrix does not apply.

### Gates

- `just lint` (worktree area, macOS): clean, 0 warnings.
- `just test` (worktree area, macOS): **1593 passed, 15 skipped**, first
  run, no failures. The load-sensitive
  `fsevents::tests::the_ceiling_spans_every_marker_round_of_a_busy_drain`
  passed.
- `just check-tier-coverage worktree`: 0 stranded.
- Scope (`git status --porcelain --untracked-files=all | awk '{print $NF}' |
  xargs python3 scripts/ci/affected_scope.py`): `packages: [worktree,
  test-toolkit, darkmatter]`, `change_class: package`. darkmatter and
  test-toolkit read the skill files; their scoped checks are below.
  - `darkmatter`: `cargo nextest run -E 'test(/shipped_schema_corpus/)'`
    passed 9.
  - `test-toolkit`: `just archive-path-guard` passed 2.
  - `worktree-cli`, the only dependent of `worktree`, is in the area run.

### Still open, for the author

- The drain interval or pacing (see the sample).
- Carried from Phase 5: a loss recorded during the `release_admin` barrier
  stays queued and is not in the diagnostics.
- Implementation complete, ready for review. The fix stays in `fixes/`;
  moving it to `_completed` is the author's step.

## Review 1 author decisions (2026-10-19)

Ken resolved review 1's open items one by one (`review-1.md`, "Author
decisions").

### Decisions

- **512 interval: accepted as a mitigation.** It lowers pressure but does
  not prevent loss. The success criterion narrows to "loss is detected,
  named, and recoverable by a plain retry" (runs 1b/1c). The controlled
  comparison is scheduled as `2026-10-19-macos-drain-interval-comparison`. The spec is
  left as written; this is the recorded departure.
- **O1, watch-release loss: fixed here, diagnostics only.** The spec lists
  `unregistration` as a reportable loss stage, and a loss in the
  `release_admin` barrier never reached the diagnostics.
- **O2, 1,012 ms round trip: investigated now** (below).

### What changed

- `Backend::queued_loss()` (default `None`; macOS, Windows, and the injected
  observer return `EventQueue::loss()`, the queue's loss left in place).
  `Observation::release_admin` records it, so it lands at stage
  `unregistration`. The release still completes: whether to unregister is
  still decided by `remove_listed_record`'s direct index/HEAD reread, as
  before. Linux has no release barrier and keeps the default.
- `docs/cli/remove.md`: the release-loss behavior and the macOS round-trip
  quantization. `.claude/skills/worktree/observation.md`: the same facts.

### O2 probe

A temporary probe (removed, not in the diff) printed each confirmed macOS
marker round trip and made stream latency overridable. Same 101,002-entry
tree shape as Phase 6, release `wt`, 512 interval:

| Sample | Load (1 min) | Result | Rounds | ~11 ms | 1,002–1,016 ms | 200–900 ms |
| --- | --- | --- | --- | --- | --- | --- |
| fresh, 0 s settle | about 19 | completed, 25.3 s | 352 | 336 | 16 | 0 |
| settled 90 s | about 4 | completed, 21.2 s | 352 | 340 | 12 | 0 |
| fresh, latency 0.5 s | about 4 | **lost** after 72,600 deletions (`UserDropped` + `MustScanSubDirs`, `entries_since_drain` 600) | 248 | 237 | 11 | 0 |

- Round trips are bimodal: about 11 ms or about 1 s, nothing between,
  whatever the settle time or stream latency. `wt`'s wait path has no 1 s
  bound (latency 0.1 s, a flush every 1 ms, `POLL_INTERVAL` 50 ms), so the
  second comes from fseventsd: a marker it has not processed when flushed
  arrives about a second later. The reported 1,012 ms is a real delay, but
  it marks a missed fast path, not backlog depth, so it cannot drive a
  pacing rule.
- The third run is a second fresh-tree loss. Its changed latency makes it
  only a loose data point for the interval question; it is recorded in the
  comparison's spec.

### Requirement → test mapping

| Requirement | Test |
| --- | --- |
| A loss during the admin release reaches the diagnostics at `unregistration`; the release still completes | `observation::tests::a_loss_during_the_admin_release_is_recorded_in_the_diagnostics` (fails with the old `record(None)`) |

### Gates

- `just test` (worktree area, macOS): **1594 passed, 15 skipped**.
- `just lint` (worktree area, macOS): clean.
- `just cross-check worktree --os windows`: **873 passed, 1 skipped**
  (compiles the `read_directory_changes` change). Not published as evidence
  because the tree is uncommitted.
- Not run: Linux or WSL2. Linux takes the trait default and `loss()` is
  `cfg`'d to macOS, Windows, and test, so no new dead code there; CI's Linux
  lint will confirm.
