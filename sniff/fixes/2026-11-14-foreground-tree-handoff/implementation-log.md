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
message_to_agent: |-
  Phase 1 added sniff::process::ForegroundHandoff (sniff/lib/src/process/foreground.rs).
  Usage order for Phase 2 consumers: TreeConfig::process_group().configure(&mut cmd);
  let mut handoff = ForegroundHandoff::prepare(&mut cmd)?; (declare BEFORE the
  ProcessTree local); spawn; ProcessTree::attach; wait/sample; terminate or
  release; reap; then handoff.restore() before any output or further lifecycle
  work. prepare() returns WouldBlock if another handoff is active in the same
  process (a process-wide atomic claim, not a mutex; see decisions_for_review).
  The guard is Send. The pre_exec hook rejects a command that is not in a fresh
  group with EINVAL (ErrorKind::InvalidInput) at spawn time, and it must be
  registered after TreeConfig::configure (configure's own setsid/pgroup setup
  runs first). The process-trees.md caller table still lists the Claudine
  "not owned" foreground exception; remove it when Phase 2 migrates Claudine
  (Phase 3 owns the full doc rewrite). Windows: the build-win cross-check
  compiled the sniff test archive but could not run tests because the rig's
  cargo volume had 34.8 GiB free (50 GiB required); a local
  x86_64-pc-windows-gnu `cargo check -p sniff --lib --tests` passes.
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
