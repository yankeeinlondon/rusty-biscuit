---
ready: true
agent: codex
model: ""
resolved_by: claude_opus_4_7
resolved_at: 2026-05-25
---

# Review #8 — Testing Best Practices

## Findings

### High — `SharedHarness` adoption does not reduce canonical `test-l2` spawn cost under nextest

Files:

- `/Volumes/coding/personal/rusty-biscuit/just/devops.just:245`
- `/Volumes/coding/personal/rusty-biscuit/biscuit-test-harness/src/shared.rs:1`
- `/Volumes/coding/personal/rusty-biscuit/biscuit-tui/cli/tests/real_terminal_render.rs:27`

The review-7 blocker was closed by replacing per-test local harness variables with `static SharedHarness<T>` values in biscuit-terminal and biscuit-tui. That compiles, but it does not actually provide the suite-level reuse the spec and review-7 resolution rely on when developers use the canonical recipes.

`just _test_l2` runs:

```bash
cargo nextest run -p {{ pkg }} -E 'test(/level2_/)' --no-tests=pass {{ args }}
```

Nextest executes each selected test in a separate test-binary process. `SharedHarness<T>` is explicitly a process-local static wrapper: its docs say reuse happens "when a test binary holds many `#[serial]` tests against the same harness." Under nextest, each individual test gets a fresh process, so each process has an empty static, initializes a fresh WezTerm/Kitty/tmux/Apple Terminal pane, then exits and drops it. The new comments in `real_terminal_render.rs` say sharing "eliminates that overhead across the suite," but that is only true for `cargo test`/libtest running multiple tests in one process, not for the canonical `just test-l2` path.

Impact: review-7's M1 is not actually resolved for the command surface this feature standardizes. The canonical Level-2 suite still pays one real-terminal spawn per test, so the claimed 2-3 second spawn savings across biscuit-terminal and biscuit-tui are largely not realized in normal package workflows or CI. This is a spec/implementation mismatch, not just a documentation nit, because Phase 1 Task 1.4 and Topic 3 selected `SharedHarness` specifically to remove duplicated harness setup and reduce L2 wall-clock cost.

Suggested fix: either change the L2 recipe shape for shared-harness suites to run through libtest in a single process, for example `cargo test -p <pkg> --test <level2_binary> -- --test-threads=1` where appropriate, or redesign `SharedHarness`/the harness tests around nextest's process model. If nextest remains mandatory for `test-l2`, the documentation and review criteria should stop claiming cross-test spawn amortization from process-local statics.

## Verification-Level Audit

| Requirement | Strongest verification observed | Verdict |
| --- | --- | --- |
| `require_level!` env gating semantics | L1: `cargo test -p test-toolkit` passed | Pass |
| biscuit-terminal L2 test code compiles after migration | Compile: `cargo check --tests -p biscuit-terminal-cli` passed | Pass |
| biscuit-tui L2/L3 test code compiles after migration | Compile: `cargo check --tests -p tui-chrome-cli` passed | Pass |
| Canonical `test-l2` command amortizes real-terminal spawn cost via `SharedHarness` | Not verified; structurally incompatible with nextest process-per-test execution | Gap |
| User-observable terminal rendering requirements remain assigned to L2/L3 tests | L2/L3 tests are present and selected by `level2_`/`level3_` names | Pass for selection; not rerun against real terminals in this review |

## Notes

The review-7 comment/doc fixes are present: `claudine/justfile` no longer references `#[ignore]`, and `docs/testing-strategy.md` now includes `biscuit-tui` in the curated areas snapshot and removes it from the exclusions table.

I did not find a new correctness regression in the migrated test bodies from static inspection, and the targeted compile checks passed. The remaining blocker is the mismatch between process-local sharing and the canonical nextest runner.

## Resolution (2026-05-25)

Redesigned `SharedHarness` around nextest's process-per-test model so
`test-l2` actually realises the 2–3 s per-spawn amortization the spec
promises.

### Cross-process attach mode

Each backend harness gained an `owned: bool` flag plus three new APIs:

- `attach(id)` — construct a borrow-mode harness whose `Drop` is a
  no-op; an outer scope owns the pane.
- `shared_or_spawn()` — read the backend's `BISCUIT_SHARED_*_ID` env
  var; attach when set, fall through to a fresh spawn otherwise.
- For `AppleTerminalHarness`, a `shared_or_else(spawn)` variant lets
  the caller keep its custom `preserve_capabilities(true)` config in
  the fallback path while still attaching to the shared window when
  the env var is set.

Backend-specific public free functions (`wezterm::kill_pane_by_id`,
`kitty::close_window_by_id`, `tmux::kill_session_by_name`,
`apple_terminal::close_window_by_id`) expose the cleanup shells so the
broker can act on panes it didn't construct.

### `biscuit-harness-broker` binary

New bin target in `biscuit-test-harness`. Supports
`spawn <backend>` (prints the resource id and `mem::forget`s the
harness so `Drop` does not tear down the pane) and
`kill <backend> <id>`. Exit code `2` signals "backend unavailable on
this host" so the recipe can silently skip it. Apple Terminal spawns
with `preserve_capabilities(true)` to match the existing consumer.

### `_test_l2` recipe rewrite

`just/devops.just` now:

1. Builds the broker once.
2. Tries `broker spawn` for each of `wezterm`, `kitty`, `tmux`, and
   `apple-terminal`; exports the printed id via the corresponding
   `BISCUIT_SHARED_*_ID` env var on success, silently skips backends
   whose tooling is missing.
3. Runs `cargo nextest run -p <pkg> -E 'test(/level2_/)' -j 1` so
   tests share the single pane without contention.
4. Tears every spawned pane down in an `EXIT` trap via
   `broker kill <backend> <id>`.

`-j 1` is required because `serial_test` mutexes are process-local and
do not serialise across nextest's child processes. L2 tests are
`#[serial(level2)]`-tagged within their binary anyway, and only one
binary can usefully drive a single pane at a time, so the parallelism
loss is theoretical.

### Test migrations

All `SharedHarness::get_or_init` closures in biscuit-terminal,
biscuit-tui, and darkmatter changed from the spawn-fresh idiom to
`<Backend>Harness::shared_or_spawn().expect(...)` (or `shared_or_else`
for the Apple Terminal capability-preserving site). Dead
`SHELL_READY_MS` constants left over from the previous
post-spawn-settle pattern were removed.

### Docs

`biscuit-test-harness/src/shared.rs` module docs now explain both the
same-process atexit cleanup AND the cross-process broker model.
`biscuit-tui/cli/tests/real_terminal_render.rs`'s "Shared harness
reuse" comment block was rewritten so it no longer claims cross-test
amortization under nextest from process-local statics; it now
describes the broker-pre-spawn path correctly.
`docs/testing-strategy.md` gained a section under `test-l2` covering
the broker pre-spawn, env var contract, `-j 1`, and trap teardown.
`.claude/skills/rust-testing/SKILL.md` was updated with the new
recipe behaviour and the broker binary, and its frontmatter `hash:`
was regenerated.

### Verification

- `cargo check --tests` passes for `biscuit-terminal-cli`,
  `tui-chrome-cli`, `darkmatter`, and `darkmatter-cli`.
- `cargo check -p biscuit-test-harness --bins` passes (broker
  compiles).
- `just --list` re-parses the modified `just/devops.just`.
