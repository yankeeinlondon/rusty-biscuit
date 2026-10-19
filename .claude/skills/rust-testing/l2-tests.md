# Running L2 Tests (read before you run)

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

`level2_*` tests spawn real terminal windows and panes. Run them **only** through
`just test-l2`, never `cargo test` or a bare `cargo nextest run -E
'test(/level2_/)'`: the recipe owns pane spawning, teardown, and the
parallel-versus-serial choice. Bypassing it leaks windows on timeout or panic and
produces ambiguous `osascript`/PTY failures that look like regressions.

## Modes

- **Parallel self-spawn (default).** No broker, no `BISCUIT_SHARED_*`, `-j N`
  from `_test_l2_threads`. Every `shared_or_spawn()` takes its owned,
  `Drop`-cleaned fallback, so tests share no pane.
- **Serial (`l2-serial`, rare).** A broker pane per backend, `-j 1`. A package
  opts in with the `l2-serial` runner tool and a required `l2-serial-reason`;
  `BISCUIT_L2_THREADS=1` forces it for one run.

Thread count, the broker, and the manifest keys are documented in the
`biscuit-test-harness` skill. Gating and `available()` semantics are in
[tiers-and-gating.md](tiers-and-gating.md).

## Serialize the resource, not the tier

Serial exists for two hazards only: the single shared broker pane, and GUI
backends with global OS state. Declare `l2-serial` only when a test needs a
resource nothing else may touch concurrently, and name that resource in the
reason. When only some tests contend, serialize those with a nextest test group
instead, as `level2_apple_terminal_*` does with `apple-terminal-l2`
(`max-threads = 1`) in `.config/nextest.toml`.

| Backend        | Parallel-safe? |
| -------------- | -------------- |
| tmux           | Yes: headless, own sessions, reaps only dead-pid sessions. Prefer it for tests you want to fan out. |
| WezTerm        | Yes: off-screen `biscuit-bg` panes, but spawns are expensive. |
| Kitty          | Likely (same background model), not yet validated. |
| Apple Terminal | No: one global AppleScript state and a focus snapshot/restore. See [apple-terminal-harness-pitfalls.md](apple-terminal-harness-pitfalls.md). |

A self-isolating L2 test needs a unique temp dir (`{pid}-{nanos}-{atomic}`, never
a fixed path), cleanup that reaps only dead-pid resources, and no dependence on
shared pane geometry or state.

## Harness invariants

Spawning never steals foreground focus and never closes a window it did not
create.

## Failures

- **Single-backend walls** (every `*_in_wezterm` red) usually mean that emulator
  is absent or unscriptable here. Confirm on another backend first.
- **No retries.** Every nextest profile, L2 included, uses `retries = 0`; a test
  that passes only on retry is a failed run. Fix the contention or widen a
  justified, scoped timeout.
- **Two-phase capture race.** Polling for an intermediate marker and then taking
  a separate `capture()` can grab a half-painted frame under load. Poll for the
  content you will assert on, in the same loop.
- **Child teardown leaks:** check the effective leak policy first; see
  [recipes.md](recipes.md) § Leaked Process Detection.
