---
total_phases: 4
created: 2026-11-14
phase: 2
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1:
- sniff/lib/src/process.rs
- sniff/lib/src/process/foreground.rs
- sniff/lib/src/process/foreground/tests.rs
docs_updated_during_phase_1:
- sniff/docs/topics/process-trees.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
- .claude/skills/sniff/SKILL.md
packages:
- sniff
---

# Shared Terminal Foreground Handoff Implementation Plan

## Summary and Definition of Done

Move Unix terminal foreground transfer and restoration from Darkmatter into
Sniff's public process API, alongside the shared process-tree owner. Use that
implementation for owned terminal editors and every Claudine lifecycle shell
command. A lifecycle command must be able to read the terminal from its own
process group, and cancellation must terminate its remaining owned descendants
before Claudine resumes lifecycle work with its terminal restored.

Completion means:

- Sniff contains the sole foreground-transfer implementation. It transfers
  foreground access before the child executes and restores the original group
  and terminal settings on ordinary return, setup failure, cancellation, wait
  failure, and panic unwinding. Restoration failure is observable without
  replacing an earlier cancellation or execution failure.
- Claudine's exact-byte and structured lifecycle commands always own their
  trees. The foreground exception and its direct-child-only runner are gone.
  Typed Ctrl-C ending a foreground command is recognized as cancellation;
  background descendants do not survive command completion or cancellation.
- Darkmatter retains its editor selection, stream routing, release policy,
  cancellation grace, and unowned launch behavior. Its existing edit L2 tests,
  including Claudine's edit tests, pass without modification.
- Detached tmux tests prove typed input, typed Ctrl-C, host-directed
  cancellation, descendant cleanup, and restoration. No test takes a terminal
  or browser window's focus. Native Windows retains its existing console and
  Job behavior; Linux and WSL2 use the Unix implementation.
- Canonical topic pages, affected repo skills, and caller-skill sources describe
  the implemented behavior. Validation records distinguish runtime evidence,
  compile evidence, and unavailable environments.

This plan follows **2026-11-14-shared-process-tree**. At planning time its
specification exists, but the shared public owner and
`sniff/docs/topics/process-trees.md` have not been implemented. Execute that
dependency before this plan through the existing sequence scheduler. This plan
extends its result; it does not repeat its ownership migration. The spec's
empty Open Questions section requires no product ruling: the author has already
required one shared handoff and migration of both consumers.

Waves are numbered globally. Tasks in a wave with separate file ownership may
run concurrently in non-interactive subagent sessions. Each wave finishes
before the next begins. All source changes remain in the starting worktree.
This plan authorizes no commits, pushes, spec lifecycle moves, or GUI focus.

## Phase 1: Implement the Shared Handoff Contract

### Necessary Rules

1. **Add an adjacent RAII guard, not a second process supervisor.** Export
   `sniff::process::ForegroundHandoff` from `sniff/lib/src/process.rs`; put its
   implementation in `sniff/lib/src/process/foreground.rs`. Its public methods
   are `prepare(command: &mut std::process::Command) -> std::io::Result<Self>`,
   `is_active(&self) -> bool`, and
   `restore(&mut self) -> std::io::Result<()>`. The guard owns terminal state;
   the predecessor's `ProcessTree` continues to own containment, sampling,
   termination, and release. Evidence: the predecessor explicitly leaves
   `Child`, streams, and wait loops with callers; Darkmatter's
   `editor/launch.rs::terminal::Handoff` is already a separate guard. This
   extension does not rename or redesign the predecessor's API.

2. **Preparation follows shared group configuration and precedes spawn.** The
   order is configure a fresh group, prepare handoff, spawn, immediately attach
   the tree, then enter the caller's sampling/wait loop. Keep the handoff guard
   in the caller's scope before the tree variable, so armed tree cleanup runs
   before terminal restoration during unwinding. On spawn/attachment failures,
   perform existing kill/wait cleanup before returning through restoration.
   On ordinary completion restore explicitly after tree cleanup or release and
   direct-child reaping, before callbacks, diagnostics, or another lifecycle
   action. Evidence: `LaunchPlan::run` already restores after its wait; the
   predecessor requires immediate attachment and caller-owned reaping. An armed
   destructor terminates best-effort; it does not promise to reap arbitrary
   descendants or execute after abort, force-exit, or `process::exit`.

3. **Use the controlling terminal through an owned descriptor.** On Unix open
   `/dev/tty` read/write with close-on-exec and no controlling-terminal
   acquisition; retain that descriptor until restoration. Share its owned
   handle with the command hook through `Arc<File>` so the hook never captures
   a descriptor whose file has been closed. The prepared command is spawned
   once while its guard is alive; document that single-spawn lifetime contract.
   Capture its
   foreground PGID, the host PGID, and `termios` before spawn. No controlling
   terminal (`ENOENT`, `ENXIO`, or `ENOTTY`) or a foreground PGID different from
   the host's produces an inactive guard and no terminal mutation. Other
   open/query failures return their original I/O error before spawn. This
   covers inherited stdin redirected away from the controlling terminal.
   Evidence: Claudine's `holds_terminal_foreground` checks `/dev/tty`, whereas
   Darkmatter's private handoff uses fd 0; the common API must preserve the
   former's controlling-terminal coverage. Darkmatter continues to request
   handoff only for a terminal editor with `LaunchStdin::Inherit`.

4. **Reject overlapping handoffs within one host process.** A private
   process-local mutex is acquired with `try_lock` before inspecting foreground
   state and held for the active guard's lifetime. A competing request returns
   `WouldBlock`, never silently launches an interactive child in a background
   group. Recover a poisoned mutex after the previous guard's unwind cleanup.
   Release the lock immediately for an inactive guard. Nested *processes* each
   have their own lock and restore their immediate parent's PGID. Evidence:
   Claudine runs group legs on worker threads, and the terminal has only one
   foreground group. The editor's thread-local terminal refusal in
   `editor/service.rs` does not protect lifecycle shell commands.

5. **Transfer in the child before exec, using only async-signal-safe work.**
   Register one `pre_exec` hook after group configuration. Verify that the
   child's positive PGID equals its PID and differs from the captured host
   group; reject an unconfigured child with `InvalidInput`. Block `SIGTTOU`,
   call `tcsetpgrp` on the retained terminal descriptor, and restore the saved
   signal mask before exec. Propagate syscall failures from spawn; preserve
   the first error while still attempting mask restoration. The hook allocates
   nothing and takes no locks. Keep the descriptor alive in the parent across
   spawn; close-on-exec removes its child copy after the hook. Evidence:
   Darkmatter's existing `terminal::Handoff::prepare` uses this pre-exec
   ordering specifically to prevent an initial `SIGTTIN` read race.

6. **Restoration is idempotent, bounded, and thread-local in its signal policy.**
   On the parent thread temporarily block `SIGTTOU` with `pthread_sigmask`,
   restore the captured foreground group, then restore settings with
   `tcsetattr(TCSANOW)`, and restore the original thread mask. Attempt the
   settings operation even after a foreground restoration failure and return
   the first error. Disarm only after complete restoration. `Drop` retries an
   armed restoration once, reports failure through tracing, and never panics.
   Do not install a process-global `SIGTTOU` disposition. Evidence: the current
   Darkmatter destructor temporarily changes `sigaction`, which affects other
   threads, and uses `TCSADRAIN`, which can wait on output. Immediate settings
   restoration meets the spec without an unbounded output-drain dependency.

7. **Windows gets a no-op guard without a changed launch policy.** All three
   public methods compile on native Windows: preparation returns an inactive
   guard, restoration succeeds, and drop does nothing. Preserve the
   predecessor's creation flags and suspended Job attachment unchanged.
   WSL2 compiles the Unix branch. Evidence: spec requirement 4 and the
   existing `editor/launch.rs` non-Unix `terminal` module. Add no dependency,
   new console handler, session detachment, or handoff to Sniff probes,
   Darkmatter login-shell queries, or Claudine provider supervision.

8. **Both Claudine lifecycle routes use one owned runner.** Modify
   `composition/sequence/task/shell.rs::run_owned_lifecycle_command`, delete
   `holds_terminal_foreground` and `run_in_own_group`, and retain its re-export
   in `task/mod.rs`. Callers are
   `composition/lifecycle/executor.rs::run_system_shell` (both
   `SystemShellRunner` and `DocumentShellRunner`) and
   `composition/lifecycle/structured_shell.rs::run_filled_chain`. They keep
   their existing signatures, permissions, environment construction,
   redirections, and short-circuiting. Tree completion always removes
   survivors; lifecycle shells do not release surviving background jobs.
   Keep the 20 ms poll and 250 ms Unix cancellation grace and drive shared
   sampling. Evidence: these are the only callers found by the repository-wide
   `run_owned_lifecycle_command` search and the current runner's constants.

9. **Recognize child-delivered Ctrl-C without a new signal relay.** With an
   active Unix handoff, a direct child's `SIGINT` status or exit code 130 marks
   `crate::interrupt::mark_interrupted()` and returns 130 after tree cleanup
   and restoration. An already established worker/sibling cancellation cause
   takes precedence and does not set a new process-wide user-interrupt flag.
   Other signal statuses retain the existing `-1` result; an ordinary
   numeric 130 without an active handoff retains the existing shell result
   policy. A host-directed cancellation request returns 130 as today. Evidence:
   `ShellRunner` returns `Result<i32, ShellRunError>`, the shared executor treats
   nonzero results as action failures, and
   `StackExecutionContext::interrupted` reads Claudine's process interrupt flag.
   The flag is needed to prevent a later structured-chain segment from
   continuing a user-cancelled run. On Unix, also implement
   `LifecycleHost::stops_on_interrupt` for `StackExecutionContext` as
   `matches!(self.signal, LifecycleSignal::Initialize | LifecycleSignal::Start | LifecycleSignal::Loop)`.
   This stops the next normal action even after `no_error` suppresses the
   shell's dispatch error. Leave terminal cleanup events runnable and keep
   the native Windows trait behavior unchanged. Evidence:
   `StackExecutor::execute_nodes` checks this hook, its trait default is
   currently false, and `MdStackHost` already overrides it for normal work.
   `LifecycleRunGuard::run_guarded` identifies success/blocked/failure/
   cancelled/finalize as cleanup scopes.

10. **Foreground ownership determines keyboard delivery.** While the child
    owns the terminal, typed Ctrl-C reaches its group, not Claudine's host
    interrupt ladder. A command that catches or ignores SIGINT keeps control
    until it exits or receives host-directed cancellation. Do not inject a
    signal handler into user commands or infer a keypress from terminal input.
    Evidence: the spec requires a foreground handoff, and Darkmatter's
    `editor/launch.rs` module docs explicitly describe this kernel behavior.
    Existing host escalation remains supported for signals delivered to the
    host and for in-process lifecycle work where the host holds foreground.

11. **Preserve primary failures while surfacing restoration errors.** A
    previously successful Claudine command becomes an I/O failure on explicit
    restoration failure through its existing `ShellRunError::Spawn` mapping.
    A previously successful editor launch becomes `EditorError::Io` with
    operation `restore the terminal`. Preparation maps to the same existing
    I/O categories with operation `prepare the terminal` for editors. Existing
    cancellation, wait failure, and nonzero editor/command results stay
    primary; report secondary restoration errors through tracing and retain
    the destructor retry. Keep the predecessor's editor release-warning policy.
    Evidence: `LaunchPlan::run`, `EditorError::Io`, and `run_system_shell`
    already provide these error boundaries. No terminal helper prints output
    directly or introduces a new diagnostic schema.

12. **Keep the editor migration narrow.** In
    `darkmatter/lib/src/editor/launch.rs::LaunchPlan::run`, request the shared
    guard only for `own_tree && editor.uses_terminal() && stdin == Inherit`.
    Delete the private `terminal` modules and their foreground syscalls.
    Preserve `LaunchOptions::PLAIN`, stream routing, GUI releases including
    nonzero completion, the two-second cancellation grace, and the
    predecessor's polling/sampling behavior. Evidence: these are explicit
    branches and policies in `LaunchPlan::run`; spec requirement 3 requires
    existing edit terminal tests to pass unchanged.

13. **Replace the obsolete host-escalation fixture, not its assertions.**
    `cli/tests/common/terminal_interrupt.rs::PaneCompose::terminal_lifecycle`
    currently blocks in `trap '' INT` inside a lifecycle shell specifically
    to simulate uninterruptible *in-process* lifecycle work. Replace that
    simulation with a `test-fixtures`-only in-process hold in
    `claudine/lib/src/composition/lifecycle/effects.rs::ClaudineLifecycleEffects::info`.
    Activate it only for the exact info text `terminal-lifecycle-test-hold`
    and `CLAUDINE_TEST_LIFECYCLE_HOLD=<prefix>`; emit the info first, write
    `<prefix>.reached`, then poll for `<prefix>.release` with a 60-second bound.
    Keep all environment reads and hold code behind the existing feature.
    In the test helper use separate shell and host barrier variants; a host
    barrier only releases files and never interprets them as PIDs or signals
    the host during teardown. Keep the existing event-log assertions and
    keypress injection in `level2_lifecycle_ctrl_c_tmux.rs` and
    `level3_lifecycle_ctrl_c.rs` unchanged. Evidence: the common fixture's
    module comment explains the simulation, the L3 file shares it, and
    `claudine-cli/test-fixtures` already enables `claudine/test-fixtures`.
    This preserves keyboard evidence for the host ladder without pretending
    it receives keys while a child owns foreground. L3 execution is excluded
    from this unattended plan because those existing tests take focus.

14. **Use `start`, not `initialize`, for shell fixtures and examples.** The
    spec's illustrative `initialize: shell` snippet is not executable under
    current lifecycle rules. `darkmatter/lib/src/lifecycle/executor.rs::run_shell_action`
    rejects it before invoking a runner, and the public lifecycle docs require
    shells after preflight. Preserve that prohibition and record this example
    correction in the implementation log; do not edit the snapshot spec.

15. **No spike or input-format matrix is required.** Code inspection establishes
    the syscall ordering, callers, types, and test fixture conflict. There is
    no performance question in this spec. The change reads terminal/process
    state, not a file format or configuration; existing lifecycle parsing is
    unchanged. Test-only readiness files are synchronization, not a new
    configuration loader. Do not add parser matrices, benchmarks, or a
    cross-platform measurement project.

### Wave 1: Shared implementation

- [x] **Implement handoff**
  - Add the public guard and private Unix implementation using rules 1–7.
    Keep the platform code in `process/foreground.rs`, with a sibling
    `process/foreground/tests.rs` module for scripted parent-side syscall
    failures. Re-export the type from the public process module.
  - Add rustdoc covering the required fresh-group configuration, guard lifetime,
    immediate attachment, explicit restoration, and error cleanup. Explain
    inactive preparation, contention, panic behavior, and the ownership of
    reaping. Audit the process module's comments for drift caused by the new
    public capability.
  - Build the library with default features and `remote`; the guard must not
    introduce network dependencies or work into launches that do not request it.

- [x] **Prove guard lifecycle**
  - After the implementation task, add focused tests in the sibling module.
    Use a private scripted parent-side syscall adapter to exercise the actual
    guard, including partial preparation failure, background/no-terminal
    inactivity, contention, poison recovery, successful restore, failed group
    restoration followed by settings restoration, saved-mask restoration,
    retry on drop, repeated restore, and `catch_unwind` cleanup ordering.
  - Verify Windows inactivity through the public methods in a Windows-gated
    test. Keep real controlling-terminal and child pre-exec failure cases for
    Wave 3; a terminal-free unit test does not prove foreground transfer.
  - Keep tests independent of ambient terminal state and prove no settings or
    foreground mutation for inactive guards. Assert the signal disposition is
    never changed and thread masks return to their saved value.

**Checkpoint:** The shared guard compiles, its lifecycle/error tests pass, and
the documented public signatures are available before the consumer migrations.
Wave 1 tasks are sequential because the tests exercise the new guard.

## Phase 2: Migrate Both Consumers

### Wave 2: Independent consumer changes

- [ ] **Migrate editors**
  - Own only `darkmatter/lib/src/editor/launch.rs` and its L1 launcher tests in
    `darkmatter/cli/tests/l1/editor_launcher.rs` and `editor_support.rs`.
    Apply rules 2, 11, and 12; remove the private handoff implementation.
  - Cover preparation/restoration error classification, a completed nonzero
    editor, cancellation, an unowned launch, and a GUI launch. Preserve the
    predecessor's release-warning tests and update no existing edit L2 file.
  - Check the function's rustdoc and inline comments alongside the change.
    Run the focused editor L1 tests through the Darkmatter recipe.

- [ ] **Migrate lifecycle shells**
  - Own `claudine/lib/src/composition/sequence/task/shell.rs`, its tests, and
    the Unix `LifecycleHost::stops_on_interrupt` override/tests in
    `composition/lifecycle/executor.rs`.
    Apply rules 2 and 8–11. Every exit after spawn settles the owned tree;
    restore the terminal before returning to the lifecycle executor.
  - Preserve command construction in `lifecycle/executor.rs` and
    `lifecycle/structured_shell.rs`; their existing caller trace is complete.
    Add control cases through `SystemShellRunner` and `DocumentShellRunner`
    for both exact bytes and a filled native-command chain.
  - Test pre-existing cancellation (nothing starts), worker cancellation,
    cancellation while the child runs, normal/nonzero completion with a
    surviving background child, and wait-error cleanup. Test that a normalized
    foreground interrupt marks user cancellation and prevents a later chain
    segment or `no_error` action from proceeding. Keep ordinary noninteractive
    exit-code behavior unchanged.
  - Remove comments accepting the foreground ownership gap; preserve accurate
    Unix best-effort containment limits from the predecessor.

**Checkpoint:** Both consumers use the same Sniff guard, the Claudine exception
functions are absent, and each task's focused L1 tests pass independently.

## Phase 3: Prove Terminal Behavior and Update the Current Record

### Wave 3: Independent tests and documentation

- [ ] **Exercise public guard**
  - Own `sniff/cli/src/bin/foreground_handoff_fixture.rs`, its explicit binary
    entry in `sniff/cli/Cargo.toml` under the existing `test-fixtures` feature,
    and a new `cli/tests/level2/level2_foreground_handoff.rs` module registered
    in `level2/main.rs`. Add no dependency or new feature. The fixture calls
    Sniff's public guard and shared tree API directly; it contains no copied
    handoff syscall implementation. Native Windows compiles a terminal-free
    no-op control path in the same fixture.
  - Give the fixture fixed modes `success`, `nonzero`, `spawn-error`,
    `unconfigured`, and `unwind`. Run each inside an isolated detached tmux
    pane, recording initial and final foreground PGIDs through shell
    `ps -o pgid= -o tpgid= -p $$` observations and settings through `stty`.
    Successful/nonzero child scripts change settings before exiting. In
    `spawn-error`, append a final pre-exec hook returning
    `std::io::Error::from(std::io::ErrorKind::Other)` after the shared handoff
    hook, proving restoration after transfer but failed spawn. This constructs
    no allocated error payload and requires no libc dependency in the helper.
    In `unconfigured`, omit group configuration and assert spawn fails without
    giving the child foreground access.
  - In `unwind`, wait for a child readiness marker after it changes settings,
    deliberately panic inside the guard/tree scope, catch the unwind outside
    that scope, and reap the direct child retained outside the closure. Assert
    the tree was terminated and the terminal restored before fixture code
    continues. Use a fixture cleanup guard independent of the panic assertion.
  - Require tmux through the test toolkit and use `bin_exe!` for the helper.
    File markers carry machine-readable test observations, not product UI.
    This task owns no Claudine or Darkmatter files and runs concurrently with
    their terminal regressions and documentation.

- [ ] **Prove foreground shells**
  - Own the new Unix-only
    `claudine/cli/tests/level2/level2_lifecycle_foreground_handoff.rs` and its
    registration in `level2/main.rs`. Use `TmuxHarness`,
    `require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux)`,
    isolated HOME/PATH/CWD, and `bin_exe!`/the area's existing CLI command
    builder. Follow `level2_edit_action.rs` for a detached pane, readiness
    markers, saved `stty` settings, and a shell-input assertion after return.
  - Use a `start` shell script that reads and records a typed line, changes
    terminal settings, starts a background descendant that ignores SIGINT and
    SIGTERM, and announces readiness. Disable shell job control inside the
    script; its background child must remain in the owned group. Test typed
    `C-c` and separately `SIGINT` directed to Claudine's recorded PID. Both
    cases must end the root and background child, run cancellation/finalize
    exactly once, skip later ordinary actions despite `no_error`, restore
    settings/host foreground, and exit 130. Use an actual SIGINT-terminating
    root, rather than an ambiguous shell trap result, for the keyboard case.
  - Add terminal controls for success and nonzero completion, a nested
    Claudine lifecycle command handing the terminal onward and restoring it
    one level at a time, and a structured command that reads the typed line.
    Assert PGID ownership with `tcgetpgrp`, not just absence of a hang.
  - Add background-host and redirected-stdin cases: a background host must
    leave the actual foreground group untouched, while a foreground host with
    redirected stdin must permit a command reading `/dev/tty`. Assert tree
    ownership in both cases. Give every spawned root and descendant a fixture
    cleanup guard, bounded readiness, and teardown independent of assertions;
    never rely solely on nextest's timeout to clean up a failed test.
  - Update only the stale explanatory comments in
    `cli/tests/l1/level1_lifecycle_shell_terminal_pty.rs`: the original
    terminal-using regression must pass with the new implementation. Keep its
    behavioral assertion. Use portable `stty` comparisons that exclude the
    kernel's transient `pendin` bit, as the existing edit tests do.

- [ ] **Preserve host escalation**
  - Own `claudine/lib/src/composition/lifecycle/effects.rs` and
    `claudine/cli/tests/common/terminal_interrupt.rs`. Implement the bounded
    test-only host barrier from rule 13, with no new feature or dependency.
  - Retain shell barriers for orphan-teardown scenarios. Replace only the
    terminal-lifecycle blocking action and its associated barrier plumbing;
    make release and drop safe for both barrier kinds.
  - Run the existing `level2_lifecycle_ctrl_c_tmux` scenarios through
    `just test-l2`. Preserve their host-escalation assertions and existing L3
    callers; compile the latter without running focus-taking tests. Verify a
    default build contains no active environment-driven hold.

- [ ] **Document shared behavior**
  - Own documentation and skill changes. Extend
    `sniff/docs/topics/process-trees.md` with a complete minimal handoff
    example and a Mermaid flow showing prepare → spawn → attach → wait →
    cleanup/release → restore, with setup-error and unwind restoration paths.
    Explain controlling terminals, fresh groups, inactive guards, contention,
    typed versus host-directed SIGINT, error priority, and the normal limits
    of RAII after process abort/force-exit. Remove the foreground-shell
    exception while retaining the predecessor's Unix containment limits.
  - Update the process API entry in `sniff/docs/sniff-library-architecture.md`
    and its library README, keeping these as overview/link changes. Add the
    caller rules to Claudine's `docs/topics/flow-control/lifecycle.md`,
    `docs/topics/flow-control/sequences.md`, and `docs/topics/signal-handling.md`.
    State clearly that host repeat-interrupt escalation applies while the
    host receives signals; an active foreground command receives terminal
    keypresses itself. Use a valid `start` example.
  - Link Darkmatter's editor behavior in
    `docs/topics/lifecycle/index.md` and `docs/cli/edit.md` to the canonical
    Sniff topic. Keep editor-specific cancellation and release policies there.
  - Update `.claude/skills/sniff/SKILL.md`, its `architecture.md`,
    `.claude/skills/claudine/SKILL.md`,
    `.claude/skills/darkmatter/lifecycle.md`, and the process-ownership entries
    in `.claude/skills/os/SKILL.md` and `windows.md`. Remove Claudine's old
    `holds_terminal_foreground` advice. Do not reintroduce assignment-race
    claims that the predecessor removed.
  - Update `claudine/docs/use-skills/use-claudine/lifecycle.md` and
    `darkmatter/docs/use-skills/use-darkmatter/authoring.md` with short caller
    guidance drawn from canonical topic passages; publish with each area's
    `just publish-skills`. Do not edit installed skills. Do not touch dependency
    documentation: this plan adds no dependency. The only new manifest entry
    is the feature-gated Sniff test helper.

**Checkpoint:** The new terminal regressions pass, existing edit tests remain
unchanged, host escalation tests retain their meaning, and docs/skills explain
the same terminal ownership contract. No format-reader behavior has changed.

## Phase 4: Validate and Produce the Review Record

### Wave 4: Integrated verification

- [ ] **Run scoped gates**
  - Run full L1 once after all changes with `just test` from `sniff/`,
    `darkmatter/`, and `claudine/`. These recipes use nextest and the area's
    declared features. Use their positional filters for earlier focused runs;
    do not substitute `cargo test` or a workspace-wide gate.
  - Run the new `level2_lifecycle_foreground_handoff`, existing
    `level2_lifecycle_ctrl_c_tmux`, `level2_edit_action`, and
    `level2_edit_interactive_capture` filters with Claudine's `just test-l2`.
    Run Sniff's `just test-l2 level2_foreground_handoff` and Darkmatter's
    `just test-l2 level2_edit_action`. Set
    `BISCUIT_TEST_REQUIRED_BACKENDS=tmux` so a skipped backend cannot masquerade
    as proof. Do not run L3 or set any focus-enabling variable.
  - Run `just lint` in all three areas. These recipes' format **checks** are
    permitted; never run `cargo fmt` or a fixing formatter. Also run
    `cargo clippy -p sniff --all-targets -- -D warnings` and
    `cargo clippy -p sniff-detection-cli --all-targets -- -D warnings` to cover
    test-target warnings missed by Sniff's local lint recipe.
  - Run `just check-windows` in `sniff/` and `claudine/`, and
    `cargo check -p darkmatter -p darkmatter-cli --tests --target x86_64-pc-windows-gnu`
    with the same `RUSTC_WRAPPER=""`, isolated target directory, and
    `-Wa,-mbig-obj` C/C++ flags documented in Sniff's Windows-check recipe.
    This is compile evidence, not native Windows runtime evidence.
  - The planning environment declares `BUILD_LINUX=build-linux` and no
    `BUILD_WIN`, `BUILD_WSL`, or `BUILD_MACOS`. For implementation-time remote
    evidence use a tar snapshot of the current source and non-interactive SSH
    (`BatchMode=yes`, bounded connection timeout) to the declared Linux host;
    exclude `.git`, `.gitnexus`, and build outputs and make no commits. Build
    and run the same three area L1 recipes and focused tmux L2 filters in that
    disposable validation copy. No source edits occur there. Preserve logs,
    then remove the owned validation copy. Read the OS skill's build-host
    guidance first; do not modify standing cross-check clones or install tools.
  - Record native Windows and WSL2 runtime evidence as unavailable when their
    destinations are undeclared; do not label cross-compilation or Linux
    results as that evidence. The normal existing CI cells remain responsible
    for those environments. No CI scope, matrix, accepted gap, or execution
    constraint changes are part of this fix. The implementation remains ready
    for review with this evidence limitation stated explicitly, without a
    human gate between phases.

- [ ] **Close implementation record**
  - Create `implementation-log.md` beside this plan. Record the shared API,
    changed callers, keyboard-delivery implications, test-only fixture
    correction, `/dev/tty` descriptor and signal-mask decisions, and the valid
    `start` example replacing the spec's illustrative `initialize` placement.
  - Record each validation command, result, feature set, OS/backend, and
    missing capability. Record the existing edit test files' unchanged diff
    status. Describe no tests as passing when their harness skipped.
  - Search the live source/docs/skills for `holds_terminal_foreground`,
    `run_in_own_group`, the former foreground cancellation exception, and
    duplicate `tcsetpgrp`/terminal-restoration implementations in both
    consumers. Require no remaining obsolete claim or implementation; retain
    unrelated foreground *detection* such as `editor/surface.rs`.
  - Check every behavior-changing symbol's comments and the final diff, retain
    unrelated worktree edits, check off completed tasks, and update phase
    progress. Leave the spec in its active directory. Final state:
    **implementation complete, ready for review**, with precise evidence
    limitations rather than an intervening approval request.

**Checkpoint:** Scoped gates and available terminal/runtime checks pass;
remaining OS evidence limitations are explicit; the final diff contains one
shared handoff and no Claudine foreground ownership exception.