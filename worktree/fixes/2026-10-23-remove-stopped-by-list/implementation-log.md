# Implementation log

## 2026-10-23 — claude/opus

Implemented all three requirements in one pass.

- `remove::lifecycle::PendingRemovals` (in `resolve.rs`, beside
  `same_operation`) is read once by `parse_worktree_state_in`;
  `entry_dirtiness` skips its entries, and `WorktreeStatus::removal` carries
  the requested name to the table (`⊘`, legend `being removed`, closing note).
- `--ff` refuses a holder under removal as `FfRefusal::RemovingHolder`, and
  only when a move is needed, like `UnavailableHolder`.
- `recovery.rs`: `first_step` picks the step before the retry; `git_change`
  names a Git command from the first admin activity's file name; identical
  process-check notes merge; `InventoryRefusal::Changed` gets its own reason
  and remedy.

### Departures and gaps

- **`--ff` refusal has no test.** Building a pending removal of the worktree
  that holds the default branch needs a fixture the lifecycle tests don't
  have; the refusal shares `UnavailableHolder`'s path through
  `fast_forward_default`. Worth adding at review.
- The "something new at the original path" step comes before the Git-command
  step when both apply, since the retry refuses until the original path is
  cleared.
- L2 (`just test-l2`) was not run; no rendering path that L2 covers changed
  other than the table marker, which `cli/tests/list_table.rs` covers at L1.

### Verification

- `just test` in `worktree/`: 1599 passed, 15 skipped.
- **Flaky test fixed:** `the_ceiling_spans_every_marker_round_of_a_busy_drain`
  failed in 2 of 4 full-suite runs. It relied on scheduling: a free-running
  feeder thread had to deliver an event inside every marker round, and the
  real FSEvents stream delivered markers too, so a round could be quiet (or
  the stream could report a drop under suite-wide FSEvents load) and the
  drain ended without reaching the ceiling. The panic text was not captured,
  so which of the two happened is not known. The test now holds the real
  stream and the feeder answers each marker itself, a busy event first, so
  no round can be quiet under any scheduling. It passed 10 of 10 runs with
  every CPU saturated and 14 consecutive full-suite runs.
- `list_remote_head::a_check_still_running_at_the_deadline_is_still_checking_and_the_next_run_shows_it`
  failed once (10.5 s, normally 1 s) in a full-suite run; see below.
- `just lint` in `worktree/`: clean.
- `a_listing_runs_no_git_status_in_a_checkout_under_removal` fails with the
  skip disabled (checked by hand).

### Open: `list_remote_head` failure, not diagnosed

`a_check_still_running_at_the_deadline_is_still_checking_and_the_next_run_shows_it`
failed once in about 48 full-suite runs. That run used the unchanged
FSEvents test while checking that test's cause. It took 10.5 s against a
normal 1 s, and its panic text was not kept. It did not recur in 25 more full
runs or 15 runs alone with every CPU saturated. The 10.5 s matches the
worker's `REMOTE_HEAD_REFRESH_DEADLINE` (10 s) ending the held `ls-remote`
check. The final assertion would then see a failed check instead of `in-sync`.
This is a hypothesis, not a finding. This change does not touch the worker
or the remote-head store. The test is still flaky: it is a failing test
until it is diagnosed.
