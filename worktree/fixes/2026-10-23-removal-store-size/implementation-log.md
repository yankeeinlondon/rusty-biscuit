---
spec: /Volumes/coding/wt/rusty-biscuit/feat-merman/worktree/fixes/2026-10-23-removal-store-size/spec.md
plan: worktree/fixes/2026-10-23-removal-store-size/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - worktree/lib/src/remove/recovery/store/retention.rs
    - worktree/lib/src/remove/recovery/store.rs
    - worktree/lib/src/remove/recovery/store/tests.rs
    - worktree/lib/src/remove/recovery/mod.rs
    - worktree/lib/Cargo.toml
    - Cargo.lock
    - worktree/cli/src/commands/remove/mod.rs
    - worktree/cli/tests/remove_recovery.rs
docs_updated_during_phase_1:
    - worktree/docs/cli/remove.md
    - worktree/README.md
    - docs/dependencies.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/worktree/remove.md
    - .claude/skills/worktree/lifecycle.md
packages:
    - worktree
    - worktree-cli
decisions_for_review:
    - |-
        An expired finished removal is renamed to a hidden `.<operation-id>.expired`
        folder before it is deleted, and a leftover `.expired` folder is deleted on
        the next `wt remove` whatever its age. The plan said to recognize only
        operation folders and `.<id>.tmp` staging folders and delete in place.
        Deleting in place can remove `record.json` first if the deletion is cut
        short; the folder is then still named as an operation but has no readable
        record, so it reads as an unreadable pending removal, and `wt remove`
        refuses to start any new removal while one exists. Renaming first makes the
        folder stop being an operation in one step. Alternatives: delete in place
        (simpler, but a crash or permission problem can block every later
        removal), or rename to the `.<id>.tmp` staging form (no new name, but it
        would then wait 14 more days, since the cut-short delete refreshes the
        folder's modification time). Reverting means dropping the `Expired` case
        and the rename in `store/retention.rs`, its docs paragraph, and the
        `a_failed_deletion_is_skipped_and_finished_on_the_next_run` test.
    - |-
        The completion time is read with a minimal, permissive parse of the report
        that looks only at `timeline[].at_ms` and ignores every other field, not
        through the strict `read_report`. That way retention dates every report
        version (4, 5, and the coming 6) the same way and never needs the
        entries. Any parse failure falls back to the folder's modification time,
        as the plan requires. A version 5 report is still read whole from disk
        (it can be 100 MB+ for a large removal) on each `wt remove` until it
        expires; Phase 2 makes reports small. Alternative: a streaming reader that
        stops after `timeline`, which is more code for a cost Phase 2 removes.
    - |-
        No test covers a report `at_ms` too large for the platform clock. On macOS
        and Linux `u64::MAX` milliseconds fits in `SystemTime` (a far-future time,
        so it is kept), while on Windows it overflows and falls back to the folder
        time, so one test cannot assert both. The code still falls back on
        overflow (`checked_add`), as the plan requires.
    - |-
        The deletion-failure test is Unix-only (it makes a subfolder read-only and
        skips itself when run as root), because Windows has no portable way to make
        `remove_dir_all` fail from a test without a production fault seam. Adding
        a seam is the alternative if the reviewer wants that case on Windows.
    - |-
        `os_failing` was not set on the spec. The Windows cross-check
        (`just cross-check worktree --os windows`) passed every retention test,
        but one unrelated deletion-engine test failed with a Windows sharing
        violation
        (`native_a_writer_in_the_moved_checkout_is_reported_and_its_file_deleted`).
        A rerun could not compile: another session working in this checkout was
        partway through rewriting `remove/usage.rs` and `remove/lifecycle.rs`
        (removing the usage gate and the Windows Search recheck), which this phase
        does not touch. Fixing that would have meant editing another session's
        unfinished work. If the reviewer wants the flag set anyway, set
        `os_failing: windows` on the spec.
message_to_agent: |-
    Phase 1 (retention) is done; read `decisions_for_review` in this log. Two
    things affect Phase 2. (1) Retention dates completion from a permissive
    `timeline[].at_ms`-only parse in `recovery/store/retention.rs`
    (`Timeline`), not from `RecoveryStore::read_report`, so the version 6
    report work need not touch retention; Wave 3's 'extend the retention
    fixture with legacy and new reports' can build version 6 reports through
    `Operation::complete` in `store/tests.rs` (helper `completed`). (2) While
    this phase ran, another session in the same checkout was editing
    `worktree/lib/src/remove/{usage.rs,lifecycle.rs,lifecycle/resume.rs}` (and
    darkmatter files). Those changes are not part of this fix. Do not revert
    them, and leave them out of this fix's file lists.
---

# Implementation Log for 2026-10-23-removal-store-size (3 phases)

## Phase 1

- Added `RecoveryStore::prune_completed(now)` in the new child module
  `worktree/lib/src/remove/recovery/store/retention.rs` (with
  `RETENTION`, 14 days, re-exported from `store`).
    - Candidates come from the store root's immediate children only:
      `OperationId`-named folders, `.<id>.tmp` staging folders, and
      `.<id>.expired` leftovers (see decisions). `locks/`, `.discarded`,
      files, and any other name are skipped.
    - An operation expires only when a valid record reads
      `completed_phase: completed`, the record's admin-name lock is taken
      without waiting, and a reread under that lock is still completed for
      the same admin name. The lock is held until deletion finishes.
    - Completion time is the last element of the report's `timeline`
      (`at_ms`), read through a minimal permissive struct (`timeline[].at_ms`
      only) so any report version dates the same way. A missing, unreadable,
      malformed, or empty-timeline report, or an `at_ms` that overflows
      `SystemTime`, falls back to the folder's mtime; an unreadable mtime
      keeps the folder. Deletion requires `now - completion > 14 days`.
    - Every failure is swallowed; nothing waits, retries, or prints.
- Wired `prune_recovery_store(base)` into `run` and `run_handoff` in
  `worktree/cli/src/commands/remove/mod.rs`, immediately after the base
  checkout resolves. It checks the store exists before `RecoveryStore::open`
  (so a run never creates it) and suppresses every error. `wt list`'s
  `PendingRemovals::read` path is untouched.
- Added `filetime = "0.2"` as a `worktree/lib` dev-dependency (already in
  `Cargo.lock` at 0.2.27); recorded in `docs/dependencies.md`.
- Doc updates: `worktree/docs/cli/remove.md` gains "How long records are
  kept" (rules, Mermaid flow, interruption safety) under "Recovery records and
  retry"; README gets a one-clause overview plus a link; the worktree skill's
  `remove.md` (retention facts and the `.expired` trap) and `lifecycle.md`
  (completed records expire) are updated; the `recovery/mod.rs` module docs
  say "kept after completion" lasts 14 days. No later-phase formats are
  documented.

### Requirement-to-test mapping

Boundary: filesystem persistence (store folders, mtimes, OS locks) at the
library's public `RecoveryStore::prune_completed` (F2, real temp store, L1),
plus the CLI wiring through the real `wt` binary (F3, L1, Unix-only like the
rest of `remove_recovery.rs`).

| Requirement | Test |
| --- | --- |
| Spec criterion: 15-day completed, 13-day completed, 30-day pending, old staging, locked completed; only the first and staging go (and the locked one goes once the lock is free) | `store::tests::retention_deletes_only_expired_unlocked_completed_operations_and_old_staging` |
| Strict 14-day boundary (exactly 14 days kept, +1 ms deleted); future completion kept even with a 30-day-old folder | `retention_keeps_exactly_fourteen_days_and_future_completions` |
| Last timeline event wins over an earlier, newer event and over the folder mtime, in both directions | `retention_dates_completion_by_the_last_timeline_event` |
| Missing, malformed, non-object, and empty-timeline reports fall back to the folder mtime (15 days deleted, 13 days kept, for each) | `retention_falls_back_to_the_folder_time_when_the_report_cannot_date_completion` |
| Pending (365 days), unreadable record, and record naming another operation are never expired | `retention_never_expires_a_pending_or_unreadable_record` |
| Young staging, `locks/` and its lock file, `.discarded`, `.<id>.tmp.extra`, `.not-an-id.tmp`, a `.<id>.tmp` *file*, other names and files are left alone | `retention_leaves_everything_that_is_not_an_operation_or_old_staging` |
| Links named as an operation or staging are not followed, and their targets survive | `retention_never_follows_a_link` |
| One failed deletion does not stop another; the leftover is hidden (`.expired`), never pending, does not block `begin`, and the next run finishes it | `a_failed_deletion_is_skipped_and_finished_on_the_next_run` (Unix) |
| `wt remove` cleans before name resolution, keeps a 13-day folder, and its stdout/stderr/exit match the clean-store control | `remove_recovery::remove_deletes_expired_finished_removals_first_and_silently` |
| `wt remove --handoff` (second entry point) cleans before consuming the token | `remove_recovery::a_handoff_run_deletes_expired_finished_removals` |
| `wt list` never cleans | `remove_recovery::list_never_deletes_expired_finished_removals` |

Input robustness (retention's report read): this is a deliberately
permissive reader whose only outcome on any invalid shape is "fall back to
the folder mtime"; it never treats invalid input as completion. Covered
shapes: absent file, malformed JSON (truncated), wrong document type (array),
empty `timeline`. The record reader is the existing strict `record::parse`
(its matrix is unchanged); a malformed record keeps the folder. The smell
search finds `.ok()` only in `completed_at`, which is exactly the fallback
the plan specifies.

### Gates

- `cargo nextest run -p worktree --lib -E 'test(retention) | test(a_failed_deletion)'`:
  8/8 pass.
- `cargo nextest run -p worktree-cli --test remove_recovery -E 'test(expired)'`:
  3/3 pass.
- `just lint` (worktree): clean.
- `just test` (worktree, both crates, 1615 tests): 1614 passed, 1 failed:
  `worktree-cli::list_prs concurrent_lists_and_workers_make_one_request_and_the_next_worker_asks_again`
  timed out at 20.8 s with the host load average around 36 (other sessions).
  Rerun alone it passed in 0.74 s, so it is recorded as load-sensitive and
  unrelated (PR-store worker; untouched).
- Dependent and test-input packages (the CI scope tool selects them for this
  phase's `Cargo.lock`, `docs/dependencies.md`, and skill edits), run
  narrowly:
    - `test-toolkit`: `ci_workflow_contracts::the_ci_documentation_states_the_implemented_behavior` passed.
    - `darkmatter`: `shipped_schema_corpus::{shipped_markdown_inline_schemas_parse_passively,shipped_markdown_named_type_references_resolve}` passed.
    - `sniff`: L1 lockfile tests (83) passed.
    - `worktree-cli` is the only crate that depends on `worktree`, and it is
      covered by `just test` above.
- Cross-OS: `just cross-check worktree --os windows` ran 885 tests: 884
  passed, including all seven cross-platform retention tests, and 1 failed:
  `remove::observation::deletion::tests::native_a_writer_in_the_moved_checkout_is_reported_and_its_file_deleted`
  (Windows sharing violation, OS error 32, while deleting a `target` folder in
  the deletion engine, which this phase does not touch). A filtered rerun
  could not compile, because another session in this checkout was editing
  `remove/usage.rs` and `remove/lifecycle.rs`. See `decisions_for_review`.

Phase 1 status: implementation complete, ready for review.
