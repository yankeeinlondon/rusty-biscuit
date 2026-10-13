---
total_phases: 12
created: 2026-10-11
phase: 12
agent: opencode/zai-coding-plan/glm-5.3
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - worktree/fixes/2026-10-05-graceful-file-contention/spikes/README.md
skills_files_updated_during_phase_1:
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/windows.md
  - .claude/skills/os/wsl.md
  - .claude/skills/os/macos.md
  - .claude/skills/os/linux.md
  - .claude/skills/worktree/remove.md
source_files_during_phase_2:
  - worktree/lib/Cargo.toml
  - Cargo.lock
  - worktree/lib/src/remove/mod.rs
  - worktree/lib/src/remove/handoff.rs
  - worktree/lib/src/remove/outcome.rs
  - worktree/lib/src/remove/recovery/mod.rs
  - worktree/lib/src/remove/recovery/record.rs
  - worktree/lib/src/remove/recovery/journal.rs
  - worktree/lib/src/remove/recovery/lock.rs
  - worktree/lib/src/remove/recovery/storage.rs
  - worktree/lib/src/remove/recovery/store.rs
  - worktree/lib/src/remove/recovery/store/tests.rs
docs_updated_during_phase_2:
  - docs/dependencies.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/worktree/remove.md
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/macos.md
source_files_during_phase_3:
  - worktree/lib/src/remove/mod.rs
  - worktree/lib/src/remove/usage.rs
  - worktree/lib/src/remove/usage/tests.rs
  - worktree/cli/src/commands/remove/mod.rs
  - worktree/cli/src/commands/remove/usage.rs
  - worktree/cli/tests/remove_usage_refusal.rs
  - worktree/cli/tests/snapshots/remove_usage_refusal__the_specs_example.snap
  - worktree/cli/tests/snapshots/remove_usage_refusal__every_evidence_location_and_holder_shape.snap
  - worktree/cli/tests/snapshots/remove_usage_refusal__untrusted_text_is_shown_literally.snap
  - worktree/cli/tests/snapshots/remove_usage_refusal__long_lists_are_counted.snap
  - worktree/cli/tests/snapshots/remove_usage_refusal__coverage_notes.snap
  - worktree/cli/tests/snapshots/remove_usage_refusal__an_unverified_checkout_refuses.snap
docs_updated_during_phase_3:
  - worktree/docs/cli/remove.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/worktree/remove.md
source_files_during_phase_4:
  - worktree/lib/Cargo.toml
  - Cargo.lock
  - worktree/lib/src/remove/mod.rs
  - worktree/lib/src/remove/quarantine/mod.rs
  - worktree/lib/src/remove/quarantine/rename.rs
  - worktree/lib/src/remove/quarantine/rename/tests.rs
  - worktree/lib/src/remove/recovery/storage.rs
docs_updated_during_phase_4:
  - docs/dependencies.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/worktree/remove.md
source_files_during_phase_5:
  - worktree/lib/src/remove/admin_entry.rs
  - worktree/lib/src/remove/recovery/mod.rs
  - worktree/lib/src/remove/recovery/storage.rs
  - worktree/lib/src/remove/quarantine/mod.rs
  - worktree/lib/src/remove/quarantine/naming.rs
  - worktree/lib/src/remove/quarantine/association.rs
  - worktree/lib/src/remove/quarantine/association/tests.rs
docs_updated_during_phase_5:
  - worktree/docs/cli/remove.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/worktree/remove.md
source_files_during_phase_6:
  - worktree/lib/Cargo.toml
  - Cargo.lock
  - worktree/lib/src/remove/mod.rs
  - worktree/lib/src/remove/usage.rs
  - worktree/lib/src/remove/observation/mod.rs
  - worktree/lib/src/remove/observation/injected.rs
  - worktree/lib/src/remove/observation/inotify.rs
  - worktree/lib/src/remove/observation/inotify/tests.rs
  - worktree/lib/src/remove/observation/tests.rs
docs_updated_during_phase_6:
  - docs/dependencies.md
  - worktree/docs/cli/remove.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/worktree/SKILL.md
  - .claude/skills/worktree/remove.md
  - .claude/skills/worktree/observation.md
source_files_during_phase_7:
  - worktree/lib/Cargo.toml
  - Cargo.lock
  - .config/nextest.toml
  - worktree/lib/src/remove/observation/mod.rs
  - worktree/lib/src/remove/observation/tests.rs
  - worktree/lib/src/remove/observation/fsevents.rs
  - worktree/lib/src/remove/observation/fsevents/tests.rs
docs_updated_during_phase_7:
  - docs/dependencies.md
  - worktree/docs/cli/remove.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/worktree/SKILL.md
  - .claude/skills/worktree/observation.md
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/macos.md
source_files_during_phase_8:
  - worktree/lib/Cargo.toml
  - worktree/lib/src/remove/observation/mod.rs
  - worktree/lib/src/remove/observation/tests.rs
  - worktree/lib/src/remove/observation/read_directory_changes.rs
  - worktree/lib/src/remove/observation/read_directory_changes/tests.rs
docs_updated_during_phase_8:
  - docs/dependencies.md
  - worktree/docs/cli/remove.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/windows.md
  - .claude/skills/worktree/SKILL.md
  - .claude/skills/worktree/observation.md
source_files_during_phase_9:
  - .config/nextest.toml
  - worktree/lib/src/remove/mod.rs
  - worktree/lib/src/remove/inventory.rs
  - worktree/lib/src/remove/missing.rs
  - worktree/lib/src/remove/outcome.rs
  - worktree/lib/src/remove/test_support.rs
  - worktree/lib/src/remove/observation/mod.rs
  - worktree/lib/src/remove/observation/injected.rs
  - worktree/lib/src/remove/observation/inventory.rs
  - worktree/lib/src/remove/observation/inventory/tests.rs
  - worktree/lib/src/remove/observation/deletion.rs
  - worktree/lib/src/remove/observation/deletion/tests.rs
  - worktree/lib/src/remove/quarantine/mod.rs
  - worktree/lib/src/remove/quarantine/rename.rs
  - worktree/lib/src/remove/quarantine/unregister.rs
  - worktree/lib/src/remove/quarantine/unregister/tests.rs
  - worktree/lib/src/remove/recovery/mod.rs
  - worktree/lib/src/remove/recovery/journal.rs
  - worktree/lib/src/remove/recovery/store.rs
  - worktree/lib/src/remove/recovery/store/tests.rs
docs_updated_during_phase_9:
  - worktree/docs/cli/remove.md
docs_created_during_phase_9: []
skills_files_updated_during_phase_9:
  - .claude/skills/worktree/SKILL.md
  - .claude/skills/worktree/observation.md
  - .claude/skills/worktree/remove.md
  - .claude/skills/os/macos.md
source_files_during_phase_10:
  - worktree/lib/src/remove/mod.rs
  - worktree/lib/src/remove/lifecycle.rs
  - worktree/lib/src/remove/lifecycle/resume.rs
  - worktree/lib/src/remove/lifecycle/resolve.rs
  - worktree/lib/src/remove/lifecycle/tests.rs
  - worktree/lib/src/remove/recovery/mod.rs
  - worktree/lib/src/remove/recovery/store.rs
  - worktree/lib/src/remove/quarantine/association.rs
  - worktree/lib/src/remove/observation/mod.rs
  - worktree/lib/src/remove/observation/injected.rs
  - worktree/lib/src/remove/observation/inotify.rs
  - worktree/lib/src/remove/observation/inotify/tests.rs
  - worktree/lib/src/remove/observation/fsevents.rs
  - worktree/lib/src/remove/observation/fsevents/tests.rs
  - worktree/lib/src/remove/observation/read_directory_changes.rs
  - worktree/lib/src/remove/observation/read_directory_changes/tests.rs
docs_updated_during_phase_10:
  - worktree/docs/cli/remove.md
docs_created_during_phase_10: []
skills_files_updated_during_phase_10:
  - .claude/skills/worktree/SKILL.md
  - .claude/skills/worktree/lifecycle.md
  - .claude/skills/worktree/observation.md
  - .claude/skills/worktree/remove.md
source_files_during_phase_11:
  - worktree/lib/src/error.rs
  - worktree/lib/src/remove/mod.rs
  - worktree/cli/src/main.rs
  - worktree/cli/src/exit.rs
  - worktree/cli/src/commands/remove/mod.rs
  - worktree/cli/src/commands/remove/recovery.rs
  - worktree/cli/src/commands/remove/report.rs
  - worktree/cli/src/commands/remove/usage.rs
  - worktree/cli/tests/remove.rs
  - worktree/cli/tests/remove_recovery.rs
  - worktree/cli/tests/remove_recovery_report.rs
  - worktree/cli/tests/powershell_wrapper_exec.rs
  - worktree/cli/tests/level2_powershell_remove.rs
  - worktree/cli/tests/snapshots/remove_recovery_report__a_branch_failure_after_the_checkout_was_removed.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_name_matching_a_pending_removal_and_a_live_worktree.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__long_lists_are_counted_and_uncertain_entries_named.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__refusals_say_nothing_was_removed_and_keep_their_exit_codes.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__the_specs_example.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__untrusted_text_is_shown_literally.snap
docs_updated_during_phase_11:
  - worktree/docs/cli/remove.md
  - worktree/README.md
docs_created_during_phase_11: []
skills_files_updated_during_phase_11:
  - .claude/skills/worktree/lifecycle.md
  - .claude/skills/worktree/remove.md
  - .claude/skills/worktree/cli-contracts.md
  - .claude/skills/worktree/testing.md
  - .claude/skills/os/windows.md
  - .claude/skills/os/SKILL.md
source_files_during_phase_12: []
docs_updated_during_phase_12:
  - worktree/docs/cli/remove.md
  - worktree/README.md
  - docs/dependencies.md
docs_created_during_phase_12: []
skills_files_updated_during_phase_12:
  - .claude/skills/worktree/lifecycle.md
  - .claude/skills/os/linux.md
source_code:
  - worktree/lib/Cargo.toml
  - Cargo.lock
  - worktree/lib/src/remove/mod.rs
  - worktree/lib/src/remove/handoff.rs
  - worktree/lib/src/remove/outcome.rs
  - worktree/lib/src/remove/recovery/mod.rs
  - worktree/lib/src/remove/recovery/record.rs
  - worktree/lib/src/remove/recovery/journal.rs
  - worktree/lib/src/remove/recovery/lock.rs
  - worktree/lib/src/remove/recovery/storage.rs
  - worktree/lib/src/remove/recovery/store.rs
  - worktree/lib/src/remove/recovery/store/tests.rs
  - worktree/lib/src/remove/usage.rs
  - worktree/lib/src/remove/usage/tests.rs
  - worktree/cli/src/commands/remove/mod.rs
  - worktree/cli/src/commands/remove/usage.rs
  - worktree/cli/tests/remove_usage_refusal.rs
  - worktree/cli/tests/snapshots/remove_usage_refusal__the_specs_example.snap
  - worktree/cli/tests/snapshots/remove_usage_refusal__every_evidence_location_and_holder_shape.snap
  - worktree/cli/tests/snapshots/remove_usage_refusal__untrusted_text_is_shown_literally.snap
  - worktree/cli/tests/snapshots/remove_usage_refusal__long_lists_are_counted.snap
  - worktree/cli/tests/snapshots/remove_usage_refusal__coverage_notes.snap
  - worktree/cli/tests/snapshots/remove_usage_refusal__an_unverified_checkout_refuses.snap
  - worktree/lib/src/remove/quarantine/mod.rs
  - worktree/lib/src/remove/quarantine/rename.rs
  - worktree/lib/src/remove/quarantine/rename/tests.rs
  - worktree/lib/src/remove/admin_entry.rs
  - worktree/lib/src/remove/quarantine/naming.rs
  - worktree/lib/src/remove/quarantine/association.rs
  - worktree/lib/src/remove/quarantine/association/tests.rs
  - worktree/lib/src/remove/observation/mod.rs
  - worktree/lib/src/remove/observation/injected.rs
  - worktree/lib/src/remove/observation/inotify.rs
  - worktree/lib/src/remove/observation/inotify/tests.rs
  - worktree/lib/src/remove/observation/tests.rs
  - .config/nextest.toml
  - worktree/lib/src/remove/observation/fsevents.rs
  - worktree/lib/src/remove/observation/fsevents/tests.rs
  - worktree/lib/src/remove/observation/read_directory_changes.rs
  - worktree/lib/src/remove/observation/read_directory_changes/tests.rs
  - worktree/lib/src/remove/inventory.rs
  - worktree/lib/src/remove/missing.rs
  - worktree/lib/src/remove/test_support.rs
  - worktree/lib/src/remove/observation/inventory.rs
  - worktree/lib/src/remove/observation/inventory/tests.rs
  - worktree/lib/src/remove/observation/deletion.rs
  - worktree/lib/src/remove/observation/deletion/tests.rs
  - worktree/lib/src/remove/quarantine/unregister.rs
  - worktree/lib/src/remove/quarantine/unregister/tests.rs
  - worktree/lib/src/remove/lifecycle.rs
  - worktree/lib/src/remove/lifecycle/resume.rs
  - worktree/lib/src/remove/lifecycle/resolve.rs
  - worktree/lib/src/remove/lifecycle/tests.rs
  - worktree/lib/src/error.rs
  - worktree/cli/src/main.rs
  - worktree/cli/src/exit.rs
  - worktree/cli/src/commands/remove/recovery.rs
  - worktree/cli/src/commands/remove/report.rs
  - worktree/cli/tests/remove.rs
  - worktree/cli/tests/remove_recovery.rs
  - worktree/cli/tests/remove_recovery_report.rs
  - worktree/cli/tests/powershell_wrapper_exec.rs
  - worktree/cli/tests/level2_powershell_remove.rs
  - worktree/cli/tests/snapshots/remove_recovery_report__a_branch_failure_after_the_checkout_was_removed.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_name_matching_a_pending_removal_and_a_live_worktree.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__long_lists_are_counted_and_uncertain_entries_named.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__refusals_say_nothing_was_removed_and_keep_their_exit_codes.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__the_specs_example.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__untrusted_text_is_shown_literally.snap
documentation:
  - worktree/fixes/2026-10-05-graceful-file-contention/spikes/README.md
  - docs/dependencies.md
  - worktree/docs/cli/remove.md
  - worktree/README.md
completed_phase: 12
implemented: true
packages:
  - worktree
  - worktree-cli
---

# Plan: Graceful removal under filesystem contention

Spec: `2026-10-05-graceful-file-contention` (`spec.md`, same directory). Depends
on `2026-10-05-filesystem-watchers` (implemented: `sniff::filesystem::query::query_path_usage`
with the shared two-second budget, typed `PathUsageError`, coverage statuses,
lossless `NativeString` paths).

## Summary and definition of success

### Work required

Replace `wt remove`'s single `git worktree remove` call for existing linked
checkouts with a contention-aware lifecycle in the Worktree library:

1. **Preflight usage discovery (refusal):** a fresh
   `sniff::filesystem::query::query_path_usage` after the existing guards,
   preparation, questions, and final identity/fingerprint checks, and — under
   the shell wrapper — in the second invocation after the handoff. Any retained
   `watch_registration` / `open_handle` / `working_directory` / `loaded_module`
   evidence (except this operation's own registered handles) refuses with exit
   3, names PIDs and evidence, and no force flag bypasses it. No matches
   continues even when discovery is partial, unavailable, or unsupported;
   coverage is retained in the operation report. Root-validation errors refuse
   before mutation. Missing-directory removal never queries Sniff.
2. **Durable preparation:** a recovery record and deletion journal under
   `<common-git-dir>/wt-removal/<operation-id>/`, written atomically with
   owner-restricted permissions, guarded by an OS-backed lock (fs4) keyed by
   repository/admin identity, validated on read without following links, and
   versioned. Every mutation's intent is persisted before it and its observed
   result after it.
3. **Quarantine move with association retained:** a collision-resistant hidden
   sibling on the same filesystem, moved with an atomic no-replacement rename
   primitive per platform (never plain `std::fs::rename`, which may replace).
   Git stays associated with the moved checkout (checked, narrowly scoped link
   updates; no broad `git worktree repair`; submodules and unsupported
   topologies refuse before mutation). The Windows rename-and-back probe is
   removed — the recorded quarantine move is the probe; a native sharing
   failure before a successful move keeps exit 4.
4. **Activity observation and checked deletion:** local watchers (parent,
   checkout/quarantine, admin identity/index files) plus one operation-start
   inventory (per-entry native identity, type, metadata, link target; content
   hashes for consent-sensitive paths via the existing fingerprint/biscuit-hash
   machinery) drive a children-first, batch-bounded, interruptible deletion
   that stops at entry boundaries on new, changed, replaced, or unexplainedly
   vanished entries, on original-path recreation, or on lost coverage.
   Unavailable local observation is an operational error before mutation.
5. **Completion, outcomes, and recovery:** typed outcomes (`usage_refused`,
   `removed`, `activity_detected`, `cleanup_failed`, `state_unverified`)
   separate from the exit-code mapping in the spec's table (a stop after the
   move is exit 1). Branch and remote actions run only after verified checkout
   removal. Retry by the original name resolves against the union of live
   registrations and pending records, refuses ambiguity, revalidates identity,
   and re-collects consent; a pending record never authorizes deleting new or
   recreated content.
6. **CLI reporting:** `biscuit-terminal` `Prose`-based refusal, partial, and
   recovery reports on stderr with both locations and the saved report path;
   stdout keeps only the wrapper protocol; the existing questions/handoff
   coordination is kept.

The work stays inside `worktree/lib/src/remove/` (new modules proposed:
`usage.rs`, `outcome.rs`, `recovery/`, `quarantine/`, `observation/`,
`lifecycle.rs`), `worktree/cli/src/commands/remove/`, the area's docs/skills,
and tests. Sniff is consumed as-is; no `sniff` CLI spawn, no OS process
discovery duplicated. New dependencies are limited to what R4-R6 select
(`inotify` and `fsevent-sys` for the R4 adapters, `libc` and
`windows-sys` for rename and Windows watch bindings) and must be
recorded in dependency docs.

### Done means

- [x] `just test`, `just test-l2`, and `just lint` pass in the worktree area;
      cross-OS evidence for macOS, Linux, native Windows, and WSL2 is recorded
      per the `os` skill, and `just ci-local --plan` shows no unexpected scope.
- [x] Every acceptance item (1-12) in the spec maps to a named test (see the
      traceability table written in Phase 12).
- [x] Controlled writers reproduce retained-cwd writes into the quarantine and
      absolute-path recreation at the original; both stop with preserved
      remnants, honest partial-deletion reports, and no branch operations.
- [x] Crash injection at each phase boundary leaves a recoverable record; retry
      by the original name works after unregistering and rejects changed
      identities and new content; completed records never authorize a second
      removal.
- [x] Existing protections (branch safety tiers, expected-tip, consent
      fingerprints, protected includes, handoff records, locked worktrees,
      missing-directory record-only removal, remote endpoints) are retained and
      still tested; exits 3 and 4 still mean "nothing was removed".
- [x] Structural checks hold: no per-process repeated checkout walks, no
      whole-tree rescan per deleted entry, no unconditional hashing of ignored
      file bodies, no scanning threads left running after a query returns.
- [x] `worktree/docs/cli/remove.md` describes current behavior with the
      **planned** markers removed only by this change; dependencies docs,
      README, worktree/OS skills, and the implementation log are updated;
      departures from the spec are recorded in the log.
- [x] Agent terminal state is "implementation complete, ready for review"; the
      fix directory is not moved to `_completed` and `just complete` is not run.

### Waves

Waves are numbered globally. Tasks inside one wave are independent and can be
given to concurrent subagents; a wave starts only after the previous wave's
validation checkpoint passes.

| Wave | Phase | Contents |
| --- | --- | --- |
| 1 | 1 | Rulings recorded; spikes S1 (three hosts), S2 (three hosts), S3 run in parallel |
| 2 | 2 | Typed outcomes; record schema + strict reader; journal format (parallel) |
| 3 | 2 | Record store (atomic writes, permissions, lock); journal writer; robustness-matrix tests |
| 4 | 3 | Library usage module + typed refusal; CLI refusal rendering (parallel) |
| 5 | 4 | No-replacement rename backends (per OS, parallel) |
| 6 | 5 | Quarantine naming; association update + rollback; submodule/lock refusals |
| 7 | 6 | Observer type; injected observer; Linux inotify adapter |
| 8 | 7, 8 | macOS FSEvents adapter; Windows ReadDirectoryChangesW adapter (concurrent) |
| 9 | 9 | Deletion inventory (journal v2); checked deletion engine; final verification |
| 10 | 10 | Lifecycle orchestration; crash recovery and retry resolution |
| 11 | 11 | CLI partial/recovery reporting, retry wiring, channel discipline |
| 12 | 12 | Docs, skills, dependency records, cross-OS validation, CI scope review |

Phases were renumbered on 2026-10-12 from 8 to 12 (old 4 became 4-5, old 5
became 6-9, old 6/7/8 became 10/11/12). Phase 1-2 text in the
implementation log uses the old numbers.

## Phase 1: Rulings, spikes, and scaffolding decisions

Goal: remove the decisions and unknowns that would otherwise force rework,
before any contract or backend code is written.

### Necessary Rules

The spec leaves these unresolved or only recommends an answer. Each needs the
marked ruling before the phase it gates starts; outcomes and author overrides
are recorded in `implementation-log.md` and the spec file stays unchanged.

- [x] **R1 - Git association strategy (author decision, confirmed 2026-10-12; gates Phases 4-10).**
      The spec's first Open Question. This plan proceeds on the recommended
      **Option 1: controlled deletion with registration retained** — a move
      backend that enforces no replacement and verifies the same admin links
      (`git worktree move` only where S3 shows it satisfies the contract,
      otherwise native rename plus narrowly scoped, checked link updates),
      deletion of approved content in Worktree preserving `.git` until the
      last stage, and a targeted, index-reconfirmed `git worktree remove`
      (never repeated `--force`) once no working content remains.
      Consequences to disclose in docs: some formerly removable submodule
      checkouts initially refuse; the final unregister step carries the
      identity/recreation/index checks. Options 2 and 3 weaken the contract
      and are rejected unless the author overrides.
- [x] **R2 - Local observation strategy (author decision, confirmed 2026-10-12; gates Phases 6-9).**
      The spec's second Open Question. Proceed on the recommended
      **Option 1: native notifications plus one inventory and targeted
      checks**. Unusable local observation refuses before mutation as an
      operational error (this differs from advisory Sniff discovery, which
      permits continuation). Options 2 (polling) and 3 (repeated hashing)
      reduce protection or re-read ignored build output and are rejected
      unless the author overrides.
- [x] **R3 - Sniff latency contract (author confirmation, given 2026-10-12; gates Phase 3).**
      `wt` adopts `query_path_usage` under the dependent spec's finalized
      Option 1: the two-second budget stops scheduling work; a native call
      already running may overrun it; no scanning threads outlive the query.
      The fix spec already accepts this subject to that API decision, which
      landed as Option 1 — confirm and record. Typed `PathUsageError`s are
      matched by variant, never by message text.
- [x] **R4 - Watcher adapter (planning ruling from spike S2, confirmed 2026-10-12; gates Phases 6-8).**
      Proposed default: the maintained portable adapter `notify` 8 with its
      per-backend capabilities recorded, or focused native adapters where
      `notify` cannot surface overflow/lost-watch or rename invalidation.
      Constraints: bounded event queue, explicit overflow/loss signaling,
      watch invalidation on rename must be observable, no thread left behind
      after the session ends. Decision, version, license, and native build
      needs go to `worktree/docs/dependencies.md`.
- [x] **R5 - No-replacement rename primitive (planning ruling from spike S1;
      gates Phase 4).** Proposed: Linux `renameat2(RENAME_NOREPLACE)`, macOS
      `renamex_np(RENAME_EXCL)`, Windows `NtSetInformationFile`
      `FileRenameInformation` with `ReplaceIfExists = FALSE` (fall back to
      `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING` only if S1 shows it
      is error-correct on collision). `std::fs::rename` is unusable (its
      contract permits destination replacement). Error kinds map to: sharing
      failure (Windows) → exit 4 path; collision/permission/other I/O →
      operational error, never evidence of another process's usage.
- [x] **R6 - `git worktree move` eligibility (planning ruling from spike S3;
      gates Phase 4).** Use it only where it enforces no replacement and
      leaves unrelated records alone; otherwise native rename plus checked
      updates of exactly the target's own links (admin `gitdir` back-reference
      and the checkout's `.git` file, honoring Git's relative-link format).
      Broad `git worktree repair` is forbidden (the worktree skill records it
      rewriting unrelated worktree links). Record the minimum Git version the
      behavior was verified against.
- [x] **R7 - Record and journal details (planning ruling; gates Phase 2).**
      Operation ID: 128-bit `getrandom` token, hex/base32-rendered (existing
      dependency). Storage: `<common-git-dir>/wt-removal/<operation-id>/`
      containing the record, `report.json`, and the journal; owner-restricted
      permissions (0o700/0o600 on Unix; on Windows the `.git`-inherited
      user-scoped ACL plus `FILE_ATTRIBUTE_HIDDEN` on the quarantine sibling).
      Persistence primitive: temp-file + atomic rename with `fsync` of file
      and directory at batch boundaries; journal intents fsynced before a
      batch, confirmed results after; the crash window between an unlink and
      its result persistence is labeled `uncertain` on recovery. Completed
      operations keep `report.json` and delete the journal. Deletion batch
      bound: one directory or 256 entries, whichever is smaller (tunable,
      recorded in the log). A stale lock file is never treated as an active
      operation; only the held OS lock is.
- [x] **R8 - Disposition of the existing checks (planning ruling; gates
      Phase 3).** The early `check_no_processes` in `run` stays as a
      preliminary, question-sparing check; the decisive usage query is the
      fresh Sniff query inside the operation, immediately before the
      quarantine mutation. The `check_no_processes` + Windows
      rename-and-back probe inside `remove_worktree` are replaced by the new
      preflight and the recorded quarantine move (spec-mandated), so a probe
      can no longer leave an unrecorded directory behind.
- [x] **R9 - Exit codes and typed outcomes (confirmed by the spec; no open
      decision).** The spec's exit table is implemented verbatim: usage or
      changed approved state before mutation → 3; missing wrapper, invalid
      handoff, or Windows sharing failure before move → 4; anything after the
      move (activity, lost coverage, I/O, uncertain state) → 1 even when no
      file was deleted; checkout-removed-but-branch-failed → 1; verified
      completion or pre-mutation cancellation → 0. The typed outcome is
      preserved separately from the exit code.
- [x] **R10 - No performance spike (author ruling recorded, not scheduled).**
      The spec adds no performance spike or timing gate, and its own figures
      answer the timing questions (two-second shared budget; ~6 ms per Windows
      file-side query already measured in the dependent feature's log). Any
      wider measurement (deletion throughput on very large trees, watcher
      overhead) is the author's separate decision; none is scheduled here.
- [x] **R11 - CI scope (planning ruling; verified in Phase 12).** All new tests
      are self-contained in temp directories with fixture-owned child
      processes and stand-in remotes; no `[package.metadata.ci.tests]
      source-inputs`, no prompts, no terminal focus. Verify with
      `just ci-local --plan` before pushing.
- [x] **R12 - Self-exclusion mechanics (planning ruling; gates Phases 3 and 6).**
      Exclusions are matched by specifically identified operation-owned
      handles (the current `wt` process's own watcher/session handles,
      registered when created) using PID plus process-lifetime identity where
      Sniff supplies it — never by excluding the shell, ancestors, a process
      group, or every record belonging to the current PID. Sniff already
      omits its detector's own handles; `wt` moved its cwd to the base before
      the query, which the existing `run` flow guarantees.

### Spikes

Each runs once, before the work it informs. None is a performance spike
(R10): each answers a capability or semantics question the spec explicitly
leaves to the plan. Output: a short finding in `implementation-log.md` and the
ruling it feeds (R4, R5, R6). Hosts follow the `os` skill.

- [x] **S1 - Atomic no-replacement rename (hosts: macOS local, native Windows
      build host, Linux or WSL2).** Question: does the proposed primitive
      (R5) fail without replacing an existing destination, preserve directory
      identity across the rename, return distinguishable error kinds for
      collision / cross-device / sharing violation, and need no special
      privileges? Gates R5 and Phase 4.
- [x] **S2 - Watcher adapter capabilities (hosts: macOS, native Windows,
      Linux or WSL2).** Question: with the candidate adapter, what events
      arrive when a watched directory is renamed away; how are queue
      overflow and lost watches surfaced; can the admin dir's `index` and
      `sharedindex.*` be watched alongside the tree; what happens at
      platform watch limits; what is the bounded-queue behavior? Gates R4 and
      Phases 6-8.
- [x] **S3 - Git move and final-unregister semantics (host: macOS, current
      Git).** Question: does `git worktree move` satisfy the no-replacement
      and scoped-update contract (which files it rewrites, how it refuses
      existing destinations and submodules); is the manual equivalence a
      checked rewrite of exactly the admin `gitdir` back-reference (and when
      the `.git` file's format makes it move-sensitive); does plain
      `git worktree remove` (no `--force`) succeed on a quarantine whose
      tree is empty except `.git`; which admin-dir files (split index, entry
      flags) must the index binding cover? Gates R6, Phases 4 and 6.

### Tasks

- [x] **Record rulings** - Write R1-R12 outcomes, author confirmations, and
      any overrides into `implementation-log.md`; mark unresolved author
      rulings (R1-R3) as "proceeding on the spec's recommendation pending
      confirmation".
- [x] **Run spikes** - S1 (three hosts), S2 (three hosts), and S3 run
      concurrently as Wave 1; each records its finding and the resulting
      ruling; failures to reach a host are reported, not skipped silently.
- [x] **Load skills** - Implementers load `worktree`, `rust-testing`, `os`,
      `rust`, and (for CLI waves) `biscuit-terminal` before their wave;
      subagent briefs repeat that the session tree is non-interactive.

Checkpoint: R1-R3 answered or explicitly accepted as "proceeding on
recommendation"; R4-R6 decided by S1-S3 and logged. Do not start Phases 4-9
work without them.

## Phase 2: Recovery records, journal, and typed outcomes (library contracts)

Module: `worktree/lib/src/remove/` — new `outcome.rs` and `recovery/`
(`record.rs`, `journal.rs`, `lock.rs`, `store.rs`). No mutation of checkouts
yet; everything is exercisable against temp directories.

### Wave 2 (parallel)

- [x] **Typed outcomes** - `outcome.rs`: the operation-level outcome enum
      (`usage_refused`, `removed`, `activity_detected`, `cleanup_failed`,
      `state_unverified`), the moved/deleted/retained state description, and
      the operation report model (timeline, discovery evidence with coverage,
      observed activity, journal summary, Git registration state, branch
      outcomes, errors, truncation flags). Library result types only — no new
      user-facing JSON CLI option.
      - Complexity: the report doubles as the persisted `report.json`; keep
        the serde model versioned from the start.
- [x] **Record schema and reader** - `recovery/record.rs`: serde model binding
      operation ID, original/quarantine native paths and identity,
      repository/admin identity, branch and approved tip, consent fingerprint,
      discovery report, requested name and aliases, intended actions, record
      version, and completed phase. Paths reuse Sniff's lossless
      `NativeString`. Strict reader: reject unknown versions, malformed
      native paths, wrong ownership, repository/admin identity mismatch,
      duplicate JSON keys, and trailing content; validate the storage
      directory without following replacement links.
      - Prerequisite: R7.
- [x] **Journal format** - `recovery/journal.rs`: the entry model
      distinguishing planned, confirmed operation-owned deletions, externally
      absent, and uncertain attempted deletions, with types (file, directory,
      link) and directory-entry-deletion vs bytes-destroyed wording retained
      for reports; batch-intent records; storage-limit behavior that stops
      deletion instead of discarding confirmed entries.

### Wave 3 (parallel; depends on Wave 2 types)

- [x] **Record store and lock** - `recovery/store.rs` + `lock.rs`: atomic
      write with owner-restricted permissions and fsync (R7's primitive,
      crash guarantees documented at the module level); OS-backed exclusive
      lock via the existing `fs4` dependency keyed by repository/admin
      identity, held for the operation's duration; pending-record detection;
      completion persistence and journal clearing with the bookkeeping-warning
      path when clearing fails.
- [x] **Journal writer** - Persistence of batch intent before unlinking and
      confirmed results afterward per R7's fsync points; the crash window
      between unlink and result persistence recovers as `uncertain`; append
      and fsync behaviors tested against injected write failures.
- [x] **Robustness matrix tests** - One test per format (record, journal)
      walks the matrix below from a real fixture written by the code's own
      writer, one edit per cell, asserting through the public
      validate/recover outcome (never the parser's return value), plus a
      control row proving the unedited fixture validates.

### Input robustness matrix

The code reads two configuration-like formats it also writes: the recovery
record and the deletion journal (restart/retry inputs). Load-bearing fields
and the required outcome per shape — the outcome is always a typed
`RecordInvalid`/journal-unreadable refusal (operation refuses to resume;
remnants and report preserved) unless the cell says otherwise:

| Shape | `record_version` | `operation_id` | paths (`original`/`quarantine`) | identity fields (native, repo/admin) | `branch`/`approved_tip`/`consent_fingerprint` | `completed_phase` | journal entry `path` | journal entry `state` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| absent | invalid | invalid | invalid | invalid | invalid | invalid (never trusted as proof anyway) | invalid | invalid |
| explicit null | invalid (not conflated with absent) | invalid | invalid | invalid | invalid | invalid | invalid | invalid |
| wrong type, whole field | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid |
| wrong type, one element | invalid (no silent filtering) | n/a | n/a | n/a | n/a | n/a | invalid | n/a |
| wrong type, every element | invalid, never read as empty | n/a | n/a | n/a | n/a | n/a | invalid | n/a |
| empty (`[]`/`{}`) | n/a | n/a | invalid (a path is never an empty collection) | invalid | invalid | n/a | valid: empty journal = nothing attempted | n/a |
| duplicate key | invalid (strict reader rejects last-wins) | invalid | invalid | invalid | invalid | invalid | invalid (jsonl: duplicate keys in one line) | invalid |
| trailing or invalid content | invalid | invalid | invalid | invalid | invalid | invalid | invalid (a partial trailing line is `uncertain`, per the crash window) | invalid |

- `aliases` is the one field where empty and absent are distinct and both
  valid (no aliases recorded vs the field omitted by an older writer is still
  invalid; the version gate carries compatibility).
- Before declaring this done, grep for the smells: `#[serde(default)]` on a
  load-bearing field, `Option<T>` where absent and null must differ, and
  `.ok()`/`unwrap_or_default()` on a load-bearing parse.
- Review discipline: a finding about one cell must list the state of every
  other cell in the same row and column.

Checkpoint: `just test` and `just lint` pass; serde round-trip and matrix
tests green on macOS; record/journal code compiles for all targets
(`cargo check` per available target).

## Phase 3: Usage discovery and preflight refusal

### Wave 4 (parallel)

- [x] **Library usage module** - `remove/usage.rs`: wrap
  `sniff::filesystem::query::query_path_usage` with default options (shared
  two-second budget, no new timeout flag or automatic retry), an
  operation-handle registry for self-exclusion (R12), and mapping to a typed
  `UsageRefusal` carrying PID, available name/executable, exact matched path,
  evidence kind, and coverage limitations (several in-scope aliases or
  native-only identity reported as facts, no invented path, no name attached
  to a reused PID). Typed `PathUsageError`s (root validation, invalid
  options, missing/unsupported target) refuse before mutation; `usable`,
  `unavailable`, and `unsupported` reports with no matches all continue,
  with coverage retained for the operation report. Injection seam (like the
  existing `PrSource` pattern) so policy tests never depend on host
  watchers.
      - Prerequisites: R3, R12; the decisive call site lands in Phase 10's
        orchestration, behind this module's seam from the start.
- [x] **CLI refusal rendering** - In `cli/src/commands/remove/`: render the
      typed refusal through `biscuit-terminal` `Prose` per the spec's example
      (concise preamble, per-PID evidence lines, rerun command with quoted
      arguments, "checkout and branches were kept", coverage note such as
      "Discovery is partial; FSEvents subscriptions cannot be enumerated on
      macOS"), all on stderr; untrusted names/paths escaped; exit 3; no
      force flag bypass; no routine ignored-file list. Unit-test against
      injected reports.
      - Prerequisite: Wave 4's library types (develop against the injected
        seam in parallel).

Checkpoint: library tests cover match/no-match/partial/unsupported/
root-error × exclusion rules; CLI rendering snapshot-tested (in an
integration test, per the double-target trap in the worktree skill); exits 3
and 4 still remove nothing.

## Phase 4: No-replacement rename backends

Small and self-contained: one FFI surface per OS behind one function. Nothing
here touches Git.

### Wave 5 (per-OS backends in parallel)

- [x] **Rename backends** - `remove/quarantine/rename.rs` with per-OS
      primitives (R5): Linux `renameat2(RENAME_NOREPLACE)`, macOS
      `renamex_np(RENAME_EXCL)`, Windows `SetFileInformationByHandle`
      (`FileRenameInfo`, `ReplaceIfExists = FALSE`) after a `DELETE` open.
      Typed error kinds: collision, unsupported (`EINVAL`, e.g. WSL drvfs),
      cross-device, permission, Windows sharing (5/32 from the rename after a
      successful open; 5/32 from the open is permission), other I/O. Windows
      directory identity from `GetFileInformationByHandleEx(FileIdInfo)`.
      Adds `windows-sys` 0.61 (workspace version) for Windows.
- [x] **Tests** - Per OS: an existing destination (empty directory, file,
      non-empty directory, dangling link) is never replaced; a failed rename
      leaves the original intact; identity survives the move; each error kind
      maps as above. Windows: an open file in the subtree yields the sharing
      kind.

Checkpoint: rename tests green on macOS locally and on Linux and native
Windows via `just cross-check worktree --os <os> quarantine` (substring
filters, not `-E`); WSL2 runs the Linux code (drvfs `EINVAL` covered by an
injected error). `just test` and `just lint` pass.

## Phase 5: Quarantine naming, Git association, and rollback

### Wave 6

- [x] **Quarantine naming** - Collision-resistant hidden sibling selection
      (`.wt-remove-<slug>-<token>` beside the target, same depth;
      `FILE_ATTRIBUTE_HIDDEN` on Windows per R7); same filesystem by
      construction; never a global temp dir; never copy-and-delete; the
      destination is created only by the Phase 4 move itself.
- [x] **Association update and rollback** - Under R1/R6: re-list and validate
      lock status and admin/checkout links immediately before moving; perform
      the move; rewrite exactly the admin `gitdir` in the record's existing
      format (absolute, or relative to the admin directory itself), verified
      by reading it back with Git; leave the `.git` file unchanged and refuse
      a record whose `.git` file would need rewriting; never unlock a locked
      worktree, never pass `--force`; refuse submodule checkouts and
      unsupported topologies before any mutation (exit 3). After a successful
      rename but failed association update: verified rollback only when the
      original path is still absent, no destructive work occurred, and no
      unexpected activity was observed; otherwise preserve the quarantine and
      report actual state. Unsupported rename (drvfs) is exit 4 before
      mutation; the Windows sharing kind is exit 4; collision, permission,
      and other I/O are operational errors never labeled as process usage.

Checkpoint: association tests show Git reading the quarantine under its
retained registration (absolute and relative link formats, Git 2.47 and
current), `prune --dry-run` leaving it alone, submodule and locked-worktree
refusals with nothing changed, and rollback rules holding; acceptance item
5's rename collision/failure cases pass.

## Phase 6: Observer type, injected observer, and Linux inotify adapter

Fixes the observer contract that Phases 7-9 build on.

### Wave 7

- [x] **Observer type** - `remove/observation/mod.rs`: one internal observer
      type per R4 with: watch the original's parent (recreation), the
      checkout/quarantine (descendant activity), and the admin files behind
      the target's identity and approved index (`index`, `index.lock`,
      `sharedindex.*`, `HEAD`, `gitdir`, `locked`); `relabel` when the
      operation reports its own move; `drain` (a barrier: every event
      caused before the call has been delivered); a bounded queue fed by a
      never-blocking handler (a full queue is lost coverage); events carrying
      receipt times; lost coverage (overflow, lost watch, setup failure) as a
      typed signal, never a silent downgrade; handle registration with Phase
      3's exclusion registry; no thread left after the session ends.
      Unestablishable observation is exit 4 before mutation.
- [x] **Injected observer** - A test observer driven by the test (events,
      overflow, lost watch, delayed delivery) so Phase 9's engine and the
      policy tests never depend on host timing.
- [x] **Linux adapter** - inotify via the `inotify` 0.11 crate (ISC): parent
      non-recursive, checkout recursive, admin directory; watch descriptors
      follow inodes across the move (relabel, no gap); `IN_Q_OVERFLOW` and an
      unexpected `IN_IGNORED` are lost coverage; drain is a marker per root
      read until `EAGAIN`; `EMFILE` at the instance limit is a setup failure.

Checkpoint: observer contract tests run against the injected observer and
the Linux adapter (on `build-linux` and WSL2); `just test` and `just lint`
pass on macOS (the adapter is `cfg`-gated, the contract compiles everywhere).

## Phase 7: macOS FSEvents adapter

### Wave 8 (Phases 7 and 8 may run concurrently)

- [x] **macOS adapter** - FSEvents via `fsevent-sys` 4.1 (MIT): one stream
      covering the checkout and quarantine paths (path-based, so the move
      does not invalidate it), the parent, and the admin directory; nonzero
      latency with file events (not `NoDefer`); drain is
      `FSEventStreamFlushSync`, never a marker; `MustScanSubDirs` and either
      `Dropped` flag are lost coverage; event kind alone never classifies an
      operation-owned unlink (coalesced flags); the run-loop thread is stopped
      and joined when the session ends, and drop never blocks on a full
      queue. No self-exclusion entry is needed (fseventsd holds the watch).

Checkpoint: the Phase 6 contract tests pass against the FSEvents adapter on
macOS, including a 2,000-file burst that must surface as lost coverage or
deliver completely, never silently partial.

## Phase 8: Windows ReadDirectoryChangesW adapter

- [x] **Windows adapter** - `ReadDirectoryChangesW` via `windows-sys` 0.61:
      the parent with the subtree flag (one handle; S1 shows it does not block
      the move) and the admin directory; a large buffer on local volumes; a
      zero-byte completion, `ERROR_NOTIFY_ENUM_DIR`, or any completion error
      other than the session's own cancel is lost coverage; drain is a marker
      (FIFO per handle); the admin handle is closed before the final
      unregister. Verify whether an open watch handle leaves a deleted
      directory delete-pending, and record the answer in the `os` skill.

Checkpoint: the Phase 6 contract tests pass against the Windows adapter on
`build-win-native`, including the overflow case S2 showed `notify` hiding.

## Phase 9: Deletion inventory, checked deletion engine, and final verification

Built and tested against the injected observer first, then run against each
real adapter.

### Wave 9

- [x] **Deletion inventory** - `remove/observation/inventory.rs`: after
      observation is active and before the decisive usage query and the move,
      capture every entry approved for deletion (ignored content included,
      FIFOs/devices/nested mounts detected as ineligible, exit 3, rather than
      opened), per-entry native identity, type, relevant metadata, and link
      target; content hashes for the existing consent-sensitive fingerprint
      paths via the existing fingerprint/`biscuit-hash` machinery; never
      directory mtime or entry count alone; a finishing consistency check
      refuses before mutation if entries changed or cannot be inspected.
      Persist it with the journal before the move: add an inventory line,
      bump `JOURNAL_VERSION`, extend the reader's ordering rules, and add the
      line's fields to `journal_robustness_matrix`.
- [x] **Checked deletion engine** - `remove/observation/deletion.rs`:
      children-first, bounded batches (R7), cap-std parent-directory handles
      and relative operations (no following links; an approved link itself
      is removed, never its target), per-entry identity revalidation
      immediately before unlink, drain and inspect exact affected paths
      between batches and after the final batch, stop before deleting a
      newly appeared or detectably changed entry (modifications and
      replacements included), record an externally disappeared approved
      entry as already absent while an unexplained disappearance stops the
      operation, journal updates at each boundary, and `.git` deleted last.
      No arbitrary sleep as proof, no retry-until-the-writer-loses.
- [x] **Final verification** - `rmdir` of the quarantine root (fails if
      anything reappeared); both locations checked (quarantine absence, no
      original-path recreation through the final drain and identity checks);
      index bytes and Git's interpreted entries re-bound to the approved
      state; then `missing.rs` `remove_missing_record`. The report states the
      concurrent-write limit and identifies already deleted content.

Checkpoint: controlled-writer tests (child readiness handshakes, never
sleeps; guaranteed cleanup of children, handles, and watches on failure)
reproduce retained-cwd writes into the quarantine and absolute-path
recreation at the original, both stopping with preserved remnants and honest
reports, on macOS, Linux, and native Windows; overflow/invalidation injection
stops with lost coverage; structural checks (no whole-tree rescan per entry,
no unconditional hashing) hold; acceptance items 3 and 4 pass at library
level.

## Phase 10: Lifecycle orchestration, crash recovery, and retry

### Wave 10 (parallel tasks, integrated at the phase checkpoint)

- [x] **Operation lifecycle** - `remove/lifecycle.rs`: one flow binding the
      existing guards/preparation/questions/fingerprint checks, the fresh
      usage query (Phase 3), durable record creation (Phase 2), the move
      (Phases 4-5), checked deletion and final verification (Phases 6-9), the
      targeted unregister, branch and remote actions only after verified
      checkout completion (existing expected-tip and endpoint rules), copy
      record removal only after verification, completion persistence before
      clearing pending state, and interruption handling at entry boundaries.
      Order (spec decision 8): observation and inventory, then the decisive
      usage query excluding the operation's own watch handles, then the
      binding rechecks, `RecoveryStore::begin`, and the move. Typed outcomes
      feed the exit-code mapping (R9 plus spec decision 7: environment limits
      before mutation exit 4, ineligible content exit 3, exit 1 only after a
      move or with uncertain state) and the operation report.
      - Prerequisite: Phases 2-9.
- [x] **Crash recovery** - On any start, a pending record blocks a new
      operation on the same target; restart inspects actual state to resolve
      the interrupted phase and never treats a saved completed-phase value as
      proof that paths or Git still have that state; a crash after rename
      leaves enough information to find retained content without the original
      registration or directory name; recovery-record/journal write failures
      stop destructive work and report the last durable phase plus subsequent
      known or uncertain actions.
- [x] **Retry resolution** - Resolve the requested name against the union of
      live registrations and pending records, deduplicating only when both
      identify this same operation's checkout; the quarantine's registered
      basename never bypasses the pending record; a new live worktree reusing
      the old branch/name lists both and refuses as ambiguous; retry
      re-collects surviving content, include rules/baseline, index and branch
      state, obtains fresh consent through the existing questions/force
      flags, never reuses an expired handoff or saved remote approval, and
      never automatically deletes a recreated original (query it only as a
      validated real directory, never through a replacement link). A partial
      quarantine whose ordinary `git status` cannot run reports the verified
      inventory and administrative index without repairing missing working
      files; inability to resume safely is an honest recovery outcome.
      Run from the base checkout when the original no longer reaches the
      repository.

Checkpoint: crash-injection tests at each phase boundary (injected failure
seam, plus real process-termination tests) recover correctly; retry after
unregistering works by original name; changed identities, new content, and
ambiguous reuse refuse; completed records never authorize a second removal;
acceptance items 6, 7 (recovery half), 10, and 11 pass at library level.

## Phase 11: CLI reporting and recovery UX

### Wave 11

- [x] **Wire the operation** - `cli/src/commands/remove/`: `execute`'s
      healthy-checkout path calls the lifecycle instead of
      `remove_worktree`; the missing-directory path keeps record-only
      removal with no Sniff query; the decisive usage query sits after
      questions and final identity/fingerprint checks in `run`, and after
      handoff verification in the wrapper's second invocation (the first
      invocation reports, collects consent, and hands off without refusing
      on the parent shell's cwd); consent/identity bindings are rechecked
      after discovery and observation setup; `files_unchanged` semantics
      carry into the new flow.
- [x] **Partial and recovery reports** - Compact human reports per the
      spec's example: both locations, deleted-before-stopping counts with
      the saved report path, remaining content, observed activity with
      receipt times, Git registration state, branch state, "kept" vs
      "already removed" distinguished when a later step fails, recovery and
      retry guidance with shell-quoted commands, full report at
      `<repo>/.git/wt-removal/<id>/report.json`. All on stderr through
      `biscuit-terminal` components with escaped untrusted text; stdout
      carries only wrapper protocol lines; no `cd:` or `remove-handoff:`
      after a mutation failure; successful removal adds no discovery
      chatter; preflight refusal stays concise.
- [x] **Tests** - CLI integration tests (`cli/tests/remove.rs` and a new
      recovery-focused integration test): representative wiring and recovery
      through the real binary; refusal exit 3 with evidence; exit 4 paths;
      post-move stop exit 1 with both locations; wrapper two-invocation
      discovery placement; stdout protocol unchanged. Snapshot tests live in
      integration tests only (the double-target trap).

Checkpoint: acceptance items 1, 2, 8, 9 pass through the real binary;
existing remove/handoff/wrapper tests still green; `just test`, `just
test-l2`, `just lint` pass locally.

## Phase 12: Docs, skills, and cross-OS validation

### Wave 12 (parallel)

- [x] **Docs** - Rewrite `worktree/docs/cli/remove.md`'s planned section into
      current behavior (exit-code meanings, eligible topologies and
      filesystem limits, retry from the base checkout, the recovery record
      location and report); touch `worktree/README.md` where public behavior
      changes; update `docs/dependencies.md` files for any crate R4-R6 added.
- [x] **Skills** - Update the worktree skill's `remove.md` (and `testing.md`
      if fixtures change) and the `os` skill with any newly learned
      platform facts, in the same change as the code that established them.
- [x] **Implementation log and traceability** - Record rulings, spike
      findings, departures from the spec, and the acceptance-item (1-12) to
      test mapping table.
- [x] **Cross-OS validation** - macOS locally; Linux, native Windows, and
      WSL2 evidence per the `os` skill (real child-process/rename/handle/
      watch tests are ordinary L1 tests when they need no terminal);
      representative shell-wrapper tests stay at their existing tiers;
      `just ci-local --plan` reviewed against R11.

Checkpoint: docs describe current behavior with planned markers removed only
for built behavior; all four OSes have recorded evidence; structural checks
re-verified; agent terminal state is "implementation complete, ready for
review".
