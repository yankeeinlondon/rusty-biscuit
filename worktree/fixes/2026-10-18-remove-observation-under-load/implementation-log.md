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
packages:
  - worktree
human_review: false
message_to_agent: |-
  Phase 1 left two things for Phase 2 to finish:
  - `observation/mod.rs` declares `mod wait;` under an unconditional `#[allow(dead_code)]`, because no adapter calls the helper yet. Once fsevents.rs and read_directory_changes.rs use it, narrow it to `#[cfg_attr(not(any(target_os = "macos", windows)), allow(dead_code))]` (Linux compiles it only for its tests) and drop the "Not yet called by the adapters." comment.
  - Both adapters still use `DRAIN_TIMEOUT` (5 s), now mapped to `CoverageLost::DeliveryStalled { waited: DRAIN_TIMEOUT, watch }`. macOS passes `watch: None`. Windows passes the first awaited root whose marker was not delivered. The module docs that name `DRAIN_TIMEOUT` are still accurate and must change when the constant goes.
  Helper API (`observation/wait.rs`): the caller passes `now: Instant` into every method; there is no Clock trait. `WaitState::new(policy, now, keys)`, `progress(key, now)`, `acknowledge(key, number, now) -> bool` (only a number above the highest seen counts as progress), `complete(key)`, `check(now) -> Check::{Continue, Stalled { waited, key }, Ceiling { waited }}`, and `until_next_deadline(now)`, which gives the timeout for a condvar `wait_timeout`. For a Windows `WaitState<Root>`, map `Stalled.key` to `watch: Some(key)`. A ceiling carries no key; pass `None`, or the root still awaited if you want to name it.
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
