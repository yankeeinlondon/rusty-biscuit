# Claudine Signal Handling

This is the canonical reference for how Claudine handles process signals
(`SIGINT`, `SIGTERM`, `SIGKILL`) and the user-interrupt flag they raise.
Other docs link here rather than duplicating the model.

The signal story has two distinct surfaces:

1. **User-driven Ctrl+C** (`SIGINT`) during a compose / inline-compose /
   sequence run, including the slow prep phase before the agent child is
   spawned and the post-execute lifecycle phase after it exits.
2. **Wrapper-driven termination** (`SIGTERM` → `SIGKILL`) of the agent
   child, used by the timeout watchdog ([timeouts.md](timeouts.md)) and
   the rate-limit / hung-process recovery paths.

The two surfaces share one infrastructure: a process-scoped
[`USER_INTERRUPTED`](#process-scoped-flag) flag that any synchronous
wrapper code can consult to short-circuit blocking work.

## Exit Codes

| Code | Meaning | Source |
|---|---|---|
| `0` | Successful run | child exit |
| `1`–`127` | Provider-defined / harness-defined error | child exit or synthesized |
| `124` | Hook handler exceeded `CLAUDINE_HANDLE_DEADLINE_SECONDS` | `claudine handle` self-abort |
| `130` | User pressed Ctrl+C — equivalent to `128 + SIGINT(2)` | compose interrupt guard |
| `143` | Wrapper-driven `SIGTERM` (rare, observable when watchdog fires before child reaps) | child exit signal |
| `137` | Wrapper-escalated `SIGKILL` after grace period | child exit signal |

Exit code `130` is the single source of truth for "user pressed Ctrl+C
during compose." The constant [`USER_INTERRUPT_EXIT_CODE`] in
[`claudine/cli/src/commands/compose.rs`](../../cli/src/commands/compose.rs)
matches the standard `128 + SIGINT(2)` shell convention.

## User-driven Ctrl+C (SIGINT)

When the user presses Ctrl+C during a compose / inline-compose / sequence
run, Claudine must:

1. Acknowledge the signal **immediately** with a one-shot stderr notice.
2. Mark a process-scoped flag so all synchronous post-prep / post-execute
   code can short-circuit instead of running through messenger sends, TTS
   playback, or summary rendering.
3. Allow the wrapped agent child (if one is currently running) to receive
   `SIGINT` via the natural foreground process group delivery, then
   escalate to `SIGTERM` / `SIGKILL` if the user keeps pressing Ctrl+C.
4. Exit with code `130`.

### The compose-scoped guard

[`install_user_interrupt_guard`](../../cli/src/commands/compose.rs) is
called once at the top of `run_compose_inner` and `run_inline_compose_inner`.
It returns an RAII [`UserInterruptGuard`] whose `Drop` removes the
registered handler so the next subcommand starts with a clean slate.

The handler itself runs in the signal-handling context, so it is restricted
to async-signal-safe operations:

- Sets the process-scoped [`USER_INTERRUPTED`] flag (atomic store, allocation-free).
- On the **first** SIGINT only, writes a pre-rendered notice to stderr via
  `libc::write(2)` (the only async-signal-safe way to print). The notice
  has a leading `\n` so it lands at column 1 (off the terminal's echoed
  `^C`) and renders the prompt path as an OSC8 hyperlink whose visible
  text is the user's CLI argument verbatim.
- Subsequent SIGINTs are no-ops at this layer — they are handled by the
  per-iteration wrapper handler described below, which escalates SIGINT →
  SIGTERM → SIGKILL on the wrapped child.

`signal_hook::low_level::register` stacks handlers, so this compose-scoped
guard composes cleanly with the per-iteration handler installed around
each agent child by `wait_with_signal_and_early_termination`.

### Process-scoped flag

The flag has two halves that are kept in sync by a single setter on the
CLI side:

```rust
// CLI: claudine/cli/src/output/mod.rs
static USER_INTERRUPTED: AtomicBool = AtomicBool::new(false);

pub(crate) fn mark_user_interrupted() {
    USER_INTERRUPTED.store(true, Ordering::SeqCst);
    claudine::interrupt::mark_interrupted();   // mirror to the lib flag
}

pub(crate) fn user_interrupt_observed() -> bool {
    USER_INTERRUPTED.load(Ordering::SeqCst)
}
```

The library mirror lives in
[`claudine/lib/src/interrupt.rs`](../../lib/src/interrupt.rs) and exposes
`mark_interrupted()`, `interrupted()`, and `clear_for_tests()`. It exists
because the lib must be able to consult the flag from blocking lifecycle
code (messenger sends, TTS playback) without taking a CLI dependency. The
CLI-side setter writes both halves atomically; the lib-side flag is
read-only from the rest of the lib.

Both halves are reset only by `clear_for_tests()` — once observed, the
flag stays observed for the rest of the process. Each compose subcommand
gets a fresh process so there is no cross-invocation leakage.

### Where the flag is checked

The flag is consulted at every point where Claudine could otherwise
spend appreciable time on work the user obviously wants stopped:

| Site | What it skips when set |
|---|---|
| [`compose.rs`](../../cli/src/commands/compose.rs) post-prep checkpoint (`run_compose_inner`, `run_inline_compose_inner`) | Returns `USER_INTERRUPT_EXIT_CODE` before launching the agent |
| [`compose.rs`](../../cli/src/commands/compose.rs) loop guard | Aborts loop iteration setup, returns `USER_INTERRUPT_EXIT_CODE` |
| [`composition::lifecycle::emit_signal`](../../lib/src/composition/lifecycle.rs) | Emits the cheap stderr line then returns; skips messenger send, desktop notification, TTS speech, sound effect |
| [`live_semantic_sink::errors::render_error_block`](../../cli/src/commands/wrap/live_semantic_sink/errors.rs) | Relabels the agent's dying-breath error from "Agent Error" to a yellow "User Action" block so operators see the real cause |

Unobserved (`false`): all the above sites run their full body. There is
no functional change to the happy path — every messenger destination,
every desktop notification, every TTS message, and every sound effect
fire exactly as configured.

### What the flag does NOT do

The flag does **not** interrupt synchronous Rust code. Rust has no
arbitrary-point cancellation, so a long-running call like
`sniff::filesystem::git::detect_git` runs to completion even after Ctrl+C
is pressed. The flag only short-circuits at the next explicit check,
which is why post-execute lifecycle work (the place where multi-second
delays accumulate) is the most important consumer.

If you are adding a new blocking call to compose / inline-compose /
sequence and it could plausibly take more than ~100 ms (network I/O,
subprocess wait, large sniff scan, TTS subprocess), guard it with
`crate::interrupt::interrupted()` (lib) or
`crate::output::user_interrupt_observed()` (CLI).

### Lifecycle short-circuit details

The lifecycle short-circuit lives in
[`LifecycleRunGuard::emit_signal`](../../lib/src/composition/lifecycle.rs)
and is the single most user-visible payoff of the interrupt flag. The
`emit_signal` body runs in this order:

1. Look up the configured notification for the signal (`Start`, `Success`,
   `Blocked`, `Failure`). If absent, return.
2. **Always** emit the configured `stderr` line. This is a cheap terminal
   write that gives the user feedback that the terminal status was
   reached, even when interrupted.
3. **If the interrupt flag is set, return.**
4. Otherwise:
   - Dispatch the configured `message` through the resolved messaging
     route (Discord webhook, Slack webhook, bot send, etc.).
   - Fire the configured desktop `notify` title.
   - Walk the audio phases: `say` / `say_first` (TTS) and `effect`
     (sound) in the configured order.

| Side effect | Normal run | After Ctrl+C |
|---|---|---|
| Stderr status line | ✓ | ✓ |
| Messenger send | ✓ | ✗ skipped |
| Desktop notification | ✓ | ✗ skipped |
| TTS speech | ✓ | ✗ skipped |
| Sound effect | ✓ | ✗ skipped |

The audio loop also re-checks the flag between phases so a Ctrl+C
landing during a multi-second TTS subprocess can still skip a queued
sound effect.

Two regression tests pin this contract:

- [`emit_signal_skips_blocking_side_effects_when_interrupted`](../../lib/src/composition/lifecycle.rs)
  asserts that with the flag set, `emit_terminal(LifecycleSignal::Failure)`
  produces only the stderr line.
- [`emit_signal_runs_all_side_effects_when_not_interrupted`](../../lib/src/composition/lifecycle.rs)
  asserts the happy path still emits every configured side effect.

### Error-block relabeling

When the wrapped agent process dies as a side effect of the user
pressing Ctrl+C, the parser sees a non-zero exit / generic agent error
and would normally render a red "Agent Error" status block. That mislead
the operator about cause.

[`render_error_block`](../../cli/src/commands/wrap/live_semantic_sink/errors.rs)
checks `user_interrupt_observed()` and, when set, replaces the block
content with a yellow "User Action" block whose body reads
`User pressed CTRL+C to stop the session`. The classification machinery
itself is unchanged — only the rendering surface relabels.

## Wrapper-driven termination of the agent child

Independently of the user-interrupt flag, the wrapper layer manages
SIGTERM / SIGKILL escalation against the wrapped agent child. The
relevant primitives live in
[`claudine/cli/src/commands/wrap/exec/`](../../cli/src/commands/wrap/exec/).

### Per-child SIGINT escalation (interactive runs)

[`wait_with_signal_and_early_termination`](../../cli/src/commands/wrap/exec/termination.rs)
installs its own `signal_hook` SIGINT handler around each spawned agent
child. The handler counts repeated SIGINTs delivered to the wrapper
process and escalates against the child's process group:

| SIGINT number | Action against child PGID |
|---|---|
| 1 | `kill(-pgid, SIGINT)` — let the child handle it gracefully |
| 2 | `kill(-pgid, SIGTERM)` — request termination |
| 3+ | `kill(-pgid, SIGKILL)` — force termination |

The handler is a no-op when `child_in_own_pgroup` is `false` (interactive
TUI children share the wrapper's process group, and the terminal driver
already delivers SIGINT directly via the foreground PGID).

A `child_exited` atomic guards against signaling a recycled PID after
the child has been reaped.

### Watchdog-driven SIGTERM

The unified timeout ticker (see [timeouts.md](timeouts.md)) sends
[`WatchdogTermination`](../../cli/src/commands/wrap/exec/termination.rs)
requests through a channel when `timeout` (wall-clock) or `step_timeout`
(stream silence) is breached. The wait loop receives the request,
sends `SIGTERM` to the child's process group, waits a configurable
grace period (default `10s`, override via `CLAUDINE_KILL_GRACE`), then
escalates to `SIGKILL`.

These signals are wrapper-initiated and are independent of any user
SIGINT. The synthesized `session_end` JSONL event records the breach as
`error_kind: "timeout"` or `"step_timeout"` so downstream tooling can
distinguish a user cancel (exit 130, no synthesized error_kind) from a
watchdog kill (exit 1, error_kind set).

### Process group isolation

When the wrapper sets `process_group(0)` on the child, it puts the
child in its own process group so:

- The wrapper can signal the child's entire group (`-pgid`) without
  hitting wrapper-internal threads.
- A misbehaving grandchild (e.g. an MCP server the agent spawned) can
  be cleaned up by signaling the group.
- The wrapper's own SIGINT handler is responsible for forwarding signals
  to the child group, since the terminal driver no longer delivers them
  automatically.

For interactive provider TUIs (Codex, OpenCode interactive, etc.) the
child shares the wrapper's process group instead so the terminal can
deliver keyboard interrupts directly. Both wait paths handle both
configurations via the `child_in_own_pgroup` flag.

## Hook handler deadline (SIGALRM-equivalent)

[`claudine handle`](../../cli/src/commands/handle.rs) — the entry point
provider hooks call into — enforces a per-invocation execution deadline
(default `15s`, overridable via `CLAUDINE_HANDLE_DEADLINE_SECONDS`).
When the deadline is exceeded the handler aborts with exit code `124`
to prevent a slow handler from blocking the parent agent session.

This deadline is enforced by checking elapsed time against the budget at
each phase boundary, not by `SIGALRM`. There is no signal involvement
beyond the standard `_exit(124)`.

## Signal-safety rules for new code

When adding code that runs in a signal-handling context (any closure
passed to `signal_hook::low_level::register`), the following are the only
allowed operations:

- Atomic loads / stores on `AtomicBool`, `AtomicU8`, etc. (lock-free).
- `libc::write(2)` to a known file descriptor with a pre-allocated buffer.
- `libc::kill` to send a signal to a known PID/PGID.
- Reading from `Arc<T>` clones whose `T` is `Sync` and lock-free.

Forbidden:

- `eprintln!` / `println!` / `format!` / any `tracing::*` macro (allocates).
- Mutex acquisition (deadlock risk if interrupted thread held it).
- Any FFI call not documented as async-signal-safe.
- Subprocess spawning (`std::process::Command::spawn` is not safe).

When in doubt, set a flag in the signal handler and do the real work in
the next synchronous code path that consults the flag — exactly the
pattern the user-interrupt guard uses.

## Cross-references

- [Timeouts](timeouts.md) — wall-clock and stream-silence rules that
  drive wrapper-initiated SIGTERM / SIGKILL.
- [Lifecycle](lifecycle.md) — the `Start` / `Success` / `Blocked` /
  `Failure` signals whose post-execute side effects are gated on the
  user-interrupt flag.
- [Execution Flow](execution-flow.md) — where the compose interrupt guard
  is installed in the larger compose / inline-compose / sequence pipeline.

## Source map

| Concern | File |
|---|---|
| `USER_INTERRUPTED` flag (CLI) | [`cli/src/output/mod.rs`](../../cli/src/output/mod.rs) |
| Lib-side mirror flag | [`lib/src/interrupt.rs`](../../lib/src/interrupt.rs) |
| `install_user_interrupt_guard` + `UserInterruptGuard` | [`cli/src/commands/compose.rs`](../../cli/src/commands/compose.rs) |
| `USER_INTERRUPT_EXIT_CODE` constant | [`cli/src/commands/compose.rs`](../../cli/src/commands/compose.rs) |
| Lifecycle short-circuit (`emit_signal`) | [`lib/src/composition/lifecycle.rs`](../../lib/src/composition/lifecycle.rs) |
| Error-block relabeling | [`cli/src/commands/wrap/live_semantic_sink/errors.rs`](../../cli/src/commands/wrap/live_semantic_sink/errors.rs) |
| Per-child SIGINT escalation | [`cli/src/commands/wrap/exec/termination.rs`](../../cli/src/commands/wrap/exec/termination.rs), [`spawn.rs`](../../cli/src/commands/wrap/exec/spawn.rs) |
| Watchdog-driven SIGTERM | [`cli/src/commands/wrap/exec/watchdog.rs`](../../cli/src/commands/wrap/exec/watchdog.rs) |
| Hook handler deadline | [`cli/src/commands/handle.rs`](../../cli/src/commands/handle.rs) |
