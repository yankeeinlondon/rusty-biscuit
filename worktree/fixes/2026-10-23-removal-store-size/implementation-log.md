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
    - |-
        Phase 2: a saved report that cannot be read is now its own error,
        `StoreError::Report`, which names the `report.json` path and keeps the
        JSON parser's line and column. Before, it was a generic I/O error. A
        removal that is refused with it counts as "refused" (exit 3) like an
        unreadable record, not "blocked" (exit 4). Today nothing refuses with it:
        the only caller (a retry carrying the earlier report) turns it into a
        warning and continues from the journals, as the plan asks. Reverting the
        exit class is one match arm in `Refusal::exit_class`.
    - |-
        Phase 2: when the journals cannot be read back for the report, the
        in-memory entry list is also cleared, not only `entries_file` set to
        `null`. The plan only says to publish `null`. Keeping a stale list in
        memory would let the returned report disagree with the saved one. Nothing
        in the CLI shows the list, so this changes no output.
    - |-
        Phase 2: the report's `operation_id` and `quarantine` members must now be
        present (they may be `null`). The derived reader used to treat a missing
        member as `null`. Every earlier writer (versions 4 and 5) always wrote both,
        so no real report is affected. The plan's "Option smell" check prompted
        this. Nested members that were already lenient (an earlier attempt's
        `quarantine` and `observation`, a timeline event's `detail`) were left
        alone because this fix does not change them. Version 4's missing
        `observation` still reads as unknown, as the plan requires. Reverting
        means removing the two `deserialize_with = "present"` attributes.
    - |-
        Phase 2: the publication fault seam is a second injector,
        `fault::inject_save` with its own `SaveStep` enum, beside the existing
        `inject`/`ReopenStep` for `reopen`. The alternative was a single enum for
        both, which would have meant editing the reopen tests in
        `lifecycle/tests.rs`, a file another session is also editing. The cost
        is a few duplicated lines in the test-only module.
    - |-
        Phase 2: the stop report prints `entries.jsonl.gz` without backticks
        ("the saved entries list (entries.jsonl.gz) names every deleted path."),
        because the terminal would print backticks literally. The spec quoted the
        sentence with Markdown backticks.
    - |-
        Phase 2 fixed a Windows-only test outside this fix's scope:
        `remove::observation::deletion::tests::native_a_writer_in_the_moved_checkout_is_reported_and_its_file_deleted`.
        Its writer process kept its working directory inside the folder being
        deleted, and Windows refuses to delete a directory that is any process's
        working directory (the sharing violation phase 1 recorded). On Windows
        the test now ends the writer right after it has written
        (`Writer::exit`). The change is test-only, and the macOS and Linux paths
        are unchanged. With it, `just cross-check worktree --os windows` passes
        892/892.
    - |-
        Phase 2 did not set `os_failing` on the spec. `just cross-check
        worktree-cli --os windows` passes 657 of 658 tests. The one failure,
        `powershell_wrapper_exec::a_directory_held_by_another_program_refuses_with_exit_3_and_nothing_removed`,
        still expects the old usage refusal ("Removal paused…", exit 3), which
        the already-committed usage change removed. On that host `wt` now exits
        1 when Windows blocks the move. This fix does not touch that test. Its
        expectation depends on the usage-gate rework another session is doing
        in this checkout (it deleted `remove_usage_refusal.rs` and added
        `remove_usage_holders.rs`), and the right exit code is that change's
        decision: the remove docs say a Windows move blocked by an open file
        exits 4. If the reviewer prefers, set `os_failing: windows`.
    - |-
        Phase 3: the reader now refuses an entry planned by two batches of one
        journal, in version 2 journals too, as the plan said. Only the writer
        refused that before, and that writer check is itself recent. A version
        2 journal from an older `wt` that did plan an entry twice now cannot be
        read, so its retry stops (exit 1) instead of resuming. No known writer
        produced such a journal. The alternative, accepting duplicates in
        version 2 only, would let the size limit's "each entry once" bound be
        broken by old files. Reverting is one `mark_planned` call in
        `read_batches`, limited to version 3.
    - |-
        Phase 3 changed production code the plan did not list: the deletion
        inventory now writes itself without making a copy first (a hand-written
        `Serialize` for `DeletionInventory` instead of `serde(into = ...)`, and
        the journal writer borrows it instead of cloning it). Each new journal
        used to copy the whole inventory twice in memory, which for a
        100,000-file checkout is about 32 MB copied for nothing; the slow
        100k test exposed it. The bytes written are the same (the size test
        and the robustness matrix compare them). The alternative was to leave
        it and accept slower tests. Reverting means restoring the two derive
        attributes in `observation/inventory.rs` and the clone in
        `JournalWriter::start`.
    - |-
        Phase 3 added three test-profile settings to the root `Cargo.toml`,
        compiling the gzip libraries (`miniz_oxide`, `crc32fast`, `adler2`)
        with optimization in test builds, as the repo already does for
        `serde_json` and the regex libraries. With them, the 100,000-entry
        completed-folder test went from 2.9 s to 2.1 s when run alone, and
        it had reached 5.9 s (over the 5 s L1 budget) in a full run. The change affects test builds only, and the CI planner treats
        it as a change to the `repo-deps` tests that read that file. The
        alternatives were a smaller fixture (the spec fixes the size at
        100,000) or a slow test. Reverting means deleting those three
        `[profile.test.package.*]` blocks.
    - |-
        Phase 3 split the plan's single stopped-journal comparison into two
        tests. One measures the bytes and reads the old-format journal back;
        the other reads the new-format journal back. Both compare against the
        same expected batches, so the two formats are shown to read back
        alike without one test parsing the 32 MB inventory twice. Building
        the old-format bytes reuses the inventory line the new writer produced,
        since that line is identical in both formats; so the check that the
        openings differ only in the version digit really covers the header.
        Reading the old-format file back still checks that the inventory line
        reads under the old format. Together these brought each test from
        about 3 s alone (5.5 to 5.9 s under load) to about 2 s.
    - |-
        Phase 3 found, and did not change, an existing mismatch. When a retry
        meets a journal it cannot read, `wt` exits 1. The library classifies
        the same error as a refusal (exit 3), but the CLI hits it earlier,
        while looking up the pending removal, and reports it as a failure.
        The new journal section in the remove docs says exit 1, matching the
        code, and the CLI tests assert 1. Also, the message gives a line
        number but not which journal file (current or archived) the line is
        in. Routing this through the refusal path (exit 3), or naming the
        file, is a separate behavior change. It would mean updating
        `assert_retry_refused_by_the_journal` and the docs sentence.
source_files_during_phase_2:
    - worktree/lib/Cargo.toml
    - Cargo.lock
    - worktree/lib/src/remove/outcome.rs
    - worktree/lib/src/remove/lifecycle.rs
    - worktree/lib/src/remove/recovery/mod.rs
    - worktree/lib/src/remove/recovery/journal.rs
    - worktree/lib/src/remove/recovery/store.rs
    - worktree/lib/src/remove/recovery/store/entries.rs
    - worktree/lib/src/remove/recovery/store/tests.rs
    - worktree/lib/src/remove/recovery/store/tests/saved_report.rs
    - worktree/lib/src/remove/lifecycle/tests.rs
    - worktree/lib/src/remove/observation/deletion/tests.rs
    - worktree/cli/src/commands/remove/recovery.rs
    - worktree/cli/tests/remove_recovery.rs
    - worktree/cli/tests/snapshots/remove_recovery_report__a_git_command_in_the_moved_checkout.snap
    - worktree/cli/tests/snapshots/remove_recovery_report__a_stop_after_partial_discovery.snap
    - worktree/cli/tests/snapshots/remove_recovery_report__long_lists_are_counted_and_uncertain_entries_named.snap
    - worktree/cli/tests/snapshots/remove_recovery_report__the_specs_example.snap
docs_updated_during_phase_2:
    - worktree/docs/cli/remove.md
    - worktree/README.md
    - docs/dependencies.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/worktree/remove.md
    - .claude/skills/worktree/lifecycle.md
source_files_during_phase_3:
    - Cargo.toml
    - worktree/lib/src/remove/recovery/journal.rs
    - worktree/lib/src/remove/recovery/mod.rs
    - worktree/lib/src/remove/observation/inventory.rs
    - worktree/lib/src/remove/recovery/store/tests.rs
    - worktree/lib/src/remove/recovery/store/tests/journal_size.rs
    - worktree/lib/src/remove/lifecycle/tests.rs
    - worktree/lib/src/remove/lifecycle/tests/resume_consent.rs
    - worktree/cli/tests/remove_recovery.rs
docs_updated_during_phase_3:
    - worktree/docs/cli/remove.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/worktree/remove.md
    - .claude/skills/worktree/lifecycle.md
source_code:
    - Cargo.lock
    - Cargo.toml
    - worktree/lib/Cargo.toml
    - worktree/lib/src/remove/outcome.rs
    - worktree/lib/src/remove/lifecycle.rs
    - worktree/lib/src/remove/recovery/mod.rs
    - worktree/lib/src/remove/recovery/journal.rs
    - worktree/lib/src/remove/recovery/store.rs
    - worktree/lib/src/remove/recovery/store/entries.rs
    - worktree/lib/src/remove/recovery/store/retention.rs
    - worktree/lib/src/remove/recovery/store/tests.rs
    - worktree/lib/src/remove/recovery/store/tests/saved_report.rs
    - worktree/lib/src/remove/recovery/store/tests/journal_size.rs
    - worktree/lib/src/remove/observation/inventory.rs
    - worktree/lib/src/remove/observation/deletion/tests.rs
    - worktree/lib/src/remove/lifecycle/tests.rs
    - worktree/lib/src/remove/lifecycle/tests/resume_consent.rs
    - worktree/cli/src/commands/remove/mod.rs
    - worktree/cli/src/commands/remove/recovery.rs
    - worktree/cli/tests/remove_recovery.rs
    - worktree/cli/tests/snapshots/remove_recovery_report__a_git_command_in_the_moved_checkout.snap
    - worktree/cli/tests/snapshots/remove_recovery_report__a_stop_after_partial_discovery.snap
    - worktree/cli/tests/snapshots/remove_recovery_report__long_lists_are_counted_and_uncertain_entries_named.snap
    - worktree/cli/tests/snapshots/remove_recovery_report__the_specs_example.snap
documentation:
    - worktree/docs/cli/remove.md
    - worktree/README.md
    - docs/dependencies.md
    - .claude/skills/worktree/remove.md
    - .claude/skills/worktree/lifecycle.md
completed_phase: 3
implemented: true
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

## Phase 2

- `REPORT_VERSION` is 6. `OperationReport::journal_entries` remains the
  full in-memory list (`#[serde(skip_serializing)]`); the new
  `entries_file: Option<String>` is always serialized (`null` or
  `"entries.jsonl.gz"`), and is set only through
  `OperationReport::set_entries`, which `Attempt::summarize_journal` calls on
  success, and with an empty list on failure.
- `Deserialize for OperationReport` is hand-written (`outcome.rs`):
  `strict_json::nested` refuses repeated keys at any depth, then
  `report_version` (it must be a whole number) dispatches. Versions 4 and 5
  require inline `journal_entries` and refuse `entries_file`. Version 6
  requires `entries_file` (`null` or the exact name) and refuses
  `journal_entries`. Any other version is refused. Every other member goes
  through `SharedMembers` (`deny_unknown_fields`); `operation_id` and
  `quarantine` are now required-present (see decisions).
- New `recovery/store/entries.rs`: `GzBuilder` (mtime 0, no name, default
  level) streams each `JournalEntry` with `serde_json::to_writer` plus a
  newline into a hidden `temp_beside` file. It then calls `finish`,
  `storage::flush`, closes the file (Windows), and runs `storage::publish`;
  on an ordinary failure the temporary is removed. `flate2 = { version =
  "1", default-features = false, features = ["rust_backend"] }` added
  (lockfile: one dependency line, no version changes).
- `Operation::save_report` publishes the sidecar first when the report
  names it, then `report.json`. A report that names none deletes a stale
  sidecar after publishing (best effort). A report naming any other
  filename is refused with an I/O error, so the store never writes to a
  path taken from a report. `complete` keeps its order: report, then the
  `Completed` phase, then journal removal gated on `journal_reported`.
- `RecoveryStore::read_report` maps a parse failure to
  `StoreError::Report { path, reason }` (the path to `report.json`, and
  serde_json's line and column), and still returns `Ok(None)` for a missing
  file.
- Test-only fault seam: `SaveStep::{EntriesWritten, EntriesFlushed,
  EntriesPublished, ReportPublished, Completed}` through
  `fault::inject_save`, as a crash or as a failed write.
- CLI: the stop caveat now reads "…the saved entries list
  (entries.jsonl.gz) names every deleted path." and the deleted-count hint
  reads "(listed in entries.jsonl.gz)". Four snapshots were edited by hand
  to those two lines only; every `.snap.new` was reviewed and deleted. The
  activity, error, and change counts still point to the saved report, which
  still holds them.
- Docs: `docs/cli/remove.md` gains a file table for the operation folder,
  "What the report holds", "The entries file" (a version 6 JSON example, a
  decompressed JSONL example, write order, `null`, failure guarantees,
  legacy versions, and the unreadable-report warning), and "Retrying a
  stopped removal". The stop example text is updated. The README overview
  names the compressed list. `recovery/mod.rs` module docs list the
  sidecar and its crash guarantee. `docs/dependencies.md` records `flate2`
  in the notes and in the catalog. The worktree skill's `remove.md` (layout,
  report v6, hand-written reader, `inject_save`) and `lifecycle.md`
  (completion order) are updated.

### Requirement-to-test mapping

Boundary: persistence (two files published by rename, gzip, owner-only
storage) through the library's public `Operation::save_report`,
`Operation::complete`, and `RecoveryStore::read_report` (F2, real temp
store, L1); the lifecycle through `remove_checkout`/`resume` (F2, real Git
fixture, L1); and the shipped `wt` binary (F3, L1, Unix-only like the rest
of `remove_recovery.rs`).

| Requirement | Test |
| --- | --- |
| Saved report has no inline list; names `entries.jsonl.gz`; sidecar decodes to every state (deleted, externally absent, kept, planned, uncertain); gzip header has no name or time; `0600`; deterministic bytes; read-back equals metadata | `store::tests::saved_report::a_saved_report_keeps_its_entries_in_the_entries_file` |
| Repeated stop replaces both files; empty list gives explicit `null`, no sidecar, stale sidecar removed; no temporaries; a non-literal `entries_file` is refused and writes nothing | `…::every_save_replaces_the_entries_and_a_report_without_entries_names_none` |
| Completion with no entries: no sidecar, journal deleted | `…::completion_without_entries_writes_no_entries_file` |
| Failed summary at completion: `null`, stale sidecar removed, journals kept | `…::completion_that_could_not_list_entries_keeps_the_journals_and_no_stale_entries` |
| Crash or failure at gzip finish, flush, sidecar publication, report publication, and after completion: each file old-or-new and complete, journals kept, completed only after both files, failed writes leave no temporary, journals still recover the same entries | `…::completion_stopped_at_each_step_leaves_complete_files_and_the_journals` |
| Lifecycle: every completed removal's saved JSON has no inline list, and its sidecar equals the in-memory entries (counts, 200-path preview, omitted count, activity, observation, timeline, earlier attempts compared in full) | `lifecycle::tests::saved_report` helper used by `assert_removed` (every completing lifecycle test), `a_completed_report_lists_every_entry_past_the_deleted_cap` |
| A stop saves every entry so far; across attempts after a resume | `lifecycle::tests::an_interrupted_removal_is_resumed` (new stop assertion plus the existing resume assertions via `assert_removed`) |
| Version 5 metadata with inline entries (frozen bytes) is read by `read_report`, carried into `earlier_attempts`, resumes and completes; final entries come from the journals, not the inline list | `lifecycle::tests::a_removal_stopped_by_a_version_5_wt_resumes_and_completes` |
| An unreadable earlier report: the warning names the path, and the retry completes from the journals | `lifecycle::tests::an_unreadable_earlier_report_is_named_and_the_retry_still_completes` |
| Journal that cannot be summarized: error recorded, `entries_file: null`, no list, no sidecar, journal kept | `lifecycle::tests::a_stop_whose_journal_cannot_be_summarized_saves_no_entries_file` |
| CLI: stop text (caveat and deleted-count location) for stops containing deletions | snapshots `remove_recovery_report__{the_specs_example,a_stop_after_partial_discovery,a_git_command_in_the_moved_checkout,long_lists_are_counted_and_uncertain_entries_named}` |
| CLI (shipped binary): a stop saves a v6 report with no inline list; the retry completes with a sidecar and no journals, and no warning | `remove_recovery::an_interrupt_after_the_move_stops_with_a_report_and_a_retry_finishes` (extended) |
| CLI (shipped binary): an unreadable saved report makes the retry warn with its path, and the retry still finishes | `remove_recovery::a_retry_past_an_unreadable_saved_report_warns_with_its_path_and_finishes` |
| Retention dates v6 (with a non-gzip sidecar, proving it is never opened) and frozen v5 reports alike: 15 days deleted, 13 kept | `store::tests::saved_report::retention_dates_reports_of_either_shape_without_reading_the_entries` |

### Input robustness (report loader, `RecoveryStore::read_report`)

Default outcome: every invalid shape is `StoreError::Report` naming the
`report.json` path, with nothing published. Exceptions: a missing file is
`Ok(None)`; version 6 `entries_file: null` is valid and names none; a
legacy empty `journal_entries: []` is valid and means no entries; a
missing `observation` (version 4) or a `null` one reads as unknown.

- Controls, versions 4, 5, and 6 (and v6 `null`), with a repeated
  write-read round trip of what this `wt` writes:
  `saved_reports_of_versions_4_5_and_6_read_back`.
- `report_version` (one validator for every version): absent, null,
  string, empty string, empty object, negative, fractional, older (3), and
  newer (7) are all refused:
  `a_report_version_that_is_missing_wrong_or_unknown_is_refused`.
- `entries_file`: v6 absent, wrong type (number, array), empty, another
  filename, absolute path, and traversal are refused; null and the exact
  literal are accepted. v4 and v5 with `null` or the literal are refused.
  v6 with inline `[]` or a full list is refused:
  `entries_file_must_be_null_or_its_one_name_and_only_in_version_6`.
- Legacy inline list (shared by v4 and v5, each tested): absent, null,
  wrong whole type, one element wrong, and every element wrong are
  refused; empty is valid:
  `a_legacy_inline_entry_list_must_be_present_and_whole`.
- Entry members `attempt`, `batch`, `path`, `kind`, `state`: absent, null,
  wrong type, and empty are each refused. Invalid content (negative
  attempt, fractional batch, unknown path member, unknown kind, unknown
  state) and an unknown entry member are refused, with one valid wiring
  case (`link`/`externally_absent`):
  `every_member_of_a_legacy_inline_entry_is_required_and_typed`.
- `operation_id`, `quarantine`: null accepted, absent refused (in the
  control test).
- Document shapes: empty file, malformed or truncated JSON (the error keeps
  "line 1 column"), trailing data, a second document, a non-object, a
  repeated top-level key, a repeated nested key, and an unknown top-level
  member are all refused:
  `a_report_document_that_is_not_one_strict_object_is_refused`.
- Smell search over `from_wire`/`SharedMembers`, `read_report`, and
  `entries.rs`: no `serde(default)`, `filter_map`, `unwrap_or_default`, or
  `.ok()`. The remaining `Option` fields are `operation_id`/`quarantine`
  (required-present via `present`) and `observation` (the documented
  version 4 exception). Nested types that are unchanged (an earlier
  attempt's `quarantine`/`observation`, a timeline event's `detail`) keep
  their previous derived behavior.
- The sidecar has no production reader (plan rule 13); the test decoder
  checks the gzip trailer, a final newline, and that each line is one
  strict `JournalEntry`.

### Gates

- Targeted: `cargo nextest run -p worktree --lib -E 'test(/^remove::(outcome|recovery|lifecycle)::/)'`
  passed 127/127. `cargo nextest run -p worktree-cli --test remove_recovery_report`
  passed 22/22. `--test remove_recovery -E 'test(an_interrupt_after_the_move) | test(unreadable_saved_report)'`
  passed 2/2.
- `just test` (worktree, both crates): 1623 run, 1623 passed (22 slow,
  all `git_graph`/`list` graph layout tests, untouched, under host load
  around 9 to 36), 15 skipped.
- `just lint` (worktree): clean. `just check-tier-coverage worktree`: 0
  stranded.
- Scope calculator (`git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `change_class: package`, packages `worktree`, `worktree-cli`,
  `test-toolkit`, `darkmatter`, `sniff` (plus `biscuit-visualized`,
  selected only by another session's uncommitted files). The test-input
  packages were run narrowly, serially:
  `test-toolkit` `the_ci_documentation_states_the_implemented_behavior`
  1/1; `darkmatter` `shipped_markdown_inline_schemas_parse_passively` and
  `shipped_markdown_named_type_references_resolve` 2/2; `sniff`
  `test(/lockfile/)` 194/194. `worktree-cli` is the only crate that
  depends on `worktree`, and `just test` covered it.
- Cross-OS: `just cross-check worktree --os windows` first failed 1/892,
  the same deletion-engine test phase 1 saw. Attempt 1 fixed it (see
  decisions), and the rerun passed 892/892. `just cross-check worktree-cli
  --os windows` passed 657/658; the failure is
  `powershell_wrapper_exec::a_directory_held_by_another_program_refuses_with_exit_3_and_nothing_removed`,
  which is stale against the committed usage-gate change and outside this
  fix (see decisions). Not attempted; `os_failing` not set.

- Note for review: while this phase ran, commit `f81cc554c` ("fix(worktree):
  report holders and persist recovery entries", 13:44) was made in this
  checkout by another session, not by this phase. It swept in a mid-phase
  snapshot of this phase's files (`outcome.rs`, `lifecycle.rs`, `store.rs`,
  `store/entries.rs`, `store/tests/saved_report.rs`, `lib/Cargo.toml`, the
  CLI caveat and snapshots, and part of `docs/cli/remove.md`) together with
  that session's usage-holder work. The rest of this phase is uncommitted.
  The phase's full change is the union of the
  `source_files_during_phase_2` list, not the working-tree diff alone.

Phase 2 status: implementation complete, ready for review.

## Phase 3

- `journal.rs` writes `JOURNAL_VERSION = 3` and reads versions 2 and 3,
  each file by its own header (`HeaderLine = Line<IgnoredAny, IgnoredAny>`),
  so archived `journal-<n>.jsonl` files of either version mix. The private
  wire enum is `Line<I, R>` with `IntentV2`/`ResultsV2` (paths and kinds,
  read only) and `IntentV3`/`ResultsV3` (`entries: Vec<u64>` positions,
  `states: Vec<ResultState>`). Both versions normalize into the unchanged
  public `Batch { entries: Vec<PlannedEntry>, results }` before the shared
  `check_batch`, which now returns each entry's inventory position. A
  position goes through `usize::try_from` and a bounds check; nothing is
  sorted or renumbered.
- Cross-batch uniqueness is enforced by the reader for both versions
  (`mark_planned`) as well as by the writer. The writer's
  `filter_map(position)` is gone: `check_batch` errors on an unknown entry
  and the positions come from it.
- `limit_for` sums, with saturating arithmetic, the exact version 3
  encoding of each position planned alone with batch `u64::MAX`, plus the
  longest results line (`externally_absent`). Header and inventory still
  do not count.
- Public `PlannedEntry`, `Batch`, `JournalLog`, `JournalWriter::{begin_batch,
  finish_batch}`, and replay results are unchanged; the deletion engine
  is untouched.
- Found while speeding up the size tests (production work the slow test
  exposed): `DeletionInventory` serialized through `#[serde(into =
  "InventoryRecord")]`, which clones the whole inventory, and
  `JournalWriter::start` cloned it once more first. It now serializes
  borrowed fields (same bytes), and the writer borrows it
  (`InventoryLine`). See decisions.
- Root `Cargo.toml`: test-profile `opt-level = 3` for `miniz_oxide`,
  `crc32fast`, `adler2` (the gzip used by `entries.jsonl.gz`), following
  the existing `[profile.test.package.*]` precedent. See decisions.
- Docs: `docs/cli/remove.md` gains a lifecycle Mermaid diagram (journal →
  entries/report save → completion → journal removal → 14-day cleanup) and
  a "The journal" section (version 3 example, what positions and states
  mean, the batch rules, the retry's exit 1 on an unreadable journal,
  version 2 example and mixed-generation reading). The worktree skill's
  `remove.md` journal facts are rewritten for version 3 (plus a trap about
  editing journals by path in tests), and `lifecycle.md` notes mixed
  versions across generations. `recovery/mod.rs`'s file table says batches
  name inventory positions. Audited `recovery/mod.rs`, `journal.rs`,
  `store.rs`, and `outcome.rs`: no remaining "only version" or
  report-inline-entries claims (the `record.rs` "only `record_version`"
  line is about records and still true).
- Work was split: the robustness matrix, the 100k size tests, and the
  lifecycle migration plus CLI cases were written by three parallel
  subagents, each owning its own file; I reviewed them and reworked the size
  tests for speed.

### Requirement-to-test mapping

Boundary: the journal is a persisted file read back by
`RecoveryStore::recover` / `Operation::journal_entries` (F2, real temp
store, L1); the lifecycle through `remove_checkout`/`resume` on a real Git
fixture (F2, L1); and the shipped `wt` binary (F3, L1, Unix-only like the
rest of `remove_recovery.rs`).

| Requirement | Test |
| --- | --- |
| Version 3 lines hold positions (in the intent's order) and bare states; no path or kind after the inventory line; header says 3 | `journal::tests::version_3_batches_hold_positions_and_states_only` |
| Frozen version 2 bytes read back equal to the version 3 writer's journal (`JournalLog` and replay) | `journal::tests::a_version_2_journal_reads_back_like_its_version_3_equivalent` |
| `limit_for` equals the frozen-literal version 3 encoding at inventory sizes 0, 9, 10, 11, 99, 100, 101, 999, 1000, 1001 (digit boundaries; empty inventory → 0) | `journal::tests::the_limit_counts_each_positions_own_digits` |
| Full 256-entry batches and every entry alone fit the limit past 1,000 entries | `journal::tests::full_batches_and_single_entries_fit_the_limit_past_a_thousand_entries`, plus the updated `the_limit_fits_every_entry_planned_alone_with_the_longest_result` |
| Exact budget / one byte short, on version 3 bytes | `journal::tests::the_limit_refuses_an_intent_while_its_results_still_fit` (updated) |
| Failed append/sync poisoning, torn results, cross-batch refusal by the writer | existing `a_torn_results_line_leaves_its_batch_uncertain`, `a_failed_intent_flush_refuses_the_batch_and_every_later_write`, `an_entry_an_earlier_batch_planned_is_refused` (now on version 3 bytes) |
| Version 3 reader matrix (below) | `store::tests::journal_robustness_matrix_version_3` |
| Version 2 reader matrix on frozen bytes (below) | `store::tests::journal_robustness_matrix_version_2` |
| Shared inventory validator, once (below) | `store::tests::journal_robustness_matrix_shared_inventory` |
| Reopen crash points and generation de-duplication still hold | existing `a_reopen_stopped_at_any_step_…`, `a_completed_reopen_archives_…`, `earlier_attempts_keep_their_deletions_and_uncertainty_without_double_counting` |
| 100,000 entries × 150-byte paths: completed folder (record + v6 report + gzip) < 5% of record + frozen v5 pretty report; gzip equals entries; no journals left; v5 baseline minus inline list equals the saved report | `store::tests::journal_size::a_completed_removal_of_100k_entries_takes_under_5_percent_of_the_version_5_folder` |
| Same inventory, full batches: openings differ only in the version digit; version 3 batch bytes < version 2; frozen v2 reads back as planned | `store::tests::journal_size::version_3_batches_of_100k_entries_are_smaller_than_version_2s` |
| The version 3 journal of the same batches reads back as planned (the same expectation, so both versions read alike) | `store::tests::journal_size::a_version_3_journal_of_100k_entries_reads_back_as_planned` |
| Migration: v5 report + v2 attempt 1 (with an uncertain entry) + v2 attempt 2 archived, fresh v3 attempt 3; headers 2, 2, 3; attempts ordered 1..3; each path counted once; uncertainty kept; `earlier_attempts` carried; gzip equals in-memory entries; journals removed | `lifecycle::tests::a_removal_stopped_twice_by_an_older_wt_resumes_with_version_3_journals` |
| Tests that edit journals by path work on version 3 | `lifecycle::tests::resume_consent::only_a_confirmed_result_explains_a_missing_file_and_an_unreadable_journal_refuses` (helper `rewrite_result` resolves positions) |
| CLI control: retry with the journal rewritten byte-identical finishes (exit 0) | `remove_recovery::a_retry_with_its_journal_rewritten_unchanged_finishes` |
| CLI: an intent naming position 999999 → `Nothing was removed.` + `deletion journal line 3 is malformed: a batch plans a position the inventory does not have`, exit 1, quarantine/branch/pending kept | `remove_recovery::a_retry_refuses_a_journal_batch_naming_a_position_the_inventory_lacks` |
| CLI: header `journal_version: 4` → `the deletion journal has version 4, which this wt does not read`, exit 1, nothing removed | `remove_recovery::a_retry_refuses_a_journal_of_an_unsupported_version` |

### Input robustness (journal loader, `journal::parse` via `RecoveryStore::recover`)

Default outcome: every invalid shape is `StoreError::Journal` (or
`InventoryMismatch` for another checkout's root), recovery is read-only
(asserted: directory names, contents, and journal bytes unchanged), and a
retry deletes nothing. Exceptions: an unterminated final line is ignored
as torn (torn intent, torn results, and a complete intent missing only its
newline are Valid with `torn_tail`); a torn opening inventory or header is
an error; a header-and-inventory-only journal is valid (no batches).

- Version 3 controls: as written; re-encoded; header+inventory only;
  closing results for the open batch.
- `intent.entries`, `results.states`, `intent.batch`, `results.batch`,
  header `journal_version`, header `operation_id`: absent, null, wrong
  type, empty, duplicate key → refused.
- Indices: 0, 1 (a directory), 6 (last, the link) valid when alone in a
  batch; 7 (first out of range), -1, 5.5, `"5"`, `u64::MAX`,
  `18446744073709551616`, `5e0` refused. Line numbers asserted (e.g. index
  7 → line 5).
- Elements: one wrong / every wrong for entries and states; too few and too
  many states refused.
- States: `deleted`, `externally_absent`, `kept` valid; `vanished`,
  `Deleted`, numeric, null, `""` refused.
- Batch rules: 257 entries (refused by the bound itself), mixed parents,
  duplicate within a batch, duplicate across batches, skipped number, batch
  1 again, intent before results, results with no open intent, repeated
  results, changed results batch, batch 0 → refused. Header version 1, 4,
  0, negative, fractional, past `u32` → refused (`UnsupportedVersion(1)` /
  `(4)` asserted); another / upper-case / too-long operation id → refused
  (`OperationMismatch` asserted).
- Document shapes: duplicate line key, extra members (header, intent,
  results, and line level), version 2 batch lines in a version 3 journal
  and a version 2 header over version 3 batches, trailing garbage, two
  values on a line, not JSON, blank line (middle and end), empty file, a
  lone newline, absent/repeated header, absent/header-only/repeated/late
  inventory → refused. The truncated intent or results bytes **with** a
  newline → refused (separately from the torn cases).
- Version 2 (frozen `json!` bytes): control and torn intent valid; `path`,
  `kind`, `state` each absent/null/wrong type/empty/duplicate/invalid;
  entries one/every wrong and empty; unknown members in path, intent entry,
  results entry; unlisted entry; wrong kind; duplicate within and across
  batches; version 3 lines in a version 2 journal; wrong results batch;
  misaligned results → refused; one inventory wiring case (`entries` wrong
  type, line 2) proves the shared validator.
- Shared inventory (once, on version 3): the ten existing columns (root,
  entries, path, identity, shape, file size/modified/mode/content, link
  target) × absent/null/wrong type/empty/duplicate/invalid, plus
  one/every element wrong, empty, unknown members, a path twice, a child
  before its directory, another checkout's root, trailing content; a
  re-spaced inventory is valid.
- Not covered: CRLF line endings and trailing whitespace on a line
  (`serde_json` accepts both, consistent with the re-spaced cell).
- Smell search over the journal loading code: `.ok()` only in
  `IntentV3::planned`, where it turns a failed `usize` conversion into the
  out-of-range error; `unwrap_or_default` only for the torn-tail split; no
  `serde(default)` or `filter_map`. `Batch::results: Option<…>` is the
  documented "no durable results" state, not a deserialized field. Report
  loading was audited in Phase 2 and not changed here.

### Measurements (100,000 entries, 150-byte paths)

| | Bytes |
| --- | --- |
| New completed folder: `entries.jsonl.gz` 306,701 + `record.json` 2,020 + `report.json` 54,703 | 363,424 |
| Version 5 completed folder: `record.json` 2,020 + pretty `report.json` with inline entries 29,734,244 | 29,736,264 |
| Ratio | 1.222% (limit 5%) |
| Journal opening (header + inventory), version 3 / version 2 | 32,189,225 / 32,189,225 |
| Journal batch lines, version 3 / version 2 | 1,624,613 / 38,036,109 (4.271%) |

### Gates

- Targeted: `cargo nextest run -p worktree --lib -E 'test(/^remove::recovery::journal::/)'`
  14/14; matrix tests 3/3; `-E 'test(/^remove::lifecycle::/)'` 63/63;
  `-p worktree-cli --test remove_recovery` 15/15 (subagent runs).
- The size tests first took 5.5 s and 5.9 s under a full run (about 3 s
  alone), over the 5 s L1 budget. After the borrowed inventory serialization,
  the gzip test-profile optimization, and splitting the version 2/3
  read-back into two tests that share one expectation, they take 1.9–2.3 s
  alone and 2.2–2.7 s in the full run. The floor is building and parsing a
  32 MB inventory once per test.
- `just lint` (worktree): clean.
- `just test` (worktree, both crates): 1636 run, 1636 passed (21 slow, all
  `git_graph`/`list` graph layout tests, untouched), 15 skipped.
- `just check-tier-coverage worktree`: 0 stranded.
- Scope calculator over this phase's files: `change_class: package`,
  packages `worktree`, `worktree-cli`, `test-toolkit`, `darkmatter`,
  `repo-deps` (the whole working tree also selects `biscuit-terminal`,
  `biscuit-visualized`, `darkmatter-cli` through other sessions'
  uncommitted files). Run narrowly, serially:
    - `test-toolkit` is selected by its `archive-path-guard` source scan:
      `just archive-path-guard` reports 11 violations, all in
      `biscuit-visualized/` (another session's uncommitted work), none in
      `worktree`.
    - `repo-deps` (root `Cargo.toml` is a test input): the tests that read
      it, `the_shipped_sidecar_table_is_readable_and_names_only_real_packages`,
      `every_real_build_record_declares_only_payloads_the_producer_can_emit`,
      `every_shipped_sidecar_is_declared_by_a_package_and_names_real_binaries`:
      3/3.
    - `darkmatter` (skill edits): `shipped_markdown_inline_schemas_parse_passively`,
      `shipped_markdown_named_type_references_resolve`: 2/2.
    - `worktree-cli` is the only crate depending on `worktree` and is covered
      by `just test`.
- Note for review: while this phase ran, commit `b0d59f187` ("fix(worktree):
  compact removal journal batches", 14:06) was made in this checkout by
  another process, not by this phase. It holds a mid-phase snapshot of
  `journal.rs`. The phase's full change is the union of
  `source_files_during_phase_3`, not the working-tree diff alone.
- Cross-OS: `just cross-check worktree --os windows` passed 902/902 (1
  skipped), including the three 100k storage-size tests and the journal
  matrices. `worktree-cli` changed only its Unix-only `remove_recovery.rs`
  tests this phase, so it was not cross-checked; `os_failing` is not set.
  On the Windows rig the three size tests took 3.5 s, 3.8 s, and 4.2 s,
  under the 5 s budget but with little margin; a slower runner could flag
  them as slow. The remaining cost is reading the 32 MB inventory, which is
  the specified fixture.

Phase 3 status: implementation complete, ready for review.
