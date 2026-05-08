---
agent: codex
model: ""
ready: false
---

# Review 2: Apple Terminal Integration Tests & Prose Graceful Degradation

## Summary

The previous review's main gaps are mostly addressed: `AppleTerminalHarness` is
exported, its unit tests compile and pass, `level2_apple_terminal_prose.rs`
exists, `just test-l2` includes the target, docs were updated, the probe now
uses the canonical OSC8 detector, and atomic `{{double-underline}}` now shares
the block-tag degradation policy.

I do **not** think this is production-ready yet. Two acceptance criteria still
have correctness / verification gaps:

- AC-3's "plain text / no escape codes" fallback still emits a final SGR reset
  in some suppressed-style paths.
- AC-2's Level-2 Terminal.app test can pass using the shell's echoed command
  line even if `bt prose` emits no rendered output.

## Verification Level Matrix

| Requirement | Strongest present verification | Required level | Status |
|---|---:|---:|---|
| AC-1: OSC8 unsupported renders `[description](url)` | Level 1 PTY + Level 2 Terminal.app | Level 2 | OK |
| AC-2: double underline falls back visibly in Apple Terminal | Level 1 PTY + flawed Level 2 Terminal.app | Level 2 | **Gap** |
| AC-3: neither double nor straight underline emits plain text with no escapes | Level 1, but assertions miss final reset | Level 1 | **Gap** |
| AC-4: Level-1 PTY with `TERM_PROGRAM=Apple_Terminal` | Level 1 PTY | Level 1 | OK |
| AC-5: AppleScript harness spawns, captures, and cleans up on Drop | Level 2, but cleanup assertion is best-effort | Level 2 | Needs tightening |
| AC-6: skip on CI / Terminal.app unavailable | Harness unit + Level 2 early return | Level 1 / Level 2 skip path | OK |

## Findings

### High: AC-3 suppressed underline can still emit `\x1b[0m`, violating the "no escape codes" fixture

The spec's no-underline fixture says `<double-underline>important text</double-underline>`
should render as `important text` with no escape codes when both double and
straight underline are unsupported. The parser still marks suppressed styles as
used:

- atomic suppress path sets `state.used_styles = true` at
  `biscuit-terminal/lib/src/components/prose.rs:1499-1508`
- block suppress path sets `state.used_styles = true` before matching
  `BlockTagAction::Suppress` at
  `biscuit-terminal/lib/src/components/prose.rs:1587-1593`
- the outer parser appends `\x1b[0m` whenever `state.used_styles` is true at
  `biscuit-terminal/lib/src/components/prose.rs:142-147`

The current tests only assert that `\x1b[4:2m`, `\x1b[4m`, and sometimes
`\x1b[24m` are absent
(`biscuit-terminal/lib/src/components/prose.rs:2230-2265` and
`biscuit-terminal/lib/src/components/prose.rs:2423-2454`). They do not assert
the exact `important text` output or absence of all `\x1b[` escapes.

Fix: for `BlockTagAction::Suppress`, do not mark `used_styles` unless the inner
content actually uses styles. For atomic `double-underline` suppressed by
capabilities, do not set `used_styles`. Add exact-output Level-1 assertions for
both block and atomic forms under a no-underline terminal profile.

### High: AC-2's Level-2 Terminal.app test can pass from command echo instead of rendered output

`level2_apple_terminal_double_underline_plain_text_visible` sends:

`bt prose '<double-underline>important text</double-underline>'`

and then asserts that the captured pane contains `important text`
(`biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs:158-168`). Terminal.app
captures the shell transcript, including the typed command line, so this
assertion passes even if `bt prose` crashes, exits early, or renders nothing.
The negative checks for `\x1b[4:2m` / `[4:2m` also still pass in that failure
mode (`biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs:171-185`).

This means AC-2's strongest real-terminal verification is effectively still
Level 1 for the rendered content. Per the requested Test Rigor rules, a
user-observable rendering requirement with the wrong strongest verification
level is a high-severity gap.

Fix: wrap the command output with unique sentinels and assert only on the slice
between them, or invoke a shell command such as:

```sh
printf '__BT_START__\n'; bt prose '<double-underline>important text</double-underline>'; printf '\n__BT_END__\n'
```

Then fail if the sentinel-bounded output is empty or lacks `important text`.

### Medium: AC-5 cleanup test is best-effort and can pass without proving Drop cleanup

The lifecycle test sends `exit` before the harness drops
(`biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs:225-275`). If the
user's Terminal.app preference closes windows automatically on shell exit, the
window may disappear before `Drop` is responsible for cleanup. If the window
does not disappear after `Drop`, the test logs a warning and still passes
(`biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs:284-315`).

That verifies "Drop did not panic", but it does not strictly verify the spec's
"cleanup runs on Drop without manual intervention" acceptance criterion.

Fix: expose a test-only cleanup outcome or injectable close command so the test
can assert that `close_window` was attempted and succeeded. If Terminal.app
preferences make physical disappearance nondeterministic, keep that part
best-effort, but make the Drop cleanup action itself observable.

### Medium: AppleTerminalHarness defaults to forced color, which disables the capability profile it is meant to test

`AppleTerminalHarness::spawn_shell` injects `FORCE_COLOR=1 CLICOLOR_FORCE=1`
(`biscuit-test-harness/src/apple_terminal.rs:169-189`). The Level-2 tests then
need a local `disable_color_forcing` helper because those variables route `bt`
through `Terminal::new_forced`, enabling `osc_link_support` and collapsing the
Apple Terminal degradation path
(`biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs:51-74`).

That is a harness ergonomics footgun: future Apple Terminal tests will silently
exercise the wrong capability profile unless every caller remembers to unset
these variables.

Fix: do not force `FORCE_COLOR` / `CLICOLOR_FORCE` in the Apple Terminal
harness by default. If a future test needs deterministic SGR, make that an
explicit opt-in on the command or harness.

## Positive Notes

- `cargo test -p biscuit-test-harness` passes: 25 tests, including the newly
  wired Apple Terminal module.
- `cargo test -p biscuit-terminal --test level1_apple_terminal_prose` passes:
  5 tests.
- `cargo test -p biscuit-terminal-cli --test level2_apple_terminal_prose --no-run`
  compiles the Level-2 target.
- `cargo test -p biscuit-terminal-cli --test level2_apple_terminal_prose -- --nocapture`
  passes on this host: 13 tests.

