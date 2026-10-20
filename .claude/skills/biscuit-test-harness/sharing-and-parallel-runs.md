# Sharing, Parallel Runs, and Cleanup

Part of the `biscuit-test-harness` skill; [SKILL.md](SKILL.md) is the index.

## `SpawnVisibility`

WezTerm and Kitty spawn in `Background` by default: an off-screen `biscuit-bg`
workspace in WezTerm, `--keep-focus` in Kitty. Use `Foreground` only for L3,
paired with `WezTermHarness::focus_spawned_pane()`. tmux sessions are detached,
and `AppleTerminalHarness` restores the frontmost window if a spawn displaced
it.

## How `just test-l2` runs

- **Parallel self-spawn is the default.** No pane is shared: every
  `<Backend>Harness::shared_or_spawn()` falls back to spawning an owned,
  `Drop`-cleaned pane, and nextest runs at half the host's logical cores
  (at least 1).
- **Serial is an opt-in that names its reason.** A package whose L2 tests
  contend for one resource declares it, and the planner rejects an empty reason:

  ```toml
  [package.metadata.ci.tests]
  runner-tools = ["l2-serial"]
  l2-serial-reason = "every test reads and writes the one system clipboard"
  ```

  Those packages get `biscuit-harness-broker`: one pre-spawned pane per backend,
  exported as `BISCUIT_SHARED_<BACKEND>_ID`, nextest at `-j 1`, and teardown in a
  trap. `BISCUIT_L2_THREADS` overrides both modes (`1` forces serial).
- **The shared Apple Terminal window opens only when the package's
  `l2-backends` lists `apple-terminal`**, because Terminal.app has no background
  mode. A package adding an Apple Terminal test declares that backend.
- When only some tests contend, serialize those with a nextest test group, not
  the whole tier (`rust-testing`'s `l2-tests.md`). `#[serial]` enforces nothing
  under nextest, which gives every test its own process.

For a binary that spawns many panes of its own, `SharedHarness<T>`
(`biscuit_test_harness::shared`) holds one per process and runs its `Drop` at
process exit, which Rust otherwise skips for statics.

## WezTerm clients take turns

The WezTerm GUI serves every `wezterm cli` client on its main thread and pushes
each connected client render changes, so many concurrent clients stall it. The
symptom is `WezTerm not available` or a spawn timeout on a different set of
tests each run, each passing alone. The harness therefore:

- admits at most two clients per mux at a time, across processes and worktrees,
  in arrival order, through lock files in the temporary directory; a crashed
  waiter's entry is detected and removed;
- runs every client with `--skip-config`, because the GUI exports the
  developer's config path into its panes;
- caches an answered `available()` probe briefly and sweeps stale panes at most
  periodically, since `list` is the heaviest call.

Go through the harness rather than calling `wezterm cli` yourself, and never
raise a WezTerm timeout to hide this symptom.

## Cleanup and ownership

Panes, sessions, and windows are tagged with an owning pid. A run closes only
tagged resources whose owner is dead, so concurrent runs are left alone.

- The owner is the spawning process unless `BISCUIT_HARNESS_OWNER_PID` names a
  live process. `_test_l2` exports its own pid before the broker spawns,
  because the broker exits immediately and a pane tagged with its pid would
  look abandoned to any other run's reaper.
- `cleanup_stale_terminal_harness_resources()` runs the sweep on demand; each
  backend also runs its own once per process.
- Untagged legacy WezTerm panes in `biscuit-bg` are swept when the workspace
  grows large, or when `BISCUIT_TEST_HARNESS_SWEEP_LEGACY_WEZTERM=1`.
