---
total_phases: 8
created: 2026-10-11
phase: 1
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
packages:
  - worktree
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
`usage.rs`, `outcome.rs`, `recovery/`, `quarantine.rs`, `observation/`,
`lifecycle.rs`), `worktree/cli/src/commands/remove/`, the area's docs/skills,
and tests. Sniff is consumed as-is; no `sniff` CLI spawn, no OS process
discovery duplicated. New dependencies are limited to what R4-R6 select
(likely `notify` plus thin `libc`/`windows-sys` rename bindings) and must be
recorded in dependency docs.

### Done means

- [ ] `just test`, `just test-l2`, and `just lint` pass in the worktree area;
      cross-OS evidence for macOS, Linux, native Windows, and WSL2 is recorded
      per the `os` skill, and `just ci-local --plan` shows no unexpected scope.
- [ ] Every acceptance item (1-12) in the spec maps to a named test (see the
      traceability table written in Phase 8).
- [ ] Controlled writers reproduce retained-cwd writes into the quarantine and
      absolute-path recreation at the original; both stop with preserved
      remnants, honest partial-deletion reports, and no branch operations.
- [ ] Crash injection at each phase boundary leaves a recoverable record; retry
      by the original name works after unregistering and rejects changed
      identities and new content; completed records never authorize a second
      removal.
- [ ] Existing protections (branch safety tiers, expected-tip, consent
      fingerprints, protected includes, handoff records, locked worktrees,
      missing-directory record-only removal, remote endpoints) are retained and
      still tested; exits 3 and 4 still mean "nothing was removed".
- [ ] Structural checks hold: no per-process repeated checkout walks, no
      whole-tree rescan per deleted entry, no unconditional hashing of ignored
      file bodies, no scanning threads left running after a query returns.
- [ ] `worktree/docs/cli/remove.md` describes current behavior with the
      **planned** markers removed only by this change; dependencies docs,
      README, worktree/OS skills, and the implementation log are updated;
      departures from the spec are recorded in the log.
- [ ] Agent terminal state is "implementation complete, ready for review"; the
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
| 5 | 4 | No-replacement rename backends (per OS, parallel); quarantine naming; association update + rollback |
| 6 | 5 | Watcher session; deletion inventory with identity capture (parallel) |
| 7 | 5 | Checked deletion engine + stop conditions + final verification |
| 8 | 6 | Lifecycle orchestration; crash recovery and retry resolution |
| 9 | 7 | CLI partial/recovery reporting, retry wiring, channel discipline |
| 10 | 8 | Docs, skills, dependency records, cross-OS validation, CI scope review |

## Phase 1: Rulings, spikes, and scaffolding decisions

Goal: remove the decisions and unknowns that would otherwise force rework,
before any contract or backend code is written.

### Necessary Rules

The spec leaves these unresolved or only recommends an answer. Each needs the
marked ruling before the phase it gates starts; outcomes and author overrides
are recorded in `implementation-log.md` and the spec file stays unchanged.

- [x] **R1 - Git association strategy (author decision; gates Phases 4-6).**
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
- [x] **R2 - Local observation strategy (author decision; gates Phase 5).**
      The spec's second Open Question. Proceed on the recommended
      **Option 1: native notifications plus one inventory and targeted
      checks**. Unusable local observation refuses before mutation as an
      operational error (this differs from advisory Sniff discovery, which
      permits continuation). Options 2 (polling) and 3 (repeated hashing)
      reduce protection or re-read ignored build output and are rejected
      unless the author overrides.
- [x] **R3 - Sniff latency contract (author confirmation; gates Phase 3).**
      `wt` adopts `query_path_usage` under the dependent spec's finalized
      Option 1: the two-second budget stops scheduling work; a native call
      already running may overrun it; no scanning threads outlive the query.
      The fix spec already accepts this subject to that API decision, which
      landed as Option 1 — confirm and record. Typed `PathUsageError`s are
      matched by variant, never by message text.
- [x] **R4 - Watcher adapter (planning ruling from spike S2; gates Phase 5).**
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
- [x] **R11 - CI scope (planning ruling; verified in Phase 8).** All new tests
      are self-contained in temp directories with fixture-owned child
      processes and stand-in remotes; no `[package.metadata.ci.tests]
      source-inputs`, no prompts, no terminal focus. Verify with
      `just ci-local --plan` before pushing.
- [x] **R12 - Self-exclusion mechanics (planning ruling; gates Phase 3/5).**
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
      Phase 5.
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
recommendation"; R4-R6 decided by S1-S3 and logged. Do not start Phase 4 or 5
work without them.

## Phase 2: Recovery records, journal, and typed outcomes (library contracts)

Module: `worktree/lib/src/remove/` — new `outcome.rs` and `recovery/`
(`record.rs`, `journal.rs`, `lock.rs`, `store.rs`). No mutation of checkouts
yet; everything is exercisable against temp directories.

### Wave 2 (parallel)

- [ ] **Typed outcomes** - `outcome.rs`: the operation-level outcome enum
      (`usage_refused`, `removed`, `activity_detected`, `cleanup_failed`,
      `state_unverified`), the moved/deleted/retained state description, and
      the operation report model (timeline, discovery evidence with coverage,
      observed activity, journal summary, Git registration state, branch
      outcomes, errors, truncation flags). Library result types only — no new
      user-facing JSON CLI option.
      - Complexity: the report doubles as the persisted `report.json`; keep
        the serde model versioned from the start.
- [ ] **Record schema and reader** - `recovery/record.rs`: serde model binding
      operation ID, original/quarantine native paths and identity,
      repository/admin identity, branch and approved tip, consent fingerprint,
      discovery report, requested name and aliases, intended actions, record
      version, and completed phase. Paths reuse Sniff's lossless
      `NativeString`. Strict reader: reject unknown versions, malformed
      native paths, wrong ownership, repository/admin identity mismatch,
      duplicate JSON keys, and trailing content; validate the storage
      directory without following replacement links.
      - Prerequisite: R7.
- [ ] **Journal format** - `recovery/journal.rs`: the entry model
      distinguishing planned, confirmed operation-owned deletions, externally
      absent, and uncertain attempted deletions, with types (file, directory,
      link) and directory-entry-deletion vs bytes-destroyed wording retained
      for reports; batch-intent records; storage-limit behavior that stops
      deletion instead of discarding confirmed entries.

### Wave 3 (parallel; depends on Wave 2 types)

- [ ] **Record store and lock** - `recovery/store.rs` + `lock.rs`: atomic
      write with owner-restricted permissions and fsync (R7's primitive,
      crash guarantees documented at the module level); OS-backed exclusive
      lock via the existing `fs4` dependency keyed by repository/admin
      identity, held for the operation's duration; pending-record detection;
      completion persistence and journal clearing with the bookkeeping-warning
      path when clearing fails.
- [ ] **Journal writer** - Persistence of batch intent before unlinking and
      confirmed results afterward per R7's fsync points; the crash window
      between unlink and result persistence recovers as `uncertain`; append
      and fsync behaviors tested against injected write failures.
- [ ] **Robustness matrix tests** - One test per format (record, journal)
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

- [ ] **Library usage module** - `remove/usage.rs`: wrap
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
      - Prerequisites: R3, R12; the decisive call site lands in Phase 6's
        orchestration, behind this module's seam from the start.
- [ ] **CLI refusal rendering** - In `cli/src/commands/remove/`: render the
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

## Phase 4: Quarantine move and Git association

### Wave 5 (parallel)

- [ ] **Rename backends** - `remove/quarantine.rs` with per-OS primitives
      (R5): no-replacement atomic rename, returning typed error kinds
      (collision / sharing violation (Windows) / cross-device / permission /
      other I/O). Per-OS tests: an existing destination is never replaced; a
      failed rename leaves the original intact; identity survives the move;
      a multi-step mover inspects both locations before claiming the original
      was kept.
      - Prerequisite: R5 (S1).
- [ ] **Quarantine naming** - Collision-resistant hidden sibling selection
      (`.wt-remove-<slug>-<token>` beside the target, per R7's Windows
      hidden-attribute note); same-filesystem by construction; never a global
      temp dir; never copy-and-delete; the destination is created only by the
      no-replacement move itself.
- [ ] **Association update and rollback** - Under R1/R6: re-list and validate
      lock status and admin/checkout links immediately before moving; perform
      the move; update exactly the target's own links with verification of
      forward/back references (or the verified `git worktree move`); never
      unlock a locked worktree, never repeat `--force`; refuse submodule and
      unsupported topologies before any mutation with the documented
      explanation. After a successful rename but failed association update:
      verified rollback only when the original path is still absent, no
      destructive work occurred, and no unexpected activity was observed;
      otherwise preserve the quarantine and report actual state. Map the
      Windows sharing failure to exit 4; permission/collision/other I/O to
      operational errors explicitly not labeled as process usage.
      - Prerequisite: R1, R6 (S3); overlaps the rename backend task only at
        the interface, so it can develop against a manual move.

Checkpoint: per-OS rename tests green (macOS locally; Windows/Linux legs
exercised per the `os` skill or their CI environment); association tests show
Git reading the quarantine under its retained registration, submodule and
locked-worktree refusals with nothing changed, and rollback rules holding;
acceptance item 5's rename-collision/failure cases pass.

## Phase 5: Activity observation and checked deletion

### Wave 6 (parallel)

- [ ] **Watcher session** - `remove/observation/watcher.rs` per R4: bounded
      queues; watch the original's parent (recreation), the captured
      checkout/quarantine (descendant activity) with explicit handling of
      platform watch invalidation across the rename, and the admin-dir files
      behind the target's identity and approved index (split-index and entry
      flags included). Overflow, lost watches, and unestablishable
      observation are surfaced as lost coverage — never a silent downgrade;
      the session registers its handles with Phase 3's exclusion registry;
      events carry receipt times and are matched narrowly by path, kind, and
      captured identity, with operation-owned rename/unlink/metadata events
      distinguished from unexplained creates, replacements, and
      modifications; parent events outside the two target trees do not stop
      removal.
      - Prerequisite: R2, R4 (S2).
- [ ] **Deletion inventory** - `remove/observation/inventory.rs`: after
      observation is active and before moving, capture every entry approved
      for deletion (ignored content included, FIFOs/devices detected as
      ineligible rather than opened), per-entry native identity, type,
      relevant metadata, and link target; content hashes for the existing
      consent-sensitive fingerprint paths via the existing
      fingerprint/`biscuit-hash` machinery; never directory mtime or entry
      count alone; a finishing consistency check refuses before mutation if
      entries changed or cannot be inspected; persist the inventory with the
      journal before the move so restart recovery can compare retained
      entries to their original identities.

### Wave 7 (depends on Wave 6)

- [ ] **Checked deletion engine** - `remove/observation/deletion.rs`:
      children-first, bounded batches (R7), cap-std parent-directory handles
      and relative operations (no following links; an approved link itself
      is removed, never its target), per-entry identity revalidation
      immediately before unlink, drain and inspect exact affected paths
      between batches and after the final batch, stop before deleting a
      newly appeared or detectably changed entry (modifications and
      replacements of existing files included), record an externally
      disappeared approved entry as already absent while an unexplained
      disappearance stops the operation, journal updates at each boundary,
      and `.git` plus the administrative index kept until ordinary content
      cleanup finishes where the chosen strategy permits. No arbitrary sleep
      as proof, no retry-until-the-writer-loses.
- [ ] **Final verification** - Both locations checked (quarantine absence,
      no original-path recreation through the final observation drain and
      identity/inventory checks); index bytes and Git's interpreted entries
      re-bound to the approved state before unregistering; the report states
      the concurrent-write limit and identifies already deleted content.

Checkpoint: controlled-writer tests (child readiness handshakes, never
sleeps; guaranteed cleanup of children, handles, and watches on failure)
reproduce retained-cwd writes into the quarantine and absolute-path
recreation at the original, both stopping with preserved remnants and honest
reports; overflow/invalidation injection stops with lost coverage; structural
checks (no whole-tree rescan per entry, no unconditional hashing) hold;
acceptance items 3 and 4 pass at library level.

## Phase 6: Lifecycle orchestration, crash recovery, and retry

### Wave 8 (parallel tasks, integrated at the phase checkpoint)

- [ ] **Operation lifecycle** - `remove/lifecycle.rs`: one flow binding the
      existing guards/preparation/questions/fingerprint checks, the fresh
      usage query (Phase 3), durable record creation (Phase 2), the move
      (Phase 4), checked deletion and final verification (Phase 5), the
      targeted unregister, branch and remote actions only after verified
      checkout completion (existing expected-tip and endpoint rules), copy
      record removal only after verification, completion persistence before
      clearing pending state, and interruption handling at entry boundaries.
      Typed outcomes feed the exit-code mapping (R9) and the operation
      report.
      - Prerequisite: Phases 2-5.
- [ ] **Crash recovery** - On any start, a pending record blocks a new
      operation on the same target; restart inspects actual state to resolve
      the interrupted phase and never treats a saved completed-phase value as
      proof that paths or Git still have that state; a crash after rename
      leaves enough information to find retained content without the original
      registration or directory name; recovery-record/journal write failures
      stop destructive work and report the last durable phase plus subsequent
      known or uncertain actions.
- [ ] **Retry resolution** - Resolve the requested name against the union of
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

## Phase 7: CLI reporting and recovery UX

### Wave 9

- [ ] **Wire the operation** - `cli/src/commands/remove/`: `execute`'s
      healthy-checkout path calls the lifecycle instead of
      `remove_worktree`; the missing-directory path keeps record-only
      removal with no Sniff query; the decisive usage query sits after
      questions and final identity/fingerprint checks in `run`, and after
      handoff verification in the wrapper's second invocation (the first
      invocation reports, collects consent, and hands off without refusing
      on the parent shell's cwd); consent/identity bindings are rechecked
      after discovery and observation setup; `files_unchanged` semantics
      carry into the new flow.
- [ ] **Partial and recovery reports** - Compact human reports per the
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
- [ ] **Tests** - CLI integration tests (`cli/tests/remove.rs` and a new
      recovery-focused integration test): representative wiring and recovery
      through the real binary; refusal exit 3 with evidence; exit 4 paths;
      post-move stop exit 1 with both locations; wrapper two-invocation
      discovery placement; stdout protocol unchanged. Snapshot tests live in
      integration tests only (the double-target trap).

Checkpoint: acceptance items 1, 2, 8, 9 pass through the real binary;
existing remove/handoff/wrapper tests still green; `just test`, `just
test-l2`, `just lint` pass locally.

## Phase 8: Docs, skills, and cross-OS validation

### Wave 10 (parallel)

- [ ] **Docs** - Rewrite `worktree/docs/cli/remove.md`'s planned section into
      current behavior (exit-code meanings, eligible topologies and
      filesystem limits, retry from the base checkout, the recovery record
      location and report); touch `worktree/README.md` where public behavior
      changes; update `docs/dependencies.md` files for any crate R4-R6 added.
- [ ] **Skills** - Update the worktree skill's `remove.md` (and `testing.md`
      if fixtures change) and the `os` skill with any newly learned
      platform facts, in the same change as the code that established them.
- [ ] **Implementation log and traceability** - Record rulings, spike
      findings, departures from the spec, and the acceptance-item (1-12) to
      test mapping table.
- [ ] **Cross-OS validation** - macOS locally; Linux, native Windows, and
      WSL2 evidence per the `os` skill (real child-process/rename/handle/
      watch tests are ordinary L1 tests when they need no terminal);
      representative shell-wrapper tests stay at their existing tiers;
      `just ci-local --plan` reviewed against R11.

Checkpoint: docs describe current behavior with planned markers removed only
for built behavior; all four OSes have recorded evidence; structural checks
re-verified; agent terminal state is "implementation complete, ready for
review".
