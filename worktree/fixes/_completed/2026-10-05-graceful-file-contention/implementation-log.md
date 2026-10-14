---
spec: /Volumes/coding/wt/rusty-biscuit/feat-filesystem-watchers/worktree/fixes/2026-10-05-graceful-file-contention/spec.md
plan: worktree/fixes/2026-10-05-graceful-file-contention/plan.md
implemented_by: claude/opus
started_phase: 1
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

# Implementation Log for 2026-10-05-graceful-file-contention (12 phases)

## Phase 1

Started and finished 2026-10-11. Phase 1 changes no package source: it records
the rulings, runs the three spikes, and adds the OS and Git facts they turned
up to the `os` and `worktree` skills.

### Hosts and evidence

Hosts declared on this machine (`env | grep '^BUILD_'`):

| Host | OS / filesystem | Git |
| --- | --- | --- |
| local | macOS, APFS (`/tmp` and `/Volumes/coding` are separate volumes) | 2.56.0 |
| `BUILD_LINUX=build-linux` | Linux 7.0, ZFS home, tmpfs `/tmp` and `/dev/shm` | 2.47.3 |
| `BUILD_WSL=build-win` | WSL2 (kernel 6.18), ext4 home, tmpfs, drvfs/9p `/mnt/b` | 2.53.0 |
| `BUILD_WIN=build-win-native` | native Windows, ReFS `B:` and NTFS `C:` | 2.55.0.windows.3 |

Every host was reached and every spike ran on each host the plan named, and
more. The spikes are throwaway programs, so they ran as scratch crates over
`ssh -o BatchMode=yes` rather than through `just cross-check`, which runs
package suites. Each remote scratch directory was deleted afterwards. The
probe sources and raw output are kept in `spikes/` (see its README) so a
reviewer can rerun them. One Windows S1 run was invalid: `cmd /c ping`'s
orphaned `ping.exe` kept its cwd inside the source directory for 3 s and
skewed every later case. That run is kept as `results/INVALID-run1-*`, and
the corrected rerun (`run2-windows.txt`) is the evidence used below.

### Rulings

| Ruling | Status | Outcome |
| --- | --- | --- |
| R1 Git association | Author decision, **pending confirmation**; proceeding on the spec's recommendation | Option 1, controlled deletion with registration retained. S3 sharpens it: never `git worktree move` (it replaces an empty directory); use a native no-replacement rename plus a checked rewrite of exactly the admin `gitdir`; the final unregister is a plain `git worktree remove` of the then-absent quarantine (details under S3) |
| R2 Local observation | Author decision, **pending confirmation**; proceeding on the spec's recommendation | Option 1, native notifications plus one inventory and targeted checks. Unusable observation refuses before mutation as an operational error |
| R3 Sniff latency contract | Author confirmation, **pending**; proceeding on the landed Option 1 | `query_path_usage` with `PathUsageOptions::default()` (the two-second shared budget, `DEFAULT_DEADLINE`). The budget stops scheduling work; a native call already running may overrun it. `PathUsageError` is matched by variant (`kind()` is the stable machine name), never by message text |
| R4 Watcher adapter | Decided from S2 | **Focused native adapters on all three OSes behind one internal observer type, not `notify`** (reasons under S2) |
| R5 No-replacement rename | Decided from S1 | Linux `renameat2(RENAME_NOREPLACE)`; macOS `renamex_np(RENAME_EXCL)`; Windows `SetFileInformationByHandle(FileRenameInfo, ReplaceIfExists = FALSE)`. Error mapping under S1 |
| R6 `git worktree move` | Decided from S3 | Not used. Native rename plus a checked rewrite of the admin `gitdir`, preserving the record's link format. Verified against Git 2.47.3, 2.53.0, 2.55.0.windows.3 (association reads only), 2.56.0 |
| R7 Record and journal | Confirmed | As planned: 128-bit `getrandom` (0.4, already a dependency) operation ID rendered as lowercase hex; `<common-git-dir>/wt-removal/<operation-id>/` holding the record, `report.json`, and the journal; owner-only permissions (0o700/0o600 on Unix; on Windows the `.git`-inherited ACL); temp file plus atomic rename with file and directory `fsync` at batch boundaries; intents fsynced before a batch and results after; a crash between an unlink and its result is `uncertain` on recovery; a completed operation keeps `report.json` and deletes the journal; only the held `fs4` lock (0.13, already a dependency) means an active operation. **Batch bound: one directory or 256 entries, whichever is smaller** (S2 found 256-event bursts deliver without drops on FSEvents; 2,000-event bursts do not) |
| R8 Existing checks | Confirmed | The early `check_no_processes` in `worktree/cli/src/commands/remove/mod.rs` `run` stays as the preliminary check that spares the questions. The `check_no_processes` and Windows `check_not_in_use` probe inside `remove_worktree` are replaced by the fresh Sniff preflight and the recorded quarantine move |
| R9 Exit codes | Confirmed by the spec | The spec's exit table verbatim; the typed outcome is kept separately from the exit code. S1 adds a Windows classification rule (below) |
| R10 No performance spike | Recorded | None scheduled. The S2 burst measurements below were capability questions (when do events drop?), not timing gates |
| R11 CI scope | Planning ruling, verified in Phase 8 | Unchanged. The `spikes/` folder holds no compiled source (`probe.rs.txt`), so it selects no CI cell |
| R12 Self-exclusion | Confirmed against Sniff's report shape | Sniff has no exclusion parameter; `wt` filters its own records out of the returned report. A record is the operation's own only when its `pid` is the current process, its `creation_token` (or `start_time`) matches the current process where Sniff supplies one, and the evidence names a handle the operation registered: an inotify `watch.watch_id` with that instance's `descriptor`, or a Windows directory handle by `identity`. FSEvents needs no descriptor (fseventsd holds the watch), so a macOS watch never shows as `wt`'s evidence. Windows' file-side API already omits the calling process |

### S1: atomic no-replacement rename (R5)

The probe moved a directory onto: an absent destination, an existing empty
directory, a file, a non-empty directory, and a dangling link. It also tried a
missing source, another volume, and a source that is in use.

| Case | macOS APFS | Linux ZFS, tmpfs, WSL ext4 | WSL drvfs (9p) | Windows ReFS and NTFS |
| --- | --- | --- | --- | --- |
| absent destination | moved; same `dev`/`ino` | moved; same `dev`/`ino` | **`EINVAL` (22)** | moved; same volume serial and 128-bit file ID |
| destination exists (empty dir, file, non-empty dir, dangling link) | `EEXIST` (17), both sides untouched | `EEXIST` | `EEXIST` | `ERROR_ALREADY_EXISTS` (183) |
| missing source | `ENOENT` | `ENOENT` | `ENOENT` | `ERROR_FILE_NOT_FOUND` (2), at the open |
| other volume | `EXDEV` (18) | `EXDEV` | n/a | `ERROR_NOT_SAME_DEVICE` (17) |
| parent not writable | `EACCES` (13) | `EACCES` | `EINVAL` | not reproducible (below) |
| in use (open file, child cwd, own watcher) | moved | moved | `EINVAL` | see below |

For contrast, `std::fs::rename` **replaced an existing empty directory** on
every OS, and on Windows it also replaced an existing file. That confirms the
plan: it is unusable for the quarantine move.

Windows in-use detail, from the corrected run. Every failure was
`ERROR_ACCESS_DENIED` (5) from `SetFileInformationByHandle`. Opening the
source with `DELETE` access always succeeded.

| Holder | Result |
| --- | --- |
| an open file anywhere in the subtree, with or without `FILE_SHARE_DELETE` | denied |
| a handle on a subdirectory | denied |
| a child process whose cwd is a subdirectory | denied |
| a handle on the directory itself (std default sharing) | moved |
| our own `notify` watcher on the directory, or a recursive one on its parent | moved |
| ACL deny of DELETE on the source, or of add-subdirectory on the parent | moved (the parent's delete-child right still allows it) |

Rulings that follow:

- **The primitive is supported on every local filesystem tested, except WSL
  drvfs.** There `RENAME_NOREPLACE` returns `EINVAL` (unsupported), even for
  the plain move. `EINVAL` from the move maps to an operational error *before
  mutation* ("this filesystem cannot move without replacement"): nothing is
  moved, and the checkout and registration stay. `EEXIST` is returned even
  on drvfs because the VFS checks the destination before the filesystem
  rejects the flag, so `EEXIST` never proves support. The consequence is a
  behavior change: a WSL checkout on `/mnt/<drive>` that `wt remove` used to
  delete will be refused. Phase 4 documents it, and Phase 8 discloses it.
  The considered fallback (exclusively create an empty placeholder, then
  `rename(2)` over it, so the only possible replacement is an empty
  directory swapped in at a random hidden name) is **rejected** as weaker
  than the contract. It is an author override if wanted.
- **Windows classification (amends R9's mapping).** A directory rename denied
  by an in-use subtree returns the same code (5) as an ACL denial, so the
  code alone cannot tell sharing from permission. The rule:
  - `ERROR_ACCESS_DENIED` or `ERROR_SHARING_VIOLATION` (32) from the
    rename call, after the `DELETE` open succeeded, is a **sharing failure
    before the move** (exit 4, nothing removed). Phase 4 runs a
    target-scoped Sniff query only to *annotate* it with PIDs.
  - The same codes from the `DELETE` open are a permission/operational
    error.
  - `ERROR_ALREADY_EXISTS` (183), `ERROR_NOT_SAME_DEVICE` (17), and
    anything else are operational.

  A genuine ACL denial could not be produced at the rename stage on this
  host, so a rare permission failure may be reported as "in use". The exit
  code (4) and "nothing was removed" are still true. This is recorded as a
  known limit, not a departure from the exit table.
- **No special privileges** were needed on any OS.
- **Windows identity:** use `GetFileInformationByHandleEx(FileIdInfo)`
  (64-bit volume serial plus 128-bit file ID). The legacy 64-bit index is
  not guaranteed unique on ReFS.
- The existing Windows rename-and-back probe is confirmed redundant: the
  quarantine move fails the same way, and when it does nothing has moved.

### S2: watcher capabilities (R4)

Candidate: `notify` 8.2.0, the maintained portable adapter. It is built on
`inotify` 0.11, `fsevent-sys` 4.1, and `windows-sys` **0.60** (the workspace
pins 0.61). Two layouts were tried on each OS: A watches the parent
non-recursively and the checkout recursively; B watches the parent
recursively. Both also watched the admin directory non-recursively.

| Question | Linux inotify | macOS FSEvents | Windows `ReadDirectoryChangesW` |
| --- | --- | --- | --- |
| Our own rename of a watched checkout (layout A) | notify emits `Name(From)` for the root, then **silently drops the watch** (a later `unwatch` gives `WatchNotFound`); writes in the quarantine produce **no events** | path-based: the quarantine is **unobserved** until a new watch is added | the handle follows the directory, but events carry the **old path** (`checkout\sub\after-move.txt`) |
| Layout B (parent recursive) | works, but means one inotify watch per directory of *every sibling worktree* | works; sibling events filtered in-process | works with correct paths; one handle on the parent; does not block the rename |
| Recreation of the original path | `Create(Folder)` seen; files written before notify re-adds watches on the new directory are missed (recursive-add race) | seen | seen |
| Admin dir `index.lock` → `index` rename and `sharedindex.*` | seen | seen | seen |
| Overflow surfaced? (backend blocked, 40,000 creates) | WSL (`max_queued_events=16384`): one `Rescan`; build-linux (8,388,608): no overflow | `Rescan` "user dropped" (45–88 times) | **silent**: 2 create events of 40,000, no rescan, no error |
| Unprovoked drops | none | **every 2,000-file burst dropped events** (`MustScanSubDirs`, "user dropped") with a non-blocking handler; no drops at 64–1,000 | none |
| Marker barrier (write N files, then a marker; wait for the marker) | marker after all N, ~1 ms | in 1 of 9 rounds at 256 files, **8 files written before the marker arrived after it**; at 2,000 files the marker itself was lost twice | marker after all N, 40–80 ms |
| Instance limit | WSL `max_user_instances=128`: watcher 121 fails at creation with `EMFILE` (an `Io` error, not `MaxFilesWatch`) | none hit at 300 (~19 ms per stream start) | none hit at 300 |
| Threads after drop | back to baseline | back to baseline (pool threads remain) | back to baseline |
| Watcher drop while its handler is blocked in `send` | not hit | **`drop` hangs forever** (the backend thread waits on the full channel) | not hit |

`notify` source confirms the Windows behavior. On a zero-byte completion (the
overflow signal), `handle_event` parses a zeroed buffer as action 0 and emits
nothing. On any completion error other than `ERROR_OPERATION_ABORTED` or
`ERROR_ACCESS_DENIED`, it logs, unwatches, and tells the handler nothing.

**R4 ruling: focused native adapters on all three OSes, behind one internal
observer type in `worktree/lib/src/remove/observation/`.** `notify` fails the
coverage contract on every OS, each in a different way:

- **Windows:** overflow and lost watches are silent.
- **Linux:** the checkout watch is dropped on our own move.
- **macOS:** no flush primitive, latency 0 with `NoDefer` drops events on
  bursts, and delivery is out of order relative to a marker.

Per-OS shape:

- **Linux: inotify** (the `inotify` 0.11 crate notify already uses, or
  `libc` directly). Watch descriptors follow inodes across our own rename, so
  the adapter relabels its paths when the operation reports its move,
  without a gap. It watches the parent non-recursively, the checkout
  recursively, and the admin directory. `IN_Q_OVERFLOW` and an unexpected
  `IN_IGNORED` are lost coverage. A new directory in the quarantine is
  already a stop condition, so the recursive-add race needs no scan. A
  drain is a marker in each watched root, read until `EAGAIN`; ordering is
  FIFO per instance.
- **macOS: FSEvents** through `fsevent-sys` 4.1. One stream covers the
  checkout and the quarantine paths (path-based, so it is not invalidated by
  the rename) plus the parent and admin directory. Use a nonzero latency
  with file events rather than notify's 0/`NoDefer`. A drain is
  `FSEventStreamFlushSync`, never a marker. `MustScanSubDirs` and either
  `Dropped` flag are lost coverage. Coalesced history flags mean event kind
  alone never classifies an operation-owned unlink: S2 saw a `Create` for a
  file reported only when we deleted it.
- **Windows: `ReadDirectoryChangesW`** through `windows-sys` 0.61 (workspace
  version). It watches the parent with the subtree flag (one handle; S1
  shows it does not block the move) and the admin directory. A large buffer
  is used for local volumes. A zero-byte completion, `ERROR_NOTIFY_ENUM_DIR`,
  or any completion error other than the operation's own cancel is lost
  coverage. A drain is a marker, reliable because delivery is FIFO per
  handle. The admin-directory handle is closed before the final unregister.
  The spike did not test whether an open watch handle leaves a deleted
  directory delete-pending; it showed only that `remove_dir` of a watched
  quarantine root succeeds. Phase 5 verifies this.

Every adapter:

- uses a bounded queue fed by a **never-blocking** handler (`try_send`; a
  full queue is lost coverage);
- turns a creation failure (for example `EMFILE` at the WSL instance limit)
  into an operational error before mutation;
- leaves no thread once the session ends;
- registers its handles for R12's self-exclusion.

The cost is three small adapters instead of one dependency. It is
accepted because the plan's R4 already allowed "focused native adapters
where `notify` cannot surface overflow/lost-watch or rename invalidation",
and S2 shows `notify` cannot do so on any of the three. Candidate dependencies
for Phase 5 (both are what `notify` itself builds on; checked in the local
registry):

- `inotify` 0.11.5, ISC, `hannobraun/inotify-rs`. Linux only; `libc`
  directly is the dependency-free alternative.
- `fsevent-sys` 4.1.0, MIT, `octplane/fsevent-rust`. macOS only; it exposes
  `FSEventStreamFlushSync`.

Phase 5 records the final choice in `worktree/docs/dependencies.md`.
`windows-sys` 0.61 is already in the workspace.

Supported filesystem capabilities, as far as the spikes show:

- local APFS, ZFS, ext4, tmpfs, ReFS, and NTFS support every mechanism above;
- WSL drvfs is refused by R5's move before observation matters;
- network filesystems and FUSE are untested. Phase 5 must refuse when watch
  setup fails, and Phase 8 records the gap.

### S3: Git move and final unregister (R6, R1 detail)

The same script ran on macOS (2.56.0), Linux (2.47.3), and WSL (2.53.0), with
identical results except where noted.

- **`git worktree move` replaces an existing empty directory.** Moving
  `q1a` into `holder/`, where `holder/q1a/` existed empty, succeeded, and the
  inode at `holder/q1a` changed. It refuses a file or a non-empty directory
  (`'…' already exists`). An existing empty directory is accepted
  deliberately (as by `worktree add`). Git also turns a directory
  destination into `<dir>/<name>`. Its existence check is followed by
  `rename(2)`, so it fails the no-replacement contract. **R6: never used.**
- **What a move rewrites:** exactly the admin `gitdir` and the checkout's
  `.git` file. Unrelated records, including a prunable one, are untouched.
  The `.git` file is rewritten in the *invoking* config's link format: a
  relative-link record moved without `worktree.useRelativePaths` came out
  absolute.
- **Manual equivalent, verified.** After a native rename plus a rewrite of
  only `<common>/worktrees/<id>/gitdir`, `git worktree list` shows the new
  path, `status` works inside it, and `prune --dry-run` leaves it alone.
  Unrelated records are unchanged.
- **Relative links (Git ≥ 2.48, `extensions.relativeWorktrees = true`).**
  The admin `gitdir` is relative to **the admin directory itself**
  (`../../../../q3b/.git` from `main/.git/worktrees/q3b/`). The `.git` file
  is relative to the checkout (`../main/.git/worktrees/q3b`). A
  same-depth sibling move leaves the `.git` file valid, so only the admin
  `gitdir` changes. A move to another depth also breaks the `.git` file.
  The spike's own off-by-one rewrite (`../../../`) made the record prunable,
  so Phase 4 computes the relative form and verifies it by reading it back
  with Git. Git 2.47 ignores `worktree.useRelativePaths`, and reads a
  relative admin `gitdir` verbatim as a path.
- **Rule for Phase 4:** the quarantine is a sibling (same depth). Rewrite the
  admin `gitdir` in the record's existing format; leave the `.git` file
  unchanged; refuse a record whose `.git` file would need rewriting.
- **Locks:** `move` and `move -f` refuse a locked worktree, and so does
  `remove --force`. The `locked` file survives a manual move, and a plain
  remove of a locked absent record refuses. So the native move must refuse
  a locked worktree itself (re-listed immediately before moving), and the
  final step cannot bypass a lock.
- **Submodules:** `move` and plain `remove` refuse ("working trees
  containing submodules cannot be moved or removed"). The submodule's git
  directory lives under the admin directory
  (`worktrees/<id>/modules/<name>`). An early, invalid run of the script
  also showed `remove --force` succeeding on such a worktree. That is the
  path today's `wt remove` takes when files may be discarded, so refusing
  submodule checkouts before mutation is the regression R1 already
  discloses.
- **Index files:** a split index puts `sharedindex.<sha>` in the
  *per-worktree admin directory*, not the common directory. Intent-to-add
  and skip-worktree show in `ls-files -v` and in the `--stage --debug`
  flags. The existing `missing.rs` binding (BLAKE3 of the `index` bytes,
  plus BLAKE3 of `ls-files --stage --debug -z` with `GIT_INDEX_FILE`)
  already covers both. The admin watch must cover `index`, `index.lock`,
  `sharedindex.*`, `HEAD`, `gitdir`, and `locked`.
- **Final unregister:**
  - A plain `git worktree remove` of a quarantine holding only `.git`
    **refuses**, because Git counts the deleted tracked files as
    modifications.
  - A single `--force` works, but it would delete whatever is there.
  - A plain remove of the **absent** directory is record-only and works.
  - A recreated directory without `.git` is refused ("validation failed").

  **Ruling:** delete approved content children-first with the quarantine's
  `.git` file last. Then `rmdir` the quarantine root; that fails if anything
  reappeared. Then reuse the tested `missing.rs` `remove_missing_record`
  path. It re-lists the record and rechecks admin identity and HEAD,
  re-confirms the absence, rebinds the index digests, and runs a plain
  `git worktree remove <quarantine>`. No `--force` is ever passed.

### Departures and items for the author

These do not block Phase 2 (records, journal, and typed outcomes touch none
of them). They need the author's confirmation before the phase each one
gates:

1. R1, R2, R3: still proceeding on the spec's recommendations, pending
   confirmation (gate Phases 3–6).
2. R4: three native observer adapters instead of `notify` (gates Phase 5),
   adding `inotify` and `fsevent-sys`.
3. R5: a WSL checkout on a drvfs (`/mnt/<drive>`) volume is refused, not
   removed (gates Phase 4). The placeholder fallback is rejected unless the
   author overrides.
4. R9 amendment: Windows `ERROR_ACCESS_DENIED` from the rename call is
   classified as "in use" (exit 4), so a rare ACL denial may be described as
   usage (gates Phase 4).

### Verification

No package source changed in this phase, so no behavior needs a new test.
The phase's requirements were capability questions, each answered by the
spike that maps to it:

| Requirement | Answered by |
| --- | --- |
| R5 | S1 on four hosts |
| R4 | S2 and B1 on four hosts |
| R6 | S3 on Git 2.47, 2.53, and 2.56 |

Close-out gates in `worktree/`, on macOS, 2026-10-11:

- `just test`: 1173 passed, 14 skipped (the area's existing skips), 0 failed.
- `just lint`: clean for `worktree` and `worktree-cli`.

Not run: `just test-l2` (not required for a phase without source changes)
and cross-OS package suites (nothing compiled changed).

Phase 1 is complete; ready for review.

## Phase 2

Started and finished 2026-10-12. Phase 2 adds the library contracts: typed
outcomes, the recovery record, the deletion journal, the operation lock, and
the store that ties them together. Nothing here moves or deletes a checkout
yet. Everything runs against temp directories.

### What was built

All of it is in `worktree/lib/src/remove/`:

| File | Contents |
| --- | --- |
| `outcome.rs` | `RemovalOutcome` (`usage_refused`, `removed`, `activity_detected`, `cleanup_failed`, `state_unverified`). `OperationReport` (`report_version` 1, persisted as `report.json`) with the original and quarantine states, Git registration, timeline, discovery runs (Sniff `PathUsageReport` or typed failure), observed activity (receipt times, no PID), journal summary, branch outcomes, errors, and truncation counts. Its lists are capped at 200 and keep exact omitted counts |
| `recovery/mod.rs` | `OperationId` (128-bit `getrandom`, 32 lowercase hex; reuses `handoff::new_token`, whose `is_token` became `pub(crate)`). The module docs state the crash guarantees |
| `recovery/record.rs` | `RecoveryRecord` (`record_version` 1) and the strict `parse`: the version is read first through `strict_json::members`, then the typed struct with `deny_unknown_fields`, then semantic validation. `check_binding` compares repository and admin-record identity |
| `recovery/journal.rs` | The JSON Lines journal (header, intent, and results lines), its strict `parse`, `JournalLog::replay` into `Deleted` / `ExternallyAbsent` / `Kept` / `Planned` / `Uncertain`, `summarize`, and `JournalWriter` behind an injectable `JournalSink` |
| `recovery/lock.rs` | The `fs4` lock on `wt-removal/locks/<blake3(admin name)>.lock`, through the existing `cache::try_lock_sidecar` |
| `recovery/storage.rs` (private) | Owner-only creation (0o700/0o600), no-follow validation (symlink and reparse-point refusal, owner and mode checks, open-then-compare identity), durable replace (temp file, flush, rename, directory flush) |
| `recovery/store.rs` | `RecoveryStore` (`open`, `lock`, `begin`, `pending`, `read_record`, `recover`) and `Operation` (`journal`, `advance`, `save_report`, `complete`, with a bookkeeping warning when the journal cannot be cleared) |

`worktree/lib/Cargo.toml` gains `libc = "0.2"` for Unix only (already in the
workspace graph; Phase 4 needs it for `renameat2`/`renamex_np` anyway). It is
used for `geteuid`, `O_NOFOLLOW`, and Apple `fsync`. The change is recorded in
`docs/dependencies.md`.

### Decisions and departures

None of these changes a spec requirement. They settle details the plan left
open, or that came up while building.

1. **Journal format.** The journal is JSON Lines, one object per line with a
   single key: `header` (`journal_version`, `operation_id`), `intent`
   (`batch`, `entries` of `{path, kind}`), or `results` (`batch`, `entries` of
   `{path, state}`, aligned with the intent in order). Paths are relative to
   the quarantine and stored as Sniff `NativeString`s. A batch number must
   follow the previous one, results must follow their own intent, and a new
   intent cannot open while a batch is unresolved.
2. **"Empty" in the journal matrix** means a header-only journal, which is
   valid ("nothing attempted"). A batch whose `entries` is `[]` is invalid, as
   is a zero-byte journal: `begin` writes the header before the operation
   directory exists, so a committed operation always has one.
3. **Torn final line.** Any unterminated last line is dropped and flagged as
   `torn_tail`. A torn intent means that batch never started; its intent was
   not durable, so nothing was unlinked. A torn results line leaves its batch
   unresolved, so absent entries replay as `Uncertain` and present ones as
   `Planned`.
4. **Storage limit.** `DEFAULT_JOURNAL_LIMIT` is 64 MiB. An intent is refused
   (`LimitReached`, nothing written) unless the batch's worst-case results
   line still fits, so confirmed results are never lost to the limit.
5. **The batch bound** (R7: one directory, at most 256 entries) is enforced by
   both the writer and the reader.
6. **Detached checkouts** use `"branch": "detached"` (an externally tagged
   enum, `{"named": "…"}` otherwise), so the record has no nullable field. Every
   record field is required; `aliases: []` is valid, while absent `aliases` is
   not.
7. **An unreadable record blocks every new removal in the repository,** not
   only one for the same worktree, because it could describe that worktree.
   Phase 6 must turn this into guidance (where the record is, how to inspect
   it).
8. **Atomic operation directory.** `begin` builds `.<id>.tmp`, writes the
   journal header and record, flushes, and renames it into place. A crash
   before the rename leaves a hidden directory that `pending()` ignores, and
   nothing has been mutated at that point. Hidden staging directories are not
   cleaned up automatically, consistent with the spec's "no background
   deletion".
9. **Windows.** The journal is closed before that rename and reopened for
   appending afterwards (`JournalWriter::after_header`), because S1 showed
   Windows refuses to rename a directory with an open file inside it. The
   first version of `begin` had this bug; the code review of my own diff
   caught it before any Windows run. Windows has no owner or mode check: files
   inherit the Git directory's ACL, as R7 states. Reparse points are refused.
   Directories cannot be flushed on Windows, which the crash guarantees in
   `recovery/mod.rs` state.
10. **macOS flush.** std's `sync_all`/`sync_data` issue `F_FULLFSYNC` on Apple
    platforms, at about 25 ms per call. `storage::flush` uses plain `fsync`
    there. R7 asks for `fsync`, and the spec claims process-termination
    durability, not power-loss durability. Measured on this host: the
    recovery tests took 0.3–4 s each with std's call and about 30 ms with
    `fsync`. Phase 5 makes two journal flushes per batch, so this also bounds
    deletion cost on macOS. Recorded in the `os` skill.
11. **Timestamps** are `u64` milliseconds since the Unix epoch (`at_ms`,
    `received_at_ms`, `created_at_ms`), avoiding a `chrono` dependency in
    `worktree`. The embedded Sniff report keeps its own RFC 3339 times.
12. **Deferred to the phase that owns the inputs:**
    - **The exit-code mapping goes to Phase 6.** Exit 4 (sharing failure,
      missing wrapper) and cancel-0 are not operation outcomes, so the mapping
      belongs to the lifecycle. `OperationReport::branch_failed` is the
      "checkout removed, branch failed → 1" input.
    - **Journal resume after a crash goes to Phase 6.** Phase 2 can recover
      and replay a journal, but cannot reopen it and continue deleting.
    - **The inventory goes to Phase 5,** which persists it with the journal.
      The journal has no inventory line yet, and adding one changes the
      format (see the message to the next agent).
    - **A `usage_refused` report goes to Phase 6.** It has no operation
      directory to be saved in; Phase 6 decides whether it is persisted.

### Requirement-to-test mapping

All tests are L1 unit tests in the `worktree` library (`just test` selects
them; none carries a tier marker). They are in-process and each runs in
under 0.1 s.

| Requirement (plan Phase 2 / spec) | Test |
| --- | --- |
| Typed outcomes and report model round-trip, versioned, unknown fields refused | `outcome::tests::a_stopped_report_round_trips_through_json`, `an_unknown_report_field_is_refused` |
| Lossless native paths in reports | `outcome::tests::a_non_utf8_path_survives_the_report_losslessly` (Unix) |
| Truncation never loses counts | `outcome::tests::activity_past_the_cap_is_counted_not_listed`, `retained_entries_keep_an_exact_count_past_the_listing_cap` |
| Branch failure visible on a removed checkout | `outcome::tests::a_failed_branch_action_is_visible_on_a_removed_checkout` |
| Operation ID: random, never a path | `recovery::tests::generated_ids_parse_and_differ`, `an_id_that_could_name_another_path_never_parses` |
| Owner-only permissions, durable replace, no-follow reads | `storage::tests::created_storage_is_owner_only`, `storage_others_can_reach_is_refused`, `links_are_never_followed`, `a_durable_write_replaces_the_file_and_leaves_no_temporary`, `a_missing_file_is_reported_as_missing` |
| Record: robustness matrix (every load-bearing field × absent, null, wrong type, empty, duplicate key, invalid content; array element cells; inner duplicate key; trailing content; unknown field; control) and the three semantic refusals (unsupported version, operation mismatch, identity mismatch) | `store::tests::record_robustness_matrix` |
| Journal: robustness matrix (`path`, `kind`, `state` × every shape; entries element cells; empty batch; trailing content and blank line; header absent, repeated, other version, other operation; wrong batch; misaligned results; valid header-only; torn intent; torn results) | `store::tests::journal_robustness_matrix` |
| Planned, confirmed, externally absent, kept, and uncertain distinguished; crash between unlink and result is uncertain | `journal::tests::confirmed_results_and_an_interrupted_batch_recover_distinctly`, `a_torn_results_line_leaves_its_batch_uncertain` |
| Journal write failure stops destructive work (injected append and flush failures; poisoned writer) | `journal::tests::a_failed_intent_flush_refuses_the_batch_and_every_later_write`, `a_torn_results_line_leaves_its_batch_uncertain` |
| Storage limit stops deletion without discarding confirmations (boundary −1 / exact) | `journal::tests::the_limit_refuses_an_intent_while_its_results_still_fit` |
| Batch bound and shape | `journal::tests::batches_outside_the_bound_are_refused_by_writer_and_reader` |
| Journal bound to its operation | `journal::tests::another_operations_journal_is_refused`, matrix "header: other operation" |
| Empty journal = nothing attempted | `journal::tests::a_header_only_journal_means_nothing_was_attempted`, matrix "empty: header only" |
| Durable, owner-only pending operation reads back | `store::tests::begin_makes_an_owner_only_pending_operation_that_reads_back` |
| OS-held lock; a stale lock file is not an active operation | `store::tests::a_held_lock_refuses_a_second_removal_and_a_stale_lock_file_does_not` |
| Lock keyed by the admin record | `store::tests::the_lock_covers_only_the_worktree_it_was_taken_for` |
| A pending record blocks a new operation on the same target | `store::tests::a_pending_record_blocks_a_new_operation_on_the_same_worktree_only` |
| A malformed record is never ignored | `store::tests::an_unreadable_record_blocks_every_new_operation` |
| A crash before commit leaves no operation | `store::tests::a_staging_directory_left_by_a_crash_is_not_an_operation` |
| A record that cannot be created refuses before mutation | `store::tests::a_record_that_cannot_be_created_leaves_no_operation` (Unix) |
| Phase persistence moves only forward; a failed write keeps the last durable phase | `store::tests::advance_persists_each_phase_and_only_forward`, `a_failed_phase_write_keeps_the_earlier_phase_on_disk_and_in_memory` (Unix) |
| Completion persisted before clearing; report kept; journal deleted; completed record never authorizes again | `store::tests::completion_keeps_the_report_drops_the_journal_and_never_authorizes_again` |
| Clearing failure is a bookkeeping warning | `store::tests::a_journal_that_cannot_be_cleared_is_a_warning_after_completion` |
| Storage validated without following replacement links | `store::tests::a_linked_operation_directory_is_never_read` |

Smell check: in `recovery/` there is no `#[serde(default)]`, no `Option` on a
deserialized field, and no `.ok()`, `unwrap_or_default()`, or `filter_map` on
a load-bearing parse. The one `unwrap_or_default` is on `split(...).pop()`,
which always yields a piece. The two matrix tests were confirmed
load-bearing by their control rows.

Not covered here: real process-termination tests. The plan schedules them
for Phase 6 with the lifecycle; Phase 2's crash windows are covered by the
injected-failure sink and by hand-built torn journals.

### Verification

Gates in `worktree/` on macOS, 2026-10-12, after the final change:

- `just test`: 1207 passed, 14 skipped (the area's existing skips), 0 failed.
- `just lint`: clean for `worktree` and `worktree-cli`.

Cross-OS runs (`just cross-check worktree --os <os> remove::recovery
remove::outcome`):

| OS | Result |
| --- | --- |
| macOS (local) | 34 passed |
| Linux (`build-linux`) | 34 passed |
| native Windows (`build-win-native`) | 29 passed (the 5 Unix-only tests are not compiled there) |
| WSL2 | not run: it runs the same Unix code as Linux, and nothing in this phase depends on the filesystem type |

The Windows run includes `begin` with the reopen-after-rename fix, which the
first draft would have failed.

Two caveats:

- `storage::tests::links_are_never_followed` and
  `store::tests::a_linked_operation_directory_is_never_read` return early
  when Windows refuses to create a symlink (no Developer Mode or privilege).
  The pass on `build-win-native` does not show which path ran.
- One earlier cross-check attempt failed before any test ran, because the
  remote `just` recipe could not parse a `-E 'test(/…/)'` filter. Pass
  substring filters instead.

Not run: `just test-l2`. No terminal-facing code changed, and Phase 2 adds no
L2 tests.

Phase 2 is complete; ready for review.

## Author rulings and renumbering

Recorded 2026-10-12, between Phases 2 and 3. These close every item Phase 1
left "pending confirmation"; no `human_review` is needed for them.

| Item | Ruling |
| --- | --- |
| R1 Git association | Confirmed: option 1, as refined by S3 |
| R2 Local observation | Confirmed: option 1 |
| R3 Sniff budget | Confirmed: the landed contract, no flag, no retry |
| R4 Watcher adapters | Confirmed: three native adapters; adds `inotify` and `fsevent-sys` |
| R5 WSL drvfs | Confirmed: refuse before mutation; exit 4; no placeholder fallback |
| Windows code 5/32 | Confirmed: from the rename after a successful `DELETE` open means in use, exit 4; from the open means permission error |
| Exit codes before mutation (new) | Environment limits (unsupported rename, observation setup, recovery storage) exit 4; ineligible content (FIFO, device, nested mount, unsupported reparse point, submodule checkout) exits 3; exit 1 only after a move or with uncertain state |
| Order of query and observation (new) | Observation and inventory first, then the decisive Sniff query excluding the operation's own watch handles, then binding rechecks, record, move. The spec's flowchart was corrected to match its text |

The spec records these under its new **Decisions** section and moved from
`draft-spec` to `planned`.

The plan was split from 8 phases to 12 so each phase fits one session with
at most one OS-specific FFI surface. Phase 1 and 2 text above uses the old
numbers:

| Old phase | New phase(s) |
| --- | --- |
| 3 | 3 |
| 4 | 4 (rename backends), 5 (naming, association, rollback) |
| 5 | 6 (observer type, injected observer, inotify), 7 (FSEvents), 8 (ReadDirectoryChangesW), 9 (inventory, deletion engine, final verification) |
| 6 | 10 |
| 7 | 11 |
| 8 | 12 |

## Phase 3

Started and finished 2026-10-12. Phase 3 adds the decisive usage check (a
library module) and the refusal `wt remove` prints for it (a CLI module).
Neither is called by `wt remove` yet: the plan puts the call site in Phase 10's
lifecycle, after observation starts. `run` and `remove_worktree` still use
`check_no_processes` and the Windows probe, so user-visible behavior is
unchanged.

### What was built

| File | Contents |
| --- | --- |
| `worktree/lib/src/remove/usage.rs` | `UsageSource` (the injection seam, like `PrSource`) and `SniffUsage` (`query_path_usage` with default options: the shared 2 s budget, no retry, no new flag). `OwnedHandles` (the R12 registry). `check_usage` (query once, then `assess`) and `assess` (the pure policy). `UsageClearance` keeps the whole report, the count of excluded own handles, and a `CoverageSummary`. `UsageRefusal::InUse(InUse)` holds `Holder`s (PID, creation token, `identity_uncertain`, name, executable, evidence). `UsageRefusal::Unverified(PathUsageError)` covers every typed query error. `EvidenceLocation` is `Path`, `Aliases` (every in-scope name, none preferred), or `Unnamed` (identity and observed spelling only) |
| `worktree/lib/src/remove/usage/tests.rs` | Policy matrix against injected reports, plus two real Sniff queries |
| `worktree/cli/src/commands/remove/usage.rs` | `refusal_markup` (Prose markup, one line per `\n`), `usage_refusal` (wraps the markup in `WorktreeError::RefusedToLoseWork`, so exit 3 through the existing mapping and printing in `main`), and `coverage_note` |
| `worktree/cli/tests/remove_usage_refusal.rs` and six snapshots | Rendering through the library's real `assess`, snapshot-tested in an integration test because of the worktree skill's double-target trap |

### Decisions and departures

1. **Self-exclusion (R12).** Sniff does not drop the caller's own evidence;
   only Windows open handles are excluded, by the kernel. `OwnedHandles`
   therefore matches `(EvidenceKind, descriptor)` under `wt`'s own PID. A
   live process's PID cannot be reused, so the PID identifies this process,
   with two exceptions that are never excluded. One is a record Sniff marks
   `identity_uncertain`. The other covers every record for this PID when its
   certain records disagree on the creation token. `wt` cannot read its own
   creation token through Sniff's public API, so token agreement is checked
   across records instead. Evidence without a descriptor (working directory,
   loaded module) is never excluded: `wt` running from inside the checkout
   refuses. Nothing registers a handle yet; Phase 6's watchers must register
   theirs before the query.
2. **A `PathUsageError` is exit 3, not 4.** Every variant means the checkout
   could not be verified right before the move: it vanished, it is no longer
   a directory, or its identity could not be read in time. That is "changed
   approved state before mutation" in the spec's table. Exit 4 means a
   different environment is needed, which does not apply. `InvalidOption`
   cannot occur with default options and is mapped the same way rather than
   given its own path. Errors are matched by variant (`unverified_reason`),
   never by message text (R3).
3. **Rendering.** It follows the spec's example. The two departures in
   wording are recorded here only (the spec is unchanged):
   - The coverage note says "cannot be listed on this system" rather than
     naming the OS, so the snapshots are identical on every host.
   - It summarizes every non-complete mechanism by status, so a real macOS
     report reads "Discovery is partial; FSEvents subscriptions and polling
     watchers cannot be listed on this system; open handles and working
     directories were only partly read." (`open_handles` and
     `working_directories` are `partial` on macOS because other users'
     processes cannot be read.)

   The refusal lists up to 10 processes and 5 evidence lines per process, and
   counts the rest. A process with no name says "name unavailable". An
   uncertain lifetime is marked, and no name is looked up for it. Names,
   executables, paths, and error text have control characters escaped
   (`\u{1b}`, `\n`) and then Prose-escaped. The rerun command quotes the
   name through the existing `shell_word`. A name no shell can spell
   identically gets "wt remove for this worktree" instead of a pasteable
   command.
4. **`#[allow(dead_code)]` on `pub mod usage` in the CLI.** The `wt` binary
   target compiles the module but does not call it until Phase 10. The
   attribute and its comment must be removed when `run` calls
   `usage_refusal`. A crate-level allow already exists in `dirty_tree.rs`.
5. **R8 is unchanged in this phase.** The early `check_no_processes` in `run`
   and the one inside `remove_worktree` (with the Windows probe) remain until
   Phase 10 replaces them.

### Requirement-to-test mapping

All new tests are L1 (no tier marker). The library tests are in the
`worktree` lib binary; the CLI tests are in the auto-discovered
`worktree-cli` integration test `remove_usage_refusal`.

| Requirement (plan Phase 3 / spec "Preflight and refusal") | Test |
| --- | --- |
| Each of `watch_registration`, `open_handle`, `working_directory`, `loaded_module` refuses, naming PID, name, executable, kind, and exact path | `usage::tests::every_evidence_kind_from_another_process_refuses` |
| No match continues for `usable` (complete and partial), `unavailable`, and `unsupported`; coverage and the whole report are retained | `usage::tests::no_match_clears_whatever_the_coverage` |
| A match in a partial report (budget exhausted, truncated) still refuses and carries the coverage | `usage::tests::a_match_in_a_partial_report_refuses_with_its_coverage` |
| Exclusion of exactly the registered operation handles. Near misses refuse: the same descriptor in another PID, an unregistered descriptor, a different kind, no descriptor, an uncertain lifetime, and conflicting lifetimes for this PID | `usage::tests::only_this_operations_registered_handles_are_excluded` |
| Not every record of the current PID is excluded | `usage::tests::this_processes_other_evidence_still_refuses` |
| Several aliases and identity-only evidence reported as facts; no name for an uncertain lifetime | `usage::tests::locations_report_aliases_and_identity_only_evidence_as_facts` |
| Every typed `PathUsageError` refuses before mutation, matched by variant; exactly one query (no automatic retry) | `usage::tests::every_query_error_refuses_as_unverified_without_a_retry` |
| Real Sniff query (crosses the library → Sniff → OS boundary): a fixture-owned child's working directory refuses by PID; an empty tree clears | `usage::tests::a_real_query_names_a_process_working_in_the_checkout` |
| Real Sniff query of a vanished checkout refuses as unverified | `usage::tests::a_real_query_of_a_vanished_checkout_refuses` |
| The spec's example output (concise preamble, PID line, evidence line, rerun command, "kept" line, harmless-usage line, coverage note) | `remove_usage_refusal::the_specs_example` (snapshot) |
| Several holders, each evidence kind, aliases, unnamed objects, a nameless and uncertain holder; no note when coverage is complete | `remove_usage_refusal::every_evidence_location_and_holder_shape` (snapshot) |
| Untrusted names and paths are escaped (markup, ANSI escape, newline); the rerun argument is quoted, or omitted when unquotable | `remove_usage_refusal::untrusted_text_is_shown_literally` (snapshot + assertions) |
| Long lists are counted, not listed | `remove_usage_refusal::long_lists_are_counted` (snapshot) |
| Coverage notes: complete, macOS-typical, budget exhausted plus not attempted, failed, unsupported, omitted evidence | `remove_usage_refusal::coverage_notes` (snapshot) |
| An unverified checkout refuses, one message per error variant | `remove_usage_refusal::an_unverified_checkout_refuses` (snapshot) |
| Exit 3 for both refusals; no override flag offered | `remove_usage_refusal::refusals_exit_3_and_offer_no_override` |

The input robustness matrix does not apply: this phase reads no file or
configuration format. Sniff's report is consumed as typed values.

"Exits 3 and 4 still remove nothing" holds without a new test: the module
mutates nothing and `wt remove`'s flow is unchanged. The existing remove
tests still pass.

### Verification

Gates in `worktree/` on macOS, 2026-10-12, after the final change:

- `just test`: 1225 passed, 14 skipped (the area's existing skips), 0 failed.
- `just lint`: clean for `worktree` and `worktree-cli`. The first run flagged
  `clippy::type_complexity` in a test, fixed with a type alias.

Cross-OS (`just cross-check worktree --os <os> remove::usage …`):

| OS | Result |
| --- | --- |
| macOS (local) | 9 library + 7 CLI tests passed; the real query test takes 0.47 s |
| Linux (`build-linux`) | 9 library tests passed |
| native Windows (`build-win-native`) | 9 library tests passed; the real query test takes 1.9 s for two queries |
| WSL2 | not run: same Unix code path as Linux, and nothing here depends on the filesystem |

`cross-check worktree` runs the `worktree` library package only, so the CLI
rendering tests ran on macOS alone. They are pure string rendering from
in-memory values and have no OS-dependent input.

Not run: `just test-l2`. No terminal-interaction code path changed, and
nothing calls the new rendering from `wt` yet.

Phase 3 is complete; ready for review.

## Phase 4

Started and finished 2026-10-12. Phase 4 adds the no-replacement rename the
quarantine move will use, with one FFI surface per OS behind one function.
Nothing calls it yet (Phase 5 does), and nothing here touches Git, so
`wt remove`'s behavior is unchanged.

### What was built

| File | Contents |
| --- | --- |
| `worktree/lib/src/remove/quarantine/mod.rs` | The `quarantine` module; only `pub mod rename` for now |
| `worktree/lib/src/remove/quarantine/rename.rs` | `rename_no_replace(from, to) -> Result<(), RenameError>`; `RenameError { kind: RenameFailure, source: io::Error }`; `RenameFailure` = `Collision`, `Unsupported`, `CrossDevice`, `Permission`, `Sharing`, `Other`; `identity(path) -> io::Result<FileIdentity>` (no-follow); `pub(crate) classify(Stage, &io::Error)` |
| `worktree/lib/src/remove/quarantine/rename/tests.rs` | Real-filesystem tests per OS plus classification tables |
| `worktree/lib/Cargo.toml` | `windows-sys` 0.61 for Windows (`Win32_Foundation`, `Win32_Storage_FileSystem`); the existing Unix `libc` gains a second use |

Primitives, as R5 decided: Linux `renameat2(RENAME_NOREPLACE)`, macOS
`renamex_np(RENAME_EXCL)`, Windows `SetFileInformationByHandle(FileRenameInfo,
ReplaceIfExists = FALSE)` on a handle opened with `DELETE | SYNCHRONIZE`,
full sharing, `FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT`.
Windows identity is `GetFileInformationByHandleEx(FileIdInfo)`.

Error mapping:

| Kind | Unix | Windows |
| --- | --- | --- |
| `Collision` | `EEXIST`, `ENOTEMPTY` | 183 `ERROR_ALREADY_EXISTS`, 80 `ERROR_FILE_EXISTS` |
| `Unsupported` | `EINVAL` (drvfs), `ENOSYS` (kernel before 3.15), `ENOTSUP` | 50 `ERROR_NOT_SUPPORTED`, 1 `ERROR_INVALID_FUNCTION` |
| `CrossDevice` | `EXDEV` | 17 `ERROR_NOT_SAME_DEVICE` |
| `Permission` | `EACCES`, `EPERM` | 5 or 32 from the `DELETE` open |
| `Sharing` | never | 5 or 32 from the rename after a successful open |
| `Other` | everything else, including `ENOENT`, `EROFS`, a NUL byte in a path | everything else, including 2/3 (missing source) |

### Decisions and departures

1. **Raw `SYS_renameat2` on Linux**, not libc's `renameat2` wrapper (which the
   S1 probe used). Older glibc and some musl builds do not export the wrapper;
   the syscall works with any libc, and `ENOSYS` on an old kernel maps to
   `Unsupported`.
2. **`identity` lives in the rename module** rather than calling Sniff's
   `identity::of_path`, which is `pub(crate)` in Sniff. Making it public would
   touch `sniff` and widen CI scope to its reverse dependencies for a function
   of a few lines; the Windows open helper is shared with the rename anyway.
   The encoding matches Sniff's (`u128::from_le_bytes` of the 128-bit ID), so
   identities compare equal with a Sniff report or a recovery record. If a
   later phase needs Sniff's version anyway, replace this one rather than
   keeping both.
3. **The rename returns `()`, not the moved identity.** Phase 5 reads
   `identity(to)` after the move and compares it with the record's
   `checkout_identity`; the destination is a random hidden sibling, so reading
   it by path is not a meaningful race.
4. **Unix targets other than Linux and macOS** refuse with `ENOTSUP`
   (`Unsupported`) rather than falling back to a rename that may replace. The
   repository supports only macOS, Linux, and Windows; the fallback exists so
   the crate still compiles elsewhere.
5. **Windows destination** is made absolute with `std::path::absolute`
   before the call, because `FileRenameInfo` without a root handle needs a
   full path.
6. **Unrelated fix in Phase 2 code.** A cross-target clippy for Windows
   (`cargo clippy -p worktree --lib --tests --target x86_64-pc-windows-gnu
   -- -D warnings`) failed on `recovery/storage.rs::create_private_dir`: its
   `let mut builder` is mutated only under `#[cfg(unix)]`, so Windows reported
   an unused `mut`. CI lints on Linux only, so this was latent. Fixed by
   building the `DirBuilder` inside each `cfg` branch. Behavior is unchanged;
   the recovery tests pass on macOS and native Windows.

### Requirement-to-test mapping

All tests are L1 unit tests in the `worktree` library binary, module
`remove::quarantine::rename::tests` (no tier marker; each runs in under
0.1 s). The input robustness matrix does not apply: nothing here reads a file
format or configuration.

| Requirement (plan Phase 4) | Test | Hosts |
| --- | --- | --- |
| An absent destination receives the tree; the original path is gone; identity survives | `an_absent_destination_receives_the_tree_with_its_identity` | all |
| An existing empty directory, file, non-empty directory, or dangling link is never replaced; `Collision`; both sides keep identity and content | `an_existing_destination_is_never_replaced` | all (the link cell ran on native Windows: no skip message) |
| A failed rename leaves the original intact | asserted in every failure test above and below | all |
| Missing source → `Other` with `NotFound`; nothing created | `a_missing_source_fails_as_other_and_creates_nothing` | all |
| Identity is no-follow; missing path is `NotFound` | `identity_names_a_link_itself_not_its_target` | all |
| A link source moves as itself, not its target | `a_link_is_moved_as_itself` | all |
| Unix permission denial → `Permission`, nothing moved (skips as root) | `a_parent_this_user_cannot_write_is_a_permission_failure` | macOS, Linux, WSL2 |
| Invalid path input (NUL) → `Other`, nothing moved | `a_path_with_a_nul_byte_fails_as_other` | macOS, Linux, WSL2 |
| Native (non-UTF-8) names move losslessly | `a_name_that_is_not_utf8_moves_losslessly` | Linux, WSL2 |
| Windows: an open file in the subtree (std default sharing, and without delete sharing) → `Sharing`; nothing moved; moves once released | `an_open_file_in_the_tree_is_a_sharing_failure` | native Windows |
| Error kinds a host cannot produce on demand: drvfs `EINVAL`, `ENOSYS`, `ENOTSUP`, `EXDEV`, `EACCES`/`EPERM`, `ENOENT`/`EROFS`/`EBUSY` | `unix_error_codes_classify` | macOS, Linux, WSL2 |
| Windows codes by stage: 5/32 at the open → `Permission`, at the rename → `Sharing`; 183/80, 17, 50/1, 2/3/87 | `windows_error_codes_classify_by_stage` | native Windows |
| A non-OS error is `Other` | `an_error_without_an_os_code_is_other` | all |

Mutation check: replacing `RENAME_EXCL` with `0` (a plain, replacing rename)
on macOS made `an_existing_destination_is_never_replaced` fail at the empty
directory cell; restored afterwards.

### Verification

Gates in `worktree/` on macOS, 2026-10-12, after the final change:

- `just test`: 1234 passed, 14 skipped (the area's existing skips), 0 failed.
- `just lint`: clean for `worktree` and `worktree-cli`.
- Windows clippy (cross-target, see decision 6): clean after the fix.

Cross-OS (`just cross-check worktree --os <os> quarantine`):

| OS | Result |
| --- | --- |
| macOS (local) | 9 passed |
| Linux (`build-linux`) | 10 passed (adds the non-UTF-8 test) |
| native Windows (`build-win-native`) | 8 passed; `remove::recovery quarantine` rerun after decision 6: 32 passed |
| WSL2 (`build-win`) | passed (ext4 home; drvfs `EINVAL` is covered by classification, as the plan says) |

Not run: `just test-l2` (no terminal code changed); a Linux-target clippy
(no remote lint recipe; the Linux code compiled and ran on `build-linux`, and
CI lints on Linux).

Phase 4 is complete; ready for review.

## Phase 5

Started and finished 2026-10-12. Phase 5 adds the quarantine's name and the
move that keeps Git's registration on the moved checkout, with verified
rollback. Nothing calls it yet (Phase 10 does), so `wt remove`'s behavior is
unchanged.

### What was built

| File | Contents |
| --- | --- |
| `worktree/lib/src/remove/quarantine/naming.rs` | `quarantine_path(original, &OperationId)` → `<parent>/.wt-remove-<slug>-<token>`: the slug is the checkout name reduced to ASCII letters, digits, `-`, `_` (at most 40 bytes, `worktree` when nothing is left); the token is the first 16 hex characters of the operation ID. `hide(path)` (Windows) sets `FILE_ATTRIBUTE_HIDDEN`, keeping other attributes |
| `worktree/lib/src/remove/quarantine/association.rs` | `plan_move(base, &WorktreeEntry, &OperationId) -> Result<MovePlan, MoveRefusal>` and `move_to_quarantine(base, &MovePlan, activity_observed) -> Result<(), MoveError>`. `MoveRefusal` (`NotARealDirectory`, `Locked { reason }`, `Submodules { entry }`, `Unsupported`, `Association`, `Changed`, `Uninspectable`); `MoveError` (`Refused`, `Rename`, `RolledBack { cause }`, `Stranded { cause, reason, state: MoveState }`) with `class()` → `MoveErrorClass` and `nothing_changed()`. `pub(crate) move_with(.., &Faults)` is the fault seam |
| `worktree/lib/src/remove/quarantine/association/tests.rs` | Real repositories, real renames, Git reading the result back |
| `admin_entry.rs`, `recovery/mod.rs`, `recovery/storage.rs` | Visibility only: `path_from_bytes`, `storage`, `storage::flush`, and `storage::sync_dir` become `pub(crate)` for reuse |

The checks `plan_move` runs, and `move_to_quarantine` runs again immediately
before the rename: the original is a real directory with the planned
identity; a fresh `git worktree list` shows the same worktree (branch, or
HEAD when detached) exactly once, not prunable; the same admin entry points at
it; the admin `gitdir` bytes are unchanged; no `locked` entry; the `.git` file
is a regular file linking to the admin directory in a form that survives a
same-depth move; and no submodule (admin `modules`, or a `160000` gitlink in
the worktree's index).

After the rename: the quarantine's identity must equal the plan's; the admin
`gitdir` is replaced (temp file, original permissions, flush, rename, directory
flush) with the recorded bytes in which only the checkout's name changed; then
Git's record must name `<quarantine>/.git`, the `.git` file must still resolve
to the admin directory, and a fresh listing must show the worktree, not
prunable, at the quarantine and nothing at the original.

### Decisions and departures

1. **The record rewrite is a byte substitution of the last directory name**
   (`rewrite_back_reference`), not a path recomputation. It keeps everything
   Git wrote: absolute or relative (relative to the admin directory), `/` on
   Windows, the trailing newline. A record whose last two components are not
   `<name>/.git` refuses as `Unsupported` (for example a Windows 8.3 short
   name, or a case difference).
2. **Forward-link rule.** A relative `.git` link is accepted only as one or
   more leading `..` steps followed by names. Lexically normalizing both
   sides was considered and rejected: `../x/../../repo/...` normalizes equal
   from the sibling, yet resolves through the checkout's own name, which no
   longer exists after the move. The test pins this case.
3. **Rollback is skipped when the quarantine's identity changed**, in
   addition to the plan's two conditions (original absent, no activity).
   Moving back whatever now sits at the quarantine would put someone else's
   directory at the original path. "No destructive work" holds by
   construction: deletion has not begun when the move runs. Rollback renames
   back first and restores `gitdir` second, so a failure in between leaves the
   content at its user-visible path; either way the stranded state is
   observed and reported (`MoveState`: checkout location by identity,
   `original_recreated`, `RegistrationState`).
4. **Exit classes.** `Refused` (exit 3) for every `MoveRefusal`, including a
   lock and a submodule. `BlockedByEnvironment` (exit 4) for rename `Sharing`
   and `Unsupported`. Everything else is `Failed`: `Collision`, `CrossDevice`,
   `Permission`, `Other` (nothing moved, never labeled usage), and a
   rolled-back or stranded move (a move happened, matching decision 7's
   "exit 1 means a move happened"). Phase 10 maps the class to
   `WorktreeError`. Today a locked worktree fails inside `git worktree
   remove` with exit 1; under the lifecycle it will exit 3 before mutation
   (the docs' planned section says so).
5. **`activity_observed: &dyn Fn() -> bool`** is the rollback's activity
   input until Phase 6's observer exists; Phase 10 passes the observer's
   "anything unexpected so far" check.
6. **Hidden attribute (Windows) is best-effort** and set last, after the
   association is verified, so a rollback never needs to undo it. A failure
   to set it is ignored: the dot prefix and the record already identify the
   quarantine.
7. **Not done here:** writing the recovery record between plan and move
   (Phase 10 owns `RecoveryStore::begin`), and the `record.quarantine` field
   is filled from `plan.quarantine`. The quarantine name satisfies
   `RecoveryRecord::validate` (prefix and sibling) by construction.

### Requirement-to-test mapping

All tests are L1 (no tier marker) in the `worktree` library binary, module
`remove::quarantine`; the slowest takes 0.3 s on macOS and 1.7 s on native
Windows. The input robustness matrix does not apply: the only files read are
Git's own `gitdir`/`.git`/`locked`, read strictly and compared byte for byte,
not a configuration this code parses into fields.

| Requirement (plan Phase 5) | Test |
| --- | --- |
| Hidden sibling `.wt-remove-<slug>-<token>`, same parent, prefix the record accepts | `naming::tests::the_quarantine_is_a_hidden_sibling_named_for_the_operation` |
| Collision-resistant (token per operation) | `naming::tests::different_operations_get_different_quarantines` |
| Slug variants: case, punctuation, non-ASCII, empty, over-long, trailing dash after truncation | `naming::tests::the_slug_is_ascii_and_bounded` |
| Non-UTF-8 name still yields a valid sibling (Unix) | `naming::tests::a_name_that_is_not_utf8_still_yields_a_valid_sibling` |
| No parent or name → no quarantine | `naming::tests::a_path_without_a_parent_or_name_has_no_quarantine` |
| Absolute format: Git reads the quarantine under its retained registration (listing, branch, `status`), `prune --dry-run` leaves it, `.git` unchanged, only the name changed in `gitdir`, unrelated and prunable records untouched, no `move`/`repair`/`prune`/`unlock`/`--force` call; hidden on Windows | `association::tests::an_absolute_record_moves_and_git_reads_the_quarantine` |
| Relative format kept (Git ≥ 2.48; skips on 2.47) | `association::tests::a_relative_record_keeps_its_format` |
| Locked worktree refuses at planning (with reason) and when locked after planning (no reason); lock kept; nothing changed | `association::tests::a_locked_worktree_refuses_at_planning_and_at_the_move` |
| Submodule refusal, both of Git's signals; nothing changed | `association::tests::a_checkout_with_a_submodule_refuses_before_anything_moves` |
| Main checkout and a `.git` link that would break refuse; a leading-`..` relative link moves | `association::tests::unsupported_shapes_refuse_before_anything_moves` |
| Re-validation immediately before moving: switched branch, replaced directory, link at the path | `association::tests::state_that_changed_since_planning_refuses_the_move` |
| Rename collision never replaces (acceptance item 5); exit class `Failed`; nothing changed | `association::tests::a_collision_at_the_quarantine_is_never_replaced` |
| Failed association write → verified rollback, every Git byte restored | `association::tests::a_failed_record_rewrite_is_rolled_back` |
| Written but unverifiable association → rollback restores the original `gitdir` | `association::tests::an_unverified_record_rewrite_is_rolled_back_and_restored` |
| Activity → no rollback; quarantine preserved; actual state reported | `association::tests::observed_activity_leaves_the_quarantine_in_place` |
| Recreated original is never overwritten or merged into | `association::tests::a_recreated_original_is_never_overwritten` |
| Replaced quarantine is not moved back | `association::tests::a_replaced_quarantine_is_not_moved_back` |
| Sharing/Unsupported → exit 4 class; collision, cross-device, permission, other → `Failed`; refusals → `Refused` | `association::tests::rename_failures_map_to_their_exit_classes` |
| Windows open file → `Sharing`, exit 4 class, nothing changed | `association::tests::an_open_file_in_the_checkout_blocks_the_move_as_in_use` (Windows only) |
| The rewrite changes only the name (absolute, relative, CRLF, no newline, mismatches; Windows backslashes) | `association::tests::the_record_rewrite_changes_only_the_checkout_name` |

Mutation checks on macOS, each restored afterwards: skipping the `gitdir`
restore during rollback failed
`an_unverified_record_rewrite_is_rolled_back_and_restored`; removing the lock
check failed `a_locked_worktree_refuses_at_planning_and_at_the_move`.

Unsupported rename (drvfs) as exit 4 before mutation is covered by the class
mapping; Phase 4's classification tests cover the `EINVAL` → `Unsupported`
step, since no host here produces it on demand.

### Verification

Gates in `worktree/` on macOS (Git 2.56), 2026-10-12, after the final change:

- `just test`: 1253 passed, 14 skipped (the area's existing skips), 0 failed.
- `just lint`: clean for `worktree` and `worktree-cli`.
- Windows cross-target clippy (`cargo clippy -p worktree --lib --tests
  --target x86_64-pc-windows-gnu -- -D warnings`): clean.

Cross-OS (`just cross-check worktree --os <os> quarantine`):

| OS | Result |
| --- | --- |
| macOS (local, Git 2.56) | 28 passed; the relative-format test ran |
| Linux (`build-linux`, Git 2.47.3) | 29 passed; the relative-format test skips (Git writes absolute links only) |
| native Windows (`build-win-native`) | 27 passed, including the open-file test and the hidden attribute |
| WSL2 | not run: the same Linux code and Git association logic; drvfs is a classification case covered above |

Not run: `just test-l2` (no terminal code changed).

Phase 5 is complete; ready for review.

## Phase 6

Started and finished 2026-10-12. Phase 6 adds the activity observer's
contract, an injected observer for tests, and the Linux inotify adapter.
Nothing calls it yet (Phase 10 does), so `wt remove`'s behavior is unchanged.
On macOS and Windows `Observation::start` returns
`ObservationError::Unsupported` until Phases 7 and 8 add their adapters.

### What was built

| File | Contents |
| --- | --- |
| `worktree/lib/src/remove/observation/mod.rs` | `Targets` (original, quarantine sibling, admin dir); `Observation` (`start`, `drain`, `moved_to_quarantine`, `release_admin`, `register_handles`, `tree_location`); `Drain { events, lost }`; `ObservedEvent { scope, kind, directory, received }`; `Scope` (`Tree`, `Original`, `Quarantine`, `Admin`); `EventKind`; `CoverageLost` (`Overflow`, `QueueFull`, `WatchLost { root }`, `ReadFailed`); `ObservationError` (`Unsupported`, `Setup`); `ADMIN_FILES` and `is_admin_file`; the crate-internal `Backend` trait, `Sink`, and the bounded `EventQueue` for adapters with a delivery thread |
| `worktree/lib/src/remove/observation/inotify.rs` | The Linux adapter |
| `worktree/lib/src/remove/observation/injected.rs` | `Observation::injected` and `Injector` (test only) |
| `worktree/lib/src/remove/observation/tests.rs` | Shared contract tests: injected, and native wherever an adapter exists |
| `worktree/lib/src/remove/observation/inotify/tests.rs` | Linux-only adapter tests |
| `worktree/lib/src/remove/usage.rs` | Test-only `OwnedHandles::is_registered` |
| `worktree/lib/Cargo.toml`, `Cargo.lock` | `inotify` 0.11.5 (ISC), Linux only, `default-features = false` (the default `stream` feature pulls in tokio) |

### Decisions and departures

1. **The inotify drain writes no marker.** The plan said "drain is a marker
   per root read until `EAGAIN`". inotify queues an event inside the system
   call that causes it (fsnotify runs synchronously in the VFS operation), so
   reading until `EAGAIN` already includes every change whose call returned
   before the drain. A marker file would put the operation's own files into
   the checkout and into Git's admin directory. The barrier is pinned by
   `native_drain_is_a_barrier_for_every_change_kind` (50 rounds of
   write-then-drain with no sleep, green on build-linux and WSL2). FSEvents
   (Phase 7) uses `FSEventStreamFlushSync`. Windows (Phase 8) still needs its
   marker.
2. **No delivery thread on Linux.** The kernel queue (`max_queued_events`)
   is the bound between drains, and `IN_Q_OVERFLOW` is `Overflow`. The
   session's own bound (`ObserverOptions::queue_capacity`, default 16,384)
   applies per drain. `EventQueue` is the bounded, never-waiting queue for
   threaded adapters. The injected observer already feeds it, so its contract
   is tested before Phase 7 uses it.
3. **Out-of-scope events never count against the bound.**
   `Targets::classify` runs before an event is queued. The first version
   counted a sibling's event before dropping it, and the injected
   queue-bound test caught it. FSEvents watches the parent recursively, so
   busy sibling worktrees would otherwise fill the queue.
4. **Scopes, not absolute paths.** A tree event is `Scope::Tree(relative)`
   before and after the move, because inotify watch descriptors follow the
   inode. `Original`/`Quarantine` are the parent's entries with those names.
   `moved_to_quarantine` changes `tree_location()` and lets a path-based
   adapter re-map paths; on Linux it needs nothing else, with no gap.
5. **Watch-end rules.** A tree directory's `IN_IGNORED` is expected only
   after its own `IN_DELETE_SELF`. The parent's and admin directory's
   `IN_DELETE_SELF`/`IN_MOVE_SELF`/`IN_UNMOUNT`, and any unexplained
   `IN_IGNORED`, are `WatchLost`. A tree directory's `IN_DELETE_SELF` and
   `IN_MOVE_SELF` produce no event, because the directory holding it already
   reports the entry. A moved tree subdirectory keeps its old label; the move
   is already new activity. `release_admin` removes the admin watch before
   Git's unregister deletes the admin directory, so that deletion is not
   loss. Windows' "close the admin handle before the final unregister"
   (Phase 8) uses the same call.
6. **Setup.** Each tree directory is watched before it is listed, so a child
   created during the walk is either listed or reported by its parent. A child
   gone or replaced between listing and watch (`ENOENT`/`ENOTDIR`) is skipped
   for that reason. A watch descriptor returned twice (a bind mount, or an
   admin directory inside the tree) fails setup, because paths could not be
   told apart. Every setup error (`EMFILE`, `ENOSPC` at `max_user_watches`, a
   link at the tree root refused by `IN_DONT_FOLLOW | IN_ONLYDIR`, a missing
   admin directory) is `ObservationError::Setup`. Phase 10 maps it to exit 4.
7. **Kinds.** `IN_MODIFY` and `IN_CLOSE_WRITE` are both `Modified`
   (`CLOSE_WRITE` also covers `mmap` writes, which raise no `IN_MODIFY`).
   `IN_EXCL_UNLINK` is not set.
8. **R12 registration.** `register_handles` registers
   `(WatchRegistration, <inotify fd>)`. The parent and admin watches are
   outside the queried tree, so Sniff never reports them. FSEvents needs no
   entry (fseventsd holds the watch).
9. **`expect(dead_code)` off Linux** on `Sink::push` and `Sink::targets`:
   only native adapters push raw events. `expect` (not `allow`) fails the lint
   once Phase 7 uses them, which forces removing the attribute.

### Requirement-to-test mapping

All tests are L1 (no tier marker) in the `worktree` library binary. Each runs
in under 0.12 s. The input robustness matrix does not apply: nothing here
reads a file format or configuration.

| Requirement (plan Phase 6) | Test |
| --- | --- |
| Targets are distinct named siblings | `observation::tests::targets_must_be_distinct_named_siblings` |
| Admin files: `index`, `index.lock`, `sharedindex.*`, `HEAD`, `gitdir`, `locked`, and nothing else | `observation::tests::admin_files_are_the_identity_and_index_files` |
| Classification: tree, original entry and below, quarantine entry, admin files; siblings, the parent itself, and other admin files dropped; a drain hands events over once | `observation::tests::injected_events_are_classified_and_unrelated_ones_dropped` |
| Delayed delivery (injected) | `observation::tests::delayed_events_arrive_only_once_delivered` |
| Overflow and lost watch (each root) are typed, sticky, keep earlier events, and the first loss wins | `observation::tests::coverage_loss_is_typed_and_stays_lost` |
| Bounded queue: a full queue is loss, keeps what fit, siblings don't count; exactly the capacity is not loss | `observation::tests::a_full_queue_is_coverage_loss_keeping_what_fit`, `exactly_the_capacity_is_not_a_loss` |
| Released admin: no events, no loss | `observation::tests::a_released_admin_directory_is_neither_reported_nor_lost` |
| Relabel, handle registration, session end on drop | `observation::tests::the_move_is_relabeled_and_handles_registered_and_released` |
| Drain is a barrier for create, modify, metadata, rename, directory create, remove (real files, no sleep) | `observation::tests::native_drain_is_a_barrier_for_every_change_kind` |
| Coverage follows the real no-replacement move without a gap; recreation of the original is seen | `observation::tests::native_coverage_follows_the_move_and_sees_recreation` |
| Real siblings ignored; real admin writes (`index.lock` → `index` rename, `sharedindex`, `HEAD`, `gitdir`, `locked`) reported; `ORIG_HEAD` and `logs/` not | `observation::tests::native_siblings_are_ignored_and_admin_files_reported` |
| Intentional children-first deletion of the quarantine and released admin removal are not loss; the parent watch still works | `observation::tests::native_intentional_deletion_is_not_coverage_loss` |
| Deleted admin dir and moved parent are `WatchLost`, sticky | `observation::tests::native_losing_a_watched_directory_is_coverage_loss` |
| Real full queue | `observation::tests::native_a_full_queue_is_coverage_loss` |
| Missing admin and a link at the tree root are setup errors; nothing changed | `observation::tests::native_setup_failure_is_an_error_before_anything_changes` |
| No adapter → `Unsupported` (macOS, Windows) | the same `native_*` tests through `native()` |
| `IN_Q_OVERFLOW` (wd -1) is `Overflow` | `inotify::tests::a_kernel_overflow_is_coverage_loss` |
| A real kernel queue overflow between drains | `inotify::tests::the_kernel_queue_overflowing_between_drains_is_coverage_loss` (runs where `max_queued_events` ≤ 32,768: ran on WSL2, skips with a message on build-linux) |
| Unexpected `IN_IGNORED` (each root) is loss; after `IN_DELETE_SELF` or release it is not; parent/admin `DELETE_SELF` is loss | `inotify::tests::an_ignored_watch_is_loss_unless_its_directory_was_deleted_or_released` |
| `IN_UNMOUNT`, moved parent/admin are loss; a moved tree subdirectory is not | `inotify::tests::an_unmount_or_a_moved_parent_or_admin_directory_is_loss` |
| Mask → kind mapping, `IN_ISDIR`, events on the watched directory itself | `inotify::tests::event_masks_map_to_kinds_under_the_watch_path` |
| `EMFILE` at the instance limit is a setup failure | `inotify::tests::the_instance_limit_is_a_setup_failure` (lowers `RLIMIT_NOFILE` in its own nextest process and restores it) |
| Registration with Phase 3's exclusion: unregistered watches refuse through a real Sniff query, registered ones clear (2 excluded), none remain after drop; no thread started | `inotify::tests::sniff_sees_the_watches_until_the_session_ends` |

Mutation check on build-linux, restored afterwards: no longer recording a tree
directory's `IN_DELETE_SELF` failed
`native_intentional_deletion_is_not_coverage_loss` and
`an_ignored_watch_is_loss_unless_its_directory_was_deleted_or_released`.

"Unestablishable observation is exit 4 before mutation" is the
`ObservationError` contract here. The exit mapping lands with Phase 10's
lifecycle, as for Phase 5's `MoveErrorClass`.

### Verification

Gates in `worktree/` on macOS, 2026-10-12, after the final change:

- `just test`: 1269 passed, 14 skipped (the area's existing skips), 0 failed.
- `just lint`: clean for `worktree` and `worktree-cli`.
- Windows cross-target clippy (`cargo clippy -p worktree --lib --tests
  --target x86_64-pc-windows-gnu -- -D warnings`): clean.
- Linux clippy was not run: cross-target clippy for Linux cannot build
  `aws-lc-sys` here (no Linux C cross-compiler), and `cross-check` has no lint
  mode. The Linux code compiled on build-linux and WSL2 without warnings, and it
  was reviewed by hand against default clippy lints (one `cmp_owned` in a test
  was fixed). CI's Linux lint job is the gate.

Cross-OS (`just cross-check worktree --os <os> [observation]`):

| OS | Result |
| --- | --- |
| macOS (local) | 17 passed: 16 observation tests (native tests assert `Unsupported`) plus one unrelated name match |
| Linux (`build-linux`) | `observation`: 24 passed (23 plus the same unrelated match) (the kernel-overflow test skips: `max_queued_events` 8,388,608); full `worktree` library: 679 passed |
| WSL2 (`build-win`) | `observation`: 24 passed, including the real kernel overflow |
| native Windows (`build-win-native`) | `observation`: 17 passed, as on macOS |

Not run: `just test-l2` (no terminal code changed).

Phase 6 is complete; ready for review.

## Phase 7

Started and finished 2026-10-12. Phase 7 adds the macOS FSEvents adapter
behind the Phase 6 observer contract. Nothing calls the observer yet (Phase
10 does), so `wt remove`'s behavior is unchanged. `Observation::start` now
succeeds on macOS; Windows still returns `Unsupported` until Phase 8.

### What was built

| File | Contents |
| --- | --- |
| `worktree/lib/src/remove/observation/fsevents.rs` | `FsEvents` (the `Backend`): one FSEvents stream (`fsevent-sys` 4.1, `kFSEventStreamCreateFlagFileEvents`, 0.1 s latency) over the original, quarantine, and admin paths plus a private marker directory, on a run-loop thread `wt-fsevents`; `Shared::locate`/`deliver` (flags → raw events or loss); `Paths::resolve`; `MarkerDirectory`; the marker barrier; identity checks; stop-and-join on drop |
| `worktree/lib/src/remove/observation/fsevents/tests.rs` | macOS-only adapter tests |
| `worktree/lib/src/remove/observation/mod.rs` | macOS dispatch in `native_backend` (now also given `ObserverOptions`); `CoverageLost::DrainTimedOut { waited }`; `EventQueue::take_loss` (macOS); the coalescing rule in the module contract |
| `worktree/lib/src/remove/observation/tests.rs` | `NATIVE_ADAPTER` (Linux, macOS), `COALESCES_BELOW_DIRECTORY` and `settle`, 25 barrier rounds on macOS, new `native_a_change_below_a_directory_is_reported_or_the_directory_event_stands_for_it` |
| `.config/nextest.toml` | test group `fsevents-l1` (`max-threads = 1`) for the macOS native observer tests |
| `worktree/lib/Cargo.toml`, `Cargo.lock` | `fsevent-sys` 4.1.0 (MIT), macOS only |

### Decisions and departures

1. **The drain is a marker plus flush, not `FSEventStreamFlushSync` alone.**
   The plan (from S2) said "drain is `FSEventStreamFlushSync`, never a
   marker". Measured: the flush delivers only what fseventsd has already read
   from the kernel, and that takes about 10 ms. The shared barrier test failed
   at round 5 of 50 with the flush alone. A marker alone is no drain either
   (S2: delivery is reordered against it). The drain therefore creates a file
   in a private marker directory the stream watches, then flushes every 1 ms
   until that file's event arrives. fseventsd reads the kernel's events in
   order, so every earlier change has reached it, and the flush that delivered
   the marker delivered them too. About 9 flushes and 11 ms per drain idle,
   and up to ~650 ms under full-suite load. Without the marker in 5 s
   (`DRAIN_TIMEOUT`) the drain records `CoverageLost::DrainTimedOut`, a new
   shared variant Phase 8 can use for its own marker.
2. **The marker directory is outside the checkout and Git.** Per Phase 6's
   reason for avoiding markers on Linux, it is not in the tree or the admin
   directory. It is `<confstr(_CS_DARWIN_USER_TEMP_DIR)>/wt-observe-<64-bit
   hex>`: per-user, owner-only, read without an environment variable, removed
   on drop. A crash leaves an empty directory for the system's temp cleanup.
   Ordering holds across volumes:
   `the_barrier_holds_for_a_checkout_on_another_volume` ran with the checkout
   on `/Volumes/coding` (disk3s7) and the marker on the Data volume (disk3s5).
3. **`start` runs one barrier.** `FSEventStreamStart` returns before events
   reach the stream; a file written right after it was never reported. If the
   first marker is not confirmed, `start` fails with `ObservationError::Setup`
   (exit 4 before mutation).
4. **The parent is not watched; identities are checked instead.** The plan
   listed the parent among the stream's paths, but FSEvents is recursive, so
   that would deliver every sibling worktree's activity, and a busy sibling (a
   build) is enough to make FSEvents drop events. The stream watches the
   original and quarantine paths (path-based, so the move needs no new
   watch), which also covers recreation of the original. A moved or deleted
   parent or admin directory produces no event under those paths, so every
   drain compares their `(dev, ino)` with start: a mismatch is
   `WatchLost { Parent }` or `WatchLost { Admin }` (admin only until
   released).
5. **FSEvents coalesces child changes into a directory's removal or rename.**
   Found while making the shared tests pass. A file written in `d` and then
   `d` renamed or deleted within one delivery batch is reported only as `d`'s
   event, even at latency 0. It was seen 0 of 5 times with no wait, and 5 of 5
   once the write was delivered first. The same hides a child's `Removed`
   when its directory is removed right after. This is now part of the observer
   contract (module docs in `mod.rs`): a caller that must see changes below a
   directory drains before removing or renaming it. The shared tests drain
   there through `settle` on macOS only, so Linux keeps its stronger
   no-drain assertions. The new shared test pins what holds everywhere: the
   directory's event is always reported. For Phase 9 this means a drain
   between a directory's children and its `rmdir`. The move's own window
   relies on Phase 9's per-entry revalidation and final `rmdir`. Recorded in
   the spec's `message_to_agent`.
6. **Kinds.** Each item flag present yields one event (`Created`, `Modified`,
   `Metadata` for inode-meta/Finder-info/owner/xattr, rename, `Removed`),
   because flags are cumulative history. A rename does not say which side it
   is: a path that exists when the event is read is `MovedTo`, otherwise
   `MovedFrom`. `MustScanSubDirs`, `UserDropped`, and `KernelDropped` are
   `Overflow` on any path; `Mount`/`Unmount` under a watched path are
   `WatchLost` for that root. The tree's root directory itself is reported as
   the parent's entry (`Original("")`/`Quarantine("")`), as the parent watch
   reports it on Linux.
7. **Relabeling.** `moved_to_quarantine` and `release_admin` each run a
   barrier first, so changes made before them are labeled as before (tree at
   the original path; admin reported); after it, the quarantine path is the
   tree and admin events are dropped.
8. **Paths.** FSEvents reports resolved paths (`/private/var/...`, on-disk
   case). `Paths::resolve` canonicalizes the parent, original, and admin.
   Parent entries are reported under the session's own names, so a
   differently cased original still classifies (tested on this
   case-insensitive volume). A link at the original, a non-directory, or a
   missing path is `ObservationError::Setup` naming the original.
9. **Thread and drop.** The run-loop thread schedules, starts, runs, then
   stops, invalidates, and releases the stream itself. The run loop is
   `CFRetain`ed, so `drop` can repeat `CFRunLoopStop` (a stop sent before the
   loop runs is lost) until the thread finishes, then join. The callback
   pushes into the never-waiting `EventQueue`, so a full queue cannot block
   drop. No handle is registered: fseventsd holds the watch.
10. **Test group.** Under full-suite load, fifteen observer tests starting
    streams together each spent up to 5 s in stream start (fseventsd serves
    registrations one at a time), so seven tests exceeded the 5 s budget.
    The macOS native observer tests now run in the nextest group
    `fsevents-l1` (`max-threads = 1`, macOS hosts only, inherited by the `ci`
    profile). Afterwards the slowest took 1.6 s, and the whole `just test` run
    dropped from 46 s to 31–36 s. The repo already uses groups for shared
    system resources (Apple Terminal, Windows networking). Barrier-round counts
    dropped to 25 on macOS; a flush-only drain already failed by round 5.
11. **`Sink::push` stays `expect(dead_code)` off Linux.** macOS feeds
    `EventQueue`, not the sink, so the Phase 6 note ("once your adapter calls
    push") did not apply; the reason text now says so.

### Requirement-to-test mapping

All tests are L1 (no tier marker) in the `worktree` library binary. The
input robustness matrix does not apply: nothing here reads a file format or
configuration.

| Requirement (plan Phase 7) | Test |
| --- | --- |
| Phase 6 contract against FSEvents: barrier for create, modify, metadata, rename, directory create, remove (no sleep) | `observation::tests::native_drain_is_a_barrier_for_every_change_kind` |
| Coverage follows the move (path-based) and sees the original's recreation | `observation::tests::native_coverage_follows_the_move_and_sees_recreation` |
| Siblings ignored; admin `index.lock` → `index`, `sharedindex.*`, `HEAD`, `gitdir`, `locked` reported; `ORIG_HEAD`, `logs/` not | `observation::tests::native_siblings_are_ignored_and_admin_files_reported` |
| Intentional deletion and released-admin removal are not loss | `observation::tests::native_intentional_deletion_is_not_coverage_loss` |
| Deleted admin directory and moved parent are `WatchLost`, sticky (identity checks) | `observation::tests::native_losing_a_watched_directory_is_coverage_loss` |
| Bounded queue: a full queue is loss | `observation::tests::native_a_full_queue_is_coverage_loss` |
| Setup failures (missing admin, link at the original) before anything changes | `observation::tests::native_setup_failure_is_an_error_before_anything_changes`; `fsevents::tests::a_setup_failure_names_the_path_and_starts_nothing` (a file, a missing path) |
| Coalescing: the directory's own event is always reported; children too where the platform does not coalesce | `observation::tests::native_a_change_below_a_directory_is_reported_or_the_directory_event_stands_for_it` |
| 2,000-file burst: complete or lost, never silently partial | `fsevents::tests::a_burst_is_delivered_completely_or_reported_as_lost` |
| `MustScanSubDirs` and either dropped flag are `Overflow`, keeping earlier events | `fsevents::tests::every_dropped_or_rescan_flag_is_coverage_loss` |
| `Mount`/`Unmount` under a watched root is loss; elsewhere ignored | `fsevents::tests::a_mount_or_unmount_under_a_watched_path_is_loss_and_elsewhere_ignored` |
| Coalesced flags each reported; type unknown without an item-type flag; no item flag reports nothing | `fsevents::tests::coalesced_item_flags_are_each_reported` |
| Rename direction | `fsevents::tests::a_rename_is_moved_to_where_the_path_exists_and_moved_from_where_it_does_not` |
| Labels before and after the move; siblings, `checkout2`, the parent, unrelated admin files dropped; released admin silent (events, removal, unmount) | `fsevents::tests::paths_are_labeled_by_where_the_tree_is` |
| Resolved (`/private`) watched paths; a differently cased original still covered | `fsevents::tests::watched_paths_are_resolved_as_fsevents_reports_them` |
| Marker barrier across volumes | `fsevents::tests::the_barrier_holds_for_a_checkout_on_another_volume` (skips with a message where temp and checkout share a volume) |
| Run-loop thread stopped and joined; drop does not block on a full queue | `fsevents::tests::the_run_loop_thread_ends_with_the_session_even_with_a_full_queue` |
| No self-exclusion entry needed: no handles, and Sniff does not refuse | `fsevents::tests::sniff_sees_nothing_of_the_session` |

Regression evidence: with the flush-only drain,
`native_drain_is_a_barrier_for_every_change_kind` failed at round 5. Without
the start barrier, `native_coverage_follows_the_move_and_sees_recreation`
missed `sub/before.txt`. The coalescing measurements (0/5 at no wait, 5/5 at
120 ms with latency 0.1; 0/5 and 5/5 at 20 ms with latency 0 or 0.01) came
from scratch tests that were removed afterwards.

### Verification

Gates in `worktree/` on macOS, 2026-10-12, after the final change:

- `just test`: 1281 passed, 14 skipped (the area's existing skips), 0
  failed, none slow.
- `just lint`: clean for `worktree` and `worktree-cli`.
- Windows cross-target clippy (`cargo clippy -p worktree --lib --tests
  --target x86_64-pc-windows-gnu -- -D warnings`): clean.
- `observation` filter run 8 times in a row on macOS: all green.
- `cargo nextest show-config test-groups --profile ci` lists the 18 macOS
  observer tests in `fsevents-l1`.

Cross-OS (`just cross-check worktree --os <os> observation`):

| OS | Result |
| --- | --- |
| macOS (local) | 29 observation tests passed (18 native, in `fsevents-l1`) |
| Linux (`build-linux`) | 25 passed |
| WSL2 (`build-win`) | passed |
| native Windows (`build-win-native`) | 18 passed (native tests assert `Unsupported`) |

Not run: `just test-l2` (no terminal code changed). Linux clippy cannot run
from this host (see Phase 6); the shared changes compiled warning-free on
build-linux and WSL2.

Phase 7 is complete; ready for review.

## Phase 8

Started and finished 2026-10-12. Phase 8 adds the Windows
`ReadDirectoryChangesW` adapter behind the Phase 6 observer contract.
`Observation::start` now succeeds on every supported OS. Nothing calls the
observer yet (Phase 10 does), so `wt remove`'s behavior is unchanged.

### What was built

| File | Contents |
| --- | --- |
| `worktree/lib/src/remove/observation/read_directory_changes.rs` | `ReadDirectoryChanges` (the `Backend`): two overlapped `ReadDirectoryChangesW` handles (parent with the subtree flag, 1 MiB buffer with a 64 KiB fallback for shares; admin directory, 64 KiB), read by one thread `wt-read-directory-changes` feeding `EventQueue`; `parse` (the `FILE_NOTIFY_INFORMATION` chain), `completion` (result → notifications, overflow, or ended watch), `kind`; `Shared::locate`/`deliver`; the per-directory marker barrier; identity checks; release and stop through two events; cancel-and-wait on drop |
| `worktree/lib/src/remove/observation/read_directory_changes/tests.rs` | Windows-only adapter tests |
| `worktree/lib/src/remove/observation/mod.rs` | Windows dispatch in `native_backend`; `EventQueue::take_loss` and the `EventQueue` dead-code allowance now cover Windows |
| `worktree/lib/src/remove/observation/tests.rs` | `NATIVE_ADAPTER` includes Windows; `METADATA_IS_DISTINCT` and `MOVE_ACROSS_DIRECTORIES_IS_A_RENAME`; a same-directory rename added to the barrier test; the admin replacing rename accepts the ReFS shape on Windows |
| `worktree/lib/Cargo.toml` | `windows-sys` 0.61 gains `Win32_Security`, `Win32_System_IO`, `Win32_System_Threading` (no new crate; `Cargo.lock` unchanged by this phase) |

### Decisions and departures

1. **Layout as planned.** One handle on the parent with the subtree flag,
   one on the admin directory. Paths arrive relative to the parent, so an
   entry named like the tree's current name is `Root::Tree`. The tree root's
   own entry stays the parent's entry, as on Linux and macOS. Sibling entries
   are dropped in `locate` before any work (no `stat`), and `EventQueue`
   classifies before counting, so a busy sibling worktree cannot fill the
   bound. Names match ASCII case-insensitively, as the file system matches
   them, and parent entries are reported under the session's own names.
2. **Marker drain, one marker per handle.** Delivery is FIFO per handle,
   not across handles, so a drain creates `.wt-observe-<16 hex>-<n>` in the
   parent and (until released) in the admin directory, then waits on a
   condition variable until each handle has delivered its marker. Each marker
   is opened with `FILE_FLAG_DELETE_ON_CLOSE` plus hidden/temporary
   attributes. Closing it deletes it, and so does a crash (the system closes
   the handle). Neither Git nor the checkout ever keeps one, which answers
   Phase 6's objection to markers in Git's admin directory. `DRAIN_TIMEOUT`
   (5 s) without a marker is `CoverageLost::DrainTimedOut`, the variant Phase
   7 added. A marker that cannot be created is `ReadFailed`. `start` runs one
   barrier, so a volume that accepts the call and never delivers (a share) is
   `ObservationError::Setup`, exit 4. A drain takes about 2 ms on
   build-win-native (the barrier test's ~55 drains took 0.12 s).
3. **Completion mapping.** A zero-byte completion and `ERROR_NOTIFY_ENUM_DIR`
   are `Overflow`, and the read is reissued. Any other completion error, or a
   failed reissue, is `WatchLost { root }`, and that handle stops. The
   session's own cancel happens only in `release_admin` and drop, where the
   thread cancels and waits itself, so `ERROR_OPERATION_ABORTED` reaching
   `complete` is someone else's doing and is loss too. A malformed buffer is
   `ReadFailed`.
4. **Identity checks, as on macOS.** A watch handle never reports its own
   directory's move. Each drain therefore compares the parent's and admin
   directory's `quarantine::rename::identity` (volume serial, 128-bit file ID)
   with start, before the barrier, so a vanished admin directory reports
   `WatchLost { Admin }` rather than a marker-write failure.
5. **Threading.** Reads are issued on the delivery thread, because Windows
   cancels a thread's outstanding I/O when that thread exits. It waits on
   `[stop, release, parent read, admin read]` with `WaitForMultipleObjects`.
   `release_admin` runs a barrier, signals `release`, and blocks until the
   thread has cancelled, awaited, and closed the admin handle. Drop signals
   `stop` and joins; each `Watch`'s drop cancels and awaits its read before
   freeing the buffer. The queue never waits, so drop cannot block.
6. **`directory` flag.** `FILE_NOTIFY_INFORMATION` carries no type. The
   adapter `stat`s the in-scope entry when reading a creation, change, or
   rename target, and reports `None` for removals. `ReadDirectoryChangesExW`'s
   extended information was not used, to keep one code path for ReFS, NTFS,
   and shares.
7. **Kinds (measured).** `FILE_ACTION_MODIFIED` covers content, attributes,
   and timestamps, so it is always `Modified`, and the shared barrier test
   accepts `Modified` for the read-only flip on Windows
   (`METADATA_IS_DISTINCT`). A move between two directories of the watched
   subtree is `REMOVED` plus `ADDED`; only a rename within one directory is
   `RENAMED_OLD_NAME`/`RENAMED_NEW_NAME` (`MOVE_ACROSS_DIRECTORIES_IS_A_RENAME`;
   the barrier test now also checks a same-directory rename everywhere). An
   unknown action is reported as `Modified` rather than dropped.
8. **ReFS differs from NTFS (found by rerunning the contract on `B:`).**
   build-win-native's temp directory is NTFS `C:`, while its checkouts live
   on the ReFS Dev Drive `B:`. A one-off rerun of every native contract test
   with fixtures under `CARGO_MANIFEST_DIR` (`B:`) found two differences.
   - A rename onto an existing file (`index.lock` → `index`) is `REMOVED` of
     the source plus `MODIFIED` of the target. The shared admin test accepts
     that shape on Windows. Either shape still reports both names, which is
     what the engine needs.
   - A directory deleted with POSIX semantics (`std::fs::remove_dir_all`)
     raises **no notification**, empty or not. Files deleted the same way, and
     directories deleted by `RemoveDirectoryW`, are reported, and the barrier
     holds. A foreign process that removes an already-empty directory this
     way is therefore not seen. Its file deletions are seen, and the deletion
     engine's per-entry revalidation (Phase 9) finds the directory gone.
     Pinned by `on_refs_a_directory_removed_with_posix_semantics_is_not_reported`,
     which runs where the package itself is on ReFS and skips with a message
     elsewhere. Recorded in the `os` skill and the spec's `message_to_agent`.
   With the widened admin assertion, all 8 native contract tests passed on
   `B:` as well as on `C:`.
9. **Delete-pending (the plan's question).** No. With the watch open
   (`FILE_SHARE_DELETE`), `RemoveDirectoryW` of the admin directory removed
   its name at once, and the same name could be created again, on NTFS `C:`
   and ReFS `B:`. The pending read then ends with an error, which is
   `WatchLost { Admin }`. `release_admin` still closes the handle first, so
   nothing depends on this. Recorded in the `os` skill.
10. **No self-exclusion entry.** Neither handle is inside the checkout, so
    `handles()` is empty and Sniff's usage query does not refuse
    (`sniff_sees_nothing_of_the_session`). This matches R12: only a directory
    handle inside the checkout would need registering by identity.
11. **No nextest group.** Windows stream starts do not serialize the way
    fseventsd registrations do. The slowest Windows observer test took 0.9 s
    (the real Sniff query), and the 1,000-file burst 0.8 s.

### Requirement-to-test mapping

All tests are L1 (no tier marker) in the `worktree` library binary. The
input robustness matrix does not apply: the only bytes parsed are the
kernel's `FILE_NOTIFY_INFORMATION` records, not a file format or
configuration. Their malformed shapes are still tested.

| Requirement (plan Phase 8) | Test |
| --- | --- |
| Phase 6 contract against `ReadDirectoryChangesW`: barrier for create, modify, attribute change, cross-directory and same-directory rename, directory create, remove (50 rounds, no sleep) | `observation::tests::native_drain_is_a_barrier_for_every_change_kind` |
| Parent subtree handle follows the move without blocking it; recreation seen | `observation::tests::native_coverage_follows_the_move_and_sees_recreation`; `read_directory_changes::tests::the_watch_does_not_block_the_move_or_the_deletion` |
| Siblings ignored; admin files reported (both replacing-rename shapes) | `observation::tests::native_siblings_are_ignored_and_admin_files_reported` |
| Intentional deletion and released-admin removal are not loss | `observation::tests::native_intentional_deletion_is_not_coverage_loss` |
| Deleted admin directory and moved parent are `WatchLost`, sticky | `observation::tests::native_losing_a_watched_directory_is_coverage_loss` |
| Bounded queue: a full queue is loss | `observation::tests::native_a_full_queue_is_coverage_loss` |
| Setup failures before anything changes (missing admin, missing original, a file, a directory link) | `observation::tests::native_setup_failure_is_an_error_before_anything_changes`; `read_directory_changes::tests::a_link_or_a_missing_directory_is_a_setup_failure` |
| Directory events stand for coalesced children (Windows does not coalesce) | `observation::tests::native_a_change_below_a_directory_is_reported_or_the_directory_event_stands_for_it` |
| **The overflow S2 showed `notify` hiding**: a burst while the thread is not reading is `Overflow`, not a short list | `read_directory_changes::tests::a_system_buffer_overflow_is_coverage_loss` (4 KiB buffer, test-only gate, 500 files) |
| A burst is complete or reported lost, never silently partial (production buffer) | `read_directory_changes::tests::a_burst_is_delivered_completely_or_reported_as_lost` |
| Zero-byte completion and `ERROR_NOTIFY_ENUM_DIR` are overflow; any other error (including an abort not the session's own) ends the watch | `read_directory_changes::tests::completions_map_to_events_overflow_or_an_ended_watch` |
| Notification buffers parsed entry by entry; overruns are errors, not shorter lists | `read_directory_changes::tests::notification_buffers_are_parsed_entry_by_entry` |
| Action → kind; `MODIFIED` never `Metadata`; unknown action still a change | `read_directory_changes::tests::actions_map_to_kinds_and_modified_is_never_metadata` |
| Labels before and after the move; case-insensitive names; siblings and `checkout2` dropped | `read_directory_changes::tests::paths_are_labeled_by_where_the_tree_is` |
| Markers never reported, nothing left behind | `read_directory_changes::tests::markers_are_never_reported_and_leave_nothing_behind` |
| Admin handle closed before the final unregister; delete-pending answer | `read_directory_changes::tests::a_watched_directory_deleted_before_release_and_one_released_first` |
| No thread left after the session | `read_directory_changes::tests::the_delivery_thread_ends_with_the_session` |
| No self-exclusion entry needed | `read_directory_changes::tests::sniff_sees_nothing_of_the_session` |
| ReFS: POSIX-semantics directory delete unreported, barrier holds | `read_directory_changes::tests::on_refs_a_directory_removed_with_posix_semantics_is_not_reported` |

Regression evidence: the first Windows run failed
`native_drain_is_a_barrier_for_every_change_kind` on the cross-directory
move (`Removed`/`Created` instead of a rename). The first ReFS run failed
`the_watch_does_not_block_the_move_or_the_deletion` (`remove_dir_all` left
no directory events) and `native_siblings_are_ignored_and_admin_files_reported`
(the replacing rename's shape). A probe test, since removed, separated
`remove_dir`, `remove_dir_all` of an empty and a full directory, and
`remove_file` on both volumes.

### Verification

Gates in `worktree/` on macOS, 2026-10-12, after the final change:

- `just test`: 1281 passed, 14 skipped (the area's existing skips), 0
  failed. Three unrelated tests (`graph::tests::fork_origin_cutoff_matrix`,
  two `list::tests::pipeline` tests) went over 5 s in the run that followed a
  fresh compile. Alone, they take 2.6–3.1 s; this phase touched no code they
  use.
- `just lint`: clean for `worktree` and `worktree-cli`.
- Windows cross-target clippy (`cargo clippy -p worktree --lib --tests
  --target x86_64-pc-windows-gnu -- -D warnings`): clean.

Cross-OS (`just cross-check worktree --os <os> [observation]`):

| OS | Result |
| --- | --- |
| native Windows (`build-win-native`) | `observation`: 31 passed (NTFS temp fixtures; the ReFS test exercised on `B:`); full `worktree` package: 655 passed |
| native Windows, fixtures on ReFS `B:` (one-off, reverted) | the 8 shared native tests and the adapter tests passed |
| Linux (`build-linux`) | `observation`: 25 passed |
| WSL2 (`build-win`) | `observation`: 25 passed |
| macOS (local) | in `just test` above |

Not run: `just test-l2` (no terminal code changed).

Phase 8 is complete; ready for review.

## Phase 9

Started and finished 2026-10-12. Phase 9 adds the operation-start deletion
inventory (persisted as journal version 2), the checked deletion engine, and
the final verification that ends in the worktree's unregister. Nothing calls
them from `wt remove` yet (Phase 10 does), so the command's behavior is
unchanged.

### What was built

| File | Contents |
| --- | --- |
| `worktree/lib/src/remove/observation/inventory.rs` | `DeletionInventory` (`capture`, `confirm`, `check`, `open_parent`, `get`, `root`, `entries`), `InventoryEntry { path, identity, shape }`, `Shape` (`File { size, modified, mode, content }`, `Directory { mode }`, `Link { target }`), `Content` (`Unhashed`, `Blake3`), `Timestamp`, `Ineligibility`, `Change`, `InventoryRefusal` (`Ineligible`, `Uninspectable`, `Changed`, `Coverage`). The persisted form is validated on read (plain relative paths, each once, parents first, BLAKE3 hex, non-empty NUL-free link targets). Shared `look`/`compare`/`open_root`/`dir_identity` used by the engine |
| `worktree/lib/src/remove/observation/deletion.rs` | `delete_content(observation, journal, interrupted, DeletionOptions) -> Deletion { deleted, externally_absent, stop }`; `Stop` (`Activity`, `Uncertain`, `CoverageLost`, `Interrupted`, `Io`, `Journal`) with `outcome()`; `DeletionOptions { drain_every, cumulative_kinds }`; `OperationReport::record_deletion`; `own_move`; the crate-internal `delete_content_with(.., before_batch)` test seam |
| `worktree/lib/src/remove/quarantine/unregister.rs` | `unregister_quarantine(base, observation, &IndexBinding)` and `UnregisterError` (`Activity`, `CoverageLost`, `IndexChanged`, `Registration`, `Refused`) with `outcome()` |
| `worktree/lib/src/remove/missing.rs` | `IndexBinding { head, index_digest, entries_digest }` with `matches`, and `bind_index(base, admin, head)` (reuses the existing index-bytes and `ls-files --stage --debug` digests) |
| `worktree/lib/src/remove/recovery/journal.rs` | `JOURNAL_VERSION` 2: a required `inventory` line after the header; `JournalLog::inventory`; `EntryKind::of`; batches must name inventory entries of the same kind (writer and reader); `JournalWriter::start(sink, id, Arc<DeletionInventory>, limit)`, `inventory()` |
| `worktree/lib/src/remove/recovery/store.rs` | `RecoveryStore::begin(lock, record, Arc<DeletionInventory>)` writes the inventory inside the staging directory; `Recovered::inventory`; `StoreError::InventoryMismatch` from `begin` and `recover` when the inventory's root is not the record's `checkout_identity` |
| `worktree/lib/src/remove/outcome.rs` | `REPORT_VERSION` 2: `deleted` (capped at `MAX_REPORTED_DELETED` = 200, `Truncation::deleted_omitted`), `notes: Vec<ReportNote>` with `ReportNote::ConcurrentWriteLimit`, `set_deleted` |
| `worktree/lib/src/remove/inventory.rs` | `Inventory::consent_sensitive_paths()` (dirty entries plus selected included files that need consent) |
| `worktree/lib/src/remove/quarantine/rename.rs` | `handle_identity` (Windows): the 128-bit file ID of an open handle, shared with `identity` |
| `worktree/lib/src/remove/observation/injected.rs` | `Injector::drains()` |
| `worktree/lib/src/remove/test_support.rs` | `link_dir`: a directory symlink, or on Windows a junction where symlinks need Developer Mode |
| `.config/nextest.toml` | the macOS `fsevents-l1` group now selects every `remove::…::native_*` test |

### Decisions and departures

1. **Drains follow depth levels, not journal batches.** The engine deletes
   one depth level at a time (deepest first, `.git` and anything below it
   after all other content). Journal batches stay as Phase 2 defined them:
   one directory, at most 256 entries. The observer is drained after each
   level and after every `DEFAULT_DRAIN_EVERY` (2,048) entries, not after
   every batch. A macOS drain costs ~11 ms (up to ~650 ms loaded), so a drain
   per batch would cost seconds for a large tree with many small
   directories. The plan's "drain and inspect exact affected paths between
   batches" is read as between deletion levels. What the spec needs from it
   still holds: a directory's children are drained before the directory is
   removed (Phase 7's FSEvents coalescing), the per-level bound keeps the
   observer queue (16,384) far from full, and every entry is re-checked
   immediately before its unlink. `drains_follow_depth_levels_and_the_bound_not_entries`
   pins the count.
2. **What the inventory compares.** Identity, type, size, modification
   time, and permission bits (Windows: the read-only, hidden, and system
   attributes) for files; identity, type, and permission bits for
   directories; identity and target for links; BLAKE3 only for
   consent-sensitive files (at or below a dirty path, so an untracked nested
   repository listed as `dir/` is hashed whole). Not compared: change time
   and link count, because deleting one hard link changes both on the
   other (Cargo hard-links `target/debug/<bin>` to `deps/`, which would have
   stopped every Rust checkout's removal), and a directory's modification
   time, which the operation's own deletions change.
3. **Windows identity is by path.** `rename::identity` (128-bit file ID,
   `FILE_FLAG_OPEN_REPARSE_POINT`) per entry, in capture and in
   revalidation; directory handles opened through cap-std are checked with
   the new `handle_identity`. Measured on build-win-native: about 0.5 ms per
   entry (a 600-file fixture took 1.2 s). Unix uses `fstatat` through the
   parent directory handle and costs nothing extra. Not measured on a large
   tree (R10: no performance spike scheduled); recorded here for the author.
4. **An entry found absent is recorded and stops.** The spec says an
   externally disappeared approved entry is recorded as already absent,
   never as deleted by `wt`, and that an unexplained disappearance stops the
   operation. The engine does both: the journal result is
   `externally_absent`, `Deletion::externally_absent` lists it, and deletion
   stops with `Vanished` activity. Retry (Phase 10) takes a fresh inventory of
   what survives, so no resume path needs "absent, continue".
5. **Event resolution is by exact path and state.** A path this operation
   deleted must still be absent, and its kinds must be `Removed` or
   `Metadata` (link count), or `Modified` for a directory (Windows reports a
   directory as modified when its children go). With
   `DeletionOptions::cumulative_kinds` (default on macOS), any history that
   includes `Removed` explains it. A write-like kind on a deleted file
   elsewhere is `Stop::Uncertain` (`state_unverified`): the spec's "could
   equally represent a concurrent write and cannot be resolved by checks".
   An approved entry not yet reached is re-checked through verified
   directory handles, and is explained only if it still matches. Anything
   else, any admin-file event, and anything at the original path except the
   operation's own move is activity.
6. **Found while running the native tests (macOS):** FSEvents reported the
   original path's own move with a `Created` flag from the fixture's
   creation moments earlier, so the engine stopped on its own move in 3 of 4
   native tests. `own_move` now applies the cumulative rule at the original
   too. Recorded in the `os` skill (`macos.md`).
7. **Outcomes of a stop.** `Activity` → `activity_detected`; `Uncertain`
   and `CoverageLost` → `state_unverified`; `Interrupted`, `Io`, and
   `Journal` → `cleanup_failed`. Interruption is not a failure as such, but
   the spec's five outcomes have no closer fit; Phase 10 may revisit this
   when it wires signals. Every stop after the move is exit 1 (R9).
8. **Final verification is split by concern.** `delete_content` ends with
   the filesystem half: the quarantine root is removed through a handle on
   the parent after an identity check (`rmdir` fails if anything reappeared,
   and the leftover names are reported), a final drain, and both paths
   confirmed absent. `unregister_quarantine` is the Git half: one more drain
   (admin events, recreation), a fresh listing showing the quarantine exactly
   once, `inspect_missing` re-bound to the `IndexBinding` taken before the
   move (index bytes, Git's reading of every entry including split-index
   files, and HEAD), `release_admin`, then `remove_missing_record` (plain
   `git worktree remove`, never `--force`).
9. **The move's own admin event is not the engine's to explain.** The
   association rewrites the admin `gitdir`, so the first drain after the
   move carries `Admin("gitdir")`, which the engine treats as activity.
   Phase 10's lifecycle must drain right after the move and confirm those
   bytes (the plan's "re-list and validate links before deleting") before it
   calls `delete_content`. The end-to-end test does that drain and asserts
   only own-move events arrived.
10. **Journal version 2.** The inventory is line 2, written with the header
    inside `begin`'s staging directory, so a committed operation always has
    one and a torn inventory can only be a corrupt file (refused). The
    journal size limit counts batch lines only: the inventory scales with the
    checkout, and counting it would make a large checkout's removal refuse to
    start. With ~150 bytes of batch lines per entry, the 64 MiB limit still
    covers about 440,000 entries; past that, deletion stops with
    `LimitReached`. Phase 10 may size the limit from the inventory.
11. **Report version 2.** `REPORT_VERSION`'s own rule is to bump with any
    change to the persisted shape; nothing has written a report yet.
12. **The copy record.** `remove_missing_record` deletes the copy record
    keyed by the path it is given, here the quarantine (which has none). The
    original's copy record is Phase 10's ("copy record removal only after
    verification").
13. **Not detected, as the spec states.** A change that keeps every
    inspected metadata field of a file that is not hashed, and raises no
    event, is not seen. The inventory test pins this as the stated limit
    rather than leaving it implicit.

### Requirement-to-test mapping

All tests are L1 (no tier marker) in the `worktree` library binary, compiled
by its unit-test target. `native_*` tests run against this platform's real
adapter (macOS group `fsevents-l1`).

| Requirement (plan Phase 9 / spec) | Test |
| --- | --- |
| Inventory after observation is active: every approved entry, ignored content included, parents first, identity, type, size, link target; links listed, never walked | `observation::inventory::tests::every_entry_is_recorded_parents_first_with_identity_and_metadata` |
| Hashes only for consent-sensitive paths, including a dirty directory's files; ignored output not read | `…inventory::tests::every_entry_is_recorded_…`, `…inventory::tests::a_file_below_a_sensitive_directory_is_hashed`, `remove::inventory::tests::collects_dirty_and_ignored_entries_from_a_real_worktree` (`consent_sensitive_paths`) |
| FIFO and socket are ineligible and never opened (marked sensitive, so a FIFO open would hang) | `…inventory::tests::a_fifo_or_socket_is_ineligible_and_never_opened` (Unix) |
| A link at the checkout root is refused | `…inventory::tests::a_link_at_the_root_is_refused` |
| Finishing consistency check: appended, same-size same-time rewrite of a hashed file, new file, deleted, replaced, directory replaced by a link | `…inventory::tests::the_finishing_check_refuses_each_kind_of_change_at_the_reported_path` |
| Unchanged reported paths pass; an unhashed metadata-preserving write is the stated limit; a changed time is caught | `…inventory::tests::an_unchanged_path_or_no_event_passes_and_unreported_metadata_preserving_writes_are_the_stated_limit` |
| A permission change is a modification | `…inventory::tests::a_permission_change_is_a_modification` (Unix) |
| Lost coverage refuses before mutation; a replaced root refuses | `…inventory::tests::lost_coverage_or_a_replaced_root_refuses` |
| Capture (including hashing) raises nothing the real observer's check refuses; a real late write is caught | `…inventory::tests::native_capturing_an_inventory_raises_nothing_the_check_refuses` |
| Inventory JSON round trip, write-read-write stable | `…inventory::tests::the_inventory_round_trips_through_json` |
| Journal v2: inventory line, ordering, cross-checks, every field shape (see the matrix below) | `recovery::store::tests::journal_robustness_matrix` |
| Batches name only inventory entries of their kind (writer) | `recovery::journal::tests::a_batch_names_only_inventory_entries_of_their_kind` |
| The limit counts batches only | `recovery::journal::tests::the_limit_refuses_an_intent_while_its_results_still_fit` |
| `begin` refuses an inventory of another checkout, saving nothing | `recovery::store::tests::begin_refuses_an_inventory_of_another_checkout` |
| Children first, `.git` last, batches ≤ 256 of one directory, both locations gone, a link's target untouched | `observation::deletion::tests::an_undisturbed_tree_is_deleted_children_first_with_git_last` |
| No per-entry drains or rescans: drains = 1 + levels + bound + 1 | `…deletion::tests::drains_follow_depth_levels_and_the_bound_not_entries` |
| Per-entry revalidation stops before deleting a modified, rewritten (hashed), or replaced entry, which is kept | `…deletion::tests::a_file_changed_without_any_event_is_kept_and_stops_deletion` |
| A reported new file stops at the next drain and is preserved | `…deletion::tests::a_reported_write_stops_at_the_next_drain` |
| An unreported new file stops when its directory cannot be removed | `…deletion::tests::an_unreported_new_file_stops_deletion_when_its_directory_cannot_be_removed` |
| Externally disappeared entry: recorded absent, never as deleted, and stops | `…deletion::tests::an_entry_deleted_by_someone_else_is_recorded_absent_and_stops` |
| Parent-directory handles: a descendant directory replaced by a link is never descended; the link target's files survive | `…deletion::tests::a_directory_replaced_by_a_link_is_never_descended` |
| Original-path recreation stops and is never touched (event, and found only by the final check) | `…deletion::tests::recreation_at_the_original_stops_and_is_never_touched`, `…deletion::tests::a_recreated_original_found_only_by_the_final_check_stops` |
| Overflow and lost watches stop with lost coverage; admin changes stop as activity | `…deletion::tests::lost_coverage_and_admin_changes_stop_deletion` |
| Own deletion and move events explained narrowly | `…deletion::tests::events_of_its_own_deletions_are_explained_by_path_kind_and_absence` |
| An ambiguous event stops as uncertain; cumulative kinds explain it | `…deletion::tests::a_write_like_event_on_a_deleted_file_is_uncertain_unless_kinds_are_cumulative` |
| Interruption at the next entry boundary, with durable batch results | `…deletion::tests::an_interrupt_stops_at_the_next_entry_boundary` |
| A journal write failure stops before the batch | `…deletion::tests::a_journal_that_cannot_record_an_intent_stops_before_its_batch` |
| The report names deleted content and states the concurrent-write limit | `…deletion::tests::the_report_names_deleted_entries_and_states_the_concurrent_write_limit` |
| Receipt times recorded | `…deletion::tests::received_times_are_recorded_for_reported_activity` |
| Native: own events explained on each OS | `…deletion::tests::native_a_tree_is_deleted_with_its_own_events_explained` |
| Native controlled writer: retained-cwd writes into the quarantine (Unix; the quarantine path on Windows, where that cwd blocks the move) stop with the file kept and remnants preserved | `…deletion::tests::native_a_writer_in_the_moved_checkout_stops_deletion_and_its_file_is_kept` |
| Native controlled writer: absolute-path recreation at the original stops; the recreated tree is untouched | `…deletion::tests::native_recreation_at_the_original_path_stops_deletion` |
| Native overflow (full queue) stops with lost coverage | `…deletion::tests::native_a_full_queue_stops_with_lost_coverage` |
| End to end (acceptance items 3 and 4 at library level): dirty and ignored content inventoried and deleted, only this worktree unregistered, branch kept, other worktree untouched, one `git worktree remove` without `--force`, no `prune`/`repair`/`move` | `quarantine::unregister::tests::native_a_checked_removal_deletes_the_content_then_unregisters_only_this_worktree` |
| Index and HEAD re-bound before unregistering | `…unregister::tests::an_entry_staged_after_approval_keeps_the_registration`, `…unregister::tests::a_branch_moved_after_approval_keeps_the_registration` |
| Both locations and admin checked by the last drain; lost coverage keeps the registration | `…unregister::tests::a_recreated_original_keeps_the_registration`, `…an_index_write_keeps_the_registration`, `…an_unreported_recreated_quarantine_keeps_the_registration`, `…lost_coverage_keeps_the_registration` |

Mutation checks on macOS, each restored afterwards: skipping the index
re-bind failed `work_staged_or_committed_after_approval_keeps_the_registration`
(since split into the two tests above); skipping per-entry revalidation
failed `a_file_changed_without_any_event_is_kept_and_stops_deletion` and
`the_report_names_deleted_entries_and_states_the_concurrent_write_limit`.

Not covered by a test: a nested mount (needs root to mount inside a temp
directory) and a Windows reparse point other than a link or junction (needs
a cloud-files or deduplication provider). Both are code paths
(`Ineligibility::MountPoint` by device number on Unix and by a volume target
on Windows; `Ineligibility::ReparsePoint`).

### Input robustness matrix (journal version 2)

One test, `recovery::store::tests::journal_robustness_matrix`, walks every
cell from a journal the store itself wrote, one edit per cell, asserting
through `RecoveryStore::recover`. The control row (the unedited journal)
recovers `d/a` deleted, `d/b` kept, `e/x` uncertain, with the inventory as
written. The fixture inventory puts a hashed file first, so the
first-occurrence duplicate-key edit lands on the edited member.

| Shape | intent `path` | intent `kind` | results `state` | inventory `root` | inventory `entries` | entry `path` | entry `identity` | entry `shape` | file `size` | file `modified` | file `mode` | file `content` | link `target` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| absent | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid |
| null | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid |
| wrong type | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid (`"5"`) | invalid | invalid (`"644"`) | invalid | invalid |
| one element wrong type | invalid (entries) | — | — | — | invalid | — | — | — | — | — | — | — | — |
| every element wrong type | invalid (entries) | — | — | — | invalid | — | — | — | — | — | — | — | — |
| empty | invalid (`{}`; batch `[]`) | invalid | invalid | invalid | invalid (`[]`: batches then name unlisted entries) | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid |
| duplicate key | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid | invalid |
| invalid content | invalid (`../d/a`) | invalid (`socket`) | invalid (`vanished`) | invalid (bad device) | invalid (entry without fields) | invalid (`/a`) | invalid (`plan9`) | invalid (`socket`) | invalid (-1) | invalid (negative nanos) | invalid (-1) | invalid (`zz` hash) | invalid (NUL) |

Line-level cells: inventory absent, header only, repeated, after an intent,
torn, trailing garbage, unknown member (line and entry), a path twice, a child
before its directory, a root other than the record's checkout (all invalid);
an intent naming an unlisted entry or another kind (invalid); the Phase 2
cells (header absent/repeated/other version/other operation, blank line,
trailing content, results for the wrong batch or misaligned) re-pointed at
version 2's line numbers (all invalid). Valid: header plus inventory with no
batches, torn intent, torn results, and the inventory re-spaced. Smell grep
over the new readers: no `#[serde(default)]`, no `Option` field, no
`unwrap_or_default`/`.ok()` on a load-bearing parse (the one `unwrap_or_default`
in `deletion.rs` lists leftover names for a report after `rmdir` already
failed).

### Verification

Gates in `worktree/` on macOS, 2026-10-12, after the final change:

- `just test`: 1320 passed, 14 skipped (the area's existing skips), 0
  failed. The first full run had no slow tests (25 s). Two later runs, taken
  while the host's load average was 121 from other work, marked 3–7 tests
  that this phase does not touch as slow (`list::tests::pipeline` tests,
  `graph::tests::fork_origin_cutoff_matrix`, two `worktree-cli::remove` tests,
  `remove::missing::tests::a_split_index_whose_shared_file_changes_after_inspection_refuses`).
  Alone, those take 1.4 s.
- `just lint`: clean for `worktree` and `worktree-cli`.
- Windows cross-target clippy (`cargo clippy -p worktree --lib --tests
  --target x86_64-pc-windows-gnu -- -D warnings`): clean.
- Native engine tests run 3 times in a row on macOS: all green.

Cross-OS (`just cross-check worktree --os <os> remove::`):

| OS | Result |
| --- | --- |
| macOS (local) | in `just test` above |
| Linux (`build-linux`) | 236 passed |
| WSL2 (`build-win`) | 236 passed |
| native Windows (`build-win-native`) | 228 passed (after splitting the unregister cases; see below) |

Windows findings and pre-existing items:

- The first Windows run had the unregister failure cases as two
  multi-case tests taking 4.6 s and 7.3 s. They are now one test per case
  (about 1.5–2.7 s each on Windows). The 3,000-file drain test took 2.8 s
  and now uses 600 files (1.2 s) with the same assertions.
- In that first run, the unrelated
  `remove::safety::tests::a_pr_counts_only_for_the_same_repository_and_exact_tip`
  failed once (4.2 s, under the load of the slow tests above); it passes
  alone (2.0 s) and in the rerun.
- Pre-existing and not touched by this phase:
  `remove::missing::tests::a_split_index_whose_shared_file_changes_after_inspection_refuses`
  takes 7.1 s alone on native Windows (14.3 s in the package run). It is over
  the 5 s budget there and should be looked at separately.
- ReFS (`B:`) was not exercised. Phase 8 found file deletions and
  `RemoveDirectoryW` reported there, and only POSIX-semantics directory
  deletes unreported; which call cap-std makes was not verified. Either way,
  the per-entry revalidation and the final `rmdir` do not depend on events,
  so an unreported deletion can stop a removal but never make it delete
  something unapproved.

Not run: `just test-l2` (no terminal code changed).

Phase 9 is complete; ready for review.

## Phase 10

Started and finished 2026-10-12. Phase 10 adds `remove::lifecycle`, which
runs one removal through every Phase 2-9 piece in the decided order, plus
crash recovery (`inspect`), retry resolution (`resolve_target`), and resume
(`resume`, `release_unstarted`). Everything is library-level: `wt remove`
still calls `remove_worktree`, so the command's behavior is unchanged until
Phase 11 wires it.

### What was built

| File | Contents |
| --- | --- |
| `worktree/lib/src/remove/lifecycle.rs` | `remove_checkout(&RemovalRequest, &Environment) -> Removal`; `Approval`, `consent_state`, `BranchActions` (`LocalBranchDeletion`, `RemoteBranchDeletion`), `Environment` (usage source, interrupt poll, observer starter), `native_observation`, `Refusal` with `exit_class()`, `Finished`, `Removal`, `ExitClass` (0/1/3/4), `Presence`; the crate-internal `run_with(.., hook)` crash seam (`Boundary`, `Crashed`); the shared `Attempt` that deletes, unregisters, runs branch actions, completes, or stops with the actual state |
| `worktree/lib/src/remove/lifecycle/resume.rs` | `inspect(base, &id) -> Inspection` with `Resolution` (`NotStarted`, `MovedUnassociated`, `Quarantined`, `PartialCheckout`, `ContentRemoved`, `CheckoutRemoved`, `OriginalRecreated`, `Unresumable`); `release_unstarted`; `resume(&ResumeRequest, &Environment)` with `RecordApproval` |
| `worktree/lib/src/remove/lifecycle/resolve.rs` | `resolve_target(base, name) -> Target` (`Live`, `Pending`, `Ambiguous(Vec<Candidate>)`), `ResolveError` |
| `worktree/lib/src/remove/lifecycle/tests.rs` | 36 tests (one `#[ignore]` subprocess fixture besides) |
| `worktree/lib/src/remove/recovery/store.rs` | `RecoveryStore::reopen` (archives `journal.jsonl` as `journal-<n>.jsonl` and starts a new journal with a fresh inventory), `Operation::advance_to`, `Operation::journal_entries`, `Operation::discard`; `complete` also deletes archived journals |
| `worktree/lib/src/remove/quarantine/association.rs` | `verify_quarantined` (the moved checkout is still Git's worktree at the quarantine: identity, back-reference, `.git` link, listed once, unlocked, no submodules) and `MovePlan::back_reference_rewritten` |
| `worktree/lib/src/remove/observation/{mod,injected,inotify,fsevents,read_directory_changes}.rs` | `Observation::start_at_quarantine` and `Observation::injected_at_quarantine`: each adapter can start with the tree already at the quarantine (a resumed removal) |

### Order and decisions

1. **Order (spec decision 8):** `plan_move` → no pending record for this
   admin name → observer start → `DeletionInventory::capture` →
   `register_handles` + `check_usage` → recheck the tip and the consent
   fingerprint (`consent_state`, the same comparison as the CLI's
   `files_unchanged`) → drain and confirm → `bind_index` → `begin` → drain and
   confirm again → move → explain the move's own events → delete → unregister
   → original's copy record → branch actions → `complete`.
2. **Two drains before the move.** The recheck runs `git status`, which may
   rewrite the index, so its admin events are drained (and allowed) before
   `bind_index`. After `begin`, a last drain refuses any tree change and any
   admin event (exit 3, nothing moved). The plan's single "binding rechecks"
   step is split this way so the index binding never covers an event it did
   not see.
3. **A refusal after `begin` that changed nothing deletes the operation
   directory** (`Operation::discard`), so no record describes a removal that
   never happened. If deleting it fails, `inspect` resolves the leftover as
   `NotStarted` and `release_unstarted` closes it.
4. **Exit classes before the move** (spec decision 7 and R9): usage, an
   unverified checkout, ineligible topology or content, changed approved
   state, missing consent, an uninspectable index, a pending or unreadable
   record → 3. Observation unavailable, lost coverage before the move, a store
   that cannot be created or written, another `wt` holding the lock, a
   Windows sharing failure, an unsupported no-replacement rename → 4. Any
   other rename failure, or a rolled-back association → 1, nothing changed.
   After the move every stop is 1. A verified removal whose branch action
   failed is outcome `removed` with exit 1.
5. **The move's own events** are explained by path and state, not kind:
   `Original("")` with `own_move` kinds and the original absent;
   `Quarantine("")`/`Tree("")` with the quarantine holding the checkout's
   identity; `Admin("gitdir")` holding exactly the rewritten bytes. Anything
   else (or lost coverage) stops after the move.
6. **Stops report the actual state,** read fresh: both paths by identity
   (`Presence`), the registration from the admin `gitdir`, what is left in the
   quarantine (exact count, capped list), the journal summary (its
   unconfirmed entries checked through a verified quarantine handle), every
   intended branch action as `not_attempted`, and a diagnostic Sniff query of
   each location that is a real directory (never through a link). A failed
   diagnostic query is recorded as `failed` and changes nothing else.
7. **Recovery reads the filesystem and Git, never the saved phase.** The
   resolution table is in the `worktree` skill's `lifecycle.md`. Two
   resolutions are additions to the plan's wording:
   - `MovedUnassociated` (the process died between the rename and the
     `gitdir` rewrite) is resolved by moving the checkout back with the
     no-replacement rename and verifying it, which is the spec's rollback rule
     (original absent, nothing deleted). Nothing else is ever moved back.
   - `PartialCheckout` (the moved checkout's `.git` link is gone) is never
     resumed: the inspection lists what remains and nothing is repaired, as
     the spec requires for a partial checkout Git cannot read.
   An admin directory whose identity changed under the recorded name (Git
   reused the name) is `Unresumable`.
8. **A retry reopens the same operation.** `RecoveryStore::reopen` keeps the
   journal so far as `journal-<n>.jsonl` (no confirmed entry is discarded) and
   starts a new journal whose inventory is a fresh capture of what survives.
   Departure from a literal reading of "Persist ... completed phase": the
   record's `consent_fingerprint`, `approved_tip`, and `discovery` keep the
   first attempt's values; they describe what was decided, and the retry's
   fresh approval is checked in memory and saved in its report. Completion
   deletes every journal.
9. **Retry needs fresh approvals for each resolution:** `Quarantined` takes an
   `Approval` read from the quarantine (`consent_state(base, quarantine,
   branch)`), a new usage query of the quarantine, a fresh inventory, the
   recheck, and a new index binding. `ContentRemoved` takes a `RecordApproval`
   (the `inspect_missing` result shown to the user) and goes through
   `remove_missing_record`. `CheckoutRemoved` runs only the branch actions in
   the request. A saved remote approval or expired handoff is never reused
   (the record holds none).
10. **Resolution by name** matches live registrations as
    `resolve_worktree` does and pending records by `requested_name`, aliases
    (branch and directory name), and the original's directory name. A live
    entry is the same target as a pending record only at the record's
    quarantine, or at the original with the recorded identity. Anything else
    matched twice is `Ambiguous`. `base` is never pending. An unreadable
    record matches nothing, but `remove_checkout` refuses while one exists.
11. **Interruption stays `cleanup_failed`** (Phase 9 decision 7): nothing
    installs a signal handler yet. `Environment::interrupted` is the hook
    Phase 11 can connect.
12. **The journal limit is not sized from the inventory.** The default 64 MiB
    still covers about 440,000 entries per attempt, and a retry starts a new
    journal. Left as is; no checkout in tests approaches it.

### Requirement-to-test mapping

All tests are L1 in the `worktree` library binary
(`remove::lifecycle::tests::…`). `native_*` tests use this platform's real
observer; the rest use the injected observer and a fake `UsageSource`.

| Requirement (plan Phase 10 / spec acceptance) | Test |
| --- | --- |
| Whole lifecycle with the real observer: dirty and ignored content deleted, only this worktree unregistered, other worktree untouched, branch after the checkout, copy record removed after verification, one decisive query and no discovery chatter, record completed and journal deleted, no `--force`/`prune`/`repair`/`move`/`unlock` (items 2, 3) | `native_a_removal_runs_end_to_end_and_closes_its_record` |
| Partial, unavailable, unsupported discovery remove and keep coverage in the report (item 2) | `partial_discovery_…`, `unavailable_discovery_…`, `unsupported_discovery_still_removes_and_is_kept_in_the_report` |
| Usage refuses before anything moves, consent does not override, no record created (item 1) | `observed_usage_refuses_before_anything_moves` |
| Root-validation error refuses (item 2) | `an_unverified_checkout_refuses` |
| Consent required; files changed after approval; tip moved after approval (item 7) | `files_that_need_consent_refuse_without_it`, `files_changed_after_approval_refuse`, `a_tip_moved_after_approval_refuses` |
| Unavailable observation is exit 4 before discovery | `unavailable_observation_is_an_environment_limit` |
| Locked worktree refuses (item 5) | `a_locked_worktree_refuses` |
| Change between `begin` and the move refuses, record discarded | `a_change_just_before_the_move_refuses_and_discards_the_record` |
| Unexplained admin activity during the move stops after it: exit 1, both paths, registration, branch kept, record pending, report saved (items 4, 8) | `unexplained_activity_during_the_move_stops_after_it` |
| Write into the quarantine mid-deletion: stop, new file kept, deletions so far and remaining entries reported, concurrent-write note, no branch action, diagnostic query of the quarantine (items 3, 8) | `a_write_into_the_quarantine_during_deletion_stops_with_an_honest_report` |
| Recreation at the original stops; the recreated tree is untouched; diagnostic query of it (items 3, 7) | `recreation_at_the_original_path_stops_and_is_left_alone` |
| Lost coverage after the move is `state_unverified`; a failed diagnostic query is recorded (item 4) | `lost_coverage_after_the_move_stops_and_a_failed_diagnostic_is_recorded` |
| Interruption after the move preserves remnants; retry finishes (item 11) | `an_interrupted_removal_is_resumed` |
| Branch failure keeps the record and stops later actions; retry by original name with fresh approvals (items 6, 11) | `a_failed_branch_deletion_keeps_the_record_and_a_retry_needs_fresh_approval` |
| Crash after `begin`: nothing moved, new run refused while pending, name resolves to the pending record | `a_crash_after_the_record_leaves_nothing_moved_and_blocks_a_new_run` |
| Releasing an unstarted record lets a new removal run | `a_released_unstarted_record_lets_a_new_removal_run` |
| Crash between rename and association: moved back, never deleted | `a_crash_between_the_rename_and_the_association_is_moved_back` |
| Crash after the move: retry by original name, branch, or quarantine basename reaches the pending record; fresh inventory and consent; finishes (items 6, 10) | `a_crash_after_the_move_is_resumed_by_the_original_name` |
| New content needs fresh consent; missing consent refuses (items 6, 11) | `new_content_in_the_quarantine_needs_fresh_consent` |
| Usage of the quarantine refuses a retry | `usage_of_the_quarantine_refuses_the_retry` |
| Changed identity: a replaced quarantine is never resumed or inspected (items 6, 12) | `a_replaced_quarantine_is_never_resumed` |
| Partial checkout without `.git` is reported, never repaired (item 10) | `a_partial_checkout_without_its_git_link_is_kept` |
| Crash after content removal: record removal after index re-check; a crash between unlink and result is uncertain, not deleted (item 11) | `a_crash_after_the_content_is_removed_resumes_with_the_record_and_uncertain_entries` |
| Crash after unregistering: retry runs only the branch actions | `a_crash_after_unregistering_runs_only_the_branch_actions_on_retry` |
| Completed records never authorize a second removal (item 11) | `a_completed_record_never_authorizes_a_second_removal` |
| Only a held lock blocks; a stale lock file does not (items 11) | `only_a_held_lock_blocks_a_retry`, `native_a_killed_removal_…` |
| Malformed record blocks a new removal (item 11) | `an_unreadable_record_blocks_a_new_removal` |
| Record write failure after the move stops before deleting and reports the last durable phase (item 11, Unix) | `an_unwritable_record_stops_before_deleting` |
| A new worktree at the original path is ambiguous with the pending removal, and neither tree is cleaned up (item 10) | `a_new_worktree_at_the_original_path_is_ambiguous_and_never_cleaned_up` |
| A reused branch is ambiguous with the pending removal (item 10) | `a_reused_branch_is_ambiguous_with_the_pending_removal` |
| Unknown names, `base`, and an ordinary live name | `names_that_match_nothing_are_not_found_and_base_is_never_pending` |
| Real process termination mid-deletion: lock released, found by original name, journal keeps confirmed deletions, resumed with the real observer starting at the quarantine (items 6, 11) | `native_a_killed_removal_is_found_and_resumed_by_its_original_name` (child: `lifecycle_child_blocks_mid_deletion`, `#[ignore]`, run with `--exact --ignored --nocapture` as Sniff's subprocess fixtures are) |

Mutation checks on macOS, each restored afterwards: skipping the recheck
failed `files_changed_after_approval_refuse` (the changed file was deleted)
and `a_tip_moved_after_approval_refuses` (caught only by the unregister's
index binding, after deletion); ignoring the move's own events failed
`unexplained_activity_during_the_move_stops_after_it`; disabling the
same-operation dedupe failed `a_crash_after_the_move_is_resumed_by_the_original_name`
and `a_crash_after_the_record_leaves_nothing_moved_…`.

Not covered by a test: a crash inside `move_to_quarantine` between the rename
and the `gitdir` write is reached by renaming by hand after a crash at
`Begun`, since the association seam (`Faults::after_rename`) is not exposed to
the lifecycle; a crash inside a deletion batch between an unlink and its
result is reached by dropping the journal's last results line.

### Verification

Gates in `worktree/` on macOS, 2026-10-12:

- `just test`: 1356 passed, 15 skipped (the area's 14 existing skips plus the
  new `#[ignore]` subprocess fixture), 0 failed, no slow tests.
- `just lint`: clean for `worktree` and `worktree-cli`.
- Windows cross-target clippy (`cargo clippy -p worktree --lib --tests
  --target x86_64-pc-windows-gnu -- -D warnings`): clean.
- `cargo doc -p worktree --no-deps`: no warning in the new or changed files.

Cross-OS (`just cross-check worktree --os <os> remove::`):

| OS | Result |
| --- | --- |
| macOS (local) | in `just test` |
| Linux (`build-linux`) | 273 passed |
| WSL2 (`build-win`) | 273 passed |
| native Windows (`build-win-native`) | 263 passed (after the split below) |

Windows timing: the first native Windows run had
`incomplete_discovery_still_removes_and_is_kept_in_the_report` (a loop over
three fixtures) at 9.2 s and the begun-crash test at 5.0 s. Both were split
into one test per case, and the fixture now writes the common `info/exclude`
instead of committing a `.gitignore` and creates the neighbor worktree only
for the end-to-end test. After that the slowest lifecycle tests take 4.0–4.7 s
under the package's parallel load there (the retry tests run two lifecycles).
One lifecycle run makes 22 Git calls; four of them are
`unregister_quarantine`'s `inspect_missing` followed by
`remove_missing_record` re-reading the same listing, HEAD, and index digest.
That redundancy is in Phase 9's unregister and is a candidate for removal if
these tests creep past 5 s on CI.

Not run: `just test-l2` (no terminal code changed).

Phase 10 is complete; ready for review.

## Phase 11

Started and finished 2026-10-12. `wt remove` now removes every checkout that
exists through `remove::lifecycle`, finds a stopped removal again by the name
the user typed, and prints compact stop, recovery, and ambiguity reports on
stderr. The old `git worktree remove` path is gone.

### What was built

| File | Contents |
| --- | --- |
| `worktree/cli/src/commands/remove/mod.rs` | `run` resolves with `resolve_target` (pending → `inspect` → release, retry, or refuse), fingerprints every existing checkout, prints a "Continuing an earlier removal" lead for a retry; `execute` calls `remove_checkout` or `resume` (`execute_record_only` keeps the missing-directory path); `branch_actions`, `report_finished`, `branch_messages`, `with_note`; `Facts` gains `requested_name`, `retry`, `branch_only` (`Facts::branch_only` for `CheckoutRemoved`); the handoff's second run sets `Facts::reported` after `verify` and runs the same `execute` |
| `worktree/cli/src/commands/remove/recovery.rs` (new) | `refusal_error` (exit class → `RefusedToLoseWork`/`BlockedByEnvironment`/`RemovalFailed`), `refusal_markup`, `stopped_markup` (both locations, deleted/possibly deleted/remaining counts, observed activity with receipt times, registration, per-branch kept/already deleted/not deleted, retry command, base checkout, saved report path, warnings), `inspection_markup`, `ambiguous_markup` |
| `worktree/cli/src/commands/remove/report.rs` | `ReportInput::checkout_removed` and its files line |
| `worktree/cli/src/commands/remove/usage.rs` | `visible` is public to the module tree; the `#[allow(dead_code)]` on the module is gone |
| `worktree/lib/src/error.rs` | `WorktreeError::RemovalFailed(markup)` (exit 1); `DirectoryInUse`, `FolderNotFullyRemoved`, `LockProbeRenameBack`, `NotARealDirectory` and their message helpers removed |
| `worktree/lib/src/remove/mod.rs` | `remove_worktree`, `check_no_processes`, `is_registered`, `finish_unregistered`, `check_not_in_use` and their tests removed (R8: nothing calls them) |
| `worktree/cli/src/main.rs`, `exit.rs` | `RemovalFailed` printed as markup; exit mapping without the removed variants |

### Decisions and departures

1. **Exit 1 with markup is a new error variant**, `WorktreeError::RemovalFailed`,
   rather than printing and calling `process::exit`. `main.rs` prints it like
   the other markup variants. A pre-move operational failure
   (`ExitClass::Failure` from a `Refusal`) uses it too, with the
   "Nothing was removed." lead.
2. **The usage check moved after the questions**, so a process standing in
   the worktree is now refused after the report with exit 3 (it was exit 4,
   before the report). The plan and spec require this order; the old test
   `a_process_standing_in_the_worktree_blocks_removal_before_the_report` was
   rewritten as `..._refuses_after_the_questions_with_exit_3`. On Windows the
   same case is now exit 3 too: Sniff reports the held directory as an open
   handle (`PING.EXE`), so the rename never runs. The Windows L1 and L2
   PowerShell tests were updated to the usage refusal.
3. **A retry is never handed off.** A caller standing inside a pending
   removal's quarantine gets exit 4 and the instruction to run from the base
   checkout. The spec says "run recovery from the base checkout"; a handoff
   record has no field for a pending operation, and adding one is not needed
   for the stated behavior.
4. **A retry never repairs.** An unlinked quarantine refuses before `prepare`
   (whose `Unlinked` branch would run `git worktree repair`). `inspect`
   already resolves that state as `PartialCheckout`; this guards the race.
5. **`CheckoutRemoved` retries** build a synthetic `WorktreeEntry` at the
   original path with the record's branch; a branch that no longer exists is
   treated as detached (nothing to delete), so the retry can still close the
   record.
6. **Remote states other than `Present`** are still the CLI's: after a
   verified removal, `Unavailable` exits 1 with the old
   `git push origin --delete` guidance, `Absent`/`NoRemote` print "Nothing to
   delete on origin.", and the lifecycle receives a remote action only for
   `Present`.
7. **A branch failure after the checkout was removed** is reported by
   `stopped_markup` with "Removed worktree X …, but a branch step did not
   finish", each branch as kept, already deleted, or not deleted (Git's first
   error line), and "Retry: wt remove X". The old `git push origin --delete`
   hint for a failed lease is replaced by the retry, which closes the pending
   record.
8. **Interruption is not wired** (`Environment::interrupted` is `|| false`):
   `worktree-cli` has no signal-handling dependency, and adding one is a
   dependency decision outside this phase's tasks. Ctrl-C still kills the
   process, which the crash recovery handles (Phase 10).
9. **Docs:** the topic page's former "Planned" section is now in the present
   tense without the banner, and the sections it contradicted ("Background
   programs writing into the worktree", the handoff's usage paragraph, the
   exit-code table) describe current behavior. The README bullet on folders
   other programs use is rewritten. Phase 12's full rewrite of that page is
   still to do.

### Requirement-to-test mapping

| Requirement (plan Phase 11 / spec acceptance) | Test |
| --- | --- |
| Usage refuses through the real binary after the questions, exit 3, PID and evidence, no force flag bypasses, nothing kept in the store, stdout empty (item 1) | `remove::a_process_standing_in_the_worktree_refuses_after_the_questions_with_exit_3` (Unix); Windows: `powershell_wrapper_exec::a_directory_held_by_another_program_refuses_with_exit_3_and_nothing_removed`, `level2_powershell_remove::level2_powershell_refuses_a_worktree_another_program_holds_with_exit_3_and_nothing_removed` |
| Ordinary removal goes through the lifecycle with no discovery chatter, no quarantine left, record completed (item 2) | `remove::example_clean_and_merged_removes_worktree_and_branch` and every existing success test in `remove.rs` |
| Write into the moved checkout: exit 1, both locations, remaining, registration at the moved path, branch kept, saved report parsed, no protocol line; retry by original name lists the new file and refuses without consent (exit 3); with `--force-worktree` finishes, deletes the branch, closes the record (items 3, 6, 8, 11) | `remove_recovery::a_write_into_the_moved_checkout_stops_then_a_retry_needs_fresh_consent` |
| Recreation at the original path: exit 1, original marked recreated and untouched; retry refuses (exit 3) with nothing deleted; after the user moves it, retry finishes (items 3, 7, 8, 10) | `remove_recovery::recreation_at_the_original_path_stops_and_is_never_cleaned_up` |
| A new worktree at the original path is ambiguous with the pending removal; both listed and untouched (item 10) | `remove_recovery::a_new_worktree_at_the_original_path_is_ambiguous_with_the_pending_removal` |
| Branch failure after removal: kept vs already deleted, retry, report path, record pending, stdout empty (items 6, 8) | `remove::a_push_after_the_second_run_preflight_fails_the_lease_with_a_partial_result`, `remove::a_branch_that_moves_after_its_worktree_is_removed_is_kept` |
| Wrapper first run hands off despite the parent shell in the worktree; the second run removes (item 9) | `remove::the_parent_shell_inside_does_not_block_the_handoff_and_the_second_run_removes` |
| A different process inside at the second run refuses there, exit 3, no `cd:`/`remove-handoff:` (item 9) | `remove::a_process_still_inside_after_the_handoff_refuses_the_second_run` |
| Exit 4 paths unchanged (no wrapper, expired/missing token, shell still inside) | existing `remove::standing_inside_without_the_wrapper_exits_4_and_removes_nothing`, `an_expired_token_refuses_with_exit_4`, `a_missing_or_malformed_token_refuses_with_exit_4`, `a_caller_still_inside_the_target_is_refused_with_exit_4` |
| Stdout protocol unchanged | existing `wrapper_protocol.rs`, `shell_wrapper_exec.rs`, `powershell_wrapper_exec.rs` |
| Missing directory keeps record-only removal, no Sniff (item 10) | existing missing-directory tests in `remove.rs` (they pass through `execute_record_only`) |
| Report rendering: the spec's example, branch failure after removal, untrusted text escaped (markup, control characters, multi-line errors), listing limits and uncertain entries, refusals' exit codes 3/4/1, ambiguity (item 8) | `remove_recovery_report::*` (6 snapshot tests, integration binary per the double-target trap) |

The writers in `remove_recovery.rs` are `git` shims that fire once a
`.wt-remove-*` sibling exists. A writer at `git worktree remove` or
`diff-index` was tried first and let the removal succeed: the last check of
both paths is at the start of `unregister_quarantine`, before those calls, so
such a write is the documented concurrent-write limit, not a stop.

### Verification

Local (macOS), in `worktree/`:

- `just test`: 1358 passed, 15 skipped, 0 failed.
- `just test-l2`: 41 passed (the slow ones are the existing Kitty graph
  tests).
- `just lint`: clean. Windows cross-target clippy
  (`-p worktree -p worktree-cli --lib --tests`, and `worktree-cli --tests
  --features terminal-tests`): clean.

Cross-OS (`./scripts/cross-check.sh worktree-cli --os <os> --build-jobs 2`):

| OS | Result |
| --- | --- |
| Linux (`build-linux`) | 609 passed (first attempts: linker OOM-killed, then the archive-mode `CARGO_BIN_EXE_wt` path; both fixed) |
| WSL2 (`build-win`) | 609 passed |
| native Windows (`build-win-native`) | 574 passed; L2 PowerShell (`--features terminal-tests level2_powershell`) 3 passed |

### Findings that need attention

- **Windows Search holds fresh files.** In one Windows run, two `example_*`
  tests were refused (exit 3) because `SearchProtocolHost.exe` had an open
  handle on a file the test had just written under `%TEMP%`, which the index
  scope excludes. 14 immediate reruns passed. This is the spec's
  conservative rule working as written, but it makes Windows CI flaky and
  will sometimes pause a user's removal. Recorded in the `os` skill and raised
  for human review in the spec.
- **Windows cost per removal.** The same six `example_*` tests take 2.3–3.5 s
  on `build-win-native` before this phase and 3.0–5.3 s after it, about
  1.5–2 s more per removal. Under the full package run, many `remove.rs` tests
  take 5–8 s there, although untouched `list_prs` tests reach 12 s in the
  same run, so the host was loaded. The L1 budget is 5 s; CI runs Windows on
  every push to `main`. The lifecycle's redundant Git calls noted in Phase 10
  (`inspect_missing` then `remove_missing_record`) are the first candidate.

Not run: WSL2 and Linux L2 (no terminal-facing change beyond the stderr
report).

Phase 11 is complete; ready for review.

## Phase 12

Started and finished 2026-10-12. Docs, skills, the dependency catalog, the
acceptance traceability table, and cross-OS validation. No source code
changed in this phase.

### What was done

| File | Change |
| --- | --- |
| `worktree/docs/cli/remove.md` | Restructured for a reader new to the repo: it now opens with what `wt remove` does and the usage examples (the former "Current behavior" heading is gone), then the report, consent, usage pause, and handoff, then three new top-level sections: **Deleting the checkout** (move-then-delete flow, its limits, Git association, then "Watching for changes" and "Checked deletion" subsections), **What `wt remove` refuses to move** (eligible topologies and filesystem limits, including WSL `/mnt/<drive>` exit 4), and **Recovery records and retry** (the `<git-common-dir>/wt-removal/<operation-id>/` files, retrying from outside the moved checkout or the base checkout, the never-handed-off rule, resolution by name, and the retry decision diagram). The two duplicate flow diagrams were merged into one; "as it does today" (spec wording) was dropped. No "planned" marker remains, and none was needed: everything the page describes is built |
| `worktree/README.md` | Removed the sub-bullet describing the old `git worktree remove` partial-failure fallback (`rm -rf` / `Remove-Item` hint), which no longer exists; added retry and refused-topology sub-bullets; exit-code table: 1 includes a stop after the move, 3 includes another process using the worktree (it was listed under 4), 4 lists change watching and the Windows open-file move block |
| `docs/dependencies.md` | Catalog entries for `fsevent-sys` 4.1 and `inotify` 0.11 under Platform-Specific (the usage note already existed; the catalog had no entries); the `windows-sys` entry now names the `ReadDirectoryChangesW` observer |
| `.claude/skills/worktree/lifecycle.md` | Points to the docs sections that must change with an exit class or refusal, and to the README exit table |
| `.claude/skills/os/linux.md` | `--build-jobs` must follow the package name in `cross-check`; before it the script exits 2 (`unknown flag --build-jobs`) |

### Rulings, spikes, and departures (index)

The rulings R1-R12 and spike results S1-S3 are recorded under "Phase 1"
(old numbering) and in `spikes/README.md`; every later phase records its
departures under its own "Decisions and departures" heading. The departures
that change what the spec says the user sees:

- The usage check runs after the questions (Phase 11), so a process standing
  in the worktree is exit 3 after the report, not exit 4 before it. On Windows
  a held directory is now usually exit 3 too: Sniff sees the handle before the
  move is attempted.
- A retry is never handed off (Phase 11); a shell inside the moved checkout
  gets exit 4 and the instruction to run from the base checkout.
- A pre-move operational failure exits 1 through `WorktreeError::RemovalFailed`
  with "Nothing was removed." (Phase 11).
- Interruption (Ctrl-C) is not wired to a stop at the next entry boundary;
  the library supports it (`Environment::interrupted`), the CLI passes
  `|| false`, and a killed process is handled by crash recovery (Phase 11).
- The last check of both paths and the index happens at the start of
  unregistering, so a write after it is the documented concurrent-write
  limit, not a stop (Phase 11, stated in the docs).

### Acceptance traceability (spec items 1-12)

Library tests are under `worktree/lib/src/remove/`; CLI tests are integration
binaries in `worktree/cli/tests/`. `native_*` tests use the real observer,
rename, and processes, and run as ordinary L1 on every OS.

| Item | Tests |
| --- | --- |
| 1. Known usage refuses before rename/deletion, names PIDs/evidence, keeps branches, no force flag overrides | `usage::tests::every_evidence_kind_from_another_process_refuses`, `usage::tests::a_real_query_names_a_process_working_in_the_checkout`, `usage::tests::only_this_operations_registered_handles_are_excluded`, `usage::tests::this_processes_other_evidence_still_refuses`, `lifecycle::tests::observed_usage_refuses_before_anything_moves`; CLI `remove::a_process_standing_in_the_worktree_refuses_after_the_questions_with_exit_3`, `powershell_wrapper_exec::a_directory_held_by_another_program_refuses_with_exit_3_and_nothing_removed`, `level2_powershell_remove::level2_powershell_refuses_a_worktree_another_program_holds_with_exit_3_and_nothing_removed`, `remove_usage_refusal::*` |
| 2. No match / partial / unsupported discovery proceed with observation; no chatter; coverage kept; root-validation errors refuse | `usage::tests::no_match_clears_whatever_the_coverage`, `usage::tests::a_match_in_a_partial_report_refuses_with_its_coverage`, `usage::tests::every_query_error_refuses_as_unverified_without_a_retry`, `usage::tests::a_real_query_of_a_vanished_checkout_refuses`, `lifecycle::tests::partial_discovery_…`, `lifecycle::tests::unavailable_discovery_…`, `lifecycle::tests::unsupported_discovery_still_removes_and_is_kept_in_the_report`, `lifecycle::tests::an_unverified_checkout_refuses`, `lifecycle::tests::native_a_removal_runs_end_to_end_and_closes_its_record` (one decisive query); CLI `remove::example_clean_and_merged_removes_worktree_and_branch`, `remove_usage_refusal::coverage_notes`, `remove_usage_refusal::an_unverified_checkout_refuses` |
| 3. Controlled writers: retained-cwd write into the quarantine and recreation at the original; remnants kept, honest report, no branch operations | `deletion::tests::native_a_writer_in_the_moved_checkout_stops_deletion_and_its_file_is_kept`, `deletion::tests::native_recreation_at_the_original_path_stops_deletion`, `lifecycle::tests::a_write_into_the_quarantine_during_deletion_stops_with_an_honest_report`, `lifecycle::tests::recreation_at_the_original_path_stops_and_is_left_alone`; CLI `remove_recovery::a_write_into_the_moved_checkout_stops_then_a_retry_needs_fresh_consent`, `remove_recovery::recreation_at_the_original_path_stops_and_is_never_cleaned_up` |
| 4. Modification, replacement, deletion-time recreation, delayed events, overflow, watch invalidation | `deletion::tests::a_file_changed_without_any_event_is_kept_and_stops_deletion`, `inventory::tests::a_permission_change_is_a_modification`, `deletion::tests::a_directory_replaced_by_a_link_is_never_descended`, `deletion::tests::an_entry_deleted_by_someone_else_is_recorded_absent_and_stops`, `deletion::tests::a_reported_write_stops_at_the_next_drain`, `deletion::tests::lost_coverage_and_admin_changes_stop_deletion`, `deletion::tests::native_a_full_queue_stops_with_lost_coverage`, `unregister::tests::a_recreated_original_found_only_by_the_final_check_stops`, `observation::tests::delayed_events_arrive_only_once_delivered`, inotify `a_kernel_overflow_is_coverage_loss` / `a_mount_or_unmount_under_a_watched_path_is_loss_and_elsewhere_ignored`, FSEvents `an_unmount_or_a_moved_parent_or_admin_directory_is_loss`, ReadDirectoryChangesW `a_system_buffer_overflow_is_coverage_loss` / `completions_map_to_events_overflow_or_an_ended_watch`, `lifecycle::tests::lost_coverage_after_the_move_stops_and_a_failed_diagnostic_is_recorded`, `lifecycle::tests::unexplained_activity_during_the_move_stops_after_it` |
| 5. Rename collision/failure, Windows handle contention, submodules, locked worktrees, links/reparse points, changed admin associations, cross-device | `association::tests::a_collision_at_the_quarantine_is_never_replaced`, `rename::tests::an_existing_destination_is_never_replaced`, `rename::tests::rename_failures_map_to_their_exit_classes` (EXDEV / `ERROR_NOT_SAME_DEVICE` → cross-device), `rename::tests::an_open_file_in_the_tree_is_a_sharing_failure` (Windows), `association::tests::an_open_file_in_the_checkout_blocks_the_move_as_in_use`, `association::tests::a_checkout_with_a_submodule_refuses_before_anything_moves`, `association::tests::a_locked_worktree_refuses_at_planning_and_at_the_move`, `lifecycle::tests::a_locked_worktree_refuses`, `rename::tests::a_link_is_moved_as_itself`, `inventory::tests::a_link_at_the_root_is_refused`, `association::tests::unsupported_shapes_refuse_before_anything_moves`, `association::tests::state_that_changed_since_planning_refuses_the_move`, `association::tests::a_failed_record_rewrite_is_rolled_back`, `association::tests::an_unverified_record_rewrite_is_rolled_back_and_restored` |
| 6. Crash injection at each phase leaves recoverable records; retry by original name after unregistering; changed identities/new content rejected | `lifecycle::tests::a_crash_after_the_move_is_resumed_by_the_original_name`, `lifecycle::tests::a_crash_after_the_content_is_removed_resumes_with_the_record_and_uncertain_entries`, `lifecycle::tests::a_crash_after_unregistering_runs_only_the_branch_actions_on_retry`, `lifecycle::tests::native_a_killed_removal_is_found_and_resumed_by_its_original_name`, `lifecycle::tests::a_replaced_quarantine_is_never_resumed`, `lifecycle::tests::new_content_in_the_quarantine_needs_fresh_consent`; CLI `remove_recovery::a_write_into_the_moved_checkout_stops_then_a_retry_needs_fresh_consent` |
| 7. Consent fingerprint, expected tip, remote endpoint retained; recreated original never traversed; rollback never overwrites | `lifecycle::tests::files_that_need_consent_refuse_without_it`, `lifecycle::tests::files_changed_after_approval_refuse`, `lifecycle::tests::a_tip_moved_after_approval_refuses`, `association::tests::a_recreated_original_is_never_overwritten`, `association::tests::a_replaced_quarantine_is_not_moved_back`, `deletion::tests::recreation_at_the_original_stops_and_is_never_touched`; CLI `remove::a_changed_remote_destination_between_the_runs_refuses_with_nothing_removed`, `remove::a_branch_that_moves_after_its_worktree_is_removed_is_kept`, and the existing handoff fingerprint tests in `remove.rs` |
| 8. Human reports and structured outcomes agree on both paths, partial deletion, coverage, registration, branch state | `deletion::tests::the_report_names_deleted_entries_and_states_the_concurrent_write_limit`, `outcome::tests::a_stopped_report_round_trips_through_json`, `remove_recovery_report::*` (6 snapshots), CLI `remove_recovery::*` (parse the saved `report.json` and compare with stderr), `remove::a_push_after_the_second_run_preflight_fails_the_lease_with_a_partial_result` |
| 9. Wrapper first run reaches handoff despite its shell; second run discovers; another process refuses; stdout protocol unchanged | `remove::the_parent_shell_inside_does_not_block_the_handoff_and_the_second_run_removes`, `remove::a_process_still_inside_after_the_handoff_refuses_the_second_run`, `wrapper_protocol.rs`, `shell_wrapper_exec.rs`, `powershell_wrapper_exec.rs`, L2 `level2_powershell_remove` |
| 10. Missing directories keep record-only removal; pending records cannot be bypassed by the quarantine name; reused branch/name ambiguous; partial checkouts never cleaned unchecked | existing missing-directory tests in `missing.rs` and `remove.rs`, `lifecycle::tests::a_crash_after_the_move_is_resumed_by_the_original_name` (quarantine basename reaches the same record), `lifecycle::tests::a_new_worktree_at_the_original_path_is_ambiguous_and_never_cleaned_up`, `lifecycle::tests::a_reused_branch_is_ambiguous_with_the_pending_removal`, `lifecycle::tests::a_partial_checkout_without_its_git_link_is_kept`; CLI `remove_recovery::a_new_worktree_at_the_original_path_is_ambiguous_with_the_pending_removal` |
| 11. Record/journal write failures, crash between unlink and result, stale locks, malformed records, interruption after rename; branch-step failures need fresh approval; completed records never authorize again | `deletion::tests::a_journal_that_cannot_record_an_intent_stops_before_its_batch`, `lifecycle::tests::an_unwritable_record_stops_before_deleting`, `lifecycle::tests::a_crash_after_the_content_is_removed_resumes_with_the_record_and_uncertain_entries`, `lifecycle::tests::only_a_held_lock_blocks_a_retry`, `lifecycle::tests::an_unreadable_record_blocks_a_new_removal`, `deletion::tests::an_interrupt_stops_at_the_next_entry_boundary`, `lifecycle::tests::an_interrupted_removal_is_resumed`, `lifecycle::tests::a_failed_branch_deletion_keeps_the_record_and_a_retry_needs_fresh_approval`, `lifecycle::tests::a_completed_record_never_authorizes_a_second_removal`, the recovery-store robustness matrices in `recovery/store/tests.rs` |
| 12. Native paths round-trip; links not followed; nested mounts/unsupported entries refuse; concurrent rename destination never replaced; exit 1 after move, 3 before, 4 for Windows sharing failure; ordinary I/O not labeled usage | `rename::tests::a_name_that_is_not_utf8_moves_losslessly`, `outcome::tests::a_non_utf8_path_survives_the_report_losslessly`, `rename::tests::identity_names_a_link_itself_not_its_target`, `inventory::tests::a_fifo_or_socket_is_ineligible_and_never_opened`, `association::tests::unsupported_shapes_refuse_before_anything_moves`, `rename::tests::an_existing_destination_is_never_replaced`, `rename::tests::rename_failures_map_to_their_exit_classes`, `rename::tests::unix_error_codes_classify`, `rename::tests::windows_error_codes_classify_by_stage`, `rename::tests::a_parent_this_user_cannot_write_is_a_permission_failure`, `rename::tests::an_error_without_an_os_code_is_other`, `remove_recovery_report::refusals_say_nothing_was_removed_and_keep_their_exit_codes` |

### Structural checks (re-verified)

- **No per-process repeated checkout walks:** one decisive Sniff query per
  attempt (`native_a_removal_runs_end_to_end_and_closes_its_record` asserts
  exactly one query and one discovery entry); the diagnostic query after a
  stop runs only on a stop.
- **No whole-tree rescan per deleted entry:** each entry is rechecked by its
  own metadata; events are drained per depth level and every 2,048 entries
  (`drains_follow_depth_levels_and_the_bound_not_entries`).
- **No unconditional hashing of ignored bodies:** only consent-sensitive paths
  are hashed (`a_file_below_a_sensitive_directory_is_hashed` asserts ignored
  output stays `Content::Unhashed`).
- **No scanning threads after a query returns:** Sniff's contract, covered by
  its own query tests; `wt` calls the query synchronously and holds nothing
  from it.

### Requirement-to-test mapping (this phase)

No behavior changed in this phase, so no test was added. The docs edits were
checked against the code they describe: exit classes (`exit.rs`,
`recovery.rs::refusal_error`), the record location and file names
(`recovery/storage.rs`), and the retry-never-handed-off rule
(`commands/remove/mod.rs`). No test reads the edited files
(`include_str!`/`repo_root()` search: none in worktree; test-toolkit reads
only other `os` skill pages).

### Verification

Local (macOS), in `worktree/`:

- `just test`: 1358 passed, 15 skipped, 0 failed.
- `just test-l2`: 41 passed.
- `just lint`: clean.

Cross-OS (`./scripts/cross-check.sh --os <os> <package>`, archive mode):

| OS | `worktree` | `worktree-cli` | L2 |
| --- | --- | --- | --- |
| Linux (`build-linux`) | 747 passed, 1 skipped | 609 passed, 55 skipped | `--features terminal-tests level2_` with `BISCUIT_TEST_REQUIRED_BACKENDS=tmux`: 41 passed |
| WSL2 (`build-win`) | 747 passed, 1 skipped | 609 passed, 55 skipped | not run (CI omits it as an accepted gap) |
| native Windows (`build-win-native`) | 720 passed, 1 skipped | first run 573 passed, **1 failed** (below); rerun 574 passed | `level2_powershell`: 3 passed |

`just ci-local --plan` against R11: the worktree, worktree-cli, and sniff cells
are the expected L1/L2/lint/check set (sniff because the filesystem-watchers
feature is on this branch). The only narrowed cells are `test-toolkit` (it
reads `os` skill pages) and two darkmatter cells from a darkmatter spec on
this branch, unrelated to this fix. No `source-inputs`, no prompts, no
terminal focus. About 25 executing cells: the 30-45 min band for a pull
request.

### Findings that need attention

- **Windows Search again.** The first Windows `worktree-cli` run failed
  `remove::example_unique_commits_without_a_terminal_keep_the_branch_with_a_warning`
  with exit 3: `SearchProtocolHost.exe` (PID 6808) held an open handle on
  `b.txt` in a fresh fixture under `%TEMP%`. That is the third observed hit,
  and the human-review item from Phase 11 is still open. Not papered over;
  the immediate rerun passed.
- **Windows timing.** On `build-win-native`, 45 of 93 `remove` tests and 34
  tests elsewhere in `worktree-cli` (for example `list_prs` at 11 s) take
  more than 5 s in a package run. The worst `remove` tests take 16-18 s and
  run several removals or the network stand-ins; every `worktree` library
  test of the new lifecycle is under 5 s (slowest 4.6 s). The overrun is
  package-wide on that host, and this fix adds about 1.5-2 s per removal
  (Phase 11). The redundant Git calls in `unregister_quarantine`
  (`inspect_missing` followed by `remove_missing_record`, about four spawns)
  account for roughly 0.2 s of that, so they are not the main cost; the
  per-removal Sniff query and the marker-confirmed watch checks are the next
  candidates to measure. Not changed here: this phase changes no behavior,
  and the remedy needs a measurement first.

Phase 12 is complete. Implementation complete, ready for review.
