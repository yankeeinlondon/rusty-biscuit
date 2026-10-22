---
total_phases: 9
created: 2026-10-22
phase: 1
agent: claude/sonnet
yolo: true
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
---

# Plan: a confirmed `wt remove` always deletes

Spec: [spec.md](spec.md). Rule numbers (R1–R29) are the spec's.

## Summary and Definition of Done

### What the work is

`wt remove` today stops whenever it cannot prove nothing else touched the
tree: an activity watcher (FSEvents, inotify, `ReadDirectoryChangesW`), a
per-entry checked deletion engine with a BLAKE3/size/mtime journal, a
free-space refusal, and a set of refusals (processes, locks, submodules,
mounts, links, unreadable Git state). The spec replaces that with one rule:
**after the user confirms (or passes `--force`), the checkout is deleted.**

Concretely the work is five things:

1. **A compact, versioned recovery record** replaces the journal and the
   `report.json` (R25, R26). It is saved durably after consent and before the
   first unlock, move or deletion, and is enough to resume after a crash
   between any two steps (R19). Legacy record v1 (with reports up to v6) is
   migrated, never trusted on its report version alone.
2. **A plain deletion engine** replaces the watcher plus checked engine
   (R20–R24, R29): no-follow handle traversal, mount boundaries, unlink of
   special files, refill handling, Windows sharing-violation waits with capped
   backoff on an injected clock, and a permanent-failure path that deletes the
   rest and exits 1 with one plain sentence.
3. **Refusals become warnings in one confirmation question** (R1–R15, R27):
   processes with context, locks, submodule-only work, links, unreadable Git
   state, nested mounts, drvfs, synced folders, the shell standing inside the
   target. Non-interactive runs refuse with exit 3 unless `--force`.
4. **Lifecycle rewire** to the flow in the spec's flowchart, including
   concurrent-removal waiting (R16), vanished checkouts (R17), the handoff
   second run (R18), and unchanged branch protections.
5. **Machinery removal and docs** (R23, R28): the watcher, its adapters and
   tests, the journal, the free-space gate, `report.json`, `--force-worktree`,
   watcher-only dependencies; docs and skills rewritten.

### What success looks like

- Every ruling R1–R29 has an L1 test that fails against the old behavior and
  passes now; tests pinning the old stops are replaced, not skipped.
- `wt remove <name>` plus "yes" (or `--force`) deletes a built checkout in one
  run on macOS: only the question and the final line appear; no hidden
  `.wt-remove-…` name, `Retry:`, report path, or watcher/journal/coverage text.
- Non-interactive without warnings removes silently; with warnings and no
  `--force` exits 3 before any deletion or unlock.
- Crash injection between every pair of flowchart steps resumes without a new
  question; replaced roots/admin records and recreated original paths are
  never deleted; unbound legacy remote actions are never replayed.
- `--force-worktree` is a usage error (exit 2); `--force` never implies
  `--force-branch` or `--force-remote`; branch tiers, tips, endpoint and lease
  checks behave as before.
- `grep -ri "fsevent\|inotify\|ReadDirectoryChanges\|CoverageLost\|journal" worktree/` finds only docs that say these are gone (or nothing); `inotify` and
  `fsevent-sys` are out of `Cargo.toml`; dependency docs updated.
- `just test`, `just test-l2`, `just lint` pass in `worktree/`; one 500,000-entry
  macOS acceptance run is recorded (elapsed time, compact-record size).
- Docs/skills match the code (see Phase 9) and carry no **Planned** markers
  left over from this spec.

### Parallelism conventions

Waves are numbered once across the whole plan. Tasks inside a wave touch
disjoint files and may run as concurrent subagents; waves run in order. Phase 4
depends only on Phase 1 and may run concurrently with Phases 2–3. Every
subagent brief must state that the session tree is non-interactive.

---

## Phase 1: Rulings, audits, and test seams

Goal: close the open decisions the spec leaves to the plan, map what the old
machinery is wired into, and fix the seams every later phase tests through.

### Necessary Rules

The spec says "Open Questions: None," but these points are left to the plan
(or are ambiguous) and block implementation. Each has a **proposed** ruling;
the author confirms or changes them before Wave 3.

- [x] **N1. Record version.** The compact record is `record_version: 2`.
  Version 1 is read only by the migrator and never written. A v2 record keeps
  `RECORD_VERSION` semantics (unknown version refused before other fields are
  read). *Proposed: v2.*
- [x] **N2. "Refill" threshold (R24).** How many consecutive
  "directory not empty → re-list → delete" passes before the usage check and
  wait? *Proposed: 3 consecutive passes on the same directory with no net
  progress, then one usage query and the R22 wait.*
- [x] **N3. Backoff schedule.** *Proposed: 50 ms doubling to a 2 s cap,
  interruptible, injected clock; the wait message is printed on entry and only
  when the holder set changes.* No overall timeout (the user's Ctrl-C ends it).
- [x] **N4. Error classification.** Which `io::ErrorKind`/OS codes are
  *temporary* (retry: `ERROR_SHARING_VIOLATION`/`ERROR_LOCK_VIOLATION`,
  `ENOTEMPTY` on a refilling dir) vs *permanent* (`EROFS`, `EACCES`/`EPERM`
  not fixable, `EIO`, `ERROR_ACCESS_DENIED` after clearing read-only, `ENOSPC`
  on the record). *Proposed:* a single `classify(error) -> Temporary |
  Permanent | Gone` function, unit-tested per OS table, `Gone` (ENOENT) being
  success. Unknown errors are **Permanent** (never loop forever).
- [x] **N5. Exit codes.** *Proposed mapping:* record cannot be saved → 1 (nothing
  deleted, message says so); permanent OS failure → 1; nested mount retained →
  1; changed/kept branch → 1; non-interactive refusal → 3; ambiguity/input →
  unchanged; `--force-worktree` → 2; Ctrl-C → existing signal mapping
  unchanged; "blocked by environment" exit 4 is retired for the cases R4, R15
  (Unix), R16 and R18 now cover, and kept only for R15 on Windows (which
  refuses before the question). The CLI exit-code table in docs is updated to
  match.
- [x] **N6. Submodule "commits on no remote."** *Proposed:* a commit counts
  as unique when it is reachable from the submodule `HEAD` or any local branch
  but from no ref under `refs/remotes/*` (`git rev-list --not --remotes`),
  no fetch. A submodule with no remotes at all counts every local commit as
  unique, and the warning says "no remote configured."
- [x] **N7. Lock handling (R3).** The lock reason is the content of
  `<admin>/locked`. *Proposed:* "unlock" is removing that file through
  `git worktree unlock` (not by hand), done only after the record is saved,
  and recorded in the record so a crash after unlock but before move is
  recoverable. The lock file's mtime is never shown as a creation time.
- [x] **N8. `--force` and non-interactive "clean" definition.** "Anything to
  warn about" = any warning the question would have shown. *Proposed:* a
  failed/partial usage query counts as a warning (spec says it adds one), so a
  non-interactive run whose usage query failed refuses without `--force`.
  Confirm this is intended.
- [x] **N9. Handoff (R18).** Expiry 10 minutes; token single-use. *Proposed:*
  the token carries the operation id (not the file fingerprint), and the second
  run reads the saved v2 record instead of recomputing content hashes.
- [x] **N10. In-place deletion marker (R4, R7, R8, Windows rename block).**
  *Proposed:* record `deletion_mode: moved | in_place` plus the original path
  and checkout identity; in-place mode removes the Git registration only after
  the checkout is empty or a retained mount exists (flowchart E→G).
- [x] **N11. Windows R15.** Refused before the question with one line naming
  the shell; reuse the existing parent-process/cwd detection. Confirm the
  Windows shell cwd check does not need new Sniff API.
- [x] **N12. Large-acceptance scope.** Spec limits the measurement to one macOS
  500,000-entry run. Recording the same on Linux/Windows/WSL2 is **not**
  planned; list it as an author decision (cost: a multi-host campaign the spec
  explicitly declines).

### Input robustness (applies: the work adds a record loader)

Loaders touched: the strict JSON loader behind `recovery::record::parse` (v2,
and v1 via the migrator's separate parse), and the optional legacy report
reader. They share `strict_json`, so **document shapes** are tested once on
the shared loader and **format syntax** once per record format; **field shapes**
are tested per validator.

- **Default outcome (stated once):** every invalid shape produces a plain,
  located error ("record `<id>` is not valid: `<field>` …"), deletes nothing,
  replays nothing, and leaves the original files in place.
- **Exceptions:** `aliases: []` is valid (empty ≠ absent); legacy report
  absent is valid (reports are optional); a stale `completed_phase` is a hint,
  not an error; an unknown `record_version` is refused before other fields are
  read; a legacy remote-branch action with no bound endpoint/head migrates to
  "left undone, exit 1 with explanation" (not an error).
- **Load-bearing fields (v2):** `record_version`, `operation_id`, `original`,
  `quarantine`, `deletion_mode`, `checkout_identity`, `repository`, admin
  record binding, `aliases`, `intended_actions`/approved branch steps (tip,
  approval kind, remote destination, endpoint, expected head), progress marker.
- **Grep before declaring the matrix done:** `#[serde(default)]`,
  `unwrap_or_default()`, `.ok()`, `filter_map(.. as_str())`, and `Option<T>`
  where absent vs null must differ, across `recovery/`.
- **Spawn the CLI** only for wiring: one control case + one corrupt-record case
  per loader asserting the located error and that nothing was deleted.

### Tasks

#### Wave 1 (parallel audits; read-only, no code changes)

- [x] **Sniff context audit** — read `sniff` path-usage query and report types
  against R27 and the filesystem-query contract.
  - List which of name/app description, start time, parent chain, activity
    kind (writing/watching/running from/standing in) already exist per OS.
  - Output: a table of missing fields to add in Sniff (never a second scanner
    in `worktree-cli`) and whether the work budget needs a new knob.
- [x] **Coupling map** — list every consumer of `observation/`, `recovery/journal`,
  `recovery/space`, `recovery/storage`, `usage.rs` gates, `Stop`, `CoverageLost`,
  `report.rs`, and `--force-worktree` in `worktree/lib`, `worktree/cli`,
  tests, justfile recipes, docs, and skills.
  - Output: a delete/keep/rewrite list per file; confirm which `windows-sys`
    features remain needed after R23 and whether any other crate uses
    `inotify`/`fsevent-sys`.
- [x] **Old-stop test inventory** — list tests that pin the old stops
  (`Stop::*`, `CoverageLost`, refusals R1–R22, `remove_usage_refusal.rs`,
  `remove_recovery_report.rs`, `lifecycle/tests/large_checkout.rs`,
  `startup_warning.rs`, `resume_consent.rs`) with the ruling that replaces each.

#### Wave 2 (sequential; depends on Wave 1 and author confirmation of N1–N12)

- [x] **Test seams** — define the injection points every later phase uses:
  - `Clock`/`Sleeper` (injected, cancellable) for backoff and waits;
  - `UsageSource` (injected usage reports, partial/failed variants);
  - `MountInfo` (injected mount table incl. same-device bind mounts);
  - `FsFaults` (injected read-only, access-denied, sharing-violation, refill);
  - crash `Hook` (already exists in lifecycle; extend to cover each flowchart
    edge: before/after save, unlock, rename, each delete batch, registration
    removal, each branch action).
  - Prefer extending the existing `Environment`/`Hook` and `test_support.rs`
    over new frameworks; use the Test Toolkit.
- [x] **Spike decision record** — add to this plan's Phase 1 notes: **no spikes
  are scheduled.** Watcher pacing was already measured
  (`2026-10-19-macos-drain-interval-comparison`) and the spec forbids repeating
  it; drvfs behavior is resolved by the spec's fallback (try atomic move, else
  in-place), so it needs a test, not a spike; the single 500,000-entry run is
  an acceptance check in Phase 8, not a spike.

### Phase 1 notes

#### Ruling status

N1–N12 are **adopted as proposed**. This plan is `yolo: true` and Phase 1 ran
headless, so the author never saw them. They follow the spec's text, and the
audits below found nothing that contradicts them. The author may still amend
any of them before Phase 2 starts. Two of them were checked against the code:

- **N11:** `wt` inherits the shell's working directory, and the existing
  check already compares it with the target
  (`cli/src/commands/remove/mod.rs:356`, `is_within(&cwd, …)`). Naming the
  shell needs only its parent process name, which Sniff already reads. No new
  Sniff API is needed.
- **N12:** nothing in the code or docs requires the measurement on Linux,
  Windows, or WSL2.

#### Audit: Sniff process context (R27)

The query is `query_path_usage` (`sniff/lib/src/filesystem/query/mod.rs:111`),
which returns `PathUsageReport`/`ProcessRecord` (`report.rs:25,171`). Today:

| R27 field | macOS | Linux/WSL2 | Windows |
|---|---|---|---|
| recognizable name | short `pbi_name` only | `comm` | image file name only |
| start time | yes (seconds, sysinfo) | yes (seconds) | yes (FILETIME) |
| parent chain | **missing** | **missing** | **missing** |
| activity kind | `OpenHandle` + `access.write`, `WorkingDirectory` | `OpenHandle` + fdinfo access, `WatchRegistration`, `WorkingDirectory` | `OpenHandle` (`access: None`), `LoadedModule` |
| where | `matched_paths`/`observed_path` | same | same |

Fields to add in Sniff (Phase 4, Wave 7):

- `parent_pid` on `ProcessDetails`/`ProcessRecord`:
    - macOS: `pbi_ppid`, from the `PROC_PIDTBSDINFO` read it already does
      (`macos.rs:473`);
    - Linux: field 4 of `/proc/<pid>/stat`, read in `read_stat` (`linux.rs:553`);
    - Windows: `InheritedFromUniqueProcessId`, from
      `NtQueryInformationProcess` (`win32.rs:483`).
- A bounded parent chain built from those per-pid reads. Each ancestor gets
  a depth cap like `MAX_EVIDENCE_PER_PROCESS` and a creation-token recheck.
  Never use `self_and_ancestors` (`os/working_processes.rs:89`): it refreshes
  the full process table.
- App name on macOS, from the enclosing `.app`'s `Info.plist`
  `CFBundleName`. This is new code.
- FileDescription on Windows, from `GetFileVersionInfoW`/`VerQueryValueW`.
  This is new code.
- Write access on Windows (`access` is always `None` there).
- An explicit unavailable value for each, using the `Limitation` pattern
  (`report.rs:359`).

**Work budget:** no new knob. The lookups run per holder behind the existing
`budget.expired()` check (`process.rs:83`). Add a counter (for example
`filesystem.query.parent_lookups`) rather than a deadline.

`wt` reads, today:

- `pid`, `creation_token`, `identity_uncertain`, `name`, `executable`,
  evidence `kind`/`descriptor`/paths, coverage, `outcome`, and
  `budget_exhausted`;
- **not** `start_time`, `user`, `access`, `event_only`, or `watch`.

#### Audit: coupling map

Corrections to later phases:

- **`cli/src/commands/remove/report.rs` is not `report.json`.** It renders the
  status report printed before the question, and it is kept.
    - `report.json` is `recovery/store.rs` (`REPORT_FILE`, `save_report`,
      `report_path`) plus `outcome::OperationReport`. Phases 6 and 7 delete
      those instead.
- **`recovery/storage.rs` has no free-space code.** The free-space gate is:
    - `recovery/space.rs`;
    - `ensure_space` and `StoreError::{InsufficientSpace, SpaceUnknown}` in
      `store.rs`;
    - `WT_TEST_AVAILABLE_SPACE_BYTES` in `cli/src/env.rs`.

  `storage.rs`'s `open_append`, `temp_beside`, `publish`, and
  `create_private_file` serve only the journal.
- **`observation/inventory.rs` (`DeletionInventory`) is load-bearing outside
  the watcher.** Its users are:
    - the usage `TreeIndex`;
    - `store.rs`, `lifecycle.rs`, and `resume.rs`.

  Its survivors (`open_root`, tree index) must move before `observation/` is
  deleted.
- **`usage::OwnedHandles`** exists only to exclude the watcher's own handles,
  and goes with the watcher. So do the Windows Search recheck parts of
  `UsageSource` (`system_directory`, `pause`) and `SEARCH_RECHECK_WAIT`.
- **`lifecycle::SLOW_STARTUP` and `Environment::slow_startup`** are a
  watcher-only warning (`startup_warning.rs`).

What is deleted, rewritten, and kept:

- **Delete:**
    - `observation/` except what moves out of `inventory.rs`;
    - `recovery/journal.rs` and `recovery/space.rs`;
    - `lifecycle/tests/startup_warning.rs`;
    - `cli/tests/remove_usage_refusal.rs` and its 6 snapshots;
    - most of `cli/tests/remove_recovery_report.rs` and its 13 snapshots;
    - `.claude/skills/worktree/observation.md`.
- **Rewrite:**
    - in the library: `recovery/{store,mod}.rs`, `lifecycle.rs`,
      `lifecycle/resume.rs`, `outcome.rs`, `quarantine/unregister.rs`, and
      `usage.rs`;
    - in the CLI: `cli/src/commands/remove/{mod,recovery,usage,policy}.rs`,
      `cli/src/{args,main,env}.rs`, and `cli/tests/{remove,remove_recovery,level2_powershell_remove}.rs`;
    - elsewhere: `worktree/README.md`, `worktree/docs/cli/remove.md`, and
      `docs/dependencies.md` (lines 318–341);
    - in the skills: `.claude/skills/worktree/{SKILL,lifecycle,remove,testing}.md`
      and `.claude/skills/os/{macos,linux,windows}.md`, whose watcher sections
      go;
    - `.config/nextest.toml`: drop or narrow the `fsevents-l1` group and the
      `remove::.*::native_` override.
- **Keep:**
    - `admin_entry.rs`, `quarantine/{association,rename,naming}.rs`, and
      `recovery/{lock,record}.rs`;
    - `handoff.rs`, `safety.rs`, `inventory.rs`, `included.rs`, `missing.rs`,
      `repair.rs`, `remote.rs`, `cli/src/commands/remove/interrupt.rs`, and
      `worktree/docs/performance-testing.md` (its watcher hits are unrelated).
- **`windows-sys` features:**
    - keep `Win32_Foundation`, `Win32_Storage_FileSystem`, and
      `Win32_Security`. `CreateFileW` needs `Win32_Security`, which
      `docs/dependencies.md` wrongly attributes to the observer;
    - drop `Win32_System_IO` and `Win32_System_Threading` (both
      `ReadDirectoryChangesW` only) and `Win32_System_SystemInformation`
      (the Search recheck only).
- **`inotify` / `fsevent-sys`:** only `worktree/lib/Cargo.toml` depends on
  them. Both leave the lockfile, with `inotify-sys`.
- **Recipes:** no `worktree/justfile` recipe and no `perf_` test is
  watcher-specific.

#### Audit: tests pinning the old stops

Approximate counts per file are delete / rewrite / keep. Rows marked "most"
or "≈" are estimates from the inventory; the remaining rows are exact counts.

| Area | Delete | Rewrite | Keep |
|---|---|---|---|
| `lifecycle/tests.rs` (52) | 12 | 27 | 13 |
| `lifecycle/tests/{large_checkout,startup_warning,resume_consent}.rs` | 4 + 3 + 8 (→ 1 new "resume asks nothing") | 1 (legacy report v4/v5 → R19 migration) | 0 |
| `observation/**` (~141) | ~130 (R23) | ~10 engine and FIFO properties ported to the new engine | 0 |
| `usage/tests.rs` (13) | 4 (Search recheck) | 6 (refusal → warning, R1/R17/R27) | 3 |
| `recovery/store/tests.rs` (24) + `journal.rs` (14) | 11 + 14 | 4 (R16 wait, R19, completion leaves nothing) | 9 |
| `handoff.rs` (9) | 0 | 2 (10 min expiry; identity-only checks) | 7 |
| `inventory.rs` (16) | 12 fingerprint tests | 0 | 4 |
| `missing.rs` (27) | 0 | 14 (R12/R14, tip moved → branch kept) | 12 |
| `quarantine/association` (15), `quarantine/unregister` (8) | 1, 3 | 5, 4 | 9, 1 |
| `outcome.rs` (9) | 7 | 1 (v4 report → migration) | 1 |
| `cli/.../policy.rs` | 0 | 2 (`force_worktree` matrix) | 1 |
| `cli/tests/remove.rs` (138) | 1 (free space) | ≈60:<br>• ≈13 flag spelling;<br>• ≈18 handoff content (R18);<br>• ≈17 branch/remote outcome: checkout deleted, branch kept, exit 1;<br>• ≈12 R1/R9–R15 | ≈77 |
| `remove_usage_refusal.rs` (8) | 1 | 7 (warnings; keep the literal-text check) | 0 |
| `remove_recovery.rs` (8) | 0 | 7 | 1 |
| `remove_recovery_report.rs` (23) | 19 | 3 | 1 |
| `level2_remove.rs` (11), `level2_powershell_remove.rs` (3) | 0 | 2, 2 | 9, 1 |
| `wrapper_protocol.rs` (15) | 0 | 0 | 15 |

Unchanged:

- branch safety: `safety.rs`, `remote.rs`, and the first-run endpoint
  refusals;
- ambiguity tests;
- main-checkout protection;
- the no-replacement and identity guarantees: `rename`, `admin_entry`,
  `naming`, `repair`, `a_replaced_quarantine_is_never_resumed`, and
  `a_completed_record_never_authorizes_a_second_removal`.

Across the tests, a branch or remote change found on resume or in the
handoff's second run no longer refuses the whole removal: the checkout is
deleted, the branch is kept, and the run exits 1.

#### Test seams (Wave 2)

All seams are in the `worktree` library (`lib/src/remove/`). Their test
doubles are in `remove/test_support.rs`.

| Seam | Production | Test double | Used by |
|---|---|---|---|
| `clock::Clock` (`sleep(duration, cancelled) -> Slept`) | `HostClock`: checks cancellation every 50 ms | `ManualClock`: records sleeps, can cancel after N | every wait (R16, R22, R24) |
| `clock::Backoff` (N3: 50 ms doubling to a 2 s cap, `reset`) | — | via `ManualClock` | engine retries (Phase 3) |
| `mounts::MountInfo` + `mounts_within` (by mount point, never device ID) | native table: Phase 3, Wave 6 | `FixedMounts` | R7 |
| `faults::FsFaults` (`inject(FsOp, path)`) | `NoFaults` | `ScriptedFaults`: `times(n)` for temporary faults, `always` for permanent ones, `fired` | engine (Phase 3), R29 |
| `usage::UsageSource` | `SniffUsage` (exists) | `Usage` in `lifecycle/tests.rs` and `Sequence` in `usage/tests.rs` (partial, failed, scripted) | R1, R17, R24, R27 |
| `lifecycle::Hook`/`Boundary` | no-op | `crash_at` in `lifecycle/tests.rs` | crash matrix |

`Boundary` variants are added with the step that passes them. A variant no
step constructs fails `-D warnings`, and Phase 5 rewrites `lifecycle.rs`
anyway. The rewired lifecycle must emit, in flowchart order:

1. `Begun` (approval saved);
2. `Unlocked`;
3. `Moved` or `MarkedInPlace`;
4. `DeletionBatch(n)`;
5. `ContentRemoved`;
6. `Unregistered`;
7. `BranchAction(i)` after each branch step;
8. `BranchesDone`.

"Before save" is the run with no record, so it needs no boundary.

#### Spike decision record

**No spikes are scheduled.**

- **Watcher pacing:** already measured in
  `2026-10-19-macos-drain-interval-comparison`, and the spec forbids
  repeating it.
- **drvfs:** the spec's fallback settles the behavior (try the atomic move,
  else delete in place). It needs a test, not a spike.
- **The single 500,000-entry run:** a Phase 8 acceptance check, not a spike.

**Checkpoint 1:** N1–N12 confirmed or amended in this file; audit outputs
attached; seams compile with `just lint` and `just test` still green.

---

## Phase 2: Compact record, store, and migration

Goal: durable approval and recovery data (R19, R25, R26) with no journal.

#### Wave 3 (parallel; disjoint files)

- [ ] **Record v2** — new `recovery/record.rs` shape.
  - Fields per R25: operation id, aliases, original/deletion paths, checkout and
    repository identities, admin record binding, deletion mode, approved branch
    steps with bindings (expected local tip; explicit vs conditional approval;
    for remote: exact destination, validated push endpoint, expected head),
    unlock-needed/unlocked flag, progress markers.
  - No expiry; strict parse per the robustness section; update the module doc
    (the old "never authorizes retry" contract is replaced — fix the doc in the
    same change).
  - Tests: control + field-shape matrix through the public parse result.
- [ ] **Legacy reader** — separate v1 parse plus optional report (≤ v6) parse.
  - Validate record v1 independently of reports; report version alone grants no
    authority; a corrupt/absent report does not block migration.
  - Produce a `LegacyApproval` (bound checkout + intent) and a list of
    unreconstructable branch actions (remote actions are never rebuilt from a
    remote name).
  - Tests: v1+report v6, v1 alone, malformed, unknown version, mismatched
    bindings, unstarted operation (resumes only for the same bound checkout),
    partially deleted `.git` file.

#### Wave 4 (sequential; depends on Wave 3)

- [ ] **Store and migration** — `recovery/store.rs`, `lock.rs`, `storage.rs`.
  - Atomic durable save (write temp, fsync file, rename, fsync dir); a failure
    returns exit 1 before any mutation and says nothing was deleted (R29).
  - `begin`/`reopen`/`recover` for v2; migration publishes the compact v2 record
    durably **before** removing legacy record/reports/journal; interrupted
    migration leaves the originals intact and is re-runnable.
  - Operation lock held through delete, registration, branch steps; waiters
    acquire the lock and re-read state (never infer ownership from a PID); the
    lock wait honors cancellation.
  - Completed operation leaves nothing (record, journal, report, copy record).
  - Never require a readable journal; ignore and delete a leftover one.
- [ ] **Store tests** — crash injection around save/migrate, stale lock,
  concurrent waiters, corrupt/foreign/unknown-version records (plain
  explanation, no deletion), record-write failure.

**Checkpoint 2:** `just test` in `worktree/` green for the new record/store
tests; the old journal still compiles (removed in Phase 7).

---

## Phase 3: Plain deletion engine

Goal: delete a bound checkout without watcher, journal, or content checks
(R20–R22, R24, R29, Deletion boundaries).

#### Wave 5 (sequential core)

- [ ] **Engine core** — new module (e.g. `remove/delete/`) replacing
  `observation/deletion.rs`.
  - Handle-based, no-follow traversal (`openat`/`unlinkat` on Unix; handle-based
    on Windows); links removed as entries, never traversed; revalidate a
    directory boundary before descending.
  - Unlink FIFOs/sockets/devices/hard links without opening them; clearing
    read-only must not follow links or alter another hard link's shared
    permissions.
  - Delete everything possible first; collect remaining entries; retry only
    `Temporary` errors with the N3 backoff on the injected clock; `Permanent`
    errors are recorded, never retried.
  - Final result enum: `Done | RetainedMount | Waiting(blocked) |
    Permanent{path, cause}`; no `Stop`.
  - Refill (R24, N2): re-list and delete; after the threshold request a usage
    query and wait.
  - Cancellation checked in every loop and wait.

#### Wave 6 (parallel; plug into the Wave 5 interface)

- [ ] **Mount detection** — Linux `/proc/self/mountinfo` (bind mounts on the
  same device included), macOS `getfsstat`/`statfs`, Windows volume mount points
  and junction-to-volume; injected `MountInfo` for L1; never unmount or elevate.
- [ ] **Windows waits** — `ERROR_SHARING_VIOLATION` classification, holder naming
  through the Sniff query (initial query only; query again to explain a wait),
  "has a file open" wording unless writing observed, wait messages only on
  holder change; a missing process record is never proof of release.
  Reparse points: cloud placeholders/deduplicated files removed without
  opening/hydrating; unknown reparse type removed as an entry or reported as a
  filesystem failure.
- [ ] **Quarantine and in-place mode** — keep `quarantine/rename.rs`
  no-replacement guarantee and `admin_entry.rs` association checks unchanged.
  - Add drvfs fallback (atomic no-replace rename unsupported → in-place using the
    same record; a check-then-rename is forbidden).
  - R8: rewrite the relative `.git` file for the move, restore the registration;
    if unsafe, use in-place mode.
  - Mount blocking an ancestor's move, and Windows rename blocked by a held file
    → in-place mode.
  - Replaced original after a quarantine move is left untouched and reported;
    in-place replaced root stops the operation (outside the approval).
- [ ] **Error classifier** — N4 `classify` with per-OS tables and unit tests.

**Checkpoint 3:** L1 engine tests: permanent failure deletes everything else and
does not retry; links swapped during traversal expose nothing; hard-linked data
outside the checkout unchanged; backoff verified via injected clock; mount
retained; cancellation in deletion and waits.

---

## Phase 4: Facts and warnings (may run concurrently with Phases 2–3)

Goal: gather everything that is shown in the confirmation question.

#### Wave 7 (parallel)

- [ ] **Sniff process context (R27)** — add missing fields found in Wave 1
  (app name/description, start time, bounded parent chain, activity kind) to
  Sniff; explicit "unavailable" values per platform; update
  `sniff/docs/topics/filesystem-query.md` and the Sniff skill in the same change.
  Must not add repeated full-tree walks.
- [ ] **Submodule inspector (R2)** — recursive over initialized submodules:
  dirty files, unique commits (N6), failed inspection → "could not be checked"
  warning; uninitialized → no scan; `.git` pointing outside the admin record →
  "external Git data retained," not deleted. Deletes `modules/<name>` only when
  it belongs to this worktree's admin record.
- [ ] **State classifier (R3, R9–R13, R15, R17)** — one `Facts` pass producing
  warning records: lock + reason; link target (own checkout vs other, via
  forward and backward association); unreadable Git (with Git's message); file
  at the path; missing directory/index (staged changes unknowable); files `wt`
  could not check (reason, count only where known); shell cwd inside target
  (Unix warn / Windows refuse); synced folder (OneDrive, R6); drvfs (R4);
  nested mounts (R7); vanished checkout → "already removed."
  - Reuse one usage query for warnings and enrichment; a failed/partial query
    is itself a warning.

**Checkpoint 4:** each classifier input has an L1 test with injected sources;
the warning set for a fixture combining several conditions renders every item.

---

## Phase 5: Lifecycle rewire

Goal: run the flowchart; delete the old stops.

#### Wave 8 (sequential)

- [ ] **Lifecycle core** — rewrite `lifecycle.rs`:
  save approval → unlock if approved → move or mark in-place → delete → (wait or
  retain pending) → remove registration and copy record → branch steps → remove
  record. Drop usage/inventory/observation refusals, the free-space gate and
  `Refusal` variants that no longer exist; keep branch/identity/main-checkout
  protections (main checkout is never removed, even with `--force`).
  - After a crash, inspect actual paths and registration, not just the phase
    marker.
  - Update `ExitClass` per N5.

#### Wave 9 (parallel)

- [ ] **Resume and waiter (R16, R17, R19)** — `lifecycle/resume.rs`: finish a
  pending removal silently with earlier approvals (alias is a lookup key only; a
  name matching a pending and a different live checkout stays ambiguous);
  waiter names the running removal, waits interruptibly on the lock, then
  reports completion or continues; vanished checkout = already removed; never
  resume into a replaced directory or admin record.
- [ ] **Handoff (R18)** — `handoff.rs`: 10-minute expiry, single-use, carries the
  operation id; second run verifies checkout/repository identities, caller
  outside the target, and all branch/remote protections, without content
  comparison or re-asking.
- [ ] **Branch steps** — keep tiers, expected tip, push endpoint and lease checks;
  persist per N-record bindings; changed branch/endpoint → kept, exit 1, never a
  new checkout question; already-absent branch succeeds; no branch action added
  because a retry supplied a new flag; unbound legacy remote action stays undone
  with explanation and exit 1; checkout cleanup completes before branch failure
  is shown.
- [ ] **Final-status check (R14)** — one `git status` immediately before
  deletion; diff against the questioned set; surface `also deleted N file(s)
  changed after you answered: …`; never blocks; never claims to be exhaustive.

**Checkpoint 5:** crash-injection matrix (before/after save, unlock, rename,
registration removal, each branch action) all resume without a new question;
replacement roots/admin records and recreated original paths are never deleted.

---

## Phase 6: CLI

Goal: one question, one `--force`, quiet success.

#### Wave 10 (parallel)

- [ ] **Args and completions (R28)** — remove `--force-worktree`; usage error
  (exit 2) when supplied; update help text, examples, shell completions
  (bash/zsh/fish/PowerShell), and `cli/src/commands/remove/policy.rs` matrix.
- [ ] **Question rendering** — via `biscuit-terminal` components (`Prose`,
  `UnorderedList`); every process name, lock reason, path and Git message
  escapes terminal controls and renders as literal text; warning order puts the
  lock first; text says what was seen and what deleting will do; unknown counts
  described as unknown; message that files created or changed before deletion
  finishes are included.
- [ ] **Policy** — `decide`: interactive → one question unless `--force`;
  non-interactive (stdin/stderr not TTY, or `CI` nonempty) → refuse exit 3
  listing what was seen when warnings exist, silent removal when clean;
  `--force` = yes everywhere including waits; branch questions unchanged.

#### Wave 11 (sequential; depends on Wave 10)

- [ ] **Execute and report** — `commands/remove/mod.rs`, `recovery.rs`, `report.rs`,
  `usage.rs`: wire the new lifecycle; stderr for human text, stdout only
  `cd:` / `remove-handoff:`; normal success prints no hidden dir, `Retry:`,
  report path or diagnostics; final line `Removed X (N entries)` with "unknown"
  when not counted; permanent failure prints one plain sentence with path and
  cause; incomplete mount removal exits 1 with an unmount instruction; a real
  failure that needs manual action names the affected path even when hidden.
  Delete `report.rs` machinery and `remove_recovery_report.rs`-style outputs.

**Checkpoint 6:** the spec's example question renders; policy matrix test
updated and green; wrapper protocol snapshot (integration test, not unit) updated.

---

## Phase 7: Remove the old machinery (R23, R25, R26)

Goal: no dead code, no dormant gates, no unused dependencies.

#### Wave 12 (sequential, then parallel clean-up)

- [ ] **Delete code** — `lib/src/remove/observation/` (all adapters, `wait`,
  `injected`, `diagnostics`, `inventory`, tests), `recovery/journal.rs`,
  `recovery/space.rs`, free-space paths in `storage.rs`, `Stop`/`CoverageLost`,
  the Windows Search recheck gate in `usage.rs`, and leftover old `Refusal`
  variants. Keep `admin_entry.rs` and `quarantine/*` logic.
- [ ] **Dependencies** — after confirming no other consumer (Wave 1 map), remove
  `inotify` and `fsevent-sys`, prune unused `windows-sys` features; update
  `worktree/docs/dependencies.md` and the root dependency docs.
- [ ] **Recipes and perf gates** — remove watcher-specific `just` recipes and
  perf gates; keep large-checkout coverage that still applies
  (`lifecycle/tests/large_checkout.rs` rewritten, not deleted blindly).
- [ ] **Drift sweep** — for each remaining symbol whose behavior changed, fix or
  delete its `///`/`//!`/`//` comments (project comment-quality rules);
  report any drift found and how it was resolved.

**Checkpoint 7:** `just lint` and `just test` green; the grep in "What success
looks like" is clean.

---

## Phase 8: Verification

Goal: every Verification bullet in the spec has evidence.

#### Wave 13 (parallel; L1 gap filling)

- [ ] **Ruling coverage audit** — a table R1–R29 → test names; fill gaps.
- [ ] **Submodules, locks, links, mounts** — the spec's L1 cases (submodule with
  uncommitted + unpushed work deleted incl. `modules/<name>`; clean submodule
  adds no warning; locked worktree reason shown and unlocked; own-link vs foreign
  link; nested mount incl. same-device bind mount, second `wt remove` finishes
  without a question).
- [ ] **Change-during-question and permanent failures** — files changed/created/
  staged while the question is open are deleted; final line names unseen
  uncommitted file; injected read-only/access-denied: one sentence, exit 1, no
  retry, later run finishes without a question; unsaveable record deletes nothing.
- [ ] **Recovery and branch regressions** — legacy v1+report v6 finishes without a
  checkout question; malformed/unknown-version records; concurrent removals and
  stale locks; `--force` alone keeps an unsafe branch; moved tips and endpoint/
  lease changes cannot delete the wrong branch.
- [ ] **Non-interactive** — refusal without `--force`, deletion with it, clean
  checkout removes silently, `CI` nonempty treated as non-interactive.

#### Wave 14 (sequential; environment-specific)

- [ ] **L2** — interactive question with a process warning, answered yes, in a tmux
  pane; no window takes focus (`just test-l2`).
- [ ] **Large-checkout acceptance** — one disposable 500,000-entry checkout on
  macOS: single run, no retry, only the question and final line; record elapsed
  time and compact-record size once in the implementation log. No new
  thresholds, no multi-host campaign (see N12).
- [ ] **WSL2 and Windows** — WSL2 `/mnt/<drive>` warns and deletes, hidden-name
  collision still detected and nothing replaced; Windows held-open file is waited
  on, named, and finishes after release. Load the `os` skill first; use
  `just ci-seed` / `just ci-local --plan` per the repo's evidence rules rather
  than claiming an OS is untestable.

**Checkpoint 8:** all L1/L2 green; Windows/WSL2 evidence recorded or the gap
stated with the reason; acceptance numbers recorded.

---

## Phase 9: Documentation and final validation

#### Wave 15 (parallel docs)

- [ ] `worktree/docs/cli/remove.md` — rewrite: the question and warnings,
  `--force`, the Windows wait, silent recovery, exit codes. Lead with what the
  reader can do, include a Mermaid flow of the lifecycle, and do not link to or
  name the feature/fix. Remove watcher/coverage/fseventsd/stop/retry sections and
  the **Planned** markers this spec added.
- [ ] `worktree/README.md` — the `wt remove` bullet and exit-code overview only.
- [ ] `.claude/skills/worktree/` — `remove.md`, `lifecycle.md`, `observation.md`
  (shrink or retire; update the router table in `SKILL.md`), and `cli-contracts.md`
  for exit codes and `--force`.
- [ ] `sniff/docs/topics/filesystem-query.md` and Sniff skill (if fields added in
  Wave 7); `worktree/docs/performance-testing.md`; dependency docs.
- [ ] Add any new OS-specific fact learned (Windows waits, drvfs) to the `os` skill.

#### Wave 16 (sequential)

- [ ] **Final validation** — `just lint`, `just test`, `just test-l2` in
  `worktree/`; `just ci-local --plan` reviewed; `/review:docs-code-sync` style
  check that docs match behavior.
- [ ] **Hand-off state** — implementation log records departures from the spec
  (docs are corrected, spec untouched). Do not move the feature to `_completed`
  and do not run `just complete`; the terminal state is "implementation complete,
  ready for review." Do not commit unless explicitly asked.
