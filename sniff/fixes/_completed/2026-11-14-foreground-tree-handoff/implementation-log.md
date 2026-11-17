---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-sequences/sniff/fixes/2026-11-14-foreground-tree-handoff/spec.md"
plan: "sniff/fixes/2026-11-14-foreground-tree-handoff/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
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
  - darkmatter
  - darkmatter-cli
  - claudine
  - claudine-cli
  - sniff-detection-cli
source_files_during_phase_2:
  - darkmatter/lib/src/editor/launch.rs
  - darkmatter/lib/src/editor/launch/tests.rs
  - darkmatter/cli/tests/l1/editor_launcher.rs
  - claudine/lib/Cargo.toml
  - claudine/lib/src/composition/sequence/task/shell.rs
  - claudine/lib/src/composition/sequence/task/shell/lifecycle_tests.rs
  - claudine/lib/src/composition/lifecycle/executor.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/interrupt_stops.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/set_shell_values.rs
  - claudine/cli/tests/l1/level1_lifecycle_shell_terminal_pty.rs
docs_updated_during_phase_2:
  - sniff/docs/topics/process-trees.md
  - claudine/docs/topics/flow-control/lifecycle.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/sniff/SKILL.md
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/darkmatter/library-surfaces.md
  - .claude/skills/os/windows.md
decisions_for_review:
  - |-
    The "one handoff at a time" rule (plan rule 4) is enforced with a
    process-wide atomic flag instead of a mutex whose guard the handoff holds.
    The plan asked for a mutex with try_lock and poison recovery. Holding a
    standard mutex guard inside the handoff would make the public
    ForegroundHandoff type impossible to move between threads, a surprising
    restriction for callers. With a flag, a second request still gets
    WouldBlock, an inactive guard releases it immediately, and the handoff's
    Drop (which also runs while a panic unwinds) releases it, so there is no
    poisoned state to recover; a test proves a new handoff succeeds after a
    panic. Choosing the mutex instead means swapping the flag for a mutex
    guard in foreground.rs and dropping the Send test; nothing else depends
    on it.
  - |-
    A "no terminal" answer (ENOENT, ENXIO, ENOTTY) from asking the terminal
    for its foreground group, not only from opening /dev/tty, also produces
    an inactive guard. The plan named only the open. The alternative is to
    return that error and refuse the launch; treating it like "no terminal"
    keeps a launch working when the terminal disappears between the open and
    the query. Reversing it is a one-line change in Handoff::capture plus its
    test case.
  - |-
    The guard is considered restored once both the foreground group and the
    settings are put back, even if restoring the calling thread's signal mask
    afterwards reports an error (that error is still returned). The plan said
    to disarm "only after complete restoration" without saying whether the
    mask counts. Counting it would leave an already-restored terminal armed
    and have Drop change it again. Changing this is one condition in
    Handoff::restore.
  - |-
    A short "Handing the terminal to an interactive child" section was added
    to sniff/docs/topics/process-trees.md and a pointer to the sniff skill in
    this phase, although the plan schedules documentation for Phase 3. The
    repository treats docs as the current record of public behavior, and the
    public type exists from this phase. Phase 3 still owns the full example,
    the Mermaid flow, and the caller-table changes; it may rewrite this
    section freely.
  - |-
    Two lifecycle commands that want the terminal at the same moment in one
    Claudine process (for example `start` shells in parallel group legs while
    Claudine holds the terminal) no longer both run: the second fails before it
    starts with a "another terminal foreground handoff is active" I/O error,
    reported as that action's failure. This follows plan rule 4 (refuse, never
    launch an interactive child in the background), but it is a visible change
    for parallel, non-interactive lifecycle shells run from a terminal; before
    this fix they all ran in Claudine's own group. Alternatives: wait for the
    terminal (serializes those commands and changes group timing), or run the
    second command owned but without the terminal (it is stopped by the kernel
    if it reads the keyboard). Either is a change inside
    run_owned_lifecycle_command in claudine/lib/src/composition/sequence/task/shell.rs.
  - |-
    The lifecycle runner's tests run without a terminal, so two test-only
    switches were added to shell.rs (compiled only under cfg(test)): one makes
    a command count as holding the terminal, the other makes the wait fail once
    the command has started its background process. They let the library tests
    drive the real runner through the keyboard-interrupt and wait-failure paths;
    the real terminal path is proven separately by a PTY test. The alternative
    is PTY-only tests, which cannot reach the wait-failure path at all.
    Removing the switches means deleting the test_seams module and the tests
    that set them.
  - |-
    An existing L1 test, set_shell_values::missing_and_interrupted_commands_fail_the_set,
    marked an interrupt before `start` and expected the `start` set action to
    run and fail. With plan rule 9's new Unix stop-on-interrupt for
    initialize/start/loop, that action now never runs. I kept the test's point
    (an interrupted `$(...)` is never a recoverable value) by running the same
    set under `finalize`, a cleanup event that still runs after an interrupt,
    and added a Unix-only assertion that `start` stops before the set. The
    alternative was to leave start running its first action, which contradicts
    the plan's rule.
  - |-
    Darkmatter's new PTY editor test opens its terminal with libc
    (posix_openpt, setsid, TIOCSCTTY) instead of the xpty crate, which
    darkmatter-cli only depends on for Windows. Using xpty would widen a
    dependency to every OS and need a dependencies.md change; the libc helper
    is about 40 lines in editor_launcher.rs. Switching means making xpty a
    dev-dependency for all targets and replacing spawn_on_terminal.
  - |-
    A pre-existing problem outside this fix was found and left unfixed: a
    `claudine compose` started as a background shell job is stopped by the
    kernel (SIGTTOU) whenever its output goes to the terminal, even for a
    document with no lifecycle shell. The likely cause is biscuit-terminal's
    capability probe (`RawModeGuard::new` in
    biscuit-terminal/lib/src/discovery/raw_mode.rs), which changes terminal
    settings without checking that the process is in the foreground. The new
    background-host L2 test therefore sends Claudine's stdout/stderr to a file
    (stdin stays on the terminal), so it still proves the handoff stays
    inactive and the tree is cleaned up. Fixing it here would widen this fix to
    biscuit-terminal and its many dependents. If the reviewer wants it fixed now:
    skip the probe when tcgetpgrp(fd) != getpgrp() in RawModeGuard::new, test
    biscuit-terminal and its dependents, and drop the redirection in
    level2_lifecycle_foreground_handoff_a_background_claudine_leaves_the_foreground_alone.
  - |-
    The test-only in-process hold that replaced the `trap '' INT` lifecycle
    shell (plan rule 13) has two behaviors the plan did not name: it gives up
    early when the directory holding its marker files has been deleted (so a
    test that cleaned up its workspace cannot leave Claudine parked for 60 s),
    and it does not hold at all if it cannot write `<prefix>.reached` (rather
    than blocking silently or panicking). The hold sits on the `failure` event,
    where Claudine holds the terminal after the provider exits, which keeps the
    existing event-log expectations unchanged. Reverting either behavior is a
    small change in the test_hold module of
    claudine/lib/src/composition/lifecycle/effects.rs.
  - |-
    claudine's `just test-l3` recipe now builds with
    `--features terminal-tests,test-fixtures` (it had `terminal-tests` only).
    The L3 terminal-lifecycle tests share the fixture that now relies on the
    `test-fixtures`-only hold, so without the feature they would wait for a
    readiness file that never appears. CI's feature set for claudine-cli
    already includes `test-fixtures`, so this aligns the local recipe with CI.
    The alternative is gating the hold on a library feature tied to
    `terminal-tests`, which the library does not have. L3 was compiled
    (`cargo test -p claudine-cli --features terminal-tests,test-fixtures --test
    level3 --no-run`) but not run, because L3 takes window focus.
  - |-
    The plan said not to touch dependency documentation, but
    darkmatter/docs/dependencies.md still said `libc` supplies the editor's
    terminal-foreground handoff in editor/launch.rs, which Phase 2 removed. I
    corrected that one sentence (libc now supplies only the terminal checks in
    surface.rs and probe.rs; the handoff comes from Sniff). No dependency was
    added or removed. Reverting restores a false statement.
  - |-
    The caller skills (use-claudine lifecycle.md, use-darkmatter authoring.md)
    got short written summaries instead of transcluding the canonical passages
    as the plan preferred. `::file` transcludes whole files, and the canonical
    passages sit in pages of 500 to 2,200 lines, so transclusion would have
    pulled far too much into the skills. The risk is drift between the summary
    and docs/. The alternative is splitting the canonical passage into its own
    small file and transcluding it from both the topic page and the skill.
  - |-
    Some documentation statements were reasoned from code or platform docs and
    have no test here: (1) .claude/skills/os/windows.md says a lifecycle
    command started with CREATE_NEW_PROCESS_GROUP ignores CTRL_C_EVENT by
    default, so Claudine's console handler takes the press (documented Windows
    behavior, untested on native Windows here); (2) process-trees.md says a
    handoff cannot be combined with `TreeConfig::session()`, because a new
    session has no controlling terminal; (3) Claudine docs name `start` and
    `loop` as the actions that stop after a foreground interrupt, although the
    code also stops `initialize`, because shell actions cannot run in
    `initialize`. Any of these can be softened or removed if the reviewer
    prefers documenting only tested behavior.
  - |-
    Phase 4 fixed seven Windows-only compiler warnings in darkmatter-cli's
    L1 tests, test code only. Four of the files (compose_edit_action.rs,
    editor_service.rs, editor_support.rs, and the imports of
    editor_launcher.rs) predate this fix and belong to the editor-backed edit
    work. The plan did not ask for this. The fixes gate Unix-only imports
    behind #[cfg(unix)], delete a Windows kill_pid that nothing on Windows
    calls, and allow unused_mut on non-Unix targets for three `let mut`
    bindings that only Unix-gated code pushes to. The alternative was to
    record the warnings and leave them, which keeps this fix's diff to its own
    scope but leaves a Windows -D warnings build of those tests red. To revert,
    restore those four files from the Phase 3 commit.
  - |-
    The WSL2 rig (build-wsl) was declared on this host, but no WSL2 run was
    attempted; the plan expected it to be undeclared. It shares a machine with
    build-win, whose storage preflight refused both native Windows runs (32.7
    GiB free of 50 GiB required). The os skill warns that the two hosts compete
    for memory and disk. So this fix has no native Windows or WSL2 runtime
    evidence and relies on the nightly CI cells for both. The alternative is
    `just cross-check sniff --os wsl` (and the other packages) once that
    machine has headroom, or overriding the Windows preflight, which the
    recipe reserves for emergencies. Nothing needs redoing if the reviewer
    wants that evidence; it is an extra run.
source_files_during_phase_3:
  - sniff/cli/src/bin/foreground_handoff_fixture.rs
  - sniff/cli/Cargo.toml
  - sniff/cli/tests/level2/level2_foreground_handoff.rs
  - sniff/cli/tests/level2/main.rs
  - claudine/cli/tests/level2/level2_lifecycle_foreground_handoff.rs
  - claudine/cli/tests/level2/main.rs
  - claudine/cli/tests/common/terminal_interrupt.rs
  - claudine/lib/src/composition/lifecycle/effects.rs
  - claudine/lib/tests/l1/context_construction_guard.rs
  - claudine/justfile
docs_updated_during_phase_3:
  - sniff/docs/topics/process-trees.md
  - sniff/docs/sniff-library-architecture.md
  - sniff/README.md
  - sniff/lib/README.md
  - claudine/docs/topics/flow-control/lifecycle.md
  - claudine/docs/topics/flow-control/sequences.md
  - claudine/docs/topics/signal-handling.md
  - claudine/docs/topics/testing.md
  - claudine/docs/use-skills/use-claudine/lifecycle.md
  - darkmatter/docs/topics/lifecycle/index.md
  - darkmatter/docs/topics/lifecycle/error-handling.md
  - darkmatter/docs/cli/edit.md
  - darkmatter/docs/dependencies.md
  - darkmatter/docs/use-skills/use-darkmatter/authoring.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/sniff/SKILL.md
  - .claude/skills/sniff/architecture.md
  - .claude/skills/sniff/testing.md
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/darkmatter/lifecycle.md
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/windows.md
source_files_during_phase_4:
  - darkmatter/cli/tests/l1/editor_launcher.rs
  - darkmatter/cli/tests/l1/editor_support.rs
  - darkmatter/cli/tests/l1/editor_service.rs
  - darkmatter/cli/tests/l1/compose_edit_action.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/os/linux.md
source_code:
  - sniff/lib/src/process.rs
  - sniff/lib/src/process/foreground.rs
  - sniff/lib/src/process/foreground/tests.rs
  - darkmatter/lib/src/editor/launch.rs
  - darkmatter/lib/src/editor/launch/tests.rs
  - darkmatter/cli/tests/l1/editor_launcher.rs
  - claudine/lib/Cargo.toml
  - claudine/lib/src/composition/sequence/task/shell.rs
  - claudine/lib/src/composition/sequence/task/shell/lifecycle_tests.rs
  - claudine/lib/src/composition/lifecycle/executor.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/interrupt_stops.rs
  - claudine/lib/src/composition/lifecycle/executor/tests/set_shell_values.rs
  - claudine/cli/tests/l1/level1_lifecycle_shell_terminal_pty.rs
  - sniff/cli/src/bin/foreground_handoff_fixture.rs
  - sniff/cli/Cargo.toml
  - sniff/cli/tests/level2/level2_foreground_handoff.rs
  - sniff/cli/tests/level2/main.rs
  - claudine/cli/tests/level2/level2_lifecycle_foreground_handoff.rs
  - claudine/cli/tests/level2/main.rs
  - claudine/cli/tests/common/terminal_interrupt.rs
  - claudine/lib/src/composition/lifecycle/effects.rs
  - claudine/lib/tests/l1/context_construction_guard.rs
  - claudine/justfile
  - darkmatter/cli/tests/l1/editor_support.rs
  - darkmatter/cli/tests/l1/editor_service.rs
  - darkmatter/cli/tests/l1/compose_edit_action.rs
documentation:
  - sniff/docs/topics/process-trees.md
  - .claude/skills/sniff/SKILL.md
  - claudine/docs/topics/flow-control/lifecycle.md
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/darkmatter/library-surfaces.md
  - .claude/skills/os/windows.md
  - sniff/docs/sniff-library-architecture.md
  - sniff/README.md
  - sniff/lib/README.md
  - claudine/docs/topics/flow-control/sequences.md
  - claudine/docs/topics/signal-handling.md
  - claudine/docs/topics/testing.md
  - claudine/docs/use-skills/use-claudine/lifecycle.md
  - darkmatter/docs/topics/lifecycle/index.md
  - darkmatter/docs/topics/lifecycle/error-handling.md
  - darkmatter/docs/cli/edit.md
  - darkmatter/docs/dependencies.md
  - darkmatter/docs/use-skills/use-darkmatter/authoring.md
  - .claude/skills/sniff/architecture.md
  - .claude/skills/sniff/testing.md
  - .claude/skills/darkmatter/lifecycle.md
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/linux.md
completed_phase: 4
implemented: true
---

# Implementation Log for 2026-11-14-foreground-tree-handoff (4 phases)

## Phase 1

- Started 2026-11-16. Read the plan's Phase 1 rules 1–7 and Darkmatter's
  private `editor/launch.rs::terminal::Handoff`, which the new guard replaces
  in Phase 2.
- Added `sniff/lib/src/process/foreground.rs` with the public
  `sniff::process::ForegroundHandoff` (`prepare`, `is_active`, `restore`),
  re-exported from `process.rs`, whose module docs list the new capability.
  - Unix: opens `/dev/tty` read/write with `O_NOCTTY` (std adds close-on-exec)
    into an `Arc<File>` shared with the `pre_exec` hook; `ENOENT`/`ENXIO`/
    `ENOTTY` from the open or from `tcgetpgrp`, or a foreground group other
    than the host's, yields an inactive guard with no mutation; other errors
    return before spawn. The hook checks `getpgrp() == getpid()` and differs
    from the host group (`EINVAL` → `InvalidInput` otherwise), blocks
    `SIGTTOU` with `sigprocmask`, calls `tcsetpgrp`, restores the mask, and
    returns the first error.
  - Restoration blocks `SIGTTOU` with `pthread_sigmask` on the calling thread
    only, restores the group, then `tcsetattr(TCSANOW)` even after a group
    failure, restores the mask, returns the first error, and disarms only
    when both terminal operations succeeded. `Drop` retries once, logs with
    `tracing::warn!`, then releases the process-wide claim.
  - Native Windows: inactive guard, `restore` is `Ok(())`, no launch change.
  - The parent-side syscalls go through a private `TerminalOps` trait; the
    generic inner `Handoff<T>` takes the claim flag as a parameter so tests use
    a private flag per test.
- Tests (`sniff/lib/src/process/foreground/tests.rs`, L1, F1 in-process
  through a scripted `TerminalOps` driving the real guard code):
  - successful arm + restore of group and settings, thread mask returns to its
    saved value (`a_foreground_host_arms_a_transfer_and_restores_group_and_settings`);
  - idempotent restore and drop (`repeated_restore_and_drop_change_nothing_more`);
  - background host inactive, no mutation, claim released immediately
    (`a_background_host_is_inactive_and_mutates_nothing`);
  - each no-terminal errno from open, and `ENOTTY` from `tcgetpgrp`
    (`no_controlling_terminal_is_inactive_for_each_no_terminal_errno`);
  - partial preparation failures (open `EACCES`, `tcgetpgrp` `EIO`,
    `tcgetattr` `EIO`) return the original error, register nothing, release
    the claim (`other_preparation_failures_return_the_original_error_and_release_the_claim`);
  - contention → `WouldBlock` without inspection, cleared by restore
    (`a_second_active_handoff_would_block_until_the_first_is_restored`);
  - failed group restore still restores settings, returns the first error,
    stays armed, drop retries (`a_failed_group_restore_still_restores_settings_and_returns_the_first_error`);
  - failed settings restore (`a_failed_settings_restore_is_reported_after_the_group_is_restored`);
  - drop retries exactly once and never panics (`drop_retries_once_and_never_panics_when_restoration_keeps_failing`);
  - mask-block failure leaves the guard armed (`a_failed_mask_block_leaves_the_guard_armed_for_drop`);
  - `catch_unwind`: tree stand-in cleaned before restoration, claim released
    after the panic (`unwinding_cleans_up_the_tree_before_restoring_the_terminal_and_releases_the_claim`);
  - public guard is `Send` (`the_public_guard_is_send`);
  - Windows-gated public-method inactivity
    (`native_windows_handoffs_are_inactive_and_restore_trivially`).
  - The scripted adapter asserts `SIGTTOU` is blocked during both
    restoration operations; no test touches `sigaction`, and the guard has
    no code path that does.
  - Real-terminal transfer and the child-side `InvalidInput`/`tcsetpgrp`
    failures are Wave 3 (detached tmux), as the plan assigns.
- Docs: added "Handing the terminal to an interactive child" to
  `sniff/docs/topics/process-trees.md` (the current record now names the
  public API); the caller table and the Claudine exception row stay until
  Phase 2 migrates the consumers, and Phase 3 adds the full example and
  Mermaid flow. Added a pointer in `.claude/skills/sniff/SKILL.md`.
- Spec example correction (plan rule 14) is not exercised in this phase.
- Gates (macOS host):
  - `cd sniff && just test`: 3646 passed, 34 skipped, `🐢 L1 run recorded:
    3646 test(s), none at or above 5.0s` (run twice; the second on the final
    tree). `just slow-tests fresh` exited 0.
  - `cd sniff && just lint` clean; `cargo clippy -p sniff --all-targets -- -D
    warnings` and the same for `sniff-detection-cli` clean.
  - Dependents: the change only adds a public symbol that no dependent uses
    yet, so `cargo check -p claudine -p darkmatter` (pass) is the downstream
    check; their consumer migration and tests are Phase 2.
  - Cross-OS: `just cross-check sniff --os linux` passed (2644 tests, the 12
    new Unix tests included, none slow). `just cross-check sniff --os windows`
    built the Windows test archive (so the module and the Windows-gated test
    compiled) but the run was refused by the rig's storage preflight (cargo
    volume 34.8 GiB free, 50 GiB required, automatic sweep freed nothing).
    That is a rig disk condition, not a test failure; I did not override the
    preflight. After a later Windows-only edit (a private field so the
    Windows struct cannot be built outside the module), a local
    `cargo check -p sniff --lib --tests --target x86_64-pc-windows-gnu` passed.
    `os_failing` is not set: no Windows test failed.
- Not using `affected_scope.py` to skip gates: the gates were run. (The
  working tree also carries the uncommitted predecessor
  `2026-11-14-shared-process-tree` changes, so the uncommitted tree is not
  this phase alone.)
- Phase 1 tasks complete: both Wave 1 checkboxes are checked.

## Phase 2

- Started 2026-11-16 from a clean tree (Phase 1 committed as 5bf7e9bc1).
- **Editors** (`darkmatter/lib/src/editor/launch.rs`): an owned terminal
  editor with inherited stdin now prepares `sniff::process::ForegroundHandoff`
  right after `TreeConfig::process_group().configure`; the guard is declared
  before the tree. A prepare failure is `EditorError::Io { operation: "prepare
  the terminal" }` (contention is `WouldBlock`). Spawn and attach failures
  keep their existing cleanup, then restore. Every path restores explicitly
  after the root is reaped through `with_restoration`: a restore failure turns
  an otherwise successful edit into `EditorError::Io { operation: "restore the
  terminal" }`; after any other failure (cancel, wait error, nonzero exit,
  launch/attach failure) it is logged with `tracing::warn!` and the first
  failure stands; `Drop` still retries. The private Unix/non-Unix `terminal`
  modules (fd 0, `sigaction`, `TCSADRAIN`) are deleted. Unowned (`PLAIN`) and
  GUI launches request no handoff; stream routing, release warning, 2 s grace
  and polling are unchanged. Module and `run` rustdoc updated.
- **Lifecycle shells** (`claudine/lib/src/composition/sequence/task/shell.rs`):
  `holds_terminal_foreground` and `run_in_own_group` are deleted; the
  `task/mod.rs` re-export is unchanged. `run_owned_lifecycle_command` always
  configures the tree, prepares the handoff (held in its own frame so an
  unwind drops the tree first), runs `run_lifecycle_tree` (spawn, attach,
  20 ms poll with sampling, `terminate(WhenObserved)` on completion,
  `terminate_gracefully` 250 ms on cancellation or wait error), restores, then
  `settle_lifecycle_command`. With an active handoff a `SIGINT` end or exit
  130 is a foreground interrupt: after cleanup and restore it marks
  `crate::interrupt` unless a cancellation cause is already established, and
  returns 130. Without the foreground `SIGINT` stays `-1` and 130 stays an
  ordinary code. A restore failure fails only a command that exited 0 (the
  caller maps it to `ShellRunError::Spawn`). `StackExecutionContext`
  overrides `stops_on_interrupt` on Unix for initialize/start/loop. Callers
  in `executor.rs` and `structured_shell.rs` are unchanged. The libc comment
  in `claudine/lib/Cargo.toml` was updated (libc now supplies only `SIGINT`
  there).
- Plan rule 14 (use `start`, not `initialize`, for shell examples) applied:
  the new PTY test and the corrected lifecycle doc paragraph use `start`;
  the spec's illustrative `initialize: shell` snippet is left as written.
- **Requirement → test mapping** (all L1):
  - Editor restoration priority (F1, `darkmatter/lib/src/editor/launch/tests.rs`):
    `a_restoration_failure_fails_an_otherwise_successful_edit`,
    `an_earlier_failure_stays_primary_over_a_restoration_failure` (cancel,
    nonzero, wait error), `a_successful_restoration_leaves_the_result_unchanged`.
  - Editor real terminal (F2, a PTY session in
    `darkmatter/cli/tests/l1/editor_launcher.rs`):
    `an_owned_terminal_editor_takes_the_foreground_and_gives_it_back` runs
    `terminal_helper` as session leader: contention → `prepare the terminal`
    `WouldBlock`; the editor reads a line typed before it started (a
    background group would be stopped), then `tcgetpgrp` shows the host back
    in the foreground after success, nonzero exit (5) and cancellation.
    Existing tests still cover nonzero/missing program, cancellation of the
    whole tree, the SIGTERM grace, GUI cancel and the unowned launch; none
    changed, and no L2 edit test file was touched.
  - Lifecycle runner (`claudine/.../task/shell/lifecycle_tests.rs`):
    `a_restoration_failure_fails_only_a_command_that_exited_zero`,
    `a_foreground_interrupt_marks_the_run_interrupted`,
    `an_established_worker_cancellation_keeps_the_interrupt_unmarked`,
    `only_a_foreground_command_turns_sigint_or_130_into_an_interrupt`,
    `a_command_is_not_started_once_cancellation_is_requested`,
    `exact_bytes_completion_stops_what_the_command_left_running`
    (SystemShellRunner and DocumentShellRunner, exit 0 and 3, background
    `sleep` dead), `a_structured_chain_segment_owns_its_tree` (both runners,
    two native segments), `worker_cancellation_while_the_command_runs_stops_its_tree`,
    `a_failed_wait_stops_the_whole_tree`,
    `without_the_foreground_sigint_and_130_are_ordinary_results`,
    `a_foreground_keyboard_interrupt_cancels_the_run` (both runners, `kill -INT
    $$` and `exit 130`, and a structured chain whose later segment never runs).
  - Executor (`.../executor/tests/interrupt_stops.rs`, Unix):
    `an_interrupted_start_command_stops_the_next_action_despite_no_error`,
    `cleanup_events_run_whole_after_an_interrupt` (cancelled, finalize),
    `only_normal_work_stops_on_an_interrupt` (every signal).
  - Real terminal, end to end (F3 through the `claudine` binary on a PTY,
    `claudine/cli/tests/l1/level1_lifecycle_shell_terminal_pty.rs`):
    `level1_pty_lifecycle_shell_interrupted_in_the_foreground_cancels_the_run`
    (start command leaves a background `sleep` and ends by SIGINT with
    `no_error`: background dead, next start action skipped, cancelled (using
    `stty`) and finalize ran, exit 130). Mutation check: forcing the
    foreground flag off makes it fail ("a start action ran after the
    interrupt"). The original `..._using_the_terminal_completes` regression
    passes unchanged in behavior; only its module comment was rewritten.
  - Typed `C-c`, host-directed SIGINT, nested Claudine and redirected-stdin
    cases are Phase 3's tmux tests, as planned.
- Changed existing test: `set_shell_values::missing_and_interrupted_commands_fail_the_set`
  (see decisions_for_review).
- No file-format reader changed, so the input-robustness matrix does not
  apply.
- Docs fixed in this phase because they described removed behavior:
  `sniff/docs/topics/process-trees.md` caller table (exception row and its
  paragraph replaced), the `shell` paragraph of
  `claudine/docs/topics/flow-control/lifecycle.md`,
  `.claude/skills/claudine/SKILL.md`, `.claude/skills/darkmatter/library-surfaces.md`,
  `.claude/skills/os/windows.md` (named the deleted `terminal::Handoff`).
  `.claude/skills/sniff/SKILL.md` now names the guard's two callers.
  Search for `holds_terminal_foreground`, `run_in_own_group`,
  `terminal::Handoff` in live source/docs/skills: no remaining hits.
- Gates (macOS, quiet host):
  - `just lint` in `darkmatter/`, `claudine/`, `sniff/`: clean (the
    `__eh_frame` linker note is pre-existing).
  - `cd sniff && just test`: 3646 passed, 34 skipped, none ≥ 5 s.
  - `cd darkmatter && just test`: 17049 passed, 10 skipped, none ≥ 5 s.
  - `cd claudine && just test`: 12261 passed, 15 skipped, none ≥ 5 s.
  - Focused runs before the full gates: `just test editor_launcher`,
    `just test editor::launch` (darkmatter); `just test lifecycle_tests
    interrupt_stops executor::tests task::shell lifecycle_shell` (claudine,
    325 passed after the set_shell_values change).
  - Dependents of darkmatter/claudine (biscuit-icon-cli, biscuit-speaks-cli,
    biscuit-terminal-cli, messenger, messenger-cli, playa-cli,
    sniff-detection-cli, research): none uses the changed symbols;
    `cargo check --tests` on all eight passes.
  - Windows compile: `cargo check -p darkmatter -p darkmatter-cli -p claudine
    -p claudine-cli --tests --target x86_64-pc-windows-gnu` (test-fixtures
    features, sniff's check-windows flags) passes; a new unused-import warning
    in `lifecycle_tests.rs` on Windows was fixed by moving the imports into
    its Unix module. The remaining warnings there (`editor_launcher.rs`
    `Duration`/`Instant`/`kill_pid`/`EDITOR_TERMINATE_GRACE`,
    `editor_service.rs` `mut`) predate this phase.
  - Linux runtime (build-linux via `just cross-check`): `darkmatter-cli
    editor_launcher` 10/10, `claudine lifecycle_tests interrupt_stops
    set_shell_values` 37/37, `claudine-cli level1_lifecycle_shell_terminal_pty`
    2/2; all pass.
  - Windows runtime: `just cross-check claudine --os windows lifecycle_tests
    set_shell_values` was refused by the build-win storage preflight (34.7 GiB
    free, 50 GiB required), the rig condition Phase 1 also hit; not a test
    failure, `os_failing` not set. Native Windows behavior is unchanged by
    design (the guard is inactive and `stops_on_interrupt` is Unix-only).
- Gates were run, so `affected_scope.py` was not used to skip them.
- Phase 2 tasks complete: both Wave 2 checkboxes are checked.

## Phase 3

- Started 2026-11-16 with Phase 2 uncommitted in the tree. The four Wave 3
  tasks have separate file ownership, so they ran as four parallel
  foreground subagents. Each verified only its own slice; the area gates
  below ran once afterwards, with no subagent running.
- **Exercise public guard** (sniff-detection-cli, F3 real controlling
  terminal in a detached tmux pane):
  - New `foreground_handoff_fixture` bin. It is declared in
    `sniff/cli/Cargo.toml` with `required-features = ["test-fixtures"]` and
    adds no dependency or feature.
  - The fixture uses only the public `TreeConfig::process_group().configure`
    → `ForegroundHandoff::prepare` → spawn → `ProcessTree::attach` → wait →
    cleanup → `restore()`.
  - Native Windows compiles a terminal-free control path.
  - Tests in `sniff/cli/tests/level2/level2_foreground_handoff.rs`:
    - `level2_foreground_handoff_child_reads_the_terminal_then_the_host_gets_it_back`
      (`success`: the child reads a typed line with echo off)
    - `level2_foreground_handoff_nonzero_exit_still_restores_the_terminal`
      (exit 7 after `-icanon -echo`)
    - `level2_foreground_handoff_failed_spawn_after_the_transfer_restores_the_terminal`
      (`spawn-error`: an extra pre_exec hook fails after the transfer, and
      the foreground had moved before `restore()`)
    - `level2_foreground_handoff_refuses_a_child_without_its_own_group`
      (`unconfigured`: `InvalidInput`, the foreground never moves)
    - `level2_foreground_handoff_unwind_stops_the_tree_then_restores_the_terminal`
      (the panic is caught outside the scope; the foreground is back before
      the reap; the child died by signal 9; the grandchild is gone; Drop did
      the restoration)
  - Each test asserts the PGID/TPGID before and after, `stty -a` equality
    (excluding `pendin`), and that the idle pane shell owns the foreground,
    read with `ps` from outside the pane.
  - Mutation check: replacing `restore()` with `mem::forget` and suppressing
    the unwind drop failed 4/5. `unconfigured` correctly still passed.
- **Prove foreground shells** (claudine-cli, F3 through the `claudine`
  binary in tmux):
  - New Unix-only `claudine/cli/tests/level2/level2_lifecycle_foreground_handoff.rs`,
    with 8 tests named `level2_lifecycle_foreground_handoff_*`:
    - `typed_ctrl_c_reaches_the_command_and_cancels_the_run`
    - `sigint_to_claudine_cancels_the_foreground_command`
    - `a_successful_command_returns_the_terminal_and_the_run_continues`
    - `a_failing_command_returns_the_terminal_and_fails_the_run`
    - `a_nested_claudine_hands_the_foreground_down_and_back`
    - `a_structured_command_reads_the_typed_line`
    - `a_background_claudine_leaves_the_foreground_alone`
    - `a_command_reads_dev_tty_when_claudine_stdin_is_redirected`
  - The C-c and SIGINT cases assert:
    - the root and the INT/TERM-ignoring background child are dead;
    - `events.log` is exactly `cancelled\nfinalize\n`;
    - the later `no_error` start action and the provider never ran;
    - exit 130, with `stty` and the host foreground restored.
  - Ownership is read with `ps -o pgid= -o tpgid=` inside each command.
  - Every pid has a SIGKILL reaper on drop; every wait is bounded.
  - Mutation check: preparing the handoff on a dummy command failed 7/8.
    The background-host case correctly still passed. `shell.rs` was
    verified byte-identical afterwards.
  - `level1_lifecycle_shell_terminal_pty.rs` needed no change (Phase 2 had
    fixed its comment).
  - Pre-existing defect found: a background `claudine` with terminal output
    is stopped by SIGTTOU from biscuit-terminal's raw-mode probe. It is not
    fixed here (see decisions_for_review), and the background-host test
    redirects stdout/stderr around it.
- **Preserve host escalation**:
  - Before the change, `level2_lifecycle_ctrl_c_tmux` failed 4/6. Every
    terminal-lifecycle scenario failed because typed keys now reach the
    foreground `trap '' INT` shell, not Claudine.
  - Added the rule-13 hold behind `#[cfg(feature = "test-fixtures")]` in
    `ClaudineLifecycleEffects::info`, under the exact text
    `terminal-lifecycle-test-hold` and `CLAUDINE_TEST_LIFECYCLE_HOLD=<prefix>`.
    It emits the text first, writes `.reached`, then polls `.release` for at
    most 60 s.
  - `terminal_interrupt.rs` now has `Barrier::{Shell, Host}`. A host
    barrier only writes its release file. `orphan_teardown` keeps the shell
    barrier.
  - The level2/level3 ctrl-c test files are unchanged. The env read is
    listed in `context_construction_guard.rs`.
  - Also fixed `claudine/justfile` `test-l3` to enable `test-fixtures` (see
    decisions), and added the seam row to `claudine/docs/topics/testing.md`.
  - After: 6/6 pass. L3 was compiled only (`--no-run`) and never run.
    `cargo check -p claudine` (default features) is clean, so the default
    build has no hold.
- **Document shared behavior**:
  - `process-trees.md` now has the full handoff section: a complete Rust
    example, a Mermaid flow including the WouldBlock, setup-error and unwind
    paths, inactive guards, contention, typed vs host-directed SIGINT,
    error priority, and RAII limits. Its caller table was corrected:
    Darkmatter hands off only to a terminal editor that inherits stdin, and
    the Claudine trigger reads "cancellation" rather than Ctrl-C.
  - Updated the architecture and README overview links.
  - Claudine: lifecycle.md (valid `start:` example and caller rules),
    signal-handling.md ("Ctrl+C while a lifecycle command holds the
    terminal"), and sequences.md (task commands never take the terminal).
  - Darkmatter: lifecycle/index.md, error-handling.md, cli/edit.md, and
    dependencies.md (stale libc sentence).
  - Skills: sniff SKILL/architecture/testing, claudine SKILL, darkmatter
    lifecycle, os SKILL/windows.
  - Caller skills: use-claudine lifecycle.md and use-darkmatter
    authoring.md, published with `just publish-skills` in both areas.
  - `holds_terminal_foreground` / `run_in_own_group` / the foreground
    exception now appear only under `sniff/fixes/`.
- Requirement → test mapping added this phase:

  | Requirement | Test |
  |---|---|
  | Real transfer and restore through the public guard (success, nonzero, spawn error after transfer, unconfigured refusal, unwind) | sniff L2 ×5 above |
  | Typed input reaches a lifecycle command | `…a_successful_command…`, `…structured_command_reads_the_typed_line`, `…dev_tty_when_claudine_stdin_is_redirected` |
  | Typed C-c and host-directed SIGINT cancel the run, kill descendants, skip later `no_error` work, and run cancelled/finalize once with exit 130 | `…typed_ctrl_c…`, `…sigint_to_claudine…` |
  | Nested restoration one level at a time | `…nested_claudine…` |
  | A background host leaves the foreground untouched and still owns its tree | `…background_claudine…` |
  | The host escalation ladder still works for in-process lifecycle work | `level2_lifecycle_ctrl_c_tmux` (6, unchanged assertions) |

- Spec example correction (rule 14): every new fixture and doc example uses
  `start:`, never `initialize: shell`.
- No file-format reader changed, so the input-robustness matrix does not
  apply.
- Gates (macOS; load average 7–10 on a 16+ core host):
  - `cd sniff && just test`: 3646 passed, 34 skipped, none at or above
    5.0 s (`🐢 … none at or above 5.0s`).
  - `cd sniff && just lint`: clean.
  - `cargo clippy -p sniff-detection-cli --all-targets --features test-fixtures -- -D warnings`:
    clean.
  - `just check-tier-coverage sniff` and `just check-tier-coverage claudine`:
    0 stranded.
  - `just slow-tests fresh`: exit 0.
  - `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_foreground_handoff`
    (sniff): 5/5 passed, `backend-proof: tmux run=5 skip=0`.
  - `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_lifecycle_foreground_handoff level2_lifecycle_ctrl_c_tmux`
    (claudine): claudine-cli 14/14 passed, `tmux run=14 skip=0`. The recipe
    exits 1 only because the filtered `claudine-gen` leg runs 0 tests and
    its required-backend proof fails. That is an artifact of the filter, not
    a test failure.
  - Claudine targeted L1 (`just test effects context_construction_guard
    level1_lifecycle_shell_terminal_pty lifecycle_tests interrupt_stops`):
    40/40 passed, none slow. `cd claudine && just lint`: clean.
- Dependent and scope cells:
  - `affected_scope.py` over the working tree gives packages
    `claudine, claudine-cli, darkmatter, darkmatter-cli, sniff-detection-cli, test-toolkit`
    with `change_class` `package`, so the gates were not skippable.
  - darkmatter is selected by the Markdown edits. Its documentation and
    skill guard tests (`just test current_root_migration_guard
    trigger_spelling_guard current_root_documentation_contract
    ambient_ctx_capture schema_roots`) passed 51/51.
  - test-toolkit is selected by the test-layout changes. `cd tools && just
    test` passed 563/563, none slow.
  - Full darkmatter and claudine L1 are Phase 4's scoped gates.
- Cross-OS:
  - The new tests are Unix-only. The fixture's Windows path and the hold
    are portable.
  - `cargo check -p sniff-detection-cli --features test-fixtures --bins
    --tests --target x86_64-pc-windows-gnu` (check-windows environment)
    passed.
  - No native Windows runtime run was attempted: Phases 1–2 found build-win
    refusing on storage, and the Windows-side change is a no-op. `os_failing`
    is not set.
- Unrelated worktree files: untracked `claudine/docs/topics/config-files.md`
  and `claudine/docs/topics/schemas/schema-triggers.md` appeared during the
  session. No agent of this fix created them. They were left untouched and
  are not listed in this phase's frontmatter.
- Phase 3 tasks complete: all four Wave 3 checkboxes are checked.

## Phase 4

- Started 2026-11-16 with Phases 1–3 committed (HEAD `d33974e76`). The only
  uncommitted file at the start was the author's
  `claudine/docs/topics/schemas/schema-triggers.md`, which this fix does not
  own; it was left untouched.
- Phase 4 is validation only; it ran serially in this session with no
  subagents, so each gate ran on an otherwise idle host (start load ≈ 7.6).
- Full L1 on macOS (each area's `just test`, run serially):
  - `cd sniff && just test`: 3646 passed, 34 skipped. The first run
    recorded one slow test, `network::tests::test_detect_network_returns_info`,
    at 8.5 s (load 7.6 → 18 on 16 cores). That test is one of the few that
    deliberately make a live WAN-IP request. With the recipe's features it
    took 2.2–2.3 s alone. A second full run passed with none at or above
    5.0 s (2.9 s for that test), and `just slow-tests fresh` exits 0. It
    was a network-latency outlier in code this fix does not touch, so no
    change was made.
  - `cd darkmatter && just test`: 17049 passed, 10 skipped, none slow.
  - `cd claudine && just test`: 12261 passed, 15 skipped, none slow.
- Focused L2 on macOS through detached tmux (`BISCUIT_TEST_REQUIRED_BACKENDS=tmux`):
  - claudine-cli (run as its own leg, `just _test_l2 claudine-cli --features terminal-tests,test-fixtures …`,
    to avoid the claudine-gen 0-test leg): 23/23 passed,
    `backend-proof: tmux run=23 skip=0`. Breakdown:
    `level2_lifecycle_foreground_handoff` 8, `level2_lifecycle_ctrl_c_tmux`
    6, `level2_edit_interactive_capture` 7, `level2_edit_action` 2.
  - sniff `just test-l2 level2_foreground_handoff`: 5/5, `tmux run=5 skip=0`.
  - darkmatter `just test-l2 level2_edit_action`: 2/2 (darkmatter-cli),
    `tmux run=2 skip=0`; the library and DMLS legs match 0 tests.
  - No L3 was run, and no focus-enabling variable was set.
- Lint: `just lint` is clean in sniff, darkmatter, and claudine. Also clean:
  `cargo clippy -p sniff --all-targets -- -D warnings`,
  `cargo clippy -p sniff-detection-cli --all-targets -- -D warnings`, and the
  same with `--features test-fixtures`.
- `just check-tier-coverage` for sniff, darkmatter, and claudine: 0 stranded.
- Windows cross-compiles (x86_64-pc-windows-gnu, `RUSTC_WRAPPER=""`,
  `target/windows-check`, `-Wa,-mbig-obj`; compile evidence only):
  - `just check-windows` in sniff and in claudine: pass, no warnings.
  - `cargo check -p sniff-detection-cli --features test-fixtures --bins --tests`:
    pass.
  - `cargo check -p darkmatter -p darkmatter-cli --tests`: passed, but the
    darkmatter-cli `l1` test target had 7 Windows-only warnings. None came
    from this fix's changes (Phase 2 added only a `#[cfg(unix)]` test and no
    imports), but this fix's file `editor_launcher.rs` had three of them.
    All 7 are fixed, test code only:
    - `editor_launcher.rs`: the Unix-only imports (`Duration`/`Instant`,
      `kill_pid`, `EDITOR_TERMINATE_GRACE`) are now `#[cfg(unix)]`.
    - `editor_support.rs`: the Windows `kill_pid` was removed; nothing calls
      it on Windows.
    - `editor_service.rs` (2) and `compose_edit_action.rs` (1): a `let mut`
      that only Unix-gated code pushes to now carries
      `#[cfg_attr(not(unix), allow(unused_mut))]`.
    - The re-check is warning-free. On macOS the touched tests (`just test
      editor_launcher editor_service compose_edit_action editor_support`)
      passed 38/38, none slow, and darkmatter `just lint` is clean.
- Obsolete-implementation search (live source, docs, and skills outside
  `fixes/`/`features/`):
  - `holds_terminal_foreground` and `run_in_own_group`: no matches.
  - `tcsetpgrp`/`TIOCSPGRP` in darkmatter or claudine: no matches.
  - `tcsetattr`/`tcgetattr` remain only in the test PTY helper
    `claudine/cli/tests/common/pty.rs`.
  - "direct-child-only" survives only in `shell.rs` docs that say no such
    degraded path exists.
  - Foreground *detection* in `darkmatter/lib/src/editor/surface.rs`
    (`tcgetpgrp == getpgrp`) is unrelated and was kept, as the plan asks.
- Edit test files: `git diff 3e32c4a7c HEAD` (the commit before this fix's
  first commit, `5bf7e9bc1`) shows no change to
  `darkmatter/cli/tests/level2/level2_edit_action.rs`,
  `claudine/cli/tests/level2/level2_edit_action.rs`, or
  `claudine/cli/tests/level2/level2_edit_interactive_capture.rs`.
- Runtime evidence on other OSes. All three rigs (`BUILD_LINUX`, `BUILD_WIN`,
  `BUILD_WSL`) are declared on this host now; the plan expected only Linux.
  Every run went through `just cross-check` / `scripts/cross-check.sh` (the
  repository's standing-clone flow, as the task prompt requires), not a
  hand-made tar copy. No source was edited on any host.
  - **Linux (build-linux) L1**, canonical archive mode unless noted:
    - sniff: 2644 passed, 32 skipped, none slow.
    - sniff-detection-cli: 979 passed, 13 skipped, none slow.
    - darkmatter-cli: 2606 passed, 85 skipped, none slow.
    - claudine: 6749 passed, 5 skipped, none slow.
    - claudine-cli: the first run failed to *link*
      (`collect2: ld terminated with signal 9`); the host killed the linker
      for memory, and SSH stopped answering meanwhile. The retry with
      `--build-jobs 1` passed 5192, skipped 315, none slow. The os skill's
      `linux.md` now records this for claudine-cli.
    - darkmatter: the archive build was killed the same way on the `browser`,
      `level2`, lib-test, and example links. `os/build-hosts.md` says not to
      retry the archive, so it ran in plain nextest mode:
      `--build-jobs 1 --features effects-instrumentation,work-counters --lib --test l1`.
      That passed 12480, skipped 4, and printed no SLOW line. This mode has
      no tier filter and records no receipt, so it is targeted Linux
      evidence, not a CI cell.
  - **Linux focused L2** (plain nextest through cross-check,
    `BISCUIT_TEST_REQUIRED_BACKENDS=tmux` forwarded, `--build-jobs 1`):
    - sniff-detection-cli `--features test-fixtures level2_foreground_handoff`:
      5/5, about 0.5 s each. These are real runs: a skip takes about 0.02 s,
      and the required backend turns a missing tmux into a panic.
    - claudine-cli `--features terminal-tests,test-fixtures` with the four
      filters: 23/23 (`…foreground_handoff` 8, `…ctrl_c_tmux` 6,
      `…edit_interactive_capture` 7, `…edit_action` 2). None finished in
      skip time.
    - darkmatter-cli `--features terminal-tests,test-fixtures level2_edit_action`:
      2/2, about 0.6 s each.
  - **Native Windows (build-win): runtime evidence unavailable.**
    `cross-check --os windows` for darkmatter-cli and sniff both stopped at
    the build host's `_storage_preflight` before compiling: 32.7 GiB free,
    50 GiB required, even after automatic Cargo reclaim. This is the same
    refusal Phases 1–2 hit. The documented override is for emergencies only,
    so it was not used. No test failed, so `os_failing` is not set. Native
    Windows runtime coverage is left to the nightly CI cells; the
    x86_64-pc-windows-gnu compiles above are compile evidence only.
  - **WSL2 (build-wsl): not run.** It shares a machine with build-win, which
    is short of storage, and the os skill warns that a WSL archive build
    competes with that host for memory and disk. No WSL2 runtime evidence
    exists for this fix; the nightly WSL2 CI cell remains responsible for it.
    Linux results are not WSL2 evidence.
- Closing record (the whole fix, Phases 1–4):
  - **Shared API.** `sniff::process::ForegroundHandoff`
    (`prepare(&mut Command)`, `is_active`, `restore`) is the only code in the
    repository that moves the terminal foreground. It works alongside
    `TreeConfig::process_group()` / `ProcessTree::attach`. The `/dev/tty`
    descriptor and signal-mask choices are in the Phase 1 entry:
    `O_NOCTTY` plus close-on-exec; `sigprocmask` in the `pre_exec` hook;
    `pthread_sigmask` on the restoring thread only. The process-wide claim
    flag is in decisions_for_review.
  - **Changed callers.** Darkmatter's `editor/launch.rs` hands off only to
    an owned terminal editor that inherits stdin. Claudine's exact-byte and
    structured lifecycle commands (`sequence/task/shell.rs`) always own
    their tree and hand off whenever Claudine holds the terminal. The former
    foreground exception and its direct-child-only runner are deleted.
  - **What changes for the keyboard.** While a lifecycle command or editor
    holds the foreground, typed keys, typed Ctrl-C included, go to that
    command, not to Claudine. A typed Ctrl-C that ends the command counts as
    cancellation, and Claudine then reaps the command's whole tree. A
    host-directed SIGINT to Claudine still cancels and reaps it. In-process
    lifecycle work keeps the host's escalation ladder.
  - **Test-only fixture correction.** The ctrl-c tmux fixture's
    `trap '' INT` lifecycle shell would now receive the typed keys itself.
    It was replaced by the `test-fixtures`-only in-process hold
    (`CLAUDINE_TEST_LIFECYCLE_HOLD`). The test files' assertions are
    unchanged.
  - **Spec example correction.** The spec's illustrative
    `initialize: shell` placement is not valid. Every doc and fixture uses
    `start:` instead.
  - Unrelated worktree state was kept as found: the author's modified
    `claudine/docs/topics/schemas/schema-triggers.md`. The untracked
    `claudine/docs/topics/config-files.md` no longer appears as untracked.
  - Behavior-changing symbols' comments were reviewed in Phases 1–3. Phase 4
    changed only test-only `cfg` gating, with no behavior change. Its one
    removed function, the Windows `kill_pid`, had no doc comment.
- **Concurrent edits by someone else.** From 16:18 on 2026-11-16, while the
  Linux runs were in progress, files outside this fix were changed in the
  working tree, most likely by the author:
  - `darkmatter/cli/src/args/policy.rs`, `commands/compose.rs`,
    `commands/compose/lifecycle.rs`, and `output/report.rs`;
  - `darkmatter/cli/tests/l1/output_envelope.rs` and `policy_verbs.rs`;
  - `darkmatter/lib/src/markdown/compose/report_diagnostics.rs`,
    `reference/file_tree/model.rs`, and `reference/validate.rs`;
  - five `darkmatter/docs/` pages;
  - `claudine/docs/topics/config-files.md`.

  None of them were touched, and none are listed in this phase's
  frontmatter. Every macOS gate above, and this phase's own test edits
  (15:03), came before 16:18, so those results describe this fix's tree
  alone. Each `cross-check` ships the whole working tree, so the last Linux
  L2 legs (about 16:10–16:38) may have included some of those in-progress
  edits; they passed anyway. The gates were not rerun over someone else's
  unfinished work.
- Scope calculator over this phase's own files (`.claude/skills/os/linux.md`,
  the four darkmatter-cli test files, plan, log, spec):
  `packages: ['darkmatter-cli', 'test-toolkit', 'darkmatter']`,
  `change_class: package`. The gates were therefore required, and they ran
  (above). Dependent check: `cd tools && just test` passed 563, skipped 4,
  none slow.
- Requirement → test mapping for this phase's one code change:

  | Change | Evidence |
  |---|---|
  | darkmatter-cli `l1` compiles warning-free for Windows | `cargo check -p darkmatter-cli --tests --target x86_64-pc-windows-gnu` (0 warnings; it was 7) |
  | No change in behavior on Unix | `just test editor_launcher editor_service compose_edit_action editor_support` 38/38; darkmatter `just lint` clean; Linux darkmatter-cli L1 2606/2606 |

  No file-format reader changed, so the input-robustness matrix does not
  apply.
- Skipped or unavailable: native Windows runtime (storage preflight), WSL2
  runtime (not attempted), and L3 (never run; it takes window focus). No
  test failed on any host.
- Phase 4 tasks complete: both Wave 4 checkboxes are checked. Final state:
  **implementation complete, ready for review**.
