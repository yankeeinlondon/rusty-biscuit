# Running L2 Tests (read before you run)

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

`level2_*` tests spawn **real terminal windows / panes**. Run them **only** via
`just test-l2`, never `cargo test` / `cargo nextest run -E 'test(/level2_/)'`
directly: the recipe owns pane spawning/teardown and the serial-vs-parallel mode
choice. Bypassing it leaks windows on timeout/panic and produces ambiguous
`osascript`/PTY failures that look like — but are not — code regressions.

The recipe has two modes:

- **Default (parallel self-spawn):** skips the broker, exports no
  `BISCUIT_SHARED_*`, and runs `-j N` (`_test_l2_threads`, half the host's cores).
  Every `shared_or_spawn()` then takes its owned-pane fallback, so there is no
  shared resource.
- **Serial (`l2-serial`, rare):** pre-spawns one shared pane per backend via
  `biscuit-harness-broker`, exports `BISCUIT_SHARED_*_ID`, runs nextest
  `-j 1`, and tears the panes down in a trap. Tests calling
  `<Backend>Harness::shared_or_spawn()` attach to that pane. A package opts in
  with the `l2-serial` runner tool and a required `l2-serial-reason`.
  `BISCUIT_L2_THREADS=1` forces it for one run.

- A wall of single-backend failures (e.g. every `*_in_wezterm`) usually means
  that emulator is **absent/unscriptable here**, not that the renderer broke —
  confirm the same test on an available backend (`_in_kitty`, `_apple_terminal`).
- The Apple Terminal backend is GUI-automated and especially fragile (focus,
  `do script` window reuse, orphan leaks). Before touching it or debugging an
  `level2_apple_terminal_*` failure, read **`apple-terminal-harness-pitfalls.md`**.
- Spawning must **never steal foreground focus** and must **never close a window
  it did not create** — these are hard harness invariants.

### Serialization is per-*resource*, not per-*tier*

The default `-j 1` is **conservative**, not fundamental. It protects two specific
hazards, not the tier as a whole:

1. **The single shared broker pane** — every `shared_or_spawn()` test attaches to
   *one* pane per backend, so two at once would clobber it.
2. **GUI backends with global OS state** — WezTerm/Kitty window lists and focus,
   and especially **Apple Terminal's single global AppleScript state**
   (`AppleTerminalHarness` *must* stay serial).

A test that (a) spawns its **own** uniquely-named session/PTY (e.g.
`TmuxHarness::new() + spawn_shell()`, or its own `tmux new-session -s
…_{pid}_{seq}` it kills at the end) and (b) targets a **headless** backend has
**no shared resource** and is parallel-safe. That is why parallel is the
default: with no `BISCUIT_SHARED_*` exported, `shared_or_spawn()` itself falls
back to an **owned, `Drop`-cleaned** pane, so the whole tier self-isolates and
runs at `-j N`. N is `max(1, floor(logical_cores / 2))` on every host, CI
runners included (`_test_l2_threads`), fewer than L1's `logical_cores - 2`:
every worker starts a login shell and a terminal pane, the panes of concurrent
runs share one tmux server or WezTerm mux, and spawn and capture deadlines are
wall-clock, so overload fails tests rather than slowing them. Worker count does not set
CPU affinity or guarantee reserved capacity. Explicit `BISCUIT_L2_THREADS` takes
precedence. Declare `l2-serial` (with `l2-serial-reason`) only when a test truly
needs a resource no other test may touch at the same time, and write that
resource down as the reason. When only some tests contend, serialize just those
with a nextest test group instead: `level2_apple_terminal_*` tests share
`apple-terminal-l2` (`max-threads = 1`) so at most one visible Terminal window
is open, while the rest of their package stays parallel.

**Backend parallel-safety:** **tmux** = headless, immune to the host gotcha,
cleanup reaps only dead-pid sessions → fully parallel-safe (validated).
**WezTerm** = background panes (`biscuit-bg` workspace, off-screen) coexist and
sidestep the focus/window-list race → parallel-safe (validated), but spawn cost
is high. **Kitty** = same off-screen background model (likely parallel-safe, not
yet validated). **Apple Terminal** = **serial-only** (single global AppleScript
state + focus snapshot/restore). Prefer tmux for any L2 test you want to fan out.

Note the **`available()` bar is runtime reachability, not "installed"**: WezTerm
needs `WEZTERM_UNIX_SOCKET`, Kitty needs `KITTY_LISTEN_ON` — each exported *only*
to processes that terminal launches. So GUI-backend tests run only when the suite
is launched from inside that terminal (or it is cold-started with remote control;
see the `biscuit-test-harness` skill). A clean SKIP there is the host gotcha, not
a missing app.

**Parallel-safety prerequisites for a self-isolating L2 test:** unique temp dir
per test (key on `{pid}-{nanos}-{atomic}`, never a fixed path); cleanup that
reaps only **dead-pid** resources, never live ones; and no dependence on shared
pane geometry/state.

**Flakiness under parallel load** concentrates in timing-sensitive tests (signal
delivery, interactive choosers). Both the `default` and `ci` profiles use
`retries = 0`, including the L2 and browser overrides: a test that passes only
on retry is still a failed run. Fix the resource contention or widen a justified,
scoped timeout instead of masking the failure. Avoid
the **two-phase capture race**: polling for an *intermediate* marker (a chooser
hint) and then taking a *separate* `capture()` for the *final* content can grab a
half-painted frame under load — poll for the content you will assert on, in the
same loop, before capturing. Check the effective leak policy before diagnosing
concurrent child teardown; see [recipes.md](recipes.md) § Leaked Process Detection.
