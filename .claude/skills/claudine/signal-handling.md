# Claudine Signal Handling

## Contents

- Exit Codes
- User-driven Ctrl+C (SIGINT)
- Wrapper-driven termination of the agent child
- Hook handler deadline (SIGALRM-equivalent)
- Signal-safety rules for new code
- Cross-references
- Source map

Use heading search to jump to the listed subsystem.


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
`claudine/cli/src/commands/compose/interrupt.rs`
matches the standard `128 + SIGINT(2)` shell convention.

### Termination labels

The numeric exit code does not fully distinguish *why* the child stopped;
the `ProcessTermination` label carried
on the attempt outcome does. Three of these are wrapper-driven kills that
share the same SIGTERM → SIGKILL escalation but classify differently:

| `ProcessTermination` | Meaning | Failure routing |
|---|---|---|
| `Completed` | Child exited on its own (exit code may still be non-zero) | `AgentFailure` only if exit ≠ 0 |
| `Interrupted` | User pressed Ctrl+C — no synthesized `error_kind` | suppressed (no recovery) |
| `TimedOut` | `timeout` / `step_timeout` watchdog kill | `Timeout` → `failure` stack `Retry`/`Resume` |
| `Aborted` | A claudine **content guard** tripped (exit-expression, runaway-repetition, or volume cap — see [timeouts.md](timeouts.md#content-guards-runaway-output)) | `AgentFailure` → `failure` fail-fast |

`Aborted` is deliberately distinct from `TimedOut` (it must **not** take
a `failure`-stack `Retry`, which would re-run the runaway) and from
`Interrupted` (a content trip is a genuine failure the operator's
lifecycle stacks must observe, not a silent user cancel).

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

`install_user_interrupt_guard` is
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
- On a **second-or-later** SIGINT that lands while **no child wait loop is
  active**, force-exits the wrapper with `libc::_exit(130)` (after a static
  async-signal-safe notice). This is the teeth the flag alone lacks: the
  interrupt flag only short-circuits at the *next explicit checkpoint*, so a
  synchronous call wedged on a network send or a hung TTS subprocess —
  notably in the prep phase or *between loop iterations*, where no agent
  child is running — would otherwise ignore Ctrl+C entirely. The guard reads
  [`crate::output::wait_loop_active`] (a depth counter raised for the
  lifetime of every `wait_with_signal_*` call); while a child wait loop *is*
  active, that loop's own handler owns the child-targeted SIGINT → SIGTERM →
  SIGKILL ladder, so the compose guard defers to it rather than killing the
  wrapper out from under a still-reaping child.

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
`claudine/lib/src/interrupt.rs` and exposes
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
| `compose/mod.rs` post-prep checkpoint (`run_compose_inner`, `run_inline_compose_inner`) | Returns `USER_INTERRUPT_EXIT_CODE` before launching the agent |
| `compose/loop_run.rs` loop guard | Aborts loop iteration setup, returns `USER_INTERRUPT_EXIT_CODE` |
| `composition::lifecycle::emit_signal` | Emits the cheap stderr line then returns; skips messenger send, desktop notification, TTS speech, sound effect |
| `EventRenderer::render_error_block` | Relabels the agent's dying-breath error from "Agent Error" to a yellow "User Action" block so operators see the real cause |

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

Two backstops cover the case where such a call is *already in flight* when
Ctrl+C lands (the flag check has already passed, so the flag cannot break
in):

1. **Bounded blocking lifecycle side effects.** The genuinely blocking
   lifecycle emitters — TTS (`say` / `say_first`) and sound `effect` — run
   on a detached worker thread bounded by `run_blocking_with_timeout`
   (`lib/src/composition/lifecycle/mod.rs`:
   `TTS_PLAYBACK_TIMEOUT` = 30 s, `EFFECT_PLAYBACK_TIMEOUT` = 15 s). A wedged
   audio device or network voice can no longer freeze a run between phases;
   the wait is abandoned and a warning logged. (`message` and `notify` are
   already fire-and-forget `tokio::spawn`s and never block the caller.)
2. **Second-press force-exit** (above) — the universal escape hatch when a
   blocking call outside any wait loop ignores the flag entirely.

### Lifecycle short-circuit details

The lifecycle short-circuit lives in
`LifecycleRunGuard::emit_signal`
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

- `emit_signal_skips_blocking_side_effects_when_interrupted`
  asserts that with the flag set, `emit_terminal(LifecycleSignal::Failure)`
  produces only the stderr line.
- `emit_signal_runs_all_side_effects_when_not_interrupted`
  asserts the happy path still emits every configured side effect.

### Error-block relabeling

When the wrapped agent process dies as a side effect of the user
pressing Ctrl+C, the parser sees a non-zero exit / generic agent error
and would normally render a red "Agent Error" status block. That mislead
the operator about cause.

`render_error_block`
checks `user_interrupt_observed()` and, when set, replaces the block
content with a yellow "User Action" block whose body reads
`User pressed CTRL+C to stop the session`. The classification machinery
itself is unchanged — only the rendering surface relabels.

## Wrapper-driven termination of the agent child

Independently of the user-interrupt flag, the wrapper layer manages
SIGTERM / SIGKILL escalation against the wrapped agent child. The
relevant primitives live in
`claudine/cli/src/commands/wrap/exec/`.

### The unified wait loop

Every spawn path now routes through one signal-aware wait loop,
`wait_with_signal_and_early_termination`:
the structured-streaming path (`run_child_stream_semantic`), the direct
path (`run_child`), and the capture path (`run_child_capture`). The
legacy `wait_with_timeout` helper — which installed **no** SIGINT
handler and killed the bare child PID rather than the process group —
was retired. Its removal closed a structural gap: opting into a
wall-clock `timeout` on the capture path used to silently disable Ctrl+C,
because the timeout branch took the no-signal helper. Timeout
enforcement is now delegated uniformly to the watchdog ticker (see
[Watchdog-driven SIGTERM](#watchdog-driven-sigterm)) for all three paths,
so a configured `timeout` is always group-targeted and signal-aware.

### Per-child SIGINT escalation

The wait loop installs its own `signal_hook` SIGINT handler around each
spawned agent child. The handler counts repeated SIGINTs delivered to the
wrapper process and escalates against the child's process group. The
ladder depends on whether the run is **interactive** (a human is present
to react to a graceful first SIGINT) or **non-interactive** (compose /
sequence with no human in the loop), the F5 shortened ladder:

| Press | Interactive (`SIGINT → SIGTERM → SIGKILL`) | Non-interactive (`SIGTERM → SIGKILL`) |
|---|---|---|
| 1 | `kill(-pgid, SIGINT)` — let the child handle it gracefully | `kill(-pgid, SIGTERM)` — request termination immediately |
| 2 | `kill(-pgid, SIGTERM)` — request termination | `kill(-pgid, SIGKILL)` — force termination |
| 3+ | `kill(-pgid, SIGKILL)` — force termination | `kill(-pgid, SIGKILL)` |

A non-interactive run compresses the ladder because there is no human to
read and act on a graceful SIGINT — pressing once (or a single
forwarded signal) should make immediate progress toward killing a
runaway. The `interactive` flag derives from the same
`effective_non_interactive` value the harness already computes.

**Visible feedback (Q14).** Each counted press emits a one-line stderr
acknowledgement (`⚠ interrupt received — press again to escalate`, then
`⚠ interrupt received — escalating`) so the user can see the press
registered even while a runaway is flooding stderr. The write is a single
async-signal-safe `libc::write(2)` of a static byte string — no
allocation, no formatting — so it obeys the
[signal-safety rules](#signal-safety-rules-for-new-code). It is emitted
even when the child shares the wrapper's process group, because the user
still deserves acknowledgement.

The handler does **not** re-signal the child when `child_in_own_pgroup`
is `false` (interactive TUI children share the wrapper's process group,
and the terminal driver already delivers SIGINT directly via the
foreground PGID); it still records the count for correct termination
labeling and emits the feedback line.

A `child_exited` atomic guards against signaling a recycled PID after
the child has been reaped.

### Watchdog-driven SIGTERM

The unified timeout ticker (see [timeouts.md](timeouts.md)) sends
`WatchdogTermination`
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

The same channel also carries **content-guard trips** (exit-expression,
runaway-repetition, volume cap — see
[timeouts.md](timeouts.md#content-guards-runaway-output)). The detector
and the stderr bridge are both senders on the single
`mpsc::Sender<EarlyTermination>` the loop's `early_rx` polls, so a trip
runs the identical SIGTERM → SIGKILL escalation. A content trip maps to
`ProcessTermination::Aborted` (not `TimedOut`), so it is fail-fast and
never retried.

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
deliver keyboard interrupts directly. The unified wait loop handles both
configurations via the `child_in_own_pgroup` flag.

### Windows parity

Windows has no SIGTERM/SIGKILL split and cannot signal a process group
the way Unix does, so the `#[cfg(not(unix))]` arm of the wait loop
(`windows_wait_loop`) mirrors the Unix semantics with native console and
Job-Object APIs (via the `windows` crate, gated to
`Win32_System_JobObjects`, `Win32_System_Console`, `Win32_Foundation`):

- The child is spawned with `CREATE_NEW_PROCESS_GROUP` and assigned to a
  **Job Object** so the entire process tree terminates as a unit (the
  Job-Object analogue of a Unix process group).
- A `SetConsoleCtrlHandler` callback observes `CTRL_C_EVENT` /
  `CTRL_BREAK_EVENT`, returning `TRUE` to suppress the default handler so
  the wrapper — not the OS — decides how to escalate.
- The escalation ladder maps onto Windows' two-state model: press 1 →
  `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, process_group)` (graceful;
  Ctrl+C events cannot target a specific group, but Ctrl+Break can),
  press 2 → `TerminateJobObject` (forceful, kills the whole tree). This
  is the Windows counterpart of `SIGTERM → SIGKILL`.
- The grace/reap deadlines are preserved so a wedged child cannot hang
  the wrapper, mirroring the Unix `POST_SIGKILL_REAP_TIMEOUT`.

The console handler must be a plain `extern "system" fn` (no captured
state), so the press counter it consults is a process-global atomic reset
at the top of each `windows_wait_loop` call.

> **Verification gap (all-OS rule).** The development host is macOS. The
> Windows path is **cross-compile-checked**
> (`cargo check --target x86_64-pc-windows-gnu -p claudine-cli --test level3_wrap_ctrl_c`).
> The package now exposes a Windows-host runtime gate,
> `just test-windows-ctrl-c`, and the path-filtered CI workflow
> `.github/workflows/claudine-windows-ctrl-c.yml` runs the same ignored
> console-control test by exact name. Full parity is not claimed until the
> Windows-host gate has a recorded green run.

#### Verification record

| Surface | Status | Where |
|---|---|---|
| Unix — OS-keyboard Ctrl+C terminates the wrapped child | **OS keyboard injection (cliclick → real WezTerm window), L3, `RUN_LEVEL3=1`.** Verified end-to-end on macOS, including passing automated `just test-l3` runs; the window-title matcher blocker is fixed, but the cliclick chord delivery is intermittent across WezTerm window placements (see note below) | `level3_wrap_ctrl_c.rs::level3_ctrl_c_terminates_wrapped_child` |
| Unix — OS-keyboard Ctrl+C terminates even with a wall-clock `timeout` configured | **OS keyboard injection (cliclick → real WezTerm window), L3, `RUN_LEVEL3=1`.** Verified end-to-end on macOS, including passing automated `just test-l3` runs; the window-title matcher blocker is fixed, but the cliclick chord delivery is intermittent across WezTerm window placements (see note below) | `level3_wrap_ctrl_c.rs::level3_ctrl_c_terminates_wrapped_child_with_timeout_configured` |
| Unix — multiplexer Ctrl+C terminates the wrapped child | **Verified on macOS** | `level2_wrap_ctrl_c_tmux.rs::level2_ctrl_c_terminates_wrapped_child` (tmux `send-keys C-c`, L2 multiplexer injection) |
| Unix — multiplexer Ctrl+C terminates even with a wall-clock `timeout` configured | **Verified on macOS** | `level2_wrap_ctrl_c_tmux.rs::level2_ctrl_c_terminates_wrapped_child_with_timeout_configured` (L2 multiplexer injection) |
| Unix — visible per-press feedback line renders in a real terminal | **Verified on macOS** | `level2_interrupt_feedback_capture.rs::level2_interrupt_feedback_renders_in_tmux` (L2, asserts the `interrupt received` substring in `frame.raw`) |
| Unix — process-signal SIGINT during prep exits 130 with notice | **Verified on macOS** | `wrap_sigint.rs::slow_compose_sigint_during_prep_exits_130_with_notice` (lower-level, retained) |
| Windows — console Ctrl+Break to the wrapped child's process group terminates the Job-Object child | **Real automated test with path-filtered Windows CI plus manual Windows-host gate; cross-compile-checked for `x86_64-pc-windows-gnu` on macOS; no recorded green Windows runtime run in this repo yet** | `level3_wrap_ctrl_c.rs::windows_ctrl_c_verification_record` (`#[cfg(windows)]`, `#[ignore]`d — needs an attached console); gates: `.github/workflows/claudine-windows-ctrl-c.yml`, `just test-windows-ctrl-c` |

Ctrl+C termination is verified at two distinct injection levels. The genuine
**L3** proof (`level3_wrap_ctrl_c.rs`) synthesises a real OS Ctrl+C chord with
`cliclick` into a focused WezTerm window: the terminal emulator's own input
encoder translates the keystroke to ETX → `SIGINT`, exercising the exact path a
physical keypress takes. The **L2** proof (`level2_wrap_ctrl_c_tmux.rs`) uses
tmux `send-keys C-c`, which is multiplexer-level terminal-CLI byte injection —
tmux writes ETX into the pane and the pane's line discipline raises `SIGINT`,
without the terminal emulator's input encoder ever participating. Both skip
cleanly when their backend is absent (the L3 tests are macOS-only, since
`cliclick` is the only injector wired into the harness), so the default `just
test` run stays green on a host without a terminal backend. Run the OS-keyboard
suite with `just test-l3` and the multiplexer suite (plus the feedback capture)
with `just test-l2`.

> **L3 automation note.** The window-title blocker is fixed. WezTerm overrides
> the OS window title with the foreground program's basename (`claudine`), so
> the harness's stamped tab title no longer matches; the test now registers the
> extra title via `WezTermHarness::with_expected_window_title("claudine")` and
> `focus_spawned_pane`'s AXRaise step reliably raises the window and returns
> valid click coordinates on every run. Automated `just test-l3` runs do pass
> (the child is terminated within ~3s on both the default and the
> `timeout`-configured paths).
>
> What remains intermittent is the cliclick chord delivery itself: on some
> WezTerm window placements (multi-monitor, cascaded positions) the OS
> focus-transfer click does not seat keyboard focus before the Ctrl chord
> fires, so WezTerm receives a bare `c` and no `SIGINT` reaches the child — the
> documented cliclick focus-transfer reliability limit, not a wrapper-behavior
> defect. The tests are not loosened to force a pass; they assert real
> termination and fail honestly when the OS event does not land.

Honest scope: the macOS host validates the Unix arm of the unified wait loop.
The Windows arm (`windows_wait_loop`) shares the loop's structure but uses a
distinct `#[cfg(not(unix))]` implementation. Its parity is exercised by
`windows_ctrl_c_verification_record` — a **real automated integration test**
(not a panic placeholder), mirroring the Unix L3 proof: it builds a
Windows-executable fake `opencode` provider (`opencode.cmd` on `PATH`), spawns
the real `claudine compose --opencode` wrapper in its own process group
(`CREATE_NEW_PROCESS_GROUP`), polls for the wrapped child's readiness marker,
injects `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, child_pid)`, and asserts the
wrapper child terminates within 15s. Ctrl+Break (not Ctrl+C) is used because
only `CTRL_BREAK_EVENT` can target a specific process group; the child runs in
its own group so the event never reaches the test runner.

On the macOS dev host this test is **cross-compile-checked** for
`x86_64-pc-windows-gnu`
(`cargo check --target x86_64-pc-windows-gnu -p claudine-cli --test level3_wrap_ctrl_c`)
but its **runtime pass has not yet been recorded**. The test stays `#[ignore]`d
in normal suites because it needs a Windows host with an attached console. The
path-filtered CI gate runs it for relevant PRs and pushes to `main`; run the
package-level Windows gate manually to reproduce or close the gap outside CI:

```text
just test-windows-ctrl-c
```

Expected environment: Windows runner/host, stable Rust toolchain, repository
checkout, and an attached console for `GenerateConsoleCtrlEvent`. A passing run
means the fake `opencode.cmd` reached its loop, Claudine delivered
`CTRL_BREAK_EVENT` to the wrapper child's process group, the Windows wait loop
terminated the Job Object tree, and the wrapper process exited within 15s. Until
that green run is recorded, the status is **Windows runtime-verification
available, not Windows runtime-verified**.

### Spawn × wait × timeout matrix

After the wait-path unification, Ctrl+C terminates the child on **every**
spawn path, including the previously-broken "timeout configured" column
that motivated the hardening (a configured `timeout` used to route the
capture and direct paths through the no-signal `wait_with_timeout`,
silently disabling Ctrl+C). All cells now share the one signal-aware
loop:

| Spawn path | no `timeout` | with `timeout` |
|---|---|---|
| `run_child_stream_semantic` (streaming) | unified loop | unified loop |
| `run_child` (direct) | unified loop | unified loop |
| `run_child_capture` (capture) | unified loop | unified loop |

Unix cells are covered by real-process tests
(`run_child_wall_clock_timeout_reaps_child`,
`run_child_capture_wall_clock_timeout_reaps_child`, and the
`escalation_signal` ladder tests in
`exec/termination/mod.rs`).
The Windows cells share the same `windows_wait_loop` and inherit its
runtime-verification gap above.

## Hook handler deadline (SIGALRM-equivalent)

`claudine handle` — the entry point
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
- Execution Flow — where the compose interrupt guard
  is installed in the larger compose / inline-compose / sequence pipeline.

## Source map

| Concern | File |
|---|---|
| `USER_INTERRUPTED` flag (CLI) | `cli/src/output/mod.rs` |
| `wait_loop_active` flag + `WaitLoopActiveGuard` | `cli/src/output/mod.rs` |
| Second-press force-exit (`_exit(130)`) | `cli/src/commands/compose/interrupt.rs` |
| Bounded lifecycle side effects (`run_blocking_with_timeout`) | `lib/src/composition/lifecycle/mod.rs` |
| Lib-side mirror flag | `lib/src/interrupt.rs` |
| `install_user_interrupt_guard` + `UserInterruptGuard` | `cli/src/commands/compose/interrupt.rs` |
| `USER_INTERRUPT_EXIT_CODE` constant | `cli/src/commands/compose/interrupt.rs` |
| Lifecycle short-circuit (`emit_signal`) | `lib/src/composition/lifecycle/mod.rs` |
| Error-block relabeling | `lib/src/render/event_renderer/error_block.rs` |
| Per-child SIGINT escalation | `cli/src/commands/wrap/exec/termination/mod.rs`, `spawn/mod.rs` |
| Watchdog-driven SIGTERM | `cli/src/commands/wrap/exec/watchdog/mod.rs` |
| Hook handler deadline | `cli/src/commands/handle.rs` |
