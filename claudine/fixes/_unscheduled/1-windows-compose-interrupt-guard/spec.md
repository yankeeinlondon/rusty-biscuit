# Windows compose interrupt guard is a no-op

`install_user_interrupt_guard` (`cli/src/commands/compose/interrupt.rs`) has a
real `#[cfg(unix)]` body and an empty `#[cfg(not(unix))]` one. On Windows,
`claudine compose` and `claudine inline-compose` therefore have:

- **no `USER_INTERRUPTED` marking** — `crate::output::mark_user_interrupted` is
  never called, so nothing downstream can tell a user cancel from a normal exit;
- **no second-press force-exit** — a wrapper wedged in a synchronous call with no
  child wait loop owning the ladder cannot be escaped with a repeated Ctrl+C.

The cross-compile emits the consequence as dead code: `mark_user_interrupted`
and `wait_loop_active` (`cli/src/output/mod.rs`) are both `never used` warnings
on `x86_64-pc-windows-gnu`, and `termination/windows.rs` still installs
`WaitLoopActiveGuard` whose only reader is Unix-only.

Surfaced as review-4 finding 8 of `features/2026-07-11-sequence-plus/review-4.md`.

## Why this is not sequence-plus's fix

Sequence-plus owns the *sequence* interrupt path, which now has a real Windows
producer (`register_sequence_interrupt_flag`). This gap is on the **compose**
path, which the feature does not otherwise touch. It is recorded here rather
than folded into that feature's close-out because:

1. Fixing it means adding new coordinator surface, not reusing existing surface.
   `register_sequence_interrupt_flag` registers an `Arc<AtomicBool>` and the
   console handler fans presses out to flags and child ladders. The compose guard
   needs neither: it needs a *callback* rung that writes a caller-supplied notice
   on press 1 and force-exits on press 2 when `!wait_loop_active()`.
2. **No runtime verification is possible from the current host.** Shipping this
   would add more never-executed Windows code, which is the exact weakness
   review-4 finding 1 identifies as this area's central risk. Closing a Low
   finding by deepening a High one is a bad trade.

## Suggested shape

Extend the process coordinator with a press-callback registration alongside the
existing flag registration, then give `install_user_interrupt_guard` a
`#[cfg(windows)]` body that registers one. The Unix body is the behavioral
specification — match its press semantics exactly, including the
`wait_loop_active()` gate that keeps the compose guard from `_exit`-ing out from
under a child escalation ladder.

## Acceptance

- `claudine compose` on Windows marks `USER_INTERRUPTED` on the first Ctrl+C.
- A second press with no active wait loop exits `130` after printing the
  force-exit notice.
- A second press *with* an active wait loop does not force-exit — the child
  ladder stays in charge.
- `mark_user_interrupted` and `wait_loop_active` no longer warn as unused under
  `just check-windows`.
- Verified by an executed run on a Windows host, not by cross-compile alone.
