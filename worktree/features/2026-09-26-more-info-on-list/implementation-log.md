---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/features/2026-09-26-more-info-on-list/spec.md"
plan: "worktree/features/2026-09-26-more-info-on-list/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages: []
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
---

# Implementation Log for 2026-09-26-more-info-on-list (5 phases)

## Phase 1

Phase 1 is investigation only: two spikes, no production edits. Both spikes
confirm the rulings; **no rule is overturned** and no Phase 3 `list.rs` task is
needed. One small Phase 4 subtask was added (an L2 `GREEN` constant).

### S1: width under the shell wrapper

**Source (`terminal_size` 0.4.4, the version in `Cargo.lock`).**

- Unix (`src/unix.rs`): `terminal_size()` tries stdout, then stderr, then
  stdin; each attempt is `isatty` + `tcgetwinsize`.
- Windows (`src/windows.rs`): the same order over `GetStdHandle(STD_OUTPUT_HANDLE)`,
  `STD_ERROR_HANDLE`, `STD_INPUT_HANDLE`, each through
  `GetConsoleScreenBufferInfo`; width is the visible window
  (`srWindow.Right - srWindow.Left + 1`), not the buffer width.
- `biscuit_terminal::Terminal::width()` is `fixed_width.unwrap_or_else(terminal_width)`,
  and `Terminal::default()` leaves `fixed_width` as `None`, so the width
  `wt list` sees is exactly `terminal_size()` (80 only when all three handles
  fail).

**Wrappers.** Every generated wrapper captures stdout only and leaves stderr on
the terminal: bash/zsh `out="$(WT_SHELL_WRAPPER=1 command wt "$@")"`, fish
`set -l out (… command wt $argv)`, PowerShell `$out = & $wtExe @args` (native
stderr is not redirected without `2>&1`).

**Empirical (macOS, detached tmux, no focus).** A scratch binary pinned to
`terminal_size = "=0.4.4"` (in `/tmp`, outside the repo) printed the width to
stderr and a `cd:` line to stdout. Results, per pane width:

| Pane | `x=$(probe)` | `probe </dev/null >/dev/null` | `probe 2>/dev/null \| cat` |
| --- | --- | --- | --- |
| 120 | 120 | 120 | 120 (via stdin) |
| 100 | 100 | 100 | 100 |
| 99 | 99 | 99 | 99 |

The captured stdout was the `cd:` line, as the wrapper expects.

**Outcome.** R2 stands with no code change: `Terminal::default()` in
`list.rs::run` already reports the pane width under the wrapper on Unix, and
the Windows source has the same stderr fallback. No `list.rs` change, no
`biscuit-terminal` change, and no wrapper L2 case is required. Windows was not
exercised empirically (source reading met the plan's outcome criterion), so
Phase 5's cross-OS note keeps "no Windows-specific code".

### S2: dim + color nesting in Prose and the L2 parser

Rendered through `Prose::new(markup).render(&terminal)` via a temporary
integration test in `worktree/cli/tests/` (deleted after the run), for
`<i><dim>clean</dim></i> <dim><green>+3</green></dim> <dim><red>-5</red></dim>`:

- `ColorDepth::TrueColor`:
  `"\u{1b}[3m\u{1b}[2mclean\u{1b}[0m\u{1b}[0m \u{1b}[2m\u{1b}[2m\u{1b}[32m+3\u{1b}[0m\u{1b}[2m\u{1b}[0m \u{1b}[2m\u{1b}[2m\u{1b}[31m-5\u{1b}[0m\u{1b}[2m\u{1b}[0m"`
- `ColorDepth::None`: `"clean +3 -5"` exactly (no escapes).

For Phase 3 L1 assertions, the stable substrings are
**`\u{1b}[2m\u{1b}[32m+3`** (dim green) and **`\u{1b}[2m\u{1b}[31m-5`** (dim red).
`<dim>` emits `\u{1b}[2m` twice, so do not assert the full prefix; assert
these substrings. Basic green is SGR 32 and red is SGR 31 (the same red as
`conflicts`, R7).

**L2 parser (`cli/tests/styled_capture/mod.rs`).** `Style` has `dim` (SGR 2,
cleared by 22) and `fg_is(Color)`; SGR 30–37 map to `Color::Indexed(0..=7)`.
A cell's dim and fg are available together, so `|s| s.dim && s.fg_is(GREEN)`
works. `level2_list_verbose.rs` has `RED = Color::Indexed(1)` but no green
constant, so Phase 4 adds `const GREEN: Color = Color::Indexed(2);` (no parser
change). Added as a Phase 4 subtask.

**Pane width.** `level2_list_verbose::list_until` uses
`TmuxHarness::new()` + `spawn_shell()`, which spawns at **120×40**
(`biscuit-test-harness/src/tmux.rs`, `resize` docs). That is ≥ 100, so the
metric assertions need no resize. A shared broker session could differ, so
Phase 4 should assert `harness.pane_cols() >= 100` before relying on metrics
(or `resize(120, 40)`); `TmuxHarness::resize` exists if a 99-column L2 check
is wanted.

### Rulings

R1–R12 stand unchanged.

### Gates

- No source, doc, or skill files changed in this phase, so there is no
  requirement-to-test mapping yet; the tests for R5, R8, and R10 are in Phase 2,
  and R1, R3, R7, and R9 are in Phase 3/4.
- `just test` in `worktree/`: 457 passed, 18 skipped (the tier-gated L2+ tests), none failed.
- `just lint` in `worktree/`: clean.
- The worktree skill was left unchanged. Phase 4 adds the metrics-gate line, and it
  can also record the S1 fact (the width comes from the stderr fallback under the wrapper).
