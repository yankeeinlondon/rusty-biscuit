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
human_review: false
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
message_to_agent: |-
  Phase 3 landed the loss payload, the macOS drain interval (512), observation diagnostics, and report v5. Facts for Phase 4 onward:
  - Diagnostics API: `Observation::diagnostics()` / `diagnostics_handle()` -> `ObservationDiagnostics { backend, startup_ms, last_round_trip_ms, longest_round_trip_ms, loss: Option<ObservationLoss { stage, reason: CoverageLost, entries_since_drain }> }`. The lifecycle copies it into `OperationReport.observation` (and `EarlierAttempt.observation` on resume) via `Attempt::observed_by`.
  - Startup confirmation loss is now `ObservationError::NotDelivering(Box<ObservationDiagnostics>)` (exit 4 via `Refusal::Observation`); `ObservationError::diagnostics()` gives the CLI the details without a report. The CLI still renders only its `Display` text; Phase 5 should render the diagnostics (stage, reason, round trips) through biscuit-terminal components.
  - macOS loss is `CoverageLost::FsEventsDropped { raw, flags }`; its Display is "FSEvents could not report every change (KernelDropped, ...; flags 0x..)". Phase 5's CLI messages should key off the variant, not the text.
  - Slow-startup warning (Phase 5): `startup_ms` is available on `observation.diagnostics()` right after `(env.observe)(...)` in `run_with` and `resume.rs::deletion`, but the lifecycle runs on to the move without returning to the CLI, so warning "before moving anything" needs a new hook (for example an `Environment` callback); none exists yet.
  - Found, not fixed: a loss recorded during `release_admin`'s barrier stays queued and no later drain returns it (all marker-based adapters), so unregistration proceeds on its direct index/HEAD reread alone and the loss is not in the diagnostics. Pre-existing; decide in review or Phase 5 whether `unregister_quarantine` should drain (or check the queue) after `release_admin`.
  - Phase 2's wording gap still stands for Phase 5: an interrupt during a resumed attempt's observation startup refuses as `Refusal::Interrupted` ("before anything was moved").
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
