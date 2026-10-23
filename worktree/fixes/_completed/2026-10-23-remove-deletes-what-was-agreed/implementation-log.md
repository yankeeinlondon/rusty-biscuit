# Implementation log

## 2026-10-23 — claude/opus

- **Deletion engine** (`observation/deletion.rs`): activity, uncertain
  events, and lost coverage become `Deletion::warnings`. The first loss
  clears `watching`, so no later drain waits. A changed approved entry, a
  non-empty folder, a changed folder on the way down, and leftovers at the
  quarantine root are deleted by the new `sweep` (no link following, no
  device crossing, a mount point is an I/O stop). `Stop` keeps only
  `Replaced`, `Interrupted`, `Io`, and `Journal`; `Activity`, `Uncertain`,
  and `CoverageLost` are gone.
- **Unregister**: returns `Unregistered { copy_warning, activity }`.
  `remove_listed_record` gained `Changes::{Refuse, Allow}`; the
  missing-directory path keeps `Refuse`. Something at the quarantine path
  still refuses.
- **Lifecycle and resume**: `recheck` and `confirm_unchanged` return
  warnings; `unexplained_move_events` returns activity. Resume's "something
  at the original path" in the registration-only path is a warning.
- **Inventory capture**: an entry that vanishes during the walk is skipped,
  and a folder replaced during it is listed but not walked.
- **Report**: `push_activity` records `CoverageLost` once.
- **CLI**: `recovery::agreed_changes_markup` prints the warning after a
  finished removal (exit 0).

### Tests

The tests that asserted a stop for these cases now assert completion with a
warning. New ones: `new_folders_and_links_inside_are_deleted_and_reported`,
`lost_coverage_before_the_move_is_reported_and_the_removal_completes`,
`a_tip_moved_after_approval_is_reported_and_the_moved_branch_kept`, and
`a_finished_removal_names_what_changed_after_you_agreed` (snapshot). The
pending-removal ambiguity test now makes its pending removal with an
interrupt, since a write no longer leaves one.

### Departures

- The spec's "what still stops" list was incomplete. The usage check, watch
  startup, special files, and the handoff's second run still refuse after
  the questions; the spec now names them as separate decisions.
