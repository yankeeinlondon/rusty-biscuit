---
created: 2026-11-14
total_phases: 4
phase: 5
agent: claude/sonnet
yolo: true
source_files_during_phase_1:
- sniff/lib/src/process.rs
- sniff/lib/src/process/tree.rs
- sniff/lib/src/process/tree/unix.rs
- sniff/lib/src/process/tree/unix/tests.rs
- sniff/lib/src/process/tree/windows.rs
- sniff/lib/src/process/tree/windows/tests.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
- sniff
- darkmatter
- claudine
source_files_during_phase_2:
- darkmatter/lib/src/markdown/compose/shell_expansion/launcher.rs
- darkmatter/lib/src/editor/launch.rs
- darkmatter/lib/src/editor/mod.rs
- darkmatter/lib/src/editor/service.rs
- darkmatter/lib/src/lifecycle/executor.rs
- darkmatter/lib/src/lifecycle/executor/tests.rs
- darkmatter/lib/src/lifecycle/md_host/tests.rs
- darkmatter/lib/src/lib.rs
- darkmatter/lib/src/process_tree.rs
- darkmatter/lib/Cargo.toml
- claudine/lib/src/composition/sequence/task/shell.rs
- claudine/lib/src/composition/sequence/task/shell/tests.rs
- claudine/lib/src/composition/error/mod.rs
- claudine/lib/Cargo.toml
- .config/nextest.toml
docs_updated_during_phase_2:
- darkmatter/docs/dependencies.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
- .claude/skills/sniff/testing.md
- .claude/skills/os/macos.md
source_files_during_phase_3:
- sniff/lib/src/programs/install/options.rs
docs_updated_during_phase_3:
- sniff/docs/sniff-library-architecture.md
- sniff/lib/README.md
- sniff/README.md
- sniff/cli/README.md
- darkmatter/docs/topics/lifecycle/index.md
- darkmatter/docs/topics/lifecycle/error-handling.md
- darkmatter/docs/inline/shell-expansion.md
- claudine/docs/topics/flow-control/lifecycle.md
- claudine/docs/topics/flow-control/sequences.md
docs_created_during_phase_3:
- sniff/docs/topics/process-trees.md
skills_files_updated_during_phase_3:
- .claude/skills/sniff/architecture.md
- .claude/skills/sniff/SKILL.md
- .claude/skills/os/SKILL.md
- .claude/skills/os/windows.md
- .claude/skills/os/macos.md
- .claude/skills/darkmatter/lifecycle.md
- .claude/skills/darkmatter/library-surfaces.md
- .claude/skills/claudine/SKILL.md
source_files_during_phase_4:
- sniff/lib/src/process.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/launcher.rs
- darkmatter/lib/src/lifecycle/executor.rs
- darkmatter/lib/src/lifecycle/md_host/tests.rs
- darkmatter/lib/tests/l1/lifecycle_edit_action.rs
- .config/nextest.toml
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
- .claude/skills/os/SKILL.md
- .claude/skills/os/macos.md
- .claude/skills/sniff/testing.md
source_code:
- sniff/lib/src/process.rs
- sniff/lib/src/process/tree.rs
- sniff/lib/src/process/tree/unix.rs
- sniff/lib/src/process/tree/unix/tests.rs
- sniff/lib/src/process/tree/windows.rs
- sniff/lib/src/process/tree/windows/tests.rs
- sniff/lib/src/programs/install/options.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/launcher.rs
- darkmatter/lib/src/editor/launch.rs
- darkmatter/lib/src/editor/mod.rs
- darkmatter/lib/src/editor/service.rs
- darkmatter/lib/src/lifecycle/executor.rs
- darkmatter/lib/src/lifecycle/executor/tests.rs
- darkmatter/lib/src/lifecycle/md_host/tests.rs
- darkmatter/lib/src/lib.rs
- darkmatter/lib/src/process_tree.rs
- darkmatter/lib/Cargo.toml
- darkmatter/lib/tests/l1/lifecycle_edit_action.rs
- claudine/lib/src/composition/sequence/task/shell.rs
- claudine/lib/src/composition/sequence/task/shell/tests.rs
- claudine/lib/src/composition/error/mod.rs
- claudine/lib/Cargo.toml
- .config/nextest.toml
documentation:
- darkmatter/docs/dependencies.md
- sniff/docs/topics/process-trees.md
- sniff/docs/sniff-library-architecture.md
- sniff/lib/README.md
- sniff/README.md
- sniff/cli/README.md
- darkmatter/docs/topics/lifecycle/index.md
- darkmatter/docs/topics/lifecycle/error-handling.md
- darkmatter/docs/inline/shell-expansion.md
- claudine/docs/topics/flow-control/lifecycle.md
- claudine/docs/topics/flow-control/sequences.md
- .claude/skills/sniff/testing.md
- .claude/skills/sniff/architecture.md
- .claude/skills/sniff/SKILL.md
- .claude/skills/os/SKILL.md
- .claude/skills/os/windows.md
- .claude/skills/os/macos.md
- .claude/skills/darkmatter/lifecycle.md
- .claude/skills/darkmatter/library-surfaces.md
- .claude/skills/claudine/SKILL.md
completed_phase: 4
implemented: true
---

# Plan: one shared process tree, race-free on Windows

Source specification: `2026-11-14-shared-process-tree`. Packages touched:
`sniff` (owner), `darkmatter` (login-shell probe launcher, editor launcher,
`edit` warning plumbing), `claudine` (task shell and lifecycle commands),
plus docs and repo skills.

## Summary and Definition of Done

Three packages each own "a spawned child and everything it starts" with a
private `ProcessTree`. Darkmatter's spawns the child running and assigns it to
the Windows Job afterwards, so a descendant started in that gap escapes.
Claudine's and Sniff's spawn suspended, assign, then resume, and each carries a
capability the others lack (Claudine: `Drop` kill and a bounded repeated group
kill on macOS; Sniff: observed Unix descendants with start times; Darkmatter:
graceful stop and `release` for a GUI editor).

This plan builds **one public owner in `sniff::process`** that is the union of
those capabilities, migrates all three callers to it, deletes the duplicates,
and publishes one canonical page, `sniff/docs/topics/process-trees.md`.

The work, in dependency order:

1. **Phase 1** — `sniff::process::{TreeConfig, TreeIsolation, ProcessTree,
   CleanupScan, TreeTermination}` with Unix and Windows implementations behind
   test seams, Sniff's own `run_command_with_timeout` moved onto it, and the
   tests for every operation.
2. **Phase 2** — Darkmatter (probe launcher, editor launcher, `EditOutcome`
   warning) and Claudine (task shells, lifecycle commands) call Sniff; the
   private owners, local thread-resume and isolation code, and unneeded
   `windows` crate features are removed.
3. **Phase 3** — the canonical topic page, and every doc, README, skill, and
   source comment that accepted the Windows assignment window or promised
   unconditional Unix containment is corrected.
4. **Phase 4** — package-wide verification, the stale-claim and
   delegation searches, OS evidence, and the implementation log.

**Done means all of the following are true:**

- [x] `sniff::process` exposes the owner; `darkmatter/lib/src/process_tree.rs`
  is gone; Claudine's `ProcessTree`, `resume_main_thread`, `as_handle`,
  `isolate_process_tree`, and `GROUP_REAP_LIMIT` are gone; Sniff's nested
  `process_tree` modules are replaced by the public implementation.
- [x] On Windows every owned child is created `CREATE_SUSPENDED` (combined with
  the caller's own flags in one place), assigned to a kill-on-close,
  non-breakaway, non-inheritable Job, and resumed only after assignment
  succeeds; every handle is owned and closed once.
- [x] A short successful probe performs zero process-table scans; the first
  sample is due 250 ms after attachment and later samples no more often than
  that; Windows sampling does nothing.
- [x] Recorded Unix descendants and the original root are identity-checked by
  start time before they are used as traversal roots and before signaling.
- [x] Immediate, graceful, `release`, and armed-`Drop` behave as the contract
  table in the specification says, including disarm-after-success, and a
  Windows `release` failure leaves cleanup armed.
- [x] The editor still survives a completed wait on a GUI client; a `release`
  failure produces a warning (the edit still succeeds); lifecycle `edit`
  warnings reach `LifecycleEffects::warn`.
- [x] Claudine keeps its grace duration, cancellation checks, runaway cap,
  error categories, and teardown-before-reader-settlement order; the Unix
  foreground-terminal lifecycle path keeps direct-child-only cancellation and
  is documented as the one exception.
- [x] `just test` and `just lint` pass in `sniff`, `darkmatter`, `claudine`;
  the terminal tests pass through `just test-l2`; Sniff builds with no
  features and Darkmatter builds with `sniff`'s `remote` feature. (Phase 4:
  two Claudine L2 tests in `level2_initialize_generated_transclusion` fail
  for a reason unrelated to process trees; see the implementation log.)
- [x] `sniff/docs/topics/process-trees.md` exists, is linked from every place
  the specification lists, and a search for the stale claims (Phase 4) finds
  none in current docs, skills, or source comments.
- [x] `sniff/fixes/2026-11-14-shared-process-tree/implementation-log.md`
  records the evidence obtained per OS and every OS evidence gap.

Out of scope, unchanged by this plan: Claudine CLI provider-session
supervision (`claudine/cli/src/commands/wrap/exec/**`), Sniff's
`configure_detached_child`, a Linux cgroup or subreaper, any process-global
signal handler, and the foreground handoff for lifecycle commands (a separate
follow-up spec).

The **Input Robustness Matrix does not apply**: nothing in this work reads a
file format or configuration. The only parsed input is the `u32` creation-flag
word and a PID, which Rule 3 and Rule 12 cover directly.

## Phase 1: The shared owner in Sniff

Goal: `sniff::process` exposes the owner, Sniff's own subprocess helper uses
it, and all owner behavior is covered by deterministic L1 tests.

### Necessary Rules

Every rule states the decision, then the evidence.

**Rule 1 — Layout.** The owner lives in `sniff/lib/src/process/tree.rs`, with
`sniff/lib/src/process/tree/unix.rs` and `sniff/lib/src/process/tree/windows.rs`
(each `#[cfg]`-gated from `tree.rs`), and tests in
`sniff/lib/src/process/tree/unix/tests.rs`, `.../windows/tests.rs`, and an
inline `#[cfg(test)] mod tests` in `tree.rs` for cross-platform items.
`sniff/lib/src/process.rs` keeps the timeout helper and gains
`mod tree; pub use tree::{CleanupScan, ProcessTree, TreeConfig, TreeIsolation,
TreeTermination, WINDOWS_CREATE_NEW_PROCESS_GROUP, WINDOWS_CREATE_NO_WINDOW};`.
The test-only `sample_hook` module, `DESCENDANT_SAMPLE_INTERVAL`, and both
nested `mod process_tree` blocks leave `process.rs` and live under `tree`;
`process.rs` tests reach the hook as
`super::tree::sample_hook::install` (declared `pub(in crate::process)`).
Evidence: `process.rs` is a file module (lines 1–1520) so a sibling
`process/` directory is the idiomatic split; `sample_hook` is used by
`a_descendant_that_detaches_between_samples_escapes_containment`
(`process.rs:1194`) and must keep running on the thread that calls
`run_command_with_timeout`.

**Rule 2 — Public API, exact signatures.** All items are in `sniff::process`.

```rust
pub const WINDOWS_CREATE_NO_WINDOW: u32 = 0x0800_0000;
pub const WINDOWS_CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeIsolation { ProcessGroup, Session }          // Session == ProcessGroup on Windows

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeConfig { /* isolation, windows_creation_flags */ }
impl TreeConfig {
    pub const fn process_group() -> Self;                 // flags = 0
    pub const fn session() -> Self;                       // flags = 0
    #[must_use] pub const fn windows_creation_flags(self, flags: u32) -> Self;
    pub fn configure(self, command: &mut Command);        // before Command::spawn
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupScan { WhenObserved, Always }

#[derive(Debug)]
pub struct TreeTermination { pub status: io::Result<ExitStatus>, pub cleanup: io::Result<()> }

pub struct ProcessTree { /* private */ }                  // manual Debug impl printing the root PID only
impl ProcessTree {
    pub fn attach(child: &Child) -> io::Result<ProcessTree>;
    pub fn sample(&mut self);
    pub fn terminate(&mut self, scan: CleanupScan) -> io::Result<()>;
    pub fn terminate_gracefully(&mut self, grace: Duration, child: &mut Child) -> TreeTermination;
    pub fn release(self) -> io::Result<()>;
}
impl Drop for ProcessTree { /* armed: best-effort hard terminate, never panics */ }
```

`ProcessTree` is `#[must_use = "dropping an owned tree terminates it"]`.
Rationale: spec "Shared API contract"; two named constants are public so
callers (Darkmatter probes, Claudine) need no `cfg` and no duplicated literals
(the literals today are `launcher.rs:160` and `shell.rs:1042`).
`terminate_gracefully` returns a struct instead of one `io::Result` because the
spec requires "a secondary cleanup failure must not turn cancellation into
success or replace its exit status": Claudine builds `exit_code` from `status`
(`shell.rs:622`), so `status` must survive a `cleanup` error.

Each platform module exports `pub(super) struct Core` with exactly:
`attach(&Child) -> io::Result<Core>`, `sample(&mut self)`,
`terminate(&mut self, CleanupScan) -> io::Result<()>`,
`terminate_gracefully(&mut self, Duration, &mut Child) -> TreeTermination`,
`release(self) -> io::Result<()>`, `Drop`, plus `pub(super) fn configure(&mut
Command, TreeConfig)`. `ProcessTree` is a thin wrapper that forwards. This is
the contract that lets the Unix and Windows tasks run in parallel.

**Rule 3 — Configuration.**
- Unix, `ProcessGroup`: `command.process_group(0)` only. `Session`:
  `command.pre_exec(|| if libc::setsid() == -1 { Err(last_os_error()) } else { Ok(()) })`
  only. Never both (the spec: `setsid` fails for a group leader). `configure`
  must be called **before** the caller registers any other `pre_exec` hook:
  std applies `process_group` before all `pre_exec` closures, and closures run
  in registration order, so caller hooks (the editor's `tcsetpgrp` handoff)
  see the new group/session. Evidence: `launch.rs` comment "after std has
  applied `process_group(0)`"; `launcher.rs:130-153` current `setsid` hook.
- Windows: the only `creation_flags` call for an owned command is inside
  `configure`, with value `combined_creation_flags(flags) = flags |
  CREATE_SUSPENDED (0x0000_0004)`. `combined_creation_flags` is a
  platform-independent `const fn` in `tree.rs` so a Unix host can unit-test it.
  Callers must not call `creation_flags` after `configure` (documented in the
  `configure` rustdoc). Flag values per caller: editor `0` (shared console),
  login-shell probe `WINDOWS_CREATE_NO_WINDOW`, Claudine
  `WINDOWS_CREATE_NEW_PROCESS_GROUP`, Sniff probes `0` (Sniff sets no
  `CREATE_NO_WINDOW` today, `process.rs:458`; unchanged).
- A config with `Session` on Windows behaves as `ProcessGroup` (there is no
  session concept); documented on the variant.

**Rule 4 — Unix core is generic over a host seam.** `unix.rs` defines
`pub(super) trait Host { fn now(&self) -> Instant; fn refresh(&mut self);
fn snapshot(&self) -> Vec<Observed>; fn kill_pid(&mut self, pid: u32, signal:
i32) -> io::Result<()>; fn kill_group(&mut self, pgid: i32, signal: i32) ->
io::Result<()>; }` with `Observed { pid: u32, parent: Option<u32>, start_time:
u64 }`, a `SystemHost` (sysinfo `System` plus `libc::kill`) and `Core<H:
Host>`; the exported `Core` is `Core<SystemHost>`. Tests use a `FakeHost` that
records every call. Evidence: the spec requires "injected process
observations for reuse and sample scheduling" and "zero scans … without timing
a benchmark"; sysinfo is already a Sniff dependency
(`sniff/lib/Cargo.toml`), `process_tree` at `process.rs:281` already uses
`ProcessesToUpdate::All`/`ProcessRefreshKind::nothing()`.

**Rule 5 — Unix identity and observation.**
`Core` fields: `root: u32`, `root_start: Option<u64>`, `observed:
HashMap<u32, u64>`, `attached_at: Instant`, `next_sample: Instant =
attached_at + DESCENDANT_SAMPLE_INTERVAL (250 ms)`, `armed: bool`.
- `attach` performs **no** `refresh`/`snapshot` (zero scans at attachment).
- `sample_at(now)` (called by `sample()` with `host.now()`) returns at once
  when `now < next_sample`; otherwise it sets `next_sample = now + 250 ms`,
  refreshes once, adopts `root_start` from the snapshot entry for `root` when
  `root_start` is `None` (the root is alive and unreaped while the caller is in
  its wait loop), drops `observed` entries that are absent or whose start time
  differs, then walks descendants from `[root if root_start == table start] ∪
  observed keys` and inserts `(pid, start_time)` for each. After a sample it
  calls `sample_hook::after_sample()` under `#[cfg(test)]`.
- `sample()` on a non-due call performs no host call except `now()`.
Evidence: spec "Observe Unix descendants…"; today's loop schedules samples in
`run_command_with_timeout` (`process.rs:687-708`) and `sample()` always
refreshes (`process.rs:319`).

**Rule 6 — Cleanup scan, targets, and ordering (Unix).**
`terminate(scan)`: if already disarmed return `Ok(())`. Run the scan block when
`scan == Always || !observed.is_empty()`: refresh once; validated roots =
`root` when `root_start == Some(start of that pid in the snapshot)` plus every
`observed` pid whose snapshot start time matches; targets = descendants of the
validated roots plus the validated observed pids; send `SIGKILL` to each target
deepest-last (reverse order, as `process.rs:388`); clear `observed`. A root
whose `root_start` is `None` or mismatched is **not** a traversal root (spec:
"missing or unverifiable roots are skipped"; no attachment-time scan to learn
it). Then hard-kill the group (Rule 7). On success set `armed = false`; on
failure leave `armed = true` so `Drop` retries.
Consequence recorded in the docs: before the first sample no `setsid`
descendant is addressable beyond group membership. The existing tests
`a_quiet_session_detached_descendant_is_terminated` (1 s timeout) and
`a_detached_descendant_is_terminated_after_its_parent_exits_successfully`
(`3 × 250 ms` sleeps) run past the first sample and keep passing.

**Rule 7 — Group kill and error classification (Unix).**
`kill_group_hard(host, pgid)`: `host.kill_group(pgid, SIGKILL)`. Result
mapping: `Ok` → success; `ESRCH` → success (already absent); `EPERM` →
success on every Unix except Linux/Android (a macOS group holding only unreaped
zombies answers `EPERM`; Linux answers `0`), an error on Linux/Android; any
other errno → error. On non-Linux/Android Unix, after the first success repeat
`kill_group` while it returns `Ok` and less than `GROUP_REAP_LIMIT` (250 ms,
moved verbatim from Claudine `shell.rs:96`) has passed, sleeping 1 ms between
calls — Claudine's macOS fork-race mitigation (`shell.rs:887-903`,
`.claude/skills/os/macos.md` "Process groups"). Linux/WSL2 skip the loop. The
module and rustdoc must **not** repeat the old claim that group signaling is
atomic with forks.

**Rule 8 — Graceful termination (Unix).**
`terminate_gracefully(grace, child)`: if already disarmed or the child has
exited, skip straight to the wait. Otherwise refresh once, compute validated
targets as in Rule 6 without killing, send `SIGTERM` to the group
(`ESRCH` ignored) and to each target, then poll `child.try_wait()` every 20 ms
until `Instant::now() >= deadline`. Then run `terminate(CleanupScan::Always)`
unconditionally (sweeps a descendant that ignored `SIGTERM` even when the root
exited first). If that returns an error, call `child.kill()` (ignore its
result) before waiting. Reap with `child.wait()` only if `try_wait` has not
already returned the status. Return `TreeTermination { status, cleanup }`.
Keep the existing comment about the accepted group-ID reuse window (Darkmatter
`process_tree.rs:80-89`) at the final sweep, in the owner. Evidence: spec
"Graceful termination"; Darkmatter `terminate_gracefully`; Claudine
`terminate` (`shell.rs:847-871`).

**Rule 9 — Release and Drop (Unix).** `release(mut self)`: set `armed = false`,
clear `observed`, return `Ok(())`; no signal. `Drop`: when `armed`, run
`terminate(CleanupScan::WhenObserved)` and discard the result; never panic,
never wait for the root. A released or successfully terminated owner never
signals on drop. A failed `terminate` leaves `armed = true`, so `Drop` retries
once. Evidence: spec "Terminate, release, and drop" and the paragraph after it.

**Rule 10 — Unix attach verification.** `attach(child)`: `pid =
i32::try_from(child.id())`; a failed conversion or `pid <= 0` is
`io::ErrorKind::InvalidInput` ("child process ID does not fit a process-group
ID"). Then `getpgid(pid)`: `ESRCH` → `Ok` (macOS answers `ESRCH` for an
exited-but-unreaped child; Claudine `shell.rs:825-837`); other negative result
→ the OS error; `pgid != pid` → `io::ErrorKind::Other`, message
`child {pid} is in foreign process group {pgid}`. The check is a free function
`verify_root(pid: i32, getpgid: impl Fn(i32) -> io::Result<i32>)` so tests can
drive it without a real process. Both isolation modes pass (`setsid` makes
pgid == pid). Rustdoc states the caller requirement: configured by
`TreeConfig::configure`, not yet reaped.

**Rule 11 — Windows attach (race-free) and failure ownership.** `Core` holds
`job: windows::core::Owned<HANDLE>` and `armed: bool`. Steps, in order, each
preceded by a `step(Step::X)?` call: `CreateJob`
(`CreateJobObjectW(None, PCWSTR::null())`, so the handle is non-inheritable),
`SetKillOnClose` (extended limit info with only
`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`; no breakaway flags),
`Assign` (`AssignProcessToJobObject(job, child handle)`),
`FindThread` (Toolhelp thread snapshot in an `Owned<HANDLE>`, first thread whose
`th32OwnerProcessID == pid`; exhausting the snapshot is `ErrorKind::NotFound`),
`Resume` (`OpenThread(THREAD_SUSPEND_RESUME)` in an `Owned<HANDLE>`, then
`ResumeThread`, `u32::MAX` is an error via `last_os_error`). Resume is
unreachable unless `Assign` succeeded. Every handle is an `Owned<HANDLE>` from
creation, so each is closed exactly once on every path, including an error
return. A failure returns the underlying error (via `io::Error::other` for
`windows::core::Error`) and the dropped Job (kill-on-close, assigned) kills a
suspended child that was assigned; a child that failed before `Assign` is left
suspended for the caller to kill. Callers always `child.kill()` + `child.wait()`
on `Err`. No fallback to killing only the root is ever a success mode.
Test seam: `#[cfg(test)] pub(super) mod hooks` with a thread-local
`Vec<Step>` log and `inject(Step, io::Error)`, consulted by `step`. Steps:
`CreateJob, SetKillOnClose, Assign, FindThread, Resume, ClearKillOnClose,
Terminate`. Evidence: Sniff `process.rs:463-534` (already suspended/assign/
resume), Claudine `shell.rs:917-1016`; spec "Configure, spawn, and attach".

**Rule 12 — Windows operations.** `sample`: no work (no hook step, no
process-table call). `terminate(_)`: `step(Terminate)?`;
`TerminateJobObject(job, 1)`; success sets `armed = false`; repeated call after
success returns `Ok(())` without touching the Job. `terminate_gracefully(_,
child)`: `terminate(Always)`; on error `child.kill()`; then `child.wait()`;
returns `TreeTermination` (grace unused; documented). `release(self)`:
`step(ClearKillOnClose)?` then `SetInformationJobObject` with empty limit
flags; success sets `armed = false` and returns `Ok(())`; failure returns the
error and the owner is dropped while armed, so `Drop` terminates the Job and
the GUI window closes (never reports success while kill-on-close is set).
`Drop`: when armed, `TerminateJobObject` best-effort, ignoring the result; the
`Owned<HANDLE>` then closes the Job.

**Rule 13 — Errors stay visible, categories stay put.** The owner returns
errors; it never logs. Every migrated caller logs a cleanup error with
`tracing::warn!(error = %e, "…")` and keeps its own error categories:
`ProcessError::Spawn(attach error)` in Sniff, `None` in the probe launcher,
`EditorError::Io { operation: "contain the editor's process tree", … }` for
the editor, `TaskShellError::Isolation` for Claudine. A cleanup error never
replaces a timeout, cancellation, or exit status. Evidence: spec final
paragraph of "Terminate, release, and drop"; `tracing` is a dependency of all
three packages.

**Rule 14 — Sniff's `run_command_with_timeout`.** Replace
`process_tree::configure(command)` with `TreeConfig::process_group().configure(command)`;
`attach` failure keeps `child.kill(); child.wait(); return
Err(ProcessError::Spawn(error))`. Remove the local `next_sample`/`elapsed >=
next_sample` logic: after the deadline check call `process_tree.sample()` on
every `Ok(None)` iteration. Timeout and wait-error paths call
`terminate(CleanupScan::Always)` (log an error, Rule 13), then `child.wait()`.
The pipe-configuration failure path does the same. The epilogue calls
`terminate(CleanupScan::WhenObserved)` (a no-op after a successful explicit
hard cleanup). `drain_control.finish()` and reader joining are unchanged, so
bounded Unix pipe draining stays. The `performance` counters are unchanged.

**Rule 15 — Existing Sniff tests.** Every existing test in `process.rs`
`mod tests` stays and keeps its assertions, including
`a_descendant_that_detaches_between_samples_escapes_containment` and its
`EscapedDescendant` guard. Evidence: spec "Move or extend existing Sniff
tests… retain Sniff's deterministic test demonstrating the unobserved escape".

**Rule 16 — Test tier and conventions.** All new tests are L1 (a controlled
subprocess exercising an OS API is still L1 per the spec). Real-process tests
use bounded readiness markers (PID files polled up to 10 s) and each fixture
has its own cleanup guard that `SIGKILL`s (Unix) or `TerminateProcess`es
(Windows) its pids on drop. No terminal or browser is touched. They run under
`just test` in `sniff` (nextest).

**Rule 17 — No spike, no benchmark.** The spec states performance
verification is structural (zero scans, 250 ms deferral, no Windows sampling)
and requires no benchmark or spike; the `FakeHost` call counts verify it.

**Rule 18 — No `Cargo.toml` change in `sniff`.** `libc`, `sysinfo`, `tracing`
and the `windows` features `Win32_System_JobObjects`,
`Win32_System_Diagnostics_ToolHelp`, `Win32_System_Threading`,
`Win32_Foundation` are already declared (`sniff/lib/Cargo.toml`). The owner
needs no `reqwest`, so a no-feature build stays free of HTTP.

### Wave 1 (parallel: Tasks 1.1 and 1.2 touch disjoint files)

- [x] **1.1 Public API and Unix core**
  - Create `process/tree.rs` (public items of Rule 2, `combined_creation_flags`,
    rustdoc with the `## Examples` required by Rule 19 below, `cfg`
    wiring) and `process/tree/unix.rs` (Rules 4–10).
  - Move `DESCENDANT_SAMPLE_INTERVAL` (make it `pub(crate)`), `sample_hook`, and
    the macOS `GROUP_REAP_LIMIT` into `tree`.
  - Delete the two nested `mod process_tree` blocks and `sample_hook` from
    `process.rs`, add the `mod tree; pub use …` lines, and keep `process.rs`
    compiling by temporarily leaving `run_command_with_timeout` calling the
    new API per Rule 14 (Task 1.3 finishes the rest).
  - Rustdoc requirement: `ProcessTree` and `configure`/`attach` rustdoc state
    who waits for the root, that reaping (waiting for an exited direct child) is
    not termination, that an attached-by-reference owner cannot reap in `Drop`,
    and show the attachment-failure cleanup (`child.kill(); child.wait();`) in
    the `## Examples` block (`no_run`).
  - Risk: none beyond keeping Unix-only code behind `#[cfg(unix)]`; the
    Windows module file does not exist until Task 1.2, so gate the `mod windows`
    declaration with `#[cfg(windows)]` and land both tasks in one change set.

- [x] **1.2 Windows core**
  - Create `process/tree/windows.rs` implementing Rules 11–12 and the `hooks`
    test seam.
  - Move the Toolhelp thread lookup here from `process.rs:501-534`, using
    `Owned<HANDLE>` for the snapshot and the thread; delete Claudine-style
    raw-handle code (none is copied).
  - Risk: cannot be compiled on the macOS host. Run
    `cargo check -p sniff --target x86_64-pc-windows-msvc` only if that target is
    installed; otherwise the Windows compile evidence comes from Phase 4
    (task 4.4). Keep every `unsafe` block annotated with a `// SAFETY:` line.

### Wave 2 (parallel: Tasks 1.3, 1.4, 1.5 touch different files)

- [x] **1.3 Migrate Sniff's helper**
  - Finish Rule 14 in `process.rs`; update the module-level docs and the docs
    of `run_with_timeout`/`run_command_with_timeout` per Phase 3 task 3.2's
    list of stale claims (rewrite lines 1–52 and the `## Notes` blocks at
    603–611 and 634–639: remove "`kill(-pgid)` is atomic with respect to
    forks", state the public owner, link
    `sniff/docs/topics/process-trees.md`, keep the installation-boundary
    paragraph about third-party installers).
  - Replace "The one public item, `configure_detached_child`" with text saying
    the module publicly exposes the owner and `configure_detached_child`
    (`configure_detached_child` contract is unchanged).
  - Pass `just test` for the existing `process::tests`.

- [x] **1.4 Unix owner tests** (`process/tree/unix/tests.rs`)
  - With `FakeHost`: `no_scan_at_attach`; `first_sample_waits_one_interval`;
    `samples_are_no_more_frequent_than_the_interval`;
    `cleanup_scan_policy` (three cases: `WhenObserved` with nothing observed →
    0 refreshes, `Always` → 1, `WhenObserved` with an observed pid → 1);
    `recycled_observed_pid_is_not_a_traversal_root_or_target` (fake snapshot
    returns a different start time for an observed pid that has children;
    assert no `kill_pid` for it or its children);
    `root_never_sampled_is_not_a_traversal_root`;
    `root_with_changed_start_is_not_a_traversal_root`;
    `successful_hard_cleanup_disarms` (a second `terminate` and the drop make
    no further `kill_group` calls); `failed_cleanup_keeps_the_drop_backstop`
    (fake returns `EACCES`, drop calls `kill_group` again);
    `release_never_signals` (drop after release: zero kills; observations
    cleared); `group_kill_repeats_off_linux` (`#[cfg(not(any(target_os =
    "linux", target_os = "android")))]`: fake answers `Ok, Ok, ESRCH` → 3 calls;
    and a `#[cfg(any(target_os = "linux", target_os = "android"))]` twin
    asserting exactly 1 call). This is the "focused regression" the spec allows
    because no existing test exercises the repeat deterministically.
  - Real-process tests: `attach_accepts_a_child_that_already_exited` (spawn
    `sh -c true` configured as a group, poll `kill(pid, 0)`-independent: sleep
    300 ms without reaping as Claudine's test does, then `attach` is `Ok`, then
    `wait`); `attach_rejects_a_live_root_in_a_foreign_group` (spawn `sleep 30`
    without `configure`, guard kills it, assert `Err` with message containing
    `foreign process group`); `verify_root_rejects_invalid_ids` (`0`, negative,
    and a `getpgid` closure returning `EPERM`);
    `session_mode_makes_the_child_a_session_leader` (`getsid(pid) == pid`) and
    `group_mode_does_not` (`getsid(pid) != pid`, `getpgid(pid) == pid`);
    `a_caller_pre_exec_hook_runs_after_group_setup` (hook returns `Err` unless
    `getpgrp() == getpid()`; spawn `Ok` proves order);
    `graceful_cleanup_when_the_root_exits_but_a_descendant_ignores_sigterm`
    (root `sh` traps `TERM` to `exit 0`, backgrounds an inner `sh` that runs
    `trap '' TERM; echo $$ > pidfile; while :; do sleep 1; done`; wait for the
    pid file, call `terminate_gracefully(Duration::from_millis(300), …)`, assert
    `status` is `Ok`, the root is reaped, the inner pid is dead within 3 s;
    guard kills the inner pid on drop);
    `immediate_terminate_kills_a_group_member_that_left_the_root_alone`
    (background `sleep 60` in the group; `terminate(Always)`; pid dead).
  - Risk: tests that share a group with the test runner must always `configure`
    their command, otherwise `terminate` would signal the runner's group.

- [x] **1.5 Windows owner tests** (`process/tree/windows/tests.rs`,
  plus cross-platform `combined_creation_flags` tests in `tree.rs`)
  - Cross-platform: `0 → 0x4`, `WINDOWS_CREATE_NO_WINDOW → 0x0800_0004`,
    `WINDOWS_CREATE_NEW_PROCESS_GROUP → 0x204`, and that suspension is never
    lost for any input.
  - Windows: `attach_assigns_before_resuming` (step log equals
    `[CreateJob, SetKillOnClose, Assign, FindThread, Resume]`);
    one injected-failure test per step (`CreateJob`, `SetKillOnClose`,
    `Assign`, `FindThread`, `Resume`) asserting `attach` is `Err`, the log stops
    at the failing step, `Resume` appears only when every earlier step
    succeeded, and after the test does `child.kill()` + `child.wait()` the
    child has exited (no suspended child);
    `failed_attach_does_not_leak_handles` (repeat a `Resume`-fault attach
    25 times; `GetProcessHandleCount` grows by fewer than 10);
    `a_descendant_created_immediately_is_in_the_job` (the existing
    `child_spawns_pipe_holding_descendant`-style fixture re-used from
    `process.rs` tests via a `pub(super)` helper writes its descendant PID to a
    file; after `attach` wait for the file, `terminate(Always)`, assert the
    descendant has exited using `OpenProcess`+`WaitForSingleObject(… 5 s)`;
    guard `TerminateProcess`es it on drop);
    `release_leaves_a_member_running` (descendant alive after `release` and
    drop; guard terminates it); `release_failure_keeps_cleanup_armed`
    (`inject(ClearKillOnClose)`; `release` returns `Err`; the member is
    terminated by the drop); `sampling_does_nothing` (`sample()` adds no step).
  - Risk: written on macOS, executed in Phase 4 (task 4.4). Keep every test
    free of scheduling races (ordering comes from the step log).

### Wave 3 (validation checkpoint)

- [x] **1.6 Phase 1 checkpoint**
  - From `sniff/`: `just test` and `just lint` pass; `cargo check -p sniff
    --all-targets` passes with no features; `cargo check -p sniff --all-targets
    --features remote` passes.
  - `git grep -n "mod process_tree" sniff/lib/src` returns nothing.
  - Do not start Phase 2 until all four hold.

## Phase 2: Migrate the consumers

Goal: Darkmatter and Claudine delegate tree setup, Job assignment/resume,
and termination to Sniff, and their private owners are deleted.

### Necessary Rules

**Rule 19 — Rustdoc and `## Examples` on the new public items.** Follow the
repo's rustdoc rules (no H1; `## Examples`, `## Errors`, `## Notes` only where
they add a constraint the types do not). The `ProcessTree` example is `no_run`
and shows configure → spawn → attach (with failure cleanup) → sample →
terminate. Evidence: spec "This low-level API … Its usage examples must show
the attachment-failure cleanup"; `CLAUDE.md` "Code Comment Quality".

**Rule 20 — Darkmatter login-shell launcher
(`darkmatter/lib/src/markdown/compose/shell_expansion/launcher.rs`).**
Delete `configure_detached` (all three cfg variants). In `run_shell_query`:
`TreeConfig::session().windows_creation_flags(WINDOWS_CREATE_NO_WINDOW).configure(command)`
after the stdio setup; spawn; `ProcessTree::attach` failure → `child.kill()`,
`child.wait()`, warn, `return None`. In the wait loop, call `tree.sample()` on
every iteration (5 ms poll); the deadline/`Err` arm calls
`tree.terminate(CleanupScan::Always)` (warn on error), then `child.kill()`,
`child.wait()`, `break None`. After the loop call
`tree.terminate(CleanupScan::WhenObserved)` (warn on error) before
`status?`. Keep the 100 ms `EOF_GRACE` wait and the stdout-tail logic. Rewrite
the module docs "Cleanup" section: the Unix child leads a new session and
group; observed descendants are covered; Windows uses the suspended-start Job
(delete the sentence accepting the assignment window); link the topic. Keep the
"Job-control hazard" section. The four existing tests in this file are kept.
Evidence: spec consumer table row "Darkmatter login-shell queries".

**Rule 21 — Darkmatter editor launcher
(`darkmatter/lib/src/editor/launch.rs`).**
- Replace `own_process_group` with
  `TreeConfig::process_group().configure(&mut command)` (no per-OS split),
  called **before** `terminal::Handoff::prepare` so the handoff `pre_exec` runs
  after group setup (Rule 3). The `terminal` module (foreground handoff and
  restore) stays in this file untouched.
- `ProcessTree::attach(&child)` `Err(source)` → `child.kill()`,
  `child.wait()`, `EditorError::Io { operation: "contain the editor's process
  tree", source }` (the synthetic "could not be placed" error is removed).
- The wait loop is polling whenever a tree is owned **or** a cancellation
  callback exists: each `try_wait` → `Ok(None)` iteration calls
  `tree.sample()` (when a tree exists), then checks `cancelled`, then sleeps
  20 ms. Only an unowned launch with no callback keeps the blocking
  `child.wait()` (this is the `PLAIN` / `md edit` path). Change `wait`'s
  signature to `wait(child, tree: Option<&mut ProcessTree>, cancelled)`.
- `Waited::Exited` (including a nonzero status): `tree.release()`; an `Err(e)`
  becomes `warning = Some(format!("Windows could not keep the editor running
  after the edit; it was closed: {e}"))` (release only fails on Windows; on
  Unix it is `Ok`). The edit outcome (`Ok` or `NonZeroExit`) is unchanged.
- `Waited::Cancelled | Waited::Failed`: `tree.terminate_gracefully(
  EDITOR_TERMINATE_GRACE, &mut child)`; log `cleanup` errors (Rule 13); ignore
  `status`; the error returned is still `Cancelled` or `Io { "wait for the
  editor" }`. The unowned path still does `child.kill(); child.wait()`.
- Add `pub struct LaunchReport { pub result: Result<(), EditorError>, pub
  warning: Option<String> }` and `LaunchPlan::run_reporting(self, options) ->
  LaunchReport` holding all the logic; `LaunchPlan::run` becomes
  `self.run_reporting(options).result` and its rustdoc says the release warning
  is dropped (the hosts without a warning channel: `md edit`, wrapper `--edit`,
  which never own a tree and so never produce one). Re-export `LaunchReport`
  from `darkmatter/lib/src/editor/mod.rs` next to `LaunchPlan`.
- Update the module docs and the `own_tree`/`EDITOR_TERMINATE_GRACE` docs;
  delete the comment accepting the Windows assignment window; keep the
  terminal-handoff section.
Evidence: spec consumer-table editor row and "An editor release failure does
not fail the edit (author ruling)"; `launch.rs:161-212`; the only
`LaunchPlan::run` callers are `service.rs:196` and `editor/mod.rs:254`.

**Rule 22 — `edit` warning plumbing.** Add `pub warning: Option<String>` to
`EditOutcome` (`darkmatter/lib/src/editor/service.rs:55`) with rustdoc: set when
the edit's outcome stands but the host should tell the user something.
`ProcessEditorService::edit` passes a `&mut Option<String>` into `run`; `run`
uses `run_reporting` and stores `report.warning` before `report.result?`.
In `darkmatter/lib/src/lifecycle/executor.rs` `dispatch_edit`
(`:1329-1349`), directly after the `outcome.attempted` block and before the
`match outcome.result`, add `if let Some(text) = &outcome.warning {
self.effects.warn(text); }` — the same channel the `Ignored` verb uses
(`:714`), so `--deny all` blocks it through the host's existing warning
handling with no new deny code. Update the `EditOutcome` literals:
`darkmatter/lib/src/lifecycle/executor/tests.rs:500` (`FakeEditor` gains a
`warning: Option<String>` field, default `None`) and
`service.rs:164`. Claudine's `ClaudineEditorService`
(`claudine/lib/src/composition/lifecycle/editor.rs:24`) forwards the outcome
unchanged, so it needs no edit. Add one executor test: a `FakeEditor` that
reports a warning makes the `edit` action succeed, the next action in the
stack still runs, and the recording effects double sees the warning text once.

**Rule 23 — Darkmatter manifests, module, and deps docs.**
Delete `darkmatter/lib/src/process_tree.rs` and the `mod process_tree;` line
(`darkmatter/lib/src/lib.rs:121`). In `darkmatter/lib/Cargo.toml` change the
Windows `windows` entry to `features = ["Win32_Foundation"]` (the only other
user is `atomic_replace.rs`, which imports `Win32::Foundation` only) and
rewrite its comment to say it supplies the error codes `atomic_replace.rs`
matches; keep `libc` (still used by `editor/launch.rs`, `editor/surface.rs`,
`shell_expansion/probe.rs`) and rewrite its comment to the terminal-foreground
handoff and tests (the launcher no longer calls `setsid` itself). Update
`darkmatter/docs/dependencies.md`: the "Login-shell Probes" bullet
(`:131-137`) — `windows` is `Win32_Foundation` only, process-tree ownership is
Sniff's — and the `libc` bullet (`:205-216`) — `setsid` is applied through
Sniff's session mode; the explanation of why a session (not a group) is needed
stays.

**Rule 24 — Claudine task shells and lifecycle commands
(`claudine/lib/src/composition/sequence/task/shell.rs`).**
- Delete: both `struct ProcessTree` definitions and their `impl`/`Drop`
  blocks, `resume_main_thread`, `as_handle`, both `isolate_process_tree`, and
  `GROUP_REAP_LIMIT`. `TREE_KILL_GRACE` loses its `#[cfg(unix)]` (it is passed to
  `terminate_gracefully` on every OS) and its doc says Windows terminates the
  Job at once and ignores it.
- Add `fn tree_config() -> TreeConfig {
  TreeConfig::process_group().windows_creation_flags(WINDOWS_CREATE_NEW_PROCESS_GROUP) }`
  and use `tree_config().configure(&mut builder)` where `isolate_process_tree`
  was called (`run_process`, `run_owned_lifecycle_command`).
- `run_process`: `let ownership = sniff::process::ProcessTree::attach(&child);`
  with the existing `#[cfg(test)] fail_isolation` injection unchanged (drop the
  real tree first); an `Err` still does `child.kill(); child.wait();
  Err(TaskShellError::Isolation(error))`. `let mut tree`. In the wait loop, on
  `Ok(None)` call `tree.sample()` before the cancellation/runaway/timeout
  checks. The three kill triggers do `let termination =
  tree.terminate_gracefully(TREE_KILL_GRACE, &mut child); warn on
  termination.cleanup; break termination.status;` (replacing `tree.terminate(&mut
  child); break child.wait();`). The epilogue replaces `drop(tree)` with
  `tree.terminate(if wait_outcome.is_err() { CleanupScan::Always } else {
  CleanupScan::WhenObserved })` (warn on error) followed by `drop(tree)`; it
  stays **before** the reader settlement, preserving "teardown before reader
  settlement".
- `run_owned_lifecycle_command`: same configure/attach (an `Err` keeps
  `child.kill(); child.wait(); return Err(error)`), `tree.sample()` per
  `Ok(None)`, `terminate_gracefully(TREE_KILL_GRACE, …)` on a wait error (return
  the wait `Err(error)`) and on cancellation (return `Ok(130)`), and an explicit
  `tree.terminate(CleanupScan::WhenObserved)` (warn on error) before returning
  the exit code.
- Untouched: `holds_terminal_foreground`, `run_in_own_group`, the runaway cap,
  `READER_SHUTDOWN_GRACE`, the live-stream flush. The Unix terminal-foreground
  lifecycle path keeps direct-child-only cancellation; this is the documented
  exception.
- Remove the unit test `ownership_survives_a_child_that_exited_before_verification`
  (`shell/tests.rs:107-132`); it tests the deleted private owner and its
  scenario is now `attach_accepts_a_child_that_already_exited` in Sniff (task
  1.4). All other Claudine shell, sequence, and lifecycle tests keep their
  assertions.
- Rewrite the module docs "Ownership contract": establishment is fallible
  (attach failure → `Isolation`), descendants are reaped at completion on every
  exit path through Sniff's owner, and the Windows bullet says the start is
  suspended-then-assigned-then-resumed by Sniff; link the topic page; keep the
  claim that a panic unwind reaches the reap through the owner's `Drop`.
- Update the doc comment on `TaskShellError::Isolation` / the
  `SequenceTaskShellIsolation` comment (`claudine/lib/src/composition/error/mod.rs:2306-2310`)
  only to say "Sniff's process-tree owner" in place of the mechanism list.
Evidence: spec consumer table row "Claudine sequence shell tasks and isolated
lifecycle commands"; `shell.rs:488-595, 1103-1136`.

**Rule 25 — Claudine manifest.** In `claudine/lib/Cargo.toml` the Windows
`windows` features become `["Win32_Foundation"]` (the remaining user is
`config/atomic.rs`, which imports `Win32::Foundation` only; `JobObjects`,
`Threading`, and `Diagnostics_ToolHelp` were used only by `shell.rs`), with the
two comments replaced by one stating what `Win32_Foundation` serves. `libc`
stays (still used by `holds_terminal_foreground`, `run_in_own_group`, and
tests); its comment is rewritten to "terminal foreground checks and direct-child
signals for lifecycle commands in `composition/sequence/task/shell.rs`".
`claudine/docs/dependencies.md` has no entry for these features, so it needs no
edit.

### Wave 4 (parallel: Tasks 2.1, 2.2, 2.3 touch different packages/files)

- [x] **2.1 Darkmatter probe launcher** — Rule 20.
- [x] **2.2 Darkmatter editor and `edit` warning** — Rules 21 and 22 (files:
  `editor/launch.rs`, `editor/service.rs`, `editor/mod.rs`,
  `lifecycle/executor.rs`, `lifecycle/executor/tests.rs`).
- [x] **2.3 Claudine** — Rules 24 and 25.

### Wave 5 (after Wave 4; Darkmatter-only)

- [x] **2.4 Darkmatter cleanup** — Rule 23 (delete `process_tree.rs` and the
  `mod` line, edit `Cargo.toml`, edit `docs/dependencies.md`). Prerequisite: 2.1
  and 2.2 no longer reference `crate::process_tree`.

### Wave 6 (validation checkpoint)

- [x] **2.5 Phase 2 checkpoint**
  - `just test` and `just lint` pass in `darkmatter` and `claudine`.
  - `cargo check -p darkmatter --all-targets` passes (Sniff with `remote`);
    `cargo check -p claudine --all-targets` passes.
  - The existing tests named in the spec still pass unchanged:
    `darkmatter/cli/tests/l1/editor_launcher.rs`
    (`cancellation_terminates_the_whole_tree_and_reaps_it`,
    `an_editor_ignoring_sigterm_is_killed_after_the_grace_period`,
    `cancelling_a_gui_client_leaves_an_already_running_editor_alone`),
    `editor_service.rs`, the four tests in `launcher.rs`, the probe tests in
    `probe.rs`, and Claudine's `composition::sequence::task` tests.
  - `git grep -n "crate::process_tree\|process_tree::" darkmatter claudine
    -- '*.rs'` returns nothing.

## Phase 3: Documentation, skills, and stale-claim removal

Goal: one canonical page, linked from every place that describes process
ownership; no current doc, skill, or comment accepts the Windows assignment
window or promises unconditional Unix containment.

### Necessary Rules

**Rule 26 — The canonical page.** `sniff/docs/topics/process-trees.md`,
written for a developer new to the repository (plain headings, no spec or
feature name, no `kind:` frontmatter — the sibling topics have none). Sections
in order: what a process tree owner is and why (cancel background work, avoid a
leftover process holding a pipe); configure → spawn → attach (Rust example
including the attachment-failure `kill`/`wait`, and the two isolation modes plus
`windows_creation_flags`); who waits for the root (reaping vs. terminating; the
caller reaps; `Drop` cannot); sampling and immediate cleanup example
(`sample()` per poll iteration, `CleanupScan` policy: zero scans for a short
success, a scan after interruption or failure, a scan when descendants were
observed); graceful cleanup example (`terminate_gracefully` returns
`TreeTermination`; Windows terminates at once); `release` example (GUI editor
survival; Windows `release` can fail and then cleanup stays armed); `Drop`
backstop; error handling (log cleanup, never replace the original failure);
**Guarantees and limits** (Windows: race-free after attach, an enclosing Job
that forbids assignment makes attach fail — failure-path, not a degraded mode;
Unix: group members plus observed descendants; the unobserved fork-and-detach
escape; group-ID reuse window; start-time checks are best effort; macOS
repeated group kill, Linux not; ownership boundary — an existing GUI server is
not a descendant; `Drop` does not run on abort or `process::exit`); **Caller
policies** table (Sniff probes/install, Darkmatter probes, Darkmatter editor,
Claudine task shells and lifecycle commands, and the Unix
terminal-foreground lifecycle exception with the reason: an owned group is not
the terminal's foreground group so the command would be stopped on its first
terminal read); and the Mermaid lifecycle diagram from the specification
(configure → spawn → attach → owned; failure cleanup; terminate / release /
drop endings). Evidence: spec "Documentation", `CLAUDE.md` "Drift
Maintenance" (docs audience rules, Mermaid for lifecycles).

**Rule 27 — Exactly where links and corrections go.** (Line numbers are
today's; locate by the quoted text.)
- `sniff/docs/sniff-library-architecture.md:631-650` ("Subprocess Deadlines"):
  replace "Tree termination is total on Windows and best-effort on Unix" block
  with a short statement that the helper uses the public owner and a link to the
  topic; keep the paragraph about `DESCENDANT_SAMPLE_INTERVAL` only as a pointer;
  keep the installer-boundary paragraph.
- `sniff/lib/README.md`: add a short "Process trees" bullet in the module list
  (near the Filesystem Usage Queries bullet, line 10 style) linking
  `../docs/topics/process-trees.md`; edit the install timeout paragraph at
  `:951-957` ("Windows containment is kernel-enforced via Job Objects and
  total") to say Windows containment applies once the owner is attached.
- `sniff/README.md`: add one bullet to the Sniff Library list linking the topic.
- `sniff/cli/README.md:262-278`: in the partial-install paragraph, keep the
  Unix `setsid` escape wording but change "confined to a kill-on-close Job
  Object … nothing escapes" to the attached-Job wording and link the topic; per
  the "README is an overview" rule keep it to two sentences.
- `.claude/skills/sniff/architecture.md:275-285`: "`process` is public only for
  `configure_detached_child`" → public for the owner and
  `configure_detached_child`; link the topic. Also add one line to
  `.claude/skills/sniff/SKILL.md` (bounded subprocess bullet, line 20) naming
  the owner and linking the topic.
- `.claude/skills/os/SKILL.md` row at line 91 (lifecycle `edit` on Windows):
  drop "the Job is assigned just after spawn (a child started in that window
  escapes, accepted)" and say the Job is assigned before the editor runs and
  that a failed assignment fails the edit; link the topic.
- `.claude/skills/os/windows.md:172-200`: rewrite the "Job assignment window"
  sub-bullet (no window; assignment precedes resume; a failed `attach` fails the
  edit), keep the shared-console and `.cmd` sub-bullets, replace
  `process_tree.rs`/`ProcessTree::attach returns None` with Sniff's owner, and
  add to the SSH-session bullet at `:172-176` one sentence: the
  degrade-to-direct-child fallback is `windows_wait_loop`'s provider policy;
  the shared owner reports the assignment failure and each caller fails closed
  (Rule 13). Link the topic.
- `.claude/skills/os/macos.md:84-93`: "Claudine's `ProcessTree` drop
  (`…/shell.rs`) repeats the kill" → "the shared owner in
  `sniff/lib/src/process/tree/unix.rs` repeats the kill".
- `.claude/skills/darkmatter/lifecycle.md:86-96` and
  `.claude/skills/darkmatter/library-surfaces.md:246-262`: replace
  `process_tree.rs` / `crate::process_tree::ProcessTree` with the Sniff owner,
  add `LaunchReport`/`EditOutcome::warning`, link the topic.
- `.claude/skills/claudine/SKILL.md:468-478`: keep the
  `holds_terminal_foreground` exception text; say task shells and lifecycle
  shells use Sniff's owner and link the topic.
- `darkmatter/docs/topics/lifecycle/index.md:636-655` and
  `darkmatter/docs/topics/lifecycle/error-handling.md:190-200`: link the
  topic, add the sentence that if Windows cannot keep a finished GUI editor
  running, the edit still succeeds and a warning says the editor was closed
  (and `--deny all` blocks that warning like any other), and remove nothing
  else.
- `darkmatter/docs/inline/shell-expansion.md` after line 140: one sentence that
  the login-shell query is bounded and cleaned up as a process tree, linking the
  topic.
- `claudine/docs/topics/flow-control/lifecycle.md:1055-1062`: keep the
  foreground exception; add the owner link and one sentence that all other
  lifecycle shell commands and sequence shell tasks own their tree.
- `claudine/docs/topics/flow-control/sequences.md` "Shell tasks" (after the
  timeout paragraph at `:1117-1120`): add two sentences — each command owns its
  process tree, so a timeout, Ctrl-C, or sibling failure stops background
  processes it started — and the link.
- Source comments: `darkmatter/lib/src/editor/launch.rs`,
  `.../launcher.rs`, Claudine `shell.rs`, Sniff `process.rs` are handled in
  Rules 20, 21, 24 and Task 1.3; Task 3.5 greps to confirm none remain.
Link paths are relative to each file and never name this spec or its feature
directory.

**Rule 28 — Caller skills are not edited and `just publish-skills` is not
run.** `darkmatter/docs/use-skills/**` and `claudine/docs/use-skills/**`
contain no description of process ownership, the editor handoff, or
cancellation mechanics (searched for `process tree`, `process group`,
`own_tree`, `kill-on-close`, `foreground`), and the caller-visible behavior
(edit succeeds; warning text) is the only new surface. The `edit` warning is
added to the executor docs in Rule 27 only. Evidence: grep in the planning
session returned no hits in those trees. If Task 3.5's search later finds one,
fix it in the canonical `docs/use-skills/` source and run that area's
`just publish-skills` — never edit `~/.claude/skills`.

### Wave 7 (parallel: Tasks 3.1–3.4 edit disjoint files)

- [x] **3.1 Canonical topic page** — Rule 26 (`sniff/docs/topics/process-trees.md`).
- [x] **3.2 Sniff docs and skills** — Rule 27 bullets for `sniff/**` and
  `.claude/skills/sniff/**`.
- [x] **3.3 OS skill and Darkmatter docs/skills** — Rule 27 bullets for
  `.claude/skills/os/**`, `.claude/skills/darkmatter/**`, and
  `darkmatter/docs/**`.
- [x] **3.4 Claudine docs and skill** — Rule 27 bullets for
  `claudine/docs/**` and `.claude/skills/claudine/SKILL.md`.

### Wave 8 (validation checkpoint)

- [x] **3.5 Phase 3 checkpoint**
  - Search current docs, skills, and source comments (exclude `_completed`,
    `features/`, `fixes/`, `research/`, `.gitnexus`, `target`) for the stale
    claims: `git grep -n -i -E "assignment window|escapes the Job|escape the Job|accepted by every caller|atomic with respect to forks|process_tree\.rs|crate::process_tree|ProcessTree::attach\` returns|returns \`None\`" -- ':!*/features/*' ':!*/fixes/*' ':!*/research/*'`
    returns nothing that still states the old behavior.
  - Every file in Rule 27 contains a link to `process-trees.md` (check with
    `git grep -l "process-trees.md"`).
  - The topic page renders: its Mermaid block has the five elements the spec
    lists (configure, spawn, attach, owned, failure cleanup, and the three
    endings).

## Phase 4: Verification and close-out

Goal: package-wide proof, delegation proof, OS evidence, and the log.

### Necessary Rules

**Rule 29 — Verification commands.** Use the package recipes, not ad hoc
`cargo test`: from `sniff/`, `darkmatter/`, `claudine/` run `just test` and
`just lint`; run `just test-l2` in `darkmatter/` and `claudine/` for the
existing terminal-foreground tests (editor and login-shell probes); no terminal
or browser may gain focus (use the existing harness; do not launch a GUI
editor). Build checks: `cargo check -p sniff --all-targets` (no features),
`cargo check -p darkmatter --all-targets` (consumes `sniff` with `remote`).
Evidence: spec "Verification".

**Rule 30 — Delegation proof, as searches.** After Phase 2, these must hold:
- `git grep -n "CreateJobObjectW" -- '*.rs'` outside
  `sniff/lib/src/process/tree/windows*`, `claudine/cli/**` (provider
  supervision), and test fixtures returns nothing in `darkmatter/lib` or
  `claudine/lib`.
- `git grep -n "CREATE_SUSPENDED\|ResumeThread\|AssignProcessToJobObject" --
  darkmatter claudine -- '*.rs'` returns nothing outside `claudine/cli/**`.
- `git grep -n "process_group(0)\|setsid" -- darkmatter/lib/src
  claudine/lib/src -- '*.rs'` returns only the test fixtures and
  `darkmatter/lib/src/editor/launch.rs`/`shell.rs` comments and tests, none in
  production setup code. (No repo-wide single-occurrence requirement; provider
  supervision and `configure_detached_child` legitimately keep their own.)

**Rule 31 — OS evidence and gaps are recorded, not waited on.** Load the `os`
skill and, for each of native Windows and WSL2 (macOS and Linux evidence comes
from the pushing/CI hosts under the evidence-reuse rules), run the package
`just test` recipes for `sniff`, `darkmatter`, and `claudine` through the
documented rig recipe (`just cross-check` / the CI archive path the skill
describes). Outcomes, defined up front: a green run on a rig → record the rig,
command, and date; an SSH host whose enclosing Job rejects assignment → record
it as failure-path evidence (assignment error surfaced and the child killed),
**not** containment proof; a rig that cannot be reached from this session →
record "not obtained: <reason>" for that OS. None of these outcomes blocks
completion. Evidence: spec "Obtain macOS, Linux, native Windows, and WSL2
evidence… Record missing evidence explicitly."

**Rule 32 — Completion state.** An agent's terminal state is "implementation
complete, ready for review": set the spec frontmatter `status: implemented`,
`implemented: true`, `implemented_by: claude/sonnet`; never move the fix to
`_completed` and never run `just complete`. Spec departures (if any) go in the
implementation log, not in the spec.

### Wave 9 (parallel)

- [x] **4.1 Package test and lint runs** — Rule 29 for `sniff`, `darkmatter`,
  `claudine`; fix failures in the owning task's files (do not weaken a test).
- [x] **4.2 Delegation and stale-claim searches** — Rule 30 plus the Phase 3
  search repeated over the final tree; fix any hit.
- [x] **4.3 Comment-drift pass** — for every function whose behavior changed
  (`run_command_with_timeout`, `run_shell_query`, `LaunchPlan::run`,
  `run_process`, `run_owned_lifecycle_command`, `dispatch_edit`), read its
  `///` and inline `//` comments against the new code; per `CLAUDE.md` assume
  the code is right, fix the comment, and list each drift fixed in the
  implementation log.
- [x] **4.4 Windows and WSL2 evidence** — Rule 31. Includes executing the
  Phase 1.5 Windows tests and the `windows`-feature compile of
  `darkmatter` and `claudine` after the feature trims (Rules 23, 25).

### Wave 10 (close-out, after Wave 9)

- [x] **4.5 Implementation log and status** — create
  `sniff/fixes/2026-11-14-shared-process-tree/implementation-log.md` with: the
  commands run and results per package, evidence per OS (Rule 31 outcomes),
  the drift fixes (4.3), and the departures from the spec (none planned; list
  any). Apply Rule 32 to the spec frontmatter. Do not commit unless the
  invoking prompt says so.