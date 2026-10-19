---
total_phases: 6
created: 2026-10-18
phase: 1
agent: claude/sonnet
yolo: true
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
packages:
  - worktree
  - worktree-cli
source_files_during_phase_4:
  - worktree/lib/src/remove/inventory.rs
  - worktree/lib/src/remove/lifecycle.rs
  - worktree/lib/src/remove/lifecycle/resume.rs
  - worktree/lib/src/remove/lifecycle/tests.rs
  - worktree/lib/src/remove/lifecycle/tests/resume_consent.rs
  - worktree/cli/src/commands/remove/mod.rs
  - worktree/cli/tests/remove_recovery.rs
docs_updated_during_phase_4:
  - worktree/docs/cli/remove.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/worktree/lifecycle.md
source_files_during_phase_5:
  - worktree/lib/src/remove/observation/mod.rs
  - worktree/lib/src/remove/observation/deletion.rs
  - worktree/lib/src/remove/lifecycle.rs
  - worktree/lib/src/remove/lifecycle/resume.rs
  - worktree/lib/src/remove/lifecycle/tests.rs
  - worktree/lib/src/remove/lifecycle/tests/startup_warning.rs
  - worktree/cli/src/commands/remove/recovery.rs
  - worktree/cli/src/commands/remove/mod.rs
  - worktree/cli/tests/remove_recovery.rs
  - worktree/cli/tests/remove_recovery_report.rs
  - worktree/cli/tests/snapshots/remove_recovery_report__a_macos_stall_after_the_move_points_at_fseventsd.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_slow_startup_warns_with_its_duration.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_stop_names_the_fsevents_flags_that_dropped_changes.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_watch_that_never_delivered_refuses_with_its_reason.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_windows_ceiling_names_the_watched_folder.snap
docs_updated_during_phase_5:
  - worktree/docs/cli/remove.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/worktree/observation.md
  - .claude/skills/worktree/lifecycle.md
  - .claude/skills/os/macos.md
  - .claude/skills/os/SKILL.md
source_files_during_phase_6:
  - worktree/lib/src/remove/observation/diagnostics.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/os/linux.md
source_code:
  - worktree/cli/src/commands/remove/mod.rs
  - worktree/cli/src/commands/remove/recovery.rs
  - worktree/cli/tests/remove_recovery.rs
  - worktree/cli/tests/remove_recovery_report.rs
  - worktree/cli/tests/snapshots/remove_recovery_report__a_macos_stall_after_the_move_points_at_fseventsd.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_slow_startup_warns_with_its_duration.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_stop_names_the_fsevents_flags_that_dropped_changes.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_watch_that_never_delivered_refuses_with_its_reason.snap
  - worktree/cli/tests/snapshots/remove_recovery_report__a_windows_ceiling_names_the_watched_folder.snap
  - worktree/lib/src/remove/inventory.rs
  - worktree/lib/src/remove/lifecycle.rs
  - worktree/lib/src/remove/lifecycle/resume.rs
  - worktree/lib/src/remove/lifecycle/tests.rs
  - worktree/lib/src/remove/lifecycle/tests/resume_consent.rs
  - worktree/lib/src/remove/lifecycle/tests/startup_warning.rs
  - worktree/lib/src/remove/observation/deletion.rs
  - worktree/lib/src/remove/observation/deletion/tests.rs
  - worktree/lib/src/remove/observation/diagnostics.rs
  - worktree/lib/src/remove/observation/fsevents.rs
  - worktree/lib/src/remove/observation/fsevents/tests.rs
  - worktree/lib/src/remove/observation/injected.rs
  - worktree/lib/src/remove/observation/inotify.rs
  - worktree/lib/src/remove/observation/inotify/tests.rs
  - worktree/lib/src/remove/observation/inventory/tests.rs
  - worktree/lib/src/remove/observation/mod.rs
  - worktree/lib/src/remove/observation/read_directory_changes.rs
  - worktree/lib/src/remove/observation/read_directory_changes/tests.rs
  - worktree/lib/src/remove/observation/tests.rs
  - worktree/lib/src/remove/observation/wait.rs
  - worktree/lib/src/remove/outcome.rs
  - worktree/lib/src/remove/quarantine/unregister.rs
  - worktree/lib/src/remove/quarantine/unregister/tests.rs
documentation:
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/linux.md
  - .claude/skills/os/macos.md
  - .claude/skills/worktree/lifecycle.md
  - .claude/skills/worktree/observation.md
  - worktree/docs/cli/remove.md
completed_phase: 6
implemented: true
---

# Plan: `wt remove` under a slow or overrun change watcher

## Summary and definition of done

The work lands in the `worktree` library (`lib/`) and `worktree-cli` (`cli/`).
Four changes, in dependency order:

1. **Wait policy.** Replace the fixed 5 s `DRAIN_TIMEOUT` in the macOS
   (`fsevents.rs`) and Windows (`read_directory_changes.rs`) adapters with a
   30 s stall limit plus a 5 min per-drain ceiling, driven by a pure,
   clock-injectable helper. `CoverageLost::DrainTimedOut` becomes
   `DeliveryStalled { waited }` and `DrainCeiling { waited }`. Waits wake for
   progress, loss, deadlines, and the interrupt signal.
2. **macOS deletion cadence.** `DeletionOptions` default drain interval is 512
   on macOS, 2,048 elsewhere.
3. **Loss reasons and diagnostics.** A macOS loss payload keeps the raw
   FSEvents flag mask and every named flag. A bounded per-attempt observation
   diagnostic is persisted in `OperationReport` (`REPORT_VERSION` 4 → 5) and
   rendered by the CLI, including a one-time slow-startup warning.
4. **Resume consent.** A library-owned calculation excludes from consent only
   dirty ` D` entries with a durable `Deleted` journal result in the same
   validated operation and a confirmed absent path; the CLI decision and the
   lifecycle recheck share it.

Done means all of the following:

- [x] Delayed progress can exceed the old 5 s total; silence stalls at 30 s;
      repeated progress cannot pass 5 min; Ctrl-C ends a wait immediately.
- [x] Sticky coverage loss is unchanged and no force flag bypasses it.
- [x] A retry after confirmed deletions needs no extra consent for the
      resulting ` D` entries; every other dirty shape still asks.
- [x] Reports carry the new diagnostics; version-4 reports still load, with
      earlier attempts preserved.
- [x] Docs and skills listed in the spec are updated, and the macOS 100k-entry
      sample result is in the implementation log.
- [x] `just test` and `just lint` pass in `worktree/` on macOS, and the
      evidence rules cover Linux, native Windows, and WSL2.

## Phase 1: Rulings and wait-policy core

### Necessary Rules

Rulings the implementer needs. Each is the plan's recommendation; the author may
override.

- [x] **R1: Interrupt plumbing.** `Observation::drain` and the startup and
      admin-release checks take no interrupt signal today; the deletion engine
      polls it only at entry boundaries. Ruling: pass the existing
      `&dyn Fn() -> bool` interrupt into the adapters' wait loops (a wait
      context struct, not a global). A wait that sees the interrupt returns
      without recording loss, and the caller maps it to the existing
      `Interrupted` path (before the move: existing refusal; after: existing
      interrupted exit).
- [x] **R2: Unclassified `waited` on ceiling and stall tie.** The ceiling wins
      when both expire at the same check (spec). `waited` for `DeliveryStalled`
      is time since the last relevant progress, not since the drain began.
- [x] **R3: What counts as macOS progress.** A newly acknowledged marker
      (greater than the highest seen) or an in-scope event. In-scope means it
      classifies to a `Scope` for this removal; sibling activity, the marker
      directory's own cleanup events, and duplicate or older marker
      acknowledgments do not count. Returning from `FSEventStreamFlushSync`
      is never progress.
- [x] **R4: Windows progress is per awaited handle.** Each handle (parent,
      admin) owns its own stall clock; a completed handle drops out. The
      ceiling is shared across the drain.
- [x] **R5: Startup and admin-release use the same policy.** The macOS
      `barrier()` already serves startup. Admin-release on Windows takes the
      same helper. Linux is unchanged (read-until-empty).
- [x] **R6: Windows diagnostic names the watch, no service remedy.** The
      stall and ceiling variants carry an `Option<Root>` for the affected
      watch (Windows sets it; macOS leaves it `None`). Only the macOS text
      mentions fseventsd.
- [x] **R7: macOS loss payload shape.** Add `CoverageLost::FsEventsDropped
      { raw: u32, flags: Vec<FsEventsLossFlag> }` (enum `KernelDropped`,
      `UserDropped`, `MustScanSubDirs`), emitted only on macOS. Generic
      `Overflow` stays for Linux and Windows. Flags are listed in a fixed
      declaration order for display, with no precedence among them; the raw
      mask is preserved verbatim, including bits with no name.
- [x] **R8: Report field and versioning.** Add an optional structured
      `observation` field to `OperationReport` and to its earlier-attempt
      representation (absent means unknown); `REPORT_VERSION` becomes 5. The
      recovery record and journal formats do not change. Diagnostics hold
      backend, last and longest marker round-trip (ms, `None` until one is
      confirmed), loss reason with raw flags, elapsed stall or drain time, the
      affected watch, the stage where loss was received (startup, pre-move,
      post-move, deletion, unregistration), and entries processed since the
      previous drain during deletion. One struct per attempt, with no sample
      list.
- [x] **R9: Resume consent shape.** Add a resume-specific function beside
      `consent_state` in `lifecycle.rs` that returns the raw inventory, the
      explained inventory (consent view), the exclusion count, and a
      fingerprint bound to both views. First-attempt `consent_state` is
      unchanged. The CLI report/policy step and the lifecycle recheck under the
      operation lock both call the new function.
- [x] **R10: Resume path matching.** Journal paths and Git porcelain paths are
      compared as native `OsString` relative paths using the repository's
      existing native-path helpers: no lowercasing, no prefix matching, no
      display strings. Porcelain parsing for ` D` must use the same `-z`
      status the inventory uses, so quoting does not leak into matching.
- [x] **R11: Wider measurement is the author's call.** The spec asks for one
      quick macOS sample at 100,000+ entries on one host. Parallel builds,
      other hosts, repeated trials, and a Windows or Linux load sample are not
      planned. If the sample loses coverage with 512, record the result and
      stop for the author (see Phase 6).

### Spikes

None. The spec names the observed delays (more than 10 s) as already justifying
the new limits, and the cadence sample is a post-implementation regression
check, not an input to the design.

### Tasks

- [x] **Wait helper.** Create a small module (for example
      `lib/src/remove/observation/wait.rs`, compiled on every platform so it
      tests on Linux CI).
    - A `Clock` trait or closure for monotonic `Instant`; a real implementation
      and a controllable test one.
    - `WaitPolicy { stall: 30 s, ceiling: 5 min }` and a `WaitState` with
      `progress()`, per-key stall clocks (key = handle, so Windows can track
      two), `complete(key)`, and `check() -> Continue | Stalled{waited,key} |
      Ceiling{waited}`.
    - Ceiling wins a tie; clocks never restart for the ceiling.
- [x] **Error type.** In `observation/mod.rs` replace `DrainTimedOut` with
      `DeliveryStalled { waited, watch }` and `DrainCeiling { waited, watch }`
      with texts that say confirmation could not be obtained, not that events
      were dropped. Fix every match arm that compiles on any platform.
- [x] **Helper tests (L1, portable).** Prove: delayed progress passes the old
      5 s total; silence stalls at 30 s; repeated progress cannot extend past
      5 min; duplicate, old, unrelated, and cleanup activity do not reset
      either clock; a busy handle beside a stalled one reports the stalled one;
      a completed handle needs no progress; tie reports the ceiling. No real
      sleeps.

### Validation checkpoint

- [x] `just test worktree` passes on the host for the helper and the error
      rename (macOS and Windows adapter bodies still use the old constant
      until Phase 2, so this phase keeps them compiling by mapping to the new
      variants at the current call sites with the present 5 s values).
- [x] `just lint` clean in `worktree/`.

## Phase 2: Adapter wiring (macOS and Windows)

Wave 1 runs the two adapters concurrently; they share only the Phase 1 helper.

### Wave 1

- [x] **macOS wait.** In `fsevents.rs`:
    - Remove `DRAIN_TIMEOUT` and rewrite `barrier()` and `marker_round()`
      around `WaitState`; the ceiling spans every marker round of one drain.
    - Feed `progress()` from newly acknowledged markers and in-scope events
      (the callback increments a counter the wait loop reads); keep the
      existing quiet-round completion rule (late events are progress but do
      not finish the drain).
    - Replace the 1 ms sleep spin with a bounded wait that wakes for progress,
      loss, deadline, and interrupt. `FSEventStreamFlushSync` blocks, so run
      the flush where the wait can still check deadlines, or bound each flush
      and recheck; do not hold the queue lock while waiting.
    - Update the module docs that cite `DRAIN_TIMEOUT`.
- [x] **Windows wait.** In `read_directory_changes.rs`:
    - Replace the single `Instant::now() + DRAIN_TIMEOUT` deadline with
      `WaitState` keyed per awaited handle; progress on the parent handle must
      not reset the admin handle's stall clock, and the reverse.
    - Apply the same policy to startup and admin-release checks; carry the
      affected `Root` into the loss.
    - Wake for events, loss, deadlines, and interrupt.

### Wave 2 (after Wave 1)

- [x] **Interrupt integration.** Thread the interrupt from the deletion
      engine and lifecycle call sites into `Observation::drain` and the
      startup path (R1); keep before/after-move interruption behavior.
      Linux adapter accepts and ignores the signal.
- [x] **Test seam at the wait boundary.** Existing `Injector::delayed` /
      `deliver_delayed` feed only the event queue. Extend `injected.rs` (and
      each adapter's test module) so marker acknowledgments and in-scope events
      can be delivered on a controlled clock, then prove startup, drain, and
      admin-release each use the policy.
- [x] **Tests (L1).** Interrupt ends a wait without waiting for a deadline;
      sticky loss ends waiting and blocks further deletion; the first loss and
      prior events are preserved. macOS tests stay in nextest's `fsevents-l1`
      group.

### Validation checkpoint

- [x] `just test worktree` on macOS; the Windows adapter cannot compile here, so
      plan the Windows and WSL2 proof through the `os` skill's reachable hosts
      (load it before claiming otherwise) and the existing receipts rules.

## Phase 3: Loss payload, cadence, and report diagnostics

Wave 1 tasks touch separate files and run concurrently; Wave 2 depends on both.

### Wave 1

- [x] **macOS loss payload.** In `fsevents.rs` callback, keep the raw mask and
      all present named flags (`KernelDropped`, `UserDropped`,
      `MustScanSubDirs`); `MustScanSubDirs` alone still loses coverage. Add the
      payload variant (R7); keep `QueueFull` distinct. Tests feed each flag
      and a combined mask through the existing callback seam.
- [x] **Deletion cadence.** In `deletion.rs` make the default
      `drain_every` 512 on macOS and 2,048 on Linux (including WSL2) and
      Windows; delete the comment equating 2,048 deletions with a kernel
      buffer size. Keep depth-level drains, the 256-entry journal batch, and
      the "evaluate after each completed batch" rule. Extend the existing
      drain-count test: macOS default, overshoot bounded by one batch minus
      one entry, depth-level drains still happen.
- [x] **Diagnostics type.** Define the observation diagnostic struct (R8)
      and have `Observation` track last and longest marker round-trip, and the
      stage where a loss was received. No sample accumulation.

### Wave 2 (after Wave 1)

- [x] **Report persistence.** Add the optional field to `OperationReport`
      and to the earlier-attempt representation; `REPORT_VERSION` = 5; resumes
      carry earlier attempts forward; old v4 files read with diagnostics
      unknown.
- [x] **Startup failures.** Carry diagnostics through `ObservationError`
      paths so the CLI can show them before a recovery record exists, without
      promising a report path.
- [x] **Tests (L1).** Serialized loss details with all flags, including
      `MustScanSubDirs` alone; generic `Overflow` and `QueueFull` unchanged on
      Linux and Windows; an old v4 report fixture stays readable with earlier
      attempts preserved; no new required fields in recovery records or
      journals.

### Input robustness note

The matrix applies only to the report reader (a deserializer). Default outcome:
a malformed report is a located read error, as today. Exceptions this plan
adds: the new `observation` field absent → unknown (not an error); explicit
`null` → also unknown, and the test says so; wrong type → read error. This
is one field through one validator, so a single control case plus these three
shapes suffice, in-process at the report-loading API. Before closing the
phase, grep the new field for `#[serde(default)]` and `unwrap_or_default()`
and confirm the absent/null choice is deliberate and documented in the field's
doc comment.

### Validation checkpoint

- [x] `just test worktree` and `just lint` pass; report round-trip and old-report
      tests are green.

## Phase 4: Resume consent

### Tasks

- [x] **Resume consent calculation.** In `lifecycle.rs` / `lifecycle/resume.rs`
      add the function from R9:
    - Validate the operation first; read every attempt's journal. An unreadable
      or invalid journal follows the existing recovery refusal path and enables
      no exemption.
    - Suppress an entry only if porcelain is exactly ` D`, its exact relative
      path has a durable `Deleted` result in this operation, and an
      `lstat`-style inspection confirms absence (an inspection error is not
      absence; a recreated file, symlink, or directory is not absent).
    - `ExternallyAbsent`, `Uncertain`, `Kept`, `Planned`, staged deletions,
      staged additions with absent working file, modifications, untracked
      files, and protected included files keep ordinary consent. A missing
      directory does not cover its descendants.
- [x] **Fingerprint binding.** Keep the raw inventory and full fingerprint,
      including raw index entries, as the change-detection input; bind approval
      to both raw and explained views so the CLI's view and the lifecycle
      recheck agree. A file recreated after the report, or a new staged
      change while consent is pending, invalidates approval.
- [x] **Lock-time recheck.** Repeat the calculation under the operation lock
      before further deletion, capture a fresh surviving-entry deletion
      inventory, and repeat usage and branch checks.
- [x] **CLI wiring.** `cli/src/commands/remove/{mod,policy,report}.rs` use the
      library calculation for the retry's report and policy decision, and
      the retry summary states how many confirmed earlier deletions were
      excluded. Journal counts and reports are unchanged. First-attempt flow is
      untouched.
- [x] **Library tests (L1).** Stop after confirmed deletion, retry, no extra
      consent for resulting ` D`; records spread over several attempts;
      uncertain and externally absent entries; staged changes; a recreated
      path (file, symlink, directory); unreadable journal; a changed path or
      index between report and recheck refuses.
- [x] **CLI test (L1).** One noninteractive retry through the real CLI
      without `--force-worktree` completes; a counterpart containing new work
      refuses. No terminal or browser opens or takes focus.

### Input robustness note

The journal and porcelain readers are loaders, but the journal format is not
changing, so no new loader matrix is added. The existing journal-validity tests
are extended only where this plan changes the outcome (the unreadable-journal
case above). Grep the new code for `filter_map(.. as_str())`, `.ok()` on a
journal or porcelain parse, and `unwrap_or_default()`; each hit must be
justified or turned into a refusal.

### Validation checkpoint

- [x] `just test worktree` and `just test worktree-cli` pass on macOS.

## Phase 5: CLI messages and documentation

Wave 1 runs the messaging and the docs concurrently.

### Wave 1

- [x] **CLI messages.** In `cli/src/commands/remove/recovery.rs`, render
      `DeliveryStalled`, `DrainCeiling`, and the macOS flag payload through
      `biscuit-terminal` components (consult its `Prose`/`UnorderedList`
      skill).
    - macOS stall or ceiling: say fseventsd may be delayed; no automatic
      action. Windows: name the affected watched path/volume, no remedy.
    - Slow startup (confirmation total over 1 s): warn once per attempt before
      moving anything; do not refuse.
    - Keep locations, the retry instruction, and the full-report link.
    - Assert messages at the existing rendering boundary and exit classes (4
      for startup failure; pre-move loss follows the refusal path; post-move
      stop is exit 1 even with zero deletions).
- [x] **Docs.**
    - `worktree/docs/cli/remove.md`: waiting limits, startup warning, loss
      reasons, manual fseventsd troubleshooting (`sudo killall fseventsd`, with
      the system-wide effect, never automatic), resume consent, report fields;
      correct the claim that a macOS marker proves every earlier change arrived.
    - `worktree/README.md`: only overview behavior that changed.
    - `.claude/skills/worktree/observation.md` and `lifecycle.md`; library
      resume entry-point docs.
    - `.claude/skills/os/macos.md`: the fseventsd failure mode and manual
      procedure.
    - Docs never link to or name this fix directory.
    - Fix stale `//!` and `///` comments the changes touched (module docs in
      `fsevents.rs`, `read_directory_changes.rs`, `observation/mod.rs`,
      `deletion.rs`), and report any comment-vs-code drift found.

### Validation checkpoint

- [x] `just test worktree-cli`, `just lint`, and the repo's docs checks for the
      edited kinded documents pass.

## Phase 6: Cross-OS verification and macOS sample

### Tasks

- [x] **Cross-OS evidence.** Load the `os` skill. Obtain L1 evidence for
      `worktree` and `worktree-cli` on Linux, native Windows, and WSL2 through
      the existing recipes and receipts, with no new CI environment or
      performance gate. The Windows and WSL2 legs follow the repo's
      fixed-forward rule.
- [x] **macOS 100k-entry sample.** Once, on this host: generate a checkout with
      at least 100,000 entries (no parallel build), run `wt remove`, and
      record completion, any loss flags from the new diagnostics, and elapsed
      time in the implementation log, using the incident rates (about 6,600 and
      7,000 deletions per second) only as context. Do not add a slow L2 test
      or CI fixture.
- [x] **If 512 still loses coverage.** Record the result, do not claim
      success, and hand the interval or pacing question to the author; a
      larger investigation is the author's decision.
- [x] **Implementation log.** Write `implementation-log.md` in this
      directory: departures from the spec (with docs corrected), the sample
      result, and the evidence per OS. The agent's terminal state is
      "implementation complete, ready for review"; it does not move the fix to
      `_completed`.

### Validation checkpoint

- [x] Every Done item above is checked, or the open item is named in the
      implementation log.
