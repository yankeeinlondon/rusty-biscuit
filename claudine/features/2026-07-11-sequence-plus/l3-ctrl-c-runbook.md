# Level-3 Ctrl+C Runbook — Sequence Interruption

Procedure for producing the Level-3 evidence review 6 finding 3 requires
(carried forward from review 5 finding 3). Current status: the macOS test
passed once on 2026-07-18, but **that pass is stale** — it predates the
task-shell process-tree ownership rewrite (fail-closed `ProcessTree` with
reap-on-completion in `lib/src/composition/sequence/task/shell.rs`) and the
descendant-cleanup assertion the macOS fixture now carries. The Linux and
Windows tests have **never been run** on any host. All three must be run at the
current revision. Only an attended desktop session can produce this evidence,
because each test raises a GUI terminal to frontmost and injects real OS
keystrokes into whatever holds focus.

## The requirement being discharged

> Pressing Ctrl+C while a parallel group is active must interrupt every task,
> prevent the next sequence step from running, and exit 130.

Phrased as user keyboard behavior, so only OS key injection discharges it —
`libc::kill(SIGINT)`, `tmux send-keys C-c`, and `GenerateConsoleCtrlEvent` all
start *downstream* of the keyboard and the terminal's input encoder. Those
lower tiers are retained for diagnosis, not as substitutes: when an L3 test
fails, they are what separate "the fan-out is broken" from "the chord never
landed".

## Tests

| OS | File | Injector | Terminal |
|---|---|---|---|
| macOS | `cli/tests/level3_sequence_ctrl_c.rs` | `cliclick` click + System Events `keystroke … using control down` (Quartz) | WezTerm |
| Linux | `cli/tests/level3_linux_sequence_ctrl_c.rs` | `xdotool key ctrl+c` (X11 XTEST) | WezTerm |
| Windows | `cli/tests/level3_windows_sequence_ctrl_c.rs` | `SendKeys.SendWait("^c")` (`keybd_event`/`SendInput`) | WezTerm running `cmd.exe` |

Each is `#[serial(level3_keyboard)]` and gated by `require_level!(Level::L3, …)`,
so a host without the backend **skips cleanly** rather than failing.

## Before you start (all platforms)

1. **The machine must be free.** Do not type during the run. The chord goes to
   whatever owns focus.
2. **Close every other window whose title contains `claudine`.** All three tests
   resolve their target window by title and treat multiple matches as a hard
   error — by design, since injecting Ctrl+C into the wrong window is
   indistinguishable from a product regression.
3. **Launch the test from inside WezTerm**, or cold-start its mux server.
   `WezTermHarness::available()` requires `WEZTERM_UNIX_SOCKET`, which WezTerm
   exports only to processes it launches. Cold start:
   ```bash
   export WEZTERM_UNIX_SOCKET="$(ls "$HOME/.local/share/wezterm/gui-sock-"* | head -1)"
   ```
4. **Build first** so the spawn is not racing a compile:
   ```bash
   cargo build -p claudine-cli
   ```

### Per-OS prerequisites

- **macOS** — `brew install cliclick`. Grant **Accessibility** permission to the
  terminal app running the test (System Settings → Privacy & Security →
  Accessibility). Without it `AXRaise` silently fails and the chord lands
  elsewhere.
- **Linux** — `xdotool` installed and an **X11** session with `DISPLAY` set.
  Wayland has no XTEST path here and the test will skip; log in to an X11/Xorg
  session. The window manager must honor programmatic activation
  (`xdotool windowactivate`).
- **Windows** — `powershell` on `PATH`. Run as the interactive desktop user (not
  over a disconnected RDP session, where there is no foreground window to
  activate).

## Commands

Run from the `claudine/` package area. `just test-l3` prompts for confirmation
when it has a TTY; answer `y`.

The filter is **positional** (a nextest substring filter), not `-E`: the recipe
already passes `-E 'test(/level3_/)'`, and a second `-E` would *union* with it
— running the entire L3 tier, chooser tests included — whereas a positional
filter intersects, selecting only the named fixture.

```bash
# macOS
just test-l3 level3_sequence_ctrl_c

# Linux
just test-l3 level3_linux_sequence_ctrl_c

# Windows
just test-l3 level3_windows_sequence_ctrl_c
```

To run the whole L3 tier on a platform, drop the filter: `just test-l3`.

### The focus opt-in

`just test-l3` refuses to start unattended — with no TTY it exits non-zero
unless `BISCUIT_L3_TAKE_FOCUS=1` is set. That variable is **for a human who
knows the machine is free**. Set it only when running the tests yourself from a
non-interactive shell:

```bash
BISCUIT_L3_TAKE_FOCUS=1 just test-l3 level3_linux_sequence_ctrl_c
```

Never set it on an agent's behalf, and never in CI.

To make a missing backend a hard failure instead of a skip — useful to confirm
the run actually executed rather than quietly skipping:

```bash
BISCUIT_TEST_LEVEL_REQUIRED=3 just test-l3 level3_linux_sequence_ctrl_c
```

## What to record, per OS

Four observations. All four are asserted by the test, so a pass implies them —
but the review asks for them to be **recorded**, because a pass alone does not
distinguish "verified" from "skipped".

1. **Pane output** — the captured terminal frame. On failure the test prints it
   in the panic message. On success, capture it yourself: run the same fixture
   manually, or re-run with `--no-capture` and copy the pane dump. Look for the
   `^C` echo and the task-attributed interrupt lines.
2. **Exit code** — must be `130`. Visible in the pane as the sentinel line
   (`L3SEQ_0rc=130` on macOS, `L3LINSEQ_0rc=130` on Linux, `L3WINSEQ_0rc=130` on
   Windows). This doubles as proof the shell regained control: the sentinel is
   printed if and only if Claudine exited.
3. **Absence of the later-step marker** — `later-step-ran.txt` must not exist in
   the test's temp workspace. Proves the step after the interrupted group never
   launched.
4. **Descendant-process cleanup** — the tasks' descendants were reaped, not just
   their direct children. This is newly meaningful twice over: review 5
   finding 1 added Unix process-group and Windows Job Object tree ownership,
   and review 6 finding 2 made that ownership **fail-closed with
   reap-on-completion** — establishment failure aborts the task with
   `TaskShellError::Isolation`, and remaining tree members are killed on every
   exit path, success included, uniformly across macOS, Linux, and Windows.
   - macOS and Linux: each task forks a SIGINT-immune background subshell and
     publishes its pid to `<task>.desc.pid`; the test asserts every descendant
     pid is dead alongside every task pid.
   - Windows: each task launches `start /b ping -n 60N 127.0.0.1`, a grandchild
     that **inherits the task's stdout pipe**. The test counts live `PING.EXE`
     processes by command line and requires zero. If the Job Object did not reap
     the tree, the surviving descendant would fail that count (Claudine itself
     no longer hangs on it — the reader settle is bounded, so a held pipe costs
     at most the two-second shutdown grace).

Record each as: tested revision (`git rev-parse HEAD`), host OS and version,
terminal and version, verification level, exact command, and result.

## Interpreting a failure

- **Test skipped** (`skipping: requires …`) — the backend gate returned false.
  Re-run with `BISCUIT_TEST_LEVEL_REQUIRED=3` to turn it into a hard error that
  names the missing piece.
- **"N windows matched"** — close the extra `claudine`-titled windows.
- **Timed out waiting for the sentinel, pane shows the group still running** —
  the chord did not land. Check the per-OS prerequisites above (Accessibility on
  macOS, X11 + WM activation on Linux, foreground activation on Windows). Then
  run the lower tiers to confirm the fan-out itself is healthy:
  `just test-l2 level2_wrap_ctrl_c` (positional filter, for the same
  union-vs-intersect reason as above), and on Windows
  `just test-l2 level2_windows_sequence_ctrl_c`.
- **A descendant survived** — a genuine product defect in tree-scoped
  termination, not a harness problem.

Do not loosen an assertion to force a pass. If a test cannot be made reliable,
fix the harness — the macOS test went from never-green to reliable via two
harness changes (poll until frontmost rather than sleeping; send the chord as
one modified key event rather than racing modifier and letter).
